# Design rationale: Workspace & qualified naming

**Companion to:** [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) (workspace), [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) (qualified naming).
**Status:** non-normative design record. Captured 2026-05-20 in an owner design session.
**Authority:** the PROPs are the contract. If this document and a PROP disagree, the PROP wins.

---

## 1. What this document is

PROP-007 and PROP-008 say *what* the workspace + qualified-naming refactor does. This document says *why* — and keeps the lore: the owner's mental model, the four-axis decomposition, every fork weighed during the session, the Cargo-vs-Maven precedents studied, the publication model that needed careful explaining, and the ideas that surfaced but were parked.

It exists because the design session that produced PROP-007/008 spent a large amount of reasoning that does not belong inside a contract-shaped PROP, yet would be expensive to lose at the next session boundary. See [`spec/design/README.md`](README.md) for the genre.

---

## 2. The owner's request and mental model

The owner asked for a structure "most like Maven submodules and cargo": a project that decomposes *naturally* into modules; modules that publish to a repository **individually** — or are deliberately **not** shared; and the whole structure declared right in the project description.

Two extremes were named as must-work cases, and they anchor the whole design:

- **Entirely invisible** — a fully structured project that lives nowhere in any external repository.
- **Entirely published** — the whole project, every sub-package individually, published.

The "everything in between" (some modules public, some private, some workspace-internal) must also be first-class.

A load-bearing phrase from the session, recorded verbatim because it shaped several decisions: *"the user works in a sub-project and doesn't even notice that it is actually a small piece of something bigger."* This is why commands bubble up to the workspace root (PROP-007 §2.4) and why there is one unified manifest rather than many files (§2.2) — "reading a million different files is hard for a human, and for a small dumb LLM agent too."

The spirit throughout: **flexibility and convention-over-configuration**. The owner repeatedly chose "support both, default sensibly" over "pick one." That preference resolved R3 (version inheritance: both modes) and W8 (member versioning: both).

---

## 3. The four axes

The request decomposes into four orthogonal axes. Keeping them separate was the single most clarifying move of the session — they have different cost, different dependencies, and ship as different milestones.

| Axis | Essence | Analogue |
|---|---|---|
| **A. Workspace** | Project = a set of modules; structure declared in `vibe.toml` | cargo `[workspace]`, Maven `<modules>` |
| **B. Selective publish** | Each module publishes / does not, by choice | cargo `publish = false`, Maven `deploy.skip` |
| **C. Qualified naming** | Reverse-FQDN at the top (`org.vibevm.wal`), short aliases (`wal`) kept | Maven `groupId:artifactId` |
| **D. Conflict-aware resolve** | Collision → show alternatives; conflict → fail without applying; full-auto → fail | npm / Maven |

Axis A alone closes roughly 80% of the request — multi-package projects, local cross-deps, selective publish, both extremes — and it depends on nothing. That was the good news delivered early: the heavy part (naming, discovery) is separable and can come later.

---

## 4. The dependency graph between axes

```
Axis A (workspace)          — independent ───────────────► do first
Axis B (selective publish)  — depends on A
Axis C (qualified naming)   — depends on PROP-005 (index)
Axis D (conflict-aware)     — depends on C; the conflict half partly exists already
```

The chain `D → C → PROP-005 index` is the key finding. Reasoning:

- **C needs the index.** A short name `wal` must be resolved to a qualified `org.vibevm.world/wal`. That requires enumerating candidates `(*, wal)` across registries. The host cannot list an org cheaply (PROP-005 §1: GitVerse exposes no org listing; GitHub is rate-limited). Therefore short-name resolution requires PROP-005 implemented.
- **D needs C.** A *collision* is two different packages sharing a short name. You can only tell a collision apart from a harmless mirror (the same package served by two registries) once `group` exists to distinguish them. Without `group`, first-match-wins is the only sane policy.
- **The conflict half of D already exists.** The depsolver (resolvo/libsolv per PROP-003), `[conflicts]`, and the conflict-explanation chain are in place; the install pipeline is already atomic (resolve → plan → confirm → apply), so a failed resolve never reaches apply. "Fail without applying the plan" is already true. Only *collision* detection is new.

This is why the recommended sequencing is **A → PROP-005 implementation → C → D**.

---

## 5. The fork-by-fork decision record

Every fork weighed in the session, the options, the choice, and the reasoning. This is the most valuable part to preserve — settled questions that would otherwise be re-litigated.

