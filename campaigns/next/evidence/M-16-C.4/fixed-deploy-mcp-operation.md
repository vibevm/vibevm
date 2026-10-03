# Explicit fixed Deploy MCP route

The default MCP factory now registers `fixed_deploy_operation`, a closed
submit/inspect adapter over the existing `FixedDeployRoute`. A normal context
has no Deploy grant. A trusted embedding must supply a route that already
holds separate project, settings and user-home roots, an authored fixed
vibe-bin target/profile, current generation observer and a host-issued C1
namespace. The request supplies only an action and key. The lifecycle Deploy
journal remains the sole reservation and result owner.

The `list_tools` cell moved unchanged into its own module so the registration
file stays below its 600-line limit. No semantic tool collection logic moved
into this new Deploy adapter.

## Exact checks

- `cargo test -p vibe-mcp --lib tools::fixed_deploy_operation::tests`: 3/3, exit 0. Ungranted registration created no owner state; forged profile/settings arguments refused before dispatch; an actual temp-root bound deployment completed and cold same-key inspection reused the owner run.
- `cargo test -p vibe-orchestrator --lib machine_operation::fixed_deploy::tests`: 3/3, exit 0. Existing root/wire refusal, completed receipt and partial-effect projection remained green.
- `cargo test -p vibe-mcp --lib context::tests::context_carries_exactly_public_roots_and_private_lifecycle_authority -- --exact`: 1/1, exit 0.
- `cargo test -p vibe-mcp --doc fixed_deploy_operation` and `--doc list_tools`: 1/1 each, exit 0. The touched Deploy dispatch's two canonical-use doctests passed 2/2.
- Native MCP coverage and installed skill/template naming: 1/1 each, exit 0.
- File-scoped conform on `fixed_deploy_operation.rs`, `list_tools.rs`, `context.rs`, `tools.rs` and the corrected Deploy dispatch: 0 findings each, exit 0.
- `cargo clippy -p vibe-mcp -p vibe-orchestrator --lib`: exit 0 with existing warnings.
- `cargo xtask specmap`: current map regenerated with 0 suspects and 0 unresolved host edges; existing orphan warnings remain.

The first fixture compile failed with exit 101 because `ExtensionConfig` does
not implement `FromStr`; the fixture now uses its existing `from_table`
constructor. The first scoped conform run on Deploy dispatch found two
pre-existing public types without doctests; both examples were added and
compiled before the green rerun.

The current source bytes are bound in
`formats/capability-coverage/fixed-deploy-mcp-operation-source-delta.json`.
This atom does not grant Deploy to ordinary `vibe mcp serve` or decide durable
reattach. B-223 (the recovery-only fixed Deploy grant choice) remains open.
No full panel or public 1.1.0 release ran.
