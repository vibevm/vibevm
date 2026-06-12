# vibevm Shrink Plan v0.1 — drain the depth-program baseline
**status: EXECUTED 2026-06-12 (same day as authored) · vibevm-specific · the work queue behind `conform-baseline.json`**

*Execution record: all six phases ran to the exit state in one session — baseline **130 → 10** (the owner-gated unsafe octet + the two MCP-parked files, exactly §0's arithmetic). 19 commits (`254b974` … `475fa75`). Three predictions falsified and recorded in place: the stale-trio premise (§0 correction), the ≥1/3 deviates rate (3/24 actual — restructures dominated), and nothing else; the Phase-1 ≤50-line, Phase-3 <10-expectations, Phase-4 tests-out ≥10, and Phase-5 zero-new-test-files predictions all held. The deviates testimony target settled on `spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules` (the grammar admits only resolvable spec:// units; the ban itself lives in the package guide outside the specmap). Cells 18 → 20. The WAL carries the full checkpoint.*

*The 2026-06-12 depth program ended with a deliberate artifact: a 130-entry conform baseline that IS the remaining debt, enumerated and frozen shrink-only. This plan drains it. Owner scope decision recorded up front: the MCP debt (DBT-0020 and both MCP-owned files) is NOT touched by this plan — it waits for an MCP spec home. The unsafe-gate octet is likewise out of scope (AUD-0016 — the audit-crate designation is an owner decision).*

*Executed as per-crate batches, each gated green and re-frozen via `cargo xtask conform freeze` with a diff review proving the baseline only shrank. Same rhythm as the depth program: centralized gates verify, batches land as topic commits.*

---

## 0. Target arithmetic

Baseline at plan time: **130** = 68 `error-message-cites-req` + 28 `file-length` + 24 `no-unwrap-in-domain` + 8 `unsafe-gate` + 1 `seam-has-doctest` + 1 `R-001`.

Exit state: **10** = 8 `unsafe-gate` (owner-gated, out of scope) + 2 `file-length` (the parked MCP pair: `commands/mcp.rs` 2460, `vibe-mcp/src/tools.rs` 681 — splitting them would mint new modules with no honest `scope!` target until DBT-0020 resolves).

Everything else reaches zero. **Correction (2026-06-12, Phase 0 execution):** the authoring premise that three `file-length` entries were stale (`commands/search.rs` 566, `output.rs` 556, `git_registry.rs` 554 "post-fmt") is **falsified** — those figures were non-blank-line counts; the rule counts physical lines (`text.lines().count()` in the frontend), and the real sizes are 609 / 612 / 608. The trio is genuinely over budget and moves to Phase 4, whose active set is **26** files, not 23. The Phase-0 freeze was a zero-diff no-op, as the empirical check proved.

## 1. Phase 0 — hygiene and one-shots (one sitting)

- **Prune the stale trio — executed and falsified (2026-06-12)**: `cargo xtask conform freeze` produced a zero diff; the trio is live at 609/612/608 physical lines (the 566/556/554 premise measured non-blank lines). No prune; the three files join Phase 4's batches (§0 correction).
- **`GitBackend` doctest** (`git_backend/mod.rs`): one compiled example of canonical use — constructing a backend and the trait-object shape consumers hold. Kills the lone `seam-has-doctest` entry. The trait has six methods and one production impl; the doctest shows the seam, not the plumbing (a `no_run` shell-git example is acceptable; an in-memory fake is better if cheap).
- **Frontend v4 — deviates-aware `UnwrapUse`**: the `no-unwrap-in-domain` escape hatch is `#[spec(deviates, reason)]` (guide §6), but the v3 facts cannot see it. The extractor gains a `deviating_depth` (same shape as `test_depth`): inside an item whose attrs carry `spec(deviates …)`, `UnwrapUse` facts set `in_deviation: true`; the rule skips them. Version bump retires v3 slots. This is the prerequisite for Phase 2's (b)-arm — without it, "legitimate boundary" and "unconverted debt" are indistinguishable in the baseline.

*Exit (revised 2026-06-12):* baseline 130 → 129 — the `GitBackend` doctest is Phase 0's only shrink; all five panel gates green.
*Prediction:* the prune is pure shrink — **falsified**: the freeze was zero-diff because the findings are live, not stale (measurement-method mismatch, recorded per the instrument discipline).

## 2. Phase 1 — R-001 wiring: Registry-cell construction leaves the veins (one sitting)

The frozen `R-001|commands/install.rs|LocalRegistry` finding records that `InstallResolver` constructs a Registry cell outside the selection registry. The fix follows the existing pattern exactly: `vibe-cli/src/registry.rs` already owns `dep_solver(flags, ProviderResource)` with the one `match`; it gains the Registry-side constructor (`install_resolver(...)` or a widened `ProviderResource` arm) so `commands/install.rs` receives the built cell and constructs nothing. No new flag — Registry selection stays config-driven (manifest `[[registry]]`/`--registry` decide); R-001's point is *where the construction site lives*, not that a flag must exist.

*Exit:* baseline −1 (R-001 = 0); `cargo test -p vibe-cli` green; the install e2e suite (`cli_pkg_cycle`) green.
*Prediction:* the move is ≤50 lines net and deletes the `InstallResolver`-vs-`registry.rs` duplication the audit's vibe-cli read flagged.

## 3. Phase 2 — the 24 unwrap sites: convert or testify (two batches)

Judgment rule per site, in priority order: **(a)** real fallibility → route through the layer's error enum, and the new variant's message is born already in the Phase-3 grammar (write it once); **(b)** a true invariant (unreachable by construction) → `#[spec(deviates, reason = "…")]` on the carrying fn, which the v4 frontend now honors; **(c)** test-support code that leaked into `src/` → move under `cfg(test)`.

| Batch | Sites | Expectation |
|---|---|---|
| **2a** vibe-workspace (10: boot_artifacts 3, publish 3, lib 2, freshness 1, vibedeps 1) + vibe-publish (git_publish 1) | 11 | mostly (a) — IO/serde edges with real failure modes |
| **2b** vibe-registry (fetch 3, redirect_follow 1, lib 1) + vibe-resolver (conditional 2, sat 1, naive 1) + conform-core (rules 2, sarif 1) + specmap-core (testgate 1) | 13 | resolver/solver sites are prime (b) candidates — "the naive checker validated this branch" is a construction invariant; conform-core's two are BTreeMap entry-pattern artifacts, trivially (a) |

*Exit:* `no-unwrap-in-domain` = 0 in the baseline; every (b) site carries a reason a reviewer can argue with.
*Prediction:* ≥1/3 of the 24 land as (b) deviates — the ban's value here is the *testimony*, not the conversion count.

## 4. Phase 3 — the 68 messages: Class-F grammar (four crate batches)

**The product-error grammar, fixed here once:** the human text stays first and unchanged in tone; the machine tail is appended in parentheses:

```
#[error("registry root `{0}` does not exist or is not a directory \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#registry-model; \
          fix: check the [[registry]] url or pass --registry <dir>)")]
```

`violates <spec-uri>; fix: <one actionable hint>` — the same two loads the conform diagnostics carry, in product clothing. The URI cites the unit the variant's *contract* comes from (the enum's `#[spec]` edge is the default; a variant may cite a more precise unit).

