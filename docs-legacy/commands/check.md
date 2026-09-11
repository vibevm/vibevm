# `vibe check`

Run the spec-consistency linter against a project tree.

## Usage

```text
vibe check [OPTIONS]
```

## Options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Select the project root; default `.`. |
| `--wal-max-age-hours <HOURS>` | Set the WAL staleness threshold; default `24`. |
| `--review-max-age-days <DAYS>` | Set the dated `REVIEW` marker threshold; default `14`. |
| `--offline` | Forbid network access for the invocation. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available.

## Examples

```bash
vibe check
vibe check --path ./my-project --wal-max-age-hours 48
vibe check --json
```

The command exits successfully when there are no error-severity findings. Warnings and informational findings remain visible without turning the result into an error.

## Related

- [`vibe show`](show.md)
- [`vibe tree`](tree.md)
- [`vibe install`](install.md)

