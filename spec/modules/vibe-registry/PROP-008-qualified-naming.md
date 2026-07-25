# PROP-008: Qualified package naming — groups, short aliases, collision detection {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: IMPLEMENTED, M1.18 + M1.19"/>

##milestone-line **Milestone:** `M1.18` + `M1.19` ([`ROADMAP.md`](../../../ROADMAP.md)) — **shipped**, implementation-locked. (The line read "design proposal … not implementation-locked" until 2026-07-25; it had never been reconciled with the IMPLEMENTED status one line below.) @impl/done

##status-line **Status:** IMPLEMENTED — Phases 1–4 + 7 landed 2026-05-22 (M1.18, see §7); Phases 5–6 + 8 landed with M1.19 (index-backed short-name resolution at the CLI boundary — `vibe-cli::commands::short_name`; collision detection with exit code `7` — `InstallError::AmbiguousPackage`; the live-registry fqdn migration). Decision units typed at REQ grain 2026-06-12 (the depth program). @impl/done

##related **Related:** [PROP-002 §2.1 / §3.4](PROP-002-decentralized-registry.md) (content-addressed identity; the rejection of *per-registry* identity — and why `group` does not violate it); [PROP-005](../vibe-index/PROP-005-package-index.md) (per-org index — **required** for short-name resolution); [PROP-007](../vibe-workspace/PROP-007-workspace.md) (workspace — companion document, same design session); [`VIBEVM-SPEC.md` §4.1 / §7.1](../../../VIBEVM-SPEC.md) (the installable kinds; current `name`-uniqueness rule). @spec/done

##design-rationale **Design rationale:** [`spec/design/workspace-and-qualified-naming.md`](../../design/workspace-and-qualified-naming.md) — the *why* and the lore behind this PROP: the owner's mental model, the fork-by-fork decision record, the Cargo-vs-Maven precedents. Non-normative; this PROP is the contract. @spec/done

##discipline-line **Discipline:** the general namespace-scaling laws — why a flat namespace fails, the mandatory *group*, the identity tuple, why a rename is a new identity, why short names live only at the human boundary, and why a *collision* and a *conflict* are different failures — are the `qualified-naming` flow: `spec://org.vibevm.world/qualified-naming/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL#root`. This PROP is vibevm's IMPLEMENTED instance of them — the `org.vibevm.*` groups, the CLI short-name resolution (`vibe-cli::commands::short_name`), and collision detection (`InstallError::AmbiguousPackage`). @spec/done

##OWNER-SANCTION **Owner sanction:** the owner granted (2026-05-20) explicit sanction to edit any specification, including `VIBEVM-SPEC.md` §7.1. PROP-008 is the requirements record; the `VIBEVM-SPEC.md` edit lands at implementation time. @impl/done

---

## 1. Motivation {#motivation}

- ##flat-namespace vibevm's package namespace is flat: a pkgref is `<kind>:<name>`, and `name` is "globally unique within its kind" (`VIBEVM-SPEC.md` §7.1). This does not scale — two unrelated authors will both want `flow:wal`. @impl/done
- ##prior-art Maven solved exactly this with `groupId` (reverse-FQDN) for global uniqueness; npm with `@scope/`. @impl/done

- ##OWNER-REQUEST The owner's request (design session 2026-05-20): introduce reverse-FQDN qualification at the top level (`org.vibevm`), while keeping short names usable — a user types `vibe install wal` in the CLI, but the package is canonically `org.vibevm.world/wal`. @impl/done
- ##collision-behavior-request On a name collision, show alternatives; on a dependency conflict, fail without applying the plan; under full-auto, fail rather than guess. @impl/done

