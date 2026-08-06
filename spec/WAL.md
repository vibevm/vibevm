# WAL — Project Continuation State {#root}

_Updated: 2026-08-06, wind-down №15 (**THE COURSE CHANGED.** A long owner
conversation authorised **eighteen work items** while the session closed **one**
backlog row. It is a new programme, not a backlog drain, and it lives in one
file: `spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md`. Order fixed
by the owner: **Б → В → А**. Built this session: B-032 closed, command nodes in
the map, the fact lifecycle made contract. Nine numbers failed to reproduce, and
the boss was wrong in front of the owner three times.)_

@fact:WAL-READ-THE-PROGRAMME-FIRST **The programme file is the plan and it is not
optional reading.** `spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md`
carries every item, the owner's order and the reasoning behind it, the rejected
alternatives, the three boss errors he corrected, and what was deliberately NOT
decided. **Nothing here restates it.** @status:impl/done

@fact:WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by
commands; run them rather than quoting this file.** @status:impl/done

```bash
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/judging-debt.py
python campaigns/packages-2026-09/tasks/text-stability.py
python campaigns/packages-2026-09/tasks/drift-registry.py
```

## Current phase {#current-phase}

@fact:WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`: Phase E
under the EXTENDED mandate. Phase T NOT started.** The backlog-drain course of
2026-08-05 is **superseded** by the owner's programme of 2026-08-06. @status:impl/done

@fact:WAL-STATE **State at checkpoint** (commands supersede): corpus **275 files,
0 unmarked facts**; registry ~11 642 confirmed / 177 drift. **Judging debt: 47
unjudged facts in 4 files, 5 orphan verdicts, 4 stale files** — all written this
week. `main` clean, `.wt/` removed, every worker report archived with `meta.md`.
**Panel green** at the last read. `vibe check` clean after the reinstall.
`gitverse` behind; **`github` UNREACHABLE**. Backlog **51 rows**, 11 live plus
four new. @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-PROGRAMME **Read the programme file, then execute group Б**, and
   inside it **Б1 first** — write the plan-closure rule into its two homes,
   because everything else in Б applies it. @status:spec/done
2. @fact:WAL-NEXT-B-REST **Then the rest of Б:** migrate 22 citations from plan rows
   to spec elements + repair 8 already-dangling ones + tombstone 35 closed rows;
   the panel step for markup validation (measure the user-home hazard FIRST);
   the two-column verdict report + re-judge the 90 shared-evidence anchors;
   the documentation-example policy; normalise `files_written` to forward
   slashes; record the three facts that need no decision. @status:spec/done
3. @fact:WAL-NEXT-V **Then group В:** the `org.vibevm.world/omnichannel` package,
   the `lang` package kind (measure the ripple first), `vibe tools`, the
   error-variant node joined at query time. @status:spec/done
4. @fact:WAL-NEXT-A **Then group А:** index auto-publication (the owner's blocker),
   private-index push **and authenticated reading** (measure whether the client
   can authenticate at all — unmeasured), the org cache + `rescan-org` + a
   freshness check, webhooks with a guide **living in the specs**, both search
   levels, generate the index wire types. @status:spec/done
5. @fact:WAL-NEXT-OWED **The boss owes the owner one thing:** options for what
   counts as «finished» for the text-interface subsystem. @status:spec/done

## Constraints — do not violate {#constraints}

- @fact:WAL-C-A-PLAN-IS-TEMPORARY **A plan is a temporary thing (owner, 2026-08-06).**
  When executed it must be deletable with nothing breaking. Significant content
  moves **into the specifications** on closure; statements point at **spec
  elements**, never at plan rows; tombstones inside a plan are process support
  for whoever walks it, not project structure. **Closed an item ⇒ rebuild the
  statements and specs so no tails remain** — part of closing, not tidying
  later. @status:impl/done
- @fact:WAL-C-MOVED-INTO-A-SPEC-IS-JUDGED-THE-SAME-PASS **Content moved into a
  specification is judged in the same pass that moves it.** An unjudged statement
  in a spec is the same kind of tail as a dangling citation, and without this the
  closure ruling above manufactures debt at every closure. @status:impl/done
- @fact:WAL-C-THE-PERIMETER-OF-A-MEASUREMENT-IS-A-CLAIM **The perimeter of a
  measurement is itself a claim, and it is the one nobody states.** Nine numbers
  failed to reproduce this session and not one from a bad pattern: a census of
  one binary taken as a count of all (29 → 43 → 71 → **56**), suppressions
  counted with a comment among them (57 → 55), a doc sweep wrong a **third**
  time against an unchanged directory, a four-layer model with **zero**
  occurrences where the row said it lived. **State the perimeter with the
  number.** @status:impl/done
- @fact:WAL-C-AN-ACCEPTANCE-NUMBER-CATCHES-WHAT-REVIEW-DOES-NOT **An acceptance
  stated as a number catches what no reading does.** Two crates declared
  `pub enum Command`; the join matched on type name alone and the map carried 29
  commands `vibe-index` does not have. The worker had recorded the name collision
  as a fact and drew only a conclusion about the number; the boss read the join
  and saw nothing. **The count on first regeneration is what spoke.** @status:impl/done
- @fact:WAL-C-A-ZERO-AFTER-AN-ENGINE-EDIT-IS-A-BUILD-QUESTION **A zero from the map
  after an engine edit is a build question before it is a code question.** Both
  authored and vendored sources verified correct by reading; `cargo clean -p
  core-ai-native-specmap` and one re-run gave the right answer. @status:impl/done
- @fact:WAL-C-A-FACT-INSIDE-A-FENCE-CANNOT-BE-JUDGED **A claim inside a fenced block
  is unverifiable by construction, and this tree has now paid for it twice in one
  week.** The owner guide's «this is in the gate panel» was false and survived
  because it sat in a ```bash block; the earlier instance was a layout diagram
  drawing a directory absent for months. **What to do about it is named, not
  answered.** @status:impl/done
