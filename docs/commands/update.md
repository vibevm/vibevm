# `vibe update`

Re-fetch and apply changes for selected installed packages, or for the whole lockfile.

## Usage

```text
vibe update [OPTIONS] [PACKAGES]...
vibe update --all [OPTIONS]
```

Named package references use `<kind>:<name>` and must already be installed. Named packages and `--all` are mutually exclusive.

## Options

| Option | Effect |
| --- | --- |
| `--all` | Update every package in `vibe.lock`. |
| `--path <PATH>` | Select the project directory; default `.`. |
| `--assume-yes` | Skip the confirmation prompt. |
| `--exact` | Tighten updated root constraints in `vibe.toml` to `=<resolved-version>`. |
| `--auth-required` | Fail on 401/403 from a public registry instead of walking to another source. |
| `--offline` | Forbid network access; resolution and fetch must be satisfied locally. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available.

## Examples

```bash
vibe update flow:wal
vibe update --all --assume-yes
vibe update --all --offline --path ./my-project
```

If an alpha format break prevents reconciliation, follow the re-fetch recipe in [ALPHA-NOTES.md](../ALPHA-NOTES.md).

## Related

- [`vibe install`](install.md)
- [`vibe cache`](cache.md)
- [`vibe check`](check.md)

