# M-05-E.4 offline correction: content-only worker report

Both original criteria are unchanged:
1. CLI/service/MCP produce the same request and prepared effect graph.
2. Machine invocation completes or returns typed input requirements without waiting on TTY.

Local disposition: both satisfied as advisory implementation claims, not acceptance. Root owns semantic review, Git, acceptance and mirroring. No Git mutations, delegation, installations, model calls, PodBay/Zap implementation or unrelated source edits were performed.

## Correction and retained work

Review source: /fast/git/v/research/2026-10-17-m05e4-semantic-review/{REPORT.md,RESULT.json}; request_changes, 0 P1 / 1 P2. Existing HEAD is 644bbd0de25e1335e21e513b3228c1a688256bf0. Reused the actual 26-path WIP and existing CLI/service/MCP host boundary. Compared against all 26 previous review hashes: exactly five paths changed; all other 21 paths remain byte-identical. No reset or history reconstruction.

The direct install choice branch now forwards root_offline. The choice adapter loads UserConfig and uses existing output::resolve_offline(root_offline || args.offline, user_config.net.offline), preserving the flag/environment/config ladder. Host Questions publishes the retained InstallRequest.offline. An effective offline client requires a proved true posture and refuses an online or missing posture before Evaluate/Prepare/Apply. The trusted host/source remains immutable; selection requests, prepared effects and application stay with their existing shared owner. An online caller may still use a stricter offline host.

Added one bounded offline mismatch arm to production_install_machine_and_tty. It supplies explicit shown/default consent, the existing direct root offline flag, and an online-posture host with a counting/refusing source. The real CLI adapter returns the expected offline-host error. Source calls remain exactly 0; no vibe.lock or vibedeps appears. The source panics and increments if normalized transaction, solve, metadata, fetch or materialization is reached. Existing compatible JSON missing/default and actual PTY apply arms also run and pass.

Refined M05E4-PREPARED-OPERATIONS to state the offline compatibility rule. No anchor, independent earlier law or status was removed/promoted. Six E4 anchors remain spec/plan. No universal host lifecycle, recovery architecture or alternate resolver was introduced.

Changed continuation paths:
- crates/vibe-cli/src/commands/install/choices.rs
- crates/vibe-cli/src/commands/install/direct.rs
- crates/vibe-orchestrator/src/machine_operation/configuration_choice.rs
- crates/vibe-orchestrator/src/machine_operation/configuration_choice_fixture.rs
- vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml

## Exact fresh checks

Build target idle check: ps -eo pid,args inspected immediately before build; only the inspection shell/rg matched, no active cargo/rustc or writer on /fast/zap-managed/2026-10-14-m10b1-correction/cargo-target. Sequential Cargo builds used that Root-authorized target. No concurrent Vibe writer was introduced.

Environment for Cargo: RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target TMPDIR=/tmp/m05e4-offline.ytAKe6Bp. umask 0077; fresh private TMP under /tmp (mode 1777). Absolute Cargo /home/olegchir/.cargo/bin/cargo, unchanged configured toolchain, --offline --locked -j 2.

- Slot 0: cargo test --offline --locked -j 2 -p vibe-cli --bin vibe commands::install::choices::m_05_e_4_cases -- --nocapture: exit 0, 1 passed, 0 failed, 966 filtered. Includes the new mismatch arm and retained real PTY/JSON arms.
- Slot 1: cargo test --offline --locked -j 2 -p vibe-mcp --lib tools::configuration_choice::m_05_e_4_cases -- --nocapture: exit 0, 2 passed, 0 failed, 219 filtered.
- Slot 2: cargo test --offline --locked -j 2 -p vibe-orchestrator --lib machine_operation::configuration_choice::m_05_e_4_cases -- --nocapture: exit 0, 7 passed, 0 failed, 668 filtered.
- Slot 3: retained wire family: 2 passed, 0 failed, 282 filtered. No new wire/evaluator change, so not rerun. Retained B2 family: 9 passed, 0 failed, 124 filtered; not rerun. Previous report/log bytes embedded verbatim below.
- Slot 4: cargo fmt --all -- --check: exit 1; globalfmt7baselineFailed. Same seven unchanged baseline files listed below. Scoped rustfmt --check --edition 2024 --config skip_children=true on the four changed Rust files: exit 0.
- Slot 5: git diff --check: exit 0. XML parse via Python ElementTree: passed.
- Slot 6: conditional check-codegen not run: no schema, generator or generated registration impact. The continuation changes only CLI/host question projection, fixture and the governing spec.
- Slot 7: conditional global wire-diff not run: no registered codec/format epoch changes. Actual CLI and MCP/service serde response/operation paths were exercised; retained wire tests remain the narrower owner oracle. No global oracle is claimed passing.

Original declared integration targets were TO CREATE proposals. Actual owning targets above are existing binary/library m_05_e_4_cases families, with nonzero counts, not fabricated --test targets. Fresh total 10 passed / 0 failed; retained original 12 E4 + 9 B2 observations remain history, not newly executed or acceptance. No fresh test/build failure occurred; all historical failures remain below.

Baseline global fmt failures (all unchanged against HEAD):
- /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs
- /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs
- /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs
- /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs
- /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs
- /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs
- /fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs

## Limits and unresolved

Global format remains Failed, literal globalfmt7baselineFailed retained. Earlier nonSerialize errors, locked dependency-edge refusal, root-shape mismatches, context destructure and Send+Sync compile failures remain in the original report below. No claim of a clean initial run. Mismatch coverage is one bounded root-offline arm; the other offline inputs use the existing shared resolver in production code, not separate new test arms. No real online acquisition/network experiment was performed or needed. Root independent rereview/acceptance remains external to these local claims.

The aggregate patch includes all 26 current paths, including the four original untracked additions. The report embeds the complete prior report and all new check logs to preserve failures and evidence. Content publication confers no candidate attribution or verified check status.


Full retained report SHA256: 9fd9e9713e22a5ec6e62a36be356c564ded5f078aebe3282648d8e8437d8df84; actual complete bytes remain in the campaign immutable artifact repository. This compact copy omits duplicated historical/raw build logs.
