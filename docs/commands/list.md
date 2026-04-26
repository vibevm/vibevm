# `vibe list` — show installed packages

Reads `vibe.lock` and renders the packages currently locked into the project. Read-only — no network, no cache touch, no mutation.

## Usage

```
vibe list [--kind <kind>] [--path <dir>]
          [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--kind <kind>` | Filter to one of `flow` / `feat` / `stack` / `tool`. | show all kinds |
| `--path <dir>` | Project directory containing `vibe.lock`. | `.` |
| `--json` | Structured payload. Schema: [`schemas/list_report.jtd.json`](../../schemas/list_report.jtd.json). Carries every lockfile-v2 provenance field — registry, source_url, source_ref, resolved_commit, overridden — for scripted analysis. | off |
| `--quiet` | One-line comma-separated list of `kind:name@version` labels. | off |

## Default output

A three-column table sorted by the lockfile order (which is install order). Example:

```
KIND   NAME              VERSION  BOOT SNIPPET
flow   wal               0.1.0    10-flow-wal.md
flow   sync-from-code    0.1.0    20-flow-sync-from-code.md
flow   atomic-commits    0.1.0    30-flow-atomic-commits.md
```

If no packages are installed, the output is the line `(no packages installed)` and exit code `0`.

## Examples

```bash
vibe list                          # all installed packages, table
vibe list --kind flow              # only flows
vibe list --quiet                  # one line: "flow:wal@0.1.0, …"
vibe --json list | jq '.count'     # how many packages, scriptable
vibe --json list | jq '.packages[] | select(.overridden) | .name'  # which are pinned via [[override]]
```

## Exit codes

- `0` — success (including the empty-lockfile case).
- `1` — `vibe.lock` is unreadable or unparseable; `vibe.toml` missing.

## Related

- [`vibe install`](install.md) — populate the lockfile.
- [`vibe uninstall`](uninstall.md) — remove an entry.
- [`vibe.lock` schema](../../VIBEVM-SPEC.md) §7.4.
