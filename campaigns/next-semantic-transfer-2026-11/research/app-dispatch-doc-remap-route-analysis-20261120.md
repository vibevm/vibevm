# App dispatch semantics and Doc/Remap public-route priorities

Report-only at accepted five-kind evidence `55f2ed50cbdb378b5161d595f90053036b175f22`. No product change or new runtime execution. Source references below were inspected; named historical selectors were located, not rerun for this report. Accepted actual observations remain in manifest `19c06a9e70e69f0d64057456fdaedb821abce55a15e82b7a05192d2a4bb8ade7`.

## 1. App: valid absence is not damage

| Subject | Normative identity | Actual boundary | What is established |
|---|---|---|---|
| `kind=app`, missing `[application]` | Not a usable application | `vibe-app/src/resolve.rs:391`, `from_manifest`, missing identity | Earlier malformed fixture; not valid no-options product evidence |
| Valid `[application]`, zero installer options | Explicitly valid product under PROP-059#declaration | `vibe-app/src/operations/mod.rs:454`, `choose`, `not_globally_installable` | No selected installer exists; must not invent one |
| Valid app with selected supported option | Installation authority belongs to selected app operation | PROP-059#installer-separation; PROP-065#steps | Separate scope from top-level package configuration |

PROP-059#declaration explicitly permits a product with no installer options and specifies global-install refusal. PROP-059#project-scope says an ordinary project install records app dependencies like other packages and materializes them into project-local `vibevm/vibeopt`; argument-free install also materializes app dependencies, consent-gated. It does not specify that an unmaterializable App silently becomes a package-only success. PROP-065#exit-codes distinguishes success, execution failure and refusal before effects; it does not authorize describing a post-publication failure as an effect-free refusal.

### Actual sequence and consequence

`crates/vibe-cli/src/commands/global_mcp_dispatch.rs:74` dispatches ordinary project install to `application_project::install`. `crates/vibe-cli/src/commands/app/project.rs:89` completes `install::run_direct` before `materialise` at93. `install_pending` at271 selects app lock rows absent from the project app index, performs consent, reads the materialized slot via `from_slot` at492, then selects/configures app operations. The shared application operation reaches `choose` and refuses empty options. This is two owned stages, not an atomic rollback spanning both.

Accepted actual valid no-options App off/on worlds: whole command nonzero after ordinary lock/slots/boot/hooks publication; `show configuration-data` succeeds on the retained authored top-level record. Repeat install returns the same application refusal with byte-exact lock and unchanged package snapshot. Hooks do not rerun: base1/extra0 off, base1/extra1 on. No rollback occurred or is implied. Selected-installer configuration was never supplied, no executable/installer ran, and package-level evidence is not whole App journey support.

### One recommended action: preflight refusal (priority P1)

Keep valid no-options App authoring legal. For a project install that promises App materialization, validate usable application identity and the existence of an admissible installer at the existing plan/admission boundary **before ordinary package effects**. Return a safe named refusal, not package-only success and not a fabricated installer. Reuse the actual selected candidate/manifest and incumbent app selection owner; do not add a second solver, fetch fallback or execute a preflight command. Whether to expose a different future package-only operation is a separate contract decision, not this repair.

This recommendation follows the existing project-scope promise while avoiding the observed late failure. A documented partial-result model would be a larger protocol/UX decision; current ordinary success output plus later error is not proof that such a model already exists. No implementation is authorized by this report. A finite implementation contract must choose ownership and ensure public controls for valid selected installers, malformed identity, valid no-options products, consent and unchanged prior state.

Existing selectors for that contract (located, NOT RUN here):

- `cargo test -p vibe-cli --test cli_app_project an_app_dependency_installs_into_the_project_home_and_follows_the_lockfile -- --exact` — project app materialization, separate project/user homes and consent. This historical fixture executes synthetic application behavior; do not run under a no-installer-execution qualification packet.
- `cargo test -p vibe-cli --test cli_app_install ambiguous_unattended_selection_is_an_error_before_any_mutation -- --exact` — selected global operation's ambiguous/no-host/no-options refusals before application-state mutation; not proof of ordinary package preflight.
- `cargo test -p vibe-cli --test cli_app_installers every_option_is_listed_grouped_with_the_detected_host_first -- --exact` — read-only option listing, no selection/execution.
- `cargo test -p vibe-cli --test cli_common_package_optional default_review::ordinary_remaining_kinds_install_show_and_cold_replay_qualification -- --exact` — accepted bounded App partial-outcome evidence, not whole-operation success.
- `cli_app_common_configuration::{global_install_keeps_package_configuration_separate_from_installer,global_install_records_selected_installer_common_configuration}` — existing selected-installer vs package separation; synthetic steps execute, so require a separate appropriate runner/packet.

