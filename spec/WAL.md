# WAL — Project Continuation State {#root}

_Updated: 2026-08-06, wind-down №17 (**the programme is executed as far as it
goes without the owner**: Б, В1–В3 closed earlier; А1, А2, А3, А4, А5a closed
this session; А6 and В4 stopped by measurements that change what the owner
decided, not by difficulty. **The corpus owes 0 unjudged and 0 orphaned facts**,
from 142 and 5. Panel green, 26 commits, mirrors rolled out by this wind-down.
Six errors this session, every one caught by an instrument.)_

@fact:WAL-READ-THE-PROMPT-FIRST **The next session's work and its order live in
`NEXT-SESSION-PROMPT.md` at the repository root**, written to the owner's
instruction: А5b → the TUI thinness audit → the P1 tail → the cheap rows → then
drain `BACKLOG.md` and refresh `TOOLING-MAP.md`. The programme file
(`spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md`) still governs
what remains of group А. @status:impl/done

@fact:WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by
commands; run them rather than quoting this file — and rescan first, or they
answer about the cache.** @status:impl/done

```bash
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/judging-debt.py
python campaigns/packages-2026-09/tasks/text-stability.py
python campaigns/packages-2026-09/tasks/drift-registry.py
```

## Current phase {#current-phase}

@fact:WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`: Phase E
under the EXTENDED mandate. Phase T NOT started.** The owner's programme of
2026-08-06 governs; groups Б and В (less В4) and А (less А6) are executed. @status:impl/done

@fact:WAL-STATE **State at checkpoint** (commands supersede): corpus **278 files,
0 unmarked facts**; verdicts **~12 065 at 98.2 %**, **63.0 % per-fact**; drift
**173**. Judging debt: **0 unjudged, 0 orphaned**, 42 stale. `main` clean,
`.wt/` empty and now gitignored, four worker reports archived **with `meta.md`**.
**Panel green**, `vibe check` clean. Backlog **21 live rows, 38 tombstones**. @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-A5B **А5b — the query language**, the last buildable programme
   item that needs no owner decision. Graph traversal on top of the filter
   level, never replacing it: the simple level is PERMANENT by the owner's
   ruling, and the library keeps it a separate entry point so a broken grammar
   cannot take it down. It introduces a grammar that will need versioning. @status:spec/plan
2. @fact:WAL-NEXT-TUI-AUDIT **The TUI thinness audit** — fully unblocked for the
   first time: the boundary is the owner's (finished = a thin surface, tested by
   deletion on paper) and the **perimeter is now written down and reproducible** —
   the two `tui/` trees, 63 files, 18 426 lines, 258 tests. It reproduces at
   exactly one of five candidate perimeters. @status:spec/plan
3. @fact:WAL-NEXT-P1-TAIL **The P1's deliberate batch grew.** 90 `PROP-008`
   anchors, plus **276 in `PROP-005`** — the largest single blob in the corpus,
   now shown to contain a false clause with no contradicting twin. The rest
   converts by itself as texts move, which is what the ruling intends. @status:spec/plan
4. @fact:WAL-NEXT-CHEAP-ROWS **The cheap rows:** B-074 (a `check` case — the
   machinery already refuses a misplaced typed anchor), B-071 (the tracing
   subscriber coupled to a publication flag), B-072 (the index's
   non-deterministic write). @status:spec/plan
5. @fact:WAL-NEXT-DRAIN **Then drain `BACKLOG.md` and refresh `TOOLING-MAP.md`** —
   the owner's instruction for this handoff. The map is dated 2026-08-04 and the
   tree is two days ahead of it; one fact in it now says so, and refreshing it
   comes BEFORE trusting any «measured gap» it lists. @status:spec/plan

## Waiting on the owner {#owner}

@fact:WAL-OWNER-A6 **А6 — generating the index's wire types.** The ruling stands
and its reasoning is sound; the **price changed**. One type is inexpressible in
the schema language (an untagged union — the manifest's file entry), 14 fields
would change absent-versus-empty on the wire, and strictness is 15 places at
this perimeter rather than the ~63 the programme measured across all host
crates. Three answers, all about the FORMAT rather than the code: B-073. @status:spec/plan

@fact:WAL-OWNER-V4 **В4 — the error-variant node.** The owner settled the
boundary between the ENGINES and settled it right. What was not concrete then
and is now: **who carries both dependencies.** Measured — exactly one host crate
depends on the quality engine, `xtask`, which is also outside the discipline
gates. Three ways to join with their prices, recommendation recorded: join on
DATA (file and line are in both), and say when the findings were taken, so a
missing report reads as «not measured» rather than «no violations». B-019. @status:spec/plan

@fact:WAL-OWNER-STANDING **Standing, unchanged:** the Phase E exit gate (six
corpus files at `work`); B-050; B-007; B-017 / B-020 / B-024; `AUDIT.md`
`2026-08-06-01` (P1, open), `-04`, `-06`, `-07`, `-11`, `-13`, `-14` and the
2026-06-12 rider; B-067 (38 packages' versions — legal while consumption is
local, blocking the moment anything is published). @status:impl/done

## Constraints — do not violate {#constraints}

- @fact:WAL-C-INSTRUMENTS-CATCH-WHAT-CARE-DOES-NOT **Six errors this session and
  an instrument caught every one.** The map's ratchet found five untagged items
  (my packet had dropped the `scope!` clause — the documented failure of a
  packet assembled mid-session); conform found two environment reads outside its
  list, one newly unsanctioned only because its file moved, and later an
  `.unwrap()` behind a real invariant and a false assumption; the length budget
  threw a file over 600 AFTER formatting, twice; `merge-verdicts` refused two
  batches, and one refusal proved my own evidence text was lying about itself.
  **Read a refusal; never route around it.** @status:impl/done
- @fact:WAL-C-A-MEASUREMENT-CAN-REFUSE-TOO **A measurement can be wrong about
  its own diagnosis, and that is more dangerous than a wrong count.** Five
  «orphaned verdicts» stood for nine days. Every anchor was present; what was
  lost was ADDRESSABILITY, to a missing blank line. The prepared repair — prune
  the five — would have destroyed five valid judgements to tidy a number. The
  real repair was three blank lines (B-074). @status:impl/done
- @fact:WAL-C-WHITESPACE-IS-LOAD-BEARING **In this markup whitespace is
  load-bearing.** Two facts on consecutive lines are ONE paragraph, and only the
  first keeps an address; the second becomes body text carrying a marker nobody
  can key a verdict to. `check` says nothing, the text reads identically, and no
  gate leaves a trace. @status:impl/done
- @fact:WAL-C-THE-BIGGEST-BLOB-CARRIES-A-LIE **The corpus's largest evidence blob
  names a file that has never existed.** `PROP-005`: 279 verdicts, four
  paragraphs, one covering 276, asserting a shipped schema at a path with no
  file, no directory and no git history. The earlier P1 specimen was findable
  because the corpus contradicted itself; this one has no contradicting twin. @status:impl/done
- @fact:WAL-C-A-DATED-READING-STAYS-DATED **A dated measurement is kept dated,
  not rewritten to match today.** Rewriting keeps the conclusion and erases the
  reasoning it was made on, which is a worse record than a reading marked with
  its date. Reframe the section and add the re-measurement beside it. @status:impl/done
- @fact:WAL-C-A-REVIEW-CORRECTION-CAN-DESTROY-COVERAGE **A correction the boss
  demands can remove the only test of the thing it protects.** Moving a scheme
  gate to the attachment step made the positive case untestable end to end,
  because this crate's mock servers are plain HTTP. The gap was the reviewer's
  and the reviewer closed it. @status:impl/done
- @fact:WAL-C-THE-PERIMETER-OF-A-MEASUREMENT-IS-A-CLAIM **State the perimeter
  with the number.** The TUI figure reproduces at exactly one of five candidate
  perimeters; the strictness count is 15 at the index's own types against ~63
  across host crates. A number whose perimeter is unstated is one the next
  reader re-derives wrongly. @status:impl/done
- @fact:WAL-C-MEASURE-WHAT-IS-BUILT-FIRST **Measure what of a plan is already
  built, and whether it CAN be built as specified, before building it** —
  against the AUTHORED tree. It stopped А6 this session, on a form the schema
  language cannot express. @status:impl/done
- @fact:WAL-C-MOVED-INTO-A-SPEC-IS-JUDGED-THE-SAME-PASS **Content moved into a
  specification is judged in the same pass that moves it**, and every landing
  this session obeyed it. `seal` refuses a file carrying any unjudged marker. @status:impl/done
- @fact:WAL-C-A-RE-JUDGEMENT-RECORDS-WHAT-IT-REPLACES **A re-judgement records
  what it replaces**, including that the earlier verdict was correct when
  formed. @status:impl/done
- @fact:WAL-C-A-CAPABILITY-LIVES-IN-A-LIBRARY **A capability lives in a library;
  surfaces are thin.** Floor: library + CLI + MCP, TUI where one exists;
  LSP/IDE deliberately undeclared, and an undeclared surface is not a debt. A
  shipped command with no spec document is the defect, not the pattern. @status:impl/done
- @fact:WAL-C-TUI-IS-DONE-WHEN-THIN **The TUI is finished when it is thin** —
  delete it on paper; if anything but presentation is lost, it is not done. @status:impl/done
- @fact:WAL-C-A-PLAN-IS-TEMPORARY **A plan is temporary** — deletable once
  executed, content moves into the specs, statements cite spec elements and
  never plan rows, tombstones are process support. @status:impl/done
- @fact:WAL-C-EVERY-MARKER-NAMES-ITS-KEY **Markup markers name their key:**
  `@fact:<ID>` and `@status:<stage>/<state>`. The legacy spellings are still
  read. **The canonical form for hashing is the LEGACY one.** @status:impl/done
- @fact:WAL-C-A-FENCE-IS-AN-EXAMPLE-UNTIL-MARKED **A fenced block is an example
  until marked `@fact/code:`.** An unknown object type is an ERROR. @status:impl/done
- @fact:WAL-C-EDITING-A-SPEC-MOVES-THE-MAP **Editing the TEXT of a spec document
  moves the committed map** — `cargo xtask specmap` in the same landing, as does
  a new `scope!` file. @status:impl/done
- @fact:WAL-C-VENDORED-INCLUDES-STACK-CRATES **`cargo xtask sync-engines` is its
  own step after any authored engine or stack-crate edit** — 51 pairs. Vendored
  copies are REGENERATED, never edited. @status:impl/done
- @fact:WAL-C-A-GREP-LIES-IN-BOTH-DIRECTIONS **A grep lies in both directions and
  «found nothing» is not a fact.** Measure on the structured field. @status:impl/done
- @fact:WAL-C-FILE-BUDGET-DOES-NOT-CHOOSE-A-TYPE **A file-length budget never
  chooses the shape of a public type** — and it is measured AFTER `cargo fmt`,
  which is where it bit twice this session. @status:impl/done
- @fact:WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe, and
  the VIEW of the output is not truncated.** The panel in background runs bare,
  and the verdict is read from the TAIL, never from the notification. @status:impl/done
- @fact:WAL-C-THE-PANEL-OWNS-THE-USER-HOME **No `vibe` verb writing the settings
  home under a running panel** — its tripwire is a global window. @status:impl/done
- @fact:WAL-C-MARK-NOT-SUPPRESS **Mark, don't suppress** · @fact:WAL-C-SIGNAL-NOT-WALL
  **a freshness check is a signal, not a gate** · @fact:WAL-C-SILENCE-IS-THE-DISEASE
  **cure the silence, not the state.** @status:impl/done
- @fact:WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS **A norm without a checker drifts
  silently.** @status:impl/done
- @fact:WAL-C-ONE-LAW-ONE-IMPLEMENTATION **One law, one implementation — and the
  divergence of two is silent by nature.** @status:impl/done
- @fact:WAL-C-CAMPAIGN-FRAME **T/F/G stay outside the good; publication is a
  separate operation; versions are not bumped before the pre-publication
  boundary** (B-067 records the debt). @status:impl/done
- @fact:WAL-C-NO-PAUSE **The mandate executes autonomously** — stop only at a
  real owner fork, one at a time. @status:impl/done
- @fact:WAL-C-PRESENTATION-FORMAT **Presentation (mandatory).** Plain meaning
  first WITHOUT reading specs → a tree for forks → exact names as an appendix;
  do not quote specs, do not cite section numbers. **If clarity and precision
  conflict, be clear first and precise immediately after.** @status:impl/done
- @fact:WAL-C-DELEGATION **Worker transport** (`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`
  read WHOLE; `SUBAGENT-MODE.toml` re-read before EVERY fan-out — currently
  `claudez`): launchers `C:\Users\olegc\opt\bin\{claudez,claudez2}`, own state
  dirs, `-c` keyed by (state dir, cwd) — **the `cd` goes INSIDE the
  parentheses**, and a `-c` goes only AFTER the run it corrects has ended. Logs
  in `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`; WORKER-REPORT
  mandatory; **`meta.md` is written as its own step**. **A packet assembled
  mid-session drops the clause the first one carried** — copy the closing
  sections (heartbeat, report template, self-verify, `scope!`, length budget)
  BEFORE writing the body. @status:impl/done
- @fact:WAL-C-SHELL-TRAPS **Shell traps:** the Bash tool's cwd PERSISTS —
  absolute paths or `git -C` (it bit again this session); editor tools only for
  edits; write python to a FILE, `PYTHONIOENCODING=utf-8` for Cyrillic. @status:impl/done
- @fact:WAL-C-STAGE-EXPLICIT **Never `git add -A`** — stage explicit paths;
  `.wt/` is now gitignored so a slip cannot sweep in a worktree. @status:impl/done
- @fact:WAL-C-COMMIT-MESSAGE-VIA-FILE **Commit messages go through a heredoc or
  `-F`.** @status:impl/done
- @fact:WAL-C-REAL-MIRROR **Rollout is `cargo xtask mirror` only**, fast-forward,
  never `--force`. @status:impl/done
- @fact:WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** — human attribution, no
  AI trailers; Conventional Commits; atomicity; autonomy. A worker is a tool,
  never a co-author. @status:impl/done

## Done (collapsed — see `git log`) {#done}

@fact:WAL-DONE **2026-08-06, second sitting: 26 commits.** А1 auto-publication
with its two startup refusals; А2 index authentication with the refused-versus-
absent probe; А3 the organisation cache with its honesty condition; А4 webhooks
and the guide that lives in the spec; А5a map search across library, CLI and
MCP. Plus: the stability instrument repaired (it read the retired markup and
inflated the debt nineteen-fold), the owner's guide corrected, the TUI
boundary's perimeter written down, 176 facts judged, ten files sealed, B-071
through B-074 filed, and both debt counters taken to zero. @status:impl/done

## In progress {#in-progress}

@fact:WAL-INFLIGHT **Nothing in flight.** No workers; `.wt/` empty; tree clean;
all reports and `meta.md` archived. @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-P1-BLOBS **4 380 verdicts share an evidence paragraph with at
  least one other**, 4 144 of them the host's. The largest blob covers 276 and
  contains a false clause. Converts by itself as texts move; the deliberate
  batches are `PROP-008`'s 90 and `PROP-005`'s 276. @status:impl/done
- @fact:WAL-KI-STALE **42 stale files**, mostly this session's own edits.
  `text-stability.py` names which facts actually moved — the per-file question
  and the per-fact question differ here by roughly seventeen to one. @status:impl/done
- @fact:WAL-KI-B074 **A second fact anchor in one paragraph is swallowed
  silently** (B-074) — borders P1; only the owner may take that estimate down. @status:impl/done
- @fact:WAL-KI-B070 **The kind vocabulary is written fourteen times** with no
  checker tying the copies together (B-070). @status:impl/done
- @fact:WAL-KI-B067 **38 packages changed content without a version bump**
  (B-067) — legal while consumption is local, blocking at publication. @status:impl/done
- @fact:WAL-KI-TOOLING-MAP-IS-BEHIND **`TOOLING-MAP.md` is dated 2026-08-04 and
  the tree is two days ahead of it.** Several of its «measured gaps» were
  measured before this session's landings; refresh it before trusting one. @status:impl/done
- @fact:WAL-KI-PHASE-E-SIX **Six corpus files stand at `work`** — the third
  condition of the Phase E exit gate, still awaiting a ruling. @status:impl/done
- @fact:WAL-KI-UNIX-HALF-UNCHECKED **Half of the `#[cfg(unix)]` stderr-capture
  cell is not compiler-checked on this machine.** @status:impl/done
- @fact:WAL-KI-VACUITY-AND-SCHEMA-ROOTS **The scan-vacuity check weakened when
  the JTD scanner landed** — named at build time, not fixed. @status:impl/done

## Session context {#session-context}

@fact:WAL-CTX-BOOT **A cold session reads `CONTINUE.md`, then
`NEXT-SESSION-PROMPT.md` (the order the owner asked for), then
`spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md` for what remains
of the programme**, then the transport law
`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`, then the live `BACKLOG.md`
rows and open `AUDIT.md` findings — and takes every number from the commands at
the top, **after rescanning**. This file supersedes `CONTINUE.md`; the owner's
rulings supersede both. @status:impl/done
