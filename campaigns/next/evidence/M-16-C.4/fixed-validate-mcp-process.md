# Fixed Validate MCP process admission proof

A real `vibe mcp serve` stdio process now has a focused oracle for the
complete fixed Validate path: generated prepare question, exact selection
answer, exact approval answer, accepted reference, same-key submit, retained
owner run, terminal result and cold inspection from a second MCP process.
The script writes one effect marker, and the cold reader finds the same run
without a second write. The request supplies force, trace, deploy profile
and control fields explicitly; no TTY or prose decision is involved.

The first test version failed with exit 101 because it assumed an asynchronous
`Starting` reply already had a run ID. The owner is allowed to assign/report
that identity later. The corrected oracle waits for `inspect` to expose the
run and terminal result under a 20-second bound, then checks that the cold
process sees the same identity and the effect count remains one.

- `cargo test -p vibe-cli --test cli_mcp_fixed_validate real_stdio_decision_submit_and_cold_inspect_retain_one_effect -- --exact`: 1/1, exit 0 after the correction.
- `cargo test -p vibe-cli --test cli_mcp_fixed_validate`: 4/4, exit 0.
- `cargo test -p vibe-mcp --lib capability_coverage_tests::matches_native_mcp_inventory -- --exact`: 1/1, exit 0 after adding the named case to the coverage row.

No product runtime or generated wire changed. This proves one fixed script
profile through actual MCP process admission and replay, not the remaining
phases, deploy profiles or general domain owners. No full panel or public
1.1.0 release ran.
