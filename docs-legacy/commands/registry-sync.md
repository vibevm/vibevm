# `vibe registry sync` — refresh per-package registry clones

Walks the project's `vibe.lock` and refreshes the on-disk clone of every package referenced by it — the per-package model's equivalent of `git fetch` against a single monorepo. Useful before a `vibe install` of a new version when you want the freshest tag list, and as a cache-warming step in CI.

## Usage

```
vibe registry sync [--path <dir>]
                   [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project directory containing `vibe.toml` and `vibe.lock`. | `.` |
| `--json` | Structured payload. Schema: [`schemas/registry_sync_report.jtd.json`](../../schemas/registry_sync_report.jtd.json). | off |
| `--quiet` | One-line summary `vibe registry sync: <N> refreshed, <K> skipped.` | off |

## What gets refreshed

For each entry in the lockfile:

| Entry shape | Refresh path |
| --- | --- |
| `registry: Some(name)` (per-package install through a `[[registry]]`) | The `[[registry]]` named here is looked up in `vibe.toml`. A `GitPackageRegistry` is built and `git fetch --prune` + `git reset --hard origin/<source_ref>` run against the per-package clone at `<cache>/<canonical-url-hash>/packages/<kind>-<name>/clone/`. |
| `overridden: true` (resolved through `[[override]]`) | The clone under `<cache>/__overrides__/<kind>-<name>/clone/` is refreshed against the override's `source_url` and `source_ref`. |
| `registry: None` and `overridden: false` (legacy installs, `--registry <path>` installs) | Reported as **skipped** with a reason. There's no per-package clone to refresh for these — the M0 local-directory model has no remote to fetch. |

## Examples

```bash
vibe registry sync                      # refresh everything in the lockfile
vibe registry sync --json | jq '.refreshed | length'   # how many were refreshed
vibe registry sync --quiet              # one-line summary
```

Run before installing a new version of an already-installed package:

```bash
vibe registry sync                      # pull new tags
vibe install flow:wal@^0.2 --assume-yes # now sees v0.2.x if upstream tagged it
```

## Edge cases

- **Empty lockfile** — exits `0` with a "lockfile is empty — nothing to refresh" note.
- **No `[[registry]]` in `vibe.toml`** — exits `0` with a "nothing to refresh" note. Override-only setups still won't trigger the registry walk; refresh those by re-running `vibe install`.
- **Per-package clone doesn't exist yet** — `bootstrap` is called instead of `update`. After `vibe registry sync` the clone is populated even if the package was never installed (matches "what's referenced by the lockfile").

## Exit codes

- `0` — success, including empty / nothing-to-refresh cases.
- `1` — git failure during refresh (network, auth, upstream gone), I/O error on cache directory.

## Related

- [`vibe install`](install.md) — installs a package; the first install of a package implicitly pulls its registry clone.
- [`PROP-001 §2.5`](../../spec/modules/vibe-registry/PROP-001-git-backend.md#freshness) — the implicit-update freshness TTL (default 1 hour) that `vibe install` honours; `registry sync` is the "force refresh now" override.
