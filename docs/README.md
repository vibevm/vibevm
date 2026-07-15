# vibevm — user documentation

End-user reference for the `vibe` CLI and the package-authoring formats. For the full project specification and design decisions, see [`VIBEVM-SPEC.md`](../VIBEVM-SPEC.md) at the repo root and the PROP / FEAT documents under [`spec/`](../spec/).

## Commands

The `vibe` binary is the single entry point for every operation. Global flags `--json` (machine-readable output) and `--quiet` (one-line summary) work on every subcommand.

| Command | Purpose |
| --- | --- |
| [`vibe init`](commands/init.md) | Scaffold a new vibevm project tree. |
| [`vibe install`](commands/install.md) | Resolve and apply one or more packages from a registry. |
| [`vibe update`](commands/update.md) | Re-fetch installed packages, diff project files, apply the update. |
| [`vibe reinstall`](commands/reinstall.md) | Recompute the materialised `vibedeps/` tree and boot artifacts without re-resolving. |
| [`vibe check`](commands/check.md) | Run the spec-consistency linter against the project tree. |
| [`vibe show`](commands/show.md) | Inspect computed project state — effective spec, configuration. |
| [`vibe list`](commands/list.md) | Show the packages currently locked into the project. |
| [`vibe search`](commands/search.md) | Full-text query across the configured `[[registry]]` indexes (per PROP-005). |
| [`vibe uninstall`](commands/uninstall.md) | Remove a package and its files (user-owned files preserved). |
| [`vibe registry list`](commands/registry-list.md) | Show the project's configured `[[registry]]` / `[[mirror]]` / `[[override]]` blocks and the host adapter each registry dispatches to. |
| [`vibe registry add`](commands/registry-add.md) | Mutate `vibe.toml` to register a new `[[registry]]`. |
| [`vibe registry set-mirror`](commands/registry-set-mirror.md) | Mutate `vibe.toml` to add a `[[mirror]]` block targeting a registry (or `*` for any). |
| [`vibe registry remove`](commands/registry-remove.md) | Mutate `vibe.toml` to drop a `[[registry]]` or `[[mirror]]` block. |
| [`vibe registry sync`](commands/registry-sync.md) | Refresh per-package registry clones referenced by the lockfile. |
| [`vibe registry test`](commands/registry-test.md) | Probe each registry's reachability and auth status — read-only diagnostic. |
| [`vibe registry vendor`](commands/registry-vendor.md) | Generate a local mirror directory for offline / air-gapped installs. |
| [`vibe registry publish`](commands/registry-publish.md) | Maintainer-side: publish a package directory as a tagged release. |
| [`vibe registry redirect`](commands/registry-redirect.md) | Maintainer-side: create a registry stub that delegates a package to an external target URL (PROP-002 §2.4.2). |
| [`vibe registry redirect-sync`](commands/registry-redirect-sync.md) | Maintainer-side: mirror target tags into an existing redirect stub. |
| [`vibe registry redirect-update`](commands/registry-redirect-update.md) | Maintainer-side: rewrite an existing stub's `vibe-redirect.toml` (retarget, switch policy, edit description). |
| [`vibe workspace publish`](commands/workspace-publish.md) | Maintainer-side: publish a multi-package workspace's self-publishing members, each as its own repository, in dependency order (PROP-007 §2.7). |
| [`vibe mcp install`](commands/mcp-install.md) | Wire vibevm into a coding agent (Claude Code, Claude Desktop, Cursor, OpenCode, Codex) — writes per-agent MCP config + optional `vibevm` SKILL.md. Scope axes: project / user / both. Wizard-driven without flags; fully scriptable. |
| [`vibe mcp upgrade`](commands/mcp-upgrade.md) | Refresh existing vibevm integrations to the version shipped in this binary. Scans installed places, rewrites only the diverged ones; never creates new installations. |
| [`vibe mcp uninstall`](commands/mcp-uninstall.md) | Remove vibevm from one or more agents — drops the `vibevm` MCP block and deletes SKILL.md, foreign keys preserved. |
| [`vibe mcp status`](commands/mcp-status.md) | Read-only counterpart of `mcp install` / `mcp upgrade`; reports per-(agent × scope) MCP and SKILL.md drift without writing. |
| [`vibe mcp serve`](commands/mcp-serve.md) | Run the JSON-RPC MCP server over stdio. Agents invoke this themselves via the configs written by `mcp install`. |
| [`vibe version`](commands/version.md) | Print the binary's version. |

## Guides

End-to-end walkthroughs that compose multiple commands into a real scenario. Each guide is **dual-purpose** — copy-paste tutorial for new operators, and an integration-test checklist for vibevm releases (every guide ends with an acceptance section that names what must be true after a successful run).

| Guide | What it covers |
| --- | --- |
| [Quickstart: opencode + vibevm hello-world](guides/agent-mcp-quickstart-opencode.md) | Wire opencode to vibevm via MCP, install a skill, run hello-world prompts; integration-test gate for M1.7 slice 4. |

