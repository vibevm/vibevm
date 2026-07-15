# PROP-041: the vibevm settings UI — the `vibe prefs` TUI surface (`vibe-settings-ui`) {#root}

**Status:** DRAFT — requirements, 2026-07-16 (owner-commissioned). The **contract** for the TUI surface
that lets a user **view and edit** the application/user preferences of [PROP-040](PROP-040-settings.md):
a settings tree, per-type edit forms, a provenance ("where does this value come from?") view, validation
feedback, and search — built on the `vibe tree` TUI (PROP-037) and drivable headless (AIUI-ready).
**Related:** [PROP-040](PROP-040-settings.md) (the settings system — the data layer this renders);
[PROP-037](../vibe-cli/PROP-037-tree-tui.md) (the `vibe tree` TUI — component library + theme this is
built on); the visual-language design-doc
[`spec/design/tui-visual-language.md`](../../design/tui-visual-language.md); the clean-room study
[`spec/research/settings-system-vscode-idea.md`](../../research/settings-system-vscode-idea.md) (§3.7
Configurable EP, §3.9 introspection); the `addressable-specs` and `managed-blocks` flows.

This contract is deliberately **granular and addressable** (owner directive): every feature is its own
`{#anchor}` REQ. It owns the **surface**; the data layer, resolver, and inspect-API are PROP-040's.

---

## 1. Overview — a settings surface, not a settings engine {#overview}