- ##why-not-per-registry **Why this does not violate [PROP-002 §3.4](PROP-002-decentralized-registry.md).** PROP-002 §3.4 rejected *per-registry identity* — `vibespecs/flow:wal` must not be a different identity from `corporate/flow:wal`, because that would make mirror-switching impossible. @impl/done
- ##GROUP-NOT-REGISTRY `group` is **not** the registry. `group` is an attribute of the *package* (exactly as Maven's `groupId` is an attribute of the artifact, not of the repository serving it). @impl/done
- ##group-orthogonal The registry remains a runtime resolution detail. Adding `group` to the identity tuple is orthogonal to §3.4 and does not reopen it. @impl/done

##naming-axis PROP-008 covers the naming axis. The companion [PROP-007](../vibe-workspace/PROP-007-workspace.md) covers workspaces; the two were specified together. @impl/done

---

## 2. Decisions {#decisions}

### 2.1 The `group` field {#group}

##req-group `req r1` @impl/done

##GROUP-MANDATORY **Decision.** `[package]` gains a **mandatory** `group` field: @impl/done

```toml
[package]
kind    = "flow"
name    = "wal"
group   = "org.vibevm"
version = "0.3.0"
```

- ##GROUP-CONVENTION Reverse-FQDN is the **recommended convention**; the core does **not** enforce it. Whether `group` looks like a reversed domain is a matter of style, left to humans and linters. (Maven likewise does not enforce groupId shape.) @impl/done
- ##GROUP-GRAMMAR Grammar: dot-separated segments, each `[a-z0-9_-]+`, ASCII lowercase. @impl/done
- ##GROUP-CANONICAL `group` is mandatory as of this PROP. The three current canonical packages migrate to `group = "org.vibevm"` (§3) — the owner's reverse-FQDN, recorded here as the canonical group for all first-party vibevm packages (domain `vibevm.org`). @impl/done

### 2.2 Identity tuple — `(group, name, version, content_hash)` {#identity}

##req-identity `req r1` @impl/done

##IDENTITY-TUPLE **Decision.** Package identity becomes `(group, name, version, content_hash)`. `kind` **leaves the identity tuple**. @impl/done

- ##NAME-UNIQUE-IN-GROUP `name` becomes unique **within a `group`** (was: within a `kind`, `VIBEVM-SPEC.md` §7.1). `(group, name)` is therefore unique on its own — `kind` is no longer needed to disambiguate. @impl/done
- ##HASH-UNCHANGED `content_hash` is unchanged — computed over package file bytes per [PROP-002 §2.1](PROP-002-decentralized-registry.md#identity). `group` lives in `vibe.toml`, so it influences the hash only as ordinary file content; the tuple lists it explicitly so that changing `group` yields a different package. @impl/done
- ##GROUP-CHANGE-NEW-PACKAGE Changing a package's `group` is a new package, not a rename — same discipline as changing `name`. @impl/done

### 2.3 `kind` becomes pure metadata {#kind}

##req-kind `req r1` @impl/done

##KIND-METADATA **Decision.** `kind` (`flow` / `feat` / `stack` / `tool`) stays a **mandatory `[package]` field** but is now a pure attribute — it identifies nothing and names nothing. @impl/done

##kind-still-needed-lead It is still needed for: @impl/done
- ##KIND-PLACEMENT content placement — `spec/flows/` vs `spec/feats/` vs `spec/stacks/`; @impl/done
- ##KIND-FILTER the `--kind` filter on `vibe list` / `vibe search`; @impl/done
- ##KIND-UX the UX signal in a kind-prefixed pkgref (§2.4). @impl/done

##TAXONOMY-UNCHANGED The four-kinds taxonomy (`VIBEVM-SPEC.md` §4.1) is unchanged in importance — it simply stops being part of identity and repository naming. @impl/done

### 2.4 pkgref grammar {#pkgref}

##req-pkgref `req r1` @impl/done

##PKGREF-GRAMMAR **Decision.** The pkgref grammar gains an optional `group` segment and makes the `kind` prefix optional: @impl/done

```
pkgref := [ <kind> ":" ] [ <group> "/" ] <name> [ "@" <version> ]
```

