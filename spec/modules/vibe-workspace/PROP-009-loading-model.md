# PROP-009: Loading model — computed boot composition and the effective spec {#root}

<status stage="impl" state="done" action="continue" comment="C 2026-07-25: M1.18 phases 1-7 shipped 2026-05-22 and this session booted on them; phase 8's engine-backed projection is v1.5 scope"/>

##milestone-line **Milestone:** `M1.18` ([`ROADMAP.md`](../../../ROADMAP.md)) — **shipped**, implementation-locked. @impl/done

##status-line **Status: IMPLEMENTED** — requirements resolved 2026-05-21; M1.18 phases 1–7 shipped 2026-05-22, and every session of this repository boots on them (`STATIC.md` first, then the TOML `INDEX.md` with its `[[entry]]` grammar). The dynamic-entry `when` gate (OS-scoped) shipped 2026-05-22 — see §8. Phase 8's **engine-backed** effective-spec projection stays v1.5 scope; the plain `vibe show effective` concatenation is live. @impl/done

##related **Related:** [`VIBEVM-SPEC.md` §4.2 / §4.6 / §6 / §13.1](../../../VIBEVM-SPEC.md); [PROP-007](PROP-007-workspace.md) (workspace — PROP-009 answers its [§6 question 3](PROP-007-workspace.md#open)); [PROP-003 §2.5](../vibe-resolver/PROP-003-dep-evolution.md) (subskills, delivery modes, the `[activation]` vocabulary); [PROP-002](../vibe-registry/PROP-002-decentralized-registry.md) (identity, registry). @spec/done

##design-rationale **Design rationale:** [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md) — the *why*, the static/dynamic-linking metaphor, the fork-by-fork record. Non-normative; this PROP is the contract. @spec/done

##OWNER-SANCTION **Owner sanction:** PROP-009 reshapes the owner-frozen `VIBEVM-SPEC.md` (§6 boot model, §4.2 layout, §4.6 effective spec, §13.1 package layout). The `VIBEVM-SPEC.md` edits required explicit owner sanction; it was **granted 2026-05-22** — for a full consistency pass, not only those four sections — and landed in Phase 7. See §5 item 8. @impl/done

---

## 1. Motivation {#motivation}

##open-question-origin PROP-007 shipped the workspace data model but left [§6 question 3](PROP-007-workspace.md#open) open: when a dependency is resolved for member M, into which member's `spec/` does its content land? @impl/done

- ##not-a-directory-choice The question is not a directory choice. vibevm's boot model (`VIBEVM-SPEC.md` §6) — a flat `spec/boot/NN-*.md` directory, one sequence, one entry point — holds for exactly one project shape: a single project with a single entry point. @impl/done
- ##workspace-shape A workspace has N nodes, N entry points (a developer opens an agent inside any member — PROP-007's "the user works in a sub-project and doesn't notice it is part of something bigger"), N boot sequences, and one shared dependency set under unified resolution. The flat model cannot be stretched over this. @impl/done

- ##MODEL-REPLACED PROP-009 replaces the loading model. @impl/done
- ##INCLUDE-RULE The owner's hard constraint: **installing a dependency must never modify any node's authored spec** — the C++ rule that you do not paste a header's text into your `#include`. @impl/done
- ##linker-frame The owner's frame for the replacement is static vs dynamic linking. The linker metaphor and the fork-by-fork record are in the [design document](../../design/loading-and-boot-model.md). @impl/done

---

## 2. Decisions {#decisions}

### 2.1 Two trees — authored spec and materialised dependencies {#two-trees}

##TWO-TREES **Decision.** A node's authored `spec/` and its materialised dependencies live in physically separate trees. `vibe install` **never writes into any node's authored `spec/`**. @impl/done

