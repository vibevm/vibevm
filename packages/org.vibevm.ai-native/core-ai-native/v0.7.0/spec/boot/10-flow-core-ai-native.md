# Discipline v0.2 (BETA) — boot snippet

This project follows the AI-Native Code Discipline. The language-neutral
corpus lives in this package: the guiding layer (`00-MANIFESTO.md`,
`01-PATTERN-CARD-FORMAT.md`, `02-EXECUTABLE-SCAFFOLDS.md`), the operating
playbooks (`03-RAID-PLAYBOOK.md` campaigns, `04-SWEEP-PLAYBOOK.md` the
standing sweep, `05-CAMPAIGN-FORM.md` the campaign paper trail,
`06-WAL-CONVENTION.md` session-durable state — optional but preferred), the
mechanism specs under `mechanisms/` (ENGINE-CONFORM, PROP-014 specmap,
BROWNFIELD-PROTOCOL, LEDGER-INTENT — the units `spec://org.vibevm.ai-native.core-ai-native/…`
tags cite), and `appendix/`. The concrete per-language `cards/` and the
runnable checkers ship in each language stack, not here.
**Do not read it all at boot** — the Discipline's own delivery rule is
minimal sufficiency: load a card's Band-3 ops block only when its trigger
fires; open a playbook when you run its procedure.

The two laws that always apply:

1. **Idiomatic inside the file, engineered around the file.** Code
   surface stays ordinary and in-distribution; all added strictness
   lives in types, contracts, metadata, and the verification loop.
2. **Explanation capital must be runnable capital.** Prose that could
   be a checker, doctest, or typed API is a WISH until it becomes one.

Card registry: the active language stack's `cards/INDEX.md` (trigger →
card; the nine executable scaffolds A–I in their per-language shape).
Cross-cutting sweeps follow `03-RAID-PLAYBOOK.md`.
A rule with no checker is a WISH; a deviation with no reason is a
defect (`#[spec(deviates, reason)]`).
