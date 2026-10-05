# M-16-C.4 fixed Deploy cold reattach

The Linux CLI/MCP fixture issued a real `vibe-bin` Deploy through MCP, then
started fresh MCP processes to reattach. The same completed reply and run ID
were retained. After manifest drift, inspect still found the run while submit
was refused. The host grant pointer, launcher bytes, and single submission
namespace remained unchanged.

Exact checks on 2026-10-05:

- `cargo +stable test -p vibe-cli --test cli_mcp_fixed_deploy`: 3 passed.
- `cargo +stable test -p vibe-mcp --lib registered_but_ungranted_deploy_refuses_without_state`: 1 passed.
- `cargo +stable clippy -p vibe-cli --test cli_mcp_fixed_deploy`: exit 0.
- `cargo +stable clippy -p vibe-mcp --lib`: exit 0 (pre-existing warnings).
- `cargo +stable run -p xtask -- specmap`: exit 0; 0 unresolved host edges.

Cargo ran from `/tmp` with the repository manifest, stable toolchain,
`CARGO_TARGET_DIR` at the repository target, and `RUSTFLAGS=-C link-arg=-fuse-ld=mold`
because the inherited parent Cargo config requests nightly-only flags. The
fixture isolated `HOME` and `VIBE_SETTINGS`; no user deployment state was used.
