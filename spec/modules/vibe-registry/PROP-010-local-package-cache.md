# PROP-010: The local package cache — a shared offline store {#root}

**Milestone:** design proposal; implementation follows [PROP-008](PROP-008-qualified-naming.md) (qualified naming, `M1.19`), on which the identity-keyed cache depends — provisionally `M1.20` (owner to confirm in [`ROADMAP.md`](../../../ROADMAP.md)). Not implementation-locked.
**Status:** DRAFT — requirements captured in owner discussions on 2026-05-21; draft 2 adopted the cache-keying and user-config decisions (§2.3, §2.4). The remaining §5 open questions need an owner design session before implementation.
**Related:** [PROP-002](PROP-002-decentralized-registry.md) (the decentralized registry — `[[registry]]`, `[[mirror]]`, `[[override]]`, and the registry cache this PROP elevates); [PROP-008](PROP-008-qualified-naming.md) (qualified naming — the registry-independent package identity the cache is keyed by); [PROP-009](../vibe-workspace/PROP-009-loading-model.md) (the loading model — `vibedeps/`, materialisation, `vibe.lock`); [PROP-005](../vibe-index/PROP-005-package-index.md) (the package index — offline search); [PROP-007](../vibe-workspace/PROP-007-workspace.md) (workspaces — members).
**Owner sanction:** this PROP extends `VIBEVM-SPEC.md` §8.3 (cache layout), §9 (CLI surface), and §9.5 (the user-level config layer). The spec edits land at implementation time and require explicit owner sanction — not yet granted; this PROP is the requirements record.

---

## 1. Motivation {#motivation}

vibevm already keeps a registry cache (`VIBEVM_REGISTRY_CACHE` / `vibe_registry::default_cache_root()`): registry repositories are cloned there, and `vibe install` / `vibe update` fetch through it. But the cache is an *implementation detail* — an opaque download accelerator, not a deliberate, inspectable, first-class store. There is no `--offline` mode: every `vibe install` / `vibe update` that re-resolves walks the network ([PROP-009](../vibe-workspace/PROP-009-loading-model.md) established that `vibe install` always re-resolves). A developer behind an air-gap, on a slow link, or simply wanting fast deterministic iteration has no way to say *resolve against what I already have*.

The model is the Maven `~/.m2` repository: a **machine-global, accretive package store** that resolution can run against with no network. vibevm adapts it. The crucial adaptation is that vibevm already commits `vibedeps/` (PROP-009 §2.1) — so a *consumer* of a project is already fully offline: a fresh clone boots and reads its spec corpus with no `vibe install` at all. The cache is therefore not for consumers. It is for **authors**, and — the headline of this PROP — for **work that does not exist yet**.

A developer (or an agent) who has used `flow:wal` once, in any project on the machine, should be able to create a *new module* inside a workspace, or `vibe init` an *entirely new project*, that depends on `flow:wal` — and install it with no network. The cache accretes across every project on the machine; new work draws from it. This is the property that makes the Maven local repository load-bearing, and it matters doubly for vibevm's agent use case: an agent that rapidly scaffolds modules and projects turns a per-scaffold network round-trip into a local copy. Without this, every new module is gated on the network; with it, the machine's accumulated corpus is instantly reusable.

---

## 2. Decisions {#decisions}

### 2.1 The cache is a machine-global, accretive store {#global}

**Decision.** The package cache is **machine-global**, not project-scoped — one store per machine, at a default path, overridable by `VIBEVM_REGISTRY_CACHE` (the existing env-var) and by a user-config key. Every package fetched for *any* project populates it; *any* project — including projects and members that do not yet exist — resolves and materialises from it.

The cache is **accretive**: a package version, once cached, is never evicted automatically. Versions are immutable (PROP-002), so a cached version is permanently valid; accretion is the point. Reclaiming space is an explicit operator action (§2.8), never a surprise.

This is largely true of `default_cache_root()` already; PROP-010 makes it **explicit, documented, and load-bearing** rather than incidental.

### 2.2 The cache serves work that does not exist yet {#scaffolding}

**Decision.** The cache is designed to serve **new modules and new projects**, not only dependency changes in an existing project. This is a first-class requirement, not an emergent side effect.