### Naming forks

- **Separator `group`↔`name`.** Chosen: `/` → `flow:org.vibevm.world/wal`. `:` is taken by `kind`, `@` by version; npm-scope (`@org/`) was rejected because `@` doubling with version would confuse.
- **Is `group` mandatory?** Chosen: **mandatory**. Maven makes groupId mandatory; an optional `group` creates a grey zone ("no group" vs "has group"). The three legacy packages migrate silently — the owner waved that through ("they are test packages anyway").
- **Enforce reverse-FQDN?** Chosen: **core does not enforce**. Whether `group` looks like a reversed domain is style — for humans and linters, not the resolver. Maven likewise does not enforce groupId shape.
- **Canonical group for vibevm.** `org.vibevm` (domain `vibevm.org`). Recorded in PROP-008 §2.1.
- **`kind` in the repository name?** The owner asked: can `kind` leave the repo name entirely? Yes — because identity is already URL-orthogonal (PROP-002 §2.1), the repo name identifies nothing. The one thing `kind` gave the name was disambiguation (`flow-wal` vs `feat-wal`); making `name` unique *within a group* (rather than within a kind) removes that need. Result: repo = `<group>.<name>` = `org.vibevm.wal`, `naming = "fqdn"`. `kind` becomes pure metadata and leaves the identity tuple.
- **`kind` prefix in pkgref — keep or drop?** Chosen: **optional but allowed, validated when present**. The owner's exact framing: if `flow:` is purely a UX feature, make it optional but possible; and if an install used the prefix explicitly, validate it matches the manifest. So `org.vibevm.world/wal` and `flow:org.vibevm.world/wal` are both legal; a present prefix is checked (`KindMismatch` on mismatch). It is validation + a UX signal — it never disambiguates, because `name` is unique within `group`.
- **Short name in manifests?** Chosen: **no — manifests store the qualified form**. The short name is CLI-only sugar; `vibe install wal` resolves once and writes `org.vibevm.world/wal`. This is the cargo/npm pattern (`cargo add serde` → `serde = "1"`). The decisive consequence: the dependency graph is built entirely from qualified names, so **transitive collisions vanish by construction** — short-name resolution only ever happens at the human-typed CLI boundary, not recursively through the graph.
- **Exit code for ambiguity.** Chosen: **new code `7`**, distinct from `3` (package conflict).

### Workspace forks (7a–7e)

- **7a — member is a "package" or a "project"?** Chosen: **universal node** — it can be both; one structure serves all roles.
- **7b — can a member be a consumer itself?** Chosen: **yes** — "and usually it should be: the user works in a sub-project without noticing it is a piece of something bigger." This drove command-bubbling.
- **7c — one lockfile per workspace or per member?** The owner asked back: how do Cargo and Maven do it? Answer studied (see §6). Chosen: **one `vibe.lock` at the absolute root** (cargo model). Commands inside a member bubble up to it — which *is* the mechanism for 7b's "doesn't notice."
- **7d — `[[registry]]` / `[active]` / `[llm]` shared or per-member?** Chosen: **cascade with override** — the root sets defaults, a member may override. This is safe for `[[registry]]` only because identity is URL-orthogonal: a member overrides *where to fetch from*, not *what* identity it gets; if two registries served a genuinely different `content_hash` under one name, that is a collision and axis D catches it.
- **7e — one manifest file or two?** Chosen: **one `vibe.toml` for everything**; `vibe-package.toml` is retired. Reason: a member is simultaneously a developed consumer and a publishable artifact — two files would duplicate `[requires]` and drift. Plus the owner's "read one file, not a million" point. The escape hatch noted: "if it becomes impossible without splitting — we'll split later."

### Forks resolved by the owner accepting recommendations

- **R1 — `[project]` vs `[package]`.** Chosen: 7-α — keep both sections distinct (a node is a non-publishable project *or* a publishable package), rather than folding `[project]` into a `[package]` with optional `kind`. Explicitness; `kind` stays strictly mandatory wherever `[package]` appears.
- **R2 — manifests store the qualified form** (see naming forks above) — confirmed.
- **R3 — version placeholders.** Chosen: named `[workspace.versions]` (Maven `<properties>` shape). And: **depth 2 is not enough — recursion to arbitrary nesting depth**. This *reversed* a default the assistant had set (W3: "nested workspaces forbidden"). The reversal is load-bearing — it is what makes the matryoshka resolution arbitrary-depth — and it raised the cost of PROP-007 (recursion in root discovery, member aggregation, placeholder resolution).
- **R4 — `org.vibevm`** (recorded above).
- **R5 — kind out of repo name + optional kind prefix** (recorded in naming forks).
- **R6 — owner sanction** to edit any specification, including the owner-frozen `VIBEVM-SPEC.md`. Granted 2026-05-20 after requirements were judged sufficiently complete.
- **dual-form path-dep** — `{ path, version }` both present: `path` for local development, `version` for when the consuming node is itself published. Required because the owner explicitly wants the mixed mode (some modules local, some published).

