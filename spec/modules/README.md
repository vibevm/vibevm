# Module-level specs

Per-crate specifications (PROP / FEAT) land here as work progresses.
Foundation decisions that cross every crate live in
[`spec/common/`](../common/).

## Index

- [`vibe-registry/`](vibe-registry/) — registry fetch, cache, resolve.
  - [PROP-001: Git-backed registry](vibe-registry/PROP-001-git-backend.md)
    — shell-out to `git` (not `libgit2`), `GitBackend` trait, cache
    layout, Windows UX.
