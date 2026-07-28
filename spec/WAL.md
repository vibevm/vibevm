# WAL — Project Continuation State

_Updated: 2026-07-28 (**Phase C — W2 and W3 both CLOSED; the cluster is 51 of
121 files and the phase is past 61 %**)_

## Current phase

**Progress Control (PROP-043) — wave 2, `packages-2026-09`. Phase C, mid-flight.**
Live zone `campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is
**archival**.

**4 222 verdicts written, sealed and committed — 61.7 % of the phase.** The
`ai-native` cluster measures **2 470 / 207 / 20 — 91.6 %**; `world` now has
**1 557 / 140 / 17 — 90.8 %** over 51 of its 121 files, with 138 self-referential
(8.1 %). One command prints all of it:
`python campaigns/packages-2026-09/tasks/summary.py`.

**W1, W2 and W3 are CLOSED — 1 714 anchors judged row by row.** W1 407 (90.4 %),
W2 692 (91.9 %), W3 615 (**89.9 %, and zero unverifiable** — the first batch
where every fact could be settled against the tree). Six packages complete:
`flow:wal` 86.5 %, `two-process-model` 96.6 %, `wal-specspaces` 93.5 %,
`sync-from-code` 93.7 %, `decision-records` 92.9 %, `conflict-protocol` 93.6 %,
`addressable-specs` 87.9 %.

**The per-file slice replaced the batch as the unit of work, and that closes the
split question §4 left open.** Seventeen slices landed here, one file each, 17 to
149 rows, each merged and sealed on its own — `merge-verdicts.py` takes a subset
of a batch's files under the same id, so a slice that lands cannot become a debt.
**W5 does not need splitting; it needs twenty-one slices.**

**The prediction is settled and it inverted.** §5-C said `world` would measure
higher than `ai-native`. Over 1 714 anchors it reads **90.8 % against 91.6 %**,
and the reason is the opposite of the one predicted: these flows make claims
about the consuming project, and this consumer is measurable to the line.

**What the three closed batches measured, by family:** the boot lane is **~16 100
tokens against its own 500-token budget** (32×) and «split when over» has fired
zero times; **4 of 153** decision-bearing sections carry all four required fields
and **127** carry the Decision line alone; the prescribed sync subject
`docs(spec): sync …` has been typed **0 times in 2 041 commits**; 857 of 982
headings are anchored and the 125 that are not are **all 23 in `spec/boot/` and
all 8 in this file**; and **59 `duplicate-anchor` warnings** say `{#root}` means
27 things inside the generated boot lane.

**Three internal contradictions were found inside single packages** — the `wal`
package's two incompatible wind-downs (F-129), `record-template` against
`revisit-triggers` on whether a trigger fires unprompted, and
`cognitive-load-split` against the wal package on whether one text serves three
readers. **And one collision of principle**: `uncertainty-protocol` prefers no new
dependency; `PROP-000` §15 decides the opposite at the governing anchor, in the
four-field form.

**Exit-gate clause (iii) is satisfied** — 39 captured runs under `harvest/`, plus
three per-batch source captures. **Clauses (ii) and (iv) have a producer**:
`tasks/summary.py`.

Nothing is blocked. The resume prompt is `CONTINUE.md` §prompt.

## Constraints — do not violate

- **The verdict standard, written down 2026-07-28 after it was applied to 690 rows
  without being stated.** A fact that **PRESCRIBES** what the discipline requires —
  an intent, a participants list, a detector seed, a goal, a tradeoff, an
  alternative, a risk, a routine step — is **confirmed** when it is coherent and
  every referent it names resolves, *including a referent the package itself
  declares as future work* (a card registry's `specified` column, a brief's «vision,
  NOT an implementation plan» line). A fact that **DESCRIBES** what this repository
  already ships or does is **checked against the tree**, and a description that does
  not match is **drift**. A fact whose subject cannot be exercised here is
  **unverifiable, in its own words** — never by a blanket rule over a filename.
  **For `world` there is a fourth clause:** a flow's fact is also checked against
  the host's observed conformance (§3.1 source 2). The host is a living consumer,
  and a law the host breaks is measured, not excused.
- **Non-adoption is not drift (the line W2/W3 ran on, and the rest of `world`
  runs on it).** A flow's prescription the host simply never adopted is
  **confirmed** — a human's morning read leaves no repository artefact and no flow
  claims the host performs one. **Drift is where the host's own written contract
  contradicts the flow**, or where a measurable rule is broken over a double-digit
  share of its window. Two corollaries: **each fact is judged on its own sentence,
  never on its family** (`NEVER-APPEND-TO-THE-WAL` prohibits appending only —
  confirmed; `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` names patching too —
  drift); and **a definition that classifies a failure correctly is confirmed BY
  that failure**, not refuted by it.
- **Never classify a group of rows by one rule to save time.** That is how 138 rows
  became a debt. Every `partial` row is read individually; a `partial` is *related
  code that does not settle the claim*, which is exactly the class that carries
  drift.
- **A wrong REASON in the record is worse than a wrong verdict** — the next reader
  acts on it, and the verdict at least pointed at a real defect. When restating,
  restate the reason too, and say which way the correction runs.
- **The perimeter law (and it cost five misses).** A mechanism's SPEC lives in
  `core-ai-native`, its ENGINE in that package's library crates, its DRIVER in each
  language stack's CLI, and its DEPLOYMENT in a consuming project —
  `research/{rust,ts,go}-demo/`, which carry `conform.toml`, `specmap.toml`,
  `specmap.json` and `discipline/registry/`. A fact can be true at one layer and
  invisible at the other three. **A `not-found` is a fact about the search perimeter
  until the perimeter has been checked.** Full path list: batch plan §4.5.
- **A count that includes `node_modules/`, `.vibe/cache/` or `vibedeps/` is a count
  of somebody else's code.** Ten TypeScript verdicts were confirmed on such a count
  and had to be restated to drift.
- **An absence you assert is not an absence you checked.** Fifteen Go verdicts were
  recorded `unverifiable` on «there is no `research/go-demo`». There is, and it is a
  complete consumer.
- **Delegation goes to the harness's built-in `opus5` subagents, not fractality**
  (owner ruling 2026-07-28). The verdict is never delegated; neither is the review of
  delegated output.
- **Cache campaign maps are load-bearing.** `run/cache.json` carries every C-phase
  verdict; mutate it by **load-and-merge only** — a from-scratch rewrite erases the
  maps and there is no second copy.
- **Never hand-write a timestamp into campaign state.** `verified_at` and
  `processed_hash` are written by `vibe progress seal` and by nothing else; a
  hand-written stamp fails UNSAFE, because `moved_crate` calls a crate moved when its
  commits are *newer* than the verdict.
- **EVERY parsing `vibe progress` subcommand writes the cache — `check` included**,
  and `check` looks read-only. Always pass `--campaign`. **Never point one at
  `campaigns/progress-2026-08`.**
- **With two campaign zones, a bare `vibe progress` writes no state** —
  `resolve_campaign` returns a zone only when exactly one exists. Always pass
  `--campaign`.
- **Do not run a real `vibe` command while `tools/self-check.sh` is running.** The
  floor snapshots the real `~/.vibe` and a concurrent write turns it red.
- **Never `git add -A` (or `git add .`) while a worker is running.** Stage explicit
  paths; read `git status --short` before every commit while a batch is out.
- **A wind-down invalidates any sealed evidence table that cites `CONTINUE.md` or
  `spec/WAL.md`.** Proved 2026-07-28: W2's four tables verified at 3 unresolvable
  when sealed and at **65** one session later, because the wind-down overwrote both
  files. None was a fiction. **Always re-run `verify-evidence.py` before reading a
  table, even one a previous session already verified**, and run
  `repair-refs.py --apply` — it re-points a moved coordinate and refuses to guess
  when the quote occurs twice.
- **`grep -v '\.vibe'` deletes this repository's own packages** — the org namespace is
  literally `org.vibevm`. Anchor such filters on a path segment. Related: PowerShell
  `-match` is case-INSENSITIVE.
- **A Python `str.replace` with `\n` in the pattern silently no-ops on this tree** —
  the working copy is CRLF, the blobs are LF. Use an editor tool that errors on a
  missed match, or anchor on a single line.
- **Never trust a substring match about a data file.** Walk the structure or anchor
  on bytes.
- **Boot pair marking.** `spec/boot/00-core.md` / `90-user.md` carry the owner's own
  machine facts and preferences: mark ADDITIVELY and prefer not to re-form their
  prose. `refs/book/` is the one remaining NOTOUCH entry.
- **`spec/boot/90-user.md` mixes project and machine scope, deliberately for now** —
  owner parked it 2026-07-26: «оставь пока». Do not tidy it unasked.
- **legacy-spec/ is an archive.** Nothing in the living corpus or crates may cite
  into it as a normative source.
- **The parse payload lives outside the repository** since 2026-07-26:
  `~/.vibe/progress-cache/<repo-id>/<branch-slug>/<campaign>/`. Pure acceleration;
  never put a verdict there.
- **Commit delegated work on the completion notification**, never on a filled-in task
  journal.
- **mtime unit in the vvm manifest.** TS port stores `mtime_ms`; the Rust twin stores
  `mtime_nanos` (PROP-019 §2.15).
- **electron-packager temp cache.** Concurrent `<product> self install` runs race on
  the shared tmpdir template rename — run sequentially.
- **CI-off gate split.** `CI` / `VIBE_NO_DEFAULT_REGISTRY` suppresses vibe-embedded
  but NOT project-local (PROP-030 §5 + §3.3).
- **conform R-001 gate.** `crates/vibe-cli/src/registry.rs` is the only sanctioned
  constructor site for embedded/local-composite providers.
- **Outstanding manual runs (owner sign-off pending):** MT-02 (`vibe tree` TUI) and
  MT-03 (`vibe prefs ui`). An agent may pre-run; only a person signs off.

## Done (collapsed — see `git log`)

- **Phase C, the reviewing debt — closed 2026-07-28, 5 commits.** 138 rows read
  individually; the cluster restated 92.4 % → 91.6 %; four cross-language
  inconsistencies resolved to one verdict each; two verdicts restated upward and one
  reason corrected. Five findings opened (F-124…F-128). Two tools added:
  `show-rows.py` (the reviewer's join) and `summary.py` (the exit gate's two
  counts, its `src` arithmetic exercised against a fixture before use).
- **Phase C, the `ai-native` cluster — closed 2026-07-28, 31 commits.** Six batches,
  2 697 verdicts, six campaign tools built with their refusals tested before use, 39
  captured runs, 15 delegated evidence tables persisted into the zone. Seven findings
  opened (F-117…F-123).
- **Phase B — closed 2026-07-27/28 at zero.** `progress check --exhaustive` exits 0
  over all 259 files; 4 276 unmarked facts on the morning of 07-27, none left.
  Sixteen batches, 61 rulings locked, three struck.
- **Wave 1 (`progress-2026-08`) — Phases B, L, C, D, E and close-out**, 93.0 %
  measured at its C gate, 99.8 % after stitching. Its durable artefact is
  `baseline.json` (921 units).
- Earlier: M1.17 / M1.18 / M1.19.

## In progress

**W4 is the next batch and nothing is in flight.** No worker is running; no table
is half-read. W1, W2 and W3 are closed and sealed, and every slice of them is
committed and mirrored.

**W4 — `campaign-plans`, `discovery-prompt`, `comparative-research`, `redbook` —
is 15 files / 564 anchors, unstarted.** Its evidence has not been gathered: the
recipe's step 1 (capture the three §3.1 sources into `harvest/`) and step 2
(commission one `opus5` worker per package) both remain. W3's harvest is the
model — `harvest/world-w3-ipc-core-ii.md`.

## Next

1. **Open W4.** Capture its three §3.1 sources into
   `harvest/world-w4-plans-and-inquiry.md` (source-1 join over the four packages,
   the boot-join for source 3, and the host observations source 2 needs), then
   commission four `opus5` workers, one per package, pointing each at
   `tasks/WORLD-WORKER-BRIEF.md` and that harvest. **Tell them to avoid citing
   `CONTINUE.md` and `spec/WAL.md`** — that two-sentence change is what gave W3
   zero unresolvable refs on the first pass.
2. **Judge one file per slice.** Read the subject document in full, then
   `show-rows.py --brief --file <name>` row by row, then a batch of that file's
   verdicts alone, `merge-verdicts.py`, `progress seal`, commit. Seventeen slices
   closed W2 and W3 this way and none became a debt.
3. **Every `world` verdict carries `src`** (a non-empty subset of `[1,2,3]`, A2);
   `merge-verdicts.py` refuses a batch without it. `world`'s self-referential
   share so far is 138 of 1 714, 8.1 %.
4. **Then W5, W6, W7** — 1 872 anchors over 55 files. **W5 does not need
   splitting** (§4's open question, answered in the LOG): the per-file slice
   replaced the batch as the unit.
5. **At the phase close:** the X/Y/Z summary in the LOG, the self-referential
   count (both from `summary.py`), and `baseline.json` (amendment A6).
6. **Phases T and G are designed and unrun. Neither starts without an explicit
   instruction.**

## Known issues

- **Opened 2026-07-28 by W2b, on the owner, not blocking: F-129** — the `wal`
  package ships **two contradictory wind-downs**. `session-end-hook.md` orders «the
  full hook, steps 1-6» (stopping state, rewrite WAL, collapse, overwrite CONTINUE,
  propose commit, report); `cold-resume.md` §wind-down orders **five** in a
  different sequence (overwrite CONTINUE **first**, rewrite WAL, commit, push, chat
  TL;DR) with no stopping-state and no collapse step; and
  `WIND-DOWN-IS-THE-EXPLICIT-FORM-OF-THE-HOOK` asserts they are the same procedure.
  All three `@impl/done`, all three in one package. The host implements
  `cold-resume.md`'s five exactly. Closing it edits a published slot → F-122's
  territory.
- **Opened 2026-07-28 by the reviewing pass, all on the owner, none blocking:**
  **F-124** (three evidence ids — `H4`, `DR1-014`, `DL1-015` — cited across all
  three stacks and resolving in no register; the `H`-series is in daily use inside
  `core-ai-native`'s appendices with no roster anywhere); **F-125** (`core-ai-native`
  v0.8.0 publishes one PLDI'25 measurement twice — ATLAS 75.3 % / 70.2 %,
  CONTRADICTION-MAP 74.8 % — and four documents quote whichever they read; this
  corrected a C3a verdict's reason); **F-126** (`rust-ai-native-tcg` names both a
  shipped consultation oracle and an unbuilt token-level masker; three names in one
  family point elsewhere); **F-127** (the Go stack prescribes `go test -race` 15
  times across 5 documents and passes it 0 times); **F-128** (`spec/boot/INLINE.md`
  does not exist, and line 5 of `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` says the four
  non-negotiable commit rules load first and verbatim from it and are not restated
  elsewhere — `link = "inline"` occurs zero times in every `vibe.toml` in the tree).
- **Open on the owner, and none unblocks Phase C:** **F-120** (the kind-line
  notation — 102 uses, 8 ranks, defined by one example, cited to a
  `GUIDE-SPEC-AUTHORING` that is not in this repository); **F-121** (four mechanism
  documents each end with «any unexercised mechanism is removed», mark it
  `@impl/done`, and are contradicted by their own contents — nothing enforces any of
  them); **F-122** (one `name@version`, two contents, **173 files across 33
  packages** — Phase B marked inside published slots; closing it is a release event
  under §5-D); **F-123** (82 of the last 400 commit subjects exceed the 72-character
  hard limit, 20.5 %); plus the older **F-114**, **F-087 / F-088**, **F-078** and
  PROP-043 §2.
- **F-087 is now measured:** 4 model mentions in 400 commit bodies, **none an
  authorship claim** — two are a colour-theme name, two describe model tiers as
  configuration data.
- **F-117** — the Phase C kick-off documents a `summary` cache field that DRIFT-033
  deleted; a session following it literally would reinstate the defect the ruling
  removed.
- **F-118** — wave 2 ran sixteen batches with no `run/journal.jsonl`; opened at C,
  not back-filled.
- **F-119** — the book's chapter 1 cites `safeharbor.md`, which exists nowhere, in
  both `redbook` slots. Invisible to the gate because the campaign's `exclude` globs
  drop `redbook/*/spec/book/ru/`.
- **F-092** — `SKILL.md` YAML frontmatter cannot carry a fact anchor; 9 files.
- **F-069** — aggregator grammar.
- **`specmap` ratchet** — 37 gated orphans host-side, unmoved.
- **vibespecs 401 on this machine** — resolution goes through project-local
  `packages/` since `vibe update` repointed it.

## Session context

**Start at `campaigns/packages-2026-09/tasks/PHASE-C-BATCHES.json`, batch W4, and
open its harvest before anything else.** The recipe is `CONTINUE.md` §recipe; the
verdict standard is §Constraints above and `CONTINUE.md` §standard; what W1-W3
already settled is `CONTINUE.md` §judged, and reading it first is what keeps the
next batch from re-deriving four measurements it already has.

**The line that made three batches consistent is now a constraint, and it is the
one to read twice.** Non-adoption is not drift. `morning-routine.md` is unadopted
end to end and scores 39 of 42 confirmed; its two drifts are both a read order
the host reverses *in writing*. Judged the other way, one unadopted document
would have produced forty-two drifts and buried the two that matter.

**The instruments failed once more and the failure is now a standing trap.** A
wind-down invalidates any sealed evidence table citing `CONTINUE.md` or
`spec/WAL.md` — W2's tables went from 3 unresolvable to 65 that way, and this
session's own checkpoint then did it again to W2c and W2d. Not one of the 71
broken refs was a fiction. The fix is upstream, in the brief: W3's workers were
told to cite durable files instead, and returned 1 805 refs with **zero**
unresolvable.

**Two workers corrected the harvest that commissioned them, and one asserted an
absence it had not checked.** The REVIEW contract is in live use with shipped
machinery around it, against a harvest that called it unexercised from a
three-file grep. And «no host rule discouraging new dependencies exists» was
written against a `PROP-000` §15 that exists and rules the opposite way — the
campaign's own named trap, in the session that wrote the trap down. Both
corrections live in the verdicts' reasons, where the next reader meets them.

**And the phase measured its own repository harder than any batch before.** The
boot lane is 32× the budget its own installed flow sets. The two least
addressable files in the tree are the boot lane and this one — all 23 headings in
`spec/boot/` and all 8 here carry no anchor, so the Constraints section above
cannot be cited. `spec://` occurs zero times in this file, in every revision
measured, while the flow that requires it is installed and read at every boot.
None of it was fixed: Phase C files findings; it does not repair the subject it
is measuring.
