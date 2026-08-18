# vibe-index — operator handbook

Audience: an org owner who wants to host a metadata index for their
vibevm-shaped registry. Spec: [`spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005`](../../../spec/modules/vibe-index/PROP-005-package-index.md).

## Mental model in one paragraph

Each vibevm registry org optionally maintains a sibling repository
named `index` (or any name; `index` is the convention). Inside that
repo, `hello.json` plus `repomd.json` plus `primary.jsonl` plus
per-package `by-name/<name>.json` files describe the catalog.
Consumers detect the index by asking the handshake `hello.json` first
and falling back to a `repomd.json` probe for indexes that predate it;
when neither answers they use the live `git ls-remote` path. The vibevm
package repos themselves are unchanged — the index is a derived hot
cache, not a source of truth.

**What you actually own is the journal.** Every mutation appends a
fact to `<data-dir>/state/journal/` and the catalog is reprojected from
it; the published files are derived and hold no fact of their own. That
is the shape that decides your backups. Lose the journal and no amount
of re-scanning brings back a yank, a rename, a freeze or a tombstone —
a scan can only re-observe what the sources still say. Lose the catalog
and you have lost nothing that is not recoverable, but note **how** it
comes back: any mutation reprojects it wholesale, and
`cargo xtask rebuild --check <data-dir>` proves a catalog matches its
journal. There is deliberately **no repair verb** — nothing rewrites a
damaged catalog in place — so recovery is "make a mutation", not "run
the fixer". Back up `state/journal/`.

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

# 3. Verify every file the manifest lists still hashes to what it claims.
vibe-index verify  ./vibespecs-index
```

Now `./vibespecs-index/` is a directory holding `hello.json`,
`repomd.json`, `primary.jsonl`, and `by-name/<name>.json`. Push it to
your hosting (a sibling repo `<org>/index`, an S3 bucket, anything that
serves files over HTTP). `state/` stays behind — `init` gitignores it,
and it holds the journal, the tokens and the scan bookkeeping.

`verify` walks the manifest's own entries, so what it covers is exactly
what `repomd.json` lists. `hello.json` is outside that map by design and
therefore outside this check — a per-world manifest is the wrong organ
to vouch for the file that outlives every world. Nothing an attacker
could do with that gap reaches package CONTENT: a consumer verifies
`content_hash` against the bytes it actually fetched, so a swapped
handshake can misdirect a client to another index but cannot hand it
different content under a name it trusts.

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

## Force a full org re-enumeration

```sh
vibe-index rescan-org ./vibespecs-index \
    --from-github vibespecs \
    --token-file  ./gh-pat.txt
