# WAL — Project Continuation State {#root}

_Updated: 2026-08-05, wind-down №10 (**PHASE E — BOTH GATE HOLES CLOSED,
REGISTRY HYGIENE CLOSED WHOLE.** The owner set the order — gate holes →
hygiene → B-056 → волна Г — and the first two are done. The discipline engine
now runs over its own package sources (B-057); the installed copies carry a
freshness signal and a declared-but-absent spec root announces itself (B-058);
and the campaign's registry reached **272 files, 0 stale, 0 unjudged** for the
first time. B-056's build design is authored and judged; the build itself is
NEXT.**)_

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
остальные волны сделать»). **Волны А, Б, В ЗАКРЫТЫ; волна Г частично.** Live
zone `campaigns/packages-2026-09/`. @impl/done

##WAL-STATE **State at checkpoint** (2026-08-05; the commands supersede):
registry **91 obligations / 182 drift verdicts**, confirmed statements
**11 531** (up from 11 252 — the hygiene pass landed 279). Corpus after
`scan`: **272 observed files, 13 240 markers, 0 unmarked facts, 0 errors, 0
stale, 0 unjudged**. Panel GREEN (bare, tail read). Conform now runs over all
seven live package workspaces: **8 crates gated, 20 exempt-with-reason, 6
findings, 2 frozen**. `gitverse` holds `main`; **`github` is BEHIND** — ssh
intercepted at `127.92.0.49`, not a divergence. @impl/done

## Next {#next}

1. ##WAL-NEXT-B056 **B-056 — the build, over the design authored this session**
   ([`spec/design/multiple-sources-and-plugins.md`](design/multiple-sources-and-plugins.md)).
   Four landings, each standing alone: *(1)* `fold_sources` over an explicit
   list with `fold_source` as its degenerate case and every existing test
   passing unchanged; *(2)* the pipeline stops at `.find(…)`
   (`crates/vibe-spec/src/pipeline.rs:221`) and passes every `#source` in
   declaration order — **this alone closes B-055**; *(3)* the fold's cycle
   guard and dedup by EXTENDING `use_graph`'s walker, never a second one;
   *(4)* resolver enumeration for the glob, sorted. @spec/done
2. ##WAL-NEXT-WAVE-G **Волна Г, what is left** (`TOOLING-MAP.md` `##WAVE-G`):
   B-040 (the pointed seam refactor over `harvest/g1-b040-seams-census.md`,
   boss design) and the F-132 schema debt — **whose target file does not
   exist**; the measured debt and why the cheap fix is a wish are recorded in
   `TASKS.md`. B-005 and B-010 closed 2026-08-04. @spec/done
3. ##WAL-NEXT-OWNER-OPEN **Waiting on the owner, none blocking:** B-020 (the
   external-LLM client — needs credentials), B-024's `disputed` ruling, the
   privacy tier's content, the own/foreign distinction (deferred), fork №6
   (query language, deferred at medium priority). @spec/done
4. ##WAL-NEXT-PARITY-DEBT **M-PARITY bar 2 owes TWO**, both owner-recorded
   deferrals: `{#b-050}` (Rust dylint / Go analysis.Analyzer vehicles) and
   `{#b-053}` (the Rust deviation-reason text). @spec/done
5. ##WAL-NEXT-B059 **B-059, filed this session:** conform's
   `exclude_substrings` matches the crate-relative path while the finding it
   prints is repo-relative, so an exclusion written from a finding's address
   is a silent no-op. Three fix shapes recorded; the cheapest is a warning
   when a `/`-bearing pattern excludes nothing. @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES → confirmed
  when coherent and every referent resolves; DESCRIBES → checked against the
  tree; unexercisable → unverifiable; `world` adds source 2, §3.8 bounds it. @impl/done
