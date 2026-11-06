# M14 environment/target transfer audit — seven original tasks

Read-only audit against accepted main `e356a0cc1` (full pin in JSON) and exact task contracts at prototype `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. No Cargo/tests, network, Git mutations, code/spec/ledger edits. Dirty A1 is not evidence. The sibling JSON contains82semantic clauses, all complete original task contracts,49source hashes and known accepted F12/F14/F15/F16 commit ancestry.

## Proposed result

| Task | Proposed status | Defining missing behavior |
|---|---|---|
| M-14-A.1 — identity roles | not-transferred | Separate definition, lock, executor, target and generation with owned inputs and compatibility meaning. |
| M-14-A.2 — injected layout/prefix | partial | Existing injected app/project/layout roots are real; target principal/path policy and prefix compatibility are missing. |
| M-14-B.1 — realization contracts | not-transferred | Environment lock and full payload/native closure identities, target ABI and metadata capability contracts. |
| M-14-B.2 — legacy/supply readers | partial | Existing old records and exact source/mirror behavior remain; no legacy→new environment proof/prefix classification. |
| M-14-C.1 — target carriers | not-transferred | Binding carried through plan/state/native/receipt/inspection with early unsupported-effect refusal. |
| M-14-C.2 — physical target fence | partial | Existing ownership/locks are useful; cross-instance root identity, principal/generation and pinned bundle rechecks are absent. |
| M-14-C.3 — first bound effect | not-transferred | Actual host-issued bound grant and pinned-root UserProfile publication through the incumbent transaction. |

**0full/3partial/4not-transferred/0unknown proposed for this seven only.** Partial credits corresponding incumbent sub-obligations, not a claim that any whole M14 task landed. In particular ordinary home installation is not a bound environment apply. Root should review and decide ledger changes; this audit changes none.

## Owner direction and existing product foundation

The owner's detailed words are preserved in `research/environment-targets-metadistribution-question.md`: extend the meta-distribution behind `vibe install -g` beyond HOME to other roots/mount points; use the same graphical/console/agent tools to assemble understandable containers and eventually an OS. The record itself calls this an open architecture question and preferred hypothesis, not an approved implementation or Docker completion. It requires investigation; it does not justify a second package manager.

Accepted main already supplies much of the user-facing foundation:

- PROP059 defines the meta-distribution; user and project homes share layout. `SettingsRoot::new` and `SettingsRoot::project` are injected constructors. A project shares the user source cache but keeps mutable work in its own home. `Storage` supports explicit libraries with marker IDs and availability checks.
- `OperationContext` injects settings, choice/feature/parameter/license interactions, `on_plan`, structured progress, resolver/source reader, UserEnvironment and PATH. `operations/planning.rs` funnels install/maintenance through the existing step/configuration planner. F12/F14 preserve original selections, dormant/default/profile meaning, portable bindings, secrets' resupply requirements and real continuation.
- F16 supplies explicit ordinary configured package activation through the current solver/lock/source/apply owners. It is not a general arbitrary-target solver and retains its documented unsupported cases. F15 gives exact source preparation and early package admission, not a new backend.
- Existing deploy separates authored resources from physical lock resources, validates prior/foreign ownership and stores plan/generation sidecars. Its roots are injected by the command surface; provider config cannot choose bin/store roots or insert an environment.

These owners should be reused. The missing part is explicit target interpretation and authority across them, not another selection UI, resolver, source store, package kind or stewardship database.

## Concrete semantic gaps and disagreements

**A path override does not create an environment target.** App CLI builds `HostFacts` and `UserEnvironment::for_host` from current settings/user home; recipe and shell steps execute local child commands. Project-scoped external methods are explicitly still system-wide installations in PROP059. Merely replacing SettingsRoot with an offline rootfs would redirect some owned files while leaving host environment/system effects active. An offline target needs effect admission and appropriate execution adapters, not a fabricated HOME.

**Platform roles are currently narrower.** `NativePlatform::current` describes the current process ABI for loading native libraries. Its pure from_pair/from_key can describe other platform values but confer no execution capability. `TargetWhen` evaluates against an explicit injected OS, and app installer selection can explicitly choose platform/arch; neither establishes five separate environment identities. `build_target_for_binary` remains provider-fresh Cargo projection with no claimed complete input closure. Vibe-bin actually chooses `LauncherFlavour::NATIVE`; describing Linux while running on Windows does not make the provider choose the Linux launcher.

**Paths/markers are not durable root authority.** App `MutationLock` locks the home file and compares canonical path in require_home. Libraries compare marker IDs. These are useful current checks, but a reused pathname or copied marker is not an admitted root object/generation. Deploy destination locks are keyed by normalized resource spelling under its state home; differing state homes are not proved to coordinate aliases of the same target. Its DeploymentHome hash contains project/package/target, not environment instance/principal/generation. Root settings paths and per-call Project opens in vibe-bin are not a retained source/settings/bin/store capability bundle across the whole operation.

**Schema presence matters.** Accepted deploy intent has schema/plan_hash/target/resources/started_at and optional prior_generation; receipt carries legacy identity/profile/target/generation/provider/digests/resources/status. No environment binding is present in these inspected schemas or current DeployExecution/LockBinding. Artifact records already distinguish measured bytes, producer and optional freshness; they do not supply recursive environment closure, Linux payload UID/GID, link/mode inventory or exact target ABI identity. No environment/realization schema family or core environment role owner was found in fixed-pin inventories.

**Legacy safety is not a new proof.** Existing F14 compatibility and current source transport must survive. Source resolver accepts local, multi-registry and embedded routes; expected content hash is a mirror-selection hint, and the reproducing caller compares returned bytes to the exact pin. This is real non-GitHub-only source behavior. It does not classify legacy records into new target/prefix/hermetic guarantees, authenticate cache producers or prove recursive build/runtime closure. No old existence-only binary should become a realized environment merely by decoding it into new fields.

**Two existing placement owners are not a license to make a third.** App payload/proxies live under library/opt/app conventions; deploy:vibe-bin uses settings store/bin plus state/deployments. They have distinct established purposes. A shared target layout must name those roles and preserve their current namespaces, not silently merge/relocate them or build another parallel environment store. Any future unification would need an explicit migration decision.

**Original task cross-reference drift:** C1 step3/safe_stop says C2 supplies first effect. Exact C2 step4/acceptance and the dedicated C3 contract instead reserve first successful bound publication to C3. Follow the more explicit C2/C3 split: C1 carries/refuses, C2 fences/prepares, C3 enables the bounded actual builtin. Do not let stale prose enable effects early. None of these tasks claims Docker, arbitrary rootfs execution or live-system capability.

## Reuse / extend / separate

| Reuse existing owner | Required extension | Remains a separate adapter/responsibility |
|---|---|---|
| Shared configuration and installer option/step plan, callbacks and machine views | Bind platform/paths/needs to explicit executor versus target interpretation, preserve original requested intent | Docker/OCI packaging or remote executor; no second UI/evaluator |
| SettingsRoot, core layout/settings, Storage | Pure selected target roots, role-relative resource IDs, principal and target path/case policy; prefix compatibility | Privilege/account/service/mount/boot effects, explicitly unsupported until implemented |
| Ordinary package resolver/source/lock and accepted exact replay | Bind exact target ABI and complete claimed source/build/runtime closure to established owners | Native distribution backend versions/policies; keep opaque rather than force Vibe SemVer |
| Deploy ownership/sidecars/checkpoints and SafeFS primitives | One host-admitted physical root bundle and lock-scoped generation/principal validation throughout publish/receipt | Cross-target native/plugin/skill effects until equivalent authority exists |
| App durable continuation and portable source/target history | Retain target binding in actual future execution state without relabelling old records stronger | Legacy classification/migration with typed unavailable/needs-input |

## Next foundational atom

Start with **M14A1**, explicitly tied to existing callers: classify current platform/host meanings, establish canonical identity/compatibility contract, and implement pure definition/lock/executor/target/generation values with workspace and user-profile compatibility and explicit unsupported offline/live execution. Windows control plane describing Linux target must not select Windows output or claim it can run Linux code. Pure construction must not read HOME or start Docker. Do not introduce package kind/store/solver/UI or change bare-install behavior.

Then **A2** adds target path/prefix/layout policy through existing root owners. Source-portable filename constraints cannot be reused blindly for Linux payload names; target UID/GID is not a Windows account ID. Preserve current paths and distinguish relocatable versus prefix-bound versus unsupported payload. Target declaration remains unverified data until a physical owner admits roots.

B1/B2 supply coherent identities/legacy classification, C1 carries binding, C2 provides physical admission and partial-truth boundaries, and C3 finally routes one real local builtin through the incumbent transaction. The common installer/choice pipeline remains the semantic source for later container composition. This order addresses original113 foundations without implementing speculative future Docker work.

## Evidence limits

All code was read from the fixed accepted pin. No runtime pass, OS qualification or attack resistance is claimed by this audit. F12/F14/F15/F16 receipts are historical accepted evidence at their stated boundaries. Alias locking observations here are source inferences, not newly run concurrency tests. Literal prototype type names are not requirements; equivalent semantics in app/deploy were considered before marking gaps.

Two guessed discovery paths (`commands/install.rs`, `PROP-065-installer-steps.xml`) did not exist; corrected via repository inventory to `commands/install/mod.rs` and `PROP-065-structured-installation.xml`. Those failed reads changed nothing and are not product defects. No protected roots or personal environment were touched.

## M-14-A.1 — exact clause matrix

| Original clause | Assessment | Observed boundary / missing meaning |
|---|---|---|
| steps 1: First create/update the governing permanent anchors in vibevm/vibespecs/common/PROP-076-environment-model.xml for this task's contract, with desired-versus-implemented status explicit. Only then add executable types or tests citing those anchors. Final promotion/retirement records evidence later; it is not permission to postpone the source contract. | missing | No accepted PROP076 environment-role contract; meta-distribution vision is an open design input. |
| steps 2: Trace each existing host/OS/platform field to its current meaning; list old-reader behavior and exact callers requiring an explicit target later. | partial | Legacy meanings can be traced now: native platform=current process ABI, applicability=injected host OS, binary projection=provider-fresh Cargo target; no completed environment-role census accepted. |
| steps 3: Define EnvironmentDefinition, EnvironmentLock, ExecutionContext, TargetBinding and EnvironmentGeneration as separate identities; distinguish build machine, produced-tool runtime and compiler output target. | missing | No joined distinct definition/lock/executor/target/generation model; app home/context and deploy generation are narrower existing identities. |
| steps 4: Add pure type constructors and compatibility results for workspace, HOME, offline rootfs and future live targets; preserve existing bare-install behavior and reject unimplemented targets explicitly. | partial | Pure NativePlatform/from_pair and TargetWhen constructors exist; no workspace/HOME/rootfs/live-target compatibility model. |
| positive_cases 1: Existing project install binds to an explicit local target with unchanged semantics. | missing | Existing install is supported but does not bind an explicit environment target identity. |
| positive_cases 2: A Windows executor can describe a Linux target without selecting a Windows launcher. | missing | Explicit platform data can describe Linux; actual vibe-bin defaults launcher to NATIVE and has no separate executor/target binding. |
| negative_cases 1: Executor ABI is substituted for target ABI. | missing | No executor-versus-target ABI invariant in the environment owner. |
| negative_cases 2: Future live-system capability is advertised as implemented. | partial | No live-system environment capability is currently advertised; positive typed Unsupported contract absent. |
| acceptance 1: Each identity has named inputs and owned authority; no environment becomes a new package kind or stewardship DB. | missing | Five identity inputs/authority boundaries not established; no new package kind currently introduced. |
| acceptance 2: Pure construction needs neither Docker nor a host HOME lookup. | partial | Existing pure platform/applicability and injected-root constructors do not need Docker/HOME; complete environment construction absent. |
| safe_stop 1: Types and compatibility tests are coherent; downstream consumers remain on their explicit legacy interpretation. | missing | No complete environment roles/compatibility suite. |

Owners: `crates/vibe-core/src/manifest/target_when.rs`, `crates/vibe-lifecycle/src/native/platform.rs`, `crates/vibe-core/src/manifest/artifact/binary_projection.rs`, `crates/vibe-app/src/select.rs`, `crates/vibe-app/src/operations/mod.rs`, `crates/vibe-core/src/lib.rs`.

## M-14-A.2 — exact clause matrix

| Original clause | Assessment | Observed boundary / missing meaning |
|---|---|---|
| steps 1: First add the exact injected-layout/resource/path/prefix policy requirements to permanent PROP-076 and validate its existing canonical readers before executable A2 edits. Desired rules and observed implementation remain separate; no existing roots or provider config are relocated implicitly. | missing | No accepted canonical injected environment layout/prefix contract. |
| steps 2: Locate the existing layout authority named by the baseline and enumerate environment configuration, store, profile, state and cache roots. | covered | Existing owners are core layout/settings, app SettingsRoot, Storage and deploy_state_home; explicit root roles are enumerated by this audit. |
| steps 3: Implement pure root-relative resource identity with environment instance, principal, path/case policy and prefix compatibility; preserve the existing project vibevm/ layout. | partial | Injected user/project roots and library IDs exist; environment instance/principal/target path-case/root-relative resource identity absent. |
| steps 4: Represent relocatable, prefix-bound and unsupported payload compatibility and reserved future account/service/mount/boot resources; inject selected roots instead of re-reading ambient HOME/PATH. | partial | App/deploy callers inject roots and host facts; no relocatable/prefix-bound payload model or reserved future resource capabilities. |
| positive_cases 1: Two definitions can share immutable content yet have different target instance/root authority. | partial | Project/user app homes can share source cache while mutable homes differ; no immutable environment-definition/target-root authority proof. |
| positive_cases 2: Valid Linux payload components survive Windows control-plane validation. | missing | Source-portable component rules are not a target Linux payload path policy; no Windows-control/Linux-payload case. |
| negative_cases 1: A reused pathname silently transfers an old receipt to a different root. | partial | Library marker and home lock checks detect some mismatches; reused pathname is not pinned root-generation identity. |
| negative_cases 2: Arbitrary /usr-oriented binaries are declared HOME-relocatable. | missing | No explicit prefix-relocatability classification; arbitrary scripts/external installers are not proven HOME-relocatable. |
| acceptance 1: Every environment root has one layout constructor and callers receive explicit bindings. | partial | Existing roots have constructors, but no one environment-wide constructor combines all target roles/bindings. |
| acceptance 2: No root privilege broker, live system mutation or pacman policy is introduced. | covered | No root privilege broker/live system/pacman policy introduced by existing roots. |
| safe_stop 1: Layout and compatibility values are tested as data; no deployment or rootfs operation runs. | partial | Pure injected root data exists; missing prefix/target compatibility tests prevent full closure. |

Owners: `crates/vibe-core/src/layout.rs`, `crates/vibe-core/src/settings.rs`, `crates/vibe-app/src/settings.rs`, `crates/vibe-app/src/storage/mod.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/model.rs`, `crates/vibe-lifecycle/src/mechanism/vibebin/config.rs`.

## M-14-B.1 — exact clause matrix

| Original clause | Assessment | Observed boundary / missing meaning |
|---|---|---|
| steps 1: First create/update the governing permanent anchors in vibevm/vibespecs/common/PROP-076-environment-model.xml for this task's contract, with desired-versus-implemented status explicit. Only then add executable types or tests citing those anchors. Final promotion/retirement records evidence later; it is not permission to postpone the source contract. | missing | No accepted PROP076 environment realization/payload contract. |
| steps 2: Use the existing format registry/codegen conventions and M-16-B envelopes; inventory every request, result, durable receipt and hash carrier that needs the new fields. | partial | Canonical codegen and artifact/deploy schemas exist; environment request/result/receipt/hash-carrier inventory and M16 convention join absent. |
| steps 3: Define dependency roles, native-backend closure, file/link/mode/principal inventory and domain-separated ordered hash frames; distinguish recipe-input and produced-output identities. | partial | Measured artifact digest and separate producer/freshness exist; no recursive environment closure/file-link-mode-principal inventory/hash recipe. |
| steps 4: Generate owned wire types/corpora; add wrong-target, unsupported-metadata and reordered-input cases with explicit recipe/schema epochs. | missing | No environment/realization schema/corpus/target ABI metadata fixtures. |
| positive_cases 1: Equivalent normalized definitions yield the same exact lock identity. | missing | No normalized environment definition/exact lock identity. |
| positive_cases 2: Changing target ABI or one payload byte changes the appropriate identity. | partial | Artifact bytes have actual digest; target ABI has no environment identity binding. |
| negative_cases 1: A package URL or mutable version string substitutes for captured identity. | partial | Exact package content hashes exist; no realization claim type preventing URL/version substitution. |
| negative_cases 2: Windows IDs are serialized as Linux payload UID/GID. | missing | No explicit payload UID/GID versus Windows principal representation. |
| acceptance 1: Generated types and retained records use one schema source and explicit epochs. | missing | Existing formats are generated/epoched, but requested environment carriers are absent. |
| acceptance 2: Metadata capabilities are explicit; unsupported privileged nodes cannot be silently dropped. | missing | No target metadata capability/privileged-node representation and refusal. |
| safe_stop 1: Schemas and fixtures pass while unimplemented target execution remains explicitly unsupported. | missing | Environment schema/fixture boundary absent. |

Owners: `crates/vibe-lifecycle/src/mechanism/record.rs`, `crates/vibe-install/src/record.rs`, `schemas/deploy_intent.jtd.json`, `schemas/deploy_receipt.jtd.json`, `schemas/native/e1/deploy_request.jtd.json`.

## M-14-B.2 — exact clause matrix

| Original clause | Assessment | Observed boundary / missing meaning |
|---|---|---|
| steps 1: Create representative old records from the characterized schema and state which fields cannot be inferred; mark them legacy rather than synthesizing target or hermetic proof. | partial | Legacy app/work and binary/artifact reader semantics remain; no explicit legacy environment classification inventory or missing-proof representation. |
| steps 2: Implement deterministic readers/migration through M-13 primitives, retaining input/output/prefix identities and exact source/mirror/substitution policy. | partial | Exact source hashes, current mirror resolver and F14 compatibility retain real old meaning; no environment identity/prefix migration reader. |
| steps 3: Require complete recursive source/build/runtime inputs for new realization claims and keep provider-native versions opaque to Vibe SemVer. | missing | No complete recursive source/build/runtime closure requirement or provider-native version domain in environment realization claims. |
| positive_cases 1: Known old records remain readable with their original weaker guarantees. | partial | Known app/record generations preserve weaker historical meaning; no new environment record adapter. |
| positive_cases 2: A same-content mirror can satisfy an exact captured source with retained provenance. | partial | Existing expected-hash mirror selection and caller lock comparison can reproduce content; no target-prefix/producer substitution admission. |
| negative_cases 1: Legacy existence-only binary becomes verified merely through deserialization. | partial | No conversion from legacy existence-only record into new environment proof currently exists; explicit new-reader refusal matrix absent. |
| negative_cases 2: A changed prefix or untrusted cache producer is silently accepted. | missing | No target-prefix/trusted cache producer comparison owner. |
| acceptance 1: Every migration has explicit unavailable/needs-input outcomes for absent proof. | missing | No environment migration NeedsInput/unavailable classifications for absent proof. |
| acceptance 2: No source/registry/cache transport requires GitHub by contract. | covered | Local/Multi/Embedded and mirror owners are not GitHub-only. |
| safe_stop 1: Legacy/new readers agree with fixtures; no real store or installed environment is migrated during tests. | partial | Existing compatibility fixtures are evidence for their original owners, not full legacy/new environment reader agreement. |

Owners: `crates/vibe-package-source/src/source.rs`, `crates/vibe-install/src/record.rs`, `crates/vibe-lifecycle/src/mechanism/record.rs`, `crates/vibe-app/src/operations/pending/identity.rs`, `campaigns/next-semantic-transfer-2026-11/features/F14-full-application-portability.md`, `campaigns/next-semantic-transfer-2026-11/features/F15-package-admission.md`.

## M-14-C.1 — exact clause matrix

| Original clause | Assessment | Observed boundary / missing meaning |
|---|---|---|
| steps 1: Trace existing deploy/native and extension request carriers, add an optional typed binding without changing legacy local operation meaning or serialized None bytes. | missing | DeployExecution/schema fields have roots and legacy identities, no opt-in environment binding carrier. |
| steps 2: Use host-issued context/target binding values from M-16 conventions; keep a distinct bound state key, include binding in bound plan hash, and validate intent, receipt, native request and structured inspection equality before replay. | missing | DeploymentHome hashes project/package/target only; no bound instance key/plan hash/carrier equality. |
| steps 3: Refuse bound apply before inverse recovery, provider selection or state creation when no pinned host root/effect grant exists; never derive one from a request body or package descriptor. C.2 supplies the first supported bound effect. | missing | No bound operation admission state; existing strict config refuses authored roots but is not a bound grant path. |
| positive_cases 1: The same package under two injected instance bindings gets distinct state keys and typed receipt/inspection identities without touching either destination. | missing | No two injected instance bindings and bound inspection/receipt state keys. |
| positive_cases 2: A legacy local compatibility request keeps the incumbent projected files and byte-compatible unbound records. | partial | Legacy operation records remain as before; no bound-versus-None compatibility extension tested. |
| negative_cases 1: A request body path or package descriptor self-grants a new root. | partial | Engine-owned settings/user roots and strict provider config already prevent manifest root override; target binding body admission absent. |
| negative_cases 2: A copied legacy or other-instance receipt is accepted under a bound key, or unsupported bound apply reaches recovery/provider. | missing | No bound key/instance receipt comparison or bound-before-provider refusal. |
| acceptance 1: Target binding survives the typed request/plan/journal/receipt/inspection carrier and legacy None remains byte-compatible. | missing | Binding absent from request/plan/journal/receipt/inspection. |
| acceptance 2: Bound effects without admitted physical roots refuse before recovery/provider; no successful native or builtin effect is claimed by C.1. | missing | No bound effects capability/refusal path; current effects are legacy local. |
| safe_stop 1: Bound carrier and early-refusal tests pass; first real bound effect and physical alias fencing remain C.2. | missing | Bound carrier gate absent; source C1 refers to C2 first effect, superseded by exact C2/C3 split. |

Owners: `crates/vibe-lifecycle/src/mechanism/deploy/model.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/state.rs`, `schemas/deploy_intent.jtd.json`, `schemas/deploy_receipt.jtd.json`, `schemas/native/e1/deploy_request.jtd.json`.

## M-14-C.2 — exact clause matrix

| Original clause | Assessment | Observed boundary / missing meaning |
|---|---|---|
| steps 1: Extend the existing physical resource identity/lease boundary to recognize equivalent roots across environment instances. | partial | App home OS lock aliases share lock file, deploy locks canonicalize logical resource spelling; no cross-environment pinned physical-root lock identity. |
| steps 2: Revalidate root handle, principal, generation and resource ownership at admission and commit; keep logical config-member ownership under a shared physical file lock. | partial | Prior/foreign receipt and generation/plan checks plus shared-document lock sets exist; root handle/principal/target generation commit checks absent. |
| steps 3: Add temporary-root concurrency/replay tests covering alias roots, relocated directories and third-state changes; record exact partial truth if drift occurs after a landed write. | partial | Existing temp-root/persistence tests cover their original scopes; no joined alias-retarget/stale-target-generation/disjoint-target evidence. |
| steps 4: Prepare the pinned builtin provider and optional bound sidecar/checkpoint/inverse carriers; refuse actual bound publication until C.3 threads a host grant through the incumbent transaction. | missing | No admitted pinned source/settings/bin/store bundle or bound sidecar carriers. |
| positive_cases 1: Disjoint injected homes can proceed independently. | partial | App SettingsRoot supports independent home locks; no environment target-instance admission proof. |
| positive_cases 2: Two aliases of one root conflict even when their logical instance IDs differ. | partial | Same home alias ordinarily locks same inode; different logical binding/retarget scenarios not established. |
| negative_cases 1: Expired generation or changed root reuses an earlier plan. | missing | Environment generation/root authority is not a plan input to revalidate. |
| negative_cases 2: Equal file bytes transfer ownership between receipt families. | partial | Deploy prior/foreign ownership and exact whole receipt comparisons exist; no environment receipt-family proof. |
| negative_cases 3: An unsupported or mismatched bound operation reaches a provider effect before C.3. | missing | No bound pre-effect gate tied to physical target capabilities. |
| acceptance 1: Equivalent physical roots conflict under the cooperating-writer destination lock while disjoint roots can be admitted independently. | partial | Current lock domain covers same state/home and normalized resources, not cross-instance root equivalence. |
| acceptance 2: Disposable-root tests verify alias retarget and stale-generation refusal without touching the developer HOME. | missing | No disposable bound alias-retarget and stale environment-generation tests. |
| acceptance 3: Pinned source/settings/bin/store capabilities and bound sidecar carriers are ready; no successful bound effect is claimed by C.2. | missing | No pinned target capability bundle or bound durable sidecar. |
| safe_stop 1: Physical fence, pinned capabilities and durable bound carrier tests pass; first actual bound builtin effect remains C.3. | missing | Physical bound-target fence not delivered. |

Owners: `crates/vibe-app/src/settings.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/protocol.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/state.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/sidecar.rs`, `crates/vibe-lifecycle/src/mechanism/vibebin/store.rs`.

## M-14-C.3 — exact clause matrix

| Original clause | Assessment | Observed boundary / missing meaning |
|---|---|---|
| steps 1: Require trusted host admission before inverse recovery, provider selection and state creation; bind the request, plan, physical resources and generation to one live destination lock. | missing | No host-issued environment effect grant tying request/plan/physical roots/generation. |
| steps 2: Use the pinned project artifact, settings, bin and store handles in the incumbent provider apply/verify path; publish payload, launcher and pointer with existing checkpoints and retain the lock through receipt/commit. | partial | Incumbent provider publishes measured payload/launcher/pointer with checkpoints; roots are paths/per-call opens, not one retained admitted target bundle. |
| steps 3: Carry the exact operation binding through intent, pending/committed lock sidecar, checkpoints, inverse, receipt and inspection; legacy None bytes remain unchanged. | missing | No environment operation binding in intent/sidecar/checkpoint/inverse/receipt/inspection. |
| steps 4: Limit first support to local UserProfile builtin deploy:vibe-bin with project-root artifact evidence and no prior bound generation. Refuse native/plugin/skill, slot/store-relative artifacts and unsupported recovery/remove until equivalent admission exists. | missing | No explicit supported local UserProfile bound branch or refusal set for other bound providers. |
| steps 5: Report a landed write without final receipt as partial/uncertain with retained intent; never reconstruct bound sidecars from legacy fallback. | partial | Existing intent/checkpoint recovery has original effect truth; no bound partial-write/carrier fallback law. |
| positive_cases 1: The same package bytes deployed into two disjoint injected homes create distinct physical outputs and bound receipts under their admitted principals. | missing | No two disposable homes with host-admitted principal-bound receipts. |
| positive_cases 2: Two aliases of one physical root cannot simultaneously publish through distinct logical bindings. | missing | No distinct logical environment bindings fenced by same pinned physical-root authority. |
| positive_cases 3: A legacy unbound operation retains its existing byte-compatible records and behavior. | partial | Legacy unbound behavior persists; bound introduction parity not tested. |
| negative_cases 1: Changed request body root, principal, target generation or provider scope reaches a bound effect after grant issuance. | missing | No issued grant tuple to compare request root/principal/generation/provider drift. |
| negative_cases 2: A copied bound sidecar, checkpoint, inverse or receipt from another instance is replayed. | missing | No bound durable record instance comparison. |
| negative_cases 3: An interrupted bound write is reported as no-effects or reconstructed through a legacy fallback. | missing | No interrupted bound write path or explicit no-fallback proof. |
| negative_cases 4: Unsupported native/plugin/skill or unadmitted artifact source reaches a bound provider effect. | missing | No bound unsupported-provider/artifact-source admission gate. |
| acceptance 1: One real bound builtin apply uses pinned roots and the incumbent transaction, produces physical output and a bound receipt in each of two disposable homes. | missing | No first actual host-admitted bound apply. |
| acceptance 2: Same-root aliases conflict under the physical fence; mismatched/stale grants refuse before effect. | missing | No joined alias/grant freshness fence. |
| acceptance 3: After possible publication without receipt, retained intent and partial/uncertain truth are observable; no unsupported bound provider is enabled. | missing | No bound possible-publication/retained-intent truth proof. |
| safe_stop 1: First supported bound builtin effect and disposable-root replay checks pass; broader recovery and provider families remain explicitly gated. | missing | First bound effect not delivered. |

Owners: `crates/vibe-lifecycle/src/mechanism/vibebin.rs`, `crates/vibe-lifecycle/src/mechanism/vibebin/launcher.rs`, `crates/vibe-lifecycle/src/mechanism/vibebin/store.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/model.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/sidecar.rs`, `crates/vibe-lifecycle/src/mechanism/deploy/state.rs`.


## Root disposition

Accepted as source-audit classification after independently verifying49 pinned source hashes and7 complete original contracts. Root also checked actual for_host environment and NATIVE launcher call sites. No runtime tests or target qualification performed, no full task promoted. Existing root/layout/legacy/ownership subparts are partial only; owner meta-distribution vision remains the basis for later target/executor design, not Docker completion.