##PKGREF-SEPARATOR The `group`↔`name` separator is `/` (`:` is taken by `kind`, `@` by version). @impl/done

| Form | Context | Behaviour |
|---|---|---|
| ##ROW-QUALIFIED `org.vibevm.world/wal` @impl/done | qualified — the form written into manifests (see §2.6, [PROP-002](PROP-002-decentralized-registry.md)) @impl/done | resolved exactly @impl/done |
| ##ROW-QUALIFIED-KIND `flow:org.vibevm.world/wal` @impl/done | qualified + kind @impl/done | resolved exactly; **kind validated against the manifest** @impl/done |
| ##ROW-SHORT `wal` @impl/done | short — CLI sugar @impl/done | resolved via the index (§2.6) @impl/done |
| ##ROW-SHORT-KIND `flow:wal` @impl/done | short + kind @impl/done | resolved via the index; kind validated @impl/done |

- ##KIND-VALIDATION **kind validation.** If the `kind` prefix is present, after resolution the resolver asserts `resolved.kind == prefix`; mismatch is a `KindMismatch` error. A kind prefix is validation + a UX signal — it does **not** disambiguate, because by §2.2 `name` is unique within a `group`, so `flow:org.vibevm.world/wal` and `feat:org.vibevm.world/wal` cannot co-exist. A short-name collision is always a *group* collision (§2.7), resolved by group-qualification, never by kind. @impl/done
- ##SHORT-CLI-ONLY **The short form is CLI-only sugar.** It is never written to a manifest (§2.6). @impl/done

### 2.5 Repository naming — `naming = "fqdn"` {#repo-naming}

##req-repo-naming `req r1` @impl/done

##FQDN-NAMING **Decision.** `kind` leaves the repository name. A new `[[registry]]` naming convention value: @impl/done

```toml
[[registry]]
name   = "vibespecs"
url    = "https://github.com/vibespecs"
naming = "fqdn"          # repo name = "<group>_<name>"  →  org.vibevm.world_wal
```

- ##JOINER-UNDERSCORE `naming = "fqdn"` maps a pkgref to the repository name `<group>_<name>` (`org.vibevm.world/wal` → `org.vibevm.world_wal`) — a flat reverse-FQDN whose group and name are joined by `_`. The joiner is `_` on purpose: a repository name cannot contain `/` (GitHub and GitVerse both restrict it to `[A-Za-z0-9._-]`), and `_` is the one character in **neither** the group (`[a-z0-9.-]`) **nor** the name (`[a-z0-9-]`), so the coordinate stays algorithmically splittable. A dot would be ambiguous — groups are dotted reverse-DNS, so `<group>.<name>` hides the boundary. This is the flat-carrier case of vibevm PROP-029's one invariant: the group↔name joiner is never `.` — it is `/` where a path segment exists (pkgrefs, `spec://`) and `_` where a single flat token is required (repo names). @impl/done
- ##COLLISION-FREE-REPO Because `(group, name)` is unique (§2.2), `<group>_<name>` is a collision-free repo name without needing `kind`. The existing `kind-name` / `name` / `kind/name` conventions (PROP-002 §2.2) remain for registries that have not adopted `group`. @impl/done
- ##SHORT-CLI-FAT-REPO This realises the owner's "short name in the CLI, fat name in the repository" goal: the repository is the pure reverse-FQDN; the CLI keeps the short alias. @impl/done

### 2.6 Short-name resolution {#short-name}

##req-short-name `req r1` @impl/done

##SHORT-AT-BOUNDARY **Decision.** A short name (`wal`, `flow:wal`) is resolved **only at the CLI input boundary**, via the index. Manifests always store the qualified form. @impl/done

