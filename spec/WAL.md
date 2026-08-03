# WAL — Project Continuation State {#root}

_Updated: 2026-08-04, checkpoint (**PHASE E — ВОЛНА Б БАТЧ 2 ПОСТРОЕН
ЦЕЛИКОМ, зелёный, 15 коммитов; остались доки/луп/пересуд до закрытия
батча. One long autonomous run built the whole parity batch: the engine
gains `go-seam-error-cites-req` (both halves), `ts-seam-error-cites-req`,
`go-conformance-assertion` (gated), two new `Fact` variants + the
`seam_error_message_no_req` kind + `[rust] floor_disable`; the extractors
emit them (go `Error()` bodies + `var _` assertions, ts discriminated-union
errors); all three rules MOUNTED and exhibited on the fixtures; B-049's
Rust floor honours `floor_disable`; the parity principle is LIFTED into the
manifesto. Panel green at every landing. NOT yet done: S4 doc re-narration
(the guides still promise things now built), the B-035 loop pass №2, and
the F-185 family re-judgement.**)_

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
развилки владельца — по одной. Волна Б: батч 1 ЗАКРЫТ; **батч 2 ПОСТРОЕН
ЦЕЛИКОМ** (B-033 Go+TS seam-error, B-030 Go conformance + Rust/TS verdicts,
B-049 floor twin, the parity-principle lift) — remaining to CLOSE batch 2:
S4 docs, B-035 loop №2, F-185 re-judge.** Live zone
`campaigns/packages-2026-09/`. @impl/done

##WAL-STATE **State at checkpoint** (2026-08-04; the commands supersede):
registry **88 obligations / 179 drift verdicts — owed 6** (UNCHANGED — no
judging batch ran yet; F-185's family re-judgement is a pending step, and
it needs S4 first). Panel GREEN (bare, tail read) after the last landing.
**Mirrors are BEHIND: the 15 batch-2 commits are local `main` only — the
mirror fan-out (`cargo xtask mirror`) has NOT run this run (held for an
explicit wind-down).** Specmap orphan ratchet at 42 (host side untouched;
the two new engine files carry `scope!`). Five observed files still stand
unsealed pending their anchors' judging pass: PROP-035, PROP-029, the three
design docs (now four with `seam-error-and-assertion-parity.md`). @impl/done

## Next — closing batch 2, then the Б/В/Г mandate {#next}

