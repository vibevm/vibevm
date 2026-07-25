# CONTINUE — cold-resume checkpoint

_Written 2026-07-25 (session end: Phase L executed and closed; Phase C
opened and 4 of 5 clusters verified — `spec/common` closed 12/12; the
modules cluster is the remaining work). `spec/WAL.md` is the canonical
living state and supersedes this snapshot wherever they diverge._

## TL;DR

Progress-Control campaign (PROP-043, plan
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`): **Phase L
(legacy relocation) executed and closed in one session** — 26
gate-binding citation sites repointed in 4 batches, 35 files `git mv`'d
to root `legacy-spec/` (the campaign plan carve-out honoured), exit
gate green (greps zero, `check --exhaustive` 0, floor real exit 0).
**Phase C (verification) is IN FLIGHT**: the verify loop is proven and
conventions are durable in the plan LOG §9; **1 398 / 4 944 units
carry verdicts — 1 353 confirmed / 42 drift / 3 unverifiable;
findings 44** (F-035…F-044 new this session). Verified clusters:
boot ✓ manual-tests ✓ design ✓ common ✓ (23 of 58 files). **Remaining:
the modules cluster — 35 files, 3 300 markers** (the specmap-richest:
626 edges). GitHub synced through `9baa7fa6`; GitVerse SSH still down
(strict ancestor; plain re-fan on recovery, never `--force`).

## How to start the next session (exact)

Открой сессию фразой вида:

> Продолжаем спец-актуализационную кампанию (PROP-043). Прочитай
> CONTINUE.md, затем campaigns/progress-2026-08/run/RESUME.md и хвост
> LOG §9 плана. **Продолжай Phase C — кластер modules** (батчи c4a…):
> вердикты по конвенциям LOG §9, Fable без fractality; opus5 — только
> если расчёт delegation-rules это оправдает.

Then the executor does, in order:

1. Boot per `CLAUDE.md`; read this file, `run/RESUME.md`, the plan §5
   Phase C + the LOG §9 tail (the c0…c3d entries carry the conventions
   and the running tally).
2. Journal `step-start` for the first batch (`c4a-…`; the phase lane
   already reads `C` — no new phase event needed).
3. Work the batches per the proven loop below; commit per batch;
   `cargo xtask mirror` at checkpoints (github must go ok; gitverse
   fails while its SSH is down — expected).
4. Exit gate for C (plan §5): 100 % of markers carry verdicts; the
   X/Y/Z summary lands in the LOG. **Ask the owner before opening
   Phase D.**

## The C verify loop (proven; follow verbatim)

Per batch: journal `step-start` → Read the file(s) IN FULL → run
targeted machine checks (specmap join, greps, CLI `--help`, file
existence; every check lands in the ev strings) → write a
verdict-writer script into the scratchpad → run → `progress scan`
(preserves campaign maps — verified) + `progress check` (stays 0) →
journal `step-done` with X/Y/Z → LOG §9 running-tally entry → commit
(campaign `run/` included) → mirror at checkpoints.

**Verdict storage (PROP-043 §7.1/§7.5):** per-file `campaign` map in
`campaigns/progress-2026-08/run/cache.json`:

```json
"campaign": {"verify_batch": "c4a-…", "verified_at": "<UTC>",
  "processed_hash": "<content_hash>", "anchors_judged": N,
  "fact_count": N, "marker_count": N,
  "coverage_note": "verdicts keyed by fact anchors; cell-markers inherit their row anchor; <status> elements judged as _elements",
  "verdicts": {"<ANCHOR>": {"v": "confirmed|drift|unverifiable", "ev": ["…"]}, "_elements": {…}},
  "summary": {"confirmed": X, "drift": Y, "unverifiable": Z}}
```

Load cache.json with python (`json.load`), mutate, dump with
`ensure_ascii=False, indent=1` + trailing newline. NEVER rebuild the
file from scratch — other files' campaign maps must survive.

**The anchor extractor** (pairs `##ID` with its `@stage/state`; count
== per-file coverage, asserted against exceptions):

