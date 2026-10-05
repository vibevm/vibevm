# Fixed Deploy partial truth across local API, MCP and CLI

An isolated MCP stdio process issues the trusted project-local grant and
completes one `partial-key` Deploy effect. The fixture removes only that run's
terminal result record. A cold trusted local API route inspects and submits
the key, then separate MCP stdio and CLI machine-frame processes submit it.
All return the same `Uncertain` reply with the original run ID, absent intent,
present receipt and present sidecar. The CLI outer effect state is uncertain.
No path recreates the missing result or launcher, and the C1 index still has
one namespace and key. The pointer and launcher bytes do not change.

Bound check:
`cargo +nightly test -p vibe-cli --test cli_mcp_fixed_deploy local_route::damaged_deploy_result_has_same_partial_truth_in_local_cli_and_mcp -- --exact`
passed 1/1 on Linux. This is a post-effect record-loss case; an interruption
during application remains open. The full C.4 acceptance remains open.
