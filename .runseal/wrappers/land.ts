import {
  booleanOption,
  helpRequested,
  parseArgs as parseCliArgs,
  requireNoPositionals,
  stringOption,
} from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";
import { runseal } from "@/lib/std/runseal.ts";

type Options = {
  base: string;
  body: string;
  dryRun: boolean;
  deleteBranch: boolean;
};

function usage(): void {
  io.print("Usage: runseal :land [options]");
  io.print("");
  io.print("Land the current clean topic branch on GitHub.");
  io.print("The branch is pushed, a PR is created or reused, checks are watched,");
  io.print("the PR is squash-merged, main is synced, and the topic branch is deleted.");
  io.print("");
  io.print("Options:");
  io.print("  --base <branch>    base branch (default: main)");
  io.print("  --body <body>      pull request body override");
  io.print("  --dry-run          print planned actions without changing git or GitHub");
  io.print("  --no-delete        keep the topic branch after merge");
}

function parseArgs(args: string[]): Options & { help: boolean } {
  const parsed = parseCliArgs(args, {
    string: ["base", "body"],
    boolean: ["dry-run", "no-delete", "help", "h"],
  });
  requireNoPositionals(parsed, "land", { allowHelp: true });
  return {
    base: stringOption(parsed, "base", "main"),
    body: stringOption(parsed, "body"),
    dryRun: booleanOption(parsed, "dry-run"),
    deleteBranch: !booleanOption(parsed, "no-delete"),
    help: helpRequested(parsed),
  };
}

const options = parseArgs([...Deno.args]);
if (options.help) {
  usage();
  Deno.exit(0);
}

await cmd.run("git", ["--version"], { stdout: "null" });
await cmd.run("gh", ["--version"], { stdout: "null" });

const branch = await currentBranch();
if (options.dryRun) {
  await ensureLandable(options.base, branch, { fetch: false });
  printPlan(options, branch);
  Deno.exit(0);
}

await cmd.run("gh", ["auth", "status"], { stdout: "piped" });
await ensureLandable(options.base, branch, { fetch: true });
await cmd.run("git", ["push", "-u", "origin", branch]);

const prUrl = await findOrCreatePr(options, branch);
io.print(prUrl);
await watchChecks(prUrl);
await mergePr(prUrl, options.deleteBranch);
await cmd.run("git", ["checkout", options.base]);
await cmd.run("git", ["pull", "--ff-only", "origin", options.base]);
if (options.deleteBranch && await gitOk(["rev-parse", "--verify", `refs/heads/${branch}`])) {
  await cmd.run("git", ["branch", "-D", branch]);
}

async function currentBranch(): Promise<string> {
  const branch = await cmd.text("git", ["branch", "--show-current"]);
  if (branch === "") {
    io.fail("land: detached HEAD is not a landable topic branch");
  }
  return branch;
}

async function ensureLandable(
  base: string,
  branch: string,
  options: { fetch: boolean },
): Promise<void> {
  if (branch === base || branch === "main" || branch === "master") {
    io.fail(`land: must run on a topic branch, not ${branch}`);
  }
  const dirty = await cmd.text("git", ["status", "--short"]);
  if (dirty.trim() !== "") {
    io.fail("land: working tree must be clean; commit or discard changes first");
  }
  if (options.fetch) {
    await cmd.run("git", ["fetch", "origin", base]);
  }
  const remoteBase = `origin/${base}`;
  if (!await gitOk(["rev-parse", "--verify", remoteBase])) {
    io.fail(`land: missing ${remoteBase}; fetch or check the base branch name`);
  }
  if (!await gitOk(["merge-base", "--is-ancestor", remoteBase, "HEAD"])) {
    io.fail(`land: current branch must contain latest ${remoteBase}; rebase onto ${base} first`);
  }
  const ahead = Number(await cmd.text("git", ["rev-list", "--count", `${remoteBase}..HEAD`]));
  if (!Number.isFinite(ahead) || ahead <= 0) {
    io.fail(`land: current branch has no commits ahead of ${remoteBase}`);
  }
}

async function gitOk(args: string[]): Promise<boolean> {
  return await cmd.status("git", args, {
    stdin: "null",
    stdout: "null",
    stderr: "null",
  }) === 0;
}

async function findOrCreatePr(options: Options, branch: string): Promise<string> {
  const existing = await cmd.text("gh", [
    "pr",
    "list",
    "--head",
    branch,
    "--base",
    options.base,
    "--state",
    "open",
    "--json",
    "url",
    "--jq",
    '.[0].url // ""',
  ]);
  if (existing !== "") {
    return existing;
  }

  const args = ["pr", "create", "--base", options.base, "--head", branch];
  if (options.body === "") {
    args.push("--fill");
  } else {
    args.push("--title", await deriveTitle(options.base), "--body", options.body);
  }
  return await cmd.text("gh", args);
}

async function deriveTitle(base: string): Promise<string> {
  const subjects = await cmd.text("git", [
    "log",
    "--reverse",
    "--format=%s",
    `origin/${base}..HEAD`,
  ]);
  const first = subjects.split(/\r?\n/).find((line) => line.trim() !== "");
  return first ?? "land branch";
}

async function watchChecks(prUrl: string): Promise<void> {
  let checksSeen = false;
  for (let attempt = 0; attempt < 12; attempt += 1) {
    checksSeen = (await runseal.text(["@tool", "github", "pr", "checks", "probe", prUrl])) ===
      "true";
    if (checksSeen) {
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
  if (!checksSeen) {
    io.print(`no checks reported on ${prUrl}; skipping watch`);
    return;
  }
  let lastCode = 0;
  for (let attempt = 0; attempt < 12; attempt += 1) {
    lastCode = await cmd.status("gh", ["pr", "checks", prUrl, "--watch", "--interval", "10"]);
    if (lastCode === 0) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
  if (lastCode !== 0) {
    io.print(`checks watch exited with ${lastCode}; continuing to merge`);
  }
}

async function mergePr(prUrl: string, deleteBranch: boolean): Promise<void> {
  const args = ["pr", "merge", prUrl, "--squash"];
  if (deleteBranch) {
    args.push("--delete-branch");
  }
  await cmd.run("gh", args);
}

function printPlan(options: Options, branch: string): void {
  const createTail = options.body === "" ? "--fill" : "--title <commit> --body <given>";
  const steps = [
    "[dry-run] would run:",
    `  git fetch origin ${options.base}`,
    `  verify ${branch} is clean, not ${options.base}, contains origin/${options.base}, ahead >= 1`,
    `  git push -u origin ${branch}`,
    `  gh pr list --head ${branch} --base ${options.base} --state open --json url --jq ...`,
    `  gh pr create --base ${options.base} --head ${branch} ${createTail}  # if missing`,
    "  gh pr checks <url> --watch --interval 10  # if checks exist",
    `  gh pr merge <url> --squash${options.deleteBranch ? " --delete-branch" : ""}`,
    `  git checkout ${options.base}`,
    `  git pull --ff-only origin ${options.base}`,
  ];
  if (options.deleteBranch) {
    steps.push(`  git branch -D ${branch}  # if still present locally`);
  }
  io.print(steps.join("\n"));
}
