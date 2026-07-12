# Glossary

Definitive vocabulary for the vibevm project. Spec-text, PROPs, code, docs, and commit messages all draw from this list — when a term appears with a specific meaning here, that's the meaning everywhere. If you see a synonym in the wild that isn't on this page, it's drift.

Where the canonical decision lives, the entry links to it.

---

### apply (install pipeline stage)

Stage 4 of the `vibe install` pipeline. After plan + confirm: writes files, updates the lockfile. Reverse-rollback on partial failure (best-effort). Spec: [`VIBEVM-SPEC.md` §5.6](../VIBEVM-SPEC.md).

### authoring

Writing a new package. Per-kind guides under [`docs/authoring-flow.md`](authoring-flow.md), [`authoring-feat.md`](authoring-feat.md), [`authoring-stack.md`](authoring-stack.md).

### boot snippet

A package's contribution to a node's boot sequence. A package declares it in its `[boot_snippet]` table with a `category` (`foundation` / `flow` / `stack` / `user-override`) and a `source` (the path to the boot file inside the package); it may carry a suggested `link` default. `vibe install` does **not** copy the snippet into a numbered `spec/boot/` slot — it folds the contribution into the node's computed boot sequence, generated into [`INLINE.md` / `INDEX.md`](#boot-artifacts). The two-digit `NN-` filename prefix and the flat numeric-order `spec/boot/` directory are **retired** — see [the loading model](loading-model.md). Spec: [PROP-009 §2.5](../spec/modules/vibe-workspace/PROP-009-loading-model.md#ordering).

### boot artifacts

The two files `vibe install` generates under each entry-point node's `spec/boot/`: `INLINE.md` (the verbatim concatenation of every `inline`-linked boot contribution — read first; generated only when there are inline contributions) and `INDEX.md` (a generated TOML manifest — a `schema` int, an optional `inline` pointer, and ordered `[[entry]]` tables, each with a `path` and a `kind` of `"static"` or `"dynamic"`). Both are git-tracked and carry a "generated — do not edit" header. The agent reads `INLINE.md` then `INDEX.md` and the entries it names. Spec: [PROP-009 §2.3](../spec/modules/vibe-workspace/PROP-009-loading-model.md#artifacts).

### computed boot sequence

