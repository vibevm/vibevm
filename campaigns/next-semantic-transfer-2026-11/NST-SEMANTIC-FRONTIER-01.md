# Semantic transfer review before domain consolidation

Source plan r545 and a fresh read-only `verify.py` run identify the review frontier. The structural check matched 222 historical task IDs and the commit-index set to their pinned Git inputs. The inventory labels 95 implemented, 49 partial, 72 planned-only and six unknown. These are assessments of the historical source, not delivery claims for the current product. Do not rerun a historical feature just to make an inventory status green.

| Stage | Remaining decision and evidence |
|---|---|
| NST-1 | Reconcile actual history and reachable source (`NST-HISTORY`, `NST-SOURCE`). For M-09-A.1/A.2/B.1/E.1 and M-13-A.1/A.2, either attach stronger evidence or retain an explicit unknown with its impact on transfer. An honest unknown is preferable to inventing historical implementation. |
| NST-2 | For implemented/partial source features link the existing necessity decision or supply the missing one. Give the environment/Docker candidate a user-outcome-based disposition rather than making a new environment engine mandatory. |
| NST-3 | Reuse the twelve accepted direct design children; review the two active common-choice and runtime-design children against real selection, evaluation and persistence journeys. |
| NST-4 | Reuse the 36 accepted direct delivery children. Reconcile the active common-choice implementation and format-change ledger against exact integrated source and tests; unchanged formats need no fabricated changelog entry. |

Trace a feature from its historical evidence to its necessity, design and delivery decision. A missing link may be missing documentation, a deliberate exclusion, or a product defect: distinguish them before dispatching code. Preserve existing accepted verdicts; cite representative tests and explain material untested paths. Keep the later NST-5 integrated final gate separate from the pre-consolidation semantic review. This packet is not an acceptance receipt or a change to the active CORP priority.

### Pinned historical audit artifacts

At `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`, the M-13 consumer map exists and cites all 14 task-declared paths; all 14 exist at that pin. The version census also exists: direct comparison of its 190 recorded path/digest pairs with Git blobs at this pin matched 21, differed for 24 and found no blob at 145 paths. This establishes the presence of two historical audit deliverables, not reproducibility of the original workspace, proof that absent paths were missing at the time, or a current-product defect. Preserve unknown completeness. The four M-09 task IDs were found by literal-ID search only in task/seed/manifest material, which does not establish that no differently named or external evidence exists. Do not relabel the six as missing implementations.

Origin check for M-13-A.2: `git log` locates the census at `4a3f80a34`. Reading each listed path as a Git blob from that commit and hashing its contents with SHA-256 reproduced **190 of 190** recorded digests. Thus the later 21/190 result is revision drift, not evidence of missing original inputs. The input set is reproducible; whether the census selected every required input and correctly classified exclusions is a separate semantic question.

Manifest-set comparison at the same origin: Git contains 523 files named `Cargo.toml` or `vibe.toml`; 184 occur in the 190 hashed inputs. Of the other 339, 178 appear by exact path in the explicit inventory lists. The other 161 are all Cargo manifests: 150 under materialized `vibedeps`, eight under fixture directories, two in the Rust research demo and one under manual-test staging. No `vibe.toml` is absent from both sets. This checks filename coverage, not whether every Rust workspace member or version constraint was semantically interpreted correctly.

At `4a3f80a34`, independently parsing the TOML and looking up each recorded field path reproduced the type and value of all 155 `measured_version_fields` rows (95 inheritance flags, 48 package versions, nine workspace versions and three project versions). An inheritance flag is not itself a resolved version; this check does not validate the separate policy dispositions.

For the 95 Cargo inheritance rows at the origin, the declared workspace membership and exclusions identify one owning workspace per package. All nine owning workspace manifests declare `workspace.package.version = "1.0.0"`. No row required an explicit package workspace override or had an ambiguous owner in this comparison. This is a source inspection, not a Cargo build.

### Necessity review outside the historical accepted set

