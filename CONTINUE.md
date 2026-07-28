# CONTINUE — cold-resume checkpoint

_Written 2026-07-28 (**Phase B CLOSED at zero; both queued wave-level DRIFTs
landed**). `spec/WAL.md` is the canonical living state and supersedes this
snapshot wherever they diverge._

## TL;DR

**`progress check --exhaustive` exits 0 over all 259 files.** Every paragraph,
list item and non-empty table body cell in the living corpus carries a stage, a
state and an address. Sixteen batches; 4 276 unmarked facts on the morning of
2026-07-27, none now.

**Both wave-level DRIFTs landed** on the owner's approval under sync-from-code:
F-097 (four dead package names) and F-103 + F-110 (five boot snippets that could
not resolve their own links).

**Nothing is running. Nothing is blocked.** The next phase is **Phase C**, its
entry gate is met, and it was deliberately not started — see
[`campaigns/packages-2026-09/PHASE-C-KICKOFF.md`](campaigns/packages-2026-09/PHASE-C-KICKOFF.md),
which carries a paste-ready prompt for a fresh session.

## Where the numbers are {#numbers}

| | |
|---|---|
| corpus | **259 files** (58 host + 201 packages) |
| unmarked | **0** — gate exits 0 |
| markers | **12 797** (host 4 988 · ai-native 2 993 · world 4 816) |
| markers with verdicts | **921** (`baseline.json`) — so Phase C owes ~11 900 |
| batches | B1, B2, B5–B16 — **all done** |
| rulings | **61**, three struck (18, 19, 45) |
| findings this session | F-102 … F-116 |

**Never decrement these; re-measure.** Every figure came from a command.

## The one thing to do first

**Nothing, until the owner picks.** The honest state is that Phase B finished
and every remaining item either needs an owner decision or is explicitly
off-limits. If the answer is "continue the campaign", paste the prompt from
`PHASE-C-KICKOFF.md` into a fresh session.

## What Phase B's last three batches produced {#phase-b}

| | B14 | B15 | B16 |
|---|---|---|---|
| files | 16 | 17 | 10 |
| units | 479 | 409 | 281 |
| predicted | 474–494 ✓ | 419–438 ✗ | 285–293 |
| coefficient | 1.089 | 1.025 | 1.021 |
| residual | 0 | 0 | 0 |

### The sizing rule stopped being a curve fit {#sizing}

```
predicted units ≈ 1.07–1.15 × terminators + pre-existing items + table cells
```

**The quantity is TERMINATORS UNDER A RECORDED REGEX, not sentences** — the rule
is in `BATCH-PLAN.md` verbatim, the coefficient is fitted to a 17 % undercount
plus two artefacts (a quoted question and `(e.g. …)` both fire it), and
repairing the counter breaks every prediction unless every coefficient is
re-derived in the same commit.

**B15 falsified the band, and the replacement is an explanation rather than
another constant: the coefficient measures how aggressively a batch splits
colons.** The formula multiplies terminators and adds items and cells at one, so
only a manufactured list can exceed one unit per terminator. Measured on three
consecutive batches — 25 manufactured bullets → +23 units, 10 → +6, 4 → +2.
**B16 was predicted in advance to be insensitive to the coefficient and was.**

Eight points span 1.021–1.153. **Read a miss as a report about colon-splitting,
not about the files.**

## Non-obvious findings this session {#findings}

1. **F-102 — a fence matched by prefix, in three parsers.** A block opened with
   four backticks was closed by a three-backtick line inside it, so the parse
   inverted: shell commands became paragraphs the gate demanded markers for,
   and the prose between them became code it could not see. **The gate and the
   review tool agreed *because* they shared the defect** — the case the
   "keep the tool independent" argument does not cover. A third implementation
   found it. Fixed in `progress-core`, `batch_review` and `vibe-spec`.
2. **A probe that passes is evidence about the probe.** The first test written
   to demonstrate the `vibe-spec` half **passed against the broken code**: it
   asked whether a section had children, and a stray `#` heading is level 1 and
   therefore the root's. Only a negative control proved the test could fail.
3. **F-114 — a contract falsified by the manifest that implements it.**
   `redbook`'s README promises two projects on the same edition run
   byte-identical practice text; the manifest, three lines above the pins, says
   the cultural-extraction wave is "accumulated here in place" with the edition
   bump deferred. **Owner's call.**
4. **A path is not a name.** Only the *package* was renamed to `git-*`; the flow
   directory, the document and a `spec://` URI's segments keep the short name.
   One URI spells the dead string twice while being entirely correct.
5. **F-110 pointed at the wrong side of its own disagreement.** Filed as "the
   READMEs contradict their manifests"; the READMEs were right and the layout
   had never caught up. Moving five files made five README edits unnecessary.
6. **Counting-unit slips, not arithmetic.** Fifteen of twenty-four briefs
   carried a factual error found by the batch running them, and the last three
   were all units-of-measure: names against sites, headline against itemisation.
   **State the unit with every count.**

## Traps that cost real time {#traps}