## 2. Doc: use its existing read projection, not installation

Read-only operation: registered MCP `read_doc`, arguments `address=spec://GROUP/NAME/DOCUMENT`, optional `format=md|xml`, optional `lang`. Owner `crates/vibe-mcp/src/tools/read_doc.rs:70`; library `vibe_doc::agent::read_page`. Sources are the checkout, in-tree documentation, retained slots and explicit store according to that existing owner. No server, browser or document installer is needed. `doc build --path MANUAL --out OUTPUT --format md --no-derived` is an existing output-producing projection, not read-only filesystem behavior; it writes only an explicit output tree and is a separate verification choice.

Exact located selectors:

- `cargo test -p vibe-mcp --lib tools::read_doc::tests::an_address_comes_back_as_a_page_and_its_provenance -- --exact` — valid in-tree Doc page and provenance.
- `cargo test -p vibe-mcp --lib tools::read_doc::tests::html_is_refused_by_name_rather_than_silently_substituted -- --exact` — projection refusal.
- `cargo test -p vibe-mcp --lib tools::read_doc::tests::a_missing_address_is_an_argument_error -- --exact` — argument boundary.
- `cargo test -p vibe-cli --test cli_doc_captured real_doc_build_quotes_declared_authored_source -- --exact` — actual CLI projection over declared authored sources, no-derived.
- `cargo test -p vibe-cli --test cli_doc_captured real_doc_build_refuses_invalid_selected_configuration -- --exact` — current settings/manifest refusal, not proof of package configuration-choice consumption.

Priority P2: qualify actual registered `read_doc` with a valid Doc source and a data-only choice/control, then identify whether selected common configuration is actually consumed. Existing read-page tests prove page/provenance, not runtime propagation of an arbitrary Doc configuration answer. `vibe-core::manifest::configuration::tests::all_nine_kinds_share_data_without_acquiring_effect_authority` is kernel/authoring evidence only, never public-read acceptance.

## 3. Remap: read state and explicit consent, never executable package effects

PROP-066#layers-packages places project remap requirements in `[requires.remaps]` and lock `[[remap]]`, never ordinary `[[package]]`/`vibedeps` closure. PROP-066#package/#consent makes rows an offer, acceptance per user/content hash, no automatic policy application. Runtime authoring validator `crates/vibe-core/src/manifest/document/validation/remap.rs:24` permits data-only configuration but forbids executable contributions. The current spec prose's closed-table list omits this implementation's data-only allowance; record the distinction rather than infer new runtime consent semantics.

Existing public operations: `vibe remap list --offline --json`, `vibe remap explain SUBJECT --offline --json` are read-only and offline disables diagnostics DNS (`commands/remap.rs:1`). Explicit `remap accept|reject PACKAGE` and `enable|disable PACKAGE` change only owned consent state; acceptance does not become permission to execute package code. `remap bind-source` is explicit legacy source admission, not automatic source guessing. `remap test` probes targets and is NOT part of a no-network read qualification.

Exact located selectors:

- `cargo test -p vibe-cli --test cli_remap list_json_carries_the_rows_and_their_computed_diagnostics -- --exact` — actual read view; inspect its fixture networking before a runner is selected.
- `cargo test -p vibe-cli --test cli_remap_packages an_installed_remap_package_is_an_offer_and_applies_nothing -- --exact` — offer state, not consent. Historical fixture starts a loopback server and later fetches, so do not invoke under the present no-service packet.
- `cargo test -p vibe-cli --test cli_remap_packages an_unattended_run_refuses_an_unanswered_offer_and_names_the_verb -- --exact` — explicit consent refusal; historical fixture starts loopback server even when hit count is zero.
- `cargo test -p vibe-cli --test cli_remap_packages disable_and_enable_switch_an_acceptance_without_forgetting_it -- --exact` — consent state transition, inspect fixture first.
- `cargo test -p vibe-cli --test cli_remap_bind_source legacy_missing_source_refuses_before_project_or_home_effects_then_explicit_binding_is_idempotent -- --exact` — source admission and idempotence.
- `cargo test -p vibe-cli --test cli_remap_lock_admission malformed_future_and_old_epoch_rich_locks_refuse_before_accept_all_is_saved -- --exact` — lock refusal before consent persistence.

Priority P2: prepare a finite local synthetic offer with no target acquisition/probe, then exercise list/explain and explicit consent state independently of installation. Determine data-choice consumption separately; a parsed data-only definition or accepted offer is not public configuration runtime evidence.

## Delivery limits

No tests above were newly executed for this source-analysis report. No external service, installer, tool or compiler was started; no new operation/schema/permission or parent status was introduced. Submitted-specific publication fault injection and mandatory USER-PROJECT real-agent adoption remain outstanding. REMAINDER/IDE unchanged.
