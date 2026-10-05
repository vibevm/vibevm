# Fixed Validate combined force and trace through MCP stdio

The C.4 process test starts a real CLI-launched MCP server with an explicit
fixed Validate host. Its typed selection requests `force=true` and
`trace=true`. An approval that drops trace is refused before the script runs;
the exact approved pair submits once. The retained admitted request and plan
both carry the pair, while the unchanged v4 capture remains unforced and
untraced. The owner report has the same run ID and a finalized `ok` trace.
After server exit, a new process cold-inspects the same terminal run. The
fixture records exactly one script effect.

## Exact checks

- `cargo test -p vibe-cli --test cli_mcp_fixed_validate combined_options::real_stdio_combined_force_trace_retains_both_options_across_cold_inspect -- --exact`: 1/1, exit 0, with the installed nightly toolchain.
- `rustfmt --check --edition 2024` on the two touched Rust files: exit 0.
- `git diff --check`: exit 0.

The first test attempt never compiled: the pod-local stable compiler rejected
the repository's `-Zthreads=8` option. The exact rerun bound the preinstalled
`/home/olegchir/.rustup` and `/home/olegchir/.cargo` with nightly and passed.
No PodBay/Zap files were changed.

This closes only the combined fixed Validate option interaction at the MCP
process boundary. Deploy profile and other phases/domains remain open; C.4 is
not accepted. The historical 36 accepted tasks were not revalidated. No full
gate panel, routine mutation testing or public 1.1.0 release ran.
