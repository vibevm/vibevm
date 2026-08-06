# Design rationale: Workspace & qualified naming

<status stage="doc" state="done" comment="B0 2026-07-24: non-normative design record, captured 2026-05-20 in an owner session"/>

@fact:companion-line **Companion to:** [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) (workspace), [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) (qualified naming). @status:doc/done

@fact:status-line **Status:** non-normative design record. Captured 2026-05-20 in an owner design session. @status:doc/done

@fact:authority-line **Authority:** the PROPs are the contract. If this document and a PROP disagree, the PROP wins. @status:doc/done

---

## 1. What this document is

@fact:what-this-is PROP-007 and PROP-008 say *what* the workspace + qualified-naming refactor does. This document says *why* — and keeps the lore: the owner's mental model, the four-axis decomposition, every fork weighed during the session, the Cargo-vs-Maven precedents studied, the publication model that needed careful explaining, and the ideas that surfaced but were parked. @status:doc/done

@fact:why-exists It exists because the design session that produced PROP-007/008 spent a large amount of reasoning that does not belong inside a contract-shaped PROP, yet would be expensive to lose at the next session boundary. See [`spec/design/README.md`](README.md) for the genre. @status:doc/done

---

## 2. The owner's request and mental model

@fact:owner-request The owner asked for a structure "most like Maven submodules and cargo": a project that decomposes *naturally* into modules; modules that publish to a repository **individually** — or are deliberately **not** shared; and the whole structure declared right in the project description. @status:doc/done

@fact:extremes-lead Two extremes were named as must-work cases, and they anchor the whole design: @status:doc/done

- @fact:extreme-invisible **Entirely invisible** — a fully structured project that lives nowhere in any external repository. @status:doc/done
- @fact:extreme-published **Entirely published** — the whole project, every sub-package individually, published. @status:doc/done

@fact:in-between-first-class The "everything in between" (some modules public, some private, some workspace-internal) must also be first-class. @status:doc/done

@fact:INVISIBLE-SUBPROJECT-PHRASE A load-bearing phrase from the session, recorded verbatim because it shaped several decisions: *"the user works in a sub-project and doesn't even notice that it is actually a small piece of something bigger."* This is why commands bubble up to the workspace root (PROP-007 §2.4) and why there is one unified manifest rather than many files (§2.2) — "reading a million different files is hard for a human, and for a small dumb LLM agent too." @status:doc/done

@fact:flexibility-spirit The spirit throughout: **flexibility and convention-over-configuration**. The owner repeatedly chose "support both, default sensibly" over "pick one." That preference resolved R3 (version inheritance: both modes) and W8 (member versioning: both). @status:doc/done

---

## 3. The four axes

@fact:four-axes-lead The request decomposes into four orthogonal axes. Keeping them separate was the single most clarifying move of the session — they have different cost, different dependencies, and ship as different milestones. @status:doc/done

| Axis | Essence | Analogue |
|---|---|---|
| @fact:ROW-AXIS-A **A. Workspace** @status:doc/done | Project = a set of modules; structure declared in `vibe.toml` @status:doc/done | cargo `[workspace]`, Maven `<modules>` @status:doc/done |
| @fact:ROW-AXIS-B **B. Selective publish** @status:doc/done | Each module publishes / does not, by choice @status:doc/done | cargo `publish = false`, Maven `deploy.skip` @status:doc/done |
| @fact:ROW-AXIS-C **C. Qualified naming** @status:doc/done | Reverse-FQDN at the top (`org.vibevm_wal`), short aliases (`wal`) kept @status:doc/done | Maven `groupId:artifactId` @status:doc/done |
| @fact:ROW-AXIS-D **D. Conflict-aware resolve** @status:doc/done | Collision → show alternatives; conflict → fail without applying; full-auto → fail @status:doc/done | npm / Maven @status:doc/done |

@fact:axis-a-eighty-percent Axis A alone closes roughly 80% of the request — multi-package projects, local cross-deps, selective publish, both extremes — and it depends on nothing. That was the good news delivered early: the heavy part (naming, discovery) is separable and can come later. @status:doc/done

---

## 4. The dependency graph between axes

```
Axis A (workspace)          — independent ───────────────► do first
Axis B (selective publish)  — depends on A
Axis C (qualified naming)   — depends on PROP-005 (index)
Axis D (conflict-aware)     — depends on C; the conflict half partly exists already
```

