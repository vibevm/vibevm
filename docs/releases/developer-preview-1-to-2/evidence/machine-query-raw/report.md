# Query adapter strictness check

The existing MCP `query` tool now requires an argument object with only `uri`, `symbol`, `kind` (strings when present) and `limit` (integer 1–200 when present). Null, wrong types, floating-point forms including 3.0, negative/zero/too-large limits and extra keys produce `ToolError::InvalidArguments` before the trace map is built. Omission still uses the trace owner's default 50. Valid filters and their semantics remain delegated to `vibe_trace::search`.

The one selected `vibe-mcp --lib tools::query::tests::` run exited **0**, 2 passed, 0 failed, 84 filtered. The malformed test invoked the real tool with a nonexistent temporary project path and required pre-dispatch `InvalidArguments`. The valid test called only the pure argument checker and asserted omission/default and all selected fields. Scoped rustfmt check exited **0**. Exact command, environment, source/log hashes and raw output are in `result.json` and `family.log`; no broader MCP or mutation test ran.

This closes the legacy adapter's advertised-shape mismatch only. It does not establish a universal JSON Schema interpreter, generated machine-service validation, native discovery or B.1 acceptance. No Git, public, provider or user-state operation ran; Root retains review and acceptance.
