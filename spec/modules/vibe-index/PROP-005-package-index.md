# PROP-005: Optional package index — per-org metadata + standalone index server {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: implemented; folded into the workspace 2026-05-22"/>

##milestone-line **Milestone:** retrofits into M2.10 (`vibe search`) and M1.10 (`vibe outdated`) from `ROADMAP.md`. Slices land independently; index is opt-in everywhere. @impl/done

##status-line **Status:** implemented; folded into the workspace 2026-05-22. Slices 1–8 (the `vibe-index` server + CLI) and slices 9–10 (publisher hook + consumer fast path) are shipped, plus M2.10 `vibe search`. `vibe-index` lives at `crates/vibe-index/` as a workspace member ([§6](#distribution)) and parses through `vibe-core::Manifest`. See [§9](#open) item 11 for the de-rot and fold that got it there. @impl/done

##related **Related:** [PROP-001](../vibe-registry/PROP-001-git-backend.md) (git backend), [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md) (`[[registry]]` / `[[mirror]]` / `[[override]]` / content-hashed identity), [PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) (features / subskills / `describes` / conditional deps), [PROP-004](../../../legacy-spec/research/PROP-004-tessl-comparative-research.md) §5.x (gap analysis), [`spec://org.vibevm.core/vibevm/common/PROP-000`](../../common/PROP-000.md) (especially §15 dep weight, §16 JTD, §17 production architecture, §18 complexity ≥ RPM, §20 token secrecy). @spec/done

##research-summary **Out-of-band research summary** (2026-05-06, prior session). Comparative inventory of indexing strategies in production package managers — Maven Central (Lucene + directory layout), npm (CouchDB replicated DB), PyPI (PEP 503/691 simple API), RPM/DNF (`repodata/primary.xml.gz` + libsolv), Deb/APT (`Packages.gz` RFC822), Cargo (git index → sparse HTTP), Go modules (proxy + Merkle sumdb), Nix flakes (per-flake `flake.lock`, no global index), Homebrew (mono-repo formula), OCI registries (`/v2/_catalog`). Three candidate paths surfaced for vibevm: **(a)** Cargo-sparse-style per-package JSON files in an org-level index; **(b)** DNF-style single repodata directory with full SAT-ready dep graph; **(c)** Nix-flake-style indexless live-resolve (current state) with optional `flake-registry`-shape short-name mapping. PROP-005 picks (a) augmented by (b)'s integrity-manifest pattern (`repomd.json`). @spec/done

---

## 1. Motivation {#motivation}

##live-resolve-lead vibevm today resolves packages **live** against the host's git API: @impl/done

- ##live-install `vibe install flow:wal` translates to `git ls-remote <org-url>/flow-wal.git` to enumerate tags, then `git archive` (or `git fetch && git checkout`) to read the manifest at the candidate ref. One pkgref = at least one round-trip per registry walked, two if the package is not in the first registry. @impl/done
- ##live-outdated `vibe outdated` (M1.10) calls `MultiRegistryResolver::resolve(<pkgref>@Latest)` per locked package — one round-trip each. @impl/done
- ##live-search `vibe search` (M2.10, not yet shipped) cannot work at all without enumerating an org. GitHub's `GET /orgs/<org>/repos` is rate-limited to 60 req/h unauth or 5000 req/h with a token; GitVerse exposes no org-scoped repo listing in its public API. @impl/done

##scale-limit This works at the M0 / M1 demonstration scale (3 packages in `vibespecs`). It does not work at v1 shipping scale (target: hundreds of packages per org, multiple orgs configured per project). @impl/done

##failure-modes-lead **Failure modes the live-resolve path produces in practice:** @impl/done

1. ##FAIL-COLD-LATENCY **Cold-cache install latency grows linearly with the dep graph.** A project with 20 transitive deps spread across two registries spends 30–60 s in `git ls-remote` alone before any actual content fetch. @impl/done
2. ##FAIL-RATE-LIMIT **Rate-limit visibility for `vibe outdated`.** Polling N packages at refresh time burns N requests; against an unauthenticated GitHub registry, this exhausts the quota at 60 packages. @impl/done
3. ##FAIL-SEARCH **`vibe search` is impossible.** Even with an authenticated org-listing endpoint, parsing every repo's `vibe.toml` at every search would be intractable. @impl/done
4. ##FAIL-DISCOVERY **Discovery story is silent.** A consumer with a fresh checkout of an unknown vibevm org has no way to enumerate "what packages live here?" without scraping the host UI. @impl/done
5. ##FAIL-OFFLINE **Mirror-driven offline workflows degrade silently.** [PROP-002 §2.3](../vibe-registry/PROP-002-decentralized-registry.md#mirror) makes mirror dispatch invisible to the lockfile, but `ls-remote` against a mirror still leaks live host calls when the resolver wants to know "what versions exist?" — there's no offline catalog. @impl/done

##INDEX-NEEDED What every other production package manager does — and what vibevm now needs — is an **index**: a small set of files, regenerable from authoritative package state, that lets consumers perform `list`, `search`, `outdated`, and `resolve-version-shortlist` operations against cached / mirror-able metadata instead of live git. RPM (`primary.xml.gz`), Cargo (sparse index), Deb (`Packages.gz`), npm (CouchDB document per package) all converge on the same shape: **derived metadata files alongside or near the artefact storage, regenerated by a tool, served as plain HTTP**. @impl/done

---

## 2. Decisions {#decisions}

### 2.1 Index is OPTIONAL — zero impact when absent {#optional}

##req-optional `req r1` @impl/done

##INDEX-OPTIONAL **Decision.** The index layer is **strictly additive**. Every existing vibevm code path keeps working exactly as today when no index is present. Consumers detect a registry's index by HTTP GET on a well-known path (`<index-base>/repomd.json`); 404 → fall back to the live `git ls-remote` path that exists today. No registry is required to have an index. No project is required to consume one. @impl/done

##optional-rationale-lead **Rationale.** @spec/done

- ##rationale-backward-compat Backward compat for existing `vibespecs` (GitHub) + `vibespecs-gitverse` (GitVerse) registries — they keep working unchanged. Index is something the org owner opts into. @spec/done
- ##rationale-decoupled Decouples the index design from Phase A of vibevm — operators with three packages do not need an index; operators with three hundred do. @spec/done
- ##rationale-no-central Removes the "central server is now load-bearing" failure mode: if the index disappears, the live path is still there. @spec/done

- ##OPPORTUNISTIC **Consequence.** All optimisations the index unlocks (cold-cache install speed, `vibe search`, faster `vibe outdated`) are **opportunistic**. @impl/done
- ##INTEGRITY-UNCHANGED The integrity story (`content_hash` per [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity)) does not change — content_hash is verified at fetch time regardless of whether the resolve path went through the index. @impl/done

### 2.2 Form factor: per-org **index** living in a separate git repository {#form-factor}

##req-form-factor `req r1` @impl/done

##INDEX-REPO **Decision.** Each vibevm registry org that opts in maintains a dedicated git repository named `index` (configurable; default name `index`) under the same org root: @impl/done

- ##index-url-github `https://github.com/vibespecs/index` @impl/done
- ##index-url-gitverse `git@gitverse.ru:vibespecs/index.git` @impl/done

##index-repo-properties-lead Inside this repository sits a fixed file layout (§2.4) holding the org's catalog. The repository is: @impl/done

- ##REPO-CLONEABLE **Cloneable like any other** — same auth model as the package repos (HTTPS public read; SSH or token push for the maintainer). @impl/done
- ##REPO-HTTP-FETCHABLE **HTTP-fetchable** at raw URLs without cloning — `https://raw.githubusercontent.com/vibespecs/index/main/repomd.json` (GitHub) / `https://gitverse.ru/api/v1/repos/vibespecs/index/raw/main/repomd.json` (GitVerse). Consumers default to raw HTTP (one GET, no clone); falling back to git clone only when the host's raw-HTTP shape is unknown. @impl/done
- ##REPO-MIRRORABLE **Mirror-able trivially** — the `[[mirror]]` machinery from [PROP-002 §2.3](../vibe-registry/PROP-002-decentralized-registry.md#mirror) applies unchanged: a mirror at `https://mirror.internal/vibespecs/index` is a drop-in. @impl/done

##why-dedicated-lead **Why a dedicated repo, not files-in-package-repos.** @spec/done

- ##why-discovery **Discovery.** A single `<org>/index` repo answers "what's in this org?" with one HTTP GET. Per-package metadata files would still require enumerating the org first — chicken-and-egg. @spec/done
- ##why-atomicity **Atomicity of catalog state.** A single index repo can be replaced as a whole, signed as a whole, mirrored as a whole. Per-package files leave catalog consistency to the consumer to reconstruct. @spec/done
- ##why-decoupling **Decoupling.** Index regeneration does not touch package repos. Authors do not need to run any utility. The org owner runs `vibe-index` on a cadence; package repos stay pristine. @spec/done
- ##why-mirror-parity **Mirror parity with [PROP-002 §2.3](../vibe-registry/PROP-002-decentralized-registry.md#mirror).** The same mirror chain that covers package repos covers the index. Operators who already understand `[[mirror]]` get the index covered for free. @spec/done