- ##TREE-AUTHORED Authored `spec/` — written only by the node's author. Unchanged definition. @impl/done
- ##TREE-VIBEDEPS Materialised dependencies — a `vibedeps/` tree at the **absolute workspace root** (PROP-007 §2.3), written only by `vibe`. One slot per resolved package, `vibedeps/<kind>-<name>/<version>/`, holding the package's published tree verbatim ([PROP-024 §2.2](../../common/PROP-024-code-bearing-packages.md#shippable-tree) re-scopes "published" to the **shippable tree** — source minus build output — for code-bearing packages). A package's prompt content lives under its own `spec/`, so a boot snippet materialises at `vibedeps/<slot>/spec/boot/<file>` (PROP-024 §2.1). Unified resolution (PROP-007 §2.4) guarantees one version per package, so one slot serves the whole workspace. @impl/done
- ##VIBEDEPS-COMMITTED `vibedeps/` is **committed** to the repository. A fresh clone is immediately bootable with no `vibe install`; the dependency corpus is visible and diffable; this matches the spec-driven principle that the committed spec corpus is the product. @impl/done

- ##MIRROR-RETIRED **Consequence — the mirror layout is retired.** `VIBEVM-SPEC.md` §13.1's mirror layout (a package's `[writes]` entry is both source and target path) worked only because a dependency landed at one fixed path in every project. @impl/done
- ##WRITES-RETIRED-WHY A materialised package is now its own verbatim subtree under `vibedeps/<slot>/`; a package's internal cross-references must become package-relative or `spec://` URIs. `[writes]` is retired (§2.6): a materialised package *is* its own subtree, and a per-file write list has nothing left to declare. @impl/done

### 2.2 The effective boot sequence {#effective-boot}

##EFFECTIVE-BOOT **Decision.** Every node has an **effective boot sequence**, computed by `vibe` from the unified resolution: @impl/done

```
inherited foundation (from ancestors) + the node's own authored boot
  + the boot of the node's transitive dependencies + user overrides
```

- ##FOUNDATION-DOWN **Inherited foundation** flows down: a member inherits the project-wide foundation boot of its ancestors up to the absolute root (conventions, the four rules, technology choices). @impl/done
- ##DEPS-UP **Dependency boot** flows up: a node's sequence includes the boot of everything it transitively requires. @impl/done
- ##MATRYOSHKA-SCOPES A node that is itself a workspace aggregates its members' sequences — the root's effective boot is the union of the whole tree; a leaf member's is its own subtree only. The hierarchy scopes cost: a session opened in a small member boots small. @impl/done
- ##COMPUTED-NOT-COPIED The sequence is **computed per node directly from the resolution graph**, never copied physically between levels (copying drifts; computation does not). @impl/done

### 2.3 Generated boot artifacts {#artifacts}

##ARTIFACTS-PAIR **Decision.** For every entry-point node, `vibe install` generates two artifacts under the node's `spec/boot/`: @impl/done

