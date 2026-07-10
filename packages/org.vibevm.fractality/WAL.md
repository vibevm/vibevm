# fractality — WAL (project continuation state)

_Updated: 2026-07-10 ~16:45 (**Campaign 2 CLOSED** — resumed under the
owner goal «campaign 2 должен быть завершен», fired the Ф6 arms,
scored, closed Ф7. Plan:
[`fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
— §2 execution record, §14 ledger, §15 deferrals; reports in
[`reports/`](reports/): Ф6 trial narrative + campaign-close +
completed-plan dashboard)._
_Prior: 2026-07-10 (IGNITION CLOSED — MT-01…05 signed off)._

## Current state

- **Campaign 2 executed Ф0–Ф7, floor green at every boundary**
  (final: conform 0 — 7/7 gated, specmap 19 units / 63 items / 63
  edges / 0 orphans / 0 warnings, test-gate xfail-strict, 159
  tests). Surface: boss sessions + attribution, the pure initiative
  engine, 7 new verbs, the CC adapter (availability law
  field-proven), nudges + question push, answer rules,
  matrix-as-data.
- **The trial (Ф6): six GLM cold-boss runs, 0 technical repeats.**
  Frozen scoring: **arm A 3/18 ≈ 16.7% · arm B 0/18 = 0%**;
  distractor delegations 0. **P1 CONFIRMED, P3 FALSIFIED**; P2/P4/
  P5/P6/P7/P8 confirmed (P4: 51/49 ms P95; P6: 58 ms; P7: 19
  commits; P8: audit clean). Falsifier mechanics, recorded as
  F23–F25: the nudge channel never fires in single-prompt `-p`; a
  fresh home renders an empty (unpersuasive) board; the staging
  `env -i` broke MSVC auto-detect and handed every boss a rational
  "workers can't self-verify" keep-reason (both arms equally — the
  delta survives).
- **Open items:** owner sign-off on MT-C2-01…04 (pre-run records
  filled; the pass is the owner's). RP2/RP3 owner-open with
  data-backed recommendations (counter ON; settings.local target).
  Next-campaign seeds: **DEF-C2-1…4** in plan §15 (mid-work
  injection seam for headless; worker-credibility facts + staging
  toolchain fix; cold-board reshape; re-run design).
- MC daemon stopped; real `~/.fractality` untouched (scratch homes
  throughout). Trial artifacts in `target/trial-results/`
  (uncommitted build state by design).

## Constraints (do not violate without discussion)

- Host Rules 1–4; the delegation law + live-observation protocol +
  two context scenarios (scoreboard in every checkpoint); clean-room
  law; I1 worker-env (pins FRACTALITY_BOSS_SESSION out); I2 bus /
  files-as-persistence; I3 one telemetry store; publish
  owner-word-only. **Language law: no Python in the shipped
  codebase** (tests/prototypes OK).
- **F15 + corollary:** stop MC daemons before builds; hook smokes
  rebuild `--workspace`.
- **Cwd law binds every launch — now with TWO strikes** (a delegate
  launch in the first session; a floor launch from the host root in
  the close session, caught by specmap). Pin cwd inside the command,
  every time; candidate future fix: a mechanical guard.
- **opencode delegate law:** inputs under the launch cwd; heartbeats
  are `echo` commands.
- **Reports law:** phase reports + plan-lifecycle dashboards in
  `reports/` (дата in год-число-месяц order).
- **Specmap drift note:** editing anchored spec docs (e.g. snippet
  75) requires re-minting `specmap.json` before the next floor —
  the wind-down that adds an anchor must carry the re-mint.

## Delegation scoreboard (this session)

Delegated 6 / delivered 6: the six trial boss-runs (GLM-5.2 executed
the full 8-task menu six times — the experiment itself was the
delegation). Kept with cause: scoring (frozen-protocol
interpretation), P4/P6 benches (20-iteration one-shots, smaller than
a packet round-trip), close authoring (plan/reports/WAL — the
never-delegate set), every review.

## Next (candidates for the owner)

1. **Sign off MT-C2-01…04** (read `Recorded runs` in both files;
   the Ф6 trial report is the narrative companion).
2. Rule RP2 (recommend: counter default-ON, P95 51 ms) and RP3
   (recommend: keep `.claude/settings.local.json` default).
3. Commission the follow-up from §15 DEF-C2-1…4 (headless-capable
   injection seam; staging toolchain fix + credibility facts;
   cold-board reshape; interactive-arena re-run).
4. Or: pivot to Campaign 3 (RLM, DEF-2) — the deferrals ledger holds.