##why-not-hosted-lead **Why not a hosted central HTTP service** (npm-style `registry.npmjs.org`): @spec/done

- ##not-hosted-infra Requires running infra. vibevm's deliberate posture per [PROP-000 §17](../../common/PROP-000.md#production-architecture) is "every org self-hosts on hosting they already use" — git platforms are that hosting. @spec/done
- ##not-hosted-single-vendor Single-vendor. We rejected this shape in [PROP-002 §1](../vibe-registry/PROP-002-decentralized-registry.md#motivation) (the "Nix's failure pattern"). Centralising the index is the same anti-pattern at one layer up. @spec/done
- ##not-hosted-available HTTP service is **available** (§2.5) — the `vibe-index serve` mode lets an operator run one — but it is not the default consumption path. Most consumers go through static raw-HTTP files in the index git repo. @spec/done

##INDEX-URL-CONFIG **Configurable but defaulted.** A `[[registry]]` block can pin a custom index location: @impl/done

```toml
[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"
naming = "kind-name"
index_url = "https://github.com/vibespecs/index"  # default; explicit override allowed
# or, to point at a hosted server:
# index_url = "https://index.vibespecs.dev"
# or, to disable index lookup entirely:
# index_url = "none"
```

##INDEX-URL-DEFAULT Default: `<registry-url>/index`. Resolver tries the default location when `index_url` is unset; 404 / connect-failure on the index → silent fallback to live ls-remote (no error message; the operator did not promise an index). @impl/done

### 2.3 Source of truth: package repos remain authoritative; index is a hot cache {#truth}

##req-truth `req r1` @impl/done

##REPOS-AUTHORITATIVE **Decision.** Package repositories are the **source of truth** for content (manifests, files, tags). The index is a **derived hot cache**, regeneratable from the authoritative package state. @impl/done

- ##REALITY-WINS This matters because it disambiguates the failure mode: if the index disagrees with reality, **reality wins**. @impl/done
- ##HASH-VERIFIED-ANYWAY A consumer that resolves a package through the index still verifies `content_hash` against the actually-fetched bytes per [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity). A mismatch between an index-recorded `content_hash` and the actually-fetched one is a hard `IntegrityError`, surfaced with both values and a hint to refresh the index. No silent acceptance. @impl/done

##SERVER-MODE-WRINKLE **Server-mode wrinkle.** The `vibe-index serve` mode (§2.5) is described in the prompt as "the only writer; source of truth". This is true *for the index data*. The server holds the canonical RAM copy, persists it to disk on every mutation, and refuses writes from other processes (file lock + mtime checks at startup). But the server's writes are **derived from package-repo state** — the ground truth for "is this version published?" is still the actual git tag in the actual package repo. A divergence (index says `v0.1.0` exists; the package repo's tag was force-deleted) surfaces at consumer install time as an integrity failure on cross-source `content_hash` verification, and the operator has the diagnostic clear: "your index lies; reindex". @impl/done

### 2.4 File layout inside the index repo {#layout}

##req-layout `req r1` @impl/done

##INDEX-FILE-LAYOUT **Decision.** The index repo's working tree (and equivalently its raw-HTTP-served file tree) carries: @impl/done

```
<index-root>/
├── repomd.json                                # manifest — hashes & metadata for all other files
├── primary.jsonl                              # one line per (group, name, version)
├── primary.jsonl.gz                           # gzipped variant
├── by-name/
│   └── <name>.json                            # candidate set — every (group, name) sharing one bare name
├── by-cap/
│   └── <capability-slug>.jsonl                # provides-index — pkgrefs that advertise this capability
├── by-purl/
│   └── <purl-slug>.jsonl                      # describes-index — pkgrefs that describe this PURL
└── README.md                                  # human-readable "what is this directory" pointer
```

##REPOMD **`repomd.json`** — the manifest, modelled after RPM's `repomd.xml`: @impl/done

```json
{
  "schema_version": 1,
  "registry": "vibespecs",
  "registry_url": "https://github.com/vibespecs",
  "naming": "fqdn",
  "generated_at": "2026-05-06T12:00:00Z",
  "generator": "vibe-index 0.1.0",
  "package_count": 42,
  "version_count": 117,
  "files": {
    "primary.jsonl": {
      "size": 184522,
      "sha256": "<hex>"
    },
    "primary.jsonl.gz": {
      "size": 38421,
      "sha256": "<hex>"
    },
    "by-name": {
      "kind": "directory",
      "entries": 42
    },
    "by-cap": {
      "kind": "directory",
      "entries": 18
    },
    "by-purl": {
      "kind": "directory",
      "entries": 5
    }
  }
}
```

- ##REPOMD-TRUST-POINT `repomd.json` is the **single point of trust**. A consumer fetches it once (with a small ETag round-trip later), then fetches whichever sub-files it actually needs, verifying each against the recorded `sha256`. @impl/done
- ##repomd-pattern-heritage This pattern (manifest-with-checksums) is what RPM, Deb, and OCI all share, and is what gives us a path to GPG signing without re-architecting. @spec/done

##PRIMARY-JSONL **`primary.jsonl`** — newline-delimited JSON, one record per `(group, name, version)`. Lines are sorted by `(group, name, version)` — the PROP-008 §2.2 identity ordering. Each line is one §2.6 entry. JSONL is chosen over JSON-array because: @impl/done

- ##jsonl-append Append-friendly (a publish hook can append and re-sort + rewrite, or for incremental update merge a sorted insert). @spec/done
- ##jsonl-streamable Streamable by consumers (line-at-a-time parse; no need to buffer the whole file). @spec/done
- ##jsonl-grepable `grep`-able (operators can inspect the index manually). @spec/done
- ##jsonl-diffable Diffable in git (per-line diffs survive re-sorts cleanly). @spec/done

##PRIMARY-GZ **`primary.jsonl.gz`** — gzip-compressed equivalent for HTTP-bandwidth-conscious consumers. ~5× compression typical for JSON. Byte-identical content; gzip is deterministic at level 6 with the standard zlib dictionary so the file is reproducible across machines (we pin level and disable `mtime` in the gzip header to keep the SHA-256 stable). Both `primary.jsonl` and `primary.jsonl.gz` ship; consumer chooses based on `Accept-Encoding`. @impl/done

##BY-NAME **`by-name/<name>.json`** — the candidate-set file for one bare package
name (PROP-008 §2.8). A single HTTP GET fetches every `(group, name)`
package that shares the short name `<name>`, each with all its versions —
~1–10 KB. This is the path short-name resolution (PROP-008 §2.6) walks:
one GET per registry yields the whole candidate set, so a collision
(PROP-008 §2.7) is detected at once. The directory level keyed on `kind`
before PROP-008; `kind` left package identity, so `<name>` alone is the key. @impl/done

```json
{
  "name": "wal",
  "indexed_at": "2026-05-06T12:00:00Z",
  "packages": [
    {
      "group": "org.vibevm",
      "name": "wal",
      "indexed_at": "2026-05-06T12:00:00Z",
      "latest_stable": "0.1.0",
      "versions": [
        { /* one §2.6 entry */ },
        { /* … */ }
      ]
    }
  ]
}
```

##BY-CAP **`by-cap/<capability-slug>.jsonl`** — for `vibe install --capability ui:landing-page` style queries (PROP-003 capability-driven resolution). Each line: `{"kind":"feat","name":"welcome-page","version":"0.3.0","capability":"ui:landing-page@0.3.0"}`. `<capability-slug>` is the capability string with `:` and `/` and `@` replaced by `--` (filesystem-safe; reversible). Optional in v0; populated when present. @impl/done

##BY-PURL **`by-purl/<purl-slug>.jsonl`** — for "what vibevm packages document this upstream library?" queries (PROP-003 §2.5.6 `describes`). Same shape as `by-cap`; `<purl-slug>` is the PURL with `/` replaced by `--`. @impl/done

##SORT-INVARIANTS **Sort invariants.** Every file with multiple entries is sorted deterministically: @impl/done

- ##sort-primary `primary.jsonl` — sort key `(group, name, version)` with versions in ascending semver order. @impl/done
- ##sort-by-name `by-name/<name>.json` — `packages` sorted by `group`; each package's `versions` array sorted ascending. @impl/done
- ##sort-by-cap-purl `by-cap/<slug>.jsonl` and `by-purl/<slug>.jsonl` — sort key `(group, name, version)`. @impl/done

##DETERMINISM-WHY Determinism matters because the index repo lives in git: a non-deterministic order would produce churn diffs on every regenerate, defeating the value of git as the transport. @spec/done

### 2.5 Two modes: CLI tool and HTTP server {#modes}