| Batch | Enums | Count |
|---|---|---|
| **3a** vibe-registry | RegistryError 11, GitError 9, IndexError 3 | 23 |
| **3b** vibe-publish | PublishError 13, HookError 5 | 18 |
| **3c** vibe-resolver | FeatureError 4, DepProviderError 4, SolveError 4, TagError 2, PredicateError 2 | 16 |
| **3d** vibe-workspace | WorkspaceError 11 | 11 |

Each batch: grep tests for asserted message substrings FIRST (`assert!(…contains(…))` and golden transcripts under `terraform/golden/`), update expectations in the same commit; the goldens must fail loudly and be re-captured deliberately, never auto-updated (R-040 discipline).

*Exit:* `error-message-cites-req` = 0; `vibe check` and one live error path eyeballed per crate (message reads sane to a human, not URI soup).
*Prediction:* fewer than 10 test expectations break across all 68 edits — most tests match discriminants, not strings.

## 5. Phase 4 — the 26 active over-budget files (six crate batches)

Two levers, in order of preference:

- **Tests-out** (cheap, structure-preserving): an inline `#[cfg(test)] mod tests` moves to a sibling file in the same cell file-set (`foo.rs` → `foo.rs` + `foo/tests.rs` with `#[cfg(test)] #[path]` mod, or the directory form). The cell's single-registration-point property is untouched.
- **Responsibility split** (where the production half alone exceeds 600): the file genuinely holds two cells' worth of meaning — split along the seam, every new module carrying the parent's `scope!` URI (the depth-program recipe).

