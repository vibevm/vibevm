# PROP-010: The local package cache — a shared offline store {#root}

<status stage="spec" state="work" comment="B0 2026-07-24: DRAFT; the S5 open questions need an owner design session"/>

@fact:milestone-line **Milestone:** design proposal; implementation follows [PROP-008](PROP-008-qualified-naming.md) (qualified naming, `M1.19`), on which the identity-keyed cache depends — provisionally `M1.20` (owner to confirm in [`ROADMAP.md`](../../../ROADMAP.md)). Not implementation-locked. @status:spec/work

@fact:status-line **Status:** DRAFT — requirements captured in owner discussions on 2026-05-21; draft 2 adopted the cache-keying and user-config decisions (§2.3, §2.4). The remaining §5 open questions need an owner design session before implementation. @status:spec/work

@fact:related **Related:** [PROP-002](PROP-002-decentralized-registry.md) (the decentralized registry — `[[registry]]`, `[[mirror]]`, `[[override]]`, and the registry cache this PROP elevates); [PROP-008](PROP-008-qualified-naming.md) (qualified naming — the registry-independent package identity the cache is keyed by); [PROP-009](../vibe-workspace/PROP-009-loading-model.md) (the loading model — `vibedeps/`, materialisation, `vibe.lock`); [PROP-005](../vibe-index/PROP-005-package-index.md) (the package index — offline search); [PROP-007](../vibe-workspace/PROP-007-workspace.md) (workspaces — members). @status:spec/done

@fact:owner-sanction-line **Owner sanction:** this PROP extends `VIBEVM-SPEC.md` §8.3 (cache layout), §9 (CLI surface), and §9.5 (the user-level config layer). The spec edits land at implementation time and require explicit owner sanction — not yet granted; this PROP is the requirements record. @status:spec/work

---

## 1. Motivation {#motivation}

- @fact:cache-exists vibevm already keeps a registry cache (`VIBE_REGISTRY_CACHE` / `vibe_registry::default_cache_root()`): registry repositories are cloned there, and `vibe install` / `vibe update` fetch through it. **The variable is `VIBE_REGISTRY_CACHE`; this document said `VIBEVM_` in both places until 2026-08-19, and nothing in the tree ever read that name** — an operator following the spec would have set a variable no code consults and concluded the override does not work. @status:impl/done
- @fact:cache-incidental But the cache is an *implementation detail* — an opaque download accelerator, not a deliberate, inspectable, first-class store. @status:spec/done
- @fact:no-offline-mode *(The motivation as captured; both halves have since been answered.)* There was no `--offline` mode: every `vibe install` / `vibe update` that re-resolved walked the network, and `vibe install` always re-resolved. `--offline` shipped with [PROP-002 §2.2.2.1](PROP-002-decentralized-registry.md), and PROP-011's freshness skip ended the unconditional re-resolve. @status:spec/done
- @fact:no-local-resolve A developer behind an air-gap, on a slow link, or simply wanting fast deterministic iteration had no way to say *resolve against what I already have*. `--offline`, the embedded and project-local registries (PROP-030) and the lockfile-respecting install now say exactly that; what this PROP still adds is the **machine-global accretive store** behind them. @status:spec/done

- @fact:maven-model The model is the Maven `~/.m2` repository: a **machine-global, accretive package store** that resolution can run against with no network. vibevm adapts it. @status:spec/done
- @fact:consumers-already-offline The crucial adaptation is that vibevm already commits `vibedeps/` (PROP-009 §2.1) — so a *consumer* of a project is already fully offline: a fresh clone boots and reads its spec corpus with no `vibe install` at all. @status:spec/done
- @fact:CACHE-FOR-AUTHORS The cache is therefore not for consumers. It is for **authors**, and — the headline of this PROP — for **work that does not exist yet**. @status:spec/done

- @fact:scaffold-scenario A developer (or an agent) who has used `flow:wal` once, in any project on the machine, should be able to create a *new module* inside a workspace, or `vibe init` an *entirely new project*, that depends on `flow:wal` — and install it with no network. @status:spec/done
- @fact:accrete-across-projects The cache accretes across every project on the machine; new work draws from it. @status:spec/done
- @fact:agent-use-case This is the property that makes the Maven local repository load-bearing, and it matters doubly for vibevm's agent use case: an agent that rapidly scaffolds modules and projects turns a per-scaffold network round-trip into a local copy. @status:spec/done
- @fact:with-without Without this, every new module is gated on the network; with it, the machine's accumulated corpus is instantly reusable. @status:spec/done

---

## 2. Decisions {#decisions}

### 2.1 The cache is a machine-global, accretive store {#global}