- @fact:WAL-C-THE-LIFE-OF-A-FACT **Three things happen to a fact and only one
  announces itself** (PROP-043 §10.1): edited ⇒ comes due and is named; **added
  ⇒ unjudged and nothing says so**; **removed ⇒ its verdict stays and keeps
  counting**. «Stale file» ≠ «a judged fact moved» — a file goes stale when facts
  are merely added. @status:impl/done
- @fact:WAL-C-DEBT-IS-A-LIST **Judging debt is a list with names, paid one file at a
  time** (PROP-043 §10.2) — sealing is a whole-file assertion, so a file is clear
  or flagged; the cheapest file is the one you were going to open anyway. Never
  judge blind to move a number. Ask it: `tasks/judging-debt.py`. @status:impl/done
- @fact:WAL-C-A-CD-BEFORE-THE-CORRECTION-RETARGETS-IT **A `cd` before
  `( claudez -c … )` sends the correction to the repository root instead of the
  worker** — conversations key on (state dir, cwd), and the stray resumed thread
  holds write access to the real tree. The `cd` goes **inside** the parentheses;
  verify with `pwd` from within the subshell. @status:impl/done
- @fact:WAL-C-EDITING-A-SPEC-MOVES-THE-MAP **Editing the TEXT of a spec document
  moves the committed map**, not only a new file or a new edge — a unit's hash is
  over its span. Cost a red panel to learn. @status:impl/done
- @fact:WAL-C-A-GREP-LIES-IN-BOTH-DIRECTIONS **A grep lies in both directions and
  «found nothing» is not a fact.** `grep -P` refuses this locale with exit 2 into
  a discarded stream; `rg` without `-P` returns a clean zero on lookarounds.
  Measure on the structured field, never on a prefix of a free-form string. @status:impl/done
- @fact:WAL-C-A-VERDICT-NO-FACT-CAN-FALSIFY **A verdict no single fact can falsify is
  not that fact's verdict.** 4 151 of ~11 862 share one evidence blob. **Ruled
  2026-08-06: two columns in the report** — per-fact and document-level;
  computable today, converts by itself as texts move; the 90 known-bad anchors
  re-judged now. @status:impl/done
- @fact:WAL-C-A-PROMISE-HAS-MANY-HOMES **A promise the code does not keep lives
  everywhere it was retold.** Paid twice more: a docblock naming two surfaces
  that both did not exist, and the index's documentation describing a file layout
  and routes the code left behind. @status:impl/done
