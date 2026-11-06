# M16 foundations — five-task transfer audit

Read-only against accepted main `33606d916` and original contracts at `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Dirty A1 is excluded. No tests/builds, network, Git mutations or code/spec/ledger edits. The sibling JSON retains the five complete tasks,53semantic clauses,32source hashes and the accepted M13C2 comparison receipt.

## Proposed status

| Original task | Proposed status | Main gap |
|---|---|---|
| A1 — whole-product capability census | not-transferred | No exact CLI/subcommand/TUI/model/MCP denominator with stable coverage rows and executable mapping. |
| A2 — authority/observability map | not-transferred | Existing domain contracts are not reconciled into the required whole-product flow/epoch/completeness map. |
| B1 — strict operation/capability contracts | partial | Strict generated domain APIs exist; common registry/descriptor/status model and universal advertised admission are incomplete. |
| B2 — entity/composite/location views | partial | Strong domain observation/occurrence/projection identities exist; no shared composite snapshot/paging/opaque resource model. |
| B3 — negotiated local session | not-transferred | Legacy MCP and one-shot calls exist; the separate framed session, negotiation and assigned trusted context IDs do not. |

Proposed bounded partition: **0full/2partial/3not-transferred/0unknown**. Root decides promotion. Existing names/types need not match the prototype; actual semantic owners were checked. No full task is inferred from generic infrastructure or matching operation names.

## Concrete B1 admission debt — source-proven path, not an executed test

`crates/vibe-mcp/src/tools/query.rs:60–68` advertises optional integer `limit` and `additionalProperties=false`. `run` at73–94 reads optional strings with `as_str`, reads limit with `as_u64`, substitutes DEFAULT_LIMIT on failed conversion and calls the shared trace query. `crates/vibe-mcp/src/lib.rs:293–314` dispatches raw arguments directly; it does not validate the tool's advertised schema. The actual default is50 at `crates/vibe-trace/src/search.rs:29`.

Minimal discriminators for a future explicitly bound test:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"query","arguments":{"limit":"wrong"}}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query","arguments":{"unexpected":true}}}
```

Expected under the advertised schema and original B1 requirement: typed invalid-arguments/preflight refusal before trace-domain entry. Actual **from inspected source**: the first takes limit50; the second ignores the unknown field; both call `vibe_trace::search::query` with the trusted server root. A valid fixture project may then produce a report, while an invalid root may fail inside the domain. No runtime response, successful output or filesystem effect is claimed here because neither request was executed. This is a concrete path to reproduce/fix, not a speculative missing-type observation. Other optional strings on this tool have analogous dropped-wrong-type handling; this audit does not assert every tool is defective.

Positive comparison: accepted `21d7683e027d81f14b2528734dd77bb7da236821` already implements strict text-only migration planning. `decode_migration_request` checks present optional values before Option lowering, generated deserialization rejects unknown fields/types, schema epoch is checked, and produced reports are validated against domain recipe meaning. CLI and MCP call the same workspace owner. Actual accepted receipt includes domain/CLI/dispatcher/corpus parity and no-state-creation evidence. LifecycleRunArgs separately uses `deny_unknown_fields` before execution. Preserve and reuse these patterns; merely wrapping a permissive handler in a new envelope does not repair admission.

## Existing owners worth keeping

**Canonical formats and strict domain services.** `formats/REGISTRY.toml`, generated FormatId/JTD types, shared vocabularies and wire behavior validators are the existing schema authority. Migration request/report/error and lifecycle reports demonstrate shared domain models across adapters. ToolOutput preserves executed success/failure, optional structured content and actionable text; typed migration preflight remains distinguishable from an executed report. ToolOutput itself carries generic JSON and is not a universal schema validator.

**MCP is one adapter, not the product denominator.** Its default registry has14tools at this pin. CLI has many additional administrative, conversion, settings, registry, binary and application operations. The `tree` build/model code still lives under the CLI and serves JSON/plain/TUI, while display selection state is intentionally excluded from its machine model. The format registry inventories serializations, not every semantic action. The222-task architecture map inventories campaign work, not public action coverage. Neither list can substitute for A1's explicit operation/model census or prove API coverage from name counts.

**Retained extension discovery data exists.** ExtensionWorld retains selected manifests and provider identities for owner projections; registry model separates typed host/dependency providers. ExtensionDecl has handler/pass/compiler controls but no authored machine-operation-schema field at this pin. That existing retained projection is the right place to carry admitted descriptor metadata later, without executing a native library or fetching a schema during discovery. A retained descriptor must remain source-observed/noncallable until actual handler/authority admission, and a selected manifest is not physical-content authentication.

