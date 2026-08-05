# WAL — Project Continuation State {#root}

_Updated: 2026-08-05, wind-down №11 (**PHASE E — B-056 BUILT WHOLE.** The owner
set the order — gate holes → hygiene → B-056 → волна Г — and the first three are
done. `#source` is no longer single-source or single-level: a contract may
declare several sources and/or a package-name glob, they fold in declaration
order, recursively, under the cycle law that already existed, and every node's
text enters the document exactly once. **B-055 closed on the way.** The build
also refuted two of its own design's claims and produced a fifth landing the
design had not foreseen.)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by
commands; run them rather than quoting this file.** @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/text-stability.py
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`:
Phase E RUNNING under the EXTENDED mandate (2026-08-04, §7 LOG: «Хочу все
остальные волны сделать»). **Волны А, Б, В ЗАКРЫТЫ; волна Г частично** —
B-005, B-010, B-055, B-056 закрыты, остаются B-040 и долг схем F-132. Live
zone `campaigns/packages-2026-09/`. @impl/done

##WAL-STATE **State at checkpoint** (2026-08-05; the commands supersede):
registry **91 obligations / 182 drift verdicts**, confirmed **11 531**. Corpus
after `scan`: **272 files, 13 249 markers, 0 unmarked facts, 0 errors**;
`progress check` clean. Panel GREEN (bare, tail read) over the whole build.
`vibe-spec` after B-056: 206 unit tests, conform 0 new findings, every file
under the 600-line budget. `gitverse` holds `main` @ `fff22ff0`; **`github` is
BEHIND** — ssh intercepted at `127.92.0.49`, not a divergence. @impl/done

## Next {#next}

1. ##WAL-NEXT-REGISTRY-DEBT **The registry debt B-056's documentation created —
   measured, named, and the cheapest judging pass in the campaign.** *(a)* **10
   facts to re-judge** in `spec/design/multiple-sources-and-plugins.md`: every
   one moved because its status flipped `@spec/plan` → `@impl/done` when the
   landing earned it, so the claim is unchanged and the flip is its own
   evidence. *(b)* **The facts added this slice, to judge fresh:** six in
   PROP-035 §7.3, two corrections in the design, three in the transport law,
   two in B-056's backlog row. *(c)* **PROP-035 is sealable as it stands** — 146
   verdicts byte-identical, because the edit only appended. Route unchanged:
   `mirror` → `scan` → boss verdicts → `merge-verdicts.py` → seal, and **never
   chain merge and seal**. @spec/done
2. ##WAL-NEXT-WAVE-G **Волна Г, what is left** (`TOOLING-MAP.md` `##WAVE-G`):
   B-040 (the pointed seam refactor over `harvest/g1-b040-seams-census.md`, boss
   design) and the F-132 schema debt — **whose target file does not exist**; the
   measured debt and why the cheap fix is a wish are recorded in `TASKS.md`. @spec/done
3. ##WAL-NEXT-OWNER-OPEN **Waiting on the owner, none blocking:** B-020 (the
   external-LLM client — needs credentials), B-024's `disputed` ruling, the
   privacy tier's content, the own/foreign distinction (deferred), fork №6
   (query language, deferred at medium priority). @spec/done
4. ##WAL-NEXT-PARITY-DEBT **M-PARITY bar 2 owes TWO**, both owner-recorded
   deferrals: `{#b-050}` (Rust dylint / Go analysis.Analyzer vehicles) and
   `{#b-053}` (the Rust deviation-reason text). @spec/done
5. ##WAL-NEXT-B059 **B-059 stands open:** conform's `exclude_substrings` matches
   the crate-relative path while the finding it prints is repo-relative, so an
   exclusion written from a finding's address is a silent no-op. @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-ONE-LAW-ONE-IMPLEMENTATION **One law, one implementation — and the
  divergence of two is SILENT by nature.** One hash, one calculator; one cycle
  law, one walker (`use_graph`'s `visit` now serves both `#use` and `#source`
  through an `EdgeKind`, never a second traversal); and ONE function computes a
  document's `#source` edges, because a glob expanded one way for the guard and
  another for the fold would build a guard over a graph the fold never walks,
  and nothing would say so. @impl/done
