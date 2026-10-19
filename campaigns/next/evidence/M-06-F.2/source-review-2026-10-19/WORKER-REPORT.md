# M-06-F.2 continuation: scale, snapshot integrity and unchanged boot law

This report records coding and local checks, not canonical Work acceptance or host verification. Both requested criterion statements have positive local evidence. Final original families selected 11 tests: 11 passed, 0 failed. Three additional direct SafeFS regressions passed. The literal global formatting check still fails on unrelated baseline files; its full output is retained below.

## Current source, retained work and changes

Current read-only HEAD is a50bcb82edcf48b8002faa259f7a0c51a03b9e5a. The operator's older source/count and acceptance statements remain attributed context. I inspected the actual dirty/untracked workspace, complete campaigns/next/tasks/M-06-F.json, governing PROP-072/PROP-048, named module owners and 90-user.xml. I reused all five retained untracked fixture/test files and the existing module registrations and DEBUGGER-SCALE-EVIDENCE clause. Prior patch c36088f903f8beaf0a2de9c5d5cf5672cdfd20eba93c4b15c9843545842c1550 and report e64d69418b992cc42719a332f55bc4e530de4423655e7d928e4f9c07f4a8260c remain historical artifacts, not this attempt's verification. No WIP was reset or discarded.

The combined patch contains that retained work plus these continuation changes:

* SafeFS file/bounded.rs: add Project::capture_file_snapshot, sharing the existing stable snapshot implementation. Trusted finite capture uses observed held-file length as the read fence, not a fixed diagnostic admission quota. The no-follow capability open, regular/single-link verification, two held-handle passes with exact equality, mode/length recheck and final-name proof remain. Checked length conversion, checked fence arithmetic and try_reserve_exact preserve explicit failure instead of truncated or empty success. The returned owned bytes remain historical after in-place mutation. Bounded readers retain their caps and usize::MAX rejection.
* DocumentAnalysisHost::for_boot_capture separates trusted debugger capture from ordinary DocumentAnalysisHost::new. Existing new retains strict resource/aggregate policy. Disk capture and freeze for the explicitly selected boot policy have no fixed resource/aggregate diagnostic quota. Open-buffer size/count and request controls remain independent; checked aggregate arithmetic remains. No transport value can select this host policy.
* InstallResolver::open_boot_debugger selects this policy through its real standard constructor. Its retained AnalysisLimits values now govern editor buffers, not boot admission. Both source/preview.rs read sites (boot snippets/fragments and recursive spec tree) use the same finite stable capture primitive.
* CLI regressions now check a real successful Capture envelope for the retained 64 MiB + 1 fixture, and separately prove strict editor capture refusal, complete trusted capture beyond small resource/aggregate limits, buffer-limit refusal, immutable old bytes and a new identity after mutation.
* Three SafeFS tests directly cover 64 MiB + 1 complete owned capture, strict bounded-reader refusal, absent/empty resources, immutable history, growth/final-name swap refusal and link refusal.
* PROP-072 adds DEBUGGER-CAPTURE-POLICY. Existing DEBUGGER-SCALE-EVIDENCE is retained. Both clauses are additive: CAPTURED-GRAPH, IDENTITY, KNOWLEDGE, READ-ONLY, CAPTURED-MEASUREMENT, protocol/joins, viewer/parity, all hypothetical and marginal-comparison laws remain independent. PROP-048 BOOT-NO-ADMISSION-QUOTAS is unchanged. No law was replaced.

Only the original task perimeter plus the six explicitly authorized corrective owners changed (including bounded_tests.rs authorized alongside bounded.rs). No manifests/dependencies, schemas, unrelated declarations, parent/global config, native/SDK/process/activation implementation or other project sources changed. No installs, new models, delegation, Git mutation, commit/push, lifecycle action or persistent-process shutdown occurred. VibeVM remains package/context owner, Zap semantic/acceptance owner, PodBay process/I/O owner.

## Execution and backend qualification

