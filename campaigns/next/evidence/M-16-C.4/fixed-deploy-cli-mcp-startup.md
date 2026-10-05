# Fixed Deploy CLI and MCP startup

The explicit project-local fixed Deploy mode now issues through the CLI
machine frame and reattaches through the registered MCP stdio server using
the same trusted root, profile, policy and retained C1 owner. Reattachment
after manifest, target or profile drift exposes old-key inspection only;
effectful submit is refused. Missing pointers and missing explicit profiles
refuse before a grant is created.

## Exact checks

- `cargo test -p vibe-cli --test cli_mcp_fixed_deploy`: 2/2 passed on the
  final formatted source. The fixture uses isolated HOME and VIBE_SETTINGS.
- `cargo clippy -p vibe-cli --test cli_mcp_fixed_deploy`: exit 0.
- Changed-file `rustfmt --edition 2024 --check` and `git diff --check`: exit 0.

The CLI/MCP startup atom does not close M-16-C.4: complete option, domain and
surface equivalence remains open. No full gate, mutation test, real-user
deployment or public 1.1.0 release ran.
