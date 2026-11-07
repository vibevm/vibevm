# Root disposition — project install event

Implementation authorized after this read-only proposal. Corrected epoch decision: current `cli-install-plan` advances to epoch2 at `schemas/install/e2/plan.jtd.json`; the old schema bytes remain unchanged and `cli-install-plan-v1` retains their historical binding. The original proposal below is preserved as the reasoning checkpoint, not the final registry instruction. Root initially conflated current family and frozen epoch IDs and corrected that before implementation.

In-place current alias is captured at preparation and also flows into CachedPackage/lock; old.registry must not remain a conflicting claimed current alias. Inspection of urls.rs confirmed package_urls primary equals package_repo_url composition, so no redundant canonical field is needed. No routing/placement semantics change. Temp-only existing Git fixtures are allowed; repository Git mutations/network are not.

---

# VN project install:plan — bounded design proposal

State: read-only design; no source edits, Cargo/tests, Git mutations or external network. Root implementation/Cargo grant pending. This is the existing real-event consumer of accepted SourceEvidence, not project read-only preview, source policy, candidate reasons or completion of need1.

## Observed current path

* `vibe-install/src/plan.rs` owns `PlannedInstall.fetched` privately. The final `resolution` is built from that same fetched vector, in the same order, after conditional expansion. Every Ready node has a CachedPackage; the planner already acquired/admitted its source.
* `vibe-orchestrator/src/install/ready.rs` narrates Resolution immediately before `confirm_install`, then consumes the same PlannedInstall in apply. The port currently loses facts by exposing only ResolvedDep[].
* CLI `install/report.rs` creates a local Serialize DTO and defers `{command:"install:plan",packages:[{package,version}]}`. It does not print immediately. Completed/failing terminal output flushes deferred plans according to the existing outer context; parked output remains one handoff document.
* Fresh/empty paths do not emit this Resolution event. Preserve that absence; do not fabricate acquisition or add new events.
* `schemas/install_plan.jtd.json` instead requires `plans` with source_url/content_hash/writes. Its metadata's immediate pre-confirmation/dry-run implication is false for the live deferred stream. No `schemas/install_plan_entry.jtd.json` exists.
* `rg` of generated install_plan/InstallPlan usages under crates found only `crates/vibe-wire/src/generated/install_plan/mod.rs`; no production or test consumer currently constructs/decodes that stale generated type. Real CLI tests explicitly pin packages, including a full golden stream.

## Proposed owner/data seam

1. Keep the existing narration variant, but borrow `&PlannedInstall` instead of only its resolution. The observer remains at the same line before confirmation. No additional event, source lookup, settings read or apply-time observation is added.
2. Add one small read-only accessor on PlannedInstall, preferably in `plan/report.rs`: an iterator of borrowed group/name/version plus shared SourceEvidence, derived directly from held Fetched/CachedPackage (or its retained in-place preparation). Use existing coordinate types, e.g. `(&Group, &str, &semver::Version, SourceEvidence)`, rather than a new DTO. No CLI command or generated report envelope belongs in vibe-install or the port. Do not zip independently mutable resolution and fetched arrays. Facts and coordinates come together from each held acquisition; the constructor already builds the initial resolution from that same vector/order.
3. CLI report alone builds the generated epoch2 package rows and InstallPlan root, with literal command, and passes it to the existing `defer_json_plan`. Human/quiet rendering continues to read the same `planned.resolution`; command sequence, counts, confirmation, parking suppression and failure framing stay unchanged.
4. Normal/cached/offline rows use only the acquisition owner's retained fields. Existing unknown source_kind/commit/route stays absent; canonical and differing actual locator remain separate; no current-settings reconstruction. Fresh/empty keep existing event absence.

## In-place is a real extra seam, not a lock fallback

`plan/fetch.rs::try_in_place_incremental` synthesizes CachedPackage from PreparedInPlace. Its URL/ref/commit come from preparation, but registry_name is copied from **old.registry**, and actual origin is lost. Passing that old alias as the newly selected registry would be false.

The inspected PreparedInPlace owner retains: actual plain applied `origin`, requested ref, observed peeled commit, canonical field, immutable manifest, and the actual Git source owner. It privately retains authenticated_origin separately; that must never enter evidence. `MultiRegistryResolver::prepare_in_place` already selects the current named Git registry and refuses override/path/direct-git/redirect/store routes for this preparation API.

Recommended bounded completion: expose a sanitized SourceEvidence projection from PreparedInPlace itself, capturing the current registry alias at its existing constructor (the Git registry owner knows its name), canonical package repository URL, actual plain origin, observed ref/commit, source_kind Registry, acquisition GitRepository, store_reused false. Feed this into the project-event row whenever Fetched.in_place is Some. Do not reuse old.registry as a current observation; do not change resolution or materialisation behavior.

