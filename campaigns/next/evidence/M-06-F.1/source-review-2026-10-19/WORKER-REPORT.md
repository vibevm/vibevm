M-06-F.1 — derivatives and debugger surface parity

Source basis: actual HEAD 3bdaad1b5fb351f4dc2f6f6723c58f97d864e0e4, initially clean tracked/untracked status. This is new content-only implementation evidence, not acceptance of Work or a planning-state mutation. No commits, pushes, installs, delegation or model calls.

Changes and preserved laws
- Reused the accepted derivative catalogue, protected same-invocation compiler read/closure capture, prepared scenario constructor, canonical BootDebuggerRoute, structured CLI adapter and registered MCP tool. These already preserve exact derivative base/local/result inventories and repeated compiler occurrence identities. No replacement provenance model or resolver was added.
- Added viewer Actions Inspect, Cost and Resource, exposed through ordinary `vibe boot --view` and `--send` commands `inspect`, `cost`, `resource ORDINAL [BYTE_OFFSET]`; `why` aliases the existing typed explanation. Inspect returns all resources, sources, graph diagnostics and the original compiler carrier. Resource ordinals select only the current immutable inventory, with bounded chunks and unchanged original_bytes/captured_projection basis. Invalid ordinals refuse.
- Added PROP-072 DEBUGGER-SURFACE-PARITY additively. All prior independent source, graph, measurement, capture, hypothetical, derivative and no-effect laws remain verbatim. No law was replaced or weakened. Existing gaps stay explicit: unverified current base, absent authored/inverse/parent joins, unavailable tokenization, billing and ambient context.
- New families are vibe-workspace --lib boot_debugger::m_06_f_1_cases, vibe-cli --bin vibe commands::boot::m_06_f_1_cases and inline vibe-mcp --lib tools::boot_debugger::m_06_f_1_cases. Reused the existing five-family current-history regression and real preparation-revocation regression through test-only pub(super) functions; the revocation regression now also checks Cost has NoEffects.

Production exposure traces and prepared positive fixtures
1. CLI: crates/vibe-cli/src/cli.rs Command::Boot -> main.rs Boot dispatch -> commands::boot::run -> run_with_io -> open_route -> InstallResolver::open_boot_debugger and configure_derivatives -> viewer::serve -> Viewer::apply -> RouteService::query -> BootDebuggerRoute::exchange. The new standard_cli_constructor_drives_cost_inspect_why_and_resource fixture parses real Boot arguments and executes run_with_io over a real local-registry derivative scenario and retained result file. It drives inspect/cost/why/source/resource/compare and a refusal; it does not stop at parser/serialization assertions. Current retained history additionally runs this same constructor for CommonMark, XML-data, Vibe-spec, opaque-file and nested-tree inputs.
2. MCP: Server::new registers default_tools -> Server::run -> tools/call dispatch -> BootDebuggerMcpTool::run -> ServerContext::boot_debugger_route -> lazy InstallResolver::open_boot_debugger + explicitly configured source derivative catalogue -> shared route. The new default_registered_host_retains_full_derivative_inspection_after_provider_removal fixture drives Server::new and Server::run with JSON-RPC MemoryTransport messages, retaining the same server context between calls. It discovers the ordinary host, explicitly prepares a real derivative scenario, removes live registry/base/local trees, and reads full inspection, qualified cost and every retained resource in seven-byte chunks. The actual preparation-service fixture separately revokes its real provider and removes the registry, then reads retained preparation, inspect, cost and compare without effects.
3. Library/canonical API: exported boot_debugger::viewer::Viewer and its typed Service; BootDebuggerRoute::request is the canonical typed route and exchange is the canonical envelope route. retained_derivative_all_surfaces_and_every_chunk_and_page_agree compares exact results/envelopes against structured CLI serve, registered MCP tool and the viewer over one immutable produced capture. It exhausts every graph viewport, explanation, resource chunk and every one-row tree page across API/CLI/MCP. Stateful page sessions use their own legitimate cursors and compare every resulting row without confusing opaque session IDs with domain identity.
4. Native mapping: standard_compiler_capture_keeps_native_format_without_inventing_authored_mapping passes a valid captured compiler source carrier with an open native transform format through the ordinary --kind compiler host constructor. The absent backend is never loaded. Exact carrier/schema/text remain available through inspect and resources; authored/derivative joins remain null and an occurrence-based source request refuses. This is a retained-carrier fixture, not a claim that native execution or inverse mapping is implemented. A workspace negative fixture also preserves Unknown for unmapped transformed output.

