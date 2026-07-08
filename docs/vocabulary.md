# Vocabulary

Single-word naming is only livable against a maintained vocabulary. The vocabulary
is the set of atoms the codebase agrees to mean something, and it never freezes.

## Deltas, not just diffs

An agent that changes code also maintains the vocabulary. A new atom, a retired
atom, or a shifted meaning is a vocabulary delta, recorded alongside the code diff
so the shared meaning stays true.

## Two directions

- Downward: an atom resolves through a language namespace mechanism, so a short
  name is unambiguous in context.
- Upward: an atom carries a business meaning, so the short name is not arbitrary.

A name that resolves downward but means nothing upward is entropy wearing a short
spelling; the vocabulary is what keeps the two directions in agreement.
