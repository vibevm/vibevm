# M-05-E.2 independent semantic/code/spec review

**No actionable P1/P2 found in the bounded E2 candidate reviewed.** Root retains source/basis reconciliation, acceptance and Git authority. This review does not accept E2 in Zap or claim that 98 tasks/campaign work are accepted.

Reviewed current VibeVM HEAD `aff3f408eb86aec8e5f7462693f36e10c13a98d0`, the exact M-05-E.2 task entry, published candidate metadata, patch `5c9de987b4224025c42492f6a03c8b6575998284ce1aa14ef6af29a9caad210c` and authored report `7a71cc672273cb99b0c66ae523d4e96a32f194d78309fa2f8e4c83d45c2feab0`, actual patched files, governing PROP-003 E2 contract and directly called request/record/package-source owners. No full boot, source edits, builds/tests, models, live API/service operations or Git mutations were performed. Only review outputs and expressly authorized private evidence copies were written.

## Artifact and current source binding

Both content digests and declared sizes verify: patch **94,990 bytes**, report **25,477 bytes**. Independently reconstructed every patch hunk against actual HEAD, asserted every context/removed line, and compared resulting bytes to all **22 actual worktree paths**: **22 matched, 0 mismatched**. EVIDENCE.json records each path's current SHA-256/size. This also covers the five untracked new Rust files; a tracked-only diff was not mistaken for the whole candidate.

The attributed dispatch source `233bd5a8f9e92baf04f65f19ede8d693d7dfabf3` predates accepted E1. The published patch actually targets HEAD aff3f408e and applies cleanly in semantic review. Overlap is expected: E1 moved current request/definition identity ahead of install freshness; E2 adds an explicit update mode, retains that pre-fast-path identity computation and gates configured update through its own intent/reconfiguration preflight. E2 does not remove the E1 comparison, discovery owners or ordinary install/replay enforcement. This is not evidence that the stale dispatch basis itself became fresh; Root must reconcile the actual reviewed source boundary without an incident-specific recovery adapter.

## Exact prepared reinstall

`crates/vibe-install/src/evolution.rs` implements an immutable PreparedReinstall with private lock/payload fields. from_planned binds the proof to the original held plan's captured catalog and checks locked-package membership rather than accepting an unrelated current catalog as historical proof. The current request must pass the retained D1 replay law. Every locked package is checked, including leaves without edges: source/content/definition identity, current payload hash and manifest, version, features, induced dependencies, embedded source pins and materialization must agree. The recomputed D1 requested/resolved record must equal the lock.

The proof construction and validate paths contain no technical solve/provider/fetch/replacement-capture call. Revalidation refuses changed lock/payload or unavailable payload; plain regeneration verifies installed slots through the incumbent full materialization verifier before continuation/lifecycle effects. Prepared force repairs the same captured resolution/bytes and uses existing lifecycle/materialization owners. It does not select another branch/default or grant hosted repark authority. Exact prepared paths suppress the existing confirmation prompt; unprepared configured routes refuse rather than pretending to hold exact authority.

EvolutionReport preserves requested/resolved/package before/after values and differentiates ExactReuse, changed semantic inputs, unavailable evidence and required reconfiguration. Missing after-state remains null. Refusal reports retain captures and current payload/slot observations instead of inventing empty diffs.

## Intent-preserving update

plan_update compares original authored consumer request, explicit/empty/dormant selections, feature flags, roots and origins before applying the incumbent resolver. It detects removed selected groups/members, newly required groups and missing exact shown-definition default consent using existing B1/B2 evaluation. It does not promote a resolved choice into authored authority or implement a second option selector.

Candidate-specific invalidity does not veto every version: preflight retains per-candidate needs; the existing resolver can select a compatible older candidate, and a selected candidate with remaining needs yields no applicable plan. Changes in defaults cannot replace an explicit choice. A successful update report includes full old/new definitions/default receipts, requested/resolved effects/provenance/selected graph and package source/payload/version/edge rows.

