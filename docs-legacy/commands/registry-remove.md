# `vibe registry remove` — drop a `[[registry]]` or `[[mirror]]` from `vibe.toml`

Mutates `vibe.toml` to remove either a registry block (by name) or a mirror block (by exact `(of, url)` match). The two targets are spelled as subsubcommands so the CLI surface is unambiguous and shell completion works cleanly:

```
vibe registry remove registry <NAME>            # remove a [[registry]]
vibe registry remove mirror   <OF> <URL>        # remove a [[mirror]]
```

`vibe registry remove` is manifest-only. It does not delete cache state on disk, does not touch the lockfile, does not contact the host. Lockfile entries that name a removed registry remain on disk; their `vibe registry sync` becomes a skipped entry until the registry is re-added or the package is re-installed against a different one.

## Usage

```
vibe registry remove registry <NAME>
                              [--path <DIR>]
                              [--json | --quiet]

vibe registry remove mirror <OF> <URL>
                            [--path <DIR>]
                            [--json | --quiet]
```

## Arguments

| Argument | Description |
| --- | --- |
| `<NAME>` | Name of the `[[registry]]` block to remove. Must match an existing `[[registry]].name`. |
| `<OF>` | `[[mirror]].of` of the entry to remove. Use `"*"` for the wildcard form. |
| `<URL>` | `[[mirror]].url` of the entry to remove. Exact string match. |

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project directory containing `vibe.toml`. | `.` |
| `--json` | Structured payload. | off |
| `--quiet` | One-line summary. | off |

## Output shape — human

```
  → Removed `[[registry]]` `vibespecs`

vibe registry remove: 1 registry remain.
```

```
  → Removed `[[mirror]]` of=`*` url=`https://offline.example/cache`

vibe registry remove: 0 mirrors remain.
```

## JSON shape

```json
{
  "ok": true,
  "command": "registry:remove",
  "target": "registry",
  "identity": "vibespecs",
  "total_registries": 0,
  "total_mirrors": 1
}
```

For mirror removal, `target = "mirror"` and `identity = "<of>:<url>"`. Both forms include `total_registries` and `total_mirrors` after the operation.

## Errors

- **Unknown registry name** (registry form) — exits non-zero with the list of known names. Pick from the list or `vibe registry add` it.
- **Removing a `[[registry]]` that named mirrors target** (registry form) — exits non-zero. The error names every mirror URL that would be orphaned and points at the `vibe registry remove mirror` invocation that fixes them. Wildcard `of = "*"` mirrors are unaffected by registry removal and do not block it.
- **No matching mirror** (mirror form) — exits non-zero. The matcher is `(of, url)` exact; `vibe registry list` shows the canonical spellings.
- **Multiple matching mirrors** (mirror form) — succeeds, drops them all, and prints a `warning:` line on stderr noting how many were removed. Should not happen for manifests written by `vibe registry set-mirror` (which refuses exact `(of, url)` duplicates), but a hand-edited `vibe.toml` may carry them.
- **No `vibe.toml`** — exits non-zero. Run `vibe init` first.

## Exit codes

- `0` — success.
- `1` — validation failure, I/O error on `vibe.toml`, or no project at `--path`.

## Related

- [`vibe registry list`](registry-list.md) — inspect what's there before removing.
- [`vibe registry add`](registry-add.md) / [`vibe registry set-mirror`](registry-set-mirror.md) — the inverse operations.
- [`PROP-002 §2.5`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md) — full schema for `[[registry]]` / `[[mirror]]` / `[[override]]`.
