# PROP-036: `vibe tree` — the spec-tree analyzer {#root}

**Status:** DRAFT — requirements, 2026-07-15 (owner-requested). Governs the
`vibe tree` command in `crates/vibe-cli`. Written in the post-rename link
vocabulary (PROP-035): two link types, `static` and `dynamic`.
**Related:** [PROP-009 §2.4](../vibe-workspace/PROP-009-loading-model.md#inclusion-types)
(the `static`/`dynamic` link types + `when`), [PROP-009 §2.3](../vibe-workspace/PROP-009-loading-model.md#artifacts)
(`STATIC.md` / `INDEX.md`), [PROP-034](../vibe-workspace/PROP-034-transitive-links-boot-graph.md)
(`static-transitive`), [PROP-035](../vibe-workspace/PROP-035-spec-compiler.md)
(`@spec` in-place uses, `STATIC.md` markers), [PROP-002 §lockfile](../vibe-registry/PROP-002-decentralized-registry.md)
(the resolved graph). Plan: [`PACKAGE-TREE-PLAN-v0.1`](../../terraforms/PACKAGE-TREE-PLAN-v0.1.md).
**Non-goal (deferred):** the runtime "what the agent actually loaded" skill and a
GUI client — a future `tool:org.vibevm.core/package-tree` (§7).

---

## 1. Motivation {#motivation}

A project's boot is composed by `vibe` from a dependency closure (PROP-009):
some packages are compiled verbatim into `STATIC.md`, others are read by
reference from `INDEX.md`, some carry an OS `when`, and `static-transitive`
edges (PROP-034) silently promote whole subtrees. Today a human cannot see this
at a glance — **what is connected, and how does it load?** `vibe tree` answers
that algorithmically: it renders the resolved package tree annotated with the
*effective* load type and the flags that explain it, and emits the same data as
JSON for downstream tools.

---

## 2. Decisions {#decisions}

### 2.1 The command {#command}

`vibe tree` is a **read-only** analyzer over the current project. It mutates
nothing (REQ: never writes to the tree, manifests, lockfile, or artifacts). It
operates on the workspace discovered from `--path` (default `.`), reading the
committed `vibe.lock`, the node manifests, and the generated boot artifacts.
Three output surfaces:

- **interactive TUI** — the default on a tty (§2.11);
- **`--json`** — the machine surface (§2.7), the same data a GUI consumes;
- **plain** — a static ASCII tree when not on a tty, or under `--plain`.

`--json` and `--plain` never launch the TUI. The command is `Tree(TreeArgs)` in
the CLI surface; `--json`/`--quiet` are the global flags (never redeclared).

### 2.2 Row semantics — the columns {#columns}

Each package is one row. Columns, left to right (REQ: this order):

1. **name** — the tree column: `group/name`, drawn with indentation + branch
   glyphs + an expand/collapse indicator when the node has children.
2. **load** — the *effective* load type (§2.3): `static`, `dynamic`, or `none`.
3. **T** (transitive) — a checkbox: the effective `static` was forced by a
   `static-transitive` ancestor, not the package's own declaration (§2.4).
4. **C** (condition) — a checkbox: the boot entry carries a `when` (§2.5).
5. **S** (STATIC.md) — a checkbox: the package physically contributes a block to
   `STATIC.md`.

`T`, `C`, `S` are the "checkbox" cluster; `load` is a value column. Detail beyond
a checkbox (the full `when` text, the source path) is shown on demand (§2.11),
never crammed into the row.

### 2.3 Effective load type {#effective-load}

The `load` value is the lane a package's boot snippet **actually lands in**, read
from the committed artifacts (REQ):

- `static` — the package appears as a `<!-- vibe:static {origin} … -->`
  contribution in `STATIC.md` (`origin = group/name`).
- `dynamic` — the package's boot file appears as an `[[entry]]` in `INDEX.md`.
- `none` — the package ships no `[boot_snippet]`, so it contributes to neither
  lane (e.g. a content-minimal family aggregator, PROP-028).

> **Decision — read the effective type from the artifacts, not a fresh recompute.**
> - **Why:** the committed `STATIC.md` / `INDEX.md` are exactly what an agent
>   reads at boot; the tool's job is to show that reality ("what is actually
>   loaded"). A stale artifact is a fact the human needs, not one to hide.
> - **Considered and rejected:** recomputing `EffectiveBoot` fresh every run —
>   shows what *should* be, masking drift the tool exists to reveal.
> - **When to revisit:** if the artifacts stop being committed (generated
>   on-demand), the source of truth moves to the recompute; until then the
>   committed lane is canonical, cross-checked by §2.10.

### 2.4 The transitive flag {#transitive-flag}

`T` is set (REQ) iff the package's effective type is `static` **and** that
static-ness was inherited from an ancestor edge declared `static-transitive`
(PROP-034), rather than from the package's own consumer-declared `link`, its own
`[boot_snippet].link` suggestion, or a direct `static` edge. The
`static-transitive` *declarer* itself carries `T = false` (its static-ness is its
own declaration); every member of its closure that is not otherwise static
carries `T = true`.

### 2.5 The condition flag {#condition-flag}

`C` is set (REQ) iff the package's boot entry carries a `when` condition
(PROP-009 §2.4; wire form `os:<name>`). A `when` forces the entry `dynamic`
regardless of link (so `C = true` implies `load = dynamic`). The full condition
text is surfaced only in the detail view (§2.11) and the JSON (§2.7), never in
the row.

### 2.6 The STATIC.md size indicator {#static-size}

The status line shows the size of the statically-compiled lane: the byte count
and line count of `STATIC.md` (REQ). This is the "how much am I loading verbatim
every session" budget the human watches.

### 2.7 JSON output {#json}

`vibe tree --json` emits one object, the same data model the TUI renders, valid
against the shipped schema (REQ: `schema_version = 1`). The envelope follows the
CLI convention (`{"ok": true, "command": "tree", …}`). The model carries: the
`project` context, the declared `roots`, the `packages` array (each with `load`
{`type`, `transitive`, `declared`, `origin`, `in_static_md`, `in_index_md`,
`boot_path`} and `condition`), the two boot lanes under `boot`
(`static_md`/`index_md`, with the lane sizes), the collected `in_place_specs`
(§2.9), and `diagnostics` (§2.10). Display state (mode, ordering, tab, selection)
is TUI-only and is **not** in the JSON. Schema home:
`crates/vibe-cli/resources/package-tree.schema.v1.json`.

### 2.8 STATIC.md decompilation {#static-decompile}

`vibe tree` decompiles `STATIC.md` into its contributions (REQ): each
`<!-- vibe:static {origin} — {path} -->` marker opens a region running to the
next marker or EOF; the region yields `origin` (the source `group/name` or host
rel-path) and `path` (the workspace-relative source file). Nested
`<!-- embed: {addr} -->` … `<!-- /embed: {addr} -->` pairs within a region are
attributed as embed spans. This is a **dedicated** decompiler for the on-disk
`vibe:static` open-marker format — it is not `vibe_spec::decompile()`, which
parses the distinct `vibe:begin`/`vibe:end` compiler format and returns empty on
`STATIC.md`.

### 2.9 In-place `@spec` collection {#in-place-specs}

`vibe tree` collects the in-place boot-lane spec markers (REQ): `@spec://` uses
and `#use` / `#embed` / `#source` directives, via the canonical fence-aware
`vibe_spec::Directives::parse`. A bare `spec://` (no `@`) is discretionary and is
**not** collected. (Out of scope: the `#[spec(...)]` code-traceability surface —
that is PROP-014.)

### 2.10 Diagnostics {#diagnostics}

`vibe tree` reports, non-fatally (REQ: never aborts rendering on these):

- **stale-artifacts** — the committed lanes disagree with a fresh `EffectiveBoot`
  recompute (the tree needs `vibe reinstall`).
- **root-drift** — `vibe.lock` `meta.root_dependencies` disagrees with the root
  `vibe.toml` `[requires.packages]` (the lock is behind).

Each diagnostic carries a severity, a stable code, a message, and an optional
locator.

### 2.11 The interactive TUI {#tui}

On a tty (default), `vibe tree` is an interactive pseudographic browser (REQ).
Contract:

- **Navigation:** `↑`/`↓` move the selection (the table scrolls to keep it
  visible; the selected row is highlighted); `←`/`→` pan horizontally when the
  tree is wider than the viewport.
- **Folding:** `Space` folds/unfolds the selected node; `F` folds/unfolds the
  whole tree.
- **Detail:** `Enter` opens a modal showing the row's full detail vertically
  (name, group, version, kind, load type, transitive + why, condition full text,
  STATIC.md membership, source, content hash, dependencies, boot file);
  `Esc` closes it; `q` quits.
- **Ordering** (`n`, shown in the status line): **topological** (the analysis
  order, default) ↔ **alphabetical**.
- **Display mode** (`x`, cycles): (a) all-together tree; (b) two stacked
  sub-tables `static dependencies` / `dynamic dependencies` (a header line each);
  (c) two tabs `Static` / `Dynamic`. `t` swaps the static/dynamic priority
  (which comes first) in (b) and (c); `TAB` and `[` / `]` switch tabs in (c).
- **Status line:** current ordering · current display mode · the `STATIC.md` size
  indicator (§2.6).
- **Fallback:** non-tty and `--plain` render a static ASCII tree; `--json` the
  JSON — neither enters interactive mode.

### 2.12 The graph is a DAG {#dag}

The dependency graph has diamonds (a shared package reached by several parents).
`vibe tree` renders each package under each parent, marks a re-occurrence with a
trailing `(*)` and does not re-expand it, and cycle-guards the walk on the
package's qualified `group/name` (REQ). The flat display modes (§2.11) collapse
the DAG to one row per package.

---

## 3. Data sources {#data-sources}

`vibe tree` joins, using the canonical parsers (REQ — no re-implemented format
readers where a `vibe-*` crate already parses it):

- **graph** — `vibe.lock` (`vibe_core::manifest::Lockfile`): roots from
  `meta.root_dependencies`, edges from each `LockedPackage.dependencies`.
- **links** — the node manifests (`vibe_core` `Requires`): consumer `declared_link`
  + the target's `[boot_snippet]` suggested link + `when`.
- **effective lanes + sizes** — the committed `spec/boot/STATIC.md` +
  `spec/boot/INDEX.md`.
- **cross-check** — `vibe_workspace` `EffectiveBoot` (for the stale-artifacts
  diagnostic).
- **in-place specs** — `vibe_spec::Directives::parse`.

---

## 4. Non-goals {#non-goals}

- **The runtime skill / prompt** — inferring what the agent *actually* loaded at
  runtime (the `loading spec://…` convention, `.vibe/` logging, multi-agency) is
  deferred to `tool:org.vibevm.core/package-tree`.
- **A GUI client** — deferred to the same future package; the `--json` schema is
  its contract.
- **Spec-graph validation** — `vibe tree` attributes and reports; it does not
  validate `spec://` targets.
- **Mutation** — never; see §2.1.