##req-modes `req r1` @impl/done

##ONE-BINARY-TWO-MODES **Decision.** A single binary, `vibe-index`, ships in two modes selected by subcommand: @impl/done

- ##MODE-CLI **CLI mode** (default, every subcommand except `serve`) — operates directly on a data directory of index files. Reads on-disk state, mutates, writes back atomically. Suited for: scripted `vibe-index reindex` invocations, manual operator commands, CI pipelines, post-publish hooks. @impl/done
- ##MODE-SERVER **Server mode** (`vibe-index serve`) — boots an HTTP server. Holds the index in RAM; persists every mutation back to disk. Single-writer (the server) — no other process should mutate the data dir while the server runs (file lock at `<data-dir>/state/server.lock`; broken lock → server refuses to start with a clear error). Suited for: hosted index endpoints, real-time publish-time updates from `vibe registry publish`. @impl/done

##one-binary-why **Why one binary, not two.** Same code paths (the in-memory `Index` struct, the persistence layer, the scanner) are shared. Two binaries would force consumers to install both. clap-style subcommand dispatch handles the mode selection. @spec/done

##distribution-pointer **Distribution.** The utility lives in `crates/vibe-index/` at the repository root — outside the main Cargo workspace under `crates/`. This is deliberate: §6 explains the separate-workspace decision and what it buys for redistribution. @spec/work

### 2.6 Index entry shape (the canonical record) {#entry}

##req-entry `req r1` @impl/done

##ENTRY-SCHEMA **Decision.** Every `(group, name, version)` entry carries the following fields. This is the schema lines of `primary.jsonl` follow, and the elements each `by-name/<name>.json` candidate's `versions[]` carry. **This section IS the schema** — measured 2026-08-05: there is no JTD file for the index entry anywhere in the tree, and `crates/vibe-index/schemas/` does not exist. The types are hand-written against this text (`crates/vibe-index/src/types/entry/`, whose own docblock says «Schema pinned in PROP-005 §2.6»), which is a different arrangement from the seven wire reports under the root `schemas/`, where the JTD file is the authority and the Rust is generated and gated by `cargo xtask check-codegen`. @impl/done

```json
{
  "schema_version": 1,
  "kind": "flow",
  "group": "org.vibevm",
  "name": "wal",
  "version": "0.1.0",
  "content_hash": "sha256:8136ecdbc25d4555cbab6e9574f153b252a05c62b55b5e0255def645458c9544",
  "source_url": "git@gitverse.ru:vibespecs/flow-wal.git",
  "source_ref": "v0.1.0",
  "resolved_commit": "1c3a1355abcdef0123456789abcdef0123456789",
  "registry": "vibespecs",
  "workspace_origin": null,
  "license": "EULA",
  "authors": ["Oleg Chirukhin"],
  "description": "Write-Ahead Log discipline for human-AI development sessions",
  "homepage": null,
  "keywords": ["wal", "memory", "discipline", "session-management"],
  "describes": null,
  "compatibility": {
    "min_vibe_version": "0.1.0",
    "requires_kinds": []
  },
  "provides": {
    "capabilities": []
  },
  "requires": {
    "packages": [],
    "capabilities": []
  },
  "requires_any": [],
  "obsoletes": { "packages": [] },
  "conflicts": { "packages": [] },
  "features": {
    "default": [],
    "exclusive": {}
  },
  "subskills": [],
  "i18n": {
    "available": ["en"],
    "default": "en"
  },
  "boot_snippet": {
    "source": "boot/10-flow-wal.md",
    "category": "flow"
  },
  "files_count": 5,
  "indexed_at": "2026-05-06T12:00:00Z",
  "indexed_by": "vibe-index 0.1.0"
}
```

##field-provenance-lead **Field provenance.** @impl/done

- ##PROV-MANIFEST-FIELDS `kind` / `group` / `name` / `version` / `license` / `authors` / `description` / `homepage` / `keywords` / `describes` / `compatibility` / `provides` / `requires` / `requires_any` / `obsoletes` / `conflicts` / `features` / `i18n` / `boot_snippet.source` / `boot_snippet.category` — read directly from `vibe.toml` at the tagged ref. (M1.18's loading model, PROP-009, retired the author-chosen `boot_snippet.filename`; a snippet is now its `source` path plus an ordering `category`.) @impl/done
- ##PROV-GROUP `group` — the mandatory reverse-FQDN qualifier from `[package].group` (PROP-008 §2.1). With `name` it forms the package identity; `kind` is metadata and identifies nothing (PROP-008 §2.2 / §2.3). @impl/done
- ##PROV-WORKSPACE-ORIGIN `workspace_origin` — the `[origin]` provenance marker (PROP-007 §2.8, PROP-008 §2.8), present only on a copy `vibe workspace publish` generated from a workspace member; absent (`null`) for a standalone publish. @impl/done
- ##PROV-SUBSKILLS `subskills` — collected by walking `<package-root>/subskills/<path>/vibe-subskill.toml` at the tagged ref; each entry: `{path, delivery, describes, description, channels}`. Same fields the lockfile records. @impl/done
- ##PROV-CONTENT-HASH `content_hash` — computed by the same algorithm `vibe-registry::compute_content_hash` uses (sha256 over deterministically-ordered file bytes). Index uses **the same hash** as the lockfile, so cross-checks are byte-equal. @impl/done
- ##PROV-SOURCE-URL `source_url` — the canonical org URL (§2.4 of [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md#registry-model)) composed with the package repo name. Mirror URLs do not appear here (same invariant as the lockfile). @impl/done
- ##PROV-SOURCE-REF `source_ref` — `v<version>` by default (the tag). @impl/done
- ##PROV-RESOLVED-COMMIT `resolved_commit` — the commit SHA the tag pointed to at index time. Pinning to commit lets us notice tag-rewrites later. @impl/done
- ##PROV-REGISTRY `registry` — local alias from `[[registry]].name`. @impl/done
- ##PROV-FILES-COUNT `files_count` — informational, useful for sanity-checking integrity diffs. @impl/done
- ##PROV-INDEXED-AT `indexed_at` / `indexed_by` — provenance for the index entry itself (when, by which tool version). @impl/done

##FORWARD-COMPAT **Forward compatibility.** `schema_version: 1` is recorded at file scope (in `repomd.json`) and at entry scope. v2 entries with new fields can coexist in v1 files via `serde(default)` on consumers; readers of v2 written by an old vibevm gracefully ignore unknown fields. @impl/done

### 2.7 Identity and trust {#trust}

##req-trust `req r1` @impl/done

##HASH-JOIN-KEY **Decision.** `content_hash` is the join key between the index and the lockfile. A consumer that fetches `flow:wal@0.1.0` via the index records the same `content_hash` in the lockfile that a no-index fetch would have produced. **Index entries are advisory; the bytes are authoritative.** @impl/done

##TWO-INTEGRITY-LAYERS The `repomd.json::files[*].sha256` covers integrity of the index files themselves. The per-entry `content_hash` covers integrity of the package content. The two are independent: a tampered index file fails its file-hash check; a tampered package repo (force-pushed tag) fails its content-hash check at fetch time. @impl/done

##trust-oos **Out of scope for v0:** GPG-signed `repomd.json.asc`, Merkle-log audit trail (Go sumdb-style). [§9](#open) tracks both. @spec/done

### 2.8 Reindexation: full and incremental {#reindex}

##req-reindex `req r1` @impl/done

##TWO-REINDEX-MODES **Decision.** Two regeneration modes, both available via CLI (`vibe-index reindex`) and HTTP (`POST /v1/admin/reindex`): @impl/done

##FULL-REINDEX **Full reindex.** Walk every package repo in the org; for each repo, list tags; for each `v<semver>` tag, read `vibe.toml` and `subskills/**/vibe-subskill.toml` at that ref; compute `content_hash`; assemble §2.6 entry. Replace the in-memory index wholesale, then atomic-write the on-disk files. @impl/done

##walk-sources-lead Sources for the walk: @impl/done

- ##SRC-FROM-CLONES `--from-clones <org-dir>` — local directory of bare/regular clones. Authoritative for the operator who already maintains a vendor mirror; offline-capable. Default path for owners who run `vibe-index reindex` on a cron against their own server's clone tree. @impl/done
- ##SRC-FROM-GITHUB `--from-github <org>` — REST API walk against `api.github.com`. Requires a token (read-only `repo`-scope). Used by hosted index instances that don't keep a clone tree. @impl/done
- ##SRC-FROM-GITVERSE `--from-gitverse <org>` — equivalent against GitVerse's API once it exposes org-scoped repo enumeration; today returns "not implemented" (mirrors the publish-stub pattern from `vibe-publish/src/gitverse.rs`). @impl/done

##INCREMENTAL-REINDEX **Incremental reindex.** Detect what changed since the last run and update only the affected entries. @impl/done

