# Agents

This repository is guarded by its own laws. An agent maintaining it holds to the
same constitution negentropy enforces on others.

## Laws in practice

- Single word: each name is one vocabulary atom. Maintain vocabulary deltas
  alongside code diffs (`docs/vocabulary.md`).
- Block depth <= 3 and path depth <= 3. Flatten before nesting deeper.
- No comments by default. Move explanation into a name, a test, the vocabulary,
  or a doc. Boundary exemptions live in `negentropy.toml`.

## Architecture

Three crates, no plugin crate: `grammar` (substrate + embedded g4), `core`
(structure tree + checks + thin adapters), `cli`. Dependency direction is
`cli -> core -> grammar`. The structure tree in `docs/structure-json.md` is the
one contract; do not reach around it. Add a language via `docs/adapter-contract.md`
(a grammar resource + a namespace-atom spec), never a new crate.

## Operating

- Never commit on `main`; the pre-commit hook refuses it. Branch, then commit.
- `runseal :guard` must pass before landing. `runseal :land` squash-merges the
  topic branch on GitHub.
- Repo-local operator flows are TypeScript under `.runseal/wrappers`. Do not add
  Python or uv for operator flows.