- **A new workspace member** (PROP-007) declares its own `[requires]`. Unified resolution (PROP-009 §2.7) folds it into the workspace graph; with a warm cache and `--offline` (§2.5) the member's dependencies resolve and materialise with no network.
- **A new project** — `vibe init` followed by `vibe install` — resolves its `[requires]` from the same machine-global cache. A package pulled for an earlier, unrelated project is immediately reusable; the new project never re-downloads it.

The mechanism rests on three decisions below: the cache is machine-global (§2.1), keyed by package identity so it is registry-config-independent (§2.3), and reachable offline (§2.5–§2.6) — and a new project inherits coherent registry configuration automatically (§2.4). PROP-010's job is to **guarantee and name** this workflow as a supported, first-class path. For an agent scaffolding many modules or projects in one session the cache becomes the dominant fast path: the first use of a package downloads it; every later module or project draws the cached copy.

### 2.3 The cache is keyed by package identity {#identity}

**Decision.** The cache is keyed by **qualified package identity** as defined by PROP-008 — not by registry URL. A cached package version is addressed by its identity (`group` / `name` / `version`) and validated by `content_hash`; the registry that served it is not part of the key.

- A package version pulled once is reusable by every project on the machine **regardless of which `[[registry]]` each project configures** — a mirror, a different organisation hosting the same package, or a redirect target all resolve to the same cache entry when the identity matches. This is what makes §2.2 seamless: offline resolution and materialisation become registry-config-independent — a new project draws on the cache by package identity, not by reproducing some earlier project's registry list.
- `content_hash` is the integrity gate: a cache entry is valid only if its content hashes to the recorded hash. Two sources claiming the same identity with divergent bytes are a collision, surfaced per PROP-008's collision rules, never silently merged.
- **Dependency.** Identity-keying requires PROP-008 (qualified naming) to be implemented — `group` and the qualified identity must exist first. PROP-010 is therefore sequenced *after* PROP-008: the cache is identity-keyed from the start, with no URL-keyed interim to migrate later (§6).

### 2.4 User-level default registry configuration {#user-registries}

**Decision.** A **user-level default registry configuration** — `[[registry]]` (and `[[mirror]]`) entries in the existing user config (`~/.config/vibe/config.toml`, the `UserConfig` layer that already promotes `[env]` per `VIBEVM-SPEC.md` §9.5). It supplies registry configuration when no project does, and seeds a new one:

- `vibe init` seeds a new project's `[[registry]]` blocks from the user-level default instead of the hardcoded `vibespecs` defaults. A developer or organisation sets its registries once, machine-wide, and every new project inherits them. Absent any user-level config, `vibe init` falls back to today's hardcoded defaults — backward-compatible.
- `vibe cache add` (§2.8) and other registry operations invoked outside any project use the user-level registries as their source.
- A new member added to a workspace already inherits the workspace's registries (resolution is unified at the root, PROP-009 §2.7); the user-level default matters at the *project* boundary — the new-project case — and for project-less invocations.

Project-level `[[registry]]` always overrides the user-level default — the same precedence the `UserConfig` `[env]` layer already follows (the project / live value wins). Identity-keying (§2.3) makes the *offline* path registry-independent; the user-level default makes a new project's *online* operations and *pre-warming* coherent without hardcoding or hand-editing. The two decisions are the offline and online halves of §2.2.

### 2.5 `--offline` — the network-forbidden policy {#offline}

**Decision.** A global `--offline` flag forbids all network access for the invocation. It resolves through the established CLI config layering — flag, then a `VIBE_OFFLINE` environment variable, then a user-config `[net]` key; the flag wins. This mirrors the resolved-posture pattern already used for `--unattended` / `VIBE_UNATTENDED` (`output::resolve_unattended`).

Under `--offline`, resolution and fetch must be satisfiable entirely from local sources — the cache (§2.7), `[[mirror]]` entries with a `file://` URL, and the project's own `vibe.lock` + `vibedeps/`. Anything not available locally is a **hard error with an actionable message**: it names the missing package and version and tells the operator how to recover (run once online, `vibe cache add`, or `vibe registry vendor`). `--offline` never silently degrades to a partial result.

Online remains the default and is unchanged: it walks the network for freshness and populates the cache as it goes. `--offline` is purely additive.

### 2.6 Offline resolution {#resolution}

**Decision.** The resolver gains an offline mode — `MultiRegistryResolver::with_offline(true)`, a builder method beside the existing `with_strict_auth`. Offline resolution reads version lists and manifests from the cache, addressed by package identity (§2.3), and never runs `git fetch` / `git ls-remote` / archive fetch.

