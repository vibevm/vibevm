# fractality — cold-resume checkpoint

_Written 2026-07-10 ~16:45, at the close of Campaign 2. `WAL.md`
(same directory) is the canonical living state and supersedes this
snapshot wherever they diverge._

## TL;DR

**Campaign 2 (the initiative system) is CLOSED.** All seven phases
executed; the Ф6 trial fired six GLM cold-boss runs (3 per arm, zero
technical repeats) and scored them by the pre-registered rules:
**arm A (snippet only) 16.7% · arm B (+ initiative hooks) 0%**.
P1 confirmed — the cold-delegation gap is real. P3 falsified — and
the transcripts explain it mechanically (F23 the nudge channel never
fires in single-prompt `-p`; F25 a fresh home shows an empty,
unpersuasive scoreboard; F24 the staging sandbox's broken MSVC
auto-detect let every boss rationally doubt worker
self-verification). Every other prediction confirmed. Floor green:
159 tests, conform 0, specmap 19/63/63/0, xfail-strict.

## Where work stands

- Branch `main`; the close lands as two commits (trial records;
  campaign close) on top of `20ab042`, mirrors fanned out after.
- The one open human action: **owner sign-off on MT-C2-01…04**
  (`fractality/v0.1.0/spec/manual-tests/` — Recorded runs are
  filled; the narrative is
  `reports/2026-10-07-16-33-campaign2-f6-trial.md`).
- RP2/RP3 owner-open, recommendations recorded with data (plan §13).
- Trial artifacts (transcripts, runs.json, sessions, final trees):
  `fractality/v0.1.0/target/trial-results/arm-{a,b}-run-{1..3}/` —
  build state, uncommitted, will vanish on clean; the durable record
  is in the MTs and reports.

## Next-steps recipe (cold start)

1. Read `WAL.md` → the plan's §2 (execution record) → the two close
   reports in `reports/` (`…16-33-campaign2-f6-trial.md`,
   `…16-40-campaign2-close.md`).
2. If signing off: read both MTs' Recorded runs; record the pass in
   the files (owner's hand).
3. Next mandate candidates, pre-argued in plan §15:
   **DEF-C2-1** (a mid-work injection seam that exists in headless
   sessions), **DEF-C2-2** (fix the staging toolchain + surface
   worker-credibility facts), **DEF-C2-3** (cold-board reshape),
   **DEF-C2-4** (interactive-arena re-run, N>3), or Campaign 3
   (RLM, DEF-2).

## Non-obvious findings this session (do not rediscover)

- **F23:** UserPromptSubmit fires once per user prompt — in `-p`
  single-prompt sessions the threshold nudge can never fire (the
  prompt arrives at slate=0; thresholds crossed at 34/44/45 after).
- **F24:** `env -i` scratch environments break rustc's MSVC
  toolchain auto-detection — bosses hand-build vcvars wrappers and
  then distrust worker self-verification. Fix the staging fixture
  before any trial re-run.
- **F25:** a fresh FRACTALITY_HOME renders "0 runs all-time" — the
  SessionStart injection greets cold bosses with anti-proof.
- **GLM quotes the matrix back:** never-delegate vocabulary
  ("review is never-delegate", "overhead exceeds work") was used to
  justify keeping everything. Guardrails read as ammunition.
- **Specmap drift trap:** adding an anchored section to a spec doc
  (snippet 75 `#scoreboard`, last wind-down) without re-minting
  `specmap.json` fails the next floor with a misleading
  "fresh project?" error. Re-mint rides the commit that adds the
  anchor.
- **Cwd law, strike two:** a floor launched while the shell's cwd
  sat at the host root half-ran against the wrong tree. Pin cwd
  inside every gate/delegate command.
- Hook benches (MC warm, n=20): post-tool-use P95 51 ms,
  user-prompt-submit 49 ms, statusline 58 ms — all half of budget
  or better (P4/P6).

## Repository map (workspace)

`packages/org.vibevm.fractality/` — contract (CLAUDE.md), WAL.md,
this file, VIBEVM-BACKLOG.md, **reports/** (IGNITION + C2 phase
narratives; campaign2 plan-lifecycle chain
started/state/paused/resumed/completed; Ф6 trial + close);
`fractality/v0.1.0/` — the Cargo workspace: crates/{core,
mission-control, pod, mc-client, backend-claude-code, cli,
initiative}, spec/ (PROP-001 + §3b sessions, the CLOSED IGNITION and
INITIATIVE plans, manual-tests MT-01…05 + **MT-C2-01…04 with
recorded trial runs** + trial/ staging+menu+runner, examples, boot
snippet 75 v2, skills/fractality-delegate v2), vibedeps/ +
discipline configs (conform.toml, specmap.toml, `specmap.json` —
re-minted this session); `delegation-rules/v0.1.0/` — the policy
package.

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -40 WAL.md
cd fractality/v0.1.0
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
./target/debug/fractality.exe scoreboard
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