- ##ARTIFACT-STATIC-MD **`STATIC.md`** — the **anchor-qualified** concatenation, in priority order, of every `static`-typed (§2.4) contribution in the node's effective boot: each contribution's content is carried whole, with its label definitions (heading anchors, fact ids) and intra-contribution label links rewritten to the origin-qualified form `<origin-slug>--<original>` (PROP-035 §8's qualify phase, B-011 — owner-approved 2026-08-04), so the compiled file's label namespace is collision-free by construction. Read first. Generated only when the node has `static` contributions. @spec/work
- ##STATIC-EMITS-ONCE-EACH **The lane emits each package's text once (B-006, owner-approved 2026-08-04).** A static entry that would embed a compiled unit artifact (a dependency that statically links children, PROP-038 §2.1) is **de-substituted** when every boot-bearing member of its static zone (itself excluded) is already present as an individual static entry of the same lane: a package with its own boot snippet reverts to that snippet — its own text still enters, once; a snippetless aggregator leaves a **generated provenance stub** — one marker comment naming the origin and stating the zone is emitted member-by-member, never a `#use` directive (which would route a mandatory read back into the unit artifact carrying the very duplicate) and never a second copy of the text. An entry with **any** zone member not individually present keeps its unit-artifact substitution whole — the rule removes proven duplicates only and never drops coverage. The decision is a pure function of the lane's entry set and the unit zones — independent of how many consumers pull the package, through which mix of `static` / `static-transitive` / `static-hard` edges, and in what order: the closure walk already emits one entry per package identity, and the single-version invariant (PROP-038 §2.6) makes every copy of a member byte-identical, so covered-zone de-substitution can never lose text. **The stated residual:** under *partial* coverage a member both individually present and embedded in the kept unit artifact is carried twice — accepted deliberately, because the composition layer only chooses paths and never rewrites a unit artifact's content; deduplicating a widely-shared member *inside* unit artifacts is the hoisting plane's job (PROP-038 §2.4–2.5 — a hoisted member's text is replaced by a `#use` marker in every unit that shares it), and the undercounting hoist trigger is recorded at PROP-038's DRIFT-030 review stop, not here. @spec/work
- ##STATIC-OPENS-WITH-RESOLUTION-PREAMBLE **`STATIC.md` opens with the resolution preamble and the tombstone table** (PROP-035 §11, B-011): the label convention, alias semantics, the lookup rule, and every renamed short name with its qualified heirs — placed first because the first lines of the first file read are the one position an agent re-reads every session and after compaction (the owner's priority-placement addition, 2026-08-04). @spec/work
- ##COMPILED-LANE-IS-NOT-A-CITATION-TARGET **A generated `STATIC.md` is not a citation target** — authored text never cites `spec://…/boot/STATIC#…`; the lane is compiler output, and source-of-truth is the package source under `vibedeps/` (PROP-035 §11's lint, B-011 §6.1). @spec/work
- ##ARTIFACT-INDEX-MD **`INDEX.md`** — a generated **TOML manifest** of the rest of the sequence: a `schema` version, a `static` pointer (the path of `STATIC.md`, when one exists), and an ordered list of `[[entry]]` tables. Each entry carries `path`, `kind` (`"static"` — a resolved file the agent reads directly; `"dynamic"` — an INCLUDE the agent resolves at boot, §2.4), and, for dynamic entries, `when` (the activation condition, §2.4). The manifest is flat and machine-precise — `vibe` performed the graph walk once at generation time; the agent parses one TOML document and reads the listed files, with no recursion, discovery, or cycle-detection. @impl/done

```toml
# spec/boot/INDEX.md — generated by vibe, do not edit.
schema = 1
static = "spec/boot/STATIC.md"

[[entry]]
path = "spec/boot/00-core.md"
kind = "static"

[[entry]]
path = "vibedeps/stack-windows/2.1.0/boot/windows.md"
kind = "dynamic"
when = "os:windows"
```

- ##ARTIFACTS-GENERATED Both artifacts are generated, git-tracked, and marked "generated — do not edit". @impl/done
- ##ARTIFACTS-CARRY-NO-TOKEN-BUDGET **A generated boot artifact carries no token budget** (owner ruling, 2026-07-29). The size budgets of the `addressable-specs` flow govern **authored** documents, where a page over budget is a page nobody re-reads and the remedy is to split it. `STATIC.md` — the project-wide one and every per-node one — is compiler output: it is not read by a human, not edited, and not splittable, because its size is the sum of what the resolution graph says the session must have. Compiling a document *into* the lane is therefore a legitimate answer to a dangling pointer, and «the lane grew» is not an objection to it. What remains measurable, and is the honest cost, is the **session's** context, which the `dynamic` lane exists to keep off the critical path (§2.4). @impl/done
- ##AUTHORED-ALONGSIDE Authored boot files (the user-owned snippets, the node's own authored boot) continue to live alongside as ordinary files; `INDEX.md` references them in computed order. @impl/done

- ##SESSION-START-ORDER **Session-start order:** the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect → `spec/boot/STATIC.md` (if present) → `spec/boot/INDEX.md` and the entries it names, in order. @impl/done
- ##PURE-FILE-READING Boot remains **pure file-reading** — the redirect never becomes "run `vibe`", preserving the zero-dependency cross-agent property of `VIBEVM-SPEC.md` §6.1. @impl/done