```python
MARK = re.compile(r'@(?P<stage>[a-z]+)/(?P<state>[a-z]+)\b')
ANCH = re.compile(r'##(?P<id>[A-Za-z][A-Za-z0-9._-]*)')
def extract(path):
    out, cur = [], None
    for line in io.open(path, encoding='utf-8').read().splitlines():
        a, m = ANCH.search(line), MARK.search(line)
        if a and m: out.append(a.group('id')); cur = None
        elif a: cur = a.group('id')
        elif m and cur: out.append(cur); cur = None
    return out
```

**The specmap join** (rebuild each session — the scratchpad dies):
index `specmap.json` `spec_units` by `uri`→`(file, anchor)`, then fold
`edges` (verb ∈ implements/verifies/deviates + `file:line` refs) into
`file → anchor → {counts, refs}`. ~20 lines of python; the c3-era
version lived at scratchpad `c_specmap_join.py`.

**Verdict semantics by stage (the honesty law):**

- `impl/done` → needs PRESENCE evidence (edges, code, live runs);
- `spec/done` → needs ABSENCE (shipped-but-still-marked-spec = the
  stale-header drift; honest design proposals grep-verify their
  "schedules nothing" claims);
- `doc/done` → must not contradict the contract/shipped reality
  (aged present-tense = drift);
- dated historical records confirm unless falsified; present-state
  claims blocked by the GitVerse outage go `unverifiable`, never
  "probably fine";
- verdicts NEVER in markup; wording changes forbidden.

**Grep pitfalls (paid for):** `grep -c` counts LINES, not occurrences
(labels+targets double up on one line — count with `s.count()` in
python and assert); rg's `-r` flag REPLACES match text in output
(never use it as "recursive"); literal backtick paths and dotted forms
(`` `spec/x/…` ``, `spec.research`) are invisible to relative-link
patterns — sweep word-level too; use lookbehind `(?<!legacy-)` when
grepping for `spec/(terraforms|research|neworder|discipline)/`.

**Findings:** before minting a new row, grep
`run/state/findings.json` for the file — the B-phase already ledgered
many stale headers (known: F-005 PROP-019, F-006 PROP-024, F-013
PROP-018, F-018 PROP-020; F-016 modules/README, F-020 OWNER-GUIDE are
pre-existing rows the c4 batches will cite). Next free id: **F-045**.
Family rows > per-unit rows (one sweepable fix per file/theme).

## The modules batch map (35 files, 3 300 markers; suggested order)

| Batch | Files | Markers | Notes |
|---|---|---|---|
| c4a | vibe-progress: PROP-043 (159, 19 edges), OWNER-GUIDE (67), templates ×3 (49) | 275 | The campaign's own contract — lived-experience evidence; F-020 pending on OWNER-GUIDE |
| c4b | vibe-registry core: PROP-002 (360, **110 edges**), PROP-001 (93, 9) | 453 | The evidence-richest file in the corpus |
| c4c | vibe-resolver: PROP-003 (314, 60), PROP-017 (106, 9) | 420 | PROP-003's F-031 internal-r2 rows pending |
| c4d | vibe-workspace ×9: PROP-007 (147, 39), PROP-009 (104), PROP-035 (135), PROP-011 (97), PROP-012 (84), PROP-038 (86), PROP-034 (66), PROP-022 (53), PROP-020 (50), PROP-025 (46) | 768+100 | PROP-020's F-018 header; PROP-034 superseded-by-038 arc |
| c4e | vibe-cli + vibe-actions: PROP-037 (137, **64 edges**), PROP-036 (80, 14), PROP-042 (58, 17), PROP-039 (52, 18) | 327 | TUI evidence already gathered in c1/c2 (goldens, keymaps, theme) |
| c4f | vibe-mcp ×3 (188), vibe-index PROP-005 (279, **91 edges**), vibe-settings ×2 (118), vibe-registry rest: PROP-008 (100), PROP-010 (93), PROP-030 (76), PROP-021 (38), PROP-023 (46), README (19) | 957 | Split further as needed (2–3 commits) |

