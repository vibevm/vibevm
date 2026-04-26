# `vibe uninstall` — reverse an install

Removes a package's files and lockfile entry. The package's contents in the per-project cache (`<project>/.vibe/cache/`) are left in place — they're cheap to re-fetch and have no effect on the project unless reinstalled.

User-owned files are **never** touched. This is the same guard as `vibe install` runs at plan time, applied symmetrically in reverse: `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, and any `00-` / `90-` boot file are filtered out of the removal list before any `unlink` call.

## Usage

```
vibe uninstall <pkgref> [--path <dir>] [--assume-yes]
               [--json | --quiet]
```

## Arguments

- `<pkgref>` — `<kind>:<name>` of the installed package. Version, if supplied, is **ignored** — uninstall keys off identity, not version constraint.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project directory containing `vibe.lock`. | `.` |
| `--assume-yes` | Skip the interactive confirmation. Required in non-TTY environments. Aliased to `--yes`. | off |
| `--json` | Structured payload. Schema: [`schemas/uninstall_report.jtd.json`](../../schemas/uninstall_report.jtd.json). | off |
| `--quiet` | One-line summary. | off |

## What happens

1. The lockfile entry for `<pkgref>` is loaded; if absent, exit code `1`.
2. The full `files_written` list from the entry is filtered against the user-owned-paths set.
3. Plan is rendered; the operator confirms (unless `--assume-yes` / `--json`).
4. Files are removed; empty parent directories are pruned upward where safe.
5. The lockfile entry is dropped. If the package was a root (`[meta].root_dependencies` contains it), it's removed from that list too. Transitives are not auto-pruned in M1 — that's reserved for `vibe update --prune` in a later milestone.

## Examples

Remove a package interactively:

```bash
vibe uninstall flow:wal
```

Remove non-interactively in CI:

```bash
vibe uninstall flow:wal --assume-yes
```

Pipe the JSON report into a follow-up step:

```bash
vibe --json uninstall flow:wal --assume-yes \
    | jq -r '.paths[]' \
    | xargs -I{} echo "removed: {}"
```

## Exit codes

- `0` — success.
- `1` — package is not installed; project has no `vibe.lock`; I/O error during file removal.
- `5` — operator declined the interactive confirmation.

## What does NOT happen

- The per-project cache (`<project>/.vibe/cache/<kind>/<name>/<version>/`) is **not** purged. Restoring the package is one `vibe install` away with no network round-trip.
- The per-machine registry clone under `~/.vibe/registries/` is **not** touched. Sharing across projects is preserved.
- Transitive packages installed only because this package required them are **not** auto-removed. That's a pruning policy for a later command.

## Related

- [`vibe install`](install.md) — the inverse operation.
- [`vibe list`](list.md) — confirm what's installed before uninstalling.
- `VIBEVM-SPEC.md` §6 — the boot-snippet model that governs which prefixes count as user-owned.