##REDIRECT-MANAGED-BLOCK **The redirect is a managed block (PROP-012).** The `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect is not a whole generated file. [PROP-012](PROP-012-managed-redirect-block.md) refines it: vibevm owns only a delimited `<vibevm>` block inside each shared instruction file and preserves every byte outside it — the file is a co-tenant surface, not vibevm's property. @impl/done

### 2.4 Inclusion types — `static`, `dynamic` {#inclusion-types}

##INCLUSION-TYPES **Decision.** Each dependency declares an **inclusion type**, set by the consumer in its `vibe.toml` on the `[requires.packages]` entry: @impl/done

```toml
[requires.packages]
"flow:wal"        = { version = "^0.3", link = "dynamic" }   # default
"flow:discipline" = { version = "^1.0", link = "static" }    # emergency priority lane
"stack:rust"      = { version = "^2.0", link = "dynamic" }   # conditional / context-gated
```

- ##LINK-DYNAMIC `link = "dynamic"` — **the default.** `vibe` resolves the contribution to a concrete path in `INDEX.md`; the agent reads it dynamically, on demand. An optional `when` condition gates the read: with a `when` it is a **conditional** INCLUDE (loaded only when the condition holds) — mechanically the subskill `lazy-pull` delivery mode; without one it is read unconditionally. The `when` draws on the subskill `[activation]` probe vocabulary (PROP-003 §2.5) — one probe grammar across both mechanisms. **v1 implements the `os:` probe end-to-end** — `when = "os:windows"` matches the session's operating system (`windows` / `macos` / `linux`); the remaining probes are reserved until PROP-003's activation engine is built. @impl/done
- ##LINK-STATIC `link = "static"` — the contribution's boot text is compiled into `STATIC.md` ahead of time (whole, anchor-qualified — §2.3). Read first, one read, maximum attention weight. The **emergency priority lane** — for top-level skills and critical disciplines whose priority must be guaranteed by position, not by trusting agent-side resolution. Used sparingly; it duplicates the text on disk. @impl/done

##LINKING-SPECTRUM The two types are the two ends of the static/dynamic-linking spectrum: `static` is compiled in ahead of time (the `STATIC.md` lane), `dynamic` is loaded by reference on demand (the `INDEX.md` lane), and the old third type is gone — a conditional load is just a `dynamic` entry carrying a `when`. @impl/done

##SUGGESTED-DEFAULT A package MAY declare a suggested default inclusion type in its own `[boot_snippet]`; the consumer's declaration always wins. Absent both, the type is `dynamic`. @impl/done

##WHEN-FORCES-DYNAMIC A `[boot_snippet]` that declares a `when` condition (§2.6) stays a conditional `dynamic` entry, irrespective of `link`: a condition cannot be honoured by the ahead-of-time `static` lane, so a `when` forces the gated INDEX form. It is a correctness constraint, not a preference — OS-specific content must never reach a session on the wrong OS. @impl/done

### 2.5 Ordering by category — the `NN-` prefix is retired {#ordering}

##NN-RETIRED **Decision.** `vibe` owns the order of entries in the generated artifacts. The author-chosen two-digit `NN-` prefix (`VIBEVM-SPEC.md` §6.2) is **retired** — it cannot survive a workspace's combined namespace, and §6.5 already admits it provisional. @impl/done

- ##CATEGORY-NOT-NUMBER A package declares a **category** for its boot snippet, not a number. The categories preserve the intent of the old range bands: `foundation`, `flow`, `stack`, `user-override`. @impl/done
- ##CATEGORY-ORDER Within the computed sequence the order is: `foundation` → the node's own → dependency boot (topologically — a dependency before its dependents) → `user-override`. `static` contributions are concatenated into `STATIC.md` in the same relative order. @impl/done
- ##NO-PREFIX-COLLISIONS Prefix collisions — the failure mode of `VIBEVM-SPEC.md` §6.3 — become impossible by construction; `BootSnippetConflict` / `BootSnippetNumericConflict` (`vibe-install`) are removed. @impl/done
- ##RESERVED-NAMES The user-owned files keep their reserved names (`00-core.md`, `90-user.md`) by convention; `vibe` places them at the foundation / override ends. @impl/done

