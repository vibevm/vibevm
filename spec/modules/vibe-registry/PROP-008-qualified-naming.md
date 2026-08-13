# PROP-008: Qualified package naming — groups, short aliases, collision detection {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: IMPLEMENTED, M1.18 + M1.19"/>

@fact:milestone-line **Milestone:** `M1.18` + `M1.19` ([`ROADMAP.md`](../../../ROADMAP.md)) — **shipped**, implementation-locked. (The line read "design proposal … not implementation-locked" until 2026-07-25; it had never been reconciled with the IMPLEMENTED status one line below.) @status:impl/done

@fact:status-line **Status:** IMPLEMENTED — Phases 1–4 + 7 landed 2026-05-22 (M1.18, see §7); Phases 5–6 + 8 landed with M1.19 (index-backed short-name resolution at the CLI boundary — `vibe-cli::commands::short_name`; collision detection with exit code `7` — `InstallError::AmbiguousPackage`; the live-registry fqdn migration). Decision units typed at REQ grain 2026-06-12 (the depth program). @status:impl/done

@fact:related **Related:** [PROP-002 §2.1 / §3.4](PROP-002-decentralized-registry.md) (content-addressed identity; the rejection of *per-registry* identity — and why `group` does not violate it); [PROP-005](../vibe-index/PROP-005-package-index.md) (per-org index — **required** for short-name resolution); [PROP-007](../vibe-workspace/PROP-007-workspace.md) (workspace — companion document, same design session); [`VIBEVM-SPEC.md` §4.1 / §7.1](../../../VIBEVM-SPEC.md) (the installable kinds; current `name`-uniqueness rule). @status:spec/done

@fact:design-rationale **Design rationale:** [`spec/design/workspace-and-qualified-naming.md`](../../design/workspace-and-qualified-naming.md) — the *why* and the lore behind this PROP: the owner's mental model, the fork-by-fork decision record, the Cargo-vs-Maven precedents. Non-normative; this PROP is the contract. @status:spec/done

@fact:discipline-line **Discipline:** the general namespace-scaling laws — why a flat namespace fails, the mandatory *group*, the identity tuple, why a rename is a new identity, why short names live only at the human boundary, and why a *collision* and a *conflict* are different failures — are the `qualified-naming` flow: `spec://org.vibevm.world/qualified-naming/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL#root`. This PROP is vibevm's IMPLEMENTED instance of them — the `org.vibevm.*` groups, the CLI short-name resolution (`vibe-cli::commands::short_name`), and collision detection (`InstallError::AmbiguousPackage`). @status:spec/done

@fact:OWNER-SANCTION **Owner sanction:** the owner granted (2026-05-20) explicit sanction to edit any specification, including `VIBEVM-SPEC.md` §7.1. PROP-008 is the requirements record; the `VIBEVM-SPEC.md` edit lands at implementation time. @status:impl/done

---

## 1. Motivation {#motivation}

- @fact:flat-namespace vibevm's package namespace is flat: a pkgref is `<kind>:<name>`, and `name` is "globally unique within its kind" (`VIBEVM-SPEC.md` §7.1). This does not scale — two unrelated authors will both want `flow:wal`. @status:impl/done
- @fact:prior-art Maven solved exactly this with `groupId` (reverse-FQDN) for global uniqueness; npm with `@scope/`. @status:impl/done

- @fact:OWNER-REQUEST The owner's request (design session 2026-05-20): introduce reverse-FQDN qualification at the top level (`org.vibevm`), while keeping short names usable — a user types `vibe install wal` in the CLI, but the package is canonically `org.vibevm.world/wal`. @status:impl/done
- @fact:collision-behavior-request On a name collision, show alternatives; on a dependency conflict, fail without applying the plan; under full-auto, fail rather than guess. @status:impl/done

