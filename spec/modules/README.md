# Module-level specs

<status stage="doc" state="done" comment="D d2d 2026-07-25: index completed to the live tree (all nine module dirs, every PROP) and the inline Status mentions refreshed against Phase C verdicts"/>

- @fact:modules-purpose Per-crate specifications (PROP / FEAT) land here as work progresses. @status:doc/done
- @fact:common-pointer Foundation decisions that cross every crate live in
  [`spec/common/`](../common/). @status:doc/done
- @fact:research-pointer Comparative research and threat-model
  backgrounder documents are archived in [`legacy-spec/research/`](../../legacy-spec/research/). @status:doc/done
- @fact:design-pointer Non-normative design rationale — the *why* and the lore behind these
  PROPs — lives in [`spec/design/`](../design/); a PROP that has a
  rationale document links to it from its `Related` header. @status:doc/done

## Index

- @fact:idx-registry [`vibe-registry/`](vibe-registry/) — registry fetch, cache, resolve. @status:doc/done
  - @fact:idx-prop-001 [PROP-001: Git-backed registry](vibe-registry/PROP-001-git-backend.md)
    — shell-out to `git` (not `libgit2`), `GitBackend` trait, cache
    layout, Windows UX. @status:doc/done
  - @fact:idx-prop-002 [PROP-002: Decentralized per-package registry](vibe-registry/PROP-002-decentralized-registry.md)
    — per-package repos, `[[registry]]`/`[[mirror]]`/`[[override]]`,
    content-addressed identity, lockfile v2. @status:doc/done
  - @fact:idx-prop-008 [PROP-008: Qualified package naming](vibe-registry/PROP-008-qualified-naming.md)
    — mandatory reverse-FQDN `group`, identity tuple
    `(group, name, version, content_hash)`, optional `kind` prefix,
    `naming = "fqdn"` repo names, index-backed short-name resolution,
    collision detection. **Status: IMPLEMENTED (M1.18 + M1.19).** @status:doc/done
  - @fact:idx-prop-010 [PROP-010: Local package cache](vibe-registry/PROP-010-local-package-cache.md)
    — the registry cache elevated to a first-class, machine-global,
    accretive, identity-keyed package store; a `--offline` policy flag,
    offline resolution, a user-level default registry configuration, and
    a `vibe cache` surface — so new modules and new projects resolve
    their dependencies offline. **Status: DRAFT — the store is future work;
    the `--offline` half shipped separately (PROP-002 §2.2.2.1).** @status:doc/done
  - @fact:idx-prop-021 [PROP-021: Submodule sources](vibe-registry/PROP-021-submodule-sources.md)
    — git submodules as an embedding form for a package: recursive clone
    and update, snapshot embedding, the in-place native form,
    `resolved_commit` reproducibility. **Status: IMPLEMENTED.** @status:doc/done
  - @fact:idx-prop-023 [PROP-023: Bridge packages](vibe-registry/PROP-023-bridge-packages.md)
    — the umbrella of the bridge four: `[package].bridge` plus the
    composition of install hooks (PROP-020), submodule sources (PROP-021)
    and materialization modes (PROP-022). **Status: IMPLEMENTED.** @status:doc/done
  - @fact:idx-prop-030 [PROP-030: The embedded registry](vibe-registry/PROP-030-embedded-registry.md)
    — the in-tree `packages/` of a source-installed `vibe` as an ambient
    default registry: the origin seam, project-local packages, the
    precedence inversion between developer and end user, the CI-off gate.
    **Status: IMPLEMENTED.** @status:doc/done
- @fact:idx-resolver [`vibe-resolver/`](vibe-resolver/) — dep solver, features, subskills. @status:doc/done
  - @fact:idx-prop-003 [PROP-003: Dep-model evolution](vibe-resolver/PROP-003-dep-evolution.md)
    — cargo-style features, vibevm-native subskills with context-based
    activation, BCP-47 sidecar i18n, the lockfile records. The SAT-engine
    sections are superseded by PROP-017 (resolvo shipped, not libsolv) and
    the live lockfile is v5. **Status: vocabulary IMPLEMENTED; engine
    superseded by PROP-017.** @status:doc/done
  - @fact:idx-prop-017 [PROP-017: Resolvo as the production resolver](vibe-resolver/PROP-017-resolvo-resolver.md)
    — the engine decision that reversed PROP-003 §2.2: pure-Rust resolvo
    instead of libsolv-via-FFI, shipped as the production default.
    **Status: IMPLEMENTED (the port is complete).** @status:doc/done