### 2.6 Manifest schema changes {#schema}

##schema-decision-lead **Decision.** @impl/done

- ##SCHEMA-LINK-FIELD `[requires.packages]` inline-table entries accept an optional `link` field (§2.4): `"static" | "dynamic"`, default `dynamic`. Valid on registry-, path-, and git-source dependencies. @impl/done
- ##SCHEMA-BOOT-SNIPPET `[boot_snippet]` (package-role) drops the `filename` field (the `NN-` target name) and gains `category` (§2.5); `source` — the path to the boot file inside the package — is retained. It may carry an optional suggested `link` default, and an optional **`when`** activation condition — the declaration site for §2.3's dynamic-entry `when`, closing the gap Phase 4 flagged. For v1 the only `when` is an operating-system match, the wire string `"os:<name>"` with `<name>` one of `windows` / `macos` / `linux`; a snippet carrying a `when` is `dynamic` (§2.4). The package author owns this declaration: whether a boot snippet is OS-specific is the author's knowledge, not the consumer's. @impl/done
- ##SCHEMA-WRITES-REMOVED `[writes]` (package-role) is **removed** (§2.1, §2.7) — a package's materialised footprint is its verbatim tree under its `vibedeps/` slot; a per-file write list has nothing left to declare. @impl/done
- ##SCHEMA-BOOT-TABLE A minimal project-level `[boot]` table carries workspace-wide loading settings — for v1, only a default `link` override. Room to grow; nothing more is added now. @impl/done
- ##SCHEMA-LOCK-BUMP A `vibe.lock` schema bump may be required to record materialisation slots and inclusion types — assessed in Phase 1. @impl/done

### 2.7 Workspace-aware `vibe install` / `vibe build` {#install}

##WORKSPACE-AWARE **Decision.** `vibe install` and `vibe build` discover the workspace and operate on it as a whole — the piece PROP-007 §6 q3 deferred, now subsumed. @impl/done

- ##INSTALL-UNIFIED Run anywhere inside a workspace, `vibe install` calls `Workspace::discover`, runs **one unified resolution** across every member's `[requires]`, materialises each resolved package once into `vibedeps/` (§2.1), and regenerates the boot artifacts (§2.3) for every entry-point node. One `vibe.lock` at the absolute root (PROP-007 §2.4). @impl/done
- ##PLAN-UNIT The plan / confirm / apply contract holds, but the plan's unit is **the set of packages to materialise plus the boot artifacts to regenerate**, not a per-file write list — `[writes]` is retired (§2.6). @impl/done
- ##SCOPE-FLAG `-p <member>` scopes resolution *reporting* to one member; the materialisation and the single root lockfile are always workspace-wide — unified resolution admits no per-member subset. @impl/done
- ##DEGENERATE-PATH A standalone single-package project is a degenerate workspace and follows the identical path (§2.9). @impl/done

### 2.8 The computed-view engine — boot and the effective spec {#engine}

##ENGINE-TWO-PROJECTIONS **Decision.** The boot artifacts (§2.3) and the **effective spec** (`VIBEVM-SPEC.md` §4.6 — the merged corpus consumed by `vibe build` and `vibe show effective`) are two projections of one **computed-view engine**: workspace walk (`Workspace::discover`) + unified resolution + two-tree layering (§2.1, §2.2). @impl/done

- ##VIEW-BOOT The **boot view** projects the boot-category content into the ordered `STATIC.md` / `INDEX.md` (§2.3). @impl/done
- ##VIEW-EFFECTIVE-SPEC The **effective-spec view** projects the full layered corpus — authored `spec/` plus materialised `vibedeps/` — into the effective spec. @spec/done
- ##BOTH-DETERMINISTIC Both are deterministic and regenerated by `vibe install`. @impl/done

##V15-SCOPE The effective-spec view's detailed shape is **v1.5 scope** (it feeds `vibe build`). PROP-009 fixes only that it shares the engine, so it is not built as a later retrofit. @spec/done

### 2.9 Uniform model — every project is a workspace {#uniform}

