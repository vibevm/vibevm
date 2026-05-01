# WAL — Project Continuation State
_Updated: 2026-05-01_

## Current phase

**M1.6 Phase B v0 — IN PROGRESS (2026-05-01).** Phase A is closed; the registry-management CLI surface and the read-only mirror-dispatch runtime are in. Active commits since the Phase A checkpoint (`9646de9`):

- `1089417 fix(vibe-install): drop uninstalled package from root_dependencies` — regression surfaced by walking `manual-tests/M1.5-gate-v2-per-package-smoke.md` top-to-bottom against the live GitHub host. `unregister_installed` now retains roots whose `(kind, name)` doesn't match the uninstalled package, symmetric with the install merge.
- `152c607 test(manual): record M1.5-gate-v2 smoke pass on GitHub host` — first formal walk of the smoke filled in. Date 2026-05-01, vibevm `1089417`, peeled SHAs `1c3a1355` / `a620157d` / `d76512034`, Windows 11 / git 2.52.
- `8260f83 feat(cli): vibe registry list` + `7c26faf docs(commands): vibe registry list reference` — read-only inspector for `[[registry]]` / `[[mirror]]` / `[[override]]` blocks; reports the host adapter `vibe registry publish` would dispatch to per PROP-002 §2.10.
- `001f364 feat(cli): vibe registry add` + `2c13276 docs(commands): vibe registry add reference` — mutating sibling: append a new `[[registry]]` (or insert as `--position primary`); validates name uniqueness, URL shape via `extract_*_segment`, naming convention, and position. Manifest-only — no host probe, no lockfile mutation.
- `3fa8c01 feat(cli): vibe registry set-mirror` + `54e64f5 docs(commands): vibe registry set-mirror reference` — append a `[[mirror]]` block; named `<OF>` requires the registry to exist, wildcard `*` is accepted even before any registry is configured (forward-compatible).
- `2e9ebf8 feat(vibe-registry): mirror-aware lookups (Phase B v0)` — read-only mirror dispatch landed. `GitPackageRegistry` carries `mirror_urls` (org-level, populated by `MultiRegistryResolver::from_manifest` from `mirrors_for(reg.name)` priority-sorted output). `list_versions` and `fetch_dep_manifest` archive path try primary first, then each mirror; the cache-mutating `fetch` and `refresh_package` paths stay primary-only until cross-source `content_hash` verification lands. The `try_lookup<T, F>` helper centralises the dispatch and returns the **primary's** error on full failure (most informative diagnostic). `tracing::info!` on mirror-served lookups, `tracing::debug!` on per-mirror failures.
- `5d7e751 feat(cli): vibe registry remove` + `1c9adf8 docs(commands): vibe registry remove reference` — closes the registry-management CRUD: drop `[[registry]]` (refuses to orphan named mirrors; wildcard `*` mirrors are unaffected) or `[[mirror]]` (exact `(of, url)` match; warns on hand-edited duplicates).

Workspace state: 217+ tests across the workspace (5 new in `vibe-registry` covering primary-success / fallback-to-mirror / multi-mirror-priority-walk / all-fail-returns-primary-error / `fetch_dep_manifest` archive path mirror dispatch; 5 new in `vibe-cli` covering `adapter_for_host` and `parse_naming`; 1 new in `vibe-install` pinning the `unregister_installed` root-dependency contract). `cargo clippy --workspace --all-targets -- -D warnings` clean.

The CLI inspector + mutator surface for registry management is now full. Next slice of Phase B is mirror-dispatch on the cache-mutating paths (`fetch`, `refresh_package`) plus cross-source `content_hash` verification — that's what makes mirrors useful for actual installs (not just dep walks). After that: `vibe vendor` (offline mirror generator), and continued use of the multi-registry surface in real-world scenarios.

---

