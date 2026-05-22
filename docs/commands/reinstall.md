# `vibe reinstall` — recompute the materialised dependencies and boot artifacts

Recomputes a workspace's materialised state without re-resolving. `vibe reinstall` is the regeneration command of the loading model: it rebuilds the `vibedeps/` tree and the per-node boot artifacts (`spec/boot/INLINE.md`, `spec/boot/INDEX.md`) from the versions `vibe.lock` already pins — it **never re-resolves**. Moving a version is [`vibe update`](update.md)'s job.

Use it when the materialised state is believed stale or a previous generation pass was wrong: a hand-edited `vibedeps/` slot, a wrongly-generated `INDEX.md`, a `vibedeps/` tree out of step with the lockfile. Per [PROP-009 §2.10](../../spec/modules/vibe-workspace/PROP-009-loading-model.md).

## Usage

```
vibe reinstall [<path>] [--force] [--assume-yes]
                        [--json | --quiet]
```

`<path>` is a positional argument — any directory inside the workspace. Discovery bubbles up from it to the absolute workspace root, so `vibe reinstall` regenerates the boot artifacts of **every node** in the workspace, not just the one the path names: a node's aggregated boot depends on its members' (the matryoshka of [PROP-009 §2.2](../../spec/modules/vibe-workspace/PROP-009-loading-model.md#effective-boot)). It defaults to `.`.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `<path>` | Positional. Any directory inside the workspace; the absolute root is discovered by walking up from it. | `.` |
| `--force` | Re-fetch every locked package's content from its source repository — at the version `vibe.lock` pins, never re-resolving — bypassing the project cache and overwriting the current `vibedeps/` files. The escape hatch for a corrupted or hand-edited `vibedeps/` subtree. Without this flag, `vibe reinstall` only recomputes the boot artifacts from the materialised tree already on disk — no fetch, no network. | off |
| `--assume-yes` | Skip the interactive confirmation prompt. Aliased to `--yes`. **Required** when stdin is not a TTY (CI / scripts). The global `--unattended` flag (or `VIBE_UNATTENDED` env-var) has the same effect. | off |
| `--json` | Structured payload (see below). With `--json` the confirmation is auto-approved. | off |
| `--quiet` | One-line summary after the run. Conflicts with `--json`. | off |

## The two modes

`vibe reinstall` runs in one of two modes depending on `--force`.

### Regenerate (no `--force`)

The default. The materialised `vibedeps/` tree already on disk is the **only** content source — `vibe reinstall` reads it, recomputes each node's effective boot sequence from `vibe.lock`, and regenerates the boot artifacts. **No fetch, no network.** This is the fix for a stale or wrongly-generated `INDEX.md` / `INLINE.md`.

Every package the lockfile pins must have its `vibedeps/` slot present on disk. A missing slot is content this mode cannot conjure — `vibe reinstall` stops, names the missing slot(s), and points you at `--force`. Re-run with `--force` to re-fetch the content from source.

### Re-fetch (`--force`)

Re-fetches every locked package's content from its source repository at the exact version `vibe.lock` pins, then re-materialises `vibedeps/` and regenerates the boot artifacts. Specifically, `--force`:

- builds the resolver from the workspace root manifest (`[[registry]]` / `[[mirror]]` / `[[override]]` / git-source declarations are root-level);
- **wipes the project package cache** (`.vibe/cache`) so every fetch re-downloads from source;
- re-fetches each locked package at its `=<version>` pin — never re-resolving. The lockfile's recorded `content_hash` is forwarded as the expected hash, so a source serving disagreeing bytes is rejected: `vibe reinstall` reproduces the lock, it never drifts it;
- re-materialises every `vibedeps/` slot and prunes any slot no longer in the lockfile;
- regenerates the boot artifacts for every node.

`--force` is the escape hatch for a corrupted, hand-edited, or wrongly-materialised `vibedeps/` subtree.

In both modes, an absent or empty `vibe.lock` is fine — `vibe reinstall` simply regenerates the boot artifacts from the authored `spec/boot/` tree, with nothing to materialise.

## Errors

| Error | Cause | Exit code |
| --- | --- | --- |
| `no vibe.toml` | The resolved `<path>` (or a parent up to the root) carries no `vibe.toml`. Run `vibe init` first. | 1 |
| incomplete `vibedeps/` | Regenerate mode (no `--force`): a package the lockfile pins has no `vibedeps/` slot on disk. The message names every missing slot and points at `--force`. | 1 |
| malformed `<vibevm>` block | A node's `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` does not contain exactly one well-formed `<vibevm>` … `</vibevm>` pair. Caught at plan time, before any mutation; you repair the file by hand (see [the loading model](../loading-model.md#the-managed-vibevm-block)). | 3 |
| network / fetch failure | `--force`: a source repository was unreachable, or served bytes whose hash disagrees with the lockfile pin. | 1 |
| `UserDeclined` | The operator answered `n` to the confirmation prompt. | 5 |

## JSON output (`--json`)

```jsonc
{
  "ok": true,
  "command": "reinstall",
  "forced": false,
  "nodes_regenerated": ["."],
  "pruned": []
}
```

`forced` echoes whether `--force` was set. `nodes_regenerated` lists the workspace nodes whose boot artifacts were rewritten, by root-relative path. `pruned` lists `vibedeps/` slots that were present before the run and removed during it (only `--force` prunes).

## Examples

```bash
# Recompute boot artifacts from the materialised vibedeps/ tree — no network.
vibe reinstall

# A boot artifact was hand-edited or a generation pass went wrong: rebuild it.
vibe reinstall --assume-yes

# The vibedeps/ tree is corrupted — re-fetch everything from source.
vibe reinstall --force --assume-yes

# Reinstall from a directory deep inside the workspace; the whole tree regenerates.
vibe reinstall packages/flow-wal

# CI-friendly: machine-readable, no prompt.
vibe --json reinstall --assume-yes | jq '.nodes_regenerated'
```

## `reinstall` vs `install` vs `update`

| Command | Re-resolves? | Fetches? | What it does |
| --- | --- | --- | --- |
| [`vibe install`](install.md) | yes | yes | Resolve `[requires]` across the workspace, materialise `vibedeps/`, regenerate boot. |
| [`vibe update`](update.md) | yes | yes | Move a pinned version, then re-fetch and re-materialise. |
| `vibe reinstall` | **no** | only with `--force` | Recompute the materialised state and boot artifacts from the existing `vibe.lock`. |

`vibe reinstall` is the cheap, deterministic regeneration step — it changes no versions and (without `--force`) touches no network. Reach for it when the loading model's *generated* output is stale; reach for `vibe install` / `vibe update` when the *resolution* itself needs to change.

## Related

- [`vibe install`](install.md) — the workspace-aware install pipeline that first materialises `vibedeps/` and generates the boot artifacts.
- [`vibe update`](update.md) — move a pinned version; re-resolves where `reinstall` does not.
- [The loading model](../loading-model.md) — the two-tree layout, the computed boot sequence, the generated `INLINE.md` / `INDEX.md`, the managed `<vibevm>` block.
- [PROP-009 §2.10](../../spec/modules/vibe-workspace/PROP-009-loading-model.md) — the design lock for `vibe reinstall`.