@fact:chain-key-finding The chain `D → C → PROP-005 index` is the key finding. Reasoning: @status:doc/done

- @fact:c-needs-index **C needs the index.** A short name `wal` must be resolved to a qualified `org.vibevm.world/wal`. That requires enumerating candidates `(*, wal)` across registries. The host cannot list an org cheaply (PROP-005 §1: GitVerse exposes no org listing; GitHub is rate-limited). Therefore short-name resolution requires PROP-005 implemented. @status:doc/done
- @fact:d-needs-c **D needs C.** A *collision* is two different packages sharing a short name. You can only tell a collision apart from a harmless mirror (the same package served by two registries) once `group` exists to distinguish them. Without `group`, first-match-wins is the only sane policy. @status:doc/done
- @fact:conflict-half-exists **The conflict half of D already exists.** The depsolver (resolvo/libsolv per PROP-003), `[conflicts]`, and the conflict-explanation chain are in place; the install pipeline is already atomic (resolve → plan → confirm → apply), so a failed resolve never reaches apply. "Fail without applying the plan" is already true. Only *collision* detection is new. @status:doc/done

@fact:recommended-sequencing This is why the recommended sequencing is **A → PROP-005 implementation → C → D**. @status:doc/done

---

## 5. The fork-by-fork decision record

@fact:forks-lead Every fork weighed in the session, the options, the choice, and the reasoning. This is the most valuable part to preserve — settled questions that would otherwise be re-litigated. @status:doc/done

### Naming forks

- @fact:fork-separator **Separator `group`↔`name`.** Chosen: `/` → `flow:org.vibevm.world/wal`. `:` is taken by `kind`, `@` by version; npm-scope (`@org/`) was rejected because `@` doubling with version would confuse. @status:doc/done
- @fact:fork-group-mandatory **Is `group` mandatory?** Chosen: **mandatory**. Maven makes groupId mandatory; an optional `group` creates a grey zone ("no group" vs "has group"). The three legacy packages migrate silently — the owner waved that through ("they are test packages anyway"). @status:doc/done
- @fact:fork-no-fqdn-enforcement **Enforce reverse-FQDN?** Chosen: **core does not enforce**. Whether `group` looks like a reversed domain is style — for humans and linters, not the resolver. Maven likewise does not enforce groupId shape. @status:doc/done
- @fact:fork-canonical-group **Canonical group for vibevm.** `org.vibevm` (domain `vibevm.org`). Recorded in PROP-008 §2.1. @status:doc/done
- @fact:fork-kind-out-of-repo-name **`kind` in the repository name?** The owner asked: can `kind` leave the repo name entirely? Yes — because identity is already URL-orthogonal (PROP-002 §2.1), the repo name identifies nothing. The one thing `kind` gave the name was disambiguation (`flow-wal` vs `feat-wal`); making `name` unique *within a group* (rather than within a kind) removes that need. Result: repo = `<group>.<name>` = `org.vibevm_wal`, `naming = "fqdn"`. `kind` becomes pure metadata and leaves the identity tuple. @status:doc/done
- @fact:fork-kind-prefix-optional **`kind` prefix in pkgref — keep or drop?** Chosen: **optional but allowed, validated when present**. The owner's exact framing: if `flow:` is purely a UX feature, make it optional but possible; and if an install used the prefix explicitly, validate it matches the manifest. So `org.vibevm.world/wal` and `flow:org.vibevm.world/wal` are both legal; a present prefix is checked (`KindMismatch` on mismatch). It is validation + a UX signal — it never disambiguates, because `name` is unique within `group`. @status:doc/done
- @fact:fork-qualified-in-manifests **Short name in manifests?** Chosen: **no — manifests store the qualified form**. The short name is CLI-only sugar; `vibe install wal` resolves once and writes `org.vibevm.world/wal`. This is the cargo/npm pattern (`cargo add serde` → `serde = "1"`). The decisive consequence: the dependency graph is built entirely from qualified names, so **transitive collisions vanish by construction** — short-name resolution only ever happens at the human-typed CLI boundary, not recursively through the graph. @status:doc/done
- @fact:fork-exit-code-7 **Exit code for ambiguity.** Chosen: **new code `7`**, distinct from `3` (package conflict). @status:doc/done

### Workspace forks (7a–7e)