## Architecture overview

[`architecture.md`](architecture.md) — contributor-facing tour of the workspace: how the crates fit together, what each abstraction trait does (`GitBackend`, `Registry`, `MultiRegistryResolver`, `DepProvider` / `DepSolver`, `RepoCreator`), how the install / publish / sync pipelines walk, where to look in the source for what.

## Version syntax

[`version-syntax.md`](version-syntax.md) — how version constraints work everywhere a pkgref appears (`vibe install`, `[requires].packages`, `[provides].capabilities`, `[[override]]`). Caret / tilde / equal / range operators, the `vibe.toml` ↔ `vibe.lock` two-file model, the `--exact` flag, comparison with Cargo / npm / Poetry / Bundler. Read this if you've ever been surprised that `flow:wal@0.3.0` matched `0.3.5`.

## Git-source dependencies (whole-repo-as-package)

[`git-source-dependencies.md`](git-source-dependencies.md) — declare a dep as `{ git = "...", tag = "..." }` in `[requires.packages]` and the resolver fetches the package directly from that git repo, bypassing `[[registry]]`. Use when a single private/internal package doesn't justify a multi-package registry org. Cargo / npm / Poetry / Bundler shape. Spec: [PROP-002 §2.4.1](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#git-source).

## Registry redirect (delegated package via stub repo)

[`registry-redirect.md`](registry-redirect.md) — a registry org's stub repo carries `vibe-redirect.toml` pointing at an external git repo where the package's actual content lives. Org owner keeps namespace control while delegating hosting / PRs / permissions to a different team. Resolver follows the marker transparently; consumers see no difference from a direct registry-resolved package. Closest analogue: Linux distro virtual `Provides:` records. Spec: [PROP-002 §2.4.2](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect).

## Registry authentication

[`registry-auth.md`](registry-auth.md) — how to authenticate against private registries. Four `auth` regimes (`none` / `token-env` / `credential-helper` / `ssh`), env-var conventions, what happens on 401 / 403 per regime (walk-vs-halt), token discipline, troubleshooting. Read this if you have a private vibespecs org or are seeing GUI credential popups during installs.

## Lockfile reference

[`lockfile-format.md`](lockfile-format.md) — exhaustive reference for `vibe.lock` v4. Field-by-field semantics, identity model, schema versioning, tooling examples (jq snippets), worked example.

## Troubleshooting

[`troubleshooting.md`](troubleshooting.md) — first-aid for every error `vibe` surfaces. Each entry: what you see, what it means, what to do. Covers install / registry / git-backend / publish / resolver / CLI error variants.

## FAQ

[`faq/`](faq/README.md) — answers to real developer questions, written up as standalone pages. Where troubleshooting maps an error message to a fix, the FAQ answers "how do I …" and "why does vibevm …" questions. First entry: [resolving version conflicts](faq/version-conflicts.md) (unified resolution, `[[override]]`, forcing a version inside the tree).

## Glossary

[`glossary.md`](glossary.md) — vocabulary reference for the project. Every term that has a specific meaning in vibevm — `kind`, `pkgref`, `capability`, `mirror`, `override`, `content_hash`, `transitive`, `user-owned`, etc. — defined in one place. Includes an "anti-vocabulary" of adjacent-ecosystem terms we deliberately don't use.

## Loading model

[`loading-model.md`](loading-model.md) — how a vibevm project boots. The two trees (authored `spec/` vs the committed `vibedeps/` materialised-dependency tree), the per-node *computed* boot sequence, the generated `INLINE.md` / `INDEX.md` artifacts, the `inline` / `static` / `dynamic` link types, ordering by `category`, and the managed `<vibevm>` block vibevm owns inside `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`. Read this to understand what `vibe install` produces and what an agent reads at session start. Spec: [PROP-009](../spec/modules/vibe-workspace/PROP-009-loading-model.md) / [PROP-012](../spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md).

## Authoring a package

Three of the four package kinds in vibevm have their own authoring guide. The fourth (`tool`) is reserved for v2+ and not yet documented.

| Kind | Guide | Purpose |
| --- | --- | --- |
| `flow` | [authoring-flow.md](authoring-flow.md) | Discipline / process modules — protocols and conventions an AI session reads at boot. |
| `feat` | [authoring-feat.md](authoring-feat.md) | Functional features — the *what* of an application built from specs. |
| `stack` | [authoring-stack.md](authoring-stack.md) | Language / framework targets — the *how* feats compile against. |

## Related setup documentation

- [`RUNTIME-GUIDE.md`](../RUNTIME-GUIDE.md) — what an end-user needs on their machine to run `vibe`.
- [`DEV-GUIDE.md`](../DEV-GUIDE.md) — what a contributor to vibevm needs to clone, build, test, publish.
- [`manual-tests/`](../manual-tests/) — runnable smoke-tests used before tagging a milestone.
