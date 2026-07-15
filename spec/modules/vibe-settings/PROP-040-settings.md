# PROP-040: the vibevm settings system — application/user preferences (`vibe-settings`) {#root}

**Status:** DRAFT — requirements, 2026-07-16 (owner-commissioned). The **contract** for a new crate
`vibe-settings`: a three-level, schema-first, introspectable store for **application/user
preferences** (Vibe Tree UI — palettes, glyphs, rendering tier, display mode, sort, tree shape, fold
state, future fonts/sizes; future vibe-app prefs) — programmatically drivable, **AIUI-ready** (surface
not built, §14). **Related:** meta-plan
[`spec/terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md`](../../terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md);
clean-room study [`spec/research/settings-system-vscode-idea.md`](../../research/settings-system-vscode-idea.md)
(deltas D-01..D-16); [PROP-037 §9](../vibe-cli/PROP-037-tree-tui.md#settings) (the `vibe tree` TUI —
first consumer; its ad-hoc `~/.vibe/tree` is subsumed); [PROP-041](PROP-041-settings-ui.md) (the TUI
settings surface); the `addressable-specs`, `qualified-naming`, `decision-records`, and `secrets-hygiene`
flows. **Format:** TOML. **Language:** keys/defaults English; values may be localised downstream.

This contract is deliberately **granular and addressable** (owner directive): every feature is its own
`{#anchor}` REQ cited by the code via `specmark`. A REQ is the unit of work. Each Δ from the study
(`spec/research/settings-system-vscode-idea.md` §7) maps to one or more REQs here.

---

## 1. Scope — application/user preferences, NOT project config {#scope}