Joining the 113 unique `old_task_id` values in `research/old113-transfer-crosswalk.json` to the 222 source task IDs leaves 33 implemented/partial source records outside that crosswalk (two implemented and 31 partial). The other 76 outside rows are 72 planned-only and four unknown. The 33 are lookup candidates for necessity decisions, not 33 assumed missing features or repairs. Search the later scoped reviews and current product obligations before assigning work.

- Brownfield observe/propose/promote and controlled existing-file production: M-11-A.1, M-11-A.2, M-11-D.1, M-11-D.2, M-11-D.3.
- Close initial-domain API gaps without hiding supported operations behind contextual unsupported results.: M-17-A.2.
- Compose change analysis over one captured fact universe.: M-04-B.1.
- Derive a complete DP1-to-candidate migration document from actual delivered changes rather than completed task counts.: NEXT-PREVIEW-RELEASE.1.
- Exact foreign bridge resources and reusable native feat journeys: M-09-B.2, M-09-C.2.
- Exercise documented examples through headless surfaces and retain a reproducible capability matrix.: M-12-B.2.
- Exercise failure and connection boundaries through independent external clients without duplicate effects.: M-17-C.2.
- Explicit source semantics and inspectable actual/hypothetical boot: M-06-G.4.
- Expose stage evidence without acquiring writer ownership, rerunning providers or silently truncating completeness.: M-17-A.3.
- Give users one explicit create-to-verify entry without adding a workflow engine.: M-03-A.2.
- Leave implementable client-neutral documentation and compatibility examples independent of temporary campaign files.: M-17-D.2.
- Make ETO/interface completeness a mechanically checked property of the delivered product.: M-17-D.1.
- Make contributed models/actions inspectable after provider removal and prove every supported initial coverage row.: M-17-A.4.
- Make scoped reuse depend on meaningful production inputs and exact target context.: M-03-B.1.
- Package-defined client adapters and safe complete deployment lifecycle: M-08-E.1, M-08-E.2.
- Preserve the separately authored documentation as an exact input before planning its product-driven update.: NEXT-PREVIEW-DOCS.1.
- Prove independent external clients can navigate, edit unsaved state and perform the saved-state rename protocol.: M-17-C.1.
- Replace CLI-local summary assumptions with exact externally queryable tree/provenance/reference views.: M-17-A.1.
- Subject-bound distribution publication, smoke, approval and recovery: M-07-B.1, M-07-B.2, M-07-C.1, M-07-C.2, M-07-D.1, M-07-D.2, M-07-E.1, M-07-E.2.
- Update the user/developer journeys from implemented CLI/API behavior and accepted prototype policy.: M-12-B.1.

#### Necessity dispositions for the two implemented source records

- **M-03-A.2 — keep the user outcome, not an independent workflow engine.** A user needs to distinguish a prepared create/verify plan from execution and inspect the actual outcome without replay. The historical atom supplied only a read-only composition and identity recipe. `research/reconciliation-report.md` records the Zap safe-stop, not acceptance of an executable journey. Review the current operation owners at DESIGN/LAND; do not issue an implementation task for a parallel identity system merely because this source atom existed.
- **M-07-B.2 — keep smoke status and publication eligibility separate.** A skipped check must remain inspectable and must not count as executed. The implemented historical default did not permit publication on a skip; a wider historical proposal did. Preserve the stricter admission unless an independently authorized target policy explicitly permits an exception. This necessity decision neither authorizes publication nor establishes that the target implements either policy.

#### Necessity dispositions for the 31 partial source records

These are architect NEED decisions, not historical or target execution results. Each ID outside the old 113 crosswalk with source status `partial` is assigned exactly once. DESIGN/LAND still require the actual current owner, user scenario and evidence or an explicit disposition.

