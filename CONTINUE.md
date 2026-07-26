# CONTINUE — cold-resume checkpoint

_Written 2026-07-26 (session end: **wave 1 closed out to zero drift, wave 2
ratified and opened, seven DRIFT tasks executed**). `spec/WAL.md` is the
canonical living state and supersedes this snapshot wherever they diverge._

## TL;DR

**Wave 1 (host `spec/`) is closed out and the ledger reads zero drift.** Its
last row — `FACT-GRAIN-EVIDENCE`, believed to need a package re-mint — closed
because the blocker turned out to be **a caret**, not a release.

**Wave 2 (`packages/`) is ratified and Phase A is done.** Scope widened, zone
seeded, pilot run. **Phase B has not started**: 16 batches, 217 package files,
6 555 unmarked facts. That is the next real work.

**Ledger: 4 495 confirmed / 0 drift / 3 unverifiable over 275 files.** Floor
green, `progress check` 0, `conform` 0, specmap ratchet 37, tree clean,
`github/main` = `3dbedba0` and **fully pushed**.

## The one thing to do first

Open **Phase B batch B1** (`core-ai-native`, live slot: 27 files, 1 487 facts).
The plan is `campaigns/packages-2026-09/BATCH-PLAN.md` — read it before B1, not
after.

**Before B1, run `vibe update` once.** It is the owner's stated
re-materialisation path and it has **never been exercised in this campaign**.
Sixteen batches should not stand on an unverified assumption.

## Executor law, as decided this session

Markup is **hybrid** (owner ruling): opus marks per package, **Fable reviews
every diff** and keeps sense-preserving splits, anchor naming and `audience`.
That is a change from plan §2, which made all markup Fable's — taken because
wave 2 is 5× wave 1's file count.

Everything else stands: no fractality; `claude-opus-5` for DRIFT tasks;
`reality-mismatch` closes only via sync-from-code with owner approval; **the
executor never edits `spec/**` — the reviewer lands the spec side and seals the
verdicts.**

## Where the two waves stand

| | |
|---|---|
| **Wave 1** `campaigns/progress-2026-08/` | **closed out.** `baseline.json` (920 units) intact. `run/` is **archival** — see the trap below. |
| **Wave 2** `campaigns/packages-2026-09/` | **live zone.** Ratified with all six §4.5 amendments; Phase A closed; Phase B not started. |

Phases **F and G were deferred from wave 1 into wave 2** — not for time. F's
three views are empty (`freeze/plan` 0, `action="rework"` 0, `stage="idea"` 0)
because Phase B records what facts *are* and was never asked what should
*happen* to them. G's `harvest/` is empty because Phase C skipped its own step.
Wave 2 must carry a judgment-marking pass before F, and a harvest pass before G.

## Traps that cost real time today — read these

1. **Every parsing `vibe progress` subcommand writes the cache — `check`
   included, and `check` looks read-only.** The observed scope is always
   global; `--campaign` only chooses where state is written. Pointing one at
   the archival wave-1 zone drags 250 package files into it. **This happened
   twice, and the second one got committed** before it was caught.
   **Never point a progress command at `campaigns/progress-2026-08`.**
2. **With two campaign zones present, a bare `vibe progress` resolves no
   campaign at all** and silently stops writing state. Always pass
   `--campaign campaigns/packages-2026-09`.
3. **Never hand-write a timestamp into campaign state.** Two hand-written
   `verified_at` values were 2 and 8.5 hours in the *future*, which suppresses
   invalidation rule 2 entirely — a future verdict is never older than a
   commit. It fails **unsafe**. Use `vibe progress seal`.
4. **Do not run a `vibe` command while `tools/self-check.sh` is running** — the
   floor snapshots the real `~/.vibe` and campaign commands write into it.
5. **Derived projections go stale and nothing notices.** Three did today:
   `tasks.json` (18 tasks behind), `campaign.summary` (claimed a drift row that
   was closed), and both findings ledgers (9 findings still marked open).
   All three were hand-maintained fields that no gate compares. **Recompute
   before trusting any count.**

## Open findings — three, all in wave 2

- **F-069** — the aggregator grammar gap. An umbrella's facts are *about other
  packages*, so the document cannot be their source of truth. **This is Phase
  C's problem, not Phase B's** — a marker records stage/state, and
  source-of-truth is a question about the *verdict*. Do not stall B on it.
- **F-075** — `seal`'s refusal is a **coverage** gate, not a **recency** one.
  Verdict entries carry only `v` and `ev`, with one date per file, so "every
  marker has a verdict" is checkable and "every verdict is fresh" is not. A
  session that seals after re-deriving one anchor of three hundred will be
  believed. Owner's call: accept, add a per-anchor date, or require an explicit
  flag.
- **F-077** — `campaign.summary` and the verdict maps are two counts of the
  same thing that can disagree. Recomputed once; nothing keeps them in sync.
  Preferred fix: delete the derived field and compute on read.

## What the corpus is actually made of

Four rounds of subtraction got Phase B from a guess to a measurement, and
**none of them were found by estimating size** — all by asking what the corpus
contains:

| removed | files | facts | why |
|---|---|---|---|
| `vibedeps/`, `.vibe/cache`, `refs`, `fixtures` | 970 | — | machine copies, ~71 % of all markdown under `packages/` |
| `LICENSE.md` × 33 | 33 | 264 | verbatim third-party text |
| three derived indexes | 3 | 265 | "hand edits are a defect", their own words |
| superseded slots | 33 | 1 908 | frozen history; owner: obsolete versions are not worked on |