```

The org-image cache and its conditional validator are a **cheap
probabilistic** freshness check: a `304` on page 1 means "page 1
unchanged", not "the org is provably unchanged". A change the probe
misses is invisible from inside the index — no freshness mechanism
gives a hard guarantee, only a full traversal does. `rescan-org` is
that traversal, made its own verb so you can force one without
remembering a flag combination: it ignores the cache, never sends
`If-None-Match`, walks every page, and rewrites the image so the next
`reindex --cache-org` starts from a known-fresh baseline.

It runs in full mode, so it re-establishes the published set from what
the org currently serves. Facts the registry recorded — yanks,
tombstones, freezes, notices — are journal facts, not scan output, and
a rescan does not disturb them: a full pass writes a watershed into the
journal and re-projects from there, so recorded facts still apply on
top of the fresh set.

**Know this before you run one.** A record the scan no longer sees
simply stops appearing — no tombstone, no note, no line in the log. If
the source was merely unreachable (a network blip, a rate limit, a
revoked token) rather than deleted, the catalog quietly shrinks and
nothing says so. Run a full pass against a source you trust to be
answering completely, and read the run's output rather than its exit
code. Whether a vanished record should be buried with a reason or held
and marked unobserved is an open decision, not a settled behaviour.

## Turn the logs up

Logging is on for **every** subcommand at `warn` by default, not only
under `--auto-commit-push`. That is deliberate: the two things an
operator most needs to see — a version refused at load because this
build lacks a capability, and a failed auto-commit push — are WARN
events on ordinary read paths, and observability that depends on which
verb you happened to run is observability by accident.

```sh
vibe-index --log-level debug serve ./vibespecs-index    # global flag
VIBE_LOG=vibe_index::server=debug,warn vibe-index serve ./vibespecs-index
```

`--log-level` is a global flag taking one of `off`, `error`, `warn`,
`info`, `debug`, `trace`, and it **writes `VIBE_LOG`** before the
subscriber starts rather than overriding it. So the environment always
explains the output you are looking at: read `VIBE_LOG` off the process
and you know why it prints what it prints. `VIBE_LOG` itself takes the
full directive language when you want a per-module dial; the flag is
the coarse knob. There is no `RUST_LOG` fallback — one lever, not two.

## Wire it into publishers

When `VIBEVM_INDEX_URL_<REGISTRY>` and `VIBEVM_INDEX_TOKEN_<REGISTRY>`
both resolve at `vibe registry publish` time, the publisher POSTs
the freshly-built entry to `<index_url>/v1/packages` after the push.

```sh
export VIBEVM_INDEX_URL_VIBESPECS=https://index.example.com
export VIBEVM_INDEX_TOKEN_VIBESPECS="$(cat ~/.vibe/index.token)"
vibe registry publish ./flow-foo
```

The hook is opt-in per-registry. A failure of the index POST does
NOT fail the publish — the operator's next `vibe-index reindex`
covers the gap.

## Wire it into consumers

The consumer-side fast path (slice 10) lives inside vibe-registry.
When `VIBEVM_INDEX_URL_<REGISTRY>` is set in the consumer's
environment, `vibe install`'s version-enumeration walk probes the URL
and, on success, consults the index instead of `git ls-remote`. The
probe asks `hello.json` first, at both `<base>/v1/index` and `<base>`,
and only then falls back to a `repomd.json` probe for indexes built
before the handshake existed.

```sh
export VIBEVM_INDEX_URL_VIBESPECS=https://index.example.com/v1/index
vibe install flow:wal
```

A 404 or transient failure on the index transparently falls back to
the existing git path — that outcome is **absent**, and it is the only
silent one. An index that answers but this build cannot use — an
unknown handshake format, no world of its epoch, or a `401`/`403`
saying the index is private — is a **refusal**: it stops with the
offered epochs, this build's epoch and a recipe, instead of degrading
to git as though nothing were there. The difference matters to you
because a misconfigured private index would otherwise look exactly like
no index at all, and every install would silently take the slow path.

`content_hash` is still verified at fetch time regardless of how
versions were enumerated, per [PROP-002 §2.1].

## Token discipline

`<data-dir>/state/admin.tokens` is `0600` and gitignored by default.
The HTTP server never echoes token bytes in logs / responses /
error messages; the same discipline `vibe-publish` follows for its
host-API tokens applies here.

## Layout reference

```
<data-dir>/
├── hello.json            # the handshake clients read first
├── repomd.json           # manifest of what the writer writes
├── primary.jsonl         # + primary.jsonl.gz
├── by-name/
│   └── <name>.json       # no <kind>/ level: identity is (group, name)
├── by-cap/<slug>.jsonl
├── by-purl/<slug>.jsonl
├── state/                # gitignored — never published
│   ├── journal/          # <YYYY>-<MM>.ndjson — the truth; back THIS up
│   ├── server.lock       # PID file when serve is running
│   ├── admin.tokens      # one bearer token per line
│   ├── org-cache.json    # org-image cache for --cache-org / rescan-org
│   └── checkpoint.json   # incremental-reindex bookkeeping
├── .gitignore            # auto-generated; ignores /state/
└── README.md             # auto-generated
```

For the byte shape of each file, see [`format.md`](format.md).
For the HTTP API consumed by clients, see [`consumer-protocol.md`](consumer-protocol.md).