- @fact:WAL-C-MEASURE-WHAT-IS-BUILT-FIRST **Measure what of a plan is already built
  before building it** — nineteen builds of built things stopped in three days.
  Measure against the AUTHORED tree. @status:impl/done
- @fact:WAL-C-FILE-BUDGET-DOES-NOT-CHOOSE-A-TYPE **A file-length budget never chooses
  the shape of a public type.** It may decide where code sits, never what it is. @status:impl/done
- @fact:WAL-C-VENDORED-INCLUDES-STACK-CRATES **`cargo xtask sync-engines` is its own
  step after any authored engine or stack-crate edit** — 51 pairs, 21 copies of
  the map engine. @status:impl/done
- @fact:WAL-C-A-NEW-FILE-CHANGES-THE-MAP **A new `scope!` file or `#[verifies]` edge
  moves the committed map** — `cargo xtask specmap` in the same landing. @status:impl/done
- @fact:WAL-C-WORKERS-DO-NOT-FMT **`cargo fmt --all` after applying worker output** —
  a law of merging, not a courtesy. @status:impl/done
- @fact:WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe, and the
  VIEW of the output is not truncated.** The panel in background runs bare. @status:impl/done
- @fact:WAL-C-THE-PANEL-OWNS-THE-USER-HOME **No `vibe` verb writing the settings home
  under a running panel** — its tripwire is a global window. @status:impl/done
- @fact:WAL-C-ONE-THREAD-ONE-WRITER **`-c` only after the run it corrects has
  ended.** @status:impl/done
- @fact:WAL-C-RESULT-EVENT-IS-TERMINAL **A worker's terminal signal is the
  `{"type":"result"}` event**, never `TASK-DONE` and never a marker grep. Spawn
  without a trailing `&`. @status:impl/done
- @fact:WAL-C-REJECTION-NAMES-WHAT-IS-ACCEPTED **A rejection names what is accepted
  first, then exactly one wrong item, then verbatim text for the affected report
  sections.** Paid again: a worker's report was not updated for its second
  rework, so the boss performed the mandated proof itself. @status:impl/done
- @fact:WAL-C-WORKER-JUDGMENT **Workers escalate real judgement — honour it.** This
  session: a worker refuted the boss's acceptance number and was right. @status:impl/done
- @fact:WAL-C-EVIDENCE-IS-NOT-A-VERDICT **Workers gather evidence; the boss writes
  the verdict.** @status:impl/done
- @fact:WAL-C-ONE-LAW-ONE-IMPLEMENTATION **One law, one implementation — and the
  divergence of two is silent by nature.** @status:impl/done
- @fact:WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS **A norm without a checker drifts
  silently.** The reason recognition is by framework declaration, not by an
  author's marker. @status:impl/done
- @fact:WAL-C-MARK-NOT-SUPPRESS **Mark, don't suppress** · @fact:WAL-C-SIGNAL-NOT-WALL
  **a freshness check is a signal, not a gate** · @fact:WAL-C-SILENCE-IS-THE-DISEASE
  **cure the silence, not the state.** @status:impl/done
- @fact:WAL-C-BUILD-FIRST **BUILD-FIRST** · @fact:WAL-C-PARITY-IS-THE-INVARIANT **parity
  is the invariant, never the same code.** @status:impl/done
- @fact:WAL-C-CAMPAIGN-FRAME **T/F/G stay outside the good; publication is a separate
  operation; versions are not bumped before the pre-publication boundary.** @status:impl/done
- @fact:WAL-C-NO-PAUSE **The mandate executes autonomously** — stop only at a real
  owner fork, one at a time. @status:impl/done
- @fact:WAL-C-PRESENTATION-FORMAT **Presentation (mandatory).** Plain meaning first
  WITHOUT reading specs → a tree for forks → exact names as an appendix; do not
  quote specs, do not cite section numbers. **Owner ruling 2026-08-06: if
  clarity and precision conflict, be clear first and precise immediately
  after.** When he says «I understood nothing», explain from zero. @status:impl/done
- @fact:WAL-C-DELEGATION **Worker transport** (`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`
  read WHOLE; `SUBAGENT-MODE.toml` re-read before EVERY fan-out — currently
  `claudez`): launchers `C:\Users\olegc\opt\bin\{claudez,claudez2}`, own state
  dirs, `-c` keyed by (state dir, cwd). Logs in
  `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`; WORKER-REPORT
  mandatory; full log paths into the chat at every spawn. @status:impl/done
