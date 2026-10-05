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
key.

The same recovery-only host also refuses a different `new-key` on both
surfaces. Both generated errors describe that key as absent, carry no run ID
and report no effect for that requested key. The framed outcome is
`no_effects`; the retained C1 index still contains only `old-key`. This
separates truthful recovery of the old effect from refusal to start a new one.
Neither case models interruption inside `apply`.

Exact checks on 2026-10-05: each extension of the bound
`frame_cross_surface::cli_frame_real_deploy_replays_through_cold_mcp` case
passed 1/1. The fixture redirects `HOME` and `VIBE_SETTINGS` to temporary
directories and touches no public host.
