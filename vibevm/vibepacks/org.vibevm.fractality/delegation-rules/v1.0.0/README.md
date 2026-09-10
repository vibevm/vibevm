# delegation-rules

The policy layer of the fractality delegation fabric: **what an
expensive boss agent hands to cheap workers, what it must keep, and
how it routes across the fleet's model slots.**

- [`vibevm/vibespecs/flows/delegation-rules/DECISION-MATRIX.xml`](vibevm/vibespecs/flows/delegation-rules/DECISION-MATRIX.xml)
  — the decidable routing calculus. One law: **delegate when
  verification is cheaper than generation.** Four axes, a five-step
  verdict procedure, the hard never-delegate set, sizing guidance, and
  the boss-as-reviewer loop.
- [`vibevm/vibespecs/flows/delegation-rules/playbooks/`](vibevm/vibespecs/flows/delegation-rules/playbooks/)
  — per-model cards: task shapes each worker model wins, budget
  defaults, tariff rules, known blind spots. `_template.xml` is the
  extension surface for future backends (Codex, VibeVM Pixel).
- [`vibevm/vibespecs/boot/77-flow-delegation-rules.xml`](vibevm/vibespecs/boot/77-flow-delegation-rules.xml)
  — the boot snippet a consuming boss loads at session start.

Authored clean-room from the study note
[`codex-first-study.xml`](https://github.com/vibevm/vibevm/blob/main/vibevm/vibepacks/org.vibevm.fractality/fractality/v1.0.0/vibevm/vibespecs/refs/notes/codex-first-study.xml) (decisions
DC1–DC6) plus the IGNITION campaign's live delegation field data —
never from any external source text.
