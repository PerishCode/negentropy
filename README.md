# negentropy

A self-contained structural checker for reducing semantic entropy in codebases,
especially agent-maintained ones.

negentropy owns a grammar-first parser substrate and delivers a uniform structure
tree to language-agnostic checks. It does not compile target languages or depend
on their toolchains for semantic validation. It is the higher-order replacement
for `flavor`.

## Shape

- `crates/grammar` — parser substrate; `g4` grammars embedded as compile-time
  resources; produces a concrete tree.
- `crates/core` — lifts the concrete tree into the structure tree, runs the
  constitutional checks, hosts thin per-language adapters.
- `crates/cli` — the `negentropy` binary.

## Laws

See `docs/principles.md`. In short: single word, block depth <= 3, path depth
<= 3, comments denied by default, boundary laws are declared in `negentropy.toml`,
and the vocabulary never freezes.

## Operating

Operator flows run through `runseal`:

- `runseal :init` — validate the repo and install versioned git hooks.
- `runseal :guard` — format, lint, test, check wrappers, and self-check.
- `runseal :land` — land the current topic branch on GitHub.