- ##WAL-C-NODE-DEDUP-IS-NOT-TEXT-DEDUP **A traversal that visits a shared node
  once does NOT make its text appear once.** The walk deduplicates NODES; the
  fold is textual INCLUSION. In a diamond both parents fold the shared source
  into themselves and the seed carries its body twice — harmless for prose and
  **lethal for facts**, because the surviving duplicate anchor sinks the build
  and an ordinary two-plugins-over-a-common-base composition stops compiling.
  The fold therefore carries an **include guard**: a node's body enters once, by
  the first path in the deterministic fold order, exactly as `#include` guards
  sit on top of a compiler that already reads each header once. Measured on the
  build's own diamond test, which first asserted two copies because that is what
  the code did. @impl/done
- ##WAL-C-JUDGE-WHERE-PROVENANCE-LIVES **Catch it where the provenance still
  exists.** `gate::first_duplicate` deliberately tolerates a repeated HEADING —
  in the merged view that shape is indistinguishable from the legitimate `:add`
  sum of a contract section with its source's — and by the time it runs, who
  brought what is gone. So two sources DEFINING one source-only section passed
  silently unless a fact happened to collide. The check belongs in the fold,
  which still holds each source's tree separately, per level, and only as a
  fallback after the fact gate so a colliding fact still names its more
  specific id. @impl/done
- ##WAL-C-A-DOCBLOCK-CAN-LIE **Code that promises more than it does is the same
  defect as a silent drop, and review catches it where tests do not.** Three
  landings this session shipped green tests with a docblock overstating the
  code: `expand_pattern` claimed a non-pattern «denotes exactly itself» while
  scanning `vibedeps/` (so a self-coordinate address expanded to EMPTY — silent
  source loss, the B-055 class in a new place); the fold claimed dedup that
  never reached the text; the design claimed a catcher that cannot catch. Read
  the code against its own promises. @impl/done
- ##WAL-C-DEGENERATE-CASE-IS-THE-REGRESSION-GATE **Keeping the old name as the
  degenerate case is the regression harness, not politeness.** `fold_source(c,s)`
  = `fold_sources(c, &[s])` made every pre-existing fold test the gate on the
  generalisation, and each landing repeated the pattern: the no-`#source` path
  returns its text unparsed, one `#source` is the singleton fold. @impl/done
- ##WAL-C-FILE-BUDGET-IS-NEUTRAL **The 600-line budget is a NEUTRAL conform key
  and counts every file, tests included** — `max_file_lines` sits in the
  language-neutral section of `conform.toml`, excluding only `/generated/`, and
  the gate stands on «0 new findings». Measured only by the boss and only AFTER
  `cargo fmt`. A worker arguing the budget away is a rework, not a judgement
  call. @impl/done
- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES → confirmed
  when coherent and every referent resolves; DESCRIBES → checked against the
  tree; unexercisable → unverifiable; `world` adds source 2, §3.8 bounds it. @impl/done
- ##WAL-C-STALENESS-OVER-REPORTS **A document-level staleness signal
  over-reports by orders of magnitude.** `tasks/text-stability.py` takes the
  commit that was HEAD at `verified_at` and byte-compares each judged fact's own
  paragraph. A clean result licenses the SEAL and **never a claim of
  freshness** — wording that never moved can still have drifted because the code
  under it moved. @impl/done
- ##WAL-C-AN-INSTRUMENT-CAN-LIE-QUIETLY **An instrument's blind spot fails in
  the dangerous direction, so re-verify every decision it made after fixing
  it.** Paid on `text-stability.py`'s extractor (list facts, then numbered
  ones), and again on the transport law's own status poll (below). @impl/done
