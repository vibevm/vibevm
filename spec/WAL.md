# WAL — Project Continuation State
_Updated: 2026-04-25_

## Current phase

**M1.1-revision — decentralized per-package registry refactor. Phase A code slice complete; live migration of the three demo packages into the `vibespecs` org is the only remaining step.**

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

**Immediate next work:** the only outstanding Phase A item is the **live migration** of the three v0.1.0 demo flows into per-package repos under `vibespecs/<kind>-<name>` via `vibe registry publish`. This is **non-routine** per CLAUDE.md Rule 4 (creates real artefacts in a public org and exercises the GitVerse API for the first time), so it requires explicit owner sign-off before push. Once those three repos exist on GitVerse and the smoke-test [`M1.5-gate-v2-per-package-smoke.md`](../manual-tests/M1.5-gate-v2-per-package-smoke.md) passes, `DEFAULT_REGISTRY_URL` rotates to the new org root and Phase A closes.

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
- **Registry default in `vibe init`.** New projects scaffold `[[registry]] name = "vibespecs" url = "git@gitverse.ru:vibespecs"` — ORG root (not a package repo). Single source of truth: `vibe_core::manifest::DEFAULT_REGISTRY_URL`. Override with `--registry-url <URL>` / `--registry-ref <REF>`; opt out with `--no-registry`.
- **Manual-test protocol:** runnable smoke-tests in [`manual-tests/`](../manual-tests/), one file per scenario, clean-slate setup + teardown. Policy in [PROP-000 §14](common/PROP-000.md#manual-tests).
- **REVIEW marker discipline:** when the spec is silent, pick the conservative interpretation, mark with `<!-- REVIEW: … -->`, surface in the session report.
- **`refs/` not committed.** Upstream reference material (book + cloned study repos).

## Remotes

- **vibevm source (this repo):** `git@gitverse.ru:anarchic/vibevm.git` (SSH) / `https://gitverse.ru/anarchic/vibevm` (web).
- **Package registry (target for M1.1-revision):** organization `vibespecs` on GitVerse — `git@gitverse.ru:vibespecs/<kind>-<name>.git` per package. Empty today; Phase A populates it via `vibe registry publish`.
- **Legacy package registry (read-only transition):** `git@gitverse.ru:anarchic/vibespecs.git`. Holds three v0.1.0 flows in monorepo form (HEAD `2203239`, 2026-04-23). No new publishes here; migrated into per-package repos during Phase A; kept readable for existing projects with schema-v1 lockfiles until they migrate.
- **Publish token (local):** `~/.vibevm/git.publish.token` or `VIBEVM_PUBLISH_TOKEN` env. Verified by the owner as having `repo:create` rights in the `vibespecs` organization.

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

## Next

**Live migration of the three v0.1.0 demo flows into `vibespecs/<kind>-<name>` via `vibe registry publish`.** Non-routine (creates real public artefacts, first GitVerse-API exercise). Procedure once approved:

1. Build release: `cargo build --release --workspace`.
2. Confirm the publish token at `~/.vibevm/git.publish.token` (or `VIBEVM_PUBLISH_TOKEN` env) has `repo:create` scope in the `vibespecs` org.
3. Dry-run each: `vibe registry publish fixtures/registry/flow/<name>/v0.1.0 --dry-run` for `wal`, `sync-from-code`, `atomic-commits`.
4. Apply: drop `--dry-run`, run for each in turn.
5. Walk [`manual-tests/M1.5-gate-v2-per-package-smoke.md`](../manual-tests/M1.5-gate-v2-per-package-smoke.md) end to end.
6. If smoke passes: rotate `DEFAULT_REGISTRY_URL` in `vibe-core::manifest::project` to `git@gitverse.ru:vibespecs`, update tests, ship the rotation as its own commit.
7. WAL / ROADMAP checkpoint Phase A complete; M1.6 becomes the next active milestone.

**Beyond Phase A.** M1.6 polishes multi-registry / mirror dispatch / `vibe vendor` per [PROP-002](modules/vibe-registry/PROP-002-decentralized-registry.md#phase-b). M1.5-gate docs (`docs/commands/*.md`, `docs/authoring-{flow,feat,stack}.md`) are still open and parallelisable.

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
  - `cargo run -p vibe-cli -- registry publish <path> [--registry <name>] [--dry-run]` — publish a package (maintainers; needs `~/.vibevm/git.publish.token`).
  - `cargo run -p vibe-cli -- registry sync --path <project>` — refresh per-package clones referenced by the lockfile.
