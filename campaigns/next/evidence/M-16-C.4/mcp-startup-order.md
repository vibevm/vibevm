# MCP migration authority before fixed host issue

The `vibe mcp serve` startup path now validates the optional manifest migration
authority before any fixed Validate, package or Cargo `issue` can publish its
durable pointer or C1 namespace. Fixed Validate receives its named host policy
facts by value after that shared validation. Default serve follows the same
migration refusal path.

## Exact evidence

- Red test before the fix: `cargo test -p vibe-cli --test cli_mcp_startup_order malformed_migration_authority_precedes_all_fixed_grant_issues -- --exact` exited 101. The first fixed Validate case found `machine-submissions.json` after invalid relative migration-store input.
- The same exact test after the fix: 1/1, exit 0. Its loop covers fixed Validate, package, Cargo and default serve in isolated projects; each refuses malformed migration authority before creating any of the three grant directories or the machine submission pointer.
- Existing process families: `cli_mcp_fixed_validate` 4/4, `cli_mcp_fixed_package` 3/3, `cli_mcp_fixed_cargo` 3/3; `mcp_migration_authority_args` 6/6, all exit 0.
- `cargo xtask conform check --scope` for `commands/mcp/mod.rs` and `commands/mcp/fixed_validate.rs`: 0 findings, exit 0 each. `cargo clippy -p vibe-cli --bin vibe --no-deps`: exit 0 with existing warnings.
- `cargo xtask specmap`: exit 0, 3775 edges, 0 suspects, 0 unresolved host edges; ratchet reports 115 gated orphans. Generated `specmap.json` carries only affected item line/fingerprint changes.
- The two source hashes in `formats/capability-coverage/mcp-startup-order-source-delta.json` match the live files and current inventory rows. Counts remain 258 sources, 247 surfaces, 219 operation identities and 19 registered MCP tools.

This corrects one startup ordering defect. M-16-C.4 remains active: the wider
surface and domain equivalence route is not accepted. No full gate panel or
public 1.1.0 release ran.
