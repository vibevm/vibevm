# `vibe registry publish` — publish a package directory

Maintainer-side command. Takes a directory containing a `vibe-package.toml` and publishes it as a tagged release in the configured registry's organization. Creates the per-package repo (or reuses an existing one), pushes contents, tags the version. Mechanical only — no semantic / LLM-backed review (that's reserved for v2+ per [`VIBEVM-SPEC.md` §8.5](../../VIBEVM-SPEC.md)).

## Usage

```
vibe registry publish <source> [--registry <name>] [--path <project>]
                      [--dry-run]
                      [--json | --quiet]
```

## Arguments

- `<source>` — path to the package directory. Must contain a `vibe-package.toml` at its root, with the package content laid out per the mirror convention ([`VIBEVM-SPEC.md` §13.1](../../VIBEVM-SPEC.md)).

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--registry <name>` | Which `[[registry]]` in `vibe.toml` to publish into. | the first registry |
| `--path <project>` | Project directory containing `vibe.toml`. | `.` |
| `--dry-run` | Describe what would happen but make no API calls or pushes. | off |
| `--json` | Structured payload. Schema: [`schemas/registry_publish_report.jtd.json`](../../schemas/registry_publish_report.jtd.json). | off |
| `--quiet` | One-line summary. | off |

## Authentication

A publish token is required. Order of precedence:

1. `VIBEVM_PUBLISH_TOKEN` environment variable (highest).
2. `~/.vibevm/git.publish.token` file (whitespace trimmed; tilde expands per platform).

Token must be issued by the registry host and must have **`repo:create`** permission in the target organization. See your host's API documentation for token creation; for GitVerse, the public-API docs at <https://gitverse.ru/docs/public-api/> walk through token issuance.

The token value is **never** logged. `Debug` and `Display` representations of the loaded token render as `***`. The CLI does report which *source* the token was loaded from (env var, file path) — that's safe metadata.

## Pipeline

1. **Read manifest** at `<source>/vibe-package.toml`. Legacy `[dependencies]` form is migrated transparently to modern `[requires]` / `[conflicts]` shape (the published manifest is always modern).
2. **Compute repo name** under the org via the registry's `naming` convention. Default `kind-name` produces `<kind>-<name>`; alternatives are `name` or `kind/name` per [PROP-002 §2.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model).
3. **Check or create repo.** A `GET /api/v1/repos/<org>/<repo>` probes existence; if missing, `POST /api/v1/orgs/<org>/repos` creates it with `auto_init = false` (we push our own initial commit).
4. **Stage in a temp working tree.** Contents copied (excluding any `.git/` subtree); `git init`, repo-local identity (`publish@vibevm.local`), `git add -A`, commit `Release <name>@<version>`.
5. **Push** `main` first, then the annotated tag `v<version>`.

`--dry-run` exits before step 3.

## Errors

| Failure | Maps to |
| --- | --- |
| `401` / `403` from API | "publish refused: token lacks `repo:create` permission in `<org>` on `<host>`". |
| `404` from API on org-level endpoint | "publish refused: organization `<org>` does not exist on `<host>` (or the token cannot see it)". |
| `git push` denied | "publish refused: no push access to `<repo>`. Ask a maintainer of that repo to grant push." |
| Tag already exists | "publish refused: tag `<tag>` already exists. Pick a new version — `vibe registry publish` does not force-push." |
| Network unreachable | "publish refused: host `<host>` is unreachable." |
| Token missing | "publish refused: no token available. Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibevm/git.publish.token`." |

## Examples

Dry-run a release first:

```bash
vibe registry publish ./fixtures/registry/flow/wal/v0.1.0 --dry-run
```

Apply against the configured default registry:

```bash
vibe registry publish ./fixtures/registry/flow/wal/v0.1.0
```

Pick a specific registry by name (useful when `vibe.toml` lists several):

```bash
vibe registry publish ./fixtures/registry/flow/wal/v0.1.0 --registry corporate
```

Publish multiple packages from a maintenance script:

```bash
for pkg_dir in fixtures/registry/flow/*/v*/; do
    vibe registry publish "$pkg_dir" --json
done
```

## Exit codes

- `0` — success (or successful dry-run).
- `1` — generic error (manifest invalid, network, host API misbehaving, etc.).

## Related

- [`vibe install`](install.md) — the consumer-side counterpart.
- [authoring guides](../README.md) — how to write a publishable package.
- [`PROP-002 §2.10`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish) — the publish design lock (`RepoCreator` adapter pattern, error surface, token model).
