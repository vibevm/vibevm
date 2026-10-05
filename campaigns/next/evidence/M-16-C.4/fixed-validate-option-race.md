# Fixed Validate option identity across local and MCP submit race

Two independently approved `force=true, trace=true` Fixed Validate plans race
the same scoped key through the local framed API and registered MCP tool. Both
submits succeed, and subsequent inspection reaches one C1 run and one counted
script effect. A separately approved `force=true, trace=false` plan with that
key is refused while the first run is live. After release, both surfaces
inspect the same retained terminal status and work outcomes; the run report
contains the finalized trace. The C1 reservation retains that run ID.

The initial exact run exposed a partial submit response without a run ID. The
test now obtains identity from inspect after the fixture enters, matching the
existing partial response contract. It does not require a complete submit
response before the owner finishes.

## Exact checks

- `cargo test -q -p vibe-mcp --lib machine_session::fixed::tests::option_race::framed_and_mcp_combined_option_race_share_c1_and_refuse_changed_options -- --exact`: three independent 1/1 runs, exit 0 each, using the installed nightly toolchain and this worktree's private target.
- `rustfmt --check --edition 2024 --config skip_children=true` on the two touched Rust files: exit 0.
- `git diff --check`: exit 0.

This is a fixture race between two in-process adapters, not a second CLI
process-boundary run. It does not claim complete C.4 surface equivalence or
exercise deploy profile, other phases or general domains. No full gate,
mutation panel or public release ran.
