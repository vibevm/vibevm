# TASKS — vibevm, active work

Live checklist for the current work-slice. Each item is a logical commit (Conventional Commits per [PROP-000 §12.2](spec/common/PROP-000.md#conventional-commits); grouped by meaning per §12.3).

**Status key:** `[ ]` queued · `[~]` in progress · `[x]` done.

---

## Current slice: Phase A of the decentralized-registry refactor

Scope: fix the Nix-style registry lock-in from day one — per-package repos, multi-registry / mirror / override schemas, content-addressed identity, resolvo-backed transitive deps, JTD wire-contract foundation, maintainer publish tool. Phase B (polished multi-registry UX, `vibe vendor`, richer publish adapters) lands as a follow-up milestone (M1.6).

Full design locked in [PROP-002](spec/modules/vibe-registry/PROP-002-decentralized-registry.md) once that file lands (queued below).

### Documentation (do first — contract before code)

- [x] `docs(prop-000)`: add §15 (dep weight) / §16 (JTD + codegen) / §17 (prod-arch lens) / §18 (complexity ≥ RPM) / §19 (load-bearing setup docs).
- [x] `docs(claude)`: "Memory discipline" section in CLAUDE.md / AGENTS.md / GEMINI.md.
- [x] `docs(guides)`: create `DEV-GUIDE.md` and `RUNTIME-GUIDE.md` scaffolds at repo root.
- [x] `docs(spec)`: amend `VIBEVM-SPEC.md` §7.3 / §7.4 / §7.5 / §8.1 / §8.2 / §8.3 / §8.4 / §8.6 (new) / §11.2 revision note / §16 M1 acceptance for decentralized per-package registry, `[[registry]]` array, `[[mirror]]`, `[[override]]`, lockfile v2, capability-based deps, depsolver, maintainer publish.
- [x] `docs(prop-001)`: mark §2.3 / §2.4 / §2.6 as superseded by PROP-002; prune size-based argument in §2.1 per PROP-000 §15.
- [x] `docs(prop-002)`: write new `spec/modules/vibe-registry/PROP-002-decentralized-registry.md` — full design lock.
- [x] `docs(roadmap)`: add M1.1-revision (per-package + resolver) and M1.6 (multi-registry polish); update snapshot.
- [x] `docs(wal)`: checkpoint new phase; retire the v1-era "current phase" text.

### Schemas and codegen foundation

- [ ] `build(tools)`: vendor `jtd-codegen` binary under `tools/jtd-codegen/`; add `cargo xtask codegen` with zero-drift CI check.
- [ ] `feat(schemas)`: first JTD schemas — GitVerse publish-API request/response shapes, `vibe --json` event schema, LLM provider wire shapes (forward-compat stubs for M1.5).
- [ ] `feat(vibe-wire)`: new `crates/vibe-wire` with generated Rust types; re-exports curated for downstream crates.

### Core types (Rust)

- [x] `feat(core)`: type-safe package dependencies — parse `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` into `PackageRef` / `CapabilityRef` values; legacy `[dependencies]` compact form migrates transparently via `PackageManifest::normalize_legacy_deps`.
- [x] `feat(core)`: `vibe.toml` schema v2 — `[[registry]]` array with `naming` convention, `[[mirror]]` with priority + wildcard `of = "*"`, `[[override]]` for surgical pkgref pins; v1 singleton auto-migrated on read; serializes in modern form on write; `primary_registry()` / `registry_by_name()` / `mirrors_for()` helpers.
- [x] `feat(core)`: `vibe.lock` schema v2 — `LockedPackage` gains `registry` / `source_url` (renamed from `source` with serde alias) / `source_ref` / `resolved_commit` / `dependencies` / `overridden`; `LockfileMeta` gains `schema_version` / `solver` / `root_dependencies`; v1 lockfiles auto-migrate on next write via serde alias + defaults; `looks_like_v1_on_disk()` heuristic for future UX nudges; `vibe list --json` and `vibe install --json` plan output renamed `source` → `source_url` to match lockfile shape.

### Resolver and registry layer

- [ ] `feat(vibe-resolver)`: new crate wrapping `resolvo`; `DepSolver` trait with `ResolvoSolver` impl (and door left open for a future `LibsolvSolver` fallback).
- [ ] `feat(registry)`: `ShellGit::list_tags` and `ShellGit::fetch_file_at_ref` — cheap ls-remote and shallow file fetch without full clone.
- [ ] `feat(registry)`: `GitPackageRegistry` (per-package repo, tag-based versions, flat layout) replacing the monorepo `GitRegistry`.
- [ ] `feat(registry)`: `MultiRegistryResolver` — priority-ordered list of `[[registry]]`, mirror fallback chain per registry, `[[override]]` short-circuit, content-hash cross-source integrity verification.
- [ ] `feat(install)`: transitive install through the resolver; plan-rendering shows the full resolved subgraph with provenance tags (`(dep of flow:foo)`); `--dry-run` full.

### Publish tooling

- [ ] `feat(vibe-publish)`: new `crates/vibe-publish` crate with `RepoCreator` trait and `GitVerseCreator` impl; `vibe registry publish <path>` subcommand; graceful non-admin error UX (HTTP 401/403 → actionable message, push-denied detection, tag-collision detection).

### Fixture migration and live packages

- [ ] `chore(fixtures)`: relocate `packages/` → `fixtures/registry/` (per-package flat layout; keeps hermetic e2e tests working without network).
- [ ] `test(e2e)`: update `cli_e2e.rs` against the new fixture layout.
- [ ] `feat(packages-live)`: migrate three v0.1.0 flows to per-package repos in the `vibespecs` organization — `vibespecs/flow-wal`, `vibespecs/flow-sync-from-code`, `vibespecs/flow-atomic-commits` — via `vibe registry publish`.
- [ ] `test(manual)`: new manual smoke `M1.5-gate-v2-per-package-smoke.md` against live per-package registry; retire or mark v1-era monorepo smoke as historical.

### Close-out

- [ ] `docs(wal, roadmap, prop-000)`: Phase A checkpoint, 81+ tests green, clippy clean, all new contracts wired.

---

## Backlog (post-Phase-A; not active)

- M1.6 polish: second live `[[registry]]`, full mirror fallback exercised in e2e, `vibe vendor` generator, `vibe registry add/list/set-mirror` CLI surface, GitHub / Gitea / Forgejo publish adapters on demand.
- JTD'd `vibe show` / `vibe plan` / `vibe build` event streams.
- Supply-chain attestation (sigstore or equivalent) — out of M1 scope, noted for architectural allowance now.
