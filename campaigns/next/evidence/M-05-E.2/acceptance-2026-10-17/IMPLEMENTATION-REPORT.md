M-05-E.2 — exact reinstall и intent-preserving update

Это content-only функциональный отчёт автора изменения. Claims ниже не являются Zap verified check results или Root acceptance. Human attribution и semantic judgment/Git/acceptance остаются у Root.

Source facts
- Первым чтением были ровно PROJECT-BOUNDARIES.xml и boot/90-user.xml. Full boot, stewardship/custody, PROP-055 и personal state не загружались.
- Прочитан governing permanent PROP-003-dep-evolution.xml целиком и оригинальный M-05-E.2 из campaigns/next/tasks/M-05-E.json.
- SHA256 оригинального task source подтверждён: 32a40b39db6e672e62e851f67eaf4dcd43330cd72d9bdd71b5406886af2b94d2.
- Supplied context атрибутирует исходный source commit 233bd5a8f9e92baf04f65f19ede8d693d7dfabf3. Наблюдаемый HEAD при финальной проверке: aff3f408eb86aec8e5f7462693f36e10c13a98d0. Момент/причина этого расхождения в этой работе не установлены; Git mutations не выполнялись. Patch сформирован относительно фактического HEAD, а не объявлен patch относительно 233bd5a.
- Входной WIP содержал только семь добавленных строк permanent E2 contract. Они сохранены без изменения, включены в общий patch; статус spec/plan не повышался.
- VibeVM остаётся владельцем packages/workspace/context; новые правила находятся в Rust owners. Zap/PodBay не дублировались. Subagents, models, probes, toolchain installs и source writes вне perimeter не использовались.

Что реализовано
1. vibe-install::evolution::PreparedReinstall проверяет retained D1 request/default receipts, исходный held plan catalog, текущие C1 source/content/definition identities для каждого locked package, включая leaf без edges, payload hashes, locked features, edges, provenance, embedded pins и materialization. Новый proof из PlannedInstall связан именно с catalog исходного plan: подмена независимым текущим catalog не становится историческим receipt. Prepared replay не вызывает technical solve/provider/fetch/replacement preparation. B2 остаётся evaluator, D1 — codec.
2. Replay report различает ExactReuse, SemanticInputChange и Unavailable. Refusal сохраняет before/after requests и captures, observed payload hashes и resource gaps; неизвестный результат остаётся null, а не пустым diff. Apply-time reobservation различает changed lock/payload и unavailable payload/slot. Plain prepared regeneration проверяет существующие slots incumbent verifier; повреждённый slot требует того же prepared --force.
3. plan_update сохраняет authored consumer requests, exact/additive/explicit-empty/dormant intent, features и origins. Смена package default не заменяет explicit choice. Removed groups/members, новые required groups и exact shown-default authorization gaps собираются в reconfiguration report до выдачи применимого plan. Invalid newer alternative не запрещает существующему resolver выбрать valid older candidate; selected candidate с оставшимся gap не выдаёт apply plan.
4. Update diff несёт полные before/after requested records (definitions/default receipts), resolved records (effects/dormant/provenance/selected graph) и package rows (source/payload/version/edges). Missing authority возвращает Unavailable. Нет второго option selector или solver/default/force/assume-yes fallback.
5. Incumbent plan/apply получил private update admission marker. Он разрешает definition evolution после E2 preflight, сохраняя D1 exact replay для остальных вызовов. Pending manifest migration в update planning не пишет authored bytes до apply. Existing selected resolver и normal source transaction сохраняются.
6. Public CLI adapters run_with_capture и run_with_intent проходят обычные lease/trace/lifecycle/source/apply owners. Prepared reinstall не спрашивает confirmation; --force ремонтирует те же bytes и branch, metadata.force остаётся false. Update сохраняет прежний apply confirmation. Typed hosts получают EvolutionReport через actual executed owner execute_prepared_reporting; registered wire schemas не менялись.
7. Raw configured CLI без current typed authority отказывает, включая scoped update, который раньше мог обойти D1. Legacy unconfigured update остаётся рабочим. Named configured update переиспользует прежнюю installed-package qualification и constraints, не превращаясь в add/install отсутствующего package.

