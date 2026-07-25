# PROP-036: `vibe tree` — the spec-tree analyzer {#root}

<status stage="impl" state="done" comment="C 2026-07-25: PACKAGE-TREE-PLAN executed; the analyzer ships (tree --json against package-tree.schema.v1.json, -t live)"/>

##status-line **Status: IMPLEMENTED** (requirements authored 2026-07-15 at the owner's
request; the PACKAGE-TREE-PLAN executed against them, verified against the tree
2026-07-25 by the spec-actualization campaign — `vibe tree --json` validates
against the shipped `package-tree.schema.v1.json` per its own `--help`, and `-t`
is live). Governs the `vibe tree` command in `crates/vibe-cli`. Written in the
post-rename link vocabulary (PROP-035): two link types, `static` and `dynamic`. @impl/done

##related **Related:** [PROP-009 §2.4](../vibe-workspace/PROP-009-loading-model.md#inclusion-types)
(the `static`/`dynamic` link types + `when`), [PROP-009 §2.3](../vibe-workspace/PROP-009-loading-model.md#artifacts)
(`STATIC.md` / `INDEX.md`), [PROP-034](../vibe-workspace/PROP-034-transitive-links-boot-graph.md)
(`static-transitive`), [PROP-035](../vibe-workspace/PROP-035-spec-compiler.md)
(`@spec` in-place uses, `STATIC.md` markers), [PROP-002 §lockfile](../vibe-registry/PROP-002-decentralized-registry.md)
(the resolved graph). Plan: [`PACKAGE-TREE-PLAN-v0.1`](../../../legacy-spec/terraforms/PACKAGE-TREE-PLAN-v0.1.md). @spec/done

##non-goal-line **Non-goal (deferred):** the runtime "what the agent actually loaded" skill and a
GUI client — a future `tool:org.vibevm.core/package-tree` (§7). @spec/done

---

## 1. Motivation {#motivation}

- ##boot-composition A project's boot is composed by `vibe` from a dependency closure (PROP-009):
  some packages are compiled verbatim into `STATIC.md`, others are read by
  reference from `INDEX.md`, some carry an OS `when`, and `static-transitive`
  edges (PROP-034) silently promote whole subtrees. @spec/done
- ##visibility-gap Today a human cannot see this
  at a glance — **what is connected, and how does it load?** @spec/done
- ##TREE-ANSWER `vibe tree` answers
  that algorithmically: it renders the resolved package tree annotated with the
  *effective* load type and the flags that explain it, and emits the same data as
  JSON for downstream tools. @spec/done

---

## 2. Decisions {#decisions}

### 2.1 The command {#command}

- ##TREE-READ-ONLY `vibe tree` is a **read-only** analyzer over the current project. It mutates
  nothing (REQ: never writes to the tree, manifests, lockfile, or artifacts). @spec/done
- ##TREE-INPUTS It
  operates on the workspace discovered from `--path` (default `.`), reading the
  committed `vibe.lock`, the node manifests, and the generated boot artifacts. @spec/done

##output-surfaces-lead Three output surfaces: @spec/done

- ##OUT-TUI **interactive TUI** — the default on a tty (§2.11); @spec/done
- ##OUT-JSON **`--json`** — the machine surface (§2.7), the same data a GUI consumes; @spec/done
- ##OUT-PLAIN **plain** — a static ASCII tree when not on a tty, or under `--plain`. @spec/done

- ##NO-TUI-UNDER-FLAGS `--json` and `--plain` never launch the TUI. @spec/done
- ##CLI-SURFACE The command is `Tree(TreeArgs)` in
  the CLI surface; `--json`/`--quiet` are the global flags (never redeclared). @spec/done

### 2.2 Row semantics — the columns {#columns}

##ROW-PER-PACKAGE Each package is one row. Columns, left to right (REQ: this order): @spec/done

1. ##COL-NAME **name** — the tree column: `group/name`, drawn with indentation + branch
   glyphs + an expand/collapse indicator when the node has children. @spec/done
2. ##COL-LOAD **load** — the *effective* load type (§2.3): `static`, `dynamic`, or `none`. @spec/done
3. ##COL-TRANSITIVE **T** (transitive) — a checkbox: the effective `static` was forced by a
   `static-transitive` ancestor, not the package's own declaration (§2.4). @spec/done
4. ##COL-CONDITION **C** (condition) — a checkbox: the boot entry carries a `when` (§2.5). @spec/done
5. ##COL-STATIC **S** (STATIC.md) — a checkbox: the package physically contributes a block to
   `STATIC.md`. @spec/done

- ##checkbox-cluster `T`, `C`, `S` are the "checkbox" cluster; `load` is a value column. @spec/done
- ##DETAIL-ON-DEMAND Detail beyond
  a checkbox (the full `when` text, the source path) is shown on demand (§2.11),
  never crammed into the row. @spec/done

### 2.3 Effective load type {#effective-load}

##EFFECTIVE-FROM-ARTIFACTS The `load` value is the lane a package's boot snippet **actually lands in**, read
from the committed artifacts (REQ): @spec/done

- ##LOAD-STATIC `static` — the package appears as a `<!-- vibe:static {origin} … -->`
  contribution in `STATIC.md` (`origin = group/name`). @spec/done
- ##LOAD-DYNAMIC `dynamic` — the package's boot file appears as an `[[entry]]` in `INDEX.md`. @spec/done
- ##LOAD-NONE `none` — the package ships no `[boot_snippet]`, so it contributes to neither
  lane (e.g. a content-minimal family aggregator, PROP-028). @spec/done

##DECISION-ARTIFACTS-CANONICAL **Decision — read the effective type from the artifacts, not a fresh recompute.** @spec/done

- ##decision-artifacts-why **Why:** the committed `STATIC.md` / `INDEX.md` are exactly what an agent
  reads at boot; the tool's job is to show that reality ("what is actually
  loaded"). A stale artifact is a fact the human needs, not one to hide. @spec/done
- ##decision-artifacts-rejected **Considered and rejected:** recomputing `EffectiveBoot` fresh every run —
  shows what *should* be, masking drift the tool exists to reveal. @spec/done
- ##decision-artifacts-revisit **When to revisit:** if the artifacts stop being committed (generated
  on-demand), the source of truth moves to the recompute; until then the
  committed lane is canonical, cross-checked by §2.10. @spec/done