Used the supplied exclusive target /fast/zap-managed/2026-10-14-m10b1-correction/cargo-target and existing private TMPDIR /tmp/vibevm-m06f2-continue-xjq4qtxa (observed mode 0700). Every test used absolute /home/olegchir/.cargo/bin/cargo, --offline --locked -j2, CARGO_NET_OFFLINE=true, RUSTUP_AUTO_INSTALL=0 and default stack. No default/full-suite/discovery builds or Zap paths.toml override were invoked. The four prebound selections were source-inspected and retained; all observed nonzero counts are below. Checks ran sequentially against this target.

The first deciding CLI command, before correction and without backend override, selected exactly 1 regression and FAILED (exit 101), reproducing exit 1 / `error: document capture unavailable`. Its complete test failure section is below. That failure is not erased by later success.

The second deciding command used only --config 'profile.dev.package.vibe-spec.codegen-backend="llvm"' and selected compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink -- --exact: 1 passed, 0 failed, exit 0. No compiler implementation change or panic suppression was necessary. This qualifies that run; it neither retroactively passes the previous configured run nor proves a blanket Cranelift defect. The earlier report's direct catch_unwind failure remains attributed prior evidence.

Read parent config declares dev Cranelift and sha2 LLVM; local rust-toolchain.toml declares stable. Actual rustc -vV here reports 1.101.0-nightly c36f1457196e315bc204b9564a6a5a7fe7f5a51f (2026-10-01), x86_64-unknown-linux-gnu, LLVM 23.1.1. Final families all used the authorized per-command vibe-spec LLVM override. Other packages retain inherited backend configuration; no configuration file or toolchain was changed. Production release behavior was not tested.

## Exact final bindings and outcomes

Each row follows `/home/olegchir/.cargo/bin/cargo --config 'profile.dev.package.vibe-spec.codegen-backend="llvm"' test --offline --locked -j2` and ends `-- --nocapture`. These are local observations, not host verification IDs.

| Selection | Selected result | Exit |
| --- | --- | --- |
| -p vibe-workspace --lib boot_debugger::m_06_f_2_cases | 3 passed, 0 failed | 0 |
| -p vibe-cli --test m_06_f_2_cases m_06_f_2_cases | 4 passed, 0 failed | 0 |
| -p vibe-mcp --lib tools::boot_debugger::m_06_f_2_cases | 2 passed, 0 failed | 0 |
| -p vibe-spec --lib compiler::m_06_f_2_cases | 2 passed, 0 failed | 0 |
| -p vibe-safefs --lib file::bounded_tests::m_06_f_2_capture | 3 passed, 0 failed | 0 |
| /home/olegchir/.cargo/bin/cargo fmt --all -- --check | baseline formatting differences, full literal output below | 1 |
| git diff --check | no whitespace errors | 0 |

Additionally rustfmt --edition 2024 --check over all 14 changed/new Rust files passed (exit 0). This does not replace the required global fmt failure. Existing compilation warnings remain in local logs; no unrelated warning cleanup occurred.

## Scale and memory: separate quantities

Synthetic recipe m06f2/v1 generates 128 ordered contributions with 8,192 multibyte `λ ` whitespace units each: 1,048,576 synthetic units. The full artifact is 3,152,896 bytes and 2,104,064 Unicode scalars including framing. These are not exact model tokens. Workspace measurement preserves 24,576 unique raw bytes versus 3,145,728 repeated raw bytes, repeated identity, ordinary opaque data and an INDEX with repeated ordinary entries and an unevaluated os:linux condition. Same captured byte/scalar/qualified-tokenizer results repeat identically. No named tokenizer exists in this recipe, so its count/identity remain explicitly unavailable, never zero or a byte ratio. Billing, ambient context and actual conditional runtime reads remain unknown separately.

The registered MCP route reassembled every captured resource twice exactly: 6,305,958 bytes across 3 resources and 3,084 chunks. All 128 ordered occurrence rows were reached and compared to Inspect, in pages of at most 7 rows. Actual observed maximum raw chunk was 2,048 bytes, serialized chunk RPC response 10,169 bytes, and page JSON 2,376 bytes. The real structured CLI independently reassembled 3,152,896 frozen bytes in 1,540 chunks after a live edit.