##UNIFORM-MODEL **Decision.** The loading model is uniform: a single-package project is a degenerate (zero-member) workspace. `Workspace::discover` already degenerates cleanly (PROP-007 §2.3). @impl/done

##ONE-CODE-PATH There is one loading model, one set of artifacts, one code path. @impl/done

- ##EVERYONE-MIGRATES Every existing project migrates (§4). vibevm is pre-release; M1.17's no-legacy hard break is the precedent. @impl/done
- ##SELF-MIGRATION The vibevm repository, itself a vibevm project, migrates too — `spec/boot/00-core.md` and `90-user.md` stay user-owned authored boot; the generated `STATIC.md` / `INDEX.md` join them. @impl/done

### 2.10 Regeneration — `vibe reinstall` {#regen}

##REINSTALL **Decision.** `vibe reinstall [<path>] [--force]` reinstalls and regenerates the materialised state. @impl/done

- ##REINSTALL-ANCESTORS It targets any node in the workspace. Reinstalling a node regenerates that node **and every ancestor up to the absolute root** — the matryoshka (§2.2) means an ancestor's aggregated artifacts depend on the node's. `vibe reinstall` run at the root regenerates the whole tree. @impl/done
- ##REINSTALL-NO-FORCE Without `--force` it recomputes the materialisation and the boot artifacts from the existing `vibe.lock` and the local cache — no fresh resolution. @impl/done
- ##REINSTALL-FORCE `--force` re-fetches the file content of the whole targeted subtree from the source repositories, overwriting the current `vibedeps/` files and bypassing the cache. The escape hatch for a corrupted, hand-edited, or wrongly-generated subtree. @impl/done

##reinstall-purpose It exists for when the materialised state is believed stale or a previous generation pass was wrong. @impl/done

### 2.11 Published-copy regeneration {#publish}

##PUBLISH-REGEN **Decision.** `vibe workspace publish` (PROP-007 §2.7) regenerates the boot artifacts of each staged copy for the **published shape** — where dependencies are registry-resolved and version-pinned, not path-sourced. @impl/done

##publish-dangling-why This consumes PROP-007 §2.5's dual-form `{ path, version }`: the local `vibedeps/` slots and path entries become registry references in the published copy's artifacts. Publishing the development tree's own path-resolved artifacts would dangle for an external consumer. @impl/done

---

## 3. Command and crate surface {#surface}

- ##SURF-INSTALL-BUILD `vibe install` / `vibe build` — workspace-aware (§2.7). @impl/done
- ##SURF-REINSTALL `vibe reinstall` — regeneration (§2.10). @impl/done
- ##SURF-PUBLISH `vibe workspace publish` — gains published-shape artifact regeneration (§2.11). @impl/done
- ##SURF-SHOW-EFFECTIVE `vibe show effective` — **ships** in its simple concatenation form (every `spec/boot` file plus every installed package's `files_written`, joined with `spec://` provenance headers, per its own `--help`); the §2.8 engine-backed projection stays v1.5 scope. @impl/done
- ##SURF-ENGINE-CRATE The computed-view engine lands either as a new crate (`vibe-boot` / `vibe-view`) or inside `vibe-workspace` (which already owns discovery and the `[workspace.versions]` finalize pass) — decided at implementation time. @impl/done

---

## 4. Migration {#migration}

- ##MIGRATION-ONCE Every existing project migrates once (§2.9). On the first `vibe install` after the upgrade, `vibe` rewrites the project: dependency content moves out of the authored `spec/` into `vibedeps/`; `NN-` boot files become categorised authored boot or generated artifacts; `STATIC.md` / `INDEX.md` are generated; the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect is rewritten. @impl/done
- ##NO-SHIM There is no compatibility shim — a pre-PROP-009 layout is migrated, not supported in place. @impl/done
- ##vibevm-migrated The vibevm repository is migrated as part of the milestone. @impl/done

---

## 5. Resolved questions {#open}

##resolved-lead The eight questions opened in draft 1 were resolved in an owner session on 2026-05-21. @impl/done

