# fractality — cold-resume checkpoint

_Written 2026-07-12 ~07:00 at session save. `WAL.md` (same directory) is
the canonical living state and supersedes this snapshot wherever they
diverge. Resume with `восстанови сессию fractality` (report-then-wait)._

## TL;DR

**Campaign 3 Stage B is COMPLETE, and the PP-003 advisor core landed too.**
This session drove the whole back half of the plan end to end: Ф4
(escalation) → Ф5 (acceptance) → Ф6 (the paid trial) → Ф7 (close Stage B) →
PP-003 (the advisor core). ~22 commits, all on `main`, pushed to both
remotes. Floor green at every boundary (test-gate 215). Two landmark
results in Ф6:

1. **fractality ran end to end as a product for the first time** — real GLM
   workers spawned under real pods, did work, wrote results, ran
   acceptance, folded into the journal.
2. **The RLM gated arm delegated 44.4% vs the 16.7% naive baseline (~2.7×).**

**No active blocker.** The whole owner goal (finish the Stage B plan, then
take PP-003) is done. What remains is future scope the owner commissions: a
validated Stage C (the advisor help/hurt trial) + the PP-004 trial
follow-ups.

## Where work stands

- Branch `main`, **in sync with BOTH remotes** at `531d83b` (GitVerse
  `origin` + GitHub `github`), working tree clean.
- Floor green: test-gate 215, conform 0, specmap clean, clippy/fmt clean.
- Real `~/.fractality` untouched by tests; the Ф6 trial read
  `~/.fractality/profiles.toml` as a template only (scratch homes for runs).

## The BIG process change this session — delegation mechanism switched

**opencode → CC+z.ai.** opencode/GLM stalled again (booted, no tool output,
killed ~3 min). The fix (owner-prompted): launch GLM the way fractality
itself does — headless Claude Code at the z.ai gateway. **Verified working
recipe** (from this workspace's `backend-claude-code/envbuild.rs` +
`spec/examples/profiles.sample.toml`):

```sh
env -u ANTHROPIC_API_KEY \
    ANTHROPIC_BASE_URL="https://api.z.ai/api/anthropic" \
    ANTHROPIC_AUTH_TOKEN="$(cat ~/.vibevm/zai.api.token)" \
    API_TIMEOUT_MS=3000000 CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 \
    claude -p '<task>' --model glm-5.2[1m] --dangerously-skip-permissions \
      --output-format stream-json --verbose
```

Never echo the token (`$(cat …)` pipes into the env var, value never hits
stdout — secrets law). Watch heuristics (owner): silent >5 min after the
first line ⇒ hung, kill; actively producing ⇒ wait for exit up to 30 min.
This IS the mechanism the trial arms run on. Use it for mechanical
carves / bulk edits / run-and-report from here on.

## Next-steps recipe (cold start)

The Stage B plan is done. The candidate next work (owner commissions):

1. **A validated Stage C — the advisor help/hurt trial.** Read
   `fractality/v0.1.0/spec/plans/FRACTALITY-ADVISOR-PLAN-v0.1.md` §3
   (deferred). Needs its OWN pre-registration (MT-C3-02-shaped) + a menu
   with genuinely uncertain tasks + a paired-arm design (caller-with-advisor
   vs caller-alone). The RD-10 inversion is the falsifier: advice must HELP
   a medium caller and NOT hurt a weak one. Also deferred: the uncertainty
   trigger (measured thresholds), the ladder-as-routing-data, a
   `fractality advise` verb.
2. **PP-004 trial follow-ups** (`plans/postponed/PP-004-…`): raise worker
   turn caps (30 bit hard), add a schema task (test P-C3-b) + a Silo task
   (test P-C3-d) to the trial menu, add a `fractality decisions` read verb
   (mirror `escalations`) so P-C3-a becomes a hard number.
3. **To re-run the Ф6 trial:** `cd fractality/v0.1.0 && cargo build
   --workspace && bash spec/manual-tests/trial/run-arm.sh g <n>` (arm g =
   gated; results in `target/trial-results/arm-g-run-<n>/`; score with
   `python spec/manual-tests/trial/score-g.py`).

Resume is report-then-wait — the owner steers; the above are candidates.

## Non-obvious findings this session (do not rediscover)

- **specmap indexes `fractality/v0.1.0/spec/**` anchors** — writing a spec
  doc (a plan under `spec/plans/`) adds a spec unit and DRIFTS specmap.
  Re-mint AFTER writing spec docs, and never write one mid-floor (it fails
  `specmap --check`). Reports under `packages/…/reports/` are OUTSIDE the
  spec tree — they do NOT drift specmap.
- **The Ф2 team pre-built the advisor bar** — `ClassPolicy.advisor_enabled`
  already existed "ready for PP-003" (Weak:false, Medium/Strong:true), and
  `CapabilityClass` derives `Ord`, so `caller_class >= Medium` just works.
  PP-003 only had to add the `advice` marker + enforce the bar.
- **The Ф6 trial harness** (`run-arm.sh`) reads the REAL
  `~/.fractality/profiles.toml` (a template, copied to a scratch home) and
  uses `python` (tomllib) to parse it. The boss runs via `claude --print`
  with the z.ai env under `env -i` + a Rust-toolchain passthrough
  (DEF-C2-2a) so scratch `env -i` does not break cargo.