Transport chunk/frame maxima are distinct from total result size and from all retained memory. MemoryTransport accumulates one full response; its measured serialized frame length does not count simultaneous copies or allocator capacity. Test-side reconstruction buffers also scale. The final MCP process observed Linux VmHWM 108,628 kB, including server/test allocations and potentially the parallel preview test. This is a real process high-water outcome, not isolated debugger heap, RSS scaling proof or constant-memory theorem. Full Inspect/Viewer metadata, captures, snapshot copies and retained::read_run Vecs can scale with content. Total debugger heap/transport-buffer allocation high-water was not instrumented; it remains unavailable. No high-size refusal is used as evidence of a bound: the 64 MiB + 1 real CLI fixture now succeeds, with a typed Capture, without reducing its size.

## Snapshot, cancellation, retention and unchanged compilation

Consumed cursors refuse replay, mixed snapshot/page identities refuse, and live STATIC mutation creates a fresh snapshot while old Inspect/Cost and chunk bytes remain unchanged. Inspection cancellation reaches ReachedSafeStop, scope InspectionOnly, build_cancelled NotRequested; further requests on that inspection refuse while retained capture inspection still works. This is inspection-only cancellation evidence, not cancellation of an in-flight compiler or OS process.

Actual TraceRun captures nine successful compiler snapshots totaling 35,672,542 bytes. The strict retained reader compares stored bytes and event sequence/carrier; one deliberately removed child becomes explicit Missing while all eight available children remain reachable. A separate disposable zero-budget TraceRun records unavailable/skipped capture explicitly, preserves compiler output, completes, and is removed by an explicit later writer retention sweep. Subsequent read says Missing, not an invented persistent Evicted status. The fixture witnesses that known retention removal; no retained read performs retention or provider fallback.

The four-repeat compiler fixture retains 1,048,576 Unicode units, four literal status fields and 3,147,154 canonical bytes. Untraced/traced and repeated unchanged-declaration outputs are byte-identical under ordinary observation, budget skip, encoder refusal and (in the qualified LLVM run) panicking sink. Events for skipped/failed snapshots remain explicitly incomplete with no snapshot bytes. No compiler emission law, order, status, compaction, externalization or admission gate was changed. Interactive CLI also checks its canonical artifact stays byte-identical and opens no trace writer.

## Production caller paths actually exercised

CLI: main.rs Command::Boot -> commands::boot::run -> run_with_io -> open_route -> InstallResolver::open_boot_debugger -> DocumentAnalysisHost::for_boot_capture -> BootDebuggerRoute::new/freeze -> SafeFS stable capture. Integration tests drive the built `vibe boot --root ... --view` executable with interactive select/source/cost/quit commands, and the real structured `vibe boot --root ...` process with initial Capture and Inspect/Chunk envelopes over pipes. The large-size regression also invokes that executable and checks the returned Capture envelope; argument parsing alone is not the proof.

MCP: Server::new -> default registered tools -> JSON-RPC tools/call boot_debugger -> BootDebuggerMcpTool::run -> ServerContext::boot_debugger_route -> ordinary InstallResolver::open_boot_debugger -> shared retained route. Tests drive registered dispatch through MemoryTransport, including complete chunks/pages and cancel. They do not claim a real stdio/network MCP process or a generic provider runtime.

Dependency-aware preview: the same registered MCP constructor has explicitly configured retained derivative history using ServerContext::with_source_derivatives. The prerequisite local app/local package fixture provides real source/manifest/catalog inputs. Scenario dispatch calls the admitted LocalProducer -> InstallResolver::capture_boot_scenario_with_derivatives -> capture_boot_world_with_derivatives (including the corrected stable source-read sites) -> incumbent normalized resolver and PreparedScenario. Preview evaluates those returned retained selectors with the incumbent compiler. It proves app -> local inclusive dependency membership and identical baseline/candidate raw values for unchanged declarations. Mutating live manifest and source preserves the old result; a new scenario has new catalog/world identities; mixed expected catalog returns typed Stale with no artifact. No automatic derivative-history discovery or native/remote preparation support is claimed. High-size prepared-preview performance itself was not benchmarked separately; the positive scenario exercises its real production capture/evaluation path with the existing dependency fixture.

## Criteria and remaining limits

Slot 0 — Scale evidence separates result size from transport chunk limits and records memory/cancellation outcomes: locally satisfied by the actual quantities, complete traversal, memory observation, cancellation and corrected large-size regression above. Synthetic units are qualified; exact model tokenization and total heap bound remain unavailable.