**Existing identity semantics are already useful.** Requirements observation IDs bind source/query/report meaning and explicitly exclude wall-clock observed_at. Compiler F19 occurrence paths differ from target/effective-view identity and emitted positions. Specdoc SourceKind::XmlProjected marks projection-relative diagnostics rather than pretending they name original XML coordinates. Tree models distinguish unique package IDs from contributions and omit TUI-only selection state. These satisfy real parts of B2; do not replace them with display-line or timestamp IDs.

Missing B2 is the joined client-facing layer: logical entity versus capture/content versus observation versus presentation occurrence, compatible participating/nonparticipating domain observations, explicit completeness, stable filters/pages/chunks under one projection, and trusted opaque resource/URI/location mapping with encoding/revision/nonUTF8/unavailable cases. Current requirements queries reobserve and have bounded truncation, not same-snapshot paging. F19 effective pre-Embed coordinates are not authored edit ranges. A struct called CompositeSnapshot would not make independent domains atomic.

## Legacy transport is compatibility, not an alleged version bug

Existing MCP Transport is newline-delimited strings; StdioTransport runs until EOF. `initialize` returns fixed `2024-11-05` and tools capability, exactly the historical behavior the original B3 task says to preserve. Its age is not evidence of a defect in this audit. B3 specifically calls for a **separate** byte-framed, on-demand session with a finite length limit, negotiated schema/API/position/capability values and a domain facade adapter; no such session owner/corpus exists at the accepted pin.

ServerContext already carries trusted root composition rather than taking arbitrary roots from every tool call. The migration grant builder privately retains explicit project/cache/recovery roots and finite allowed files; request fields must match, and later changed server roots refuse. This is a sound domain authority seam, but it is not the requested per-client assigned context/target ID registry for workspace/member/HOME/container contexts.

One-shot use is independently real: CLI machine_entry precedes settings/tracing/context initialization, and pure workspace planning consumes supplied text. It must continue working without a server. A future session should reuse this domain publisher through a thin adapter; no duplicate migration decoder/replay, eval/shell endpoint, permanent daemon or new scheduler is needed. Streaming/replay/control flags must reflect executable capabilities, not a fake transport fixture.

## Architectural consequences for M02/M10

| Need named by upper tasks | Real prerequisite | Not a prerequisite merely by label |
|---|---|---|
| Complete change fact selection (M02C2) | Retained source set, exact selected addresses, completeness and compatible observation identity | Full all-domain transport/session server |
| Dirty change analysis (M02B3) | Shared client-scoped dirty/saved document observations and location basis | MCP descriptor inventory alone; a disk migration lock is not editor coordination |
| Derivative source/boot navigation (M10D1) | Base/local/result domain identity plus occurrence/source-location joins | New compiler, graph engine or scheduler |
| Authored derivative apply/rebase (M10C2) | Real shared saved-editor publication/recovery authority when that later owner is available | B2 data model or B3 context ID by itself does not grant writes |
| Pure domain planning / local models | Existing strict domain schema and functions, explicit scope | A mandatory server; accepted M13C2 already proves independence |

A1/A2 should capture current interfaces and desired gaps before broad facade rollout, but must not become a ceremonial gate blocking correct independent domain work. B1 should reuse the existing generator/format policies and shared service owners; the actual query admission discrepancy is a bounded concrete obligation, not a reason to introduce another dispatcher engine. B2's pure identity/location/compatibility subset is an early useful foundation; source/world/target/operation axes can be explicitly nonparticipating or unverified rather than waiting for every downstream runtime. Full source/target guarantees still cannot be invented by the model.

B3 follows real domain publishing and trusted context registration. Later C/D action scheduling, mutation ownership, cancellation, document buffers and editor barriers are outside this five-task audit and are not commissioned here. The full original A1/A2 census and B1/B2/B3 requirements remain outstanding even if one concrete adapter bug is fixed.

## Bounds and uncertainty

No full command/action census was constructed by this audit; source inventories establish missing deliverables and specific seams, not a fresh whole-product denominator. Original A2 names D025/D026 obligations, but their full decision bodies were not supplied/read in this bounded packet; no complete assignment is claimed. PROP031/032 still contain proposed/open graph/refactoring elements; their desired text is not runtime proof. All five statuses remain root-reviewed proposals, and historical accepted gate receipts are not fresh tests.

## M-16-A.1 — exact clause matrix