Left: **217 package files, 6 555 facts**, plus the host's 58 files at 0 unmarked.

## Non-obvious findings from this session

- **The long-lead item was a caret.** All three `-lang` stacks required
  `core-ai-native ^0.7` while current was 0.8.0 — and on a 0.x version that
  caret *excludes* 0.8.0. The lockfile pinned 0.7.0 because nothing was allowed
  to resolve higher. Fixed in place; no re-mint, no publication.
- **Fixing it moved specmap from 1 041 to 5 267 spec units and fact-targeting
  edges from 0 to 65** — and dropped unresolved edges from 77 to 12, because
  65 of those "dangling" edges were correct code tags the unit-grain engine
  could not see.
- **Two of my own task premises were disproved by executors measuring rather
  than agreeing** — that PROP-043 could not be sealed (it can), and that ids
  are unique corpus-wide (they are per-file; 316 names live in more than one
  file, `root` in 168). Both stop rules fired correctly.
- **The name registry vibevm-project asked for already exists.** Corpus-wide
  uniqueness was never intended; the globally unique name is `path#anchor` —
  the `spec://` URI — and `specmap.json` publishes 5 267 of them.
- **A phrase sweep is not an audit.** Searching for "generated"/"do not edit"
  returned 11 files and *all eleven were false positives* — prose *about*
  generated code. The three real derived files used wording the sweep never
  looked for.

## Repository map (top level)

- `spec/` — the living corpus (58 files, 0 unmarked) + `WAL.md` + both campaign
  plans in `spec/terraforms/`.
- `packages/` — wave 2's subject: `org.vibevm.world` (27) +
  `org.vibevm.ai-native` (10). `org.vibevm.fractality` is a separate specspace,
  out of scope by the include globs rather than by an exclusion.
- `crates/` — the workspace (18 crates; `vibe-test-support` is new and
  test-only).
- `campaigns/progress-2026-08/` — **archival.** Baseline intact; do not scan.
- `campaigns/packages-2026-09/` — **live.** `BATCH-PLAN.md`, `deferrals.md`,
  `tasks/` (DRIFT-024…028), `run/`.
- `tools/self-check.sh` — the floor, now carrying the user-home tripwire.

## Quick start

```bash
bash tools/self-check.sh
cargo run -q -p vibe-cli --bin vibe -- progress scan  --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress check --campaign campaigns/packages-2026-09
cargo xtask conform check
```

Ledger read-out (the authoritative one — verdict maps, not `summary`):

```bash
python -c "import json,io,collections; c=json.load(io.open('campaigns/packages-2026-09/run/cache.json',encoding='utf-8')); t=collections.Counter(v['v'] for r in c['files'].values() for v in ((r.get('campaign') or {}).get('verdicts') or {}).values()); print(dict(t))"
```

## Recent commits

```
3dbedba0 feat(progress-core): a unit can be void, and it stops counting as work
4dbe1987 fix(vibe-core): one config home, the way there is one credential home
e9fc7b44 feat(progress): sealing a verdict stops depending on memory
f1fd586b docs(boot): NOTOUCH moves up beside the rules it belongs with
8d673303 docs(wal): withdraw F-074, and park the 90-user scope question
1ea4815c docs(boot): the NOTOUCH list keeps one entry, and F-063 closes because of it
c1e2a0ff refactor(vibe-cli): the progress adapter splits before it is forced to
fc1782d8 feat(campaign): one live zone — wave 2 takes the host corpus's verdicts
bed47ad0 docs(campaign): audit the observed scope for generated content, and find three
b3ada517 docs(packages): the pilot marks the aggregator, and finds licence boilerplate
269273ed docs(wal): wave 2 is open, and the caret was the long-lead item
07a38e1a feat(specmap): the fact-grain engine lands, and wave 1's last drift row closes
0aa4ba01 feat(packages): the family moves to core-ai-native 0.8.0, caret and all
27336263 docs(campaign): the engine re-mint is deferred, and Phase C waits on it
3c87cd11 docs(campaign): the pilot confirms prediction 2 and finds drift in nineteen lines
30728dd7 feat(campaign): wave 2 opens — the packages join the observed tree
6ad264da docs(campaign): wave 2 is ratified, and the amendments change the plan
d3482dd7 docs(campaign): wave 1 gets the artefact that makes the next run cheap
812bfecc docs(spec): the command list was two verbs short of the tool
db7186ef feat(progress): the baseline gets a writer, and the recurrence becomes runnable
```

## Parked on the owner

- **`90-user.md` mixes project and machine scope** in a public repo (it names a
  workstation host). Raised, **owner parked it** — "оставь пока". Do not tidy
  unasked.
- **`-lang` slots carry 0.8.0 engines under 0.7.0 version numbers.** Works and
  is green; semantically muddy. Owner: obsolete versions are not worked on and
  versions are not bumped per change.
- **MT-02 / MT-03** await a human sign-off; an agent may pre-run.
- **GitVerse SSH has been down since 2026-07-25** — banner-exchange timeout,
  network-level. GitHub carries everything. Recovery: plain
  `cargo xtask mirror`; **never** `--force`.

A pointer worth repeating: **the WAL is the canonical living state.** If this
file and `spec/WAL.md` disagree, the WAL wins.
