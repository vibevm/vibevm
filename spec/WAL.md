# WAL — Project Continuation State {#root}

_Updated: 2026-08-04, wind-down №8 (**PHASE E — ВОЛНА Б ЗАКРЫТА ЦЕЛИКОМ,
зелёная. One long autonomous run closed BOTH remaining batches: batch 3
built three rule classes that existed in no language
(`invariant-comment-position`, `cell-name-is-computed`,
`declared-test-matrices`) plus the TS custom-lint layer, and batch 4 built
the findings model (`FindingStatus` — mark, don't suppress) and the SARIF
ingest. Three of the seven pending cards are authored, each because its
checker exists. The owner took map fork №1 (computed cell names) — 16
renames landed. B-050…B-053 filed. 41 verdicts merged and 11 files
re-sealed. **M-PARITY: the recorded-reason bar IS met; build-completion is
not, and exactly four named gaps stand between them.** Panel green at every
landing; mirrors rolled out. NEXT: ВОЛНА В.**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file.** @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
python campaigns/packages-2026-09/tasks/summary.py
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`:
Phase E RUNNING under the EXTENDED mandate (2026-08-04, §7 LOG: «Хочу все
остальные волны сделать») — волны Б/В/Г целиком, карта задаёт порядок,
развилки владельца — по одной. **Волна Б ЗАКРЫТА ЦЕЛИКОМ** (батчи 1–4);
next is ВОЛНА В.** Live zone `campaigns/packages-2026-09/`. @impl/done

##WAL-STATE **State at checkpoint** (2026-08-04; the commands supersede):
registry **87 obligations / 176 drift verdicts — 98.1 % confirmed**,
confirmed statements **11 252**. Panel GREEN (bare, tail read); conform
reports **27 findings, ALL `DeviationAcknowledged`, 0 live, 0 new** — that
is not debt, it is the new visibility B-025 built. Mirrors ROLLED OUT
(`main @ 414b7224` on gitverse + github, fast-forward-only). Specmap orphan
ratchet at 42. `.wt/` holds seven handle-locked leftover dirs (gitignored,
pruned from git). @impl/done

## Next — ВОЛНА В {#next}

1. ##WAL-NEXT-WAVE-V **Волна В — карта и её потребители** (`TOOLING-MAP.md`
   `##WAVE-V`): B-013 done → **one format change** carrying B-019а
   (fingerprints) + B-016 half 1 (the map ships in the package) + B-017
   (contract fields) TOGETHER — the entries' own one-change rule → B-018.1/.2
   → B-018.4 + B-016.2 → B-020 + B-021; B-024 and B-014 are decided inside the
   wave. **B-020 unlocks the four LEDGER-INTENT interims.** Exit: **M-ASK**
   (the canonical query answered by vibe's own agent interface for an
   INSTALLED package) + **M-DRIFT** (the map notices a code edit before a
   human does). @spec/done
2. ##WAL-NEXT-FORKS **Five owner forks wait inside волна В** (map §5): №3
   fingerprint substance (raw text vs token stream — noise measurement
   FIRST), №4 what a code-side fragment IS, №5 the `contract` privacy tier's
   content, №6 the map query language v0, №7 `disputed`'s fate when the
   lifecycle vocabularies merge. One at a time, tree-shaped,
   `AskUserQuestion` with a recommendation. @spec/done
3. ##WAL-NEXT-WAVE-G **Волна Г parallel-opportunistic:** B-040 (census cut —
   `harvest/g1-b040-seams-census.md`), B-005, the F-132 schemas, B-010's
   check-verb. @spec/done
4. ##WAL-NEXT-PARITY-DEBT **The four gaps between M-PARITY's two bars**, all
   routed, none silent: row 6 (the Go flag/registry rule), rows 8/12 (the Go
   floor's `./...` scoping, B-048's sibling), `{#b-050}` (the Rust dylint and
   Go analysis.Analyzer vehicles — owner-ruled «don't build now, don't drop
   the promise»), `{#b-053}` (the Rust deviation-reason text, cost measured at
   ~33 sites + a frontend version bump, deliberately deferred). @spec/done
5. ##WAL-NEXT-HYGIENE **Registry hygiene still open (NOT gating):** five
   observed files unsealed pending their anchors' judging pass — PROP-035,
   PROP-029, and the design docs. `TASKS.md` is stale (still describes Phase A
   of the registry refactor); `.wt/` leftovers to remove. @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES → confirmed
  when coherent and every referent resolves; DESCRIBES → checked against the
  tree; unexercisable → unverifiable; `world` adds source 2, §3.8 bounds it. @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline rule
  is never weakened for being unused; an annotation is legitimate only as an
  interim naming a recorded build. A rule that lands vacuous (R-060 fires on
  nothing in this tree) is exhibited on fixtures and kept. @impl/done
- ##WAL-C-PREDICATE-RUN-ON-THE-TREE **A new syntactic predicate is run over
  the LIVE TREE before its rule lands, not after.** Paid TWICE in one batch,
  both times correcting the BOSS's design rather than a worker's code: bare
  `NEVER` matched emphasis inside ordinary prose, and «loop nest ≥ 3» matched
  exhaustion of CLOSED enumerations — which is the DECLARED form the rule
  asks for. Exhibiting a rule on fixtures proves it fires; it says nothing
  about where it fires that it should not. @impl/done
- ##WAL-C-NEVER-FREEZE-A-FALSE-POSITIVE **A false positive is never frozen
  into the baseline.** Freezing turns it into a lie the ratchet then defends.
  Fix the rule; the baseline records real debt only. @impl/done
- ##WAL-C-BOSS-GREP-UNDERSTATES **A quick boss grep systematically understates
  the perimeter — three times this session.** A window of 8 lines after
  `#[cell(` dropped three cells whose declaration sat past a doc comment (the
  fork's cost was 16 renames, not 13, and the CHECKER found the rest); counting
  FILES instead of RULES gave 2 suppression sites where there were 6; a census
  scoped to the packages missed the host entirely. Measure with the machine,
  or state the perimeter the measurement covered. @impl/done
- ##WAL-C-MARK-NOT-SUPPRESS **Mark, don't suppress (owner, 2026-08-01).** An
  acknowledged deviation ALWAYS produces a finding, stamped
  `FindingStatus::DeviationAcknowledged`; `baseline::diff` keeps it out of
  `new` in ONE place; SARIF renders it with an `inSource` suppression. Nothing
  drops out of the IR — «нужно всё видеть». `Default = Live`, so a forgotten
  status reds the gate loudly instead of passing quietly. `in_test` is a
  SCOPE exclusion, not a status — the difference is written in the code. @impl/done
- ##WAL-C-COMPUTED-CELL-NAMES **Computed cell names (owner fork №1,
  2026-08-04).** A cell's canonical name is `Pascal(variant)` + the seam
  spelled AS WRITTEN (`SatDepSolver`, never `SatDepsolver`). One engine rule
  serves Rust and Go; TS carries a recorded reason (no cell manifest). No cell
  name is wire-visible — MCP tool names are separate string literals. @impl/done
- ##WAL-C-MARKER-IS-A-LABEL **An invariant marker is a LABELLED TAG, not a
  word in prose** — the colon is the markup signal. `SAFETY:` is excluded on
  purpose: in Rust it must hug its `unsafe` block by language convention, so
  it is block-local justification, not a file-level invariant. @impl/done
- ##WAL-C-FOREIGN-REPORT-IS-NOT-A-GATE **A broken or unfamiliar SARIF report
  is an ABSENCE OF FACTS, never a refusal** — warned visibly, gate proceeds. A
  gate that dies on someone else's malformed output makes a foreign tool a
  single point of failure. A suppressed foreign diagnosis maps onto the
  EXISTING `DeviationAcknowledged` status, never a second vocabulary. @impl/done
- ##WAL-C-VENDOR-SYNC-ORDER **A worker's hand-sync of the six vendored engine
  copies is REQUIRED, not redundant.** Host crates path-dep into the package
  crates, so `cargo xtask sync-engines` itself will not BUILD until the
  vendored copies already match. Merge order: apply the worker's vendor edits
  (including NEW files) → `sync-engines --check` to verify → never revert them
  first «to let the tool regenerate». @impl/done
- ##WAL-C-KIND-VS-VARIANT-RIPPLE **A new engine `Fact` VARIANT is a
  cross-package ripple of FOUR sites** — two exhaustive `Fact` matches (the
  Rust FE total sort, the Rust health census; the engine's own `summary()`
  joined them with B-025) plus two `RawFact` matches (the Go and TS CLI health
  censuses). Adding a FIELD to an existing variant also breaks every
  constructor — only `--all-targets` finds those (E0063). `cargo clean -p`
  kills the stale-fingerprint ghost. A `Fact` KIND is neither. @impl/done
- ##WAL-C-EXTRACTION-CACHE **Extraction is cached per (source content,
  frontend version).** A changed marker SET is not a subset of the old, so the
  version must move even if the rule did not (`MUST:` replacing bare `MUST`
  re-keys every cached fact). Clear fixture `target/` caches before a gate test
  that exercises a changed extractor. @impl/done
