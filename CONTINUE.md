# CONTINUE — cold-resume checkpoint

_Written 2026-07-25 (session end: **Phase C COMPLETE** — the modules
cluster verified in ten batches c4a…c4f3; exit gate green; the corpus
measured). `spec/WAL.md` is the canonical living state and supersedes
this snapshot wherever they diverge._

## TL;DR

Progress-Control campaign (PROP-043, plan
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`): **Phase C
(verification) is COMPLETE and its exit gate is green** — 58/58 files
carry campaign maps, **4 944/4 944 markers carry verdicts**. **The
first measured actuality level of the spec tree: 4 455 units judged =
4 141 confirmed / 311 drift / 3 unverifiable (93.0 % / 7.0 % /
0.07 %). Findings: 55** (F-001…F-055; this session minted F-045…F-055,
extended F-018/F-024/F-047/F-048). The §5-C prediction confirmed
**mirrored**: drift concentrates in Status lines that promised *less*
than the tree delivers (shipped-under-proposed families), while
honestly-updated headers rode to **eight zero-drift files**
(PROP-011/012/025/042/015/026/027/005). GitHub synced through
`242085d4`; GitVerse SSH still down (strict ancestor; plain re-fan on
recovery, never `--force`). **The campaign holds at the C→D boundary:
opening Phase D (stitching) is the owner's call** (plan §5 entry law);
the phase lane in the journal stays `C` until that ruling lands as a
`phase` event.

## The blocker (owner decision)

**Open Phase D or not — nothing else is blocked.** Phase D's entry law
(plan §5): *"Entry: C verdicts exist for the cluster"* — satisfied
corpus-wide. The plan requires the owner's explicit call before the
next phase opens (the resume-boundary law). Secondary question for the
owner: did you create `~/.vibe/registry.toml` today at 15:37 (vibespecs
+ vibespecs-gitverse)? It is what turned the floor red post-gate
(F-055, see Findings below) — if it wasn't you, that's worth a look.

## How to start the next session (exact)

**Option A — the owner opens Phase D** (paste as the first message):

> Продолжаем спец-актуализационную кампанию (PROP-043). Прочитай
> CONTINUE.md, затем spec/WAL.md, план §5 Phase D и хвост LOG §9.
> **Открываю Phase D (stitching)** — запиши phase-событие `D` в журнал
> и работай волнами по §5-D: family-свипы по ледеру (55 строк —
> `run/state/findings.json`), `reality-mismatch` — строго через
> sync-from-code (спек-диффы мне на аппрув ДО применения), пары без
> сходимости две волны — эскалация мне. Fable без fractality; opus5 —
> только если расчёт delegation-rules это оправдает.

**Option B — restore context first, decide after** (report-then-wait):

> ВОССТАНОВИ СЕССИЮ

Then the executor (Option A) does, in order:

1. Boot per `CLAUDE.md`; read this file, `spec/WAL.md`, plan §5
   Phase D + the LOG §9 tail (the ten c4-entries + the exit-gate
   boundary entry carry all conventions).
2. Append the phase event to the journal:
   `{"kind":"phase","value":"D","ts":"<UTC>"}` (append-only, LF
   newline; then `vibe progress scan` so `campaign.json`/RESUME
   re-derive the lane).
3. Journal `step-start` for wave 1 (`d1-<slug>`), work SPEC-task
   sweeps per §5-D, commit per topic, `cargo xtask mirror` at
   checkpoints (github ok; gitverse fails while SSH is down —
   expected).
4. Phase D mechanics (plan §5): obligation types `contradiction ·
   duplication · missing-support · terminology · relocation ·
   reality-mismatch`; waves until the ledger is empty or a pair
   stalls two waves (→ owner escalation); `reality-mismatch` resolves
   via the sync-from-code flow — **owner approves every spec diff
   before it is applied**; templates in
   `spec/modules/vibe-progress/templates/spec-task.md`.

## The Phase D work-map (what the ledger looks like)

~15 family rows cover ~80 % of the 311 drift verdicts. The big sweeps,
each a one-file (or one-cluster) re-mark + header fix:

| Family | Scope | Shape |
|---|---|---|
| F-018 | PROP-020/021/022/023 (bridge four) — 137 rows | proposed→IMPLEMENTED headers + spec/done→impl/done re-mark |
| F-053 | PROP-030 — 63 rows | same shape (embedded registry ships wholesale) |
| F-052 | PROP-036/037/039 headers + 036 §2.11 keymap tail + 039 AIUI rows | DRAFT→shipped + supersede-mark §2.11 |
| F-043 | PROP-000 — six aged spots | one foundation refresh |
| F-046 | PROP-043 §5–§7 parity (both directions) | wire-or-demote (owner picks per item) |
| F-050 | PROP-003 solver tail outside the §2.2 supersede marker | extend marker + fix MIG/PE rows |
| F-016/F-020/F-026/F-028/F-029/F-010/F-019/F-054/F-051 | single-file header/index touch-ups | small sweeps |
| F-024 | PROP-038 internal (§2.7 note + fuzz deferral) | two-line fix |
| F-039/F-040/F-041/F-034/F-037/F-038 | design-doc tense + MT keymap era | re-author (MT rows need human sign-off after) |

Code-side rows route to **Phase E** (Opus DRIFT tasks), queue currently
EMPTY: F-036 (`--plain` clap lie), F-047 (three stale deviates denying
the shipped ResolvoDepSolver — naive.rs:32, sat.rs:174, lib.rs:288-296),
F-048 (`--trust-mirror` / `vibe list --overrides` promised-absent),
F-055 (e2e harness must pin `VIBE_SETTINGS`), F-017 (aiui scrollbar
code-ahead).

## Non-obvious findings from this session (keep in mind)

- **Coverage counts come from `vibe progress mirror`'s ParsedDoc**
  (authoritative fence-aware parse) — the CONTINUE-recipe raw-grep
  extractor over-counts code-span shorthands (`@test/plan` inside
  backticks). Coverage identity per file:
  `anchors_with_id + marked_cells_without_id + document_elements ==
  marker_count`. `run/mirror/` is ephemeral — regenerate at will,
  delete after.
- **Verdict conventions held all phase** (cache campaign maps only;
  impl/done ⇒ presence, spec/done ⇒ absence, doc/done ⇒
  no-contradiction; family rows > per-unit rows; never rebuild
  cache.json from scratch — load-and-merge, `ensure_ascii=False,
  indent=1` + trailing newline).
- **Journal appends need `newline='\n'`** in python `io.open` — plain
  text-mode append writes CRLF on Windows (git normalizes, but keep it
  clean).
- **The floor-red post-gate was environmental** (F-055): a
  `~/.vibe/registry.toml` appeared 2026-07-25 15:37 and
  `cli_pkg_cycle::install_from_git_registry` does not isolate the
  settings chokepoint — proven by `VIBE_SETTINGS=<empty tmp>` turning
  it green. Docs-only campaign commits are unrelated. Until F-055 is
  fixed, `self-check` on this machine is red at that one test; judge
  floor claims accordingly (and remember: REAL exit codes, never
  `| tail`-piped).
- **One verdict was corrected mid-phase** (c4b SOLVER-IDENTITY-FIELD
  confirm → drift in c4c) — the correction pattern is: fix the cache
  verdict via load-and-merge, extend the finding note, record it in
  the LOG entry of the correcting batch.
- **resolvo IS the production default** (`registry.rs:117
  unwrap_or("resolvo")`) — do not trust the three in-code deviates
  that say otherwise (F-047).

## State

- Branch `main`; **GitHub synced through `242085d4`**; working tree
  clean (untracked `.zcode/` only). Journal closed (last:
  `c4f3-index` step-done + the `floor-red-postgate` note); phase lane
  reads **C**.
- `progress check` = **0 errors** (must stay 0); scan: 58 files,
  4 944 markers, 4 880 facts, 0 unmarked.
- **Findings 55** (next free id **F-056**), all in
  `campaigns/progress-2026-08/run/state/findings.json`.
- **GitVerse SSH down** all 2026-07-25 (banner-exchange timeout;
  strict ancestor verified via HTTPS). Plain `cargo xtask mirror`
  re-fan on recovery; NEVER `--force`.
- specmap ratchet: 37 gated orphans host-side (re-based at
  `f311f429`; gate passes, 0 suspects).
- vibespecs 401 on this machine — redbook/rust-ai-native resolve via
  vibe-embedded; consuming lockfiles carry `source_kind = "embedded"`.

## Standing decisions in force (unchanged unless noted)

Fact-exhaustive granularity (PROP-043 §3.9); anchored-when-marked; two
registers; scope 58 files (include-only); phase order B → L → C → D
(B/L/C CLOSED 2026-07-25); the campaign plan stays in
`spec/terraforms/` (owner reversal — §6 recurrence consumes it in
place); `legacy-spec/` is an archive — never cited normatively;
**no fractality for this campaign** (Fable = markup/verification/
stitching-review; Opus = DRIFT coding, queue EMPTY); coder-tier engine
pin `claude-opus-5` (`.claude/agents/opus5.md`); campaign zone excluded
from scan; verdicts live in cache campaign maps, never markup;
`reality-mismatch` closes only through sync-from-code with owner
approval; the four CLAUDE.md rules bind every executor. Amend only
unpushed commits.

## Recent commits (newest first)

```
242085d4 docs(spec): post-gate floor red root-caused — the test drinks user state
ddf7c0ca docs(spec): Phase C closes — the spec tree measures 93.0% true
5aa5ba86 docs(spec): Phase C c4f2 — mcp and settings; the cleanest batch yet
727f6840 docs(spec): Phase C c4f1 — the registry rest; PROP-030 takes the crown
74025dd9 docs(spec): Phase C c4e — Spec 2 is the reality, DRAFT is the costume
26a72e28 docs(wal): mid-session checkpoint — modules cluster past half (c4a…c4d2)
dcfa6301 docs(spec): Phase C c4d2 — the workspace tail; honesty at its densest
c325d333 docs(spec): Phase C c4d1 — the loading model verifies as lived reality
7392fbdd docs(spec): Phase C c4c — the resolver pair verified; a c4b verdict falls
09327502 docs(spec): Phase C c4b — the registry core holds; resolvo is real
baffe617 docs(spec): Phase C c4a — the campaign's contract verifies itself
3b10a37b docs(wal): session-end checkpoint — L closed, C mid-flight at c3d
4cf97e35 docs(continue): cold-resume checkpoint — Phase C modules handoff
9baa7fa6 docs(spec): Phase C c3d — the common tail verified; spec/common closes 12/12
f82582f7 docs(spec): Phase C — LOG records the c3b and c3c boundaries
3d237c7d docs(spec): Phase C c3c — the proposed-era pair confirms on shipped evidence
49d67c39 docs(spec): Phase C c3b — the foundation verified; six aged spots in one file
f2beeff4 docs(spec): Phase C c3a — five common PROPs verified; family roster aged
5c5e1058 docs(spec): Phase C c1 — manual-tests verified; three drift findings
bb337e90 docs(spec): Phase C opens — verify loop proven on the boot pair
15c5bb30 docs(wal): Phase L closed — boundary entry, status line, checkpoint
f311f429 chore(specmap): regenerate the canonical index after the relocation
70f3cbdd docs(spec): Phase L lands — the legacy dirs leave the spec tree
1ec6a27c docs(spec): Phase L batch 4 — the last corpus and crate refs go archive-form
9514e8fb docs(spec): Phase L batch 3 — modules index, MCP and hybrid contracts repoint
```

## Repository map (top level)

- `spec/` — the living corpus (boot authored pair, common 12,
  modules 35, design 6, manual-tests 3; **all 58 verified**) +
  `WAL.md` + the campaign plan in `spec/terraforms/` (sole live file
  there).
- `legacy-spec/` — the archive (terraforms 25 / research 8 /
  neworder 1 / discipline 1); zero normative inbound refs.
- `crates/` — the workspace (17 crates; progress-core + the vibe-cli
  adapter run the campaign tooling).
- `campaigns/progress-2026-08/` — journal (closed), state (findings
  55), cache with the FULL corpus verdict maps (load-bearing!),
  RESUME.md; `run/mirror/` deleted (ephemeral).
- `tools/self-check.sh` (the floor; currently red at ONE test —
  F-055, environmental), `tools/progress-dashboard/`.
- `specmap.json` — regenerate via `cargo xtask specmap`.

## Quick start

```bash
bash tools/self-check.sh                                  # floor (REAL exit; F-055 red expected until fixed)
cargo run -q -p vibe-cli --bin vibe -- progress check     # markup gate (must be 0)
cargo run -q -p vibe-cli --bin vibe -- progress scan      # refresh state (preserves campaign maps)
cargo run -q -p vibe-cli --bin vibe -- progress resume    # regenerate RESUME.md
node tools/progress-dashboard/serve.mjs                   # dashboard (X/Y/Z per file)
GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes" cargo xtask mirror
```

Verdict-читалка (сводка по файлу из cache):

```bash
python -c "import json,io; c=json.load(io.open('campaigns/progress-2026-08/run/cache.json',encoding='utf-8')); r=c['files']['spec/modules/vibe-registry/PROP-030-embedded-registry.md']['campaign']; print(r['summary']); [print(k,v['v']) for k,v in list(r['verdicts'].items())[:10]]"
```