| Batch | Files (current lines) | Lever |
|---|---|---|
| **4a** vibe-registry | fetch.rs 1406 · walk.rs 1077 · shell.rs 1021 · mrr/mod.rs 628 · gpr/mod.rs 626 · sources.rs 611 · git_registry.rs 608 | tests-out first (the four split children are test-heavy by construction; git_registry.rs has its inline mod at :341 — prime tests-out, cell file-set form); shell.rs splits preflight/classify/ops if needed |
| **4b** vibe-workspace | publish.rs 1057 · lib.rs 887 · boot_artifacts.rs 761 · install.rs 748 | tests-out first; publish.rs likely also splits staging vs selection |
| **4c** vibe-cli commands | redirect.rs 1457 · install.rs 1234 · config.rs 900 · show.rs 899 · workspace.rs 898 · search.rs 609 | redirect.rs splits create/sync/update; install.rs splits along its ten pipeline stages (and inherits Phase 1's slimming); show/workspace/config per-subcommand; search.rs has no inline tests — needs a real seam look (report assembly vs querying is the likely cut) |
| **4d** vibe-cli core + xtask | xtask/main.rs 1193 · cli.rs 1014 · output.rs 612 | xtask per-subcommand modules; cli.rs per command-family arg structs; output.rs inline mod at :371 — prime tests-out |
| **4e** conform + resolver | rules.rs 1097 · naive.rs 753 | rules.rs per rule family (structure/diagnostics/budget); naive.rs is a manifested cell — tests-out only, the solver body stays one file |
| **4f** vibe-publish + vibe-core | lib.rs 798 · git_publish.rs 696 · document.rs 675 · package_ref.rs 614 | publish lib.rs splits token/creator-trait/orchestrator (RepoCreator seam intact); the two vibe-core files are tests-out candidates |

Every batch: build + crate tests + clippy + `conform check` (scope the crate) + re-freeze with shrink-only diff; specmap regen rides each batch (line moves).

*Exit:* `file-length` = 2 (the MCP pair), explicitly annotated in the baseline's note field if one is added, else in the WAL.
*Predictions:* the tests-out lever alone clears ≥10 of 26 (output.rs and git_registry.rs joined as prime candidates); no public API changes anywhere in the phase.

## 6. Phase 5 — the PackageScanner seam (audit -09; one raid)

vibe-index's scanner trio (`from_clones`, `from_github`, `git_cli` plumbing) is concrete dispatch today. The raid: a `PackageScanner` trait (the scan entry the reindex path consumes), two manifested cells — `#[cell(seam = "PackageScanner", variant = "from-clones")]`, `"from-github"` — selected at the reindex composition root (CLI arg → one `match`, R-001 shape); the GitVerse stub stays an error-returning arm, not a cell. Item tags onto `PROP-005#reindex`; the existing `scanner_e2e` / `from_github_e2e` suites are the oracles (verify they reference the cell *types* — add the import/construction if they drive through free fns today).

*Exit:* `cell-has-oracle` green at 20 cells; audit finding 2026-06-12-09 closes (the seam half; the doctest/error-gating half of -09 belongs to the gate-expansion question below, not this plan).
*Prediction:* zero new test files — the two e2e suites already exercise both variants end-to-end.

## 7. Order, sizing, cadence

Phases 0→1→2→3→4→5 in that order: 2 before 3 so unwrap conversions birth their messages in the final grammar (enums touched once); 3 before 4 so message strings settle before files move; 5 last because it is design-flavored and independent. Estimated sittings: 0+1 one; 2 two; 3 four; 4 six; 5 one — **~14 gated batches**, each a topic commit series, each ending in a shrink-only freeze diff. Any batch is a safe stopping point; the baseline is the resume pointer.

## 8. What this plan deliberately does NOT do

- Does NOT touch DBT-0020 or the two MCP files (owner instruction, 2026-06-12) — they park at exit as the baseline's residual 2.
- Does NOT expand `CONFORM_GATED` to vibe-core / vibe-index (their error enums and seams would freeze new findings). That is the **next** plan's opening move — the expand-as-you-conform rhythm — and it should land only after this baseline is drained, so one queue closes before another opens.
- Does NOT redesign the unsafe-gate posture (AUD-0016 stays an owner decision).
- Does NOT build the `vibe-install` orchestrator crate the audit sketched — Phase 4c's install.rs split keeps the door open without committing to it.