- ##WAL-C-STALENESS-OVER-REPORTS **A document-level staleness signal
  over-reports by orders of magnitude, and re-reading everything it flags is
  theatre.** A file goes stale when ANY byte moves — a `<status>` comment, a
  line in a fenced block, prose no fact owns. Measured: **1214 flagged
  verdicts, 19 facts actually moved.** `tasks/text-stability.py` takes the
  commit that was HEAD at `verified_at` and byte-compares each judged fact's
  own paragraph. A clean result licenses the SEAL (the verdict was formed
  against exactly today's text) and **never a claim of freshness** — a claim
  whose wording never moved can still have drifted because the code under it
  moved, and only judging catches that. @impl/done
- ##WAL-C-AN-INSTRUMENT-CAN-LIE-QUIETLY **An instrument's blind spot fails in
  the dangerous direction, so re-verify every decision it made after fixing
  it.** `text-stability.py` missed list facts (`- ##ID`), then numbered ones
  (`5. ##ID`): such a fact folded into its neighbour and **compared equal**, so
  a moved fact could be sealed as stable. Both gaps were caught the same day —
  one by a seal refusal naming facts the extractor called absent, one by a
  specific anchor. After each fix EVERY seal of the session was re-checked
  against the pre-seal cache; ten held, one did not, and that one was restated
  with the blind spot recorded in its evidence row. @impl/done
- ##WAL-C-DEBT-IS-NAMED-NOT-FROZEN **A gate landing names its debt with a
  number instead of freezing it.** Pointing conform at the packages found 134
  findings, not the recorded «almost zero». None was frozen: each policy gates
  the crates that are already clean and lists every other crate `exempt` WITH
  its finding count — the host's own expand-as-you-conform posture. The
  ratchet baseline is for real, drainable debt (2 file-length findings), never
  for hiding a class. @impl/done
- ##WAL-C-MARK-NOT-SUPPRESS **Mark, don't suppress (owner, 2026-08-01).** An
  acknowledged deviation ALWAYS produces a finding stamped
  `DeviationAcknowledged`; `baseline::diff` keeps it out of `new` in ONE
  place. The four `unsafe` blocks in the stderr-capture cell carry
  `#[spec(deviates)]` testimony rather than a baseline entry — conform still
  reports them and still exits zero. @impl/done
- ##WAL-C-SIGNAL-NOT-WALL **A freshness check is a SIGNAL, not a gate.**
  Editing package code needs no reinstall (all workspace path-deps point into
  `packages/`), so an error would redden the panel routinely and teach people
  to route around it. `local-source-freshness` warns, names both hashes and
  the remedy, and `vibe check` still exits 0. Exercised live: it named exactly
  the five installed packages whose sources moved and went quiet after the
  reinstall. @impl/done
- ##WAL-C-SILENCE-IS-THE-DISEASE **When something legitimate goes unnoticed,
  cure the silence, not the state.** A declared `[[external_specs]]` root that
  is absent is a LEGITIMATE «not yet installed» state — the resolution layer
  says so in its own test, and four live entries are in it. The first build
  made it a hard load error; the fix was a loud warning at the same place,
  matching the engine's own idiom (it warns for a defaulted policy and for a
  vacuously gated crate). @impl/done
- ##WAL-C-A-KEY-THAT-MATCHES-A-DIFFERENT-STRING **A config key filtering on a
  different string than the tool prints is a silent no-op** (B-059):
  `exclude_substrings` matches the CRATE-relative path (`src/lib.rs`) while a
  finding prints the repo-relative one. An exclusion written from a finding's
  address can never match, and nothing says so. @impl/done
- ##WAL-C-DERIVED-PERIMETER-NEEDS-A-DERIVED-GUARD **Naming a perimeter
  literally costs the guard a glob gave for free, so derive the guard.** The
  mcp policies name `roots` literally (the glob would scan vendored copies and
  the exclusion cannot stop it), which loses the gated-or-exempt check on a
  NEW crate. Panel step 10c restores it by DERIVING each slot's authored set —
  the tree minus `vendor/` minus what `sync-engines.toml` vendors in. Nothing
  is spelled; a version bump moves with the manifest. @impl/done
- ##WAL-C-VENDOR-INVISIBILITY-MECHANISM **The vendored engines are invisible
  to the conform scan because `crates/vendor/` owns no `src/`/`tests/`, NOT
  because it lacks a `Cargo.toml`.** The glob pushes EVERY subdirectory; the
  Cargo.toml check lives only in unit classification. The conclusion was right
  and the stated reason was wrong — a worker's measurement corrected it. @impl/done
