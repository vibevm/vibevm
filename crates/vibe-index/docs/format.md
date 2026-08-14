# vibe-index — file format reference

Spec: [`PROP-005 §2.4 / §2.6`](../../../spec/modules/vibe-index/PROP-005-package-index.md#layout).

## On-disk layout

```
<data-dir>/
├── repomd.json                          manifest of every other file
├── primary.jsonl                        one VersionEntry per line, sorted
├── primary.jsonl.gz                     deterministic gzip sibling
├── by-name/
│   └── <name>.json                      candidate set for one bare name
├── by-cap/<slug>.jsonl                  inverted index by capability
├── by-purl/<slug>.jsonl                 inverted index by `describes` PURL
├── README.md                            auto-generated pointer
└── state/                               gitignored
    ├── server.lock                      PID file when `serve` is running
    ├── admin.tokens                     bearer tokens — keep 0600; nothing enforces it
    └── checkpoint.json                  incremental-reindex bookkeeping
```

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
  "generator": "vibe-index 0.1.0-dev",
  "package_count": 42,
  "version_count": 117,
  "files": {
    "primary.jsonl":            { "size": 184522, "sha256": "sha256:..." },
    "primary.jsonl.gz":         { "size": 42110,  "sha256": "sha256:..." },
    "by-name":                  { "kind": "directory", "entries": 42 },
    "by-name/wal.json":         { "size": 5120,  "sha256": "sha256:..." },
    "by-cap":                   { "kind": "directory", "entries": 7 },
    "by-cap/wal.jsonl":         { "size": 890,   "sha256": "sha256:..." },
    "by-purl":                  { "kind": "directory", "entries": 3 }
  }
}
```

`files[*]` is either a `File` entry (`size` + `sha256`) or a
`Directory` entry (`kind: "directory"` + `entries`). Tagged via
serde untagged so a single map carries both shapes. `primary.jsonl.gz`
is byte-deterministic (level 6, `mtime=0`), so its `sha256` is stable
across machines for identical input.

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
  "indexed_by": "vibe-index 0.1.0-dev"
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
serialises as `null`. The only nullable spots in the whole tree are
`state/checkpoint.json`'s `generated_at` and `head_commit` (plain
`Option`s without skip guards), and that file is gitignored runtime
state, not part of the published index.

The four reader-facing slots added in this cycle follow the same
omission rule, and it cuts the deepest there: `must_understand` (an
empty list), `yanked` (`false`) and `frozen` (`false`) — plus
`tombstone` on a `by-name` entry (absent) — are **omitted when
empty**. A real `primary.jsonl` line for an un-yanked snapshot
package carries none of the four.

- `must_understand` — capabilities a reader **must** understand to act
  on this record. A reader that does not know at least one of the
  listed strings skips the record (today — with a warning). Unknown
  fields *outside* this list are ignored as before; the list is the
  explicit exception to that rule.
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
