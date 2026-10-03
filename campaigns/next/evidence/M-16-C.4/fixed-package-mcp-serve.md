# Explicit fixed package MCP host

`vibe mcp serve` now accepts an explicit fixed package `issue` or `reattach`
choice at an absolute project root. It uses the existing package grant service,
pointer, principal, shared policy and C1 namespace used by the local machine
session. The default MCP server has no package grant; a failed reattach does
not issue a replacement namespace. Request bodies still carry only the typed
submit/inspect action and key. No package result or reservation was added to
the transport.

## Exact acceptance evidence

- `cargo test -p vibe-cli --test cli_mcp_fixed_package`: 3/3, exit 0. Two MCP processes issue and reattach to one run/copy; a local machine-frame process can issue the same host pointer before MCP reattaches; relative root, missing issue lifetime and absent pointer refuse before copy.
- `cargo test -p vibe-cli --test mcp_migration_authority_args`: 4/4, exit 0. The default and migration grammar remain available; fixed package mode and lifetime are explicit.
- `cargo test -p vibe-cli --test machine_session mcp_serve_initialize_remains_one_newline_json_rpc_response -- --exact`: 1/1, exit 0. Legacy MCP newline JSON-RPC is unchanged.
- `cargo test -p vibe-cli --bin vibe cli::capability_coverage_tests::matches_native_clap_inventory -- --exact`: 1/1, exit 0.
- `cargo test -p vibe-mcp --lib tools::fixed_package_operation::tests`: 6/6, exit 0, including same-key local API/MCP concurrency followed by retained replay without a second package journal.
- `cargo test -p vibe-mcp --lib capability_coverage_tests::matches_native_mcp_inventory -- --exact` and `install::tests::every_served_tool_is_named_in_the_skill_template -- --exact`: 1/1 each, exit 0.
- `cargo xtask conform check --scope crates/vibe-cli/src/cli/mcp.rs` and `--scope crates/vibe-cli/src/commands/mcp/mod.rs`: 0 findings each, exit 0.
- `cargo clippy -p vibe-cli --bin vibe`: exit 0 with existing warnings.
- `cargo xtask specmap`: one host composition edge added, with 0 suspects and 0 unresolved host edges; existing orphan warnings remain.

The current source-byte delta is
`formats/capability-coverage/fixed-package-mcp-serve-source-delta.json`.
This proves the one fixed package startup route, not general package execution
or complete CLI/MCP/local API equivalence. Other C.4 options, domain owners
and cross-surface races remain. No full panel or public 1.1.0 release ran.
