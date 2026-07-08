import {
  booleanOption,
  helpRequested,
  parseArgs as parseCliArgs,
  requireNoPositionals,
  stringOption,
} from "@/lib/cli.ts";
import { cmd } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";
import { json } from "@/lib/std/json.ts";

type Options = {
  channel: string;
  ref: string;
  version: string;
  watch: boolean;
  dryRun: boolean;
};

function workflowForChannel(channel: string): string {
  switch (channel) {
    case "stable":
      return "release-stable.yml";
    case "beta":
      return "release-beta.yml";
    default:
      return io.fail(`invalid choice: ${channel}`, 2);
  }
}

function usage(): void {
  io.print("Usage: runseal :release --channel=stable|beta [options]");
  io.print("");
  io.print("Trigger one GitHub release workflow for the selected channel.");
  io.print("Use --ref for branch beta runs; the default ref is main.");
  io.print("");
  io.print("Options:");
  io.print("  --channel <name>      release channel: stable or beta");
  io.print("  --ref <ref>           git ref passed to the workflow (default: main)");
  io.print("  --version <version>   optional release version override, e.g. v0.9.0-beta.2");
  io.print("  --watch              watch the triggered workflow run");
  io.print("  --dry-run            print planned action without triggering a workflow");
}

function parseArgs(args: string[]): Options & { help: boolean; argc: number } {
  const parsed = parseCliArgs(args, {
    string: ["channel", "ref", "version"],
    boolean: ["watch", "dry-run", "help", "h"],
  });
  requireNoPositionals(parsed, "release", { allowHelp: true });
  return {
    channel: stringOption(parsed, "channel"),
    ref: stringOption(parsed, "ref", "main"),
    version: stringOption(parsed, "version"),
    watch: booleanOption(parsed, "watch"),
    dryRun: booleanOption(parsed, "dry-run"),
    help: helpRequested(parsed),
    argc: args.length,
  };
}

const options = parseArgs([...Deno.args]);
if (options.argc === 0 || options.help) {
  usage();
  Deno.exit(0);
}
if (options.channel === "") {
  io.fail("release: --channel is required");
}

const workflow = workflowForChannel(options.channel);

const dryRunCommand =
  `gh workflow run ${workflow} --ref ${options.ref} -f ref=${options.ref} -f version_override=${options.version}`;
if (options.dryRun) {
  io.print(dryRunCommand);
  Deno.exit(0);
}

await cmd.run("gh", ["--version"]);
await cmd.run("gh", ["auth", "status"]);
const refSha = await cmd.text("git", ["rev-parse", options.ref]);
const triggerOutput = await cmd.text("gh", [
  "workflow",
  "run",
  workflow,
  "--ref",
  options.ref,
  "-f",
  `ref=${options.ref}`,
  "-f",
  `version_override=${options.version}`,
]);
if (triggerOutput !== "") {
  io.print(triggerOutput);
}
io.print(`triggered ${workflow} for ref ${options.ref}`);

if (options.watch) {
  let runId = triggerOutput.match(/\/actions\/runs\/([0-9]+)/)?.[1] ?? "";
  if (runId === "") {
    let raw = "[]";
    for (let attempt = 0; attempt < 6; attempt += 1) {
      raw = await cmd.text("gh", [
        "run",
        "list",
        "--workflow",
        workflow,
        "--branch",
        options.ref,
        "--commit",
        refSha,
        "--event",
        "workflow_dispatch",
        "--limit",
        "1",
        "--json",
        "databaseId",
      ]);
      if (!json.empty(raw)) {
        runId = json.get(raw, ".[0].databaseId");
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 2000));
    }
  }
  if (runId === "") {
    io.fail(`release: could not find a recent run for ${workflow} on ${options.ref}`);
  }
  await cmd.run("gh", ["run", "watch", runId, "--interval", "10"]);
}
