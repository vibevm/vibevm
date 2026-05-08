# vibevm — user documentation

End-user reference for the `vibe` CLI and the package-authoring formats. For the full project specification and design decisions, see [`VIBEVM-SPEC.md`](../VIBEVM-SPEC.md) at the repo root and the PROP / FEAT documents under [`spec/`](../spec/).

## Commands

The `vibe` binary is the single entry point for every operation. Global flags `--json` (machine-readable output) and `--quiet` (one-line summary) work on every subcommand.

| Command | Purpose |
| --- | --- |
| [`vibe init`](commands/init.md) | Scaffold a new vibevm project tree. |
| [`vibe install`](commands/install.md) | Resolve and apply one or more packages from a registry. |
| [`vibe update`](commands/update.md) | Re-fetch installed packages, diff project files, apply the update. |
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
| [`vibe registry vendor`](commands/registry-vendor.md) | Generate a local mirror directory for offline / air-gapped installs. |
| [`vibe registry publish`](commands/registry-publish.md) | Maintainer-side: publish a package directory as a tagged release. |
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

## Registry authentication

[`registry-auth.md`](registry-auth.md) — how to authenticate against private registries. Four `auth` regimes (`none` / `token-env` / `credential-helper` / `ssh`), env-var conventions, what happens on 401 / 403 per regime (walk-vs-halt), token discipline, troubleshooting. Read this if you have a private vibespecs org or are seeing GUI credential popups during installs.

## Lockfile reference

[`lockfile-format.md`](lockfile-format.md) — exhaustive reference for `vibe.lock` v2. Field-by-field semantics, identity model, v1 → v2 migration, tooling examples (jq snippets), worked example.

## Troubleshooting

[`troubleshooting.md`](troubleshooting.md) — first-aid for every error `vibe` surfaces. Each entry: what you see, what it means, what to do. Covers install / registry / git-backend / publish / resolver / CLI error variants.

## Glossary

[`glossary.md`](glossary.md) — vocabulary reference for the project. Every term that has a specific meaning in vibevm — `kind`, `pkgref`, `capability`, `mirror`, `override`, `content_hash`, `transitive`, `user-owned`, etc. — defined in one place. Includes an "anti-vocabulary" of adjacent-ecosystem terms we deliberately don't use.

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