- @fact:why-not-per-registry **Why this does not violate [PROP-002 §3.4](PROP-002-decentralized-registry.md).** PROP-002 §3.4 rejected *per-registry identity* — `vibespecs/flow:wal` must not be a different identity from `corporate/flow:wal`, because that would make mirror-switching impossible. @status:impl/done
- @fact:GROUP-NOT-REGISTRY `group` is **not** the registry. `group` is an attribute of the *package* (exactly as Maven's `groupId` is an attribute of the artifact, not of the repository serving it). @status:impl/done
- @fact:group-orthogonal The registry remains a runtime resolution detail. Adding `group` to the identity tuple is orthogonal to §3.4 and does not reopen it. @status:impl/done

@fact:naming-axis PROP-008 covers the naming axis. The companion [PROP-007](../vibe-workspace/PROP-007-workspace.md) covers workspaces; the two were specified together. @status:impl/done

---

## 2. Decisions {#decisions}

### 2.1 The `group` field {#group}

@fact:req-group `req r1` @status:impl/done

@fact:GROUP-MANDATORY **Decision.** `[package]` gains a **mandatory** `group` field: @status:impl/done

```toml
[package]
kind    = "flow"
name    = "wal"
group   = "org.vibevm"
version = "0.3.0"
```

- @fact:GROUP-CONVENTION Reverse-FQDN is the **recommended convention**; the core does **not** enforce it. Whether `group` looks like a reversed domain is a matter of style, left to humans and linters. (Maven likewise does not enforce groupId shape.) @status:impl/done
- @fact:GROUP-GRAMMAR Grammar (owner ruling 2026-08-13 — «настоящие домены»): dot-separated segments, each an **LDH hostname label** — `[a-z0-9-]+`, ASCII lowercase, hyphen never at a label edge; `_` is forbidden (it is not legal in a domain). Interior doubled hyphens stay legal, as DNS itself allows (`xn--…` punycode). A group is therefore grammatically a valid reversed FQDN even though semantically it is a claim, not a credential (§2.10). Enforced by `Group::parse`. *Considered and rejected:* keeping `_` (groups would not even be formally domains, and the flat `<group>.<name>` carrier §2.5 would lose its unambiguous split); recording "FQDN-like, not FQDN-valid" as a deliberate looseness (the ruling chose real domain rules). *Revisit:* a real-world group needing `_` appears — it cannot, if groups track domains. @status:impl/done
- @fact:SEGMENT-COUNT-IS-REGISTRY-POLICY The core grammar requires **≥ 1 segment** — `acme` parses. Requiring ≥ 2 segments, reverse-FQDN shape, or any other domain-likeness is **registry policy**, enforced at registration time by the registry's moderation or web surface (owner ruling 2026-08-13), never by the core. Short single-segment groups are the norm for local registries (§2.10). @status:impl/done
- @fact:GROUP-CANONICAL `group` is mandatory as of this PROP. The three current canonical packages migrate to `group = "org.vibevm"` (§3) — the owner's reverse-FQDN, recorded here as the canonical group for all first-party vibevm packages (domain `vibevm.org`). @status:impl/done

### 2.2 Identity tuple — `(group, name, version, content_hash)` {#identity}

@fact:req-identity `req r1` @status:impl/done

@fact:IDENTITY-TUPLE **Decision.** Package identity becomes `(group, name, version, content_hash)`. `kind` **leaves the identity tuple**. @status:impl/done

- @fact:NAME-UNIQUE-IN-GROUP `name` becomes unique **within a `group`** (was: within a `kind`, `VIBEVM-SPEC.md` §7.1). `(group, name)` is therefore unique on its own — `kind` is no longer needed to disambiguate. @status:impl/done
- @fact:HASH-UNCHANGED `content_hash` is unchanged — computed over package file bytes per [PROP-002 §2.1](PROP-002-decentralized-registry.md#identity). `group` lives in `vibe.toml`, so it influences the hash only as ordinary file content; the tuple lists it explicitly so that changing `group` yields a different package. @status:impl/done
- @fact:GROUP-CHANGE-NEW-PACKAGE Changing a package's `group` is a new package, not a rename — same discipline as changing `name`. @status:impl/done

### 2.3 `kind` becomes pure metadata {#kind}

@fact:req-kind `req r1` @status:impl/done

@fact:KIND-METADATA **Decision.** `kind` (`flow` / `feat` / `stack` / `tool`) stays a **mandatory `[package]` field** but is now a pure attribute — it identifies nothing and names nothing. @status:impl/done

@fact:kind-still-needed-lead It is still needed for: @status:impl/done
- @fact:KIND-PLACEMENT content placement — `spec/flows/` vs `spec/feats/` vs `spec/stacks/`; @status:impl/done
- @fact:KIND-FILTER the `--kind` filter on `vibe list` / `vibe search`; @status:impl/done
- @fact:KIND-UX the UX signal in a kind-prefixed pkgref (§2.4). @status:impl/done

@fact:TAXONOMY-UNCHANGED The four-kinds taxonomy (`VIBEVM-SPEC.md` §4.1) is unchanged in importance — it simply stops being part of identity and repository naming. @status:impl/done

### 2.4 pkgref grammar {#pkgref}

@fact:req-pkgref `req r1` @status:impl/done

@fact:PKGREF-GRAMMAR **Decision.** The pkgref grammar gains an optional `group` segment and makes the `kind` prefix optional: @status:impl/done

```
pkgref := [ <kind> ":" ] [ <group> "/" ] <name> [ "@" <version> ]
```

@fact:PKGREF-SEPARATOR The `group`↔`name` separator is `/` (`:` is taken by `kind`, `@` by version). @status:impl/done

| Form | Context | Behaviour |
|---|---|---|
| @fact:ROW-QUALIFIED `org.vibevm.world/wal` @status:impl/done | qualified — the form written into manifests (see §2.6, [PROP-002](PROP-002-decentralized-registry.md)) @status:impl/done | resolved exactly @status:impl/done |
| @fact:ROW-QUALIFIED-KIND `flow:org.vibevm.world/wal` @status:impl/done | qualified + kind @status:impl/done | resolved exactly; **kind validated against the manifest** @status:impl/done |
| @fact:ROW-SHORT `wal` @status:impl/done | short — CLI sugar @status:impl/done | resolved via the index (§2.6) @status:impl/done |
| @fact:ROW-SHORT-KIND `flow:wal` @status:impl/done | short + kind @status:impl/done | resolved via the index; kind validated @status:impl/done |

- @fact:KIND-VALIDATION **kind validation.** If the `kind` prefix is present, after resolution the resolver asserts `resolved.kind == prefix`; mismatch is a `KindMismatch` error. A kind prefix is validation + a UX signal — it does **not** disambiguate, because by §2.2 `name` is unique within a `group`, so `flow:org.vibevm.world/wal` and `feat:org.vibevm.world/wal` cannot co-exist. A short-name collision is always a *group* collision (§2.7), resolved by group-qualification, never by kind. @status:impl/done
- @fact:SHORT-CLI-ONLY **The short form is CLI-only sugar.** It is never written to a manifest (§2.6). @status:impl/done

### 2.5 Repository naming — `naming = "fqdn"` {#repo-naming}

@fact:req-repo-naming `req r1` @status:impl/done

@fact:FQDN-NAMING **Decision.** `kind` leaves the repository name. A new `[[registry]]` naming convention value: @status:impl/done

```toml
[[registry]]
name   = "vibespecs"
url    = "https://github.com/vibespecs"
naming = "fqdn"          # repo name = "<group>.<name>"  →  org.vibevm.world.wal
```

- @fact:JOINER-UNDERSCORE `naming = "fqdn"` maps a pkgref to the repository name `<group>.<name>` (`org.vibevm.world/wal` → `org.vibevm.world.wal`) — the composite is itself a valid reversed FQDN, and that is the point of the ruling. **Owner ruling 2026-08-13** («убери подчёркивания везде, чтобы получились настоящие FQDN с доменными правилами»), superseding the 2026-05 `_`-joiner decision recorded in this unit's earlier text. The split stays deterministic without a charset-excluded joiner: the name is a **single dot-free LDH label** (`validate_package_name`), so the **last dot** is always the boundary — parse back by taking the last label as `name`, the rest as `group`. The old rationale's premises are both gone: `_` was then legal inside groups (no longer — §2.1 LDH), and the composite was not required to be a domain (now it is). *Considered and rejected:* keeping `_` (the composite is not even formally a domain; and a `_`-joined name contradicts the LDH ruling the halves now obey). *Migration:* live `_`-joined repositories in `vibespecs` (M0/M1 scale) rename to the dot form as a follow-up of the 2026-08-13 landing — pre-public and cheap, and GitHub redirects renamed repositories; the §3 history below records the `_`-era as it happened and is not rewritten. *Revisit:* a hosting provider that forbids `.` in repository names appears in the registry set — then that provider's adapter gets its own naming value, never a silent re-join. @status:impl/done
- @fact:COLLISION-FREE-REPO Because `(group, name)` is unique (§2.2), `<group>.<name>` is a collision-free repo name without needing `kind`. The existing `kind-name` / `name` / `kind/name` conventions (PROP-002 §2.2) remain for registries that have not adopted `group`. @status:impl/done
- @fact:SHORT-CLI-FAT-REPO This realises the owner's "short name in the CLI, fat name in the repository" goal: the repository is the pure reverse-FQDN; the CLI keeps the short alias. @status:impl/done

### 2.6 Short-name resolution {#short-name}

@fact:req-short-name `req r1` @status:impl/done

@fact:SHORT-AT-BOUNDARY **Decision.** A short name (`wal`, `flow:wal`) is resolved **only at the CLI input boundary** — through the index, or from `vibe.lock` alone where the verb acts on installed state (`##INSTALLED-STATE-RESOLVES-LOCALLY`). Manifests always store the qualified form. @status:impl/done

- @fact:RESOLVE-ONCE-WRITE-QUALIFIED `vibe install wal` resolves the collision once, at the top level, and writes `org.vibevm.world/wal` into `[requires]`. Manifests are therefore always qualified — exactly the cargo/npm pattern (`cargo add serde` on the CLI, `serde = "1"` in `Cargo.toml`). @status:impl/done
- @fact:NO-TRANSITIVE-COLLISIONS **Consequence — no transitive collisions.** Every package's `[requires]` is qualified (its author published through the same flow). The dependency graph is built from qualified names; short-name resolution never recurses into the graph. It happens once, for a human-typed CLI argument. @status:impl/done
- @fact:INDEX-DEPENDENCY **Index dependency.** Resolving a short name requires enumerating candidates `(*, name)` across registries. The host cannot list an org cheaply ([PROP-005 §1](../vibe-index/PROP-005-package-index.md) — GitVerse exposes no org listing, GitHub is rate-limited). Therefore short-name resolution **requires [PROP-005](../vibe-index/PROP-005-package-index.md)**: one HTTP GET of `by-name/<name>.json` per registry yields the candidate set. Without an index, a registry's short names are unavailable and the qualified form is required. @status:impl/done
- @fact:LOCKFILE-AUTHORITATIVE **Lockfile is authoritative.** If `vibe.lock` already pins `org.vibevm.world/wal`, a later `vibe install wal` resolves to the locked entry — the short name prefers what is already locked. @status:impl/done
- @fact:INSTALLED-STATE-RESOLVES-LOCALLY **A verb that acts on an already-installed package resolves a short name from `vibe.lock` alone — no index, no network.** `vibe uninstall wal` and `vibe update wal` operate over installed state, and the lockfile *is* that state's record, so the answer sits in the file beside them and `##INDEX-DEPENDENCY` does not bind: a name the lockfile does not carry is **not installed** — a local failure with a local remedy, never a lookup that could not be performed. Requiring the qualified form there would be a restriction with no cause behind it. The counter-case is the registry-side redirect verbs (`vibe registry redirect`, `redirect-sync`, `redirect-update`), which create and maintain a stub for a package that need **not** be installed at all: there is no lockfile to answer from, so `##INDEX-DEPENDENCY` binds as written and the qualified form stays required — an honest requirement rather than an unfinished one. @status:impl/done

### 2.7 Collision vs conflict {#collision}

@fact:req-collision `req r1` @status:impl/done

@fact:TWO-FAILURE-CLASSES **Decision.** Two distinct failure classes, with distinct handling. This terminology is fixed by this PROP. @status:impl/done

- @fact:COLLISION-DEF **Collision (a naming ambiguity).** Two *different* packages match one short name (`wal`) with different `group`. Detected during short-name resolution (§2.6). @status:impl/done
- @fact:CONFLICT-DEF **Conflict (a dependency conflict).** The depsolver cannot satisfy version constraints — incompatible constraints, declared `[conflicts]`, an unsatisfiable diamond. Already handled (PROP-002 §2.9 — resolvo/libsolv conflict-explanation chain). @status:impl/done

@fact:collision-handling-lead Collision handling (new): @status:impl/done

- @fact:COLLECT-ALL-CANDIDATES The resolver collects *all* candidates of a short name — it does **not** stop at the first registry. (PROP-002 §2.2's first-match-wins remains correct for the *same* package mirrored across registries — identical identity. It is wrong for *different* packages sharing a short name; the two are distinguishable only once `group` exists.) @status:impl/done
- @fact:COLLISION-BEHAVIOR One candidate → resolve. Multiple candidates with different identity → **collision**: @status:impl/done
  - @fact:collision-interactive interactive TTY — print the alternatives and fail with a hint pointing at the qualified form (no interactive pick: the choice must be recorded deliberately, not clicked); @status:impl/done
  - @fact:collision-unattended `--unattended` / full-auto — fail-fast; the resolver never guesses. @status:impl/done
- @fact:EXIT-CODE-7 A new exit code **`7`** ("ambiguous package") is assigned, distinct from `3` ("package conflict", `VIBEVM-SPEC.md` §9.4). @status:impl/done

```
flow:wal is ambiguous — 2 packages match:
  1. org.vibevm.world/wal   (registry vibespecs)
  2. com.acme/wal     (registry acme-internal)
Re-run with the qualified form, e.g. `vibe install org.vibevm.world/wal`.
```

@fact:CONFLICT-UNCHANGED Conflict handling is unchanged: the install pipeline is already atomic (resolve → plan → confirm → apply); a failed resolve never reaches apply — "fail without applying the plan", as the owner specified. @status:impl/done

### 2.8 Index extension {#index-ext}

@fact:req-index-ext `req r1` @status:impl/done

@fact:INDEX-FIELDS **Decision.** [PROP-005](../vibe-index/PROP-005-package-index.md)'s entry schema (§2.6) gains two fields: `group` (mandatory, §2.1) and `workspace_origin` (optional — set when the package was published from a workspace, [PROP-007 §2.8](../vibe-workspace/PROP-007-workspace.md) `[origin]`). @status:impl/done

- @fact:BY-NAME-CANDIDATES The `by-name/` layer indexes by `name` and returns the candidate set with each candidate's `group`, so §2.6 short-name resolution is one GET per registry. @status:impl/done
- @fact:draft-edit-note PROP-005 is currently a draft; these are edits to a draft, not a shipped contract. @status:spec/done

### 2.9 Registry explorer {#explorer}

@fact:design-explorer `design r1` @status:spec/done

@fact:EXPLORER-DIRECTION **Decision (forward-looking, out of implementation scope).** The index makes a Maven-Central-style browsable visualisation possible — and richer. A **vibevm registry explorer** is recorded here as a long-term direction (a `ROADMAP.md` M3+ entry): @status:spec/done

- @fact:explorer-group-tree a reverse-FQDN group tree with drill-down (`org` → `org.vibevm` → packages → versions), as Maven Central does; @status:spec/done
- @fact:explorer-beyond-maven beyond Maven Central: filter by `kind`; a capability graph (`[provides]`/`[requires]`); `describes`/PURL links to upstream libraries; redirect-stub delegation; the full dependency DAG; and **workspace provenance** ("Y is a sub-package of X", from `workspace_origin`). @status:spec/done

- @fact:EXPLORER-SEPARATE-LAYER The explorer is a separate, optional layer over the index — not part of PROP-008's implementation. PROP-005 §2.10 already reserves the hook (`vibe-index serve`, CORS-open read endpoints). @status:spec/done
- @fact:EXPLORER-INDEX-OBLIGATION The only obligation on this refactor is that the index carry `group` and `workspace_origin` (§2.8) so the explorer is not a retrofit later. @status:spec/done

---

## 3. Migration {#migration}

@fact:design-migration `design r1` @status:impl/done

@fact:breaking-window The breaking-change window is open: vibevm has no public release, no external users ([PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) — "schema churn before v0.1.0 is free"). @status:impl/done

- @fact:MIG-CANONICAL **Canonical packages.** `flow-wal`, `flow-sync-from-code`, `flow-atomic-commits` migrate to `group = "org.vibevm"`. Repositories rename to the `naming = "fqdn"` shape (`org.vibevm_wal`, …). The owner authorised migrating the test fixtures and these three without further questions. **Superseded in part by [PROP-029](../../common/PROP-029-fully-qualified-addresses.md):** the redbook trio's *local* group later moved to `org.vibevm.world`, so their real repos render `org.vibevm.world_wal`; the still-published `org.vibevm` repos and the group-`org.vibevm` test fixtures trail that move and rename on their next publish. @status:impl/done
- @fact:MIG-TEST-ORGS **Test orgs.** `vibespecstest1/2/3` fixtures re-laid-out to the new naming. @status:impl/done
- @fact:MIG-MANIFESTS **Manifests.** `vibe-package.toml` → `vibe.toml` ([PROP-007 §2.2](../vibe-workspace/PROP-007-workspace.md)); add the `group` field. @status:impl/done
- @fact:MIG-LOCKFILE-V5 **Lockfile.** Schema bumps to **v5** — PROP-007 had already taken v4 for `source_kind = "path"`; adds the `group` field per `[[package]]`. @status:impl/done
- @fact:MIG-SPEC-EDIT **`VIBEVM-SPEC.md` §7.1** is edited (under the owner sanction) — the `name`-uniqueness rule changes from "within a kind" to "within a group", and the identity tuple and pkgref grammar are updated. @status:impl/done

---

## 4. Rejected alternatives {#rejected}

- @fact:REJ-PER-REGISTRY **Per-registry identity.** Already rejected in [PROP-002 §3.4](PROP-002-decentralized-registry.md). `group` is a package attribute, not a registry — §1 explains why it does not reopen that decision. @status:spec/done
- @fact:REJ-KIND-IN-REPO **`kind` in the repository name.** Rejected (this PROP, §2.5). With `(group, name)` unique, `kind` in the repo name is redundant noise; `naming = "fqdn"` drops it. @status:spec/done
- @fact:REJ-SHORT-IN-MANIFESTS **Short names inside manifests.** Rejected (§2.6). Manifests store the qualified form; short names are CLI-only sugar. This eliminates transitive collisions by construction. @status:spec/done
- @fact:REJ-KIND-DISAMBIGUATOR **kind prefix as a disambiguator.** Rejected (§2.4). With `name` unique within `group`, the kind prefix can only validate, never disambiguate; a real ambiguity is a group collision. @status:spec/done

---

## 5. Open questions {#open}

1. @fact:OPEN-EXIT-CODE-7 Exit code `7` — finalise the assignment against `VIBEVM-SPEC.md` §9.4 and confirm no clash with a future code. @status:spec/work
2. @fact:OPEN-EXPLORER-SCOPE Registry explorer scope (§2.9) — when (if) it becomes a funded milestone, it gets its own PROP. @status:spec/work
3. @fact:OPEN-FQDN-KIND-VARIANT Whether `naming = "fqdn"` should also offer a `kind`-bearing variant for registries that want it, or stay strictly `<group>.<name>`. @status:spec/work

---

## 6. Phase plan {#phases}

@fact:phase-plan PROP-008 depends on [PROP-005](../vibe-index/PROP-005-package-index.md) being implemented (short-name resolution, §2.6) and is best sequenced after [PROP-007](../vibe-workspace/PROP-007-workspace.md). Suggested order: PROP-007 (workspace) → PROP-005 implementation (index) → PROP-008 (qualified naming) → collision-detection slice (§2.7). The `group` field, identity-tuple change, pkgref grammar, and `naming = "fqdn"` can land before short-name resolution; short-name resolution and collision detection land once the index is real. @status:impl/done

---

## 7. Version history {#history}

- @fact:HISTORY-DRAFT-1 **2026-05-20 — draft 1.** Initial proposal. Requirements locked in an owner design session (decisions on `group`, identity tuple, `kind`-as-metadata, pkgref grammar, `fqdn` repo naming, index-backed short-name resolution, collision detection, exit code 7, registry explorer as a long-term direction). Open for review. @status:spec/done
- @fact:HISTORY-PHASES-1-4-7 **2026-05-22 — Phases 1–4 + 7 implemented (under MFBT).** The identity core landed on `main`: the `Group` newtype and the mandatory `[package].group` (Phase 1); the `(group, name, version, content_hash)` identity refactor with `kind` demoted to metadata (Phase 2); the lockfile `group` field at schema v5 (Phase 3); the group-native registry with `NamingConvention::Fqdn` as the default (Phase 4). Phase 7 (§2.8) then made the package index group-native — the [PROP-005](../vibe-index/PROP-005-package-index.md) entry schema gained `group` + `workspace_origin`, the `by-name/` layer became the candidate-set file `by-name/<name>.json`, and the `vibe-registry` index client + `vibe-publish` post-publish hook were realigned. **Remaining:** Phase 5 (index-backed short-name resolution at the CLI boundary, §2.6), Phase 6 (collision detection + exit code `7`, §2.7), Phase 8 (canonical-package migration + the `VIBEVM-SPEC.md §7.1` edit + docs, §3). @status:spec/done
- @fact:HISTORY-PHASES-5-6-8 **2026-05-23 — Phases 5 + 6 + 8 shipped with M1.19.** Short-name resolution at the CLI input boundary (`vibe-cli::commands::short_name` — index-backed candidate sets, lockfile-prefers-locked); collision detection with the dedicated exit code `7` (`InstallError::AmbiguousPackage`); the live-registry migration to `fqdn` naming and the `vibe init` default fix (`cc32d7e` — the M1.19 defect AUDIT 2026-05-23-02 records). This entry back-fills the record: the work shipped with M1.19 but the history was not updated at the time. @status:spec/done
- @fact:HISTORY-UNIT-TYPING **2026-06-12 — unit typing (the depth program).** §2.1–2.8 typed `req r1`; §2.9 and §3 typed `design r1`; the Status line updated from the stale DRAFT to the shipped reality. @status:spec/done
