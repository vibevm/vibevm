# PROP-002: Decentralized, mirror-friendly registry with capability-based depsolver {#root}

**Milestone:** M1.1-revision ([`ROADMAP.md`](../../../ROADMAP.md#m11-revision--decentralized-per-package-registry-active-started-2026-04-24)). Phase B lands in M1.6.
**Status:** accepted 2026-04-24.
**Supersedes (partially):** [PROP-001](PROP-001-git-backend.md) §2.3 (`Registry` trait), §2.4 (cache layout), §2.6 (lockfile `source_uri` format). PROP-001 §2.1 (shell-out-to-git), §2.2 (`GitBackend` trait), §2.5 (freshness TTL), §2.7 (Windows UX) remain authoritative.
**Related:** [spec://vibevm/common/PROP-000](../../common/PROP-000.md) (especially §15 — dep weight, §16 — JTD, §17 — production architecture, §18 — complexity ≥ RPM), [`VIBEVM-SPEC.md` §7](../../../VIBEVM-SPEC.md) (manifest / lockfile schemas), [`VIBEVM-SPEC.md` §8](../../../VIBEVM-SPEC.md) (registry).

---

## 1. Motivation {#motivation}

M1.1 shipped a monorepo-shaped registry: one git repository (`anarchic/vibespecs`) contained every package under `<kind>/<name>/v<ver>/` directories, `[registry]` in `vibe.toml` was a singleton URL, the lockfile recorded each package as `git+ssh://…#<kind>/<name>/v<ver>`. This was the cheapest shape to prove the end-to-end install loop worked.

It is the **wrong** shape for v1 shipping. The failure mode, named: Nix.

**Nix's failure pattern, precisely.** Nix's flake URL grammar hard-codes hosts (`github:owner/repo`, `gitlab:owner/repo`, `sourcehut:~owner/repo`); every new hosting platform is a `nix` core PR away. Nix's flake registry — the global namespace that maps `nixpkgs` to `github:NixOS/nixpkgs` — is itself hosted on GitHub. Every `flake.nix` in the ecosystem pins an absolute `github:` URL, so migrating `nixpkgs` to a different host would require rewriting every downstream `flake.nix` in the world. Mirror mechanisms (`nix registry add`) are redirects, not true indirection. Identity is URL-tied: `flake.lock` pins URL + rev, so even a transparent mirror causes lockfile churn. The 2022 Russian-maintainer GitHub freeze illustrated the blast radius — a hosting-platform policy decision rippled into the resolve path of the entire ecosystem.

**Underlying mistakes:** URL scheme tied to host, central index on one host, lockfile identity = URL (not content), no indirection layer, no first-class mirror support, naming decentralization never considered.

The shape vibevm ships instead:

- Each package is its **own** git repository — no monorepo. Per-package maintainer permissions are hosting-native (a package repo's owner controls access); no central merge queue.
- `[[registry]]` is an **array**, priority-ordered. `[[mirror]]` is a first-class fallback layer, transparent to the lockfile. `[[override]]` bypasses the resolver for pins. Schema and code path support all three from day one.
- Package identity is `(kind, name, version, content_hash)`. `source_url` is informational. Switching mirrors, migrating between hosts, reconciling with a fork — none of these touch identity.
- URL syntax is **just git URL** — `git@host:…`, `ssh://`, `https://`, `file://`. No `github:` / `gitverse:` shorthands. New hosts "just work" as long as `git` speaks to them.

This PROP locks those decisions. Implementation lands in two phases:

- **Phase A (M1.1-revision):** single live `[[registry]]`, structures support multi; mirror / override parsing present, runtime limited to one registry without mirrors; publish utility shipped.
- **Phase B (M1.6):** real multi-registry exercised end-to-end, mirror fallback chain, `vibe vendor`, richer publish adapters.

---

## 2. Decisions {#decisions}

### 2.1 Identity: content-addressed, URL-orthogonal {#identity}

**Decision.** A package's identity is the tuple `(kind, name, version, content_hash)`. The `content_hash` is `sha256:<hex>` over the deterministically-ordered concatenation of `(rel_path_bytes || 0x00 || file_bytes || 0x00)` for every file in the package directory (the existing scheme from `compute_content_hash` in `vibe-registry`). The URL used to fetch the content is **informational** — recorded in the lockfile for debuggability, not for identity.

**Consequence.** Fetching the same `(kind, name, version)` from two different URLs (canonical + mirror, original + fork, upstream + vendored copy) must produce the same `content_hash`. Mismatch is a fatal `IntegrityError`. The effect is:

- mirror-switching, host-migration, and vendoring never change the lockfile;
- a compromised mirror cannot silently substitute content — the mismatch triggers hard fail before any write;
- a force-pushed tag upstream is caught by the same machinery on the next install.

Escape hatch for legitimate mirror-vs-upstream divergence (e.g. during an upstream outage): `--trust-mirror` flag on `vibe install` / `vibe update`. Never silent; always operator-initiated.

### 2.2 Registry model: `[[registry]]` array, priority-ordered {#registry-model}

**Decision.** `vibe.toml` carries an array of registries:

```toml
[[registry]]
name   = "vibespecs"
url    = "git@gitverse.ru:vibespecs"
ref    = "main"                              # registry-level metadata ref (reserved; not used today)
naming = "kind-name"
```

- `name` — local alias, used in lockfile `registry` field and in `[[override]]` / `[[mirror]]` targeting.
- `url` — **organization root URL**, not a package repo URL. A registry is a hosting-org; packages are children of it.
- `ref` — reserved for a future registry-level metadata branch (e.g. capability index, trust policy). Not consumed today.
- `naming` — convention for mapping a pkgref to a package repo name under this org. Values: `"kind-name"` (default — `flow:wal` → `<org>/flow-wal`), `"name"` (if kind-name collisions are impossible in a given registry), `"kind/name"` (for hosts supporting nested repos). Other registries may ship with different conventions; the setting is per-registry, not global.

Resolution: the solver iterates registries in array order; the first that has a satisfying match for a pkgref wins. Versions of the same pkgref are **not** unioned across registries — this prevents a lower-trust registry from influencing resolve when a higher-trust one already has a valid answer.

### 2.3 Mirror layer: transparent, integrity-verified {#mirror}

**Decision.** `[[mirror]]` entries are parallel alternative URLs for a specific registry (or `*` for any). During fetch:

1. For the target registry, try mirrors in `priority` ascending order.
2. Fall through to the canonical `[[registry]].url` if all mirrors fail or return content whose hash disagrees with an existing lockfile pin.
3. The canonical URL is always recorded as the `source_url` in the lockfile when the fetch produces a new pin. Mirror URLs do not appear there.

```toml
[[mirror]]
of       = "vibespecs"      # or "*" to mirror any registry by default
url      = "https://mirror.internal/vibespecs"
priority = 1
```

Mirror integrity verification is **mandatory**, not optional. A mirror whose `content_hash` for `(kind, name, version)` differs from the lockfile pin fails the install with an actionable error. This closes the supply-chain hole where a hijacked mirror could substitute content.

Phase A: `[[mirror]]` parser and lockfile-canonical-URL invariant ship. Runtime fallback chain lands in Phase B, because we want to exercise it against a second *real* mirror, not just a constructed test fixture.

### 2.4 Overrides: surgical pin of source location {#override}

**Decision.** `[[override]]` bypasses the registry layer for a named pkgref:

```toml
[[override]]
pkgref     = "flow:wal"
source_url = "git@mycompany:forks/wal"
ref        = "my-fix-branch"                # tag, branch, or commit
reason     = "awaiting upstream PR #42"     # surfaces in `vibe list --overrides`
```

The resolver short-circuits: it does not consult `[[registry]]` for this pkgref at all; it fetches directly from the given URL at the given ref. Content hash is still pinned in the lockfile and verified on each install — an override does not relax integrity. The lockfile records `overridden = true` on that entry; `vibe list` gains an `--overrides` flag.

This is the vibevm analogue of Cargo's `[patch]` and Go's `replace`. Same shape, same use case: pinning a fork during an in-flight upstream PR, emergency hotfixes, internal forks of public packages.

### 2.5 Per-package layout: flat, tag-based {#layout}

**Decision.** A package repository contains the package content flat at the repository root:

```
<org>/<kind>-<name>.git
├── vibe-package.toml
├── README.md
├── boot/<NN>-<kind>-<name>.md    # optional
├── spec/…                        # mirrored into consumer project
└── …

tags: v0.1.0, v0.2.0, v1.3.0-rc.1
```

- Version = git tag with `v<semver>` prefix. Tag is immutable by convention.
- No versioned subdirectories inside the repo. A tagged checkout **is** the package content.
- Integrity of the tag is verified per §2.1 on every install; a force-pushed tag rewrite is caught as `IntegrityError`.

**Rationale for flat, not versioned subdirs:** tag-based is the idiomatic "git-as-package-source" shape (Go modules, Swift PM, many others). Consumers can browse a tag on GitVerse / GitHub and see exactly the package content, not navigate to a subdirectory. Authoring is natural — `main` branch is dev, tag `v0.x` is release.

### 2.6 Cache layout: organized by canonical registry URL {#cache}

**Decision.** The per-user registry cache is rooted at the **canonical registry URL**, not the mirror URL — a transparent mirror does not invalidate the cache:

```
~/.vibe/registries/
└── <canonical-url-hash>/
    ├── meta.toml                 # { canonical_url, last_mirror_used?, last_synced_at }
    └── packages/
        └── <kind>-<name>/
            ├── clone/            # per-package git working tree
            └── meta.toml         # { source_url_last_used, last_synced_at, last_known_tags[] }
```

- `<canonical-url-hash>` = lowercase hex of the first 16 bytes of `sha256(normalize(canonical_registry_url))`. Normalization per PROP-001 §2.4 (lowercase, trailing `.git` and `/` stripped).
- Outer `meta.toml` carries the full hash, the canonical URL, and — for diagnostic purposes — the URL of the last mirror that actually answered.
- Inner `meta.toml` (per package clone) carries the exact `source_url` that fetched this package last, its freshness timestamp, and the last set of tags observed by `ls-remote`.
- Fetching is lazy per pkgref: a project that installs only `flow:wal` from an org of 50 packages clones exactly one of them.

Freshness TTL per package repo is 1 hour (PROP-001 §2.5 carries over). `VIBE_REGISTRY_CACHE` env override applies.

### 2.7 Lockfile schema v2 {#lockfile}

**Decision.** `vibe.lock` gains `schema_version = 2` and the following record shape per package:

```toml
[meta]
generated_by      = "vibe 0.2.0"
generated_at      = "<RFC-3339 UTC>"
schema_version    = 2
solver            = "resolvo-<ver>"
root_dependencies = ["flow:wal", "stack:rust-cli"]

[[package]]
kind            = "flow"
name            = "wal"
version         = "0.3.0"
registry        = "vibespecs"                               # name from [[registry]]; "__override__" for override-resolved
source_url      = "git@gitverse.ru:vibespecs/flow-wal.git"  # canonical URL of the registry entry, NOT the mirror URL
source_ref      = "v0.3.0"
resolved_commit = "abc123…def"
content_hash    = "sha256:…"                                # identity per §2.1
boot_snippet    = "10-flow-wal.md"
files_written   = [ … ]
dependencies    = []                                        # resolved transitive deps, kind:name@=version
overridden      = false
```

- `schema_version = 1` (monorepo-era) is accepted read-only and auto-migrated to v2 on the next `vibe install` / `vibe update`, with a user-visible notice.
- `root_dependencies` distinguishes what the user directly asked for from what the solver pulled in transitively. `vibe uninstall` of a root dep removes it from that list and prunes orphaned transitives; `vibe uninstall` of a pure transitive is rejected with an explanation.
- `dependencies` field per package is **resolved** (exact version, not constraint) — the lockfile is the full resolved graph, not a constraint manifest.

### 2.8 Depsolver: resolvo primary, DepSolver trait for fallback {#solver}

**Decision.** The primary depsolver is the [`resolvo`](https://crates.io/crates/resolvo) crate (pure Rust, BSD-3-Clause-or-Apache-2.0, used by Pixi and Rattler at conda scale). Chosen for:

- **Feature completeness for complexity ≥ RPM (PROP-000 §18):** virtual packages, disjunctions, obsoletes-driven upgrades, boolean-style constraints, custom constraint operators.
- **Rust-native ergonomics.** Provider trait (`DependencyProvider` analog) maps cleanly onto our existing `Registry` / `MultiRegistryResolver` types; no FFI, no impedance mismatch, no C-toolchain dependency.
- **Active upstream, production scale.** Pixi resolves over the conda ecosystem (hundreds of thousands of packages) in production; active development at prefix-dev.

**Not** `pubgrub` — the algorithm does not handle virtual packages or disjunctions, undershoot relative to PROP-000 §18.

**libsolv as explicit fallback.** A `DepSolver` trait in the new `vibe-resolver` crate mirrors the PROP-001 §2.2 `GitBackend` pattern: primary impl is `ResolvoSolver`; a future `LibsolvSolver` (FFI to C libsolv, BSD-3-Clause) drops in as a feature-gated alternative if resolvo ever hits a ceiling we can't raise. Swap cost: one impl block, one factory line. PROP-000 §15 (dep-weight not an argument) removes the size-based objection; PROP-000 §18 explicitly contemplates the switch if complexity demands.

The `lockfile.meta.solver = "resolvo-<ver>"` field records the solver identity so a future lockfile produced by `libsolv` is distinguishable, and a lockfile produced by an older resolvo can be re-verified by the same solver version when integrity investigation matters.

### 2.9 Capability-based deps: `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` {#capability}

**Decision.** The package manifest gains the full capability-based dependency vocabulary, pinned in [`VIBEVM-SPEC.md` §7.3](../../../VIBEVM-SPEC.md). Summary:

- `[provides].capabilities = ["<namespace>:<name>[@<semver>]", …]` — abstract capabilities the package advertises.
- `[requires].packages = ["kind:name@<constraint>", …]` — concrete pkgref requirements.
- `[requires].capabilities = ["<namespace>:<name>[@<constraint>]", …]` — satisfied by any package that provides that capability.
- `[[requires_any]] one_of = [ pkgrefs… ]` — disjunction; exactly one must be satisfied. Repeatable table for multiple independent disjunctions.
- `[obsoletes].packages = [ pkgrefs… ]` — the package supersedes these; the solver flags them for removal on upgrade.
- `[conflicts].packages = [ pkgrefs… ]` — mutually exclusive installs.

**Legacy compact form** (`[dependencies] required = [...] conflicts = [...]`) remains parse-compatible — auto-migrated at read time to the new fields with a one-time deprecation warning. Three existing `vibe-package.toml`-s in the fixtures / live registry all declare empty `required` and `conflicts`, so no live data migration is forced.

Semantic: the solver computes a satisfying assignment over the declared constraints. Conflict-detection renders through resolvo's native conflict-explanation surface — produced as a human-readable chain of incompatibilities, not a stack trace.

### 2.10 Publish utility: `vibe registry publish <path>` {#publish}

**Decision.** Ship a maintainer utility in v1. Scope: mechanical-only publish — **create repo, push contents, tag version**. Semantic review (LLM-backed safety analysis per `VIBEVM-SPEC.md` §8.5) remains v2+.

Architecture:

- New crate `vibe-publish` in the workspace.
- Core trait `RepoCreator`:
  ```rust
  pub trait RepoCreator {
      fn create(&self, org: &str, repo_name: &str, opts: &CreateOpts) -> Result<RepoInfo, PublishError>;
      fn exists(&self, org: &str, repo_name: &str) -> Result<bool, PublishError>;
  }
  ```
- First concrete impl: `GitVerseCreator`. Hits the GitVerse public API ([docs](https://gitverse.ru/docs/public-api/)). Auth: token from `~/.vibevm/git.publish.token` or `VIBEVM_PUBLISH_TOKEN` env-var.
- Host adapters for GitHub / Gitea / Forgejo are Phase B additions; pattern is pinned now so adding each is a single impl block.

**Error surface** (tuned for non-admin contributors — PROP-000 §18 acknowledges this will hit routinely):

- `401` / `403` from the API → `Publish refused: token lacks 'repo:create' permission in organization <org>. Contact an org owner or use a token with broader scope.`
- `git push` denied → `Publish refused: no push access to <repo>. Ask a maintainer of <repo> to grant you push access.`
- Tag already exists → `Publish refused: <repo> already has tag <tag>. Pick a new version — force-push is not automated.`
- Org does not exist / network unreachable → differentiated from auth errors so operators can tell a typo from a permissions issue.

Never force-push. Never overwrite an existing tag. Never create a repo in a different org than the configured one unless `--org <other>` is passed explicitly.

### 2.11 JTD + codegen for wire contracts {#jtd}

**Decision.** Per [PROP-000 §16](../../common/PROP-000.md#jtd), wire-format contracts — here, the GitVerse API request/response shapes, the `vibe --json` CLI output event schema, and future LLM provider wire shapes — are defined in JTD and codegen'd into Rust types via `jtd-codegen`.

Layout:

- `tools/jtd-codegen/` — vendored `jtd-codegen` binary (gitignored; version pinned via README).
- `schemas/` — `.jtd.json` files at repo root, committed. One file per contract.
- `crates/vibe-wire/` — new crate housing `schemas/` → Rust codegen output in `src/generated/`, re-exports curated for downstream crates (`vibe-publish`, future `vibe-llm`).
- `cargo xtask codegen` — regenerates every schema. CI runs it and fails on diff.

Manifests (`vibe.toml`, `vibe.lock`, `vibe-package.toml`) stay TOML and serde-driven — JTD is for wire, not for human configs.

### 2.12 Performance and resolver I/O strategy {#perf}

**Decision.** The resolver is driven by a `DependencyProvider` adapter that sits on top of `MultiRegistryResolver` and exposes only what resolvo needs:

- `list_versions(pkgref)` → backed by `ShellGit::list_tags(repo_url)` — a new method using `git ls-remote --tags <url>` to enumerate versions **without cloning**.
- `get_dependencies(pkgref, version)` → backed by `ShellGit::fetch_file_at_ref(repo_url, tag, "vibe-package.toml")` — a new method using `git archive` to pull a single file from a tag **without a working tree**.

In-memory caching across one `vibe install` invocation: `(registry, kind, name) → Vec<Version>` and `(registry, kind, name, version) → Deps`. Same pkgref touched twice within a resolve pass hits memory, not network.

Parallel I/O via `rayon` scoped thread pool: N parallel `ls-remote` subprocess invocations when resolving a multi-dep top-level install. No tokio introduction — our runtime is synchronous, `git` subprocesses are blocking, and a thread pool for blocking-I/O fan-out is exactly rayon's sweet spot.

A full clone happens only once the solver has committed to installing a specific version. The pathological case (resolver evaluates 50 versions of a package, then picks one) clones once, not 50 times.

Future (Phase B): resolved-graph cache keyed by `sha256((vibe.toml-registries) || all-requires || all-overrides)`. A second `vibe install` in the same project without any input change hits the cache, skipping resolve entirely. Listed here so the cache-key grammar is chosen once, not retrofitted.

---

## 3. Rejected alternatives {#rejected}

### 3.1 libsolv as primary solver

Rejected for primary role — accepted as explicit fallback (§2.8). libsolv is battle-tested at RPM scale (DNF, zypper, SUSE) and its feature set is a superset of what we need. But its API is C-style Pool / Solvable / Transaction semantics, and modelling vibevm's pkgref / capability types in Solvable form is an ongoing impedance mismatch that would live in the codebase forever. `resolvo` is a generic Rust-native solver with the same feature coverage for our problem, so the Rust-native path wins on architectural consistency. The libsolv slot exists so we can switch if resolvo ever proves inadequate.

### 3.2 pubgrub as primary solver

Rejected. PubGrub is an excellent algorithm (first-class error messages, used by `uv` and others), but it does not handle virtual packages or disjunctions — and PROP-000 §18 expects both from day one. PubGrub may still be used in the future for explanatory rendering of conflicts in CLI output if its incompatibility-chain format proves superior; not as the authoritative solver.

### 3.3 Registry union (merge versions across all registries)

Rejected. In a multi-registry setup, unioning versions from all `[[registry]]` entries into one candidate list requires trusting all of them equally — a lower-trust registry could influence resolution when a higher-trust one already had a valid answer. Priority-ordered resolution (§2.2) gives operators clear control: the registry at array index 0 is always preferred, subsequent ones are fallbacks.

### 3.4 Per-registry identity (pkgref includes registry)

Rejected. Treating `vibespecs/flow:wal` and `corporate/flow:wal` as different identities makes mirror-switching impossible and forces every consumer's `vibe.toml` to pin a specific registry for every dep. Identity is package-level (kind, name, version, content_hash); the registry is a runtime resolution detail.

### 3.5 `github:` / `gitverse:` / `gitlab:` URL shorthands

Rejected. This is **the** Nix failure mode (§1). Every host-specific shorthand in the URL grammar is an invitation to tie the ecosystem to that host. vibevm accepts any URL that `git` accepts — full stop. Operators type full URLs; the CLI does not do host-specific magic.

### 3.6 Central `vibevm/index` registry of registries

Rejected. A global index would reintroduce the Nix centralization problem one level up. Registries are peer-level; operators configure their own `[[registry]]` array per project. Discovery of "what registries exist" is out of CLI scope (it's a search-engine / documentation problem, not a package-manager problem).

### 3.7 Optional / recommended / supplemental deps in v1

Rejected for v1 (not rejected forever). `required` + `conflicts` + capabilities + disjunctions + obsoletes cover every concrete use case we've identified. `recommends` / `suggests` / `supplements` (RPM's weak-deps) can be added later as extensions to `[requires]` — the fields slot in without breaking the schema.

### 3.8 HTTPS-only hosted-registry API

Rejected for v1. Git-over-SSH is what works today on contributor machines with existing GitVerse keys; HTTPS with token auth is a Phase B / M2 add. The `GitBackend` abstraction makes this additive, not architectural.

---

## 4. Out of scope for Phase A {#out-of-scope}

- Polished multi-registry UX (`vibe registry add / list / remove / set-mirror` commands) — Phase B.
- Live multi-registry exercised end-to-end — Phase B.
- Real mirror fallback chain (code paths exist, tested with fixtures; live mirror comes in Phase B).
- `vibe vendor` — Phase B.
- Publish adapters beyond GitVerse — added per adopter demand.
- Signed / attested packages (sigstore-style) — M2 design, noted as architectural allowance here.
- LLM-backed publish review — v2 (already spec'd in `VIBEVM-SPEC.md` §8.5).
- `--offline` install mode — M2 polish.

---

## 5. Acceptance (Phase A) {#acceptance}

Phase A is code-complete when every item below is green:

- [ ] `VIBEVM-SPEC.md` §7.3 / §7.4 / §7.5 / §8.1 / §8.2 / §8.3 / §8.4 / §8.6 reflect the new design. PROP-001 carries its super-session marker. TASKS.md / ROADMAP / WAL updated.
- [ ] `jtd-codegen` vendored under `tools/`; `schemas/` seeded with at least the GitVerse publish-API contract and the `vibe --json` event shape; `cargo xtask codegen` regenerates; CI enforces zero drift.
- [ ] `vibe-core` parses `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` into typed values; legacy compact form auto-migrates with a deprecation warning.
- [ ] `vibe.toml` parser accepts `[[registry]]` array, `[[mirror]]`, `[[override]]`; singleton legacy `[registry]` auto-migrates.
- [ ] `vibe.lock` schema v2 written by fresh installs; v1 lockfiles read and migrated on next write.
- [ ] `vibe-resolver` crate exists with `DepSolver` trait and `ResolvoSolver` impl; unit-tested against constructed dep graphs including a virtual-package and a disjunction case.
- [ ] `GitPackageRegistry` replaces `GitRegistry`; `MultiRegistryResolver` coordinates the array; `ShellGit::list_tags` and `ShellGit::fetch_file_at_ref` implemented.
- [ ] Content-hash integrity verified on every fetch; cross-source mismatch fails hard with actionable message.
- [ ] `vibe install` runs transitive resolution; plan rendering shows the full subgraph with `(dep of flow:foo)` provenance tags; `--dry-run` surfaces the same.
- [ ] `vibe-publish` crate with `RepoCreator` trait and `GitVerseCreator` impl; `vibe registry publish <path>` subcommand; non-admin error paths render per §2.10.
- [ ] Local fixtures relocated from `packages/` to `fixtures/registry/` in per-package layout; `cli_e2e.rs` updated.
- [ ] Three demo packages live in `vibespecs/flow-wal` / `vibespecs/flow-sync-from-code` / `vibespecs/flow-atomic-commits` via `vibe registry publish`, each tagged `v0.1.0`.
- [ ] `manual-tests/M1.5-gate-v2-per-package-smoke.md` written and passes against the live per-package registry.
- [ ] `cargo test --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` clean.

---

## 6. Phase B preview (M1.6) {#phase-b}

Pinned here so Phase A does not accidentally foreclose any of these options:

- Real multi-registry: a second live `[[registry]]` exercised end-to-end; priority ordering verified in smoke-test.
- Mirror fallback chain: second live mirror URL; integrity hard-fail; `--trust-mirror` escape hatch.
- `vibe vendor [--out <dir>]`: generates a local mirror directory, usable as `file://` `[[mirror]]`.
- CLI management surface: `vibe registry add / list / remove / set-mirror / status`.
- Publish adapters: GitHub, Gitea, Forgejo on adopter demand — one new `RepoCreator` impl per host.
- Resolved-graph cache: incremental resolve skipped when inputs unchanged.
- Supply-chain attestation: sigstore-style signing of tags; consumer verification on install. Architectural slot only in Phase B.

---

## 7. Open questions {#open}

None blocking Phase A. Parking lot:

- **Registry-level metadata ref.** `[[registry]] ref = "main"` is reserved for a future capability index / trust policy branch at the org level. Design deferred until use case emerges (likely Phase B or M2).
- **Cross-registry content-hash cache.** Today each `(registry, kind, name, version)` gets its own cache entry. If two registries mirror the same content (same content_hash), the second fetch does redundant work. Optimization — Phase B.
- **Registry-level naming beyond `kind-name` / `name` / `kind/name`.** Real-world adopters may want custom mappings (e.g. `pkg-<kind>-<name>`). If that arises, `naming` becomes a template string. Not speculative engineering today — react to first real request.
- **Solver-level lockfile verification.** The lockfile records `solver = "resolvo-<ver>"`. Should `vibe install --verify-solver` re-run resolution and assert the graph matches the lockfile? Useful audit tool. Phase B.
- **JTD codegen ergonomics on Windows.** `jtd-codegen` is a Go binary; we vendor it project-local. Whether the experience is clean enough to not require PATH tinkering will be answered during the tooling commit.
