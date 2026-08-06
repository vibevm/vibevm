# WAL — Project Continuation State {#root}

_Updated: 2026-08-06, wind-down №16 (**the programme is half executed**:
group Б closed in full, В1–В3 closed, В4 waits on А5, group А untouched and
is where the next session starts. 34 commits, panel green, both mirrors
synced. The markup migration landed — 27 407 substitutions, zero verdicts
disturbed. Six measurement errors this session, each caught by an
instrument rather than by care.)_

@fact:WAL-READ-THE-PROGRAMME-FIRST **The programme file is the plan and it is not
optional reading.** `spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md`
carries every item, the owner's order, the reasoning, the rejected
alternatives and the boss errors he corrected. Б is closed; its rows carry
tombstones. **Nothing here restates it.** @status:impl/done

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
2026-08-06 governs; groups Б and В (less В4) are executed. @status:impl/done

@fact:WAL-STATE **State at checkpoint** (commands supersede): corpus **278 files,
0 unmarked facts**; verdicts ~**11 870 at 98.2 %**, now in two grains —
**62.4 % per-fact**, the rest document-level; drift **173**. Judging debt
**~142 unjudged in 12 files**, mostly facts this session wrote. `main`
clean, `.wt/` empty, every worker report archived **with `meta.md`**.
**Panel green**, `vibe check` clean, **both mirrors synced**. Backlog **55
rows**, 35 of them tombstones. @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-GROUP-A **Group А — the index, and it is where the session
   starts.** А1 auto-publication (the owner called it a blocker), А2 private
   index — **measure first whether the client can authenticate at all**, А3
   the org cache with `--cache-org` (name ruled; the freshness check is what
   keeps its default honest), А4 webhooks with the guide living in the specs,
   А5 both search levels, А6 generate the index wire types. @status:spec/done
2. @fact:WAL-NEXT-V4 **Then В4** — the error-variant node, joined at query
   time. It waits on А5's tool by construction, not by choice. @status:spec/done
3. @fact:WAL-NEXT-TUI-AUDIT **The TUI thinness audit** — the boundary is now
   defined (PROP-037 §0, owner ruling): finished = a thin surface, tested by
   deletion on paper. 63 files, 18 426 lines to walk. @status:spec/plan
4. @fact:WAL-NEXT-B4-TAIL **Б4's tail:** 88 anchors still share one evidence
   blob. None is known false; they convert as their texts move, which is what
   variant (в) chose. @status:spec/plan
5. @fact:WAL-NEXT-DEBT **The judging debt is up on purpose** — this session
   wrote ~56 facts into the new omnichannel package alone. Clear per file,
   cheapest first, never blind to move a number. @status:spec/plan

## Constraints — do not violate {#constraints}

- @fact:WAL-C-THE-PERIMETER-OF-A-MEASUREMENT-IS-A-CLAIM **The perimeter of a
  measurement is a claim, and it is almost always wider than it looks.** Six
  errors this session, none from a bad pattern: a stale jump 5→30 blamed on a
  migration that had not caused it; an argument built on a stage dictionary
  that was short in the SCRIPT, not in the corpus; a clash between `##` and
  headings that does not exist; 42 citations where the rule governs 22; a
  package kind invisible in three JSON schemas because the search looked for
  a comma-separated list and a schema writes an `enum` array; a fourth
  different count for one doc finding. **State the perimeter with the
  number.** @status:impl/done
- @fact:WAL-C-INSTRUMENTS-CATCH-WHAT-CARE-DOES-NOT **Almost every one of those
  was caught by an instrument, not by attention.** The panel found the
  schemas; conform demanded a behaviour oracle; a skill-template test refused
  a tool no agent would be taught to call; `merge-verdicts` refused to
  overwrite without `--force`; `seal` refused a file whose new facts this
  same session had left unjudged. **A tool that refuses is worth more than one
  that obliges.** @status:impl/done
- @fact:WAL-C-ONE-GRAMMAR-THREE-READERS **One markup grammar had three
  readers, and their failures rank by loudness.** The progress reader would
  have dropped facts; the map reported *4654 units removed*; the boot compiler
  **said nothing** — it merely stopped qualifying 466 labels, and two packages
  sharing a fact id would have collided in silence. Teach every reader, and
  assume there is one more. @status:impl/done
- @fact:WAL-C-A-VOCABULARY-WRITTEN-FOURTEEN-TIMES **A closed vocabulary lives
  in fourteen places and the compiler keeps two honest** (B-070). Two JTD
  schemas had already drifted — they never learned `mcp`. Adding a value
  breaks 5 matches loudly and leaves 12 lists silently stale. @status:impl/done