Exposure и намеренные границы
- Positive CLI fixtures реально вызывают commands::reinstall::run_with_capture и commands::update::run_with_intent, а значит normal execute_prepared_reporting и lifecycle/materialization; preparation использует стандартный vibe_package_source::build_install_resolver + bind_request.
- Captures/normalized definitions/default receipts предоставляет trusted Rust host; JSON-флаг или DTO не создаёт authority. Новые raw CLI selection grammar/TTY/machine transport, E3 recovery/removal и F/G не заявляются: это отдельные original tasks. Неподготовленная команда не получает prepared guarantee.
- Существующие D1 fixtures и recording toolkit переиспользованы. Для install actual target — explicit [[test]] m_05_e_2_cases с path=src/lib.rs; lib.test=false сохранён. Это позволяет исполнять полезные существующие cfg(test) fixtures и E2 cases без нового dev-dependency/Cargo.lock edit вне perimeter.
- Orchestrator и CLI используют реальные --lib и --bin targets соответственно. Исходные illustrative --test invocations для этих двух crates НЕ запускались и не объявляются успешными.

Исходные критерии — авторские functional claims
0. Replay/update reports distinguish exact reuse from changed semantic inputs.
   Claimed satisfied: typed reports на реальных prepared command paths возвращают ExactReuse и SemanticInputChange; refusals сохраняют Unavailable/reconfiguration, exact before/after и gaps. Functional examples: unchanged locked replay против default definition evolution с неизменным authored selection.
1. No new option is selected as an incidental solver decision.
   Claimed satisfied: B2 retained intent + exact receipt gates, explicit-empty/dormant preservation, removed/new-required refusal, force/assume-yes negative cases и valid-older regression. Никакой claim здесь не означает Root acceptance.

Build authority и test bindings
- Использован только текущий Root-authorized CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target, absolute Cargo=/home/olegchir/.cargo/bin/cargo.
- CARGO_NET_OFFLINE=true, RUSTUP_AUTO_INSTALL=0, Cargo tests с --offline --locked; umask 0077, fresh private TMPDIR. Toolchain и Cargo.lock не менялись.
- До первой компиляции actual cargo/rustc/rustdoc конфликта на authorized target не наблюдалось. Позже обнаружены три чужих cargo --workspace процесса в других checkout; они не были остановлены, target/env/FD inspection не показал использования нашего authorized target.
- Не запускались workspace/full suites или unrelated targets для discovery. Nonzero case counts ниже — фактические stdout, не TO CREATE declarations.

Финальные основные проверки (15 unique tests)
[
  {
    "name": "install",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-install",
      "--test",
      "m_05_e_2_cases",
      "m_05_e_2_",
      "--",
      "--test-threads=1"
    ],
    "exit": 0,
    "log": "/tmp/m05-e2-report-final-install.log",
    "passed": 3,
    "failed": 0,
    "filtered": 28
  },
  {
    "name": "orchestrator",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-orchestrator",
      "--lib",
      "install::m_05_e_2_cases",
      "--",
      "--test-threads=1"
    ],
    "exit": 0,
    "log": "/tmp/m05-e2-report-final-orchestrator.log",
    "passed": 6,
    "failed": 0,
    "filtered": 660
  },
  {
    "name": "cli",
    "argv": [
      "/home/olegchir/.cargo/bin/cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-cli",
      "--bin",
      "vibe",
      "commands::m_05_e_2_cases",
      "--",
      "--test-threads=1"
    ],
    "exit": 0,
    "log": "/tmp/m05-e2-report-final-cli.log",
    "passed": 6,
    "failed": 0,
    "filtered": 952
  }
]
После последнего уточнения classification отсутствующего retained request дополнительно запущен один exact affected case:
/home/olegchir/.cargo/bin/cargo test --offline --locked -p vibe-orchestrator --lib install::m_05_e_2_cases::prepared_replay_binds_leaf_capture_without_solve -- --exact --test-threads=1
Observed exit 0: 1 passed, 0 failed, 665 filtered out. Это повтор case с дополнительной missing-request assertion, не шестнадцатый unique test.