- ##INC-CLONES For `--from-clones`: compare each repo's `git rev-parse HEAD` and `git tag -l` output to a checkpoint stored at `<data-dir>/state/checkpoint.json`. Repos with new tags or a new HEAD commit on `main` (in case a manifest changed without a tag) are re-walked; others skip. @impl/done
- ##INC-GITHUB For `--from-github`: use the `If-Modified-Since` / ETag headers on `/orgs/<org>/repos` and `/repos/<org>/<name>/tags` to skip unchanged repos. @impl/done

##CADENCE-TARGET Incremental is the default cadence target (one run per minute on an active org); full is the bootstrap path and the "trust nothing" recovery option. @impl/done

##triggers-lead **Triggers.** @impl/done

- ##TRIGGER-CLI **CLI:** `vibe-index reindex <data-dir> --from-clones <org-dir>` — direct invocation. @impl/done
- ##TRIGGER-HTTP **HTTP:** `POST /v1/admin/reindex` body `{"mode":"full"|"incremental","source":"clones"|"github","args":{...}}`. Auth required. Returns a job id; status pollable at `GET /v1/admin/reindex/<job-id>` (in v1 — v0 just blocks until done). @impl/done
- ##TRIGGER-GIT-HOOK **git hook (server-side, on the index repo's host):** owner installs a `post-receive` hook on the org's hosted git that posts to `POST /v1/admin/reindex` whenever a package repo gets a push to a `v*` tag. Documented in §11; not shipped as part of the binary. @spec/done
- ##TRIGGER-CRON **cron:** `crontab` line invokes `vibe-index reindex --incremental` every N minutes. Documented; not enforced. @spec/done

### 2.9 Single-writer server mode {#server-mode}

##req-server-mode `req r1` @impl/done

##SINGLE-WRITER **Decision.** The HTTP server is the **only writer** when running. It locks the data directory via a PID file (`<data-dir>/state/server.lock`) at startup; refuses to start if the lock is held by another live process; refuses CLI mutations against the same data directory by detecting the lock from CLI side (CLI-mode `add` / `remove` / `reindex` errors with "server is running on this data dir; use the HTTP API"). @impl/done

##state-model-lead In-memory state model: @impl/done

```text
Arc<RwLock<Index>>
   │
   ├─ readers (search, list, get)        — RwLock::read()
   └─ writers (add, remove, reindex)     — RwLock::write()
```

##write-protocol-lead On every successful write, the server: @impl/done

1. ##WRITE-MEMORY Updates the in-memory `Index`. @impl/done
2. ##WRITE-RESERIALISE Re-serialises the affected files (`primary.jsonl`, the touched `by-name/<kind>/<name>.json`, optionally `by-cap` / `by-purl`). @impl/done
3. ##WRITE-ATOMIC Writes each file atomically: `tmp` next to the destination, `fsync`, `rename`. @impl/done
4. ##WRITE-REPOMD-LAST Updates `repomd.json` last (the manifest is replaced as a whole; readers that hold the old `repomd.json` see a consistent old view; readers that pick up the new `repomd.json` see consistent new files). @impl/done
5. ##WRITE-AUTO-COMMIT Optionally (if `--auto-commit-push` flag is on): `git add -A && git commit -m "auto: index update" && git push origin <branch>` against the data directory if it is a git working tree. v0 ships without this — operator runs commit/push manually or via separate cron. v1 adds `--auto-commit-push`. @spec/done

##CONCURRENCY **Concurrency.** axum + tokio. Reads do not block reads. Writes block reads (RwLock) for the duration of the in-memory mutation; disk I/O happens after lock release for any path it can (e.g. `primary.jsonl` rewrites are queued and serialised by a single dedicated writer task). For the request rates we target (max ~10 writes/min during a publish burst, ~1000 reads/min during a CI install storm), a coarse RwLock is sufficient. @impl/done

##PROCESS-MODEL **Process model.** Single process. No replication. An operator who needs HA runs the server behind a load balancer with N replicas and a shared filesystem — but that's outside v0. v0 expects one process per data directory. @impl/done

### 2.10 HTTP API surface {#http}

##req-http `req r1` @impl/done

##HTTP-API **Decision.** REST API, JSON over HTTP. CORS open on read endpoints (so a future web UI can hit it from a browser). Routes: @impl/done

```
GET    /healthz                                   # liveness
GET    /readyz                                    # readiness (index loaded, no in-flight reindex)

# Static index files (raw — same shape as the on-disk files; mirror-friendly)
GET    /v1/index/repomd.json
GET    /v1/index/primary.jsonl
GET    /v1/index/primary.jsonl.gz
GET    /v1/index/by-name/{name}.json
GET    /v1/index/by-cap/{slug}.jsonl
GET    /v1/index/by-purl/{slug}.jsonl

# Structured query (richer than the raw files)
GET    /v1/packages                               # ?kind=&q=&limit=&offset=
GET    /v1/packages/{group}/{name}                # all versions of one package
GET    /v1/packages/{group}/{name}/{version}      # one specific version (entry)
GET    /v1/capabilities/{cap}                     # who provides this capability
GET    /v1/purls/{purl}                           # who describes this upstream

# Mutations (auth required)
POST   /v1/packages                               # body: full §2.6 entry — insert/upsert
DELETE /v1/packages/{group}/{name}/{version}      # remove one version
DELETE /v1/packages/{group}/{name}                # remove all versions of a package

# Admin (auth required)
POST   /v1/admin/reindex                          # body: { mode, source, args }
GET    /v1/admin/status                           # uptime, last reindex, pkg count, server version

# Observability
GET    /metrics                                   # Prometheus text format
```

##HTTP-AUTH **Authentication.** Bearer tokens via `Authorization: Bearer <token>`. Tokens are read from `<data-dir>/state/admin.tokens` (one token per line; comment lines start with `#`). Read endpoints accept missing/invalid tokens silently. Write endpoints require a valid token; mismatch → 401 with a generic message ("authentication required"; do not echo the supplied token nor say which valid prefix it matched). Tokens never appear in logs (logging redacts the `Authorization` header). @impl/done

##HTTP-LOCKDOWN **Per-host lockdown.** By default the server binds to `127.0.0.1:8412` — local-only. Operators expose externally by setting `--bind 0.0.0.0:8412` and putting it behind a reverse proxy with TLS. v0 does not ship TLS termination; this is the reverse proxy's job. (Same posture as `cargo`'s sparse index protocol: the upstream is HTTP — TLS is for the CDN / proxy in front.) @impl/done

##HTTP-ERRORS **Errors.** Application/json error shape, taken from RFC 7807 Problem Details (lightweight subset): @impl/done

```json
{ "type": "vibe-index/error/integrity-mismatch", "title": "content_hash mismatch", "status": 409, "detail": "…", "instance": "/v1/packages/flow/wal/0.1.0" }
```

### 2.11 CLI surface {#cli}

##req-cli `req r1` @impl/done

##CLI-SURFACE **Decision.** `vibe-index <subcommand> [args]`. All subcommands accept `--data-dir <path>` (or use `$VIBE_INDEX_DATA_DIR`, default `./vibe-index-data`). All emit human-readable text by default; `--json` for machine-readable shape mirroring the HTTP API responses. @impl/done

```
# Lifecycle
vibe-index init <data-dir> [--registry NAME --registry-url URL --naming fqdn|kind-name|name|kind/name]
vibe-index dump <data-dir> [--format jsonl|json|toml]
vibe-index verify <data-dir>                         # recompute file hashes, check repomd

# Reindex
vibe-index reindex <data-dir> --from-clones <org-dir>           [--full | --incremental]
vibe-index reindex <data-dir> --from-github <org> [--token-file FILE]  [--full | --incremental]
vibe-index reindex <data-dir> --from-gitverse <org>             # emits stub-not-implemented today

# Read
vibe-index get <data-dir> <group> <name> [--version V]
vibe-index list <data-dir> [--kind K] [--limit N] [--offset M]
vibe-index search <data-dir> <query> [--kind K] [--limit N]
vibe-index capabilities <data-dir> <capability>
vibe-index purls <data-dir> <purl>
vibe-index outdated <data-dir> [--lockfile PATH]                # given a vibe.lock, print upgrade candidates

# Write (CLI-mode; refused if server is holding the lock)
vibe-index add <data-dir> --manifest <package.toml-path> --repo-url URL [--ref REF --commit SHA]
vibe-index remove <data-dir> <group> <name> [--version V]

# Server
vibe-index serve <data-dir> [--bind ADDR] [--auth-tokens-file FILE] [--read-only] [--auto-commit-push]
vibe-index stop <data-dir>                                       # graceful shutdown via lock-file PID
```

##HELP-SMOKE **Help-text smoke** lives under `crates/vibe-index/tests/help_smoke.rs`, mirroring `every_subcommand_renders_help` in `vibe-cli`. @impl/done

### 2.12 Data structures {#types}

##req-types `req r1` @impl/done

