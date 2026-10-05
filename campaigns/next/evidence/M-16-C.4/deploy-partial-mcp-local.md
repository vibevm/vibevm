# Fixed Deploy partial truth across MCP and local API

This M-16-C.4 atom exercises the incumbent fixed Deploy owner in temporary
project, settings and home roots. After a completed fixture deployment, the
test removes its retained input capsule. Registered MCP inspection, local
route inspection and same-key MCP submit all return the same known run and
`Uncertain` partial observation, with receipt and sidecar still present.
The C1 index retains one namespace. The test adds no Deploy grant pointer
or cross-process reattach claim.

## Exact checks

- `cargo test -p vibe-mcp --lib tools::fixed_deploy_operation::tests::fixed_deploy_damaged_capsule_keeps_partial_truth_across_mcp_and_local -- --exact --nocapture`: exit 0, 1 passed, 0 failed, 185 filtered out, using the installed nightly toolchain and this worktree's private `target/`.
- File-scoped `rustfmt --edition 2024 --check`: exit 0.
- `git diff --check`: exit 0.

The initial stable-toolchain attempt failed before compilation because the
repository passes nightly `-Zthreads=8`; it was rerun on the installed nightly.
No full gate, mutation panel or public VibeVM 1.1.0 release ran. M-16-C.4
remains active; the owner-decided recovery-only Deploy pointer is separate
work.