- ##RESOLVE-ONCE-WRITE-QUALIFIED `vibe install wal` resolves the collision once, at the top level, and writes `org.vibevm.world/wal` into `[requires]`. Manifests are therefore always qualified — exactly the cargo/npm pattern (`cargo add serde` on the CLI, `serde = "1"` in `Cargo.toml`). @impl/done
- ##NO-TRANSITIVE-COLLISIONS **Consequence — no transitive collisions.** Every package's `[requires]` is qualified (its author published through the same flow). The dependency graph is built from qualified names; short-name resolution never recurses into the graph. It happens once, for a human-typed CLI argument. @impl/done
- ##INDEX-DEPENDENCY **Index dependency.** Resolving a short name requires enumerating candidates `(*, name)` across registries. The host cannot list an org cheaply ([PROP-005 §1](../vibe-index/PROP-005-package-index.md) — GitVerse exposes no org listing, GitHub is rate-limited). Therefore short-name resolution **requires [PROP-005](../vibe-index/PROP-005-package-index.md)**: one HTTP GET of `by-name/<name>.json` per registry yields the candidate set. Without an index, a registry's short names are unavailable and the qualified form is required. @impl/done
- ##LOCKFILE-AUTHORITATIVE **Lockfile is authoritative.** If `vibe.lock` already pins `org.vibevm.world/wal`, a later `vibe install wal` resolves to the locked entry — the short name prefers what is already locked. @impl/done

### 2.7 Collision vs conflict {#collision}

##req-collision `req r1` @impl/done

##TWO-FAILURE-CLASSES **Decision.** Two distinct failure classes, with distinct handling. This terminology is fixed by this PROP. @impl/done

- ##COLLISION-DEF **Collision (a naming ambiguity).** Two *different* packages match one short name (`wal`) with different `group`. Detected during short-name resolution (§2.6). @impl/done
- ##CONFLICT-DEF **Conflict (a dependency conflict).** The depsolver cannot satisfy version constraints — incompatible constraints, declared `[conflicts]`, an unsatisfiable diamond. Already handled (PROP-002 §2.9 — resolvo/libsolv conflict-explanation chain). @impl/done

##collision-handling-lead Collision handling (new): @impl/done

- ##COLLECT-ALL-CANDIDATES The resolver collects *all* candidates of a short name — it does **not** stop at the first registry. (PROP-002 §2.2's first-match-wins remains correct for the *same* package mirrored across registries — identical identity. It is wrong for *different* packages sharing a short name; the two are distinguishable only once `group` exists.) @impl/done
- ##COLLISION-BEHAVIOR One candidate → resolve. Multiple candidates with different identity → **collision**: @impl/done
  - ##collision-interactive interactive TTY — print the alternatives and fail with a hint pointing at the qualified form (no interactive pick: the choice must be recorded deliberately, not clicked); @impl/done
  - ##collision-unattended `--unattended` / full-auto — fail-fast; the resolver never guesses. @impl/done
- ##EXIT-CODE-7 A new exit code **`7`** ("ambiguous package") is assigned, distinct from `3` ("package conflict", `VIBEVM-SPEC.md` §9.4). @impl/done

```
flow:wal is ambiguous — 2 packages match:
  1. org.vibevm.world/wal   (registry vibespecs)
  2. com.acme/wal     (registry acme-internal)
Re-run with the qualified form, e.g. `vibe install org.vibevm.world/wal`.
```

##CONFLICT-UNCHANGED Conflict handling is unchanged: the install pipeline is already atomic (resolve → plan → confirm → apply); a failed resolve never reaches apply — "fail without applying the plan", as the owner specified. @impl/done

### 2.8 Index extension {#index-ext}

##req-index-ext `req r1` @impl/done

##INDEX-FIELDS **Decision.** [PROP-005](../vibe-index/PROP-005-package-index.md)'s entry schema (§2.6) gains two fields: `group` (mandatory, §2.1) and `workspace_origin` (optional — set when the package was published from a workspace, [PROP-007 §2.8](../vibe-workspace/PROP-007-workspace.md) `[origin]`). @impl/done