Slot 1 — M-06 exit includes interactive UI, structured input/output and dependency-aware preview: locally satisfied by real CLI processes plus registered MCP Scenario/Preview over prepared source inputs above. Existing line UI is the exposed interactive UI; no browser/IDE claim is made.

Remaining validation gap: global fmt fails in unrelated baseline files. Backend cause of the earlier panic failure remains unproven; positive unwind evidence is qualified to this command's vibe-spec LLVM configuration. Exact model tokenization, isolated heap/transport allocation high-water and total heap bounds remain unavailable. These are not fabricated successes. Root retains independent semantic review, check references and acceptance authority.

Follow-up: Root can review the combined patch and current evidence, retain the baseline global-fmt failure, and decide acceptance. Any backend diagnosis or unrelated formatting repair belongs to separately scoped owner work; no further out-of-perimeter source correction is claimed necessary by these focused results.

## Retained current command evidence

### cli final output

```text
     Running tests/m_06_f_2_cases.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-cli/89f82673b817a5f0/out/m_06_f_2_cases-89f82673b817a5f0)

running 4 tests
test m_06_f_2_cases_capture_policy_separates_editor_limits_and_aggregate_capture ... ok
m06f2 real vibe boot --view stdin: 128 rows, selection/source/cost, bytes preserved, units=1048576
test m_06_f_2_cases_interactive_line_ui_keeps_large_capture_and_canonical_bytes ... ok
test m_06_f_2_cases_large_capture_must_not_be_refused_by_diagnostic_size_gate ... ok
m06f2 real structured CLI: result_bytes=3152896 chunks=1540 chunk_limit=2048 old capture unchanged after live edit
test m_06_f_2_cases_structured_process_reads_frozen_chunks_after_live_change ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.84s

```

### workspace final output

```text
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-workspace/e859dae3a34946cb/out/vibe_workspace-e859dae3a34946cb)

running 3 tests
m06f2 trace budget=0: explicit unavailable; retention removed owned completed run; absent read=Missing, not inferred Evicted; canonical identical
test boot_debugger::m_06_f_2_cases::trace_budget_retention_and_missing_children_do_not_change_boot ... ok
m06f2 recipe=v1 units=1048576 bytes=3152896 scalars=2104064 unique_raw=24576 repeated_raw=3145728 tokenizer=unavailable
test boot_debugger::m_06_f_2_cases::million_units_keep_repeats_conditions_and_reproducible_qualified_measurements ... ok
m06f2 complete trace units=1048576 snapshots=9 snapshot_bytes=35672542 one removed child explicit Missing; canonical bytes identical
test boot_debugger::m_06_f_2_cases::large_complete_trace_keeps_exact_snapshots_and_explicit_missing_child ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 772 filtered out; finished in 3.25s

```

### mcp final output

```text
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-mcp/43d81e3b7d492f81/out/vibe_mcp-43d81e3b7d492f81)

running 2 tests
m06f2 registered MCP preview app->local dependency retained; changed live catalog preserves old preview; new catalog selector refuses old scenario
test tools::boot_debugger::m_06_f_2_cases::preview::registered_server_previews_dependencies_and_refuses_changed_catalog_selectors ... ok
m06f2 twice complete resource traversal bytes=6305958 resources=3
m06f2 units=1048576 result_bytes=3152896 rows=128 chunks=3084 max_chunk_bytes=2048 max_chunk_rpc_frame=10169 max_page_json=2376 cancel=ReachedSafeStop memory_process_high_water=Some("VmHWM:\t  108628 kB"); no total heap bound asserted
test tools::boot_debugger::m_06_f_2_cases::million_units_registered_server_traverses_chunks_pages_and_cancels_only_inspection ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 231 filtered out; finished in 4.96s

```

### spec final output

```text
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-spec/b9c73778e63b1d66/out/vibe_spec-b9c73778e63b1d66)

running 2 tests

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure

thread 'compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink' (1174800) panicked at crates/vibe-spec/src/compiler/m_06_f_2_cases.rs:26:13:
deliberate observer failure
test compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink ... ok
m06f2 compiler units=1048576 bytes=3147154 status_fields=4 canonical=identical budget/encoder complete_capture=false
test compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_budget_and_encoder_defects ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1013 filtered out; finished in 0.53s

```

