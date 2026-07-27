# CONTINUE — cold-resume checkpoint

_Written 2026-07-27 (session end: **Phase B is nine batches further on; B13
landed and the corpus is at 870**). `spec/WAL.md` is the canonical living state
and supersedes this snapshot wherever they diverge._

## TL;DR

**Phase B has run B6 → B13 plus DRIFT-037 in one session.** The corpus went
**4 276 → 870 unmarked**, and the last six batches all finished at **zero
residual**. Three batches remain: **B14, B15, B16**.

**The mechanical half of batch review is now a tool**, `cargo xtask
batch-review`, with 33 hermetic controls that the floor runs on every commit.
Fourteen checks; three of them surface a queue for judgement rather than
passing verdict; the output ends with the list of what it did **not** check.

**The single most useful habit this session confirmed:** every number in a
brief comes from a command, and every brief still gets checked by the batch that
runs it. Fifteen of twenty-one briefs contained a factual error found that way —
including one that proved the plan's own sizing rule wrong.

## The one thing to do first

**Batch B14** — `sync-from-code` + `licensing` + `manual-tests`, 16 files.
Everything it needs exists. Measure, write `MARKUP-B14.md` beside B13's,
dispatch to the `opus5` agent, review with the tool, commit, push.

Read first: `campaigns/packages-2026-09/tasks/MARKUP-B1.md` — its LOCKED
sections carry **52 rulings** that bind every batch. Two are struck (18, 19).

## Where the numbers are {#numbers}

| | |
|---|---|
| corpus unmarked | **870** (was 4 276 at session start) |
| host corpus | **0** — wave 1 stays measured |
| batches done | B1, B2, B5–B13 |
| batches left | **B14, B15, B16** |
| rulings locked | **52**, two struck |
| findings this session | F-092 … F-101 |

**Never decrement these; re-measure.** Every figure above came from
`progress check --exhaustive`, never from subtraction.

## The exact recipe for one batch {#recipe}

This ran eight times without variation. Each step is a command.

```bash
# 1. measure the next batch from the live gate log
grep -aE '^packages/org.vibevm.world/(pkg-a|pkg-b|pkg-c)/' <gate.log> | cut -d: -f1 | sort | uniq -c
grep -aE '^packages/…' <gate.log> | grep -oE 'Cell unit|Para unit|Item unit' | sort | uniq -c

# 2. write campaigns/packages-2026-09/tasks/MARKUP-B14.md, then dispatch to opus5

# 3. when it returns — gate first, tool second, judgement third
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
git diff --name-only | sort > /tmp/scope.txt
cargo xtask batch-review --gate-log <gate.log> --scope /tmp/scope.txt \
  --expect-unmarked 0 --expect-total <n> --campaign campaigns/packages-2026-09

# 4. commit markup, then ledgers, then run/ state — separately
bash tools/self-check.sh ; echo "EXIT=$?"
cargo xtask mirror
```

**Order matters in step 3.** The gate is truth; the tool is a second opinion
that has been wrong four times; judgement is what neither provides.

## Sizing — read the warning before the number {#sizing}

```
predicted units ≈ 1.07–1.15 × terminators + pre-existing items + table cells
```

**The quantity is TERMINATORS UNDER A REGEX, not sentences**, and the rule is in
`BATCH-PLAN.md` verbatim. B13 measured the gap: 274 where a true sentence count
is ≈320, a structural +17 %. **The coefficient is fitted to that undercount** —
repairing the counter toward its name would break every prediction built on it,
and correcting it requires re-deriving every coefficient in the same commit.

The counting rule reproduces across three independent implementations. Five
measured points span 1.068–1.153. **Do not lock a point value**; this rule has
been wrong three times and each version was locked on too few measurements.

## Traps that cost real time this session {#traps}

1. **`git add -A` while an executor is running** swept 13 files of an in-flight
   batch into a bookkeeping commit. Stage explicit paths; read `git status`
   before every commit while a batch is out. Recovery is
   `git reset --mixed HEAD~1` while unpushed.
2. **`grep -v '\.vibe'` deletes this repository's own packages** — the namespace
   is `org.vibevm`. Anchor filters on a path segment, never a substring.
   PowerShell's `-match` is case-insensitive and will compound it.
3. **A `str.replace` that does not match is a silent no-op.** It reported
   success and changed nothing; only `git diff --cached --stat` caught it. Use
   an editor tool that errors, or assert before writing.
4. **Two regexes disagreed by 35 % over the same files** and nobody noticed
   until someone tried to use the number. A measurement is only as portable as
   its written definition.
5. **Do not correct a plan and explain the correction in the same commit** — the
   explanation then describes a tree no reader ever saw. Correct first, cite
   after.

## Open findings {#open}

| id | what | whose |
|---|---|---|
| **F-097** | four dead package names (`atomic-commits`, `attribution-policy`, `conventional-commits`, `autonomy`), **21 files, 33 refs, 6 unusable `vibe install` lines** | needs one wave-level DRIFT under sync-from-code |
| **F-093 / F-094** | the Rust stack's wiring recipe fails when followed; its README carries four wrong paths and cites a retracted rule | drift stage |
| **F-098 / F-101** | a promise whose "next release" shipped without it; a template disagreeing with its own worked example | drift stage |
| **F-099 / F-100** | a README miscounting its own contents; an example citing a real package at a version it never had | drift stage |
| **F-087 / F-088** | commit bodies naming a model; `ATLAS.md` declaring a generator tracked nowhere | **owner** |
| **F-078** | boot-lane duplication — DRIFT-035 written and deliberately not dispatched | **owner** |
| PROP-043 §2 | the spec names what a unit **is** and never what structure **is**; two DRIFTs have moved that boundary in code | **owner** |

## Phases T and G — designed, not run {#designed}

Both have full specs. **Phase T was rewritten this session** for GLM sessions and
their sub-agents: everything derivable moves into the packet, everything
stateful moves to the boss, and the packet is the unit so the design does not
depend on whether the harness offers sub-agents. It gained a prerequisite it
lacked — one calibration packet against a real GLM session, whose deliverable is
the corrected packet template rather than tests.

**Do not start either without an explicit instruction.**

## Quick start

```bash
bash tools/self-check.sh ; echo "EXIT=$?"
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
cargo xtask batch-review --selftest
cargo xtask mirror
```

Always pass `--campaign`. Use `--no-cache` after any parser change. **Never
point a progress command at `campaigns/progress-2026-08`** — it is archival.

## Repository map

- `spec/` — living corpus (0 unmarked) + `WAL.md` + both campaign plans in
  `spec/terraforms/`.
- `packages/` — wave 2's subject. `org.vibevm.fractality` and
  `org.vibevm.vibeapp` are out of scope.
- `campaigns/packages-2026-09/` — **live.** `BATCH-PLAN.md`, `tasks/` (52
  rulings live in `MARKUP-B1.md`), `PHASE-T-*`, `PHASE-G-SPEC.md`, `run/`.
- `xtask/src/batch_review/` — the review tool: `text.rs`, `checks.rs`,
  `index.rs`, `refs.rs`, `report.rs`.
- `BACKLOG.md` — P1/P2/P3, drained by the next wave.

**The WAL is the canonical living state.** If this file and `spec/WAL.md`
disagree, the WAL wins.