##RUST-TYPES **Decision.** Rust types live in `crates/vibe-index/src/types/`, **hand-written against §2.6 rather than generated** — there is no JTD schema for them (measured 2026-08-05), so the text is the contract and the compiler checks nothing between them: @impl/done

```rust
pub struct Index {
    pub schema_version: u32,
    pub registry: String,
    pub registry_url: String,
    pub naming: NamingConvention,
    pub generated_at: DateTime<Utc>,
    pub generator: String,

    pub by_pkgref: BTreeMap<PkgKey, PackageEntry>,
    pub by_capability: BTreeMap<String, BTreeSet<VersionedPkgKey>>,
    pub by_purl: BTreeMap<String, BTreeSet<VersionedPkgKey>>,
    pub text_index: TextIndex,
}

pub struct PackageEntry {
    pub kind: PackageKind,
    pub name: String,
    pub latest_stable: Option<Version>,
    pub versions: BTreeMap<Version, VersionEntry>,
    pub indexed_at: DateTime<Utc>,
}

pub struct VersionEntry { /* §2.6 schema, one-to-one */ }
```

##PKGKEY-SHAPE `PkgKey = (PackageKind, String)` — interned for cheap clones.
`VersionedPkgKey = (PkgKey, Version)`. @impl/done

##TEXT-INDEX `TextIndex` — simple inverted index in v0: `BTreeMap<String, BTreeSet<VersionedPkgKey>>` mapping each tokenised word from name / description / keywords / capabilities / purls to the matching pkgrefs. Token = lowercased ASCII word; ~30-stopword filter (same filter `vibe-check::activation_conflict` uses, deliberately reused for consistency). Search: tokenise query, intersect token-postings, rank by term-overlap. Good enough for ≤10k packages; tantivy is a v1 upgrade if it isn't. @impl/done

### 2.13 Persistence layer {#persistence}

##req-persistence `req r1` @impl/done

##DATA-DIR-LAYOUT **Decision.** `<data-dir>/` layout: @impl/done

```
<data-dir>/
├── repomd.json                       # the manifest (§2.4)
├── primary.jsonl
├── primary.jsonl.gz
├── by-name/
│   └── <kind>/
│       └── <name>.json
├── by-cap/
│   └── <slug>.jsonl
├── by-purl/
│   └── <slug>.jsonl
├── README.md                         # auto-generated; explains "this is a vibevm index"
└── state/                            # NOT mirrored (gitignored when data-dir is a git working tree)
    ├── server.lock                   # PID file, present only when serve is running
    ├── admin.tokens                  # bearer tokens (gitignored)
    ├── checkpoint.json               # incremental-reindex bookkeeping (last commit/tag per repo)
    └── stats.json                    # counters for /metrics endpoint
```

##DATA-DIR-IS-WORKTREE The data-dir doubles as a git working tree of the org's `index` repo. `state/` is `.gitignore`d (the `init` subcommand writes a default `.gitignore`). Operators commit + push the rest manually (v0) or via `--auto-commit-push` (v1). @impl/done

##ATOMIC-WRITE-PROTOCOL **Atomic write protocol.** For each file F to be replaced: @impl/done

1. ##AW-TMP Write `F.tmp` next to `F`. @impl/done
2. ##AW-FSYNC `fsync(F.tmp)`. @impl/done
3. ##AW-RENAME `rename(F.tmp, F)`. @impl/done
4. ##AW-FSYNC-DIR `fsync(parent_dir(F))` on POSIX. (No-op on Windows where the directory has no fsync semantics; rename itself is atomic.) @impl/done

##REPOMD-LAST-LAW `repomd.json` is replaced **last** in any batch update, so a reader that fetches `repomd.json` first then chases hashes always sees consistent files. @impl/done

### 2.14 Integration with the rest of vibevm {#integration}

##req-integration `req r1` @impl/done

##consumer-side-lead **Consumer side (`vibe-cli`, `vibe-registry`).** @impl/done

- ##INT-FAST-PATH `crates/vibe-registry/src/multi_registry_resolver.rs` gains an optional **index-aware fast path**. Before falling back to per-repo `git ls-remote`, it tries `HTTP GET <registry.index_url>/repomd.json`. On 200, it reads `by-name/<name>.json` for the pkgref, selects the candidate whose `group` matches, and picks the matching version locally — zero ls-remote calls. On 404 / connect failure, fall through to today's path. @impl/done
- ##INT-VERIFY-ANYWAY Index-derived `content_hash` does NOT replace fetch-time verification. The actual `git fetch` still happens; the post-fetch `compute_content_hash` still runs; mismatch still errors out per [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity). @impl/done

##publisher-side-lead **Publisher side (`vibe-publish`).** @impl/done

- ##INT-PUBLISH-HOOK `crates/vibe-publish/src/lib.rs::Publisher::publish` gets an optional post-publish hook: if the registry has an `index_url` configured AND a `[[registry]].index_token` (env: `VIBEVM_INDEX_TOKEN_<HOST>`), Publisher POSTs the new entry to `<index_url>/v1/packages` after a successful `push_release`. Failure of the index POST does NOT fail the publish — it logs a warning and the operator's next `vibe-index reindex` covers the gap. @impl/done
- ##INT-DIRECT-PUSH Direct-push (`--repo-url`) bypasses index updates entirely (no registry context). @impl/done

##outdated-lead **`vibe outdated` (M1.10 follow-up).** @impl/done

- ##INT-OUTDATED-FAST Adds a fast path: when a registry has an index, query `by-name/<kind>/<name>.json` for the latest version instead of `git ls-remote`. Same envelope shape; ~100× faster for large lockfiles. @impl/done

##search-lead **`vibe search` (M2.10 — this is what unblocks it).** @impl/done

- ##INT-SEARCH Walks every configured registry's `index_url`, fetches `primary.jsonl.gz`, scans for matches against the user's query. Index is the enabling layer for M2.10; `vibe search` is the headline consumer of this PROP. @impl/done

##INT-SLICED Each integration point is a separate slice. v0 of `vibe-index` ships without any of them — the index can be populated and consumed via raw HTTP / git clone before vibevm consumers know about it. Integration slices land in M2.10 / M1.10 follow-ups. @impl/done

### 2.15 What index must NEVER do {#never}

##req-never `req r1` @impl/done

- ##NEVER-REPLACE-TRUTH **Never replace `vibe.toml` as the source of truth.** A package with a missing index entry still installs from git per the live path. A package with a divergent index entry triggers `IntegrityError`, never silent acceptance. @impl/done
- ##NEVER-MODIFY-REPOS **Never modify package repos.** The index utility reads package repos (for the `--from-clones` walk) but never writes to them. @impl/done
- ##NEVER-ECHO-TOKENS **Never echo tokens.** Same discipline as [PROP-000 §20](../../common/PROP-000.md#token-secrecy). Auth tokens for the server, GitHub API tokens for `--from-github`, publish tokens propagated through hooks — none ever appear in stdout / stderr / logs / JSON envelopes. @impl/done
- ##NEVER-ASSUME-MIRROR **Never assume mirror infrastructure.** The index is opt-in everywhere; no consumer or publisher fails because the index disappeared. @impl/done
- ##NEVER-SILENT-SCHEMA **Never make breaking schema changes silently.** `schema_version` bumps. Old consumers parsing a higher schema must surface a "your vibevm is older than this index; please upgrade" message — not silently parse the subset they understand. @impl/done

---

## 3. Architecture {#architecture}

### 3.1 Crate layout {#crate-layout}

##design-crate-layout `design r1` @impl/done

