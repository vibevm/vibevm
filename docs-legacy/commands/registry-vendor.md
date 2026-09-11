# `vibe registry vendor` — generate a local mirror directory

Walks the project's `vibe.lock` and produces a directory containing one bare git repo per `[[registry]]`-served lockfile entry. The directory is a drop-in source for `[[mirror]] url = "file:///<abs-path>"` so subsequent installs can resolve everything without network access (offline / air-gapped path).

Spec: [PROP-002 §2.3 (mirror layer)](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#mirror), [§6 Phase B preview](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#phase-b).

## Usage

```
vibe registry vendor [--out <dir>] [--force]
                     [--path <project>]
                     [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--out <dir>` | Output directory for the vendor mirror. Each package becomes a bare repo at `<out>/<naming>(<kind>,<name>).git/`. | `<project>/vendor/` |
| `--force` | If `--out` exists and is non-empty, wipe it before vendoring. Without this flag, a non-empty target dir is a hard error — vibe never silently overwrites operator content. | off |
| `--path <project>` | Project root with `vibe.toml` and `vibe.lock`. | `.` |
| `--json` | Structured payload. Top-level fields: `out_dir`, `suggested_mirror_url`, `vendored[]`, `skipped[]`. | off |
| `--quiet` | One-line summary `vibe registry vendor: <N> vendored, <K> skipped.` | off |

## What gets vendored

For each entry in the lockfile:

| Entry shape | Vendor path |
| --- | --- |
| `registry: Some(name)` and `overridden: false` | The `[[registry]]` named here is looked up in `vibe.toml`. The per-package clone is refreshed via the registry's mirror-aware `refresh_package` (so vendor still works when the canonical primary is unreachable as long as some `[[mirror]]` URL is). The clone's `.git/` is then copied into `<out>/<repo>.git/` where `<repo>` follows the registry's `naming` convention. |
| `overridden: true` | **Skipped.** `[[override]]`-served packages have no mirror concept — the override is the authoritative source by design. Vendor a fork manually if you need offline coverage of it. |
| `registry: None` and `overridden: false` | **Skipped.** Local-directory installs (`--registry <path>`) don't have a registry-attributed source to mirror. |

The bare repo carries the same refs and tags the per-package clone holds — at minimum the lockfile-pinned tag, plus whatever else the upstream had at the time of the most recent `refresh_package`. Identity is preserved end-to-end: `git ls-remote <out>/<repo>.git` and `git clone <out>/<repo>.git` produce content with the same `content_hash` the lockfile pinned, so the cross-source verification gate in `vibe install` accepts it.

## Wiring the result into `vibe.toml`

After vendoring, add a `[[mirror]]` block pointing at the vendor directory's `file://` URL:

```toml
[[mirror]]
of = "vibespecs"          # or "*" to mirror every registry
url = "file:///abs/path/to/vendor"
priority = 0
```

The exact URL is printed at the end of `vibe registry vendor` and recorded under `suggested_mirror_url` in `--json` output, so you can paste it directly. On Windows the URL takes the form `file:///C:/Users/.../vendor`.

When the canonical `[[registry]]` is reachable, `vibe install` walks it first (PROP-002 §2.3); the file:// mirror takes over when the primary fails to clone. For full force-offline (don't even try the network primary), a future `--offline` flag (M2) will short-circuit; today, a `[[registry]] url = "file:///..."` configured directly is the closest workaround.

## Examples

```bash
# Vendor into <project>/vendor/.
vibe registry vendor

# Vendor into a custom location, refreshing if it exists.
vibe registry vendor --out /opt/vibevm-mirror --force

# Inspect the suggested mirror URL programmatically.
vibe registry vendor --json | jq -r '.suggested_mirror_url'
```

## Edge cases

- **Empty lockfile** — fails with `no \`vibe.lock\` ...` if the file doesn't exist. Run `vibe install` first.
- **No `[[registry]]` in `vibe.toml`** — hard error: vendor only mirrors registry-served packages, so projects using only `--registry <path>` or `[[override]]` have nothing to vendor.
- **`--out` is non-empty without `--force`** — refuses to wipe operator content. Pass `--force` or pick a fresh path.
- **Primary unreachable during vendor** — `refresh_package` walks `[[mirror]]`s in priority order, so vendor still works as long as at least one mirror URL is reachable. The vendor is then sourced from that mirror.
- **`--out` is inside the project root** — fine; the vendor dir is just an output. Add the path to `.gitignore` if you don't want it tracked.

## Output (JSON)

```jsonc
{
  "ok": true,
  "command": "registry:vendor",
  "out_dir": "/abs/path/to/vendor",
  "suggested_mirror_url": "file:///abs/path/to/vendor",
  "vendored": [
    {
      "kind": "flow",
      "name": "wal",
      "registry": "vibespecs",
      "repo_dir": "/abs/path/to/vendor/flow-wal.git",
      "ref": "v0.1.0"
    }
  ],
  "skipped": [
    {
      "kind": "flow",
      "name": "atomic-commits",
      "reason": "[[override]]-served (source_url ...) — vendor it manually if you need offline coverage"
    }
  ]
}
```

## Exit codes

- `0` — success, including a vendor with all entries skipped (no registry-served packages).
- `1` — missing `vibe.toml` / `vibe.lock`, missing `[[registry]]` blocks, non-empty `--out` without `--force`, git failure during `refresh_package`, I/O error on the vendor directory.

## Related

- [`vibe registry sync`](registry-sync.md) — refreshes per-package clones in the cache; `vibe registry vendor` calls into the same machinery to ensure clones are at the lockfile's `source_ref` before copying.
- [`vibe registry set-mirror`](registry-set-mirror.md) — adds the `[[mirror]]` entry that wires the vendor dir into `vibe install`'s resolve path.
- [PROP-002 §2.1](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#identity) — content-hashed identity, the invariant that lets a `file://` mirror substitute for the canonical source without lockfile churn.
