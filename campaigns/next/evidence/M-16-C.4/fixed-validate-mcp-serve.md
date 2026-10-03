# Explicit fixed Validate MCP host

`vibe mcp serve` now accepts a mutually exclusive fixed Validate `issue` or
`reattach` choice at an absolute project root. The CLI supplies the same
physical-root target, project-local principal, versioned policy source and
host-owned C1 pointer used by the local machine session. Captured-script
approval and each control policy require their own explicit startup flag;
absent flags deny them. The default MCP server remains ungranted.

## Exact checks

- `cargo test -p vibe-cli --test cli_mcp_fixed_validate`: 3/3, exit 0. An MCP process can prepare a typed fixed Validate question after explicit issue; reattach shares the host pointer but refuses the old process-local answer and produces a new question. A machine-frame-issued pointer is accepted by MCP. Relative root, missing issue lifetime and missing pointer refuse before provider effect.
- `cargo test -p vibe-cli --test mcp_migration_authority_args`: 6/6, exit 0. The three fixed host modes are mutually exclusive; policy flags require fixed Validate; default/migration grammar remains.
- `cargo test -p vibe-cli --test cli_mcp_fixed_package --test cli_mcp_fixed_cargo`: 3/3 each, exit 0.
- `cargo test -p vibe-cli --test machine_session mcp_serve_initialize_remains_one_newline_json_rpc_response -- --exact`: 1/1, exit 0.
- Exact native clap and MCP coverage inventory cases: 1/1 each, exit 0; MCP skill/template naming: 1/1, exit 0.
- `cargo xtask conform check --scope` each of `crates/vibe-cli/src/cli/mcp.rs`, `crates/vibe-cli/src/commands/mcp/mod.rs` and `crates/vibe-cli/src/commands/mcp/fixed_validate.rs`: 0 findings each, exit 0.
- `cargo clippy -p vibe-cli --bin vibe`: exit 0 with existing warnings.
- `cargo xtask specmap`: one fixed Validate host-composition edge added, 0 suspects, 0 unresolved host edges; existing orphan warnings remain.

`formats/capability-coverage/fixed-validate-mcp-serve-source-delta.json` binds
the current grammar/composition bytes. This atom makes one fixed captured-script
host reachable; it does not restore questions or accepted references after
process loss. Explicit force and trace still require typed decisions; deploy
profiles and other phases remain unavailable. No full panel or public 1.1.0
release ran.