- @fact:WAL-C-EVERY-MARKER-NAMES-ITS-KEY **Markup markers name their key:**
  `@fact:<ID>` and `@status:<stage>/<state>`. The legacy `##ID` /
  `@stage/state` are still read. **The canonical form for hashing is the
  LEGACY one** — that direction is what let 27 407 substitutions leave every
  baseline, seal and verdict valid. @status:impl/done
- @fact:WAL-C-A-FENCE-IS-AN-EXAMPLE-UNTIL-MARKED **A fenced block is an example
  until marked `@fact/code:`**; then it is that fact's body and comes due when
  its text moves. Default unchanged: 372 fences stay examples. An unknown
  object type is an ERROR, never a shrug. @status:impl/done
- @fact:WAL-C-A-PLAN-IS-TEMPORARY **A plan is temporary** — deletable once
  executed, content moves into the specs, statements cite spec elements and
  never plan rows, tombstones are process support. Two homes, one pointer
  (`addressable-specs#disposable-targets`, `campaign-plans#temporary`). @status:impl/done
- @fact:WAL-C-A-CAPABILITY-LIVES-IN-A-LIBRARY **A capability lives in a
  library; surfaces are thin.** vibevm's floor: library + CLI + MCP, TUI where
  one exists; LSP/IDE deliberately undeclared, and an undeclared surface is not
  a debt. The coverage table is DERIVED, never hand-kept. @status:impl/done
- @fact:WAL-C-TUI-IS-DONE-WHEN-THIN **The TUI is finished when it is thin**
  (owner, 2026-08-06) — delete it on paper; if anything but presentation is
  lost, it is not done. Not a screen count, not a suppression count: those
  move when a screen is added, and thinness does not. @status:impl/done
- @fact:WAL-C-MOVED-INTO-A-SPEC-IS-JUDGED-THE-SAME-PASS **Content moved into a
  specification is judged in the same pass that moves it** — enforced in
  practice this session: `seal` refused a file whose seven new facts were
  unjudged an hour after they were written. @status:impl/done
- @fact:WAL-C-A-RE-JUDGEMENT-RECORDS-WHAT-IT-REPLACES **A re-judgement records
  what it replaces**, including that the earlier verdict was correct when
  formed. Erasing it would make the corpus look as though it had never been
  wrong, and saying "this was false, here is when" is what makes the rest
  believable. @status:impl/done
- @fact:WAL-C-GIT-DOES-NOT-RESTORE-LINE-ENDINGS **`git checkout` does not
  restore line endings** — git does not consider them changed, so a revert
  leaves the damage on disk and `diff` shows nothing. Read and write with
  newline translation OFF in any migration. @status:impl/done
- @fact:WAL-C-EDITING-A-SPEC-MOVES-THE-MAP **Editing the TEXT of a spec document
  moves the committed map** — `cargo xtask specmap` in the same landing, as
  does a new `scope!` file. @status:impl/done
- @fact:WAL-C-VENDORED-INCLUDES-STACK-CRATES **`cargo xtask sync-engines` is its
  own step after any authored engine or stack-crate edit** — 51 pairs.
  Vendored copies are REGENERATED, never edited. @status:impl/done
- @fact:WAL-C-A-GREP-LIES-IN-BOTH-DIRECTIONS **A grep lies in both directions and
  «found nothing» is not a fact.** Measure on the structured field, never on a
  prefix of a free-form string. @status:impl/done
- @fact:WAL-C-MEASURE-WHAT-IS-BUILT-FIRST **Measure what of a plan is already
  built before building it** — against the AUTHORED tree. @status:impl/done
- @fact:WAL-C-FILE-BUDGET-DOES-NOT-CHOOSE-A-TYPE **A file-length budget never
  chooses the shape of a public type.** It decides where code sits, never what
  it is — paid this session by moving `PackageKind` to its own cell rather
  than trimming its documentation. @status:impl/done
- @fact:WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe, and
  the VIEW of the output is not truncated.** The panel in background runs bare. @status:impl/done
- @fact:WAL-C-THE-PANEL-OWNS-THE-USER-HOME **No `vibe` verb writing the settings
  home under a running panel** — its tripwire is a global window. Measured
  2026-08-06: `progress check` does NOT write it, in either form. @status:impl/done
