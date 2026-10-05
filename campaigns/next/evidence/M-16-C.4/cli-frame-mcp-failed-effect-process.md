# Failed post-effect Validate truth across CLI frame and MCP processes

This bounded M-16-C.4 atom starts from clean `e66cc12fe` and local plan
revision 423. The historical 36 accepted tasks and prior single-surface
failure proof were not rerun; C.4 remains active.

A real CLI machine-frame process issues a typed fixed Validate approval and
starts a captured script. The script writes once, waits at a fixture
boundary, then exits 17. A distinct CLI-launched MCP stdio process reattaches,
obtains its own typed approval and submits the same scoped key while the CLI
provider is active. After release, both inspect the same C1 run, terminal
`Failed/Uncertain` status, partial flag, `validate/fail` phase, failed work row
and scope. MCP same-key replay returns that run. The C1 index has one key/run
and the script counter contains exactly one effect.

This extends the earlier MCP-only failure result to two real process adapters.
The provider wait gives an active same-key contender; it is not a
barrier-aligned winner-order race. The final report exists, so missing-report
recovery is a separate accepted atom.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_fixed cross_surface::cli_frame_and_mcp_stdio_share_failed_post_effect_truth -- --exact`: 1/1, exit 0 before and after scoped formatting; 18 filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

No product implementation, PodBay/Zap file or real-user project was changed.
No full gate panel, routine mutation testing or public VibeVM 1.1.0 release
ran. General domains, deploy profiles and full adapter equivalence remain
open.
