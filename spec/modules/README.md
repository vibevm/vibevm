# Module-level specs

Per-crate specifications (PROP / FEAT) land here as work progresses.
Foundation decisions that cross every crate live in
[`spec/common/`](../common/). Comparative research and threat-model
backgrounder documents live in [`spec/research/`](../research/).

## Index

- [`vibe-registry/`](vibe-registry/) — registry fetch, cache, resolve.
  - [PROP-001: Git-backed registry](vibe-registry/PROP-001-git-backend.md)
    — shell-out to `git` (not `libgit2`), `GitBackend` trait, cache
    layout, Windows UX.
  - [PROP-002: Decentralized per-package registry](vibe-registry/PROP-002-decentralized-registry.md)
    — per-package repos, `[[registry]]`/`[[mirror]]`/`[[override]]`,
    content-addressed identity, lockfile v2.
- [`vibe-resolver/`](vibe-resolver/) — dep solver, features, subskills.
  - [PROP-003: Dep-model evolution](vibe-resolver/PROP-003-dep-evolution.md)
    — SAT solver via libsolv (BSD-3-Clause), cargo-style features,
    vibevm-native subskills with context-based activation, BCP-47
    sidecar i18n, lockfile v3. **Status: design proposal.**
