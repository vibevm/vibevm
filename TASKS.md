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

- [x] `build(tools)`: scaffolding for the JTD toolchain — `tools/jtd-codegen/README.md` pins version 0.4.1 with per-platform install commands; `tools/.gitignore` keeps binaries out of git; `xtask` crate carries `cargo xtask codegen` and `cargo xtask check-codegen`; `.cargo/config.toml` aliases `xtask = "run --quiet --package xtask --"`. Binary itself is not committed — first run after install populates generated code under `crates/vibe-wire/src/generated/`.
- [~] `feat(schemas)`: first schema landed — `schemas/registry_sync_report.jtd.json` describes the `vibe registry sync --json` wire shape. Migration of existing hand-rolled JSON outputs (install plan, list, publish report, GitVerse API client) to JTD-driven types lands incrementally; each migration is one schema added + one struct swap in the consuming crate.
- [x] `feat(vibe-wire)`: new `crates/vibe-wire` crate with `pub mod generated` placeholder, `[default-members]` excludes `xtask` from the published-as-`vibe` dependency tree.

### Core types (Rust)

- [x] `feat(core)`: type-safe package dependencies — parse `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` into `PackageRef` / `CapabilityRef` values; legacy `[dependencies]` compact form migrates transparently via `PackageManifest::normalize_legacy_deps`.
- [x] `feat(core)`: `vibe.toml` schema v2 — `[[registry]]` array with `naming` convention, `[[mirror]]` with priority + wildcard `of = "*"`, `[[override]]` for surgical pkgref pins; v1 singleton auto-migrated on read; serializes in modern form on write; `primary_registry()` / `registry_by_name()` / `mirrors_for()` helpers.
- [x] `feat(core)`: `vibe.lock` schema v2 — `LockedPackage` gains `registry` / `source_url` (renamed from `source` with serde alias) / `source_ref` / `resolved_commit` / `dependencies` / `overridden`; `LockfileMeta` gains `schema_version` / `solver` / `root_dependencies`; v1 lockfiles auto-migrate on next write via serde alias + defaults; `looks_like_v1_on_disk()` heuristic for future UX nudges; `vibe list --json` and `vibe install --json` plan output renamed `source` → `source_url` to match lockfile shape.

### Resolver and registry layer

- [x] `feat(vibe-resolver)`: new crate with `DepProvider` / `DepSolver` traits; `NaiveDepSolver` (DFS, no backtracking) handles concrete deps + capabilities + obsoletes + conflicts + simple disjunctions; `MultiRegistryProvider` adapts `MultiRegistryResolver`, `LocalRegistryProvider` adapts `LocalRegistry`. `ResolvedNode.dependencies` post-processed to exact-pinned `=<version>` for the lockfile. Resolvo / libsolv impls behind the same trait still pending — naive covers today's all-empty-deps fixtures and any first-cut realistic graph.
- [x] `feat(registry)`: `ShellGit::list_tags` (via `git ls-remote --tags`, dedupes annotated-tag peeled-form) and `ShellGit::fetch_file_at_ref` (via `git archive --remote=<url> --format=tar`, in-process tar extraction, no `tar` crate); `GitBackend` trait widened with both methods plus `FileNotFoundInRef` and `ArchiveUnsupported` error variants.
- [x] `feat(registry)`: `GitPackageRegistry` — per-package repo addressing through `NamingConvention`, tag-based versions, lazy clones (`bootstrap` / `update` only when committing to a version, not during dep-walk). `fetch_dep_manifest` reads `vibe-package.toml` via `git archive` without cloning. Exists alongside the legacy monorepo `GitRegistry` until `MultiRegistryResolver` switches `vibe install` over.
- [x] `feat(registry)`: `MultiRegistryResolver` — priority-ordered registry walk with fall-through on `UnknownPackage`, `[[override]]` short-circuit (with manifest-identity verification at the pinned ref so a misnamed override fails loud), `mirrors_for(name)` exposing priority-sorted mirror chain (runtime mirror dispatch + cross-source content_hash verification deferred to M1.6 Phase B). `MultiResolution` / `MultiCached` carry registry-name / source_url / source_ref / overridden provenance for lockfile v2.
- [x] `feat(install)`: switch `vibe install` to `MultiRegistryResolver`; `CachedPackage` carries `registry_name` / `source_ref` / `resolved_commit` / `overridden`; `register_installed` forwards them to lockfile v2. `git+` prefix stripped at the backend boundary across `GitPackageRegistry` and override paths so `git+file://` / `git+ssh://` URLs in `vibe.toml` Just Work. `cli_e2e::install_from_git_registry` rewritten for the per-package fixture layout.
- [x] `feat(registry)`: per-package `vibe registry sync` — walks lockfile entries, refreshes each per-package clone via `MultiRegistryResolver::refresh_lockfile_clones`; registry-served and override-served entries refresh through their respective subtrees; legacy / local / unattributed entries reported as skipped.
- [x] `feat(install)`: transitive install through `NaiveDepSolver` — `vibe install` runs the solver before fetching; transitive packages materialise after roots; lockfile entries' `dependencies` populated with exact-pinned pkgrefs; `[meta].root_dependencies` carries the user-typed roots; CLI step output marks transitives as `(transitive)`.

### Publish tooling

- [x] `feat(vibe-publish)`: new `crates/vibe-publish` crate with `RepoCreator` trait, `GitVerseCreator` (Gitea-compatible HTTP via reqwest blocking + rustls), `Publisher` orchestrator (manifest read → repo create/reuse → init+push+tag), `Token` with debug/display redaction, `vibe registry publish <path> [--registry <name>] [--dry-run]` subcommand. Error surface per PROP-002 §2.10 (auth-forbidden / org-not-found / push-denied / tag-collision / host-unreachable). Live API verification deferred to first real publish run; assumed Gitea-compatible request shapes documented inline.

### Fixture migration and live packages

- [x] `chore(fixtures)`: relocated `packages/` → `fixtures/registry/` via `git mv` (history preserved). Layout intentionally stays M0-monorepo for the LocalRegistry hermetic-fixture path; `cli_e2e::fixture_registry()` updated; `packages/` is now reserved for the future dogfooding tree (vibevm using vibevm).
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