- @fact:idx-index [`vibe-index/`](vibe-index/) — optional per-org package index + HTTP server. @status:doc/done
  - @fact:idx-prop-005 [PROP-005: Optional package index](vibe-index/PROP-005-package-index.md)
    — per-org `<org>/index` git repo with cargo-sparse-style
    `by-name/` + DNF-style `repomd.json` manifest + JSONL primary; the
    `crates/vibe-index/` utility (one binary, two modes — CLI + HTTP
    server); single-writer in-RAM with atomic on-disk persistence;
    full-and-incremental reindex; opt-in everywhere. **Status: implemented (2026-05-22).** @status:doc/done
- @fact:idx-workspace [`vibe-workspace/`](vibe-workspace/) — multi-package projects. @status:doc/done
  - @fact:idx-prop-007 [PROP-007: Workspace](vibe-workspace/PROP-007-workspace.md)
    — `[workspace] members`, one unified `vibe.toml` (retires
    `vibe-package.toml`), recursive nesting, single lockfile at the
    absolute root, `path`-source cross-member deps, `[workspace.versions]`
    placeholders, selective publish, published-package-repo signalling.
    **Status: IMPLEMENTED (M1.17; workspace-aware install landed in M1.18).** @status:doc/done
  - @fact:idx-prop-009 [PROP-009: Loading model](vibe-workspace/PROP-009-loading-model.md)
    — computed boot composition across a workspace hierarchy: two trees
    (authored `spec/` vs committed `vibedeps/`), the per-node effective boot
    sequence, generated `STATIC.md` / `INDEX.md` artifacts, the
    `static` / `dynamic` inclusion types (renamed 2026-07-16), category-based
    ordering (retires `NN-` prefixes), workspace-aware `vibe install`,
    one computed-view engine for boot and the effective spec. Answers
    PROP-007 §6 question 3. **Status: IMPLEMENTED (M1.18 phases 1–7);
    phase 8's engine-backed effective-spec view is v1.5.** @status:doc/done
  - @fact:idx-prop-011 [PROP-011: Incremental install](vibe-workspace/PROP-011-incremental-install.md)
    — refine PROP-009's whole-tree `vibe install` into an incremental
    operation: skip the depsolver when `vibe.lock` is fresh (making
    `vibe install` lockfile-respecting), re-materialise only the changed
    `vibedeps/` slots; boot regeneration stays whole-tree, the cheap
    phase. **Status: SHIPPED 2026-05-22.** @status:doc/done
  - @fact:idx-prop-012 [PROP-012: Managed redirect block](vibe-workspace/PROP-012-managed-redirect-block.md)
    — vibevm owns only a `<vibevm>`-delimited block of each shared agent
    instruction file (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`), never the
    whole file: exactly one block, a hard stop on a malformed file,
    absent → create. Corrects the destructive whole-file overwrite
    shipped in PROP-009 Phase 4. **Status: IMPLEMENTED (M1.18 Phase 7).** @status:doc/done
  - @fact:idx-prop-020 [PROP-020: Install hooks](vibe-workspace/PROP-020-install-hooks.md)
    — `[hooks]` in a package manifest, the pre/post phases, the
    interpreter selection, the trust gate and the hook environment.
    **Status: IMPLEMENTED.** @status:doc/done
  - @fact:idx-prop-022 [PROP-022: Materialization modes](vibe-workspace/PROP-022-materialization-modes.md)
    — snapshot (default) vs in-place materialization as a property of any
    package, the hardlink machinery, the destructive guard.
    **Status: IMPLEMENTED.** @status:doc/done
  - @fact:idx-prop-025 [PROP-025: vibe-native binary delivery](vibe-workspace/PROP-025-binary-delivery.md)
    — code-bearing packages ship runnable tools; `vibe bin build` /
    `bin exec` dispatch through the lockfile. **Status: v1 IMPLEMENTED
    (§§2–5); §6–§7 are specified v2 surface.** @status:doc/done
  - @fact:idx-prop-034 [PROP-034: Transitive links and the static boot graph](vibe-workspace/PROP-034-transitive-links-boot-graph.md)
    — `static-transitive`, dedup, topological ordering and tie-break
    emission. **Status: IMPLEMENTED under the renamed link types;
    absorbed as PROP-035 §12 / PROP-038's emission layer.** @status:doc/done
  - @fact:idx-prop-035 [PROP-035: The spec compiler](vibe-workspace/PROP-035-spec-compiler.md)
    — the directive preprocessor, `simple` / `normal` package formats, the
    `contract` / `source` split and the two-mode boot linker.
    **Status: IMPLEMENTED (§5–§13 as the `vibe-spec` crate); the §13 JIT
    loader and §10 link tables remain.** @status:doc/done
  - @fact:idx-prop-038 [PROP-038: Hybrid boot linking](vibe-workspace/PROP-038-hybrid-boot-linking.md)
    — fingerprint storage and granularity for the hybrid boot graph, with
    property-based mutation fuzzing over random DAGs. **Status: IMPLEMENTED.** @status:doc/done
- @fact:idx-mcp [`vibe-mcp/`](vibe-mcp/) — the MCP surface: servers, tool families, packaged servers. @status:doc/done
  - @fact:idx-prop-015 [PROP-015: MCP integration](vibe-mcp/PROP-015-mcp-integration.md)
    — `vibe mcp serve` / `install` / `status`: vibevm as an MCP server a
    coding agent queries about the lockfile and the installed corpus.
    **Status: IMPLEMENTED.** @status:doc/done
  - @fact:idx-prop-026 [PROP-026: The tcg tool family](vibe-mcp/PROP-026-tcg-tool-family.md)
    — the type-constrained-generation tool grammar. **Status: grammar
    normative through the family MCP servers; the standalone topology was
    superseded and removed.** @status:doc/done
  - @fact:idx-prop-027 [PROP-027: MCP packages](vibe-mcp/PROP-027-mcp-packages.md)
    — the `mcp` installable kind and the sovereign per-family servers.
    **Status: IMPLEMENTED.** @status:doc/done
- @fact:idx-cli [`vibe-cli/`](vibe-cli/) — the command surface and its terminal applications. @status:doc/done
  - @fact:idx-prop-036 [PROP-036: `vibe tree` — the spec-tree analyzer](vibe-cli/PROP-036-package-tree.md)
    — the boot/dependency tree model, `--json` against a published schema,
    project resolution from anywhere. **Status: IMPLEMENTED.** @status:doc/done
  - @fact:idx-prop-037 [PROP-037: `vibe tree` — the interactive TUI](vibe-cli/PROP-037-tree-tui.md)
    — the full TUI application on the action system: the F-key map, Search
    Everywhere, the four-layer MVC. **Status: IMPLEMENTED (Spec 2).** @status:doc/done
  - @fact:idx-prop-042 [PROP-042: AIUI observation](vibe-cli/PROP-042-aiui-observation.md)
    — the render plane and the `vibe aiui` verbs; `vibe term` and the
    cross-repo terminal contracts. **Status: ACTIVE.** @status:doc/done
- @fact:idx-settings [`vibe-settings/`](vibe-settings/) — application and user preferences. @status:doc/done
  - @fact:idx-prop-040 [PROP-040: The settings system](vibe-settings/PROP-040-settings.md)
    — a three-level, schema-first, introspectable preference store behind
    `vibe prefs`. **Status: IMPLEMENTED.** @status:doc/done
  - @fact:idx-prop-041 [PROP-041: The settings UI](vibe-settings/PROP-041-settings-ui.md)
    — the `vibe prefs` TUI: settings tree, per-type edit forms, provenance
    view, validation. **Status: IMPLEMENTED.** @status:doc/done
- @fact:idx-actions [`vibe-actions/`](vibe-actions/) — the frontend-agnostic behaviour layer. @status:doc/done
  - @fact:idx-prop-039 [PROP-039: The action system](vibe-actions/PROP-039-action-system.md)
    — addressable actions (`action://`), typed enablement, the keymap, i18n
    and the headless AIUI surface. **Status: IMPLEMENTED.** @status:doc/done
- @fact:idx-progress [`vibe-progress/`](vibe-progress/) — inline progress markup and the `vibe progress` tool. @status:doc/done
  - @fact:idx-prop-043 [PROP-043: The facts markup](vibe-facts/PROP-043-facts-markup.md) · [PROP-047: Progress campaigns](vibe-progress/PROP-047-progress-campaigns.md)
    — the `<status>` markup language, the campaign zone, the scan / check /
    report / mirror / weave / rescan / resume tool. **Status: RATIFIED and
    in execution — the spec-actualization campaign is its first consumer.** @status:doc/done
  - @fact:idx-owner-guide [OWNER-GUIDE](vibe-progress/OWNER-GUIDE.md)
    — the owner's daily reading of the markup: what each marker means and
    how to steer a campaign by it. @status:doc/done
