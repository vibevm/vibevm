# M-16-C.4 fixed Deploy drift refusal across CLI frame and MCP

An isolated Linux fixture completes one real fixed Deploy through the framed
CLI. A cold MCP submit with the same key first replays the retained completed
run. The fixture then changes the project manifest, making the old host grant
recovery-only. Fresh MCP and CLI frame processes still inspect the original
completed run, but both refuse a further submit with identical generated error
values.

The refusal names the original run, the `deploy` phase, all three nonempty
scope fields and a completed owner view. `effect_observation=landed`, and the
framed outcome reports a completed effect state; refusal is not rewritten as
no effects. The pointer and launcher bytes are unchanged, and the parsed C1
index is identical before and after the denied calls. This proves
cross-surface recovery-only behavior after source drift for one fixed Deploy
key. It does not model a changed key or interruption inside `apply`.

Exact check on 2026-10-05: the extended
`frame_cross_surface::cli_frame_real_deploy_replays_through_cold_mcp` case
passed 1/1. The fixture redirects `HOME` and `VIBE_SETTINGS` to temporary
directories and touches no public host.