- ##WAL-C-SCOPE-ON-NEW-FILES **`specmark::scope!` belongs on every new `.rs`
  whose crate siblings carry one** — not only «engine crates». The narrower
  wording let a new frontend file ship without it. @impl/done
- ##WAL-C-CHARACTERIZATION-COUPLING **A new gate rule breaks EVERY by-rule
  count** (gate counts, TCG parity, extractor record counts) — mount and fix
  the counters in the SAME landing. A rule requiring compliant exemplars
  cascades to fixtures, init templates and goldens. @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport** (mechanics
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` — read WHOLE; switch
  `SUBAGENT-MODE.toml` re-read before EVERY fan-out, now `claudez`): launchers
  `C:\Users\olegc\opt\bin\{claudez,claudez2}`, per-launcher state dirs, `-c`
  continues its own thread; disjoint perimeters → parallel (≤5/launcher, ≤10
  total; cargo-heavy 2–3), intersection → ONE thread; stream-json logs DIRECTLY
  into `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`; WORKER-REPORT
  mandatory and its EXISTENCE part of the set-compare. Boss scaffolds carry
  named refinement points AND the right to decline with a measured number —
  used well twice this session. Boss states worker log paths in chat at every
  fan-out. @impl/done
- ##WAL-C-COMPLETION-SIGNAL **Worker completion is the harness NOTIFICATION,
  never the marker** — and grepping the log for `TASK-DONE` is worse than
  useless: the pattern matches the packet's own instruction text echoed back.
  Reports are routinely written AFTER the marker. @impl/done
- ##WAL-C-WORKER-JUDGMENT **GLM workers escalate real judgment — honour it.**
  This session: one refused to fabricate a spec that did not exist and filed
  the gap (B-051); one measured the cost of a refinement point and declined
  (B-053); one corrected the packet's suppression count from 2 to 6 and
  applied the owner's directive uniformly; one proved a fix on the host
  before/after rather than on fixtures. A flagged judgement is as reviewable
  as code — and often right. @impl/done
- ##WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe/grep.**
  Bare `bash tools/self-check.sh` in background; the mirror fan-out waits for
  the READ TAIL. The panel runs on an UNTOUCHED tree. @impl/done
- ##WAL-C-PACKAGE-FMT **The fmt/vendor reach after package edits:** host
  `cargo fmt --all` does NOT cover package workspaces — fmt per manifest;
  `cargo xtask sync-engines` from the HOST root the same pass; `vibe install`
  rematerialises vibedeps. Workers don't fmt. Fixtures sit outside the Go
  floor's gofmt scope by design (B-003) — a gofmt-driven blank line would
  shift exhibit line numbers the arithmetic depends on. @impl/done
- ##WAL-C-PARITY-LAW **The parity principle is discipline law** (manifesto
  `##PARITY-ACROSS-PROJECTIONS` + `##PARITY-GAP-IS-NEVER-SILENT`): a gap
  carries a recorded reason AND a route. **An INVERSION is a gap too** — TS
  gaining the lint layer Rust and Go lack is recorded exactly like any other. @impl/done
- ##WAL-C-CONFIG-V2 **The config surface is per-language.** Root =
  language-neutral (`max_file_lines`, the invariant-marker vocabulary, the
  length floor, `sarif_reports`) + uniform `[rust]`/`[go]`/`[typescript]`
  sections; retired flat keys are loud tombstones; fractality's flat policy is
  DELIBERATE. @impl/done
- ##WAL-C-ANCHOR-CASE **A `spec://` fragment is spelled as its anchor.**
  Lookup is an exact map get (`doctree.rs`), so lowercase belongs to headings
  (which declare `{#anchor}`) and a MARKER's fragment is its own spelling.
  Three dangling parity citations were fixed this session. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror --campaign
  <zone>`**; any anchor-set change requires it before `merge-verdicts.py`;
  never chain merge and seal. Seal REFUSES files carrying unjudged markers. @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame.** The map's waves execute
  through the campaign's phases; **T/F/G остаются вне добра**; publication is
  a separate operation after the refactor; versions are NOT bumped until the
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
  рекомендацией) → точные имена приложением; спеки не цитировать. Развилки
  карты — по одной. @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps:** Bash-tool cwd PERSISTS — absolute
  paths; **CRLF vs `str.replace` — editor tools only**; a worktree with build
  artifacts blows MAX_PATH on removal (`cmd //c "rd /s /q …"` → `git worktree
  prune`; a handle-lock leaves a gitignored `.wt/<id>` behind — harmless);
  a stale cargo fingerprint compiles fixed code against a dead rmeta. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A`;** stage explicit paths. `git
  diff` omits untracked — a worker's NEW file is hand-carried at merge. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** (human attribution — no
  AI trailers; Conventional Commits; atomicity; autonomy). A worker is a tool,
  never credited. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **2026-08-04, this run: ВОЛНА Б CLOSED WHOLE — 48 commits.**
