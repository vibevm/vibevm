# vibe-index — standalone package index utility

`vibe-index` builds and serves an opt-in metadata catalog for one or
more package repos in a vibevm-shaped registry. It runs in two modes:

- **CLI mode** — operates directly on a data directory of index files.
  Suited for scripted reindex invocations, post-publish hooks, CI
  pipelines.
- **Server mode** (`vibe-index serve`) — boots an HTTP server that
  holds the index in RAM, persists every mutation back to disk, and
  exposes a REST API. Single-writer; reads open, writes guarded by
  bearer-token auth.

Specification: [`spec://vibevm/modules/vibe-index/PROP-005`](../../spec/modules/vibe-index/PROP-005-package-index.md).

## Standalone redistribution

This subdirectory is its own Cargo workspace, deliberately outside the
top-level vibevm `crates/` workspace. An org owner who wants to host
their own index server can vendor only `services/vibe-index/` and:

```
cargo install --path .
```

— without pulling the rest of vibevm.

## Status

Slice 1 — skeleton (CLI dispatch, stub subcommands, help-smoke test).
Subsequent slices land per [PROP-005 §4](../../spec/modules/vibe-index/PROP-005-package-index.md#phases).

## Quick start

```sh
vibe-index --help                          # subcommand catalog
vibe-index init   ./vibespecs-index --registry vibespecs --registry-url https://github.com/vibespecs
vibe-index serve  ./vibespecs-index --bind 127.0.0.1:8412
```

(Subcommand bodies land in slices 2+; slice 1 stubs respond with a
"not yet implemented" envelope so the dispatch surface is exercisable
end to end.)

## Licensing

Inherits the project license at the repo root (`../../LICENSE.md`).
When vendoring this subdirectory standalone, copy `LICENSE.md` along
with it.