- **Worker/boss turn caps dominated trial completion** — boss 50, worker 30;
  bosses timed out mid-menu, workers mid-task. `delegated` (8) is honest;
  `delegated-and-collected` (3) trails it purely because of the caps.
- **The 600-line conform cell budget bit twice more** this session — the
  journal fold carved to `journal_fold.rs`, and the whole MC pod leg carved
  to `http_pods.rs` + `pod_leg.rs` (headroom for the escalate endpoint/verb).
- **Adding a field to `OutputSpec` or `RunRecord`** updates the `hello_glm`
  Debug snapshot (`fractality_core/src/snapshots/…hello_glm…snap`) and needs
  every RunRecord literal site touched (journal_fold `fixed_run`, metrics
  `run`, http `register_run`, `admission_primitives` `record`).

## Repository map (workspace)

`packages/org.vibevm.fractality/` — `CLAUDE.md` (contract), `WAL.md`
(canonical state), this file, `WORKSPACES.md` row in the host,
`VIBEVM-BACKLOG.md`; **`plans/`** (postponed.md + PP-001/002/003/004);
**`reports/`** (per-phase reports incl. `…-f4-escalation`, `…-f5-acceptance`,
`…-f6-trial`, `…-campaign3-close`, the state-plan tracker). `fractality/
v0.1.0/` — the Cargo workspace: `crates/{core, mission-control, pod,
mc-client, backend-claude-code, cli, initiative}`; `spec/` (PROP-001,
VISION, plans/**RLM-PLAN v0.1** + **ADVISOR-PLAN v0.1**, manual-tests/
**MT-C3-01** + trial harness, refs/ notes); `delegation-rules/v0.1.0/`
(routing policy). New cells this session: core `journal_fold.rs`; mc
`http_pods.rs` / `http_escalate.rs`; mc-client `pod_leg.rs`; tests
`escalate.rs` / `verifier.rs` / `advisor.rs`.

## Decisions / policy in force (long form)

- Host Rules 1–4; **plan §10 executor guide is BINDING**; clean-room §10.4
  (never open `refs/src|papers|articles` while coding); commit via
  `git commit -F - <<'MSG'` heredoc; editor-tool edits only (PS 5.1
  corrupts UTF-8-no-BOM); no Python in shipped code (test/prototype only —
  the trial runner + scorer are legitimate); scratch homes; no `*install*`
  test binaries; F15 (stop MC before builds); domain code has no
  `unwrap`/`expect` (conform); 600-line conform cell budget (carve before
  adding); specmap re-mint in-commit on ANY scoped-file change.
- **70%-context stop rule LIFTED** (owner 2026-07-12): run straight through
  with compaction.
- **RP-C3-2 paid trial arms PRE-AUTHORIZED** — fired at Ф6 (MT-C3-01, 3
  runs). A future advisor trial needs its own pre-registration + word.
- Floor = the gate panel run FROM `fractality/v0.1.0/`: fmt → test → clippy
  → conform → specmap → test-gate; green at every phase boundary.

## Recent commit chain (last ~22, newest first)

```
531d83b docs(fractality): WAL + tracker — PP-003 advisor core landed
9bea86d docs(fractality): PP-003 advisor plan + registry status
40687ca feat(fractality): PP-003 advisor core — the caller-class bar (D-C3-7)
7c6c232 docs(fractality): Ф7 close — Stage B COMPLETE
7020e68 docs(fractality): Ф6 close — trial phase report, ledger, WAL
67a3e4a test(fractality): Ф6 MT-C3-01 recorded runs — the gated trial fired
1c4a8f8 test(fractality): Ф6 trial harness — arm g (gated) runner + preamble
3c8ea76 docs(fractality): Ф6 pre-register MT-C3-01 (the RLM gated trial)
2fe365c docs(fractality): Ф5 close — phase report, ledger, WAL
af977a4 feat(fractality): Ф5.2 verifier-accept surfaced (FD-9)
85ac2a7 feat(fractality): Ф5.1 verifier marker + cold-verifier suppression
7850be7 docs(fractality): Ф4 close — phase report, ledger, WAL
0bf4242 feat(fractality): Ф4.3b escalate MCP tool in the broker (D-C3-6)
3f9a2e4 feat(fractality): Ф4.3a escalate endpoint + client verb (D-C3-6)
2e10aa9 docs(fractality): record the opencode→CC+z.ai delegation switch
2ce35f8 refactor(fractality): carve the MC pod leg into its own cells
e355557 docs(fractality): Ф4.2 ledger + operating-rule update
6ed04e6 feat(fractality): Ф4.2 escalation climbs to the top (D-C3-6)
ee2fc46 docs(fractality): Ф4.1 ledger — escalation core outcome
e13ddbf feat(fractality): Ф4.1 escalation core outcome (D-C3-6)
```

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -30 WAL.md
cd fractality/v0.1.0
# floor (ALWAYS from v0.1.0):
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
# delegate to GLM (the CC+z.ai recipe above) for mechanical/bulk/run-and-report.
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
Wind-down: `заверши сессию fractality`.
