# One package C1 key across CLI frame, local route and MCP

This bounded M-16-C.4 atom starts at clean `8843c0c87` and local plan
revision 427. The existing fixed package owner, generated values, separate
CLI grant and CLI/MCP process race are accepted historical work; this test
does not rerun those tasks or change the owner.

A CLI machine-frame process explicitly issues a fixed package grant and
finishes. A new CLI framed process reattaches while a trusted library caller
independently reconstructs the same host through `PackageGrantService` and
`FixedPackageRoute::exchange_local`. A barrier releases their same-key package
submits. Cold CLI frame, direct local route and separate MCP stdio reattach
then inspect or replay the same completed owner reply and run. The pointer
bytes are unchanged, the C1 index has one namespace/key/run, the package
journal has one intent/capsule/result set, and the verified launcher output
has the expected bytes. No caller supplies a root, target, profile, namespace
or destination in the action request.

This tests the three real adapter reaches against the same issued pointer. A
raced caller may receive a temporary lease refusal; it never authorizes a
second copy. Deploy reattach and B-223 are unrelated and unchanged.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_package local_route::cli_frame_and_local_route_race_then_mcp_cold_inspect_one_package_key -- --exact`: 1/1 initially, then five independent exact 1/1 passes after formatting; five other cases filtered out each run.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

No PodBay/Zap file, real-user project or product implementation was changed.
No full gate panel, mutation panel or public VibeVM 1.1.0 release ran.
M-16-C.4 remains open.
