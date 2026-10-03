# Fixed Validate trace through CLI MCP stdio

Zap Current revision 11, strategy `strategy.vibevm-next.coarse-c4.1`, names
`next.m16c4.parent` as the C.4 workstream. This bounded source proof uses the
active local M-16-C.4 contract; the strategic card itself does not confer C.4
acceptance.

An explicit `trace=true` requested option reaches `vibe mcp serve
--fixed-validate issue` over real stdio. The client must answer the typed
selection and approval questions; a premature approval is refused before any
script effect. The accepted reference submits one Validate operation. Terminal
inspection retains the same run identity and a report with that run ID,
`trace.status=ok` and `trace.finalised=true`. Replaying the same key and
reference in the same process returns that run and leaves one script effect.

## Exact checks

- `python3 campaigns/packages-2026-09/tasks/chatgpt-cargo.py -- cargo test -p vibe-cli --test cli_mcp_fixed_validate real_stdio_explicit_trace_retains_report_and_replays_one_operation -- --exact`: 1/1, exit 0 (worker run).
- The same wrapper with `cargo test -p vibe-cli --test cli_mcp_fixed_validate`: 5/5, exit 0 (coordinator run, `/tmp/vibevm-next-c4-stdio-trace-family.log`).
- `git diff --check`: exit 0. Only the CLI MCP process test source changed.

This proves the process boundary for one explicit Validate trace option. It
does not authorize a startup default, Deploy profile, other domain/phase,
cross-process accepted-reference replay or full C.4 acceptance. The historical
36 accepted tasks were not revalidated. No full gate panel, routine mutation
testing or public 1.1.0 release ran.
