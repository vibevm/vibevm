# vibevm documentation

Operator documentation for vibevm 1.0.0. The live binary remains the authority for CLI syntax: use `vibe <command> --help` alongside these pages.

## Start here

- [`../README.md`](../README.md) — installation, quick start, registry configuration, cache/offline workflow, and contributor entry points.
- [`ALPHA-NOTES.md`](ALPHA-NOTES.md) — why 1.0.0 may break and how to recover after a breaking update.
- [`../CHANGELOG.md`](../CHANGELOG.md) — release and milestone history.
- [`architecture.md`](architecture.md) — contributor-facing crate and data-flow map.
- [`loading-model.md`](loading-model.md) — authored specs, `vibedeps/`, and computed boot lanes.

## Core commands

| Command | Page |
| --- | --- |
| `vibe init` | [`commands/init.md`](commands/init.md) |
| `vibe install` | [`commands/install.md`](commands/install.md) |
| `vibe update` | [`commands/update.md`](commands/update.md) |
| `vibe uninstall` | [`commands/uninstall.md`](commands/uninstall.md) |
| `vibe list` | [`commands/list.md`](commands/list.md) |
| `vibe search` | [`commands/search.md`](commands/search.md) |
| `vibe check` | [`commands/check.md`](commands/check.md) |
| `vibe cache` | [`commands/cache.md`](commands/cache.md) |
| `vibe registry` | [`commands/registry.md`](commands/registry.md) |
| `vibe show` | [`commands/show.md`](commands/show.md) |
| `vibe tree` | [`commands/tree.md`](commands/tree.md) |
| `vibe mcp` | [`commands/mcp.md`](commands/mcp.md) |

Every page above was checked against the 1.0.0 binary's live `--help` output. Focused subcommand pages elsewhere in `docs/commands/` are older long-form references; consult live help when they disagree.

## Focused operator guides

- [`version-syntax.md`](version-syntax.md) — package constraints and exact pins.
- [`git-source-dependencies.md`](git-source-dependencies.md) — direct git package declarations.
- [`registry-auth.md`](registry-auth.md) — private-registry authentication modes.
- [`registry-redirect.md`](registry-redirect.md) — registry-side external package redirects.

## Machine-readable inventory

[`SITE-MANIFEST.toml`](SITE-MANIFEST.toml) is the curated set a website agent should ingest. It intentionally excludes pages that still describe retired pre-1.0 paths, versions, or deferred features.