### Defaults the assistant set; the owner accepted silently (except W3)

W1 root discovery by walking up to `[workspace]`; W2 glob in `members`; **W3 nested workspaces — reversed by R3 into recursion**; W4 dependency cycles between members are an error; W5 path outside the workspace allowed but path-only consumers are non-publishable; W6 `vibe install` addressing (root vs `-p` vs cwd); W7 `vibe workspace publish` is non-atomic, stop-on-first-failure; W8 member versioning supports both independent and inherited; collision in an interactive run shows alternatives and fails (no interactive pick — the choice must be recorded deliberately); the lockfile is authoritative when resolving a short name; `group` grammar follows Maven groupId; lockfile schema bumps to v4.

---

## 6. Cargo vs Maven — the precedent lore

Both were studied point by point. The findings, kept here so a future session need not re-derive them.

| Aspect | Cargo | Maven |
|---|---|---|
| Lockfile | One `Cargo.lock` at the workspace root; no per-crate locks | **No lockfile at all** — a known reproducibility gap; the parent POM's `<dependencyManagement>` plays the "single source of versions" role |
| Resolution | Unified across the workspace — one version of each dep | "Nearest-wins" per build; reproducibility is discipline + fixed versions |
| Members | Each is a full crate with its own `Cargo.toml` | Each is a full module with its own `pom.xml` + `<parent>` |
| Version inheritance | `version.workspace = true` from `[workspace.package]` | Modules often share the parent's `${project.version}` |
| Centralised versions | `[workspace.dependencies]` + `{ workspace = true }` | `<dependencyManagement>` (per-artifact) + `<properties>` (named placeholders) |
| Nested workspaces | **Forbidden** — to avoid "which workspace is mine" ambiguity | Parent POMs nest to arbitrary depth |
| Publish | `cargo publish -p <crate>` — per-crate, to crates.io | `mvn deploy` — per-module; `<skip>` opts a module out |
| Top-level naming | Flat crate names on crates.io | `groupId:artifactId` — reverse-FQDN groupId |

The decisions vibevm drew from this:

