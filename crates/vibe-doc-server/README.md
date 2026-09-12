# vibe-doc-server

The local documentation reader: the HTTP surface of the `vibe-doc`
pipeline, started by `vibe doc serve`.

It binds `127.0.0.1` and nothing else, serves the documentation package
it was pointed at, and reaches the network never — no request to
vibevm.org, no CDN, no font service, no analytics. That is what the mode
is for: this is how the documentation of a proprietary package is read,
and content that leaves the machine has already failed
([PROP-057 §11](../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)).

## What it serves today

Bare islands. A page comes back as the finished HTML of its content and
nothing else — no `<html>`, no navigation, no styles. The shell that
wraps them is a separate package and a later phase; what this server
returns is exactly what the public site glues into its own frame, which
is the whole point of there being one content path.

| address | what comes back |
|---|---|
| `<base><group>/<name>/<version>/<document>/` | the island, `text/html` |
| `<base>…/<document>.md` | `text/markdown` |
| `<base>…/<document>.xml` | `application/xml` |
| `<base>manifest.json` | the page manifest |
| `<base>llms.txt`, `llms-small.txt`, `llms-medium.txt`, `llms-full.txt` | the agent files |
| `/healthz` | `{"status":"ok"}` |

`<base>` is `/doc/` by default — the same mount the public site uses, so
a link written on a page works in both worlds. An address without its
trailing slash is a permanent redirect to the one with it.

## Shape, and one form it must not repeat

The server repeats `vibe-index`'s form — axum 0.8, one router builder,
RFC 7807 errors, `oneshot` tests with no listener bound — and imports
none of its code: `##PIPE-CRATES` says so, and the reason is that
`vibe-index`'s router is inseparable from a package index.

The form it must not repeat is `data_dir.join(<capture>)` over a
percent-decoded path capture. A router matches the raw path and decodes
the capture afterwards, so an escape that decodes to a separator becomes
one only once the match is made. Here the address is decoded first and
split by the server itself, and a segment that is not exactly one
ordinary name never reaches the filesystem.