Batch 3: the S4-citation defect fixed (three dangling parity fragments), three
censuses, the design sketch `spec/design/new-rule-classes.md`, fork №1 taken
and executed (16 renames), `invariant-comment-position` +
`cell-name-is-computed` + `declared-test-matrices` built end to end with three
cards authored, the TS lint plugin, two panel-driven predicate corrections.
Batch 4: `FindingStatus` (27 acknowledged findings made visible on the host)
and the SARIF ingest with a live citation form. Registry: 41 verdicts merged,
11 files re-sealed, 11 252 confirmed statements. Parity passes №3 and №4;
B-050…B-053 filed. Earlier: батчи 1–2 (2026-08-04), волна А whole, Phase D
closed 2026-08-03. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out; tree clean; `main`
synced to every mirror. The next session opens on ВОЛНА В — no re-asking, the
mandate is live. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-MIRRORS-SYNCED **Mirrors synced** — `cargo xtask mirror` rolled
  `main @ 414b7224` out this wind-down, fast-forward-only. @impl/done
- ##WAL-KI-OPEN **Open on the owner, none blocking:** five map forks inside
  волна В (№3 fingerprints, №4 fragment identity, №5 contract tier, №6 query
  language, №7 `disputed`); F-129; the H-roster; the pre-publication boundary. @impl/done
