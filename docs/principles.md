# Principles

negentropy reduces semantic entropy in a codebase by pushing explanation pressure
out of prose and into structure, tests, vocabulary, and docs. These are the laws
the checker enforces on itself and on every codebase it guards.

## Single word

A name should be one vocabulary atom in the current language namespace, not one
English word. Single-word pressure points downward to the language namespace
mechanisms that let a short name resolve, and upward to a living business
vocabulary that gives the atom meaning.

## Block depth <= 3

Nesting deeper than three blocks is a signal that a unit is carrying more than one
idea. Extract until the shape is flat.

## Path depth <= 3

Directory nesting deeper than three levels is the same signal at the file-tree
scale. A path is a name; keep it a short one.

## Comments denied by default

A comment is explanation that failed to become structure. Deny comments by default
so the pressure moves into a better name, a test, a vocabulary entry, or a doc.

## Boundary laws

Path-scoped bans express boundaries. Tests live only under test paths. App
packages do not own styling syntax. Vendored infra is exempt from product laws.
A boundary is declared, not assumed, in `negentropy.toml`.

## The vocabulary never freezes

Single-word only works against a living vocabulary. Agents maintain vocabulary
deltas alongside code diffs; see `vocabulary.md`.
