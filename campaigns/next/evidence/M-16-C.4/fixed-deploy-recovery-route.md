# Fixed Deploy recovery-only route

The retained Deploy grant now enters the same surface-neutral generated
request/reply/error route as an explicit recovery-only host. Inspect uses the
old C1 namespace and lifecycle owner. Submit is refused before any owner
dispatch. The refusal first inspects the requested old key: a completed key
publishes the known run and `Completed` effect state, an absent key publishes
`NoEffects`, and unavailable inspection retains uncertainty. The adapter
cannot silently reissue a namespace or turn the old pointer into authority.

## Exact checks

- `cargo test -q -p vibe-orchestrator --lib machine_submission::deploy_grant::tests::recovery_route_rejects_submit_with_retained_effect_truth`: 1/1. Real fixture Deploy, manifest drift, generated inspect, old-key submit/error envelope and fresh-key C1 absence.
- `cargo test -q -p vibe-orchestrator --lib machine_operation::fixed_deploy::tests::`: 3/3 existing route cases.
- `cargo test -q -p vibe-orchestrator --lib machine_submission::deploy_grant::tests::`: 9/9 grant and recovery cases.
- `cargo test -q -p vibe-mcp --lib tools::fixed_deploy_operation::tests::`: 5/5 registered MCP adapter cases.
- Changed-file `rustfmt --edition 2024 --config skip_children=true --check` and `git diff --check`: exit 0.

This is a library route usable by a trusted local or MCP embedding. CLI
startup and registered MCP process composition do not yet issue or reattach
this grant. No full gate, routine mutation panel, real-user deployment or
public VibeVM 1.1.0 release ran. M-16-C.4 remains active.