REQ {#surface-not-engine}. The settings UI is a **Surface** over `vibe-settings` (PROP-040): it renders
`ResolvedPrefs` and captures edits, but owns **no** preference logic, schema, or merge — those are
PROP-040's. The UI reads through `inspect`/`get`/`list` and writes through `set` (PROP-040 §5). This is
the data/surface split that lets the same prefs be driven headless by the AIUI unchanged.

REQ {#built-on-tree-tui}. The UI is built from the PROP-037 component library (`ui::` facade — Window,
Menu, Form fields, Card) and the visual-language `Theme` (PROP-037 §2.2 /
[`spec/design/tui-visual-language.md`](../../design/tui-visual-language.md)). It does **not** re-invent
widgets; it composes the TUI's components. It is built in **Step 4** of the meta-plan, after the TUI
itself (Step 3) lands.

REQ {#english-default-i18n-ready}. UI strings are English and live behind the `vibe-actions` i18n
indirection (PROP-039 §8), English the only mandatory-complete locale.

---

## 2. The page registry — Configurable-EP-style {#registry}

REQ {#declarative-pages} (Δ-15; `spec/research` §3.7). Settings are organised into **pages** declared
in a registry (the IntelliJ `Configurable` EP, clean-room — *not* its code). A page declaration carries:

- `id` — a stable, non-localised identifier (the join-key);
- `parent_id` / `group_id` — for the tree hierarchy (unresolved groups land in "Other");
- `display_name` (localisable) + `description`;
- `group_weight` — ordering within a parent;
- `scope_flag` — application-level vs project-level page (the IntelliJ `nonDefaultProject` analogue);
- a **lazy** page body (created on first open) — the registry metadata is cheap so the whole tree
  renders without constructing every form (IntelliJ §3.7).

REQ {#registry-is-introspectable}. The page registry is the **enumerable** source for both the settings
tree (§3) and the search index (§7): adding a page means registering a declaration, which then appears
in the tree, search, and the AIUI with no further wiring (the IntelliJ `searchableOptions.xml`
build-time index is the model; vibevm builds it from the registry at runtime).

REQ {#stable-id-law}. A page `id` is immutable once published (the `qualified-naming` rule): a rename
is a new id with the old one retired as a tombstone/alias, so a bookmark/link/search-result never
silently retargets.

---

## 3. The settings tree widget {#tree-widget}

REQ {#tree-widget}. The left pane is a **tree of pages** (groups → pages) rendered through the PROP-037
`Tree` widget (so it inherits its glyphs, theme, fold, keyboard model — no bespoke renderer). `↑`/`↓`
move, `←`/`→` fold/expand, `Enter` opens the focused page's form (§4) in the right pane.

REQ {#tree-shows-origin-hint}. Each page row carries a compact **origin hint** when one of its keys is
shadowed — a glyph + the winning layer (e.g. `▾ appearance  [L3]`) — so the user sees at a glance where
a value is coming from (the §4.1.1 precedence pain, surfaced visually).

REQ {#tree-context}. The tree respects the active project context (which repo's `.vibe/` is L2); a
no-project session shows only L1 (user-machine) pages.

---

## 4. The edit form — per-type fields {#edit-form}

REQ {#form-per-type} (Δ-16; `spec/research` §3.7). The right pane renders the focused page as a **form**
of typed fields, one per preference key, by `KeyMeta.type` (PROP-040 §6):

- **bool** → a toggle/checkbox;
- **enum** → a `RadioGroup` (PROP-037 §2.7) or dropdown `Menu` (§2.4);
- **int/string** → a `TextField` (§2.8), with the schema's constraints surfaced;
- **array** → an editable list (add/remove/reorder), honouring the key's `merge` strategy (PROP-040 §4);
- **table** → a nested group (`Group`, §2.6) of sub-fields.

REQ {#configurable-lifecycle} (`spec/research` §3.7). Each form follows the IntelliJ `Configurable`
contract, clean-room: a cheap `is_modified()` (form vs current `ResolvedPrefs`), `apply()` (writes via
PROP-040 `set`, throws a typed error on validation failure), `reset()` (model → form). `apply` is gated
on `is_modified()`; the form never writes a no-op.

REQ {#write-layer-choice}. Editing a field writes to a **chosen layer** (default L3 for a project
session, L1 for a no-project session), selected in the form — never silently to the wrong layer (the
VSCode `.vscode-overwrites-contributors` pain, §4.1.2). Writing to a layer the key's `scope` forbids is
refused with the reason (PROP-040 §7).

REQ {#apply-indicator}. The `applies` metadata (PROP-040 §10) is shown per field — a "needs restart" /
"needs reload" badge — so the user knows when a change takes effect (the §4.3.2 hot-reload pain).

---

## 5. The provenance view — "where does this value come from?" {#provenance}

REQ {#provenance-view} (PROP-040 §5, §8). A field shows its **provenance** on demand: the resolved
value plus each layer's contribution (`default / L1 / L2 / L3 / CLI / env`), the winning `origin`
(file:line where known), and which layers are shadowed. This is the visual form of `vibe prefs
--show-origins` (PROP-040 §8) — the first-class answer to "which layer is winning?".

REQ {#provenance-edit}. From the provenance view the user can **override at a specific layer** (set L3
without touching L2, or clear L3 to fall back to L2) — direct, layer-aware editing, not a single
mystery write.

---

## 6. Validation feedback {#validation}

REQ {#validation-feedback} (PROP-040 §6). Schema violations (unknown key, wrong type, out-of-range,
deprecated) render inline next to the offending field, in the warning style, with the rule cited — the
VSCode "silent typo" pain (§4.1.5) made visible. A field in error blocks `apply` for that field and
reports why.

REQ {#lint-all}. `vibe prefs` UI offers a "check all layers" action (the `vibe prefs check` command,
PROP-040 §6) — a flat list of every warning across L1/L2/L3, jump-to-field.

---

## 7. Search {#search}

REQ {#settings-search} (Δ-15; `spec.research` §3.7). A search (the `vibe-actions` Search Everywhere
engine, PROP-039 §10 — the same engine the `vibe tree` TUI uses) finds settings by **key, display name,
description, and synonyms**. Selecting a result opens the owning page with that field focused. The
search index is built from the page registry (§2) so a new page is searchable with no extra wiring.

REQ {#deprecated-discoverable}. Deprecated keys remain searchable (they surface the `replaced_by`
migration path, PROP-040 §6), so a user looking for an old name is guided to the new one.

---

## 8. Built on the TUI + the action system {#built-on}

REQ {#commands-are-actions}. Every settings-UI command (open a page, apply, reset, search, jump-to-next
warning, switch write-layer) is a `vibe-actions` **Action** in group `vibe.prefs`, addressed
`action://vibe.prefs/<name>` (PROP-039 §3), bound through the PROP-037 keymap (§5). The footer lists the
enabled actions for the current context (PROP-037 §5.2). This makes the settings UI consistent with the
`vibe tree` TUI and AIUI-drivable like it.

REQ {#modal-stack}. Settings forms use the PROP-037 modal stack (§6): an enum-edit dropdown or the
provenance detail opens as a modal over the form; `Esc` pops back; depth-N is supported.

---

## 9. AIUI-ready {#aiui-ready}

REQ {#aiui-ready}. Because the UI is a Surface over PROP-040's inspect/get/set and its commands are
addressed actions, the **same settings** can later be driven headless (the AIUI, PROP-039 §11.3): an
agent lists pages (§2), reads/writes keys (PROP-040 §5), and observes change events (PROP-040 §10)
without this UI. The AIUI surface itself is **not built** here (PROP-040 §14); this contract only keeps
the design AIUI-clean (no logic leaks into the surface).

---

## 10. Non-goals {#non-goals}

- **A settings engine** — PROP-040 owns the data layer; this contract owns only the surface.
- **A separate component library** — uses PROP-037's `ui::` components + theme.
- **The AIUI surface** — AIUI-ready (§9), built with AIUI.
- **Cloud-sync UI** — deferred with PROP-040's cloud sync (DEF-1).
- **A GUI** — this is a TUI surface; a future GUI is a separate surface over the same PROP-040 API.

---

## 11. AI-Native Rust discipline {#discipline}

REQ. The settings-UI code follows the AI-Native Rust discipline
(`spec://org.vibevm.ai-native/core-ai-native`): cells; `specmark::scope!` citing the PROP-041 anchor on
every file; per-fn `#[spec(implements = "spec://…#…")]` where a fn implements a REQ; `anyhow` at the
command edge, typed errors citing REQs; no `unwrap`/`expect` in domain logic; ≤600-line file budget;
green `conform` + `specmap` at every commit. Each REQ anchor here is an addressable target.
