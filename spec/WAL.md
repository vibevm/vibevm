# WAL — Project Continuation State {#root}

_Updated: 2026-08-05, wind-down №9 (**PHASE E — ВОЛНА В ЗАКРЫТА, ОБЕ ЕЁ ВЕХИ
ВЗЯТЫ.** One long run under the extended mandate closed wave В whole: the one
map-format change (schema 2→3, span + fingerprint), `vibe explain` for the host,
a package carrying its own map under its coordinate, answers about **installed**
packages (**M-ASK**), the fragment view with fingerprint re-check (**M-DRIFT**),
and the two threshold warnings. Two of M-PARITY's four remaining debts also
closed. The owner was in the conversation throughout and **refuted the boss's
reasoning three times — each time the work got SMALLER.** Panel green at every
landing; `gitverse` rolled out, `github` unreachable. NEXT: волна Г, or registry
hygiene, or the five new backlog rows.**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file.** @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
python campaigns/packages-2026-09/tasks/summary.py
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`:
Phase E RUNNING under the EXTENDED mandate (2026-08-04, §7 LOG: «Хочу все
остальные волны сделать»). **Волны А, Б и В ЗАКРЫТЫ.** Live zone
`campaigns/packages-2026-09/`. @impl/done

##WAL-STATE **State at checkpoint** (2026-08-05; the commands supersede):
registry **87 obligations / 176 drift verdicts — 98.1 % confirmed**, confirmed
statements **11 252** — unchanged, because verdicts move only under a re-judging
pass and none ran. Corpus after `scan`: **271 observed files, 13 216 markers,
0 unmarked facts, 0 errors**. Panel GREEN (bare, tail read); conform **24
findings, 0 new** (down from 27 — three were out-of-line test files miscounted
as debt). 14 crates gated / 6 exempt. `gitverse` holds `main`; **`github` is
BEHIND** — ssh intercepted at `127.92.0.49`, not a divergence. @impl/done

## Next {#next}

1. ##WAL-NEXT-WAVE-G **Волна Г** (`TOOLING-MAP.md` `##WAVE-G`): B-040 (census
   cut — `harvest/g1-b040-seams-census.md`; the pointed refactor is boss design
   over it), the F-132 schema debt. **B-005 and B-010 closed this session.** @spec/done
2. ##WAL-NEXT-HYGIENE **Registry hygiene (NOT gating):** five observed files
   await their anchors' judging pass — PROP-035, PROP-029, the design docs.
   Machinery: `vibe progress mirror --campaign <zone>` → `merge-verdicts.py` →
   seal; never chain merge and seal. @spec/done
3. ##WAL-NEXT-NEW-ROWS **Five new backlog rows, three of them CLASSES rather
   than incidents:** B-054 (a test file 13 lines from the budget), B-055 (a
   second `#source` swallowed in silence), B-056 (multiple sources + the plugin
   form — four owner rulings recorded), **B-057** (the discipline engine is not
   pointed at itself — conform never runs over package sources; the debt behind
   the hole is three files), **B-058** (derived artefacts with no freshness
   gate: `vibedeps/` and the write-once `specmap.toml`; B-014 is its third
   instance). @spec/done
4. ##WAL-NEXT-OWNER-OPEN **Waiting on the owner, none blocking:** B-020 (the
   external-LLM client — needs credentials), B-024's `disputed` ruling (census
   cut: 0 carriers, 0 acting consumers, `retired` absent from code entirely),
   the privacy tier's content, the own-project/foreign distinction (deferred),
   fork №6 (query language, deferred at medium priority). @spec/done