| Original clause | Assessment | Evidence / missing semantics |
|---|---|---|
| steps 1: Enumerate every public CLI/subcommand/TUI semantic action and MCP registration, then trace its owning library, effects and existing structured carriers. | partial | CLI dispatcher, refactor/tree owners and MCP14registration list exist; no accepted whole public CLI/subcommand/TUI action census. |
| steps 2: Enumerate resolver/compiler/trace/lifecycle/receipt/tree/cache/settings/registry/bin/refactor models and all planned M-01 through M-15 operation families; distinguish presentation-only state. | partial | Format registry/domain schemas and task architecture list different inventories; no complete public semantic-model/operation denominator joining planned M01-M15. |
| steps 3: Create a proposed versioned CapabilityCoverage inventory with stable IDs, owner/schema/source/target/effect/authority/completeness/retention/test columns and explicit unimplemented rows. | missing | No versioned capability coverage register with owner/schema/effects/authority/completeness/retention/test columns. |
| positive_cases 1: Every discovered registration/model family maps to a stable coverage row. | missing | No stable coverage row/disposition for every registration/model. |
| positive_cases 2: Legacy administrative and source-conversion operations remain visible. | partial | Legacy/refactor/admin commands remain executable/discoverable in CLI source; no coverage rows retain them. |
| negative_cases 1: MCP's tool list is treated as the whole product census. | unverified | No full product census exists; MCP list cannot be treated as denominator. |
| negative_cases 2: Matching name counts without an executable path is marked covered. | missing | No registration-to-executable coverage checker. |
| acceptance 1: No identified command/action/model lacks a disposition and domain owner. | missing | No complete owner/disposition assignment. |
| acceptance 2: The register is interface coverage only, not a requirements engine or scheduler. | partial | Existing domain format registry is not a scheduler; requested interface register absent. |
| safe_stop 1: Census and observed gaps are complete against the inspected baseline; no runtime behavior changes. | missing | Census and gaps not complete against full product. |

Owners: `crates/vibe-cli/src/main.rs`, `crates/vibe-cli/src/commands/refactor/mod.rs`, `crates/vibe-cli/src/commands/tree/mod.rs`, `crates/vibe-mcp/src/tools.rs`, `formats/REGISTRY.toml`.

## M-16-A.2 — exact clause matrix

| Original clause | Assessment | Evidence / missing semantics |
|---|---|---|
| steps 1: For each coverage family identify retained authoritative records, currently CLI-local logic, reusable library seams and missing inspection surfaces. | partial | Existing retained domain records and library seams can be traced; CLI-local tree build remains; no full coverage-family authority map. |
| steps 2: Reconcile PROP-032/031 proposals with actual commands and define semantic/debug schema epochs, redaction/unavailable reasons and applicability versus implementation-gap states. | partial | PROP031/032 retain deterministic graph/refactoring design and explicit open work; no all-family semantic/debug epoch and gap-vs-unavailable reconciliation. |
| steps 3: Record transport-neutral discovery/capture/query/prepare/admit/submit/observe/recover flow and per-domain closure requirements; leave LSP/DAP/client adapters, plugins and MVP out. | missing | No complete transport-neutral operation flow and per-domain closure register. |
| positive_cases 1: Every semantic view links to its owning domain and a proposed executable conformance case. | missing | No each-view domain/conformance mapping. |
| positive_cases 2: Debug schemas can evolve independently while retained data remains interpretable. | partial | Existing compiler IR/report/schema epochs are separate; no common debug compatibility/retention policy across all domains. |
| negative_cases 1: Private debug memory is exposed as a stable ABI. | partial | Current typed generated roots avoid equating Rust Debug with wire; no all-domain exposure audit. |
| negative_cases 2: A supported feature's missing API is disguised as contextual unsupported. | missing | No explicit capability status distinguishing implementation gap from contextual unsupported across product. |
| acceptance 1: All D-025/D-026 obligations are assigned without replacing resolver/compiler/journals. | missing | No accepted D025/D026 whole-map assignment; full decision bodies are outside this bounded packet, so no completeness claim. |
| acceptance 2: The design identifies exact source families for downstream M-16/M-17 work. | missing | No exact full source-family assignment map for downstream foundations. |
| safe_stop 1: Reviewed authority/coverage map is ready for permanent specification promotion; no transport is implemented yet. | missing | Full authority/coverage map not delivered. |

Owners: `vibevm/vibespecs/common/PROP-031-algorithmic-refactoring.xml`, `vibevm/vibespecs/common/PROP-032-project-model-ide-substrate.xml`, `crates/vibe-cli/src/commands/tree/mod.rs`, `crates/vibe-workspace/src/extension_world.rs`.

