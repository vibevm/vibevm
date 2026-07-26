# CONTINUE — cold-resume checkpoint

_Written 2026-07-26 (session end: **Phase B opened and three batches landed;
eleven findings; Phases T and G designed**). `spec/WAL.md` is the canonical
living state and supersedes this snapshot wherever they diverge._

## TL;DR

**Phase B is running and 34 of 202 package files are marked.** B1 + B2 closed
`core-ai-native` entirely (943 units, 16 files); B5 closed `go-ai-native-lang`
(665 units, 19 files). **4 276 unmarked facts remain**, measured, not derived.

**Eleven findings, F-081 to F-091.** The through-line is uncomfortable and worth
carrying into the next session: **three of them were the measuring instrument
being wrong, not the corpus** — a floor gating a frozen slot, a parser blind to
units its own grammar allows, a sync gate covering four of seven workspaces.
Each was green and each was wrong.

**Two phases were designed from scratch this session** and are not yet run:
**Phase T** (tests, swarm) and **Phase G** (documentation as a package). Both
have full specs. Phase T also has a worker prompt and an operator runbook.

## The one thing to do first

**Batch B6** — `typescript-ai-native-lang`, 18 files. Everything it needs
exists: write a thin `MARKUP-B6.md` beside `MARKUP-B5.md` and dispatch, exactly
as B5 was.

Read first: `campaigns/packages-2026-09/tasks/MARKUP-B1.md` — its two **LOCKED**
sections carry **29 rulings** (15 from B1a, 6 from B1b, 8 from B2) that bind
every batch. **Rulings 18 and 19 are struck** — DRIFT-031 closed the findings
they encoded.

## Where the numbers are, and why they keep moving

| | |
|---|---|
| observed corpus | **260 files** (58 host + 202 packages) |
| unmarked facts | **4 276** — re-measured after B5 |
| package files at 0 unmarked | **34 of 202** |
| batches left | **B6–B16**, minus what F-091 dissolved |

**Never decrement these; re-measure.** The count has read 6 561 → 5 685 → 5 068
→ 4 685 → 4 276, and every step down was a **measurement**, never a subtraction.
Two of the drops were corpus subtractions, not progress.

**The `facts` column in `BATCH-PLAN.md` is a pre-markup scan count and
under-predicts a batch by ~62 %.** B5 scanned at 411 and finished at 665 units —
84 paragraphs became 338. Size B6–B16 with a ×1.6 multiplier, and **do not let
Phase T size its swarm off the scan number.**

## Owner rulings this session — all applied {#rulings}

- **F-080** — `spec/legacy-projections/` is frozen history; excluded (11 files).
- **F-091** — `spec/book/**` is reference depth, not a contract; excluded (4
  files, 383 facts). **A new category:** the earlier subtractions were staleness
  or duplication; the book is authored, current, and excluded for its **genre**.
- **F-082** — boot snippets **are** marked, in packages and in vibevm itself.
  The +52 % growth is accepted as the price of an addressable boot lane.
- **F-085** — the `spec://` URI parser accepts fact ids; heading anchors then
  widened to the same grammar, **no case folding**. Anchors are
  **case-sensitive at every level** — writing, duplicate detection, resolution.
- **F-075** — needed no code; `seal` has written `processed_hash` since
  `e9fc7b44`. **Amendment A4 is discharged.**
- **F-077** — the per-file `summary` is gone and computed on read. **`counters`
  stays**, but must be written from the single computation and pinned by a test
  — *that fix is not yet made.*
- **F-086** — both package gates now carry a denominator; sync went 5 → 7
  workspaces, the floor 4 → 7.
- **Attribution** — `**Executor:** Opus` stays in campaign task files.
- **`BACKLOG.md`** — created; findings that are neither campaign work nor an
  emergency land there, severity-triaged, and the next wave drains from it.

## Still open, and whose {#open}

| id | what | who |
|---|---|---|
| **F-069** | aggregator grammar — a fact about another package | Phase C's, not B's |
| **F-078** | boot-lane duplication; the counter is necessary and **not sufficient** — `##HOIST-LCA` puts the hoist target at the root, which is also the root's own compile site. **DRIFT-035 is written and NOT dispatched** | owner decides timing |
| **F-083** | `SKILL.md` YAML frontmatter cannot carry an anchor — **9 files across 6 packages**, the only observed files starting with `---`. A worker found an in-file dodge and correctly did not apply it; the cheaper fix is one exemption in `blocks.rs` | needs a task |
| **F-087** | 17 commit bodies in history name a model, which the attribution policy forbids. **Cannot be cleaned** — that needs a history rewrite and the mirror law forbids `--force`. Accept, or amend the policy | owner |
| **F-088** | `ATLAS.md` declares itself generated from `findings.jsonl`, which is tracked **nowhere**; this campaign minted 93 hand anchors into a file forbidding hand-edits | owner |
| **F-089/090** | PROP-014 names a crate with zero occurrences (`specmap-core`); three of its decision subsections omit the kind line its own normativity rule requires | drift stage |
| **F-077 tail** | `counters` written from one computation, pinned by a test | task not written |

## What was designed but never run {#designed}

**Phase T — tests by swarm** (`campaigns/packages-2026-09/PHASE-T-SPEC.md`).
Between E and F. **≥3 tests of distinct kinds per testable assertion.** The
decisions that carry it:

- **The fact's text is the oracle, the code is not.** The worker never sees the
  implementation body and writes the expected value as a **literal before
  running anything** — a literal cannot be copied from a run that has not
  happened. §3.1 is a 7-step routine, §3.2 bans the vacuous shapes, §3.3 is a
  worked pair. **§3.3 is the load-bearing part**, per the discipline's own
  result about weak readers.
- **Cargo is off the parallel path entirely.** Sub-agents write text and never
  invoke it; the lead runs one wave build and one **batched** red exhibit. A
  wave of 10 × 5 tests is 3 cargo invocations, not 150.
- **Waves close.** A fact correction is what invalidates tests, not a code fix —
  so every `verifies` edge carries `r=N` and PROP-014's asymmetric invalidation
  names the affected tests. **Wave 0 is a calibration whose deliverable is the
  red rate.**
- **Phase T never builds a surface.** A fact whose surface is absent is
  **T-unbuilt** — a fourth triage bucket, a drift verdict owed to the ledger,
  and a **P2** in `BACKLOG.md`. The `#[ignore]`d test written from it **is** the
  specification of the missing work.
- Two accounts, 10 sub-agents each: `PHASE-T-WORKER-PROMPT.md` (template +
  integration procedure) and `PHASE-T-RUNBOOK.md` (what the operator does).
  **`git worktree add` needs `-c core.longpaths=true` on this repo** or it fails
  opaquely.

**Phase G — documentation** (`PHASE-G-SPEC.md`). After F. `docs/` (43 files,
**in no include glob — unmeasured, not stale**) moves to `docs-legacy/`;
documentation becomes `org.vibevm.doc/doc` with `org.vibevm.doc/web` reserved.
**Documentation cites a spec unit and never restates it**; links run one way and
the spec tree does not know documentation exists. **Confirm "quick.dev (v2)"
means Qwik before writing that manifest.**

## Traps that cost real time this session {#traps}

1. **A ledger entry is a claim with a date. Quoting it is not checking it.**
   Three stale lines: `deferrals.md`'s F-067 (fixed a day earlier, and the stale
   wording went into two task files), `#engine` (said Phase C was blocked;
   `vibe update` had unblocked it), and **`CLAUDE.md` itself** saying `TASKS.md`
   was not present — it has existed since April, and `Write` blocked an
   overwrite that claim nearly caused.
2. **A phrase sweep is not an audit.** DRIFT-024's audit missed `ATLAS.md`
   because it searched for the wording three known files used.
3. **Watching failures iteratively under-counts.** Enumerating blocked tests by
   running until failure gave 3, then 5; `--no-fail-fast` gave the true 6.
4. **Amending a clause without re-reading its own document** left PROP-014
   contradicting itself inside one bullet list, eight hours apart.
5. **`vibe update` may be run freely** (owner grant). It is what repointed the
   resolve off the stale second working copy.
6. **A convention list is a derived thing too** — two of the 29 rulings had
   outlived their findings by a day.

## Quick start

```bash
bash tools/self-check.sh ; echo "EXIT=$?"
cargo run -q -p vibe-cli --bin vibe -- progress check --no-cache --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
cargo xtask mirror
```

Always pass `--campaign`; with two zones a bare command writes no state. Use
`--no-cache` after any parser or grammar change. **Never point a progress
command at `campaigns/progress-2026-08`** — it is archival.

## Repository map

- `spec/` — living corpus (58 files, 0 unmarked) + `WAL.md` + both campaign
  plans in `spec/terraforms/`.
- `packages/` — wave 2's subject: `org.vibevm.world` (27) + `org.vibevm.ai-native`
  (10). `org.vibevm.fractality` and `org.vibevm.vibeapp` are out of scope.
- `campaigns/packages-2026-09/` — **live.** `BATCH-PLAN.md`, `deferrals.md`,
  `PHASE-T-SPEC.md`, `PHASE-T-WORKER-PROMPT.md`, `PHASE-T-RUNBOOK.md`,
  `PHASE-G-SPEC.md`, `tasks/`, `run/`.
- `campaigns/progress-2026-08/` — **archival.**
- `BACKLOG.md` — new; P1/P2/P3, drained by the next wave.
- `docs/` — 43 files, unobserved, Phase G's subject.

## Recent commits

```
c1ba307f docs(backlog): a home for what the programme found and did not do
b3aebc4b docs(campaign): a fact can claim a surface that does not exist
6ff66825 docs(campaign): waves, because a fact correction is what invalidates
4b03630c docs(campaign): cargo comes off the parallel path instead of being queued
be2c712f docs(campaign): two locked rulings had outlived their findings
d3242f99 docs(go-ai-native-lang): the go stack becomes fact-addressable
bfdad4ec docs(campaign): the integration procedure, and the check it was missing
39d25545 docs(campaign): the prompt is the only channel between two worker accounts
d6b9f186 docs(campaign): two accounts in parallel is a file-ownership problem
3495a8d2 docs(campaign): a method for writing a real test, aimed at a weak reader
8eab314d feat(progress): the book leaves the corpus, on genre and not on age
48c57866 docs(campaign): B2 locks eight more rulings and finds a second derived file
744ec8ed docs(core-ai-native): the mechanism specs become fact-addressable
9648433d docs(campaign): a third gate turns out to be green by not looking
322f2313 feat(gates): both package gates learn their own denominator
```

**The WAL is the canonical living state.** If this file and `spec/WAL.md`
disagree, the WAL wins.
