# First real common application configuration journey

Read-only design input at main `926a30f8e`, 2026-11-04. Named user/configuration/structured-installation contracts and existing app/index/CLI/generator sources were inspected. No builds, product/spec/state/Git edits or new agents. Recommendations are for root's decision, not implementation claims.

## Recommended vertical boundary

Deliver one complete selected-installer path: describe → explicit/attended answers → shared evaluation → existing app plan/runner → installation record → fresh-process modify/update. Reuse the incumbent installation home, mutation lock, index transaction, rollback, requirement installer and callbacks. Do not require a generic package solver to execute this independent app operation, but do not claim the app vertical completes package dependency configuration.

### Package acquisition is not a second implicit configuration instance

Observed: `resolve::from_local_registry` resolves/fetches one directory and parses its Manifest; `from_global` calls resolve_and_fetch for one package. Neither creates a consumer workspace/lock nor evaluates package FeaturesTable/requires. App runtime dependencies are separately declared by option/step `RequirementDecl`, resolved by `plan_option`'s requirement callback and installed by `operations/deps::install_required`. Package `[requires]` is used when that package participates in the ordinary workspace resolution/build world; the global app installer historically does not execute that world.

Therefore select **installer scope** for `install -g`, app plan, modify and update. Top-level common package configuration and package requires input are retained source data for package-scoped description/evaluation and workspace operations, not automatically merged into the installation. No client-created instance label and no automatic second configured package installation.

Replace the temporary package/requires guards in `from_manifest` only when ResolvedPackage retains the relevant package definition/requires data instead of dropping it, and the new description explicitly identifies scope and supported operations. A small retained package-configuration source value is sufficient; do not duplicate the selected installer definition into another independently mutable graph. Installer requirements and package requires must not be reinterpreted as interchangeable. Existing package install guards remain until that domain is connected. Package-level effects are not executed by merely acquiring source for an installer; surface that fact in the common description and format log.

## Concrete request and planning connection

Current `InstallRequest` and `ModifyRequest` hold FeatureRequest and raw parameter assignments; `Previous` holds only effective features/public parameters. Add an optional common input path while retaining the legacy path for absent common definitions. Reuse ConfigurationIntentDecl for its policy/profile/features/default_features/selections/public typed parameters. App-specific hard deselections still need an explicit `disabled_features` vector because the existing source intent deliberately has no disable field. Keep ephemeral raw parameter assignments in the operation request, separate from durable common input; target declarations classify which are secret.

Suggested domain shapes, names illustrative:

- `AppConfigurationInput { intent: ConfigurationIntentDecl, disabled_features: Vec<String> }` — durable, secret-free original intent, scoped by actual installation.option rather than an authored instance ID.
- `ConfigurationSubmission { input, expected_definition: Option<DefinitionFingerprint>, reviewed_defaults: ... }` — ephemeral split-view validation/review data, not a second database. The host verifies the supplied fingerprint against its actual definition.
- `Plan.configuration: Option<PlannedAppConfiguration>` — original input, public review basis, selected/dormant/effective choice results and typed config-value DATA. Effective `Plan.features` and `Plan.parameters` remain what existing steps consume.
- `Previous.configuration: Option<&RecordedAppConfiguration>` — rich state when genuinely recorded; None means legacy effective-only, not reconstructed intent.

The exact complete request vs patch distinction must be explicit. A supplied complete common input replaces the previous common input. With no such submission on update/modify, start from the recorded original input. Existing CLI feature/profile/parameter flags are explicit per-consumer deltas over that working copy, then materialize the resulting input before evaluation/persistence. Group entries overwrite by key, including Omitted and explicit []; missing entries in a delta retain existing answers. A changed profile does not silently erase explicit group/parameter overrides; a complete replacement input can intentionally remove them. Console editing also starts from this working copy. Do not use `Default::default()` to turn absent maintenance input into a request to reset defaults.

Pipeline in `steps::plan`:

1. Select actual option as today; branch only on legacy vs common declaration, not package kind.
2. Assemble/validate original input and review freshness. Do not call legacy `declared_only`/`keep_recorded` on rich answers: they would delete dormant/removed intent or infer original requests from effective state.
3. `ConfigurationDecl::normalize_input` produces FeatureInput/ConsumerChoices/public presets. Apply app hard deselections in this adapter with public visibility checks. Invoke the existing ChoiceModel; it alone owns group guards/merge/closure. Unresolved outcomes return questions/conflicts/reconfiguration data with no runner plan.
4. The attended adapter renders controls using existing chooser/multi-select callbacks. Radio groups use chooser; multi groups use feature_chooser. Reevaluate after each accepted answer so eligibility/questions update; cancellation remains refusal. An ineligible answer remains dormant. A default profile may be preselected in UI but is not silently executed. A hidden/default-only answer must not be invented by assume-yes.
5. Resolve typed values with the existing canonical validator and host facts. Refactor `resolve_parameters`, `param_holds` and `param_equals` to take the effective declaration slice/accessor, instead of reading only option.parameters. Legacy passes its existing rows; common passes configuration.parameters. Do not clone a fake legacy option or maintain a second parameter validator.
6. Fill effective Plan.features/parameters and continue current condition/need/dependency/prompt preparation/runner. Keep common original/evaluation separately for inspection and recording.