### 2.4 The transitive flag {#transitive-flag}

- ##TRANSITIVE-FLAG-DEF `T` is set (REQ) iff the package's effective type is `static` **and** that
  static-ness was inherited from an ancestor edge declared `static-transitive`
  (PROP-034), rather than from the package's own consumer-declared `link`, its own
  `[boot_snippet].link` suggestion, or a direct `static` edge. @spec/done
- ##TRANSITIVE-DECLARER The
  `static-transitive` *declarer* itself carries `T = false` (its static-ness is its
  own declaration); every member of its closure that is not otherwise static
  carries `T = true`. @spec/done

### 2.5 The condition flag {#condition-flag}

- ##CONDITION-FLAG-DEF `C` is set (REQ) iff the package's boot entry carries a `when` condition
  (PROP-009 §2.4; wire form `os:<name>`). @spec/done
- ##WHEN-FORCES-DYNAMIC A `when` forces the entry `dynamic`
  regardless of link (so `C = true` implies `load = dynamic`). @spec/done
- ##CONDITION-DETAIL-ONLY The full condition
  text is surfaced only in the detail view (§2.11) and the JSON (§2.7), never in
  the row. @spec/done

### 2.6 The STATIC.md size indicator {#static-size}

- ##STATIC-SIZE-INDICATOR The status line shows the size of the statically-compiled lane: the byte count
  and line count of `STATIC.md` (REQ). @spec/done
- ##static-size-purpose This is the "how much am I loading verbatim
  every session" budget the human watches. @spec/done

### 2.7 JSON output {#json}

- ##JSON-CONTRACT `vibe tree --json` emits one object, the same data model the TUI renders, valid
  against the shipped schema (REQ: `schema_version = 1`). @spec/done
- ##JSON-ENVELOPE The envelope follows the
  CLI convention (`{"ok": true, "command": "tree", …}`). @spec/done
- ##JSON-MODEL The model carries: the
  `project` context, the declared `roots`, the `packages` array (each with `load`
  {`type`, `transitive`, `declared`, `origin`, `in_static_md`, `in_index_md`,
  `boot_path`} and `condition`), the two boot lanes under `boot`
  (`static_md`/`index_md`, with the lane sizes), the collected `in_place_specs`
  (§2.9), and `diagnostics` (§2.10). @spec/done
- ##JSON-NO-DISPLAY-STATE Display state (mode, ordering, tab, selection)
  is TUI-only and is **not** in the JSON. @spec/done
