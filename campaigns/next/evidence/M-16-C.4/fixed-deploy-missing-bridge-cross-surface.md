# M-16-C.4 fixed Deploy missing bridge across CLI frame and MCP

An isolated Linux fixture completes one real fixed Deploy through the framed
CLI, closes that process, then removes only its retained bridge record from the
temporary project. The original result, C1 reservation, pointer and launcher
remain in place. Fresh MCP stdio and CLI frame processes use the old host grant
to inspect and submit the same key. Both actions on both surfaces return the
same `recovery_required` reply and original run ID; the CLI frame marks the
effect `uncertain`.

After these calls the bridge is still absent, the pointer and launcher bytes
are unchanged, and the C1 index still holds one namespace and one key for the
original run. Missing bridge evidence does not license another effect or a new
host grant. This case tests lost retained correlation after completion. It does
not model process interruption inside the provider's `apply` call.

Exact check on 2026-10-05: the bound
`frame_cross_surface::cli_frame_and_mcp_refuse_replay_without_deploy_bridge`
case passed 1/1. The fixture redirects `HOME` and `VIBE_SETTINGS` to temporary
directories and touches no public host.