The private update marker permits definition evolution only through that preflight while keeping D1 exact replay behavior elsewhere. It continues to enforce original configuration/feature intent at apply. Pending manifest migration during update planning remains in memory until apply; a reconfiguration result has no applicable plan. Source capture/fetch during normal planning is not misrepresented as an authored-world apply.

ConfiguredUpdateSource binds trusted normalized intent to the existing factory/transaction; it has no legacy solve/default fallback. Existing technical solver and source transactions still own dependency selection. Named prepared updates reuse installed-package qualification and manifest constraints; they cannot turn an absent package into an add/install operation. Raw configured scoped/whole update paths cannot discard configuration authority via force or assume-yes. Legacy unconfigured update remains on its ordinary route.

## CLI and coverage boundary

The positive CLI tests run commands::reinstall::run_with_capture and commands::update::run_with_intent through the normal lease, trace, source, confirmation and lifecycle owners, rather than a test-only reducer. The prepared source uses the standard build_install_resolver/bind_request path. Negative tests exercise raw command refusal without authority, including force/assume-yes; positive prepared force repairs the same payload and leaves lock bytes unchanged.

These tests are direct typed command-adapter tests in the real `vibe` bin test target. They do **not** launch a parser-driven user process with newly implemented raw choice syntax, nor establish JSON/TTY/machine choice transport. The authored report states that boundary; the original E4 task owns that grammar/interaction work. The ordinary unprepared CLI does not acquire typed proof from a flag. This review does not silently expand E2 into E4.

## Recorded checks and retained failures

Copied the original existing /tmp logs into this review's evidence/ directory without rerunning anything. Their literal summaries match the authored report:

| Recorded target | Passed | Failed | Filtered |
|---|---:|---:|---:|
| vibe-install --test m_05_e_2_cases, E2 family | 3 | 0 | 28 |
| vibe-orchestrator --lib, E2 family | 6 | 0 | 660 |
| vibe-cli --bin vibe, E2 family | 6 | 0 | 952 |
| affected exact orchestrator repeat | 1 | 0 | 665 |

The exact repeat is one of the existing six cases with a missing-request assertion, not a sixteenth unique test. The report's retained checks JSON records exit0 for the three family runs. Final log/source binding is retained evidence, not an independent test execution or a Cargo-generated source-hash attestation.

The report truthfully retains the initial locked dependency/Cargo.lock refusal, compilation-only zero-test CLI check, initial orchestrator 2 passed/2 failed and its corrected meaningful nonzero reruns. It does not count compilation as functional test success. Global cargo fmt remains **exit1**; XML/scoped new-file formatting and diff whitespace are reported separately. No broad new check was invented for this review.

## Formatting report inconsistency resolved by actual evidence

The authored report's heading “Global fmt baseline files” is followed by an erroneous duplicate of the E2 changed-path list, starting with modified command files. That list is **not** the actual formatter failure set.

The retained actual /tmp/m05-e2-fmt.log and literal formatter stdout identify seven distinct files:

- crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs
- crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs
- crates/vibe-mcp/src/machine_session/incremental.rs
- crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs
- crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs
- crates/vibe-publish/src/package_git_payload/tests.rs
- crates/vibe-wire/src/generated/format_id/properties.rs

Every file's current bytes equal HEAD aff3f408e **and** pre-E1 source 233bd5a8. They are outside the E2 patch. Thus the formatter failure is an actual preexisting baseline failure, introduced by earlier committed source, not by E1 or E2. EVIDENCE.json records per-file hash and last modifying commit. The erroneous prose list should be corrected in acceptance evidence; the actual exit1 must remain visible. No source formatting was changed here.

## Recommendation and limits

The concrete E2 mechanisms, spec and recorded focused evidence support a bounded integration recommendation, subject to Root's independent acceptance and source/basis handling. No actionable P1/P2 is established by this review. Keep the global formatting failure/report-list correction explicit.

E3 transaction/recovery/removal, E4 raw choice interaction, full request/catalog race/failure-window qualification and broader campaign acceptance remain outside this result. No manual incident adapter, new solver, Zap task-semantic owner, PodBay process owner or cross-project implementation was added. VibeVM remains packages/workspace/context; Zap remains long-horizon task semantics; PodBay remains physical process/lifecycle.