- ##JSON-SCHEMA-HOME Schema home:
  `crates/vibe-cli/resources/package-tree.schema.v1.json`. @spec/done

### 2.8 STATIC.md decompilation {#static-decompile}

- ##STATIC-DECOMPILE `vibe tree` decompiles `STATIC.md` into its contributions (REQ): each
  `<!-- vibe:static {origin} — {path} -->` marker opens a region running to the
  next marker or EOF; the region yields `origin` (the source `group/name` or host
  rel-path) and `path` (the workspace-relative source file). @spec/done
- ##EMBED-SPANS Nested
  `<!-- embed: {addr} -->` … `<!-- /embed: {addr} -->` pairs within a region are
  attributed as embed spans. @spec/done
- ##DEDICATED-DECOMPILER This is a **dedicated** decompiler for the on-disk
  `vibe:static` open-marker format — it is not `vibe_spec::decompile()`, which
  parses the distinct `vibe:begin`/`vibe:end` compiler format and returns empty on
  `STATIC.md`. @spec/done

### 2.9 In-place `@spec` collection {#in-place-specs}

- ##INPLACE-COLLECTION `vibe tree` collects the in-place boot-lane spec markers (REQ): `@spec://` uses
  and `#use` / `#embed` / `#source` directives, via the canonical fence-aware
  `vibe_spec::Directives::parse`. @spec/done
- ##BARE-SPEC-SKIPPED A bare `spec://` (no `@`) is discretionary and is
  **not** collected. @spec/done
- ##inplace-oos (Out of scope: the `#[spec(...)]` code-traceability surface —
  that is PROP-014.) @spec/done

### 2.10 Diagnostics {#diagnostics}

##DIAG-NON-FATAL `vibe tree` reports, non-fatally (REQ: never aborts rendering on these): @spec/done

- ##DIAG-STALE-ARTIFACTS **stale-artifacts** — the committed lanes disagree with a fresh `EffectiveBoot`
  recompute (the tree needs `vibe reinstall`). @spec/done
- ##DIAG-ROOT-DRIFT **root-drift** — `vibe.lock` `meta.root_dependencies` disagrees with the root
  `vibe.toml` `[requires.packages]` (the lock is behind). @spec/done

##DIAG-SHAPE Each diagnostic carries a severity, a stable code, a message, and an optional
locator. @spec/done

### 2.11 The interactive TUI {#tui}

##TUI-DEFAULT On a tty (default), `vibe tree` is an interactive pseudographic browser (REQ).
Contract: @spec/done

- ##TUI-NAVIGATION **Navigation:** `↑`/`↓` move the selection (the table scrolls to keep it
  visible; the selected row is highlighted); `←`/`→` pan horizontally when the
  tree is wider than the viewport. @spec/done
- ##TUI-KEYMAP-SUPERSEDED **The key bindings sketched in this section are superseded.** This §2.11 is
  the analyzer-era sketch; the shipped keymap is [PROP-037](PROP-037-tree-tui.md)
  Spec 2's F-key scheme, and it is normative there — this section names the
  *capabilities*, never the keys. @impl/done
- ##TUI-FOLDING **Folding:** the selected node folds and unfolds (shipped as the
  `fold.toggle` action, [PROP-037 §13.5](PROP-037-tree-tui.md#actions)). The
  whole-tree fold sketched here was **not carried into Spec 2 and does not
  ship** — the action catalogue has `fold.toggle` only. @impl/done
- ##TUI-DETAIL **Detail:** `Enter` opens a modal showing the row's full detail vertically
  (name, group, version, kind, load type, transitive + why, condition full text,
  STATIC.md membership, source, content hash, dependencies, boot file);
  `Esc` closes it. Quitting is `Esc` plus a confirm dialog
  ([PROP-037 §7.4](PROP-037-tree-tui.md)), not the bare `q` this sketch assumed. @impl/done
- ##TUI-ORDERING **Ordering** (shown in the status line): **topological** (the analysis
  order, default) ↔ **alphabetical**. Chosen from the F2 sort menu
  ([PROP-037 §7.2](PROP-037-tree-tui.md), which replaces any bare mode-cycle key). @impl/done
- ##TUI-DISPLAY-MODES **Display mode:** (a) all-together tree; (b) two stacked
  sub-tables `static dependencies` / `dynamic dependencies` (a header line each);
  (c) two tabs `Static` / `Dynamic`, with a swappable static/dynamic priority in
  (b) and (c). Modes are chosen from the F3 menu and tabs switch with
  `Shift`+arrows ([PROP-037 §4.4 / §5.3](PROP-037-tree-tui.md)). @impl/done
