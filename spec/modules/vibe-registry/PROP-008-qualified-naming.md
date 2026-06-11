# PROP-008: Qualified package naming — groups, short aliases, collision detection {#root}

**Milestone:** design proposal; targets a new `M1.18` ([`ROADMAP.md`](../../../ROADMAP.md)). Not implementation-locked.
**Status:** IMPLEMENTED — Phases 1–4 + 7 landed 2026-05-22 (M1.18, see §7); Phases 5–6 + 8 landed with M1.19 (index-backed short-name resolution at the CLI boundary — `vibe-cli::commands::short_name`; collision detection with exit code `7` — `InstallError::AmbiguousPackage`; the live-registry fqdn migration). Decision units typed at REQ grain 2026-06-12 (the depth program).
**Related:** [PROP-002 §2.1 / §3.4](PROP-002-decentralized-registry.md) (content-addressed identity; the rejection of *per-registry* identity — and why `group` does not violate it); [PROP-005](../vibe-index/PROP-005-package-index.md) (per-org index — **required** for short-name resolution); [PROP-007](../vibe-workspace/PROP-007-workspace.md) (workspace — companion document, same design session); [`VIBEVM-SPEC.md` §4.1 / §7.1](../../../VIBEVM-SPEC.md) (the four kinds; current `name`-uniqueness rule).
**Design rationale:** [`spec/design/workspace-and-qualified-naming.md`](../../design/workspace-and-qualified-naming.md) — the *why* and the lore behind this PROP: the owner's mental model, the fork-by-fork decision record, the Cargo-vs-Maven precedents. Non-normative; this PROP is the contract.
**Owner sanction:** the owner granted (2026-05-20) explicit sanction to edit any specification, including `VIBEVM-SPEC.md` §7.1. PROP-008 is the requirements record; the `VIBEVM-SPEC.md` edit lands at implementation time.

---

## 1. Motivation {#motivation}

vibevm's package namespace is flat: a pkgref is `<kind>:<name>`, and `name` is "globally unique within its kind" (`VIBEVM-SPEC.md` §7.1). This does not scale — two unrelated authors will both want `flow:wal`. Maven solved exactly this with `groupId` (reverse-FQDN) for global uniqueness; npm with `@scope/`.

The owner's request (design session 2026-05-20): introduce reverse-FQDN qualification at the top level (`org.vibevm`), while keeping short names usable — a user types `vibe install wal` in the CLI, but the package is canonically `org.vibevm/wal`. On a name collision, show alternatives; on a dependency conflict, fail without applying the plan; under full-auto, fail rather than guess.

