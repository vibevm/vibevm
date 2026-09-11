# `vibe registry test` — probe each registry's reachability and auth

Read-only diagnostic. For every `[[registry]]` configured in the project's `vibe.toml`, the command performs a single `git ls-remote` (with token injection if `auth = "token-env"`) and reports the registry's current operational status without cloning, downloading packages, or mutating any local state.

Use cases:

- A new contributor lands on a project with private registries — `vibe registry test` tells them at a glance which tokens they still need to set, before they hit a confusing `vibe install` failure.
- A CI gate before the build step — fail the job early with a precise message if the internal mirror is unreachable, instead of letting `vibe install` fail much later in the pipeline with a multi-line resolver error.
- Triaging a broken install — the per-registry table pinpoints which registry is the problem (auth-required vs. network-unreachable vs. genuinely-not-found).

## Usage

```
vibe registry test [--path <dir>]
                   [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project directory containing `vibe.toml`. | `.` |
| `--json` | Structured payload (see schema below). | off |
| `--quiet` | One-line summary `vibe registry test: <ok>/<total> reachable.` | off |

## Output shape — human

```
NAME              AUTH         STATUS         URL
vibespecs         public       reachable      https://github.com/vibespecs
internal-mirror   token-env    missing-token  https://gitlab.company.com/vibespecs
crisis            token-env    auth-required  https://gitea.crisis.example/vibespecs
gone              public       unreachable    https://gitlab.dead.example/vibespecs

Legend:
  reachable      — the host responded, registry is healthy.
  auth-required  — host returned 401/403; token rejected or missing scopes.
  missing-token  — auth = "token-env" declared but the env-var is empty / unset.
  unreachable    — DNS / TLS / network failure reaching the host.

vibe registry test: 1/4 reachable.
```

The exit code is `0` when **every** registry is reachable, `1` otherwise — so the command is suitable as a precondition gate without parsing.

## JSON shape

```json
{
  "ok": false,
  "command": "registry:test",
  "summary": { "total": 4, "reachable": 1 },
  "registries": [
    {
      "name": "vibespecs",
      "url": "https://github.com/vibespecs",
      "auth": "public",
      "status": "reachable",
      "detail": null
    },
    {
      "name": "internal-mirror",
      "url": "https://gitlab.company.com/vibespecs",
      "auth": "token-env",
      "status": "missing-token",
      "detail": "VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM is not set"
    }
  ]
}
```

`status` is one of `reachable`, `auth-required`, `missing-token`, `unreachable`. `auth` is the registry's declared regime (`public`, `token-env`, `credential-helper`, `ssh`). `detail` is `null` when there is nothing to add (every reachable case); otherwise it carries a human-readable elaboration of the failure.

## Examples

```bash
# Quick status pass:
vibe registry test

# CI gate — fails the step if any registry is broken:
vibe registry test --quiet

# Pinpoint which registry needs a token:
vibe registry test --json | jq '.registries[] | select(.status == "missing-token")'

# Are we ready for a clean install?
vibe registry test --json | jq '.summary.reachable == .summary.total'
```

## Exit codes

- `0` — every registry returned `reachable`.
- `1` — at least one registry returned a non-reachable status, or `vibe.toml` is missing / unparseable.

## How it works under the hood

For each registry, the command builds an ephemeral single-registry `MultiRegistryResolver` and probes with a guaranteed-not-to-exist pkgref (`flow:vibe-probe-99zzqq`). The resulting `RegistryWalkAttempt` is classified:

- The registry returns `not-found` for the fake package — the host responded, mapping is healthy, registry status is `reachable`.
- The registry returns `auth-failed` (401 / 403 from the host) — registry status is `auth-required`.
- A precheck flagged a missing token (`auth = "token-env"`, env-var empty) — registry status is `missing-token`. No network call was made; this is detected before git is invoked, exactly the same as in `vibe install`.
- A network or DNS failure — registry status is `unreachable`.

Token discipline matches `vibe install`: tokens are read once, injected into the URL only inside the in-memory `git ls-remote` call, never written to disk, never logged. If you have a `[[registry]] auth = "token-env"` block and you set the right env-var, `vibe registry test` is the cheapest way to confirm the token works without paying for a full clone.

## Related

- [`vibe registry list`](registry-list.md) — view the configuration `vibe registry test` is probing.
- [`vibe install`](install.md) — when `vibe registry test` reports a non-reachable registry, `vibe install`'s structured JSON error envelope (`error_kind: "package_not_found_everywhere"`) shows the same per-registry attempt vector for any specific package resolution failure.
- [`registry-auth.md`](../registry-auth.md) — full reference for the four auth regimes.
- [`PROP-002 §2.3.1`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#failure-discriminator) — the auth-aware failure-mode classifier this command surfaces.