5. ##WAL-NEXT-PARITY-DEBT **M-PARITY bar 2 owes TWO, down from four.** Closed:
   row 6 (the Go flag rule) and rows 8/12 (the Go floor's perimeter). Open, both
   owner-recorded deferrals rather than ignorance: `{#b-050}` (Rust dylint / Go
   analysis.Analyzer vehicles) and `{#b-053}` (the Rust deviation-reason text). @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES → confirmed
  when coherent and every referent resolves; DESCRIBES → checked against the
  tree; unexercisable → unverifiable; `world` adds source 2, §3.8 bounds it. @impl/done
- ##WAL-C-MEASURE-BEFORE-ASSERTING **A plausible model of the system is not a
  measurement, and this session paid it three times.** Every one was a coherent
  story wrong in one load-bearing detail: «the consumer renames a package»
  (packages self-declare their namespace, in their own `specmap.toml`); «a map
  inside a package re-pins consumers» (the content hash already covers all
  sources, so the map adds no new trigger); «`:replace` makes a contract
  unpublishable» (an outsider reads the contract under either mode). None was
  caught by review or tests — each was caught by the owner asking «ты уверен?»
  and the boss then measuring. **When a decision rests on how the system
  behaves, measure it before asserting it.** @impl/done
- ##WAL-C-REFUTATION-SHRINKS-WORK **A refuted premise makes the work smaller,
  not larger — expect that and go looking for it.** Owner's three challenges
  deleted a manifest key with its newtype, validation and tests; a migration of
  seven packages; and the whole opt-in rationale. What replaced them was one
  line: derive the namespace from the coordinate the manifest already carries.
  The tree usually already holds what a new mechanism was being invented for. @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline rule is
  never weakened for being unused; an annotation is legitimate only as an
  interim naming a recorded build. A rule that lands vacuous is exhibited on
  fixtures and kept — B-021's thresholds fire 0 and 4 times here and stay. @impl/done
- ##WAL-C-PARITY-IS-THE-INVARIANT-NOT-THE-CODE **Parity means the same
  invariant, never the same code.** The Go flag rule could not copy Rust's — R-001
  keys on a constructor-call fact Go's extractor does not emit — and that was the
  point: Go's own form of the invariant («a cell is imported only by the
  registry») stands on an `import` fact that already existed. **A rule is built
  on data the language HAS.** @impl/done
