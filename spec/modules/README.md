# Module-level specs

Per-crate specifications (PROP / FEAT) land here as work progresses.
Foundation decisions that cross every crate live in
[`spec/common/`](../common/). Comparative research and threat-model
backgrounder documents live in [`spec/research/`](../research/).
Non-normative design rationale — the *why* and the lore behind these
PROPs — lives in [`spec/design/`](../design/); a PROP that has a
rationale document links to it from its `Related` header.

## Index

- [`vibe-registry/`](vibe-registry/) — registry fetch, cache, resolve.
  - [PROP-001: Git-backed registry](vibe-registry/PROP-001-git-backend.md)
    — shell-out to `git` (not `libgit2`), `GitBackend` trait, cache
    layout, Windows UX.
  - [PROP-002: Decentralized per-package registry](vibe-registry/PROP-002-decentralized-registry.md)
    — per-package repos, `[[registry]]`/`[[mirror]]`/`[[override]]`,
    content-addressed identity, lockfile v2.
  - [PROP-008: Qualified package naming](vibe-registry/PROP-008-qualified-naming.md)
    — mandatory reverse-FQDN `group`, identity tuple
    `(group, name, version, content_hash)`, optional `kind` prefix,
    `naming = "fqdn"` repo names, index-backed short-name resolution,
    collision detection. **Status: DRAFT 2026-05-20.**
  - [PROP-010: Local package cache](vibe-registry/PROP-010-local-package-cache.md)
    — the registry cache elevated to a first-class, machine-global,
    accretive, identity-keyed package store; a `--offline` policy flag,
    offline resolution, a user-level default registry configuration, and
    a `vibe cache` surface — so new modules and new projects resolve
    their dependencies offline. **Status: DRAFT 2026-05-21.**
- [`vibe-resolver/`](vibe-resolver/) — dep solver, features, subskills.
  - [PROP-003: Dep-model evolution](vibe-resolver/PROP-003-dep-evolution.md)
    — SAT solver via libsolv (BSD-3-Clause), cargo-style features,
    vibevm-native subskills with context-based activation, BCP-47
    sidecar i18n, lockfile v3. **Status: design proposal.**
- [`vibe-index/`](vibe-index/) — optional per-org package index + HTTP server.
  - [PROP-005: Optional package index](vibe-index/PROP-005-package-index.md)
    — per-org `<org>/index` git repo with cargo-sparse-style `by-name/`
    + DNF-style `repomd.json` manifest + JSONL primary; the
    `crates/vibe-index/` utility (one binary, two modes — CLI + HTTP
    server); single-writer in-RAM with atomic on-disk persistence;
    full-and-incremental reindex; opt-in everywhere. **Status: implemented (2026-05-22).**
- [`vibe-workspace/`](vibe-workspace/) — multi-package projects.
  - [PROP-007: Workspace](vibe-workspace/PROP-007-workspace.md)
    — `[workspace] members`, one unified `vibe.toml` (retires
    `vibe-package.toml`), recursive nesting, single lockfile at the
    absolute root, `path`-source cross-member deps, `[workspace.versions]`
    placeholders, selective publish, published-package-repo signalling.
    **Status: DRAFT 2026-05-20.**
  - [PROP-009: Loading model](vibe-workspace/PROP-009-loading-model.md)
    — computed boot composition across a workspace hierarchy: two trees
    (authored `spec/` vs committed `deps/`), the per-node effective boot
    sequence, generated `INLINE.md` / `INDEX.md` artifacts, the
    `inline` / `static` / `dynamic` inclusion types, category-based
    ordering (retires `NN-` prefixes), workspace-aware `vibe install`,
    one computed-view engine for boot and the effective spec. Answers
    PROP-007 §6 question 3. **Status: DRAFT 2026-05-21.**
  - [PROP-011: Incremental install](vibe-workspace/PROP-011-incremental-install.md)
    — refine PROP-009's whole-tree `vibe install` into an incremental
    operation: skip the depsolver when `vibe.lock` is fresh (making
    `vibe install` lockfile-respecting), re-materialise only the changed
    `vibedeps/` slots; boot regeneration stays whole-tree, the cheap
    phase. **Status: DRAFT 2026-05-21.**
  - [PROP-012: Managed redirect block](vibe-workspace/PROP-012-managed-redirect-block.md)
    — vibevm owns only a `<vibevm>`-delimited block of each shared agent
    instruction file (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`), never the
    whole file: exactly one block, a hard stop on a malformed file,
    absent → create. Corrects the destructive whole-file overwrite
    shipped in PROP-009 Phase 4. **Status: DRAFT 2026-05-22.**