- **M-03-B.1, M-04-B.1** — Keep request identity distinct from a concrete attempt, and distinguish missing evidence from verified completion. Inspect existing operation receipts before proposing new state or wire formats.
- **M-06-G.4** — Keep actionable long-path diagnostics. Classify Windows behavior from a native test or an explicit platform limitation, not conditional source alone.
- **M-07-B.1, M-07-C.1, M-07-C.2** — Keep installed-artifact smoke evidence and bind it to the artifact, producer and check. Do not require a separate verification service or CI change if existing receipts suffice.
- **M-07-D.1, M-07-D.2, M-07-E.1, M-07-E.2** — Keep explicit authorization before remote mutations and truthful recovery after interruption. Compare effects on isolated fixtures; neither alternate publishing implementations nor real remote fault injection are mandated. Publication itself is not authorized by these records.
- **M-08-E.1, M-08-E.2** — Keep native agent commissioning for claimed versions and modes. Reuse the accepted SYS and CORP cases where their evidence fits; a launcher version probe is not a conversation or tool lifecycle.
- **M-09-B.2, M-09-C.2** — Conditional on an explicitly supported bridge: preserve its exact resources and invoke only admitted actions. A missing external fixture does not justify inventing an adapter or running foreign procedures.
- **M-11-A.1, M-11-A.2** — Keep explicit read scope, skipped/unreadable entries and evidence of changes during observation. Do not promise an atomic snapshot from a directory listing.
- **M-11-D.1, M-11-D.2, M-11-D.3** — Keep ownership and causality when an operation modifies an existing input: distinguish changes it made from unrelated edits and verify without running a producer again. Reuse domain witnesses where available; no universal transition ledger by default.
- **M-12-B.1, M-12-B.2, NEXT-PREVIEW-DOCS.1, NEXT-PREVIEW-RELEASE.1** — Keep runnable examples and migration guidance aligned with supported public behavior. Verify referenced topics, commands and format changes; a draft generator is neither a release nor proof the guidance works.
- **M-17-A.1, M-17-A.2, M-17-A.3, M-17-A.4** — Keep shared meanings across CLI and machine clients for supported operations, including partial effects and read-only inspection. Bound schemas and coverage to supported domains; no all-purpose new service or resurrection of retired administrative APIs.
- **M-17-C.1, M-17-C.2, M-17-D.1, M-17-D.2** — Keep client isolation, cancellation, reconnect and honest coverage for promised machine workflows. Verify supported scenarios against actual operations; do not infer semantic rename, Docker or external GUI support from transport tests. This is distinct from building the post-NST IDE.

### Necessity decisions for the accepted-set comparison

Use `research/old113-transfer-crosswalk.json` for individual criterion-level correspondence and unmet clauses. The following are NEED decisions for its 111 source-implemented/partial records; they do not upgrade the crosswalk’s delivery verdicts. The other two of the 113 have unknown source completeness. Together with the 33 decisions above these cover all 144 implemented/partial rows in the pinned source inventory, not all NST acceptance obligations.