Get the live list + edge counts any time:

```bash
python -c "import json,io,collections; c=json.load(io.open('campaigns/progress-2026-08/run/cache.json',encoding='utf-8')); [print(p, r['marker_count']) for p,r in sorted(c['files'].items()) if p.startswith('spec/modules')]"
```

## State

- Branch `main`; **GitHub synced through `9baa7fa6`**; working tree
  clean (untracked `.zcode/` only). Journal fully closed (last:
  `c3d-common-tail` step-done); phase lane reads **C**.
- `progress check` = **0 errors** (must stay 0); scan: 58 files,
  4 944 markers, 4 880 facts, 0 unmarked.
- **C tally: 1 398 units = 1 353 C / 42 D / 3 U; findings 44** (next
  id F-045). Drift profile: "the spec aged behind its own success" —
  fired triggers (UPL relicense F-043, engine split F-044), executed
  deferrals, proposed-era headers over shipped systems (F-005/006/013),
  MT keymap era (F-037/038), stale clap help (F-036, Phase E DRIFT
  candidate). Honest design proposals pass clean.
- **GitVerse SSH down** all 2026-07-25 (banner-exchange timeout;
  strict ancestor verified via HTTPS). Plain `cargo xtask mirror`
  re-fan on recovery; NEVER `--force`.
- specmap ratchet: 37 gated orphans host-side (re-based at `f311f429`;
  gate passes, 0 suspects).

## Standing decisions in force (unchanged unless noted)

Fact-exhaustive granularity (PROP-043 §3.9); anchored-when-marked; two
registers; scope 58 files (include-only); phase order B → L → C (L
CLOSED 2026-07-25); the campaign plan stays in `spec/terraforms/`
(owner reversal — §6 recurrence consumes it in place); `legacy-spec/`
is an archive — never cited normatively from corpus/crates (WAL
constraint); **no fractality for this campaign** (Fable =
markup/porting/verification/review; Opus = DRIFT coding, queue EMPTY);
coder-tier engine pin `claude-opus-5`; campaign zone excluded from
scan; verdicts live in cache campaign maps, never markup; the four
CLAUDE.md rules bind every executor. Amend only unpushed commits.

## Recent commits (newest first)

```
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
f8f347d8 docs(spec): Phase L batch 2 — action + settings contracts drop research refs
83346e78 docs(spec): Phase L batch 1 — design cluster repoints to the archive
```

## Repository map (top level)

- `spec/` — the living corpus (boot authored pair, common 12,
  modules 35, design 6, manual-tests 3; all marked, 23 verified) +
  `WAL.md` + the campaign plan in `spec/terraforms/` (sole file).
- `legacy-spec/` — the archive (terraforms 25 / research 8 /
  neworder 1 / discipline 1); zero normative inbound refs.
- `crates/` — the workspace (17 crates); `campaigns/progress-2026-08/`
  — journal, state (findings 44), cache with verdict maps, RESUME.
- `tools/self-check.sh` (the floor), `tools/progress-dashboard/`.
- `specmap.json` — regenerate via `cargo xtask specmap`.

## Quick start

```bash
bash tools/self-check.sh                                  # the floor (REAL exit code)
cargo run -q -p vibe-cli --bin vibe -- progress check     # markup gate (must be 0)
cargo run -q -p vibe-cli --bin vibe -- progress scan      # refresh state (preserves campaign maps)
cargo run -q -p vibe-cli --bin vibe -- progress resume    # regenerate RESUME.md
node tools/progress-dashboard/serve.mjs                   # dashboard
GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes" cargo xtask mirror
```
