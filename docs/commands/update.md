# `vibe update` — re-fetch and apply package updates

Re-fetches one or more installed packages against their original root constraint, computes a per-file diff (added / removed / modified / identical), and applies it after operator confirmation. Lock-aware: respects whatever version constraint was originally typed at install time.

Spec: [`VIBEVM-SPEC.md` §16](../../VIBEVM-SPEC.md) (M1 acceptance), [ROADMAP §M1.2](../../ROADMAP.md#m12--vibe-update).

## Usage

```
vibe update <pkgref> [<pkgref> ...] [--path <dir>] [--assume-yes]
                                    [--json | --quiet]
vibe update --all                   [--path <dir>] [--assume-yes]
                                    [--json | --quiet]
```

`<pkgref>` is `<kind>:<name>` (no version — version is taken from the lockfile / root constraint, not from the command line). Mutually exclusive with `--all`.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--all` | Update every package in `vibe.lock` (roots + transitives). Mutually exclusive with named pkgrefs. | off |
| `--path <dir>` | Project directory containing `vibe.toml` and `vibe.lock`. | `.` |
| `--assume-yes` | Skip the interactive confirmation prompt. Aliased to `--yes`. **Required** when stdin is not a TTY (CI / scripts). The global `--unattended` flag (or `VIBE_UNATTENDED` env-var) has the same effect. | off |
| `--exact` | After re-resolving each root, tighten its `vibe.toml` `[requires].packages` constraint to `=<resolved>` (the new version). Equivalent of cargo `cargo update --precise X.Y.Z` plus a manifest pin in one step. Without this flag, `vibe update` only refreshes the lockfile pin and leaves the manifest constraint (`^` / `~` / range) untouched — cargo's default. | off |
| `--auth-required` | Strict authentication gate — same shape as `vibe install --auth-required`. A 401 / 403 against an `auth = "none"` (public) registry halts the update instead of walking past. Useful in CI / cron where a fallback to a public substitute would mask a private-registry outage. | off |
| `--json` | Two structured documents: the plan (command `"update:plan"`) before apply, the report (command `"update"`) after. With `--json` the confirmation is auto-approved. | off |
| `--quiet` | One-line summary after apply. Conflicts with `--json`. | off |

## Pipeline

1. **Resolve.** For each target package, look up its original constraint in `[meta].root_dependencies` (e.g. `flow:wal@^0.3`); fall back to an exact-version pin for non-root transitives. The `MultiRegistryResolver` walks `[[registry]]` priority order, falls through `[[mirror]]` URLs on primary failure (PROP-002 §2.3), and applies the cross-source `content_hash` gate from [`vibe install`](install.md). Override-pinned packages re-fetch at the override's ref.
2. **Compare.** If the resolved version + content_hash matches the lockfile entry, the package is reported as `up-to-date` and skipped. Otherwise build an [`UpdatePlan`](#per-file-classification) by diffing the new package's published tree against the materialised content in the old version's `vibedeps/` slot.
3. **Refuse on user-edits.** For every file in either the old or new package tree, compare its on-disk bytes in `vibedeps/` to the **install-time** cache (`.vibe/cache/<kind>/<name>/v<old-version>/`). If they differ, the slot was hand-edited post-install — refuse with [`UserEditedFile`](#errors). Restore the original cache, or run `vibe uninstall && vibe install` to consciously discard the edits.
4. **Refuse on dep-shape change.** If the new manifest's `[requires].packages` declares a different set of `(kind, name)` than the locked transitive set, refuse with [`DependencyShapeChanged`](#errors). Narrow v0 of `vibe update` does not cascade graph changes — run `vibe uninstall && vibe install` to apply the new graph.
5. **Confirm.** Unless `--assume-yes` or `--json` is set, the operator sees the combined plan and confirms interactively. Decline → exit code `5`.
6. **Apply.** The package's `vibedeps/` slot is re-materialised at the new version: `Removed` files are deleted, `Added` and `Modified` files are written from the new cache, `Identical` files are no-ops. The boot artifacts (`spec/boot/INLINE.md`, `spec/boot/INDEX.md`) are then regenerated for every node so the updated package's boot contribution is recomputed. Best-effort rollback on partial failure (snapshots taken at apply start are restored).
7. **Bump lockfile.** The lockfile entry's `version`, `content_hash`, `source_url`, `source_ref`, `resolved_commit` are rewritten; `dependencies` and `overridden` are preserved (the dep-shape gate kept them stable). The lockfile no longer carries a per-file `files_written` list or a `boot_snippet` filename — see [`docs/lockfile-format.md`](../lockfile-format.md).

## Per-file classification

The diff is over the files of the package's `vibedeps/` slot, old version against new:

| Sigil | Meaning | Apply behaviour |
| --- | --- | --- |
| `[+]` `Added` | In the new package tree; not in the old slot. | Write into the slot from the new cache. |
| `[-]` `Removed` | In the old slot; not in the new package tree. | Delete from the slot. |
| `[~]` `Modified` | In both old and new with **different** bytes; the slot file is **pristine** (matches the old cache). | Overwrite in the slot from the new cache. |
| `[=]` `Identical` | In both old and new with byte-identical content. | No-op. |

A change to the package's `[boot_snippet]` — a new `category`, a moved `source` file — is reflected automatically: the slot is re-materialised verbatim, and regenerating the boot artifacts recomputes the package's place in every node's boot sequence. There is no boot-filename rename to track, because there is no boot filename.

## Errors

| Error | Cause | Exit code |
| --- | --- | --- |
| `NotInstalled` | Named pkgref isn't in the lockfile. | 1 |
| `AlreadyUpToDate` | Resolved version + content_hash matches the lockfile pin. (Surfaces as a step-line, not an error, in the typical UX — listed here for completeness.) | 0 |
| `UserEditedFile` | A project file's bytes diverge from the install-time cache; the update would silently destroy the user's edit. | 1 |
| `OldCacheMissing` | The install-time cache directory for the old version is gone (e.g. user wiped `.vibe/cache/`). Without it the user-edit check can't run. | 1 |
| `DependencyShapeChanged` | New manifest declares a `[requires]` set that differs from the locked transitive set. | 1 |
| `UserDeclined` | Operator answered `n` to the confirmation prompt. | 5 |

## Lockfile

The lockfile entry for the updated package is rewritten in place. Its on-disk shape is the standard schema v4 (see [`docs/lockfile-format.md`](../lockfile-format.md)). Notable: `content_hash` shifts when the new payload differs from the old, and `dependencies` is **preserved** byte-for-byte (the dep-shape gate refuses to plan when it would change).

`[meta].generated_at` is bumped to the apply timestamp; `[meta].root_dependencies` is unchanged — `vibe update` is a version bump, not a constraint change.

## Examples

```bash
# Update one package, prompt for confirmation.
vibe update flow:wal

# Update every installed package in one go.
vibe update --all --assume-yes

# Inspect what an update would do without applying.
vibe update --json --assume-yes flow:wal | jq '.plans[].changes'

# CI-friendly: quiet, no prompt, exit non-zero on any failure (e.g. user-edit).
vibe update --all --assume-yes --quiet
```

## Edge cases

- **Empty lockfile.** Hard error: nothing to update. Run `vibe install` first.
- **No `[[registry]]` configured.** Hard error: `vibe update` re-fetches from a registry, so a registry must exist. Add one with `vibe registry add <name> <url>` or run `vibe install --registry <path>` for the local-directory model (which doesn't support `vibe update` — see below).
- **Local-directory registry (`--registry <path>`).** `vibe update` doesn't accept `--registry <path>` — local-directory installs have no version-bump mechanism beyond rewriting fixtures by hand. Re-install via `vibe install --registry <path>` after rewriting the fixture.
- **Override-pinned package.** Re-fetched at the override's `ref`. If the manifest at that ref declares a different `(kind, name)` from the override's pkgref, the resolver refuses (`MalformedMeta`) — same gate as install-time.
- **Tag force-pushed upstream (same version, different bytes).** Surfaces as a `Modified` plan with the same version on both sides; `content_hash` shifts in the lockfile after apply. The cross-source gate on the install path is the supply-chain check; `vibe update` is a deliberate refresh and accepts the new bytes.

## Limitations (v0)

- `vibe update` does not currently cascade dep-graph changes. A package whose new version pulls in a new transitive (or drops one) is refused with `DependencyShapeChanged`. Use `vibe uninstall <pkg> && vibe install <pkg>` for that case until v1 lands graph-cascade.
- Non-root transitives are re-resolved at their **exact** locked version, not against any user constraint. They only move on a force-push (same `=<version>` constraint, different content_hash).

## Related

- [`vibe install`](install.md) — initial install pipeline; `vibe update` re-uses its resolver and lockfile shape.
- [`vibe registry sync`](registry-sync.md) — refresh registry clones; useful before `vibe update` to ensure the freshest tag list.
- [`vibe uninstall`](uninstall.md) — the consciously-discard-edits-and-rebuild path when `vibe update` refuses on `UserEditedFile`.
- [PROP-002 §2.7](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#lockfile) — lockfile schema v4; `vibe update` writes back into the same shape.
