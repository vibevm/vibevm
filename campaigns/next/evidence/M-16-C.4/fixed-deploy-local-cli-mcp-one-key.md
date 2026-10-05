# Fixed Deploy: local API, CLI and MCP share one C1 key

The isolated Linux fixture issues a project-local fixed Deploy grant through
the CLI. A separate trusted local API client reconstructs the same held roots,
authored target, scope and policy source, reattaches the grant, and submits
`one-key`. Fresh MCP stdio and CLI machine-frame processes then submit that
same key. They return the identical completed owner reply and run ID. A cold
local inspection returns the same reply. The C1 index has one namespace and
one key; the grant pointer and launcher bytes remain unchanged after replay.

Bound check:
`cargo +nightly test -p vibe-cli --test cli_mcp_fixed_deploy local_route::cli_mcp_and_local_api_replay_one_fixed_deploy_result -- --exact`
passed 1/1 on Linux. This proves a completed fixed Deploy effect across those
three clients. Interruption during the effect, other domains and the full C.4
acceptance remain open. No full gate, mutation suite or publication ran.
