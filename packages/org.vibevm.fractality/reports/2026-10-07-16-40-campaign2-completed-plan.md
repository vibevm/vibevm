# Campaign 2 (initiative system) — COMPLETED plan dashboard

_2026-07-10 16:40. Plan identifier: `campaign2`. Closes the chain
started/state/paused/resumed. Source of truth:
[`FRACTALITY-INITIATIVE-PLAN-v0.1.md`](../fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
(§2 record, §14 ledger, §15 deferrals)._

## Final checklist

- [x] Ф0 — spikes (P2 4/4; F20–F22; no commits)
- [x] Ф1 — sessions + attribution (`1c9757b`)
- [x] Ф2 — scoreboard engine + verbs (`6f5788a`, `6d8397e`)
- [x] Ф3 — CC adapter (`4e2c71c`)
- [x] Ф4 — nudges + routing-as-data + question push (`2b24288`)
- [x] Ф5 — answer rules (`337ea86`)
- [x] Ф6 — the trial
  - [x] pre-registration frozen & committed before any run
  - [x] arm A × 3 (16.7% pooled) · arm B × 3 (0% pooled) · 0 repeats
  - [x] scored by the frozen rules; MT-C2-01/-04 Recorded runs filled
  - [x] P4/P6 benches (51/49/58 ms P95 — all under budget)
  - [ ] owner sign-off on the MT index ← **the one open human action**
- [x] Ф7 — close
  - [x] §2 execution record; §14 Ф6/Ф7 ledger; §15 deferrals seeded
  - [x] P1–P8 ruled: 7 confirmed, P3 falsified with channel analysis
  - [x] Ф6 trial report + campaign-close report
  - [x] this completed-plan dashboard
  - [x] WAL / CONTINUE / WORKSPACES refreshed
  - [x] floor green at the boundary (specmap re-minted: snippet-75
        `#scoreboard` unit drift from the pre-pause wind-down)
  - [x] commits + mirror push

## Headline

**Arm A 16.7% · Arm B 0%.** P1 confirmed (the cold gap is real);
P3 falsified (visibility alone moved nothing in headless
single-prompt sessions). Why, mechanically: F23 (nudge channel never
fires in `-p`), F25 (fresh home = empty board = zero social proof),
F24 (broken staging linker made "workers can't self-verify" a
rational keep). Next moves live as DEF-C2-1…4 in §15.

## Risks / problems / uncertainties at close

- GLM-5.2 proxies the Opus-class boss (RP1 caveat) — absolute rates
  loose; the A↔B delta is the trustworthy shape, and it was ≤ 0.
- N=3 per arm by owner ruling — honest, small.
- RP2/RP3 remain owner-open (recommendations recorded with data).
- The persuasion layer is untested in interactive sessions — that is
  the follow-up's arena, not a defect of this campaign's record.
