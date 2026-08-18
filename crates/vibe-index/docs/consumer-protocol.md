# vibe-index — consumer protocol

Audience: clients of a vibe-index instance (vibevm itself; web UIs;
mirror tooling). Spec: [`PROP-005 §2.10`](../../../spec/modules/vibe-index/PROP-005-package-index.md#http)
for the HTTP surface, [`§2.1`](../../../spec/modules/vibe-index/PROP-005-package-index.md#optional)
for discovery, [`§2.19`](../../../spec/modules/vibe-index/PROP-005-package-index.md#unavailable)
for the refusal a client must be able to read.

## Two consumption shapes

A vibe-index data directory can be consumed two ways:

- **Static files over plain HTTP** — `<base>/hello.json`,
  `<base>/repomd.json`, `<base>/primary.jsonl`,
  `<base>/by-name/<name>.json`. Works against any HTTP host that can
  serve a directory of files (raw GitHub URLs, an S3 bucket, an nginx
  static site).
- **Live HTTP server** — `vibe-index serve` exposes the same files
  under `/v1/index/...` plus structured query routes under
  `/v1/packages*`, `/v1/capabilities/{cap}`, `/v1/purls/{purl}`,
  plus mutating POST/DELETE behind bearer-token auth.

Both shapes answer the same bytes for the same file. Nothing below is
specific to the server unless it says so.

## Discovery — ask the handshake first {#discovery}

`hello.json` is the one file in this system that never changes meaning.
Everything else — the catalog, its schema, its layout — is disposable
and may be re-minted at a new path. So a client's entry point is the
handshake, and it is asked **before** any `repomd.json` probe, at
**both** candidate bases:

```
GET <base>/v1/index/hello.json
GET <base>/hello.json
```

Asking it second would defeat the only key it exists for. `successor`
is the in-band forwarding pointer of a moved index, and it is readable
exactly when the old address no longer serves a catalog — a handshake
sought only beside a *found* `repomd.json` would never be read
precisely when it matters.

```json
{
  "vibe": "hello/1",
  "worlds": [ { "epoch": 1, "path": "." } ]
}
```

- `vibe` — the version of the **handshake format itself**. A document
  whose value this build does not know is a loud refusal, never a
  guess.
- `worlds[]` — the catalog worlds this index offers. `epoch` is the
  catalog world; `path` refines the base (`"."` leaves it untouched).
  A world may carry `sunset` (RFC-3339) to announce its retirement.
- Optional top-level keys: `min_client` (semver), `notice` (operator
  text), `successor` (where this index moved to).

The two version families answer different questions and are not
interchangeable: `vibe` is the handshake's own format, `worlds[].epoch`
is which catalog world a client can read.

### A probe has three outcomes, not two

| outcome | when | what a client does |
|---|---|---|
| **found** | a handshake offered a world of this client's epoch — or, on an index with no handshake at all, `repomd.json` answered 200 | use the index |
| **absent** | nothing answered (404, connect failure, 5xx) | fall back to the live `git ls-remote` path; say nothing |
| **refused** | the handshake parsed but this build cannot use it (unknown handshake format, no world of its epoch, unparseable body), or any probe step answered 401/403 | stop and report, with the recipe the refusal carries |

Only **absent** is silent. A refusal names the epochs the index offers,
the epoch this build reads, and the fix; when the document carried
`min_client`, `notice` or `successor`, the refusal names those too.

**`successor` is named, never followed.** Following a pointer to
another host automatically would widen the client's scope without a
loop guard or a trust rule, so the refusal prints it and the next move
is a person's — one command away.

**Compatibility with pre-handshake indexes is unchanged.** When neither
candidate answers `hello.json` with 200, the client probes
`<base>/repomd.json` and `<base>/v1/index/repomd.json`, first 200 wins.
The price of asking the handshake first is paid only by handshake-less
indexes: up to two extra GETs, on a 5-second probe timeout.

## Read endpoints

```
GET /healthz                                   liveness
GET /readyz                                    readiness

GET /v1/index/hello.json                       the eternal handshake — ask this first
GET /v1/index/repomd.json                      manifest of the files the WRITER writes
GET /v1/index/primary.jsonl                    one VersionEntry per line, sorted
GET /v1/index/primary.jsonl.gz                 deterministic gzip sibling
GET /v1/index/by-name/{name}.json              candidate set for one bare name
GET /v1/index/by-cap/{slug}                    inverted index by capability
GET /v1/index/by-purl/{slug}                   inverted index by `describes` PURL

GET /v1/packages?kind=&q=&limit=&offset=       list / search
GET /v1/packages/{group}/{name}                all versions of one package
GET /v1/packages/{group}/{name}/{version}      single VersionEntry

GET /v1/capabilities/{capability}              who provides this
GET /v1/purls/{purl}                           who describes this upstream library

GET /v1/admin/status                           uptime + counts (read-only)
GET /metrics                                   Prometheus 0.0.4 text
```

Identity is `(group, name)` — a reverse-FQDN group plus a bare name.
`kind` rides on the entry as metadata and is a query filter, never a
path segment: there is no `<kind>/` level anywhere in the layout or the
routes.

`repomd.json` is the manifest of **what the writer writes** — the
catalog files and directories — and not of every byte in the directory.
`hello.json` is deliberately outside it: `repomd.json` describes ONE
world, while the handshake stands above worlds and dispatches to them,
so a per-world manifest is the wrong organ to vouch for the one file
that outlives every world. `README.md`, `.gitignore` and all of
`state/` are outside it for the plainer reason that the writer does not
produce them.

CORS is open on every read endpoint (web UIs from any origin).

## Write endpoints

Behind `Authorization: Bearer <token>` against tokens loaded from
`<data-dir>/state/admin.tokens`.

```
POST   /v1/packages                            body: a VersionEntry — insert/upsert
DELETE /v1/packages/{group}/{name}             drop every version
DELETE /v1/packages/{group}/{name}/{version}   drop one version
```

Refused with 403 when the server runs in `--read-only` mode or no
tokens are loaded; 401 when the bearer is missing or invalid.

A mutation is an append to the registry journal followed by a
reprojection of the whole catalog — the server never reads a catalog
file in order to rewrite it. Two consequences a client can rely on: a
repeated identical upsert changes nothing and produces no publish, and
the catalog a client reads is always a projection of facts, never a
partially-edited previous catalog.

## Error envelope

RFC-7807 problem details:

```json
{
  "type":   "vibe-index/error/not-found",
  "title":  "resource not found",
  "status": 404,
  "detail": "`org.vibevm/wal` is not in the index"
}
```

Error codes (`type` suffix): `not-found`, `bad-request`, `internal`,
`unauthorized`, `forbidden`, `rate-limited`, `unavailable`.

### `unavailable` — a version that stands but cannot be used {#unavailable}

A record may declare `must_understand`: capabilities a reader must
understand before acting on it. A client — or this server — that lacks
one of them may not use that version, and **must not be told nothing**.

The status stays `404`: "you did not get the thing" is still true, and
moving it would change the contract for every existing client over a
case none of them handles specially. What changes is the body — `type`
and `title` name the refusal in its own words, and an **extension
member** (which RFC 7807 provides for exactly this) carries the whole
row:

```json
{
  "type":   "vibe-index/error/unavailable",
  "title":  "version unavailable to this build",
  "status": 404,
  "detail": "`org.vibevm/wal@0.2.0` stands in the index, but this build cannot act on it — …",
  "unavailable": {
    "group":   "org.vibevm",
    "name":    "wal",
    "version": "0.2.0",
    "missing": ["wal/epoch-bump"],
    "recipe":  "this build does not understand `wal/epoch-bump` (reader capabilities — spec://org.vibevm.core/vibevm/common/PROP-044#machinery); fix: update vibe-index to a build that names them, or ask for a version this build can act on"
  }
}
```

The `recipe` is built in one place and never written as a literal at a
call site, so every surface says the same thing about the same missing
capability. It is a function of the **capability**, not of the format
the catalog happens to be written in — what decides that a capability
is understood decides what to say when it is not.

**Which surfaces owe this answer, and which do not.** Every surface
that *computes* an answer owes it — the five query routes
(`/v1/packages` list/search, package versions, single version,
`/v1/capabilities/{cap}`, `/v1/purls/{purl}`) and the seven CLI verbs
that answer by name. A surface that hands back a **stored file
verbatim** does not: `GET /v1/index/by-name/{name}.json` returns the
record word for word, `must_understand` included — and that declaration
*is* the explanation. It was never the silent one; silence lived in the
surfaces that computed an answer and dropped a record without a word.

The judgement rides the envelope and never enters the record. Quarantine
is a reader's judgement about the pair «record × build», not a property
of the record, so it is never stored on the wire — which is also why the
command line and the server answer identically by construction, however
the index reached memory.

The operational counters (`/v1/admin/status`, `/metrics`) deliberately
report the raw state: they say what the index **holds**, including
versions unusable to this build. They are telemetry about content, not
an answer about a package.

## Identity invariant

The index records a `content_hash` per VersionEntry, but consumers
that act on the data MUST still verify hash equality at fetch time.
The index is a hot cache; the package repos remain source of
truth. Per [PROP-002 §2.1], a `content_hash` mismatch between an
index claim and the actually-fetched bytes is a hard
`IntegrityError` that aborts install — the index can mislead the
version selector but cannot substitute content.

The hash string names the recipe that produced it — `sha256-tree/1:<hex>`.
A bare `sha256:<hex>` is recipe 0, frozen legacy, and readers accept it.
A client comparing hashes compares recipes too; two values computed
under different recipes are not comparable and never equal by accident.

## Pagination

The mutation endpoints do not paginate. The list endpoint accepts
`?limit=` (default 50) and `?offset=` (default 0). For very large
indices, prefer `GET /v1/index/primary.jsonl` (a single streaming
response) over walking the paginated REST surface.