One exact remaining root decision, without extending this audit: Prepared currently initializes its field called canonical from `package_urls().primary`, while ordinary acquisition distinguishes canonical package_repo_url from remapped primary. Authorize a narrow read of that URL-construction owner during implementation and capture the authoritative canonical package URL at the same preparation constructor for the new evidence, using the already-held plain actual origin for fetched_from. Do not change existing lock/placement semantics in this slice. If the current stored canonical is already authoritative, reuse it; do not add another source model. This is the only unresolved in-place detail; selected alias must be captured at the actual current registry owner in either case.

## Explicit epoch verdict proposal

Treat this as a breaking **declared-schema correction**, while keeping actual CLI output compatible:

* Leave `schemas/install_plan.jtd.json` and existing `format.cli-install-plan` epoch1 unchanged as the frozen historical declared schema/ID.
* Add `schemas/install/e2/plan.jtd.json`, generated as `vibe_wire::generated::install::e2::plan`, registered under new `format.cli-install-plan-v2`, epoch2, recoverable=true, foreign_parsers=many, same permissive reader policy.
* Epoch2 root requires exactly the live command/packages members. Package rows require package (`group/name`) and version; optional source_evidence uses the existing shared vocabulary with `x-reader-projection="permissive"`. Missing evidence in old live packages rows remains readable as unknown. Do not emit invented plans/writes/boot_snippet/content_hash members merely to fit the obsolete schema.
* Do not add/rename the actual command or add a runtime protocol field. Existing consumers still receive command/packages/package/version in the same stream positions; only optional source_evidence is added.
* New schema metadata explicitly says capture before confirmation, deferred output according to the current terminal framing, and planning may already acquire/cache source. It is not an interception/dry-run promise.
* Add epoch2 corpora under `formats/corpora/install-plan/e2/`, including old live packages without evidence, normal observed facts, unknown fields and in-place facts. Frozen old schema bytes and old format binding remain unchanged. Root owns final FMT/epoch acceptance.

## Exact proposed write perimeter

Production:

* crates/vibe-install/src/plan.rs + new plan/report.rs (projection; no planner algorithm change)
* crates/vibe-orchestrator/src/ports.rs and install/ready.rs (borrow existing plan at existing narration point)
* crates/vibe-cli/src/commands/install/observer.rs and report.rs (generated report, unchanged defer/human policy)
* crates/vibe-registry/src/git_package_registry/fetch/prepared.rs (current captured in-place report facts; no transport/placement changes)
* new schemas/install/e2/plan.jtd.json; formats/REGISTRY.toml registration; generated wire output; new corpus files

Tests named before edits:

* new crates/vibe-cli/tests/cli_install_source_evidence.rs — actual local-registry/global-sandbox project output, closure membership, current alias, cold/reused source facts and generated decoding
* crates/vibe-cli/tests/cli_lifecycle_golden.rs — preserve entire existing command/order/root golden and add concrete normalized source evidence; never delete the golden assertion
* crates/vibe-cli/tests/cli_hook_lifecycle.rs — generated event decode and failed-apply framing/source capture
* crates/vibe-cli/tests/cli_lifecycle_hosted_install.rs — retain one-document parking invariant (existing tests may suffice unchanged)
* crates/vibe-cli/tests/cli_trace_success.rs and cli_trace_update_slot_failed.rs — retain event-before-root and trace parity (existing tests may suffice unchanged)
* new crates/vibe-wire/tests/project_install_plan_corpus.rs — frozen epoch1 unaffected, epoch2 generated roundtrip, legacy live packages accepted as unknown
* new plan/report.rs unit tests and existing PreparedInPlace fake-backend tests — held-value/unknown and in-place current-vs-old origin assertions. Exact existing prepared-test file will be read/named under root's implementation grant.

Any additional existing InstallNarration test adapter needing a mechanical signature update will be identified to root before editing; no unrelated orchestrator work.

## Meaningful RED and acceptance plan

No tests have been run yet.

1. Before implementation, actual sandbox `vibe install ... --json` event must decode through its registered generated report. Current generated epoch1 decoder fails because the live event has packages, not required plans: a real producer/schema RED, not a fixture syntax failure.
2. Separate raw JSON assertion on the same successful event requires expected source_evidence registry/source_url/source_kind/acquisition and observed store_reused. Current producer lacks it: direct missing-observation RED.
3. After schema2/codegen/producer change, legacy live event packages-only and new output decode with the current generated reader; no source guesses for absent legacy data. Old epoch1 corpus and binding remain unchanged.
4. Use local or fake sources only. Prove conditional/transitive rows retain their acquired facts, and repeated acquisition reports actual reuse. Change surrounding settings after planning in an owner-level test and verify retained plan output does not change or call resolution again.
5. In-place fake owner: current registry differs from old lock alias; captured commit differs from old HEAD; actual origin differs from canonical; authentication-bearing URL is redacted; plan projection neither materialises nor performs another backend/source operation. Existing in-place apply semantics remain intact.
6. Preserve completed golden stream order, failure preview/root order, one-document parking and fresh/empty event absence. JSON timing is established by these existing tests, not by the old schema's comment.
7. Scoped tests/doctests/strict Clippy/Conform, schema regeneration+idempotence and old/new corpus gates after root grants the Cargo lease. Root handles canonical map/status/FMT and acceptance.