- **M-01 (13 records):** Keep scoped, unambiguous reads and evidence of the exact returned bytes; distinguish local agreement from independent verification. Do not introduce a second resolver. IDs: M-01-A.1, M-01-A.2, M-01-A.3, M-01-B.1, M-01-B.2, M-01-C.1, M-01-C.2, M-01-C.3, M-01-C.4, M-01-D.1, M-01-D.2, M-01-E.1, M-01-E.2.
- **M-02 (9 records):** Keep reusable intent separate from project decisions; preserve source identity and report incomplete scope. Reuse existing authored changes and operation owners rather than importing a parallel change database. IDs: M-02-A.1, M-02-A.2, M-02-B.1, M-02-B.2, M-02-B.3, M-02-C.1, M-02-C.2, M-02-D.1, M-02-D.2.
- **M-03 (1 records):** Keep an explicit operation entry and disclose prerequisite effects; preparation cannot imply execution. IDs: M-03-A.1.
- **M-05 (18 records):** Keep deterministic package choices, selection provenance and faithful replay across supported clients. Reconfiguration is explicit; no competing feature solver or hidden default answer. IDs: M-05-A.1, M-05-A.2, M-05-B.1, M-05-B.2, M-05-B.3, M-05-C.1, M-05-C.2, M-05-C.3, M-05-D.1, M-05-D.2, M-05-E.1, M-05-E.2, M-05-E.3, M-05-E.4, M-05-F.1, M-05-F.2, M-05-G.1, M-05-G.2.
- **M-06 (17 records):** Keep explicit source semantics and distinguish actual boot from a hypothetical view. Reuse existing composition and publication owners; recoveries and physical platforms require their own evidence. IDs: M-06-A.1, M-06-A.2, M-06-A.3, M-06-A.4, M-06-B.1, M-06-B.2, M-06-C.1, M-06-C.2, M-06-D.1, M-06-D.2, M-06-E.1, M-06-E.2, M-06-F.1, M-06-F.2, M-06-G.1, M-06-G.2, M-06-G.3.
- **M-07 (2 records):** Keep evidence bound to the exact distribution subject and require separate authority to publish. Reuse release tooling; no new publication grant. IDs: M-07-A.1, M-07-A.2.
- **M-08 (9 records):** Keep package-defined adapters and ownership-aware install, upgrade and removal. Agent capability and platform claims require actual scoped evidence. IDs: M-08-A.1, M-08-A.2, M-08-B.1, M-08-B.2, M-08-C.1, M-08-C.2, M-08-C.3, M-08-D.1, M-08-D.2.
- **M-10 (8 records):** Keep local derivatives distinguishable from their origin, with independent activation and ownership. Do not infer permission to overwrite or publish the source. IDs: M-10-A.1, M-10-A.2, M-10-B.1, M-10-B.2, M-10-C.1, M-10-C.2, M-10-D.1, M-10-D.2.
- **M-13 (6 records):** Keep planned, validated migration of owned inputs with a recovery boundary. Separate package versions, format epochs, compatibility constraints and upstream pins; do not rewrite foreign or historical records. IDs: M-13-B.1, M-13-B.2, M-13-C.1, M-13-C.2, M-13-D.1, M-13-D.2.
- **M-14 (7 records):** Keep target identity and filesystem authority separate from the executor. Reuse installer decisions and storage owners; see environment-target-design-disposition.md. No presumed Docker backend. IDs: M-14-A.1, M-14-A.2, M-14-B.1, M-14-B.2, M-14-C.1, M-14-C.2, M-14-C.3.
- **M-16 (16 records):** Keep a machine-readable path to the same supported operations and truthful partial outcomes, cancellation, replay and unsaved-buffer inspection. Do not build a universal domain model or expose unadmitted effects. IDs: M-16-A.1, M-16-A.2, M-16-B.1, M-16-B.2, M-16-B.3, M-16-C.1, M-16-C.2, M-16-C.2.1, M-16-C.2.2, M-16-C.2.3, M-16-C.2.4, M-16-C.3, M-16-C.4, M-16-D.1, M-16-D.2, M-16-D.3.
- **NEXT-PREVIEW (5 records):** Keep an exact release baseline and traceable documentation/format changes; publication and approval remain distinct. Do not duplicate underlying domain journals. IDs: NEXT-PREVIEW-BASE.1, NEXT-PREVIEW-BASE.2, NEXT-PREVIEW-BASE.3, NEXT-PREVIEW-BIND.1, NEXT-PREVIEW-BIND.2.

### NST-SOURCE acceptance boundary

Accept the source classification of the 222 historical task records, not their implementation on the target. At prototype revision `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`, every row cites at least one existing Git path, all 144 implemented/partial rows cite evidence beyond the task/seed/manifest, and every partial, planned-only or unknown row records the specific missing evidence or behavior. The scoped source audits and `research/reconciliation-report.md` explain where narrower source outcomes differ from Zap acceptance. Keep the six unknown classifications (including the four external bridge records) as justified limits. The independent origin checks above refine the two census rows without treating audit artifacts as product delivery. This disposition is not acceptance of the separate NST-HISTORY atom or of target runtime behavior.

