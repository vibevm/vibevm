# R5 native ABI ratification record

This file holds acceptance evidence factored out of the frozen architecture so
that the architecture remains within its authored-file budget.

## R5.3-WIRING acceptance — 2026-08-31

Product commit `332f8e28` and map `48fcb39e` retain every enabled native row in
registry order without premature selector filtering, carry that exact epoch
with its mechanism registry and host routes, and run source-native builds before
authored artifact targets at the complete build fence. Phase and slot
compositions inject the same ARTIFACT-backed native backend and one process-
lifetime loader; compile rows remain build candidates and never enter lifecycle
dispatch. Context roots cross field-exactly, reply status/artifacts/message
retain their wire values, and lifecycle tasks/streams remain empty.

Central review rejected the first worker PASS on two integration defects: a
source-record lookup spawned `cargo -Vv`/`rustc -V`, and loading mutable Cargo or
prebuilt paths into the process cache could retain stale code or lock a Windows
DLL across rebuild. The native `gpt-5.6-sol`/`xhigh` correction leaves the
build-time toolchain digest in the artifact record but makes resolution process-
free, then publishes either admitted origin as a recoverable, non-authoritative,
no-overwrite image at `.vibe/native-load/e1/<sha256>/<basename>`. Link-free
contained parents, copied-byte verification, atomic hard-link publication and
final non-symlink digest validation make the canonical loader key immutable.

A real same-loader A→B source rebuild returns B while A remains loaded; the slot
path proves the same image rule for prebuilt and refuses a missing source record
without creating a slot target tree. Central independently made build order,
loader lifetime, digest identity and production image bypass RED, then restored
all four. Gates pass native 14, handlers 10, mechanism wiring 11, install native
1, strict check/clippy/fmt, conform at lifecycle 2 standing/0 new and
orchestrator/install 0/0. Specmap is 6,833 units / 3,028 tagged items / 2,781
edges with 0 suspects, gated orphans or unresolved host edges and 25 warnings.

## R5.3 parent acceptance — 2026-08-31

Gate commit `be037d77`, trace correction `c2fe99fc` and map `ee3f4b49` add only
integrated test/oracle surface over the accepted ARTIFACT + WIRING machine. One
Windows process composes a real SDK source cdylib and current-platform prebuilt
through production phase dispatch, then reuses the prebuilt digest image through
slot dispatch. The gate also proves Cargo's second scheduled build is fresh
with an unchanged artifact mtime; Linux/macOS selection remains a non-loading
unit oracle; a corrupted config witness refuses before an already-loaded source
handle can answer; compile-native cannot enter lifecycle dispatch; skip
continues, fail stops, and an SDK panic becomes typed failure while the same
process loader later invokes a valid row. Direct slot callbacks prove the
existing pre-install stop and post-install installed-but-flagged semantics
without inventing a stronger rollback claim.

Central reproduced stale-record admission bypass and fail→skip mutations before
acceptance. The generated map then exposed eight new deviations pointed at a
nonexistent external anchor; those edges were rejected and retargeted to the
already resolved ENGINE-CONFORM rules unit before the map commit. Final gates:
native 15, handlers 11, loader 11 unit + 1 real + 3 docs, SDK 7, mechanism
wiring 15, install native 1, strict check/clippy/fmt and conform zero-new. The
gate's five worker plus two central mutations were RED and restored byte-exact.
Specmap is 6,833 units / 3,036 tagged items / 2,789 edges with 0 suspects,
gated orphans or unresolved host edges and 25 warnings. R5.3-ARTIFACT,
R5.3-WIRING and R5.3-GATE are accepted; parent R5.3 is complete.