@fact:CACHE-MACHINE-GLOBAL **Decision (override clause corrected 2026-08-20 to the later, more specific ruling).** The package store is **machine-global**, not project-scoped — one store per machine at `<settings-home>/cache`, relocated only with the settings home (`$VIBE_SETTINGS`); **no store-specific override exists** — [`##THE-STORE-IS-DOT-VIBE-CACHE`](#layout) is the governing ruling. (`VIBE_REGISTRY_CACHE`, which this decision originally named, governs the registry **clone** cache — a different layer that keeps its own job.) @status:impl/done

- @fact:CACHE-POPULATION-SHARED Every package fetched for *any* project populates it; *any* project — including projects and members that do not yet exist — resolves and materialises from it. @status:impl/work

- @fact:CACHE-ACCRETIVE The cache is **accretive**: a package version, once cached, is never evicted automatically. @status:impl/done
- @fact:accretion-why Versions are immutable (PROP-002), so a cached version is permanently valid; accretion is the point. @status:spec/done
- @fact:EXPLICIT-RECLAIM Reclaiming space is an explicit operator action (§2.8), never a surprise. @status:impl/done

@fact:explicit-not-incidental This is largely true of `default_cache_root()` already; PROP-010 makes it **explicit, documented, and load-bearing** rather than incidental. @status:spec/done

### 2.2 The cache serves work that does not exist yet {#scaffolding}

@fact:SCAFFOLDING-FIRST-CLASS **Decision.** The cache is designed to serve **new modules and new projects**, not only dependency changes in an existing project. This is a first-class requirement, not an emergent side effect. @status:spec/done

- @fact:NEW-MEMBER **A new workspace member** (PROP-007) declares its own `[requires]`. Unified resolution (PROP-009 §2.7) folds it into the workspace graph; with a warm cache and `--offline` (§2.5) the member's dependencies resolve and materialise with no network. @status:spec/done
- @fact:NEW-PROJECT **A new project** — `vibe init` followed by `vibe install` — resolves its `[requires]` from the same machine-global cache. A package pulled for an earlier, unrelated project is immediately reusable; the new project never re-downloads it. @status:spec/done

- @fact:mechanism-rests The mechanism rests on three decisions below: the cache is machine-global (§2.1), keyed by package identity so it is registry-config-independent (§2.3), and reachable offline (§2.5–§2.6) — and a new project inherits coherent registry configuration automatically (§2.4). @status:spec/done
- @fact:GUARANTEE-AND-NAME PROP-010's job is to **guarantee and name** this workflow as a supported, first-class path. @status:spec/done
- @fact:agent-fast-path For an agent scaffolding many modules or projects in one session the cache becomes the dominant fast path: the first use of a package downloads it; every later module or project draws the cached copy. @status:spec/done

### 2.3 The cache is keyed by package identity {#identity}

@fact:IDENTITY-KEYED **Decision.** The cache is keyed by **qualified package identity** as defined by PROP-008 — not by registry URL. A cached package version is addressed by its identity (`group` / `name` / `version`) and validated by `content_hash`; the registry that served it is not part of the key. @status:impl/done

- @fact:REGISTRY-INDEPENDENT A package version pulled once is reusable by every project on the machine **regardless of which `[[registry]]` each project configures** — a mirror, a different organisation hosting the same package, or a redirect target all resolve to the same cache entry when the identity matches. This is what makes §2.2 seamless: offline resolution and materialisation become registry-config-independent — a new project draws on the cache by package identity, not by reproducing some earlier project's registry list. @status:impl/work
- @fact:HASH-INTEGRITY-GATE `content_hash` is the integrity gate: a cache entry is valid only if its content hashes to the recorded hash. Two sources claiming the same identity with divergent bytes are a collision, surfaced per PROP-008's collision rules, never silently merged. @status:impl/work
- @fact:SEQUENCED-AFTER-008 **Dependency.** Identity-keying requires PROP-008 (qualified naming) to be implemented — `group` and the qualified identity must exist first. PROP-010 is therefore sequenced *after* PROP-008: the cache is identity-keyed from the start, with no URL-keyed interim to migrate later (§6). @status:spec/done

### 2.4 User-level default registry configuration {#user-registries}

@fact:USER-LEVEL-REGISTRIES **Decision (corrected 2026-08-20 — the original text named the wrong file AND the wrong directory, and both were already settled in code).** A **user-level default registry configuration** lives in its own file, `~/.vibe/registry.toml`, beside — not inside — the general user settings at `~/.vibe/config.toml`. It supplies registry configuration when no project does, and seeds a new one: @status:spec/work

- @fact:THE-SETTINGS-HOME-IS-DOT-VIBE-NOT-XDG **The settings home is `~/.vibe`** (owner, 2026-08-20). This document previously named `~/.config/vibe/config.toml`; the code has treated `~/.vibe` as canonical all along and the XDG path only as a legacy location an operator is invited to migrate out of. The correction is to this document, not to the tree. @status:impl/done
- @fact:REGISTRIES-KEEP-THEIR-OWN-FILE **Registries keep their own file, and the reason is that one of the two is shareable and the other is not** (owner, 2026-08-20). A team can hand a colleague `registry.toml` — «here is where we get packages from» — without handing over every personal preference in `config.toml`. Merging them would make the shareable thing inseparable from the private one. **This is already how the tree works**, so the decision costs nothing to keep and would cost a migration to undo. @status:impl/done
- @fact:THE-PRECEDENT-IS-GITMODULES-NOT-GITCONFIG **The precedent, stated accurately because the obvious one points the other way.** Git keeps **one** config file per scope — system, user, repository — and remotes sit in it beside everything else, so «git splits config by topic» is false. What git does do is exactly the distinction above: the thing that must be **shared with everyone** gets its own file (`.gitmodules`, versioned in the tree, listing where submodules come from), secrets get their own (`~/.git-credentials`), and `include` / `includeIf` exist so a config can be split and partly shared on purpose. Cargo agrees on the secret half (`credentials.toml` apart from `config.toml`) and not on the source half. So the split here follows the `.gitmodules` line of reasoning — shareability — and not a general habit of one-file-per-topic. @status:spec/work

- @fact:INIT-SEEDS `vibe init` seeds a new project's `[[registry]]` blocks from the user-level default instead of the hardcoded `vibespecs` defaults. A developer or organisation sets its registries once, machine-wide, and every new project inherits them. Absent any user-level config, `vibe init` falls back to today's hardcoded defaults — backward-compatible. @status:spec/done
- @fact:PROJECTLESS-SOURCE `vibe cache add` (§2.8) and other registry operations invoked outside any project use the user-level registries as their source. @status:impl/done
- @fact:MEMBER-INHERITS A new member added to a workspace already inherits the workspace's registries (resolution is unified at the root, PROP-009 §2.7); the user-level default matters at the *project* boundary — the new-project case — and for project-less invocations. @status:spec/done

- @fact:PROJECT-OVERRIDES Project-level `[[registry]]` always overrides the user-level default — the same precedence the `UserConfig` `[env]` layer already follows (the project / live value wins). @status:spec/done
- @fact:halves-of-scaffolding Identity-keying (§2.3) makes the *offline* path registry-independent; the user-level default makes a new project's *online* operations and *pre-warming* coherent without hardcoding or hand-editing. The two decisions are the offline and online halves of §2.2. @status:spec/done

### 2.5 `--offline` — the network-forbidden policy {#offline}

@fact:OFFLINE-FLAG **Decision.** A global `--offline` flag forbids all network access for the invocation. @status:impl/done

@fact:OFFLINE-LAYERING It resolves through the established CLI config layering — flag, then a `VIBE_OFFLINE` environment variable, then a user-config `[net]` key; the flag wins. This mirrors the resolved-posture pattern already used for `--unattended` / `VIBE_UNATTENDED` (`output::resolve_unattended`). @status:impl/done

- @fact:OFFLINE-LOCAL-ONLY Under `--offline`, resolution and fetch must be satisfiable entirely from local sources — the cache (§2.7), `[[mirror]]` entries with a `file://` URL, and the project's own `vibe.lock` + `vibedeps/`. @status:impl/work
- @fact:OFFLINE-HARD-ERROR Anything not available locally is a **hard error with an actionable message**: it names the missing package and version and tells the operator how to recover (run once online, `vibe cache add`, or `vibe registry vendor`). @status:impl/work
- @fact:OFFLINE-NO-DEGRADE `--offline` never silently degrades to a partial result. @status:impl/work

@fact:ONLINE-DEFAULT Online remains the default and is unchanged: it walks the network for freshness and populates the cache as it goes. `--offline` is purely additive. @status:impl/done

### 2.6 Offline resolution {#resolution}

@fact:RESOLVER-OFFLINE-MODE **Decision.** The resolver gains an offline mode — `MultiRegistryResolver::with_offline(true)`, a builder method beside the existing `with_strict_auth`. Offline resolution reads version lists and manifests from the cache, addressed by package identity (§2.3), and never runs `git fetch` / `git ls-remote` / archive fetch. @status:spec/done

- @fact:AS-OF-LAST-REFRESH Offline resolution is therefore computed against the cache **as of its last refresh**. This is correct and expected — Maven `mvn -o` and `cargo --offline` have the same property — but it must be explicit: a `--offline` resolve may pick an older version than an online resolve would. @status:spec/done
- @fact:SYNC-COMPANION The companion is `vibe registry sync` (already implemented), the deliberate "refresh the cache while the network is available" step. @status:spec/done
- @fact:intended-workflow The intended workflow is `vibe registry sync` online, then `vibe install --offline` later — the analogue of `mvn` then `mvn -o`. @status:spec/done

@fact:A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY **Decision (owner, 2026-08-19).** A package version present in the cache is **usable, and materialises, even when it exists in no registry at all** — deleted upstream, the whole organisation gone, every mirror down. This is not the `--offline` policy: `--offline` forbids the network, while this governs a run where the network is allowed, was consulted, and answered "no such package". @status:spec/work

- @fact:WHY-THE-CACHE-OUTRANKS-A-SILENT-REGISTRY **Why the cache wins.** The store holds content validated against `content_hash` ([§2.3](#identity)) — bytes we already fetched and already verified. A registry that no longer lists a version has told us about *its* present inventory, which is not evidence that the verified bytes on this disk are wrong. Treating an upstream removal as a reason to refuse content we hold would make every consumer's build hostage to a repository we do not control, which is precisely the failure the store exists to prevent. @status:spec/work
- @fact:THE-BEHAVIOUR-THIS-CONTRADICTS-TODAY **What this rule contradicted in the pre-2026-08-20 code, said here so it was not discovered during implementation — and resolved that day.** The per-package fetch path used to **delete** its local clone when an update failed — origin unreachable, ref missing, repository gone — and then re-bootstrap from the same URL, destroying the last local copy at exactly the moment this rule needs it. That wipe was not gratuitous: it existed so the next mirror in the chain takes over without stale state. The resolution is exactly the owed mechanism: failover runs as a source switch (clone beside, swap on success), so the mirror chain works without ever deleting the only copy — and the extracted store is separate from the clone, which is what makes both possible at once. @status:impl/done

@fact:A-REFRESH-AND-A-SOURCE-SWITCH-ARE-DIFFERENT-OPERATIONS **Decision (owner, 2026-08-20): the wipe stays legal for exactly one case and is forbidden for the other** — and the code now tells them apart by construction (`BringIntent { RefreshExisting, SwitchSource }` in the per-package fetch machinery). @status:impl/done

- @fact:REFRESH-HAPPENS-IN-PLACE **Refreshing a copy we already hold happens IN PLACE** — update the existing working copy, never delete and re-download it. A failed refresh (network blinked, ref missing) is retried or repaired where it stands; it is not grounds for destroying anything. @status:impl/done
- @fact:A-SOURCE-SWITCH-CLONES-BESIDE-AND-SWAPS **Switching to a DIFFERENT source** — the next mirror, another registry — is the only case that fetches from scratch, and even then it clones **into a temporary directory beside the target and replaces the existing copy only after the clone succeeds.** An interrupted switch therefore leaves the previous copy exactly as it was. @status:impl/done
- @fact:THE-TEN-GIGABYTE-TEST **The measure that makes the distinction concrete, in the owner's words:** a dependency may weigh ten gigabytes. «Delete and re-download» as the response to any hiccup is then not a small inefficiency but an unusable tool — and the cost is invisible in every test fixture, which is exactly why the rule is written here rather than left to judgement at the call site. @status:spec/done
- @fact:TODAYS-CODE-CONFLATES-THEM **What had to change, named precisely — and changed 2026-08-20.** The path used to wipe on **any** failed update, not only on a deliberate source switch. The fix was, as written, not «add a temporary directory» but to separate the two intents first (the `BringIntent` split), and only then give the switching one its temp-and-swap. The four oracles were proved red on the pre-change code: a failed refresh really did delete the copy. @status:impl/done

@fact:AN-ABSENCE-HAS-THREE-SHAPES-AND-TWO-ANSWERS **Decision (owner, 2026-08-19).** When a package cannot be supplied, what the user is told depends on which of three absences it is: @status:spec/work

| the package | what the user is told |
|---|---|
| @fact:ABSENCE-CACHED is in the cache @status:spec/work | nothing — it is used and materialised ([`##A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY`](#resolution)) @status:spec/work |
| @fact:ABSENCE-WITHDRAWN is gone, but a tombstone stands for its name @status:spec/work | **the tombstone's reason and its successor** — we know it existed and why it went ([PROP-005 §2.4](../vibe-index/PROP-005-package-index.md#layout)) @status:spec/work |
| @fact:ABSENCE-NEVER-THERE is gone with no record, or never existed @status:spec/work | **"no such package"** — the same error a typo in `vibe.toml` produces @status:spec/work |

- @fact:THE-LAST-TWO-ARE-DELIBERATELY-INDISTINGUISHABLE **The third row is not a shortfall — it is the requirement.** Full deletion is a **mechanism the operator invokes**; this project builds it and does not ask what it is for. What the mechanism owes is that deletion actually deletes: a residue that still names the deleted package would defeat the operation for a whole class of the reasons an operator might have. The clearest illustration is a complaint about the *name itself* — there a tombstone would have to be named after the very thing being removed, and would reproduce what it was meant to record — but that is one case, not the justification. A fully deleted package is therefore indistinguishable from one that never existed, and a reader who cannot tell them apart is getting the correct answer. @status:spec/work
- @fact:THE-CARVE-OUT-FROM-THE-NEVER-SILENT-LAW **This carves an exception out of a law already recorded, and the exception is named rather than left to be noticed.** [PROP-005 §2.4](../vibe-index/PROP-005-package-index.md#layout) states that a name which ever existed answers with the current thing, a forwarding pointer, or a tombstone — never with silence. That law governs **withdrawal**, where a record is kept on purpose. Full deletion is the other operation: it removes the fact that there was anything to answer about, and the silence is the deliverable. The two must never be conflated, because choosing deletion where withdrawal was meant destroys recoverable history, and choosing withdrawal where deletion was demanded leaves the violation standing. @status:spec/work

@fact:SKIP-RESOLUTION-SYNERGY There is a strong synergy with the deferred *skip-resolution-when-fresh* optimisation (when `vibe.lock` is already consistent with every node's `[requires]`, no resolution runs at all, so no network is touched): once that lands, the common path is offline-clean for free, and `--offline` governs specifically the resolution path taken when dependencies genuinely changed. The two should be designed together. @status:spec/done

### 2.7 Cache layout and population {#layout}

@fact:LOCAL-INDEX-VIEW **Decision.** The cache is keyed by package identity (§2.3) and carries a **local index view** — identity → versions present — so the resolver and the management commands (§2.8) answer cache queries without walking the whole store. *(Built 2026-08-20 as the layout itself: `<store>/<group>/<name>/v<version>/` walked per identity IS the view — a second representation would drift; `store::lookup` / `list_versions` / `list_all` are the query surface.)* @status:impl/done

@fact:layout-open <status stage="spec" state="void">Retired 2026-08-19 when the owner ruled the layout. It recorded that the on-disk form was undecided between extracted per-identity directories and git clones indexed by identity, and leaned toward the former. The lean was right and the question is closed; the heir is [`##LAYOUT-EXTRACTED-DIRECTORIES`](#layout) below. This line stays so its name is never reused and inbound links do not break.</status> @status:spec/void

@fact:LAYOUT-EXTRACTED-DIRECTORIES **Decision (owner, 2026-08-19).** The on-disk layout is **per-identity extracted directories**, one per `(group, name, version)`. Git clones indexed by identity are rejected. @status:impl/done

- @fact:WHY-EXTRACTED-AND-NOT-AN-ARCHIVE **Why extracted rather than an archive.** Materialisation into `vibedeps/` is a directory copy, and `content_hash` is computed over the shippable tree — so the extracted form is already the thing the integrity gate checks. An archive would add a packing step and a second on-disk representation of one artifact, which then has to be kept in agreement with the first. @status:spec/work
- @fact:WHY-NOT-CLONES **Why not clones, and this is the load-bearing half.** A clone is bound to the liveness of its origin **by construction**: refreshing it *is* a call to the origin, and the refresh brings it to whatever the origin says now. A store whose entries heal toward upstream cannot be the thing that survives upstream — and surviving upstream is the entire purpose of [`##A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY`](#resolution). A clone is also keyed by *where the bytes came from*, which contradicts identity-keying ([§2.3](#identity)) rather than merely differing from it. @status:spec/work
- @fact:CLONES-KEEP-THEIR-OWN-JOB The registry clone cache does not go away and is not in competition: it exists so one registry is not re-cloned for every project on the machine, which is a separate and still-valid purpose. What changes is that it stops being the only local copy of package content, and therefore stops being load-bearing for availability. @status:impl/work

@fact:CACHE-FILLS The cache fills as a side effect of any online `vibe install` / `vibe update` / `vibe registry sync`, and by deliberate pre-warming (`vibe cache add`, §2.8). It is never auto-evicted (§2.1). @status:impl/done

@fact:THE-STORE-IS-DOT-VIBE-CACHE **Decision (owner, 2026-08-20): the store is `~/.vibe/cache/`**, beside `~/.vibe/registries/` (the registry git clones, which keep their own separate job) and under the one settings home. @status:impl/done

- @fact:THE-NAME-DOES-NOT-COLLIDE-BECAUSE-THE-OTHER-CACHE-GOES **The objection to this name, and why it does not survive.** «Cache» would mean two things — this store and the per-project `<workspace-root>/.vibe/cache/`. It would not: the project directory is precisely what this store **replaces**. Today a fetch extracts into it and `vibedeps/` is copied from there; once the machine store is the source, the project copy is pure duplication and goes. Three layers remain and none of them is it — the store (source), `vibedeps/` (materialised into the project, committed), `vibe.lock` (the pinned resolution). @status:impl/done
- @fact:THE-PROJECT-CACHE-IS-REMOVED-BY-THE-SAME-WORK **The removal is part of this work, not a follow-up** — otherwise the collision the name was questioned for is real for the whole transition, which is exactly when the confusion costs most. Note the scope precisely: the subdirectory `cache/` goes; the project's `.vibe/` directory itself stays, since it also carries project settings and parked agentic commands. @status:impl/done

@fact:THE-EXTRACTED-LAYER-IS-PROMOTED-NOT-INVENTED **Decision (owner, 2026-08-20).** The extracted per-identity layer is **not built from scratch** — it already exists, project-scoped, at `<workspace-root>/.vibe/cache/<group>/<name>/v<version>/`, holding `.git`-stripped content, created at `vibe init` and used by the update and reinstall paths. The work is to change three things about it and nothing else: its **level** (per project → machine-global), its **mode** (freely rewritten → written once), and its **role** (an incidental by-product → a source resolution reads from first). Measured 2026-08-19, `campaigns/packages-2026-09/harvest/prop010-current-state.md`. @status:impl/done

@fact:WRITTEN-ONCE-IS-A-RULE-FOR-OUR-CODE-NOT-A-CLAIM-ABOUT-THE-DISK **Decision (owner, 2026-08-20), and the wording matters because the naive reading is unimplementable.** The store cannot be made immutable — it is the operator's own disk and they may edit anything on it. «Written once» is therefore exactly three commitments, all of them ours to keep: @status:impl/done

1. @fact:OUR-CODE-NEVER-REWRITES-AN-ENTRY **Our code never rewrites an entry in place.** A version is written when it is first fetched and is read-only to us afterwards. This is testable and is the only half of the rule that is fully in our hands. @status:impl/done
2. @fact:VERIFICATION-IS-A-COMMAND-NOT-A-TAX **Verification is a command an operator runs, not a cost every install pays** (owner, 2026-08-20). Re-hashing the store on every resolve would make a ten-gigabyte dependency unusable; the integrity sweep therefore lives in `vibe cache check` ([§2.8](#management)) and the ordinary path does not pay it. @status:impl/done
3. @fact:A-MISMATCH-IS-NAMED-NEVER-SWALLOWED **A mismatch is named.** When the sweep finds content that no longer hashes to what the lockfile pinned, it says which package and offers repair — it never silently re-downloads and never silently uses the altered bytes. *(Both halves hold: the read path fails as `StoreEntryMismatch` naming the package, the entry path and both hashes; the sweep — `vibe cache check` — names every mismatched identity with both hashes and repairs only under the explicit `--repair`.)* @status:impl/done

### 2.8 Cache management surface {#management}

@fact:CACHE-COMMANDS **Decision.** The cache becomes operator-visible through a command family. @status:impl/done

@fact:namespace-leaning <status stage="spec" state="void">Retired 2026-08-19 when the owner confirmed the leaning it recorded. It stated that the namespace was open between top-level `vibe cache` and `vibe registry cache`, and leaned toward the former. Heir: [`##NAMESPACE-IS-TOP-LEVEL-VIBE-CACHE`](#management). This line stays so its name is never reused and inbound links do not break.</status> @status:spec/void

@fact:NAMESPACE-IS-TOP-LEVEL-VIBE-CACHE **Decision (owner, 2026-08-19): the family is top-level `vibe cache …`.** The reason is the one the leaning already carried and the owner confirmed: the store is machine-global and its headline case is work that has no project yet — and a not-yet-created project has no `[[registry]]` section for a `vibe registry` subcommand to hang on. Putting the store under the registry family would make its most important use the one place the name does not fit. @status:impl/done

- @fact:CMD-PATH `vibe cache path` — print the cache root. @status:impl/done
- @fact:CMD-LIST `vibe cache list` — the packages and versions present locally; the offline-resolvable inventory. @status:impl/done
- @fact:CMD-ADD `vibe cache add <pkgref>…` — deliberately pre-warm: fetch a package and its dependency closure into the cache while online, so a later `--offline` run finds it. The "I am about to go offline, pull down what I will need" workflow. It fetches from the project's `[[registry]]` when run inside a project, otherwise from the user-level registries (§2.4). @status:impl/done
- @fact:CMD-CLEAN `vibe cache clean` — reclaim space: all, by age, or by package. @status:impl/done
- @fact:CMD-CHECK `vibe cache check` (owner, 2026-08-20) — **the integrity sweep, and the only place the store is fully re-hashed.** It walks every entry, recomputes the content hash, and reports each one that no longer matches what was recorded. It is the answer to «how do you forbid overwriting»: nothing forbids it, and this is what notices. *(«What was recorded» is an integrity sidecar `v<version>.sha256` written once beside the entry at insert — beside, not inside, or the record would change the very tree it pins; an entry without one is the honest `unrecorded` class, not an error.)* @status:impl/done
- @fact:CMD-CHECK-REPAIR `vibe cache check --repair` (owner, 2026-08-20) — the same sweep, and then it fixes what it found. @status:impl/done

@fact:REPAIR-CLIMBS-A-LADDER-CHEAPEST-FIRST **Decision (owner, 2026-08-20): repair tries the cheap local restore before the expensive re-fetch, and the order is the whole point.** A ten-gigabyte dependency must not be re-downloaded because one file was touched. @status:impl/done

1. @fact:REPAIR-STEP-IS-IT-A-CLONE **First establish what the entry IS.** An entry that is a git working copy can be restored locally; an extracted directory carrying no `.git` cannot, and skips to the last rung. This check is not a formality — the store's own layout strips `.git`, so both shapes exist on disk. @status:impl/done
2. @fact:REPAIR-STEP-LOCAL-RESTORE **For a git working copy: discard local damage** — remove untracked and ignored files, then hard-reset the tree to the commit the entry is pinned to. Re-hash. If it matches, repair is done and nothing was downloaded. *(Measured 2026-08-20 at the build: the store records a content hash and no commit, so this rung has nothing to reset TO — a mismatched git copy is honestly classified «unrepairable locally» and takes the re-fetch rung; the rung revives if the store ever records the pinned commit.)* @status:impl/work
3. @fact:REPAIR-STEP-REFETCH **Otherwise re-fetch the entry from scratch**, which is correct but is the expensive rung and therefore the last one. @status:impl/done

- @fact:REPAIR-DOES-NOT-PULL **Repair never advances the entry to a newer commit** — no fetch-and-merge step belongs on this ladder, and the reason is what repair MEANS. The hash being restored belongs to one pinned version; moving the working copy forward would change the content that is being checked, turning a repairable entry into a guaranteed mismatch and then into a needless re-download. Advancing to a newer version is `vibe update`'s job and is a different intent from «make this entry be what it was recorded to be». @status:impl/done

@fact:SYNC-VENDOR-COMPLEMENT These complement, and do not replace, the existing `vibe registry sync` (refresh the cache) and `vibe registry vendor` (export a project's locked set to a `file://` mirror — see §6). @status:spec/done

### 2.9 Layering — the cache, `vibedeps/`, and the lockfile {#layering}

@fact:LAYERS-EXPLICIT **Decision.** PROP-010 changes none of the three existing layers; it makes their relationship explicit. @status:spec/done

- @fact:LAYER-CACHE **The cache** — machine-global, accretive, identity-keyed, the *source* of package content. Shared across every project on the machine. @status:impl/done
- @fact:LAYER-VIBEDEPS **`vibedeps/`** — per-project, committed, the *materialised* dependency content for that project's locked resolution (PROP-009 §2.1). Produced by copying from the cache. @status:spec/done
- @fact:LAYER-LOCK **`vibe.lock`** — per-project, the pinned resolution (PROP-009). @status:spec/done

@fact:OFFLINE-FLOW An offline `vibe install` of a new project resolves `[requires]` against the cache, then materialises each resolved package by copying from the cache into the new project's `vibedeps/`. No layer is bypassed; the cache simply becomes a first-class, offline-capable, registry-independent source feeding materialisation. @status:spec/done

---

## 3. Command and crate surface {#surface}

- @fact:SURF-OFFLINE-FLAG A global `--offline` flag (and `VIBE_OFFLINE`) on the `vibe` CLI (§2.5). @status:impl/done
- @fact:SURF-CACHE-CMDS `vibe cache path` / `list` / `add` / `clean` (§2.8). @status:impl/done
- @fact:SURF-CORE `vibe-core` — the `UserConfig` schema gains a `[[registry]]` / `[[mirror]]` section and a `[net]` key (§2.4, §2.5). @status:spec/done
- @fact:SURF-REGISTRY `vibe-registry` — the identity-keyed cache and its local index view, `MultiRegistryResolver::with_offline(...)` (§2.3, §2.6, §2.7). Depends on PROP-008's identity types. @status:spec/done
- @fact:SURF-CLI `vibe-cli` — flag wiring, the resolved offline posture, `vibe init` seeding registries from the user-level default, the `vibe cache` commands, actionable cache-miss errors. @status:spec/done
- @fact:SURF-SYNC-VENDOR `vibe registry sync` / `vibe registry vendor` — unchanged; documented as the cache's refresh and export companions. @status:spec/done

---

## 4. Migration {#migration}

- @fact:ABANDON-NOT-MIGRATE The existing registry cache is keyed by registry URL; the identity-keyed cache (§2.3) is a different layout. The existing cache is **abandoned, not migrated** — a cache is reconstructible from registries, never authoritative data. @status:spec/done
- @fact:REPOPULATE The first run on the new layout repopulates from the network; the stale URL-keyed directory can be removed by hand or by a one-shot cleanup. @status:spec/done
- @fact:one-time-cost A single re-download is an acceptable one-time cost for a pre-release tool, and it avoids carrying a re-keying migration path that would exist only once. @status:spec/done

@fact:ADDITIVE-OTHERWISE Everything else is additive: a project that never passes `--offline` and sets no user-level registry config sees identical behaviour. @status:spec/done

---

## 5. Open questions {#open}

1. @fact:OPEN-LAYOUT <status stage="spec" state="void">**RESOLVED by the owner 2026-08-19** — per-identity extracted directories; clones rejected. The ruling and its reasoning are [`##LAYOUT-EXTRACTED-DIRECTORIES`](#layout). This line stays so the question's name is never reused and inbound links do not break.</status> @status:spec/void
2. @fact:OPEN-NAMESPACE <status stage="spec" state="void">**RESOLVED by the owner 2026-08-19** — top-level `vibe cache …`. The ruling and its reasoning are [`##NAMESPACE-IS-TOP-LEVEL-VIBE-CACHE`](#management). This line stays so the question's name is never reused and inbound links do not break.</status> @status:spec/void
3. @fact:OPEN-STALENESS **Staleness signalling** — should an `--offline` run warn when the cache is older than some threshold, or when an online resolve would likely differ? @status:spec/work
4. @fact:OPEN-EVICTION **Eviction** — pure manual `vibe cache clean`, or an optional size cap / LRU? @status:spec/work
5. @fact:OPEN-SCAFFOLD-UX **Scaffolding UX** — should `vibe init` and new-member creation actively report "your declared `[requires]` are fully cached — you can work offline", or stay silent? @status:spec/work
6. @fact:OPEN-STORE-DIRECTORY-NAME <status stage="spec" state="void">Resolved the day it was asked, 2026-08-20 — the store is `~/.vibe/cache/`. The question assumed a name collision with the per-project `.vibe/cache/`; the owner pointed out that the project directory is what the store REPLACES, so after the work there is only one cache and no collision. Heir: [`##THE-STORE-IS-DOT-VIBE-CACHE`](#layout). This line stays so the question's name is never reused and inbound links do not break.</status> @status:spec/void

@fact:draft2-resolved Resolved in draft 2: cache keying (§2.3 — keyed by PROP-008 package identity) and the project-less registry source (§2.4 — a user-level default registry configuration). @status:spec/done

@fact:draft3-resolved Resolved by the owner 2026-08-19, in the session that measured the clone cache against this document's own motivation: the on-disk layout (§2.7 — extracted per-identity directories), and two rules this document did not previously carry — a cache hit outranks a registry that no longer lists the version (§2.6), and the three shapes of absence with the two answers they get (§2.6). **Four questions remain open (§5.2–§5.5) and none of them blocks building.** The dependency this document was sequenced behind — qualified naming — has been implemented since it was written, so the sequencing note in §2.3 is satisfied rather than pending. @status:spec/work

---

## 6. Rejected / deferred alternatives {#rejected}

- @fact:REJ-OFFLINE-DEFAULT **Make offline the default, auto-detecting the network.** Rejected — implicit mode-switching makes a build's inputs unpredictable. Online stays the explicit default; `--offline` is an explicit opt-in. (The *common* path still avoids the network once skip-resolution-when-fresh lands — but by being a no-op, not by guessing.) @status:spec/done
- @fact:REJ-URL-KEYED-INTERIM **A URL-keyed cache now, re-keyed to identity later.** Rejected — it would carry a one-time re-keying migration for no benefit. PROP-010 is instead sequenced after PROP-008 (§2.3) so the cache is identity-keyed from day one. @status:spec/done
- @fact:REJ-REPLACE-VENDOR **Replace `vibe registry vendor` with the cache.** Rejected — they solve different problems. `vendor` exports *one project's locked set* to a portable `file://` mirror for handing to an air-gapped machine or another person. The cache is the *machine-local accretive store* of everything that machine has used. Both stay. @status:spec/done
- @fact:REJ-PROJECT-SCOPED **A project-scoped cache.** Rejected — it defeats §2.2 entirely. A per-project cache cannot serve a project that does not exist yet. @status:spec/done

---

## 7. Phase plan {#phases}

@fact:phases-sequencing Sequenced after PROP-008 (M1.19), on which §2.3 depends. @status:spec/done

1. @fact:PHASE-1-IDENTITY-CACHE **The identity-keyed cache** — the cache keyed by PROP-008 package identity; a documented, stable layout (§5.1); the local index view; `vibe cache path` / `vibe cache list`. `vibe-registry` + `vibe-cli`. @status:impl/done
2. @fact:PHASE-2-USER-REGISTRIES **User-level default registry configuration** — `[[registry]]` / `[[mirror]]` in `UserConfig`; `vibe init` seeds from it; project config overrides. `vibe-core` + `vibe-cli`. @status:impl/plan
3. @fact:PHASE-3-OFFLINE **`--offline`** — the global flag, `VIBE_OFFLINE`, the resolved posture; `MultiRegistryResolver` offline mode (resolve from the cache, never touch the network); actionable cache-miss errors. @status:impl/work
4. @fact:PHASE-4-PREWARM **Pre-warm + clean** — `vibe cache add` (deliberate population) and `vibe cache clean`. @status:impl/done
5. @fact:PHASE-5-SCAFFOLDING **Scaffolding integration** — guarantee a new project (`vibe init` + `vibe install --offline`) and a new workspace member resolve and materialise from the cache, end to end; the §2.2 workflow plus any §5.5 UX hint. @status:impl/plan
6. @fact:PHASE-6-DOCS **Docs + `VIBEVM-SPEC.md`** — §8.3 / §9 / §9.5 edits under owner sanction; a `docs/` page for the cache and offline mode. @status:impl/plan

---

## 8. Version history {#history}

- @fact:HISTORY-DRAFT-1 **2026-05-21 — draft 1.** Requirements captured in an owner discussion: the cache as a machine-global, accretive store that serves not only dependency changes in the current project but new modules and new projects (§2.2); the `--offline` policy flag; offline resolution against the cache; a `vibe cache` management surface. Cache keying and the project-less registry source were left as open questions. @status:spec/done
- @fact:HISTORY-DRAFT-2 **2026-05-21 — draft 2.** The owner adopted two draft-1 open questions as decisions: the cache is keyed by PROP-008 qualified package identity (§2.3), making it registry-config-independent; and a user-level default registry configuration (§2.4) seeds new projects and supplies project-less invocations. The keying decision sequences PROP-010 implementation after PROP-008 (M1.19) so the cache is identity-keyed from the start. Five open questions remain — cache layout, command namespace, staleness signalling, eviction, scaffolding UX — for a follow-up owner design session. Not yet implementation-ready. @status:spec/done