Existing callbacks can render finite controls, but returning a complete structured question result to an external caller needs a library `evaluate_configuration` seam before the successful Plan. Do not encode questions as an untyped error string or depend on `on_plan`, which modify deliberately suppresses during initial planning.

## Persistence that actually preserves intent

`Installation` currently stores option/platform/arch, effective features and nonsecret canonical parameter strings. It stores no profile ID, source policy, exact/additive/omitted answers, dormant choices or reviewed-default basis. `operations/apply.rs` constructs this record only after the runner succeeds; `Index::save` atomically replaces applications/index.json and publishes the existing catalog event. Add optional `configuration: RecordedAppConfiguration` there, populated from the exact successful plan. Keep old effective fields for existing listing/steps consumers; they are result projections, never the source of rich reevaluation.

A useful minimal record contains:

- original AppConfigurationInput, with public parameter answers only;
- the selected installer identity already implied by record.option, validated rather than duplicated as a user-created scope;
- semantic review basis for defaults/profile inputs actually used, with usage records distinguishing choice defaults, feature-default seeds, parameter defaults and profile-derived inputs;
- public effective configuration DATA needed for inspection/diff, separate from original intent;
- value-free presence/resupply metadata for secret parameters where continuation needs it, never values or reusable hashes of answers.

The basis must be defined, not merely called a digest. Use a deterministic semantic projection of the consumed configuration definition: stable feature relations/visibility/effects and referenced config values, group membership/cardinality/guards/defaults, parameter type/bounds/allowed-values/defaults, and the named profile's inputs. Exclude public display labels/descriptions and irrelevant presentation order; sort set-valued material. Keep a versioned recipe or typed structured snapshot, not opaque JSON stuffed into an old string. The receipt binds current selected definition semantics; it does not authenticate the entire source tree. Local/global source acquisition currently supplies no uniform immutable whole-tree witness in ResolvedPackage.

For the first implementation, a whole semantic configuration snapshot is a conservative basis; it may require review on unrelated semantic additions. If root wants review limited strictly to affected defaults/profiles, project their dependency closure explicitly and test it. Do not quietly omit effects/guards from the basis merely to reduce invalidations. Stable explicit answers with no reliance on changed defaults/profile expansion should validate against the new definition and survive a default change. A retained UseDeclaredDefaults policy is not permission to renew a changed receipt automatically on update. Changed/removed reviewed defaults or changed named profile meaning return rediscovery before requirements/uninstall/steps; attended UI can show and accept the new basis, external tools resubmit against its fingerprint.

First-time unattended default use also needs an explicit review rule: prefer a describe/evaluate fingerprint returned to the caller and an explicit accepted-default submission, rather than pretending a persisted policy string proves a shown default. Attended callback acceptance can produce the same host-owned receipt directly. This is two ordinary local operations, not a manually bootstrapped choice-host service.

### Index compatibility and generator ownership

Current index is `vibe-user-applications/3`, generated from `schemas/application/e2/index.jtd.json`, registry entry `formats/REGISTRY.toml [format.application-index]` epoch2, and corpus `formats/corpora/application/e2`. Generated source is `crates/vibe-wire/src/generated/application/e2/index`; domain conversion is `vibe-app/src/index/wire.rs`. Unknown fields are denied; load deserializes one schema and rejects every other protocol today.

