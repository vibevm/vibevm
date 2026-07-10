# fractality — WAL (project continuation state)

_Updated: 2026-07-10 ~21:40 (**Campaign 2 CLOSED + owner rulings
recorded + the DEF-C2 slice landed**, one session: the Ф6 arms fired
and scored, Ф7 closed, MT-C2-01…04 owner-signed, RP2/RP3 resolved,
then the owner's direct order «…вначале доделать DEF-C2-1…4» executed
as a post-close slice. Plan:
[`fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md`](fractality/v0.1.0/spec/plans/FRACTALITY-INITIATIVE-PLAN-v0.1.md)
— §2 record, §14 ledger, §15 dispositions; reports in
[`reports/`](reports/): Ф6 trial + campaign close + defc2slice
started/report/completed)._
_Prior: 2026-07-10 (IGNITION CLOSED — MT-01…05 signed off)._

## Current state

- **Campaign 2: CLOSED, all rulings in.** Trial numbers (frozen
  scoring): arm A 3/18 ≈ 16.7%, arm B 0/18 = 0%; P1 confirmed, P3
  falsified (mechanics F23/F24/F25), P2/P4–P8 confirmed. MT-C2-01…04
  **owner-signed** («подписываю MT-C2-01…04»). **RP1–RP4 all
  RESOLVED** (RP2: counter ON, P95 51 ms; RP3: settings.local.json
  default).
- **The DEF-C2 slice (post-close, owner-ordered): landed and
  verified.** DEF-C2-2a — the trial runner passes rustup homes + the
  ProgramFiles family (repro: `cargo test --no-run` link-fail →
  link-clean). DEF-C2-3 — cold-start board (zero counters never
  render; route verb first; tests pin the text). DEF-C2-1 — the
  mid-work nudge: PostToolUse additionalContext on threshold
  crossing, shared cooldown anchor (one nudge per window per session
  across ALL channels), journal reason
  `work-tool-threshold-midwork`, `midwork_nudges` switch; D5
  rewritten in place; staged smoke green (fires at event 7, cooldown
  at 8, switches silent); P4 re-bench P95 50 ms. DEF-C2-4 —
  **MT-C2-05 pre-registered, UNFIRED, RP5-gated**: arms A′/B′,
  PR1–PR3 frozen, fatigue facts defined.
- **Floor at the checkpoint: all green** — 164 tests, conform 0
  (7/7 gated), specmap 19 units / 63 items / 63 edges / 0 orphans,
  test-gate xfail-strict. MC daemon stopped; real `~/.fractality`
  untouched (scratch homes throughout).
- Commit chain this session: `3409de1` (trial records) → `375b2d1`
  (campaign close) → `7a49159` (owner rulings) → `0b5b68c` (runner
  fix) → `356d252` (cold board) → `6d20af2` (mid-work nudge) →
  `4fae0de` (MT-C2-05 + slice record) → the wind-down commit.

## Constraints (do not violate without discussion)

- Host Rules 1–4; the delegation law + live-observation protocol +
  two context scenarios (scoreboard in every checkpoint); clean-room
  law; I1 worker-env (pins FRACTALITY_BOSS_SESSION out); I2 bus /
  files-as-persistence; I3 one telemetry store; publish
  owner-word-only. **Language law: no Python in the shipped
  codebase** (tests/prototypes OK).
- **F15 + corollary:** stop MC daemons before builds; hook smokes
  rebuild `--workspace`.
- **Cwd law binds every launch — two strikes on record** (a delegate
  launch; a floor from the host root). Pin cwd inside the command,
  every time.
- **opencode delegate law:** inputs under the launch cwd; heartbeats
  are `echo` commands.
- **Reports law:** phase reports + plan-lifecycle dashboards in
  `reports/` (дата in год-число-месяц order).
- **Specmap drift law:** a commit that adds an anchored spec section
  re-mints `specmap.json` in the same commit, or the next floor
  fails with a misleading "fresh project?" error.
- **MT-C2-05 is RP5-gated:** no paid re-run arms without the owner's
  explicit word recorded in that file.

## Delegation scoreboard (session total)

Delegated 6 / delivered 6: the six Ф6 trial boss-runs (GLM-5.2
executed the full 8-task menu six times — the experiment WAS the
delegation; ~2 h 10 m of GLM wall bought the campaign's headline
numbers). Kept with cause: trial scoring (frozen-protocol
interpretation), the P4/P6 benches and repro builds (each smaller
than a packet round-trip), the DEF-C2 slice (seam design in
hook/engine + spec/report authoring — the never-delegate set), every
review.

## Next (candidates for the owner)

1. **Rule RP5** (MT-C2-05): authorize the re-run arms — recommend
   3+3 GLM (Ф6-comparable), cap 8; an Opus-class arm is a separate
   ask. Firing + scoring is one session's work with the fixed
   toolchain.
2. Or commission DEF-C2-2b-full (acceptance-backed worker-credibility
   facts on the boss surface) — needs acceptance-schema plumbing in
   MC.
3. Or pivot to Campaign 3 (RLM, DEF-2) — §15 holds the standing
   deferrals (savings methodology, cross-harness adapters, hook
   debug channel, session TTL reaping, per-packet answer rules,
   `auto_answered` counter, quota ceilings).