## M-16-B.1 — exact clause matrix

| Original clause | Assessment | Evidence / missing semantics |
|---|---|---|
| steps 1: First create/update the governing permanent anchors in vibevm/vibespecs/common/PROP-080-machine-operations-and-observability.xml for this task's contract, with desired-versus-implemented status explicit. Only then add executable types or tests citing those anchors. Final promotion/retirement records evidence later; it is not permission to postpone the source contract. | missing | No accepted PROP080 common operation/facade contract. |
| steps 2: Extend owned JTD registry/codegen with namespaced operation/model/schema descriptors and typed prepared/input/admission/result/error/event unions. | partial | Existing JTD FormatId registry/shared vocabularies and domain request/report/error roots exist; no namespaced common operation/model descriptors and unions. |
| steps 3: Validate requests before dispatch and results/errors/events before publication; wrong types, unknown fields and unsupported epochs are explicit refusals. | partial | Migration request/results are strict; generic MCP dispatcher does not enforce advertised schema and query optional wrong types default (concrete discriminator below). |
| steps 4: Register domain handlers through shared application-service composition; discovery reads retained descriptors and never executes a provider or fetches a schema. | partial | Lifecycle/migration/query use shared domain owners; no retained callable-status/schema discovery facade or extension operation-schema projection. |
| positive_cases 1: One domain operation has identical validated shape through library and transport adapters. | covered | Accepted M13C2 library/CLI/MCP share exact validated domain shape and results. |
| positive_cases 2: An admitted extension schema is discoverable without native execution. | missing | ExtensionDecl retains handler/pass metadata but no admitted operation-schema declaration/discovery; native loading not needed for existing retained rows but schema join absent. |
| negative_cases 1: Wrong optional argument types silently select defaults. | contradicted | query.run optional limit as_u64 failure takes DEFAULT_LIMIT; wrong optional strings are similarly dropped. Source inference, not executed test. |
| negative_cases 2: An advertised restrictive schema is not enforced at dispatch. | contradicted | query advertises additionalProperties=false and type constraints, but dispatch/run do not enforce them before domain call. |
| acceptance 1: Capability implementation status, contextual availability and authorization are separate fields. | missing | ToolDescriptor has name/description/inputSchema only; implementation/context/authorization not separated common fields. |
| acceptance 2: No eval/shell endpoint or duplicate domain engine appears. | partial | No new eval/shell or duplicate engine introduced by existing strict adapters; common facade absent. |
| safe_stop 1: Contracts and fixture dispatch are validated; real mutating handlers remain behind existing authority. | partial | One strict domain path works; common contracts and extension discovery not delivered. |

Owners: `crates/vibe-mcp/src/lib.rs`, `crates/vibe-mcp/src/tools/query.rs`, `crates/vibe-mcp/src/tools/migration_plan.rs`, `crates/vibe-mcp/src/tools/lifecycle_run.rs`, `crates/vibe-workspace/src/migration_planning.rs`, `crates/vibe-core/src/manifest/extension.rs`, `crates/vibe-workspace/src/extension_world.rs`, `formats/REGISTRY.toml`.

## M-16-B.2 — exact clause matrix

| Original clause | Assessment | Evidence / missing semantics |
|---|---|---|
| steps 1: First extend the permanent PROP-080 source anchors for logical/capture/observation/occurrence identity, composite snapshot and paging compatibility, and source-location basis with desired-versus-implemented status. Only then add code and focused tests citing those anchors. | missing | No accepted PROP080 entity/composite/paging/source-location law. |
| steps 2: Define separate logical entity, capture/content, observation/snapshot and presentation occurrence references using existing domain IDs where valid. | partial | Compiler occurrences, package IDs, source hashes and requirements observation IDs exist separately; no shared navigable entity/capture/observation/occurrence view. |
| steps 3: Describe CompositeSnapshot as compatible source/world/target/operation observations with completeness; bind filters/pages/children/chunks to one projection and refuse mixed versions. | missing | No CompositeSnapshot compatibility model or bound filters/pages/chunks; bounded queries reobserve each call. |
| steps 4: Define opaque resource IDs, trusted URI mapping and raw/projection position basis with encoding/revision, including non-UTF8 resources and unavailable source mappings. | partial | Raw Markdown versus projected XML notice exists; no opaque resource map with encoding/revision/nonUTF8/unavailable-location states. |
| positive_cases 1: Repeated inclusion shares logical identity but has distinct occurrences. | partial | F19 recursion path preserves repeated occurrence independent of target/effective-view identity; common external entity projection absent. |
| positive_cases 2: All pages from one captured query reconstruct the full authorized structure. | missing | No same-capture pages reconstructing authorized complete structure. |
| negative_cases 1: Wall-clock time or display line is used as exact snapshot identity. | covered | Requirements observation digest excludes observed_at and binds source/query meaning; F19 occurrence identity is not emitted line position. |
| negative_cases 2: Projected Markdown positions are presented as original XML locations. | covered | SourceKind::XmlProjected explicitly marks projection-relative locations; F19 documents effective-view rather than raw authored coordinates. |
| acceptance 1: A client can navigate provenance without unrestricted host paths or false atomicity claims. | partial | Existing models expose domain provenance but tree includes root paths; no trusted opaque URI mapping or composite false-atomicity guard. |
| acceptance 2: Every incomplete/inconsistent mapping has a typed reason. | missing | No common complete set of typed inconsistent/incomplete mapping reasons. |
| safe_stop 1: Pure models and projections are tested; no watcher or disk mutation is added. | partial | Domain identity/location models exist; composite client model/paging not delivered. |