```
crates/vibe-index/                          # a member of the vibevm workspace
├── Cargo.toml                              # depends on vibe-core; no [workspace] table
├── README.md                               # operator-facing — how to run, common recipes
├── LICENSE                                 # EULA (vibevm's proprietary license)
├── src/
│   ├── main.rs                             # bin entrypoint — clap dispatch
│   ├── lib.rs                              # exports, top-level Error/Result
│   ├── cli/
│   │   ├── mod.rs                          # subcommand router
│   │   ├── init.rs
│   │   ├── reindex.rs
│   │   ├── add.rs
│   │   ├── remove.rs
│   │   ├── get.rs
│   │   ├── list.rs
│   │   ├── search.rs
│   │   ├── verify.rs
│   │   ├── dump.rs
│   │   ├── outdated.rs
│   │   ├── capabilities.rs
│   │   ├── purls.rs
│   │   ├── serve.rs
│   │   └── stop.rs
│   ├── index/
│   │   ├── mod.rs                          # Arc<RwLock<Index>>
│   │   ├── memory.rs                       # Index struct + ops
│   │   ├── persistence.rs                  # atomic write/read of files
│   │   ├── primary.rs                      # JSONL serialise/parse
│   │   ├── by_name.rs                      # per-package JSON
│   │   ├── repomd.rs                       # repomd.json
│   │   ├── checkpoint.rs                   # incremental-reindex state
│   │   └── search.rs                       # text index
│   ├── scanner/
│   │   ├── mod.rs                          # source-of-truth walkers
│   │   ├── from_clones.rs                  # walk org-dir clones via git2 / shell git
│   │   ├── from_github.rs                  # GitHub REST API walk
│   │   └── from_gitverse.rs                # stub today
│   ├── server/
│   │   ├── mod.rs                          # axum app builder
│   │   ├── routes/
│   │   │   ├── health.rs
│   │   │   ├── index_files.rs              # /v1/index/*
│   │   │   ├── packages.rs                 # /v1/packages*
│   │   │   ├── capabilities.rs
│   │   │   ├── purls.rs
│   │   │   ├── admin.rs                    # /v1/admin/*
│   │   │   └── metrics.rs
│   │   ├── auth.rs
│   │   ├── error.rs                        # RFC-7807 mapper
│   │   └── state.rs                        # AppState
│   ├── types/                              # Serializable schema types
│   │   ├── mod.rs
│   │   ├── entry.rs                        # VersionEntry + sub-types
│   │   ├── repomd.rs
│   │   └── kinds.rs                        # PackageKind, NamingConvention dupes
│   └── content_hash.rs                     # mirrors vibe-registry::compute_content_hash exactly
├── fixtures/
│   ├── sample-org/
│   │   ├── flow-wal/
│   │   ├── flow-atomic-commits/
│   │   └── stack-rust/
│   └── golden-index/
│       ├── repomd.json
│       └── primary.jsonl
├── tests/
│   ├── help_smoke.rs                       # clap renders help for every subcommand
│   ├── cli_e2e.rs                          # init + reindex + get + search round-trips
│   ├── server_e2e.rs                       # spawn server, drive HTTP API, shut down
│   ├── persistence_atomic.rs               # crash-mid-write recovery
│   ├── content_hash_parity.rs              # hash matches vibe-registry's exactly
│   └── scan_clones.rs                      # walks fixtures/sample-org/
└── docs/
    ├── operator-handbook.md
    ├── consumer-protocol.md                # HTTP API reference
    └── format.md                           # repomd / primary / by-name / by-cap / by-purl
```

### 3.2 Dependencies {#deps}

##design-deps `design r1` @impl/done

##deps-lead Minimal Rust crates to keep redistribution clean: @impl/done

- ##dep-clap `clap` (derive) — CLI dispatch. @impl/done
- ##dep-tokio `tokio` (full) — async runtime for the server. @impl/done
- ##dep-axum `axum` — HTTP framework. Mature, minimal, integrates with `tower` middleware. @impl/done
- ##dep-tower `tower` / `tower-http` — auth, CORS, tracing layers. @impl/done
- ##dep-serde `serde` / `serde_json` — JSON. @impl/done
- ##dep-toml `toml` — read package manifests. @impl/done
- ##dep-semver `semver` — version handling. Same dep `vibe-core` uses; pin same version. @impl/done
- ##dep-sha2 `sha2` — content_hash. Same as `vibe-registry`. @impl/done
- ##dep-flate2 `flate2` — gzip primary.jsonl.gz. @impl/done
- ##dep-walkdir `walkdir` — directory traversal (matches `vibe-registry`). @impl/done
- ##dep-tracing `tracing` / `tracing-subscriber` — logging. @impl/done
- ##dep-chrono `chrono` — timestamps. @impl/done
- ##dep-thiserror `thiserror` — error enums. @impl/done
- ##dep-git `gix` (or shell-out via `std::process::Command`) — read git tags / show files at refs. Decision §3.3. @impl/done
- ##dep-reqwest `reqwest` — `--from-github` HTTP client. @impl/done
- ##dep-tempfile `tempfile` — atomic write helpers. @impl/done
- ##dep-prometheus `prometheus` — `/metrics` endpoint. @impl/done

##VIBE-CORE-DEP **`vibe-core` dependency.** `vibe-index` parses `vibe.toml` and `vibe-subskill.toml` through `vibe-core`'s own `Manifest` / `SubskillManifest` types, so the index can never drift from the manifest schema. This reverses the proposal's original standalone-no-`vibe-core` stance — [§6](#distribution) records the reversal, [§9](#open) item 11 the de-rot finding that forced it. What stays duplicated is small and stable: the four-variant `PackageKind` / `NamingConvention` (`src/types/kinds.rs`, frozen by `VIBEVM-SPEC.md` §4, needing the `Ord` + `clap::ValueEnum` the `vibe-core` originals lack) and the `compute_content_hash` algorithm (`src/content_hash.rs`, gated by `tests/content_hash_parity.rs` against a byte-for-byte copy of `fixtures/registry/flow/wal/v0.1.0/`). `compute_content_hash` folds into `vibe-core` once it is lowered out of `vibe-registry`. @impl/done

##not-pulling-lead **Deliberately NOT pulling:** @spec/done

- ##not-pulling-db A database (SQLite / PostgreSQL). All state in RAM + flat files. @spec/done

### 3.3 Git access in the scanner {#git-access}

##design-git-access `design r1` @impl/done

##SCANNER-SHELL-OUT **Decision.** Use shell-out to `git` via `std::process::Command` for the scanner's read paths (`git tag`, `git show <ref>:<path>`, `git rev-parse <tag>`). Same path `vibe-registry::shell.rs` already follows. Rationale matches PROP-001 §2.1: shell-out works on every platform git works on, no per-host bindings to maintain. @impl/done

##not-gix **Not** `gix` for v0: smaller dep tree wins. v1 may switch if perf demands and gix's read API matures further. @spec/done

### 3.4 Threading model {#threading}

##design-threading `design r1` @impl/done

- ##THREAD-CLI-SYNC CLI mode: synchronous. tokio runtime is created only in `serve` subcommand. @impl/done
- ##THREAD-SERVER-ASYNC Server mode: tokio multi-thread runtime. Routes are async; `Arc<RwLock<Index>>` is `tokio::sync::RwLock` (async lock). @impl/done
- ##THREAD-WRITER-TASK Disk writes serialised through a single dedicated tokio task: `index_writer`. The server posts mutations to it via an mpsc channel; the writer applies them in order. This avoids fsync stalls blocking the request handlers. @impl/done

### 3.5 Configuration precedence {#config}

##design-config `design r1` @impl/done

##CONFIG-PRECEDENCE For every flag with a default, precedence is: explicit CLI flag > env var (`VIBE_INDEX_*`) > on-disk config (`<data-dir>/state/config.toml`, optional) > built-in default. Same shape `vibe show config` already uses on the consumer side. @impl/done

---

## 4. Phase plan (slices) {#phases}

##slices-lead Each slice = one or more conventional commits. The utility becomes useful at slice 5 (read endpoints + reindex from clones); the rest are progressive enhancements. @impl/done

### 4.1 Slice 1 — skeleton {#slice-1}

##SLICE-1 `crates/vibe-index/` standalone crate with `Cargo.toml` + `src/main.rs` + `src/lib.rs`. clap dispatch with stub subcommands that all print "not yet implemented". `vibe-index --version` works. `tests/help_smoke.rs` passes. @impl/done

##slice-1-commit Commit: `feat(services/vibe-index): skeleton crate + clap subcommand dispatch`. @impl/done

### 4.2 Slice 2 — types + persistence {#slice-2}

##SLICE-2 `src/types/` (entry / repomd), `src/index/` (memory, persistence, primary, by_name, repomd). JTD schemas in `schemas/`. Atomic write protocol. `vibe-index init` works (writes empty `repomd.json` + empty `primary.jsonl`). `vibe-index dump` works. `vibe-index verify` works (checks file hashes). Round-trip tests. @impl/done

##slice-2-commits Commits: @impl/done

- ##slice-2-c1 `feat(services/vibe-index): index entry + repomd schemas + JTD` @impl/done
- ##slice-2-c2 `feat(services/vibe-index): in-memory index + atomic persistence` @impl/done
- ##slice-2-c3 `feat(services/vibe-index): vibe-index init/dump/verify` @impl/done

### 4.3 Slice 3 — scanner + reindex from clones {#slice-3}

##SLICE-3 `src/scanner/from_clones.rs` walks `<org-dir>/<repo>/.git` directories; `src/content_hash.rs` mirrors `vibe-registry::compute_content_hash`; `vibe-index reindex --from-clones` works against `fixtures/sample-org/`. Parity test against `vibe-registry`. Incremental mode = full for now (deferred to slice 7). @impl/done

##slice-3-commits Commits: @impl/done

- ##slice-3-c1 `feat(services/vibe-index): content_hash parity with vibe-registry` @impl/done
- ##slice-3-c2 `feat(services/vibe-index): scanner — walk org-dir clones` @impl/done
- ##slice-3-c3 `feat(services/vibe-index): vibe-index reindex --from-clones` @impl/done

### 4.4 Slice 4 — read CLI subcommands {#slice-4}

