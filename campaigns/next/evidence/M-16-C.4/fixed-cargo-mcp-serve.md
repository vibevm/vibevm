# Explicit fixed Cargo MCP host

`vibe mcp serve` now accepts a mutually exclusive fixed Cargo `issue` or
`reattach` choice at an absolute project root. It uses the existing
`CargoGrantService`, host pointer, project-local principal, fixed builtin
mechanism plane and C1/build journal. Default MCP serve remains ungranted;
the request cannot select a toolchain, target, provider, root or namespace.
The owner's `ambient-rustup-unproven` warning remains in completed replies.

## Exact checks

- `cargo test -p vibe-cli --test cli_mcp_fixed_cargo`: 3/3, exit 0. MCP issue/reattach preserved one retained build run; a machine-frame-issued grant was reattached by MCP; relative root, missing lifetime and missing pointer refused before build.
- `cargo test -p vibe-cli --test cli_mcp_fixed_package`: 3/3, exit 0. The other fixed host still issues and reattaches independently.
- `cargo test -p vibe-cli --test mcp_migration_authority_args`: 5/5, exit 0. The two effectful grants are mutually exclusive and default/migration grammar remains.
- `cargo test -p vibe-cli --test machine_session mcp_serve_initialize_remains_one_newline_json_rpc_response -- --exact`: 1/1, exit 0.
- Exact native clap and MCP coverage inventory cases: 1/1 each, exit 0; MCP skill/template naming: 1/1, exit 0.
- `cargo xtask conform check --scope crates/vibe-cli/src/cli/mcp.rs` and `--scope crates/vibe-cli/src/commands/mcp/mod.rs`: 0 findings each, exit 0.
- `cargo clippy -p vibe-cli --bin vibe`: exit 0 with existing warnings.
- `cargo xtask specmap`: one host-composition edge added, 0 suspects, 0 unresolved host edges; existing orphan warnings remain.

The first parser compile failed with exit 101 because this clap version has no
`Arg::requires_any`. The supported `ArgGroup` form replaced it; the exact
parser case and affected process tests then passed. The source-byte delta is
`formats/capability-coverage/fixed-cargo-mcp-serve-source-delta.json`.

This proves only the fixed offline Cargo fixture route. Ambient rustup is
unproven, general Cargo remains unavailable, and C.4 still has other typed
options and domain/surface obligations. No full panel or public 1.1.0 release
ran. The B-221 backlog design candidate was committed separately as
`2775e08f6` and was not included in this atom.
