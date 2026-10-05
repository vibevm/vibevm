# M-16-C.4 fixed Deploy damaged result across CLI frame and MCP

An isolated Linux fixture completes one real fixed Deploy through the framed
CLI. After that process exits, the test removes only its terminal result record
from the temporary project. Fresh MCP and CLI frame processes reattach and
inspect the same key. Both return the same run ID and `uncertain`, with partial
observations `intent=absent`, `receipt=present`, and `sidecar=present`. The CLI
frame outcome also reports `effect_state=uncertain`.

After inspection the result remains absent, the pointer and launcher bytes are
unchanged, and the C1 index retains one namespace and one key for the original
run. Only inspect actions follow the damage, so this fixture never retries the
effect. It establishes post-effect uncertainty when the terminal result record
is lost; it does not model an interruption during the effect.

Exact checks on 2026-10-05: the new case passed 1/1; the affected
`frame_cross_surface::` family passed 3/3. Scoped rustfmt and `git diff --check`
passed. The fixture redirects `HOME` and `VIBE_SETTINGS` to temporary
directories and touches no public host.