### Historical preview amendments

Four previously inventory-only changes were read against their first parents: `2387e346fea` (dependency direction), `c279d6410c8` (opaque identity), `8f2810868d8` (documentation capture versus retrieval), and `d4d8af1e69d` (local read boundary). Each changed contract now has a scoped finding and diff digest in `commit-index.json`. They specify behavior; none is a product execution receipt.

Five historical Zap backlog amendments (`31314cdbb`, `c7e550ba6`, `96336d860`, `ff83fe2bd`, `865d4d8d9`) record import costs, inactive acceptance, incomplete obligation routing, uncertain recovery and limits on work dispatch. None of the five diffs implements a product change or grants authority. See the per-commit evidence in the index.

Historical local-smoke and host-guard records (`095187156`, `115b946de`, `2659c42b3`) distinguish a launched Linux process from preparation, an explicit skip and a foreign target. These records are not cross-platform acceptance for a later candidate.

Five preview amendments distinguish documentation byte coverage, tool readiness, temporary routing, inherited exclusions and affected-source review from semantic acceptance (`2cfdabd30ee6`, `a81a91fe9606`, `e32abf8e68a6`, `6488f4fd5af7`, `b9e913919be4`).

Migration and environment amendments (`640ed8e33409`, `7012d485eca6`, `c7517ffd0c2c`, `92d30ff72aae`, `b80b3ce83dd2`, `8f072d3325d4`) distinguish disclosure from parser output, null from omission, and lexical target constraints from concrete target identity and authority. None of these six changes establishes runtime execution.

At the 813-row frontier, 394 commits touched possible production Rust under `crates/` or `xtask/`, 48 touched only other Rust there, and 371 touched no Rust there. This path heuristic prioritizes inspection, not acceptance. In `b347fa04e611` the actual atomic push driver was private and transport-injected despite the subject mentioning production; no live push was performed or inferred.

Historical repository creation and release finalization (`05d60fbe0`, `d3eee83b3`) used private transport interfaces. Existing remote state or a lost response is not a successful receipt and confers no automatic retry; neither owner establishes a reachable public-host path at its reviewed revision.

Task packet review: 25 task-JSON-only commits amended 15 distinct original tasks. A full before/after field comparison found one amended task per commit; only named read/write perimeters, notes and checks changed. Existing task identities and acceptance fields were preserved. No new product implementation or executed tests are inferred from those edits. The index retains the exact task IDs, changed field names and first-parent diff digests.

Eight specmap-only commits were compared as complete parent/child JSON: three changed only the spec-unit collection; five changed spec units, code items and edges. The exact counts are recorded per commit. No source or specification was edited in those commits and a projected edge is not evidence that its behavior ran.

Five release-impact-only commits changed 18 records. All 43 distinct commit subjects named by those records exist locally. The index records their change identities and diff hashes separately from the product and publication evidence they describe.

Ten preview-transition commits changed only 36 release JSON files. All 24 explicit top-level commit/tree pairs matched Git. No product execution or publication was inferred from these records.

Five combined manifest/task changes amended the permissible paths or notes of M-16-C.3 or M-16-C.4 and the registry digest field only. Task identity, its acceptance fields and product code were unchanged.

Six historical reviews separate accepted design from available lifecycle/source behavior. In the M14 release-impact recount (32 → at least 34 → at least 33 product changes), all seven paths of `e68c64679` are environment-C1 evidence. Its retention should not inflate product-change coverage.

Four single-file code amendments distinguish captured inventory from physical completeness, plan reservation size from payload size, test-only publication owners from release routes, and root comparison from dispatch. See `87023f56ac0e`, `c570f59e2f4d`, `696542ece2b4`, `ef8816a77129` in the evidence index.

Nine single-file test-labeled changes were inspected by actual content. One only retargets a specification annotation, one switches a fixture, and one adds only a historical race report; the others add assertions around decoding, replay, domain isolation or multi-client operation behavior. No current-product run is inferred.