- ##TUI-STATUS-LINE **Status line:** current ordering · current display mode · the `STATIC.md` size
  indicator (§2.6). @spec/done
- ##TUI-FALLBACK **Fallback:** non-tty and `--plain` render a static ASCII tree; `--json` the
  JSON — neither enters interactive mode. @spec/done

### 2.12 The graph is a DAG {#dag}

- ##dag-diamonds The dependency graph has diamonds (a shared package reached by several parents). @spec/done
- ##DAG-RENDERING `vibe tree` renders each package under each parent, marks a re-occurrence with a
  trailing `(*)` and does not re-expand it, and cycle-guards the walk on the
  package's qualified `group/name` (REQ). @spec/done
- ##DAG-FLAT-COLLAPSE The flat display modes (§2.11) collapse
  the DAG to one row per package. @spec/done

### 2.13 Project resolution — VibeTree works from anywhere {#project-resolution}

##project-needed `vibe tree` shows a project's tree, so it needs one — but a GUI launcher
(`VibeTree.exe` / a Start-menu shortcut) or an arbitrary shell may sit
outside any project. The launchers live in the `vibevm-term` products repo with
vibeterm and vibeframe ([PROP-019 §STEP-VIBE-ONLY](../../common/PROP-019-version-manager.md)),
and are governed there — this host contract only specifies what `vibe tree` does
when invoked from outside a project. @impl/done

##RESOLUTION-ORDER Resolution order for the **human** surfaces (the TUI and
`-t`; **not** `--json`, a scripting surface resolved strictly from `--path`)
(REQ): @spec/done

1. ##RES-GIVEN-PATH **The given path** — `--path` (default: cwd, walked up for `vibe.toml`). On
   success it is **recorded as the last project** (`vibe.tree.last-project`, an
   L1 setting), so a later context-free launch reopens it. An explicit `--path`
   that is not a project is a hard error — never silently redirected. @spec/done
2. ##RES-LAST-PROJECT **The remembered last project** — when the cwd is not a project and no
   explicit `--path` was given, the recorded `last-project` opens (if it is still
   a project). @spec/done
3. ##RES-FOLDER-PICKER **A folder picker** — a `-t` (VibeTree / GUI) launch with neither of the above
   opens a native folder chooser; the pick is recorded as the last project.
   Cancelling is a clean no-op (no error dialog), never a failure. @spec/done

##console-fallback A console launch (no `-t`) with neither a cwd project nor a memory keeps the
original `run vibe init` guidance. @spec/done

---

## 3. Data sources {#data-sources}

##CANONICAL-PARSERS `vibe tree` joins, using the canonical parsers (REQ — no re-implemented format
readers where a `vibe-*` crate already parses it): @spec/done

- ##SRC-GRAPH **graph** — `vibe.lock` (`vibe_core::manifest::Lockfile`): roots from
  `meta.root_dependencies`, edges from each `LockedPackage.dependencies`. @spec/done
- ##SRC-LINKS **links** — the node manifests (`vibe_core` `Requires`): consumer
  `declared_link` + the target's `[boot_snippet]` suggested link + `when`. @spec/done
- ##SRC-LANES **effective lanes + sizes** — the committed `spec/boot/STATIC.md` +
  `spec/boot/INDEX.md`. @spec/done
- ##SRC-CROSS-CHECK **cross-check** — `vibe_workspace` `EffectiveBoot` (for the stale-artifacts
  diagnostic). @spec/done
- ##SRC-INPLACE **in-place specs** — `vibe_spec::Directives::parse`. @spec/done

---

## 4. Non-goals {#non-goals}

- ##NG-RUNTIME-SKILL **The runtime skill / prompt** — inferring what the agent *actually* loaded at
  runtime (the `loading spec://…` convention, `.vibe/` logging, multi-agency) is
  deferred to `tool:org.vibevm.core/package-tree`. @spec/done
- ##NG-GUI-CLIENT **A GUI client** — deferred to the same future package; the `--json` schema is
  its contract. @spec/done
- ##NG-SPEC-VALIDATION **Spec-graph validation** — `vibe tree` attributes and reports; it does not
  validate `spec://` targets. @spec/done
- ##NG-MUTATION **Mutation** — never; see §2.1. @spec/done