1. ##WAL-NEXT-S4 **S4 — the guides speak the built state (the doc debt).**
   The guides still carry «Specified, not built» notes for things now BUILT:
   `GUIDE-AI-NATIVE-GO.md:192` (`##CONFORMANCE-IS-MADE-LOUD`, `@impl/plan` →
   the go-conformance-assertion rule polices gated cells now) and the Go
   seam-error contract (`conform-frontend-go.md:41`, both halves built; the
   message half's marker is `spec://` OR `violates REQ`). The TS guide's
   seam-error clause gains an honest note (the `ts-seam-error-cites-req` rule
   + its recorded limits: Form-1 union, name-based error position, closed
   `{kind,tag,_tag}` discriminant). And the parity principle (manifesto
   `##PARITY-ACROSS-PROJECTIONS`) is CITED by the three guides (fork №9's
   «stacks cite»). **The delegated S4 worker returned EMPTY (echoed
   TASK-DONE, no edits, no report — the §8 report-skip failure) — re-do it
   (boss-side or re-commission).** Non-gating (panel is green without it),
   but F-185's re-judge depends on it. @spec/done
2. ##WAL-NEXT-LOOP **B-035 loop pass №2 — re-cut the parity table** by the
   fact of the tree after batch 2 (`harvest/e10-b035-parity-pass.md` is
   pass №1; write pass №2). Rows that now CLOSE: **row 1** (seam-error
   REQ-citation — all three check it: Rust 2 rules, Go 1 rule both halves,
   TS 1 rule); **row 7** (conformance-assertion — Go built gated, Rust
   recorded [compiler], TS routed [type-level tests, parity debt]); **row
   13** (floor-disable — all three now, B-049 closed the inversion).
   Remaining open: row 6 (Go flag/registry rule — batch 3), rows 8/12 (Go
   floor `vet`/`tests`/`staticcheck` residual — B-048), the record-reason
   rows 9/10/11 (narrate in the guides). @spec/done
3. ##WAL-NEXT-F185 **F-185 family re-judgement (B-033 completes the
   family)** — AFTER S4 (the guides must speak the built state first, else
   F-185's anchors stay drift): `vibe progress mirror --campaign <zone>` →
   `merge-verdicts.py` → seal, **never chained** (seal takes explicit
   PATHs). Then registry numbers move (F-185's promises now match reality).
   @spec/done
4. ##WAL-NEXT-BATCH-34 **Batch 3 → batch 4 → M-PARITY.** Батч 3
   (B-036 invariant-comment position + B-037 custom REQ-citing lints +
   B-038 pending cards; **map fork №1 computed-names arrives WITH B-038** —
   stop for the owner one at a time). Батч 4 (B-025 mark-don't-suppress →
   F-146's last anchor; B-026 SARIF ingest → F-206). Exit: **M-PARITY**
   (the B-035 table shows no language cell weaker than Rust without a
   recorded reason). @spec/done
5. ##WAL-NEXT-WAVES **Волна В then Г.** В: B-013 done → the one format
   change (B-019а + B-016.1 + B-017; B-024 alongside) → B-018.1/.2 →
   B-018.4 + B-016.2 → B-020 + B-021 (B-014 there); B-020 unlocks the four
   LEDGER-INTENT interims. Exit M-ASK + M-DRIFT. Г parallel-opportunistic:
   B-040 (seams census cut — `harvest/g1-b040-seams-census.md`), B-005,
   F-132 schemas, B-010 check-verb. @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES → confirmed
  when coherent and every referent resolves; DESCRIBES → checked against the
  tree; unexercisable → unverifiable; `world` adds source 2, §3.8 bounds it. @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline rule
  is never weakened for being unused; an annotation is legitimate only as an
  interim naming a recorded build. @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame.** The map's waves execute
  through the campaign's phases. The 2026-08-04 mandate covers волны Б/В/Г
  whole; **T/F/G остаются вне добра**; publication is a separate operation
  after the refactor ends; versions are NOT bumped until the pre-publication
  boundary (mint + publication = one operation). @impl/done
- ##WAL-C-NO-PAUSE **The 2026-08-04 pause is LIFTED** (the continuation
  prompt lifted it); the mandate runs autonomously — stop only on a genuine
  owner map-fork (the eleven named decisions, one at a time), never on a
  work boundary. @impl/done
- ##WAL-C-PARITY-LAW **The parity principle is now discipline law**
  (manifesto `##PARITY-ACROSS-PROJECTIONS`, §4): no language projection
  enforces the discipline more weakly than another; a gap carries a recorded
  reason, never silent. The projection-level twin of `##BAN-WITHOUT-HATCH-IS-A-BUG`. @impl/done
- ##WAL-C-CONFIG-V2 **The config surface is per-language (B-029+B-034,
  live).** Root `conform.toml` = `max_file_lines` + `[rust]`/`[go]`/
  `[typescript]` of ONE uniform shape (`roots`, `exclude_substrings`, neutral
  `gated`, `[[<lang>.exempt]] {unit, reason}`, `floor_disable` as
  `[[<lang>.floor_disable]] {step, reason}` — **now on Rust too, B-049**);
  units = each language's native one (crate / package / cell); retired flat
  keys are LOUD TOMBSTONES; the fractality package's flat policy is DELIBERATE
  (frozen 0.7.0 slot). @impl/done
- ##WAL-C-SEAM-RULES **The batch-2 rules (live).** `go-seam-error-cites-req`
  (one Go rule, both halves, per-half fingerprints `…-structure`/`…-message`;
  the message half fires on the **Error() method line**, distinct from the
  structure half's type-decl line, so the gate keys them separately by
  (file,line)); `ts-seam-error-cites-req` (Form-1 discriminated-union errors,
  name-based error position, `{kind,tag,_tag}` discriminant — limits recorded);
  `go-conformance-assertion` (polices **gated** cells only — seam-less/exempt
  cells are out; the extractor emits `go_conformance` from `var _ Seam =
  (*Impl)(nil)`). @impl/done
- ##WAL-C-MESSAGE-MARKER **The Go seam-error message half's REQ marker is
  `spec://` OR `violates REQ`** — NOT bare `spec://` (Go renders the URI from
  the `Spec` FIELD, so the format string carries only the `violates REQ %s`
  marker; a literal-`spec://` check would false-red the correct idiom). Bare
  `REQ` is too loose (matches REQUEST) — the exact marker is `violates REQ`. @impl/done
- ##WAL-C-KIND-VS-VARIANT-RIPPLE **A new engine `Fact` VARIANT is a
  cross-package ripple** (grep the WHOLE tree for exhaustive `Fact` matches +
  the RawFact matches in the bridges' consumers — the Rust FE total sort, the
  three health censuses; budget arms in every frontend; `cargo clean -p` kills
  the stale-fingerprint ghost). A new `Fact` KIND (a string value on an
  existing variant) is neither ripple nor cache-invalidator. @impl/done
- ##WAL-C-EXTRACTION-CACHE **Extraction is cached per (source content,
  frontend version).** Changing the extractor's LOGIC (a new kind, changed
  detection) WITHIN a frontend version does NOT invalidate the cached facts —
  a real project re-extracts only when a Fact-VARIANT bump moves the frontend
  version (S1's variants do this cross-batch). Locally, a fixture's stale
  cache masks new facts: **clear the fixture `target/` caches before a gate
  test that exercises a changed extractor** (`rm -rf …/fixtures/*/target`). @impl/done
- ##WAL-C-CHARACTERIZATION-COUPLING **Moving a finding-kind out of an
  umbrella into its own rule breaks EVERY by-rule count test** (the gate
  count, the TCG finding-parity) — mount the new rule AND update every count
  in the SAME landing. **A new gate rule that requires compliant exemplars
  cascades** to fixtures (add the idiom), init templates, goldens (regenerate)
  and every characterization test — plan the cascade, don't discover it
  per-panel. Specmap golden regen: `run_specmap_go(root, false)` writes; no
  CLI — use a throwaway bless test in `tests/`, then delete it. @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport** (mechanics
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` — read WHOLE; switch
  `SUBAGENT-MODE.toml` re-read before EVERY fan-out, now `claudez`): launchers
  `C:\Users\olegc\opt\bin\{claudez,claudez2}`, per-launcher state dirs, `-c`
  continues its own thread; disjoint perimeters → parallel (≤5/launcher, ≤10
  total; cargo-heavy 2–3), intersection → ONE thread; stream-json logs
  DIRECTLY into `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`;
  ~30 s polls (log growth primary); WORKER-REPORT mandatory, **its EXISTENCE
  part of the set-compare** (the S4 worker echoed TASK-DONE with no report and
  no edits — always verify artifacts, never the string). Boss scaffolds carry
  named refinement points; verdicts/commits — boss; every cycle in `meta.md`;
  workers get no git verbs. **§8 carries SIX paid facts** (report existence,
  clippy -D, bare panel + tail, wc-l ≤ 600, new-engine-files carry `scope!`,
  the enum ripple). **Boss states worker log paths in chat at every
  fan-out.** @impl/done
- ##WAL-C-WORKER-JUDGMENT **GLM workers escalate real judgment well — honour
  it.** This batch: the go worker caught the packet's `spec://`-only marker
  (Go's field-substitution idiom needed `violates REQ` too); it caught the
  packet's `floor_disable: Vec<String>` contradiction (Go/TS use
  `Vec<FloorDisable>` — building the bare form would have violated the parity
  principle this batch LIFTS); the B-049 worker caught the packet's `tests` vs
  `test` step-name error; the ts worker recorded its heuristic limits and
  cleaned up `package-lock.json` unprompted. A worker's flagged judgment is as
  reviewable as its code — and often right. @impl/done
- ##WAL-C-REAL-EXITS **Exit codes are read REAL, never through a pipe/grep.**
  Bare `bash tools/self-check.sh` in background, exit = the task's; the mirror
  fan-out waits for the READ TAIL, never a notification. The panel runs on an
  UNTOUCHED tree — applying diffs mid-panel invalidates the run. @impl/done
- ##WAL-C-PACKAGE-FMT **The fmt/vendor reach after package-crate edits:**
  host `cargo fmt --all` does NOT cover the package workspaces — fmt per
  package manifest; `cargo xtask sync-engines` from the HOST root the same
  pass (vendored engine ×6 + the lang↔mcp twins — extractors, bridges, CLIs,
  tests, fixtures all twin); `vibe install` rematerialises vibedeps after
  canonical package edits (vibedeps DOES carry the engine crates under
  `flow-/stack-/mcp-` names). Workers don't fmt (gofmt the `.go` too). @impl/done
- ##WAL-C-CENSUS-NOT-PROOF **A census/reader-table is evidence, never a
  completeness proof.** The merge plan greps the whole tree; the panel's
  package-workspace sweep is the real perimeter check. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror --campaign
  <zone>`**; any anchor-set change requires it before `merge-verdicts.py`;
  never chain merge and seal. Seal REFUSES files carrying unjudged markers;
  `vibe progress seal` takes explicit PATHs. @impl/done
- ##WAL-C-SELF-COORDINATE **The host is a package (B-031, live).** Root
  identity `org.vibevm.core/vibevm`; `spec://vibevm/…` parses and NEVER
  resolves; never reintroduce the old form; the one legacy fixture is
  `concat!` on purpose. @impl/done
- ##WAL-C-DEFERRED-IS-OWNER-RULED **`deferred` in the registry = an
  owner-ruled row.** The gate reads owed + rulings. @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`, ENGINE
  in its crates (vendored ×6), DRIVER per stack CLI, DEPLOYMENT in the
  consumer; `legacy-spec/**` excluded. @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**`; historical evidence JSON keeps pre-migration
  spellings BY DESIGN. @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **«Замеров нет и нескоро будет»** — the
  standing answer; never re-raised. @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (binding).** Суть
  по-человечески БЕЗ чтения спек → дерево для развилок (AskUserQuestion с
  рекомендацией работает) → точные имена приложением; спеки не цитировать.
  Развилки карты — по одной. @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps that fired:** Bash-tool cwd PERSISTS —
  absolute paths; **CRLF vs `str.replace` — editor tools only**; a worktree
  with build artifacts / node_modules blows MAX_PATH on removal (`rm -rf` →
  `cmd //c "rd /s /q …"` → `git worktree prune`; a handle-lock can leave a
  gitignored `.wt/<id>` dir behind — harmless, remove later); a stale cargo
  fingerprint compiles fixed code against a dead rmeta — `cargo clean -p`;
  a structural Edit can EAT an adjacent heading. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A`;** stage explicit paths (`.wt/`
  is untracked-not-ignored in status — a blind `-A` would stage the worktree).
  `git add -u` misses NEW files; `git diff` omits untracked — a worker's NEW
  file is hand-carried at merge (check `status --short` for `??`). @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-ATTRIBUTION **Rules 1–4 bind every commit** (human attribution — no
  AI trailers; Conventional Commits; atomicity/topic-grouped; autonomy). A
  worker is a tool, never credited. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **2026-08-04, this run (batch 2 BUILD):** 15 green commits.
Design sketch (`3c5f51e5`) → parity-principle lift into the manifesto
(`8e03348a`) → S1 engine spine (`ae927800`…`c0f99902`: the three rules + two
Fact variants + the `seam_error_message_no_req` kind + `[rust] floor_disable`
field, the Fact-variant ripple across the Rust FE sort + health census, the
go-seam mount, the config field) → S2 extractors (`8f1fc914` go `Error()`
bodies + `var _`, `a5ba2b0b` ts discriminated-union errors, `0393ce91` sync)
→ S3 mounts + B-049 (`bd4291d5` the conformance rule scopes to the gate list,
`d09e2a19` go mounts conformance + clean fixture asserts, `f63c1d32` ts mounts
seam-error, `32aba0ab` the Rust floor honours floor_disable, `94e6db0e` sync).
Every landing panel-green (the panel earned its keep on the go
characterization coupling — the seam-error move + message half broke the gate
count and the TCG parity three times; each a same-landing fix). 3 claudez
workers accepted (2 with boss corrections that were the worker's own correct
escalations); 1 doc worker (S4) returned empty and is a re-do. Earlier:
батч 1 whole (2026-08-04), волна А whole, Phase D closed 2026-08-03. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out, `.wt/` holds one
handle-locked leftover dir (`E12-S4-DOCS`, gitignored, pruned from git —
remove when the handle releases), tree clean, `main` 15 commits ahead of the
mirrors. The next session opens at S4 (the doc re-narration the failed worker
did not do), then the B-035 loop №2, then the F-185 re-judge — no re-asking;
the mandate is live. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-S4-REDO **S4 doc re-narration is owed** (the delegated worker
  returned empty). Guides still promise built things as «not built»
  (`GUIDE-AI-NATIVE-GO.md:192` `@impl/plan`; the Go/TS seam-error notes) and
  the parity principle is not yet cited by the three guides. Non-gating;
  blocks the F-185 re-judge. @impl/done
- ##WAL-KI-MIRRORS-BEHIND **Mirrors are 15 commits behind** — `cargo xtask
  mirror` has not run this run (held for an explicit wind-down / batch close). @impl/done
- ##WAL-KI-OPEN **Open on the owner, none blocking:** nine remaining map forks
  (№1 arrives with B-038 in batch 3); F-129; the H-roster; the pre-publication
  boundary. @impl/done
- ##WAL-KI-RATCHET **Specmap orphan ratchet at 42** (host side untouched; the
  two new engine files carry `scope!`). @impl/done
- ##WAL-KI-UNSEALED **Observed files await their anchors' judging pass:**
  PROP-035, PROP-029, and the four design docs (lane-composition-dedup,
  host-as-package, gate-parity-config, seam-error-and-assertion-parity). @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**, reads
`CONTINUE.md` (the batch-2-close recipe), the transport law
`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` WHOLE, the parity table
`harvest/e10-b035-parity-pass.md` (loop №1 — write №2), the design sketch
`spec/design/seam-error-and-assertion-parity.md`, the BACKLOG rows of the
next builds — and takes every number from the two commands at the top.
`CONTINUE.md` is the cold-resume snapshot; this file supersedes it. @impl/done
