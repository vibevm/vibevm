# rlmplan — started plan (2026-07-11 16:56)

**Plan:** `fractality/v0.1.0/spec/plans/FRACTALITY-RLM-PLAN-v0.1.md`
— Campaign 3 · Stage B: the descent core. **RP-C3-1 RULED →
Option B** (owner verbatim: «Вариант 1. Вариант плана - B
(нисхождение + эскалация). Вариант C с адвайзором - отдельная
задача, запланируй.»). Launched from the revised draft — scope cut
to descent + ascent; the advisor (Option C / D-C3-7) is deferred to
[PP-003](../plans/postponed/PP-003-option-c-advisor-slice.md).
Posture: the plan's §10 executor guide is BINDING; one D-C3
decision = one commit-sized slice, floor green at every boundary;
Ф0 spikes carry no commits; paid trial arms (Ф6) stay gated behind
RP-C3-2.

## Checklist

- [x] Draft revised with Fugu findings (`d0cf6e9`) + §10 executor
      guide (`4ccb7af`); specmap green
- [x] RP-C3-1 — owner ruled Option B; recorded in plan §1, §8
- [x] Option C (advisor) postponed → PP-003 filed
- [ ] Ф0 spikes (no commits) — s1 schema-validate-at-seam · s2
      FileRef slice handoff · s3 settings-injection promotion (CC,
      RD-12) · s4 escalated-outcome round-trip; each green or its
      Decision rewritten in place
- [ ] Ф1 packets & budgets (D-C3-2, D-C3-3) → floor green
- [ ] Ф2 need-gate + delegation-rules columns (D-C3-1, D-C3-10) →
      policy-table goldens
- [ ] Ф3 descent verbs (D-C3-4, D-C3-5) → await any/all + merge
      node + refuse-duplicate
- [ ] Ф4 escalation channel (D-C3-6; Option B)
- [ ] Ф5 acceptance / PP-002 fold-in (RD-11, FD-9) — verifier-
      accept gating + cold-verifier refusal
- [ ] Ф6 trial (D-C3-9): pre-register MT-C3-01 → RP-C3-2 → fire →
      score → orchestration-collapse probe
- [ ] Ф7 close: verdicts, deferrals ledger, reports, WAL

Detailed living state moves to a `-state-plan.md` companion when Ф0
opens (per the big-plan dashboard rule — bulk stays out of status
files).

## Key decisions taken at commissioning

- **Option B, not C** — ascent ships (the Silo theorem makes it
  part of descent's correctness, RD-6); the advisor plane (V4) is a
  clean separate task (PP-003), not a half-built stub.
- **MT-C3-01 fires first** — RP5 (MT-C2-05 re-run) is not yet
  ruled, so this campaign's trial is MT-C3-01 alone; the
  cross-trial firing order returns at RP-C3-2 if RP5 also fires.
- **No new build scope beyond the descent core** — §10.5
  minimalism holds: no learned router, no Python, no NL workflow
  grammar, no daemon beyond MC, no string sentinels, no crate
  rewrites; every D-C3 lands at a named seam (§10.6).

## Risks / problems / uncertainties

- **Ф0 is the load-bearing gate** — the four spikes probe the
  riskiest seams (schema-validate-at-seam, FileRef slice handoff,
  settings-injection promotion, escalated-outcome round-trip). A
  spike that fails rewrites its Decision in place BEFORE Ф1 code —
  never build on an unproven seam.
- **Clean-room law is legally load-bearing (§10.4)** — during any
  implementation slice, never open `refs/src|papers|articles`. If a
  note under-specifies, STOP, write the question into plan §9, ask
  the owner. Re-reading a source is a separate STUDY act.
- **Depth-cap discipline (RD-2)** — default `max_depth = 1`; depth 2
  only behind the experimental flag for provably super-linear
  tasks. Wrapping natively-capable models makes them worse — the
  need-gate's `route` verdict is the default, not `spawn`.
- **delegation-rules is its OWN Cargo workspace / version dir**
  (§10.6) — policy tables do NOT land inside `fractality/v0.1.0`.
  Wiring it (requires redbook + rust-ai-native, `vibe install`) is
  part of the first slice that needs it (Ф2).
- **Floor / cwd / delegate laws** — floor from `fractality/v0.1.0/`;
  long runs backgrounded with a first-output ≤3 min watchdog;
  delegate mechanical work to GLM, boss reviews every diff; never
  touch the real `~/.fractality`; specmap re-mint in the same commit
  as any anchored-section change.
- **Trial arms are paid** — Ф6 fires only after MT-C3-01
  pre-registration is committed AND RP-C3-2 is ruled verbatim.

## Source of truth

The spec tree wins on any divergence: the plan
`FRACTALITY-RLM-PLAN-v0.1.md` (decisions, §10 executor guide), the
two syntheses (`RD-n` / `FD-n`), `WAL.md` (living state). This
dashboard is the owner-facing snapshot.