### safefs final output

```text
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-safefs/297b95d156dbb768/out/vibe_safefs-297b95d156dbb768)

running 3 tests
test file::bounded_tests::m_06_f_2_capture_preserves_link_refusals ... ok
test file::bounded_tests::m_06_f_2_capture_refuses_growth_and_replaced_final_name ... ok
test file::bounded_tests::m_06_f_2_capture_owns_bytes_without_weakening_bounded_readers ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 177 filtered out; finished in 0.29s

```

### Initial deciding CLI failure (before correction)

```text
     Running tests/m_06_f_2_cases.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-cli/871e583e5def0855/out/m_06_f_2_cases-871e583e5def0855)

running 1 test
test m_06_f_2_cases_large_capture_must_not_be_refused_by_diagnostic_size_gate ... FAILED

failures:

---- m_06_f_2_cases_large_capture_must_not_be_refused_by_diagnostic_size_gate stdout ----

thread 'm_06_f_2_cases_large_capture_must_not_be_refused_by_diagnostic_size_gate' (1141751) panicked at /rustc/c36f1457196e315bc204b9564a6a5a7fe7f5a51f/library/core/src/ops/function.rs:250:5:
Unexpected failure.
code=1
stderr=```"error: document capture unavailable\n"```
command=`VIBE_NO_DEFAULT_REGISTRY="1" "/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/vibe" "boot" "--root" "/tmp/vibevm-m06f2-continue-xjq4qtxa/.tmpzZ8SCn"`
stdin=`""`
code=1
stdout=""
stderr="error: document capture unavailable\n"

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    m_06_f_2_cases_large_capture_must_not_be_refused_by_diagnostic_size_gate

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.03s

error: test failed, to rerun pass `-p vibe-cli --test m_06_f_2_cases`
```

### Initial deciding LLVM sink-panic qualification

```text
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-spec/b9c73778e63b1d66/out/vibe_spec-b9c73778e63b1d66)

running 1 test
test compiler::m_06_f_2_cases::million_units_canonical_bytes_survive_panicking_sink ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1014 filtered out; finished in 0.30s

```

### Literal global formatting failure

```text
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
```

### Exact recorded final command vectors

```json
[
  {
    "name": "cli",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "--config",
      "profile.dev.package.vibe-spec.codegen-backend=\"llvm\"",
      "test",
      "--offline",
      "--locked",
      "-j2",
      "-p",
      "vibe-cli",
      "--test",
      "m_06_f_2_cases",
      "m_06_f_2_cases",
      "--",
      "--nocapture"
    ],
    "exit": 0
  },
  {
    "name": "workspace",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "--config",
      "profile.dev.package.vibe-spec.codegen-backend=\"llvm\"",
      "test",
      "--offline",
      "--locked",
      "-j2",
      "-p",
      "vibe-workspace",
      "--lib",
      "boot_debugger::m_06_f_2_cases",
      "--",
      "--nocapture"
    ],
    "exit": 0
  },
  {
    "name": "mcp",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "--config",
      "profile.dev.package.vibe-spec.codegen-backend=\"llvm\"",
      "test",
      "--offline",
      "--locked",
      "-j2",
      "-p",
      "vibe-mcp",
      "--lib",
      "tools::boot_debugger::m_06_f_2_cases",
      "--",
      "--nocapture"
    ],
    "exit": 0
  },
  {
    "name": "spec",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "--config",
      "profile.dev.package.vibe-spec.codegen-backend=\"llvm\"",
      "test",
      "--offline",
      "--locked",
      "-j2",
      "-p",
      "vibe-spec",
      "--lib",
      "compiler::m_06_f_2_cases",
      "--",
      "--nocapture"
    ],
    "exit": 0
  },
  {
    "name": "safefs",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "--config",
      "profile.dev.package.vibe-spec.codegen-backend=\"llvm\"",
      "test",
      "--offline",
      "--locked",
      "-j2",
      "-p",
      "vibe-safefs",
      "--lib",
      "file::bounded_tests::m_06_f_2_capture",
      "--",
      "--nocapture"
    ],
    "exit": 0
  }
]
```
