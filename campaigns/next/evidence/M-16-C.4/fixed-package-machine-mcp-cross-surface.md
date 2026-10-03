# Fixed package machine frame to MCP replay

Zap Current revision 11, strategy `strategy.vibevm-next.coarse-c4.1`, names
`next.m16c4.parent`. The active local M-16-C.4 contract supplies this bounded
cross-surface test; its strategy card does not itself grant C.4 acceptance.

A real machine-frame process issues the fixed package host and submits one
scoped key. After that process exits, a separate CLI-launched MCP server
reattaches and inspects the same key; another MCP process submits it again.
Both responses contain the complete retained owner reply and identical run
identity from the machine frame. The host pointer is byte-identical, one C1
namespace remains, the package operation journal has one intent/capsule/result
set, and the fixture output bytes are unchanged.

## Exact checks

- `python3 campaigns/packages-2026-09/tasks/chatgpt-cargo.py -- cargo test -p vibe-cli --test machine_session_package machine_submit_then_mcp_reattach_reuses_same_package_operation -- --exact`: 1/1, exit 0 (worker run).
- The same wrapper with `cargo test -p vibe-cli --test machine_session_package`: 4/4, exit 0 (coordinator run, `/tmp/vibevm-next-c4-package-cross-surface-family.log`).
- `git diff --check`: exit 0. Only the package machine-session process test source changed.

This is sequential cross-surface replay for one fixed package intent. It does
not prove simultaneous competing submissions, a different package intent,
general package execution or all C.4 surfaces. The historical 36 accepted
tasks were not revalidated. No full gate panel, routine mutation testing or
public 1.1.0 release ran.
