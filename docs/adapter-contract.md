# Adapter contract

There is no plugin crate. A language is a thin adapter inside `core` plus data,
never a new crate. Adding one is a bounded, declared change.

## What a language provides

- A grammar written in a g4 (ANTLR-ish) subset, embedded in `grammar` as a
  compile-time resource. `grammar` owns a fully self-built dynamic lexer and
  parser that interpret this format; there is no ANTLR or JVM dependency. The
  grammar is structural only: it brackets scopes, classifies each brace as scope
  versus literal or pattern, and marks comments and words.
- A namespace-atom spec: what counts as one vocabulary atom in this language, so
  the single-word law can resolve names against the language namespace. The first
  rule splits identifiers on snake_case and camelCase boundaries; the living
  vocabulary then reconciles which multi-atom names are legitimate.

## What core provides

The constitutional checks (block depth, path depth, comments, single word) are
language-agnostic and run once, on the uniform structure tree. They are not
reimplemented per language. The adapter is only the map from native constructs to
the shared kind vocabulary plus the namespace-atom spec.

## Two layers of law

Law is two layers, like a constitution and the local/domain law under it:

- The constitution (universal): block depth, path depth, comments, single word.
  Every language obeys it; it lives in core.
- Per-language law (local/domain): a language's own hints, edge cases, and the
  alignment that makes the constitution land correctly on that language. It must
  align to the constitution, never contradict it.

Language-specific calibration is per-language law, not constitution: what counts as
a scope for the block-depth law in this language (e.g. whether `unsafe`/`async`
blocks or match arm blocks add depth), lifetime-vs-char-literal lexing, the
literal-vs-pattern distinction, and language-native checks (e.g. "no unwrap outside
tests"). These do not belong hardcoded in core. The per-language layer is a growth
point: it materializes when the first real local law is needed; until then core
carries only the universal constitution and languages contribute grammar + spec
data.

## Extension point, not an ABI

The adapter is an internal trait, resolved at compile time. It is not a stable
cross-crate ABI and carries no dynamic-loading surface. Adding `rust`, `md`, or
`ts` is the same shape of change: a grammar resource, a namespace-atom spec, and a
kind map.