Offline resolution is therefore computed against the cache **as of its last refresh**. This is correct and expected — Maven `mvn -o` and `cargo --offline` have the same property — but it must be explicit: a `--offline` resolve may pick an older version than an online resolve would. The companion is `vibe registry sync` (already implemented), the deliberate "refresh the cache while the network is available" step. The intended workflow is `vibe registry sync` online, then `vibe install --offline` later — the analogue of `mvn` then `mvn -o`.

There is a strong synergy with the deferred *skip-resolution-when-fresh* optimisation (when `vibe.lock` is already consistent with every node's `[requires]`, no resolution runs at all, so no network is touched): once that lands, the common path is offline-clean for free, and `--offline` governs specifically the resolution path taken when dependencies genuinely changed. The two should be designed together.

### 2.7 Cache layout and population {#layout}

**Decision.** The cache is keyed by package identity (§2.3) and carries a **local index view** — identity → versions present — so the resolver and the management commands (§2.8) answer cache queries without walking the whole store. The on-disk layout — per-identity extracted directories versus git clones indexed by identity — is an open question (§5.1); identity-keying leans toward extracted, version-keyed directories that map one-to-one onto identity.

The cache fills as a side effect of any online `vibe install` / `vibe update` / `vibe registry sync`, and by deliberate pre-warming (`vibe cache add`, §2.8). It is never auto-evicted (§2.1).

### 2.8 Cache management surface {#management}

**Decision.** The cache becomes operator-visible through a command family. The namespace — top-level `vibe cache` versus `vibe registry cache` — is an open question (§5.2), with a leaning toward **top-level `vibe cache`**: the cache is machine-global and serves work with no project at all (a not-yet-created project has no `[[registry]]` config to hang a `vibe registry` subcommand on).

- `vibe cache path` — print the cache root.
- `vibe cache list` — the packages and versions present locally; the offline-resolvable inventory.
- `vibe cache add <pkgref>…` — deliberately pre-warm: fetch a package and its dependency closure into the cache while online, so a later `--offline` run finds it. The "I am about to go offline, pull down what I will need" workflow. It fetches from the project's `[[registry]]` when run inside a project, otherwise from the user-level registries (§2.4).
- `vibe cache clean` — reclaim space: all, by age, or by package.

These complement, and do not replace, the existing `vibe registry sync` (refresh the cache) and `vibe registry vendor` (export a project's locked set to a `file://` mirror — see §6).

### 2.9 Layering — the cache, `vibedeps/`, and the lockfile {#layering}

**Decision.** PROP-010 changes none of the three existing layers; it makes their relationship explicit.

- **The cache** — machine-global, accretive, identity-keyed, the *source* of package content. Shared across every project on the machine.
- **`vibedeps/`** — per-project, committed, the *materialised* dependency content for that project's locked resolution (PROP-009 §2.1). Produced by copying from the cache.
- **`vibe.lock`** — per-project, the pinned resolution (PROP-009).

An offline `vibe install` of a new project resolves `[requires]` against the cache, then materialises each resolved package by copying from the cache into the new project's `vibedeps/`. No layer is bypassed; the cache simply becomes a first-class, offline-capable, registry-independent source feeding materialisation.

---

## 3. Command and crate surface {#surface}

- A global `--offline` flag (and `VIBE_OFFLINE`) on the `vibe` CLI (§2.5).
- `vibe cache path` / `list` / `add` / `clean` (§2.8).
- `vibe-core` — the `UserConfig` schema gains a `[[registry]]` / `[[mirror]]` section and a `[net]` key (§2.4, §2.5).
- `vibe-registry` — the identity-keyed cache and its local index view, `MultiRegistryResolver::with_offline(...)` (§2.3, §2.6, §2.7). Depends on PROP-008's identity types.
- `vibe-cli` — flag wiring, the resolved offline posture, `vibe init` seeding registries from the user-level default, the `vibe cache` commands, actionable cache-miss errors.
- `vibe registry sync` / `vibe registry vendor` — unchanged; documented as the cache's refresh and export companions.

---

## 4. Migration {#migration}

The existing registry cache is keyed by registry URL; the identity-keyed cache (§2.3) is a different layout. The existing cache is **abandoned, not migrated** — a cache is reconstructible from registries, never authoritative data. The first run on the new layout repopulates from the network; the stale URL-keyed directory can be removed by hand or by a one-shot cleanup. A single re-download is an acceptable one-time cost for a pre-release tool, and it avoids carrying a re-keying migration path that would exist only once.

Everything else is additive: a project that never passes `--offline` and sets no user-level registry config sees identical behaviour.

---

## 5. Open questions {#open}

1. **Cache layout** — per-identity extracted directories, or git clones indexed by identity? Extracted maps cleanly onto identity and materialises faster; clones carry every version and git-level integrity for free but duplicate what extraction would hold.
2. **Command namespace** — `vibe cache …` (top-level, project-independent) versus `vibe registry cache …`.
3. **Staleness signalling** — should an `--offline` run warn when the cache is older than some threshold, or when an online resolve would likely differ?
4. **Eviction** — pure manual `vibe cache clean`, or an optional size cap / LRU?
5. **Scaffolding UX** — should `vibe init` and new-member creation actively report "your declared `[requires]` are fully cached — you can work offline", or stay silent?

Resolved in draft 2: cache keying (§2.3 — keyed by PROP-008 package identity) and the project-less registry source (§2.4 — a user-level default registry configuration).

---

## 6. Rejected / deferred alternatives {#rejected}

- **Make offline the default, auto-detecting the network.** Rejected — implicit mode-switching makes a build's inputs unpredictable. Online stays the explicit default; `--offline` is an explicit opt-in. (The *common* path still avoids the network once skip-resolution-when-fresh lands — but by being a no-op, not by guessing.)
- **A URL-keyed cache now, re-keyed to identity later.** Rejected — it would carry a one-time re-keying migration for no benefit. PROP-010 is instead sequenced after PROP-008 (§2.3) so the cache is identity-keyed from day one.
- **Replace `vibe registry vendor` with the cache.** Rejected — they solve different problems. `vendor` exports *one project's locked set* to a portable `file://` mirror for handing to an air-gapped machine or another person. The cache is the *machine-local accretive store* of everything that machine has used. Both stay.
- **A project-scoped cache.** Rejected — it defeats §2.2 entirely. A per-project cache cannot serve a project that does not exist yet.

---

## 7. Phase plan {#phases}

Sequenced after PROP-008 (M1.19), on which §2.3 depends.

1. **The identity-keyed cache** — the cache keyed by PROP-008 package identity; a documented, stable layout (§5.1); the local index view; `vibe cache path` / `vibe cache list`. `vibe-registry` + `vibe-cli`.
2. **User-level default registry configuration** — `[[registry]]` / `[[mirror]]` in `UserConfig`; `vibe init` seeds from it; project config overrides. `vibe-core` + `vibe-cli`.
3. **`--offline`** — the global flag, `VIBE_OFFLINE`, the resolved posture; `MultiRegistryResolver` offline mode (resolve from the cache, never touch the network); actionable cache-miss errors.
4. **Pre-warm + clean** — `vibe cache add` (deliberate population) and `vibe cache clean`.
5. **Scaffolding integration** — guarantee a new project (`vibe init` + `vibe install --offline`) and a new workspace member resolve and materialise from the cache, end to end; the §2.2 workflow plus any §5.5 UX hint.
6. **Docs + `VIBEVM-SPEC.md`** — §8.3 / §9 / §9.5 edits under owner sanction; a `docs/` page for the cache and offline mode.

---

## 8. Version history {#history}

- **2026-05-21 — draft 1.** Requirements captured in an owner discussion: the cache as a machine-global, accretive store that serves not only dependency changes in the current project but new modules and new projects (§2.2); the `--offline` policy flag; offline resolution against the cache; a `vibe cache` management surface. Cache keying and the project-less registry source were left as open questions.
- **2026-05-21 — draft 2.** The owner adopted two draft-1 open questions as decisions: the cache is keyed by PROP-008 qualified package identity (§2.3), making it registry-config-independent; and a user-level default registry configuration (§2.4) seeds new projects and supplies project-less invocations. The keying decision sequences PROP-010 implementation after PROP-008 (M1.19) so the cache is identity-keyed from the start. Five open questions remain — cache layout, command namespace, staleness signalling, eviction, scaffolding UX — for a follow-up owner design session. Not yet implementation-ready.
