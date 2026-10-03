# Fixed package MCP registration

The default MCP factory now registers `fixed_package_operation`. Its descriptor
accepts only the generated epoch-one submit/inspect action and key. A normal
context has no package grant; an embedding must supply an existing
`FixedPackageRoute` with its physical root and host-issued C1 scope. The adapter
passes the closed action to that route and publishes the generated reply or
error without storing a second result or reservation.

## Exact checks on this atom

- `cargo test -p vibe-mcp --lib tools::fixed_package_operation::tests`: 5/5, exit 0. No-grant, malformed and oversized requests refused before copy; same-key MCP replay retained one run; missing output stayed incomplete; the direct library route and MCP observed the same run.
- `cargo test -p vibe-orchestrator --lib machine_operation::fixed_package::native_tests`: 13/13, exit 0. Owner route refusal, partial publication and cold replay remained green.
- `cargo test -p vibe-mcp --lib capability_coverage_tests::matches_native_mcp_inventory -- --exact`: 1/1, exit 0.
- `cargo test -p vibe-mcp --lib install::tests::every_served_tool_is_named_in_the_skill_template -- --exact`: 1/1, exit 0.
- `cargo test -p vibe-mcp --doc fixed_package_operation`: 1/1, exit 0.
- `cargo clippy -p vibe-mcp -p vibe-orchestrator --lib`: exit 0 with existing warnings.
- `cargo xtask conform check --scope crates/vibe-mcp/src/tools/fixed_package_operation.rs`: 0 new findings, exit 0; the matching dispatch-file scope also had 0 new findings. Crate-wide `vibe-mcp` conform exited 1 with 80 findings, including one introduced missing doctest; that one was fixed before the file-scoped rerun. The other crate-wide findings were in existing files and were not changed here.
- `cargo xtask specmap`: one source edge added, 0 suspects, 0 unresolved host edges; existing gated orphans remain.

`formats/capability-coverage/fixed-package-operation-source-delta.json` binds
the changed registration/adapter/dispatch bytes. The source inventory and
descriptor are observations, never host authority.

This atom registers one fixed package MCP route. C.4 remains active: general
domain owner routing, remaining typed options, complete CLI/MCP/local API
equivalence and cross-surface concurrent admission are still open. No full gate,
public release or native Windows/macOS claim was made.