- ##WAL-C-MEASURE-BEFORE-ASSERTING **A plausible model of the system is not a
  measurement.** This session paid it three more times: the vendor-invisibility
  mechanism, the exclusion key that excludes nothing, and a hard error where
  the layer below deliberately tolerates the state. Each was caught by
  measuring, never by review. @impl/done
- ##WAL-C-REFUTATION-SHRINKS-WORK **A refuted premise makes the work smaller.**
  The mcp policy lost 22 entries when the exclusion mechanism was refuted; the
  specmap error shrank to a warning and dissolved a layer conflict; the
  recursion ruling turned out to be an existing law needing reach, not a new
  rule. @impl/done
- ##WAL-C-EVIDENCE-IS-NOT-A-VERDICT **Workers gather evidence; the boss
  writes every verdict, and the split is mechanical.** Every gathered row
  arrives stamped `v: "PENDING"`, which `merge-verdicts.py` refuses — evidence
  cannot become a verdict by being filed. A worker can also cite a file that
  does not exist WITH a byte count (paid on `schemas/specmap.jtd.json`), so a
  row is corrected in place rather than accepted or silently swapped. @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline rule is
  never weakened for being unused; an annotation is legitimate only as an
  interim naming a recorded build. @impl/done
- ##WAL-C-PARITY-IS-THE-INVARIANT-NOT-THE-CODE **Parity means the same
  invariant, never the same code** — a rule is built on data the language HAS. @impl/done
- ##WAL-C-ONE-IMPLEMENTATION-PER-LAW **One hash, one calculator — and one
  cycle law, one walker.** `use_graph` already implements the no-deadlock
  invariant for `#use`; B-056's fold must EXTEND its reach to `#source`, never
  grow a second traversal. @impl/done
- ##WAL-C-GATE-RULES-CAN-FIGHT **Two rules of one gate can work against each
  other, and only the panel will say so.** @impl/done
- ##WAL-C-FILE-LENGTH-IS-MEASURED-AFTER-FMT **The 600-line budget is a number
  only the merge tail can read**, because workers do not format and the boss
  does. @impl/done
- ##WAL-C-SELF-DESCRIBING-FINGERPRINT **The fingerprint names its own scheme**
  (`tok1:<sha256>`): changing the hashed substance later is a REGENERATION,
  not a second format change. @impl/done
- ##WAL-C-COMMITTED-MAP-IS-BYTE-STABLE **The committed map must not change by a
  byte for a feature** — answers about installed packages assemble a second,
  never-persisted resolver in memory. @impl/done
- ##WAL-C-OPTIONAL-IS-A-PARITY-DECISION **A field only one scanner can fill is
  OPTIONAL, and that is parity, not convenience.** @impl/done
- ##WAL-C-STRUCT-FIELD-RIPPLE **Adding a field breaks every literal
  construction of the struct**, and only `--all-targets` finds them; two of
  three scanners compile against VENDORED copies. @impl/done
- ##WAL-C-DIVERGENCE-IS-A-FINDING-NOT-A-REFUSAL **A divergence is reported and
  the request is still served.** @impl/done
- ##WAL-C-PREDICATE-RUN-ON-THE-TREE **A new syntactic predicate is run over the
  LIVE TREE before its rule lands.** @impl/done
- ##WAL-C-NEVER-FREEZE-A-FALSE-POSITIVE **A false positive is never frozen into
  the baseline.** @impl/done
- ##WAL-C-BOSS-GREP-UNDERSTATES **A quick boss grep systematically understates
  the perimeter** — and can also MISS on a wrong pattern: `vibe:static git-`
  returned 0 where the real form is `vibe:static <group>/git-…`. State the
  perimeter the measurement covered. @impl/done
- ##WAL-C-VENDOR-SYNC-ORDER **The six vendored copies are synced by the tool,
  never by hand**, and `sync-engines --check` verifies. @impl/done
