# vibe-index — file format reference

Spec: [`PROP-005 §2.4 / §2.6`](../../../spec/modules/vibe-index/PROP-005-package-index.md#layout).

## On-disk layout

```
<data-dir>/
├── repomd.json                          manifest of every other file
├── primary.jsonl                        one VersionEntry per line, sorted
├── by-name/
│   └── <kind>/
│       └── <name>.json                  cargo-sparse-style per-package file
├── README.md                            auto-generated pointer
└── state/                               gitignored
    ├── server.lock                      PID file when `serve` is running
    ├── admin.tokens                     bearer tokens (0600)
    └── checkpoint.json                  incremental-reindex bookkeeping
```

The `<data-dir>` doubles as the working tree of a `git`-tracked
index repository. Operators commit + push the non-`state/` content
themselves (v0) or via `--auto-commit-push` (v1, deferred).

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
    "by-name":                  { "kind": "directory", "entries": 42 },
    "by-name/flow/wal.json":    { "size": 5120,  "sha256": "sha256:..." }
  }
}
```

`files[*]` is either a `File` entry (`size` + `sha256`) or a
`Directory` entry (`kind: "directory"` + `entries`). Tagged via
serde untagged so a single map carries both shapes.

## `primary.jsonl` — JSON Lines

One [`VersionEntry`](#versionentry) per line, terminated by `\n`.
Sorted by `(kind, name, version)` with versions in ascending semver
order. Append-friendly + `grep`-able + `git`-diff-able.

## `by-name/<kind>/<name>.json` — per-package aggregate

```json
{
  "kind": "flow",
  "name": "wal",
  "indexed_at": "2026-05-06T12:00:00Z",
  "latest_stable": "0.1.0",
  "versions": [
    { /* a VersionEntry */ },
    /* … */
  ]
}
```

## VersionEntry {#versionentry}

Canonical record per `(kind, name, version)`. Schema pinned in
PROP-005 §2.6.

```json
{
  "schema_version": 1,
  "kind": "flow",
  "name": "wal",
  "version": "0.1.0",
  "content_hash": "sha256:8136ecdbc25d…",
  "source_url": "git@gitverse.ru:vibespecs/flow-wal.git",
  "source_ref": "v0.1.0",
  "resolved_commit": "1c3a1355…",
  "registry": "vibespecs",
  "license": "EULA",
  "authors": ["Oleg Chirukhin"],
  "description": "Write-Ahead Log discipline …",
  "homepage": null,
  "keywords": ["wal", "memory", "discipline"],
  "describes": null,
  "compatibility": {
    "min_vibe_version": "0.1.0",
    "requires_kinds": []
  },
  "provides": { "capabilities": [] },
  "requires": { "packages": [], "capabilities": [] },
  "requires_any": [],
  "obsoletes": { "packages": [] },
  "conflicts": { "packages": [] },
  "features": { "default": [], "exclusive": {} },
  "subskills": [],
  "i18n": { "available": ["en"], "default": "en" },
  "boot_snippet": { "source": "boot/10-flow-wal.md", "category": "flow" },
  "files_count": 5,
  "indexed_at": "2026-05-06T12:00:00Z",
  "indexed_by": "vibe-index 0.1.0-dev"
}
```

Empty subsections (`provides`, `requires`, etc.) are omitted on
serialise via `#[serde(skip_serializing_if = "is_empty")]`. `null`
appears for actual operator-omitted optionals (`homepage`,
`describes`, `resolved_commit`).

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
to the recorded snapshot and skip walking unchanged ones.

## Determinism invariants

- Every file with multiple entries is sorted (`primary.jsonl` by
  `(kind, name, version)`; `by-name` versions ascending; `by-cap` /
  `by-purl` by `(kind, name, version)`).
- Atomic-replace via `tmp` + `fsync` + `rename`. `repomd.json` is
  written LAST in any batch update so partial views remain
  consistent against an older manifest until the new one lands.
- `gzip` (when shipped) uses level 6 with `mtime=0` so the byte
  output is reproducible across machines.

These invariants give a clean diff in git when small subsets change
and a stable `repomd.json::files[*].sha256` for the unchanged
files.
