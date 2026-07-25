# Module-level specs

<status stage="doc" state="done" action="drift" comment="B2 2026-07-24: index stops at PROP-012 — vibe-mcp/ vibe-cli/ vibe-settings/ vibe-actions/ vibe-progress/ and every PROP after 012 are missing, and the inline Status mentions predate later work (F-016)"/>

- ##modules-purpose Per-crate specifications (PROP / FEAT) land here as work progresses. @doc/done
- ##common-pointer Foundation decisions that cross every crate live in
  [`spec/common/`](../common/). @doc/done
- ##research-pointer Comparative research and threat-model
  backgrounder documents are archived in [`legacy-spec/research/`](../../legacy-spec/research/). @doc/done
- ##design-pointer Non-normative design rationale — the *why* and the lore behind these
  PROPs — lives in [`spec/design/`](../design/); a PROP that has a
  rationale document links to it from its `Related` header. @doc/done

## Index

- ##idx-registry [`vibe-registry/`](vibe-registry/) — registry fetch, cache, resolve. @doc/done
  - ##idx-prop-001 [PROP-001: Git-backed registry](vibe-registry/PROP-001-git-backend.md)
    — shell-out to `git` (not `libgit2`), `GitBackend` trait, cache
    layout, Windows UX. @doc/done
  - ##idx-prop-002 [PROP-002: Decentralized per-package registry](vibe-registry/PROP-002-decentralized-registry.md)
    — per-package repos, `[[registry]]`/`[[mirror]]`/`[[override]]`,
    content-addressed identity, lockfile v2. @doc/done
  - ##idx-prop-008 [PROP-008: Qualified package naming](vibe-registry/PROP-008-qualified-naming.md)
    — mandatory reverse-FQDN `group`, identity tuple
    `(group, name, version, content_hash)`, optional `kind` prefix,
    `naming = "fqdn"` repo names, index-backed short-name resolution,
    collision detection. **Status: DRAFT 2026-05-20.** @doc/done
  - ##idx-prop-010 [PROP-010: Local package cache](vibe-registry/PROP-010-local-package-cache.md)
    — the registry cache elevated to a first-class, machine-global,
    accretive, identity-keyed package store; a `--offline` policy flag,
    offline resolution, a user-level default registry configuration, and
    a `vibe cache` surface — so new modules and new projects resolve
    their dependencies offline. **Status: DRAFT 2026-05-21.** @doc/done
- ##idx-resolver [`vibe-resolver/`](vibe-resolver/) — dep solver, features, subskills. @doc/done
  - ##idx-prop-003 [PROP-003: Dep-model evolution](vibe-resolver/PROP-003-dep-evolution.md)
    — SAT solver via libsolv (BSD-3-Clause), cargo-style features,
    vibevm-native subskills with context-based activation, BCP-47
    sidecar i18n, lockfile v3. **Status: design proposal.** @doc/done
- ##idx-index [`vibe-index/`](vibe-index/) — optional per-org package index + HTTP server. @doc/done
  - ##idx-prop-005 [PROP-005: Optional package index](vibe-index/PROP-005-package-index.md)
    — per-org `<org>/index` git repo with cargo-sparse-style
    `by-name/` + DNF-style `repomd.json` manifest + JSONL primary; the
    `crates/vibe-index/` utility (one binary, two modes — CLI + HTTP
    server); single-writer in-RAM with atomic on-disk persistence;
    full-and-incremental reindex; opt-in everywhere. **Status: implemented (2026-05-22).** @doc/done
- ##idx-workspace [`vibe-workspace/`](vibe-workspace/) — multi-package projects. @doc/done
  - ##idx-prop-007 [PROP-007: Workspace](vibe-workspace/PROP-007-workspace.md)
    — `[workspace] members`, one unified `vibe.toml` (retires
    `vibe-package.toml`), recursive nesting, single lockfile at the
    absolute root, `path`-source cross-member deps, `[workspace.versions]`
    placeholders, selective publish, published-package-repo signalling.
    **Status: DRAFT 2026-05-20.** @doc/done
  - ##idx-prop-009 [PROP-009: Loading model](vibe-workspace/PROP-009-loading-model.md)
    — computed boot composition across a workspace hierarchy: two trees
    (authored `spec/` vs committed `deps/`), the per-node effective boot
    sequence, generated `STATIC.md` / `INDEX.md` artifacts, the
    `inline` / `static` / `dynamic` inclusion types, category-based
    ordering (retires `NN-` prefixes), workspace-aware `vibe install`,
    one computed-view engine for boot and the effective spec. Answers
    PROP-007 §6 question 3. **Status: DRAFT 2026-05-21.** @doc/done
  - ##idx-prop-011 [PROP-011: Incremental install](vibe-workspace/PROP-011-incremental-install.md)
    — refine PROP-009's whole-tree `vibe install` into an incremental
    operation: skip the depsolver when `vibe.lock` is fresh (making
    `vibe install` lockfile-respecting), re-materialise only the changed
    `vibedeps/` slots; boot regeneration stays whole-tree, the cheap
    phase. **Status: DRAFT 2026-05-21.** @doc/done
  - ##idx-prop-012 [PROP-012: Managed redirect block](vibe-workspace/PROP-012-managed-redirect-block.md)
    — vibevm owns only a `<vibevm>`-delimited block of each shared agent
    instruction file (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`), never the
    whole file: exactly one block, a hard stop on a malformed file,
    absent → create. Corrects the destructive whole-file overwrite
    shipped in PROP-009 Phase 4. **Status: DRAFT 2026-05-22.** @doc/done