A node's full boot order, *computed* by `vibe` from the unified resolution rather than authored as a flat directory: inherited foundation (from ancestors) + the node's own authored boot + the boot of its transitive dependencies + user overrides. Projected into the [boot artifacts](#boot-artifacts). Spec: [PROP-009 §2.2](../spec/modules/vibe-workspace/PROP-009-loading-model.md#effective-boot).

### link type (boot inclusion type)

How a dependency's boot contribution is folded into a consumer's boot. Set per dependency in `vibe.toml` `[requires.packages]` via `link = "..."`: `static` (default — resolved to a concrete path in `INDEX.md`, read directly), `inline` (concatenated verbatim into `INLINE.md`, read first — the emergency priority lane), or `dynamic` (an INCLUDE the agent resolves at boot, gated by a `when` activation condition). Spec: [PROP-009 §2.4](../spec/modules/vibe-workspace/PROP-009-loading-model.md#inclusion-types).

### canonical URL (registry)

The `[[registry]].url` value verbatim, before mirror substitution. The cache bucket is keyed on `sha256(normalize(canonical_url))`, NOT on whichever mirror URL actually answered the fetch. Mirror swaps therefore don't invalidate the cache — see [PROP-002 §2.6](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#cache).

### capability

An abstract interface a package can `[provides]` and another package can `[requires]`. Syntax: `<namespace>:<name>[@<version>]`, e.g. `ui:landing-page-host@^0.1`, `db:any@>=1.0`. The depsolver matches consumer capabilities against producer capabilities at install time. See [`CapabilityRef`](../crates/vibe-core/src/capability_ref.rs) and [PROP-002 §2.9](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#capability).

### content_hash

`sha256:<hex>` over the deterministically-ordered file tree of a package. **The identity** half of `(group, name, version, content_hash)`. Computed by [`vibe-registry::compute_content_hash`](../crates/vibe-registry/src/lib.rs); recorded per-`[[package]]` in `vibe.lock`. PROP-002 §2.1 makes content_hash the load-bearing identity field; URL is informational only.

### content drift

Mismatch between a `vibe.lock` entry's pinned `content_hash` and the freshly-fetched bytes' hash for the same `(group, name, version)`. Surfaces as `InstallError::ContentDrift`; refused at plan time. Catches force-pushed tags, malicious mirrors, override-source rotations.

### `flow` (kind)

Discipline / process module. Specs read at session boot that govern *how the team works* (commit conventions, WAL protocol). Authoring: [`authoring-flow.md`](authoring-flow.md). Examples: `flow:wal`, `flow:atomic-commits`.

### `feat` (kind)

Functional feature. The *what* of a project, expressed as specification (purpose, behaviour rules, acceptance criteria). Stack-agnostic at authoring time. Authoring: [`authoring-feat.md`](authoring-feat.md). Examples (planned M1.5): `feat:welcome-page`, `feat:user-authentication`.

### `stack` (kind)

Language / framework target. The *how* a feat becomes real software. Authoring: [`authoring-stack.md`](authoring-stack.md). Examples (planned M1.5): `stack:rust-cli`, `stack:rust-axum`, `stack:typescript-next`.

### `tool` (kind)

Reserved for v2+. Not yet authorable.

### group

The reverse-FQDN qualifier in a package's identity — `org.vibevm`, `com.acme`. Dot-separated segments, each `[a-z0-9_-]+`, ASCII lowercase; **mandatory** in `[package]`. With `name` it forms the `(group, name)` package identity — `name` is unique within a `group`, not globally and not within a `kind`. Reverse-FQDN is the recommended convention, not a shape the core enforces (the posture Maven takes on `groupId`). `org.vibevm` is the canonical group for every first-party vibevm package. [PROP-008 §2.1](../spec/modules/vibe-registry/PROP-008-qualified-naming.md).

### identity (of a package)

The tuple `(group, name, version, content_hash)` — content-addressed per [PROP-002 §2.1](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#identity), `group`-qualified per [PROP-008 §2.2](../spec/modules/vibe-registry/PROP-008-qualified-naming.md). `kind` is **not** part of it — it is metadata. Two installs with the same identity are *the same install* regardless of which URL served them — that's the property that makes mirrors transparent and host-migration cheap.

### kind

One of `flow`, `feat`, `stack`, `tool`. Closed enum; adding a fifth is a spec change. Defined in [`VIBEVM-SPEC.md` §4.1](../VIBEVM-SPEC.md). Since [PROP-008](../spec/modules/vibe-registry/PROP-008-qualified-naming.md) `kind` is **metadata, not identity** — it places content and drives the `--kind` filter, but package identity is `(group, name, …)` and the default `fqdn` repo name does not carry it.

### lockfile

`vibe.lock` at the project root. Records exactly what is installed, with full provenance (registry name, source kind, source URL, source ref, resolved commit, content hash, transitive deps, override flag). Schema v5 today. Reference: [`docs/lockfile-format.md`](lockfile-format.md).

### manifest

The TOML schema describing a vibevm node. Every node carries one `vibe.toml`; its role is set by which sections it carries — `[project]` (a consumer), `[package]` (a publishable artifact), `[workspace]` (a coordinator). The lockfile `vibe.lock` is the third TOML schema. Schemas: [`VIBEVM-SPEC.md` §7](../VIBEVM-SPEC.md), Rust source: [`crates/vibe-core/src/manifest/`](../crates/vibe-core/src/manifest).

### mirror

Transparent fallback URL for a registry. `[[mirror]] of = "<name>" url = "<alt>" priority = N` adds an alternative source for the named registry; `of = "*"` matches any. The lockfile records the *canonical* URL only — mirror identity does not leak to lockfile. Runtime fallback chain lands in M1.6 (Phase B); schema is wired today. [PROP-002 §2.3](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#mirror).

### materialised dependency

A resolved package's content as it lands on disk: its published tree, copied verbatim into a slot under [`vibedeps/`](#vibedeps). A materialised package *is* its subtree under its slot — there is no per-file write list. Spec: [PROP-009 §2.1](../spec/modules/vibe-workspace/PROP-009-loading-model.md#two-trees).

### `naming` (registry naming convention)

Per-registry rule for mapping a pkgref to a per-package repo name under the registry's org URL. Four values:

- `fqdn` (default): `org.vibevm.world/wal` → `<org>/org.vibevm.wal.git`. Repo name is `<group>.<name>` — collision-free, since `(group, name)` is unique. The convention vibevm's own registries use; the default since [PROP-008](../spec/modules/vibe-registry/PROP-008-qualified-naming.md).
- `kind-name`: `flow:wal` → `<org>/flow-wal.git`. The pre-PROP-008 default; kept for registries that have not adopted `group`.
- `name`: `flow:wal` → `<org>/wal.git`. Legal when names are globally unique across the registry.
- `kind/name`: `flow:wal` → `<org>/flow/wal.git`. Requires host support for nested repo paths.

A property of the registry, not a global CLI rule. [PROP-002 §2.2](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model), [PROP-008 §2.5](../spec/modules/vibe-registry/PROP-008-qualified-naming.md).

### override

`[[override]]` entry in `vibe.toml`. Surgical pin that bypasses the registry layer entirely for a specific pkgref — `vibe install <pkgref>` resolves through the override's `source_url` / `ref` directly. Lockfile entry carries `overridden = true`. Use case: pinning a fork during an upstream PR, internal forks of public packages. [PROP-002 §2.4](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#override).

### pkgref (package reference)

A package reference: `[<kind>:][<group>/]<name>[@<version>]`. The qualified `<group>/<name>` form is what manifests and the lockfile store; the short `<name>` form is CLI-only sugar, resolved to the qualified form via the package index at the CLI input boundary ([PROP-008 §2.6](../spec/modules/vibe-registry/PROP-008-qualified-naming.md)). Variants:

- `org.vibevm.world/wal` — qualified; resolved exactly.
- `flow:org.vibevm.world/wal` — qualified, kind-prefixed; `kind` is validated against the manifest after resolution.
- `wal` — short form, CLI-only sugar; index-resolved.
- `wal@^0.3` — short form with a semver caret constraint.

`kind` is metadata, not identity ([PROP-008 §2.3](../spec/modules/vibe-registry/PROP-008-qualified-naming.md)); a short-name collision is a `group` collision, never a `kind` one. Type: [`PackageRef`](../crates/vibe-core/src/package_ref.rs). Defined in [`VIBEVM-SPEC.md` §7.1](../VIBEVM-SPEC.md), [PROP-008 §2.4](../spec/modules/vibe-registry/PROP-008-qualified-naming.md).

### plan (install pipeline stage)

Stage 3 of `vibe install`. After resolve + fetch, before confirm. Under the [loading model](loading-model.md) the plan's unit is **the set of packages to materialise into `vibedeps/` plus the boot artifacts to regenerate** — not a per-file write list ([PROP-009 §2.7](../spec/modules/vibe-workspace/PROP-009-loading-model.md#install)). Plan-time validation also classifies every entry-point node's `<vibevm>` instruction-file block; a malformed block aborts the operation. Output: [`InstallPlan`](../crates/vibe-install/src/lib.rs).

### priority (registry / mirror)

The `[[registry]]` array order is priority order — first registry whose `GitPackageRegistry::resolve` succeeds wins. Within a registry, `[[mirror]]` entries try in `priority` ascending order before the canonical URL. PROP-002 §2.2 (registries), §2.3 (mirrors).

### PROP

Project Proposal — a binding architectural decision document. Lives under `spec/common/` (cross-cutting) or `spec/modules/<crate>/` (subsystem-specific). PROP-000 is the foundation; subsequent PROPs assume it. PROP-001 is the git-backend decision; PROP-002 is the decentralized-registry refactor. New PROPs require explicit owner approval.

### registry

A git-hosted organization URL with one repository per package underneath. Modern (per-package) form; the legacy single-repo monorepo form lives only in M1.1-shipping consumers until they migrate. [PROP-002 §2.2](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-model).

### repo (per-package)

A git repository hosting one vibevm package. Under a registry's organization URL, named per the registry's `naming` convention. Versions are git tags (`v<semver>`); content lives at the repo root (no per-version subdirectory). [PROP-002 §2.5](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#layout).

### resolve (install pipeline stage)

Stage 1 of `vibe install`. The depsolver expands user-typed roots into the full transitive graph; per-pkgref version pick happens against the configured registries / overrides. Output: [`ResolvedGraph`](../crates/vibe-resolver/src/lib.rs).

### root dependency

A package the user *directly* asked for, as opposed to a transitive dep the solver pulled in. `vibe.lock`'s `[meta].root_dependencies` records them. `vibe uninstall <root>` works; `vibe uninstall <transitive>` is rejected — transitives are managed by the solver, not by direct user action.

### `source_url`

URL the package's content was fetched from on the install that produced this lockfile entry. **Informational** — package identity does not depend on it. Mirror-switching, host-migration, and override pins all change `source_url` without changing identity.

### `source_ref`

Git ref the content was fetched at. Typically `v<version>` for per-package registries; the override's ref for `[[override]]`-resolved entries; `None` for non-git sources. Recorded per-`[[package]]` in `vibe.lock`.

### `transitive` (dep)

A dep the solver pulled in because some other dep declared it, not because the user typed it on the command line. Tracked by `LockedPackage.dependencies` (resolved exact-version pin) and **not** in `[meta].root_dependencies`.

### user-owned (file)

A node's **authored `spec/` tree** — written only by the node's author, never by `vibe`. The C++-`#include` rule of the [loading model](loading-model.md): installing a dependency never edits authored content. A dependency's content lands instead in the separate [`vibedeps/`](#vibedeps) tree. The conventional user-owned boot files `spec/boot/00-core.md` and `spec/boot/90-user.md`, plus `spec/WAL.md`, are part of the authored `spec/`; `vibe` references them in the [computed boot sequence](#computed-boot-sequence) but never rewrites them. The generated [boot artifacts](#boot-artifacts) `INLINE.md` / `INDEX.md`, by contrast, are vibevm-owned. Spec: [PROP-009 §2.1](../spec/modules/vibe-workspace/PROP-009-loading-model.md#two-trees).

### `[package]` (manifest table)

The table in a node's `vibe.toml` that marks it as a publishable artifact. Present at the root of every per-package repo and at every `<root>/<kind>/<name>/v<version>/` directory in a M0 / fixture-shape registry. A node carries `[package]` XOR `[project]`. Schema: [`VIBEVM-SPEC.md` §7.3](../VIBEVM-SPEC.md), Rust source: [`crates/vibe-core/src/manifest/package.rs`](../crates/vibe-core/src/manifest/package.rs).

### vibe.lock

The project lockfile. Schema v5 today; an older schema version is rejected, not migrated. Reference: [`docs/lockfile-format.md`](lockfile-format.md).

### vibe.toml

The single manifest file every vibevm node carries. Its role is set by which tables it carries: `[project]` (a non-publishable consumer) XOR `[package]` (a publishable artifact); `[workspace]` composes with either or neither. Other sections: `[active]`, `[llm]`, `[[registry]]`, `[[mirror]]`, `[[override]]`, `[requires.packages]`, `[origin]`. Schema: [`VIBEVM-SPEC.md` §7.5](../VIBEVM-SPEC.md), Rust source: [`crates/vibe-core/src/manifest/project.rs`](../crates/vibe-core/src/manifest/project.rs).

### `vibedeps/`

The materialised-dependency tree at the **absolute workspace root**, written only by `vibe` and committed to git. One slot per resolved package — `vibedeps/<kind>-<name>/<version>/` — holding that package's published tree verbatim. Physically separate from any node's authored `spec/`, so installing a dependency never modifies authored content. Spec: [PROP-009 §2.1](../spec/modules/vibe-workspace/PROP-009-loading-model.md#two-trees).

### `<vibevm>` block

The single delimited region vibevm owns inside each agent instruction file (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`) — bounded by the literal bare tags `<vibevm>` and `</vibevm>`, each alone on its own line. `vibe` writes only between the markers (the boot redirect); everything outside is preserved verbatim, since those files are a shared contact surface. Exactly one block per file; a malformed file (not exactly one ordered pair) is a hard error the user repairs by hand. Spec: [PROP-012](../spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md).

### `vibevm`

The project. The CLI binary it produces is `vibe`.

### WAL (Write-Ahead Log)

Two distinct meanings:

1. The **flow** package `flow:wal` — a discipline module (the "log every session's intent before committing" protocol).
2. The **file** `spec/WAL.md` at the root of every vibevm project — a checkpoint of current project state, rewritten each session, not appended. The structural counterpart of the flow.

Both come from book chapter 4 ("the discipline of writing what you intend before doing it"). The flow installs the protocol; the file holds the live state.

---

## Anti-vocabulary

Words used in adjacent ecosystems that we deliberately do **not** use, with their vibevm equivalents:

| Don't say | Say | Why |
| --- | --- | --- |
| "lifecycle" | the relevant install / build / sync stage | Maven-ism. Vibevm has graphs, not lifecycles. |
| "phase" | install stage / build node | Same — Maven baggage. |
| "goal" | task graph node | Same. |
| "plugin" | package | "Plugin" is a passable synonym for `package` in casual use; never use it for "code that contributes graph nodes" — that's a v2 contribution model that has its own name when it ships. |
| "module" | crate (Rust) / spec module (`spec/modules/<name>/`) | "Module" overloads. Use one of the two specific forms. |
| "vendor" (verb, for packages) | mirror, or `vibe vendor` (when M1.6 ships) | "Vendoring" connotes copy-into-tree; we use mirror for transparent alternative sources. |

Vocabulary lock pinned in [`spec/WAL.md`](../spec/WAL.md) §Constraints.

---

## Related

- [`VIBEVM-SPEC.md` §15`](../VIBEVM-SPEC.md) — the canonical glossary the spec ships with; this page expands on it.
- [`PROP-000`](../spec/common/PROP-000.md) — foundational decisions; many terms originate here.
- [`docs/architecture.md`](architecture.md) — for the bigger picture of how these terms relate.
