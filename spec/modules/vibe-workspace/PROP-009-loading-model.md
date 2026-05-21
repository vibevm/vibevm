# PROP-009: Loading model — computed boot composition and the effective spec {#root}

**Milestone:** design proposal; targets a new `M1.18` ([`ROADMAP.md`](../../../ROADMAP.md)). Not implementation-locked.
**Status:** DRAFT — requirements and the §5 open questions resolved 2026-05-21 in owner design sessions; ready for M1.18 implementation.
**Related:** [`VIBEVM-SPEC.md` §4.2 / §4.6 / §6 / §13.1](../../../VIBEVM-SPEC.md); [PROP-007](PROP-007-workspace.md) (workspace — PROP-009 answers its [§6 question 3](PROP-007-workspace.md#open)); [PROP-003 §2.5](../vibe-resolver/PROP-003-dep-evolution.md) (subskills, delivery modes, the `[activation]` vocabulary); [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md) (identity, registry).
**Design rationale:** [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md) — the *why*, the static/dynamic-linking metaphor, the fork-by-fork record. Non-normative; this PROP is the contract.
**Owner sanction:** PROP-009 reshapes the owner-frozen `VIBEVM-SPEC.md` (§6 boot model, §4.2 layout, §4.6 effective spec, §13.1 package layout). This PROP is the requirements record; the `VIBEVM-SPEC.md` edits land at implementation time and **require explicit owner sanction** — not yet granted (M1.17's sanction covered the workspace + qualified-naming refactor only). See §5 item 8.

---

## 1. Motivation {#motivation}

PROP-007 shipped the workspace data model but left [§6 question 3](PROP-007-workspace.md#open) open: when a dependency is resolved for member M, into which member's `spec/` does its content land?

The question is not a directory choice. vibevm's boot model (`VIBEVM-SPEC.md` §6) — a flat `spec/boot/NN-*.md` directory, one sequence, one entry point — holds for exactly one project shape: a single project with a single entry point. A workspace has N nodes, N entry points (a developer opens an agent inside any member — PROP-007's "the user works in a sub-project and doesn't notice it is part of something bigger"), N boot sequences, and one shared dependency set under unified resolution. The flat model cannot be stretched over this.

PROP-009 replaces the loading model. The owner's hard constraint: **installing a dependency must never modify any node's authored spec** — the C++ rule that you do not paste a header's text into your `#include`. The owner's frame for the replacement is static vs dynamic linking. The linker metaphor and the fork-by-fork record are in the [design document](../../design/loading-and-boot-model.md).

---

## 2. Decisions {#decisions}

### 2.1 Two trees — authored spec and materialised dependencies {#two-trees}

**Decision.** A node's authored `spec/` and its materialised dependencies live in physically separate trees. `vibe install` **never writes into any node's authored `spec/`**.

- Authored `spec/` — written only by the node's author. Unchanged definition.
- Materialised dependencies — a `vibedeps/` tree at the **absolute workspace root** (PROP-007 §2.3), written only by `vibe`. One slot per resolved package, `vibedeps/<kind>-<name>/<version>/`, holding the package's published tree verbatim. Unified resolution (PROP-007 §2.4) guarantees one version per package, so one slot serves the whole workspace.
- `vibedeps/` is **committed** to the repository. A fresh clone is immediately bootable with no `vibe install`; the dependency corpus is visible and diffable; this matches the spec-driven principle that the committed spec corpus is the product.

**Consequence — the mirror layout is retired.** `VIBEVM-SPEC.md` §13.1's mirror layout (a package's `[writes]` entry is both source and target path) worked only because a dependency landed at one fixed path in every project. A materialised package is now its own verbatim subtree under `vibedeps/<slot>/`; a package's internal cross-references must become package-relative or `spec://` URIs. `[writes]` is retired (§2.6): a materialised package *is* its own subtree, and a per-file write list has nothing left to declare.

### 2.2 The effective boot sequence {#effective-boot}

**Decision.** Every node has an **effective boot sequence**, computed by `vibe` from the unified resolution:

> inherited foundation (from ancestors) + the node's own authored boot + the boot of the node's transitive dependencies + user overrides

- **Inherited foundation** flows down: a member inherits the project-wide foundation boot of its ancestors up to the absolute root (conventions, the four rules, technology choices).
- **Dependency boot** flows up: a node's sequence includes the boot of everything it transitively requires.
- A node that is itself a workspace aggregates its members' sequences — the root's effective boot is the union of the whole tree; a leaf member's is its own subtree only. The hierarchy scopes cost: a session opened in a small member boots small.
- The sequence is **computed per node directly from the resolution graph**, never copied physically between levels (copying drifts; computation does not).

### 2.3 Generated boot artifacts {#artifacts}

**Decision.** For every entry-point node, `vibe install` generates two artifacts under the node's `spec/boot/`:

- **`INLINE.md`** — the verbatim concatenation, in priority order, of every `inline`-typed (§2.4) contribution in the node's effective boot. Read first. Generated only when the node has `inline` contributions.
- **`INDEX.md`** — a generated **TOML manifest** of the rest of the sequence: a `schema` version, an `inline` pointer (the path of `INLINE.md`, when one exists), and an ordered list of `[[entry]]` tables. Each entry carries `path`, `kind` (`"static"` — a resolved file the agent reads directly; `"dynamic"` — an INCLUDE the agent resolves at boot, §2.4), and, for dynamic entries, `when` (the activation condition, §2.4). The manifest is flat and machine-precise — `vibe` performed the graph walk once at generation time; the agent parses one TOML document and reads the listed files, with no recursion, discovery, or cycle-detection.

```toml
# spec/boot/INDEX.md — generated by vibe, do not edit.
schema = 1
inline = "spec/boot/INLINE.md"

[[entry]]
path = "spec/boot/00-core.md"
kind = "static"

[[entry]]
path = "vibedeps/stack-rust/2.1.0/boot/rust.md"
kind = "dynamic"
when = "rust"
```

Both artifacts are generated, git-tracked, and marked "generated — do not edit". Authored boot files (the user-owned snippets, the node's own authored boot) continue to live alongside as ordinary files; `INDEX.md` references them in computed order.

**Session-start order:** the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect → `spec/boot/INLINE.md` (if present) → `spec/boot/INDEX.md` and the entries it names, in order. Boot remains **pure file-reading** — the redirect never becomes "run `vibe`", preserving the zero-dependency cross-agent property of `VIBEVM-SPEC.md` §6.1.

### 2.4 Inclusion types — `inline`, `static`, `dynamic` {#inclusion-types}

**Decision.** Each dependency declares an **inclusion type**, set by the consumer in its `vibe.toml` on the `[requires.packages]` entry:

```toml
[requires.packages]
"flow:wal"        = { version = "^0.3", link = "static" }   # default
"flow:discipline" = { version = "^1.0", link = "inline" }   # emergency priority lane
"stack:rust"      = { version = "^2.0", link = "dynamic" }  # conditional / context-gated
```

- `link = "static"` — **default.** `vibe` resolves the contribution to a concrete path in `INDEX.md`. The agent reads it directly; reads parallelise across one turn.
- `link = "inline"` — the contribution's boot text is concatenated verbatim into `INLINE.md`. Read first, one read, maximum attention weight. The **emergency priority lane** — for top-level skills and critical disciplines whose priority must be guaranteed by position, not by trusting agent-side resolution. Used sparingly; it duplicates the text on disk.
- `link = "dynamic"` — `INDEX.md` carries an INCLUDE pointer; the agent resolves it at boot. Supports **conditional boot** (load only when a context probe fires) — mechanically the subskill `lazy-pull` delivery mode. The `when` condition reuses the subskill `[activation]` probe vocabulary verbatim (PROP-003 §2.5) — one probe grammar across both mechanisms, no parallel one.

A package MAY declare a suggested default inclusion type in its own `[boot_snippet]`; the consumer's declaration always wins. Absent both, the type is `static`.

### 2.5 Ordering by category — the `NN-` prefix is retired {#ordering}

**Decision.** `vibe` owns the order of entries in the generated artifacts. The author-chosen two-digit `NN-` prefix (`VIBEVM-SPEC.md` §6.2) is **retired** — it cannot survive a workspace's combined namespace, and §6.5 already admits it provisional.

- A package declares a **category** for its boot snippet, not a number. The categories preserve the intent of the old range bands: `foundation`, `flow`, `stack`, `user-override`.
- Within the computed sequence the order is: `foundation` → the node's own → dependency boot (topologically — a dependency before its dependents) → `user-override`. `inline` contributions are concatenated into `INLINE.md` in the same relative order.
- Prefix collisions — the failure mode of `VIBEVM-SPEC.md` §6.3 — become impossible by construction; `BootSnippetConflict` / `BootSnippetNumericConflict` (`vibe-install`) are removed.
- The user-owned files keep their reserved names (`00-core.md`, `90-user.md`) by convention; `vibe` places them at the foundation / override ends.

### 2.6 Manifest schema changes {#schema}

**Decision.**

- `[requires.packages]` inline-table entries accept an optional `link` field (§2.4): `"inline" | "static" | "dynamic"`, default `static`. Valid on registry-, path-, and git-source dependencies.
- `[boot_snippet]` (package-role) drops the `filename` field (the `NN-` target name) and gains `category` (§2.5); `source` — the path to the boot file inside the package — is retained. It may carry an optional suggested `link` default.
- `[writes]` (package-role) is **removed** (§2.1, §2.7) — a package's materialised footprint is its verbatim tree under its `vibedeps/` slot; a per-file write list has nothing left to declare.
- A minimal project-level `[boot]` table carries workspace-wide loading settings — for v1, only a default `link` override. Room to grow; nothing more is added now.
- A `vibe.lock` schema bump may be required to record materialisation slots and inclusion types — assessed in Phase 1.

### 2.7 Workspace-aware `vibe install` / `vibe build` {#install}

**Decision.** `vibe install` and `vibe build` discover the workspace and operate on it as a whole — the piece PROP-007 §6 q3 deferred, now subsumed.

- Run anywhere inside a workspace, `vibe install` calls `Workspace::discover`, runs **one unified resolution** across every member's `[requires]`, materialises each resolved package once into `vibedeps/` (§2.1), and regenerates the boot artifacts (§2.3) for every entry-point node. One `vibe.lock` at the absolute root (PROP-007 §2.4).
- The plan / confirm / apply contract holds, but the plan's unit is **the set of packages to materialise plus the boot artifacts to regenerate**, not a per-file write list — `[writes]` is retired (§2.6).
- `-p <member>` scopes resolution *reporting* to one member; the materialisation and the single root lockfile are always workspace-wide — unified resolution admits no per-member subset.
- A standalone single-package project is a degenerate workspace and follows the identical path (§2.9).

### 2.8 The computed-view engine — boot and the effective spec {#engine}

**Decision.** The boot artifacts (§2.3) and the **effective spec** (`VIBEVM-SPEC.md` §4.6 — the merged corpus consumed by `vibe build` and `vibe show effective`) are two projections of one **computed-view engine**: workspace walk (`Workspace::discover`) + unified resolution + two-tree layering (§2.1, §2.2).

- The **boot view** projects the boot-category content into the ordered `INLINE.md` / `INDEX.md` (§2.3).
- The **effective-spec view** projects the full layered corpus — authored `spec/` plus materialised `vibedeps/` — into the effective spec.
- Both are deterministic and regenerated by `vibe install`.

The effective-spec view's detailed shape is **v1.5 scope** (it feeds `vibe build`). PROP-009 fixes only that it shares the engine, so it is not built as a later retrofit.

### 2.9 Uniform model — every project is a workspace {#uniform}

**Decision.** The loading model is uniform: a single-package project is a degenerate (zero-member) workspace. `Workspace::discover` already degenerates cleanly (PROP-007 §2.3). There is one loading model, one set of artifacts, one code path.

Every existing project migrates (§4). vibevm is pre-release; M1.17's no-legacy hard break is the precedent. The vibevm repository, itself a vibevm project, migrates too — `spec/boot/00-core.md` and `90-user.md` stay user-owned authored boot; the generated `INLINE.md` / `INDEX.md` join them.

### 2.10 Regeneration — `vibe reinstall` {#regen}

**Decision.** `vibe reinstall [<path>] [--force]` reinstalls and regenerates the materialised state.

- It targets any node in the workspace. Reinstalling a node regenerates that node **and every ancestor up to the absolute root** — the matryoshka (§2.2) means an ancestor's aggregated artifacts depend on the node's. `vibe reinstall` run at the root regenerates the whole tree.
- Without `--force` it recomputes the materialisation and the boot artifacts from the existing `vibe.lock` and the local cache — no fresh resolution.
- `--force` re-fetches the file content of the whole targeted subtree from the source repositories, overwriting the current `vibedeps/` files and bypassing the cache. The escape hatch for a corrupted, hand-edited, or wrongly-generated subtree.

It exists for when the materialised state is believed stale or a previous generation pass was wrong.

### 2.11 Published-copy regeneration {#publish}

**Decision.** `vibe workspace publish` (PROP-007 §2.7) regenerates the boot artifacts of each staged copy for the **published shape** — where dependencies are registry-resolved and version-pinned, not path-sourced. This consumes PROP-007 §2.5's dual-form `{ path, version }`: the local `vibedeps/` slots and path entries become registry references in the published copy's artifacts. Publishing the development tree's own path-resolved artifacts would dangle for an external consumer.

---

## 3. Command and crate surface {#surface}

- `vibe install` / `vibe build` — workspace-aware (§2.7).
- `vibe reinstall` — regeneration (§2.10).
- `vibe workspace publish` — gains published-shape artifact regeneration (§2.11).
- `vibe show effective` — projects the effective-spec view (§2.8).
- The computed-view engine lands either as a new crate (`vibe-boot` / `vibe-view`) or inside `vibe-workspace` (which already owns discovery and the `[workspace.versions]` finalize pass) — decided at implementation time.

---

## 4. Migration {#migration}

Every existing project migrates once (§2.9). On the first `vibe install` after the upgrade, `vibe` rewrites the project: dependency content moves out of the authored `spec/` into `vibedeps/`; `NN-` boot files become categorised authored boot or generated artifacts; `INLINE.md` / `INDEX.md` are generated; the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect is rewritten. There is no compatibility shim — a pre-PROP-009 layout is migrated, not supported in place. The vibevm repository is migrated as part of the milestone.

---

## 5. Resolved questions {#open}

The eight questions opened in draft 1 were resolved in an owner session on 2026-05-21.

1. **`vibedeps/` directory** — the materialised-dependency tree (§2.1); slot layout `vibedeps/<kind>-<name>/<version>/`.
2. **`vibe reinstall`** — the regeneration command (§2.10), replacing the working name `vibe boot`; it regenerates a node and every ancestor to the root, and `--force` re-fetches a subtree from source.
3. **`INDEX.md` is a TOML manifest** — `schema` / `inline` / `[[entry]]` (§2.3); machine-precise over an LLM-native list.
4. **`[writes]` is retired** (§2.6) — a package's footprint is its verbatim tree under its `vibedeps/` slot.
5. **Dynamic conditions reuse the subskill `[activation]` vocabulary** verbatim (§2.4; PROP-003 §2.5) — one probe grammar, no parallel one.
6. **A minimal `[boot]` table** (§2.6) — for v1 it carries only a workspace-wide default `link`.
7. **The effective-spec view stays v1.5 scope** (§2.8) — PROP-009 fixes only that it shares the computed-view engine.

**Deferred:**

8. `VIBEVM-SPEC.md` edits (§6, §4.2, §4.6, §13.1) require explicit owner sanction. Phases 1–6 do not need it; the decision is taken when Phase 7 (docs / spec) is reached.

---

## 6. Rejected / deferred alternatives {#rejected}

- **Bubble every dependency's boot into the root `spec/boot/`.** Rejected — it is the "merge dependency specs into the authored spec" the owner ruled out, and it makes one flat namespace for the whole workspace.
- **Boot by running `vibe` at session start.** Rejected — it would always be fresh, but it breaks the zero-dependency cross-agent property (`VIBEVM-SPEC.md` §6.1) and adds a process exec to every session. Boot stays pure file-reading (§2.3).
- **Copy boot snippets physically leaf-to-root (the literal matryoshka).** Rejected in favour of computing each level directly from the resolution graph (§2.2) — physical copying drifts between levels.
- **A gitignored dependency cache.** Rejected — a committed `vibedeps/` keeps a fresh clone bootable and the corpus reviewable.

---

## 7. Phase plan {#phases}

Targets M1.18. PROP-008 (qualified naming) shifts to M1.19; `ROADMAP.md` updates in the docs phase.

1. **Schema** — the `link` field, `[boot_snippet]` `category`, retire the `NN-` filename and the `[writes]` section; `vibe.lock` bump if needed. `vibe-core`.
2. **Materialisation tree** — the `vibedeps/` layout, materialise packages verbatim; retire the mirror layout.
3. **Computed-view engine** — per-node effective boot computation from the unified resolution.
4. **Artifact generation** — `INLINE.md` / the TOML `INDEX.md`; the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect.
5. **Workspace-aware `vibe install` / `vibe build`** — discover, unified resolve, materialise, regenerate (§2.7).
6. **`vibe reinstall` regeneration** (§2.10) and **published-copy regeneration** in `vibe workspace publish` (§2.11).
7. **Migration + docs** — existing-project migration, the vibevm self-migration, `VIBEVM-SPEC.md` edits (under owner sanction — §5 item 8), `ROADMAP.md` / `CHANGELOG.md`, the `docs/` sweep.
8. **Effective-spec view** — shares the engine; the detailed shape is v1.5 scope (§2.8).

---

## 8. Version history {#history}

- **2026-05-21 — draft 1.** Requirements captured in an owner design session: the loading-model redesign answering PROP-007 §6 question 3, the static/dynamic-linking spine, the four-fork resolution. Rationale recorded in [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md).
- **2026-05-21 — draft 2.** The eight §5 open questions resolved in a follow-up owner session: `vibedeps/`, `vibe reinstall`, the TOML `INDEX.md`, `[writes]` retired, dynamic conditions reusing the subskill `[activation]` vocabulary, a minimal `[boot]` table, the effective-spec view kept v1.5-scoped. The `VIBEVM-SPEC.md` sanction (§5 item 8) is the one item carried to Phase 7. Ready for M1.18 implementation.