**M1.1-revision Phase A — DONE (2026-04-29).** Decentralized per-package registry shipped end-to-end on its production host. All three v0.1.0 demo flows (`flow:wal`, `flow:sync-from-code`, `flow:atomic-commits`) live at `https://github.com/vibespecs/flow-<name>` with `v0.1.0` tags; a fresh `vibe init` → `vibe install flow:wal` / `flow:sync-from-code` / `flow:atomic-commits` resolves all three, populates lockfile v2, refreshes per-package clones via `vibe registry sync`. Registry org migrated from GitVerse to GitHub on 2026-04-29 because GitVerse's public REST API does not expose org-scoped repo creation; `GitHubCreator` adapter behind the existing `RepoCreator` trait drives the publish flow against `POST /orgs/{org}/repos`. The vibevm tool source itself stays on GitVerse — only the registry org moves.

**Phase A close-out summary:**

- 6 commits since the prior checkpoint: `docs(spec,guides,manual-tests)` migration policy → `feat(vibe-publish,cli)` GitHub adapter + per-host token loader → `feat(core,cli)` `DEFAULT_REGISTRY_URL` rotation → `fix(vibe-publish)` credential redaction in error messages → `fix(vibe-registry)` clone-fallback + tag-aware update → this WAL checkpoint.
- 3 live publishes performed (`https://github.com/vibespecs/flow-wal`, `flow-sync-from-code`, `flow-atomic-commits`), each tagged `v0.1.0`. Token never displayed in any output, log line, error message, or commit body during the run.
- Cargo workspace stays green: `cargo test --workspace` (~210 tests across the workspace, 30 in `vibe-publish` alone covering host adapter selection, token redaction, scope-violation guards), `cargo clippy --workspace --all-targets -- -D warnings` clean.

**Next milestone:** M1.6 (multi-registry polish — Phase B of the decentralized-registry refactor). M1.5-gate docs landed; M1.2 / M1.3 / M1.4 still open.

The M1.1 monorepo-shaped registry (one `anarchic/vibespecs` repo, `<kind>/<name>/v<ver>/` directories, `[registry]` singleton in `vibe.toml`) was replaced — at the design level — with a decentralized per-package model before any downstream consumer is at risk of being locked into it. Full design lock lives in [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md).

What this means architecturally:

- **Packages become standalone repos** under a hosting organization (`git@gitverse.ru:vibespecs`). Default repo naming `<kind>-<name>`. Versions are git tags (`v0.1.0`, `v0.2.0`). No monorepo.
- **`vibe.toml` gains `[[registry]]` array** + `[[mirror]]` + `[[override]]`. Priority-ordered resolve; mirrors are transparent; overrides bypass the resolver for pins. Schema supports the full shape; Phase A runtime exercises one registry, Phase B (M1.6) exercises several live.
- **Identity is `(kind, name, version, content_hash)`** — URL is informational. Mirror-switching and host-migration never churn the lockfile. Integrity check enforced on every fetch.
- **Lockfile schema v2** — `registry`, `source_url`, `source_ref`, `resolved_commit`, `content_hash`, `dependencies`, `overridden` per package; `schema_version`, `solver`, `root_dependencies` in `[meta]`. v1 lockfiles auto-migrate on next write.
- **Transitive depsolver** — `resolvo` crate (BSD-3-Clause, Rust-native, used by Pixi / Rattler at conda scale). `DepSolver` trait leaves a `libsolv` fallback slot. Capability-based deps: `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` — all semantic, not advisory.
- **Maintainer utility** `vibe registry publish <path>` — creates a package repo through a host adapter (GitVerse in v1), pushes content, tags version. Non-admin error surface tuned (401/403/push-denied/tag-collision all render actionably).
- **JTD + codegen** for wire contracts — GitVerse API client, `vibe --json` events, future LLM provider wrappers. Toolchain project-local under `tools/jtd-codegen/`.
- **Local fixtures relocate** from `packages/` to `fixtures/registry/` — keeps `packages/` free for the future dogfooding path (vibevm using vibevm).

The three live v0.1.0 flows (`flow:wal`, `flow:sync-from-code`, `flow:atomic-commits`) stay at `anarchic/vibespecs` for now — read-only, pointer README forthcoming. Phase A migrates them into per-package repos under `vibespecs/<kind>-<name>` via the new publish utility.

**Standing owner directives** that landed this slice (see [PROP-000](common/PROP-000.md) §15–§19 and [`CLAUDE.md`](../CLAUDE.md)):