- @fact:fork-7a-universal-node **7a — member is a "package" or a "project"?** Chosen: **universal node** — it can be both; one structure serves all roles. @status:doc/done
- @fact:fork-7b-member-consumer **7b — can a member be a consumer itself?** Chosen: **yes** — "and usually it should be: the user works in a sub-project without noticing it is a piece of something bigger." This drove command-bubbling. @status:doc/done
- @fact:fork-7c-one-lockfile **7c — one lockfile per workspace or per member?** The owner asked back: how do Cargo and Maven do it? Answer studied (see §6). Chosen: **one `vibe.lock` at the absolute root** (cargo model). Commands inside a member bubble up to it — which *is* the mechanism for 7b's "doesn't notice." @status:doc/done
- @fact:fork-7d-cascade-override **7d — `[[registry]]` / `[active]` / `[llm]` shared or per-member?** Chosen: **cascade with override** — the root sets defaults, a member may override. This is safe for `[[registry]]` only because identity is URL-orthogonal: a member overrides *where to fetch from*, not *what* identity it gets; if two registries served a genuinely different `content_hash` under one name, that is a collision and axis D catches it. @status:doc/done
- @fact:fork-7e-one-manifest **7e — one manifest file or two?** Chosen: **one `vibe.toml` for everything**; `vibe-package.toml` is retired. Reason: a member is simultaneously a developed consumer and a publishable artifact — two files would duplicate `[requires]` and drift. Plus the owner's "read one file, not a million" point. The escape hatch noted: "if it becomes impossible without splitting — we'll split later." @status:doc/done

### Forks resolved by the owner accepting recommendations

- @fact:fork-r1-project-package **R1 — `[project]` vs `[package]`.** Chosen: 7-α — keep both sections distinct (a node is a non-publishable project *or* a publishable package), rather than folding `[project]` into a `[package]` with optional `kind`. Explicitness; `kind` stays strictly mandatory wherever `[package]` appears. @status:doc/done
- @fact:fork-r2-qualified-confirmed **R2 — manifests store the qualified form** (see naming forks above) — confirmed. @status:doc/done
- @fact:fork-r3-versions-recursion **R3 — version placeholders.** Chosen: named `[workspace.versions]` (Maven `<properties>` shape). And: **depth 2 is not enough — recursion to arbitrary nesting depth**. This *reversed* a default the assistant had set (W3: "nested workspaces forbidden"). The reversal is load-bearing — it is what makes the matryoshka resolution arbitrary-depth — and it raised the cost of PROP-007 (recursion in root discovery, member aggregation, placeholder resolution). @status:doc/done
- @fact:fork-r4-org-vibevm **R4 — `org.vibevm`** (recorded above). @status:doc/done
- @fact:fork-r5-kind-out **R5 — kind out of repo name + optional kind prefix** (recorded in naming forks). @status:doc/done
- @fact:fork-r6-owner-sanction **R6 — owner sanction** to edit any specification, including the owner-frozen `VIBEVM-SPEC.md`. Granted 2026-05-20 after requirements were judged sufficiently complete. @status:doc/done
- @fact:fork-dual-form-path-dep **dual-form path-dep** — `{ path, version }` both present: `path` for local development, `version` for when the consuming node is itself published. Required because the owner explicitly wants the mixed mode (some modules local, some published). @status:doc/done

### Defaults the assistant set; the owner accepted silently (except W3)

@fact:w-defaults W1 root discovery by walking up to `[workspace]`; W2 glob in `members`; **W3 nested workspaces — reversed by R3 into recursion**; W4 dependency cycles between members are an error; W5 path outside the workspace allowed but path-only consumers are non-publishable; W6 `vibe install` addressing (root vs `-p` vs cwd); W7 `vibe workspace publish` is non-atomic, stop-on-first-failure; W8 member versioning supports both independent and inherited; collision in an interactive run shows alternatives and fails (no interactive pick — the choice must be recorded deliberately); the lockfile is authoritative when resolving a short name; `group` grammar follows Maven groupId; lockfile schema bumps to v4. @status:doc/done

---

## 6. Cargo vs Maven — the precedent lore

@fact:precedent-lead Both were studied point by point. The findings, kept here so a future session need not re-derive them. @status:doc/done

