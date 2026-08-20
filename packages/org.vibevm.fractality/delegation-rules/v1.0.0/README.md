# delegation-rules

The policy layer of the fractality delegation fabric: **what an
expensive boss agent hands to cheap workers, what it must keep, and
how it routes across the fleet's model slots.**

- [`spec/flows/delegation-rules/DECISION-MATRIX.md`](spec/flows/delegation-rules/DECISION-MATRIX.md)
  — the decidable routing calculus. One law: **delegate when
  verification is cheaper than generation.** Four axes, a five-step
  verdict procedure, the hard never-delegate set, sizing guidance, and
  the boss-as-reviewer loop.
- [`spec/flows/delegation-rules/playbooks/`](spec/flows/delegation-rules/playbooks/)
  — per-model cards: task shapes each worker model wins, budget
  defaults, tariff rules, known blind spots. `_template.md` is the
  extension surface for future backends (Codex, VibeVM Pixel).
- [`spec/boot/77-flow-delegation-rules.md`](spec/boot/77-flow-delegation-rules.md)
  — the boot snippet a consuming boss loads at session start.

Authored clean-room from the study note
`fractality/v0.1.0/spec/refs/notes/codex-first-study.md` (decisions
DC1–DC6) plus the IGNITION campaign's live delegation field data —
never from any external source text.