REQ {#app-prefs-not-project}. The system stores **application/user preferences** — how vibevm's
surfaces *look and behave for this user* (the TUI's palette/glyph/tier/mode/sort/shape/fold; future
vibe-app prefs). It does **not** store project properties. `vibe.toml` (the vibe-PROJECT manifest —
package/deps/build, the `pom.xml` analogue, governed by [PROP-000 §4](../../common/PROP-000.md) and
the `Manifest`/`UserConfig` types) is a **separate subsystem** this contract does **not** extend or
mutate. The split mirrors IntelliJ `.idea/` (IDE settings) vs `pom.xml` (build), and VSCode
`.vscode/settings.json` (workspace UI) vs `package.json` (project).

REQ {#new-subsystem}. `vibe-settings` is a **new crate**, parallel to `vibe-core`'s project-config —
owning its own schema, loaders, resolver, and persistence. Project-config is reused only as
architectural reference (how layering is *not* done today), never as the object of extension.

REQ {#frontend-agnostic}. The crate is **frontend-agnostic** — zero rendering dependencies (no
`ratatui`/`crossterm`/GUI toolkit). Preferences are data + logic-on-data; any surface (the TUI, a
future GUI, the headless AIUI) reads them through the resolver (§5). This is what makes the AIUI and
the settings-UI (PROP-041) possible.

---

## 2. The three levels + precedence {#levels}

REQ {#three-levels}. Preferences resolve over **three file layers**, lowest-to-highest precedence:

- **L1 — user-machine** (`~/.vibe/`, e.g. `~/.vibe/settings.toml`): this user's global defaults on this
  machine (analogue: VSCode User settings; IntelliJ Application `options/*.xml`).
- **L2 — repo-shared** (`.vibe/settings.toml` inside the repo, **committed**): the team's preferences
  for this project (analogue: VSCode `.vscode/settings.json`; IntelliJ `.idea/` shared `*.xml`,
  `RoamingType.DEFAULT`).
- **L3 — user-project** (`.vibe/settings.local.toml`, **gitignored**): this user's fine-tuning for
  this specific project (analogue: IntelliJ `$WORKSPACE_FILE$` = `.idea/workspace.xml`, personal,
  `RoamingType.DISABLED`).

REQ {#precedence-law} (Δ-11, imperative 1). The precedence is a **law**, fixed in one place (this
section) and encoded in the binary, never ambiguous:

```
built-in default  ⊂  L1 user-machine  ⊂  L2 repo-shared  ⊂  L3 user-project  ⊂  CLI flag  ⊂  env var
```

A higher layer **overrides** a lower one per the merge semantics (§4). L3 wins among file layers;
`--set`/`--config` CLI flags and `VIBE_*` env vars override every file layer. This law is the direct
answer to the VSCode precedence-confusion pain (`spec/research` §4.1.1, issue #228983): there is one
documented order, machine-readable, and introspectable (§8).

REQ {#cli-layers-not-replaces} (Δ-11, imperative 10). A CLI `--config <file>` flag **adds a layer**
above the file layers (it does not replace them, à la Helix `-c` — `spec/research` §4.4.3); a `--set
key=value` flag sets one key in that top layer. Cascade shadows (a higher layer overriding a lower)
are **reported**, never silent (the `.editorconfig root=true` pitfall, §4.4.6).

---

## 3. File locations + roles {#locations}

REQ {#file-layout} (Δ-05, imperative 5). Exactly these files carry preferences:

| layer | path | committed? | role |
|---|---|---|---|
| L1 | `~/.vibe/settings.toml` | no (user home) | user's global defaults |
| L2 | `<repo>/.vibe/settings.toml` | **yes** | team's project preferences |
| L3 | `<repo>/.vibe/settings.local.toml` | **no** (gitignored) | user's per-project fine-tuning |

REQ {#role-marker}. Each file carries a header comment naming its role and precedence (e.g.
`# L2 — repo-shared (committed). Overrides L1; overridden by L3 and CLI/env.`), so a reader never
confuses layers (the `.idea` what-to-commit pain, §4.2.1).

REQ {#dotvibe-not-cache} (risk R1). `.vibe/` today is a cache dir (`.vibe/cache/...`,
[PROP-000 §13](../../common/PROP-000.md)). Preference files live at `.vibe/settings.toml` /
`.vibe/settings.local.toml` — siblings of `cache/`, never inside it. The layout is fixed by this
contract so cache-vs-prefs never collide.

REQ {#missing-is-default}. A missing or corrupt file falls back to defaults — **never a hard error**
(analogous to PROP-037 §9). A parse error is reported as a non-fatal diagnostic and the layer is
treated as absent.

---

## 4. Merge semantics {#merge}

REQ {#merge-algorithm} (Δ-01, imperative 3). Layers merge left-to-right (lowest → highest) by
**deep-merge** (`spec/research` §2.4 — the VSCode `ConfigurationModel.mergeContents` semantics,
re-implemented clean-room):

- **Scalars** (string/number/boolean): **last-wins** (the higher layer's value replaces).
- **Objects** (tables): **recursive deep-merge** — children merge by the same rules.
- **Arrays**: **replace, not concatenate** — a higher-layer array fully replaces the lower one (the
  non-obvious VSCode semantics; the dotnet #118204 trap is avoided by making this explicit and
  documented).

REQ {#merge-strategy-opt-in} (Δ-01). A schema key may declare a `merge` strategy for arrays:
`replace` (default) | `append` | `prepend` | `merge-by-key`. The default `replace` is the safe,
predictable choice; opt-in strategies are explicit per-key so no array is ever merged silently.

REQ {#null-semantics} (imperative; §4.3.4). `null` means **"explicitly unset — shadow the parent with
empty"**; deleting a key means **"fall back to the parent layer"**. The two are distinct and documented.

---

## 5. The resolver — `ResolvedPrefs` + inspect-API (AIUI-ready) {#resolver}

REQ {#resolved-prefs} (Δ-02). A single resolver entry point produces a **`ResolvedPrefs`** — the
composed view over `default + L1 + L2 + L3 (+ CLI/env)`, with **per-field provenance**. Consumers (the
TUI, the future vibe app, the AIUI) read resolved values through this one entry; no consumer reads a
raw layer.

REQ {#inspect} (Δ-02, Δ-12; `spec/research` §2.8). `inspect(key)` returns, for one key:

```
{ value,                  // the resolved value
  default,                // built-in default
  l1, l2, l3,             // per-layer values (None if the layer doesn't set it)
  cli, env,               // CLI/env overrides if any
  origin                  // which layer the resolved value came from (file:line where known)
}
```

This is the **key AIUI API** (the VSCode `IConfigurationValue<T>` shape, clean-room) — one operation
yields the effective value *and* which layer established it *and* whether a higher layer shadows it.
This is what IntelliJ lacks (`spec/research` §3.9 — three disconnected query paths, no unified
introspection); vibevm designs it in from day one.

REQ {#unified-introspection} (Δ-12). The schema is a **single typed, tagged, enumerable tree**:
`keys() → Iterator<KeyMeta>`, `get(path) → Value`, `set(path, value, layer)`. An agent (or test, or
CLI) can enumerate every setting and read/write it **without knowing service class names or parsing
files** — the introspection bottleneck of IntelliJ (§3.9) is avoided by construction.

REQ {#get-section}. `get(section)` returns a whole namespace as a typed value (e.g. all `tree.*`
prefs), mirroring VSCode's section access.

---

## 6. Schema + validation + deprecation {#schema}

REQ {#schema-first} (Δ-04, imperative 4). The preference surface is **schema-first**: every key is
declared with `type`, `default`, and metadata. Unknown keys (typos, retired names) produce a **loud
warning at boot and at `vibe prefs check`** — never a silent ignore (the VSCode JSON-schema-gap pain,
§4.1.5; the IntelliJ un-validated-XML pain).

REQ {#schema-fields}. A `KeyMeta` carries: `path` (dotted), `type` (bool/int/string/enum/array/table),
`default`, `description` (mandatory, non-empty), `scope` (§7), `applies` (§10), `merge` (§4),
`deprecated`/`replaced_by` (below), `restricted` (§11.3-equivalent for untrusted L2).

REQ {#deprecation} (Δ-08, imperative 8). A key may be `deprecated` with a `replaced_by` target. Boot
emits a warning naming the migration; `vibe prefs migrate` rewrites the file automatically. No
"greyed-out-in-JSON" non-migration (the VSCode §4.1.6 / IntelliJ PersistentStateComponent §4.2.5 pain).

REQ {#diff-from-default} (Δ-08; `spec/research` §3.4). Persistence writes **only non-default values**
(IntelliJ's `SkipDefaultValuesSerializationFilters`, clean-room): tiny files, clean diffs, trivial
reset-to-default (delete the key). A file that drifts back to byte-identical-with-default is
collapsed to empty.

---

## 7. `scope` metadata per key {#scope-meta}

REQ {#scope-metadata} (Δ-07; `spec/research` §2.2). Every key declares a **`scope`** governing where
it may be set and whether it roams:

- **`user`** — settable in L1 (and overridden by L2/L3). Roams (future cloud, §14).
- **`machine`** — machine-specific (paths, OS); settable in L1, **does not roam** (the VSCode
  `machine`/`machine-overridable` answer to §4.1.7 discoverability).
- **`project`** — settable in L2/L3, not L1 (project-specific only).
- **`team-only`** — L2 only (a team preference a user may not override in L3 — e.g. a project's
  canonical palette).

REQ {#scope-matrix}. The scope→writable-layer matrix is encoded in the resolver; writing a key to a
layer that its scope forbids is a typed error (the VSCode scope-target validation,
`spec/research` §2.2), never a silent drop.

---

## 8. The layer-order law, made visible — `--show-origins` {#show-origins}

REQ {#show-origins} (Δ-03, imperative 2). `vibe prefs --show-origins` prints, for every key, the
**resolved value and its origin** — which layer set it, with `file:line` where known (the git
`--show-origin` + VSCode `inspect()` combined). This is the first-class answer to "which layer is
winning?" (§4.1.1) and the debugging surface for the AIUI.

REQ {#prefs-command}. The `vibe prefs` command surface: `vibe prefs get <key>`, `vibe prefs set <key>
<value> [--layer L1|L2|L3]`, `vibe prefs list`, `vibe prefs check` (validate all layers), `vibe prefs
migrate`, `vibe prefs --show-origins`. (Distinct from `vibe show config`, which remains the
project-config view.)

---

## 9. `.gitignore` auto-generation + the path classifier {#gitignore}

REQ {#gitignore-autogen} (Δ-06, imperative 6). `vibe init` writes a `.gitignore` entry for
`.vibe/settings.local.toml` (and the L3 pattern) so a personal file is **never accidentally
committed** — the IntelliJ `workspace.xml` "keeps popping up" pain (§4.2.3) avoided by default, not
by user discipline.

REQ {#path-classifier} (Δ-06; `spec/research` §3.2). The layer of a file is **mechanically determined
by its path**, overriding any declaration (IntelliJ's `getEffectiveRoamingType` — path-macro-beats-
declaration, clean-room): `settings.local.toml` is L3 *because of its name*, not because its author
remembered to mark it so. This makes the `badWorkspaceComponents`-style blacklist (§3.2) impossible —
the wrong layer cannot be chosen by mistake.

---

## 10. Change events + `applies` {#events}

REQ {#change-events} (Δ-09; `spec/research` §2.8). The resolver emits a **granular change event**
when a layer mutates: `{ affected_keys: Set<path>, source_layer }`. A subscriber filters by
`affects(namespace)` (prefix-match) so a TUI component re-renders only its own keys, not everything
(VSCode `IConfigurationChangeEvent`, clean-room).

REQ {#applies} (Δ-09, imperative 9). Each key declares `applies = "live" | "reload" | "restart"` —
whether a change takes effect immediately, on next surface reload, or only after restart. A surface
shows the indicator (the hot-reload-vs-restart pain, §4.3.2) so the user is never left guessing.

REQ {#file-watch}. Layer files are watched; an external edit reloads the layer and re-resolves (the
VSCode debounced file-watcher, §2.3) — edit your `~/.vibe/settings.toml` in `$EDITOR` and the TUI
picks it up.

---

## 11. Secrets + untrusted L2 {#secrets}

REQ {#no-secrets-in-committed} (imperative 7; `secrets-hygiene` flow). Preference files are
**non-secret** (UI look/behaviour). The schema forbids a committed `.vibe/settings.toml` from carrying
a `[secret]`-style section; `vibe prefs check` refuses such a file (the `.idea`/`.vscode` keystore-leak
vector, §4.3.3). Secrets belong in `vibe.toml`'s `api_key_env` (env-var name) or a per-user keychain —
never in app-prefs.

REQ {#restricted-l2} (`spec/research` §2.5). A key may be `restricted` — its value is read from L2
(the committed, possibly-cloned repo) only when the project is **trusted**; in an untrusted clone,
restricted keys fall back to L1/default (the VSCode `restricted`/trust-gating pattern, clean-room).

---

## 12. Boundaries {#boundaries}

REQ {#crate-boundary}. `vibe-settings` owns: the schema/registry, the three loaders, the
`ResolvedPrefs` resolver + inspect/get/set API, change-events, validation, the `vibe prefs` command
plumbing (logic; the CLI surface lives in `vibe-cli`). It depends on `vibe-core` only for shared
error/path utilities, never on `vibe-cli` or any rendering crate.

REQ {#vs-project-config}. Project-config (`Manifest`/`UserConfig`) and app-prefs (`vibe-settings`) are
**disjoint**: different files (`vibe.toml`/`~/.config/vibe/config.toml` vs `~/.vibe/`/`.vibe/`),
different commands (`vibe show config` vs `vibe prefs`), different schemas. The two never merge into
one resolver (risk R8 — collision).

REQ {#vs-vibe-actions}. `vibe-settings` is the *data* layer (what the prefs are); `vibe-actions`
([PROP-039](../vibe-actions/PROP-039-action-system.md)) is the *behaviour* layer (what the TUI does).
A preference may gate an action's enablement (the action reads `ResolvedPrefs`), but the two crates
do not depend on each other's internals.

---

## 13. AI-Native Rust discipline {#discipline}

REQ. The crate follows the AI-Native Rust discipline
(`spec://org.vibevm.ai-native/core-ai-native`): cells with single registration points and no
sibling-cell coupling; `specmark::scope!` on every file citing the governing PROP-040 anchor; per-fn
`#[spec(implements = "spec://…#…")]` where a fn implements a specific REQ; `anyhow` at the command
edge, typed errors citing REQs in library layers; no `unwrap`/`expect` in domain logic; the ≤600-line
file budget; green `conform` + `specmap` at every commit. Each REQ anchor here is an addressable
target the code traces to.

---

## 14. Non-goals {#non-goals}

- **Cloud sync of L1** — **design-for**, not built (Δ-14, deferral DEF-1). The `scope`/`roamable`
  metadata and the ignore-and-preserve pattern (`spec/research` §2.7) are in place so a future
  three-way sync does not reshape the model; the sync transport itself is a later PROP.
- **The AIUI surface** — AIUI-**ready** (the inspect/enumerate API, §5), surface not built (built with
  AIUI).
- **The settings UI** — owned by [PROP-041](PROP-041-settings-ui.md) (TUI surface), built in Step 4
  after the TUI (PROP-037).
- **Schemes (named pref-sets)** — Δ-13, a candidate separate PROP (palette/profile presets), deferred.
- **Per-language / per-resource overrides** (VSCode `[lang]`) — Δ-15 in study; not now.
- **Touching project-config** (`vibe.toml`/`Manifest`/`UserConfig`) — out of scope (§1).

---

## 15. What this leaves to PROP-041 (settings UI) {#handoff}

The settings UI (TUI surface) is [PROP-041](PROP-041-settings-ui.md): the Configurable-EP-style page
registry (Δ-15), the settings-tree widget, per-type edit forms, the provenance view, validation
rendering, and search (Δ-16). It consumes this crate's inspect/get/set API and is built on the TUI
(PROP-037) in Step 4 of the meta-plan.
