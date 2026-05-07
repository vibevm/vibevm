# `vibe search` — full-text query over configured registries

Search every `[[registry]]` configured in `vibe.toml` for packages whose name, description, keywords, capabilities, or `describes` PURL match a query. Read-only — does not touch the lockfile, the registry cache, or any remote git host.

`vibe search` consults each registry's optional **package index** ([PROP-005](../../spec/modules/vibe-index/PROP-005-package-index.md)) — a per-org metadata service that ships separately from the registry itself. Without an index URL configured for a registry, that registry is reported as `registries_unconfigured` in the envelope and silently skipped. Without an index there is nothing fast to query — naive `git ls-remote`-shape enumeration across an org of 100+ packages would be unacceptably slow, so the search command refuses to do that. Operators that want search either run their own [`vibe-index`](../../services/vibe-index/) instance or wait for one to land at the upstream org.

Spec: [ROADMAP §M2.10](../../ROADMAP.md), [PROP-005 §2.10](../../spec/modules/vibe-index/PROP-005-package-index.md#index-routes), [PROP-004 §5.12](../../spec/research/PROP-004-tessl-comparative-research.md#search).

## Usage

```
vibe search <query>...
            [--purl <PURL>]
            [--kind <flow|feat|stack|tool>]
            [--registry <name>]
            [--limit <N>]
            [--full-scan]
            [--no-cache | --cache-ttl <SECONDS>]
            [--path <dir>]
            [--json | --quiet]
```

Two query modes (mutually exclusive):

- **Free-text** — positional `<query>...` arguments. Multiple words are joined with a single space before being sent to the index.
- **PURL lookup** — `--purl <PURL>` runs an exact match against `[package].describes` and any subskill-level `describes`. Hits carry a `binding_site` field (`package` vs `subskill`) so consumers see where the match originated.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `<query>...` | Free-text query. Tokenised server-side (lowercase ASCII alphanumeric runs, ~30-stopword filter, single-character tokens dropped). Mutually exclusive with `--purl`. | — |
| `--purl <PURL>` | Direct PURL lookup. Errors if the value does not start with `pkg:`. Mutually exclusive with the positional query. | — |
| `--kind <K>` | Restrict to one package kind. Applies only to free-text search; PURL lookup ignores it. | all |
| `--registry <NAME>` | Walk only the named `[[registry]]`. Errors if `NAME` is not in `vibe.toml`. | walk every configured registry |
| `--limit <N>` | Maximum hits to fetch from each registry's index. The server may apply its own cap; the union is then deduplicated by `(kind, name)` keeping the highest-score variant. Ignored on `--purl` lookups. | `20` |
| `--full-scan` | For registries without a configured `VIBEVM_INDEX_URL_<R>`, fall back to a naive org-walk via the host's REST API. v0 supports `github.com` only. Slower than an index; rate-limited. Ignored on `--purl` lookups. | off |
| `--no-cache` | Bypass the persistent search cache under `~/.vibe/search-cache/`. Reads still go to the network even when a fresh entry exists; freshly-fetched results are not written back. | off |
| `--cache-ttl <SECONDS>` | Override the default cache TTL. Entries older than this many seconds are misses. | `3600` (1h) |
| `--path <dir>` | Project root with `vibe.toml`. | `.` |
| `--json` | Structured JSON envelope — see [Output (JSON)](#output-json). | off |
| `--quiet` | One-line summary `vibe search: N hits across M registries`. | off |

## Index URL convention

Per registry, set `VIBEVM_INDEX_URL_<NAME>` in the environment to point at a vibe-index server (or any endpoint that serves the [PROP-005 §2.10](../../spec/modules/vibe-index/PROP-005-package-index.md#index-routes) wire shape). The suffix is uppercase ASCII alphanumeric with non-alphanumeric characters folded to `_`:

- `vibespecs` → `VIBEVM_INDEX_URL_VIBESPECS`
- `vibespecs-gitverse` → `VIBEVM_INDEX_URL_VIBESPECS_GITVERSE`

The same convention is used by `vibe registry publish`'s post-publish hook ([PROP-005 §2.14](../../spec/modules/vibe-index/PROP-005-package-index.md#publish-hook)) and the `vibe-registry`-side index fast path. One env-var feeds every consumer.

For each registry, `vibe search`:

1. Reads `VIBEVM_INDEX_URL_<R>` from the environment. Unset → `registries_unconfigured`.
2. Probes `<base>/repomd.json` (and `<base>/v1/index/repomd.json` for the static-mirror layout). 200 → keep going. Anything else → `registries_unreachable`.
3. Fetches `<base>/v1/packages?q=<query>[&kind=][&limit=]`. Non-200 → `registries_unreachable`.

A 404 on the search route specifically means the URL points at a static raw-file mirror without the live-server route — search is unavailable on that mirror, but version-fetch (`list_versions`) still works through the `by-name/` files.

## Output

### Human-readable

```
query     : wal
registries: 1 searched, 0 unreachable, 1 without index URL
  searched: vibespecs
  no VIBEVM_INDEX_URL_<R> set: vibespecs-gitverse

KIND    NAME                          LATEST       SCORE  REGISTRY              DESCRIPTION
flow    wal                           0.1.0        3      vibespecs             Write-ahead log discipline for spec-driven projects.

1 hit across 1 registry
```

When no registry has an index URL configured, the summary line spells out the index-vs-install distinction:

```
(no registry has VIBEVM_INDEX_URL_<R> configured; search returns empty.
 To install a known package, run `vibe install <kind>:<name>` directly —
 install resolves through `[[registry]]` over git and does not need an
 index. The index is a discovery optimisation, not a runtime dependency.
 See docs/commands/search.md for setting up an index server.)
```

`vibe install` does not consult the index: it walks `[[registry]]` priority order, asks each adapter to enumerate tags, and fetches the matching version. The index speeds up discovery and listing — running an index server is optional and can be deferred until a project actually wants `vibe search`.

### Output (JSON, free-text)

```jsonc
{
  "ok": true,
  "command": "search",
  "project": "/path/to/project",
  "query": "wal",
  "registries_searched": ["vibespecs"],
  "registries_unconfigured": ["vibespecs-gitverse"],
  "registries_unreachable": [],
  "hit_count": 1,
  "hits": [
    {
      "kind": "flow",
      "name": "wal",
      "latest_stable": "0.1.0",
      "score": 3,
      "matched_tokens": ["wal", "log", "ahead"],
      "description": "Write-ahead log discipline for spec-driven projects.",
      "registry": "vibespecs",
      "source": "index"
    }
  ]
}
```

`hits[].source` is `"index"` for results served from a `vibe-index` server and `"full-scan"` for results from the `--full-scan` org-walk fallback. `registries_unreachable[]` carries `{ name, reason }` per failure (HTTP status / connect-fail / malformed JSON). When `--full-scan` is active, two additional fields surface: `registries_full_scanned[]` (registries whose org-walk succeeded) and `registries_full_scan_unsupported[]` (non-GitHub hosts the v0 fallback can't handle). Both are omitted from the envelope when `--full-scan` is off.

`ok` stays `true` even when every registry fails — the command surfaces the failure mode in the envelope rather than aborting, so a CI step that wants strict semantics can `jq -e '.registries_unreachable | length == 0'`.

### Output (JSON, --purl lookup)

```jsonc
{
  "ok": true,
  "command": "search:purl",
  "project": "/path/to/project",
  "purl": "pkg:cargo/sqlx@0.8.0",
  "registries_searched": ["vibespecs"],
  "registries_unconfigured": ["vibespecs-gitverse"],
  "registries_unreachable": [],
  "hit_count": 2,
  "hits": [
    {
      "kind": "flow",
      "name": "sqlx-skin",
      "version": "0.1.0",
      "binding_site": "package",
      "registry": "vibespecs"
    },
    {
      "kind": "stack",
      "name": "rust",
      "version": "0.2.0",
      "binding_site": "subskill",
      "registry": "vibespecs"
    }
  ]
}
```

`binding_site` is `"package"` when the PURL appears on the entry's top-level `[package].describes` field, `"subskill"` when it comes from a subskill-level `describes`. Hits are deduplicated by `(kind, name, version)` across registries — earlier registries (vibe.toml priority order) win on ties.

## Examples

```bash
# Search every configured registry for the WAL flow.
vibe search wal

# Restrict to flows only.
vibe search wal --kind flow

# One specific registry.
vibe search atomic --registry vibespecs

# Multi-word query.
vibe search "ahead of time" --kind feat

# Higher limit for deep org-wide searches.
vibe search auth --limit 100

# Find every package binding to a specific upstream library.
vibe search --purl pkg:cargo/sqlx@0.8.0

# Org-walk fallback for a registry without a vibe-index server (GitHub only in v0).
vibe search auth --full-scan

# Force a refresh past the 1-hour TTL.
vibe search auth --no-cache

# Force a refresh past a smaller TTL — useful in CI to keep search results fresh.
vibe search auth --cache-ttl 60

# Programmatic.
vibe --json search auth | jq '.hits[].name'
```

## Edge cases

- **Query is empty after trimming.** Errors before any HTTP call. Multi-arg queries that consist entirely of whitespace or stopwords still send the original string to the server, which decides matching semantics.
- **`--registry NAME` does not match any `[[registry]]`.** Errors with the list of configured registry names so the operator sees the typo.
- **No registry has an index URL set.** Hit count is 0; summary line surfaces the missing-config state. Not an error — this is the expected state for projects whose orgs do not run an index yet.
- **Registry probes succeed but search returns 503 / 5xx.** Reported as `registries_unreachable` for that registry; the run continues across the remaining registries.
- **Same `(kind, name)` shows up on two registries.** Deduplicated to one row; the row with the higher `score` wins. Ties resolve to whichever registry came earlier in `vibe.toml`.

## Limitations (v0)

- **`--full-scan` is GitHub-only.** v0 supports `github.com`-hosted registries only. Other hosts surface in `registries_full_scan_unsupported[]`. GitVerse parity unblocks once their public REST API exposes org-scoped repo enumeration ([PROP-002 §2.10](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish) tracks the same gap for `vibe registry publish`).
- **Stable-only ranking signal.** Server-side score is term-overlap (one point per matched query token). Tantivy / BM25 upgrades land server-side without a client change — the wire shape `{score: u32}` is the same.
- **`--full-scan` reads HEAD, not tags.** What lands in `latest_stable` for a full-scan hit is the manifest's declared `version`, not the highest semver tag in the repo. For most vibevm packages these match; the gap matters only when a repo is mid-release.
- **`--full-scan` rate limit.** Anonymous GitHub API allows 60 req/h; setting `VIBEVM_PUBLISH_TOKEN_GITHUB` lifts that to 5000 req/h. Hard cap of 500 repos scanned per registry per invocation prevents runaway scans on huge orgs — operators with bigger orgs should run an index instead.
- **Cache scoped per `(registry, query, kind, limit)`.** Changing the index URL for a registry without a fresh-fetch flag continues to serve the old cache for up to 1 hour. Use `--no-cache` to force a refresh after a server-side rotation.
- **PURL lookups bypass the cache.** Exact-match queries are cheap on the server side; we don't bother caching them. v0 keeps it simple; a follow-up may add a parallel cache layer if PURL searches become a bottleneck.

## Related

- [PROP-005](../../spec/modules/vibe-index/PROP-005-package-index.md) — full design of the per-org package index, including the `/v1/packages` query route and operator handbook.
- [`services/vibe-index/`](../../services/vibe-index/) — standalone utility that produces and serves the index.
- [`vibe registry publish`](registry-publish.md) — populates the index via the post-publish hook when `VIBEVM_INDEX_URL_<R>` and `VIBEVM_INDEX_TOKEN_<R>` are set.
- [`vibe outdated`](../../ROADMAP.md#m110--vibe-outdated) — cousins-by-source: same `IndexClient` underpins outdated-checks and search.
- [`vibe show config`](show.md) — surfaces the `VIBEVM_INDEX_URL_<R>` env-vars that decide which registries are searchable.
