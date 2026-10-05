# Missing retained Cargo result across CLI frame, local API and MCP

This bounded M-16-C.4 atom starts from clean `71eea02bb` and local plan
revision 430. It uses a disposable fixture and does not rerun the historic
CLI-only fault test or the 36 accepted tasks.

A CLI-issued C1 key completes one fixed Cargo build. The test removes only
that fixture's retained `cargo-result.json`, leaving the run journal and C1
index intact. A direct `FixedCargoRoute::exchange_local` inspect, registered
MCP submit and independently reattached CLI machine frame agree on the same
structured uncertain reply and known run. The CLI outer outcome is Uncertain.
Same-key CLI submit preserves the reply without starting a new build; the C1
index still contains one namespace/key/run. This atom does not alter Cargo's
typed request options or ambient toolchain policy.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_cargo local_route::missing_cargo_result_keeps_uncertain_run_across_cli_local_and_mcp -- --exact`: 1/1 before formatting and 1/1 after scoped formatting; six filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

No PodBay/Zap file, real-user project, full gate panel, mutation panel or
public VibeVM 1.1.0 release was touched. M-16-C.4 remains active.
