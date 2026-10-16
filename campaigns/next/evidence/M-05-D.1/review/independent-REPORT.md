# Full D1 independent source/evidence review

**Recommendation: reviewed source is suitable for Root's checkpoint commit; do not mark WorkAccepted until the required postcommit check-codegen gate passes or a concrete generation defect is corrected. No blocking semantic defect identified in this bounded review.** I did not run tests/builds/oracles, mutate source/Git, or control the live producer.

Reviewed the full D1 task, Root replay ruling, report/findings, source patch/semantic map and final receipts, including the core envelope, generated requested/resolved formats, wire behavior, production plan/apply/record adapters and focused tests. REPORT.source_files all match current bytes. Direct implementation findings below distinguish source readiness from the still-failed generation gate.

## Requested and resolved meaning

Core `LockedConfiguration` holds separate requested JSON, request digest and resolved JSON, with an independent configuration epoch1. It checks required payload/digest and refuses unknown configuration epoch. `Lockfile::read` distinguishes configured unsupported schema from intentionally regenerable legacy locks. This does not equate registry lockfile format epoch1 with core schema version7, and adds no core→wire dependency.

The production wire adapter decodes the generated strict formats, checks canonical serialization/order and matching request digests, validates embedded B2 request/result values, and refuses resolved configuration without authored configuration. The normalized definition projection retains dependency IDs/source/version/metadata/optionality, aliases, all B1 effects, subskill/config declarations, B2 group cardinality/members/guards/defaults, member guards and domain/target/instance identities. It is compared to current admitted Rust inputs; it is not decoded into a new authority from derived selections.

`configuration_record` recomputes selected B2 outcomes from the original relevant consumer edges and checks the fetched complete FeatureExpansion. It preserves dormant authored member intent separately from the active selection/closure, and checks exact shown-default receipts using current and shown default-governed intent. C3 provenance supplies declaration/alternative/provider and both C1 identities; each persisted edge binds the shared authored request digest. This is more than string presence: the actual test expects active `a,closure`, authored `a`, dormant `later`, and real dependency/capability witnesses.

## Actual persistence and replay

`plan.rs` validates generated payloads before legacy freshness/migration handling. A configured lock requires current authoritative source-owned normalized input; missing configuration refuses rather than stripping it. `validate_replay` compares exact request/default/definition evidence for bare replay and checks retained source identities against the current catalog. After solving, `validate_replayed_result` compares the newly constructed complete record with the retained record for bare exact reuse, so edited derived effects/providers cannot become replay authority.

Root's explicit-root ruling is implemented as a bounded fresh-version distinction: current configuration/feature intent must remain equal, unnamed configured definitions remain retained, and selected defaults still require legitimate exact receipts. Named CLI roots are not permission to drop configuration or invent defaults. `InstallRequest.exact` is untouched as manifest version pinning.

`apply.rs` constructs/validates the record before effects and checks replay again against the confirmed plan. It writes the final record with the actual final fetched state through the incumbent lock writer. The core envelope stays opaque to avoid the dependency cycle; actual install readers/writers use the wire adapter. Full offline CLI reinstall/update and broader freshness behavior remain E2, as explicitly ruled; no new CLI flag or derived-only fallback is introduced.

PROP003 additions preserve earlier laws and remain candid spec/plan claims pending Root judgment. The new default receipt accessor exposes the existing immutable receipt; it does not mint permission.

## Receipts and generated gate

Final V1/V2/V3 plus two V4 compatibility tests report **3/3/2/2 = 10 passing cases**. I matched referenced log hashes. Final V1 source bindings match current source. V2–V4 snapshots differ only in later install apply/record/test files; their directly consumed core/wire/compatibility owner bytes match, so those are not identical-full-source final runs. V5 scoped formatting and V6 diff checks pass; V7 is structural, not semantic proof. Earlier E0433/schema-policy/fixture failures are retained and not promoted to passes.

The check-codegen exit1 is real. I read `xtask/src/codegen/mod.rs:437`: it regenerates, then runs `git diff --exit-code` over generated paths. The retained failure log reaches that comparison and includes intended registration/schema-home changes. This explains why an uncommitted candidate can fail, but does **not** prove the post-format candidate will regenerate byte-identically after commit. `write_generated` performs emission/replacement, not rustfmt normalization. Keep the planned postcommit gate: if formatting/emission differs then, the source candidate needs a bounded managed correction; do not waive it as “expected.”

Sixty changed sibling generated module files were independently compared to HEAD with comment lines removed: all bodies are identical. The schema-home comment churn is therefore harmless registration output. New lock_configuration bodies and format-id additions are substantive generated changes, included in the candidate. `wire-diff` exit0 is limited by its tracked-HEAD view: direct new-family test decoding/roundtrip provides the actual untracked corpus coverage. Neither result replaces check-codegen cleanliness.

## Decision boundary

Checkpoint commit is reasonable for the reviewed implementation and required Git-baseline oracle. Work acceptance remains conditional on the actual postcommit generation result and Root integration judgment. No additional full suite, new human default-consent flow, global freshness rewrite, format redesign or E2 implementation is requested. Root retains commit, acceptance/count and producer stop ownership.