The three migration JTD declarations in `aa48a5f98d13` and two preview catalogue fixtures in `db29ee46f531` establish format inputs, not execution of the reader or a production migration.

Four M-13-D records in `529ee8eb7853` document the numerical edit perimeter, slot moves, and inheritance guards. A retained report does not grant subtree writes or establish a migration run.

Four publication amendments distinguish subject preparation, pure approval, repeated observation and required pre-effect baseline from a live remote write (`8ef1fd0ba0db`, `a65993704180`, `bd40c8d7b85a`, `9c5810bb04bc`).

Across 31 evidence-only Markdown commits (37 changed-file occurrences), the records describe bounded language proofs, fixed machine operations, migration, source readers, environment and release checks. Their first-parent deltas touched no product or test implementation. In particular Linux availability does not certify other platforms, and a recorded test run is not an independent rerun on current source.

Thirty further commits touched only retained evidence (179 changed-file occurrences). Their file and digest inventories distinguish source reviews, acceptance records, test logs and input captures from production/test changes. No historical outcome was rerun or promoted by this classification.

Of the remaining historical amendments, 51 touched Markdown only (54 changed-file occurrences), including issue ledgers and guides. The index records the actual paths, added/removed line counts, cited task or issue identifiers where present and per-path diff hashes. This is a record-level classification, not acceptance of the changes described in those documents.

Eleven remaining XML-only changes were compared by named fact text and first-parent diff. Eight concerned common specifications and three amended boot instructions. Their changed contracts and instructions are not evidence of product delivery.

Three remaining Rust test-only changes exercise cold partial installation, deferred TCP session stop, and simulated draft replacement. Their assertions were inspected; no live publishing or current-product execution was inferred.

`65d631fd88` introduced the machine-operation census contract and its projected map, not executable operations. Inspecting a registration must not invoke its handler.

Another 90 revisions touched only original campaign seed/registry/task inputs (271 changed-file occurrences). Parsed first-parent comparisons identify the changed keys and task-file IDs in the index. They describe historical planning, not the present roadmap or executed product changes.

12 further commits changed only the historical campaign evidence tree (1246 changed-file occurrences). Embedded patches, inputs, scripts and logs were classified as retained artifacts; their presence does not establish acceptance or execution in the target. Per-file diffs are fingerprinted for later claim-specific inspection.

Eleven remaining campaign-only revisions (39 changed-file occurrences) update rulings, evidence, task bindings or historical campaign tooling. Python tooling changes are distinguished from record-only changes; no current-product effect or historical test execution is inferred.

Five inspected revisions distinguish mode-only source changes, wire vocabulary, codegen slot selection, report-duration limits and documentation about late errors. These are not five newly executed product journeys.

`036497cb67` added generated migration request, journal and report types and registrations in six files. Its authored schemas were not changed in this commit; a generated reader is not evidence that a migration ran.

Another 14 commits were confined to release documentation, evidence and fixtures (212 changed-file occurrences). Structured records were parsed where applicable; fixture helpers and recorded commands were not executed. This does not establish a publication or target behavior.

Fourteen xtask-only revisions were inspected for capture, local staging, compatible-host smoke, persisted evidence, pure publication planning, reader policy and finite reference checks. Fake release ordering is not host publication; staging a bundle is not executing it.

Seven publication changes distinguish preparation and identity, smoke readiness, pure remote planning, local capture and scoped approval. None is accepted as an observed release on a real host by this review.

Eight lifecycle revisions were checked for narrowly scoped admission, target OS, physical effect uncertainty, missing recovery input, stopped-phase reports and replay identity. A completed report is not proof that uncertain physical effects completed.

Ten small revisions were checked for annotation binding, mirror configuration, worker disclosure, schema provenance, CLI guidance and target policy. A mirror ref configuration or changed spec revision is not evidence of a push or executed migration.

