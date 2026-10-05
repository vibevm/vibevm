# One fixed Cargo C1 key across CLI frame, local route and MCP

This bounded M-16-C.4 atom starts from clean `e2d39e235` and local plan
revision 428. It exercises an existing fixed offline Cargo owner; no general
build, arbitrary command or hermetic toolchain capability is claimed.

A CLI machine-frame process explicitly issues the fixed Cargo grant. A new CLI
framed process and a trusted direct `FixedCargoRoute::exchange_local` caller
reattach that same host, then race a shared key. Cold CLI frame, direct local
route and registered MCP inspect/replay one completed result and C1 run. The
pointer bytes stay fixed, the C1 index has one namespace/key/run, the build
journal retains one Cargo result, and the exact output exists. The typed
result keeps the existing ambient-rustup warning. A raced contender may see a
lease refusal; cold replay cannot dispatch a second build.

This is a separate direct-local-API intersection beyond the accepted
CLI/registered-MCP in-process race and CLI-launched MCP sequential reach. It
uses only a disposable fixed Cargo fixture and does not touch Deploy B-223.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_cargo local_route::cli_frame_and_direct_local_route_share_one_cargo_run_then_mcp_inspects -- --exact`: 1/1 initially and three independent exact 1/1 passes after formatting; five other cases filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

No PodBay/Zap file, real-user project or product implementation was changed.
The historical 36 accepted tasks were not rerun. No full gate panel,
mutation panel or public VibeVM 1.1.0 release ran; M-16-C.4 remains active.