| Aspect | Cargo | Maven |
|---|---|---|
| @fact:ROW-PREC-LOCKFILE Lockfile @status:doc/done | One `Cargo.lock` at the workspace root; no per-crate locks @status:doc/done | **No lockfile at all** — a known reproducibility gap; the parent POM's `<dependencyManagement>` plays the "single source of versions" role @status:doc/done |
| @fact:ROW-PREC-RESOLUTION Resolution @status:doc/done | Unified across the workspace — one version of each dep @status:doc/done | "Nearest-wins" per build; reproducibility is discipline + fixed versions @status:doc/done |
| @fact:ROW-PREC-MEMBERS Members @status:doc/done | Each is a full crate with its own `Cargo.toml` @status:doc/done | Each is a full module with its own `pom.xml` + `<parent>` @status:doc/done |
| @fact:ROW-PREC-VERSION-INHERITANCE Version inheritance @status:doc/done | `version.workspace = true` from `[workspace.package]` @status:doc/done | Modules often share the parent's `${project.version}` @status:doc/done |
| @fact:ROW-PREC-CENTRAL-VERSIONS Centralised versions @status:doc/done | `[workspace.dependencies]` + `{ workspace = true }` @status:doc/done | `<dependencyManagement>` (per-artifact) + `<properties>` (named placeholders) @status:doc/done |
| @fact:ROW-PREC-NESTED Nested workspaces @status:doc/done | **Forbidden** — to avoid "which workspace is mine" ambiguity @status:doc/done | Parent POMs nest to arbitrary depth @status:doc/done |
| @fact:ROW-PREC-PUBLISH Publish @status:doc/done | `cargo publish -p <crate>` — per-crate, to crates.io @status:doc/done | `mvn deploy` — per-module; `<skip>` opts a module out @status:doc/done |
| @fact:ROW-PREC-NAMING Top-level naming @status:doc/done | Flat crate names on crates.io @status:doc/done | `groupId:artifactId` — reverse-FQDN groupId @status:doc/done |

@fact:drawn-lead The decisions vibevm drew from this: @status:doc/done