Owners: `crates/vibe-cli/src/commands/tree/model.rs`, `crates/vibe-specdoc/src/load.rs`, `crates/vibe-spec/src/embed/occurrences.rs`, `crates/vibe-requirements/src/query.rs`, `crates/vibe-requirements/src/digest.rs`.

## M-16-B.3 — exact clause matrix

| Original clause | Assessment | Evidence / missing semantics |
|---|---|---|
| steps 1: First state the separate on-demand framed machine-session, negotiation, trusted-context and MCP-compatibility law in permanent PROP-080 with desired-versus-implemented status. Preserve the existing fixed MCP JSON-RPC initialize and newline stdio protocol as historical executable behavior. | partial | Legacy MCP newline/fixed initialize behavior exists; separate framed machine-session canonical law absent. |
| steps 2: Implement versioned local framed request/event transport with initialization for API/schema versions, position encoding and streaming/replay/control capabilities. | missing | No byte-framed versioned session/init/schema/position/capability negotiation. |
| steps 3: Register workspace/member/HOME/container contexts only from the trusted owning surface; requests refer to assigned context/target IDs. | partial | ServerContext and finite migration authority are trusted constructor inputs; no assigned workspace/member/HOME/container context/target ID registry. |
| steps 4: Map actual negotiated MCP versions/capabilities into the same domain facade while preserving explicit legacy tools; reject malformed frames, unknown versions and unbound roots. | missing | No negotiated session mapping to common facade; existing MCP fixed initialize is preserved legacy, not new negotiation. |
| positive_cases 1: Two clients negotiate supported versions and independently bind authorized contexts. | missing | No two-client independent negotiated context binding exchange. |
| positive_cases 2: One-shot library/CLI usage remains available without starting a server. | covered | Accepted pure library and CLI migration run without server/context startup. |
| negative_cases 1: Caller body path or workspace manifest self-authorizes a new root. | partial | Finite MCP migration grant rejects payload roots/files differing from configured authority; no generic session context-ID admission. |
| negative_cases 2: Initialization reports a fixed unsupported protocol version. | missing | No new session version-negotiation response; fixed legacy MCP version is intentional compatibility, not alleged defect. |
| acceptance 1: Framing and handshake are documented from executable behavior. | partial | Existing newline/fixed MCP framing is documented/executable; requested independent framing/handshake absent. |
| acceptance 2: No permanent daemon, LSP/DAP adapter, IDE plugin or MVP is introduced. | covered | No permanent daemon/LSP/DAP/plugin/MVP introduced. |
| safe_stop 1: Transport and fake service exchanges pass; long-running action ownership lands in M-16-C. | missing | Separate framed session/fake service exchange not delivered; no C/D action owner work audited. |

Owners: `crates/vibe-mcp/src/lib.rs`, `crates/vibe-mcp/src/transport.rs`, `crates/vibe-mcp/src/context.rs`, `crates/vibe-mcp/src/context/migration.rs`, `crates/vibe-cli/src/main.rs`, `crates/vibe-cli/src/commands/refactor/migration_plan.rs`.


## Root disposition

Root verified32 pinned hashes and5 complete original contracts, read all53 clause assessments and inspected the actual MCP query optional-value lowering. B1/B2 partial; A1/A2/B3 not-transferred. This is source-audit acceptance, no runtime reproduction or completed task. The concrete query admission gap remains owned by this campaign under M16B1, not deferred outside its mandate. A new facade is not required to repair a permissive adapter.
