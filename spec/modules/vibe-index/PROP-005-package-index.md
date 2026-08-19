# PROP-005: Optional package index — per-org metadata + standalone index server {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: implemented; folded into the workspace 2026-05-22"/>

@fact:milestone-line **Milestone:** retrofits into M2.10 (`vibe search`) and M1.10 (`vibe outdated`) from `ROADMAP.md`. Slices land independently; index is opt-in everywhere. @status:impl/done

@fact:status-line **Status:** implemented; folded into the workspace 2026-05-22. Slices 1–8 (the `vibe-index` server + CLI) and slices 9–10 (publisher hook + consumer fast path) are shipped, plus M2.10 `vibe search`. `vibe-index` lives at `crates/vibe-index/` as a workspace member ([§6](#distribution)) and parses through `vibe-core::Manifest`. See [§9](#open) item 11 for the de-rot and fold that got it there. @status:impl/done

@fact:related **Related:** [PROP-001](../vibe-registry/PROP-001-git-backend.md) (git backend), [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md) (`[[registry]]` / `[[mirror]]` / `[[override]]` / content-hashed identity), [PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) (features / subskills / `describes` / conditional deps), [PROP-004](../../../legacy-spec/research/PROP-004-tessl-comparative-research.md) §5.x (gap analysis), [`spec://org.vibevm.core/vibevm/common/PROP-000`](../../common/PROP-000.md) (especially §15 dep weight, §16 JTD, §17 production architecture, §18 complexity ≥ RPM, §20 token secrecy). @status:spec/done

@fact:research-summary **Out-of-band research summary** (2026-05-06, prior session). Comparative inventory of indexing strategies in production package managers — Maven Central (Lucene + directory layout), npm (CouchDB replicated DB), PyPI (PEP 503/691 simple API), RPM/DNF (`repodata/primary.xml.gz` + libsolv), Deb/APT (`Packages.gz` RFC822), Cargo (git index → sparse HTTP), Go modules (proxy + Merkle sumdb), Nix flakes (per-flake `flake.lock`, no global index), Homebrew (mono-repo formula), OCI registries (`/v2/_catalog`). Three candidate paths surfaced for vibevm: **(a)** Cargo-sparse-style per-package JSON files in an org-level index; **(b)** DNF-style single repodata directory with full SAT-ready dep graph; **(c)** Nix-flake-style indexless live-resolve (current state) with optional `flake-registry`-shape short-name mapping. PROP-005 picks (a) augmented by (b)'s integrity-manifest pattern (`repomd.json`). @status:spec/done

---

## 1. Motivation {#motivation}

@fact:live-resolve-lead vibevm today resolves packages **live** against the host's git API: @status:impl/done

- @fact:live-install `vibe install flow:wal` translates to `git ls-remote <org-url>/flow-wal.git` to enumerate tags, then `git archive` (or `git fetch && git checkout`) to read the manifest at the candidate ref. One pkgref = at least one round-trip per registry walked, two if the package is not in the first registry. @status:impl/done
- @fact:live-outdated `vibe outdated` (M1.10) calls `MultiRegistryResolver::resolve(<pkgref>@Latest)` per locked package — one round-trip each. @status:impl/done
- @fact:live-search `vibe search` (M2.10, not yet shipped) cannot work at all without enumerating an org. GitHub's `GET /orgs/<org>/repos` is rate-limited to 60 req/h unauth or 5000 req/h with a token; GitVerse exposes no org-scoped repo listing in its public API. @status:impl/done

@fact:scale-limit This works at the M0 / M1 demonstration scale (3 packages in `vibespecs`). It does not work at v1 shipping scale (target: hundreds of packages per org, multiple orgs configured per project). @status:impl/done

@fact:failure-modes-lead **Failure modes the live-resolve path produces in practice:** @status:impl/done

1. @fact:FAIL-COLD-LATENCY **Cold-cache install latency grows linearly with the dep graph.** A project with 20 transitive deps spread across two registries spends 30–60 s in `git ls-remote` alone before any actual content fetch. @status:impl/done
2. @fact:FAIL-RATE-LIMIT **Rate-limit visibility for `vibe outdated`.** Polling N packages at refresh time burns N requests; against an unauthenticated GitHub registry, this exhausts the quota at 60 packages. @status:impl/done
3. @fact:FAIL-SEARCH **`vibe search` is impossible.** Even with an authenticated org-listing endpoint, parsing every repo's `vibe.toml` at every search would be intractable. @status:impl/done
4. @fact:FAIL-DISCOVERY **Discovery story is silent.** A consumer with a fresh checkout of an unknown vibevm org has no way to enumerate "what packages live here?" without scraping the host UI. @status:impl/done
5. @fact:FAIL-OFFLINE **Mirror-driven offline workflows degrade silently.** [PROP-002 §2.3](../vibe-registry/PROP-002-decentralized-registry.md#mirror) makes mirror dispatch invisible to the lockfile, but `ls-remote` against a mirror still leaks live host calls when the resolver wants to know "what versions exist?" — there's no offline catalog. @status:impl/done

@fact:INDEX-NEEDED What every other production package manager does — and what vibevm now needs — is an **index**: a small set of files, regenerable from authoritative package state, that lets consumers perform `list`, `search`, `outdated`, and `resolve-version-shortlist` operations against cached / mirror-able metadata instead of live git. RPM (`primary.xml.gz`), Cargo (sparse index), Deb (`Packages.gz`), npm (CouchDB document per package) all converge on the same shape: **derived metadata files alongside or near the artefact storage, regenerated by a tool, served as plain HTTP**. @status:impl/done

---

## 2. Decisions {#decisions}

### 2.1 Index is OPTIONAL — zero impact when absent {#optional}

@fact:req-optional `req r1` @status:impl/done

@fact:INDEX-OPTIONAL **Decision.** The index layer is **strictly additive**. Every existing vibevm code path keeps working exactly as today when no index is present. No registry is required to have an index. No project is required to consume one; a consumer that finds none falls back to the live `git ls-remote` path that exists today. @status:impl/done

@fact:DISCOVERY-ASKS-THE-HANDSHAKE-FIRST **Discovery is a ladder, and the eternal handshake is its first rung.** A consumer probes two candidate bases in order — `<index-url>/v1/index`, then `<index-url>` — and at each one asks `hello.json` **before** any `repomd.json`. A 200 whose body parses as a handshake and carries a world of the epoch this build reads settles the probe: that world's `path` refines the base every later file is fetched from. Only «no handshake here» (404, 5xx, connect failure) moves on to the next candidate and, when neither answers, to the `repomd.json` probe at the same two bases — the compatibility surface for indexes published before the handshake existed. Nothing at all → the live path, silently, exactly as before. @status:impl/done

@fact:WHY-THE-HANDSHAKE-IS-ASKED-BEFORE-THE-MANIFEST **Asked first, not asked beside** — because `successor` is the in-band forwarding pointer for an index that MOVED, and it is readable exactly when the old address no longer serves a catalog. A handshake sought only next to a `repomd.json` that answered would be read in every case except the one it exists for. The price is up to two extra GETs, and it is paid only by indexes that have no handshake. @status:impl/done

@fact:A-PROBE-HAS-THREE-OUTCOMES-NOT-TWO **A probe answers `found`, `absent`, or `refused`, and the third is what keeps this section honest.** «Absent» is the only outcome that falls through quietly, because it is the one that means what the fall-through assumes: nothing is published here. An index that IS there and cannot serve this consumer — it refused us (401/403), its body does not parse as a handshake, its handshake format is one this build does not read, or it publishes no world of this build's epoch — answers **`refused`**, carrying the offered epochs, this build's epoch, a recipe, and whatever the document said in `min_client` / `notice` / `successor`. Collapsing that into «absent» would make a private, broken or newer-than-us index indistinguishable from a missing one, which is the silence [PROP-044 §2](../../common/PROP-044-change-native-formats.md#laws) forbids: a break that announces itself is normal life, a riddle is what strands users. @status:impl/done

@fact:A-SUCCESSOR-IS-NAMED-NEVER-FOLLOWED **A `successor` is named to the operator, never followed by the client.** Automatic following needs a cycle watchman and a trust rule for the address it lands on, and neither is decided; naming the address costs the human one command and invents no policy. @status:impl/done

@fact:optional-rationale-lead **Rationale.** @status:spec/done

- @fact:rationale-backward-compat Backward compat for existing `vibespecs` (GitHub) + `vibespecs-gitverse` (GitVerse) registries — they keep working unchanged. Index is something the org owner opts into. @status:spec/done
- @fact:rationale-decoupled Decouples the index design from Phase A of vibevm — operators with three packages do not need an index; operators with three hundred do. @status:spec/done
- @fact:rationale-no-central Removes the "central server is now load-bearing" failure mode: if the index disappears, the live path is still there. @status:spec/done

- @fact:OPPORTUNISTIC **Consequence.** All optimisations the index unlocks (cold-cache install speed, `vibe search`, faster `vibe outdated`) are **opportunistic**. @status:impl/done
- @fact:INTEGRITY-UNCHANGED The integrity story (`content_hash` per [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity)) does not change — content_hash is verified at fetch time regardless of whether the resolve path went through the index. @status:impl/done

### 2.2 Form factor: per-org **index** living in a separate git repository {#form-factor}

@fact:req-form-factor `req r1` @status:impl/done

@fact:INDEX-REPO **Decision.** Each vibevm registry org that opts in maintains a dedicated git repository named `index` (configurable; default name `index`) under the same org root: @status:impl/done

- @fact:index-url-github `https://github.com/vibespecs/index` @status:impl/done
- @fact:index-url-gitverse `git@gitverse.ru:vibespecs/index.git` @status:impl/done

@fact:index-repo-properties-lead Inside this repository sits a fixed file layout (§2.4) holding the org's catalog. The repository is: @status:impl/done

- @fact:REPO-CLONEABLE **Cloneable like any other** — same auth model as the package repos (HTTPS public read; SSH or token push for the maintainer). @status:impl/done
- @fact:REPO-HTTP-FETCHABLE **HTTP-fetchable** at raw URLs without cloning — `https://raw.githubusercontent.com/vibespecs/index/main/repomd.json` (GitHub) / `https://gitverse.ru/api/v1/repos/vibespecs/index/raw/main/repomd.json` (GitVerse). Consumers default to raw HTTP (one GET, no clone); falling back to git clone only when the host's raw-HTTP shape is unknown. @status:impl/done
- @fact:REPO-MIRRORABLE **Mirror-able trivially** — the `[[mirror]]` machinery from [PROP-002 §2.3](../vibe-registry/PROP-002-decentralized-registry.md#mirror) applies unchanged: a mirror at `https://mirror.internal/vibespecs/index` is a drop-in. @status:impl/done

@fact:why-dedicated-lead **Why a dedicated repo, not files-in-package-repos.** @status:spec/done

- @fact:why-discovery **Discovery.** A single `<org>/index` repo answers "what's in this org?" with one HTTP GET. Per-package metadata files would still require enumerating the org first — chicken-and-egg. @status:spec/done
- @fact:why-atomicity **Atomicity of catalog state.** A single index repo can be replaced as a whole, signed as a whole, mirrored as a whole. Per-package files leave catalog consistency to the consumer to reconstruct. @status:spec/done
- @fact:why-decoupling **Decoupling.** Index regeneration does not touch package repos. Authors do not need to run any utility. The org owner runs `vibe-index` on a cadence; package repos stay pristine. @status:spec/done
- @fact:why-mirror-parity **Mirror parity with [PROP-002 §2.3](../vibe-registry/PROP-002-decentralized-registry.md#mirror).** The same mirror chain that covers package repos covers the index. Operators who already understand `[[mirror]]` get the index covered for free. @status:spec/done

@fact:why-not-hosted-lead **Why not a hosted central HTTP service** (npm-style `registry.npmjs.org`): @status:spec/done

- @fact:not-hosted-infra Requires running infra. vibevm's deliberate posture per [PROP-000 §17](../../common/PROP-000.md#production-architecture) is "every org self-hosts on hosting they already use" — git platforms are that hosting. @status:spec/done
- @fact:not-hosted-single-vendor Single-vendor. We rejected this shape in [PROP-002 §1](../vibe-registry/PROP-002-decentralized-registry.md#motivation) (the "Nix's failure pattern"). Centralising the index is the same anti-pattern at one layer up. @status:spec/done
- @fact:not-hosted-available HTTP service is **available** (§2.5) — the `vibe-index serve` mode lets an operator run one — but it is not the default consumption path. Most consumers go through static raw-HTTP files in the index git repo. @status:spec/done

@fact:INDEX-URL-CONFIG **Configurable but defaulted — the specified form, which the manifest does not yet carry.** A `[[registry]]` block pins a custom index location: @status:spec/plan

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

@fact:INDEX-URL-DEFAULT Default: `<registry-url>/index`; the resolver tries the default location when `index_url` is unset. @status:spec/plan

@fact:THE-KEY-DOES-NOT-EXIST-YET-AND-THE-SECTION-IS-STRICT **Neither the key nor the default is built, and the manifest section is strict — so the block above is a parse refusal today, not a working example.** `RegistrySection` carries `deny_unknown_fields` and the fields `name` / `url` / `ref` / `naming` / `auth` / `token_env` / `enabled`; an `index_url` line in a real `vibe.toml` fails to load. Recorded rather than deleted because the requirement stands and only its implementation is missing — but a reader copying this block gets an error, which is why the status marker above says `plan` and not `done`. @status:impl/done

@fact:INDEX-URL-TODAY-IS-AN-ENVIRONMENT-VARIABLE **What locates an index today is one environment variable per registry:** `VIBEVM_INDEX_URL_<REGISTRY>`, read by the resolver when it attaches an index client, and the only source there is. It is deliberately weaker than the manifest field it stands in for — an env var is per-shell and per-run, so an index configured this way travels with neither the project nor the lockfile — and that is precisely why it does not close the requirement above. @status:impl/done

@fact:AN-ABSENT-INDEX-FALLS-BACK-WITHOUT-A-WORD 404 / connect-failure on the index → **silent** fallback to live `ls-remote`: no error message, because the operator never promised an index. This half is built, and it is the `absent` outcome of [`##A-PROBE-HAS-THREE-OUTCOMES-NOT-TWO`](#optional) — the other two outcomes are never silent. @status:impl/done

### 2.3 Source of truth: package repos remain authoritative; index is a hot cache {#truth}

@fact:req-truth `req r1` @status:impl/done

@fact:REPOS-AUTHORITATIVE **Decision.** Package repositories are the **source of truth** for content (manifests, files, tags). The index is a **derived hot cache**, regeneratable from the authoritative package state. @status:impl/done

- @fact:REALITY-WINS This matters because it disambiguates the failure mode: if the index disagrees with reality, **reality wins**. @status:impl/done
- @fact:HASH-VERIFIED-ANYWAY A consumer that resolves a package through the index still verifies `content_hash` against the actually-fetched bytes per [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity). A mismatch between an index-recorded `content_hash` and the actually-fetched one is a hard `IntegrityError`, surfaced with both values and a hint to refresh the index. No silent acceptance. @status:impl/done

@fact:SERVER-MODE-WRINKLE **Server-mode wrinkle.** The `vibe-index serve` mode (§2.5) is described in the prompt as "the only writer; source of truth". This is true *for the index data*. The server holds the canonical RAM copy, persists it to disk on every mutation, and refuses writes from other processes (file lock + mtime checks at startup). But the server's writes are **derived from package-repo state** — the ground truth for "is this version published?" is still the actual git tag in the actual package repo. A divergence (index says `v0.1.0` exists; the package repo's tag was force-deleted) surfaces at consumer install time as an integrity failure on cross-source `content_hash` verification, and the operator has the diagnostic clear: "your index lies; reindex". @status:impl/done

### 2.4 File layout inside the index repo {#layout}

@fact:req-layout `req r1` @status:impl/done

@fact:INDEX-FILE-LAYOUT **Decision.** The index repo's working tree (and equivalently its raw-HTTP-served file tree) carries: @status:impl/done

```
<index-root>/
├── hello.json                                 # the eternal handshake — read FIRST, dispatches to a world
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

@fact:HELLO-JSON **`hello.json`** — the eternal handshake ([PROP-044 §3](../../common/PROP-044-change-native-formats.md#truth)): `{vibe, worlds[], min_client?, notice?, successor?}`, the one document whose keys never change meaning, through which a client of any age learns which worlds this index currently serves, where each lives, and where the handshake itself moved. Its shape is `schemas/hello/e1/hello.jtd.json` and its type is generated like every other wire type ([§2.12](#types)); the index writes it, and [§2.1](#optional) is where a consumer reads it. @status:impl/done

@fact:THE-HANDSHAKE-IS-NOT-AN-ENTRY-OF-THE-MANIFEST **The handshake is deliberately absent from `repomd.json::files`, and the asymmetry is the design.** `repomd.json` is the manifest of **one** world; the handshake stands **above** worlds and dispatches to them, so a world's manifest can no more vouch for it than a chapter can vouch for the table of contents. The consequence to hold: the handshake is the one served file the manifest's `sha256` map does not cover, and what verifies it instead is that it **parses** — an HTTP 200 whose body is not a handshake is a loud refusal naming the broken index, never a quiet fall-through to `repomd.json` ([§2.1](#optional)). @status:impl/done

@fact:THE-WRITERS-OWN-SURFACE-IS-A-WHITELIST **What the writer owns is a stated whitelist, not whatever the directory happens to hold** — four root files (`hello.json`, `repomd.json`, `primary.jsonl`, `primary.jsonl.gz`) and three trees (`by-name/`, `by-cap/`, `by-purl/`), named once in the code so the two readers that ask "is this catalog still the projection of its journal?" — `cargo xtask rebuild --check` and the golden-corpus test — cannot compare different sets. The rest of the directory is not the writer's and is not compared: `README.md` and `.gitignore` are written once by `init`, and the whole of `state/` ([§2.13](#persistence)) is the server's own bookkeeping. A blacklist would have to enumerate the world and would rot the day the directory grew a file nobody listed. @status:impl/done

@fact:REPOMD **`repomd.json`** — the manifest, modelled after RPM's `repomd.xml`: @status:impl/done

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
      "kind": "file",
      "size": 184522,
      "sha256": "<hex>"
    },
    "primary.jsonl.gz": {
      "kind": "file",
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

- @fact:REPOMD-FILES-ARE-SYMMETRICALLY-TAGGED Every entry of `files` carries a `kind` tag — `"file"` or `"directory"` — and a reader dispatches on it rather than on which shape happens to fit. The tag was half-present until 2026-08-14: directories carried it, files did not, and the union was matched by shape, so an entry that lost a field was silently re-read as the *other* kind instead of being refused. A wrong answer that looks like a right one is the one failure re-fetching cannot cure ([PROP-044 §2](../../common/PROP-044-change-native-formats.md#laws)), which is why the asymmetry was broken deliberately rather than tolerated; the break note is `formats/breaks/001.md`. A `file` entry missing its tag is now a parse refusal. @status:impl/done
- @fact:REPOMD-TRUST-POINT `repomd.json` is the **single point of trust**. A consumer fetches it once (with a small ETag round-trip later), then fetches whichever sub-files it actually needs, verifying each against the recorded `sha256`. @status:spec/plan
- @fact:NO-SHIPPED-CONSUMER-VERIFIES-A-SUB-FILE-YET **No shipped consumer does that yet, and saying so is the point of writing it down.** The index client asks `repomd.json` as an existence probe and then fetches the candidate-set file directly: it reads no `sha256` and sends no `ETag` — measured as zero occurrences of either in the client, against 43 elsewhere in the same crate, so the zero is the client's silence and not the instrument's. The comparison the fact above describes exists only as the operator verb `vibe-index verify`, run against a data directory, which is a different party at a different time. @status:impl/done
- @fact:WHAT-IS-NEVERTHELESS-PROTECTED-AND-WHAT-IS-NOT **What that leaves exposed is metadata in transit, not content.** `content_hash` is verified against the actually-fetched bytes at fetch time no matter how the version was chosen ([§2.3](#truth)), so a tampered index can misdirect a consumer toward the wrong version — it cannot make it install bytes nobody checked. A substituted `by-name` file, by contrast, is read as it arrives. Both halves belong in one sentence: an integrity story stated only in its strong half is the kind of claim a reader plans around and a defect hides behind. @status:impl/done
- @fact:repomd-pattern-heritage This pattern (manifest-with-checksums) is what RPM, Deb, and OCI all share, and is what gives us a path to GPG signing without re-architecting. @status:spec/done

@fact:PRIMARY-JSONL **`primary.jsonl`** — newline-delimited JSON, one record per `(group, name, version)`. Lines are sorted by `(group, name, version)` — the PROP-008 §2.2 identity ordering. Each line is one §2.6 entry. JSONL is chosen over JSON-array because: @status:impl/done

- @fact:jsonl-append Append-friendly (a publish hook can append and re-sort + rewrite, or for incremental update merge a sorted insert). @status:spec/done
- @fact:jsonl-streamable Streamable by consumers (line-at-a-time parse; no need to buffer the whole file). @status:spec/done
- @fact:jsonl-grepable `grep`-able (operators can inspect the index manually). @status:spec/done
- @fact:jsonl-diffable Diffable in git (per-line diffs survive re-sorts cleanly). @status:spec/done

@fact:PRIMARY-GZ **`primary.jsonl.gz`** — gzip-compressed equivalent for HTTP-bandwidth-conscious consumers. ~5× compression typical for JSON. Byte-identical content; gzip is deterministic at level 6 with the standard zlib dictionary so the file is reproducible across machines (we pin level and disable `mtime` in the gzip header to keep the SHA-256 stable). Both `primary.jsonl` and `primary.jsonl.gz` ship; consumer chooses based on `Accept-Encoding`. @status:impl/done

@fact:BY-NAME **`by-name/<name>.json`** — the candidate-set file for one bare package
name (PROP-008 §2.8). A single HTTP GET fetches every `(group, name)`
package that shares the short name `<name>`, each with all its versions —
~1–10 KB. This is the path short-name resolution (PROP-008 §2.6) walks:
one GET per registry yields the whole candidate set, so a collision
(PROP-008 §2.7) is detected at once. The directory level keyed on `kind`
before PROP-008; `kind` left package identity, so `<name>` alone is the key. @status:impl/done

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

@fact:BY-NAME-TOMBSTONE `tombstone` — `{reason, superseded_by?}`, carried by the candidate-set file **only when the bare name is buried**, and omitted from the wire otherwise (which is why the live example above does not show it). A name that ever existed answers with the current thing, a forwarding pointer, or a tombstone carrying a reason — never with silence ([PROP-044 §2](../../common/PROP-044-change-native-formats.md#laws)). The consequence that is easy to miss: the file is written **even when the name has no packages left**, because an absent file *is* the silence the law forbids. A buried name therefore looks like this: @status:impl/done

```json
{
  "name": "wal-old",
  "indexed_at": "2026-05-06T12:00:00Z",
  "tombstone": { "reason": "renamed to `wal`", "superseded_by": "org.vibevm/wal" },
  "packages": []
}
```

@fact:BY-CAP **`by-cap/<capability-slug>.jsonl`** — for `vibe install --capability ui:landing-page` style queries (PROP-003 capability-driven resolution). Each line: `{"kind":"feat","name":"welcome-page","version":"0.3.0","capability":"ui:landing-page@0.3.0"}`. `<capability-slug>` is the capability string with `:` and `/` and `@` replaced by `--` (filesystem-safe; reversible). Optional in v0; populated when present. @status:impl/done

@fact:BY-PURL **`by-purl/<purl-slug>.jsonl`** — for "what vibevm packages document this upstream library?" queries (PROP-003 §2.5.6 `describes`). Same shape as `by-cap`; `<purl-slug>` is the PURL with `/` replaced by `--`. @status:impl/done

@fact:SORT-INVARIANTS **Sort invariants.** Every file with multiple entries is sorted deterministically: @status:impl/done

- @fact:sort-primary `primary.jsonl` — sort key `(group, name, version)` with versions in ascending semver order. @status:impl/done
- @fact:sort-by-name `by-name/<name>.json` — `packages` sorted by `group`; each package's `versions` array sorted ascending. @status:impl/done
- @fact:sort-by-cap-purl `by-cap/<slug>.jsonl` and `by-purl/<slug>.jsonl` — sort key `(group, name, version)`. @status:impl/done

@fact:DETERMINISM-WHY Determinism matters because the index repo lives in git: a non-deterministic order would produce churn diffs on every regenerate, defeating the value of git as the transport. @status:spec/done

### 2.5 Two modes: CLI tool and HTTP server {#modes}

@fact:req-modes `req r1` @status:impl/done

@fact:ONE-BINARY-TWO-MODES **Decision.** A single binary, `vibe-index`, ships in two modes selected by subcommand: @status:impl/done

- @fact:MODE-CLI **CLI mode** (default, every subcommand except `serve`) — operates directly on a data directory of index files. Reads on-disk state, mutates, writes back atomically. Suited for: scripted `vibe-index reindex` invocations, manual operator commands, CI pipelines, post-publish hooks. @status:impl/done
- @fact:MODE-SERVER **Server mode** (`vibe-index serve`) — boots an HTTP server. Holds the index in RAM; persists every mutation back to disk. Single-writer (the server) — no other process should mutate the data dir while the server runs (file lock at `<data-dir>/state/server.lock`; broken lock → server refuses to start with a clear error). Suited for: hosted index endpoints, real-time publish-time updates from `vibe registry publish`. @status:impl/done

@fact:one-binary-why **Why one binary, not two.** Same code paths (the in-memory `Index` struct, the persistence layer, the scanner) are shared. Two binaries would force consumers to install both. clap-style subcommand dispatch handles the mode selection. @status:spec/done

@fact:distribution-pointer **Distribution.** The utility lives at `crates/vibe-index/` as a **member of the top-level vibevm workspace** — built, tested and gated by the same `cargo … --workspace` invocations as every other crate. [§6](#distribution) records why the original standalone-workspace decision was reversed and what the reversal bought. @status:impl/done

### 2.6 Index entry shape (the canonical record) {#entry}

@fact:req-entry `req r1` @status:impl/done

@fact:ENTRY-SCHEMA **Decision.** Every `(group, name, version)` entry carries the following fields. This is the shape lines of `primary.jsonl` follow, and the elements each `by-name/<name>.json` candidate's `versions[]` carry. **The authority for that shape is `schemas/index/e1/entry.jtd.json`; this section is its reading aid.** The JTD file is the source of truth, the Rust is generated from it into `crates/vibe-wire/src/generated/` and re-exported through `crates/vibe-index/src/types/entry/`, and `cargo xtask check-codegen` refuses any drift between the two. The record is defined once, in the shared `version_entry` vocabulary, because the candidate-set file and the journal carry it transitively and the schema language has no cross-file reference — `entry.jtd.json` is the root that names it. So the index entry stands in exactly the same arrangement as the wire reports under the root `schemas/`, not a different one. Where this section and the schema disagree, **the schema wins and this section is the defect**. What this section carries that the schema cannot is the *provenance* below — where each field comes from — and that is a copy of nothing. @status:impl/done

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
  "must_understand": [],
  "yanked": false,
  "frozen": false,
  "indexed_at": "2026-05-06T12:00:00Z",
  "indexed_by": "vibe-index 0.1.0"
}
```

@fact:SLOTS-ARE-OMITTED-WHEN-EMPTY The three slots — `must_understand`, `yanked`, `frozen` — appear above in their *significant* form, but all three are **omitted from the wire when empty** (an empty list; `false`). A live record for an un-yanked snapshot package therefore carries none of them, and the same holds for `tombstone` on the candidate-set file ([§2.4](#layout)). Reading this example as "these keys are always present" is the one mistake it invites. @status:impl/done

@fact:field-provenance-lead **Field provenance.** @status:impl/done

- @fact:PROV-MANIFEST-FIELDS `kind` / `group` / `name` / `version` / `license` / `authors` / `description` / `homepage` / `keywords` / `describes` / `compatibility` / `provides` / `requires` / `requires_any` / `obsoletes` / `conflicts` / `features` / `i18n` / `boot_snippet.source` / `boot_snippet.category` — read directly from `vibe.toml` at the tagged ref. (M1.18's loading model, PROP-009, retired the author-chosen `boot_snippet.filename`; a snippet is now its `source` path plus an ordering `category`.) @status:impl/done
- @fact:PROV-GROUP `group` — the mandatory reverse-FQDN qualifier from `[package].group` (PROP-008 §2.1). With `name` it forms the package identity; `kind` is metadata and identifies nothing (PROP-008 §2.2 / §2.3). @status:impl/done
- @fact:PROV-WORKSPACE-ORIGIN `workspace_origin` — the `[origin]` provenance marker (PROP-007 §2.8, PROP-008 §2.8), present only on a copy `vibe workspace publish` generated from a workspace member; absent (`null`) for a standalone publish. @status:impl/done
- @fact:PROV-SUBSKILLS `subskills` — collected by walking `<package-root>/subskills/<path>/vibe-subskill.toml` at the tagged ref; each entry: `{path, delivery, describes, description, channels}`. Same fields the lockfile records. @status:impl/done
- @fact:PROV-CONTENT-HASH `content_hash` — computed by the same algorithm `vibe-registry::compute_content_hash` uses (sha256 over deterministically-ordered file bytes). Index uses **the same hash** as the lockfile, so cross-checks are byte-equal. @status:impl/done
- @fact:PROV-SOURCE-URL `source_url` — the canonical org URL (§2.4 of [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md#registry-model)) composed with the package repo name. Mirror URLs do not appear here (same invariant as the lockfile). @status:impl/done
- @fact:PROV-SOURCE-REF `source_ref` — `v<version>` by default (the tag). @status:impl/done
- @fact:PROV-RESOLVED-COMMIT `resolved_commit` — the commit SHA the tag pointed to at index time. Pinning to commit lets us notice tag-rewrites later. @status:impl/done
- @fact:PROV-REGISTRY `registry` — local alias from `[[registry]].name`. @status:impl/done
- @fact:PROV-FILES-COUNT `files_count` — informational, useful for sanity-checking integrity diffs. @status:impl/done
- @fact:PROV-INDEXED-AT `indexed_at` / `indexed_by` — provenance for the index entry itself (when, by which tool version). @status:impl/done
- @fact:PROV-MUST-UNDERSTAND `must_understand` — the **reader** capabilities a consumer must have to act on this record ([PROP-044 §4.5](../../common/PROP-044-change-native-formats.md#machinery)) — a different vocabulary from the package's own `provides.capabilities`. Written by the projector; never read from `vibe.toml`. A reader that does not understand every string in the list **skips this record and says so** — what «says so» is, exactly, is [§2.19](#unavailable); unknown fields *outside* the list are ignored as before. This is the exact inversion of additive-only: the writer declares what is mandatory, per record, addressably and revocably, instead of the schema promising ignorability forever. @status:impl/done
- @fact:PROV-YANKED `yanked` — the version is withdrawn. Journal-borne, not authored: frozen content cannot withdraw itself, so the fact arrives from the registry's facts journal and is projected here ([PROP-044 §2a](../../common/PROP-044-change-native-formats.md#laws)). @status:impl/done
- @fact:PROV-FROZEN `frozen` — projected from the package manifest's `[package].frozen`, never a registry's opinion. Absence = `false` = **snapshot**: content may flow under the same version string, and a hash mismatch is *news*. `true` is the author's one-way freeze: bytes immutable, and a hash mismatch is an *alarm*. The flag lives inside the hashed content, so a version self-describes even offline and every registry serving those bytes necessarily agrees — a registry only *observes* the freeze in its journal and projects it ([PROP-044 §2a](../../common/PROP-044-change-native-formats.md#laws); terminology §2b — `snapshot` and `frozen` are the two states of one boolean axis, with no third). @status:impl/done

@fact:FORWARD-COMPAT **Forward compatibility.** `schema_version: 1` is recorded at file scope (in `repomd.json`) and at entry scope. Entries carrying fields a reader does not know coexist with it: unknown fields are tolerated, and known-but-absent ones default. **This sentence was false for as long as it stood.** Fifteen catalog aggregates carried `deny_unknown_fields`, so a reader meeting one unknown key refused the whole file — the opposite of what the fact promised — and the contradiction survived because nobody could remove the attribute safely. That was not caution: while every mutation read the catalog to rewrite it, tolerance would have SILENTLY DELETED the tolerated fields on the next write, which is worse than refusing. Phase 3 removed the condition rather than the symptom — a mutation is now an append to the journal plus a reprojection, so nothing read is ever written back, there is nothing to lose, and the strictness could finally go ([PROP-044 §4.4](../../common/PROP-044-change-native-formats.md#machinery)). What tolerance still does NOT mean is acting on a record one does not understand; that refusal moved to the per-record capability set (`##NEVER-SILENT-SCHEMA`). @status:impl/done

### 2.7 Identity and trust {#trust}

@fact:req-trust `req r1` @status:impl/done

@fact:HASH-JOIN-KEY **Decision.** The **digest** of `content_hash` is the join key between the index and the lockfile, and it joins only **at a stated recipe** ([PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity)). A consumer that fetches `flow:wal@0.1.0` via the index records the digest a no-index fetch would have produced — for every tree except the ones recipe 1 exists to disambiguate, where the two recipes deliberately disagree and that disagreement is the point. **Index entries are advisory; the bytes are authoritative.** @status:impl/done

@fact:HASH-LABELS-DIFFER-BY-DESIGN-TODAY The index and the lockfile currently stand at **different** recipes: the index emits `sha256-tree/1:`, the lockfile keeps the bare `sha256:` until its own format moves. So their strings are deliberately unequal, and nothing compares them as strings — the index client does not read the field at all. Written down because the asymmetry looks like a defect to a cold reader and is instead the thing that keeps a live lockfile from being rewritten by a change it did not ask for. @status:impl/done

@fact:TWO-INTEGRITY-LAYERS The `repomd.json::files[*].sha256` covers integrity of the index files themselves. The per-entry `content_hash` covers integrity of the package content, and carries its recipe so a check compares like with like. The two are independent: a tampered index file fails its file-hash check; a tampered package repo (force-pushed tag) fails its content-hash check at fetch time. @status:impl/done

@fact:trust-oos **Out of scope for v0:** GPG-signed `repomd.json.asc`, Merkle-log audit trail (Go sumdb-style). [§9](#open) tracks both. @status:spec/done

### 2.8 Reindexation: full and incremental {#reindex}

@fact:req-reindex `req r1` @status:impl/done

@fact:TWO-REINDEX-MODES **Decision.** Two regeneration modes. Both are available via the CLI (`vibe-index reindex`); the HTTP trigger is specified and unbuilt ([§2.10](#http)): @status:impl/done

@fact:FULL-REINDEX **Full reindex.** Walk every package repo in the org; for each repo, list tags; for each `v<semver>` tag, read `vibe.toml` and `subskills/**/vibe-subskill.toml` at that ref; compute `content_hash`; assemble §2.6 entry. Replace the in-memory index wholesale, then atomic-write the on-disk files. @status:impl/done

@fact:walk-sources-lead Sources for the walk: @status:impl/done

- @fact:SRC-FROM-CLONES `--from-clones <org-dir>` — local directory of bare/regular clones. Authoritative for the operator who already maintains a vendor mirror; offline-capable. Default path for owners who run `vibe-index reindex` on a cron against their own server's clone tree. @status:impl/done
- @fact:SRC-FROM-GITHUB `--from-github <org>` — REST API walk against `api.github.com`. Requires a token (read-only `repo`-scope). Used by hosted index instances that don't keep a clone tree. @status:impl/done
- @fact:SRC-FROM-GITVERSE `--from-gitverse <org>` — equivalent against GitVerse's API once it exposes org-scoped repo enumeration; today returns "not implemented" (mirrors the publish-stub pattern from `vibe-publish/src/gitverse.rs`). @status:impl/done

@fact:INCREMENTAL-REINDEX **Incremental reindex.** Detect what changed since the last run and update only the affected entries. @status:impl/done

- @fact:INC-CLONES For `--from-clones`: compare each repo's `git rev-parse HEAD` and `git tag -l` output to a checkpoint stored at `<data-dir>/state/checkpoint.json`. Repos with new tags or a new HEAD commit on `main` (in case a manifest changed without a tag) are re-walked; others skip. @status:impl/done
- @fact:INC-GITHUB For `--from-github`: use the `If-Modified-Since` / ETag headers on `/orgs/<org>/repos` and `/repos/<org>/<name>/tags` to skip unchanged repos. @status:impl/done

@fact:CADENCE-TARGET Incremental is the default cadence target (one run per minute on an active org); full is the bootstrap path and the "trust nothing" recovery option. @status:impl/done

#### 2.8.1 The organisation image, and what keeps caching it honest {#cache-org}

@fact:CACHE-ORG-THE-COST-IS-THE-SMALL-HALF **Enumerating the organisation on
every operation is a cost, and the cost is the small half of the problem.** For
local clones it is a directory walk and cheap. For a git host it is a paged API
walk on every single operation. But the reason to think about it is not speed —
it is that the picture is stale the moment it is taken. @status:impl/done

@fact:CACHE-ORG-THE-AXIS-IS-NOT-HOW-MANY-WORKERS **The premise «between
operations nobody can change the organisation» is already false, and not because
of sibling workers** *(owner accepted this correction, 2026-08-06)*. A developer
publishing a package creates a repository and pushes a tag **straight to the git
host**, never passing through the index service. The image goes stale with one
worker exactly as with ten. The real axis is whether every change goes through
the index — and today none has to. @status:impl/done

@fact:CACHE-ORG-IS-ON-BY-DEFAULT **`--cache-org` is on by default** *(owner
ruling, 2026-08-06)*, with an explicit negative form to turn it off. The name
describes the mechanism rather than the assumption, which is deliberate: the
assumption the default must NOT make is the one the fact above rejects. @status:impl/done

@fact:CACHE-ORG-THE-FRESHNESS-CHECK-IS-A-CONDITION-NOT-AN-IMPROVEMENT **The
cheap freshness check is what makes that default honest, and it is therefore not
an enhancement.** Git hosts answer «has anything changed» with a conditional
request that costs almost nothing and needs no walk; the cached image carries the
validator its enumeration came with, and every run offers it back. Without this
step, «on by default» would silently mean the very assumption the owner
rejected. With it, the image is cached and never treated as truth without asking.
@status:impl/done

@fact:CACHE-ORG-CANNOT-CHECK-MEANS-ENUMERATE **A host that gives no validator
makes the index enumerate, never trust.** The absence of an answer is not an
answer. This is the direction the whole design has to fail in, because the
opposite default — no validator, assume fresh — is indistinguishable from a
working cache right up until a package cannot be found. @status:impl/done

@fact:CACHE-ORG-BELONGS-TO-ITS-ORGANISATION **An image is keyed to the
organisation and the API base it came from**, and a cache taken for one is never
used for another. Cheap to state, expensive to omit: the failure would be an
index confidently serving another organisation's picture. @status:impl/done

@fact:CACHE-ORG-HIT-AND-MISS-ARE-VISIBLE **Hit and miss are reported, in both
renderings.** An operator must be able to tell «this came from the cache» from
«this was enumerated», because a cache that silently serves stale data is
indistinguishable from one that works — which is the disease this whole section
is written against. @status:impl/done

@fact:RESCAN-ORG-IS-UNCONDITIONAL **`rescan-org` is its own verb and it is
unconditional** *(owner ruling, 2026-08-06)*. It enumerates regardless of the
cache and regardless of any validator, and refreshes the image. It exists
because a missed change is invisible from the inside: no freshness mechanism
promises completeness, and a full walk does. Webhooks ([§2.16](#webhooks))
reduce how often it is needed; they never remove the need. @status:impl/done

@fact:CACHE-ORG-APPLIES-WHERE-ENUMERATION-IS-EXPENSIVE The cache and its
freshness check govern the host-API path. For a local-clone walk the enumeration
is a directory read, and wrapping a validator around it would buy nothing and
add a way to be wrong. @status:impl/done

@fact:CACHE-ORG-FIRST-RUN-IS-UNCHANGED **With no image on disk the behaviour is
exactly what it was** — enumerate, build, write the image — with no warning and
no error, and turning the flag off leaves no image and no report field. A
default that changes the first run's behaviour is a default that has to be
explained; this one does not. @status:impl/done

@fact:triggers-lead **Triggers.** @status:impl/done

- @fact:TRIGGER-CLI **CLI:** `vibe-index reindex <data-dir> --from-clones <org-dir>` — direct invocation. @status:impl/done
- @fact:TRIGGER-HTTP **HTTP:** `POST /v1/admin/reindex` body `{"mode":"full"|"incremental","source":"clones"|"github","args":{...}}`. Auth required. Returns a job id; status pollable at `GET /v1/admin/reindex/<job-id>` (in v1 — v0 just blocks until done). **Specified, not built** — the router carries no such route ([§2.10](#http) `##THE-ADMIN-SURFACE-IS-ONE-ROUTE`); until it does, the operator's trigger is the CLI verb, reached by cron or by whatever the host's own hook mechanism can invoke. @status:spec/plan
- @fact:TRIGGER-GIT-HOOK **git hook (server-side, on the index repo's host):** owner installs a `post-receive` hook on the org's hosted git that posts to `POST /v1/admin/reindex` whenever a package repo gets a push to a `v*` tag. Documented in §11; not shipped as part of the binary. @status:spec/done
- @fact:TRIGGER-CRON **cron:** `crontab` line invokes `vibe-index reindex --incremental` every N minutes. Documented; not enforced. @status:spec/done

### 2.9 Single-writer server mode {#server-mode}

@fact:req-server-mode `req r1` @status:impl/done

@fact:SINGLE-WRITER **Decision.** The HTTP server is the **only writer** when running. It locks the data directory via a PID file (`<data-dir>/state/server.lock`) at startup; refuses to start if the lock is held by another live process; refuses CLI mutations against the same data directory by detecting the lock from CLI side (CLI-mode `add` / `remove` / `reindex` errors with "server is running on this data dir; use the HTTP API"). @status:impl/done

@fact:state-model-lead In-memory state model: @status:impl/done

```text
Arc<RwLock<Index>>
   │
   ├─ readers (search, list, get)        — RwLock::read()
   └─ writers (add, remove, reindex)     — RwLock::write()
```

@fact:write-protocol-lead On every successful write, the server: @status:impl/done

1. @fact:WRITE-MEMORY Updates the in-memory `Index`. @status:impl/done
2. @fact:WRITE-RESERIALISE Re-serialises the affected files (`primary.jsonl`, the touched `by-name/<name>.json`, optionally `by-cap` / `by-purl`). @status:impl/done
3. @fact:WRITE-ATOMIC Writes each file atomically: `tmp` next to the destination, `fsync`, `rename`. @status:impl/done
4. @fact:WRITE-REPOMD-LAST Updates `repomd.json` last (the manifest is replaced as a whole; readers that hold the old `repomd.json` see a consistent old view; readers that pick up the new `repomd.json` see consistent new files). @status:impl/done
5. @fact:WRITE-AUTO-COMMIT Optionally (if `--auto-commit-push` flag is on): `git add -A && git commit -m "auto: index update" && git push origin <branch>` against the data directory if it is a git working tree. v0 ships without this — operator runs commit/push manually or via separate cron. v1 adds `--auto-commit-push`. @status:spec/done

@fact:THE-WRITER-TAKES-ITS-CLOCK-AS-AN-INPUT **The writer never calls `now()`.** Every timestamp a write stamps — the manifest's `generated_at` and each candidate-set file's `indexed_at` — arrives as an argument: the CLI passes the moment its command began, the server passes the moment of the mutation event, and the index and entry modules contain no clock call at all (a panel step refuses one). One state therefore produces one byte sequence, which is what makes "rebuild and compare" a real verification, an empty diff a real no-op, and a wire-diff a quantitative measure of a break rather than a wall of timestamp churn ([PROP-044 §4.3](../../common/PROP-044-change-native-formats.md#machinery)). Determinism here is an instrument, not tidiness: without it every later recoverability check measures the clock instead of the content. @status:impl/done

@fact:THE-WRITER-KEEPS-THE-VERSION-IT-READ **The writer stamps its own schema version only into an artifact it creates from scratch.** A catalog it *read* keeps the version that catalog carried: the value is state, not a constant of whichever binary happens to be running. Otherwise a catalog written by a later version and opened by an older binary for any mutation would silently shed its own marker and start claiming ours — a file that still looks consistent while asserting something untrue about itself, which is the failure re-fetching cannot cure ([PROP-044 §2, law 1](../../common/PROP-044-change-native-formats.md#laws)). What the writer does when the version it read is one it cannot serve is a separate question and not answered here; this fact only forbids the silent overwrite. @status:impl/done

@fact:A-PROJECTION-READS-NOTHING-SO-ITS-OWN-VERSION-IS-TRUE **The clause the journal adds: a projection has no version to keep.** Once a mutation is an append to the journal followed by a reprojection, the writer no longer reads a catalog at all — it builds one from the facts. There is therefore nothing to preserve, and stamping this build's constant asserts something true about the artifact just written, rather than overwriting a claim some other writer made. The rule above is unchanged and still binds every path that *does* read a catalog before rewriting it; what changed is how many such paths exist. The protection it was reaching for did not disappear with them — it moved up a floor, to the journal's own epoch and to each record's `must_understand` set, where a build meeting facts from a newer world refuses them by name instead of quietly reading a subset and re-labelling the result ([PROP-044 §4.5](../../common/PROP-044-change-native-formats.md#machinery)). Read the two together and the invariant is one thing said twice: a file never claims an authorship it does not have. @status:impl/done

@fact:CONCURRENCY **Concurrency.** axum + tokio. Reads do not block reads. Writes block reads (RwLock) for the duration of the in-memory mutation; disk I/O happens after lock release for any path it can (e.g. `primary.jsonl` rewrites are queued and serialised by a single dedicated writer task). For the request rates we target (max ~10 writes/min during a publish burst, ~1000 reads/min during a CI install storm), a coarse RwLock is sufficient. @status:impl/done

@fact:PROCESS-MODEL **Process model.** Single process. No replication. An operator who needs HA runs the server behind a load balancer with N replicas and a shared filesystem — but that's outside v0. v0 expects one process per data directory. @status:impl/done

### 2.10 HTTP API surface {#http}

@fact:req-http `req r1` @status:impl/done

@fact:HTTP-API **Decision.** REST API, JSON over HTTP. CORS open on read endpoints (so a future web UI can hit it from a browser). Routes: @status:impl/done

```
GET    /healthz                                   # liveness
GET    /readyz                                    # readiness (index loaded, no in-flight reindex)

# Static index files (raw — same shape as the on-disk files; mirror-friendly).
# The handshake leads the block because it leads the client (§2.1).
GET    /v1/index/hello.json
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
POST   /v1/admin/reindex                          # body: { mode, source, args } — SPECIFIED, NOT BUILT
GET    /v1/admin/status                           # uptime, last reindex, pkg count, server version

# Observability
GET    /metrics                                   # Prometheus text format
```

@fact:THE-ADMIN-SURFACE-IS-ONE-ROUTE **The admin surface the server actually builds is one route — `GET /v1/admin/status`.** The reindex trigger above is specified and unbuilt: the router registers **16 paths** and none of them is it, the handler module holds `status` alone, and the code's own note («reindex POST lands in slice 6») describes a slice that closed without it. The requirement is not withdrawn here — code conforms to the spec and not the reverse — but three places in this document asserted it as shipped, so all three now say `plan` and the fork (build it, or retire it in favour of the CLI verb) is `BACKLOG.md` B-085. It is not a paper cut: [§11](#wire-up)'s documented `post-receive` hook posts to exactly this route, so an operator following that recipe wires a hook to nothing. @status:impl/done

@fact:HTTP-AUTH **Authentication.** Bearer tokens via `Authorization: Bearer <token>`. Tokens are read from `<data-dir>/state/admin.tokens` (one token per line; comment lines start with `#`). Read endpoints accept missing/invalid tokens silently. Write endpoints require a valid token; mismatch → 401 with a generic message ("authentication required"; do not echo the supplied token nor say which valid prefix it matched). Tokens never appear in logs (logging redacts the `Authorization` header). @status:impl/done

@fact:HTTP-LOCKDOWN **Per-host lockdown.** By default the server binds to `127.0.0.1:8412` — local-only. Operators expose externally by setting `--bind 0.0.0.0:8412` and putting it behind a reverse proxy with TLS. v0 does not ship TLS termination; this is the reverse proxy's job. (Same posture as `cargo`'s sparse index protocol: the upstream is HTTP — TLS is for the CDN / proxy in front.) @status:impl/done

@fact:HTTP-ERRORS **Errors.** Application/json error shape, taken from RFC 7807 Problem Details (lightweight subset) — four members always, and one extension member when the error is a quarantine refusal ([§2.19](#unavailable)): @status:impl/done

```json
{ "type": "vibe-index/error/integrity-mismatch", "title": "content_hash mismatch", "status": 409, "detail": "…" }
```

@fact:THE-BODY-CARRIES-NO-INSTANCE-MEMBER **`instance` is not emitted, and the subset is the four members above plus `unavailable`.** RFC 7807 makes every member optional, so omitting `instance` is conformance rather than a gap — but naming a member the body does not carry teaches a client to look for it. What the body does carry beyond the four is the refusal row, as an extension member, which is the mechanism that RFC provides for precisely this and the reason the status can stay `404` while the answer stops being «not found». @status:impl/done

### 2.11 CLI surface {#cli}

@fact:req-cli `req r1` @status:impl/done

@fact:CLI-SURFACE **Decision.** `vibe-index [--log-level LEVEL] <subcommand> <data-dir> [args]`. The data directory is a **required positional** on every verb — the one argument no invocation can omit, so it is not dressed as an option. One global flag stands above the verbs: `--log-level off|error|warn|info|debug|trace`. @status:impl/done

```
# Lifecycle
vibe-index init <data-dir> --registry NAME --registry-url URL [--naming fqdn|kind-name|name|kind/name] [--force]
vibe-index dump <data-dir> [--format jsonl|json]
vibe-index verify <data-dir> [--json]                # recompute file hashes, check repomd

# Reindex
vibe-index reindex <data-dir> --from-clones <org-dir>                  [--full | --incremental] [--json]
vibe-index reindex <data-dir> --from-github <org> [--token-file FILE] [--api-base URL] [--clone-cache DIR]
                                                    [--cache-org | --no-cache-org] [--full | --incremental] [--json]
vibe-index reindex <data-dir> --from-gitverse <org>                    # emits stub-not-implemented today
vibe-index rescan-org <data-dir> --from-github <org> [--token-file FILE] [--api-base URL] [--clone-cache DIR] [--json]

# Read
vibe-index get <data-dir> <group> <name> [--version V] [--json]
vibe-index list <data-dir> [--kind K] [--limit N] [--offset M] [--json]
vibe-index search <data-dir> <query> [--kind K] [--limit N] [--json]
vibe-index capabilities <data-dir> <capability> [--json]
vibe-index purls <data-dir> <purl> [--json]
vibe-index outdated <data-dir> [--lockfile PATH] [--json]        # given a vibe.lock, print upgrade candidates

# Write (CLI-mode; refused if server is holding the lock)
vibe-index add <data-dir> --manifest <package.toml-path> --repo-url URL [--ref REF --commit SHA]
vibe-index remove <data-dir> <group> <name> [--version V]

# Server
vibe-index serve <data-dir> [--bind ADDR] [--auth-tokens-file FILE] [--read-only] [--auto-commit-push]
                            [--rate-limit-per-token N] [--rate-limit-per-ip N]
vibe-index stop <data-dir>                                       # graceful shutdown via lock-file PID
```

@fact:WITHDRAWAL-IS-TWO-OPERATIONS-AND-NEITHER-OF-THEM-IS-REMOVE **Decision (owner, 2026-08-19).** Taking a package out of circulation is **three distinct operations over two different entities**, and the surface must offer all three rather than making one stand in for the others: @status:spec/plan

| operation | entity | what remains | state today |
|---|---|---|---|
| @fact:OP-REMOVE full deletion — `remove` @status:impl/done | a version, or every version of a name @status:impl/done | **nothing.** The package is indistinguishable from one that never existed @status:impl/done | **built** — the verb emits its journal fact and the projector applies it @status:impl/done |
| @fact:OP-YANK withdraw one version — a `yank` verb @status:spec/plan | one version @status:spec/plan | the record, carrying `yanked` on the wire: a build that already pinned this version keeps working, a fresh resolution passes over it @status:spec/plan | **all but the verb.** The journal fact exists, the projector already applies it by setting the flag, and the field already ships — nothing emits the fact @status:spec/plan |
| @fact:OP-RETIRE retire a name — the `bury` verb @status:spec/plan | a bare name @status:spec/plan | a tombstone: the reason, and a successor to redirect to ([§2.4](#layout)) @status:spec/plan | **all but the verb**, as of the `buried` fact landing. The journal carries the fact, and the projector is the first arm that PRODUCES a tombstone from it rather than refusing — nothing emits the fact yet @status:spec/plan |

- @fact:WHY-THREE-AND-NOT-ONE **Why three verbs and not one with flags.** They differ in what a reader is owed afterwards, which is the only thing a caller actually chooses between: deletion owes silence, yanking owes "still here, do not pick it", retiring owes "gone, and here is where to look instead". A single verb with a mode would let the wrong one be selected by a typo — and two of the three outcomes are not reversible by re-running the command. @status:spec/plan
- @fact:YANK-IS-A-VERB-AWAY **The measured state, so implementation does not rediscover it.** Both remaining operations are now one verb away from working, and for the same reason: the journal carries the fact and the projector applies it, but nothing emits it. Yank sets `yanked = true` and the wire omits the flag when false. Retirement inserts a tombstone and drops the name's packages across every group — the carrier it needed was built with the `buried` fact, which also retired the `renamed` arm rather than reusing it. **This sentence recorded the opposite until the fact landed** («retirement is genuinely unbuilt»; the nearest existing fact, a rename, refused outright as an unbuilt carrier), and it is corrected rather than deleted so the two states of this contract stay legible to a reader arriving from either side. @status:spec/plan
- @fact:A-TOMBSTONE-THAT-IS-NOT-A-JOURNAL-FACT-ERASES-ITSELF **A latent mine, measured while recording this, and it decides the shape of the work.** The tombstone carrier is populated **only** by reading a catalog off disk; the projection built from the journal never sets it. Since the journal phase there is no read-then-write path — a mutation builds its state from the facts and writes that out — so a tombstone placed on disk by anything other than a journal fact would be **erased by the next mutation**, silently and with no failure anywhere. Nothing could reach this while nothing produced a tombstone at all, which is exactly what made it worth writing down before a producer existed: the retirement verb is not «write the field», it is «add the fact», and an implementation that takes the shorter route passes its own tests and loses the tombstone on the first unrelated publish. **The producer built to this rule is the `buried` fact's projector arm**, and the rule is now guarded rather than remembered — a test that buries a name, publishes something unrelated after it, and asserts the stone still stands, proved red by neutering the producer before it was believed green. @status:impl/done
- @fact:A-RENAME-IS-A-RETIREMENT-THAT-NAMES-ITS-SUCCESSOR **Decision (owner, 2026-08-19): renaming gets no carrier of its own — the tombstone already is one, and the journal gets ONE retirement fact carrying `reason` plus an optional successor.** The `renamed` arm of the event vocabulary is retired in the same act. @status:spec/plan
- @fact:WHY-THE-TOMBSTONE-ALREADY-IS-THE-RENAME-CARRIER **Why not a second thing.** This document's own worked example of a tombstone *is* a rename — `{reason: "renamed to …", superseded_by: "org.vibevm/wal"}` ([§2.4](#layout)) — and `superseded_by` is precisely the "go here instead" pointer a rename needs. More binding than the example: the standing naming law says **a rename is a NEW IDENTITY**, versions never transfer. A first-class rename relation would assert continuity between the old coordinate and the new one, which is the thing that law forbids; "the old name is closed, and here is where to look" is the only model consistent with it. And one question — *where did this package go?* — must have one place to look, or the two places eventually disagree. @status:spec/plan
- @fact:WHY-ONE-JOURNAL-FACT-AND-NOT-TWO **Why the journal collapses them too, against this project's usual habit of keeping distinctions the projection folds.** The deciding fact is measured, not aesthetic: the existing `renamed` arm carries `from` and `to` and **no reason**, while a tombstone requires one. So keeping it forces a choice between synthesising prose into a required field and adding a reason to it — after which the two facts differ only in whether the successor is optional, which is one thing spelled twice. The usual argument for keeping them apart (a rename is a *stronger* claim than a retirement-with-pointer) does not survive the naming law: under it, «renamed A→B» asserts nothing beyond «A is closed; B is where to look». The distinction the vocabulary would preserve is one this project has already decided not to make. @status:spec/plan
- @fact:THE-VOCABULARY-CHANGE-IS-DECLARED-AND-ITS-MOMENT-IS-NOW **What it costs, and why now.** This edits the **truth layer's** vocabulary, which is heavier than a catalog change and is done as a declared break with a note, never as a side effect. It was nearly free at the moment it was made and will not stay so for the next such change: nothing emitted `renamed` — only tests constructed it, and the projector refused it by design — **no rename had ever been recorded anywhere in the tree, and that was measured beside a control rather than assumed** (the same search that found no `"kind":"renamed"` did find `"kind":"yanked"` in the golden corpus), and there is no external consumer of the journal. The reverse direction is not lost, only relocated: a reader holding the NEW name learns its old one from the journal, which keeps the retirement fact and its successor forever. That is the right home — the catalog answers *where do I go*, the journal answers *what happened*. @status:spec/plan
- @fact:THE-STALE-SENTENCE-THIS-CREATES **A consequence carried out with the change, named here before it was made so it would not be left lying — and discharged.** [§2.18](#channels) listed `Renamed` among the arms the projector refuses because their carriers are unbuilt; `renamed` left the vocabulary and retirement gained a projected carrier in the same commit, so the sentence stopped being true in both halves at once and was corrected there. **What this record could not name, and the landing had to find:** the same commit falsifies three more statements of present state in this section — the `state today` column above, `##YANK-IS-A-VERB-AWAY`'s «retirement is genuinely unbuilt», and `##A-TOMBSTONE-THAT-IS-NOT-A-JOURNAL-FACT-ERASES-ITSELF`'s «nothing produces a tombstone at all». A contract that predicts one stale sentence and carries four is the argument for measuring the perimeter by file rather than by naming what one remembers ([`harvest/renamed-perimeter.md`](../../../campaigns/packages-2026-09/harvest/renamed-perimeter.md)). @status:impl/done
- @fact:THE-RETIREMENT-VERB-NEEDS-A-NAME <status stage="spec" state="void">Retired 2026-08-19, hours after it was written, when the owner named the verb. It recorded that `yank` had a precedent to borrow and retirement did not, and left the name to the owner. Heir: [`##THE-RETIREMENT-VERB-IS-BURY`](#cli). This line stays so its name is never reused and inbound links do not break.</status> @status:spec/void
- @fact:A-PUBLISH-UNDER-A-BURIED-NAME-RE-OPENS-IT **Decision, taken while building the fact and recorded here because a future reader will re-open it.** A `Published` fact for a name that carries a tombstone **clears the tombstone**; the name lives again and the projection carries no stone beside its packages. *Why:* [§2.4](#layout) says the candidate-set file carries a tombstone «only when the bare name is buried», and its worked example shows an empty package list beside it — so a file holding packages AND a stone is a shape this contract never describes, and a reader would have to consult something else to tell «gone» from «here». The fold has no veto: it answers with the state as of the last fact, and refusing a publish would make the projection a policy engine rather than a projection. *Considered and rejected:* keeping the stone as history (it would make the two states of a candidate file overlap, and history is the journal's job — the burial is recorded there forever either way); suppressing the stone at write time when packages exist (that is a rendering trick over untruthful state, and the state is what `rebuild --check` compares). *Revisit when:* an operator needs «this name is closed» to outlive a re-publication — that is a different claim from a tombstone and would need its own carrier, not a change to this one. @status:impl/done
- @fact:THE-RETIREMENT-VERB-IS-BURY **The verbs are `yank` and `bury`** (owner, 2026-08-19). `yank` borrows the ecosystem precedent it already has. `bury` is not invented for the occasion: this contract **already describes the state in that word** — the tombstone is «carried by the candidate-set file only when the bare name is **buried**», and the worked example is introduced as «a **buried** name therefore looks like this» ([§2.4](#layout)). The command and the state it produces therefore speak one word instead of two, which is the property the neighbours' candidates (deprecate / retract / relocate) each failed in a different way. @status:spec/plan

@fact:MACHINE-OUTPUT-IS-PER-VERB-NOT-UNIVERSAL **`--json` is a property of nine verbs, not of the binary.** Every verb that ANSWERS a question carries it — `get`, `list`, `search`, `capabilities`, `purls`, `outdated`, `verify`, `reindex`, `rescan-org`; the six that perform an action and report only success (`init`, `add`, `remove`, `dump`, `serve`, `stop`) do not. `dump` is the instructive exception: it is machine output already, so a `--json` switch on it would be a second spelling of `--format json`. Saying «all subcommands» would send a script author looking for a flag six verbs do not have. @status:impl/done

@fact:THE-LOG-DIAL-AND-THE-VARIABLE-ARE-ONE-LEVER `--log-level` is global (it may be written before or after the subcommand) and it folds into the one lever `VIBE_LOG`, which the subscriber reads exactly once at start-up: passing the flag SETS that variable, so the process environment always explains the output an operator is looking at. The flag speaks a closed set of six values while the variable keeps the full directive language — one thing with a coarse dial and a fine one, never two spellings of the same power. @status:impl/done

@fact:THE-SUBSCRIBER-IS-INSTALLED-UNCONDITIONALLY-AT-WARN **The tracing subscriber is installed on every invocation, at `warn` by default, and that is a decision rather than a default nobody chose.** It is the binary's job and not the library's; there is no `RUST_LOG` fallback and no second lever. The reason is the refusal path: a version this build cannot act on is reported at WARN when a catalog loads ([§2.19](#unavailable)), and a publication failure is reported at WARN when the server pushes ([§2.17](#auto-publish)). Both are things an operator must be able to see on **any** subcommand — so a subscriber installed only under some flag would make observability an accidental property of which verb happened to be running, which is how a message that exists is never read. The ordering that makes the flag honest is part of the same decision: the fold writes `VIBE_LOG` at the very top of `main`, after the parse and before the subscriber, so `--help` and parse errors still answer before any log and the variable is never left describing output it no longer governs. @status:impl/done

@fact:HELP-SMOKE **Help-text smoke** lives under `crates/vibe-index/tests/help_smoke.rs`, mirroring `every_subcommand_renders_help` in `vibe-cli`. @status:impl/done

### 2.12 Data structures {#types}

@fact:req-types `req r1` @status:impl/done

@fact:RUST-TYPES **Decision.** The catalog's wire types are **generated from the schemas of [§2.6](#entry) and re-exported**, never written by hand: the definitions live in `vibe_wire::generated` beside the JTD they come from, `crates/vibe-index/src/types/` re-exports them so every `vibe_index::types::*` path keeps its meaning, and `cargo xtask check-codegen` is the gate that refuses a drift between schema and type. `VersionEntry` comes from the shared `version_entry` vocabulary; `NameEntry` / `PackageEntry` / `Tombstone` from `schemas/index/e1/by_name.jtd.json`; `BindingSite` from `by_purl`. @status:impl/done

@fact:TWO-SHAPES-STAY-HAND-WRITTEN-AND-SAY-WHY Two shapes stay hand-written, and each says why. `Repomd` / `RepomdFileEntry` — its `size` is a `u64` where the schema language reaches only `u32` (an open owner fork, `BACKLOG.md` B-091 — filed as B-056 and renumbered 2026-08-19, because that coordinate carried a closed row too and a reader following it landed there), and its `files` union is tagged by this document's own law ([§2.4](#layout)). And `Index` below, which is not a wire type at all: it is the server's in-RAM state, no single document ever carries it, and it holds two members the catalog deliberately never serialises — the reader's quarantine record and the per-name tombstones the writer projects back onto the candidate-set files: @status:impl/done

```rust
pub struct Index {
    pub schema_version: u32,
    pub registry: String,
    pub registry_url: String,
    pub naming: NamingConvention,
    pub generator: String,
    pub generated_at: DateTime<Utc>,

    pub by_pkgref: BTreeMap<PkgKey, PackageEntry>,
    /// The reader's record of the versions it refused to act on —
    /// in memory only, never written into any catalog file.
    pub quarantined: Vec<Quarantined>,
    /// Per-name tombstones; `write_to` projects them back onto the
    /// `by-name/<name>.json` it builds.
    pub tombstones: BTreeMap<String, Tombstone>,
}

// generated — schemas/index/e1/by_name.jtd.json
pub struct PackageEntry {
    pub group: Group,
    pub name: String,
    pub indexed_at: Timestamp,
    pub versions: Vec<VersionEntry>,        // ascending by version
    pub latest_stable: Option<Version>,
}

// generated — the shared `version_entry` vocabulary (§2.6)
pub struct VersionEntry { /* … */ }
```

@fact:THE-TRAIT-FLOOR-STOPS-SHORT-OF-DEFAULT **The generated types carry a fixed trait floor — `Debug`, `Clone`, `PartialEq`, `Eq` beside the serde pair — and `Default` is deliberately not in it.** The dividing question is whether the trait says anything about the FORMAT: the four are properties of the Rust representation and the wire knows nothing of them, so emitting them unconditionally is the same class of decision as canonical ordering. `Default` is different in kind — «does this type have a meaningful empty value» is a judgement about the type, not a fact about its form. An empty `ProvidesEntry` means «provides nothing»; an empty `VersionEntry` means nothing at all, since twenty-odd of its fields are required. So `Default` lives in hand-written impls beside the generated tree, on the sub-structures where it is meaningful, and **`VersionEntry` has none**. @status:impl/done

@fact:A-RECORD-LITERAL-NAMES-EVERY-FIELD **The consequence, which nothing in the tree guards:** because the record derives no `Default`, every literal that builds one names all its fields, and there is no `..Default::default()` tail anywhere to shorten them. That is not an inconvenience to be optimised away — it is what makes adding a field a decision at every construction site instead of a silent zero. A future session that "simplifies" by deriving `Default` on the record would change how records are built from fixtures without any test going red, which is why the boundary is written here rather than left to be re-derived. @status:impl/done

@fact:WHAT-THE-RE-EXPORT-COST-AND-WHY-IT-WAS-PAID **What moving to generated types cost, measured rather than estimated, so the price is not re-paid in reverse.** Three traits left with the hand-written shapes and the tree was checked for each: `Ord` / `Hash` / `PartialOrd` — wanted by **nobody**, and a comment claiming one of them justified duplicating a vocabulary had been false since it was written; `Copy` on the kind vocabulary — really lost, and the call sites that had it became explicit clones; `Default` — restored by hand where it means something. The classification is what matters, not the counts, which is why the counts live in the dated measurement (`campaigns/packages-2026-09/harvest/f42c-reexport-radius.md`) and not in this sentence. Anyone proposing to restore a duplicate type for the sake of `Copy` is proposing to unpay this, and should read what it bought first. @status:impl/done

@fact:AN-EPOCH-ARRIVES-IN-A-FRAGMENTS-NAME **A vocabulary fragment that changes in a new epoch is a DIFFERENT fragment, and it is separated by its NAME, not by a directory.** The shared home holds fragments once, by name, and every schema module that pulls one re-exports it; when a second epoch needs a changed shape, the changed shape gets its own name and sits beside the old one in the same home. Worlds stay separated because two names never denote one type — the same rule that makes the shared home possible at all. Putting epochs into the home's directory structure instead would fork the home and give one fragment two addresses, which is the collision the single-home phase exists to prevent. *Revisit when:* the first second-epoch schema pulls a first-epoch fragment — that is the day this rule is exercised for real, and until then it has never been tested. @status:spec/done

@fact:PKGKEY-SHAPE `PkgKey = (Group, String)` — the `(group, name)` identity of
PROP-008 §2.2, and the order `by_pkgref` walks in. `kind` is metadata and
identifies nothing, so it is not part of the key. @status:impl/done

@fact:TEXT-INDEX **Search keeps no stored index.** The postings are built per query against the loaded `Index` (`index/search.rs`) rather than held as a field, so no mutation has anything to invalidate. Token = lowercased ASCII alphanumeric run; ~30-stopword filter (the same list `vibe-check::activation_conflict` uses, deliberately reused for consistency). Ranking is term-overlap — one point per query token a hit carries — with the `(group, name)` identity breaking ties. Good enough for ≤10k packages; tantivy is a v1 upgrade if it isn't. Search answers only over what this build can act on: it asks the `quarantine::usable_*` accessors, never `pkg.versions` raw ([§2.6](#entry)). @status:impl/done

### 2.13 Persistence layer {#persistence}

@fact:req-persistence `req r1` @status:impl/done

@fact:DATA-DIR-LAYOUT **Decision.** `<data-dir>/` layout: @status:impl/done

```
<data-dir>/
├── hello.json                        # the eternal handshake (§2.4)
├── repomd.json                       # the manifest (§2.4)
├── primary.jsonl
├── primary.jsonl.gz
├── by-name/
│   └── <name>.json                   # no <kind>/ level — kind left package identity (PROP-008)
├── by-cap/
│   └── <slug>.jsonl
├── by-purl/
│   └── <slug>.jsonl
├── README.md                         # auto-generated; explains "this is a vibevm index"
├── .gitignore                        # written by `init`; covers state/
└── state/                            # NOT mirrored (gitignored when data-dir is a git working tree)
    ├── journal/<YYYY>-<MM>.ndjson    # the registry facts journal — the AUTHORITATIVE layer
    ├── server.lock                   # PID file, present only when serve is running
    ├── admin.tokens                  # bearer tokens (gitignored)
    ├── checkpoint.json               # incremental-reindex bookkeeping (last commit/tag per repo)
    └── org-cache.json                # the organisation image and its validator (§2.8.1)
```

@fact:THE-TRUTH-LIVES-UNDER-THE-DIRECTORY-THAT-IS-NOT-SERVED **The one thing to notice in that tree: `state/` is not mirrored, and the journal lives there.** Everything above `state/` is a projection and may be deleted and rebuilt; `state/journal/` is the layer it is rebuilt FROM ([§2.3](#truth)), and the server refuses to start without it. So the directory's gitignore boundary and its truth boundary run in opposite directions — the served half is disposable, the unserved half is not — and an operator who backs up «the index» by copying what the mirror carries has backed up the derivative and left the original. @status:impl/done

@fact:COUNTERS-ARE-NOT-A-FILE **`/metrics` counts from memory, not from a file.** The counters are atomics in the server's own state and reset with the process, which is what an operational counter means; no `state/stats.json` exists, and a reader looking for one would find a durable-looking name for a volatile fact. @status:impl/done

@fact:DATA-DIR-IS-WORKTREE The data-dir doubles as a git working tree of the org's `index` repo. `state/` is `.gitignore`d (the `init` subcommand writes a default `.gitignore`). Operators commit + push the rest manually, or via `--auto-commit-push` — built 2026-08-06, contract in [§2.17](#auto-publish). @status:impl/done

@fact:ATOMIC-WRITE-PROTOCOL **Atomic write protocol.** For each file F to be replaced: @status:impl/done

1. @fact:AW-TMP Write `F.tmp` next to `F`. @status:impl/done
2. @fact:AW-FSYNC `fsync(F.tmp)`. @status:impl/done
3. @fact:AW-RENAME `rename(F.tmp, F)`. @status:impl/done
4. @fact:AW-FSYNC-DIR `fsync(parent_dir(F))` on POSIX. (No-op on Windows where the directory has no fsync semantics; rename itself is atomic.) @status:spec/plan

@fact:THE-DIRECTORY-FSYNC-IS-NOT-DONE **Step 4 is not performed, anywhere.** Every `sync_all` in the crate is on a FILE — the temp file here, the journal shard, the lockfile, a test fixture — and no code path opens a directory to flush it. Steps 1–3 are exactly as written and the temp file is `<F>.tmp.<pid>`, which changes nothing about the contract. What step 4 buys is the durability of the *rename* across a power loss on POSIX: without it the new bytes are safe and the directory entry pointing at them may not be, so a crash can leave the old name resolving to nothing rather than to either version. That is a narrow window and a real one, it costs a few lines to close, and it is filed as `BACKLOG.md` B-087 rather than quietly dropped from the protocol — a durability step deleted because nobody implemented it is how a guarantee becomes folklore. @status:impl/done

@fact:REPOMD-LAST-LAW `repomd.json` is replaced **last among the files it vouches for**, so a reader that fetches `repomd.json` first then chases hashes always sees consistent files. The handshake is written after it and is the one root file that does not weaken the rule, because the manifest never claimed it ([§2.4](#layout)) — the precedent being `README.md` and `.gitignore`, which `init` writes and the map does not carry either. @status:impl/done

### 2.14 Integration with the rest of vibevm {#integration}

@fact:req-integration `req r1` @status:impl/done

@fact:consumer-side-lead **Consumer side (`vibe-cli`, `vibe-registry`).** @status:impl/done

- @fact:INT-FAST-PATH `crates/vibe-registry/src/multi_registry_resolver/` carries an optional **index-aware fast path**. Before falling back to per-repo `git ls-remote`, it opens a session by the discovery ladder of [§2.1](#optional) — the handshake first, the manifest as the compatibility tail — and on success reads `by-name/<name>.json` for the pkgref, selects the candidate whose `group` matches, and picks the matching version locally: zero ls-remote calls. An `absent` probe falls through to today's path; a `refused` one surfaces its reason instead of pretending nothing was there. @status:impl/done
- @fact:INT-VERIFY-ANYWAY Index-derived `content_hash` does NOT replace fetch-time verification. The actual `git fetch` still happens; the post-fetch `compute_content_hash` still runs; mismatch still errors out per [PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity). @status:impl/done

@fact:publisher-side-lead **Publisher side (`vibe-publish`).** @status:impl/done

- @fact:INT-PUBLISH-HOOK `crates/vibe-publish/src/post_hook.rs` carries an optional post-publish hook: when **both** `VIBEVM_INDEX_URL_<REGISTRY>` and `VIBEVM_INDEX_TOKEN_<REGISTRY>` are set for the registry being published to, the publisher POSTs the new entry to `<index-url>/v1/packages` with a bearer token after a successful `push_release`. Failure of the index POST does NOT fail the publish — it logs a warning and the operator's next `vibe-index reindex` covers the gap. @status:impl/done
- @fact:THE-HOOKS-TWO-SETTINGS-ARE-KEYED-BY-REGISTRY-NOT-BY-HOST **Both settings are per-REGISTRY environment variables, and the distinction matters.** The suffix is the registry's local alias from `[[registry]].name`, not the host — one host can serve several registries and one registry can move hosts, so keying on the host would name the wrong thing in both directions. The manifest fields this document once promised (`index_url`, `index_token`) do not exist ([§2.2](#form-factor)), so the environment is not one source among several here: it is the only one. @status:impl/done
- @fact:INT-DIRECT-PUSH Direct-push (`--repo-url`) bypasses index updates entirely (no registry context). @status:impl/done

@fact:outdated-lead **`vibe outdated` (M1.10 follow-up).** @status:impl/done

- @fact:INT-OUTDATED-FAST Adds a fast path: when a registry has an index, query `by-name/<name>.json` for the latest version instead of `git ls-remote`. Same envelope shape; ~100× faster for large lockfiles. @status:impl/done

@fact:search-lead **`vibe search` (M2.10 — this is what unblocks it).** @status:impl/done

- @fact:INT-SEARCH Walks every configured registry's index through the same client the resolver uses — probe, then query — rather than downloading `primary.jsonl.gz` and scanning it locally. The whole-file scan was the shape this document first imagined and is not what shipped: asking the index a question keeps the bandwidth proportional to the answer instead of to the catalog, and it puts one discovery ladder ([§2.1](#optional)) under every consumer instead of two. Index is the enabling layer for M2.10; `vibe search` is the headline consumer of this PROP. @status:impl/done

@fact:INT-SLICED Each integration point is a separate slice. v0 of `vibe-index` ships without any of them — the index can be populated and consumed via raw HTTP / git clone before vibevm consumers know about it. Integration slices land in M2.10 / M1.10 follow-ups. @status:impl/done

### 2.15 What index must NEVER do {#never}

@fact:req-never `req r1` @status:impl/done

- @fact:NEVER-REPLACE-TRUTH **Never replace `vibe.toml` as the source of truth.** A package with a missing index entry still installs from git per the live path. A package with a divergent index entry triggers `IntegrityError`, never silent acceptance. @status:impl/done
- @fact:NEVER-MODIFY-REPOS **Never modify package repos.** The index utility reads package repos (for the `--from-clones` walk) but never writes to them. @status:impl/done
- @fact:NEVER-ECHO-TOKENS **Never echo tokens.** Same discipline as [PROP-000 §20](../../common/PROP-000.md#token-secrecy). Auth tokens for the server, GitHub API tokens for `--from-github`, publish tokens propagated through hooks — none ever appear in stdout / stderr / logs / JSON envelopes. @status:impl/done
- @fact:NEVER-ASSUME-MIRROR **Never assume mirror infrastructure.** The index is opt-in everywhere; no consumer or publisher fails because the index disappeared. @status:impl/done
- @fact:NEVER-SILENT-SCHEMA **Never make breaking schema changes silently.** The refusal is what matters and it survives; where it LIVES moved with the journal. A build must never read the subset it understands and carry on as though it understood the whole — but the carrier of that refusal is no longer a version number on the catalog, because the catalog is a projection any build rewrites from facts. It is the per-record capability set: a record naming a capability the reader lacks is refused BY NAME, with a recipe, and the rest of the catalog still loads ([PROP-044 §4.5](../../common/PROP-044-change-native-formats.md#machinery); the answer's shape and the surfaces that owe it are [§2.19](#unavailable)). That is strictly louder than a version compare, which could only say "somewhere in here is something newer than you". Unknown FIELDS are a different question and are answered by `##FORWARD-COMPAT`: they are tolerated, because tolerating them can no longer lose them. @status:impl/done

### 2.16 Webhooks — feeding the index instead of polling it {#webhooks}

@fact:WEBHOOK-THE-PROBLEM **The problem, and it is not performance.** To learn
what changed, the index enumerates the organisation. That is a cost, but the
cost is the small half. The large half is that **the picture goes stale without
anyone doing anything wrong**: a developer publishing a package creates a
repository and pushes a tag straight to the git host, never passing through the
index service. So the index's image is behind from the moment it is taken, and a
stale index is worse than a slow one — the package exists and cannot be found.
@status:spec/done

@fact:WEBHOOK-IS-THE-ANSWER-TO-THE-CACHE **This is what makes the organisation
cache honest.** [§2.8](#reindex)'s cache and its cheap freshness check reduce how
often the index asks; they do not change who knows first. A webhook does: the
image becomes authoritative **because it is fed**, rather than because we assumed
nobody else writes. Both mechanisms ship — the freshness check is what keeps the
cache truthful when no webhook is configured, and every deployment starts that
way. @status:spec/done

@fact:WEBHOOK-ENDPOINT **The endpoint is per host flavour, deliberately.**
`POST /v1/hooks/{source}` where `{source}` names the git host's flavour
(`github` today; other flavours are added when one is measured, not reserved in
advance). One route per flavour rather than one generic route, because the
payload shape and the signature scheme are host-specific and a single endpoint
would have to sniff which it received — which is guessing, dressed as
convenience. @status:spec/done

@fact:WEBHOOK-SECRET-IS-NOT-THE-ADMIN-TOKEN **A webhook authenticates with its
own shared secret, never with an admin token.** The sender signs the request body
under a per-hook secret and the index verifies that signature; an unverifiable
request is refused and not processed. Two reasons, and the second is the
important one. *(i)* The secret is held by a third party — the git host — and
must be rotatable without touching the tokens that authorise real writes.
*(ii)* **Least authority:** an admin token authorises arbitrary writes, while a
webhook may only cause the index to re-read one repository. Handing a notification
channel the authority to write anything is how a notification becomes an attack
surface. Secret storage follows [§7](#secrets) — file, never a flag, never a log.
@status:spec/done

@fact:WEBHOOK-PAYLOAD-IS-A-NOTIFICATION-NOT-DATA **The load-bearing rule: a
payload says *that* something changed, never *what* it now is.** The index reads
the manifest from the git host itself and applies the per-package `add` / `remove`
of [§2.8](#reindex) for the named repository only. Nothing from the request body
is ever written into an index record. This follows from [§2.3](#truth) — package
repos remain authoritative — and it is the rule that keeps an attacker-influenced
input from becoming index content. **A webhook may never trigger a full
reindex**, both because that is the expensive thing it exists to avoid and
because a cheap request that causes an expensive walk is a denial-of-service
lever. @status:spec/done

@fact:WEBHOOK-DELIVERY-IS-UNRELIABLE **Deliveries arrive twice, out of order, or
not at all, and the design assumes all three.** The handler is idempotent by
construction — re-reading a repository and upserting its versions is the same
operation performed twice — and it never infers from one event that it saw the
previous one. Consequence, stated so it is not quietly dropped later:
**webhooks reduce staleness, they do not abolish the full walk.** The explicit
`rescan-org` verb of [§2.8](#reindex) stays unconditional exactly because a
missed delivery is invisible from inside. @status:spec/done

@fact:WEBHOOK-FAILURE-POSTURE **What each failure answers.** An unverifiable
signature is `401` and is not processed. A payload naming a repository outside
this server's configured organisation is `400` and is not processed — the scope
check is not optional, since the repository name is the one field of the payload
we act on. A verified, in-scope delivery whose re-read fails is `202`: the
request was accepted and the work is ours to retry. Answering `5xx` to a sender
that retries on a schedule we do not control would turn our own outage into a
retry storm — the failure is on our side, and the status code should say so
rather than invite the sender to hammer. @status:spec/done

@fact:WEBHOOK-VS-ACTIONS **The GitHub-Actions alternative, and why it is the
fallback rather than the default.** The same effect is reachable without any
endpoint: an Action in the package repository `POST`s to the write API of
[§2.10](#http) with an admin token. It is genuinely simpler — no new route, no
signature verification, nothing to specify. What it costs is exactly what
`#WEBHOOK-SECRET-IS-NOT-THE-ADMIN-TOKEN` protects: **every participating
repository then holds a credential that can write anything**, and the number of
places a broad secret lives grows with the organisation. The webhook keeps one
narrow secret in one place. So: webhook by default; the Action is the honest
answer for an organisation that already runs its publishing through Actions and
prefers one mechanism to two, and it has one capability the webhook does not —
it can also push the built index files, being a runner with a checkout.
@status:spec/done

@fact:WEBHOOK-NOT-DECIDED **Named, not invented — two things this section does
NOT settle.** *(i)* Whether the server should require the pushed ref to look
like a release tag before acting, or re-read on any push to the default branch
as well: the second catches a manifest edited without a tag, which
[§2.8](#reindex)'s incremental walk already treats as a real case, and the first
is cheaper. Decide it against a measured event volume, not here. *(ii)* The
GitVerse flavour is **unmeasured**: their public API could not enumerate an
organisation, and nothing here establishes what their webhooks can do. Writing a
`gitverse` route from that ignorance would be inventing a contract for a system
nobody in this repository has watched. @status:spec/plan

#### 2.16.1 Setting one up — the operator's walkthrough {#webhooks-guide}

@fact:WEBHOOK-GUIDE-LIVES-HERE **Why this walkthrough sits inside the
specification and not in `docs/`** *(owner ruling, 2026-08-06)*. It describes how
to configure a mechanism **whose properties this document defines**. Kept beside
the contract it changes when the contract changes, because the two are the same
file and the same commit; kept in `docs/` it drifts. That is not a hypothetical:
this repository measured two independent instances of exactly that drift in a
single week — the index's own format documentation against its code, and an
owner guide promising a gate step that did not exist. @status:spec/done

@fact:WEBHOOK-GUIDE-IS-NOT-YET-A-CLAIM **Read the steps below as a
specification, not as instructions that work today.** The endpoint is designed
here and not built; the block is therefore an example and carries no
`@fact/code:` marker, which is precisely the distinction that keeps a fenced
block from asserting something nobody can falsify. When the route ships, this
block becomes the fact's body and comes due with it. @status:spec/plan

```
# 1. On the index host: put the shared secret where the server reads it.
#    One secret per configured hook; file, not a flag (§7).
$ printf '%s' "$SECRET" > ./vibespecs-index/state/webhook.secret

# 2. Start the server with the hook route enabled.
$ vibe-index serve ./vibespecs-index --bind 0.0.0.0:8412 \
    --auth-tokens-file ./vibespecs-index/state/admin.tokens \
    --webhook-secret-file ./vibespecs-index/state/webhook.secret

# 3. On the git host, at the ORGANISATION level (not per repository —
#    a per-repo hook is one more thing to remember on every new package):
#      payload URL   https://<index-host>/v1/hooks/github
#      content type  application/json
#      secret        the same $SECRET
#      events        pushes and tag/release creation only — not "everything"
#
# 4. Verify the wiring before trusting it: push a tag to any package repo,
#    then ask the index what it now knows about that package.
$ vibe-index get ./vibespecs-index <group> <name>
```

@fact:WEBHOOK-GUIDE-VERIFY-STEP **Step 4 is not politeness.** A hook that is
configured and silently not arriving looks exactly like a hook that is arriving
and finding nothing to do, and the difference is invisible from the index side —
which is the same shape as `#WEBHOOK-DELIVERY-IS-UNRELIABLE`, seen at setup time
instead of at run time. Verify once against a known change; after that, trust
the mechanism and keep `rescan-org` for the deliveries you will never know you
missed. @status:spec/done

### 2.17 Auto-publication — the server carries its own result to the host {#auto-publish}

@fact:AUTO-PUBLISH-CLOSES-THE-ONE-MANUAL-HOLE **What it fixes.** The server
already accepts an authenticated write, writes the files atomically, recomputes
the manifest and verifies integrity. The one thing it could not do was **carry
the result to where it is served from**. `--auto-commit-push` closes that: after
each successful mutation the server commits the data directory and pushes it.
Built 2026-08-06 on the owner's ruling; the flag had been declared and discarded
by one line since the server shipped. @status:impl/done

@fact:AUTO-PUBLISH-TARGET-IS-THE-WORKING-COPYS-OWN-UPSTREAM **Where it publishes
is the operator's setting, and vibe-index does not mint a second place to say
so.** The data directory is already a git working tree ([§2.4](#layout)), and a
working tree's remote and branch are configured with plain git. So the push
carries no refspec and names no remote — it goes where the tree is already
pointed. A private repository is a legitimate target by construction: it is
simply what the operator cloned. Rejected: a `--push-remote` / `--push-url`
pair, and a target block in the on-disk config — both would be a second home for
a value git already owns, which is the defect class this repository keeps
paying for. @status:impl/done

@fact:AUTO-PUBLISH-REFUSES-TO-SHIP-SECRETS **Startup refuses rather than warns,
and this is the flag's most important behaviour.** `state/` holds the bearer
tokens ([§7](#secrets)). If the data directory's `.gitignore` does not cover
them — a directory created before `init` wrote one, or one edited since —
`git add -A` would stage those tokens and push them to a host that may be
public. So with the flag set, the server **does not start** unless it confirms
`state/admin.tokens` is ignored, and the refusal says so in those words. The
check runs once at startup rather than per mutation, because the operator must
learn the configuration is unsafe **before** the first token leaves the machine,
not after. @status:impl/done

@fact:AUTO-PUBLISH-REFUSES-WITHOUT-A-WORKING-COPY **The second refusal:** the
flag set over a data directory that is not a git working copy also stops the
server, naming what to do. Publishing by committing a directory that is not
tracked is not a degraded mode, it is a no-op that would look like success
forever. @status:impl/done

@fact:AUTO-PUBLISH-A-FAILED-PUSH-IS-NOT-A-FAILED-WRITE **A push failure never
turns a successful write into an error.** By the time publication runs the
mutation is on disk and in memory; the HTTP write has happened. So a failure is
logged at `warn` with git's own message and counted as
`vibe_index_publish_failures_total`, and the request still answers as it would
have. It is not rolled back either: a network outage must not be able to corrupt
index state. Transient failures self-heal — git accumulates, and the next
successful push carries the queued commits. @status:impl/done

@fact:AUTO-PUBLISH-AN-EMPTY-DIFF-IS-SUCCESS **Nothing to commit is success, not
an error** — the opposite of the publish flow's rule for the same operation, and
deliberately so. The index lock is released before the push, so a second
mutation can land while the first is publishing and the first commit carries
both. The second then finds nothing staged, and that is the normal course of
events rather than a caller's mistake. @status:impl/done

@fact:AUTO-PUBLISH-IS-SERIALISED-AND-AWAITED **One publication at a time, and the
response waits for it.** Two concurrent mutations must not interleave two
commits in one working copy, so publication takes its own lock — not the index
lock, which is released earlier and correctly so. The handler awaits the result
on a blocking thread, which means a `200` says «persisted **and** published».
For an index that is what the operator asked for, and mutations are publish
events rather than a hot path. @status:impl/done

@fact:AUTO-PUBLISH-EVERY-COMMIT-NAMES-ITS-CHANGE **The commit message names what
moved** — the upsert or the removal, with the package coordinate. Each of the
three mutating routes knows its own change, so the index's history reads as a
log of publications rather than a wall of identical messages. @status:impl/done

@fact:AUTO-PUBLISH-EVERY-MUTATION-COMMITS-EVEN-A-NO-OP <status stage="spec" state="void">Retired 2026-08-14 by the determinism phase. It recorded a real consequence of the writer stamping a fresh generation time on every write: a repeated identical upsert produced a diff and therefore a commit, so the empty-diff path fired only on the overlap case. Both halves of that consequence are gone — the writer takes its clock as an input (`##THE-WRITER-TAKES-ITS-CLOCK-AS-AN-INPUT`), and a mutation that changes nothing no longer writes at all (`##A-MUTATION-THAT-CHANGES-NOTHING-COMMITS-NOTHING`). Its closing sentence asked whether the index's own write should be deterministic and called that a question about the format rather than about this flag; the format answered yes. This tombstone stays so the old sentence's name is never reused and inbound links do not break.</status> @status:spec/void

@fact:A-MUTATION-THAT-CHANGES-NOTHING-COMMITS-NOTHING **A mutation that changes nothing writes nothing, and therefore commits nothing.** An upsert whose entry equals the one already stored under that version number leaves the in-memory state untouched, never reaches the writer, and never reaches the publisher; the two removal routes have always behaved this way, and the upsert route now matches them. The response is still success — the resource is already in the requested state, which is what idempotency means over HTTP — and the distinction between *created* and *changed* is kept, because a differing entry under an existing version number is an update and must still land. Determinism alone would not have bought this: with the clock arriving per mutation event, a repeat would still have moved `generated_at` and produced a diff. The point is not to produce an empty diff but to not create the work, so that the catalog's history records events that actually happened. @status:impl/done

@fact:AUTO-PUBLISH-COMMITTER-IDENTITY-IS-THE-OPERATORS **No identity is invented.**
The commit uses whatever git identity the host is configured with; if there is
none, `git commit` fails and that failure takes the path above — logged and
counted, never fatal to the write. Inventing a fallback author would put a name
in an organisation's published history that nobody chose. @status:impl/done

### 2.18 Channels — author-named version pointers {#channels}

@fact:channels-req `req r2` @status:spec/plan

@fact:CHANNELS-ARE-AUTHOR-POINTERS **Decision (owner rulings, 2026-08-13; not
built — this section is the contract the build will follow).** A **channel**
is an author-controlled named pointer `(group, name, channel) → version` —
npm's dist-tags and Docker's tags are the prior art. The pointer map is
**flat**: several channels may point at one version (a release that is both
`latest` and `stable` is the everyday case), and no promotion semantics
(`beta` → `stable` as a registry operation) are baked into the format —
promotion is the author's workflow, not registry law. A channel may point at
a snapshot or at a frozen version (PROP-044 §2b) — the axes are orthogonal. @status:spec/plan

- @fact:THE-JOURNALS-HALF-OF-THIS-CONTRACT-IS-ALREADY-MINTED **What already exists, so nobody mints it twice:** the journal's event vocabulary carries `ChannelSet {group, name, channel, version}` and `ChannelUnset {group, name, channel}` in exactly the shape below, and the generated entry types carry a `channels` list. What is NOT built is the projection and the surfaces. And the projector's treatment of that gap is the load-bearing part: meeting a channel act it **refuses the whole projection by name** — «the journal holds a `ChannelSet` record, but its carrier (channels) is not built in this vibe-index; skipping the record would project a catalog the journal does not describe» — rather than skipping the record and continuing. The journal is truth ([§2.3](#truth)); a projector that quietly dropped an event it did not understand would publish a catalog asserting a state nobody recorded, which is the one failure re-fetching cannot cure. `Notice` and `ForceReplaced` stand in the same place for the same reason. `Renamed` stood there too until the retirement collapse: it left the vocabulary, and its successor `Buried` is the one arm that went the other way — it gained a carrier and now PRODUCES a tombstone instead of refusing, which is what makes it the first producing arm this projector has ever had ([§2.11](#cli)). @status:impl/done
- @fact:CHANNEL-NAME-GRAMMAR **Channel-name grammar:** `[a-z][a-z0-9-]*` — it
  must not start with a digit (versions do) or a version-requirement operator
  (`^ ~ = < > *`), which is what makes `@beta` unambiguous in a pkgref's
  version position. @status:spec/plan
- @fact:CHANNELS-AUTHORITY-IS-THE-JOURNAL **Authority is the journal; the
  catalog only projects.** Channel state changes are registry facts:
  `Published` carries the manifest-declared channels (below), and the explicit
  acts `ChannelSet {group, name, channel, version}` / `ChannelUnset` retarget
  or clear a pointer. No hand-edited pointer file exists anywhere — a
  hand-written `vibeversions.toml` inside the derived catalog would be a
  secretly-authoritative fact (PROP-044 law 2). The projection lands the map
  in the `NameEntry` (`channels: {stable → 1.1.0, beta → 1.2.0-rc.1}`) — the
  same `by-name/<name>.json` candidate file the resolver already fetches, so
  channels cost **zero additional round-trips**. @status:spec/plan
- @fact:MANIFEST-CHANNELS-ARE-PUBLISH-TIME-FACTS **The manifest declares
  membership as a publish-time fact, never as the pointer.** `[package]
  channels = ["stable", "lts-2026"]` (a list — multiplicity is first-class;
  the singular `channel = "…"` is rejected with a did-you-mean) records which
  channels this version was *published into* — immutable with the content,
  honest forever. Publication moves each named pointer to this version (npm's
  `publish --tag` semantics), so **routine channel management is just
  publishing** — no separate command. The pointer itself cannot live in the
  manifest: retargeting `stable` back to a frozen `1.1.0` (the rollback — the
  main use case) would require editing frozen bytes, which is forbidden;
  that act is the journal's `ChannelSet`, via `vibe registry channel set
  <group>:<name> stable 1.1.0`. @status:spec/plan
- @fact:LATEST-AND-STABLE-ARE-BUILT-IN **LATEST and STABLE are the two
  built-in channels** (Maven's `<latest>`/`<release>` pair). A channel is
  *authored* from the first explicit act (a manifest declaration or a
  `channel set`) and stays authored until `channel unset`; while unauthored it
  is **computed at projection time**: STABLE = the greatest non-prerelease
  version, LATEST = the greatest version outright — both by the ordering
  below. A new publication without declarations does **not** move an
  authored pointer: if the author said `stable = 1.2.0`, releasing 1.3.0 does
  not silently make it stable. @status:spec/plan
- @fact:VERSION-ORDERING-WITH-BUILD-TIEBREAK **The ordering (owner ruling,
  2026-08-13): SemVer precedence first, natural-sort tie-break on build
  metadata second.** SemVer 2.0.0 is not modified: `+build` is legal in
  published versions, the **coordinate is the full version string including
  `+…`** (uniqueness is hard on the string), and precedence ignores metadata
  exactly as the standard demands — every foreign semver library agrees with
  us. Where SemVer declares two versions equal, our resolver breaks the tie
  deterministically: versions **with** metadata outrank the bare version (a
  `+stamp` is a rebuild atop it), and among metadata the greater under
  **natural sort** (digit runs compare numerically, text lexicographically)
  is the fresher — so `+20260813…` beats yesterday's stamp. Reproduction is
  never at stake — the lockfile pins `content_hash` — the tie-break only
  answers "which is latest", and only those who chose to publish `+` twins
  pay the axis any attention. @status:spec/plan
- @fact:RESOLVER-DEFAULT-IS-STABLE-THEN-LATEST **The resolver default (owner
  ruling, 2026-08-13): `vibe install pkg` with no requirement and no channel
  takes STABLE when it exists, else LATEST — and the frozen/snapshot state
  does not influence selection at all.** Selection and integrity are separate
  axes: the chosen version is pinned by hash in the lockfile, and *after*
  selection the freeze contract governs mismatches (frozen — alarm; snapshot —
  news). Requesting a channel is the explicit act: `{ channel = "beta" }` on
  a dependency in `vibe.toml` (mutually exclusive with a version
  requirement) or `@beta` in a pkgref's version position. @status:spec/plan
- @fact:CHANNEL-RESOLUTION-PINS **Resolution through a channel pins
  `{channel, resolved version, content_hash, locator}`** in the lockfile:
  `vibe install` reproduces the pin; `vibe update` re-follows the pointer;
  `--locked` turns any drift into a loud CI error. @status:spec/plan
- @fact:DEAD-POINTER-IS-LOUD **An authored pointer at a dead target is a loud
  state.** When a channel's target version is yanked or removed, computed
  channels simply recompute past it, but an *authored* pointer refuses at
  resolve time with a recipe («stable указывает на отозванную 1.2.0 — автор
  должен переставить или снять; потребитель может явно взять версию»).
  Silently hopping to "the next best" would be a choice the author never
  made. @status:spec/plan
- @fact:CHANNELS-DEGRADED-RESOLUTION **The degraded ladder (catalog
  unreachable).** (1) A lockfile answers without the catalog at all —
  resolve-by-lock never needs it. (2) No lock but a local catalog cache →
  resolve against the cache, loudly stamped «по снимку каталога от <даты>».
  (3) Cold resolve with the provider alive → enumerate versions at the
  provider (`ls-remote --tags`-class), read manifests at tags, and let the
  **local resolver** reconstruct channels from the publish-time `channels`
  declarations — approximate exactly where the author manually retargeted,
  and the output says so («по перечислению провайдера, каталог недоступен»).
  (4) Neither reachable → refusal with a recipe. Degradation is always
  announced, never silent (PROP-044 law 1). @status:spec/plan

### 2.19 The `unavailable` answer — what a surface says about a version it will not serve {#unavailable}

@fact:req-unavailable `req r1` @status:impl/done

@fact:THE-REFUSAL-IS-AN-ANSWER-NOT-AN-OMISSION **Decision.** A surface that cannot act on a record does not drop it — it **names it**. [PROP-044 §4.5](../../common/PROP-044-change-native-formats.md#machinery) gives the law («the refusal surfaces at the point of use with a generated recipe»); this section gives its shape in this catalog. A version whose `must_understand` ([§2.6](#entry)) names a capability this build lacks is **unavailable to this build**, and every surface that computes an answer says so out loud. Quietly narrowing the answer instead would be the silence [PROP-044 §2](../../common/PROP-044-change-native-formats.md#laws) forbids: the package exists and cannot be found, which is a riddle rather than a break. @status:impl/done

@fact:UNAVAILABLE-SHAPE **The answer row is one shape, used by every surface:** `{group, name, version, missing, recipe}`. It carries the **full coordinate even where the envelope around it already names the package** — a row that identifies itself survives being copied out of its envelope by a script, and a context-dependent one does not. `missing` is exactly the subset of the record's `must_understand` this build does not understand — not the whole declaration, because a reader that understands three of four capabilities must be told about the fourth, not about all four. `recipe` is the generated text that says what a person or a script does about it. @status:impl/done

@fact:THE-RECIPE-HAS-ONE-HOME **The recipe is built in one place and never written as a literal at a call site.** One home, N surfaces: a literal per surface is N texts that drift, and the one that drifts is the one nobody reads until it matters. It is **degenerate today by measurement, not by omission** — no reader capability has been built yet, so every missing capability is one this build simply does not know and there is no second class of recipe to write. The per-capability table this grows into gets its first row from the first capability that lands; inventing rows for capabilities that do not exist would be machinery for a consumer that does not exist. @status:impl/done

@fact:QUARANTINE-IS-A-READERS-JUDGEMENT-AND-IS-NEVER-CARRIED **Quarantine is the READER's judgement about a (record × build) pair, never a property of the record** — so it is derived at the point of use from the record's own `must_understand` and is **never stored on the wire**. The consequence worth the ink: the command line and the server agree **by construction**, not by two implementations being kept in step. The predicate reads the record, so it does not matter which carrier the record arrived in — and the carriers genuinely differ, since a catalog LOADED from disk arrives with a quarantine record while one PROJECTED from the journal arrives with an empty one. Two surfaces that agreed only because someone remembered to update both would disagree the first time one of them was forgotten. @status:impl/done

@fact:THE-SAFE-DEFAULT-IS-A-CONSTRUCTION-NOT-AN-AGREEMENT **The safe default is a property of the construction.** The answering path asks the **named accessors** (`quarantine::usable_*`) and never reads the stored version list or `latest_stable` raw; the **writer's** path, the mutations, and the operational counters ask the raw state deliberately — the catalog is the projection of the journal ([§2.3](#truth)), and a reader's capabilities have no business shrinking what is WRITTEN or miscounting what the index HOLDS. The asymmetry is stated in the doc-comments of both sides, and that statement is the only defence against the next author reaching for the wrong accessor: the two calls look identical at the call site and differ only in what they mean. @status:impl/done

@fact:WHICH-SURFACES-OWE-THE-ANSWER **Which surfaces owe it, as a rule rather than a list** — because a list rots and a rule does not: **every surface that COMPUTES an answer owes the refusal; a surface that serves a stored file verbatim does not.** Computing covers each read verb that selects, narrows, ranks or aggregates, and each HTTP route that answers from the in-RAM index. Serving verbatim covers the raw file routes of [§2.10](#http). @status:impl/done

@fact:THE-RAW-FILE-WAS-NEVER-THE-ONE-KEEPING-SILENT **The raw candidate-set file is not silent, and making it «speak» could only mean removing information from it.** `by-name/<name>.json` hands back the record word for word, `must_understand` included — and that declaration IS the explanation of the refusal, delivered to a client that can then apply its own capability set rather than ours. Silence lived exactly in the surfaces that computed an answer and dropped a record without a word; the file that says everything was never the problem. @status:impl/done

@fact:A-REFUSED-VERSION-IS-A-404-CARRYING-ITS-REASON **Over HTTP the status stays `404` and the body carries the reason.** «You did not get the thing» is preserved for every client that only reads status codes, while the problem document's `type` and `title` name the refusal in its own words — not «resource not found» — and an **extension member** carries the whole answer row ([§2.10](#http) fixes the RFC 7807 shape; extension members are what that RFC provides for exactly this). The judgement rides the envelope and never enters the record: a `VersionEntry` is generated from the schema and says nothing about any reader. @status:impl/done

---

## 3. Architecture {#architecture}

### 3.1 Crate layout {#crate-layout}

@fact:design-crate-layout `design r1` @status:impl/done

```
crates/vibe-index/                          # a member of the vibevm workspace
├── Cargo.toml                              # depends on vibe-core + vibe-wire; no [workspace] table
├── README.md                               # operator-facing — how to run, common recipes
├── src/
│   ├── main.rs                             # bin entrypoint — clap dispatch
│   ├── lib.rs                              # exports, top-level Error/Result
│   ├── error.rs
│   ├── cli/                                # one file per verb (§2.11) + kinds.rs
│   ├── journal/                            # THE AUTHORITATIVE LAYER (§2.3)
│   │   ├── record.rs                       # the event vocabulary
│   │   ├── store.rs                        # append-only shards under state/journal/
│   │   ├── project.rs                      # journal → catalog; refuses unbuilt carriers
│   │   └── mod.rs
│   ├── index/
│   │   ├── mod.rs                          # the writer's owned surface (§2.4)
│   │   ├── memory.rs                       # Index struct + ops
│   │   ├── quarantine.rs                   # the reader's judgement + the refusal (§2.19)
│   │   ├── persistence.rs                  # atomic write/read of files
│   │   ├── primary.rs                      # JSONL serialise/parse
│   │   ├── by_name.rs                      # candidate-set JSON
│   │   ├── inverted.rs                     # by-cap / by-purl
│   │   ├── repomd.rs                       # repomd.json
│   │   ├── checkpoint.rs                   # incremental-reindex state
│   │   └── search.rs                       # per-query postings, not a stored index
│   ├── scanner/
│   │   ├── mod.rs                          # source-of-truth walkers
│   │   ├── from_clones.rs                  # walk org-dir clones via shell git
│   │   ├── from_github.rs                  # GitHub REST API walk
│   │   ├── org_walk.rs                     # the organisation enumeration
│   │   ├── org_cache.rs                    # the org image + its validator (§2.8.1)
│   │   ├── manifest.rs                     # parses through vibe-core
│   │   └── git_cli.rs                      # the shelled-out git
│   ├── server/
│   │   ├── mod.rs                          # axum app builder — the 16 routes of §2.10
│   │   ├── routes/                         # health · index_files · packages · capabilities
│   │   │                                   #   · purls · admin · metrics
│   │   ├── auth.rs
│   │   ├── error.rs                        # RFC-7807 mapper + the refusal extension
│   │   ├── rate_limit.rs                   # per-token / per-IP buckets (§9 Q10)
│   │   ├── metrics.rs                      # hand-rolled text serialiser
│   │   └── state.rs                        # AppState
│   ├── types/                              # re-export seam over the generated wire types
│   │   ├── mod.rs
│   │   ├── entry/                          # aggregate · content · relations
│   │   ├── repomd.rs                       # the one hand-written shape (§2.12)
│   │   └── kinds.rs                        # PackageKind, NamingConvention dupes
│   ├── publish.rs                          # auto-commit-and-push (§2.17)
│   ├── lock.rs                             # the single-writer PID lock
│   ├── lockfile.rs                         # reading a vibe.lock for `outdated`
│   ├── hash_recipe.rs                      # the recipe a content_hash rides with
│   └── content_hash.rs                     # mirrors vibe-registry::compute_content_hash exactly
├── fixtures/
│   ├── golden-flow-wal-0.1.0/              # the parity fixture
│   └── golden-order-trap-0.1.0/            # the tree where recipes 0 and 1 disagree
├── tests/                                  # help_smoke · cli_{lifecycle,read,write} · server_e2e
│                                           #   · server_writes · auto_publish · rate_limit_e2e
│                                           #   · org_cache_e2e · scanner_e2e · from_github_e2e
│                                           #   · golden_corpus · round_trip_published
│                                           #   · content_hash_parity · six wire_parity_*
└── docs/
    ├── operator-handbook.md
    ├── consumer-protocol.md                # HTTP API reference
    └── format.md                           # repomd / primary / by-name / by-cap / by-purl
```

@fact:THE-CRATE-CARRIES-NO-LICENCE-FILE-OF-ITS-OWN **There is no `LICENSE` inside the crate, and that is the correct state.** `Cargo.toml` carries `license-file.workspace = true`, so the crate inherits the repository's licence rather than keeping a second copy that can disagree with it — the same single-home rule this document applies to normative values, applied to the one value a licence is. The repository's licence is UPL-1.0. @status:impl/done

### 3.2 Dependencies {#deps}

@fact:design-deps `design r1` @status:impl/done

@fact:deps-lead Minimal Rust crates to keep redistribution clean: @status:impl/done

- @fact:dep-clap `clap` (derive) — CLI dispatch. @status:impl/done
- @fact:dep-tokio `tokio` — async runtime for the server, with the four features named below. @status:impl/done
- @fact:dep-axum `axum` — HTTP framework. Mature, minimal, integrates with `tower` middleware. @status:impl/done
- @fact:dep-tower `tower` / `tower-http` — auth, CORS, tracing layers. @status:impl/done
- @fact:dep-serde `serde` / `serde_json` — JSON. @status:impl/done
- @fact:dep-toml `toml` — read package manifests. @status:impl/done
- @fact:dep-semver `semver` — version handling. Same dep `vibe-core` uses; pin same version. @status:impl/done
- @fact:dep-sha2 `sha2` — content_hash. Same as `vibe-registry`. @status:impl/done
- @fact:dep-flate2 `flate2` — gzip primary.jsonl.gz. @status:impl/done
- @fact:dep-walkdir `walkdir` — directory traversal (matches `vibe-registry`). @status:impl/done
- @fact:dep-tracing `tracing` / `tracing-subscriber` — logging. @status:impl/done
- @fact:dep-chrono `chrono` — timestamps. @status:impl/done
- @fact:dep-thiserror `thiserror` — error enums. @status:impl/done
- @fact:dep-git `gix` (or shell-out via `std::process::Command`) — read git tags / show files at refs. Decision §3.3. @status:impl/done
- @fact:dep-reqwest `reqwest` — `--from-github` HTTP client. @status:impl/done
- @fact:dep-tempfile `tempfile` — atomic write helpers. @status:impl/done
- @fact:dep-prometheus <status stage="spec" state="void">Retired 2026-08-18 by measurement: the `prometheus` crate is not a dependency and never became one — `/metrics` renders the exposition format from a hand-written serialiser. The heir is `##THE-METRICS-DEPENDENCY-WAS-NOT-TAKEN` below, which records the choice and its reason. This tombstone stays so the anchor's name is never reused and inbound links do not break.</status> @status:spec/void
- @fact:dep-specmark `specmark` — the in-code spec markers (`scope!`, `#[spec]`) the traceability map is built from. @status:impl/done
- @fact:dep-vibe-wire `vibe-wire` — the generated wire types this crate's `types` module re-exports ([§2.12](#types)). A runtime dependency, not a test one: the library's types ARE the wire's types. @status:impl/done

@fact:THE-METRICS-DEPENDENCY-WAS-NOT-TAKEN **No `prometheus` crate is pulled, and the omission is the design.** `/metrics` renders the Prometheus text exposition format from a hand-written serialiser, because the surface is a handful of counters and the exposition format is stable text — so the dependency would buy formatting we can write once and cost a tree we then carry forever. This is what «minimal crates to keep redistribution clean» means when it is applied rather than stated. @status:impl/done

@fact:TOKIO-IS-NARROWED-NOT-FULL `tokio` is taken with four features — `signal`, `sync`, `time`, `fs` — not `full`. `full` is the shape a project reaches for before it knows what it uses; naming the four is the same discipline as the paragraph above, one level down. @status:impl/done

@fact:VIBE-CORE-DEP **`vibe-core` dependency.** `vibe-index` parses `vibe.toml` and `vibe-subskill.toml` through `vibe-core`'s own `Manifest` / `SubskillManifest` types, so the index can never drift from the manifest schema. This reverses the proposal's original standalone-no-`vibe-core` stance — [§6](#distribution) records the reversal, [§9](#open) item 11 the de-rot finding that forced it. What stays duplicated is small and stable: the four-variant `PackageKind` / `NamingConvention` (`src/types/kinds.rs`, frozen by `VIBEVM-SPEC.md` §4, needing the `Ord` + `clap::ValueEnum` the `vibe-core` originals lack) and the `compute_content_hash` algorithm (`src/content_hash.rs`, gated by `tests/content_hash_parity.rs`). `compute_content_hash` folds into `vibe-core` once it is lowered out of `vibe-registry`. @status:impl/done

@fact:THE-PARITY-GATE-RUNS-TWO-FIXTURES-IN-TWO-RECIPES **The parity gate is wider than one fixture and one algorithm.** It runs BOTH implementations over BOTH fixtures in BOTH recipes: `fixtures/golden-flow-wal-0.1.0/` is the ordinary package, and `fixtures/golden-order-trap-0.1.0/` is the tree built to make recipes 0 and 1 disagree — a directory whose name is continued by a sibling file at a byte below `/`, the only shape on which component-wise and byte-wise ordering part company ([PROP-002 §2.1](../vibe-registry/PROP-002-decentralized-registry.md#identity)). A single fixture would let the two implementations agree by accident on every tree that never exercises the difference, which is exactly how a hash regression once reached a consumer before any golden noticed. @status:impl/done

@fact:not-pulling-lead **Deliberately NOT pulling:** @status:spec/done

- @fact:not-pulling-db A database (SQLite / PostgreSQL). All state in RAM + flat files. @status:spec/done

### 3.3 Git access in the scanner {#git-access}

@fact:design-git-access `design r1` @status:impl/done

@fact:SCANNER-SHELL-OUT **Decision.** Use shell-out to `git` via `std::process::Command` for the scanner's read paths (`git tag`, `git show <ref>:<path>`, `git rev-parse <tag>`). Same path `vibe-registry::shell.rs` already follows. Rationale matches PROP-001 §2.1: shell-out works on every platform git works on, no per-host bindings to maintain. @status:impl/done

@fact:not-gix **Not** `gix` for v0: smaller dep tree wins. v1 may switch if perf demands and gix's read API matures further. @status:spec/done

### 3.4 Threading model {#threading}

@fact:design-threading `design r1` @status:impl/done

- @fact:THREAD-CLI-SYNC CLI mode: synchronous. tokio runtime is created only in `serve` subcommand. @status:impl/done
- @fact:THREAD-SERVER-ASYNC Server mode: tokio multi-thread runtime. Routes are async; `Arc<RwLock<Index>>` is `tokio::sync::RwLock` (async lock). @status:impl/done
- @fact:THREAD-WRITER-TASK <status stage="spec" state="void">Retired 2026-08-18 by measurement. It described a dedicated `index_writer` tokio task fed by an mpsc channel, so that fsync stalls would not block the request handlers. Neither the task nor the channel was ever built — measured as zero occurrences of both names in the crate, against a live control — and the problem they were designed for dissolved when a mutation became an append to the journal plus a reprojection. The heir is `##THE-MUTATION-IS-WRITTEN-BY-ITS-OWN-HANDLER` below. This tombstone stays so the old sentence's name is never reused and inbound links do not break.</status> @status:spec/void

@fact:THE-MUTATION-IS-WRITTEN-BY-ITS-OWN-HANDLER **Each mutating handler does the whole write itself, under the index's async write-lock:** replay the journal, project a probe, append the event, reproject, write the catalog, swap the in-memory index. There is no writer task and no channel to post to. The queue that the task was reaching for turned out to be unnecessary once mutations became journal appends: a handler holds the lock for one append plus one projection, and the operations it serialises are publish events rather than a hot path ([§2.17](#auto-publish) makes the same argument about awaiting the push). What IS serialised separately is publication — its own lock, and a blocking thread, because a git command must not run on the async executor. @status:impl/done

### 3.5 Configuration precedence {#config}

@fact:design-config `design r1` @status:impl/done

@fact:CONFIG-PRECEDENCE For every flag with a default, precedence is: explicit CLI flag > env var (`VIBE_INDEX_*`) > on-disk config (`<data-dir>/state/config.toml`, optional) > built-in default. Same shape `vibe show config` already uses on the consumer side. @status:spec/plan

@fact:THERE-IS-NO-PRECEDENCE-MACHINE-YET **None of that ladder exists.** There is no `config.toml` anywhere in the crate and no `VIBE_INDEX_*` family: a flag with a default gets it from its own declaration, full stop. Two environment variables do exist and neither is part of a precedence chain — `VIBE_INDEX_GIT` overrides the git binary the scanner shells out to, and `VIBE_LOG` is the logging lever `--log-level` folds into ([§2.11](#cli)). The requirement stands and the fork is `BACKLOG.md` B-086: build the ladder, or say plainly that this binary is configured by flags and two named variables. What must not survive is the middle state, where a document describes a resolution order an operator can neither use nor observe. @status:impl/done

---

## 4. Phase plan (slices) {#phases}

@fact:slices-lead Each slice = one or more conventional commits. The utility becomes useful at slice 5 (read endpoints + reindex from clones); the rest are progressive enhancements. @status:impl/done

### 4.1 Slice 1 — skeleton {#slice-1}

@fact:SLICE-1 `crates/vibe-index/` standalone crate with `Cargo.toml` + `src/main.rs` + `src/lib.rs`. clap dispatch with stub subcommands that all print "not yet implemented". `vibe-index --version` works. `tests/help_smoke.rs` passes. @status:impl/done

@fact:slice-1-commit Commit: `feat(services/vibe-index): skeleton crate + clap subcommand dispatch`. @status:impl/done

### 4.2 Slice 2 — types + persistence {#slice-2}

@fact:SLICE-2 `src/types/` (entry / repomd), `src/index/` (memory, persistence, primary, by_name, repomd). JTD schemas in `schemas/`. Atomic write protocol. `vibe-index init` works (writes empty `repomd.json` + empty `primary.jsonl`). `vibe-index dump` works. `vibe-index verify` works (checks file hashes). Round-trip tests. @status:impl/done

@fact:slice-2-commits Commits: @status:impl/done

- @fact:slice-2-c1 `feat(services/vibe-index): index entry + repomd schemas + JTD` @status:impl/done
- @fact:slice-2-c2 `feat(services/vibe-index): in-memory index + atomic persistence` @status:impl/done
- @fact:slice-2-c3 `feat(services/vibe-index): vibe-index init/dump/verify` @status:impl/done

### 4.3 Slice 3 — scanner + reindex from clones {#slice-3}

@fact:SLICE-3 `src/scanner/from_clones.rs` walks `<org-dir>/<repo>/.git` directories; `src/content_hash.rs` mirrors `vibe-registry::compute_content_hash`; `vibe-index reindex --from-clones` works against `fixtures/sample-org/`. Parity test against `vibe-registry`. Incremental mode = full for now (deferred to slice 7). @status:impl/done

@fact:slice-3-commits Commits: @status:impl/done

- @fact:slice-3-c1 `feat(services/vibe-index): content_hash parity with vibe-registry` @status:impl/done
- @fact:slice-3-c2 `feat(services/vibe-index): scanner — walk org-dir clones` @status:impl/done
- @fact:slice-3-c3 `feat(services/vibe-index): vibe-index reindex --from-clones` @status:impl/done

### 4.4 Slice 4 — read CLI subcommands {#slice-4}

@fact:SLICE-4 `get`, `list`, `search`, `capabilities`, `purls`, `outdated`. Inverted text index for search. JSON output for every subcommand. `cli_e2e.rs` covers each. @status:impl/done

@fact:slice-4-commits Commits: @status:impl/done

- @fact:slice-4-c1 `feat(services/vibe-index): inverted text index for search` @status:impl/done
- @fact:slice-4-c2 `feat(services/vibe-index): vibe-index get/list/search/capabilities/purls` @status:impl/done
- @fact:slice-4-c3 `feat(services/vibe-index): vibe-index outdated against a vibe.lock` @status:impl/done

### 4.5 Slice 5 — HTTP server (read-only) {#slice-5}

@fact:SLICE-5 `vibe-index serve --read-only`. axum app exposes `/healthz`, `/readyz`, `/v1/index/*`, `/v1/packages*`, `/v1/capabilities/*`, `/v1/purls/*`, `/metrics`. PID lock file. CORS open. `server_e2e.rs` covers each route. @status:impl/done

@fact:slice-5-commits Commits: @status:impl/done

- @fact:slice-5-c1 `feat(services/vibe-index): axum server skeleton + healthz/readyz` @status:impl/done
- @fact:slice-5-c2 `feat(services/vibe-index): GET /v1/index/* file routes` @status:impl/done
- @fact:slice-5-c3 `feat(services/vibe-index): GET /v1/packages query routes` @status:impl/done
- @fact:slice-5-c4 `feat(services/vibe-index): /metrics prometheus endpoint` @status:impl/done

@fact:MVP-MARK After slice 5: vibe-index is **independently usable** as a read-only server fed by `reindex --from-clones`. This is the "MVP" mark. @status:impl/done

### 4.6 Slice 6 — write CLI + write HTTP + auth {#slice-6}

@fact:SLICE-6 `vibe-index add` / `vibe-index remove`. HTTP `POST /v1/packages`, `DELETE /v1/packages/...`. Bearer-token auth via `<data-dir>/state/admin.tokens`. Write-side server-vs-CLI lock arbitration. @status:impl/done

@fact:slice-6-commits Commits: @status:impl/done

- @fact:slice-6-c1 `feat(services/vibe-index): vibe-index add/remove (CLI)` @status:impl/done
- @fact:slice-6-c2 `feat(services/vibe-index): bearer-token auth + admin.tokens loader` @status:impl/done
- @fact:slice-6-c3 `feat(services/vibe-index): POST/DELETE /v1/packages routes` @status:impl/done

### 4.7 Slice 7 — incremental reindex {#slice-7}

@fact:SLICE-7 `<data-dir>/state/checkpoint.json`. `vibe-index reindex --incremental --from-clones` walks the diff between checkpoint and current state. Test: full vs incremental produce identical output. @status:impl/done

@fact:slice-7-commit Commit: `feat(services/vibe-index): incremental reindex via checkpoint`. @status:impl/done

### 4.8 Slice 8 — `--from-github` mode {#slice-8}

@fact:SLICE-8 `reqwest`-based GitHub API walk. `--token-file FILE`. Rate-limit-aware backoff. Same shape as `--from-clones` from caller's POV. @status:impl/done

@fact:slice-8-commit Commit: `feat(services/vibe-index): reindex --from-github (REST API walk)`. @status:impl/done

### 4.9 Slice 9 — vibe-publish post-publish hook {#slice-9}

@fact:SLICE-9 `crates/vibe-publish/src/lib.rs::Publisher::publish` gains optional index POST after successful push. New env var `VIBEVM_INDEX_TOKEN_<HOST>`. New `[[registry]].index_url` / `[[registry]].index_token` fields in the project manifest. @status:impl/done

@fact:slice-9-commit Commit: `feat(vibe-publish): POST to registry index after publish (opt-in)`. @status:impl/done

### 4.10 Slice 10 — consumer-side integration {#slice-10}

@fact:SLICE-10 `crates/vibe-registry/src/multi_registry_resolver.rs` gains the index-aware fast path. Falls back transparently on 404 / connect-failure. Live e2e test against an index-equipped registry. @status:impl/done

@fact:slice-10-commit Commit: `feat(vibe-registry): consume registry index for resolve fast path (opt-in)`. @status:impl/done

### 4.11 Slice 11 — docs + manual-test smoke {#slice-11}

@fact:SLICE-11 `crates/vibe-index/docs/` filled in (operator-handbook, consumer-protocol, format). `manual-tests/M2.10-index-smoke.md` walks the live e2e: bootstrap an index from a fresh org dir, serve it, install a package through it, search it. @status:impl/done

@fact:slice-11-commits Commits: @status:impl/done

- @fact:slice-11-c1 `docs(vibe-index): operator handbook + consumer protocol + format reference` @status:impl/done
- @fact:slice-11-c2 `test: manual-test smoke for index bootstrap + consume` @status:impl/done

---

## 5. Test plan {#tests}

@fact:tests-lead Per slice (specifics in §4); cumulative state at GA: @status:impl/done

- @fact:TEST-UNIT **Unit:** every type round-trips through serde JSON / TOML; every CLI subcommand has at least one happy-path test; every server route has at least one happy-path + one auth-fail test. @status:impl/done
- @fact:TEST-INTEGRATION **Integration:** the byte-identity check moved out of the crate's own fixtures and into the campaign's golden corpus — `formats/corpora/index/e1/` holds a journal and the catalog it projects to, `tests/golden_corpus.rs` compares them, and `cargo xtask rebuild --check` tears the catalog down and rebuilds it from the journal. Incremental reindex applied to the same starting state is still byte-identical to a full one. The move is the point: the golden now lives beside the format registry that governs it rather than beside one consumer of it. @status:impl/done
- @fact:TEST-PARITY **Parity:** `tests/content_hash_parity.rs` runs both implementations over both fixtures in both recipes ([§3.2](#deps)), and asserts equality within each recipe. CI gates the merge if they diverge. @status:impl/done
- @fact:TEST-E2E **End-to-end:** `tests/server_e2e.rs` spawns the server in-process (axum's `oneshot` style), drives every documented route over HTTP, asserts response shapes. @status:impl/done
- @fact:TEST-CRASH **Crash recovery:** `tests/persistence_atomic.rs` simulates mid-write crash by failing the rename step; asserts the previous version remains readable. @status:impl/done
- @fact:TEST-HERMETIC **Hermetic, with no live tier at all:** every test runs without network, and the GitHub API walk is proved against a **mock REST server on a random port whose canned responses point at local bare repositories**, so `git clone` resolves entirely against the filesystem (`tests/from_github_e2e.rs`). The opt-in live run against the real registry this section once promised does not exist and is not owed: a test that needs the internet and a real organisation is one nobody runs, so it proves nothing on the day it would have mattered, while the mock proves the walk's shape on every commit. What it deliberately does not prove is that the real host still answers the way the mock does — that question belongs to the manual tier, not to an ignored test. @status:impl/done

---

## 6. Distribution — a workspace crate {#distribution}

@fact:design-distribution `design r1` @status:impl/done

@fact:WORKSPACE-MEMBER **Decision (revised 2026-05-22).** `vibe-index` lives at `crates/vibe-index/` as a member of the top-level vibevm workspace. It is built, tested, clippy-gated, and fmt-checked by the same `cargo … --workspace` invocations as every other crate, and it depends on `vibe-core` directly. @status:impl/done

@fact:fold-in-why **Why this reverses the original standalone-workspace decision.** The proposal first placed `vibe-index` in its own Cargo workspace under `services/`, outside `crates/`, so an org owner could vendor just that subdirectory. The cost was a hand-duplicated `vibe.toml` parser with nothing tying it to `vibe-core` — and, sitting outside `cargo test --workspace`, nothing routinely exercising it. It rotted silently against the M1.17 / M1.18 manifest-schema churn ([§9](#open) item 11). Folding the crate back in kills both failure modes at once: the scanner now parses through `vibe-core::Manifest` (one source of truth — the schema cannot drift), and the routine workspace gate covers it (drift is caught the moment it appears). @status:spec/done

@fact:REDISTRIBUTION **Redistribution.** An org owner who wants to host their own index server clones the vibevm repository and runs `cargo install --path crates/vibe-index`. The "vendor only the subdirectory" affordance is gone; in exchange the binary can never ship a stale view of the manifest schema. The HTTP-server deps (`axum` / `tower` / `tower-http`) enter the workspace `Cargo.lock` — `reqwest` was already there for the `vibe-registry` index client, so the marginal cost is `tower` / `tower-http` / `flate2`. @status:impl/done

@fact:GATE-COVERS **Gate.** `tools/self-check.sh` no longer special-cases a second workspace — the workspace-wide steps (fmt, then `cargo test --workspace`, then `cargo clippy --workspace --all-targets -- -D warnings`) cover `vibe-index` like any member. @status:impl/done

@fact:THE-PANEL-NOW-HAS-STEPS-THAT-LOOK-ONLY-AT-THIS-CRATE **What has changed since is the opposite of a special case: the panel grew steps that look *specifically* at this crate, and they are the interesting half.** A clock gate greps `crates/vibe-index/src/{index,types,journal}` for any call that reads the wall clock, because determinism here is an instrument rather than a preference ([§2.9](#server-mode)); `check-codegen` refuses a drift between the schemas and the generated types ([§2.12](#types)); `specmap --check` refuses a stale traceability map; and the wire-derive ratchet refuses a hand-written wire. None of them is a workspace step, and none of them would fire from `cargo test` alone — which is why the number of steps in the panel is not a proxy for what it checks. @status:impl/done

---

## 7. Auth, secrets, scope {#secrets}

@fact:req-secrets `req r1` @status:impl/done

@fact:SECRECY-INHERITED [PROP-000 §20](../../common/PROP-000.md#token-secrecy) covers the token-secrecy invariant; PROP-005 inherits it verbatim. Specifically: @status:impl/done

- @fact:SECRET-ADMIN-TOKENS **Server admin tokens** (`<data-dir>/state/admin.tokens`) — file mode 0600, never echoed in logs / responses / error messages, gitignored. @status:impl/done
- @fact:SECRET-GITHUB-TOKENS **GitHub API tokens** (`--from-github --token-file FILE`) — same discipline. Read once into memory, scrubbed from the env, never persisted outside the source file. @status:impl/done
- @fact:SECRET-INDEX-TOKENS **Index POST tokens** (`VIBEVM_INDEX_TOKEN_<HOST>` for the publish-side hook) — per-host shape mirrors `VIBEVM_PUBLISH_TOKEN_<HOST>`. @status:impl/done

@fact:SCOPE-DISCIPLINE **Scope discipline.** The server's mutation endpoints accept entries only for the registry the server was started with (`<data-dir>/repomd.json::registry`). A POST attempting to land an entry whose `registry` field disagrees with the server's configured registry → 400 with a clear message. Same shape `vibe-publish::validate_scope` enforces on the publish side. @status:impl/done

---

## 8. Operations {#ops}

@fact:ops-lead A typical setup for an org owner who wants to host an index: @status:impl/done

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

@fact:ops-consumers-note Most consumers see only the static raw-HTTP files; the server is for orgs that need real-time publish updates. @status:impl/done

---

## 9. Open questions {#open}

1. @fact:OPEN-LOCATION **Index file location: `<org>/index` repo vs `<org>/<package-repo>/index/...` per-package?** PROP-005 picks `<org>/index`. The alternative — per-package files inside each package repo — was rejected because it leaves catalog discovery a chicken-and-egg problem (you need to enumerate the org first). If new evidence emerges that orgs object to a top-level `index` repo (naming conflicts, permission boundaries), we revisit. @status:spec/work
2. @fact:OPEN-COMPRESSION **`primary.jsonl.gz` compression: gzip vs zstd?** v0 picks gzip — universally supported by every HTTP client; deterministic with `mtime=0`. v1 may add a `primary.jsonl.zst` alongside. @status:spec/work
3. @fact:OPEN-GPG **GPG signing of `repomd.json`?** Tracked here, not shipped in v0. Shape: `repomd.json.asc` next to `repomd.json`; consumers verify against a per-registry public key recorded in `[[registry]].pgp_key`. v1. @status:spec/work
4. @fact:OPEN-MERKLE **Merkle log (Go sumdb-style transparent log)?** Tracked here. v2+. Useful for adversarial environments; v0/v1 trust the host. @status:spec/work
5. @fact:OPEN-AUTO-PUSH **Auto-commit-and-push from server — ANSWERED and built 2026-08-06; the contract is [§2.17](#auto-publish).** The risk this question named — the server holding push credentials is a step up in trust — is unchanged and is why the flag stays opt-in with manual commit/push as the default. What the build added to the answer is a second risk the question had not seen: the credentials are not the only secret in reach, because the data directory also holds the server's own bearer tokens, and the publishing step is a `git add -A` away from them. Hence the startup refusal in §2.17 rather than a warning. @status:impl/done
6. @fact:OPEN-MULTI-REGISTRY **Multi-registry server** — should one server instance host multiple data dirs (one per registry)? v0 says no (one process per registry). Trivial scale-out via process supervision; we revisit if multi-tenancy demand emerges. @status:spec/work
7. @fact:OPEN-SSE **WebSockets / Server-Sent Events for live publish notifications** — out of scope. Polling `/v1/admin/status::last_reindex` is sufficient at our scale. @status:spec/work
8. @fact:OPEN-OCI **OCI registry shape** — could we host the index inside an OCI registry instead of git? Out of scope; revisit if the OCI tooling becomes universal among vibevm operators. @status:spec/work
9. @fact:OPEN-CAP-VS-PURL **Capability- vs PURL-driven search** — v0 ships `by-cap` and `by-purl` as separate files. If usage shows one dominates, the loser may be folded into the inverted text index. Empirical question. @status:spec/work
10. @fact:OPEN-RATE-LIMIT **Rate-limiting on the server** — shipped after the v0 plan: `server/rate_limit.rs` is a per-token and per-IP token-bucket limiter, disabled by default and opt-in by flag. Production deployments may still front it with a reverse proxy's limiter; the two compose. @status:impl/done

11. @fact:OPEN-STANDALONE-RESOLVED **Standalone-workspace duplication — RESOLVED 2026-05-22 by folding the crate in.** The 2026-05-22 de-rot found `crates/vibe-index/` had silently rotted: its duplicated `vibe.toml` parser still expected the pre-M1.17 shape (`[writes]`, `[dependencies]`, `[boot_snippet].filename`) and could not parse a current manifest, and its `content_hash` parity test had drifted off a fixture renamed by the M1.17 manifest unification. §3.2 had weighed the duplication cost for `compute_content_hash` alone ("the algorithm doesn't change"); the *manifest schema*, by contrast, churned hard through M1.17 / M1.18, and the duplicate parser had no cross-check to catch the drift — the standalone workspace also sat outside the routine `cargo test --workspace` gate. **Resolution:** fold `vibe-index` into the `crates/` workspace and parse through `vibe-core::Manifest` (see [§6](#distribution)). The duplicated parser is deleted; only the tiny, schema-frozen `PackageKind` / `NamingConvention` and the `compute_content_hash` algorithm remain duplicated, both justified in §3.2. @status:impl/done

---

## 10. Acceptance criteria {#acceptance}

@fact:acceptance-lead A given slice is considered accepted when: @status:impl/done

- @fact:ACC-TESTS All tests in its slice pass. @status:impl/done
- @fact:ACC-CLIPPY `cargo clippy --workspace --all-targets -- -D warnings` is clean. @status:impl/done
- @fact:ACC-FMT `cargo fmt --check` is clean. @status:impl/done
- @fact:ACC-SELF-CHECK `tools/self-check.sh` is green. @status:impl/done
- @fact:ACC-HELP-SMOKE Help-text smoke covers any new subcommand. @status:impl/done
- @fact:ACC-MANUAL-WALK A manual walk through `crates/vibe-index/docs/operator-handbook.md` succeeds against `fixtures/sample-org/`. @status:impl/done

@fact:CLOSURE-CRITERION PROP-005 is considered closed once slices 1–8 land. Slices 9–11 are integration with the rest of vibevm and ship under their respective milestone PRs. @status:impl/done

---

## 11. Wire-up scripts (informational, not shipped) {#wire-up}

@fact:wire-up-lead For operators wiring the index into their hosting: @status:spec/done

@fact:WIRE-POST-RECEIVE **git `post-receive` hook on the org's hosted git** (Forgejo/Gitea/GitVerse-style) — push to a package repo triggers an HTTP POST to the index server: @status:spec/done

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

@fact:WIRE-CRON **cron line:** @status:spec/done

```cron
*/5 * * * *  vibe-index reindex /home/owner/vibespecs-index --incremental --from-clones /var/lib/vibespecs-mirror >>/var/log/vibe-index.log 2>&1
```

@fact:wire-up-not-shipped Neither is shipped as a binary — operators integrate at their own host, and the hook shape varies enough across hosting platforms that one-size-fits-all isn't worth shipping. @status:spec/done

@fact:ONLY-THE-CRON-LINE-REACHED-THE-HANDBOOK **Of the two, only the cron line is in `crates/vibe-index/docs/operator-handbook.md`; the `post-receive` hook is documented here and nowhere else.** That is worth stating rather than quietly fixing, because the hook posts to `POST /v1/admin/reindex` — the route [§2.10](#http) records as specified and unbuilt. So the artefact that did not reach the handbook is the one that would not have worked from it, and copying it across before the route exists would turn a documentation gap into a support ticket. @status:impl/done

---

## 12. Version history {#history}

- @fact:HISTORY-DRAFT-1 **2026-05-06 — draft 1.** Initial proposal. Open for review. @status:spec/done
- @fact:HISTORY-RECONCILED **2026-05-22 — reconciled with the implementation, then folded into the workspace.** A state review found PROP-005 already implemented (slices 1–10 + M2.10 `vibe search`) but rotted; the de-rot realigned the scanner with the current `vibe.toml` schema and corrected this document (§2.6 `boot_snippet`, the `vibe.toml` filename, §2.10 rate-limiter status). The fold then moved `vibe-index` from its own `services/` workspace into `crates/vibe-index/` and switched it to parse through `vibe-core::Manifest` — §3.2, §6, and §9 item 11 are revised for the reversed standalone-workspace decision. @status:spec/done
- @fact:HISTORY-GROUP-NATIVE **2026-05-22 — group-native (PROP-008 Phase 7).** The index entry gained the mandatory `group` field and the optional `workspace_origin` (§2.6); the `by-name/` layer was re-keyed from `by-name/<kind>/<name>.json` to the candidate-set file `by-name/<name>.json` (§2.4) — one GET per registry now yields every group sharing a bare name; `primary.jsonl` / `by-cap` / `by-purl` sort on the `(group, name, version)` identity; the HTTP `/v1/packages/{group}/{name}` routes, the `vibe-index get/remove` CLI, and the `naming = "fqdn"` default followed. The `vibe-registry` index client and the `vibe-publish` post-publish hook were realigned to the new shape. PROP-008 §2.8's index extension is shipped. @status:spec/done