- @fact:DRAWN-LOCKFILE-CARGO **Lockfile: Cargo model.** vibevm already mandates a lockfile (content-hash integrity, PROP-002 §2.1). Maven's lockless model would mean discarding working machinery. One `vibe.lock` at the absolute root. @status:doc/done
- @fact:DRAWN-NESTED-PERMITTED **Nested workspaces: vibevm permits them, unlike Cargo.** Cargo forbids them to dodge ambiguity. vibevm can afford them because it fixes "lock always at the absolute root" — that rule resolves the ambiguity deterministically. Maven shows arbitrary nesting works when version coordination aggregates upward. @status:doc/done
- @fact:DRAWN-MAVEN-PROPERTIES **Version placeholders: Maven `<properties>` shape**, because that is literally what the owner described ("write `0.0.1` once, reference it by name"). `[workspace.dependencies]` (Cargo's centralised per-pkgref defaults) is noted as a possible companion mechanism, not the primary one. @status:doc/done
- @fact:DRAWN-GROUPID **groupId: adopted as `group`** — but as a package attribute, not a registry attribute, which is why it does not reopen PROP-002 §3.4's rejection of per-registry identity. @status:doc/done

---

## 7. The physical publication model

@fact:publication-model-lead This needed careful explanation — the owner flagged it as confusing and asked directly. @status:doc/done

@fact:PUBLICATION-MODEL The model: **the development tree is one source tree** (one git repo, or not in git at all if private). Workspace members are subdirectories; the split into packages is logical, at the resolver level. **Publishing is a separate operation that copies a package directory's content into a new, separate repository** in the registry org and tags it — exactly what `vibe registry publish` does today for one package, repeated per member by `vibe workspace publish`. @status:doc/done

@fact:NO-FILE-MOVING A nested package does **not** "surface" by moving files. Only a *copy of its content* is published, into its own repository, at publish time. The source tree stays unified — a monorepo for development; the registry holds split copies. @status:doc/done

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

@fact:flat-siblings-recommendation Recommendation: keep members as flat siblings, not physically nested. Logical hierarchy ("X is built from Y") is a path-dependency, not a nested directory. Physical nesting is supported (with subtree excision when publishing the outer package, cargo-style) but discouraged for the "holes in the tree" complexity. @status:doc/done

---

## 8. "Do not contribute here" — the signalling lore

@fact:signalling-question The owner asked: when a sub-package's published copy is cloned, how do we make it super-obvious that pull requests there are pointless and contribution belongs to the main project? @status:doc/done

@fact:prior-art-mirrors Prior art studied: AOSP / Chromium / Bazel keep read-only mirrors on GitHub with explicit "this is a mirror, do not send PRs, contribute upstream" banners. GitHub offers no "disable PRs only" switch — only full archival blocks PRs. @status:doc/done

@fact:layered-answer The layered answer (detail in PROP-007 §2.8): README banner as the first block; repo `description`; Issues disabled; a `PULL_REQUEST_TEMPLATE.md` STOP notice; and `archived = true` as the nuclear `--archive` option (full read-only — vibevm drives the unarchive→push→archive cycle on re-publish). Plus a machine-readable `[origin]` marker in the published copy, and a `published_repos = "read-only" | "open"` setting for operators who want the inverse model. @status:doc/done

@fact:terminology-published-repo Terminology fixed: the published copy is a **published package repository**; the source of truth is the **workspace**. It is *not* a `[[mirror]]` (that term is taken — an availability copy of a registry). @status:doc/done

---

## 9. Ideas parked for the future

- @fact:parked-registry-explorer **vibevm registry explorer.** The owner asked whether a Maven-Central-style browsable visualisation is possible — and wanted it richer. Yes: the per-org index (PROP-005) carries the data. Beyond Maven Central: a reverse-FQDN group tree with drill-down, plus `kind` filtering, a capability graph, `describes`/PURL links to upstream libraries, redirect-stub delegation, the full dependency DAG, and workspace provenance ("sub-package of X" from the `[origin]` marker). Recorded as a `ROADMAP.md` M3+ entry and in PROP-008 §2.9. The only obligation on the refactor: the index must carry `group` and `workspace_origin` so the explorer is not a retrofit. @status:doc/done
- @fact:parked-workspace-dependencies **`[workspace.dependencies]`** — Cargo-style centralised per-pkgref version defaults, alongside the named placeholders. Deferred until a concrete need surfaces. @status:doc/done
- @fact:parked-inverse-model **Inverse contribution model** — `published_repos = "open"` for projects where the split repo, not the monorepo, is the canonical contribution target. @status:doc/done

---

## 10. Session log

- @fact:session-log-2026-05-20 **2026-05-20.** Session restored from `CONTINUE.md` + `spec/WAL.md`. First closed the `vibe registry redirect-update` tech-debt item (M1.16 deferred-list — 4 commits, `f8af587..b44729d`). Then the owner opened the question of structuring a project with packages without necessarily publishing to a repository. The discussion grew into the workspace + qualified-naming refactor: the four-axis decomposition, the fork-by-fork resolution recorded in §5, two DRAFT PROPs (PROP-007 + PROP-008, commit `ff23a0f`), and finally the decision to create the `spec/design/` genre so this lore is not lost when the implementation moves to a fresh session. Implementation deferred to a new session; this document plus PROP-007/008 are the handoff. @status:doc/done
- @fact:session-log-2026-05-21 **2026-05-21.** PROP-007 implemented — M1.17 Phases 1–5 (the workspace data model, discovery, path-source + `vibe.lock` v4, `[workspace.versions]`, `vibe workspace publish`) shipped on branch `m1.17-workspace`; Phase 6 documented it. The detailed implementation record is [PROP-007 §9](../modules/vibe-workspace/PROP-007-workspace.md#impl). Workspace-aware `vibe install` remains, gated on the materialisation-target decision (PROP-007 §6 q3). PROP-008 (qualified naming) is still unimplemented — its turn comes after PROP-005 (index). @status:doc/done

---

## 11. Pointers

- @fact:ptr-prop-007 [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) — workspace (the contract). @status:doc/done
- @fact:ptr-prop-008 [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) — qualified naming (the contract). @status:doc/done
- @fact:ptr-prop-002 [PROP-002](../modules/vibe-registry/PROP-002-decentralized-registry.md) — identity, registry, the per-registry-identity rejection (§3.4). @status:doc/done
- @fact:ptr-prop-005 [PROP-005](../modules/vibe-index/PROP-005-package-index.md) — the index; a prerequisite for short-name resolution. @status:doc/done
- @fact:ptr-roadmap [`ROADMAP.md`](../../ROADMAP.md) — milestones M1.17 (workspace), M1.19 (qualified naming — the number shifted after this session was captured; M1.18 went to the loading model), and the M3+ registry-explorer entry. @status:doc/done
