# Combined fixed Validate options across CLI frame and MCP processes

This bounded M-16-C.4 adapter proof starts at clean `a4705df22` and local
plan revision 422. The earlier missing-report and other accepted atoms were
not rerun; M-16-C.4 remains active with 36 historical tasks accepted.

A real CLI machine-frame process issues typed selection and approval for
`force=true, trace=true`, submits a scoped key and pauses its captured script
after one counted effect. A distinct CLI-launched MCP stdio process reattaches
to the same host, obtains its own typed approval with the same options and
submits that key. After release, both surfaces inspect the same terminal C1
run, scope, phase, work rows and status. MCP replay returns the same run; a
separately approved `force=true, trace=false` submit under the key is refused.
The admitted request and plan retain both option bits, the retained trace is
finalized `ok` for that run, and the script counter contains one effect.

This is a sequential cross-process contention proof while the CLI provider is
active. It does not claim a barrier-aligned winner-order race. Existing
barrier races for other combinations remain historical evidence; this test
covers the combined-option intersection with actual CLI frame and MCP process
surfaces.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_fixed cross_surface::cli_frame_and_mcp_stdio_combined_options_share_one_run_and_trace -- --exact`: 1/1, exit 0 initially and after the changed-intent assertion and formatting; 17 filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

No product implementation, PodBay/Zap file or real-user project was changed.
No full gate panel, routine mutation testing or public VibeVM 1.1.0 release
ran. General domains, deploy profile and full adapter equivalence remain open.