- ##WAL-C-STATUS-POLL-MATCHES-THE-PACKET **The worker status poll reports
  completion before the worker says it.** `--output-format stream-json` logs the
  PROMPT too, and every packet quotes its own closing clause, so a grep for the
  done-marker hits the packet's text from the first line. A boss reading it sees
  a finished worker while it is mid-run, and the natural next move — a `-c`
  correction — is precisely the two-writers-on-one-worktree failure. Completion
  is the harness notification plus the report file; liveness is the worker's own
  tool CALLS (`"command":"echo \"PROGRESS`), which packet text cannot forge. @impl/done
- ##WAL-C-A-C-CARRIES-ITS-OWN-CD **A `-c` correction carries its own explicit
  `cd` into the worker's worktree.** Conversations key on (state dir, cwd), so
  relying on the Bash tool's persisted cwd is a coin flip between correcting the
  worker and editing the HOST tree. It landed correctly once this session by
  accident, which is not a method. @impl/done
- ##WAL-C-FOLLOW-UP-PACKET-DROPS-BOILERPLATE **A packet assembled mid-session
  from a review note drops the boilerplate the first packet carried.** The
  collision packet omitted the heartbeat clause and the worker emitted no
  progress marker for its whole first run — ten minutes indistinguishable from a
  stall by the prescribed poll. The clause belongs to the packet, not the
  worker's discipline; copy the closing sections (heartbeat, report template,
  self-verify block) before writing the body. @impl/done
- ##WAL-C-LOG-VOLUME-IS-TELEMETRY **Log size is not activity** — with a thinking
  budget set the stream-json log carries one line per thinking token, so it
  reaches megabytes while nothing is done. Read the last non-telemetry event. @impl/done
- ##WAL-C-DEBT-IS-NAMED-NOT-FROZEN **A landing names its debt with a number
  instead of freezing or hiding it.** Applied again at this wind-down: the
  registry debt is 10 re-judgements plus the named new facts, not «some
  bookkeeping left». @impl/done
- ##WAL-C-MARK-NOT-SUPPRESS **Mark, don't suppress (owner, 2026-08-01).** An
  acknowledged deviation ALWAYS produces a finding stamped
  `DeviationAcknowledged`; `baseline::diff` keeps it out of `new` in ONE place. @impl/done
- ##WAL-C-SIGNAL-NOT-WALL **A freshness check is a SIGNAL, not a gate** —
  `local-source-freshness` warns, names both hashes and the remedy, and `vibe
  check` still exits 0. @impl/done
- ##WAL-C-SILENCE-IS-THE-DISEASE **When something legitimate goes unnoticed,
  cure the silence, not the state.** @impl/done
- ##WAL-C-A-KEY-THAT-MATCHES-A-DIFFERENT-STRING **A config key filtering on a
  different string than the tool prints is a silent no-op** (B-059). @impl/done
- ##WAL-C-DERIVED-PERIMETER-NEEDS-A-DERIVED-GUARD **Naming a perimeter literally
  costs the guard a glob gave for free, so derive the guard** (panel step 10c). @impl/done
- ##WAL-C-MEASURE-BEFORE-ASSERTING **A plausible model of the system is not a
  measurement.** This session paid it three more times, and each time the
  measurement came from a worker's own test rather than from review. @impl/done
- ##WAL-C-REFUTATION-SHRINKS-WORK **A refuted premise makes the work smaller** —
  except when it makes it larger and RIGHT, as the include guard did. @impl/done
- ##WAL-C-EVIDENCE-IS-NOT-A-VERDICT **Workers gather evidence; the boss writes
  every verdict, and the split is mechanical** — a gathered row arrives stamped
  `PENDING`, which `merge-verdicts.py` refuses. @impl/done
- ##WAL-C-WORKER-JUDGMENT **GLM workers escalate real judgement — honour it.**
  This session two of the three central findings came from workers measuring
  instead of complying: the diamond's double inclusion and the gate's blindness
  to a heading-only collision. @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline rule is
  never weakened for being unused. @impl/done
