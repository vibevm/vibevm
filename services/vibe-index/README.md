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

PROP-005 slices 1–8 are implemented — the full CLI (`init` / `reindex`
/ `add` / `remove` / `get` / `list` / `search` / `capabilities` /
`purls` / `outdated` / `verify` / `dump`) and the read + write HTTP
server (`serve`), fed by `reindex --from-clones` or `--from-github`.
The consumer-side integration (PROP-005 slices 9–10) lives in the main
`crates/` workspace. Slice plan: [PROP-005 §4](../../spec/modules/vibe-index/PROP-005-package-index.md#phases).

## Quick start

```sh
vibe-index --help                          # subcommand catalog
vibe-index init   ./vibespecs-index --registry vibespecs --registry-url https://github.com/vibespecs
vibe-index serve  ./vibespecs-index --bind 127.0.0.1:8412
```

Most consumers read the static index files over raw HTTP (or a git
clone of the `index` repo); the `serve` mode is for orgs that need
real-time, publish-time index updates.

## Licensing

Inherits the project license at the repo root (`../../LICENSE.md`).
When vendoring this subdirectory standalone, copy `LICENSE.md` along
with it.
