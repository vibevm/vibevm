# vibe-index — operator handbook

Audience: an org owner who wants to host a metadata index for their
vibevm-shaped registry. Spec: [`spec://vibevm/modules/vibe-index/PROP-005`](../../../spec/modules/vibe-index/PROP-005-package-index.md).

## Mental model in one paragraph

Each vibevm registry org optionally maintains a sibling repository
named `index` (or any name; `index` is the convention). Inside that
repo, `repomd.json` plus `primary.jsonl` plus per-package
`by-name/<kind>/<name>.json` files describe the catalog. Consumers
detect the index via HTTP GET on `repomd.json`; they fall back to
the live `git ls-remote` path when the index is absent. The vibevm
package repos themselves are unchanged — the index is a derived hot
cache, not a source of truth.

## Bootstrap from existing clones

The most common shape: you already have an org clone tree on a
host, and you want to expose its metadata as a static set of files.

```sh
# 1. Initialise an empty index.
vibe-index init  ./vibespecs-index \
    --registry      vibespecs \
    --registry-url  https://github.com/vibespecs \
    --naming        kind-name

# 2. Populate from your local clones.
vibe-index reindex ./vibespecs-index --from-clones /var/lib/vibespecs-mirror --full

# 3. Verify the manifest matches every file's recomputed hash.
vibe-index verify  ./vibespecs-index
```

Now `./vibespecs-index/` is a directory holding `repomd.json`,
`primary.jsonl`, and `by-name/<kind>/<name>.json`. Push it to your
hosting (a sibling repo `<org>/index`, an S3 bucket, anything that
serves files over HTTP).

## Bootstrap from GitHub directly (no local clones)

Slice 8 path. Walks the GitHub REST API, clones each repo into a
scratch dir, then runs the same scanner.

```sh
vibe-index reindex ./vibespecs-index \
    --from-github vibespecs \
    --token-file  ./gh-pat.txt \
    --clone-cache ./clones \
    --full
```

`--clone-cache` is optional; without it a tempdir is used and
discarded at end of run. With it, subsequent runs reuse the warm
cache.

## Run the live HTTP server

When you want real-time updates from `vibe registry publish` (the
post-publish hook from slice 9 POSTs to a server, not to static
files):

```sh
# 1. Drop one bearer token per line into the admin file.
mkdir -p ./vibespecs-index/state
chmod 700 ./vibespecs-index/state
echo "$(openssl rand -hex 32)" > ./vibespecs-index/state/admin.tokens
chmod 600 ./vibespecs-index/state/admin.tokens

# 2. Start the server.
vibe-index serve ./vibespecs-index \
    --bind             0.0.0.0:8412 \
    --auth-tokens-file ./vibespecs-index/state/admin.tokens
```

The default bind is `127.0.0.1:8412` (local-only). For external
exposure put a TLS-terminating reverse proxy in front. v0 does not
ship TLS termination — that is the proxy's job, same posture
`cargo`'s sparse index protocol takes.

`vibe-index stop ./vibespecs-index` reads `state/server.lock` and
sends `SIGTERM` (Unix) or prints the PID for `taskkill` (Windows).

## Schedule reindexing

Cron line that refreshes incrementally every 5 minutes:

```cron
*/5 * * * *  vibe-index reindex /home/owner/vibespecs-index \
                 --incremental \
                 --from-clones /var/lib/vibespecs-mirror \
             >>/var/log/vibe-index.log 2>&1
```

Incremental compares each repo's HEAD commit + tag list against
`state/checkpoint.json` and re-walks only what's changed.

## Wire it into publishers

When `VIBEVM_INDEX_URL_<REGISTRY>` and `VIBEVM_INDEX_TOKEN_<REGISTRY>`
both resolve at `vibe registry publish` time, the publisher POSTs
the freshly-built entry to `<index_url>/v1/packages` after the push.

```sh
export VIBEVM_INDEX_URL_VIBESPECS=https://index.example.com
export VIBEVM_INDEX_TOKEN_VIBESPECS="$(cat ~/.vibevm/index.token)"
vibe registry publish ./flow-foo
```

The hook is opt-in per-registry. A failure of the index POST does
NOT fail the publish — the operator's next `vibe-index reindex`
covers the gap.

## Wire it into consumers

The consumer-side fast path (slice 10) lives inside vibe-registry.
When `VIBEVM_INDEX_URL_<REGISTRY>` is set in the consumer's
environment AND the URL responds at `<base>/repomd.json` (or
`<base>/v1/index/repomd.json`), `vibe install`'s version-enumeration
walk consults the index instead of `git ls-remote`.

```sh
export VIBEVM_INDEX_URL_VIBESPECS=https://index.example.com/v1/index
vibe install flow:wal
```

A 404 or transient failure on the index transparently falls back to
the existing git path. `content_hash` is still verified at fetch
time regardless of how versions were enumerated, per
[PROP-002 §2.1].

## Token discipline

`<data-dir>/state/admin.tokens` is `0600` and gitignored by default.
The HTTP server never echoes token bytes in logs / responses /
error messages; the same discipline `vibe-publish` follows for its
host-API tokens applies here.

## Layout reference

```
<data-dir>/
├── repomd.json
├── primary.jsonl
├── by-name/
│   └── <kind>/
│       └── <name>.json
├── state/                # gitignored
│   ├── server.lock       # PID file when serve is running
│   ├── admin.tokens      # one bearer token per line
│   └── checkpoint.json   # incremental-reindex bookkeeping
└── README.md             # auto-generated
```

For the byte shape of each file, see [`format.md`](format.md).
For the HTTP API consumed by clients, see [`consumer-protocol.md`](consumer-protocol.md).
