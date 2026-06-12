# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-12 at session end (the second session of this day).
**The SHRINK PLAN is executed to completion**: all six phases of
[`spec/terraforms/SHRINK-PLAN-v0.1.md`](spec/terraforms/SHRINK-PLAN-v0.1.md)
ran the same day the plan was authored — conform baseline **130 → 10**
(the owner-gated unsafe octet + the two MCP-parked files, exactly the
plan's §0 exit arithmetic), the five-gate panel green on the final
tree, ~20 commits (`254b974` … `d39769b`) pushed. The session also
recorded the **session-resume contract** in the `CLAUDE.md` trio:
«восстанови сессию» restores state, reports, and STOPS — the owner
steers from there._

> **`spec/WAL.md` is the canonical living state.** If this snapshot
> and the WAL disagree, the WAL wins. Boot first (`CLAUDE.md` →
> `spec/boot/INDEX.md` → its files, including the two installed
> Discipline snippets → `spec/WAL.md`), then read this.

---

## TL;DR

The owner set the goal «SHRINK-PLAN-v0.1.md должен быть выполнен до
конца», and it was: Phase 0 (hygiene + `GitBackend` doctest +
frontend v4 deviates-awareness), Phase 1 (R-001 wiring of
Registry-cell construction), Phase 2 (all 24 unwrap sites — 18
restructures, 3 honest conversions, 3 `#[spec(deviates)]`
testimonies; the rule's count is **zero**), Phase 3 (all 68 error
messages in the Class-F grammar via four parallel agents; **zero**),
Phase 4 (all 26 active over-budget files ≤ 600 physical lines via six
parallel agents, ~40 new modules; `file-length` = 2, the parked MCP
pair), Phase 5 (the `PackageScanner` seam: trait + two `#[cell]`
variants + seam-driving oracles; **cell-has-oracle green at 20
cells**). Three plan predictions were falsified and recorded in place
(the stale-trio premise, the ≥1/3 deviates rate); four held (≤50-line
Phase 1, <10 broken expectations in Phase 3, tests-out ≥10 in Phase
4, zero new test files in Phase 5). Earlier the same day (previous
session): the PROP-013 depth audit and the depth program itself.

## Where work stands

- **Branch `main` @ `d39769b`** (plus this checkpoint's commits), in
  sync with `origin/main`; working tree clean. (`new`,
  `m1.17-workspace` remain as retained merged branches.)
- **No active blocker. No self-running next unit remains** — the
  shrink plan was the queue, and it is drained. Everything that
  follows is owner-gated (see "Owner court" below); the natural
  candidate for the next working session is **authoring the
  gate-expansion plan** (`CONFORM_GATED` → vibe-core / vibe-index),
  which the shrink plan explicitly reserved as the NEXT plan's
  opening move.
- **Gate panel, all green on this tree** (each gate's own exit code):
  `cargo xtask specmap --check` — 442 units / 394 items / 404 edges /
  0 suspects / 0 gated orphans (10 DBT-0020 dispositions, 7 exempt
  crates); `cargo xtask conform check` — 10 frozen / 0 new (9 rules;
  residual = 8 unsafe-gate + 2 MCP file-length); `cargo xtask
  test-gate` — 1123 results / 0 failed / 3 skipped, xfail-strict;
  `cargo xtask fast-loop --enforce-budget` — 18/18 < 60s;
  `bash tools/self-check.sh` — fmt, workspace tests, clippy -D
  warnings, `vibe check` 0/0/0.

## Session-resume contract (NEW — read before resuming)

`CLAUDE.md` (and its identical `AGENTS.md` / `GEMINI.md` copies) now
carries a **Session-resume command** section: «ВОССТАНОВИ СЕССИЮ» /
«RESUME SESSION» means boot fully, verify repository state
empirically, **report in chat, and stop**. No code edits, no plan
execution, no commits, no pushes until the owner sets direction. Any
"next step" named in this file is a *candidate for the report*, not
an authorisation.

## Candidate next steps (for the resume report)

1. **Author the gate-expansion plan** (the reserved next move):
   `CONFORM_GATED` grows to vibe-core and vibe-index, freezing their
   error enums / seams / unwraps as the new ratcheted queue.
   vibe-index is pre-paid: it already carries two cells, a seam
   doctest, and oracle tests from Phase 5.
2. **Owner-court items** (unchanged, any time): the history-rewrite
   question (what re-hashed the adoption-day chain — audit -01 rider,
   still unanswered); publishing the two Discipline packages;
   production solver selection (R-001 flag `solver=sat`); the
   PROP-010 design session; DBT-0020 (MCP spec home; two files parked
   behind it — the baseline's residual file-length pair); the four
   open-instrument predictions; the PROP-014 external-namespace
   amendment — **new input from this session**: unwrap testimonies
   cite `ENGINE-CONFORM-v0.1#rules` because `discipline://` is not
   addressable in specmark's grammar; Discipline v0.3 inputs.
3. The unsafe-gate octet (AUD-0016) stays an owner decision; the
   audit-crate designation would drain it.

## Non-obvious findings (this session)

- **Measure line counts with the gate's own counter.** The plan's
  "stale trio" premise came from `Measure-Object -Line` (skips
  blanks); the conform rule counts `text.lines().count()` (physical).
  566/556/554 vs 609/612/608 — the prune was a no-op and the trio was
  real Phase-4 work. Probe empirically; never trust a number whose
  measuring tool you can't name.
- **The conform frontend parses files standalone.** A
  `#[cfg(test)] #[path = "<stem>/tests.rs"] mod tests;` include keeps
  the module tree identical, but the scanner never sees the parent's
  cfg-gate — non-`#[test]` fixture items in the moved file leak
  `no-unwrap-in-domain` facts. House device: wrap fixtures in an
  inner `#[cfg(test)] mod fixtures { … }` (+ re-export). Every
  Phase-4 tests-out file uses it.
- **Frozen unsafe-gate ordinals pin code in place.** All four
  `unsafe-gate|output.rs|block#N` fingerprints live in *test-only*
  env guards; moving them to the new tests file would mint four new
  findings. The guards stayed in `output.rs` as `#[cfg(test)]` items
  in original order.
- **The deviates grammar is stricter than the discipline's sketch.**
  `#[spec(deviates, reason)]` (the guide's shorthand) does not parse;
  specmark requires `deviates = "<spec://uri>"` + mandatory `reason`,
  and the URI must resolve to a live unit (suspect check). The
  spec-side home chosen for unwrap testimonies:
  `spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules` (the unit that
  defines deviation-acknowledgment). Frontend v4 honors the verb at
  **fn grain only** — impl/struct/mod-level deviates (the live
  solver-choice edges) grant no unwrap amnesty.
- **A freeze never legalises new findings.** The first 2a freeze
  would have written 3 grammarless new variants into the baseline
  (+3/−11); the baseline was `git restore`d, the messages fixed to
  carry the grammar, and only then re-frozen (−11/+0). New error
  variants must be *born* in the Class-F grammar.
- **Two "invariants" weren't.** `Manifest::validate()` provably does
  not check var-dep names (the old expect's claim was unverified →
  real user-reachable error, new `BadVarDepRef` variant), and
  `RedirectSection`'s pub fields admit pinned-without-pinned_ref
  programmatic construction (→ `MalformedMeta`). Also fixed latent:
  `VersionReq::parse("={version}")` panics on build metadata —
  replaced with typed `semver::Comparator` construction.
- **`pub(super)` items cannot be re-exported wider** (E0364) — the
  Phase-4c children use `pub(in crate::commands::registry)`.
- **CellHasOracle detects references via Import/Ctor facts** in
  `crates/<crate>/tests/` — importing the cell type in the test file
  satisfies it; struct-literal construction alone does not (no Ctor
  fact without `::new`).
- Machine quirks (carried over, still true): Windows UAC blocks
  test exes named \*install\* (PROP-007 §9.5); PowerShell 5.1
  corrupts UTF-8-no-BOM round-trips (edit via tools only; `git
  restore` to recover); `bash` in PowerShell resolves to WSL —
  `tools/self-check.sh` runs through Git Bash; PS 5.1 wraps native
  stderr in fake NativeCommandError noise (check `$LASTEXITCODE`,
  not the red text); one transient `cargo test` exit 255 with all
  binaries green resolved on re-run.

## Repository map (delta vs the depth-program era)

```
vibevm/
├── spec/terraforms/SHRINK-PLAN-v0.1.md   ← status: EXECUTED (header carries the record)
├── conform-baseline.json                 ← 10 entries: 8 unsafe-gate + 2 MCP (all owner-gated)
├── CLAUDE.md / AGENTS.md / GEMINI.md     ← + Session-resume command section
├── crates/
│   ├── vibe-registry/src/
│   │   ├── git_package_registry/{mod,auth,urls,fetch,lookup,test_support}.rs
│   │   │                                 ← fetch split; package_urls returns (primary, mirrors)
│   │   ├── multi_registry_resolver/{mod,walk,dispatch,redirect_follow,sources,refresh,test_support}.rs
│   │   └── */tests.rs                    ← tests-out idiom everywhere (#[path] + fixtures mod)
│   ├── vibe-index/src/scanner/           ← THE NEW SEAM
│   │   ├── mod.rs                        ← trait PackageScanner + doctest
│   │   ├── from_clones.rs                ← cell: PackageScanner/from-clones
│   │   ├── from_github.rs                ← cell: PackageScanner/from-github
│   │   └── org_walk.rs                   ← the shared walk (ex-from_clones.rs, renamed with history)
│   ├── vibe-cli/src/
│   │   ├── cli.rs + cli/{registry,pkg,mcp,inspect,workspace}.rs   ← arg families
│   │   ├── output.rs + output/tests.rs   ← unsafe guards pinned in output.rs
│   │   └── commands/
│   │       ├── install/{mod,pipeline,planning,recording,resolver,tests}.rs
│   │       ├── registry/redirect/{mod,create,sync,update,tests}.rs
│   │       ├── registry/config/{mod,list,add,mirror,remove,test,tests}.rs
│   │       ├── show/{mod,effective,config,features,subskills,purls}.rs
│   │       ├── workspace/{mod,publish,origin,tests}.rs
│   │       └── search.rs + search/purl.rs
│   ├── vibe-workspace/src/{lib,expand}.rs + publish/{staging,tests}.rs
│   ├── vibe-publish/src/{lib,creator,orchestrator}.rs
│   ├── conform-core/src/rules/{mod,structure,diagnostics,budget,tests}.rs
│   ├── conform-frontend-rust/            ← v4: UnwrapUse.in_deviation (fn-grain)
│   └── vibe-core/src/manifest/document/tests.rs, package_ref/tests.rs
├── xtask/src/main.rs + {codemod,codegen,conform,fast_loop,tripwire,test_gate,specmap,trace}.rs
└── crates/vibe-cli/src/commands/mcp.rs   ← still 2638 lines, PARKED (DBT-0020)
```

## Decisions in force (this session's additions, long form)

- **The resume boundary is the owner's steering point.** Resume
  commands restore and report only; execution starts when the owner
  says so. (`CLAUDE.md` §Session-resume command; born from this
  morning's misfire.)
- **Deviates-testimony target policy:** pure-invariant unwrap escapes
  cite `ENGINE-CONFORM-v0.1#rules` with a reason naming the rule and
  the invariant; fn-grain only. An unrelated deviation on a wider
  item never exempts what's inside it.
- **The tests-out idiom is fixed:** production file keeps its place;
  `#[cfg(test)] #[path = "<stem>/tests.rs"] mod tests;` + the moved
  body wraps its fixtures in `#[cfg(test)] mod fixtures`. One idiom,
  used by every Phase-4 file.
- **New error variants are born in the Class-F grammar** — the rule
  catches grammarless newcomers, and the freeze must never widen.
- **The shrink plan's residual is owner court by design:** the unsafe
  octet (AUD-0016) and the MCP pair (DBT-0020) park in the baseline
  until their owner decisions land.
- All prior-era decisions stand (four rules; spec-first flow; no CI
  by owner decision; xfail-strict; derived data regenerated, never
  hand-edited; `specmap.json` regenerated with every unit/tag/line
  move; MCP parked; baseline shrink-only with freeze + diff review).

## Recent commit chain (newest first; all 2026-06-12, this session)

```
d39769b docs(wal): shrink-plan checkpoint - executed to completion
475fa75 feat(index): the PackageScanner seam - two scanner cells manifested
eb21099 chore(conform): freeze the Phase-4 baseline - the file debt is drained
c79d685 refactor(publish): batch 4f - publish lib splits, core sheds tests
f420afc refactor(conform): batch 4e - rules split per family, naive sheds tests
c89c9aa refactor(cli): batch 4d - the arg hub, output, and xtask decompose
15babfa refactor(cli): batch 4c - six command files become module families
31886c2 refactor(workspace): batch 4b - four files under the budget
7ebcae4 refactor(registry): batch 4a - seven files under the 600-line budget
52067e3 chore(conform): freeze the Phase-3 baseline - the message debt is gone
7747f1a refactor(workspace): batch 3d - 11 messages enter the Class-F grammar
56832f5 refactor(resolver): batch 3c - 16 messages enter the Class-F grammar
9902980 refactor(publish): batch 3b - 18 messages enter the Class-F grammar
77b3f8b refactor(registry): batch 3a - 23 messages enter the Class-F grammar
729578d refactor(registry): batch 2b - the unwrap ban reaches zero
01d9eaa refactor(workspace): batch 2a - eleven unwrap sites leave the domain
ad714fc refactor(cli): Registry-cell construction moves into the registry
9fa6d54 docs(boot): the resume command restores state and stops
30639e6 feat(conform): frontend v4 - fn-grain deviates testimony on unwraps
83c0b7f docs(registry): GitBackend seam doctest - baseline sheds its entry
254b974 docs(spec): the stale-trio premise is falsified - trio to Phase 4
158806c docs(wal): session-end checkpoint                  (prior session)
10e2d64 docs(continue): session-end cold-resume checkpoint (prior session)
```

(The session-end checkpoint commits for this file and the WAL land
immediately after `d39769b`.)

## Quick-start

```sh
cargo xtask specmap --check              # index + orphan ratchet
cargo xtask conform check                # facts → 9 rules → SARIF → baseline
cargo xtask conform freeze               # rewrite baseline (legality: new rule or shrink, diff-reviewed)
cargo xtask test-gate                    # nextest, xfail-strict
cargo xtask fast-loop --enforce-budget   # 18 cells < 60s
bash tools/self-check.sh                 # via Git Bash, NOT WSL
cargo xtask trace explain <symbol|uri> [--text|--json|--prose]
```

Session-resume phrase: `восстанови сессию` — **restores state and
reports, then waits for the owner's direction** (the CLAUDE.md
contract). The WAL supersedes this snapshot wherever they diverge.