- Dependency weight is not a decision factor — pick best-in-class.
- JTD + codegen is the default for wire contracts.
- Production architecture in the prototype phase ("Google-principal lens").
- Complexity expectation ≥ RPM for the dep model.
- Load-bearing setup docs at repo root: [`DEV-GUIDE.md`](../DEV-GUIDE.md), [`RUNTIME-GUIDE.md`](../RUNTIME-GUIDE.md).
- Project facts stay in the project; no project-level state in tool-specific global user-memory.

**Immediate next work (after this checkpoint).** Phase A code adjustments for the host migration land first: new `GitHubCreator` behind `RepoCreator`, host-aware adapter selection in the CLI, per-host token loader (`~/.vibevm/<host>.publish.token` precedence), `DEFAULT_REGISTRY_URL` rotated to `https://github.com/vibespecs`, manual-test rewritten for the GitHub-shape flow. After the workspace stays green (`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`), the live publish of `flow:wal@0.1.0` / `flow:sync-from-code@0.1.0` / `flow:atomic-commits@0.1.0` runs against `github.com/vibespecs`. **Non-routine** per CLAUDE.md Rule 4 (creates real public artefacts in the new org), so it requires explicit owner sign-off before push.

**Host migration to GitHub (2026-04-29).** GitVerse's public REST API does not expose an org-scoped repo creation endpoint — `POST /orgs/{org}/repos` returns 404 / WAF 403 against `https://api.gitverse.ru` even with correct auth and Accept headers; only `POST /user/repos` is documented, and there is no documented user-to-org transfer endpoint. Without org-scoped creation `vibe registry publish` cannot drive the publish loop end-to-end on GitVerse without manual web-UI pre-creation per release, which defeats the point of a publish utility. The owner's decision (2026-04-29): keep the **vibevm project repository** on GitVerse (`anarchic/vibevm` — unaffected) and migrate the **package registry organization** to GitHub — `https://github.com/vibespecs`. Identity remains content-hashed per [PROP-002 §2.1](modules/vibe-registry/PROP-002-decentralized-registry.md#identity); no `content_hash` is invalidated by the host change. Full architectural rationale: [PROP-000 §7](common/PROP-000.md#registry) and [PROP-002 §2.10](modules/vibe-registry/PROP-002-decentralized-registry.md#publish).

**GitHub API surface (assumed; live-verified during this slice).** Base URL `https://api.github.com`. Auth: `Authorization: Bearer <T>`. Accept: `application/vnd.github+json`. Versioning header: `X-GitHub-Api-Version: 2022-11-28`. Endpoints used: `GET /repos/{owner}/{repo}` (presence check); `POST /orgs/{org}/repos` (repo creation — works natively, returns 201 with full repo metadata). Push auth: HTTPS via the publish token, embedded into the push URL as `https://x-access-token:<TOKEN>@github.com/vibespecs/<repo>.git` for the duration of `git remote add` / `git push`; modern git ≥ 2.31 redacts URL passwords in its own log output. Adapter source: `crates/vibe-publish/src/github.rs`.

**GitVerse API surface (live-verified 2026-04-26, retained).** Base URL `https://api.gitverse.ru`. Auth: `Authorization: Bearer <T>`. Accept header MUST carry the version: `application/vnd.gitverse.object+json;version=1`. `GET /repos/{owner}/{repo}` works; `POST /orgs/{org}/repos` does not. Findings baked into `crates/vibe-publish/src/gitverse.rs` (commit `36cbf08`); the GitVerse adapter remains in tree for any future Gitea-shape host that fully supports the org-scoped POST.

**Token convention (per PROP-000 §20).** Publish-token loader walks: `VIBEVM_PUBLISH_TOKEN` env → `~/.vibevm/<host-prefix>.publish.token` (`github.publish.token`, `gitverse.publish.token`) → legacy `~/.vibevm/git.publish.token`. CLI prints token *source* only; value never appears in any vibevm-produced output. Adapter scope: each `RepoCreator` impl refuses operations outside the org named in the project's `[[registry]].url`.

**JTD toolchain.** Scaffolding is in place (`tools/jtd-codegen/`, `xtask`, `schemas/`, `crates/vibe-wire/`); the `jtd-codegen` binary itself needs a one-time install per `tools/jtd-codegen/README.md` before the first `cargo xtask codegen` run. Migration of existing hand-rolled `Serialize` structs to JTD-driven types is incremental and lands as the consumers are touched.

## Constraints (do not violate without discussion)

- **Language:** Rust only for the CLI. See [spec://vibevm/common/PROP-000#language](common/PROP-000.md#language).
- **License:** proprietary EULA placeholder (see [`LICENSE.md`](../LICENSE.md)); eventual target is UPL 1.0 — owner's decision. See [spec://vibevm/common/PROP-000#license](common/PROP-000.md#license). Third-party deps: permissive only (MIT / Apache-2.0 / BSD / Unlicense; MPL-2.0 case-by-case; GPL / AGPL / LGPL forbidden).
- **Manifest format:** TOML for human-edited configs (`vibe.toml`, `vibe.lock`, `vibe-package.toml`); JTD+codegen for wire contracts ([PROP-000 §16](common/PROP-000.md#jtd)).
- **Vocabulary lock:** only `flow`, `feat`, `stack`, `tool`. Never `lifecycle`, `phase`, `goal`, `plugin` (except as passing synonym for `package`).
- **User-owned files** (`vibe install`/`uninstall` never modifies): `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`, any 00-09 or 90-99 boot file.
- **Four project rules** authoritative in [spec://vibevm/common/PROP-000#commits](common/PROP-000.md#commits), copied into `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`: (1) attribution — human-authored; (2) Conventional Commits; (3) group by meaning; (4) autonomy on routine changes only.
- **Memory discipline** pinned in `CLAUDE.md` (and copies): project facts go into the repo (`CLAUDE.md`, `MEMORY.md`, `TASKS.md`, `spec/**`); tool-specific global user-memory holds only machine-local facts.
- **Setup doc obligation** ([PROP-000 §19](common/PROP-000.md#setup-docs)): any change to toolchain / prereqs / env / paths updates `DEV-GUIDE.md` or `RUNTIME-GUIDE.md` in the same commit.
- **Dependency weight** not a decision factor ([PROP-000 §15](common/PROP-000.md#dep-weight)) — pick best library, reject only on license / abandonment / security / bad API.
- **Architect with production lens** ([PROP-000 §17](common/PROP-000.md#prod-arch)): load-bearing surfaces (lockfile, registry protocol, dep-resolver, wire formats) ship production-quality even in prototype phase.
- **Complexity expectation ≥ RPM** ([PROP-000 §18](common/PROP-000.md#complexity)): capability-based, virtual-package-aware, disjunction-supporting dep model from day one.
- **Git backend:** shell-out to system `git`, behind `GitBackend` trait (PROP-001 §2.1 — size argument pruned per PROP-000 §15; Windows SSH-auth and diagnostic clarity still carry the call).
- **Cache root:** `~/.vibe/registries/<canonical-url-hash>/packages/<kind>-<name>/` per PROP-002 §2.6. `VIBE_REGISTRY_CACHE` env-var overrides.
- **Registry default in `vibe init`.** New projects scaffold `[[registry]] name = "vibespecs" url = "https://github.com/vibespecs"` — ORG root on GitHub (not a package repo). Single source of truth: `vibe_core::manifest::DEFAULT_REGISTRY_URL`. Override with `--registry-url <URL>` / `--registry-ref <REF>`; opt out with `--no-registry`.
- **Manual-test protocol:** runnable smoke-tests in [`manual-tests/`](../manual-tests/), one file per scenario, clean-slate setup + teardown. Policy in [PROP-000 §14](common/PROP-000.md#manual-tests).
- **REVIEW marker discipline:** when the spec is silent, pick the conservative interpretation, mark with `<!-- REVIEW: … -->`, surface in the session report.
- **`refs/` not committed.** Upstream reference material (book + cloned study repos).

## Remotes

- **vibevm source (this repo):** `git@gitverse.ru:anarchic/vibevm.git` (SSH) / `https://gitverse.ru/anarchic/vibevm` (web). **Stays on GitVerse.**
- **Package registry (target as of 2026-04-29):** organization `vibespecs` on **GitHub** — `https://github.com/vibespecs/<kind>-<name>` per package. Phase A populates it via `vibe registry publish` driving the new `GitHubCreator` adapter.
- **Legacy package registry (read-only transition):** `git@gitverse.ru:anarchic/vibespecs.git`. Holds three v0.1.0 flows in monorepo form (HEAD `2203239`, 2026-04-23). No new publishes here; superseded by the GitHub-hosted per-package repos during Phase A; kept readable for existing projects with schema-v1 lockfiles until they migrate.
- **Publish tokens (local).** Per-host file precedence: `~/.vibevm/<host>.publish.token` (e.g. `github.publish.token` for github.com, `gitverse.publish.token` for gitverse.ru) → legacy `~/.vibevm/git.publish.token`. Env-var `VIBEVM_PUBLISH_TOKEN` overrides everything. Token secrecy invariant per [PROP-000 §20](common/PROP-000.md#token-secrecy) — never displayed, never persisted outside `~/.vibevm/`, never committed. Verified by the owner as having `repo:create` (GitHub) / equivalent rights in the `vibespecs` organization.

## Done

### M0 — walking skeleton (complete, published)

- [x] `VIBEVM-SPEC.md` received (v1.0), book and reference sources read.
- [x] Project rules landed in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` and [PROP-000 §12](common/PROP-000.md#commits).
- [x] `git init`, `.gitignore`, `LICENSE.md`.
- [x] Boot snippets, PROP-000 foundation.
- [x] Cargo workspace with 7 crates.
- [x] Full plan / apply / register / uninstall loop against a local-directory registry. 64 tests green at M0 tag.

### M1.1 — monorepo git-backed registry (shipped 2026-04-22, now partially superseded by M1.1-revision)

- [x] [PROP-001](modules/vibe-registry/PROP-001-git-backend.md), `GitBackend` trait + `ShellGit`, `Registry` trait, `LocalRegistry` + `GitRegistry`, normalized-URL hash cache at `~/.vibe/registries/<hash>/`, 1-hour freshness TTL, `git+<transport>://…` lockfile source URIs.
- [x] End-to-end test `install_from_git_registry`; live smoke [`M1.1-git-registry-smoke.md`](../manual-tests/M1.1-git-registry-smoke.md).
- [x] `vibe init` writes `[registry]` pointing at the default registry.
- **Partially superseded:** cache layout (§2.4), Registry trait shape (§2.3), lockfile `source_uri` format (§2.6) replaced by PROP-002. GitBackend / ShellGit / freshness / Windows UX remain authoritative.

### M1.5-gate content — three v0.1.0 demo flows (published 2026-04-22 / 2026-04-23 on the legacy monorepo)

- [x] `flow:wal@0.1.0` at vibespecs `98e51fc` — canonical flow, boot-snippet prefix `10-`.
- [x] `flow:sync-from-code@0.1.0` at vibespecs `47582af` — prefix `20-`.
- [x] `flow:atomic-commits@0.1.0` at vibespecs `2203239` — prefix `30-`.
- [x] Live multi-package smoke [`M1.5-gate-multi-package-smoke.md`](../manual-tests/M1.5-gate-multi-package-smoke.md) passed 2026-04-23 against monorepo registry.
- **Now:** these three flows are the live-migration target of M1.1-revision Phase A — they move into per-package repos `vibespecs/flow-wal`, `vibespecs/flow-sync-from-code`, `vibespecs/flow-atomic-commits` via the new publish utility.

### M1.1-revision documentation slice (landed 2026-04-24, this session)

- [x] [PROP-000](common/PROP-000.md) §15–§19 — dep-weight, JTD, production-architecture lens, complexity ≥ RPM, load-bearing setup docs.
- [x] [`CLAUDE.md`](../CLAUDE.md) / [`AGENTS.md`](../AGENTS.md) / [`GEMINI.md`](../GEMINI.md) — "Memory discipline: project facts stay in the project" section.
- [x] [`DEV-GUIDE.md`](../DEV-GUIDE.md) and [`RUNTIME-GUIDE.md`](../RUNTIME-GUIDE.md) at repo root, minimal skeletons.
- [x] `VIBEVM-SPEC.md` §7.3 (capability-based deps), §7.4 (lockfile v2), §7.5 (`[[registry]]` / `[[mirror]]` / `[[override]]`), §8.1 (decentralized registry frame), §8.2 (per-package layout), §8.3 (canonical-URL-rooted cache + `ls-remote` / `git archive` optimisations), §8.4 (maintainer publish utility), new §8.6 (depsolver), §11.2 revision note, §16 M1 acceptance expanded.
- [x] [PROP-001](modules/vibe-registry/PROP-001-git-backend.md) — "Superseded parts" block identifying §2.3 / §2.4 / §2.6 as revised by PROP-002; size-based argument in §2.1 pruned per PROP-000 §15.
- [x] [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md) — full design lock for the decentralized registry refactor.
- [x] [`ROADMAP.md`](../ROADMAP.md) — M1.1-revision active section, M1.6 (multi-registry polish) queued.
- [x] [`TASKS.md`](../TASKS.md) at repo root — live checklist for the current slice.

## Code slice landed (2026-04-24 → 2026-04-25)

The full Phase A code slice is in. Each item below is one or more
shipped commits on `origin/main`; cross-reference the commit log for
specifics. Total workspace state: 169+ tests green, clippy clean
with `-D warnings` across the workspace, six new crates / modules
since the documentation checkpoint:

- **`chore(git): pin line endings to LF`** — `.gitattributes` everywhere; content_hash is OS-stable.
- **`feat(core): capability-based package dependencies`** — `CapabilityRef`, `[provides]`/`[requires]`/`[[requires_any]]`/`[obsoletes]`/`[conflicts]` typed and serde-wired; legacy `[dependencies]` migrates transparently.
- **`feat(core): vibe.toml schema v2`** — `[[registry]]` array + `[[mirror]]` + `[[override]]`; singleton legacy form auto-migrates on read; `NamingConvention` enum with three forms.
- **`feat(core): vibe.lock schema v2`** — `schema_version`, `solver`, `root_dependencies` in `[meta]`; `registry`/`source_url`/`source_ref`/`resolved_commit`/`dependencies`/`overridden` per package; serde alias on `source` reads v1 transparently.
- **`feat(registry): shallow ShellGit primitives`** — `list_tags` (via `git ls-remote --tags`, peeled-form deduped) + `fetch_file_at_ref` (via `git archive`, in-process tar extraction).
- **`feat(registry): GitPackageRegistry`** — per-package repo addressing through `NamingConvention`, tag-based versions, lazy clones, `fetch_dep_manifest` reads manifest without cloning.
- **`feat(registry): MultiRegistryResolver`** — priority + override + mirror schema; identity verification on overrides; `mirrors_for(name)` accessor for Phase B; `refresh_lockfile_clones` for `vibe registry sync`.
- **`refactor(registry): provenance through CachedPackage`** — `registry_name`/`source_ref`/`resolved_commit`/`overridden` flow from registry into lockfile.
- **`feat(install): switch CLI to MultiRegistryResolver`** — `git+` prefix stripping at backend boundary; e2e test rewritten for per-package fixture.
- **`feat(registry): per-package vibe registry sync`** — walks lockfile, refreshes per-package clones; legacy / override / unattributed entries reported correctly.
- **`feat(vibe-resolver): DepSolver trait + NaiveDepSolver`** — DFS solver with capability/obsoletes/conflicts/disjunction handling; `MultiRegistryProvider` and `LocalRegistryProvider` adapters; resolvo / libsolv slots reserved.
- **`feat(install): transitive install via NaiveDepSolver`** — `vibe install` now drives the solver end-to-end; lockfile `dependencies` populated with exact pins; `[meta].root_dependencies` carries user-typed roots.
- **`feat(vibe-publish): RepoCreator + GitVerseCreator + vibe registry publish`** — Gitea-compatible HTTP client (reqwest+rustls); `Token` redaction; `Publisher` orchestrator; CLI subcommand with `--dry-run`. Live API verification deferred to first real publish.
- **`build(tools): JTD codegen scaffolding`** — `xtask` crate, `tools/jtd-codegen/` README + gitignore, first JTD schema, `crates/vibe-wire/` placeholder, `.cargo/config.toml` alias.
- **`chore(fixtures): relocate packages/ → fixtures/registry/`** — `git mv`, history preserved; `packages/` reserved for future dogfooding.
- **`test(manual): M1.5-gate-v2-per-package-smoke.md`** — protocol for the live three-package smoke against the new `vibespecs` org. Fill in "Last known pass" on first successful run.
- **`feat(vibe-publish): correct GitVerse API surface from live probing`** (commit `36cbf08`, 2026-04-26) — base URL `api.gitverse.ru`, Bearer auth, versioned Accept header, dry-run UX fix on the publisher. Live API discovery findings documented inline in `gitverse.rs` doc-comment so future readers don't re-walk the rabbit hole.
- **`docs(claude,agents,gemini): session-end checkpoint command spec`** (2026-04-26) — `ЗАВЕРШИ СЕССИЮ` / `END SESSION` and variants now drive a defined wind-down: overwrite `CONTINUE.md`, update this WAL, commit + push, emit TL;DR. Section lives at the bottom of all three boot files (kept byte-identical).
- **`docs(continue): cold-resume checkpoint at root`** (2026-04-26) — comprehensive `CONTINUE.md` written so any next session can pick up Phase A from cold without re-deriving GitVerse API findings, repo map, or decision history.

### Phase A close-out — live migration to GitHub (2026-04-29)

- **`docs(spec,guides,manual-tests): migrate registry org to GitHub`** (`72dae08`) — PROP-000 §7 split-host posture (vibevm source on GitVerse, registry org on GitHub), PROP-000 §20 token-secrecy invariant, PROP-002 §2.10 host-adapter selection + `RepoCreator::push_url` + per-host token loader, WAL/boot 90-user/ROADMAP/RUNTIME-GUIDE/DEV-GUIDE/docs/commands updates, manual-test rewritten for the GitHub host.
- **`feat(vibe-publish,cli): GitHub host adapter and per-host token loader`** (`ab0a3d4`) — `GitHubCreator` against `https://api.github.com` with the canonical `Accept: application/vnd.github+json` and `X-GitHub-Api-Version: 2022-11-28` headers, scope-guarded `RepoCreator::expected_org` / `validate_scope`, `creator_for_url(...)` factory, per-host token-file precedence (`~/.vibevm/github.publish.token` first, legacy `git.publish.token` last), CLI host-aware adapter selection.
- **`feat(core,cli): rotate DEFAULT_REGISTRY_URL to GitHub vibespecs`** (`39a2152`) — single-source-of-truth constant moves to `https://github.com/vibespecs`; default registry name from `default` to `vibespecs`.
- **`fix(vibe-publish): redact credentials from git error messages`** (`6e1bb3a`) — `redact_credentials(s)` helper closes a leak vector where `args.join(" ")` and `clone_url.to_string()` baked credentialed push URLs into `PublishError::Git` / `PushDenied` / `HostUnreachable` / `TagCollision` variants. Six unit tests pin the redaction.
- **`fix(vibe-registry): clone fallback and tag-aware update for GitHub`** (`86dfae3`) — two latent M1.1-revision bugs surfaced by GitHub: `git archive --remote` is not exposed by GitHub (returns HTTP 422 + flush-packet), so `fetch_dep_manifest` now falls back to a per-package shallow clone on `ArchiveUnsupported`; `update()` couldn't reset to a tag because `origin/<tag>` doesn't exist as a remote-tracking branch, so it now fetches with `--tags` and tries `refs/tags/<ref>` before `origin/<ref>`.
- **Live migration applied (3 publishes):** `https://github.com/vibespecs/flow-wal`, `flow-sync-from-code`, `flow-atomic-commits` each tagged `v0.1.0`. Token loaded from `~/.vibevm/github.publish.token`, never displayed. End-to-end smoke verified: anonymous `vibe init` → install all three → lockfile v2 with `registry = "vibespecs"` / GitHub `source_url`s / `content_hash`s populated; `vibe registry sync` refreshes 3, skips 0; `vibe list` shows three packages.

## Next

**Phase A is closed.** M1.6 (Phase B of the decentralized-registry refactor — polished multi-registry / mirror dispatch, `vibe vendor` for offline mirrors, richer publish adapters) becomes the next active milestone. M1.2 (`vibe update`), M1.3 (`vibe check`), M1.4 (`vibe show …`) are open in their original positions in the roadmap. M1.5-gate docs landed.

**Optional follow-up tasks.** Useful but not required to declare Phase A complete:

- Smoke-test Last-known-pass line in [`manual-tests/M1.5-gate-v2-per-package-smoke.md`](../manual-tests/M1.5-gate-v2-per-package-smoke.md) — the manual protocol still says "TBD" since the in-session smoke ran an automated bash equivalent, not the full markdown protocol.
- Schedule a recurring agent to verify the `vibespecs` org on GitHub stays reachable and `v0.1.0` tags don't drift (peeled SHAs as of 2026-04-29: `flow-wal` `1c3a1355`, `flow-sync-from-code` `a620157d`, `flow-atomic-commits` `d76512034`).

Comprehensive cold-resume document (long form, with repo map, decision history, exact recipes) lives at [`CONTINUE.md`](../CONTINUE.md). It is written by the session-end checkpoint command (`ЗАВЕРШИ СЕССИЮ` / `END SESSION`) and supersedes itself wholesale on each invocation; if it disagrees with this WAL, trust the WAL.

**Beyond Phase A.** M1.6 polishes multi-registry / mirror dispatch / `vibe vendor` per [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md#phase-b). M1.5-gate docs (`docs/commands/*.md`, `docs/authoring-{flow,feat,stack}.md`) all landed.

## Known issues

- **Legacy lockfile v1 auto-migration UX.** Every project with an existing `vibe.lock` from M1.1 will see a migration notice on next `vibe install`. Behaviour benign (resolution unchanged); message must be actionable, not noisy.
- **Line-ending warnings** on every commit — `.gitattributes` with `* text=auto eol=lf` side-quest still open.
- **Registry cache locking** — two concurrent `vibe` invocations can race on the same per-package clone directory. Noted in PROP-001 §6 as M2 hardening; behaviour today: if a clone fails, delete the cache dir and retry.
- **Path display on Windows** strips `\\?\` UNC prefixes; lockfile stores forward-slash relative paths (portable).

## Session context

- **Entry point for next session:** read `CLAUDE.md`, then this WAL, then [PROP-000](common/PROP-000.md) and [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md); consult [`TASKS.md`](../TASKS.md) for the current queue. The remaining Phase A item is the live migration — see "Next" above for the procedure.
- **Do NOT touch:** `VIBEVM-SPEC.md` (owner-frozen — the approved PROP-002-driven amendments landed in the documentation slice; any further edit needs a new owner sign-off), `refs/book/**`, `spec/boot/00-core.md`, `spec/boot/90-user.md`, any `fixtures/registry/flow/<name>/v0.1.0/` snapshot (canonical test payloads — changes must be a new version).
- **Key commands to know:**
  - `cargo test --workspace` — 169+ tests green on `main` at checkpoint.
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  - `cargo xtask codegen` — regen JTD-derived Rust types (requires `tools/jtd-codegen/` install per its README).
  - `cargo xtask check-codegen` — drift check; CI uses this once a schema is wired into a real consumer.
  - `cargo run -p vibe-cli -- init --path <dir>` — scaffold a project.
  - `cargo run -p vibe-cli -- install flow:wal --path <project>` — transitive resolve via `NaiveDepSolver`, populated lockfile v2 entry.
  - `cargo run -p vibe-cli -- registry publish <path> [--registry <name>] [--dry-run]` — publish a package (maintainers; reads token from `~/.vibevm/<host>.publish.token`, value never echoed).
  - `cargo run -p vibe-cli -- registry sync --path <project>` — refresh per-package clones referenced by the lockfile.
