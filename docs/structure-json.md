# Structure JSON

`grammar` parses a source into a concrete tree; `core` lifts that into a uniform
structure tree that every rule and every language adapter reads. The structure
tree is the one contract between the parser substrate and the checks. Rules never
touch lexer or parser internals.

## Node

A node is language-agnostic:

- `kind` — the structural role, drawn from a shared kind vocabulary (`root`,
  `block`, `word`, `path`, ...), not a language-specific token name.
- `depth` — block depth from the root, the value the block law reads.
- `span` — byte range in the source, for reporting.
- `kids` — child nodes.

## Kind vocabulary

The kind set is small and shared across languages. A language adapter maps its
native constructs onto these kinds; it does not invent per-language kinds. Adding
a kind is a contract change, reviewed against `principles.md`.

- `root` — the whole source.
- `scope` — a real nesting scope (a function body, a control-flow block). Only
  `scope` counts toward the block-depth law.
- `item` — a named declaration (a function, a type, a module).
- `literal` — a value form, including struct literals. A struct literal brace is
  a `literal`, not a `scope`, so it does not inflate block depth.
- `pattern` — a destructuring form, including struct patterns in match arms. Like
  `literal`, its braces are not scopes. RESERVED: not yet emitted — struct patterns
  currently carry `literal`, since telling construct from destructure needs match-
  arm context and no law consumes the distinction yet. It is per-language law (see
  adapter-contract.md), added when a local law needs it.
- `comment` — explanation text; denied by default.
- `word` — a name, split into vocabulary atoms for the single-word law.

## Stability

The structure tree is the expensive-to-change surface. Rules, adapters, and the
cli all depend on it, so its shape is settled before breadth of grammar coverage
grows. See `adapter-contract.md` for how a language plugs in.
