# rlmresearch — drafted plan (2026-07-10 22:15)

**Plan:** `fractality/v0.1.0/spec/plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md`
— Campaign 3 · Stage A: the RLM research. **NOT EXECUTING** — RP-R1
(owner review) is OPEN per the owner's order «Не исполняй план,
вначале дай посмотреть на него». Stage B (the Campaign 3
implementation plan) is this stage's exit deliverable, authored
from the synthesis.

## Checklist

- [x] Ф0 — plan drafted, dashboard landed, specmap re-minted
- [ ] **RP-R1 — owner reviews the plan (BLOCKS EVERYTHING BELOW)**
- [ ] Ф1 — Wave 1 (deep research, background) + Wave 2 (plain web
      search, independent — executed before reading W1 output)
  - [ ] raw outputs → `refs/study/rlm-waves/` (gitignored)
- [ ] Ф2 — Wave 3 merge → 5 repos / 5 papers / 5 articles
  - [ ] `spec/refs/notes/rlm-source-selection.md` (verdicts +
        runners-up + provenance + overlap metric)
  - [ ] INVENTORY rows with license verdicts (BEFORE study)
  - [ ] RP-R2 — 5/5/5 table shown to owner; swaps applied
- [ ] Ф3 — intake: clones @pins → `refs/src/`, PDFs →
      `refs/papers/`, snapshots → `refs/articles/` (new dir)
- [ ] Ф4 — study: T1 anchor (Zhang RLM: paper+repo+blog →
      `rlm-study.md`, boss-only) → T2 papers/repos → T3 articles
      (GLM surveys allowed, boss authors every note)
- [ ] Ф5 — `RLM-SYNTHESIS.md` (RD-deltas, two-way gaps, re-fetch
      list) → **draft `FRACTALITY-RLM-PLAN-v0.1.md` (Stage B)** →
      close (prediction verdicts, WAL, completed-plan dashboard)
  - [ ] RP-R3 — Stage B execution = separate owner word

## Key decisions taken while planning

- **New plan, not an edit** — no Campaign 3 plan exists (verified:
  `spec/plans/` holds two closed plans); "improve C3 with our
  ideas" = author Stage B from the synthesis (D-R1).
- Waves 1 and 2 are **independent by construction**; overlap is a
  measured prediction, not an accident (D-R2, P-R2).
- "Most important" is defined: mechanism relevance → idea density
  for the fabric → influence → recency; articles must include a
  counterpoint (D-R3).
- Clean-room pipeline is law: INVENTORY row + license verdict
  before deep study; downloads gitignored; notes are decisions,
  never code shapes; methods free, expression never (D-R4, owner
  verbatim in plan §1/§4).
- Tiered study: T1 boss-deep (the anchor), T2 medium, T3 survey;
  GLM does first-pass surveys under live observation, the boss
  authors all notes (D-R5).
- One project = one note (paper+code+blog together, D-R7).

## Risks / problems / uncertainties (mandatory section)

- Deep-research harness cost/availability — fallback: manual
  fan-out recorded as deviation.
- Link rot & paywalls — snapshot at first touch; abstract-level
  study flagged honestly.
- License surprises in top-5 repos — harmless by construction
  (inspiration-only class regardless; no code adopted from any
  source).
- Volume blowout (15 sources) — tiers absorb it; floor is "every
  source has a recorded verdict", never zero-note adoption.
- arXiv id for S4 (2512.24601) — verify at Ф3, correct INVENTORY
  if drifted.
- Glob tool unreliable on this tree (observed twice today) —
  recipes use PowerShell/git listings.
- Delegate cwd law — two strikes on record; every launch pins cwd.
