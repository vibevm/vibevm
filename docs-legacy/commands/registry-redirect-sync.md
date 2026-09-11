# `vibe registry redirect-sync` — mirror target tags into a stub

Maintainer-side command. Reads an existing registry stub's `vibe-redirect.toml`, enumerates target-side tags, and pushes the missing ones into the stub. Per PROP-002 §2.4.2 `pass-through-tag` policy: the stub's tags determine which target versions the org's namespace exposes; this command keeps them in sync.

`pinned`-policy stubs are refused — every consumer resolves to `pinned_ref` regardless of stub tag, so syncing has no effect.

## Usage

```
vibe registry redirect-sync <pkgref>
                             [--registry <name>] [--path <project>] [--dry-run]
                             [--json | --quiet]
```

## Arguments

- `<pkgref>` — `<kind>:<name>` of the existing stub to sync.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--registry <name>` | Which `[[registry]]` in `vibe.toml` hosts the stub. | the first registry |
| `--path <project>` | Project root with `vibe.toml`. | `.` |
| `--dry-run` | Describe what would happen but make no API calls or pushes. | off |
| `--json` | Structured payload. | off |
| `--quiet` | One-line summary. | off |

## Authentication

Same publish-token loading as [`vibe registry publish`](registry-publish.md). The push leg requires `repo:create` permission on the registry's host (a no-op `repo_exists` API call is also made to fail fast if the stub does not exist).

If the stub's `[redirect].auth` is `token-env`, the **target-side** read needs the env-var the marker declares (or the host-derived default). The CLI returns a clean error pointing at the env-var name when it is unset.

## Pipeline

1. Resolve registry and stub URL from `vibe.toml`.
2. Probe `repo_exists` for the stub. Refuse with an actionable message if the stub does not exist (and point at `vibe registry redirect <pkgref> --to <url>`).
3. Shallow-clone the stub repo at `main` into a tempdir.
4. Read `vibe-redirect.toml`. Refuse if `ref_policy = "pinned"` (sync is meaningless under pinned semantics).
5. `git ls-remote --tags` against both the target URL (target-side) and the stub URL (stub-side).
6. Diff: collect tags present on the target but absent on the stub.
7. For each missing tag, create an annotated tag on the stub's `main` commit (the marker-file commit) and `git push origin <tag>`.

The stub commit graph stays minimal — every tag points at the same `main` commit (the marker file), since stub content is identical regardless of which target version it fronts. The pass-through happens during the consumer's `vibe install` resolve, not during version listing.

## Examples

```bash
# Periodic sync — fits naturally into a CI cron job.
vibe registry redirect-sync flow:internal-helper

# Dry-run: print which tags would be added without touching anything.
vibe registry redirect-sync flow:internal-helper --dry-run
```

## Error surface

- **Stub does not exist** — refusal pointing at `vibe registry redirect`. The CLI does not silently create.
- **`ref_policy = "pinned"`** — refusal with a clear "nothing to sync" message.
- **Target unreachable / unauthenticated** — surfaces the underlying git error verbatim. Inspect with `--dry-run` before re-running.
- **Stub's `vibe-redirect.toml` missing or malformed** — clean error; points the operator at the marker file's required shape.

## Related

- [`vibe registry redirect`](registry-redirect.md) — create a fresh stub.
- [`docs/registry-redirect.md`](../registry-redirect.md) — operator reference for the redirect protocol.
- [PROP-002 §2.4.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect) — the architectural decision and contract.
