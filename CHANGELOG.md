# Changelog

vibevm has not shipped a stable release yet — every commit on `main` is part of the path to v0.1.0. This file is a curated chronicle of what landed when, organised by milestone rather than commit-by-commit. The single source of truth for "what changed in this commit" is `git log`; this file is the source of truth for "what does this milestone mean".

Format roughly follows [Keep a Changelog](https://keepachangelog.com/), grouped by milestone. The "Unreleased" section accumulates work-in-progress.

---

## [Unreleased]

### PROP-005 — Package index (`vibe-index`) (2026-05-22)

A registry-org metadata index, so `vibe` can `list` / `search` / `outdated` / shortlist versions against cached, mirror-able metadata instead of a live `git ls-remote` per package. Design lock: [PROP-005](spec/modules/vibe-index/PROP-005-package-index.md). The index layer is strictly optional — a registry without one keeps working through the live path unchanged. This block records work that landed across earlier sessions but was never written up here, plus the 2026-05-22 reconciliation that brought it green.

- **`vibe-index` — server + CLI** (`crates/vibe-index/`, PROP-005 slices 1–8). One binary, two modes: a CLI that builds an index data directory from a registry's package repos (`reindex --from-clones` / `--from-github`, plus `init` / `add` / `remove` / `get` / `list` / `search` / `capabilities` / `purls` / `outdated` / `verify` / `dump`), and an axum HTTP server (`serve`) exposing the same surface — read routes open, write routes bearer-token-authed, with a per-token / per-IP rate limiter. The on-disk format is RPM-shaped: a `repomd.json` manifest over `primary.jsonl`, `by-name/`, `by-cap/`, `by-purl/`.
- **Publisher hook** (slice 9). `vibe registry publish` optionally POSTs the new entry to the registry's index after a successful push; failure is non-fatal — the next `reindex` covers the gap.
- **Consumer fast path** (slice 10). `vibe-registry` gains an `IndexClient`: before falling back to per-repo `git ls-remote`, the resolver consults `<registry>/repomd.json` and reads `by-name/<kind>/<name>.json` for the version shortlist. Index entries stay advisory — `content_hash` is still verified against the fetched bytes, so a stale or hostile index can mislead version selection but never substitute content.
- **`vibe search`** (M2.10). Index-backed package search across configured registries, `--purl` lookup for "what describes this upstream library?", a `--full-scan` indexless fallback, results cached under `~/.vibe/search-cache/`.
- **2026-05-22 de-rot.** `vibe-index` was then a standalone Cargo workspace, outside the routine `cargo test --workspace` gate, and it had silently rotted against the M1.17 unified manifest and the M1.18 loading model: the duplicated `vibe.toml` parser still expected `[writes]` / `[dependencies]` / `[boot_snippet].filename` and rejected every current manifest, and the `content_hash` parity test had drifted off a renamed fixture. The scanner was realigned with the current schema and made lenient against future additions; the golden fixture and parity hash were re-synced (cross-checked against the canonical `vibe-registry::compute_content_hash`); a current-schema scanner regression test was added; the dead slice-1 scaffolding was retired. The suite is green again — 169 tests, clippy `-D warnings` and rustfmt clean.

- **2026-05-22 fold.** The de-rot exposed the root cause (PROP-005 §9 item 11): a standalone workspace hand-duplicating a `vibe.toml` parser, nothing cross-checking it against `vibe-core`, and the workspace itself outside `cargo test --workspace`. The fix moved `vibe-index` from `services/vibe-index/` into `crates/vibe-index/` as a member of the vibevm workspace, deleted the duplicated parser, and switched the scanner to parse through `vibe-core`'s own `Manifest` / `SubskillManifest`. The index schema can no longer drift from the manifest schema, and the routine workspace gate now covers the crate. `tools/self-check.sh` drops its standalone-workspace special-case.

### M1.21 — Incremental install (PROP-011) (2026-05-22)

PROP-009's `vibe install` re-ran the depsolver and re-copied every `vibedeps/` slot on every invocation, whatever had changed. PROP-011 makes it incremental — the `cargo build` / `npm install` shape. Design lock: [PROP-011](spec/modules/vibe-workspace/PROP-011-incremental-install.md).

- **Skip resolution when fresh.** Before the depsolver runs, a freshness check asks whether `vibe.lock` is still a correct resolution of every node's `[requires]` — a cargo-style satisfiability test, no manifest digest stored. When it is, a bare `vibe install` skips the depsolver entirely: no registry walk, no network, just a whole-tree boot regeneration. This also makes `vibe install` **lockfile-respecting** — an unchanged `[requires]` honours the locked versions verbatim, ending the silent version drift the unconditional re-resolve caused.
- **Materialise only the diff.** Materialisation skips a `vibedeps/` slot already present for the resolved version — versions are immutable, so the content is correct; only a new or version-bumped dependency is copied. A `slot_integrity` key in the vibevm user config (`trust-presence` default, or `verify`) governs the skip; `verify` re-materialises every slot.
- **Minimum-churn re-resolution.** When `[requires]` *has* changed, `vibe install` re-resolves but holds the locked version of every dependency the change did not touch, so an untouched dependency never drifts; a held-pin conflict falls back to a full re-resolve.
- **No new flag, no schema bump.** `vibe update` and `vibe reinstall --force` stay the bypasses; `vibe.lock` is unchanged — the lockfile itself is the freshness baseline. `VIBEVM-SPEC.md` §9.1 records the lockfile-respecting contract.

Boot-artifact regeneration deliberately stays whole-tree — the cheap, self-healing phase. Skipping the registry walk for an unchanged subtree (true incremental re-resolution) is deferred to PROP-003's SAT solver. Every phase landed clippy-clean with its test suite green.

### M1.18 — Loading model (PROP-009 + PROP-012) (2026-05-22)

The flat `spec/boot/NN-*.md` boot model is replaced by a computed loading model. Design locks: [PROP-009](spec/modules/vibe-workspace/PROP-009-loading-model.md), [PROP-012](spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md). Shipped across seven phases.

- **Two trees.** A node's authored `spec/` and its materialised dependencies are physically separate. `vibe install` copies each resolved package's published tree verbatim into a committed `vibedeps/<kind>-<name>/<version>/` slot at the workspace root, and never writes into authored `spec/`. The `[writes]` package section is retired — a materialised package *is* its subtree.
- **Computed boot.** Each node's boot sequence is computed from the unified resolution — inherited foundation + the node's own authored boot + dependency boot + user overrides — and projected into generated `spec/boot/INLINE.md` (the verbatim inline priority lane) and `spec/boot/INDEX.md` (a TOML manifest of `static` / `dynamic` entries). Three inclusion types — `inline` / `static` / `dynamic` — are declared per dependency via `link`. The `NN-` filename prefix is retired; `vibe` owns ordering by `category` band.
- **OS-gated boot snippets.** A package's `[boot_snippet]` may declare a `when = "os:<name>"` condition (`windows` / `macos` / `linux`). `vibe` renders the contribution as a `dynamic` `INDEX.md` entry carrying the condition — irrespective of any `link`, since a condition cannot be inlined — and the agent reads the file at boot only on a matching operating system. This closes the dynamic-entry `when` gap PROP-009 §2.3 flagged at Phase 4. The same OS probe is reserved as `if_os` in the subskill `[activation]` vocabulary, so the two mechanisms share one grammar.
- **`vibe reinstall`.** Regenerates the materialised state and boot artifacts without re-resolving; `--force` re-fetches every locked package from source.
- **The managed `<vibevm>` block (PROP-012).** `vibe` no longer overwrites the whole of `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` — it owns only a delimited `<vibevm>` … `</vibevm>` block and preserves every byte outside it, so the developer and other tools can co-tenant the file. A malformed block is a hard error, validated before any mutation; `vibe install` maps it to exit code 3.
- **The boot-directory linter** (`vibe check`) stops enforcing the retired `NN-` filename pattern; it now verifies only that `spec/boot/` exists and holds markdown.
- **vibevm self-migration.** vibevm migrated to its own loading model — `spec/boot/INDEX.md` generated, a `<vibevm>` block appended to its `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (every hand-authored line, the four rules included, preserved).
- **Docs.** `VIBEVM-SPEC.md` §6 and the rest of the retired-model footprint rewritten for the loading model; the `docs/` sweep covers the new model and `vibe reinstall`.

Every phase landed clippy-clean (`-D warnings`) with its full test suite green.

### M1.17 — Workspace (multi-package projects) (2026-05-21)

The cargo-`[workspace]` / Maven-multi-module shape: a project decomposes into member packages, each published independently — or not at all. Design lock: [PROP-007](spec/modules/vibe-workspace/PROP-007-workspace.md). Shipped in five implementation phases plus documentation.

- **Unified manifest.** The two manifest files — `vibe.toml` and `vibe-package.toml` — collapse into **one file, `vibe.toml`**, carried by every node. The role is set by which sections are present: `[project]` (a consumer) and `[package]` (a publishable artifact) are mutually exclusive; `[workspace]` composes with either or neither. One `Manifest` type in `vibe-core` replaces `ProjectManifest` + `PackageManifest`; `Manifest::validate` enforces the role rules.
- **Hard compatibility break — all manifest legacy removed.** vibevm is pre-release; there is no migration path and none is needed. Gone: the `vibe-package.toml` filename, the `[dependencies]` section, the array-form `packages = ["…"]`, the singleton `[registry]` table, and the `vibe.lock` v1/v2/v3 readers. A manifest or lockfile using a removed form is a hard error.
- **Workspace model + discovery.** A new `vibe-workspace` crate. `Workspace::discover` walks up from anywhere inside the tree to the topmost `[workspace]` that transitively encloses the start node — the absolute root, where the single `vibe.lock` lives. `[workspace] members` accepts glob patterns; nesting recurses to arbitrary depth; nesting cycles are rejected. A standalone single-package project is a degenerate workspace, so discovery is the universal entry point.
- **Path-source dependencies.** A `[requires.packages]` entry may be `{ path = "../sibling", version = "^0.1" }` — a third source-kind beside registry-resolved and git-source. Resolution priority is `[[override]]` > path > git-source > registry-walk. `vibe.lock` bumps to schema v4 with `source_kind = "path"`, whose `source_url` is the member's path relative to the workspace root — portable, never absolute.
- **`[workspace.versions]` placeholders.** Named version-constraint placeholders (Maven `<properties>`). A member references one as `{ version.var = "core" }`; it resolves bottom-up through the enclosing-workspace chain, nearest wins.
- **Selective publish.** `[package].publish` (`true` / `false` / `["registry"]`) declares each node's posture. `vibe workspace publish` walks the self-publishing members dependency-first and publishes each as its own repository, reusing the per-package publish machinery; the development monorepo is never modified. Each published copy carries an `[origin]` provenance marker, a "generated copy — contribute upstream" README banner, and a `PULL_REQUEST_TEMPLATE.md` STOP notice.
- `VIBEVM-SPEC.md` §4.2 / §7 document the workspace model.
- **Not yet wired:** `vibe install` / `vibe build` do not yet discover the workspace for unified multi-member resolution — a follow-up milestone that turns on a per-member materialisation decision PROP-007 §2.4 / §3 leaves open. Standalone single-package projects — every project today — are unaffected.

Every phase landed clippy-clean (`-D warnings`) with its full test suite green.

### M1.16 closer — `vibe registry redirect-update` (2026-05-19)

Closes the v0 manual-procedure gap surfaced in the M1.16 ship-complete WAL: editing an existing stub's marker used to require a hand-driven `git clone` / edit / `git commit` / `git push` recipe. The new CLI command automates it.

- **`vibe registry redirect-update <pkgref>`** — partial-update CLI for an existing stub's `vibe-redirect.toml`. Each flag is optional; omitted fields retain their current value. Flags: `--to`, `--ref-policy`, `--pinned-ref`, `--target-auth`, `--target-token-env`, `--description`, `--clear-description`, `--registry`, `--trust-redirect`, `--resync`, `--path`, `--dry-run`. Refuses with `no changes requested` if the computed marker is byte-identical to the existing one — never records an empty commit on the stub's history.
- **Trust model.** Per PROP-002 §2.4.2, changes that alter what content consumers materialise (`target_url`, `ref_policy`, `pinned_ref`) require `--trust-redirect`. Operator-side metadata (`auth`, `token_env`, `description`) does not. Without `--trust-redirect` for a content-affecting change, the command bails with the list of trust-required fields detected and a pointer at the flag. Mirrors the `--trust-mirror` shape on `vibe install` / `vibe update`.
- **Auto-clearing cross-field invariants.** Switching `auth` away from `token-env` clears `token_env` automatically (otherwise the marker would fail to re-parse). Switching `ref_policy` to `pass-through-tag` clears `pinned_ref` automatically. Switching `ref_policy` to `pinned` without `--pinned-ref` (and without an existing pinned_ref to preserve) is a hard error.
- **JSON envelope.** `--json` emits `{ ok, command, registry, pkgref, stub_url, target_url, ref_policy, target_auth, changes: [{field, before, after}], trust_required, dry_run, sync? }`. The `changes` array is the canonical per-field diff (target_url, ref_policy, pinned_ref, auth, token_env, description). `trust_required` is `true` when any change in the diff touches a trust-gated field — CI gating can decide on manual review from this alone.
- **`vibe-publish::git_publish::commit_and_push`** — new helper for committing in-place on an existing clone and fast-forward-pushing `main` to a remote. Symmetric to `push_initial` but for the "existing clone" path: refuses to record an empty commit if `git status --porcelain` is empty after `git add -A`. 2 unit tests against a local bare origin.
- **15 new unit tests** for `compute_updated_redirect_section` + helpers: partial-update detection, clear-description, switch-to-pinned with and without explicit ref, switch-to-pass-through clears pinned_ref, auth flip clears token_env, all rejection paths (empty `--to`, `--pinned-ref` on pass-through, `--target-token-env` without matching auth, switching to pinned with no ref). 4 new hermetic e2e tests for args-level guard rails: `--help` flag coverage, `--description` + `--clear-description` mutual exclusion, invalid pkgref, missing `vibe.toml`.
- Docs: new `docs/commands/registry-redirect-update.md` (full operator reference). `docs/registry-redirect.md` gains a "Rewriting an existing stub's marker" section pointing at the new command; "Out of scope for v0" no longer lists this item. `docs/commands/registry-redirect.md` "Error surface" + pipeline references the new command instead of the manual procedure. `docs/README.md` index gets a new row.

The M1.16 deferred-list is now empty.

### Test fixture re-homing (2026-05-12)

End-to-end and smoke fixtures moved out of the canonical `vibespecs` org (and `olegchir` personal namespace) into dedicated test orgs. The canonical orgs now host only real, installable packages.

- **GitHub `vibespecstest1`** — registry-side test fixtures. Holds `flow-vibevm-github-smoke` (used by `cli_live_e2e::install_github_smoke_alone`) and `feat-helper` (M1.16 redirect stub).
- **GitHub `vibespecstest2`** — external-author / target-side test fixtures. Holds `vibevm-m1-smoke-flow-internal` (M1.15 git-source target), `vibevm-m1-smoke-feat-helper` (M1.16 redirect target), `vibevm-private-probe` (M1.14.4 private-probe smoke target, kept private).
- **GitVerse `vibespecstest3`** — GitVerse-side test fixtures. Holds `vibevm-direct-push-smoke` (used by `cli_live_e2e::install_gitverse_smoke_alone`). Reached over SSH; the live test uses `git@gitverse.ru:vibespecstest3` to bypass GitVerse's HTTPS-requires-auth posture on test repos.
- **`cli_live_e2e.rs`** — `init_project` now overwrites `vibe.toml` with `[[registry]]` blocks pointing at the test orgs. Asserts updated: `registry = "vibespecstest1"` and `"vibespecstest3"` instead of the canonical names. All three live tests still pass.
- **`manual-tests/M1.15-git-source-smoke.md`** / **M1.16-redirect-smoke.md** — recipes rewritten to provision repos via `POST /orgs/vibespecstest2/repos` (instead of `/user/repos` under `olegchir`); install steps write `[[registry]] url = "https://github.com/vibespecstest1"` after `vibe init` so the consumer routes through the test org.

### M1.15 — Git-source dependencies (2026-05-10)

The Cargo / npm / Poetry / Bundler / Go-modules-style affordance — declare a dep as `{ git = "https://...", tag = "v0.1.0" }` instead of resolving it through `[[registry]]`. Spec: [PROP-002 §2.4.1](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#git-source).

- **`[requires.packages]` table-form schema** in `vibe-core`: `Vec<PackageRef>` (legacy) and `BTreeMap<PackageRef, GitPackageDep>` (modern) parse transparently; round-trip writes the modern map. Inline-table values declare git-source: `"flow:internal-helper" = { git = "...", tag = "v0.1.0", auth = "token-env", ... }`.
- **`GitPackageRegistry::open_single_package`** — single-package URL constructor that bypasses `org_url + naming`. Reuses M1.14 token-injection / bootstrap-with-scrub plumbing.
- **`MultiRegistryResolver` short-circuits** the registry walk for any pkgref in `[requires.packages]` git-source declarations. Resolution priority: override > git-source > registry-walk.
- **`vibe install` flags** — `--git <url>`, `--tag/--branch/--rev`, `--git-auth`, `--git-token-env` add a git-source declaration without hand-editing `vibe.toml`.
- **Lockfile** — new `source_kind = "registry" | "git" | "override"` discriminant per `[[package]]`. Wire-compatible — `Option<SourceKind>` defaults to `None` for pre-M1.15 lockfiles.
- **Hermetic e2e** in `vibe-cli/tests/cli_e2e.rs` — install with `--tag`, install with `--branch`, repeat install rejection, uninstall removal from both `requires.packages` and `requires.git_packages`.
- **Production smoke walk** documented at `manual-tests/M1.15-git-source-smoke.md`. Validated against `https://github.com/vibespecstest2/vibevm-m1-smoke-flow-internal` — `git archive --remote` → shallow-clone fall-back exercised on the GitHub case. Smoke fixtures live in dedicated test orgs (`vibespecstest1/2/3`) so the canonical `vibespecs` org stays populated with real packages only.
- **Bug fix** along the way: `fetch_manifest_at_ref` (used by git-source path) now falls back to `refresh_package` when the host refuses `upload-archive`, matching `fetch_dep_manifest`. Without this, GitHub-hosted git-source targets failed at resolution time.
- **Bug fix**: `vibe uninstall <pkgref>` now removes the entry from BOTH `requires.packages` and `requires.git_packages` (was a one-list-only walk).
- Docs: new `docs/git-source-dependencies.md` operator reference (in M1.15 spec landing); `docs/commands/install.md` extended with the new flags.

### M1.16 — Registry redirect (delegated package via stub repo) (2026-05-10)

The Linux-distro-style virtual-package mechanism — a registry org's stub repo carries `vibe-redirect.toml` pointing at an external git repo where the package's actual content lives, instead of carrying the content directly. Spec: [PROP-002 §2.4.2](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect).

- **`vibe-redirect.toml` schema** in `vibe-core::manifest::redirect`: `[redirect]` block with required `target_url`, optional `ref_policy = pass-through-tag | pinned`, `pinned_ref` (required iff pinned), `auth` / `token_env` (target-side, mirrors PROP-002 §2.2.1), `description`. Mutually exclusive with `vibe-package.toml` at the same ref (`AmbiguousStub`). 11 unit tests.
- **`MultiRegistryResolver::follow_redirect`** — resolver detects the marker after a registry-walk success, opens a synthetic single-package registry on `target_url`, fetches manifest at the pass-through-tag (or `pinned_ref`). Hop limit = 1: target cannot itself be a stub. `MultiResolution.via_redirect` carries the stub URL through the resolve→fetch boundary; `redirect_target_auth` / `redirect_target_token_env` propagate target-side auth.
- **`MultiRegistryResolver::fetch_manifest`** — new redirect-aware DepProvider entry point. Reuses `resolve()` to converge on the same `MultiResolution` the install pipeline already saw, then reads the manifest from whichever URL the resolution recorded (target for redirects, declared URL for git-source, registry's URL otherwise).
- **`fetch_via_redirect`** — clones target into `<cache>/__redirects__/<kind>-<name>/clone/`, distinct from registry / override / git-source cache tiers. Token-discipline preserved.
- **`try_fetch_redirect_for_url`** — two-path read: `git archive` first (cheap, file://-friendly), shallow-clone fall-back when the host refuses `upload-archive` (the GitHub case). Marker-first hop check fires before manifest fetch so chain rejection works against stub-only target repos.
- **`vibe registry redirect <pkgref> --to <url>`** — CLI helper that creates the stub repo automatically through the `RepoCreator` infrastructure. Flags: `--ref-policy`, `--pinned-ref`, `--target-auth`, `--target-token-env`, `--description`, `--sync` (mirror target tags immediately), `--dry-run`. Refuses if the stub already exists (editing is manual for v0).
- **`vibe registry redirect-sync <pkgref>`** — mirrors target tags into the stub. Reads stub's `vibe-redirect.toml`, lists target tags, pushes the missing ones. Refuses for `pinned`-policy stubs (semantically meaningless to sync).
- **Lockfile** — `via_redirect` field per `[[package]]` records the stub URL; `source_url` carries the target URL; `source_kind = "registry"` (redirect-resolved packages came through a registry stub, just delegated).
- **Hermetic e2e** in `vibe-cli/tests/cli_e2e.rs` — pass-through-tag install, pinned-policy install, identity-mismatch reject, hop-limit chain reject. Plus 9 helper unit tests for `parse_target_auth`, `build_redirect_readme`, `derive_target_token_env`, `inject_token_into_url`, `build_target_fetch_url`.
- **Production smoke walk** documented at `manual-tests/M1.16-redirect-smoke.md`. Validated against a `vibespecstest1/feat-helper` stub → `vibespecstest2/vibevm-m1-smoke-feat-helper` target pair on real GitHub: `vibe registry redirect`, `vibe registry redirect-sync`, then `vibe install feat:helper@^0.1` resolving through the stub. Lockfile records `via_redirect = "https://github.com/vibespecstest1/feat-helper.git"` and `source_url = "https://github.com/vibespecstest2/..."`. Smoke fixtures live in dedicated test orgs so the canonical `vibespecs` org stays populated with real packages only.
- Docs: new `docs/commands/registry-redirect.md` and `docs/commands/registry-redirect-sync.md`; `docs/registry-redirect.md` updated with the CLI workflow (manual procedure kept as a fallback section).

### v0.1.0-ready package-management bundle — 2026-05-08

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