1. **A Python `str.replace` with `\n` in the pattern silently no-ops** — this
   tree's working copy is CRLF (the blobs are LF), so a multi-line pattern never
   matches and the script reports success. Struck twice. Use an editor tool that
   errors, or anchor on one line.
2. **`run/cache.json` carries the C-phase verdicts.** Load-and-merge only.
3. **Every parsing `progress` subcommand writes the cache — `check` included**,
   and it looks read-only. Always pass `--campaign`. **Never point one at
   `campaigns/progress-2026-08`.**
4. **Never hand-write `verified_at`** — a future stamp makes the invalidation
   rule never fire, and it fails UNSAFE.
5. **Never `git add -A` while an executor is running.** Stage explicit paths.
6. **`grep -v '\.vibe'` deletes this repository's own packages.**
7. **A rustdoc fence in a doc comment is a doctest** — and a four-backtick
   wrapper is closed by a four-backtick line inside it. Wrap examples that quote
   fences in five.

## Decisions still in force {#decisions}

- **Hybrid markup (owner, 2026-07-26):** the executor marks a package; the
  reviewer reads every diff and owns sense-preserving splits, anchor names and
  `audience`. **The executor never touches root `spec/**`.**
- **Verdicts live in the cache, never in markup** (PROP-043 §7.1/§7.5).
- **Superseded version slots and frozen history are neither marked nor edited** —
  `redbook/v0.1.0` kept its four dead names deliberately.
- **A finding is reported, never fixed, by a markup batch.**
- **`spec/boot/` is the boot-snippet layout** — all 27 `world` packages now agree
  and `vibe init` scaffolds it.
- **Owner grant (2026-07-28):** work autonomously, including across batch
  boundaries; stop only for genuine semantic or architectural decisions.
  **sync-from-code still requires surfacing a draft before applying.**

## Recent commits {#commits}

```
17b1415b docs(wal): the two drifts landed, and one of them was smaller than filed
96383299 docs(campaign): two findings closed, and each was wrong about itself
7b0ec6aa fix(world): four package names that no reader could install
521bb6cd fix(world): a boot snippet resolves from its own package
053f1671 docs(wal): Phase B closed, and the instruments agreed once too often
fc731127 chore(campaign): the phase boundary's baseline, and a headline that lied
56172a8f chore(campaign): Phase B closes at zero, and the mechanism holds a third time
c09c2827 docs(campaign): a contract falsified by the manifest that implements it
c06ba73e docs(campaign): the one ruling whose obedience was a semantic edit
66b6f04b docs(world): the last ten files, and the corpus reaches zero
2378d394 docs(campaign): the last batch, and the one the rule cannot mis-size
a8752fbb docs(campaign): a citation that fails for its own author too
ad026076 chore(campaign): refresh the zone's measurement after B15
647159a8 docs(campaign): three numbers, three criteria, and only one of them mine
125940b4 docs(campaign): a same-file control bounds the widest colon ruling
d5c9ec08 docs(campaign): the coefficient measures the reviewer, not the corpus
545943ae docs(world): the git family becomes fact-addressable
23b593d7 docs(campaign): the batch that has to tell a path from a name
a7e769cb docs(campaign): the broken-link defect is the trait, not an accident
1fcf8329 feat(batch-review): a licensed repair stops reading as a rewording
742f72ac chore(campaign): refresh the zone's measurement after B14
d679c273 docs(campaign): a sixth point, and the band stops being news
8218ec13 docs(campaign): the counting rule gains the two clauses it was missing
5464b679 docs(campaign): five findings kept, one dismissed on checking
a7ef4322 docs(campaign): a count decides what an argument could not
```

## Quick start {#quickstart}

```bash
bash tools/self-check.sh ; echo "EXIT=$?"
```

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --no-cache --campaign campaigns/packages-2026-09
```

```bash
cargo xtask batch-review --selftest
```

```bash
cargo xtask mirror
```

Always pass `--campaign`. Use `--no-cache` after any parser change.

## Repository map {#map}

- `spec/` — living corpus (0 unmarked) + `WAL.md` + both campaign plans in
  `spec/terraforms/`.
- `packages/` — wave 2's subject, now fully marked. `org.vibevm.fractality` and
  `org.vibevm.vibeapp` are out of scope.
- `campaigns/packages-2026-09/` — **live.** `BATCH-PLAN.md`, `baseline.json`,
  **`PHASE-C-KICKOFF.md`**, `PHASE-T-*`, `PHASE-G-SPEC.md`, `run/`, and `tasks/`
  (61 rulings live in `MARKUP-B1.md`; DRIFT-038/039 record the two fixes).
- `crates/progress-core/src/parse/` — the gate's parser; `delimiters.rs` is the
  run-matched delimiter cell F-102 created.
- `xtask/src/batch_review/` — the review tool: `text.rs`, `fences.rs`,
  `checks.rs`, `gate.rs`, `index.rs`, `refs.rs`, `report.rs`.
- `BACKLOG.md` — P1/P2/P3, drained by the next wave.

**The WAL is the canonical living state.** If this file and `spec/WAL.md`
disagree, the WAL wins.