- **Lockfile: Cargo model.** vibevm already mandates a lockfile (content-hash integrity, PROP-002 §2.1). Maven's lockless model would mean discarding working machinery. One `vibe.lock` at the absolute root.
- **Nested workspaces: vibevm permits them, unlike Cargo.** Cargo forbids them to dodge ambiguity. vibevm can afford them because it fixes "lock always at the absolute root" — that rule resolves the ambiguity deterministically. Maven shows arbitrary nesting works when version coordination aggregates upward.
- **Version placeholders: Maven `<properties>` shape**, because that is literally what the owner described ("write `0.0.1` once, reference it by name"). `[workspace.dependencies]` (Cargo's centralised per-pkgref defaults) is noted as a possible companion mechanism, not the primary one.
- **groupId: adopted as `group`** — but as a package attribute, not a registry attribute, which is why it does not reopen PROP-002 §3.4's rejection of per-registry identity.

---

## 7. The physical publication model

This needed careful explanation — the owner flagged it as confusing and asked directly.

The model: **the development tree is one source tree** (one git repo, or not in git at all if private). Workspace members are subdirectories; the split into packages is logical, at the resolver level. **Publishing is a separate operation that copies a package directory's content into a new, separate repository** in the registry org and tags it — exactly what `vibe registry publish` does today for one package, repeated per member by `vibe workspace publish`.

A nested package does **not** "surface" by moving files. Only a *copy of its content* is published, into its own repository, at publish time. The source tree stays unified — a monorepo for development; the registry holds split copies.

```
DEVELOPMENT — one tree, one git repo (or no git):
  my-project/
  ├── vibe.toml          [workspace] members = ["packages/X", "packages/Y"]
  └── packages/{X,Y}/    each: vibe.toml with [package]

PUBLISH — splits into separate repos:
  packages/X/  --content copy-->  <registry-org>/org.vibevm.X   tag v…
  packages/Y/  --content copy-->  <registry-org>/org.vibevm.Y   tag v…
  The development tree is NOT modified.
```

Recommendation: keep members as flat siblings, not physically nested. Logical hierarchy ("X is built from Y") is a path-dependency, not a nested directory. Physical nesting is supported (with subtree excision when publishing the outer package, cargo-style) but discouraged for the "holes in the tree" complexity.

---

## 8. "Do not contribute here" — the signalling lore

The owner asked: when a sub-package's published copy is cloned, how do we make it super-obvious that pull requests there are pointless and contribution belongs to the main project?

Prior art studied: AOSP / Chromium / Bazel keep read-only mirrors on GitHub with explicit "this is a mirror, do not send PRs, contribute upstream" banners. GitHub offers no "disable PRs only" switch — only full archival blocks PRs.

The layered answer (detail in PROP-007 §2.8): README banner as the first block; repo `description`; Issues disabled; a `PULL_REQUEST_TEMPLATE.md` STOP notice; and `archived = true` as the nuclear `--archive` option (full read-only — vibevm drives the unarchive→push→archive cycle on re-publish). Plus a machine-readable `[origin]` marker in the published copy, and a `published_repos = "read-only" | "open"` setting for operators who want the inverse model.

Terminology fixed: the published copy is a **published package repository**; the source of truth is the **workspace**. It is *not* a `[[mirror]]` (that term is taken — an availability copy of a registry).

---

## 9. Ideas parked for the future

- **vibevm registry explorer.** The owner asked whether a Maven-Central-style browsable visualisation is possible — and wanted it richer. Yes: the per-org index (PROP-005) carries the data. Beyond Maven Central: a reverse-FQDN group tree with drill-down, plus `kind` filtering, a capability graph, `describes`/PURL links to upstream libraries, redirect-stub delegation, the full dependency DAG, and workspace provenance ("sub-package of X" from the `[origin]` marker). Recorded as a `ROADMAP.md` M3+ entry and in PROP-008 §2.9. The only obligation on the refactor: the index must carry `group` and `workspace_origin` so the explorer is not a retrofit.
- **`[workspace.dependencies]`** — Cargo-style centralised per-pkgref version defaults, alongside the named placeholders. Deferred until a concrete need surfaces.
- **Inverse contribution model** — `published_repos = "open"` for projects where the split repo, not the monorepo, is the canonical contribution target.

---

## 10. Session log

- **2026-05-20.** Session restored from `CONTINUE.md` + `spec/WAL.md`. First closed the `vibe registry redirect-update` tech-debt item (M1.16 deferred-list — 4 commits, `f8af587..b44729d`). Then the owner opened the question of structuring a project with packages without necessarily publishing to a repository. The discussion grew into the workspace + qualified-naming refactor: the four-axis decomposition, the fork-by-fork resolution recorded in §5, two DRAFT PROPs (PROP-007 + PROP-008, commit `ff23a0f`), and finally the decision to create the `spec/design/` genre so this lore is not lost when the implementation moves to a fresh session. Implementation deferred to a new session; this document plus PROP-007/008 are the handoff.
- **2026-05-21.** PROP-007 implemented — M1.17 Phases 1–5 (the workspace data model, discovery, path-source + `vibe.lock` v4, `[workspace.versions]`, `vibe workspace publish`) shipped on branch `m1.17-workspace`; Phase 6 documented it. The detailed implementation record is [PROP-007 §9](../modules/vibe-workspace/PROP-007-workspace.md#impl). Workspace-aware `vibe install` remains, gated on the materialisation-target decision (PROP-007 §6 q3). PROP-008 (qualified naming) is still unimplemented — its turn comes after PROP-005 (index).

---

## 11. Pointers

- [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) — workspace (the contract).
- [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) — qualified naming (the contract).
- [PROP-002](../modules/vibe-registry/PROP-002-decentralized-registry.md) — identity, registry, the per-registry-identity rejection (§3.4).
- [PROP-005](../modules/vibe-index/PROP-005-package-index.md) — the index; a prerequisite for short-name resolution.
- [`ROADMAP.md`](../../ROADMAP.md) — milestones M1.17 (workspace), M1.18 (qualified naming), and the M3+ registry-explorer entry.