- ##BY-NAME-CANDIDATES The `by-name/` layer indexes by `name` and returns the candidate set with each candidate's `group`, so §2.6 short-name resolution is one GET per registry. @impl/done
- ##draft-edit-note PROP-005 is currently a draft; these are edits to a draft, not a shipped contract. @spec/done

### 2.9 Registry explorer {#explorer}

##design-explorer `design r1` @spec/done

##EXPLORER-DIRECTION **Decision (forward-looking, out of implementation scope).** The index makes a Maven-Central-style browsable visualisation possible — and richer. A **vibevm registry explorer** is recorded here as a long-term direction (a `ROADMAP.md` M3+ entry): @spec/done

- ##explorer-group-tree a reverse-FQDN group tree with drill-down (`org` → `org.vibevm` → packages → versions), as Maven Central does; @spec/done
- ##explorer-beyond-maven beyond Maven Central: filter by `kind`; a capability graph (`[provides]`/`[requires]`); `describes`/PURL links to upstream libraries; redirect-stub delegation; the full dependency DAG; and **workspace provenance** ("Y is a sub-package of X", from `workspace_origin`). @spec/done

- ##EXPLORER-SEPARATE-LAYER The explorer is a separate, optional layer over the index — not part of PROP-008's implementation. PROP-005 §2.10 already reserves the hook (`vibe-index serve`, CORS-open read endpoints). @spec/done
- ##EXPLORER-INDEX-OBLIGATION The only obligation on this refactor is that the index carry `group` and `workspace_origin` (§2.8) so the explorer is not a retrofit later. @spec/done

---

## 3. Migration {#migration}

##design-migration `design r1` @impl/done

##breaking-window The breaking-change window is open: vibevm has no public release, no external users ([PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) — "schema churn before v0.1.0 is free"). @impl/done

- ##MIG-CANONICAL **Canonical packages.** `flow-wal`, `flow-sync-from-code`, `flow-atomic-commits` migrate to `group = "org.vibevm"`. Repositories rename to the `naming = "fqdn"` shape (`org.vibevm_wal`, …). The owner authorised migrating the test fixtures and these three without further questions. **Superseded in part by [PROP-029](../../common/PROP-029-fully-qualified-addresses.md):** the redbook trio's *local* group later moved to `org.vibevm.world`, so their real repos render `org.vibevm.world_wal`; the still-published `org.vibevm` repos and the group-`org.vibevm` test fixtures trail that move and rename on their next publish. @impl/done
- ##MIG-TEST-ORGS **Test orgs.** `vibespecstest1/2/3` fixtures re-laid-out to the new naming. @impl/done
- ##MIG-MANIFESTS **Manifests.** `vibe-package.toml` → `vibe.toml` ([PROP-007 §2.2](../vibe-workspace/PROP-007-workspace.md)); add the `group` field. @impl/done
- ##MIG-LOCKFILE-V5 **Lockfile.** Schema bumps to **v5** — PROP-007 had already taken v4 for `source_kind = "path"`; adds the `group` field per `[[package]]`. @impl/done
- ##MIG-SPEC-EDIT **`VIBEVM-SPEC.md` §7.1** is edited (under the owner sanction) — the `name`-uniqueness rule changes from "within a kind" to "within a group", and the identity tuple and pkgref grammar are updated. @impl/done

---

## 4. Rejected alternatives {#rejected}

- ##REJ-PER-REGISTRY **Per-registry identity.** Already rejected in [PROP-002 §3.4](PROP-002-decentralized-registry.md). `group` is a package attribute, not a registry — §1 explains why it does not reopen that decision. @spec/done
- ##REJ-KIND-IN-REPO **`kind` in the repository name.** Rejected (this PROP, §2.5). With `(group, name)` unique, `kind` in the repo name is redundant noise; `naming = "fqdn"` drops it. @spec/done
- ##REJ-SHORT-IN-MANIFESTS **Short names inside manifests.** Rejected (§2.6). Manifests store the qualified form; short names are CLI-only sugar. This eliminates transitive collisions by construction. @spec/done
- ##REJ-KIND-DISAMBIGUATOR **kind prefix as a disambiguator.** Rejected (§2.4). With `name` unique within `group`, the kind prefix can only validate, never disambiguate; a real ambiguity is a group collision. @spec/done

