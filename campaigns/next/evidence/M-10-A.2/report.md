M-10-A.2 unreviewed candidate — no acceptance or next-task dispatch

Scope and ownership
Implemented directly in the PodBay-managed session without child agents, Git
staging/commit/mutation, network/platform probes, deployment, or payload execution.
The patch contains exactly five task files: workspace lib registration, the new
workspace derivatives module and six-case test module, PROP074, and derivative
fixture README. No dependency or lockfile changes. Existing M08 WIP is excluded.
Historical accepted receipts were treated only as source context.

Implementation
PROP074 COMMON-SERVICE was written before adding its source anchor. One owning
DerivativeService exposes derive/show/diff/rebase/apply using immutable private
SourceInput fields, borrowed AuthoredReplacement and DerivedOutput, and a closed
NoExternalEffects outcome. SourceInput reuses M-01 CapturedSourceFile and M-16
SourceDescriptor plus exact qualified PackageRef. It refuses duplicate/aliased,
ancestor-colliding, empty or out-of-selection inventories. Local replacements
require a distinct package identity and matching descriptor. Exact base comparison
uses coordinate, descriptor, path inventory and bytes rather than a fixture hash.

Derive returns the whole selected local inventory (including tree removals).
Show returns its captured source. Diff emits Added/Removed/Replaced paths without
parsing bytes. Rebase requires the expected old basis and explicit authored input
bound to the new basis; it performs no merge. Apply rechecks the current basis and
returns an inert ApplyHandoff. It does not write or activate anything. No operation
accepts a provider, process launcher, grant, filesystem destination or installed
slot writer. Caller-owned durable authored files are an explicit precondition;
output borrows the independent inputs and cannot own the only copy of local edits.

Owner trace
Both fixture shapes are captured with the existing safe-filesystem Project bounded
snapshot API into CapturedSourceFile. Tests mount relative fixture inventories
under payload/ for the SourceDescriptor selector and use typed example pins;
neither these pins nor fixture hashes are verified production capture proofs.
vibe-install::slot_verify::verify_original_source_capture retains independent
selected-hash replay and its private VerifiedSourceMembership. Workspace does not
add an install dependency or attempt proof promotion. Existing replay_declared_captured
owns conversion; SlotRecord distinguishes source hash and derived output footprint.
vibe-spec::resolver::lookup retains canonical lookup and collision refusal.
Compiler EmittedArtifact provenance remains compiler-manager-owned; replacements
are not falsely presented as compiled artifacts. ExtensionWorldEpoch retains the
caller-ordered resolved world. Existing artifact/build/package declarations and
DeployTarget validation remain downstream owners. Projection preparation calls
hydrate_declared_skill and may fetch resources/publish a composed cache; this
service never invokes it. Projection receipt reconciliation remains untouched.

Bound selections made before compiling
vibe-workspace --lib m_10_a_2_cases: exactly six new cases, all under the permitted
workspace source module. Selected text/tree lifecycle, authored retention after
cache deletion, stale basis/identity refusal, passive no-effect inspection, and
invalid inventory refusal. The text case checks full CommonMark bytes against the
expected fixture; the tree case checks exact bytes/inventory and four changes,
including owned removal/addition. Both exercise derive/show/diff/rebase/apply.
The separate projection selection was the exact existing owned-update/uninstall
neighbor test; no projection registration, fixture or code edits were made.

Actual commands, exits and counts
All cargo tests used CARGO_TARGET_DIR=/tmp/vibevm-managed-m06b1/cargo-target.
Before compiling, fuser on debug/.cargo-lock returned exit 1 with no holders.
The transferred target was used in place, with sequential cargo invocations.

1. cargo test --offline --locked -p vibe-workspace --lib m_10_a_2_cases
   First attempt: exit 101, 0 passed / 6 failed / 693 filtered out. The test loader
   used the wrong fixture manifest key, families instead of cases. Corrected.
   Log: workspace-test-initial.log.
2. Same command, second attempt: exit 101, 0 passed / 6 failed / 693 filtered out.
   Safe Project opening rejected dot components in the test root. Changed test
   root construction to parent()/parent()/join, without relaxing safe filesystem
   validation. Log: workspace-test-path-failure.log.