- @fact:WAL-C-ARCHIVE-BEFORE-CLEANUP **A worker's report is archived as its own
  step, before any cleanup.** Paid off again this session — a worktree that
  would not delete kept nothing hostage. @status:impl/done
- @fact:WAL-C-SHELL-TRAPS **Shell traps:** the Bash tool's cwd PERSISTS — absolute
  paths or `git -C`; CRLF versus `str.replace` — editor tools only; a python
  script in a heredoc breaks on escaping — write the script to a file;
  `PYTHONIOENCODING=utf-8` for Cyrillic output. @status:impl/done
- @fact:WAL-C-STAGE-EXPLICIT **Never `git add -A`** — stage explicit paths. @status:impl/done
- @fact:WAL-C-COMMIT-MESSAGE-VIA-FILE **Commit messages go through a heredoc or
  `-F`.** @status:impl/done
- @fact:WAL-C-REAL-MIRROR **Rollout is `cargo xtask mirror` only**, fast-forward,
  never `--force`. @status:impl/done
- @fact:WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** — human attribution, no AI
  trailers; Conventional Commits; atomicity; autonomy. A worker is a tool, never
  a co-author. @status:impl/done

## Done (collapsed — see `git log`) {#done}

@fact:WAL-DONE **2026-08-06: 28 commits.** B-032 closed; B-019(б) slice 1 built
(56 command nodes); the `via_redirect` docblock made honest; PROP-043 gained the
fact lifecycle and the debt-clearance procedure; the resume contract reports the
debt; four backlog rows filed (B-063…B-066); three audit findings re-measured
with every number moved; the programme file authored. @status:impl/done

## In progress {#in-progress}

@fact:WAL-INFLIGHT **Nothing in flight.** No workers; `.wt/` removed; tree clean;
all reports and `meta.md` archived. @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-GITHUB-UNREACHABLE **`github` UNREACHABLE** — ssh to `git@github.com`
  redirected to `127.92.0.49`. **Not a divergence; must not be forced. The only
  thing needing the owner's hands.** @status:impl/done
- @fact:WAL-KI-GITVERSE-BEHIND **`gitverse` is behind** — run `cargo xtask mirror`. @status:impl/done
- @fact:WAL-KI-JUDGING-DEBT **47 unjudged facts in 4 files, 5 orphan verdicts, 4
  stale files** — all written this week. Ask `tasks/judging-debt.py`; clear per
  file. @status:impl/done
- @fact:WAL-KI-VERDICT-EVIDENCE-P1 **P1 `2026-08-06-01`** — 4 151 verdicts share
  their evidence. **Ruled: two columns.** Execution is programme item Б4. @status:impl/done
- @fact:WAL-KI-AUDIT-OPEN **`AUDIT.md` open:** `2026-08-06-01` (P1), `-04`, `-06`,
  `-07`, `-10`, `-11`, `-13`, `-14`, the 2026-06-12 rider. `-04`/`-10`/`-14` now
  carry rulings and are programme items. @status:impl/done
- @fact:WAL-KI-BACKLOG-51 **Backlog 51 rows**, 35 of them finished history awaiting
  the tombstone pass (programme item Б2). @status:impl/done
- @fact:WAL-KI-PHASE-E-SIX **Six corpus files stand at `work`** — the third condition
  of the Phase E exit gate, still awaiting a ruling. @status:impl/done
- @fact:WAL-KI-UNIX-HALF-UNCHECKED **Half of the `#[cfg(unix)]` stderr-capture cell
  is not compiler-checked on this machine.** @status:impl/done
- @fact:WAL-KI-VACUITY-AND-SCHEMA-ROOTS **The scan-vacuity check weakened when the
  JTD scanner landed** — named at build time, not fixed. @status:impl/done

## Session context {#session-context}

@fact:WAL-CTX-BOOT **A cold session reads `CONTINUE.md`, then
`spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md` IN FULL**, then the
transport law `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`, then the live
`BACKLOG.md` rows and open `AUDIT.md` findings — and takes every number from the
commands at the top, **after rescanning**. `CONTINUE.md` is the cold-resume
snapshot; this file supersedes it; the programme file is the plan. @status:impl/done
