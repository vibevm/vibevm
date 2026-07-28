# CONTINUE — cold-resume checkpoint

_Written 2026-07-28 (**Phase C: the `ai-native` cluster is CLOSED, 80 of 80 files;
`world` is untouched**). `spec/WAL.md` is the canonical living state and supersedes
this snapshot wherever they diverge._

## TL;DR

**Phase C of the packages campaign is 39.4 % done: 2 697 verdicts written, sealed
and committed; 4 150 anchors remain, all of them in the `world` cluster.**

The `ai-native` cluster measured **92.4 % confirmed** (2 491 / 175 drift / 31
unverifiable) across 80 files and six batches. Seven findings were opened, four of
which need the owner. The exit gate's clause (iii) — the one wave 1 skipped — is
**satisfied**: 39 captured runs live under `campaigns/packages-2026-09/harvest/`.

**Nothing is blocked. Nothing is running.** The next session has a paste-ready
prompt in
[`campaigns/packages-2026-09/PHASE-C-RESUME.md`](campaigns/packages-2026-09/PHASE-C-RESUME.md).

## Where the numbers are {#numbers}

| | |
|---|---|
| host | 58 / 58 files, 4 499 verdicts (4 496 confirmed · 3 unverifiable) |
| **ai-native** | **80 / 80 files CLOSED** — 2 697 verdicts, **2 491 / 175 / 31, 92.4 %** |
| **world** | **0 / 121 files** — **4 150 anchors owed**, batches W1…W7 |
| phase | **2 697 of 6 847 — 39.4 %** |
| gate | `progress check --exhaustive` clean, 259 files, 0 warnings |
| tree | clean, in sync with `origin/main`, mirrored to GitVerse + GitHub |

**Never decrement these; re-measure.** Every figure came from a command.

## The one thing to do first {#first}

**Not `world`. Close the reviewing debt.** 138 rows were classified in bulk instead
of read: 60 `partial` rows in `tasks/evidence/ev-C45-go.json` sorted by filename, and
78 in `ev-C45-rust.json` sorted by a single two-branch rule. A `partial` is *related
code that does not settle the claim* — the class that carries drift — so sorting 138
of them by filename is the thinnest reviewing this phase has done, and the verdicts
do not say so.

Read them, judge each on its own evidence, restate what moves with
`merge-verdicts.py … --force`. The tables are on disk and their refs are already
machine-verified, so the job is reading plus a merge.

## Then: the `world` cluster {#world}

Seven batches, fixed in `tasks/PHASE-C-BATCHES.json`: **W1** git family (16 files,
407) · **W2** two-process-model/wal/wal-specspaces/sync-from-code (20, 692) ·
**W3** addressable-specs/decision-records/conflict-protocol (15, 615) · **W4**
campaign-plans/discovery-prompt/comparative-research/redbook (15, 564) · **W5**
operating-modes/health-audit/manual-tests/secrets-hygiene (21, 697) · **W6**
licensing/source-mirrors/spec-genres/dev-runtime-docs (19, 572) · **W7**
managed-blocks/qualified-naming/tool-design-lessons (15, 603).

**Two of §3.1's three sources are already mechanised and captured**: the source-1
link join reads **185 citations, 0 broken** over the observed corpus, and the
source-2/3 boot-lane join reads **17 of 31 contributions carrying the package's exact
word stream**. W2 and W5 are provisional at ~695 anchors; re-measure the per-anchor
cost after the first world batch.

**`world` verdicts need a `src` field that `ai-native` did not** — a non-empty subset
of `[1,2,3]` naming which source class the evidence rests on (amendment A2).
`merge-verdicts.py` refuses a `world` batch without it.

## Non-obvious findings from this phase {#findings}

1. **The discipline gates everything except itself.** No package under
   `packages/org.vibevm.ai-native/` carries a `conform.toml` or a `discipline/`
   directory, so every discipline-specific floor step fails or is skipped in all six
   slots, while the three portable steps pass wherever the toolchain is present.
2. **F-121 is a family of four.** ENGINE-CONFORM, LEDGER-INTENT, BROWNFIELD and
   PROP-014 each end with «any unexercised mechanism is removed from this document
   rather than carried as aspiration», each marks it `@impl/done`, each is
   contradicted by its own contents, and nothing enforces any of them.
3. **F-122 — one `name@version`, two contents, 173 files across 33 packages.** Phase
   B marked package files inside already-published version slots. Closing it is a
   release event, not an edit.
4. **F-123 — we break a rule we ship.** 82 of the last 400 commit subjects exceed
   `conventional-commits`' 72-character hard limit (20.5 %), spread across the
   campaign's working days. F-087 measured at the same time: 4 model mentions in 400
   bodies, **none an authorship claim** — two are a colour-theme name.
5. **The perimeter was wrong five times, and never in a worker's work.** A
   mechanism's spec lives in `core-ai-native`, its engine in that package's library
   crates, its driver in each stack's CLI, and its deployment in a consuming
   project — `research/{rust,ts,go}-demo/`. A fact can be true at one layer and
   invisible at the other three.
6. **A count that includes `node_modules` is a count of somebody else's code.** Ten
   TypeScript verdicts were confirmed on such a count and had to be restated to
   drift; fifteen Go verdicts were recorded unverifiable on an absence asserted
   without checking (`research/go-demo` exists and is complete).
7. **Rust 100 %, TypeScript 98.8 %, Go 93.9 %** on the same nine scaffolds and the
   same oracle shape. The ordering is about this repository: the host dogfoods the
   Rust stack, the TypeScript consumer is complete, and the Go consumer's toolchain
   (gopls) is not installed.

