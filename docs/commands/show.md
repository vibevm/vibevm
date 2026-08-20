# `vibe show`

Inspect computed project state.

## Usage

```text
vibe show [OPTIONS] <COMMAND>
```

## Subcommands

| Command | Purpose |
| --- | --- |
| `vibe show effective` | Print the effective spec with stable `spec://` provenance headers. |
| `vibe show config` | Print effective configuration with per-value provenance. |
| `vibe show features` | Print active lockfile features by package. |
| `vibe show subskills` | Print active subskills, delivery modes, and `describes` PURLs. |
| `vibe show purls` | Print the Package URLs bound by the lockfile. |

The common `--json`, `--quiet`, `--invoked-by`, `--unattended`, and `--offline` options are available. Run `vibe show <command> --help` for child-specific options such as project path selection.

## Examples

```bash
vibe show effective
vibe show config --json
vibe show features
vibe show subskills
vibe show purls
```

## Related

- [`vibe list`](list.md)
- [`vibe tree`](tree.md)
- [`vibe check`](check.md)

