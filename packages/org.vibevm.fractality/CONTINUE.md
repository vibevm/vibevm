# fractality — cold-resume checkpoint

_Written 2026-07-10 ~21:40 at session wind-down. `WAL.md` (same
directory) is the canonical living state and supersedes this snapshot
wherever they diverge._

## TL;DR

One session did three things, in order: **(1) closed Campaign 2** —
fired all six Ф6 GLM cold-boss arms, scored them by the frozen rules
(arm A 16.7%, arm B 0% — P1 confirmed, P3 falsified with mechanics
F23/F24/F25), closed Ф7 with every prediction ruled; **(2) recorded
the owner's rulings** — MT-C2-01…04 signed, RP2 resolved ON, RP3
resolved `settings.local.json`; **(3) executed the DEF-C2 slice on
the owner's order** — the trial runner's toolchain passthrough
(repro-verified), the cold-start board (no more zero-counter
anti-proof), the mid-work nudge channel (PostToolUse
additionalContext — the seam that actually exists in single-prompt
headless sessions), and MT-C2-05, the pre-registered re-run protocol
whose paid arms wait on **RP5 (OPEN)**. Floor green at the end: 164
tests, conform 0, specmap 19/63/63/0, xfail-strict.

## Where work stands

- Branch `main`, wind-down commit on top of: `3409de1` trial records
  → `375b2d1` campaign close → `7a49159` owner rulings → `0b5b68c`
  runner fix → `356d252` cold board → `6d20af2` mid-work nudge →
  `4fae0de` MT-C2-05 + slice record. Mirrors fanned out at the
  wind-down.
- **No active blocker.** The one open decision is **RP5** in
  `fractality/v0.1.0/spec/manual-tests/MT-C2-05-initiative-rerun.md`:
  authorize the re-run arms (recommend 3+3 GLM, cap 8) or leave the
  protocol parked. Nothing fires without it.
- Trial artifacts from Ф6 still sit in
  `fractality/v0.1.0/target/trial-results/arm-{a,b}-run-{1..3}/`
  (uncommitted build state; the durable record is in the MTs and
  reports). MT-C2-05 says archive or clear them before any re-run so
  runs do not interleave.

## Next-steps recipe (cold start)

1. Boot per the workspace contract (CLAUDE.md → WAL → this file →
   the plan the WAL names).
2. If the owner rules RP5: record the ruling verbatim in MT-C2-05
   §RP5, then `cd fractality/v0.1.0 && cargo build --workspace`,
   archive `target/trial-results/`, fire
   `bash spec/manual-tests/trial/run-arm.sh a|b 1..3` sequentially
   under the live-observation law, score per MT-C2-01's rules +
   extract the fatigue facts (nudge reasons in the session journal),
   fill MT-C2-05 Recorded runs, rule PR1–PR3.
3. Otherwise: pick the next mandate from WAL §Next (DEF-C2-2b-full /
   Campaign 3 RLM / the standing §15 leftovers).

## Non-obvious findings this session (do not rediscover)

- **F23:** UserPromptSubmit never re-fires in single-prompt `-p`
  sessions — mid-turn injection must ride PostToolUse (now shipped:
  `decide_midwork_nudge`, shared cooldown anchor, reason
  `work-tool-threshold-midwork`).
- **F24 (two layers!):** under `env -i`, cargo breaks FIRST at the
  rustup shim (no toolchain under a scratch USERPROFILE) and only
  THEN at MSVC auto-detect (vswhere needs `ProgramFiles(x86)`;
  fallback is Git's GNU link.exe). `cargo build` on a lib crate
  hides the second layer — lib crates do not link; repro with
  `cargo test --no-run`. Fixed in the runner (rustup homes +
  ProgramFiles family).
- **F25:** a fresh home's board read "all-time: 0 runs" at the only
  moment the injection speaks — cold boards now lead with the route
  verb, never zero counters.
- **GLM quotes the matrix back:** never-delegate vocabulary was used
  to justify keeping everything — guardrails read as ammunition; the
  next surface iteration should encode pro-delegation
  counter-arguments as citable data.
- **Specmap drift trap:** an anchored spec edit without a re-mint
  fails the next floor with "fresh project?" — the re-mint rides the
  commit that adds the anchor (now a WAL constraint).
- **Cwd law, strike two:** a floor launched from the host root
  half-ran against the wrong tree; the specmap gate caught it. Pin
  cwd inside every gate/delegate command.
- Hook latencies (MC warm, n=20): post-tool-use P95 50 ms WITH the
  midwork metrics round-trip (was 51 without), user-prompt-submit
  49 ms, statusline 58 ms.

## Repository map (workspace)

`packages/org.vibevm.fractality/` — contract (CLAUDE.md), WAL.md,
this file, VIBEVM-BACKLOG.md, **reports/** (IGNITION + C2 narratives;
campaign2 plan chain started/state/paused/resumed/completed; Ф6
trial + close; defc2slice started/report/completed);
`fractality/v0.1.0/` — the Cargo workspace: crates/{core,
mission-control, pod, mc-client, backend-claude-code, cli,
initiative}, spec/ (PROP-001 + §3b sessions, the CLOSED IGNITION and
INITIATIVE plans, manual-tests MT-01…05 signed + MT-C2-01…04 signed +
**MT-C2-05 pre-registered/RP5-gated** + trial/ staging+menu+runner
(toolchain-fixed), examples incl. initiative.sample.toml with
`midwork_nudges`, boot snippet 75 v2, skills/fractality-delegate v2),
vibedeps/ + discipline configs (conform.toml, specmap.toml,
specmap.json); `delegation-rules/v0.1.0/` — the policy package.

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -40 WAL.md
cd fractality/v0.1.0
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
./target/debug/fractality.exe scoreboard   # cold box → "fabric ready" board
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