**Why this does not violate [PROP-002 §3.4](PROP-002-decentralized-registry.md).** PROP-002 §3.4 rejected *per-registry identity* — `vibespecs/flow:wal` must not be a different identity from `corporate/flow:wal`, because that would make mirror-switching impossible. `group` is **not** the registry. `group` is an attribute of the *package* (exactly as Maven's `groupId` is an attribute of the artifact, not of the repository serving it). The registry remains a runtime resolution detail. Adding `group` to the identity tuple is orthogonal to §3.4 and does not reopen it.

PROP-008 covers the naming axis. The companion [PROP-007](../vibe-workspace/PROP-007-workspace.md) covers workspaces; the two were specified together.

---

## 2. Decisions {#decisions}

### 2.1 The `group` field {#group}

`req r1`

**Decision.** `[package]` gains a **mandatory** `group` field:

```toml
[package]
kind    = "flow"
name    = "wal"
group   = "org.vibevm"
version = "0.3.0"
```

- Reverse-FQDN is the **recommended convention**; the core does **not** enforce it. Whether `group` looks like a reversed domain is a matter of style, left to humans and linters. (Maven likewise does not enforce groupId shape.)
- Grammar: dot-separated segments, each `[a-z0-9_-]+`, ASCII lowercase.
- `group` is mandatory as of this PROP. The three current canonical packages migrate to `group = "org.vibevm"` (§3) — the owner's reverse-FQDN, recorded here as the canonical group for all first-party vibevm packages (domain `vibevm.org`).

### 2.2 Identity tuple — `(group, name, version, content_hash)` {#identity}

`req r1`

**Decision.** Package identity becomes `(group, name, version, content_hash)`. `kind` **leaves the identity tuple**.

- `name` becomes unique **within a `group`** (was: within a `kind`, `VIBEVM-SPEC.md` §7.1). `(group, name)` is therefore unique on its own — `kind` is no longer needed to disambiguate.
- `content_hash` is unchanged — computed over package file bytes per [PROP-002 §2.1](PROP-002-decentralized-registry.md#identity). `group` lives in `vibe.toml`, so it influences the hash only as ordinary file content; the tuple lists it explicitly so that changing `group` yields a different package.
- Changing a package's `group` is a new package, not a rename — same discipline as changing `name`.

### 2.3 `kind` becomes pure metadata {#kind}

`req r1`

**Decision.** `kind` (`flow` / `feat` / `stack` / `tool`) stays a **mandatory `[package]` field** but is now a pure attribute — it identifies nothing and names nothing.

It is still needed for:
- content placement — `spec/flows/` vs `spec/feats/` vs `spec/stacks/`;
- the `--kind` filter on `vibe list` / `vibe search`;
- the UX signal in a kind-prefixed pkgref (§2.4).

The four-kinds taxonomy (`VIBEVM-SPEC.md` §4.1) is unchanged in importance — it simply stops being part of identity and repository naming.

### 2.4 pkgref grammar {#pkgref}

`req r1`

**Decision.** The pkgref grammar gains an optional `group` segment and makes the `kind` prefix optional:

```
pkgref := [ <kind> ":" ] [ <group> "/" ] <name> [ "@" <version> ]
```

The `group`↔`name` separator is `/` (`:` is taken by `kind`, `@` by version).

| Form | Context | Behaviour |
|---|---|---|
| `org.vibevm/wal` | qualified — the form written into manifests (see §2.6, [PROP-002](PROP-002-decentralized-registry.md)) | resolved exactly |
| `flow:org.vibevm/wal` | qualified + kind | resolved exactly; **kind validated against the manifest** |
| `wal` | short — CLI sugar | resolved via the index (§2.6) |
| `flow:wal` | short + kind | resolved via the index; kind validated |

- **kind validation.** If the `kind` prefix is present, after resolution the resolver asserts `resolved.kind == prefix`; mismatch is a `KindMismatch` error. A kind prefix is validation + a UX signal — it does **not** disambiguate, because by §2.2 `name` is unique within a `group`, so `flow:org.vibevm/wal` and `feat:org.vibevm/wal` cannot co-exist. A short-name collision is always a *group* collision (§2.7), resolved by group-qualification, never by kind.
- **The short form is CLI-only sugar.** It is never written to a manifest (§2.6).

### 2.5 Repository naming — `naming = "fqdn"` {#repo-naming}

`req r1`

**Decision.** `kind` leaves the repository name. A new `[[registry]]` naming convention value:

```toml
[[registry]]
name   = "vibespecs"
url    = "https://github.com/vibespecs"
naming = "fqdn"          # repo name = "<group>.<name>"  →  org.vibevm.wal
```

- `naming = "fqdn"` maps a pkgref to the repository name `<group>.<name>` — a clean, flat reverse-FQDN (`org.vibevm.wal`). Dots in repository names are accepted by both GitHub and GitVerse (Gitea-shape).
- Because `(group, name)` is unique (§2.2), `<group>.<name>` is a collision-free repo name without needing `kind`. The existing `kind-name` / `name` / `kind/name` conventions (PROP-002 §2.2) remain for registries that have not adopted `group`.
- This realises the owner's "short name in the CLI, fat name in the repository" goal: the repository is the pure reverse-FQDN; the CLI keeps the short alias.

### 2.6 Short-name resolution {#short-name}

`req r1`

**Decision.** A short name (`wal`, `flow:wal`) is resolved **only at the CLI input boundary**, via the index. Manifests always store the qualified form.

- `vibe install wal` resolves the collision once, at the top level, and writes `org.vibevm/wal` into `[requires]`. Manifests are therefore always qualified — exactly the cargo/npm pattern (`cargo add serde` on the CLI, `serde = "1"` in `Cargo.toml`).
- **Consequence — no transitive collisions.** Every package's `[requires]` is qualified (its author published through the same flow). The dependency graph is built from qualified names; short-name resolution never recurses into the graph. It happens once, for a human-typed CLI argument.
- **Index dependency.** Resolving a short name requires enumerating candidates `(*, name)` across registries. The host cannot list an org cheaply ([PROP-005 §1](../vibe-index/PROP-005-package-index.md) — GitVerse exposes no org listing, GitHub is rate-limited). Therefore short-name resolution **requires [PROP-005](../vibe-index/PROP-005-package-index.md)**: one HTTP GET of `by-name/<name>.json` per registry yields the candidate set. Without an index, a registry's short names are unavailable and the qualified form is required.
- **Lockfile is authoritative.** If `vibe.lock` already pins `org.vibevm/wal`, a later `vibe install wal` resolves to the locked entry — the short name prefers what is already locked.

### 2.7 Collision vs conflict {#collision}

`req r1`

**Decision.** Two distinct failure classes, with distinct handling. This terminology is fixed by this PROP.

- **Collision (a naming ambiguity).** Two *different* packages match one short name (`wal`) with different `group`. Detected during short-name resolution (§2.6).
- **Conflict (a dependency conflict).** The depsolver cannot satisfy version constraints — incompatible constraints, declared `[conflicts]`, an unsatisfiable diamond. Already handled (PROP-002 §2.9 — resolvo/libsolv conflict-explanation chain).

Collision handling (new):

- The resolver collects *all* candidates of a short name — it does **not** stop at the first registry. (PROP-002 §2.2's first-match-wins remains correct for the *same* package mirrored across registries — identical identity. It is wrong for *different* packages sharing a short name; the two are distinguishable only once `group` exists.)
- One candidate → resolve. Multiple candidates with different identity → **collision**:
  - interactive TTY — print the alternatives and fail with a hint pointing at the qualified form (no interactive pick: the choice must be recorded deliberately, not clicked);
  - `--unattended` / full-auto — fail-fast; the resolver never guesses.
- A new exit code **`7`** ("ambiguous package") is assigned, distinct from `3` ("package conflict", `VIBEVM-SPEC.md` §9.4).

```
flow:wal is ambiguous — 2 packages match:
  1. org.vibevm/wal   (registry vibespecs)
  2. com.acme/wal     (registry acme-internal)
Re-run with the qualified form, e.g. `vibe install org.vibevm/wal`.
```

Conflict handling is unchanged: the install pipeline is already atomic (resolve → plan → confirm → apply); a failed resolve never reaches apply — "fail without applying the plan", as the owner specified.

### 2.8 Index extension {#index-ext}

`req r1`

**Decision.** [PROP-005](../vibe-index/PROP-005-package-index.md)'s entry schema (§2.6) gains two fields: `group` (mandatory, §2.1) and `workspace_origin` (optional — set when the package was published from a workspace, [PROP-007 §2.8](../vibe-workspace/PROP-007-workspace.md) `[origin]`). The `by-name/` layer indexes by `name` and returns the candidate set with each candidate's `group`, so §2.6 short-name resolution is one GET per registry. PROP-005 is currently a draft; these are edits to a draft, not a shipped contract.

### 2.9 Registry explorer {#explorer}

`design r1`

**Decision (forward-looking, out of implementation scope).** The index makes a Maven-Central-style browsable visualisation possible — and richer. A **vibevm registry explorer** is recorded here as a long-term direction (a `ROADMAP.md` M3+ entry):

- a reverse-FQDN group tree with drill-down (`org` → `org.vibevm` → packages → versions), as Maven Central does;
- beyond Maven Central: filter by `kind`; a capability graph (`[provides]`/`[requires]`); `describes`/PURL links to upstream libraries; redirect-stub delegation; the full dependency DAG; and **workspace provenance** ("Y is a sub-package of X", from `workspace_origin`).

The explorer is a separate, optional layer over the index — not part of PROP-008's implementation. PROP-005 §2.10 already reserves the hook (`vibe-index serve`, CORS-open read endpoints). The only obligation on this refactor is that the index carry `group` and `workspace_origin` (§2.8) so the explorer is not a retrofit later.

---

## 3. Migration {#migration}

`design r1`

The breaking-change window is open: vibevm has no public release, no external users ([PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) — "schema churn before v0.1.0 is free").

- **Canonical packages.** `flow-wal`, `flow-sync-from-code`, `flow-atomic-commits` migrate to `group = "org.vibevm"`. Repositories rename to the `naming = "fqdn"` shape (`org.vibevm.wal`, …). The owner authorised migrating the test fixtures and these three without further questions.
- **Test orgs.** `vibespecstest1/2/3` fixtures re-laid-out to the new naming.
- **Manifests.** `vibe-package.toml` → `vibe.toml` ([PROP-007 §2.2](../vibe-workspace/PROP-007-workspace.md)); add the `group` field.
- **Lockfile.** Schema bumps to **v5** — PROP-007 had already taken v4 for `source_kind = "path"`; adds the `group` field per `[[package]]`.
- **`VIBEVM-SPEC.md` §7.1** is edited (under the owner sanction) — the `name`-uniqueness rule changes from "within a kind" to "within a group", and the identity tuple and pkgref grammar are updated.

---

## 4. Rejected alternatives {#rejected}

- **Per-registry identity.** Already rejected in [PROP-002 §3.4](PROP-002-decentralized-registry.md). `group` is a package attribute, not a registry — §1 explains why it does not reopen that decision.
- **`kind` in the repository name.** Rejected (this PROP, §2.5). With `(group, name)` unique, `kind` in the repo name is redundant noise; `naming = "fqdn"` drops it.
- **Short names inside manifests.** Rejected (§2.6). Manifests store the qualified form; short names are CLI-only sugar. This eliminates transitive collisions by construction.
- **kind prefix as a disambiguator.** Rejected (§2.4). With `name` unique within `group`, the kind prefix can only validate, never disambiguate; a real ambiguity is a group collision.

---

## 5. Open questions {#open}

1. Exit code `7` — finalise the assignment against `VIBEVM-SPEC.md` §9.4 and confirm no clash with a future code.
2. Registry explorer scope (§2.9) — when (if) it becomes a funded milestone, it gets its own PROP.
3. Whether `naming = "fqdn"` should also offer a `kind`-bearing variant for registries that want it, or stay strictly `<group>.<name>`.

---

## 6. Phase plan {#phases}

PROP-008 depends on [PROP-005](../vibe-index/PROP-005-package-index.md) being implemented (short-name resolution, §2.6) and is best sequenced after [PROP-007](../vibe-workspace/PROP-007-workspace.md). Suggested order: PROP-007 (workspace) → PROP-005 implementation (index) → PROP-008 (qualified naming) → collision-detection slice (§2.7). The `group` field, identity-tuple change, pkgref grammar, and `naming = "fqdn"` can land before short-name resolution; short-name resolution and collision detection land once the index is real.

---

## 7. Version history {#history}

- **2026-05-20 — draft 1.** Initial proposal. Requirements locked in an owner design session (decisions on `group`, identity tuple, `kind`-as-metadata, pkgref grammar, `fqdn` repo naming, index-backed short-name resolution, collision detection, exit code 7, registry explorer as a long-term direction). Open for review.
- **2026-05-22 — Phases 1–4 + 7 implemented (under MFBT).** The identity core landed on `main`: the `Group` newtype and the mandatory `[package].group` (Phase 1); the `(group, name, version, content_hash)` identity refactor with `kind` demoted to metadata (Phase 2); the lockfile `group` field at schema v5 (Phase 3); the group-native registry with `NamingConvention::Fqdn` as the default (Phase 4). Phase 7 (§2.8) then made the package index group-native — the [PROP-005](../vibe-index/PROP-005-package-index.md) entry schema gained `group` + `workspace_origin`, the `by-name/` layer became the candidate-set file `by-name/<name>.json`, and the `vibe-registry` index client + `vibe-publish` post-publish hook were realigned. **Remaining:** Phase 5 (index-backed short-name resolution at the CLI boundary, §2.6), Phase 6 (collision detection + exit code `7`, §2.7), Phase 8 (canonical-package migration + the `VIBEVM-SPEC.md §7.1` edit + docs, §3).
- **2026-05-23 — Phases 5 + 6 + 8 shipped with M1.19.** Short-name resolution at the CLI input boundary (`vibe-cli::commands::short_name` — index-backed candidate sets, lockfile-prefers-locked); collision detection with the dedicated exit code `7` (`InstallError::AmbiguousPackage`); the live-registry migration to `fqdn` naming and the `vibe init` default fix (`cc32d7e` — the M1.19 defect AUDIT 2026-05-23-02 records). This entry back-fills the record: the work shipped with M1.19 but the history was not updated at the time.
- **2026-06-12 — unit typing (the depth program).** §2.1–2.8 typed `req r1`; §2.9 and §3 typed `design r1`; the Status line updated from the stale DRAFT to the shipped reality.