- ##WAL-KI-PARITY-DEBT **M-PARITY bar 2 owes four builds** — see
  `##WAL-NEXT-PARITY-DEBT`. Bar 1 (recorded reason) is MET. @impl/done
- ##WAL-KI-RATCHET **Specmap orphan ratchet at 42.** @impl/done
- ##WAL-KI-UNSEALED **Observed files await their anchors' judging pass:**
  PROP-035, PROP-029, the design docs. @impl/done
- ##WAL-KI-TASKS-STALE **`TASKS.md` is stale** — it still describes Phase A of
  the decentralized-registry refactor, months past. The campaign runs on
  `BATCH-PLAN.md`; the root checklist needs a rewrite or an honest tombstone. @impl/done
- ##WAL-KI-WT-LEFTOVERS **Seven handle-locked `.wt/` dirs** (gitignored,
  pruned from git) — remove when the handles release. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**, reads
`CONTINUE.md` (the волна В recipe), the transport law
`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` WHOLE, the latest parity
table `harvest/e14-b035-parity-pass.md`, `TOOLING-MAP.md` §4–§5 (waves and the
five forks волна В carries), the BACKLOG rows of the next builds — and takes
every number from the two commands at the top. `CONTINUE.md` is the cold-resume
snapshot; this file supersedes it. @impl/done
