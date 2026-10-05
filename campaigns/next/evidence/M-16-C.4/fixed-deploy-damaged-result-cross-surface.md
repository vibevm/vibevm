# M-16-C.4 fixed Deploy damaged result across CLI frame and MCP

An isolated Linux fixture completes one real fixed Deploy through the framed
CLI. After that process exits, the test removes only its terminal result record
from the temporary project. Fresh MCP and CLI frame processes reattach and
inspect the same key. Both return the same run ID and `uncertain`, with partial
observations `intent=absent`, `receipt=present`, and `sidecar=present`. The CLI
frame outcome also reports `effect_state=uncertain`.

Both fresh processes then submit the same key again. MCP returns the retained
typed partial reply, and the CLI frame returns that same reply with
`effect_state=uncertain`. The result remains absent, the pointer and launcher
bytes are unchanged, and the C1 index retains one namespace and one key for
the original run. This proves same-key replay does not retry the effect after
result loss. It establishes post-effect uncertainty when the terminal result
record is lost; it does not model an interruption during the effect.

Exact checks on 2026-10-05: the extended case passed 1/1. The earlier affected
`frame_cross_surface::` family passed 3/3 before this extension. Scoped rustfmt
and `git diff --check` passed after applying rustfmt to one pre-existing stale
assertion in the same test file. The fixture redirects `HOME` and `VIBE_SETTINGS`
to temporary directories and touches no public host.