- ##WAL-C-CHARACTERIZATION-COUPLING **A new gate rule breaks EVERY by-rule
  count**; a new check cell breaks the cell oracle's counts and the
  `all_checks()` doctest. Mount and fix the counters in the SAME landing. @impl/done
- ##WAL-C-CONFIG-V2 **The config surface is per-language.** Root =
  language-neutral (`max_file_lines`, the invariant-comment keys); uniform
  `[rust]`/`[go]`/`[typescript]` sections; the gate key is the **neutral
  `gated`** in an idiomatic home; retired flat keys are loud tombstones. @impl/done
- ##WAL-C-ANCHOR-CASE **A `spec://` fragment is spelled as its anchor.** @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror --campaign
  <zone>`**; any anchor-set change requires it before `merge-verdicts.py`;
  **never chain merge and seal.** Seal REFUSES a file whose markers are not all
  judged — that refusal is a feature and it caught two files this session. @impl/done
- ##WAL-C-CHECK-NO-LONGER-WRITES **`vibe progress check` is read-only**;
  projections are refreshed by `scan` or `check --write-state`. @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame.** The map's waves execute through
  the campaign's phases; **T/F/G остаются вне добра**; publication is a separate
  operation after the refactor; versions are NOT bumped until the
  pre-publication boundary. @impl/done
- ##WAL-C-NO-PAUSE **The mandate runs autonomously** — stop only on a genuine
  owner map-fork (one at a time), never on a work boundary. @impl/done
- ##WAL-C-SELF-COORDINATE **The host is a package (B-031).** Root identity
  `org.vibevm.core/vibevm`; `spec://vibevm/…` parses and NEVER resolves;
  `HOST_NAMESPACE` is gone from `crates/` entirely. @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`, ENGINE in
  its crates (vendored ×6), DRIVER per stack CLI, DEPLOYMENT in the consumer. @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**`; historical evidence JSON keeps pre-migration spellings. @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **«Замеров нет и нескоро будет»** — the
  standing answer; never re-raised. @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (binding).** Суть
  по-человечески БЕЗ чтения спек → дерево для развилок (`AskUserQuestion` с
  рекомендацией) → точные имена приложением; спеки не цитировать. Развилки —
  по одной. **И когда владелец говорит «я ничего не понял» — объяснять с нуля.** @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport** (mechanics
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` — read WHOLE; switch
  `SUBAGENT-MODE.toml` re-read before EVERY fan-out, now `claudez`): launchers
  `C:\Users\olegc\opt\bin\{claudez,claudez2}`, per-launcher state dirs, `-c`
  keyed by (state dir, cwd) — **so a `-c` rework MUST run from the worker's own
  worktree, and ONLY after that run has ended** (a mid-flight `-c` starts a
  second writer on the same files). Logs stream into
  `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`; WORKER-REPORT
  mandatory; boss states log paths in chat at every fan-out. @impl/done
- ##WAL-C-PACKET-CITES-ONLY-WHAT-GIT-CARRIES **A packet may only cite what git
  carries.** `campaigns/*/run/mirror/` is gitignored, so a fresh worktree lacks
  it; both hygiene workers were pointed at files that did not exist on their
  side. Check `git check-ignore` before citing a generated artifact and copy it
  in at provisioning time. @impl/done
- ##WAL-C-NO-DELIVERABLE-IS-NOT-NO-WORK **«No deliverable» is not «no work» —
  look at the disk before re-commissioning.** A worker echoed `TASK-DONE` twice
  with no evidence file, having written a complete 312-line generator carrying
  real evidence rows and never run it. The boss audited the script and ran it. @impl/done
- ##WAL-C-WORKER-JUDGMENT **GLM workers escalate real judgement — honour it.**
  This session one refuted the boss's stated mechanism for vendor invisibility,
  one proved the exclusion key inert with an arithmetic argument, and one
  surfaced a layer disagreement the packet had asserted away. @impl/done
- ##WAL-C-COMPLETION-SIGNAL **Worker completion is the harness NOTIFICATION,
  never the marker** — and a background `( … ) &` spawn yields no notification
  at all, so judge by artifacts. @impl/done
- ##WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe/grep.**
  Bare `bash tools/self-check.sh` in background; the mirror fan-out waits for
  the READ TAIL. Paid twice this session by piping through `tail`. @impl/done
- ##WAL-C-COMMIT-MESSAGE-VIA-FILE **A commit message goes in through a heredoc
  or `-F`, never through `-m "…"` with backticks.** @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps:** Bash-tool cwd PERSISTS — absolute paths
  or `git -C`; **CRLF vs `str.replace` — editor tools only** (a python rewrite
  of `TASKS.md` introduced 64 CRLF into a pure-LF file); Python on Windows
  writes CRLF to a redirected stdout, so a path list must be `tr -d '\r'`-ed
  before use; a worktree with build artifacts blows MAX_PATH on removal. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A`;** stage explicit paths. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** (human attribution — no
  AI trailers; Conventional Commits; atomicity; autonomy). A worker is a tool,
  never credited. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **2026-08-05, this run: BOTH GATE HOLES + HYGIENE, 20 commits.**
B-057 closed — conform over all seven live package workspaces, one binary,
policy per slot, 134 findings named rather than frozen, four `unsafe` marked
as acknowledged deviations, and a DERIVED authored-crate denominator for the
mcp slots. B-058 closed — a `local-source-freshness` cell needing no new panel
step and no new machinery, plus a loud warning for an absent external spec
root. B-059 filed. The B-056 build design authored and judged. Registry
hygiene closed whole: `text-stability.py` reduced 1214 flagged verdicts to 19
real ones, 178 verdicts judged over the never-judged files, **272 files, 0
stale, 0 unjudged**, six drifts found. F-132's debt measured and found to name
a file that does not exist. Earlier: волна В whole (2026-08-04/05, M-ASK +
M-DRIFT), волна Б whole, волна А whole, Phase D closed 2026-08-03. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out; tree clean; `main`
synced to `gitverse`. `github` is behind for a machine-local network reason. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-GITHUB-BEHIND **`github` mirror is BEHIND** — `git@github.com` ssh is
  intercepted at `127.92.0.49` (a loopback address ⇒ local VPN/proxy/hosts).
  **Not a divergence**; a later fan-out catches it up fast-forward. Never
  `--force`. @impl/done
- ##WAL-KI-B059 **Conform's `exclude_substrings` cannot exclude a crate**
  (B-059) — a silent no-op, worked around by literal roots in the mcp policies. @impl/done
- ##WAL-KI-UNIX-HALF-UNCHECKED **The `#[cfg(unix)]` half of the stderr-capture
  cell is not compiler-checked on this box**, so its four deviation attributes
  never faced the compiler that cleared the windows half. Three supports lower
  the risk; recorded in `harvest/g2-b057-conform-debt-lang.md` §6.4. @impl/done
- ##WAL-KI-F132 **F-132 names a file that does not exist**
  (`schemas/specmap.jtd.json`); the real debt is that none of the seven report
  schemas carries a spec tag, and the cheap fix would be a tag nothing reads. @impl/done
- ##WAL-KI-DESIGN-DRIFTS **Six drift verdicts stand against documents that
  outlived their subject** — most in `spec/design/map-format-change.md`, whose
  manifest-key proposal the owner refuted and whose census finding the build
  falsified. Fix surfaces are named in the evidence rows. @impl/done
- ##WAL-KI-OPEN **Open on the owner, none blocking:** B-020's credentials,
  B-024's `disputed` ruling, the privacy tier's content, the own/foreign
  distinction, fork №6. @impl/done
- ##WAL-KI-WT-LEFTOVERS **`.wt/` holds a handle-locked leftover**
  (gitignored, pruned from git) — remove when the handle releases. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**, reads
`CONTINUE.md`, the transport law `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`
WHOLE, `TOOLING-MAP.md` §4–§5, the BACKLOG rows of the next builds — and takes
every number from the commands at the top. `CONTINUE.md` is the cold-resume
snapshot; this file supersedes it. @impl/done