New rich durable data requires an explicit next schema/protocol (e.g. e3/protocol4, subject to root's version decision). Retain the v3 decoder: inspect protocol then dispatch to the matching strict generated schema. A v3 record lowers as LegacyEffective and leaves configuration=None. Never infer Exact/default consent/profile/dormant input from recorded effective sets. Keep v3 writing for unchanged legacy-only state where representable; upgrade when the first rich record is committed, and retain that newer state version afterward unless an explicit lossless downgrade is designed. Do not migrate on mere read, write a sidecar, or rewrite records before the actual operation succeeds.

`Index::to_wire` currently returns the old generated type. It cannot remain an infallible projection that silently drops rich records. Use versioned serialization inside save plus an explicit/fallible legacy projection, or a closed domain output enum of the two generated document types. Record the Rust API change. Old clients encountering the new index must refuse its version rather than lose data on save.

Author schema/registry/corpus/epoch notes first, then run `cargo xtask codegen`, affected wire/admission tests, `cargo xtask check-codegen` and `cargo xtask wire-diff`. The source loader/generator and strictness/format ownership live in `xtask/src/codegen/{mod,layout,format_id,...}.rs`. Current formats/EPOCHS.toml is public=false, break_window_open=true; do not flip owner flags. Existing app index is nonrecoverable state, so preserving v3 read compatibility is warranted even though the general pre-publication policy allows breaks.

## Maintenance behavior

- **Update:** use prior rich original input plus explicit request delta. Reevaluate against the selected new option definition. Removed explicit/dormant members are reconfiguration diagnostics; new mandatory groups produce questions. Do not pass prior effective public parameters as authoritative input, because that would silently freeze old defaults and lose their origin. Keep the old installation/config record hidden exactly as update already does; rollback swaps exposure without reinterpreting its choices.
- **Modify:** replace `declared_only`/`keep_recorded` only in the rich branch. Plan/validate/review before `deps::install_required` and before recorded uninstall/undo. Reuse the existing apply and commit owner. Switching installer changes the definition scope; matching IDs alone do not carry answers. Ask for a new input or an explicit validated transfer; guard dormancy within one definition is different from an archive of every installer ever chosen.
- **Legacy record→rich option:** there is no recorded original intent. Use old effective values only as visibly labeled UI suggestions, then obtain real answers/review. Do not manufacture a rich receipt.
- **Repair/install --force:** currently install_entered passes Previous=None even when a prior same-key record exists. A common repair must deliberately recover the prior rich input when no replacement input was supplied; otherwise repair resets choices. Keep legacy behavior separate where compatibility requires it.

## External views and secrets

Keep legacy describe/plan e1 documents unchanged for legacy callers. Rich definitions need a generated common description/evaluation schema exposing groups/cardinalities/eligibility/default policy/original and dormant answers/questions/review basis. A concrete first CLI surface can add an explicit common-format selector to app describe/plan plus a complete configuration-input document, all sharing the library evaluator; old describe continues to refuse rich views rather than lie. New common plan output contains configuration evaluation and the domain execution plan; do not append unknown fields to old e1 or serialize a Rust Result. Existing successful install/report output need not change merely to persist rich input, but interactive or external preflight must expose structured unresolved results.

Run records call `Plan::to_wire` today. Wire selection must be deliberate there too: write the new versioned rich plan to the existing plan.json owner, not an untracked sidecar. Old e1 plan fields can remain a faithful execution projection where representable, but they are not a complete common configuration view and cannot substitute for the rich result. Generated typed scalar transport should preserve i64 exactly (existing canonical string representation is preferable to unqualified JSON float64); never hand-maintain a second serializer in CLI.

Secret values stay in the ephemeral parameter channel; ConfigIntentDecl correctly refuses authored secret values. Classify against the actual selected target, persist only public answers, redact plans/errors/Debug, and preserve the now-landed template-reference barrier and captured prompt behavior. `param_holds/equals` must consult common parameter types; otherwise a common bool false may be treated as nonempty true. Missing optional secrets during resume must not silently become absent/default after previously being supplied; use value-free required-resupply metadata and the existing continuation owner. Raw-argv hosted replay remains separate open work; do not claim the new index alone fixes it or add secret transport to agents.

A concrete runner representation issue must be handled before enabling common execution: Run::new exports every declared feature using `step_env_name`, which uppercases and maps '-' and '.' to '_'. Common IDs permit arbitrary literal spellings; '=' is invalid in an environment key and distinct IDs can collide. Add a named app export representability/collision check for actual recipe/shell environment consumers, including parameters, before any step. Do not rewrite core IDs or silently overwrite exported values. Other pure/read consumers keep literal IDs unchanged.

## Smallest integrated work order and acceptance

1. Add app input/evaluation/parameter-view adapters, explicit scope retention and review-basis semantics with tests; keep existing runner/condition/requirements owners.
2. Add versioned durable record conversion and v3 compatibility; wire apply/update/modify/repair to original input before removing runtime guards.
3. Add common generated description/evaluation and actual console/serialized submission path, including run-plan recording and stale review responses. Preserve legacy e1 and existing wizard.
4. Demonstrate one ordinary local-registry common app through install, process restart, modify, update, rollback and unchanged repair. Only then call this app vertical delivered.

Required discriminators: exactly-one and multi choice install; typed parameter changes an actual step; dormant addon survives switch-away/back; explicit empty never becomes default; changed/removed default/profile asks before mutations while stable explicit input survives; removed dormant ID reports reconfiguration; v3 legacy records retain effective-only meaning and v4 saves only after success; old e1 callers fail clearly for unrepresentable rich views; secret canaries absent from framework plan/index/review/log output while shell receives authorized environment; bool false controls step correctly; environment collisions/unrepresentable IDs refuse before sentinel execution; selected legacy option still works alongside rich options; same local IDs in package/installer scopes do not merge; package data retained/described without executing package effects during installer acquisition. Include public CLI human/JSON and library callback parity, not only evaluator tests.

These are actionable design requirements, not executed results. The actual package-level configured solver, all-kind installation effects and common campaign remain separate obligations after this selected-installer vertical.
