# `vibe registry add` — register a new `[[registry]]` in `vibe.toml`

Mutates `vibe.toml` to add a new `[[registry]]` block. The new entry is appended by default; pass `--position primary` to make it the first registry (the default for publish + the first stop on resolve fallback).

`vibe registry add` is a manifest-only operation — it does not contact the host, does not check that the org actually exists, does not read or write the lockfile. The first `vibe install` that uses the new registry is what triggers actual network access.

## Usage

```
vibe registry add <NAME> <URL>
                  [--ref <REF>]
                  [--naming kind-name|name|kind/name]
                  [--position primary|append]
                  [--auth none|token-env|credential-helper|ssh]
                  [--token-env <ENV_VAR_NAME>]
                  [--path <DIR>]
                  [--json | --quiet]
```

## Arguments

| Argument | Description |
| --- | --- |
| `<NAME>` | Local alias for the new registry. Used in lockfile `registry` fields, in `[[mirror]] of = "<name>"`, in `[[override]]`, and as the `--registry <name>` selector for `vibe registry publish`. Must be unique within `vibe.toml`. |
| `<URL>` | Organization-root URL. Any git URL `git` accepts: `git@host:org`, `ssh://git@host/org`, `https://host/org`, `git+https://...`. Must yield non-empty host and org segments. |

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--ref <REF>` | Registry-level git ref. Reserved for a future registry-metadata branch and not consumed by install today. | `main` |
| `--naming <CONV>` | Convention mapping `<kind>:<name>` to a repo name under the org. `kind-name` produces `<org>/<kind>-<name>`; `name` produces `<org>/<name>` (only valid when names are globally unique across kinds); `kind/name` produces `<org>/<kind>/<name>` (requires host support for nested repository paths). | `kind-name` |
| `--position <pos>` | `primary` inserts the new entry at index 0; `append` adds it at the tail. | `append` |
| `--auth <kind>` | Authentication regime — see [PROP-002 §2.2.1](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-auth). `none` = public read-only (default). `token-env` = read PAT from env-var. `credential-helper` = opt in to system git credential helpers (allows GUI prompts on interactive TTY). `ssh` = ssh-form URL with ssh-agent / keys. | `none` |
| `--token-env <NAME>` | Override the env-var name for `--auth token-env`. Default (omitted) is derived from the registry host: `VIBEVM_REGISTRY_TOKEN_<HOST_UPPER>`, dots and hyphens mapped to underscores (`gitlab.company.com` → `VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM`). Only meaningful with `--auth token-env`; rejected otherwise. | (derived) |
| `--path <dir>` | Project directory containing `vibe.toml`. | `.` |
| `--json` | Structured payload. | off |
| `--quiet` | One-line summary. | off |

## Output shape — human

```
  → Added `[[registry]]` `private` → git@gitverse.ru:somecorp on host gitverse.ru (adapter: gitverse)

vibe registry add: `private` registered (2 total registries).
```

The `(adapter: <name>)` suffix names the host adapter `vibe registry publish` will dispatch to for this registry. `(adapter: none — vibe registry publish won't dispatch here)` means the host has no `RepoCreator` impl in this build; install/sync still work for that host (they shell out to plain `git`), but `vibe registry publish` will refuse with `UnsupportedHost`.

## JSON shape

```json
{
  "ok": true,
  "command": "registry:add",
  "registry": {
    "name": "private",
    "url": "git@gitverse.ru:somecorp",
    "ref": "main",
    "naming": "kind-name",
    "host": "gitverse.ru",
    "org": "somecorp",
    "adapter": "gitverse",
    "mirrors": []
  },
  "position": "append",
  "total_registries": 2
}
```

## What gets written

The new `[[registry]]` block is written into `vibe.toml` at the chosen position. Default values (`ref = "main"`, `naming = "kind-name"`) are skip-on-serialize, so a freshly-added registry with all defaults renders as just `name` + `url`:

```toml
[[registry]]
name = "private"
url = "git@gitverse.ru:somecorp"
```

Non-default values render explicitly:

```toml
[[registry]]
name = "fork"
url = "https://github.com/me/forks"
ref = "develop"
naming = "kind/name"
```

The whole `vibe.toml` is rewritten by `toml::to_string_pretty` — comments and bespoke whitespace from the prior version are not preserved. Settings persist; presentation does not. (If you need comments-preserving editing, edit `vibe.toml` by hand.)

## Examples

```bash
vibe registry add private "git@gitverse.ru:somecorp"
vibe registry add fork "https://github.com/me/forks" --position primary --naming "kind/name" --ref develop
vibe registry add scratch "file:///abs/path/to/local-org" --quiet
vibe registry add public "https://github.com/vibespecs" --json | jq .registry.adapter

# Private registry over HTTPS with PAT in an env-var.
vibe registry add internal "https://gitlab.company.com/vibespecs" \
                  --auth token-env \
                  --token-env VIBEVM_REGISTRY_TOKEN_INTERNAL

# Same but with the env-var name derived from host.
vibe registry add internal "https://gitlab.company.com/vibespecs" --auth token-env

# Private registry over SSH (delegates to ssh-agent / keys).
vibe registry add internal-ssh "git@gitlab.company.com:vibespecs" --auth ssh
```

## Errors

- **Duplicate `<NAME>`** — exits non-zero with the file path and remediation. Pick a different name or remove the existing entry first.
- **Empty `<NAME>`** — exits non-zero. Names must be non-empty.
- **Malformed `<URL>`** — exits non-zero with the `extract_org_segment` / `extract_host_segment` error chain. Both segments must be non-empty for the URL to be usable as a `[[registry]].url`.
- **Bad `--naming`** — exits non-zero listing the valid spellings.
- **Bad `--position`** — exits non-zero listing the valid spellings.
- **No `vibe.toml`** — exits non-zero. Run `vibe init` first.

## Exit codes

- `0` — success.
- `1` — validation failure, I/O error on `vibe.toml`, or no project at `--path`.

## Related

- [`vibe registry list`](registry-list.md) — inspect the result.
- [`vibe registry sync`](registry-sync.md) — refresh per-package clones referenced by the lockfile (touches every `[[registry]]`-served entry).
- [`vibe registry publish`](registry-publish.md) — uses the primary `[[registry]]`'s host adapter unless `--registry <name>` overrides it.
- [`PROP-002 §2.5`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md) — full schema for `[[registry]]` / `[[mirror]]` / `[[override]]`.