- ##WAL-C-GATE-RULES-CAN-FIGHT **Two rules of one gate can work against each
  other, and only the panel will say so.** The file-length budget's remedy is to
  move tests out of line; the test-context detector marked a module by walking
  INTO it, and an out-of-line module has nothing to walk into — so the remedy
  silently reclassified tests as domain. Fixed by a cross-file post-process
  (paths by Rust's own module resolution, never by filename). The gate's count
  fell 27 → 24: three findings had been false all along. @impl/done
- ##WAL-C-DERIVED-NEEDS-A-FRESHNESS-GATE **A derived artefact with a producer
  and no freshness gate rots silently — three instances live** (B-058): `vibedeps/`
  (nothing compares them to `packages/`), `specmap.toml` (written by `init`
  through **`write_once`**, discovery included, and never regenerated — hence a
  `0.7.0` root while `0.8.0` was installed, twelve citations resolving into
  nothing), and B-014's committed index. **The cost is not too many reinstalls —
  it is no signal that one is needed**, and an agent with no signal reinstalls
  defensively. Measured: editing package CODE needs no reinstall at all (all 8
  workspace path-deps point into `packages/`, none into `vibedeps/`). @impl/done
- ##WAL-C-FILE-LENGTH-IS-MEASURED-AFTER-FMT **The 600-line budget is a number
  only the merge tail can read**, because workers do not format and the boss
  does. Paid twice in one hour: `mirror.rs` 590 → 608, `floor.rs` 596 → 599.
  A worker reporting «under budget» is reporting a pre-format number; ask for
  the margin, and treat anything within 40 lines as over. @impl/done
- ##WAL-C-SELF-DESCRIBING-FINGERPRINT **The fingerprint names its own scheme**
  (`tok1:<sha256>`): readers compare fingerprints and never parse them, so
  changing the hashed substance later is a REGENERATION, not a second format
  change. That is what let Rust ship the token scheme while Go and TypeScript
  ship nothing at all, each with a recorded reason. @impl/done
- ##WAL-C-ONE-CALCULATOR **One hash, one calculator.** The fingerprint recorded
  in a carried map, the one from a fresh build, and the one recomputed at read
  time are byte-identical — proven end to end. A second implementation of the
  same hash is forbidden even when calling the first is inconvenient. @impl/done
- ##WAL-C-COMMITTED-MAP-IS-BYTE-STABLE **The committed map must not change by a
  byte for a feature.** It deliberately excludes foreign sections and its gate
  rests on byte-reproducibility, so answers about installed packages assemble a
  **second, never-persisted** resolver in memory. Adding foreign units to the
  real map is the forbidden shortcut. @impl/done
- ##WAL-C-OPTIONAL-IS-A-PARITY-DECISION **A field only one scanner can fill is
  OPTIONAL, and that is parity, not convenience.** A required field forces the
  other two to invent a value, and an invented end-of-range is a lie the map
  serves to whoever asks for a fragment. Absent means «this scanner does not
  produce it», visible in the data with its reason beside the scanner. @impl/done
- ##WAL-C-STRUCT-FIELD-RIPPLE **Adding a field breaks every literal
  construction of the struct, optional or not**, and only `--all-targets` finds
  them. Two of three scanners live in other packages compiling against VENDORED
  copies, so they do not break until the sync runs: the landing is two slices
  with the sync between them. @impl/done
- ##WAL-C-MARK-NOT-SUPPRESS **Mark, don't suppress (owner, 2026-08-01).** An
  acknowledged deviation ALWAYS produces a finding stamped
  `DeviationAcknowledged`; `baseline::diff` keeps it out of `new` in ONE place;
  SARIF renders it `inSource`. `Default = Live`. @impl/done
- ##WAL-C-DIVERGENCE-IS-A-FINDING-NOT-A-REFUSAL **A divergence is reported and
  the request is still served.** `--fragment` on drifted code prints both
  fingerprints and returns the text anyway: someone asking for a fragment came
  for the code, and a check people route around protects nothing. @impl/done
- ##WAL-C-PREDICATE-RUN-ON-THE-TREE **A new syntactic predicate is run over the
  LIVE TREE before its rule lands.** Exhibiting a rule on fixtures proves it
  fires; it says nothing about where it fires that it should not. @impl/done
- ##WAL-C-NEVER-FREEZE-A-FALSE-POSITIVE **A false positive is never frozen into
  the baseline.** Freezing turns it into a lie the ratchet then defends. @impl/done
- ##WAL-C-BOSS-GREP-UNDERSTATES **A quick boss grep systematically understates
  the perimeter.** Measure with the machine, or state the perimeter the
  measurement covered. @impl/done
- ##WAL-C-VENDOR-SYNC-ORDER **The six vendored copies are synced by the tool,
  never by hand**, and `sync-engines --check` verifies. Host crates path-dep
  into the package crates, so the copies must match before anything builds. @impl/done
- ##WAL-C-CHARACTERIZATION-COUPLING **A new gate rule breaks EVERY by-rule
  count**, and a rule demanding a compliant exemplar cascades into fixtures AND
  their committed goldens — the Go flag rule cost an untagged export (orphan
  ratchet) and a one-edge-stale fixture index. Mount and fix the counters in the
  SAME landing. @impl/done
- ##WAL-C-CONFIG-V2 **The config surface is per-language.** Root =
  language-neutral; uniform `[rust]`/`[go]`/`[typescript]` sections; retired
  flat keys are loud tombstones; fractality's flat policy is DELIBERATE. @impl/done
- ##WAL-C-ANCHOR-CASE **A `spec://` fragment is spelled as its anchor.** Lookup
  is an exact map get. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror --campaign
  <zone>`**; any anchor-set change requires it before `merge-verdicts.py`;
  never chain merge and seal. Seal REFUSES files carrying unjudged markers. @impl/done
- ##WAL-C-CHECK-NO-LONGER-WRITES **`vibe progress check` is read-only now**
  (B-010, this session): state projections are refreshed by `scan`, or by
  `check --write-state` when explicitly asked. A wind-down that wants true
  numbers runs `scan` first. @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame.** The map's waves execute through
  the campaign's phases; **T/F/G остаются вне добра**; publication is a separate
  operation after the refactor; versions are NOT bumped until the pre-publication
  boundary. @impl/done
- ##WAL-C-NO-PAUSE **The mandate runs autonomously** — stop only on a genuine
  owner map-fork (one at a time), never on a work boundary. @impl/done
- ##WAL-C-SELF-COORDINATE **The host is a package (B-031).** Root identity
  `org.vibevm.core/vibevm`; `spec://vibevm/…` parses and NEVER resolves — and
  five `#[verifies]` tags still cited the dead form after a migration that
  reported residue zero. @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`, ENGINE in
  its crates (vendored ×6), DRIVER per stack CLI, DEPLOYMENT in the consumer. @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**`; historical evidence JSON keeps pre-migration spellings. @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **«Замеров нет и нескоро будет»** — the
  standing answer; never re-raised. @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (binding).** Суть
  по-человечески БЕЗ чтения спек → дерево для развилок (`AskUserQuestion` с
  рекомендацией) → точные имена приложением; спеки не цитировать. Развилки —
  по одной. **И когда владелец говорит «я ничего не понял» — объяснять с нуля,
  а не защищать формулировку.** @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport** (mechanics
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` — read WHOLE; switch
  `SUBAGENT-MODE.toml` re-read before EVERY fan-out, now `claudez`): launchers
  `C:\Users\olegc\opt\bin\{claudez,claudez2}`, per-launcher state dirs, `-c`
  continues its own thread **keyed by (state dir, cwd)** — so a `-c` rework MUST
  run from the worker's own worktree; logs stream DIRECTLY into
  `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`; WORKER-REPORT mandatory.
  Boss scaffolds carry named refinement points AND the right to decline with a
  measured number. Boss states worker log paths in chat at every fan-out. @impl/done
- ##WAL-C-WORKER-JUDGMENT **GLM workers escalate real judgement — honour it.**
  This session: one flagged a command-line-length limit the packet never
  mentioned; one flagged its own file crossing the budget; one found that
  `specmap.json` was already taken as a filename; one reported that the TCG
  oracle uses the very filename heuristic its own fix forbids. **The right to
  decline with a number pays off not in refusals but in workers naming what
  they see out of the corner of their eye.** @impl/done
- ##WAL-C-COMPLETION-SIGNAL **Worker completion is the harness NOTIFICATION,
  never the marker** — grepping the log for `TASK-DONE` matches the packet's own
  instruction text echoed back. @impl/done
- ##WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe/grep.**
  Bare `bash tools/self-check.sh` in background; the mirror fan-out waits for
  the READ TAIL. The panel runs on an UNTOUCHED tree. @impl/done
- ##WAL-C-COMMIT-MESSAGE-VIA-FILE **A commit message goes in through a heredoc
  or `-F`, never through `-m "…"` with backticks** — bash performs command
  substitution inside double quotes, and this session shipped one message with a
  stale binary's error output spliced into it (amended). @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps:** Bash-tool cwd PERSISTS — absolute paths
  or `git -C`; **CRLF vs `str.replace` — editor tools only**; a worktree with
  build artifacts blows MAX_PATH on removal; a patch may not apply after the
  boss formatted the host — copy the worker's files wholesale and re-format. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A`;** stage explicit paths. `git
  diff` omits untracked — a worker's NEW file is hand-carried at merge. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence citing `CONTINUE.md`/`spec/WAL.md`.** `TASKS.md` is cited
  by `file:line` in seven frozen evidence files — its rewrite quotes the two
  cited lines in a tombstone rather than leaving them dangling. @impl/done
- ##WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** (human attribution — no
  AI trailers; Conventional Commits; atomicity; autonomy). A worker is a tool,
  never credited. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **2026-08-04/05, this run: ВОЛНА В CLOSED — 45 commits, both of its
milestones taken.** The one format change (schema 2→3: span + fingerprint,
optional by parity decision, vendored ×6); `vibe explain` giving the host a map
capability it never had, in its own gated crate with CLI and MCP as thin
surfaces; `vibe specmap` letting a package carry a map minted under its
coordinate (**no manifest key — the owner refuted it**); **M-ASK** via a second,
non-committed resolver; **M-DRIFT** via `--fragment` with fingerprint re-check;
two threshold warnings (0 and 4 firings here). M-PARITY: rows 6 and 8/12 closed,
two owner-deferred debts left. Volume Г advanced: B-005 (ancestry) and B-010
(read-only check) closed. Hygiene: `TASKS.md` rewritten, 19 unmarked units
closed (`--exhaustive` zero for the first time), 15 worktrees pruned. Five
backlog rows filed (B-054…B-058). Earlier: волна Б whole (2026-08-04), волна А
whole, Phase D closed 2026-08-03. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out; tree clean; `main`
synced to `gitverse`. `github` is behind for a machine-local network reason. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-GITHUB-BEHIND **`github` mirror is BEHIND** — `git@github.com` ssh is
  intercepted at `127.92.0.49` (a loopback address ⇒ local VPN/proxy/hosts).
  **Not a divergence**; a later fan-out catches it up fast-forward. Never
  `--force`. @impl/done
- ##WAL-KI-OPEN **Open on the owner, none blocking:** B-020's credentials,
  B-024's `disputed` ruling, the privacy tier's content, the own/foreign
  distinction, fork №6 — all deferred by him, not forgotten. @impl/done
- ##WAL-KI-PARITY-DEBT **M-PARITY bar 2 owes two builds**, both recorded
  deferrals — see `##WAL-NEXT-PARITY-DEBT`. @impl/done
- ##WAL-KI-UNSEALED **Observed files await their anchors' judging pass:**
  PROP-035, PROP-029, the design docs (now including `map-format-change.md`). @impl/done
- ##WAL-KI-ENGINE-UNGATED **The discipline engine is not conform-gated** (B-057)
  and derived artefacts have no freshness gate (B-058) — both filed, both cheap
  to close while the debt behind them is still nearly zero. @impl/done
- ##WAL-KI-WT-LEFTOVERS **`.wt/` holds a few handle-locked leftovers**
  (gitignored, pruned from git) — remove when the handles release. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**, reads
`CONTINUE.md`, the transport law `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`
WHOLE, `TOOLING-MAP.md` §4–§5 (waves and the forks each carries), the BACKLOG
rows of the next builds — and takes every number from the two commands at the
top. `CONTINUE.md` is the cold-resume snapshot; this file supersedes it. @impl/done