1. ##RES-VIBEDEPS **`vibedeps/` directory** — the materialised-dependency tree (§2.1); slot layout `vibedeps/<kind>-<name>/<version>/`. @impl/done
2. ##RES-REINSTALL **`vibe reinstall`** — the regeneration command (§2.10), replacing the working name `vibe boot`; it regenerates a node and every ancestor to the root, and `--force` re-fetches a subtree from source. @impl/done
3. ##RES-INDEX-TOML **`INDEX.md` is a TOML manifest** — `schema` / `static` / `[[entry]]` (§2.3); machine-precise over an LLM-native list. @impl/done
4. ##RES-WRITES-RETIRED **`[writes]` is retired** (§2.6) — a package's footprint is its verbatim tree under its `vibedeps/` slot. @impl/done
5. ##RES-ACTIVATION-VOCAB **Dynamic conditions reuse the subskill `[activation]` vocabulary** verbatim (§2.4; PROP-003 §2.5) — one probe grammar, no parallel one. @impl/done
6. ##RES-BOOT-TABLE **A minimal `[boot]` table** (§2.6) — for v1 it carries only a workspace-wide default `link`. @impl/done
7. ##RES-V15-VIEW **The effective-spec view stays v1.5 scope** (§2.8) — PROP-009 fixes only that it shares the computed-view engine. @spec/done

##deferred-lead **Deferred:** @spec/done

8. ##RES-SPEC-SANCTION `VIBEVM-SPEC.md` edits — **resolved 2026-05-22.** The owner granted the sanction for a full consistency pass (not only §6 / §4.2 / §4.6 / §13.1); it landed in Phase 7. @impl/done

---

## 6. Rejected / deferred alternatives {#rejected}

- ##REJ-BUBBLE-UP **Bubble every dependency's boot into the root `spec/boot/`.** Rejected — it is the "merge dependency specs into the authored spec" the owner ruled out, and it makes one flat namespace for the whole workspace. @spec/done
- ##REJ-RUN-VIBE **Boot by running `vibe` at session start.** Rejected — it would always be fresh, but it breaks the zero-dependency cross-agent property (`VIBEVM-SPEC.md` §6.1) and adds a process exec to every session. Boot stays pure file-reading (§2.3). @spec/done
- ##REJ-PHYSICAL-COPY **Copy boot snippets physically leaf-to-root (the literal matryoshka).** Rejected in favour of computing each level directly from the resolution graph (§2.2) — physical copying drifts between levels. @spec/done
- ##REJ-GITIGNORED-CACHE **A gitignored dependency cache.** Rejected — a committed `vibedeps/` keeps a fresh clone bootable and the corpus reviewable. @spec/done

---

## 7. Phase plan {#phases}

##phases-lead Targets M1.18. PROP-008 (qualified naming) shifts to M1.19. **Phases 1–7 shipped 2026-05-22**; phase 8 is v1.5 scope. @impl/done

1. ##PHASE-1-SCHEMA **Schema** — the `link` field, `[boot_snippet]` `category`, retire the `NN-` filename and the `[writes]` section; `vibe.lock` bump if needed. `vibe-core`. @impl/done
2. ##PHASE-2-MATERIALISATION **Materialisation tree** — the `vibedeps/` layout, materialise packages verbatim; retire the mirror layout. @impl/done
3. ##PHASE-3-ENGINE **Computed-view engine** — per-node effective boot computation from the unified resolution. @impl/done
4. ##PHASE-4-ARTIFACTS **Artifact generation** — `STATIC.md` / the TOML `INDEX.md`; the `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` redirect. @impl/done
5. ##PHASE-5-WORKSPACE-AWARE **Workspace-aware `vibe install` / `vibe build`** — discover, unified resolve, materialise, regenerate (§2.7). @impl/done
6. ##PHASE-6-REINSTALL **`vibe reinstall` regeneration** (§2.10) and **published-copy regeneration** in `vibe workspace publish` (§2.11). @impl/done
7. ##PHASE-7-MIGRATION **Migration + docs** — existing-project migration, the vibevm self-migration, `VIBEVM-SPEC.md` edits (under owner sanction — §5 item 8), `ROADMAP.md` / `CHANGELOG.md`, the `docs/` sweep. @impl/done
8. ##PHASE-8-EFFECTIVE-SPEC **Effective-spec view** — shares the engine; the detailed shape is v1.5 scope (§2.8). @impl/plan