Check slots
0 — install actual target выше: 3 passed, 0 failed, 28 filtered out.
1 — orchestrator --lib family: 6 passed, 0 failed, 660 filtered out; затем affected exact case 1/0.
2 — CLI --bin vibe family: 6 passed, 0 failed, 952 filtered out.
3 — /home/olegchir/.cargo/bin/cargo fmt --all -- --check: observed exit 1. Семь diff-файлов лежат вне task changes. Это retained known baseline globalfmt failure; unrelated formatting не исправлялось. Список и полный stdout ниже.
4 — git diff --check: observed exit 0.
Дополнительно: XML parse permanent contract — exit 0; rustfmt --check для всех пяти новых Rust files — exit 0 (edition 2024, skip_children=true).

Retained intermediate failures / deviations
- Первый install --no-run после proposed test dependency вернул exit 101: --locked запретил Cargo.lock update. Dependency удалена; Cargo.lock не менялся. Реальный target привязан к src/lib.rs, повторные functional runs nonzero и успешны.
- Ранний CLI --bin vibe --no-run был только compilation check, исполнил 0 tests и не служит доказательством выбора ещё не зарегистрированной family. Позднее созданы/зарегистрированы cases и выполнены реальные nonzero runs.
- Первый orchestrator functional run: 4 cases, 2 passed/2 failed, exit 101. Failures выявили fixture store coordinate collision после намеренной corruption и прежнюю exact-replay проверку в update apply. Исправлены unique fixture coordinates и guarded update admission; первый retry 4/4, расширенная family затем 6/6. Failures не удалены из учёта.
- Оставлены existing compiler warnings; unrelated cleanup не делался.

Оставшееся вне авторских functional claims
- Global fmt остаётся exit 1 на unrelated source files; это не скрытое успешное check.
- Root должен reconcile observed HEAD против supplied attributed source/basis, review patch и самостоятельно решить acceptance/Git. Публикация ниже удостоверяет только content.

Changed source files
crates/vibe-cli/src/commands/mod.rs
crates/vibe-cli/src/commands/reinstall.rs
crates/vibe-cli/src/commands/reinstall/force.rs
crates/vibe-cli/src/commands/reinstall/prepare.rs
crates/vibe-cli/src/commands/reinstall/regenerate.rs
crates/vibe-cli/src/commands/update.rs
crates/vibe-cli/src/commands/update/scoped.rs
crates/vibe-cli/src/commands/update/whole.rs
crates/vibe-install/Cargo.toml
crates/vibe-install/src/apply.rs
crates/vibe-install/src/fetched.rs
crates/vibe-install/src/lib.rs
crates/vibe-install/src/plan.rs
crates/vibe-install/src/record/m_05_d_1_cases.rs
crates/vibe-orchestrator/src/install/mod.rs
crates/vibe-orchestrator/src/lib.rs
vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml
crates/vibe-cli/src/commands/m_05_e_2_cases.rs
crates/vibe-install/src/evolution.rs
crates/vibe-orchestrator/src/install/configured_source.rs
crates/vibe-orchestrator/src/install/m_05_e_2_cases.rs
crates/vibe-orchestrator/src/install/m_05_e_2_fixture.rs

Global fmt baseline files
FMTcrates/vibe-cli/src/commands/mod.rs
crates/vibe-cli/src/commands/reinstall.rs
crates/vibe-cli/src/commands/reinstall/force.rs
crates/vibe-cli/src/commands/reinstall/prepare.rs
crates/vibe-cli/src/commands/reinstall/regenerate.rs
crates/vibe-cli/src/commands/update.rs
crates/vibe-cli/src/commands/update/scoped.rs
crates/vibe-cli/src/commands/update/whole.rs
crates/vibe-install/Cargo.toml
crates/vibe-install/src/apply.rs
crates/vibe-install/src/fetched.rs
crates/vibe-install/src/lib.rs
crates/vibe-install/src/plan.rs
crates/vibe-install/src/record/m_05_d_1_cases.rs
crates/vibe-orchestrator/src/install/mod.rs
crates/vibe-orchestrator/src/lib.rs
vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml
crates/vibe-cli/src/commands/m_05_e_2_cases.rs
crates/vibe-install/src/evolution.rs
crates/vibe-orchestrator/src/install/configured_source.rs
crates/vibe-orchestrator/src/install/m_05_e_2_cases.rs
crates/vibe-orchestrator/src/install/m_05_e_2_fixture.rs

