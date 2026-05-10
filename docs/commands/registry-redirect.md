# `vibe registry redirect` — create a registry stub

Maintainer-side command. Creates a registry stub repo carrying `vibe-redirect.toml` instead of package content; consumers reach the package transparently through the stub via `vibe install <pkgref>`. Per PROP-002 §2.4.2; see [`docs/registry-redirect.md`](../registry-redirect.md) for the full operator reference.

## Usage

```
vibe registry redirect <pkgref> --to <url>
                      [--registry <name>] [--ref-policy <policy>] [--pinned-ref <ref>]
                      [--target-auth <regime>] [--target-token-env <NAME>]
                      [--description <text>] [--sync]
                      [--path <project>] [--dry-run]
                      [--json | --quiet]
```

## Arguments

- `<pkgref>` — `<kind>:<name>` of the package to delegate. The version segment of the pkgref is ignored; stubs live at the `(kind, name)` slot and any version gating is done via stub tags (see `--sync`).

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--to <url>` | Target git URL where the package's actual content lives. Required. Any git URL `git` accepts. | — |
| `--registry <name>` | Which `[[registry]]` in `vibe.toml` will host the stub. | the first registry |
| `--ref-policy <pass-through-tag\|pinned>` | How stub tags map onto target refs. `pass-through-tag` (default): stub tag `T` resolves to `target_url@T`. `pinned`: every consumer resolves to `--pinned-ref` regardless of stub tag. | `pass-through-tag` |
| `--pinned-ref <ref>` | Required when `--ref-policy pinned`. Tag, branch, or commit on the target URL that every consumer pins to. | — |
| `--target-auth <regime>` | Auth regime for the redirect's `[redirect].auth`. Same enum as `[[registry]] auth` — `none` (default), `token-env`, `credential-helper`, `ssh`. | `none` |
| `--target-token-env <NAME>` | Override the env-var name used by `--target-auth token-env`. Default is derived from the target URL's host. | derived |
| `--description <text>` | Free-form text recorded in `[redirect].description`. Surfaces to operators via `vibe show <pkgref>`. | — |
| `--sync` | Mirror current target tags into the freshly-created stub immediately after creation. Equivalent to running `vibe registry redirect-sync <pkgref>` once the stub exists. No-op for `--ref-policy pinned`. | off |
| `--path <project>` | Project root with `vibe.toml`. | `.` |
| `--dry-run` | Describe what would happen but make no API calls or pushes. | off |
| `--json` | Structured payload. | off |
| `--quiet` | One-line summary. | off |

## Authentication

Same publish-token loading as [`vibe registry publish`](registry-publish.md): `VIBEVM_PUBLISH_TOKEN` env-var (highest), then `~/.vibevm/<host-prefix>.publish.token`, then legacy `~/.vibevm/git.publish.token`. The token must have `repo:create` permission in the target organization on the registry's host (`github.com` for GitHub-hosted registries).

Token secrecy invariants are identical to `vibe registry publish` (PROP-000 §20). The token is never logged, never recorded in any vibevm-produced output, and embedded into the push URL only at the moment of `git remote add`.

## Pipeline

1. Validate the `<pkgref>` shape and the `--to` URL.
2. Resolve the target registry from `vibe.toml`.
3. Build the stub source dir in a tempdir: `vibe-redirect.toml` carrying the `[redirect]` block + a small README explaining the delegation.
4. Load the publish token for the registry's host.
5. Probe `repo_exists` for the stub slot. If the repo already exists, the command refuses — editing an existing stub is a manual procedure for v0 (clone, edit `vibe-redirect.toml`, push back).
6. `POST /orgs/<org>/repos` to create the empty stub repo.
7. Initialise a fresh git tree in the staging dir, commit, push to `main`. Token is embedded into the push URL only for that single invocation.
8. (Optional `--sync`) Mirror target tags into the stub — see [`registry-redirect-sync.md`](registry-redirect-sync.md).

## Examples

```bash
# Pass-through-tag stub. Consumers see whatever stub-side tags exist.
vibe registry redirect flow:internal-helper \
  --to git@gitlab.acme.example:flows/internal-helper \
  --description "Delegated to acme-corp; contact maintainers@acme.example"

# Pinned-policy stub: every consumer resolves to v1.0.0 regardless
# of which stub tag they probed.
vibe registry redirect flow:legacy-pinned \
  --to https://github.com/legacy-vendor/flow-pinned \
  --ref-policy pinned --pinned-ref v1.0.0

# Private target.
vibe registry redirect flow:internal-secret \
  --to https://gitlab.company.com/specs/internal-secret \
  --target-auth token-env

# Create the stub AND mirror current target tags in one shot.
vibe registry redirect flow:internal-helper \
  --to git@gitlab.acme.example:flows/internal-helper \
  --sync
```

## Error surface

- **`--ref-policy pinned` without `--pinned-ref`** — clean refusal at flag-validation time, before any side-effecting work.
- **`--pinned-ref` set with `--ref-policy pass-through-tag`** — same. Either drop the flag or change the policy.
- **`--target-token-env` paired with `--target-auth` other than `token-env`** — refused.
- **GitVerse `[[registry]]`** — refused early (GitVerse public API does not expose org-scoped repository creation; same shape as `vibe registry publish`). Use a GitHub registry, or run the manual procedure documented in [`docs/registry-redirect.md`](../registry-redirect.md).
- **Stub repo already exists** — refused. Updating an existing stub's marker is a manual `git clone` / edit / push procedure for v0.

## Related

- [`vibe registry redirect-sync`](registry-redirect-sync.md) — mirror target tags into an existing stub.
- [`docs/registry-redirect.md`](../registry-redirect.md) — operator reference for the redirect protocol (wire grammar, resolver behaviour, identity rules, lockfile shape).
- [PROP-002 §2.4.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect) — the architectural decision and contract.
