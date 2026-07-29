# CONTINUE — cold resume

**Do not quote the numbers in this file. Measure them.**

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
```

```bash
python campaigns/packages-2026-09/tasks/summary.py
```

Everything below that looks like a measurement was true when written and is a
hint about where to look, not a fact to repeat.

---

## TL;DR

**Phase D (Stitching) of the PROP-043 wave-2 campaign is OPEN and roughly a
third through.** It opened on 601 drift verdicts / 228 obligations and stands at
**470 / 181**; the corpus moved from 94.3 % to **95.5 %** confirmed.

**The phase turned out not to be what its plan expected.** It is not mainly a
document-repair phase — it is a **routing** phase. Of ~255 anchors examined,
about 40 moved. The rest split two ways: the rule is sound and the *host* does
not keep it (153 anchors, routed out), or the verdict itself was **wrong** (~40
re-judged `confirmed`, most of them because the original search could not see
the host).

Branch `main`, clean, **31 commits ahead of origin** at the time of writing.

**Two queues wait on the owner and nothing in them proceeds without a ruling:**
[`PHASE-D-RELEASE-QUEUE.md`](campaigns/packages-2026-09/PHASE-D-RELEASE-QUEUE.md)
and
[`PHASE-D-HOST-OBLIGATIONS.md`](campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md).

---

## The one thing to read before touching anything {#the-lesson}

**A search confined to `packages/` reads every successful adoption as an
absence.** Written up as
[§3.7 of the batch plan](campaigns/packages-2026-09/PHASE-D-BATCH-PLAN.md#compliance-blindness).

These packages *specify* a discipline; the host is the project that *adopted*
it; and the artefacts that prove adoption — `discipline/registry/`,
`discipline/golden/`, `terraform/`, `specmap.json`, `conform.toml` — live in the
**consumer**, because creating them is what complying means.

Measured over 76 `build-or-demote` verdicts: **18 claimed absences were false,
17 of them disproved by host artefacts.** And §3.3 closes a `missing-support` by
*moving a marker*, so every one would have written «specified, not built» over a
mechanism that ships.

**Practical consequence: never demote before searching the whole tree, and name
the perimeter in the record.** Six perimeter misses have now been paid for in
this phase alone.

---

## Where work stands, by route {#state}

| route | obligations | verdicts | who approves |
|---|---:|---:|---|
| `prose-edit` | 80 | 181 | **boss** — but ~all that remain are the address family |
| `build-or-demote` | 33 | 59 | boss; re-verify the perimeter first, always |
| `sync-from-code` | 51 | 171 | **owner**, on every spec diff |
| `release` | 17 | 59 | **owner**, before publication |

Convergence, which is what the exit gate measures:

```
obligations with nothing left owed to the package :   60
drift verdicts still owed a package repair        :  317 of 470
routed out of the package (route b / owner)       :  153
```

---

## The next step, and the two before it {#next}

**Unblocked and next:** the remaining **33 `build-or-demote`** obligations —
`managed-blocks` 4, `source-mirrors` 3, `licensing` 3, `campaign-plans` 3, and
sixteen packages with one or two each. Run them exactly like wave 5: a
re-verification pass first, demotion only for what survives the whole-tree
perimeter. Expect a quarter of the absences to be false.

**Then the address family** — 26 obligations, 54 verdicts, 22 packages, the
largest single defect in the campaign. **The owner has already ruled on the
repair** (2026-07-29): the 69 `../flows/…` links take `@spec://` where they are
pointers and `#embed` where the target belongs in the lane; a generated boot
artifact carries no token budget
([PROP-009 `##ARTIFACTS-CARRY-NO-TOKEN-BUDGET`](spec/modules/vibe-workspace/PROP-009-loading-model.md#artifacts));
PROP-035 §10's link tables are **not** a precondition and are `BACKLOG.md`
B-001. **What is not ruled is publication**, and every one of the 22 packages
needs it.

**And the two owner queues.** Neither is work the boss may start.

---

## What is delegable and what is not {#delegation}

Everything in this phase went to built-in `opus5` subagents (owner override of
the fractality default, batch plan §6). The briefs that worked are recoverable
from the session's own harvest records under `campaigns/packages-2026-09/harvest/d*-*.md`
— nine of them, each an entry per obligation with its re-verification command.

**Cut the batch by `closure_route` FIRST.** The first wave was cut by
`falsifier` instead and 24 of 28 obligations landed on owner routes; the owner
ordered the lot reverted. That is §6.1 `##ROUTE-BEFORE-FALSIFIER`.

**The verdict is never delegated, and neither is routing an anchor out of a
package** — both are the same class of judgement.

---

## Instruments, and the three that refuse {#instruments}

| script | what it does |
|---|---|
| `drift-registry.py` | the registry: 470 drifts → 181 obligations, with route, falsifier, convergence. `--task F-NNN` prints one obligation as a SPEC task's §2 |
| `summary.py` | verdict breakdown by zone |
| `merge-verdicts.py` | load-and-merge into `run/cache.json`; `--force` to restate |
| `verify-evidence.py` / `repair-refs.py` | every ref resolves, or is named / re-pointed |
| `batch-progress.py` | Phase C's coverage, now historical |

**`merge-verdicts.py` refused four times this session and was right every
time** — a JSON mixing two clusters, `src` on an ai-native verdict, and three
anchors that were **citations in backticks rather than definitions**. Trust it
over your reading.

Two mechanics that cost real work:

- **Never chain `merge-verdicts.py` and `progress seal`.** A refused merge
  writes nothing and the `&&` still seals — vouching *old* verdicts against
  *new* text.
- **Write verdict JSON with the Write tool, or a QUOTED heredoc.** An unquoted
  one let bash eat every backticked identifier out of the reasons. This rule was
  already written down and was broken anyway.

---

## Records this phase created {#records}

- **`run/state/routing.json`** — 153 anchors examined and deliberately *not*
  repaired in the package, one entry each with its obligation and why. **Without
  it the phase cannot converge**: an anchor routed to the host never stops
  reading `drift`, so the registry could never empty and the gate could not tell
  «not worked» from «worked, and the work belongs to the host». Written by the
  boss at review time, never by a worker.
- **`PHASE-D-BATCH-PLAN.md`** — the phase's contract. §1.2 routes, §3.3
  demotion, §3.6 which side yields, **§3.7 compliance blindness**, §6.1 the
  delegation rules bought at full price.
- **`PHASE-D-RELEASE-QUEUE.md`** — 17 release events in four groups, two needing
  a product decision before an edit exists to approve.
- **`PHASE-D-HOST-OBLIGATIONS.md`** — 53 obligations where the rule is sound and
  the host does not keep it, each taking one of three answers, none of them
  «soften the package».
- **`BACKLOG.md` B-001…B-003** — PROP-035 §10 link tables; the `addressable-specs`
  budget row owed a scope clarification; the Go floor gating a fixture directory
  named `dirty`.

---

## Rulings still in force {#rulings}

- **A package does not yield to a consumer that simply does not comply.** Three
  routes and never a fourth: the package's own *statement* is wrong → it yields;
  the rule is sound and the host should keep it → host obligation; the host
  deliberately does otherwise → **the exception is written down host-side** and
  the fact is confirmed with the exception named. Softening the package is the
  *профанация* §0's mandate exists to prevent.
- **A finding that spans a package boundary is a release event** — closed by a
  published version and a re-vendor, never by an edit in one consumer.
- **A closure is an edit AND a re-judge.** An edit without a verdict leaves the
  cache saying `drift` about text that no longer drifts.
- **A re-judge that edits nothing produces no spec diff** and therefore needs no
  owner approval, even on the `sync-from-code` route. Only an edit does.
- **A phase files findings; it does not fix them.** `RULE-NO-SILENT-REPAIRS`.
- **The resume boundary exists so the owner can steer.** A pointer to a next
  step here or in the WAL is a candidate for a report, never authorisation.

---

## Repository map

- `crates/` — the Rust workspace: `vibe-core`, `vibe-cli`, `vibe-spec`
  (the PROP-035 preprocessor + linker), `vibe-workspace` (boot artifacts),
  `xtask` (`mirror`, `conform`, `sync-engines`).
- `spec/` — `spec/boot/` the compiled boot lane; `spec/common/` and
  `spec/modules/` PROP/FEAT; `spec/terraforms/` campaign plans; `spec/WAL.md`.
- `packages/org.vibevm.*/` — the shipped packages; `world/` and `ai-native/` are
  what this campaign judges.
- `discipline/`, `terraform/` — **the host's adoption artefacts. Read §3.7
  before concluding anything is missing.**
- `campaigns/packages-2026-09/` — `tasks/` the instruments, `harvest/` the
  evidence, `run/cache.json` the verdicts, `run/state/` the registry and routing.

## Quick start

```bash
tools/self-check.sh
```

```bash
cargo xtask mirror
```

`mirror` is the sanctioned push — GitVerse and GitHub, fast-forward only, never
`--force`. *(Note: `CLAUDE.md:191` says «push to origin/main» instead, and
`spec/boot/90-user.md` both forbids and prescribes that at `:13` and `:34`.
That contradiction is registered as F-331 and is an open owner question.)*

---

## Recent commits

```
f488e711 fix(core-ai-native): a package-scoped search reads every successful adoption as an absence
23938ce4 fix(ai-native): a quarter of the claimed absences did not survive re-verification
4206c61b docs(campaign): waves 2-4 in the LOG, and the ratio that redefines the phase
270f1dc2 fix(packages): the prose-edit tail, and a demotion the host had already disproved
8b7f240f docs(campaign): what the host owes — the other half of the exit gate
f7a4b3cf fix(core-ai-native): three appendix claims the appendix can check against itself
ce5f8d8a fix(world): a Phase C verdict family that read a taxonomy as a priority chain
820622b8 fix(world): three sentences falsified by a sibling package, not by the consumer
d7803b97 feat(campaign): the routing record, without which Phase D cannot converge
89ff6aa4 fix(campaign): five citations pointing at obligation ids that name nothing
42ec3938 fix(campaign): a registry that forgets what it closed cannot evidence convergence
2913d238 fix(world): nine sentences a package can falsify without leaving its own tree
b1d46cec fix(campaign): thirteen verdicts, and the id-carry rule that a partial closure broke
710f48d0 revert(ai-native): 24 diffs that my own §1.2 says the owner approves, taken back
```

`git log --oneline -40` for the rest.

---

**`spec/WAL.md` is the canonical living state.** Where it and this file
disagree, the WAL wins and this file is stale.
