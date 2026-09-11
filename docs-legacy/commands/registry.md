# `vibe registry`

Inspect, configure, test, synchronize, publish, redirect, and export package registries.

## Usage

```text
vibe registry [OPTIONS] <COMMAND>
```

## Subcommands

| Command | Purpose |
| --- | --- |
| `sync` | Force a fetch of configured registry clones. |
| `publish` | Publish a package directory into a configured registry organization. |
| `list` | Print `[[registry]]`, `[[mirror]]`, and `[[override]]` declarations. |
| `add` | Add a `[[registry]]` block to `vibe.toml`. |
| `set-mirror` | Add a `[[mirror]]` block. |
| `remove` | Remove a registry or mirror declaration. |
| `test` | Probe reachability and authentication without fetching or writing. |
| `redirect` | Create an external-target registry stub. |
| `redirect-sync` | Mirror missing target tags into a pass-through stub. |
| `redirect-update` | Partially update an existing redirect marker. |
| `vendor` | Export the locked package set as a local `file://` mirror. |

The common `--json`, `--quiet`, `--invoked-by`, `--unattended`, and `--offline` options are available. Publishing and redirect mutation require the configured authentication; never put credential values in documentation or command logs.

## Examples

```bash
vibe registry list
vibe registry test
vibe registry sync
vibe registry vendor --help
```

An optional `index_url` field belongs directly in each `[[registry]]` block. Run `vibe registry <command> --help` for the exact child command and see the existing focused pages in this directory for detailed operator workflows.

## Related

- [`vibe search`](search.md)
- [`vibe install`](install.md)
- [`vibe cache`](cache.md)

