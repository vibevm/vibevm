# Fixed Deploy over the bounded local machine frame

An explicitly granted `ServerContext` now opens a fixed Deploy machine
session. The session exposes one trusted context and target, generated Deploy
request/reply/error formats, and the incumbent byte framing. Active submit
uses the retained owner and publishes its actual run/effect outcome. A
recovery-only route permits old-key inspection and refuses submit, including
fresh keys, while preserving known completed effects in the typed refusal.
Neither the frame nor the request body can issue a Deploy grant.

## Exact checks

- `cargo test -p vibe-mcp --lib machine_session::deploy::tests::`: 3/3 passed
  after the final code changes. The cases cover active submit and inspect of
  the same run, retained completed inspection, and recovery-only refusal with
  completed versus absent key outcomes.
- `cargo test -p vibe-mcp --doc machine_session::deploy`: 2/2 passed for
  explicit-host construction and unbound-context refusal.
- `cargo clippy -p vibe-mcp --lib`: exit 0; no warning in the new Deploy runner.
- Changed-file `rustfmt --edition 2024 --config skip_children=true --check`
  and `git diff --check`: exit 0.
- `cargo xtask specmap`: exit 0, generated 8985 spec units, 4474 tagged code
  items and 3782 edges, with 0 unresolved host edges. This refresh also
  records earlier accepted Deploy grant citations absent from the prior map.
- `cargo xtask conform check --scope crates/vibe-mcp`: exit 1 with 84 existing
  findings elsewhere in the crate and 0 in the new Deploy files. The initial
  run identified two new public-error documentation findings; both were
  removed before the final scoped rerun.

This is a library-callable local frame. CLI startup and registered MCP
process composition do not yet issue or reattach the fixed Deploy grant, so
cross-process Deploy equivalence remains open in M-16-C.4. No full panel,
routine mutation test, real-user deployment or public VibeVM 1.1.0 release ran.
