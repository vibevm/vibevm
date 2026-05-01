# `vibe registry list` — show configured registries, mirrors, overrides

Read-only inspector for the project's resolution configuration. Prints every `[[registry]]`, `[[mirror]]`, and `[[override]]` block from `vibe.toml`, the host adapter each registry would dispatch to (per [PROP-002 §2.10](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish)), and which mirrors fall through to which registry. Touches no state — purely a window onto `vibe.toml`.

## Usage

```
vibe registry list [--path <dir>]
                   [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project directory containing `vibe.toml`. | `.` |
| `--json` | Structured payload (see Schema below). | off |
| `--quiet` | One-line summary `vibe registry list: <N> registries, <M> mirrors, <K> overrides.` | off |

## Output shape — human

```
Registries (2; primary listed first)
  1. vibespecs (primary)
     url:     https://github.com/vibespecs
     org:     vibespecs
     host:    github.com (adapter: github)
     naming:  kind-name
     ref:     main
     mirrors:
       - of=`vibespecs` priority=10 url=https://github-mirror.example.com/vibespecs
       - of=`*` priority=20 url=https://offline-mirror.example.com
  2. private
     url:     git@gitverse.ru:somecorp
     org:     somecorp
     host:    gitverse.ru (adapter: gitverse)
     naming:  kind-name
     ref:     main
     mirrors:
       - of=`*` priority=20 url=https://offline-mirror.example.com

Overrides (1)
  flow:wal → https://github.com/me/flow-wal-fork@v0.1.0-fork — testing fork during M1.6 development

vibe registry list: 2 registries, 3 mirrors, 1 override.
```

The first registry in the list is the **primary** — the one `vibe registry publish` defaults to when `--registry <name>` is omitted. The remaining registries serve as fall-through targets for `vibe install` per priority order.

The `adapter:` label is the host-specific publish adapter `vibe registry publish` would dispatch to. `none (publish unsupported)` means the host has no `RepoCreator` impl in this build; install/sync still work for that host (they shell out to plain `git`), but `vibe registry publish` will refuse with `UnsupportedHost`.

A `[[mirror]]` with `of = "*"` attaches to **every** registry — it appears under each registry's `mirrors:` block. A mirror with `of = "<name>"` attaches only to the registry whose `name` matches. Mirrors are sorted by priority ascending (lower = tried first); ties hold the order in `vibe.toml`.

## JSON shape

```json
{
  "ok": true,
  "command": "registry:list",
  "registries": [
    {
      "name": "vibespecs",
      "url": "https://github.com/vibespecs",
      "ref": "main",
      "naming": "kind-name",
      "host": "github.com",
      "org": "vibespecs",
      "adapter": "github",
      "mirrors": [
        { "of": "vibespecs", "url": "...", "priority": 10 },
        { "of": "*",         "url": "...", "priority": 20 }
      ]
    }
  ],
  "mirrors": [
    { "of": "vibespecs", "url": "...", "priority": 10 },
    { "of": "*",         "url": "...", "priority": 20 }
  ],
  "overrides": [
    {
      "pkgref": "flow:wal",
      "source_url": "...",
      "ref": "v0.1.0-fork",
      "reason": "..."
    }
  ]
}
```

`registries[].adapter` is `"github"`, `"gitverse"`, or `null` (host has no adapter). The top-level `mirrors` is the raw list straight from `vibe.toml` — useful when you want the wildcard / per-registry attribution split that the per-registry `mirrors` already does for you. `overrides[].ref` and `overrides[].reason` are omitted when absent in the manifest.

## Examples

```bash
vibe registry list                                # human-readable
vibe registry list --json | jq '.registries[].name'    # registry names
vibe registry list --json | jq '.registries[] | select(.adapter == null)'   # registries without publish support
vibe registry list --quiet                        # one-line summary
```

## Exit codes

- `0` — success, including no-registries case.
- `1` — `vibe.toml` missing or unparseable, or `--path` points at a non-existent directory.

## Related

- [`vibe registry sync`](registry-sync.md) — refreshes per-package clones referenced by the lockfile.
- [`vibe registry publish`](registry-publish.md) — uses the primary registry's host adapter shown here.
- [`PROP-002 §2.5`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md) — full schema for `[[registry]]` / `[[mirror]]` / `[[override]]`.
