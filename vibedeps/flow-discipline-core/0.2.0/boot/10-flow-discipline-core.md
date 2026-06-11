# Discipline v0.2 (BETA) — boot snippet

This project follows the AI-Native Code Discipline. Full corpus lives in
this package (`00-MANIFESTO.md`, `01-PATTERN-CARD-FORMAT.md`,
`02-EXECUTABLE-SCAFFOLDS.md`, `03-RAID-PLAYBOOK.md`, `cards/`,
`appendix/`). **Do not read it all at boot** — the Discipline's own
delivery rule is minimal sufficiency: load a card's Band-3 ops block
only when its trigger fires.

The two laws that always apply:

1. **Idiomatic inside the file, engineered around the file.** Code
   surface stays ordinary and in-distribution; all added strictness
   lives in types, contracts, metadata, and the verification loop.
2. **Explanation capital must be runnable capital.** Prose that could
   be a checker, doctest, or typed API is a WISH until it becomes one.

Card registry: `cards/INDEX.md` (trigger → card; nine executable
scaffolds A–I). Cross-cutting sweeps follow `03-RAID-PLAYBOOK.md`.
A rule with no checker is a WISH; a deviation with no reason is a
defect (`#[spec(deviates, reason)]`).
