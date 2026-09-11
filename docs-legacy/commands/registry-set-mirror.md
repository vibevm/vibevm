# `vibe registry set-mirror` — add a `[[mirror]]` block to `vibe.toml`

Mutates `vibe.toml` to add a new `[[mirror]]` block. A mirror is a transparent alternative URL for a registry: when the primary URL fails (or by priority order), the resolver falls through to the mirror with no change to the lockfile or to per-package identity. `vibe registry set-mirror` is the manifest-side configuration; runtime mirror dispatch lands in M1.6 Phase B.

## Usage

```
vibe registry set-mirror <OF> <URL>
                         [--priority <N>]
                         [--path <DIR>]
                         [--json | --quiet]
```

## Arguments

| Argument | Description |
| --- | --- |
| `<OF>` | Target registry name. Either an exact `[[registry]].name` (the mirror attaches to that registry only) or `*` (the mirror attaches to every registry, including registries added later). Must be non-empty. |
| `<URL>` | Mirror URL. Any git URL `git` accepts. Unlike a `[[registry]]` URL, a mirror URL is **not** required to have a host or org segment — `[[mirror]]` is an availability copy, not a publish target, so `file:///<dir>` URLs (the shape `vibe registry vendor` produces) are accepted verbatim. The only client-side gate is non-empty after trim; anything else is `git`'s job to reject at fetch time. |

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--priority <N>` | Position in the target registry's mirror chain. Lower = tried first. Negative values are legal. | `0` |
| `--path <dir>` | Project directory containing `vibe.toml`. | `.` |
| `--json` | Structured payload. | off |
| `--quiet` | One-line summary. | off |

## Output shape — human

```
  → Added `[[mirror]]` of=`vibespecs` priority=10 → https://github-mirror.example/vibespecs (attaches to registry `vibespecs`)

vibe registry set-mirror: 2 total mirrors configured.
```

For `*` mirrors the attaches-to list is the full set of registries currently in `vibe.toml`:

```
  → Added `[[mirror]]` of=`*` priority=50 → https://offline.example/cache (attaches to every registry (`vibespecs`, `private`))
```

If no `[[registry]]` blocks exist yet (legal — wildcard mirrors are forward-compatible), the message reads "every future registry (no `[[registry]]` configured yet)".

## JSON shape

```json
{
  "ok": true,
  "command": "registry:set-mirror",
  "mirror": {
    "of": "vibespecs",
    "url": "https://github-mirror.example/vibespecs",
    "priority": 10
  },
  "attached_to": ["vibespecs"],
  "total_mirrors": 2
}
```

`attached_to` lists the registry names this mirror is now wired to. For named `<OF>`, that's a single-element list. For `*`, it lists every currently-configured registry; future registries get the mirror automatically when `vibe registry add` runs.

## What gets written

A new `[[mirror]]` block is appended to `vibe.toml`:

```toml
[[mirror]]
of = "vibespecs"
url = "https://github-mirror.example/vibespecs"
priority = 10
```

`priority = 0` is skip-on-serialize, so a mirror with default priority renders without the line.

The whole `vibe.toml` is rewritten on save — comments and bespoke whitespace from the prior version are not preserved, same as `vibe registry add`.

## Examples

```bash
vibe registry set-mirror vibespecs "https://github-mirror.example/vibespecs" --priority 10
vibe registry set-mirror "*" "https://offline.example/cache" --priority 50
vibe registry set-mirror vibespecs "git@backup-host:vibespecs" --priority 100 --quiet
vibe registry set-mirror "*" "file:///abs/path/to/local-org" --json | jq .attached_to
```

## Errors

- **Empty `<OF>`** — exits non-zero. Use `*` for any-registry.
- **Unknown `<OF>`** — exits non-zero, listing the names of registries that *do* exist in `vibe.toml`. The error message reminds the user that `*` is the wildcard.
- **Empty / whitespace `<URL>`** — exits non-zero with `mirror URL must be non-empty`. Past that, an unreachable / unparseable URL surfaces only when the mirror is actually consulted at fetch time, in the operator-actionable diagnostic the resolver renders.
- **Exact duplicate `(of, url)`** — exits non-zero. Two mirrors with identical `of` + `url` is almost always a typo; rejected so the priority chain doesn't end up with silent duplicates. Different priorities for the same `(of, url)` are also rejected. Different URL with the same `of` is fine — that's the whole point of a mirror chain.
- **No `vibe.toml`** — exits non-zero. Run `vibe init` first.

## Exit codes

- `0` — success.
- `1` — validation failure, I/O error on `vibe.toml`, or no project at `--path`.

## Related

- [`vibe registry list`](registry-list.md) — inspect mirrors per-registry; the wildcard `of = "*"` mirror correctly fans out to every registry's view there.
- [`vibe registry add`](registry-add.md) — register the registry that this mirror targets (or that will pick up wildcard mirrors).
- [`PROP-002 §2.5`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md) — full schema for `[[registry]]` / `[[mirror]]` / `[[override]]` and the runtime mirror-dispatch contract that lands in M1.6 Phase B.