- ##WAL-C-PARITY-IS-THE-INVARIANT-NOT-THE-CODE **Parity means the same
  invariant, never the same code.** @impl/done
- ##WAL-C-CHARACTERIZATION-COUPLING **A new gate rule breaks EVERY by-rule
  count**; mount and fix the counters in the SAME landing. @impl/done
- ##WAL-C-STRUCT-FIELD-RIPPLE **Adding a field or an enum variant breaks every
  literal construction and every exhaustive match**; only `--all-targets` finds
  them, and two of three scanners compile against VENDORED copies. Measured
  again this session: `vibe_spec::ResolveError` and the `ResolveError` in
  `crates/vibe-cli/src/commands/vvm/source.rs` are DIFFERENT enums of one name. @impl/done
- ##WAL-C-CONFIG-V2 **The config surface is per-language**; the gate key is the
  neutral `gated`; retired flat keys are loud tombstones. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror --campaign
  <zone>`**; any anchor-set change requires it before `merge-verdicts.py`;
  **never chain merge and seal.** @impl/done
- ##WAL-C-CHECK-NO-LONGER-WRITES **`vibe progress check` is read-only.** @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame.** The map's waves execute through
  the campaign's phases; **T/F/G остаются вне добра**; publication is a separate
  operation after the refactor; versions are NOT bumped until the
  pre-publication boundary. @impl/done
- ##WAL-C-NO-PAUSE **The mandate runs autonomously** — stop only on a genuine
  owner map-fork (one at a time), never on a work boundary. @impl/done
- ##WAL-C-SELF-COORDINATE **The host is a package (B-031).** Root identity
  `org.vibevm.core/vibevm`; `spec://vibevm/…` parses and NEVER resolves. @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`, ENGINE in
  its crates (vendored ×6), DRIVER per stack CLI, DEPLOYMENT in the consumer. @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**`; historical evidence JSON keeps pre-migration spellings. @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **«Замеров нет и нескоро будет»** — the
  standing answer; never re-raised. @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (binding).** Суть
  по-человечески БЕЗ чтения спек → дерево для развилок (`AskUserQuestion` с
  рекомендацией) → точные имена приложением; спеки не цитировать. Развилки — по
  одной. **И когда владелец говорит «я ничего не понял» — объяснять с нуля.** @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport** (mechanics
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` — read WHOLE; switch
  `SUBAGENT-MODE.toml` re-read before EVERY fan-out, now `claudez`): launchers
  `C:\Users\olegc\opt\bin\{claudez,claudez2}`, per-launcher state dirs, `-c`
  keyed by (state dir, cwd) — **so a `-c` rework MUST run from the worker's own
  worktree and ONLY after that run has ended.** Logs stream into
  `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`; WORKER-REPORT
  mandatory; boss states log paths in chat at every fan-out. @impl/done
- ##WAL-C-PACKET-CITES-ONLY-WHAT-GIT-CARRIES **A packet may only cite what git
  carries** — `campaigns/*/run/mirror/` is gitignored, so a fresh worktree lacks
  it; check `git check-ignore` before citing a generated artifact. @impl/done
- ##WAL-C-NO-DELIVERABLE-IS-NOT-NO-WORK **«No deliverable» is not «no work» —
  look at the disk before re-commissioning.** @impl/done
- ##WAL-C-COMPLETION-SIGNAL **Worker completion is the harness NOTIFICATION plus
  the report file, never a marker grep** (see the status-poll constraint); a
  background `( … ) &` spawn yields no notification at all. @impl/done
- ##WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe/grep**
  (`${PIPESTATUS[0]}` when a tail is unavoidable). Bare `bash tools/self-check.sh`
  in background; the mirror fan-out waits for the READ TAIL. @impl/done
