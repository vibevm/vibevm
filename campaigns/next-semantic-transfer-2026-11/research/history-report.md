# NEXT history inventory and cause-oriented chronology

This is archaeology for semantic transfer, not an instruction to resume NEXT and not a product acceptance verdict. Source is pinned at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`; target at `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`. Git reads used `/fast/git/vibevm`; the packet also identified `/fast/git/v/vibevm-next` as source availability. No source files, Git state, services or databases were changed; no code, tests, builds or old campaign commands were executed.

## Proven inventory and its limits

The common ancestor is `30d217bfce47bcaccadabd9468f00e315dbcfead` (2026-09-28). There are exactly **1,056 source-only commits**, **158 target-only commits**, and zero merge commits in the source-only set. `/tmp/nst-history.json` contains every source-only SHA, parents, dates, subject, affected paths with added/deleted line counts, preliminary path categories, explicit task tokens, feature candidates and their rationale. Changed paths are against the first parent with rename detection disabled; an apparent delete/add may be a move. There are 19,070 distinct affected paths across the history, not 19,070 independently changed capabilities.

The subject-prefix census is 384 feat, 382 docs, 93 test, 89 fix, 64 chore, 30 plan, 8 refactor, 3 spec, 2 perf and 1 style. These labels are author metadata. The largest commits, `2786c49bf5e0` (6,982 paths, mutable 1.1 cohort move) and `af9251c24850` (4,477 paths, dependency/lock regeneration), make path volume particularly unsuitable as a feature-completion measure.

**Review levels:** the complete 1,056-row mapping is mechanical subject/path/task-token inventory. The first pass recorded twenty-one bounded file reviews in `bounded_content_reviews`; the follow-up classifies all135 previously unassigned commits in `/tmp/nst-unassigned-review.json`. Together these cover153 distinct commits at bounded content level, not every file of those commits. There has been no runtime revalidation. After the first direct task-token pass,135 commits were unassigned. The follow-up gives each a content-supported category/topic, including process-only and generated-only classifications rather than arbitrary feature assignment. The compact index now distinguishes903 inventory-only commits from153 bounded content reviews; all mappings still require applicable semantic and transfer judgment. A task ID appearing only in an evidence path is not proof of product implementation.

## The beginning predates the branch split

The earliest recoverable campaign authoring commit is `6ed477b1ba7fbfafd83ba29e31a57b136dc6f09e`, 2026-09-12. It already contains the **revision-4** refined plan and review, plus the master XML and 201 task records. The master quotes a September 11 owner request to turn the plan into an executable campaign and a September 12 clarification that this deliverable was planning only. Those dates are retained documentary statements; the authoring commit is the locally observed Git event.

The review names external immutable originals at `C:/Users/olegc/git/v/final-improvement/IMPLEMENTATION-PLAN.md` and `REVIEW-REQUEST.md`. Neither basename was found in source-reachable or all-ref Git path history. A permitted names-only search below `/fast/git/v/research` found unrelated Zap implementation-plan names, not that pair; their unrelated contents were not read. The recorded pair fingerprint is `sha256:0b6524d4128c5ece0763b14f791cbdc3e9cc4c6fd30aefa14b4f93df12892084`, defined as SHA-256(plan bytes + one NUL byte + request bytes). **Without the original bytes that fingerprint cannot be independently reproduced.** No original or intermediate revision was invented.

Recoverable initial blobs:

| Input | First Git blob | Current pinned blob |
|---|---|---|
| Refined plan | `7837ee3b9855d98139e7d048bb3bc31d6de6f7ed` | `833c1d7b00c7f0535afd80c54b4fdfb605544cce` |
| Project review | `d693d085bb368af84a4afaf82964f8390cf1b4b3` | `af9530aa2df3905c37b149b3a85d58058b780b9f` |

Both inputs changed only once afterward, at `f93580f13` on September 17: the removed external-skills component became an unbound future replacement, and its old evidence reference was withdrawn. This is a substantive source-binding correction, not a new complete review. Exact SHA-256 values and all master-document revisions are retained in JSON.

The master cites historical baseline `b1291b06d5704ee32f7486c86e1fae617fcbbddf`; `git show -s` in the inventory repository returned exit 128, bad object. That baseline is documentary provenance, not a locally verified object. A source-only inventory starting at the September 28 common ancestor would therefore omit the actual planning origin and preview-accounting mandate.

## What the reviewed design grew into

Revision 4 explicitly retains an earlier 19-decision walkthrough. Its own change narrative says package/environment requirements D-020…D-024 had expanded the original scope, then D-025/D-026 added universal external IDE/service access and Extreme Total Observability. Those are claims recoverable from revision-4 prose; separate revision-1/2/3 blobs were not recovered.

The architectural correction was to compose existing authorities, with stronger boundaries: exact selected-source identity and four integrity levels; consumer-owned configuration with candidate-correct solving; small repo-owned change bindings; separate create/build/verify purposes and post-state evidence; complete closure evidence; explicit source interpretation for ordinary content; pure observation separated from effectful preparation. The later scope includes HOME/environment generations, Linux userspace/Docker, local service descriptions, Nix-inspired independent supply paths, and externally addressable models/actions. Full distribution boot, an actual IDE/plugin, and unbounded workflow execution are not implied by this scope.

The frozen review makes these causes explicit: F-004/005 unsafe source lookup and overstated slot integrity; F-002/003 create-to-verify causality and workspace isolation; F-007/008 feature integration and solver backtracking; F-010/011 release ordering and producer provenance; F-013 hidden hydration; F-021 suffix-driven interpretation; F-022…028 environment/backend/closure/supply-chain gaps; F-029…034 machine-model, responsive-operation and observability gaps. This gives a causal feature map to preserve even where temporary task names disappear.

## Task arithmetic: 201 → 212 → 222

| Git capture | Group files | Task records | Meaning |
|---|---:|---:|---|
| `6ed477b1b`, September 12 | 86 | 201 | Initial 17-milestone decomposition |
| `c6e6e86e8`, September 12 | 90 | 212 | Adds 11 preview-accounting tasks |
| common ancestor, September 28 | 90 | 212 | Same identity set |
| pinned source | 92 | 222 | Ten later refinement records |

The ten are fully attributable: `89f983b48` adds M-01-C.4 (failed conversion diagnostics); `1cec235c6` adds M-06-G.1…G.4 (normal directive sharing, init dialect, generated-lane exclusion, Windows paths); `00d0ede33` adds M-14-C.3 (first genuinely host-admitted builtin effect); `9261c4a98` adds M-16-C.2.1…2.4 (lifecycle/build/package/deploy/publication recovery owners). All four introduction events are September 30. M-16-C.2 remains a parent task record and also becomes a subgroup identity: **222 task records must not casually be renamed 222 terminal graph leaves**.

The master still says 201 + 11 = 212 and still describes implementation as unstarted. Those are stale authoring statements. They cannot override Git additions or the recovered durable Zap plan.

## Content-supported changes that matter to transfer

The following mappings are supported by bounded reads of actual file changes. “Code” means static source observation; “contract” means normative text; neither means freshly tested acceptance.

| Change / cause | Concrete evidence | Supported conclusion and limit |
|---|---|---|
| Roles must not change through a relay | `fdab4da0c`, PROP-018 | Contract makes build deterministic, create the producer, and forbids ordinary verify from secretly producing via reasoning relay. Runtime enforcement is a separate claim. |
| Source APIs need one authority | `23dce2e7185`, `crates/vibe-cli/src/commands/spec/mod.rs` | Complete new CLI dispatch uses `SourceQueryService`, read/resolve modes, expected identities and cursor/chunk fields; request-option conflicts refuse. Does not by itself audit the entire query engine. |
| Dry-run preparation was effectful | `4f55717b78`, `pkgskill/projection.rs` | Inspection chooses `prepare=false`; embedded/composed sources return `SourcePreparationRequired`; explicit preparation retains hydration. Clear M-08/F-013 correction. |
| Phase names cannot establish safe effects | `5acc1e517b`, `extension/effects.rs` | Closed declared-effect enum, canonical order, empty/duplicate refusal; omitted declaration stays unclassified. It is a cooperative declaration, not OS confinement. |
| Inspection must share machine semantics | `c9e5eac5803`, MCP `tools/change.rs` | `change_query` delegates to shared MachineOperationService with the server root, generated response checks and structured refusal. Present negative tests were not rerun. |
| Exact identity also needs wire compatibility | `85e50713`, `environment/exact.rs` | Digest spelling changes from prefixed to 64 lowercase hex, with deterministic identity assertions. Broader environment completion is not implied. |
| Repeated normal roots duplicated authored sections | `901b3cd25`, `compiler/link.rs` + `link/normal_once.rs` | Whole-lane tracking deduplicates matching authored document/anchor/text and retains uncovered siblings/preambles. Equal prose from different documents remains distinct. |
| Fresh shortcut lost configuration intent | `aff3f408e`, install `plan.rs` | Request/definition identity is computed before Fresh shortcuts; configured replay is validated; mismatches re-resolve with current/retained identity. Shared discovery and other changed files need their own review. |
| A newer boot graph replaced older source laws | `4bc69c31f`, PROP-072 | Restores source grammar, precedence, migration and observation sections alongside captured graph law. Preserve both semantics, not merely the latest section title. |
| Zap/PodBay/VibeVM boundaries grew | `65c028ccc`, `db1b8c4d6`, PROJECT-BOUNDARIES | Rich Zap planning interpretations are permitted with source provenance; simultaneous physical descendant capacity belongs to PodBay and does not count task acceptance. These are architecture contracts. |
| Native path constraints cannot be guessed | `0ce122984`, safefs `path_limits.rs` | Calls GetVolumeInformationByHandleW on a held directory; zero/error is unavailable; maximum is UTF-16 component length, not total pathname limit. |
| Native CI needs executable evidence plumbing | `0ce122984`, `7fe6376c`, Windows workflow | Adds selected G4 Windows tests and evidence upload, then fixes runner.temp environment placement to step scope. A workflow definition or this repair is not a passing native run. |

The commit `5698d64d9` (“preserve requests across visibility retries”) has substantial resolver/install/package-source changes and is an important M-05 candidate, but this worker did **not** audit that implementation; it remains explicitly heuristic. This distinction also applies to the vast majority of the 1,056 commits.

## Work not captured by a fixed task-count story

**Lean is a separate addition outside tasks/*.json.** At `77cb41111` (October 5), the owner-priority record adds NEXT-LEAN-A0, three atoms and a gate through manifest/seed, halting the remaining route until a same-subject Rust/Lean capability is reviewed. The retained rationale identifies partial-map overwrites and freshness loss; a `verifies` relation is not a kernel verdict. `c6698c64f` adds a pinned Lean 4.34.1 family, `55b23c986` adds composed declaration-map selection, `48bcda3b3` adds fresh joined checking, and `50ee2cbf6` adds installed-consumer coverage. Read source explicitly restricts this to `g1_model` with `correspondence = not_established`; saved claims are rechecked, not trusted. Installed family availability is distinct from the host's compiled-in checker. No Rust refinement proof is inferred.

**Preview is earlier scope growth, already included in 222.** `c6e6e86e8` predates the common ancestor and adds BASE/BIND/DOCS/RELEASE accounting. Its master contract requires accounting for extra fixes, net DP1→DP2 behavior and separately supplied documentation; completion counts do not retire that obligation. September 28 implementation commits (`16dd60e83`, `7d8a0bdcc`, the wire-capture/checker series) and October 17 BIND commits therefore must not be lost as “only campaign bookkeeping.” Their exact implementation mapping is still subject to code review.

**CI and regression repair cross task boundaries.** Only three source-only commits touch `.github/`: `b6458a573` (Docker step-label wording plus separately inventoried regression tests), `0ce122984` and `7fe6376c` (Windows workflow). Do not expand the first into an invented Docker CI redesign. Boot/format/source-reader fixes, the restored PROP-072 clauses, and native path diagnostics are product changes or contract repairs even when introduced during campaign maintenance. Conversely M-06-G was explicitly added to the task inventory; it is extra to the original 201, not outside the current 222.

**Version migration is not product completion.** `aaf0fb563` (September 29) records DP1 already having users, mutable 1.1.0 as the next owned cohort, and a then-current publication hold. It supersedes frozen “no users/current own 1.0.0” prose. This report preserves that historical amendment without turning an old hold or old authorization into today's authority.

## Condensed chronology and current-plan separation

1. September 11–12: documentary commissioning; revision-4 design becomes 201 tasks, then gains preview accounting, affected verification and parallel-frontier inspection. The initial architecture is already broader than the unrecovered original proposal.
2. September 17: external-skills source is withdrawn and replacement binding made explicit.
3. September 28–29: branch-specific work begins; preview baseline/identity/accounting and mutable-1.1 migration accompany stronger role/source/machine contracts. High path counts largely include cohort moves and regenerated state.
4. September 30–October 3: the operational seams require finer recovery ownership, actual bound effects, declared source migration and ported regressions. Ten task refinements record discovered scope instead of pretending the original decomposition was complete.
5. October 5: same-method Lean/Rust work becomes a new gate, with explicit model-versus-implementation proof limits. It is not reducible to one of the original task rows.
6. October 12–19: source lookup, compiler sharing, boot observations, derivatives/configuration, preview bindings and native path work continue; source-law restoration demonstrates architectural rework, not only monotonic feature addition. The pinned endpoint is a CI-path fix, not a whole-product finish.

Current status comes from the independently recovered Zap evidence supplied by root, `/tmp/nst-zap-plan-report.md`, not this historical master: **113 claimed accepted and 109 remaining original task records**, with M-06-G.4 active in durable state. The report says the active work awaits material/scope correction and exact-SHA native Windows evidence; active does not mean a live worker. All 17 strategy obligations remain active and final retirement/documentation/native/environment gates survive leaf counts. This worker read that report, did not reopen the live DB, and did not execute its next task.

## Compact durable inventory proposal

Do not commit the 5.4 MB working JSON or reproduce all changed paths. Retain a short provenance/method document, this cause-oriented chronology after root review, and one compact TSV/JSONL row per source-only commit:

`sha | subject | category_ids | feature_ids | task_ids | rationale_ids | review_level | path_count | added_lines | deleted_lines | binary_path_count`

Use full SHA as identity; obtain paths/diffs from Git when needed. Store the classification rules/rationale dictionary once (for example `subject-token`, `path-owner`, `task-token`, `bounded-file-review`, `unassigned`) instead of repeated prose. Retain explicit `inventory-only` for the default review level; a bounded-file review must point to a separate small review row naming exact SHA/path, observation and limit, never upgrade the entire commit implicitly. Preserve `unassigned` rows. Feature/workstream IDs are candidate mappings until a reviewer approves them.

Bind the compact index to source/target/common ancestor, the exact 1,056-row count, classifier version, and source-inventory hash. Keep initial/preview/pinned task-set counts and the 21 extra task identities in a small change table. Git retains full paths; the large `/tmp` inventory is disposable after validating the compact export. No repository write or acceptance decision was made by this worker.

## Follow-up: all135 initially unassigned rows investigated

The original5.4MB inventory remains byte-identical and is also preserved at `/tmp/nst-history-before-unassigned-review.json`. New compact output is `/tmp/nst-commit-index.json` (1,056 unique SHA rows, about584KB, no complete affected-path lists). `/tmp/nst-unassigned-review.json` contains the135 content classifications with selected path/diff hashes and remaining uncertainty. Each patch was examined through bounded changed-line excerpts; deeper follow-up covered cases where initial views showed only exports/generated metadata. Complete patch hashing is a provenance check, not a claim of complete reading.

Review-level census:903 inventory-only;60 bounded code/schema/test classifications;21 bounded contract classifications;42 bounded record/document classifications;12 generated/projection classifications;18 additional distinct commits with prior bounded file reviews. Mixed commits retain multiple categories. No full-commit semantic audit or test execution is asserted.

Concrete additions to the earlier narrative:

- **D-022 was resolved in retained history.** `448d5770f8` records the September28 pacman/libalpm coexistence ruling and explicitly excludes host mutation/publication authority. `ccf1f4e446` makes the campaign checker consult accepted decision nodes. The frozen review's OPEN is stale; current implementation completion still requires separate evidence.
- **Pure migration became a detailed contract family.** `cc4d9b0ff4` through `36e16a43a2` refine frozen grammar, lossless wire offsets, identity framing, guarded apply, disjoint recovery store and truthful durability. `90d6371cc4`/`79a53367ab` add corresponding held-filesystem support. Normative guarantees and implementation coverage remain separate.
- **Machine discovery grew beyond a command list.** Capability coverage differentiates registration from effect routes; model references distinguish logical, captured and observed identity; pages have process-local owned cursors; URI metadata retains unverified-source status. SourceCaptureBinding explicitly fixes a mistaken page/source join. These changes warrant preservation beyond their temporary task labels.
- **Selected-source semantics spread across consumers.** Facts adoption, native/replay boot, citations, tree/effective display and progress grounding acquire descriptor/lock/witness-aware routes. The reader changes and their tests are distinct from generated specmap updates.
- **Independent product repairs are present.** `d7601b96d0` implements static-registry search through primary.jsonl rather than a missing server route; `b942b96d33` changes nested de-substitution to a fixpoint; `c6b78c7a5d` explicitly unlocks retained guards; init validates identity before writes and suppresses progress during prompts. Their relationship to the transfer must be assessed without forcing every change into an original milestone.
- **Recorded debt is not repair.** B-241…246 at `3cbcc32f4b` record Change manifest-role aliasing, native Deploy allowed-set mismatch, MCP rollback overwriting detected drift, adaptive-node planning-hold gap, stop/refusal guarantee wording and traced-predecessor fresh-key interaction. B-231 retains one-way Doc Markdown semantics, and the cross-package once-only fixture was initially explicitly ignored pending B-224. Preserve these distinctions.

Generated specmaps/corpus stamps, local lock-path rebinding, historical release accounts, operator preferences and incident narratives are separately classified. Their presence neither proves feature completion nor authorizes replaying their operational instructions. The next review still needs final-state/source-to-target comparison, permanent-law reconciliation, concrete compatibility checks and disposition of every remaining obligation.

## 2026-11-04 follow-up: ordinary documents and selected descriptor consumers

Thirteen further bounded rows (six ordinary-document, seven descriptor/boot integration) now have complete changed-path accounting:74entries, with exact diff/blob hashes and90final/main proof entries verified. The compact index now has220bounded classifications and836inventory-only rows out of1056. These are review levels, not completed features or whole-commit/runtime acceptance. Separate reports retain inspected/excluded boundaries and hash representations.

The useful distinctions are concrete. A nested-list converter fix is already present in main. The prototype's inline-code correction removes a raw second duplicate scan that conflicts with current code-span opacity, while historical B-092 deliberately introduced that scan; the compatibility correction still needs its own runtime proof. CommonMark/XML-data parsing has real internal consumers, but named ordinary exports do not reach the public Vibe-only source query. Descriptor identity and captured bytes are useful; rebuilding complete maps per consumer, dropping captures before a later path read, and inconsistent empty-list admission are avoidable duplication or integration gaps. Selected Simple projection and lifecycle prompts still have specific suffix/admission seams at the final pin. Main F05's stronger pattern/anchor/selected-world behavior must be retained.

M-06-A.4 remains partial. Its inventory explanation was corrected: documented legacy compatibility is not itself a missing feature; the concrete caller gaps and unresolved package-empty semantics are the outstanding evidence. Project absent/empty legacy behavior is explicitly governed separately. No task status or runtime_verified flag was upgraded by these audits.