Eight environment revisions distinguish exact captured configuration and target identity from legacy observations and bounded layout reports. Native execution or final publication is not inferred from those records.

At `0e945cb009`, each of the four parent log blobs matches the manifest raw digest, and each child matches its repository digest and the parent bytes with only trailing blank lines normalized. This checks retained M-16-C.4 evidence, not its external directory or a new execution.

Five first-parent revisions clarify historical scaffold defaults, source declarations, delegation ownership, Zap source selection and prototype version policy. In particular, an exact bridge coordinate did not freeze the mutable application branch; a package/context owner did not thereby own task admission or process lifecycle. No target execution or publication was established.

Four first-parent changes separate pre-effect reservation from the domain-owned terminal outcome, and profile admission from execution. A syntactically valid digest on a nonexecution record never promoted the historical fixture to apply-ready. In target crates at `b11538dad` the historical fixed-deploy and publication-subject symbols were not found by exact name. This does not rule out differently named owners. The M-16-C.4 partial disposition in `research/reconciliation-report.md` is unchanged; prove a currently advertised route before crediting one.

Seven historical preview revisions distinguish the declaration of affected files from verification of selected bytes. Missing required path/anchor membership is a refusal; matching membership without all required evidence remains pending. A successful late-stage readiness check itself returns Pending, not a release decision. Scope and per-file fingerprints are in the commit index.

Bootstrap evidence at `0adc39a38d`: all 193 present endpoints for 155 listed paths matched Git blob ID and independently computed SHA-256 at the two recorded revisions; twelve listed paths were absent at both ends. The interval contains 48 commits. This verifies the listed identities, not that every touched path was listed. Separately, `9ccd39edf2` keeps capture identity opaque without relaxing source path validation.

Independent census check for the same bootstrap interval: the union of first-parent changes over its 48 commits contains 167 unique tracked paths. The five historical DP1 directory exclusions agree in the original and wire captures. Five changed paths fall under `campaigns/next/`, seven under `docs/releases/developer-preview-1-to-2/`, and none under the other three exclusions. The remaining path set equals the 155 review rows exactly, with no duplicates, missing paths or extra rows. This establishes coverage under those historical exclusions, not broader capability or release acceptance.

The operation inventory at `9f45ef0de2` reproduces 161/161 source digests at its baseline `4a3f80a34`. Its 206 surface entries distinguish registered commands/tools/actions from direct routes and an extension seam; the separate model and planned families are not executable surface rows. The authority design receipt at `e203b9dabf` does not attest current external facade delivery.

The coverage format, native registration comparison and reviewed planner mapping at `723948861d`, `ac1cbb72ee` and `44cbaf2845` answer different questions. An owning test selected but filtered to zero does not count; matched names do not prove effects; a known planner mapping still needs service admission. The historical source inspection did not execute the registered operations.

The historical 1.1 cohort transitions have complete one-to-one removed/added path mappings by changing only the outer version directory; nested fixture and dependency versions are not rewritten by that mapping. At `2786c49bf5`, 3429/3489 pairs retain the exact Git object and mode; at `af9251c248`, 2084/2231 do. The 60 and 147 differing pairs are not classified as mere renames. `b584d9b7aa` adjusts local tooling references, not an execution receipt.

Historical migration review separates typed decoding, planning and journal-owned mutation. At `df629cfa8e` exactly 25 CRLF sequences distinguish the preserved diagnostic from its parent; SHA-256 `5e3b101d3a43090c92eca0039e0ee1631e81aa520fd88fd2612a582b45770d92` matches the recorded failure history. Reproducing those bytes does not convert the failed test into a pass.

Six historical migration revisions separate the planning report and journal schema from an admitted file target and observed recovery state. Recovery treats a published journal with validated after-bytes as no-op; an unrecognized phase/content pair is not authorization to overwrite. Both current source hashes and both previous ledger bindings in the MCP startup-order correction match the corresponding Git records. No present migration execution is inferred.

