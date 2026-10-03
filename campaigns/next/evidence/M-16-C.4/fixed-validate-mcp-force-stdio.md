# Fixed Validate force through CLI MCP stdio

Zap Current revision 11, strategy `strategy.vibevm-next.coarse-c4.1`, names
`next.m16c4.parent`. The active local M-16-C.4 contract supplies this bounded
process proof; C.4 remains active.

An explicit `force=true` requested option reaches a real `vibe mcp serve
--fixed-validate issue` process. A typed approval with `force` changed to false
is refused before the script runs. The original approved option produces one
Validate operation. Its retained admitted input binds `force=true` in both
request and plan while the captured base request remains `force=false`.
Terminal inspection and same-key replay retain one run identity and one script
effect.

## Exact checks

- `cargo test -p vibe-cli --test cli_mcp_fixed_validate real_stdio_explicit_force_keeps_approved_intent_and_one_effect -- --exact`: 1/1, exit 0.
- `cargo test -p vibe-cli --test cli_mcp_fixed_validate`: 6/6, exit 0.
- Both commands ran through the campaign two-slot wrapper with its temporary lock root under `/tmp` and `RUSTC_WRAPPER=` because this sandbox denies writes to the shared slot directory and sccache. The initial normal-wrapper attempt failed before Cargo on `Errno 30`; the first temporary-lock run failed in sccache before the test. Neither is a test result.
- `rustfmt --edition 2024` on the changed test file and `git diff --check`: exit 0.

This proves one explicit per-operation force option through the CLI process
boundary. It does not grant force by startup default, prove Deploy profiles or
complete C.4 surface equivalence. Historical accepted tasks were not
revalidated. No full gate panel, routine mutation testing or public 1.1.0
release ran.
