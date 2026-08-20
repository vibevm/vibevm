# `vibe cache`

Operate on the machine-global package store at `~/.vibe/cache/`. The store works outside a project, grows only through explicit fetches, and is never evicted automatically.

## Usage

```text
vibe cache [OPTIONS] <COMMAND>
```

## Subcommands

| Command | Purpose |
| --- | --- |
| `vibe cache path` | Print the store root. |
| `vibe cache list` | List the offline-resolvable inventory. |
| `vibe cache add <PACKAGES>...` | Pre-warm packages and their dependency closure without touching a project lockfile or `vibedeps/`. |
| `vibe cache clean` | Remove entries selected by exactly one of `--all`, `--package`, or `--older-than`. |
| `vibe cache check` | Re-hash entries against their sidecars and report `ok`, `mismatch`, or `unrecorded`. |

`vibe cache check --repair` records missing sidecars and re-fetches mismatched entries at the same version. Advancing a package is [`vibe update`](update.md), not repair.

## Examples

```bash
vibe cache path
vibe cache list
vibe cache add org.vibevm.world/wal --path ./my-project
vibe cache check
vibe cache check --repair --path ./my-project
vibe cache clean --older-than 90
vibe cache clean --package org.vibevm.world/wal@1.0.0
```

The common `--json`, `--quiet`, `--invoked-by`, `--unattended`, and `--offline` options are available. `vibe cache clean --all` is confirmation-gated unless an accepted non-interactive option implies consent.

## Related

- [`vibe install`](install.md)
- [`vibe update`](update.md)
- [Alpha recovery notes](../ALPHA-NOTES.md)