Retained logs (self-contained excerpts)

m05-e2-report-final-install.log
running 3 tests
test record::m_05_d_1_cases::m_05_e_2_exact_capture_replays_induced_edges_and_dormant_choices ... ok
test record::m_05_d_1_cases::m_05_e_2_missing_authority_and_unknown_epoch_refuse ... ok
test record::m_05_d_1_cases::m_05_e_2_update_requires_new_shown_default_receipt ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.28s


m05-e2-report-final-orchestrator.log
running 6 tests
test install::m_05_e_2_cases::force_cannot_admit_changed_payload_or_lock ... ok
test install::m_05_e_2_cases::invalid_newer_definition_does_not_discard_valid_older_candidate ... ok
test install::m_05_e_2_cases::prepared_replay_binds_leaf_capture_without_solve ... ok
test install::m_05_e_2_cases::removed_option_and_new_required_group_return_all_needs_without_writes ... ok
test install::m_05_e_2_cases::update_keeps_explicit_choice_when_default_changes ... ok
test install::m_05_e_2_cases::update_retains_explicit_empty_and_dormant_intent ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 660 filtered out; finished in 0.46s


m05-e2-report-final-cli.log
running 6 tests
test commands::m_05_e_2_cases::force_and_assume_yes_do_not_replace_missing_authority ... ok
test commands::m_05_e_2_cases::legacy_unconfigured_update_keeps_public_route ... ok
test commands::m_05_e_2_cases::prepared_force_repairs_same_payload_and_branch_without_prompt ... ok
test commands::m_05_e_2_cases::prepared_offline_reinstall_reproduces_lock_without_prompts ... ok
test commands::m_05_e_2_cases::prepared_update_still_requires_named_package_to_be_installed ... ok
test commands::m_05_e_2_cases::public_update_uses_standard_source_and_retains_choice ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 952 filtered out; finished in 0.63s


m05-e2-missing-final.log
running 1 test
test install::m_05_e_2_cases::prepared_replay_binds_leaf_capture_without_solve ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 665 filtered out; finished in 0.02s


Initial failed orchestrator run (exit 101)
running 4 tests
test install::m_05_e_2_cases::force_cannot_admit_changed_payload_or_lock ... ok
test install::m_05_e_2_cases::prepared_replay_binds_leaf_capture_without_solve ... FAILED
test install::m_05_e_2_cases::removed_option_and_new_required_group_return_all_needs_without_writes ... ok
test install::m_05_e_2_cases::update_keeps_explicit_choice_when_default_changes ... FAILED

failures:

---- install::m_05_e_2_cases::prepared_replay_binds_leaf_capture_without_solve stdout ----

thread 'install::m_05_e_2_cases::prepared_replay_binds_leaf_capture_without_solve' (3717971) panicked at crates/vibe-orchestrator/src/install/m_05_e_2_fixture.rs:57:122:
called `Result::unwrap()` on an `Err` value: Solve(UnsupportedRoute("reinstall stale: payload/source/graph differs for org.e2fixture/app@1.0.0"))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- install::m_05_e_2_cases::update_keeps_explicit_choice_when_default_changes stdout ----

thread 'install::m_05_e_2_cases::update_keeps_explicit_choice_when_default_changes' (3717976) panicked at crates/vibe-orchestrator/src/install/m_05_e_2_cases.rs:31:87:
called `Result::unwrap()` on an `Err` value: Solve(UnsupportedRoute("configuration lock: resolved choices, closure or selected providers changed; exact replay refused"))


failures:
    install::m_05_e_2_cases::prepared_replay_binds_leaf_capture_without_solve
    install::m_05_e_2_cases::update_keeps_explicit_choice_when_default_changes

test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 660 filtered out; finished in 0.11s

error: test failed, to rerun pass `-p vibe-orchestrator --lib`

Global cargo fmt stdout (exit 1)
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