##SLICE-4 `get`, `list`, `search`, `capabilities`, `purls`, `outdated`. Inverted text index for search. JSON output for every subcommand. `cli_e2e.rs` covers each. @impl/done

##slice-4-commits Commits: @impl/done

- ##slice-4-c1 `feat(services/vibe-index): inverted text index for search` @impl/done
- ##slice-4-c2 `feat(services/vibe-index): vibe-index get/list/search/capabilities/purls` @impl/done
- ##slice-4-c3 `feat(services/vibe-index): vibe-index outdated against a vibe.lock` @impl/done

### 4.5 Slice 5 — HTTP server (read-only) {#slice-5}

##SLICE-5 `vibe-index serve --read-only`. axum app exposes `/healthz`, `/readyz`, `/v1/index/*`, `/v1/packages*`, `/v1/capabilities/*`, `/v1/purls/*`, `/metrics`. PID lock file. CORS open. `server_e2e.rs` covers each route. @impl/done

##slice-5-commits Commits: @impl/done

- ##slice-5-c1 `feat(services/vibe-index): axum server skeleton + healthz/readyz` @impl/done
- ##slice-5-c2 `feat(services/vibe-index): GET /v1/index/* file routes` @impl/done
- ##slice-5-c3 `feat(services/vibe-index): GET /v1/packages query routes` @impl/done
- ##slice-5-c4 `feat(services/vibe-index): /metrics prometheus endpoint` @impl/done

##MVP-MARK After slice 5: vibe-index is **independently usable** as a read-only server fed by `reindex --from-clones`. This is the "MVP" mark. @impl/done

### 4.6 Slice 6 — write CLI + write HTTP + auth {#slice-6}

##SLICE-6 `vibe-index add` / `vibe-index remove`. HTTP `POST /v1/packages`, `DELETE /v1/packages/...`. Bearer-token auth via `<data-dir>/state/admin.tokens`. Write-side server-vs-CLI lock arbitration. @impl/done

##slice-6-commits Commits: @impl/done

- ##slice-6-c1 `feat(services/vibe-index): vibe-index add/remove (CLI)` @impl/done
- ##slice-6-c2 `feat(services/vibe-index): bearer-token auth + admin.tokens loader` @impl/done
- ##slice-6-c3 `feat(services/vibe-index): POST/DELETE /v1/packages routes` @impl/done

### 4.7 Slice 7 — incremental reindex {#slice-7}

##SLICE-7 `<data-dir>/state/checkpoint.json`. `vibe-index reindex --incremental --from-clones` walks the diff between checkpoint and current state. Test: full vs incremental produce identical output. @impl/done

##slice-7-commit Commit: `feat(services/vibe-index): incremental reindex via checkpoint`. @impl/done

### 4.8 Slice 8 — `--from-github` mode {#slice-8}

##SLICE-8 `reqwest`-based GitHub API walk. `--token-file FILE`. Rate-limit-aware backoff. Same shape as `--from-clones` from caller's POV. @impl/done

##slice-8-commit Commit: `feat(services/vibe-index): reindex --from-github (REST API walk)`. @impl/done

### 4.9 Slice 9 — vibe-publish post-publish hook {#slice-9}

##SLICE-9 `crates/vibe-publish/src/lib.rs::Publisher::publish` gains optional index POST after successful push. New env var `VIBEVM_INDEX_TOKEN_<HOST>`. New `[[registry]].index_url` / `[[registry]].index_token` fields in the project manifest. @impl/done

##slice-9-commit Commit: `feat(vibe-publish): POST to registry index after publish (opt-in)`. @impl/done

### 4.10 Slice 10 — consumer-side integration {#slice-10}

##SLICE-10 `crates/vibe-registry/src/multi_registry_resolver.rs` gains the index-aware fast path. Falls back transparently on 404 / connect-failure. Live e2e test against an index-equipped registry. @impl/done

##slice-10-commit Commit: `feat(vibe-registry): consume registry index for resolve fast path (opt-in)`. @impl/done

### 4.11 Slice 11 — docs + manual-test smoke {#slice-11}

##SLICE-11 `crates/vibe-index/docs/` filled in (operator-handbook, consumer-protocol, format). `manual-tests/M2.10-index-smoke.md` walks the live e2e: bootstrap an index from a fresh org dir, serve it, install a package through it, search it. @impl/done

##slice-11-commits Commits: @impl/done

- ##slice-11-c1 `docs(vibe-index): operator handbook + consumer protocol + format reference` @impl/done
- ##slice-11-c2 `test: manual-test smoke for index bootstrap + consume` @impl/done

---

## 5. Test plan {#tests}

##tests-lead Per slice (specifics in §4); cumulative state at GA: @impl/done

- ##TEST-UNIT **Unit:** every type round-trips through serde JSON / TOML; every CLI subcommand has at least one happy-path test; every server route has at least one happy-path + one auth-fail test. @impl/done
- ##TEST-INTEGRATION **Integration:** full-reindex against `fixtures/sample-org/` produces a byte-identical `primary.jsonl` to `fixtures/golden-index/primary.jsonl`. Incremental reindex applied to the same starting state is byte-identical to a full reindex. @impl/done
- ##TEST-PARITY **Parity:** `tests/content_hash_parity.rs` runs the same fixture package through `vibe-registry::compute_content_hash` AND `crates/vibe-index/src/content_hash.rs`, asserts equality. CI gates the merge if they diverge. @impl/done
- ##TEST-E2E **End-to-end:** `tests/server_e2e.rs` spawns the server in-process (axum's `oneshot` style), drives every documented route over HTTP, asserts response shapes. @impl/done
- ##TEST-CRASH **Crash recovery:** `tests/persistence_atomic.rs` simulates mid-write crash by failing the rename step; asserts the previous version remains readable. @impl/done
- ##TEST-HERMETIC **Hermetic vs live:** all tests above run hermetically (no network). A separate `cli_live_e2e.rs` (`#[ignore]`-d, opt-in via `cargo test -- --ignored`) walks `--from-github vibespecs` against the real registry to confirm the API walk works against actual infrastructure. @impl/done

---

## 6. Distribution — a workspace crate {#distribution}

##design-distribution `design r1` @impl/done

##WORKSPACE-MEMBER **Decision (revised 2026-05-22).** `vibe-index` lives at `crates/vibe-index/` as a member of the top-level vibevm workspace. It is built, tested, clippy-gated, and fmt-checked by the same `cargo … --workspace` invocations as every other crate, and it depends on `vibe-core` directly. @impl/done

##fold-in-why **Why this reverses the original standalone-workspace decision.** The proposal first placed `vibe-index` in its own Cargo workspace under `services/`, outside `crates/`, so an org owner could vendor just that subdirectory. The cost was a hand-duplicated `vibe.toml` parser with nothing tying it to `vibe-core` — and, sitting outside `cargo test --workspace`, nothing routinely exercising it. It rotted silently against the M1.17 / M1.18 manifest-schema churn ([§9](#open) item 11). Folding the crate back in kills both failure modes at once: the scanner now parses through `vibe-core::Manifest` (one source of truth — the schema cannot drift), and the routine workspace gate covers it (drift is caught the moment it appears). @spec/done

##REDISTRIBUTION **Redistribution.** An org owner who wants to host their own index server clones the vibevm repository and runs `cargo install --path crates/vibe-index`. The "vendor only the subdirectory" affordance is gone; in exchange the binary can never ship a stale view of the manifest schema. The HTTP-server deps (`axum` / `tower` / `tower-http`) enter the workspace `Cargo.lock` — `reqwest` was already there for the `vibe-registry` index client, so the marginal cost is `tower` / `tower-http` / `flate2`. @impl/done

##GATE-COVERS **Gate.** `tools/self-check.sh` no longer special-cases a second workspace — steps 1–2 (`cargo test --workspace`, `cargo clippy --workspace`) cover `vibe-index` like any member. @impl/done

---

## 7. Auth, secrets, scope {#secrets}

##req-secrets `req r1` @impl/done

##SECRECY-INHERITED [PROP-000 §20](../../common/PROP-000.md#token-secrecy) covers the token-secrecy invariant; PROP-005 inherits it verbatim. Specifically: @impl/done

- ##SECRET-ADMIN-TOKENS **Server admin tokens** (`<data-dir>/state/admin.tokens`) — file mode 0600, never echoed in logs / responses / error messages, gitignored. @impl/done
- ##SECRET-GITHUB-TOKENS **GitHub API tokens** (`--from-github --token-file FILE`) — same discipline. Read once into memory, scrubbed from the env, never persisted outside the source file. @impl/done
- ##SECRET-INDEX-TOKENS **Index POST tokens** (`VIBEVM_INDEX_TOKEN_<HOST>` for the publish-side hook) — per-host shape mirrors `VIBEVM_PUBLISH_TOKEN_<HOST>`. @impl/done

