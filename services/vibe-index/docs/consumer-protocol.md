# vibe-index — consumer protocol

Audience: clients of a vibe-index instance (vibevm itself; web UIs;
mirror tooling). Spec: [`PROP-005 §2.10`](../../../spec/modules/vibe-index/PROP-005-package-index.md#http).

## Two consumption shapes

A vibe-index data directory can be consumed two ways:

- **Static files over plain HTTP** — `<base>/repomd.json`,
  `<base>/primary.jsonl`, `<base>/by-name/<kind>/<name>.json`. Works
  against any HTTP host that can serve a directory of files (raw
  GitHub URLs, an S3 bucket, an nginx static site).
- **Live HTTP server** — `vibe-index serve` exposes the same files
  under `/v1/index/...` plus structured query routes under
  `/v1/packages*`, `/v1/capabilities/{cap}`, `/v1/purls/{purl}`,
  plus mutating POST/DELETE behind bearer-token auth.

Consumers should probe both `<base>/repomd.json` and
`<base>/v1/index/repomd.json`; the first 200 response wins. The
vibe-registry slice-10 fast path does this auto-probe at
`MultiRegistryResolver::from_manifest` time.

## Read endpoints

```
GET /healthz                                   liveness
GET /readyz                                    readiness

GET /v1/index/repomd.json                      manifest (sha256 of every other file)
GET /v1/index/primary.jsonl                    one VersionEntry per line, sorted
GET /v1/index/by-name/{kind}/{name}.json       cargo-sparse-style per-package file

GET /v1/packages?kind=&q=&limit=&offset=       list / search
GET /v1/packages/{kind}/{name}                 all versions of one package
GET /v1/packages/{kind}/{name}/{version}       single VersionEntry

GET /v1/capabilities/{capability}              who provides this
GET /v1/purls/{purl}                           who describes this upstream library

GET /v1/admin/status                           uptime + counts (read-only)
GET /metrics                                   Prometheus 0.0.4 text
```

CORS is open on every read endpoint (web UIs from any origin).

## Write endpoints

Behind `Authorization: Bearer <token>` against tokens loaded from
`<data-dir>/state/admin.tokens`.

```
POST   /v1/packages                            body: a VersionEntry — insert/upsert
DELETE /v1/packages/{kind}/{name}              drop every version
DELETE /v1/packages/{kind}/{name}/{version}    drop one version
```

Refused with 403 when the server runs in `--read-only` mode or no
tokens are loaded. 401 when the bearer is missing or invalid.

## Error envelope

Lightweight RFC-7807-shape:

```json
{
  "type":   "vibe-index/error/not-found",
  "title":  "resource not found",
  "status": 404,
  "detail": "`flow:wal` is not in the index"
}
```

Error codes (`type` suffix): `not-found`, `bad-request`, `internal`,
`unauthorized`, `forbidden`, `integrity-mismatch`.

## Identity invariant

The index records a `content_hash` per VersionEntry, but consumers
that act on the data MUST still verify hash equality at fetch time.
The index is a hot cache; the package repos remain source of
truth. Per [PROP-002 §2.1], a `content_hash` mismatch between an
index claim and the actually-fetched bytes is a hard
`IntegrityError` that aborts install — the index can mislead the
version selector but cannot substitute content.

## Pagination

The mutation endpoints do not paginate. The list endpoint accepts
`?limit=` (default 50) and `?offset=` (default 0). For very large
indices, prefer `GET /v1/index/primary.jsonl` (a single streaming
response) over walking the paginated REST surface.
