# CONTINUE — cold-resume checkpoint

_Written 2026-07-25 (session end: fact-grain campaign day two — B1f/B2 markup,
the DRIFT loop 5/5, the fact-links commission). `spec/WAL.md` is the canonical
living state and supersedes this snapshot wherever they diverge._

## TL;DR

Progress-Control campaign (PROP-043, plan
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`) is mid-**Phase B** at
**fact grain**. `spec/common` is fully re-marked (1 009 anchored facts, 0
issues); `spec/modules` stands at **14/35 files**. Five Opus-executed DRIFT
tasks landed with **zero returned round-trips**, including the owner's
**fact-links commission**: `##<ID>` fact anchors are now first-class across
all three layers — the specmap engine (core v0.8.0), the host spec compiler
(`vibe-spec`), and the contracts (PROP-014 §2.1, PROP-035 §5/§7.3). Code can
cite `spec://…#<FACT-ID>` per statement. The **coder-tier engine pin
(`claude-opus-5`) binds from the next session** — this is why the session
ended here. Floor: **all green**. Both mirrors in sync through `5d567efd`.

## State

- Branch `main`, synced to **both** mirrors (GitVerse + GitHub) via
  `cargo xtask mirror`; working tree clean (only the untracked `.zcode/`).
- Floor (`bash tools/self-check.sh`): **all green**, verified with the real
  exit code after DRIFT-005.
- Campaign journal clean (every step closed); dashboard/RESUME render the
  journal-derived **Phase B**; scope = **94 files** (~8 2xx facts).
- No blocker. The only deliberate stop: new DRIFT spawns should run on the
  engine pin, which activates at session start.

## Resume recipe (exact)

1. Boot per `CLAUDE.md` (this file's session started the same way), then:
   - `campaigns/progress-2026-08/run/RESUME.md` — recover/next per §4;
   - plan LOG §9 (`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`) —
     the **Next step** bullet is authoritative.
2. **B2 remainder** (21 files, smallest first):
   `vibe-mcp/PROP-015` (48 facts) → `vibe-workspace/PROP-034` (50) →
   `vibe-mcp/PROP-027` (53) → `vibe-cli/PROP-036` (54) → PROP-030/011/012/
   010/040/038 → PROP-008/001/009/017/043/035/037/007 → PROP-005/003/002;
   then `spec/design` / `spec/research` / `spec/terraforms` (incl. re-mark
   of the two pilot files carrying the wave's 40 expected errors).
   Per file: journal `step-start` → mark (fact grammar per the PROP-029
   pattern; verify with `cargo run -q -p vibe-cli --bin vibe -- progress
   check` + `scan`, corpus row must read 0 unmarked / 0 issues) → journal
   `step-done` (write counts AFTER the scan). Batch commits of ~2–4 files;
   `cargo xtask mirror` at checkpoints.
3. **New DRIFT tasks**: author per
   `spec/modules/vibe-progress/templates/impl-task.md`, spawn with
   `subagent_type: "opus5"` (the committed agent type;
   `.claude/settings.local.json` also pins `CLAUDE_CODE_SUBAGENT_MODEL` as
   the blanket override — remove that env line if only selective pinning is
   wanted). Review gate: read the diff, run the floor with the REAL exit
   code, close task file + INDEX + `tasks.json` + journal.
4. Candidate next DRIFT material (owner to prioritise): F-016 (modules
   README rewrite — Phase D), F-020 (OWNER-GUIDE §1 lags the fact grammar —
   Phase D/G), F-017 (aiui `scrollbar` sync-from-code), engine-family
   minting so rust/ts/go pick up the fact-aware mdspec (core v0.8.0 →
   family versions; a separate release step, NOT part of B).

## The fact-links stack (what landed 2026-07-24, in one screen)

- **Citation form** (identical for headings and facts, one id space per doc):
  `spec://vibevm/common/PROP-019#LAYER-CURRENT-FILE`,
  `spec://vibevm/common/PROP-000#INV-VOCABULARY`; carriers:
  `#[spec(implements = "…")]` / JSDoc `@spec` / Go `//spec:`.
- **Declaration ≠ citation**: `##ID` is the source-sigil only; citations are
  always `#ID`. `UPPER-SLUG` = normative fact, `kebab` = service (owner
  ruling; the register is a citation-priority signal too).
- **Contracts**: PROP-014 §2.1 (fact units; kebab law stays for headings,
  wider grammar for `##` ids) mirrored at PROP-043 §6; PROP-035 §5 (IR fact
  leaves) + §7.3 (inheritance: section fate by default; **per-fact override**
  — a source fact redeclaring a contract fact's id supersedes it; merged-view
  uniqueness gate = build error, with the precision that pure
  heading-vs-heading repeats are the `:add` artifact, not a collision).
- **Engine**: `core-ai-native-specmap` mdspec + `is_valid_fact_id`
  (v0.8.0, the OPEN line — vendored nowhere yet; families inherit at their
  next minting with zero per-language work).
- **Compiler**: `crates/vibe-spec` — `NodeKind::Fact`, `facts.rs`
  recognition, `merge.rs` override, `gate.rs` → `CompileError::DuplicateId`,
  fact-addressed `#embed`.

## Non-obvious findings from this session (the trap list)

- **`+ ` at the start of a wrapped continuation line** parses as a phantom
  list item and silently splits the unit — caught by the unmarked counter
  four times. Reflow the wrap; never let `+`/`-`/`*` open a continuation.
- **Blockquotes cannot carry `##` anchors** (scanner doesn't strip `> `;
  grammar never mentions them) — F-015 pending a ruling; workaround: re-form
  the unit (bold paragraph / fence for template content).
- **Heading and fact ids share one namespace per document** — PROP-041's
  `{#tree-widget}` section vs same-named REQ collided; suffix the fact side
  (`-req`), never rename the cited `{#}`.
- **Pre-existing owner-minted per-REQ `{#anchor}`s** (PROP-039/041): reuse
  the exact name in `##` notation; the REQ is the owner's chosen fact grain —
  do not deconstruct below it.
- **`EXIT=$?` after a `| tail` lies** (captures tail's code) — redirect to a
  file and echo `$?` in the same command (90-user.md quirk, bitten live).
- **Agent-type files and settings-env changes bind at session start**, not
  mid-session (verified empirically thrice); `claude -p --model <id>` is the
  in-session fallback for a specific engine.
- **Journal step-done counts**: write them AFTER the verifying scan (two
  entries carry pre-scan guesses; corpus.json is authoritative).
- **Campaign markers record the DOC's claims, not reality** — stale
  status-vs-shipped-code goes to the ledger (F-013/018/019/021 class),
  verdicts are Phase C's.

## Standing decisions in force (long form lives at the anchors)

Fact-exhaustive granularity supersedes paragraph grain (PROP-043 §3.9);
anchored-when-marked (§3.8); two anchor registers (§3.8 decision); scope =
94 files (generated boot pair + WAL excluded — include-only enumeration in
`progress.toml`); verdicts live in cache/baseline, never in markup (§7.5 —
also why cache pruning drops-with-warning, DRIFT-001 review ruling);
**no fractality for this campaign** (owner override in plan §0/§2: Fable =
markup/verification/review/task-authoring, Opus = DRIFT coding);
coder-tier engine = `claude-opus-5` (plan LOG 2026-07-24); campaign zone
excluded from scans/packaging; the surface is called "dashboard"; the four
CLAUDE.md rules bind every executor.

## Recent commits (this session, newest first)

```
5d567efd docs(spec): PROP-035 §7.3 heading-repeat precision; F-022 resolved
5c89839b feat(spec-compiler): fact inheritance lands end to end (DRIFT-005)
78322dac chore(harness): pin the campaign coder tier to the owner-designated engine
06b30f31 docs(spec): PROP-035 fact-inheritance ratified — R1-R4 land as contract
3eeaa53a docs(progress): ledger F-022 — the live merge is fact-blind
508bbdb9 feat(specmap): fact anchors are addressable spec units (DRIFT-004)
db6e6ca3 docs(spec): B2 batch 7 — PROP-039 marked at the owner's REQ grain
37528524 docs(spec): PROP-014 fact-anchor amendment — the contract before the code
c9007a3f fix(progress): prune cache records that leave the observed scope (DRIFT-001)
8731c850 docs(spec): B2 batch 6 — the owner guide marked, and it lags its own law
3e727fad chore(progress): drop the WAL from the observed scope (owner ruling)
05355fb4 docs(wal): DRIFT-003 through the loop, B2 at 12/35
5e75ff4d feat(progress): derive the campaign phase from the journal (DRIFT-003)
51a161c8 docs(spec): B2 batch 5 — PROP-041 marked, owner anchors become fact ids
ce645e6f docs(spec): B2 batch 4 — PROP-020/022 close the bridge-family marking
34df2f2e docs(spec): B2 batch 3 — PROP-026/021/023 marked at fact grain
17232d1c docs(wal): floor green again — DRIFT-002 through the loop
18ab7cb3 refactor(progress): split the scanner along its seams (DRIFT-002)
099ff022 docs(spec): B2 batch 2 — PROP-042/025 marked at fact grain
6f55242a docs(spec): B2 batch 1 — progress templates and the modules index marked
5f1c11f5 docs(wal): B1f boundary checkpoint — common cluster clean, B2 next
0759f59e docs(progress): queue DRIFT-002/003 — the red floor and the stale phase
d639bcf5 docs(spec): B1f batch 3 — PROP-000/019 re-marked; common cluster clean
4aed13f5 docs(spec): B1f batch 2 — PROP-024/032/018 re-marked at fact grain
83bed352 docs(spec): B1f batch 1 — six spec/common files re-marked at fact grain
b15d3d9f docs(spec): resolve the two Phase B review points by owner ruling
```

## Repository map (top level)

- `spec/` — the corpus: `common/` (12 PROPs, fact-grain DONE), `modules/`
  (35 files, 14 marked), `design/` `research/` `terraforms/` (B2+ later
  batches), `boot/` (00-core/90-user authored + generated STATIC/INDEX),
  `WAL.md` (checkpoint, out of scan scope).
- `crates/` — host workspace: `progress-core` (the fact scanner, `parse/`
  split), `vibe-spec` (the compiler — now fact-aware), `vibe-cli`
  (`commands/progress.rs` adapter), the rest of the vibe product.
- `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/` — the OPEN engine
  line (fact-aware mdspec + grammar); v0.7.0 = published, host-pinned.
- `campaigns/progress-2026-08/` — the campaign zone: `tasks/`
  (DRIFT-001…005 all done + INDEX), `run/` (journal, state/*.json, RESUME,
  cache), `deferrals.md`, `harvest/`.
- `tools/progress-dashboard/serve.mjs` — the read-only dashboard;
  `tools/self-check.sh` — the floor.
- `.claude/agents/opus5.md` — the committed coder-tier agent type.

## Quick start

```bash
bash tools/self-check.sh                                  # the floor (check REAL exit code)
cargo run -q -p vibe-cli --bin vibe -- progress check     # markup gate
cargo run -q -p vibe-cli --bin vibe -- progress scan      # refresh state
cargo run -q -p vibe-cli --bin vibe -- progress resume    # regenerate RESUME.md
node tools/progress-dashboard/serve.mjs                   # dashboard
cargo xtask mirror                                        # rollout to both mirrors
```