Observed checks
All test commands used the existing /home/olegchir/.cargo/bin/cargo, --offline --locked -j2, RUSTUP_AUTO_INSTALL=0, CARGO_NET_OFFLINE=true, exclusive CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target, private mode0700 TMPDIR=/tmp/vibevm-m06f1-raadqrws. /proc scans before the initial build and final workspace run found zero relevant holders and zero permission gaps. Own builds were sequential and their exit codes observed. No toolchain install/version change or additional target discovery build.
- cargo test --offline --locked -j2 -p vibe-workspace --lib boot_debugger::m_06_f_1_cases: exit 0, 2 passed, 0 failed, 770 filtered out.
- cargo test --offline --locked -j2 -p vibe-cli --bin vibe commands::boot::m_06_f_1_cases: exit 0, 4 passed, 0 failed, 978 filtered out.
- cargo test --offline --locked -j2 -p vibe-mcp --lib tools::boot_debugger::m_06_f_1_cases: exit 0, 2 passed, 0 failed, 229 filtered out. Final version exercises Server::run.
- cargo fmt --all -- --check: exit 1, literal output retained below. Seven unrelated files are outside this write perimeter. No unrelated formatting applied. Changed Rust files were formatted with the existing rustfmt.
- git diff --check: exit 0, no output.

Retained unsuccessful attempts
The first workspace run selected 2 tests: 1 passed and 1 failed because the new fixture omitted the required exact current-base witness. The fixture now captures and supplies that witness; no production admission was weakened.
The first CLI run selected 2 tests: 0 passed, 2 failed. One fixture omitted mandatory baseline selection fields, returning Incompatible; another expected 11 frames for 9 commands plus initial frame. Both fixture errors were corrected, then all selected cases passed. The second CLI run passed 3/3; the final run added the reused current-history case and passed 4/4. Original failure excerpts are retained below. Compiler warnings were present and are not reported as clean warning checks.

Criteria assessment
Slot 0 (Equivalent requests agree on structured nodes/metrics/diagnostics across surfaces): implementation evidence supports satisfied for the tested current and prepared derivative captures. Exact retained typed results, unavailable diagnostics and identities agree; no adapter recalculates cost or provenance.
Slot 1 (Whole authorized structure is reachable without human-only hidden fields): implementation evidence supports satisfied for finite authorized captures. Full inspect exposes inventory and original carrier; every retained resource and occurrence is traversable through the viewer and machine routes. Missing mappings remain missing rather than fabricated structure.
These assessments do not perform host-run verification or ordinary acceptance. F2 scale/concurrency/stress and hostile-writer/atomic observation proofs are not claimed. No new provider/compiler authority is granted by retained inspection. The sole outstanding check failure is unrelated global formatting; ordinary independent review and host verification remain with the host.

Actual effects: local source/spec/test edits, Cargo build/test outputs in the granted target and private TMP, report/patch creation, and authorized content-only artifact publication. Fixture preparation/revocation effects are confined to temporary directories. No project manifest, installed dependency or canonical boot mutation outside fixtures; no Git mutation or Work-state mutation.

Final workspace result

running 2 tests
test boot_debugger::m_06_f_1_cases::unmapped_native_emission_never_invents_inverse_source_or_parent ... ok
test boot_debugger::m_06_f_1_cases::equal_local_bytes_keep_distinct_base_and_result_after_sources_disappear ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 770 filtered out; finished in 0.02s


Final CLI result

running 4 tests
test commands::boot::m_06_f_1_cases::standard_compiler_capture_keeps_native_format_without_inventing_authored_mapping ... ok
test commands::boot::m_06_f_1_cases::standard_cli_constructor_drives_cost_inspect_why_and_resource ... ok
test commands::boot::m_06_f_1_cases::retained_derivative_all_surfaces_and_every_chunk_and_page_agree ... ok
test commands::boot::m_06_f_1_cases::current_retained_history_preserves_all_five_derivative_families ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 978 filtered out; finished in 0.21s


Final MCP result

running 2 tests
test tools::boot_debugger::m_06_f_1_cases::revoked_preparation_provider_retains_captured_data_without_execution ... ok
test tools::boot_debugger::m_06_f_1_cases::default_registered_host_retains_full_derivative_inspection_after_provider_removal ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 229 filtered out; finished in 0.36s


