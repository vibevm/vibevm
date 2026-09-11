# `vibe list`

List packages recorded in the project's lockfile.

## Usage

```text
vibe list [OPTIONS]
```

## Options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Select the project directory; default `.`. |
| `--kind <KIND>` | Filter to `flow`, `feat`, `stack`, `tool`, `mcp`, or `lang`. |
| `--verbose` | Append active features and subskill paths in text output. |
| `--offline` | Forbid network access for the invocation. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available. JSON already carries the fields exposed by `--verbose`.

## Examples

```bash
vibe list
vibe list --kind flow --verbose
vibe list --json --path ./my-project
```

## Related

- [`vibe show`](show.md)
- [`vibe tree`](tree.md)
- [`vibe check`](check.md)

