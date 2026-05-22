# `vibe uninstall` — reverse an install

Removes a package from the project: drops its `vibedeps/` slot, its lockfile entry, and its `[requires.packages]` declaration, then regenerates the boot artifacts for every node so the package no longer appears in any computed boot sequence. The package's contents in the per-project cache (`<project>/.vibe/cache/`) are left in place — they're cheap to re-fetch and have no effect on the project unless reinstalled.

A node's **authored `spec/` is never touched** — uninstall removes the package's materialised `vibedeps/` subtree, not authored content. Under the loading model `vibe install` never wrote into a node's authored `spec/` in the first place ([PROP-009 §2.1](../../spec/modules/vibe-workspace/PROP-009-loading-model.md#two-trees)), so there is nothing there to reverse. The conventional user-owned boot files `spec/boot/00-core.md` and `spec/boot/90-user.md`, and `spec/WAL.md`, are authored content and are left exactly as they are.

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
| `--assume-yes` | Skip the interactive confirmation. Required in non-TTY environments. Aliased to `--yes`. The global `--unattended` flag (or `VIBE_UNATTENDED` env-var) has the same effect. | off |
| `--json` | Structured payload. Schema: [`schemas/uninstall_report.jtd.json`](../../schemas/uninstall_report.jtd.json). | off |
| `--quiet` | One-line summary. | off |

## What happens

1. The lockfile entry for `<pkgref>` is loaded; if absent, exit code `1`.
2. Plan is rendered — the `vibedeps/` slot to remove plus the boot artifacts to regenerate. Every node's `<vibevm>` instruction-file block is validated at plan time; a malformed block aborts before any change.
3. The operator confirms (unless `--assume-yes` / `--json`).
4. The package's `vibedeps/<kind>-<name>/<version>/` slot is removed.
5. `vibe.toml` `[requires].packages` is rewritten without the matching entry (no-op when the package was a pure transitive, never declared in the manifest).
6. The lockfile entry is dropped. If the package was a root (`[meta].root_dependencies` contains it), it's removed from that list too. Transitives are not auto-pruned in M1 — that's reserved for `vibe update --prune` in a later milestone.
7. The boot artifacts (`spec/boot/INLINE.md`, `spec/boot/INDEX.md`) are regenerated for every node, so the removed package's boot contribution disappears from each computed boot sequence.

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
- [The loading model](../loading-model.md) — the `vibedeps/` tree uninstall removes from, and the boot artifacts it regenerates.