At `7c034cf75b`, all 104 present endpoints of the 81 listed migration paths agree with Git at `b486af778` and `149bcea52`; absent endpoints were checked as well. All 48 package rows are identical between captures. The operator guide matches its digest. The corrected contribution report contains 278 checked claims, not 278 newly executed cases. Native macOS and Windows were not qualified by this inspection.

For M-08-C.1/C.2, the retained before/after source maps agree and all 104/13 file sizes and hashes match the respective pinned source commits. Both manifests explicitly separate historical work acceptance from runtime acceptance. For M-05-C.1 all 45 listed archive artifacts match their retained lengths and hashes. No rerun or current acceptance follows.

The entity-view account at `cc21c4d50b` has 79 listed paths; both endpoint presences were independently checked and all 91 present blobs matched their Git IDs and SHA-256. This binds the historical pure-model/projection scope, not live resource access or durable cursors.

For six derivative and boot revisions, checked the saved-file digests in the retained manifests against the originating Git trees. Matched/listed entries: `8b096070cd` 19/19, `0c9bf19b47` 21/21, `76b60b2a36` 18/18, `dbbd0b5a5c` 33/33, `d8e437a20b` 30/30, `fe9caaa2cf` 60/60. External raw files and the product code changes in those revisions were not qualified by this archive check.

For the four-client commissioning contract, all 39 saved artifact hashes matched. For the prepared-plan bridge characterization, all 72 archived mappings reproduce both stored and raw sizes/hashes, including the decompressed trace, and all four source records match Git. Neither check constitutes a new native client qualification.

Four historical changes separate a lock-selected package source, bounded retained bytes, cited host facts and boot-file classification. A file that disappears after observation does not establish a complete capture; a directory declaration does not establish its membership. Scope is limited to the named code owners, not present runtime acceptance.

Three historical migration changes bind decisions to the exact authored path and replay against original bytes rather than choosing media by suffix. Separately the `8c8d623602` source receipt yielded 77/79 matching recorded source hashes against the commit tree (0 missing, 2 differing). The source comparison is not a conversion runtime test.

The same revision’s `ROOT-FINAL-INTEGRATION.json` explicitly lists the two differing paths, `crates/vibe-install/src/slot_verify/derivation.rs` and `crates/vibe-specdoc/src/canonical_node.rs`, as root metadata-only changes. That is the historical integrator’s classification, not an independent reconstruction of the two earlier source blobs. Neither mismatch is explained by simple LF-to-CRLF conversion.

The BIND.2 manifest at `68728811bd` reproduces all 20 declared sizes and SHA-256 values at the recording commit, not at the stated base (`ddc502417`), where none match. A base reference is not the candidate snapshot.

At `39c7d99040` all four raw digests cited by the typed adapter review match the logs in the originating tree; each retained log contains its one-case pass. These are the original selected receipts, not additional tests on top of the later normalized copies or closure of the two deferred gates.

The historical Redbook declared `mup` as the exactly-one continuity default, but the host separately supplied exact `mup`. Identical resulting graphs are insufficient to classify the latter as automatic reliance. Feature-effect lowering and resolver characterization do not establish the current schema-12 replay behavior.

The B.1 machine-operation source account binds all 200 present endpoint blobs for its 122 listed paths. Separately, M-03-A.2 records thirteen candidate hashes found at `0f3d637098`, not at the manifest head. These are source identity checks rather than new operation runs.

Historical source-receipt check (matching/declared Git blob digests at recording commit): `72dccbcaa` 9/9, `31308f76d` 12/12, `c2353d546` 8/8, `4adf1e35a` 19/19, `001e94f46` 9/9, `a49821e1a` 24/24, `bf19bd716` 12/12, `f39287bf1` 24/24, `1744df49c` 59/59, `dee186645` 11/11, `78963f02b` 20/20, `8f68f3447` 14/14, `7c71ffd8d` 86/86. Counts are receipt entries across revisions, not distinct implementations or executed cases. No mismatched or absent entry is credited as a verified candidate byte.