Initial workspace failure

running 2 tests
test boot_debugger::m_06_f_1_cases::unmapped_native_emission_never_invents_inverse_source_or_parent ... ok
test boot_debugger::m_06_f_1_cases::equal_local_bytes_keep_distinct_base_and_result_after_sources_disappear ... FAILED

failures:

---- boot_debugger::m_06_f_1_cases::equal_local_bytes_keep_distinct_base_and_result_after_sources_disappear stdout ----

thread 'boot_debugger::m_06_f_1_cases::equal_local_bytes_keep_distinct_base_and_result_after_sources_disappear' (541923) panicked at crates/vibe-workspace/src/boot_debugger/m_06_f_1_cases.rs:11:10:
called `Result::unwrap()` on an `Err` value: BindingError("exact current base absent")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    boot_debugger::m_06_f_1_cases::equal_local_bytes_keep_distinct_base_and_result_after_sources_disappear

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 770 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p vibe-workspace --lib`

Initial CLI failures

running 2 tests
test commands::boot::m_06_f_1_cases::retained_derivative_all_surfaces_and_every_chunk_and_page_agree ... FAILED
test commands::boot::m_06_f_1_cases::standard_cli_constructor_drives_cost_inspect_why_and_resource ... FAILED

failures:

---- commands::boot::m_06_f_1_cases::retained_derivative_all_surfaces_and_every_chunk_and_page_agree stdout ----

thread 'commands::boot::m_06_f_1_cases::retained_derivative_all_surfaces_and_every_chunk_and_page_agree' (566208) panicked at crates/vibe-cli/src/commands/boot/m_06_f_1_cases.rs:73:5:
assertion `left == right` failed
  left: Incompatible
 right: Complete
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- commands::boot::m_06_f_1_cases::standard_cli_constructor_drives_cost_inspect_why_and_resource stdout ----

thread 'commands::boot::m_06_f_1_cases::standard_cli_constructor_drives_cost_inspect_why_and_resource' (566209) panicked at crates/vibe-cli/src/commands/boot/m_06_f_1_cases.rs:271:5:
assertion `left == right` failed
  left: 10
 right: 11


failures:
    commands::boot::m_06_f_1_cases::retained_derivative_all_surfaces_and_every_chunk_and_page_agree
    commands::boot::m_06_f_1_cases::standard_cli_constructor_drives_cost_inspect_why_and_resource

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 978 filtered out; finished in 0.10s

error: test failed, to rerun pass `-p vibe-cli --bin vibe`

Literal global formatting failure (exit 1)
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs:571:
             .count(),
         1
     );
-    assert!(fixture
-        .dir
-        .path()
-        .join("settings/bin/local-helper")
-        .exists());
+    assert!(
+        fixture
+            .dir
+            .path()
+            .join("settings/bin/local-helper")
+            .exists()
+    );
 }

Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:3:
 use std::sync::Arc;
 use vibe_core::manifest::Manifest;
 use vibe_lifecycle::{
-    process::SelectedFileWitness, ClientExecutable, ClientExecutables, SubmissionScope,
+    ClientExecutable, ClientExecutables, SubmissionScope, process::SelectedFileWitness,
 };
 use vibe_orchestrator::{
     machine_operation::fixed_deploy::FixedDeployRoute,
Diff in /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs:10:
     machine_submission::{
-        deploy_grant::{DeployGrantAttachment, DeployGrantService},
         TrustedPolicySource,
+        deploy_grant::{DeployGrantAttachment, DeployGrantService},
     },
 };
 use vibe_safefs::Project;
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:3:
 //! implementations can block. Only TcpFramePort establishes nonblocking mode here.
 specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

-use super::framing::{decode_body, FrameCodecError, FRAME_EPOCH};
+use super::framing::{FRAME_EPOCH, FrameCodecError, decode_body};
 use std::io::{self, Read, Write};
 use std::net::TcpStream;
 use vibe_wire::generated::machine::e1::session_frame::SessionFrame;
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:83:
                         FrameCodecError::TruncatedPrefix
                     } else {
                         FrameCodecError::TruncatedBody
-                    })
+                    });
                 }
                 Ok(count) => count,
                 Err(e)
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:92:
                         io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                     ) =>
                 {
-                    return Ok(ReadPoll::Pending)
+                    return Ok(ReadPoll::Pending);
                 }
                 Err(e) => return Err(FrameCodecError::Io(e)),
             };
Diff in /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs:234:
                         io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                     ) =>
                 {
-                    return Ok(WritePoll::Pending)
+                    return Ok(WritePoll::Pending);
                 }
                 Err(e) => return Err(FrameCodecError::Io(e)),
             }
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs:100:
     let host = host(root.clone(), policy, source);
     let question = host.prepare_requested_options(no_options()).unwrap();
     assert_eq!(question.operation_id.as_deref(), Some(OPERATION));
-    assert!(question.source_id.as_deref().unwrap().starts_with("sha256:"));
-    assert!(!question.source_id.as_deref().unwrap().starts_with("sha256:sha256:"));
+    assert!(
+        question
+            .source_id
+            .as_deref()
+            .unwrap()
+            .starts_with("sha256:")
+    );
+    assert!(
+        !question
+            .source_id
+            .as_deref()
+            .unwrap()
+            .starts_with("sha256:sha256:")
+    );
     assert_eq!(question.policy_generation.is_some(), true);
     assert!(matches!(question.body, Body::ApprovalQuestion(_)));
     let refusal = host.answer_typed(&answer(&question, false)).err().unwrap();
Diff in /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs:2:
 //! Requested is only a bounded host-memory latch, never durable stop evidence.
 use super::*;
 use crate::machine_operation::stream_message::{self as validation};
-use vibe_wire::generated::lifecycle_outcome_marker::ReportEvidence;
-use vibe_wire::generated::machine::e1::operation_stream_message::*;
 use BodyControlReplyOperationEffectState as Effect;
 use BodyControlReplyStatus as Status;
+use vibe_wire::generated::lifecycle_outcome_marker::ReportEvidence;
+use vibe_wire::generated::machine::e1::operation_stream_message::*;

 #[derive(Clone)]
 pub(super) enum Gate {
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:34:
     fs::write(source.path().join(".gitmodules"), b"[submodule \"x\"]\n").unwrap();
     let complete = stage_local_payload(&input, &output, limits()).unwrap();
     assert_eq!(
-        inspect_local_payload(&output, limits(), &complete.source_root_token, &complete.tree_sha256),
+        inspect_local_payload(
+            &output,
+            limits(),
+            &complete.source_root_token,
+            &complete.tree_sha256
+        ),
         LocalPayloadInspection::Complete(complete.clone())
     );
     assert_eq!(complete.format, FORMAT);
Diff in /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs:184:
     let (_source, destination, input, output) = projects();
     let complete = stage_local_payload(&input, &output, limits()).unwrap();
     fs::write(destination.path().join("payload/vibe.toml"), b"tampered").unwrap();
-    assert!(matches!(inspect_local_payload(&output, limits(), &complete.source_root_token,
-        &complete.tree_sha256), LocalPayloadInspection::Uncertain(_)));
+    assert!(matches!(
+        inspect_local_payload(
+            &output,
+            limits(),
+            &complete.source_root_token,
+            &complete.tree_sha256
+        ),
+        LocalPayloadInspection::Uncertain(_)
+    ));
 }

 #[test]
Diff in /fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs:340:
             FormatId::MachineOperationDescriptor => "machine-operation-descriptor",
             FormatId::MachineOperationEnvelope => "machine-operation-envelope",
             FormatId::MachineOperationStreamMessage => "machine-operation-stream-message",
-            FormatId::MachineProfiledDeployActionRequest => "machine-profiled-deploy-action-request",
+            FormatId::MachineProfiledDeployActionRequest => {
+                "machine-profiled-deploy-action-request"
+            }
             FormatId::MachineSessionFrame => "machine-session-frame",
             FormatId::MachineSourceLocation => "machine-source-location",
             FormatId::Manifest => "manifest",

Changed paths
 M crates/vibe-cli/src/commands/boot/m_10_d_1_cases.rs
 M crates/vibe-cli/src/commands/boot/mod.rs
 M crates/vibe-cli/src/commands/boot/viewer.rs
 M crates/vibe-mcp/src/tools/boot_debugger.rs
 M crates/vibe-workspace/src/boot_debugger/mod.rs
 M crates/vibe-workspace/src/boot_debugger/viewer.rs
 M vibevm/vibespecs/common/PROP-072-boot-observability.xml
?? crates/vibe-cli/src/commands/boot/m_06_f_1_cases.rs
?? crates/vibe-workspace/src/boot_debugger/m_06_f_1_cases.rs
