# Campaign 2 (initiative system) — RESUMED plan dashboard

_2026-07-10 14:19 (filename in the owner's reverse date order). Plan
identifier: `campaign2`. Prior stage files: `…-11-53-…-started-plan.md`,
`…-13-45-…-state-plan.md`, `…-14-05-…-campaign2-paused-plan.md`.
Source of truth:
[`FRACTALITY-INITIATIVE-PLAN-v0.1.md`](../fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
(§14 ledger); this file is the owner-facing dashboard only._

**Resume trigger:** owner goal, verbatim: «campaign 2 должен быть
завершен». RP1 stands resolved (GLM-served cold boss, 3 runs per arm,
technical cap 8). The session resumed exactly at the paused item: fire
the Ф6 arms.

## Checklist

- [x] Ф0 — spikes (P2 4/4, F20–F22; no commits)
- [x] Ф1 — sessions + attribution (`1c9757b`)
- [x] Ф2 — scoreboard engine + verbs (`6f5788a`, `6d8397e`)
- [x] Ф3 — CC adapter: hooks/statusline/harness/fetch (`4e2c71c`)
- [x] Ф4 — nudges + routing-as-data + question push (`2b24288`)
- [x] Ф5 — answer rules (`337ea86`)
- [ ] **Ф6 — the trial** ← RESUMED HERE
  - [x] staging crate + neutral menu + runner committed
  - [x] MT-C2-01/-04 scoring FROZEN before any run
  - [x] boss surface v2 (snippet 75 + skill)
  - [x] workspace rebuilt (`cargo build --workspace`, exit 0)
  - [ ] arm A runs 1–3 (run 1 **firing now**, 14:18)
  - [ ] arm B runs 1–3
  - [ ] score per frozen rules; fill both MTs' Recorded runs
  - [ ] P4 hook-latency bench (if convenient)
- [ ] Ф7 — close
  - [ ] §2 execution record + §14 Ф6/Ф7 ledger entries
  - [ ] P1–P8 verdicts (incl. P8 shadow-state grep audit)
  - [ ] Ф6 phase report + campaign-close report
  - [ ] completed-plan dashboard
  - [ ] WAL / CONTINUE / WORKSPACES row refresh
  - [ ] deferrals ledger (§15): hook debug channel, session TTL
        reaping, per-packet answer rules, auto-answered counter,
        quota plan limits
  - [ ] commits per phase + mirror push

## Key decisions in force at resume

- RP1 (owner verbatim): GLM boss proxies the Opus-class boss —
  **the A↔B delta is the headline number**; absolute rates are
  secondary. 3+3 runs, cap 8 including technical repeats.
- Scoring is frozen (MT-C2-01): attempted = transcript addresses the
  task; delegated = an MC run's packet maps to it; metric =
  delegated ÷ attempted over E={1..6}, pooled per arm; distractor
  delegations (7, 8) reported separately.
- Runs are sequential per the frozen protocol (no parallel arms).

## Risks, problems, uncertainties (mandatory section)

- **The runner has never fired live before today** — first-run env
  friction expected (env -i strips nearly everything; claude is a
  node shim). The cap of 8 exists for exactly these technical repeats.
- **GLM turn latency**: up to 25 min wall per run (timeout 1500 s),
  ~50 turns max; six runs could cost 1–2.5 h wall. Watcher polls
  telemetry every ~30 s (live-observation law).
- **F20**: statusline does not render in `-p` — arm B measures
  SessionStart + UserPromptSubmit + Stop channels only
  (pre-registered caveat in MT-C2-04).
- **Validity**: N=3 per arm is small by design (owner: «не супер
  большое но достаточное»); numbers reported honestly as such.
- **Owner sign-off on the MT index is the one step the agent cannot
  perform** — the close will record runs + pre-run verdicts and mark
  sign-off pending.