- ##WAL-C-COMMIT-MESSAGE-VIA-FILE **A commit message goes in through a heredoc
  or `-F`, never through `-m "…"` with backticks.** @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps:** Bash-tool cwd PERSISTS — absolute paths
  or `git -C`; **CRLF vs `str.replace` — editor tools only**, and a Python
  rewrite must open with `newline=''` on both ends (verified this session: 8
  status flips, 0 CR introduced); Python on Windows writes CRLF to a redirected
  stdout; a worktree with build artifacts can blow MAX_PATH on removal
  (`-c core.longpaths=true` cleared all seven this session). @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A`;** stage explicit paths. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** (human attribution — no
  AI trailers; Conventional Commits; atomicity; autonomy). A worker is a tool,
  never credited. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **2026-08-05, this run: B-056 BUILT WHOLE, 18 commits.** Five
landings: `fold_sources` over a sequence with `fold_source` as its degenerate
case; the pipeline passing every `#source` in declaration order (**B-055
closed**); the cycle law reaching `#source` through the SAME walker, with the
fold recursive under it AND an include guard the design had not foreseen;
resolver enumeration for the glob, sorted by (name, slot); and the glob reaching
the fold through ONE edge law — plus a fifth landing, the source-only definition
collision, judged in the fold because the gate cannot see it. PROP-035 §7.3, the
design, BACKLOG and TASKS brought level with what was built; two design claims
recorded as refuted; three transport-law facts added. Panel green over the whole
build; conform 0 new. Earlier: both gate holes (B-057, B-058) and registry
hygiene whole, 2026-08-05; волна В whole 2026-08-04/05; волны Б and А whole;
Phase D closed 2026-08-03. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out; all seven session
worktrees removed; tree clean; `main` synced to `gitverse`. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-GITHUB-BEHIND **`github` mirror is BEHIND** — `git@github.com` ssh is
  intercepted at `127.92.0.49` (a loopback address ⇒ local VPN/proxy/hosts).
  **Not a divergence**; a later fan-out catches it up fast-forward. Never
  `--force`. @impl/done
- ##WAL-KI-REGISTRY-DEBT **The registry is behind by a named amount** — 10 facts
  to re-judge in the B-056 design (all moved by the status flip the landing
  earned) plus this slice's new facts to judge fresh; PROP-035 sealable as it
  stands at 146 verdicts. See `#next`. @impl/done
- ##WAL-KI-USE-GRAPH-NEAR-BUDGET **`use_graph.rs` is 590 lines against the
  600-line budget** — the next edit there starts with a split, not with the
  edit. @impl/done
- ##WAL-KI-B059 **Conform's `exclude_substrings` cannot exclude a crate**
  (B-059) — a silent no-op, worked around by literal roots in the mcp policies. @impl/done
- ##WAL-KI-UNIX-HALF-UNCHECKED **The `#[cfg(unix)]` half of the stderr-capture
  cell is not compiler-checked on this box.** @impl/done
- ##WAL-KI-F132 **F-132 names a file that does not exist**
  (`schemas/specmap.jtd.json`); the real debt is that none of the seven report
  schemas carries a spec tag, and the cheap fix would be a tag nothing reads. @impl/done
- ##WAL-KI-DESIGN-DRIFTS **Six drift verdicts stand against documents that
  outlived their subject**; fix surfaces are named in the evidence rows. @impl/done
- ##WAL-KI-OPEN **Open on the owner, none blocking:** B-020's credentials,
  B-024's `disputed` ruling, the privacy tier's content, the own/foreign
  distinction, fork №6. @impl/done
- ##WAL-KI-WT-LEFTOVERS **`.wt/` holds two handle-locked leftovers** from
  earlier sessions (`E14-L4-PARITY-LOOP`, `P-GOFLAG-RULE`) — remove when the
  handles release. This session's seven were all removed cleanly. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**, reads
`CONTINUE.md`, the transport law `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`
WHOLE, `TOOLING-MAP.md` §4–§5, the BACKLOG rows of the next builds — and takes
every number from the commands at the top. `CONTINUE.md` is the cold-resume
snapshot; this file supersedes it. @impl/done