3. cargo test --offline --locked -p vibe-agent-projection --lib
   pkgskill::tests::owned_update_and_uninstall_preserve_unowned_neighbors -- --exact
   Exit 0: 1 passed, 0 failed, 80 filtered out. Log: projection-test.log.
   No M08 semantic interference observed in this exact test. This does not assert
   clean HEAD or validate M08's uncommitted registrations as accepted work.
4. cargo test --offline --locked -p vibe-workspace --lib m_10_a_2_cases
   Final attempt: exit 0, 6 passed, 0 failed, 693 filtered out.
   Log: workspace-test.log. No source changes after this successful run other than
   fixture README and PROP074 wording.
5. cargo fmt --all -- --check
   Exit 1: pre-existing/out-of-scope formatting differences; no changed workspace
   source appears in the output. Nothing outside scope repaired. Log: fmt.log.
6. rustfmt --edition 2024 --check crates/vibe-workspace/src/derivatives.rs
   crates/vibe-workspace/src/derivatives/m_10_a_2_cases.rs
   Exit 0. The same files were formatted with rustfmt before the final test run.
7. git diff --check
   Exit 0, no output. Read-only check, no staging.
8. python3 fixtures/next/derivatives/validate.py
   Exit 0: 5 families, 27 payload files, 6 payload XML documents, 1 canonical XML
   document, 1 manifest JSON document, 13 unique anchors. Its service-case count
   is zero because it is the design validator; the Rust family independently ran
   six cases. Log: fixture-validation.log.

Conditional checks
No composition/adoption/extraction/closure rules, facts, boot or dependency
registration changed. PROP074 adds a design section without fact declarations.
Focused XML/fixture validation and bounded Rust checks apply; full project check,
facts exhaustive and full test suites were not run.

Limitations and review boundary
This establishes a common passive Rust service/harness, not all derivative
features. Caller preparation must retain durable authored storage; this service
is not a durable authoring repository and does not prove an arbitrary caller's
storage choice. Cache-retention tests delete a real disposable temporary tree,
drop output, recapture durable fixture sources and regenerate from retained input.
Production proof promotion, persistence/publication, selected-export editing,
XML/Vibe semantic replacement and downstream consumer wiring remain unimplemented.
The five-family matrix and all later obligations remain intact. The projection
test proves its existing ownership rule, not integration of ApplyHandoff with a
live projection. Passive no-effect claims rely on the API having no effect ports,
plus exact payload comparisons and filesystem sentinels in the bounded harness;
no live process/network/activation instrumentation was attempted.
Both required criteria are proposed satisfied for this common-service scope:
explicit operation/effect boundaries, and unchanged acyclic owner dependencies.
This is a candidate for independent Root review, never self-acceptance.

Global format differences (reported only)
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs:571:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:10:
Diff in /fast/git/v/vibevm-next/crates/vibe-core/src/manifest/change/m_02_b_1_cases.rs:188:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:3:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:83:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:92:
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:234:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs:100:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs:2:
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission.rs:48:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:34:
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:184:
Diff in /fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs:289:

Final bounded test output
     Running unittests src/lib.rs (/tmp/vibevm-managed-m06b1/cargo-target/debug/build/vibe-workspace/a8265efdd2c697f3/out/vibe_workspace-a8265efdd2c697f3)

running 6 tests
test derivatives::m_10_a_2_cases::invalid_inventory_cannot_become_output ... ok
test derivatives::m_10_a_2_cases::text_uses_common_lifecycle ... ok
test derivatives::m_10_a_2_cases::passive_inspection_has_no_provider_process_grant_or_slot_effect ... ok
test derivatives::m_10_a_2_cases::stale_basis_and_upstream_identity_are_refused ... ok
test derivatives::m_10_a_2_cases::opaque_tree_uses_common_lifecycle ... ok
test derivatives::m_10_a_2_cases::authored_provenance_survives_output_and_cache_deletion ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 693 filtered out; finished in 0.02s


Exact projection output
     Running unittests src/lib.rs (/tmp/vibevm-managed-m06b1/cargo-target/debug/build/vibe-agent-projection/ad04ecdd82433f5a/out/vibe_agent_projection-ad04ecdd82433f5a)

running 1 test
test pkgskill::tests::owned_update_and_uninstall_preserve_unowned_neighbors ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 80 filtered out; finished in 0.00s
