# vibe-index — file format reference

Spec: [`PROP-005 §2.4 / §2.6`](../../../spec/modules/vibe-index/PROP-005-package-index.md#layout).

## On-disk layout

```
<data-dir>/
├── hello.json                           the eternal handshake — read FIRST
├── aliases.toml                         the short names the OWNER ISSUES — an INPUT
├── repomd.json                          manifest of what the WRITER writes
├── primary.jsonl                        one VersionEntry per line, sorted
├── primary.jsonl.gz                     deterministic gzip sibling
├── by-name/
│   └── <name>.json                      candidate set for one bare name
├── by-cap/<slug>.jsonl                  inverted index by capability
├── by-purl/<slug>.jsonl                 inverted index by `describes` PURL
├── README.md                            auto-generated pointer
├── .gitignore                           written once by `init`; ignores /state/
└── state/                               gitignored — never reaches a client
    ├── journal/<YYYY>-<MM>.ndjson       the registry's fact journal — the TRUTH
    ├── server.lock                      PID file when `serve` is running
    ├── admin.tokens                     bearer tokens — keep 0600; nothing enforces it
    ├── org-cache.json                   org-scan bookkeeping
    └── checkpoint.json                  incremental-reindex bookkeeping
```

**The catalog is a projection; the journal is the truth.** Every
mutation appends a fact to `state/journal/` and then reprojects the
whole catalog from the journal — nothing here is ever read in order to
be rewritten. So the files above hold no fact of their own, and
`cargo xtask rebuild --check <data-dir>` proves it: it projects the
journal into a scratch directory and asserts the catalog is
byte-identical. A difference means the catalog carries something the
journal does not describe, which is a derived artifact holding truth.

**The proof ships; the repair does not.** `rebuild` has only
`--check` — there is no verb that rewrites a damaged catalog from its
journal in place, and that is a stated decision rather than a gap. What
regenerates the catalog today is any mutation, because every mutation
reprojects it wholesale. The direction of repair is fixed and only one
way round: a catalog that disagrees with its journal is the wrong one,
and editing the journal to match it would launder the discrepancy into
the truth layer.

The journal lives under `state/` because it is OURS and not the
client's: the whole of `state/` is gitignored, absent from
`repomd.json`, and served by no route.

**`repomd.json` is the manifest of what the writer writes** — the
catalog files and directories above it — not of every byte in the
directory. "The writer" is the projection that emits the catalog, and
it is not `init`: `README.md` and `.gitignore` are written once at
initialisation and never again, `state/` is runtime data, and none of
the three is catalog, which is why none is in the map. The rule is
positive on purpose — compare the set the writer WRITES, rather than
excluding a list of everything else, because a blacklist rots the day
the directory grows a file nobody listed. `hello.json` is outside it
for a different and load-bearing reason: `repomd.json` describes ONE
world, while the handshake stands above worlds and dispatches to them.
Making a per-world manifest vouch for the one file that outlives every
world would invert the layers, and the inversion becomes unanswerable
the day there are two worlds — whose `repomd` hashes the handshake?

**`hello.json`** is documented with the discovery ladder it belongs to,
in [`consumer-protocol.md`](consumer-protocol.md#discovery).

The `<data-dir>` doubles as the working tree of a `git`-tracked
index repository. Operators commit + push the non-`state/` content
themselves, or run `serve --auto-commit-push` and the index commits
and pushes itself after every successful mutation (its preflight
refuses to boot unless the data dir is a git working copy with
`state/` gitignored).

`by-cap` / `by-purl` slugs are filesystem-safe encodings of the
capability / PURL string: `:`, `/` and `@` each become `--` (NTFS
reserves `:`, so no slug may carry it). The canonical string lives
inside the file's lines; the slug is only a lookup key.

## `repomd.json` — RPM-style manifest

```json
{
  "schema_version": 1,
  "registry": "vibespecs",
  "registry_url": "https://github.com/vibespecs",
  "naming": "kind-name",
  "generated_at": "2026-05-06T12:00:00Z",
  "generator": "vibe-index 1.0.0",
  "package_count": 42,
  "version_count": 117,
  "files": {
    "primary.jsonl":            { "kind": "file", "size": "184522", "sha256": "sha256:..." },
    "primary.jsonl.gz":         { "kind": "file", "size": "42110",  "sha256": "sha256:..." },
    "by-name":                  { "kind": "directory", "entries": 42 },
    "by-name/wal.json":         { "kind": "file", "size": "5120",  "sha256": "sha256:..." },
    "by-cap":                   { "kind": "directory", "entries": 7 },
    "by-cap/wal.jsonl":         { "kind": "file", "size": "890",   "sha256": "sha256:..." },
    "by-purl":                  { "kind": "directory", "entries": 3 }
  }
}
```

`files[*]` is a **tagged union**: every value carries `kind`, either
`"file"` (with `size` + `sha256`) or `"directory"` (with `entries`).
`size` is a **canonical decimal string** — ASCII digits only, no sign, no
leading zeros except the bare `0`, value within u64 (integers wider than
32 bits ride the wire as strings; `formats/breaks/003.md`).
Tags are lowercase. Both arms carry the tag — a file entry without
`kind` is **refused**, not guessed at, which is the whole point of the
shape: an untagged union silently swallows a value it half-recognises,
and it was the one type in this catalog that could. The change was a
real wire break and carries its note in
[`formats/breaks/001.md`](../../../formats/breaks/001.md); the recipe
there is one line, because a derived catalog is rebuilt rather than
migrated.

`primary.jsonl.gz` is byte-deterministic (level 6, `mtime=0`), so its
`sha256` is stable across machines for identical input.

## `primary.jsonl` — JSON Lines

One [`VersionEntry`](#versionentry) per line — compact JSON,
terminated by `\n`. Sorted by `(group, name, version)` with versions
in ascending semver order. Append-friendly + `grep`-able +
`git`-diff-able.

## `by-name/<name>.json` — candidate set for one bare name

One pretty-printed file per distinct bare package name, holding every
`(group, name)` package that publishes under it — the candidate set
that makes short-name resolution one HTTP GET per registry. One
candidate resolves; more than one is a short-name collision the
consumer reports.

```json
{
  "name": "wal",
  "indexed_at": "2026-05-06T12:00:00Z",
  "packages": [
    {
      "group": "org.vibevm",
      "name": "wal",
      "indexed_at": "2026-05-06T10:00:00Z",
      "latest_stable": "0.1.0",
      "versions": [
        { /* a VersionEntry */ }
      ]
    }
  ]
}
```

`packages[]` is sorted by `group`; a length greater than one is a
short-name collision. Each candidate's `versions[]` sorts ascending
and `latest_stable` is its highest non-prerelease. A retired name
answers with a tombstone instead of silence:

```json
{
  "name": "wal-old",
  "indexed_at": "2026-05-06T12:00:00Z",
  "tombstone": {
    "reason": "renamed to `wal`",
    "superseded_by": "wal"
  },
  "packages": []
}
```

`tombstone` is an object with a mandatory `reason` and an optional
`superseded_by` re-address pointer. A name that ever existed always
answers — with the current thing, a re-address pointer, or a tombstone
with a reason — never with silence. `tombstone` is omitted when the
name is alive.

A name this registry has ISSUED as a short spelling carries an `alias`
pointer beside its candidate set (PROP-008 §2.6):

```json
{
  "name": "qwen",
  "indexed_at": "2026-11-11T12:00:00Z",
  "alias": { "group": "org.vibevm.qwen", "name": "qwen-code" },
  "packages": []
}
```

An issued name MAY be the name a package in this registry actually
bears, and since the owner's ruling of 2026-11-12 that is the ordinary
row — it is the only thing that makes a bare `vibe install node` reach
`org.vibevm.node/node` at all. Such a file carries both members:

```json
{
  "name": "node",
  "indexed_at": "2026-11-12T12:00:00Z",
  "alias": { "group": "org.vibevm.node", "name": "node" },
  "packages": [ { "group": "org.vibevm.node", "name": "node", "versions": [ … ] } ]
}
```

`alias` rides **beside** `packages[]` and never inside it, and that
placement is the whole compatibility story: a reader that has never
heard of aliases sees the candidate set it always saw — empty, for a
spelling of a differently-named package; the real package, for an
issued name the registry also publishes — and in neither case is it
misled about what exists. An alias smuggled in as a candidate would
have been read as a real package of that name.

What the two members MEAN to a current reader changed on 2026-11-12:
`alias` is the resolution candidate, and `packages[]` is enumeration.
Before that date `packages[]` outranked `alias`; now a bare spelling
resolves through `alias` alone, and `packages[]` serves the refusal's
hint for a name nobody issued. A vibe of 1.0.12 or older still prefers
`packages[]`, which is why issuing a name equal to the package's own
name is **safely redundant** there rather than a behaviour change: both
readers reach the same `(group, name)`.

`alias` is omitted when the registry has issued no spelling under that
name.

## VersionEntry {#versionentry}

Canonical record per `(group, name, version)`. Schema pinned in
PROP-005 §2.6. `group` is the reverse-FQDN namespace qualifier; with
`name` it forms the package's identity (`kind` stays on the entry as
pure metadata).

```json
{
  "schema_version": 1,
  "kind": "flow",
  "group": "org.vibevm",
  "name": "wal",
  "version": "0.1.0",
  "must_understand": ["wal/epoch-bump"],
  "content_hash": "sha256:8136ecdbc25d…",
  "source_url": "git@gitverse.ru:vibespecs/flow-wal.git",
  "source_ref": "v0.1.0",
  "resolved_commit": "1c3a1355…",
  "registry": "vibespecs",
  "yanked": true,
  "frozen": true,
  "workspace_origin": {
    "upstream": "https://github.com/vibespecs/monorepo",
    "path": "packages/flow-wal",
    "commit": "9f2b7c1…",
    "generated_by": "vibe 0.1.0",
    "generated_at": "2026-05-01T00:00:00Z"
  },
  "license": "EULA",
  "authors": ["Oleg Chirukhin"],
  "description": "Write-Ahead Log discipline …",
  "homepage": "https://vibespecs.example/flow-wal",
  "keywords": ["wal", "memory", "discipline"],
  "describes": "pkg:github/vibespecs/flow-wal@0.1.0",
  "compatibility": {
    "min_vibe_version": "0.1.0",
    "requires_kinds": ["stack"]
  },
  "provides": { "capabilities": ["wal"] },
  "requires": { "packages": ["org.vibevm/core"], "capabilities": ["boot"] },
  "requires_any": [ { "one_of": ["org.vibevm/git", "org.vibevm/svn"] } ],
  "obsoletes": { "packages": ["org.vibevm/wal-legacy"] },
  "conflicts": { "packages": ["org.vibevm/wal-ng"] },
  "features": { "exclusive": { "ui": ["gtk", "tty"] } },
  "subskills": [ { "path": "wal/v01", "delivery": "lazy-pull" } ],
  "i18n": { "available": ["en", "ru"], "default": "en" },
  "boot_snippet": { "source": "boot/10-flow-wal.md", "category": "flow" },
  "files_count": 5,
  "indexed_at": "2026-05-06T12:00:00Z",
  "indexed_by": "vibe-index 1.0.0"
}
```

The example above is a showcase — every slot populated. A real line
omits everything it does not carry: an absent optional field is
**absent from the JSON**, not written as `null`. `homepage`,
`describes`, `resolved_commit`, `license`, `workspace_origin` and
friends carry `#[serde(default, skip_serializing_if =
"Option::is_none")]`; empty subsections (`provides`, `requires`,
`obsoletes`, `conflicts`, `features`, `i18n`, `compatibility`) and
empty lists (`authors`, `keywords`, `requires_any`, `subskills`) are
skipped via their `is_empty` guards. No field of the published index
records — `VersionEntry`, `by-name`, the inverted files — ever
serialises as `null`. The only nullable spots among the index's own
files are `state/checkpoint.json`'s `generated_at` and `head_commit`
(plain `Option`s without skip guards), and that file is gitignored
runtime state, not part of the published index.

The four reader-facing slots added in this cycle follow the same
omission rule, and it cuts the deepest there: `must_understand` (an
empty list), `yanked` (`false`) and `frozen` (`false`) — plus
`tombstone` on a `by-name` entry (absent) — are **omitted when
empty**. A real `primary.jsonl` line for an un-yanked snapshot
package carries none of the four.

- `must_understand` — capabilities a reader **must** understand to act
  on this record. A reader missing one of the listed strings may not
  use that version — but the record is **kept, marked and answered
  about**, never dropped. The loader notes it with a WARN and every
  surface that computes an answer says `unavailable` with what is
  missing and what to do; the raw file still hands the record back word
  for word, `must_understand` included. Unknown fields *outside* this
  list are ignored as before; the list is the explicit exception to
  that rule. The refusal's shape is in
  [`consumer-protocol.md`](consumer-protocol.md#unavailable).
- `yanked` — the version is withdrawn. The fact comes from the
  registry journal, not from the author's manifest: frozen content
  cannot yank itself.
- `frozen` — the version is frozen. Projected from the package
  manifest (`[package].frozen`), not the registry's opinion.
  `snapshot` and `frozen` are the two ends of this one boolean axis:
  absent means `false`, i.e. **snapshot** — content may flow under
  the same version name, and a hash mismatch is news. `true` is the
  author's one-way act — the bytes are immutable, and a hash mismatch
  is an alarm.

## `aliases.toml` — the short names this registry ISSUES {#aliases}

The short names this registry issues for packages it carries (PROP-008
§2.6). Authored by the registry OWNER — whoever builds and publishes
this index — and by nobody else: a package cannot declare its own
alias, because a short spelling is a registry-wide namespace while a
package author owns one `(group, name)` corner of it.

**This table is the whole short-name namespace.** Owner ruling,
2026-11-12: «мы не можем дать людям самим выбирать любое имя» — a
package's own `name` is not a short name either, so a bare `vibe
install <x>` resolves through a row here and through nothing else. A
package this registry carries and has issued no name for is reachable
only as `<group>/<name>`.

```toml
# vibe registry alias table 1 — the short names this registry issues.
schema = 1

[aliases]
node = "org.vibevm.node/node"
qwen = "org.vibevm.qwen/qwen-code"
```

The `node` row is the shape most rows have: the name a package already
bears, issued so that it can be typed bare.

**It is an INPUT, not a projection.** It sits beside the journal among
the things the catalog is computed FROM, so it is not in
`repomd.json`'s `files` map and not in the writer's whitelist — exactly
as `README.md`, `listing.txt` and `state/` are not. It is a file rather
than a journal fact on purpose: a layout registry's journal is committed
and replayed by every client that finds the persisted index stale, and
the replay refuses a record whose `kind` it does not know, so an alias
fact would have made every older vibe fail outright on a registry that
uses one.

Edit it with `vibe-index alias <data-dir> add|remove|list` (a standalone
index) or `vibe registry alias <registry> add|remove|list` (a `monorepo`
or `localdir` registry, where the table and the rebuilt `_index/` land in
one commit). Both validate the **whole** table against this registry's
own index and refuse it entire:

| rule | why |
|---|---|
| the alias obeys the package-name grammar | it is typed where a package name is typed |
| the target is fully qualified and unversioned | supplying the group is the point; version selection stays the operator's |
| the target is a package THIS registry carries | an alias points into the registry that offers it, never across |
| the alias is not an alias of an alias | one spelling, one step, one readable answer, no loops — the test is whether the target's name is another row meaning a DIFFERENT package, so `node = "…/node"` and two rows for one package both stand |
| the alias is not a buried name | a retirement is not reversible by another hand; `superseded_by` is the sanctioned redirect |

The rule this table had until 2026-11-12 and no longer has: *the alias
is not a name this registry publishes*. It protected a precedence
(«a real name outranks every alias») that the owner's ruling removed,
and it forbade exactly the rows the ruling now requires.

One target per alias needs no rule of ours: a TOML table cannot hold a
key twice. A READER that finds a faulty row drops it with a warning and
answers anyway — the file belongs to somebody else, and one stranger's
typo must not take a whole registry down for everyone reading it.

## `state/checkpoint.json` — incremental bookkeeping

```json
{
  "schema_version": 1,
  "generated_at": "2026-05-06T12:00:00Z",
  "repos": {
    "flow-wal": {
      "head_commit": "1c3a1355…",
      "tags": ["v0.1.0"]
    },
    "stack-rust": {
      "head_commit": "abcd…",
      "tags": ["v0.1.0", "v0.2.0"]
    }
  }
}
```

Subsequent `--incremental` runs compare each repo's current state
to the recorded snapshot and skip walking unchanged ones. Unlike the
published files, this record's `Option`s have no skip guards — an
unknown head serialises as `null` here.

## Determinism invariants

- Every multi-entry file is sorted (`primary.jsonl` by
  `(group, name, version)`; `by-name` candidates by `group` and their
  `versions[]` ascending; `by-cap` / `by-purl` lines by
  `(group, name, version)`).
- Atomic-replace via `tmp` + `fsync` + `rename`. `repomd.json` is
  written LAST in any batch update so partial views remain
  consistent against an older manifest until the new one lands.
- `gzip` uses level 6 with `mtime=0` so the byte output is
  reproducible across machines.

These invariants give a clean diff in git when small subsets change
and a stable `repomd.json::files[*].sha256` for the unchanged
files.

## The index inside a layout registry {#layout-registry}

A `monorepo` or `localdir` registry (`<group>/<name>/v<version>/`) keeps
its own catalog at `<registry>/_index/` — the data directory above, with
three differences ([PROP-005 §2.4.1](../../../vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml)):

```
<registry>/
├── org.corp/demo/v1.0.0/            the packages — the tree is the truth
└── _index/
    ├── hello.json  repomd.json  primary.jsonl(.gz)  by-name/  by-cap/  by-purl/
    ├── listing.txt                  what the catalog was computed from
    ├── .gitignore                   /state/* except /state/journal/
    └── state/journal/<YYYY>-<MM>.ndjson   COMMITTED: shared by every publisher
```

- **The journal is committed** (in a monorepo, in the same commit as the
  versions), so yanks, tombstones and provenance are shared.
- **The projection has three inputs** (`vibe_index::layout::project`): the
  tree decides which versions exist and what bytes they hold (a directory
  filled by hand counts); the journal adds identity, provenance (a
  `Published` entry, used only while its `content_hash` matches the tree),
  yanks and tombstones; [`aliases.toml`](#aliases) adds the short
  spellings this registry's owner offers.
- **`listing.txt`** — `# vibe layout-registry listing 1`, `# journal
  sha256:<hex>` (the digest of the journal shards), `# aliases
  sha256:<hex>` when the alias table has rows, then every version path.
  The alias line is conditional so that a registry offering no spelling
  writes the listing it always wrote: an older vibe computes a listing
  without it and would otherwise read every registry as stale. A
  registry that does offer one makes an older vibe fall back to
  computing the projection in memory, which is correct — it simply does
  not see the aliases.
  A reader uses the persisted files only while this matches the tree and
  journal; otherwise it computes the same projection in memory
  (`load_or_project`). `vibe registry test` reports `current`, `stale` or
  `absent`.
