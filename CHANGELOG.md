# Changelog

vibevm has not shipped a stable release yet — every commit on `main` is part of the path to v0.1.0. This file is a curated chronicle of what landed when, organised by milestone rather than commit-by-commit. The single source of truth for "what changed in this commit" is `git log`; this file is the source of truth for "what does this milestone mean".

Format roughly follows [Keep a Changelog](https://keepachangelog.com/), grouped by milestone. The "Unreleased" section accumulates work-in-progress.

---

## [Unreleased]

The 2026-05-08 push bundled four milestones in one day. They land here under one block because the surface-consistency closer (M1.14.3) only makes sense in the context of M1.14 having shipped first; together they constitute the v0.1.0-ready package-management story.

### M1.12 — `vibe.toml` `[requires]` + cargo-shape install (2026-05-08)

- `ProjectManifest` gains `[requires].packages` / `.capabilities` re-using the `Requires` type from `vibe-package.toml`.
- `vibe install <pkgref>` now writes the user-supplied pkgref to `vibe.toml` after a successful apply — the cargo / npm pattern. `vibe install` with no arguments installs everything in `[requires]`.
- `vibe uninstall <pkgref>` symmetrically drops from `[requires]`.
- First-run migration: pre-`[requires]` projects get the manifest seeded from `vibe.lock` `meta.root_dependencies` on the next install.

### M1.13 — Cargo-shape version constraints (2026-05-08)

- `VersionSpec::parse` collapses to a single `semver::VersionReq::parse` call: bare semver `0.3.0` is shorthand for `^0.3.0` (caret), matching Cargo / npm / Poetry. Use `=0.3.0` for strict equal.
- `vibe install <pkgref>` (no version) records caret-of-resolved in the manifest. Explicit constraints are preserved verbatim.
- `--exact` flag (npm `--save-exact` shape) overrides both with `=<resolved>`.

### M1.14 — Authenticated registries (2026-05-08)

The big one — turns vibevm from "public registries only" into "production-ready for private repos."

- **Per-registry `auth` axis** (PROP-002 §2.2.1): `none` (default) / `token-env` / `credential-helper` / `ssh`. CLI: `vibe registry add --auth --token-env <NAME>`.
- **Token-env runtime**: `VIBEVM_REGISTRY_TOKEN_<HOST>` (or explicit `token_env`) loaded once at registry-open, injected as `https://x-access-token:<TOKEN>@host/...` in per-package URLs only at git-invocation time. Token never persists on disk — `set_remote_url(.., "origin", plain_url)` scrubs the credential out of `.git/config` immediately after `bootstrap`. `MissingToken` precheck before spawning git.
- **Auth-aware 401 classifier** (PROP-002 §2.3.1): public-registry 401 walks past as "no public answer here"; authenticated-registry 401 halts with actionable error. Closes the GitVerse-returns-401-for-missing-public-repo regression that surfaced via opencode + glm-flash.
- **TTY-aware credential silencing** in `apply_common_env` — non-TTY / `--unattended` runs silence GCM, `credential.helper`, `core.askPass` so a 401 cannot become a blocking GUI window.
- **Stderr classifier** extended for `could not read Username/Password`, `User cancelled dialog`, `HTTP 401/403`, `401 Unauthorized`, `403 Forbidden` (M1.14.1).
- **`--auth-required` flag** for strict CI gating: public-401 halts instead of walking, useful when fallback to a public substitute would be wrong (M1.14.2).
- **Aggregated per-registry error report** — `PackageNotFoundEverywhere { kind, name, summary }` lists each walked registry with URL, auth regime, and outcome. Inline multi-line `Display` flows through the standard error chain (M1.14.2).
- **`toml_edit`-based comment-preserving writes** — operator's hand-edited comments in `vibe.toml` survive every `vibe install` / `uninstall` / `registry add` mutation. Three layers preserved: header, per-table prefix, document trailing (M1.14.2).
- **Surface consistency closing slice** (M1.14.3): MCP `--yes` flag wired to actual TTY confirm prompt (was vestigial); `--assume-yes` alias on every MCP confirm-skip flag for symmetry with package commands; `--exact` extends from `install` to `update` (cargo `cargo update --precise X.Y.Z` shape — re-resolve and tighten manifest in one step); `--auth-required` extends from `install` to `update` + `outdated`.

### Other UX

- **Global `--unattended` flag + `VIBE_UNATTENDED` env-var**: implies `--assume-yes` / `--yes`, blocks wizards from opening, stamps `unattended: true` on JSON envelopes. Replaces the awkward `--invoked-by user-provisioning` workaround.
- **`docs/registry-auth.md`**: new operator reference covering all four auth regimes, env-var conventions, walk-vs-halt matrix, troubleshooting.
- **`docs/version-syntax.md`**: new operator reference for semver constraints (caret / tilde / equal / range), the two-file model (manifest = declaration, lockfile = materialisation), Cargo / npm / Poetry / Bundler comparison.
- **`vibe mcp install --scope both` works without `vibe.toml`**: provisioning scripts on a fresh user account succeed (project-leg silently skipped, user-leg writes as normal).

Phase A of M1.1-revision shipped earlier on `main` between 2026-04-23 and 2026-04-25; M1.7 (vibe-mcp server) shipped 2026-05-05; M1.10 (`vibe outdated`) shipped 2026-05-04. The next major milestone is M1.5 (LLM-based generation) — non-routine, needs separate sign-off.

---

## M1.1-revision Phase A — 2026-04-24 / 2026-04-25

The decentralized per-package registry refactor. Replaced the M1.1 monorepo-shaped registry with the model spelled out in [`PROP-002`](spec/modules/vibe-registry/PROP-002-decentralized-registry.md): one git repo per package under an organization URL, identity = `(kind, name, version, content_hash)`, `[[registry]]` array + `[[mirror]]` + `[[override]]` schema, transitive dependency resolution, maintainer-side publish command, JTD-driven wire contracts.

### Documentation slice (2026-04-24)

- Added `spec/common/PROP-000` §15–§19: dependency-weight pragmatism, JTD + codegen pattern, production-architecture-in-prototype lens, complexity-≥-RPM expectation, load-bearing setup-doc obligation.
- Added `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` "Memory discipline" section: project facts live in the repo, never in tool-specific user-memory.
- Added repo-root `DEV-GUIDE.md` and `RUNTIME-GUIDE.md` scaffolds.
- Amended `VIBEVM-SPEC.md` §7.3 / §7.4 / §7.5 / §8.1 / §8.2 / §8.3 / §8.4 / §8.6 / §11.2 / §16 for the per-package registry, capability-based deps, lockfile schema v2, and the new `vibe registry publish` command.
- Marked `PROP-001` §2.3 / §2.4 / §2.6 superseded by `PROP-002`; pruned the size-based argument in §2.1 per PROP-000 §15.
- Added `spec/modules/vibe-registry/PROP-002-decentralized-registry.md`: full design lock for the new registry model.
- Added `ROADMAP.md` M1.1-revision active section + M1.6 multi-registry-polish queued section.
- Added repo-root `TASKS.md` as the live work-slice checklist.
- Refreshed `spec/WAL.md` for the new phase.

### Schemas and codegen foundation (2026-04-25)

- `chore(git)` — `.gitattributes` pins LF line endings everywhere; `content_hash` is now OS-stable.
- `build(tools)` — JTD codegen scaffolding: `xtask` crate carries `cargo xtask codegen` / `check-codegen`, `tools/jtd-codegen/` README pins version 0.4.1 with per-platform install commands, `crates/vibe-wire/` placeholder ready to receive generated types, `.cargo/config.toml` aliases the runner.
- `feat(schemas)` — seven JTD schemas under `schemas/` document every CLI `--json` wire format: `init_report`, `install_plan`, `install_report`, `list_report`, `registry_sync_report`, `registry_publish_report`, `uninstall_report`. Schema-first authoring; struct migration follows when `jtd-codegen` is installed.

### Core types (2026-04-24)

- `feat(core)` — capability-based package dependencies. New `CapabilityRef` type (`<namespace>:<name>[@<version>]`). `PackageManifest` gains `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]`. Legacy `[dependencies]` compact form auto-migrates via `normalize_legacy_deps` on read; on next write the manifest renders in modern shape.
- `feat(core)` — `vibe.toml` schema v2: `[[registry]]` array (with `name` / `url` / `ref` / `naming`), `[[mirror]]` (with `of` / `url` / `priority`, where `of = "*"` matches any registry), `[[override]]` (surgical pkgref pin with `pkgref` / `source_url` / `ref` / `reason`). Legacy singleton `[registry]` form auto-migrates on read with `name = "default"` and `naming = "kind-name"`. `NamingConvention` enum (`KindName`, `Name`, `KindSlashName`) is a per-registry property.
- `feat(core)` — `vibe.lock` schema v2: `[meta]` gains `schema_version`, `solver`, `root_dependencies`; per-`[[package]]` gains `registry`, `source_url` (renamed from `source` with serde alias), `source_ref`, `resolved_commit`, `dependencies`, `overridden`. v1 files auto-migrate on next write. `Lockfile::looks_like_v1_on_disk()` heuristic for future UX nudges.

### Registry layer (2026-04-25)

- `feat(registry)` — shallow `ShellGit` primitives: `list_tags` (via `git ls-remote --tags`, deduped peeled-form), `fetch_file_at_ref` (via `git archive`, in-process tar extraction). Resolver walks N candidate versions of a package with N `git archive` round-trips, not N clones.
- `feat(registry)` — `GitPackageRegistry` for the per-package model: addresses each package as `<org>/<naming(kind, name)>.git`, versions are git tags, lazy clones (`bootstrap` / `update` only when committing to a version, not during dep walk). `fetch_dep_manifest` reads `vibe-package.toml` via `git archive` without cloning.
- `feat(registry)` — `MultiRegistryResolver` orchestrates priority + override + mirror dispatch. `[[override]]` short-circuits with manifest-identity verification (refuses if the manifest at the pinned ref names a different `(kind, name)`). `mirrors_for(name)` exposes priority-sorted mirror chain (runtime mirror dispatch + cross-source `content_hash` verification deferred to M1.6 Phase B).
- `feat(registry)` — `MultiRegistryResolver::refresh_lockfile_clones` walks lockfile and refreshes per-package clones; registry-served and override-served entries refresh through their respective subtrees; legacy / local entries reported as skipped.
- `refactor(registry)` — `CachedPackage` carries lockfile-v2 provenance (`registry_name` / `source_ref` / `resolved_commit` / `overridden`). All registry impls populate per their semantics. Dropped intermediate `MultiCached` wrapper.

### Resolver (2026-04-25)

- `feat(vibe-resolver)` — new crate. `DepProvider` / `DepSolver` traits. `NaiveDepSolver` (DFS, no backtracking) handles concrete deps + capabilities + obsoletes + conflicts + simple disjunctions. `MultiRegistryProvider` adapts `MultiRegistryResolver`; `LocalRegistryProvider` adapts `LocalRegistry`. `ResolvedNode.dependencies` post-processed to exact-pinned `=<version>` for the lockfile. Resolvo / libsolv impls reserved behind the same trait.

### Install pipeline (2026-04-25)

- `feat(install)` — switched `vibe install` to `MultiRegistryResolver`. `git+` prefix stripped at the backend boundary. `cli_e2e::install_from_git_registry` rewritten for the per-package fixture layout.
- `feat(install)` — transitive install via `NaiveDepSolver`. `vibe install` runs the solver before fetching; transitive packages materialise after roots; lockfile entries' `dependencies` populated with exact-pinned pkgrefs; `[meta].root_dependencies` carries the user-typed roots; CLI step output marks transitives as `(transitive)`.
- `feat(install)` — content_hash integrity check on plan. Lockfile-pinned `content_hash` must match a fresh fetch's hash for the same `(kind, name, version)`; mismatch surfaces as `InstallError::ContentDrift` with the pinned vs actual hashes named. PROP-002 §2.1 invariant enforced at plan time.
- `feat(registry)` — per-package `vibe registry sync` walks the lockfile and refreshes every per-package clone (`MultiRegistryResolver::refresh_lockfile_clones`).

### Publish tooling (2026-04-25)

- `feat(vibe-publish)` — new crate. `RepoCreator` trait + `GitVerseCreator` (Gitea-compatible HTTP via reqwest blocking + rustls). `Publisher` orchestrator (manifest read → repo create/reuse → init+push+tag). `Token` with debug/display redaction (renders as `***`). `vibe registry publish <path> [--registry <name>] [--dry-run]` subcommand. Error surface per PROP-002 §2.10 (auth-forbidden / org-not-found / push-denied / tag-collision / host-unreachable). Live API verification deferred to first real publish run.

### Fixtures, manuals, and end-user docs (2026-04-25)

- `chore(fixtures)` — relocated `packages/` → `fixtures/registry/` via `git mv` (history preserved). `packages/` is now reserved for the future dogfooding tree (vibevm using vibevm).
- `test(manual)` — `manual-tests/M1.5-gate-v2-per-package-smoke.md` walkthrough for the per-package model end-to-end against `vibespecs/`. Companion to the existing legacy-monorepo smoke.
- `docs(commands)` — reference pages under `docs/commands/` for every shipped CLI subcommand: `init`, `install`, `list`, `uninstall`, `registry sync`, `registry publish`, `version`. Each page has usage, flag table, examples, exit codes, schema links, related references.
- `docs(authoring)` — per-kind authoring guides under `docs/`: `authoring-flow.md`, `authoring-feat.md`, `authoring-stack.md`. Manifest examples, capability-name conventions, versioning rules, publish procedure.
- `docs` — repo-root `README.md`: hero, status, quick start, doc map, the four kinds, workspace layout, build/test, contributing, license. The landing page for anyone hitting the GitVerse repo URL.
- `docs(architecture)` — `docs/architecture.md`: contributor-facing tour of the workspace. Mental model (package / registry / pipeline), per-crate purposes with dep direction, key traits with future-impl slots, ASCII pipeline diagrams for install / publish / sync, wire formats, cache layout, file-tree quick reference, reading order for a new contributor.
- `docs` — `docs/lockfile-format.md`: exhaustive reference for `vibe.lock` v2. Field-by-field semantics, identity model, v1 → v2 migration, jq snippets for tooling, worked example.

### Test count

vibe-core: 38 → 63 tests. vibe-registry: 19 → 55. vibe-install: 6 → 11. vibe-cli: 11 + 6 e2e (unchanged). vibe-resolver: new — 14. vibe-publish: new — 10. vibe-wire / xtask / vibe-graph / vibe-llm / vibe-check: 0 each (placeholders or built-in-Rust modules with no Rust tests yet). Workspace total at the close of Phase A: **170+ tests**, clippy clean with `-D warnings`.

---

## M1.5-gate content slice — 2026-04-22 / 2026-04-23

Content for the M1.5-gate target: three demo flows live on the (then-monorepo) `anarchic/vibespecs` registry, end-to-end installable as a multi-package smoke.

- `feat(packages)` — published `flow:wal@0.1.0`, `flow:sync-from-code@0.1.0`, `flow:atomic-commits@0.1.0` to `git@gitverse.ru:anarchic/vibespecs.git`. Each ships a boot snippet at a distinct numeric prefix (`10-` / `20-` / `30-`).
- `test(manual)` — `manual-tests/M1.5-gate-multi-package-smoke.md`: three-package end-to-end smoke against the live monorepo registry. Distinct prefixes coexist; one shared clone under `~/.vibe/registries/<hash>/`; symmetric uninstall preserves user-owned files byte-identical.
- `feat(cli)` — `vibe init` writes the default `[registry]` (legacy singleton form) pointing at the public registry on first scaffold. Override with `--registry-url` / `--registry-ref`; opt out with `--no-registry`.
- `docs(wal,roadmap)` — checkpointed M1.5-gate content complete.

---

## M1.1 — 2026-04-22

Git-backed registry. Decisions pinned in [`PROP-001`](spec/modules/vibe-registry/PROP-001-git-backend.md).

- `feat(registry)` — `GitBackend` trait with `ShellGit` impl: shells out to system `git`, no `libgit2` runtime dep. Windows-specific spawn flags (`CREATE_NO_WINDOW`, `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0`) so the child never flashes a console window or hangs CI. Stable stderr classification for `RepoNotFound`, `AuthFailed`, `NetworkUnreachable`, `RefNotFound`.
- `feat(registry)` — `Registry` trait at the crate root with `LocalRegistry` (M0 path, kept) and `GitRegistry` (new) implementations. `git+<transport>://` source URIs in the lockfile.
- `feat(registry)` — `~/.vibe/registries/<hash>/` cache layout with first-use clone + 1-hour freshness TTL. `VIBE_REGISTRY_CACHE` env-var override.
- `feat(cli)` — `vibe install` reads the `[registry]` section from `vibe.toml`. Added `vibe registry sync` to force-refresh the registry cache.
- `refactor(core)` — lifted UTC timestamp helper into `vibe-core`.
- `test(manual)` — `manual-tests/M1.1-git-registry-smoke.md` against the real GitVerse registry.

---

## M0 — Walking skeleton — 2026-04-16 / 2026-04-17

The M0 milestone — proves the file-management mechanics work end-to-end.

- `chore` — repo scaffold, `.gitignore`, `LICENSE.md` (proprietary EULA placeholder).
- `docs` — recorded the four non-negotiable project rules (attribution, Conventional Commits, group by meaning, autonomy on routine changes only) in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`. Bootstrapped the `spec/` tree per `VIBEVM-SPEC.md` §14.1. Pinned the registry URL.
- `build` — Cargo workspace with seven crates: `vibe-cli`, `vibe-core`, `vibe-graph`, `vibe-registry`, `vibe-install`, `vibe-llm`, `vibe-check`.
- `feat(core)` — manifest schemas (`PackageManifest`, `ProjectManifest`, `Lockfile`), `PackageRef` / `PackageKind` / `VersionSpec`, `ValueTag` typed-value tags exchanged between graph nodes.
- `feat(registry)` — `LocalRegistry`: directory layout `<root>/<kind>/<name>/v<ver>/`, content-addressed cache at `<project>/.vibe/cache/<kind>/<name>/<ver>/`, `sha256:<hex>` content hashes computed deterministically across OSes.
- `feat(install)` — plan / apply / register / unregister loop. Boot-snippet conflict detection (filename + numeric `NN-` prefix). User-owned-paths guard. Exit codes per `VIBEVM-SPEC.md` §9.4.
- `feat(cli)` — `vibe init` / `install` / `list` / `uninstall` with plan → confirm → apply discipline. Output as human / `--json` / `--quiet`.
- `feat(packages)` — hand-wrote `flow:wal@0.1.0` as the canonical registry payload (the test-fixture template every smoke uses).
- `docs(wal)` — recorded the verified GitVerse push command and ready-to-publish state.
- `docs(spec)` — reconciled `VIBEVM-SPEC.md` with the shipped M0, pinned the mirror layout convention.
- 64 tests green at the M0 tag.

---

## Format notes

This file is curated, not auto-generated. Each milestone block is a hand-written rollup of conventional-commit subjects since the previous milestone, organised by area. Conventional Commits per [`PROP-000 §12.2`](spec/common/PROP-000.md#conventional-commits) make the rollup mechanical; the value-add of this file over `git log --oneline` is the milestone framing and cross-references to PROPs / SPEC sections that explain *why* a change happened.

Future format tightening: once we have a tagged release, `[Unreleased]` becomes a normal milestone block dated when the tag was cut, and a new `[Unreleased]` opens at the top.
