# `vibe registry publish` — publish a package directory

Maintainer-side command. Takes a directory containing a `vibe.toml` with a `[package]` table and publishes it as a tagged release in the configured registry's organization. Creates the per-package repo (or reuses an existing one), pushes contents, tags the version. Mechanical only — no semantic / LLM-backed review (that's reserved for v2+ per [`VIBEVM-SPEC.md` §8.5](../../VIBEVM-SPEC.md)).

## Usage

```
vibe registry publish <source> [--registry <name>] [--path <project>]
                      [--dry-run]
                      [--json | --quiet]
```

## Arguments

- `<source>` — path to the package directory. Must contain a `vibe.toml` carrying a `[package]` table at its root, with the package content laid out per the mirror convention ([`VIBEVM-SPEC.md` §13.1](../../VIBEVM-SPEC.md)).

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--registry <name>` | Which `[[registry]]` in `vibe.toml` to publish into. | the first registry |
| `--path <project>` | Project directory containing `vibe.toml`. | `.` |
| `--dry-run` | Describe what would happen but make no API calls or pushes. | off |
| `--json` | Structured payload. Schema: [`schemas/registry_publish_report.jtd.json`](../../schemas/registry_publish_report.jtd.json). | off |
| `--quiet` | One-line summary. | off |

## Host adapter

The CLI picks a `RepoCreator` from the registry URL's hostname:

| Host | Adapter | Status |
| --- | --- | --- |
| `github.com` (and subdomains) | `GitHubCreator` | primary, used by `vibespecs` registry |
| `gitverse.ru` | `GitVerseCreator` | retained; `POST /orgs/{org}/repos` not exposed by live host — `repo_exists()` works, but creation requires manual web-UI pre-create |

Other hosts: clean error pointing at [PROP-002 §2.10](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish). Adding a new host is one new `impl RepoCreator`.

## Authentication

A publish token is required. Order of precedence:

1. `VIBEVM_PUBLISH_TOKEN` environment variable (highest; useful in CI).
2. `~/.vibe/<host-prefix>.publish.token` — per-host file. The prefix is the first label of the host: `github.publish.token` for `github.com`, `gitverse.publish.token` for `gitverse.ru`, `gitlab.publish.token` for `gitlab.com`, etc.
3. `~/.vibe/git.publish.token` — legacy host-agnostic fallback. Kept so existing GitVerse-only setups keep working without rename.

Token must be issued by the host and must have `repo:create` permission in the target organization. For GitHub, that's a personal access token (PAT) with the `repo` scope on the `vibespecs` org. For GitVerse, see <https://gitverse.ru/docs/public-api/>.

**Token secrecy** ([PROP-000 §20](../../spec/common/PROP-000.md#token-secrecy)). The token value is **never** logged, never printed, never embedded in any `vibe`-produced artefact. `Debug` and `Display` of the in-process `Token` render as `***`. The CLI reports the *source* of the token (env-var name or file path) — that's safe metadata. For HTTPS-token-auth hosts (GitHub), the publisher injects the token into the push URL as `https://x-access-token:<TOKEN>@host/org/repo.git` for the duration of one `git remote add` / `git push` invocation; modern git (≥ 2.31) redacts URL passwords in its own log output. The token is never written to disk by `vibe` and never appears in any committed file.

## Pipeline

1. **Read manifest** at `<source>/vibe.toml`. It must carry a `[package]` table and use the current `[requires]` / `[conflicts]` shape — there is no legacy `[dependencies]` form, and a manifest using one is a hard parse error.
2. **Compute repo name** under the org via the registry's `naming` convention. Default `fqdn` produces `<group>.<name>`; alternatives are `kind-name`, `name`, or `kind/name` per [PROP-002 §2.2](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model) / [PROP-008 §2.5](../../spec/modules/vibe-registry/PROP-008-qualified-naming.md).
3. **Pick host adapter** from the registry URL's host. Loads the appropriate token via the precedence above.
4. **Check or create repo.** `GET /repos/{org}/{repo}` (or the host's analogue) probes existence; if missing, `POST /orgs/{org}/repos` creates it with `auto_init = false` (we push our own initial commit). On hosts that don't expose org-scoped creation (current GitVerse), the operator pre-creates the empty repo via the web UI; the publisher then takes the `repo_exists() == true` path and proceeds straight to push.
5. **Stage in a temp working tree.** Contents copied (excluding any `.git/` subtree); `git init`, repo-local identity (`publish@vibevm.local`), `git add -A`, commit `Release <name>@<version>`.
6. **Push** `main` first, then the annotated tag `v<version>`. The push URL — `clone_url` for SSH-auth hosts (GitVerse), `https://x-access-token:<TOKEN>@…` for HTTPS-token-auth hosts (GitHub) — is constructed by the host adapter via `RepoCreator::push_url()`.

`--dry-run` runs steps 1–4's read leg only. The `repo_exists()` probe still runs (read-only); creation, staging, and push are skipped. The dry-run output describes what *would* happen.

## Errors

| Failure | Maps to |
| --- | --- |
| `401` / `403` from API | "publish refused: token lacks `repo:create` permission in `<org>` on `<host>`". |
| `404` from API on org-level endpoint | "publish refused: organization `<org>` does not exist on `<host>` (or the token cannot see it)". |
| `git push` denied | "publish refused: no push access to `<repo>`. Ask a maintainer of that repo to grant push." |
| Tag already exists | "publish refused: tag `<tag>` already exists. Pick a new version — `vibe registry publish` does not force-push." |
| Network unreachable | "publish refused: host `<host>` is unreachable." |
| Token missing | "publish refused: no token available. Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibe/<host>.publish.token` (e.g. `github.publish.token`)." |
| Unsupported host | "publish refused: no RepoCreator adapter for host `<host>`. Configured registry URL points at an unsupported host; add an adapter per PROP-002 §2.10 or use a supported host." |

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
