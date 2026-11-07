# SYSTEM draft technical review

Verdict (updated): all five prior clarifications are resolved in the revised draft. No remaining concrete technical contradiction found within reviewed scope. Direction is consistent with existing boot owners. Literal prelude, static-link-only prefix and live INDEX suffix are feasible as a new checked projection; no second compiler or second frozen profile is needed. No implementation or tests were run. Findings are specification clarifications, not demonstrated runtime regressions of an unimplemented feature.

## T1 — P2-design-clarification

Target: SYSTEM-PROMPT-SPEC.md §7; SYS-CAPTURE/SYS-OPERATIONS

Read-only prepare must not invoke an effectful native/backend provider to recompute freshness. Existing analyze bound/native APIs accept OwnerNativeCompileProvider; reuse of those entrypoints alone does not establish no-network/no-write capability.

Evidence: install/bootgen/analyze.rs::analyze_node_lane_bound_native_using, compile_node_backend_using and analyze_effective_bound_native_with_sources explicitly pass a provider into managed compilation; native_managed.rs binds continuations to owner epochs.

Suggested edit: Define check/prepare as admitted finite read and witness comparison, plus only explicitly effect-free builtin preparation if needed. Unsupported/pending/native recomputation requires explicit build/preparation and otherwise returns unavailable/stale. Do not fall back to an empty plan. Add corresponding contract to SYS-CAPTURE/OPERATIONS.

## T2 — P2-design-clarification

Target: SYSTEM-PROMPT-SPEC.md §7

Runner prepare needs an explicit boundary from ordinary session boot, because PROP-009 forbids changing the redirect into run vibe, even if that command is read-only. Draft is compatible if prepare belongs to API transport preflight, but wording should close this implementation shortcut.

Evidence: PROP-009#PURE-FILE-READING and #REJ-RUN-VIBE; boot_artifacts/redirect.rs renders read STATIC then INDEX instructions.

Suggested edit: State that ordinary on-disk redirects and file-only boot remain unchanged, including opt-in projects. API runner invokes prepare outside model boot as its explicit request preparation; exported bridge never asks agent to execute prepare.

## T3 — P2-design-clarification

Target: SYSTEM-PROMPT-SPEC.md §7 publication

Format-change or first-init can change AGENTS managed redirect in the same publication as SYSTEM. Witnessing pre-write AGENTS would make a newly published sidecar stale immediately or bind prompt to the wrong generation.

Evidence: boot_artifacts.rs::write_boot_artifacts_inner compiles first, then transaction::write_production_with_selectors invokes redirect::write_redirect_blocks_with_transaction; redirect.rs replacement depends on target spec_format.

Suggested edit: Prepare the prospective managed-redirect projection through the same redirect owner, retain source-before and expected-source-after separately, and publish/check prompt, sidecar and changed redirect as one coherent generation under existing transaction extension. Do not read AGENTS again after publication to invent evidence.

## T4 — P3-precision

Target: SYSTEM-PROMPT-SPEC.md §4

Literal template presence opt-in should explicitly remain an export-input capability, not native scan enrollment or generated-output ownership. Explicit empty project.sources otherwise creates an ambiguous choice between no semantic sources and template auto-discovery.

Evidence: own_boot.rs::PreparedNodeBoot respects policy and captured native scan entries; PROP-072 SOURCE-HOST-POLICY-LAW and SOURCE-OPERATION-ENROLLMENT-LAW distinguish explicit raw reads from automatic native semantics; layout.rs reserves only current generated paths.

Suggested edit: Template exact-path opt-in is explicit literal export selection under held read owner, independent of native source scanning; it is excluded from own boot without becoming generated or acquiring VibeSpec meaning. Existing native content policy remains unchanged.

## T5 — P3-precision

Target: SYSTEM-PROMPT-SPEC.md §4/§11 item 2

No-DSL template may contain prose/examples mentioning SYSTEM.md; rejecting such strings as indirect self-inclusion would contradict literal semantics.

Evidence: Draft §4 says curly braces and examples are literal; existing compiler distinguishes actual semantic source edges from provenance/elided/hoisted inputs.

Suggested edit: Define self-inclusion rejection over admitted dependency edges and actual file aliases only, never prose mentions/Markdown links without an explicit semantic loading role.

Other checks: selected-node ownership protection, mixed/XML stream distinction, quiet-worker exclusion, prefix versus provider cache identity, literal authored instructions and runtime coverage qualifications are retained correctly. §6 original hashes must stay honest observed original/output witnesses: converted slot output never proves unavailable original bytes. No need to hardcode provider economics into this boot layer. Source hashes below are current file seals, not an atomic repository snapshot.

## Revised-draft disposition

Re-read only SYSTEM-PROMPT-SPEC.md, SHA256 fe96762ad56c12c646d63dac8238c0d297a1ad9b45e918eb67fbad7549d6a270. T1 resolved by §7 effect-free prepare capability/refusal and acceptance 10 provider sentinels. T2 resolved by explicit external API preflight and unchanged file-only AGENTS boot. T3 resolved by same-generation prospective entrypoint witness. T4/T5 resolved by §4 explicit ordinary template role independent of empty project.sources and semantic-edge self-inclusion. Acceptance 5 now preserves permitted root subtree aggregation inside selected effective scope. All earlier findings above are historical review evidence and are closed at the specification level. No new feature request, implementation verdict or runtime test claim.