- @fact:WAL-C-MARK-NOT-SUPPRESS **Mark, don't suppress** · @fact:WAL-C-SIGNAL-NOT-WALL
  **a freshness check is a signal, not a gate** · @fact:WAL-C-SILENCE-IS-THE-DISEASE
  **cure the silence, not the state.** @status:impl/done
- @fact:WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS **A norm without a checker drifts
  silently.** @status:impl/done
- @fact:WAL-C-ONE-LAW-ONE-IMPLEMENTATION **One law, one implementation — and the
  divergence of two is silent by nature.** @status:impl/done
- @fact:WAL-C-CAMPAIGN-FRAME **T/F/G stay outside the good; publication is a
  separate operation; versions are not bumped before the pre-publication
  boundary** (B-067 records the debt the migration added). @status:impl/done
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
  parentheses**. Logs in `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`;
  WORKER-REPORT mandatory; **`meta.md` is written as its own step, not at
  cleanup** — six were missing until this session. @status:impl/done
- @fact:WAL-C-SHELL-TRAPS **Shell traps:** the Bash tool's cwd PERSISTS —
  absolute paths or `git -C`; editor tools only for edits; a python script in a
  heredoc breaks on escaping — write the script to a file;
  `PYTHONIOENCODING=utf-8` for Cyrillic. @status:impl/done
- @fact:WAL-C-STAGE-EXPLICIT **Never `git add -A`** — stage explicit paths. @status:impl/done
- @fact:WAL-C-COMMIT-MESSAGE-VIA-FILE **Commit messages go through a heredoc or
  `-F`.** @status:impl/done
- @fact:WAL-C-REAL-MIRROR **Rollout is `cargo xtask mirror` only**, fast-forward,
  never `--force`. @status:impl/done
- @fact:WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** — human attribution, no
  AI trailers; Conventional Commits; atomicity; autonomy. A worker is a tool,
  never a co-author. @status:impl/done

## Done (collapsed — see `git log`) {#done}

@fact:WAL-DONE **2026-08-06: 34 commits.** The markup migration (engine reads
both spellings, a committed reversible rewriter, 327 files, three readers
taught); `@fact/code:`; group Б entire; В1 omnichannel + host floor; В2 the
`lang` kind with its VIBEVM-SPEC amendment; В3 `vibe tools` with two thin
surfaces and an oracle; the TUI boundary; B-067/B-068/B-069/B-070 filed. @status:impl/done

## In progress {#in-progress}

@fact:WAL-INFLIGHT **Nothing in flight.** No workers; `.wt/` empty; tree clean;
all reports and `meta.md` archived. @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-JUDGING-DEBT **~142 unjudged facts in 12 files** — mostly written
  this session (omnichannel alone ~56). Visible, named, cleared per file. @status:impl/done
- @fact:WAL-KI-B4-TAIL **88 anchors share one evidence blob** (Б4's tail). None
  known false; they convert as their texts move. @status:impl/done
- @fact:WAL-KI-B070 **The kind vocabulary is written fourteen times** with no
  checker tying the copies together (B-070). Two schemas had already drifted. @status:impl/done
- @fact:WAL-KI-B067 **38 packages changed content without a version bump**
  (B-067) — legal while consumption is local, blocking the moment anything is
  published. @status:impl/done
- @fact:WAL-KI-AUDIT-OPEN **`AUDIT.md` open:** `2026-08-06-01` (P1, partly
  closed — measurement built, 88-anchor re-judgement outstanding), `-04`,
  `-06`, `-07`, `-11`, `-13`, `-14`, the 2026-06-12 rider. `-10` closed today. @status:impl/done
- @fact:WAL-KI-PHASE-E-SIX **Six corpus files stand at `work`** — the third
  condition of the Phase E exit gate, still awaiting a ruling. @status:impl/done
- @fact:WAL-KI-UNIX-HALF-UNCHECKED **Half of the `#[cfg(unix)]` stderr-capture
  cell is not compiler-checked on this machine.** @status:impl/done
- @fact:WAL-KI-VACUITY-AND-SCHEMA-ROOTS **The scan-vacuity check weakened when
  the JTD scanner landed** — named at build time, not fixed. @status:impl/done

## Session context {#session-context}

@fact:WAL-CTX-BOOT **A cold session reads `CONTINUE.md`, then
`spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md` IN FULL**, then
the transport law `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`, then the
live `BACKLOG.md` rows and open `AUDIT.md` findings — and takes every number
from the commands at the top, **after rescanning**. `CONTINUE.md` is the
cold-resume snapshot; this file supersedes it; the programme file is the plan. @status:impl/done