---

## 5. Open questions {#open}

1. ##OPEN-EXIT-CODE-7 Exit code `7` — finalise the assignment against `VIBEVM-SPEC.md` §9.4 and confirm no clash with a future code. @spec/work
2. ##OPEN-EXPLORER-SCOPE Registry explorer scope (§2.9) — when (if) it becomes a funded milestone, it gets its own PROP. @spec/work
3. ##OPEN-FQDN-KIND-VARIANT Whether `naming = "fqdn"` should also offer a `kind`-bearing variant for registries that want it, or stay strictly `<group>.<name>`. @spec/work

---

## 6. Phase plan {#phases}

##phase-plan PROP-008 depends on [PROP-005](../vibe-index/PROP-005-package-index.md) being implemented (short-name resolution, §2.6) and is best sequenced after [PROP-007](../vibe-workspace/PROP-007-workspace.md). Suggested order: PROP-007 (workspace) → PROP-005 implementation (index) → PROP-008 (qualified naming) → collision-detection slice (§2.7). The `group` field, identity-tuple change, pkgref grammar, and `naming = "fqdn"` can land before short-name resolution; short-name resolution and collision detection land once the index is real. @impl/done

---

## 7. Version history {#history}

- ##HISTORY-DRAFT-1 **2026-05-20 — draft 1.** Initial proposal. Requirements locked in an owner design session (decisions on `group`, identity tuple, `kind`-as-metadata, pkgref grammar, `fqdn` repo naming, index-backed short-name resolution, collision detection, exit code 7, registry explorer as a long-term direction). Open for review. @spec/done
- ##HISTORY-PHASES-1-4-7 **2026-05-22 — Phases 1–4 + 7 implemented (under MFBT).** The identity core landed on `main`: the `Group` newtype and the mandatory `[package].group` (Phase 1); the `(group, name, version, content_hash)` identity refactor with `kind` demoted to metadata (Phase 2); the lockfile `group` field at schema v5 (Phase 3); the group-native registry with `NamingConvention::Fqdn` as the default (Phase 4). Phase 7 (§2.8) then made the package index group-native — the [PROP-005](../vibe-index/PROP-005-package-index.md) entry schema gained `group` + `workspace_origin`, the `by-name/` layer became the candidate-set file `by-name/<name>.json`, and the `vibe-registry` index client + `vibe-publish` post-publish hook were realigned. **Remaining:** Phase 5 (index-backed short-name resolution at the CLI boundary, §2.6), Phase 6 (collision detection + exit code `7`, §2.7), Phase 8 (canonical-package migration + the `VIBEVM-SPEC.md §7.1` edit + docs, §3). @spec/done
- ##HISTORY-PHASES-5-6-8 **2026-05-23 — Phases 5 + 6 + 8 shipped with M1.19.** Short-name resolution at the CLI input boundary (`vibe-cli::commands::short_name` — index-backed candidate sets, lockfile-prefers-locked); collision detection with the dedicated exit code `7` (`InstallError::AmbiguousPackage`); the live-registry migration to `fqdn` naming and the `vibe init` default fix (`cc32d7e` — the M1.19 defect AUDIT 2026-05-23-02 records). This entry back-fills the record: the work shipped with M1.19 but the history was not updated at the time. @spec/done
- ##HISTORY-UNIT-TYPING **2026-06-12 — unit typing (the depth program).** §2.1–2.8 typed `req r1`; §2.9 and §3 typed `design r1`; the Status line updated from the stale DRAFT to the shipped reality. @spec/done
