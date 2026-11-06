# Remaining M02 transfer audit — five exact original tasks

Read-only source audit at main `b1b43b0da96b8d9dea82e86696760bd887b9947c`; prototype contracts from immutable `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. No tests/builds, Git mutations, network or product writes. The sibling JSON stores all five complete original contracts,44semantic clauses and26source bindings with hashes. This report proposes classifications; root retains acceptance and ledger authority.

| Original task | Proposed status | User-visible meaning still missing |
|---|---|---|
| M-02-B.3 — saved/overlay change analysis | partial | Two editors/agents can inspect different unsaved changes without mixing bytes; dirty plans cannot gain write authority merely after a save. |
| M-02-C.1 — retained origin and explicit rebase | partial | Updating a package cannot silently redefine what a change agreed to implement; drift and old/new fact differences are reviewable. |
| M-02-C.2 — complete fact selection | partial | A change selecting more than a query page receives every selected fact or exact gaps, never an apparently complete truncated set. |
| M-02-D.1 — no-overwrite scaffold | not-transferred | Create actual local change files from an exact origin, with ownership and interruption evidence, refusing existing or stale destinations. |
| M-02-D.2 — shared change inspection | not-transferred | Usable check/show/coverage through one semantics owner and thin CLI/MCP/service adapters. |

Proposed partition of this bounded five:0full/3partial/2not-transferred/0unknown. This is not five completed tasks and makes no global113 count claim. Partial status means an actual sub-obligation already has useful accepted support, not that an arbitrary percentage is done. D1/D2 have reusable prerequisites but none of their defining scaffold/change-service operations, so prerequisite existence alone is not counted as partial delivery.

## What accepted main actually supplies

Accepted `940f93db4115c83484916849580f085d47c4b8ae` gives strict ChangeManifest/role grammar, exact origin declarations, immutable IDs, saved role-byte observations and independent intent/work hashes. `read_saved_change` opens the trusted selected node, reads change manifest and each declared input through incumbent source reads, retains bytes, then `identity_analysis` consumes those retained bytes without disk reopening. The canonical PROP071 explicitly says this is per-file observation, not an atomic whole-change snapshot or authenticated origin. It expressly excludes scaffold, overlay, scheduler and closure publication. These are meaningful foundations for B3/C1, not their missing joins.

`ChangeOrigin` stores exact package pin and declared manifest/document SHA256. There is no accepted origin resolver comparing those declarations against retained captured source, no current-versus-accepted origin report, and no explicit old/new fact rebase service. Generic lock selection/reinstall cannot be credited as that capability: ordinary requirements observes current selected package slots, while the change declaration merely stores expected identity. A matching editable digest is not authentication.

Existing requirements query already has valuable semantics that should survive C2/D2: one selected discovery epoch, one registry read, per-source facts/witnesses, separate authoring/adoption, typed available/invalid/unavailable/orphaned states, unclassified requirements, exhaustive artifact kinds and no terminal verdict. Its `all_rows` vector is assembled before globally sorting/truncating to the effective1..256 limit. The result exposes `truncated`; it has no cursor or selected-address complete-set API. Repeating prefix queries rereads current sources, so assembling multiple pages is not same-snapshot proof.

The committed `vibe-facts::observe_authored_source` returns the entire per-source fact list with raw-byte witnesses. It is a useful complete-source primitive, but its result does not retain all raw source buffers or a whole multi-source authority/membership epoch. The separate source path opens/walks and finite read window must not be relabelled immutable capture merely because a digest exists. Current dirty A1 descriptor/scanner implementation is deliberately excluded; committed scan bytes at the fixed main pin are authoritative here.

Accepted `a4b322150c494e5d7245252ace109b7be9e62c1a` supplies strong M13D1 migration publication/recovery for one explicitly granted existing authored file. Runtime `apply_plan` observes that target and refuses if it is absent. It does not prepare a missing change directory, publish a set of new files, track editor buffers or capture origins. Its designated physical project/recovery-store barrier coordinates participating disk writers; calling it M16 client/editor epoch coordination would be false. Reuse its actual SafeFS/durability primitives when justified, but do not enlarge its accepted guarantee by naming it a scaffold transaction.

## Architectural order and next useful atom

Among these five, C2 is the earliest useful read-only domain capability: exact selected-address observation over one retained source set, with explicit available/missing/unavailable/unclassified/out-of-scope outcomes and complete inventory. It consumes source interpretation/capture semantics; current A1 must be accepted before relying on its new reader. Inspect/reuse incumbent scan/query assembly, and where stronger membership/capture is truly needed consume the M01 owner rather than building a competing service. Preserve the ordinary paginated query and its compatibility limits. A loop that rereads pages is not a substitute.

C1 then binds and compares accepted origin captures and prepares a nonpublishing old/new fact rebase. B3 can progress on its independent reader adapter only after the shared M16 snapshot/overlay interface exists; dirty-buffer provenance and saved-write admission are not obtained from a filesystem lock. D1 needs exact origin/completeness plus shared saved/editor admission and a real absent-destination publication design. D2's pure check/show/coverage domain and thin surfaces can precede D1's effectful creation; the original criteria do not require waiting for scaffolding to expose existing changes. It must not manufacture another verdict engine.

## Evidence limits

Accepted-record gate receipts are historical evidence, not fresh executions in this audit. Inspected test definitions include no-new-files requirements queries, honest bounded truncation, preserved authored required kinds and explicit absence of terminal/fulfilled/verdict fields. No dedicated >256 complete change selection, two-client dirty change, origin drift/rebase, scaffold interruption or change surface parity runtime is claimed. Source searches were checked against complete public module/registration inventories and actual implementation bodies; absence conclusions are bounded to these change operations, not assertions that every generic helper is missing. No original filenames/types are required when an equivalent accepted semantic owner exists.

## M-02-B.3 — Integrate saved-state and overlay change analysis

| Exact clause | Assessment | Evidence meaning |
|---|---|---|
| steps 1: Connect change parsing to the accepted source/overlay snapshot interface. | partial | Saved parser retains manifest/role bytes; no source/overlay snapshot interface. |
| steps 2: Allow pure diagnostics over a client's dirty change documents without updating authored files or lock. | missing | No client-scoped dirty-document analysis input/service. |
| steps 3: Prepare future authored changes only from saved captured state; revalidate buffer/disk epochs through the existing writer coordination contract. | missing | Pure saved migration proposal has no buffer/disk epoch coordination; M13D1 is a disk writer barrier. |
| positive_cases 1: Two clients analyze different dirty refinements without cross-client leakage. | missing | No two-client dirty refinement service or fixture. |
| negative_cases 1: Dirty or changed affected document produces save-required/stale before writes. | missing | No change dirty-buffer save-required/stale admission. |
| negative_cases 2: A saved buffer does not automatically authorize an older dirty-buffer plan. | missing | No client epoch identity to invalidate earlier dirty plan after save. |
| acceptance 1: Analysis results identify the exact overlay/disk snapshot and location basis. | partial | Retained per-file bytes/paths exist; overlay/disk snapshot ID and location basis absent. |
| acceptance 2: No new editor coordination or writer authority duplicates M-16. | unverified | No duplicate editor protocol only because no joined editor implementation; not completion. |
| safe_stop 1: Discard overlays without saving; no execution or closure writes. | partial | Existing saved read is nonpublishing; overlays are absent. |

Source owners: `crates/vibe-requirements/src/change/read.rs`, `crates/vibe-requirements/src/change/identity.rs`, `crates/vibe-mcp/src/context.rs`, `vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml`.

Next bounded atom: Adapt change parser to shared M16 client dirty/saved snapshots and coordination epochs; do not invent competing editor protocol.

## M-02-C.1 — Retain accepted origin captures and explicit rebase

| Exact clause | Assessment | Evidence meaning |
|---|---|---|
| steps 1: Resolve each origin against M-01 exact captures and store accepted identity in authored change intent. | partial | Strict ChangeOrigin stores exact pin and declared hashes, but no capture resolution/proof. |
| steps 2: Compare current selected capture to accepted origin without replacing it silently. | missing | No current selected capture versus accepted origin comparison owner. |
| steps 3: Implement explicit rebase preparation with old/new fact identity diff and saved-state preconditions. | missing | No old/new fact diff or saved-state rebase preparation. |
| positive_cases 1: Reinstall reuses the accepted origin capture; explicit rebase binds a new one. | missing | No change-origin reinstall/rebase integration; generic reinstall is insufficient. |
| negative_cases 1: Same-version changed content reports origin-drift. | missing | No same-version origin-drift result. |
| negative_cases 2: Missing retained origin cannot be replaced by newest SemVer. | partial | Exact pin syntax exists; missing retained capture has no implemented lookup policy. |
| acceptance 1: Origin/rebase reports show exact old/new capture and affected fact addresses. | missing | No origin/rebase report with old/new captures and affected facts. |
| acceptance 2: Package slots and historical accepted evidence remain unchanged. | partial | Saved read/proposals preserve slots/evidence; actual rebase path absent. |
| safe_stop 1: Return drift/unavailable with a reviewable rebase plan. | missing | Reviewable drift/unavailable/rebase result absent. |

Source owners: `crates/vibe-core/src/manifest/change/mod.rs`, `crates/vibe-requirements/src/change/identity.rs`, `crates/vibe-requirements/src/sources.rs`.

Next bounded atom: Compare accepted origin against selected exact source captures and prepare explicit fact rebase; no mutable latest substitution.

## M-02-C.2 — Provide a complete same-snapshot fact selection seam

| Exact clause | Assessment | Evidence meaning |
|---|---|---|
| steps 1: Add exact-address batch/internal complete observation over one captured source set. | partial | Unbounded per-source fact observation exists; no selected-address batch over retained whole source set. |
| steps 2: Preserve missing/unavailable/unclassified distinctions and explicit out-of-scope omissions. | partial | Invalid/unavailable sources and unclassified rows differ; exact missing/out-of-scope inventory absent. |
| steps 3: Use snapshot-bound pagination where necessary; refuse mixed captures and propagate completeness to change analysis. | missing | Query truncates and has no continuation; repeat calls reobserve; no change completeness propagation. |
| positive_cases 1: A fact set larger than the existing query page limit is completely selected. | missing | No selected set larger than256 through change seam; private pre-truncation vector is not that capability. |
| positive_cases 2: Spec-only requirements stay representable. | partial | Specification kind is preserved by exhaustive mapper; inspected fixture includes all kinds, no dedicated change spec-only scenario. |
| negative_cases 1: Truncated pages, changed membership and unresolved selected facts cannot appear complete. | partial | truncated flag is honest; no joined classifier covers membership drift and missing selected addresses. |
| acceptance 1: Selected-address inventory equals observed complete universe or lists exact gaps. | missing | No selected-address universe equality or exact gap result. |
| acceptance 2: Existing neutral query semantics and limits remain explicit compatibility behavior. | covered | Current query has explicit1..256 bound, sorted truncation, neutral metadata semantics and inspected regression assertions. |
| safe_stop 1: Expose incomplete analysis, never allow incomplete data to certify closure. | partial | Bounded query exposes incompleteness; no change closure/report consumes it. |

Source owners: `crates/vibe-facts/src/scan.rs`, `crates/vibe-requirements/src/query.rs`, `crates/vibe-requirements/src/sources.rs`, `crates/vibe-requirements/src/rows.rs`, `crates/vibe-requirements/src/tests_query.rs`, `crates/vibe-requirements/src/tests_requirements.rs`.

Next bounded atom: First among these five: complete exact fact selection over one retained source set, preserving bounded neutral query compatibility; source interpretation/capture prerequisite must be accepted.

## M-02-D.1 — Implement no-overwrite change scaffolding

| Exact clause | Assessment | Evidence meaning |
|---|---|---|
| steps 1: Prepare new --from output from one selected origin/node and the accepted parser/hash contract. | missing | No new --from preparation over exact origin/node. |
| steps 2: Validate destination absence, role paths, captured origin and writer preconditions before creating anything. | missing | No scaffold destination/origin/editor-precondition admission. |
| steps 3: Apply owned files with retained before-state/journal semantics; return exact created artifacts and partial failure truth. | missing | M13D1 replaces one existing approved file and refuses absent target; no multi-artifact creation journal. |
| positive_cases 1: A new scaffold references origin facts and adds only local roles/refinement. | missing | No origin-reference/local-role scaffold output. |
| negative_cases 1: Existing change directory, dirty affected buffer or stale origin refuses without overwrite. | missing | No joined existing-dir/dirty-buffer/stale-origin scaffold refusal. |
| acceptance 1: Prepared diff matches authored created files and declared IDs. | missing | No prepared scaffold diff/artifact/ID result. |
| acceptance 2: Scaffolding does not adopt package facts or start lifecycle execution. | unverified | No scaffold route; effect-free parser is not a scaffold implementation. |
| safe_stop 1: A prepared scaffold is safe to inspect; reject rather than overwrite. | missing | Inspectable prepared scaffold absent. |

Source owners: `crates/vibe-core/src/manifest/change/mod.rs`, `crates/vibe-requirements/src/change/mod.rs`, `crates/vibe-orchestrator/src/lib.rs`, `crates/vibe-cli/src/cli.rs`, `crates/vibe-workspace/src/migration_application.rs`, `crates/vibe-workspace/src/migration_application/runtime.rs`.

Next bounded atom: After capture/completeness/saved-editor admission: prepared no-overwrite scaffold via incumbent filesystem ownership/recovery; not one-file replacement pretending to create a directory.

## M-02-D.2 — Expose check, show and coverage through shared services

| Exact clause | Assessment | Evidence meaning |
|---|---|---|
| steps 1: Register generated change query request/results using the canonical M-16 service facade. | missing | No generated change query facade/registration. |
| steps 2: Implement check/show/coverage joins with exact origin/local provenance, completeness and separate work/evidence axes. | missing | No change check/show/coverage join with exact provenance/completeness/work-evidence axes. |
| steps 3: Add CLI/MCP/service parity including schema/unknown-field/refusal cases and empty-cache purity. | missing | No change CLI/MCP/service parity or strictness/purity case family. |
| positive_cases 1: Two changes sharing an origin expose different local intent with the same package capture. | missing | No two-change inspection result exposing shared accepted origin capture. |
| negative_cases 1: Work edge, adoption flag or incomplete result cannot become a satisfied verdict. | partial | Neutral requirements has no verdict and separate adoption; complete change join absent. |
| negative_cases 2: Read queries never install or mutate personal/adoption state. | partial | Existing requirements query is read-only with no-new-files assertions; change query absent. |
| acceptance 1: All surfaces return one semantic result and typed failures. | missing | No shared change semantic result across surfaces. |
| acceptance 2: M-02 is usable without starting or migrating an execution. | missing | Parser/identity library exists, but requested check/show/coverage access absent. |
| safe_stop 1: Accept read-only value while lifecycle/closure remain separate milestones. | missing | Read-only service value absent; parser is a separate accepted boundary. |

Source owners: `crates/vibe-requirements/src/change/mod.rs`, `crates/vibe-requirements/src/query.rs`, `crates/vibe-requirements/src/rows.rs`, `crates/vibe-mcp/src/tools.rs`, `crates/vibe-cli/src/cli.rs`, `crates/vibe-orchestrator/src/lib.rs`.

Next bounded atom: After read domains: one pure change report with thin generated CLI/MCP/service adapters; can precede effectful scaffolding, no second verdict engine.

## Operational note

The first JSON-report construction attempt failed with a Python SyntaxError before writing any file. Corrected the report-building script; no product file or ledger was changed. Both output reports are advisory evidence only.

## Root disposition

Accepted as source-audit classification after independently verifying26 pinned source hashes and all5 complete original contracts and reading the incumbent query assembly. No runtime tests executed and no full task promoted. Ledger now9full/27partial/18not-transferred/59unknown. The implementation-order recommendation remains subordinate to the complete222-task architectural dependencies.