##SCOPE-DISCIPLINE **Scope discipline.** The server's mutation endpoints accept entries only for the registry the server was started with (`<data-dir>/repomd.json::registry`). A POST attempting to land an entry whose `registry` field disagrees with the server's configured registry → 400 with a clear message. Same shape `vibe-publish::validate_scope` enforces on the publish side. @impl/done

---

## 8. Operations {#ops}

##ops-lead A typical setup for an org owner who wants to host an index: @impl/done

```
# One-time bootstrap (on a host with the org's clones available).
$ vibe-index init  ./vibespecs-index   --registry vibespecs   --registry-url https://github.com/vibespecs   --naming kind-name
$ vibe-index reindex ./vibespecs-index --from-clones  /var/lib/vibespecs-mirror

# Push the static files to <org>/index repo (operators wire this once):
$ cd ./vibespecs-index
$ git init && git remote add origin https://github.com/vibespecs/index
$ git add . && git commit -m "initial index"
$ git push -u origin main

# Run the live server (optional — only if hosting the HTTP-API path):
$ vibe-index serve ./vibespecs-index --bind 0.0.0.0:8412 \
    --auth-tokens-file ./vibespecs-index/state/admin.tokens

# Periodic incremental refresh (cron):
$ */5 * * * *  vibe-index reindex /home/owner/vibespecs-index --incremental --from-clones /var/lib/vibespecs-mirror
```

##ops-consumers-note Most consumers see only the static raw-HTTP files; the server is for orgs that need real-time publish updates. @impl/done

---

## 9. Open questions {#open}

1. ##OPEN-LOCATION **Index file location: `<org>/index` repo vs `<org>/<package-repo>/index/...` per-package?** PROP-005 picks `<org>/index`. The alternative — per-package files inside each package repo — was rejected because it leaves catalog discovery a chicken-and-egg problem (you need to enumerate the org first). If new evidence emerges that orgs object to a top-level `index` repo (naming conflicts, permission boundaries), we revisit. @spec/work
2. ##OPEN-COMPRESSION **`primary.jsonl.gz` compression: gzip vs zstd?** v0 picks gzip — universally supported by every HTTP client; deterministic with `mtime=0`. v1 may add a `primary.jsonl.zst` alongside. @spec/work
3. ##OPEN-GPG **GPG signing of `repomd.json`?** Tracked here, not shipped in v0. Shape: `repomd.json.asc` next to `repomd.json`; consumers verify against a per-registry public key recorded in `[[registry]].pgp_key`. v1. @spec/work
4. ##OPEN-MERKLE **Merkle log (Go sumdb-style transparent log)?** Tracked here. v2+. Useful for adversarial environments; v0/v1 trust the host. @spec/work
5. ##OPEN-AUTO-PUSH **Auto-commit-and-push from server** — slice 9 question. Risk: server gets push credentials, which is a step up in trust. Mitigation: keep CLI-driven commit/push the default; `--auto-commit-push` opt-in. @spec/work
6. ##OPEN-MULTI-REGISTRY **Multi-registry server** — should one server instance host multiple data dirs (one per registry)? v0 says no (one process per registry). Trivial scale-out via process supervision; we revisit if multi-tenancy demand emerges. @spec/work
7. ##OPEN-SSE **WebSockets / Server-Sent Events for live publish notifications** — out of scope. Polling `/v1/admin/status::last_reindex` is sufficient at our scale. @spec/work
8. ##OPEN-OCI **OCI registry shape** — could we host the index inside an OCI registry instead of git? Out of scope; revisit if the OCI tooling becomes universal among vibevm operators. @spec/work
9. ##OPEN-CAP-VS-PURL **Capability- vs PURL-driven search** — v0 ships `by-cap` and `by-purl` as separate files. If usage shows one dominates, the loser may be folded into the inverted text index. Empirical question. @spec/work
10. ##OPEN-RATE-LIMIT **Rate-limiting on the server** — shipped after the v0 plan: `server/rate_limit.rs` is a per-token and per-IP token-bucket limiter, disabled by default and opt-in by flag. Production deployments may still front it with a reverse proxy's limiter; the two compose. @impl/done

11. ##OPEN-STANDALONE-RESOLVED **Standalone-workspace duplication — RESOLVED 2026-05-22 by folding the crate in.** The 2026-05-22 de-rot found `crates/vibe-index/` had silently rotted: its duplicated `vibe.toml` parser still expected the pre-M1.17 shape (`[writes]`, `[dependencies]`, `[boot_snippet].filename`) and could not parse a current manifest, and its `content_hash` parity test had drifted off a fixture renamed by the M1.17 manifest unification. §3.2 had weighed the duplication cost for `compute_content_hash` alone ("the algorithm doesn't change"); the *manifest schema*, by contrast, churned hard through M1.17 / M1.18, and the duplicate parser had no cross-check to catch the drift — the standalone workspace also sat outside the routine `cargo test --workspace` gate. **Resolution:** fold `vibe-index` into the `crates/` workspace and parse through `vibe-core::Manifest` (see [§6](#distribution)). The duplicated parser is deleted; only the tiny, schema-frozen `PackageKind` / `NamingConvention` and the `compute_content_hash` algorithm remain duplicated, both justified in §3.2. @impl/done

---

## 10. Acceptance criteria {#acceptance}

##acceptance-lead A given slice is considered accepted when: @impl/done

- ##ACC-TESTS All tests in its slice pass. @impl/done
- ##ACC-CLIPPY `cargo clippy --workspace --all-targets -- -D warnings` is clean. @impl/done
- ##ACC-FMT `cargo fmt --check` is clean. @impl/done
- ##ACC-SELF-CHECK `tools/self-check.sh` is green. @impl/done
- ##ACC-HELP-SMOKE Help-text smoke covers any new subcommand. @impl/done
- ##ACC-MANUAL-WALK A manual walk through `crates/vibe-index/docs/operator-handbook.md` succeeds against `fixtures/sample-org/`. @impl/done

##CLOSURE-CRITERION PROP-005 is considered closed once slices 1–8 land. Slices 9–11 are integration with the rest of vibevm and ship under their respective milestone PRs. @impl/done

---

## 11. Wire-up scripts (informational, not shipped) {#wire-up}

##wire-up-lead For operators wiring the index into their hosting: @spec/done

##WIRE-POST-RECEIVE **git `post-receive` hook on the org's hosted git** (Forgejo/Gitea/GitVerse-style) — push to a package repo triggers an HTTP POST to the index server: @spec/done

```sh
#!/bin/sh
# /var/git/<org>/<repo>.git/hooks/post-receive
while read oldrev newrev refname; do
    case "$refname" in
        refs/tags/v*)
            curl -sf -X POST "https://index.example/v1/admin/reindex" \
                -H "Authorization: Bearer $(cat /etc/vibe-index/admin.token)" \
                -H "content-type: application/json" \
                -d '{"mode":"incremental","source":"clones"}' \
                || echo "vibe-index reindex POST failed (non-fatal)"
            ;;
    esac
done
```

##WIRE-CRON **cron line:** @spec/done

```cron
*/5 * * * *  vibe-index reindex /home/owner/vibespecs-index --incremental --from-clones /var/lib/vibespecs-mirror >>/var/log/vibe-index.log 2>&1
```

##wire-up-not-shipped These live in `crates/vibe-index/docs/operator-handbook.md` rather than as shipped binaries — operators integrate at their own host, and the hook shape varies enough across hosting platforms that one-size-fits-all isn't worth shipping. @spec/done

---

## 12. Version history {#history}

- ##HISTORY-DRAFT-1 **2026-05-06 — draft 1.** Initial proposal. Open for review. @spec/done
- ##HISTORY-RECONCILED **2026-05-22 — reconciled with the implementation, then folded into the workspace.** A state review found PROP-005 already implemented (slices 1–10 + M2.10 `vibe search`) but rotted; the de-rot realigned the scanner with the current `vibe.toml` schema and corrected this document (§2.6 `boot_snippet`, the `vibe.toml` filename, §2.10 rate-limiter status). The fold then moved `vibe-index` from its own `services/` workspace into `crates/vibe-index/` and switched it to parse through `vibe-core::Manifest` — §3.2, §6, and §9 item 11 are revised for the reversed standalone-workspace decision. @spec/done
- ##HISTORY-GROUP-NATIVE **2026-05-22 — group-native (PROP-008 Phase 7).** The index entry gained the mandatory `group` field and the optional `workspace_origin` (§2.6); the `by-name/` layer was re-keyed from `by-name/<kind>/<name>.json` to the candidate-set file `by-name/<name>.json` (§2.4) — one GET per registry now yields every group sharing a bare name; `primary.jsonl` / `by-cap` / `by-purl` sort on the `(group, name, version)` identity; the HTTP `/v1/packages/{group}/{name}` routes, the `vibe-index get/remove` CLI, and the `naming = "fqdn"` default followed. The `vibe-registry` index client and the `vibe-publish` post-publish hook were realigned to the new shape. PROP-008 §2.8's index extension is shipped. @spec/done