---

## 8. Version history {#history}

- ##HISTORY-DRAFT-1 **2026-05-21 — draft 1.** Requirements captured in an owner design session: the loading-model redesign answering PROP-007 §6 question 3, the static/dynamic-linking spine, the four-fork resolution. Rationale recorded in [`spec/design/loading-and-boot-model.md`](../../design/loading-and-boot-model.md). @spec/done
- ##HISTORY-DRAFT-2 **2026-05-21 — draft 2.** The eight §5 open questions resolved in a follow-up owner session: `vibedeps/`, `vibe reinstall`, the TOML `INDEX.md`, `[writes]` retired, dynamic conditions reusing the subskill `[activation]` vocabulary, a minimal `[boot]` table, the effective-spec view kept v1.5-scoped. The `VIBEVM-SPEC.md` sanction (§5 item 8) is the one item carried to Phase 7. Ready for M1.18 implementation. @spec/done
- ##HISTORY-PHASE-7 **2026-05-22 — Phase 7 shipped.** The migration-and-docs phase landed in M1.18: the vibevm self-migration, the `VIBEVM-SPEC.md` consistency pass (owner sanction granted — §5 item 8), and [PROP-012](PROP-012-managed-redirect-block.md), which refines §2.3's redirect into a managed `<vibevm>` block. Phases 1–7 are shipped; phase 8 (the effective-spec view) remains v1.5 scope. @spec/done
- ##HISTORY-WHEN-SITE **2026-05-22 — the `when` declaration site.** §2.3's dynamic-entry `when` is pinned to `[boot_snippet].when` (§2.6), closing the contract gap Phase 4 flagged — §2.3 showed `when` but no field declared it. v1 scope is deliberately small: the only condition is an operating-system match (`when = "os:<name>"`), shipped end-to-end through the `vibe-core` schema, the computed-view engine, and the `INDEX.md` renderer. A `[boot_snippet]` carrying a `when` is `dynamic` irrespective of `link` (§2.4). The OS probe is also reserved as `if_os` in the subskill `[activation]` vocabulary (PROP-003 §2.5.2), so the two mechanisms share one grammar. The wider probe set follows when PROP-003's activation engine is built. @spec/done
- ##HISTORY-B011-QUALIFIED **2026-08-04 — the static lane becomes anchor-qualified (B-011, owner-approved).** §2.3's `##ARTIFACT-STATIC-MD` changes from «verbatim concatenation» to «anchor-qualified concatenation» (labels rewritten under the contribution's origin slug — PROP-035 §8's new qualify phase), the lane opens with the resolution preamble + tombstone table, and a generated lane stops being a legal citation target. Motivation: 59 `duplicate-anchor` warnings over the compiled lane (`{#root}` ×26) and the strip-safety design; the full rationale and fork record: [`spec/design/deterministic-loading-aliasing.md`](../../design/deterministic-loading-aliasing.md). Landed the same day (five worker slices; the host lane regenerated 59 → 0), retiring the §2.3 annotated interim. @spec/work
- ##HISTORY-B006-ONCE-EACH **2026-08-04 — the lane emits once each (B-006, owner-approved: A1·B1·C1).** §2.3 gains `##STATIC-EMITS-ONCE-EACH`: a static entry embedding a compiled unit artifact is elided to a provenance stub when its whole static zone is already present member-by-member — the compose-time dedup that removes the aggregator double-emission (the git-practices family carried twice, 323 nested lines, 164 double-qualified labels) without touching the unit artifacts, hoisting, or coverage in a dynamic-member topology. Rationale, forks and the measured traps of the alternatives: [`spec/design/lane-composition-dedup.md`](../../design/lane-composition-dedup.md); the linker-side twin sentence: PROP-038 §2.1; the per-node-qualify rider: PROP-035 §8. @spec/work
