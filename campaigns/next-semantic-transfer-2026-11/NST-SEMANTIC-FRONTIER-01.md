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