## Decisions in force {#decisions}

- **Verdicts live in the cache, never in markup** (PROP-043 §7.1/§7.5); mutate by
  **load-and-merge only**; `verified_at` and `processed_hash` are written by
  `vibe progress seal` and never by hand.
- **Three ratified at the phase opening** (2026-07-28): a verdict carries its §3.1
  source class in an `src` field rather than as a prose prefix; the subject is never
  modified to make the measurement pass, so `<lang>-ai-native init` is not run; and
  `vibedeps/` substitutes for §3.1's third source because `files_written` is `[]` for
  all 36 packages.
- **A finding is reported, never fixed, by a Phase C batch.**
- **Delegation goes to the harness's built-in `opus5` subagents, not fractality**
  (owner ruling, 2026-07-28). The verdict is never delegated; neither is the review
  of delegated output.
- **Owner grant:** work autonomously across batch and cluster boundaries; stop only
  for a genuine semantic or architectural decision.

## Campaign tooling built this phase {#tools}

All under `campaigns/packages-2026-09/tasks/`, each with its refusals tested before
use:

| tool | what it settles |
|---|---|
| `merge-verdicts.py` | the three verdict rules stop being prose — six refusals, all made to fire |
| `verify-evidence.py` | a delegated `path:line` is checked by machine — 3 947 refs, 12 unresolvable |
| `source1-join.py` | §3.1 source 1: does every cited document exist and carry its anchor |
| `source23-boot-join.py` | §3.1 sources 2 and 3, over the compiled boot lane |
| `scaffold-three-way.py` | the parallel corpora — nine scaffolds ×3, and the `discipline-mcp` trio |
| `coordinate-divergence.py` | one `name@version`, two contents (F-122) |

## Quick start {#quickstart}

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --campaign campaigns/packages-2026-09
```

```bash
python campaigns/packages-2026-09/tasks/verify-evidence.py campaigns/packages-2026-09/tasks/evidence/*.json
```

```bash
bash tools/self-check.sh ; echo "EXIT=$?"
```

```bash
cargo xtask mirror
```

Always pass `--campaign`. Use `--no-cache` after any parser change.

## Repository map {#map}

- `spec/` — living corpus (0 unmarked) + `WAL.md` + both campaign plans in
  `spec/terraforms/`. The §9 LOG of `PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` carries
  this phase's record, entry by entry.
- `packages/` — wave 2's subject. `org.vibevm.ai-native/` is **judged in full**;
  `org.vibevm.world/` is next. `org.vibevm.fractality` and `org.vibevm.vibeapp` are
  out of scope.
- `campaigns/packages-2026-09/` — **live.** `PHASE-C-RESUME.md` (start here),
  `PHASE-C-BATCH-PLAN.md`, `PHASE-C-KICKOFF.md`, `BATCH-PLAN.md`, `baseline.json`,
  `harvest/` (39 captured runs), `tasks/` (6 tools + 15 evidence tables + the batch
  assignment), `run/` (cache, journal, state).
- `research/{rust,ts,go}-demo/` — **the discipline's consuming projects.** Not a
  sandbox: they carry `conform.toml`, `specmap.toml`, `specmap.json` and
  `discipline/registry/`, and they are where a language guide's rules are in force.
- `crates/progress-core/` — the gate's parser. `xtask/src/batch_review/` — the review
  tool. `BACKLOG.md` — P1/P2/P3.

## Recent commits {#commits}

```
6d82b5cf chore(campaign): the ai-native cluster closes at 80 of 80 files
d9270e75 chore(campaign): C4+C5 in two languages, and a name that outlived its crate
106e09c5 chore(campaign): C6 closes at 92.7 % and corrects C3 twice
6702441a chore(campaign): C3 closes at 89.7 %, and Go's gap is the tree's
55975e60 chore(campaign): C3a — the demos are the consumer, and forty facts turn on it
89c90aed docs(campaign): F-123 — we break a rule we ship, at a fifth of commits
9cabe34d docs(campaign): F-122 — one coordinate, two contents, 173 times
bf679a1c chore(campaign): C7 closes at 99/99, and F-116 is about the family
c8911c29 feat(campaign): F-116 stops being a reading and becomes a command
4b266611 feat(campaign): C4's parallel corpus gets diffed instead of re-read
76c6a142 chore(campaign): C2 closes at 92.4 %, and the drift is one thing said eleven ways
2ff1cbed fix(campaign): an elided quote is one rule, not a list of cases
0413154a chore(campaign): C2a — the ATLAS keeps its books and misstates its own source
666fe2c6 feat(campaign): sources 2 and 3 become one command, over the boot lane
1480aa25 docs(campaign): the perimeter law, written after C1 paid for it three times
38f9816c chore(campaign): C1 closes at 353 verdicts, and the perimeter was wrong thrice
036c7525 docs(campaign): the host does adopt PROP-014, by citation
fd6d5ac2 chore(campaign): five workers' readings stop living in a scratchpad
55bb7e71 chore(campaign): C1c — a not-found is a fact about the perimeter first
423d2883 fix(campaign): the evidence checker stops crying wolf twice
8a4d6b08 chore(campaign): C1b — the ledger is honest below the floor and empty above it
1bd51ee4 chore(campaign): C1a — the transport holds and the engine spec does not
f45202fc feat(campaign): a delegated line number gets checked by machine
6f276b5d docs(campaign): the one broken citation sits where nothing looks
726160a1 feat(campaign): source 1's mechanical half stops being a reading task
```

**The WAL is the canonical living state.** If this file and `spec/WAL.md` disagree,
the WAL wins.
