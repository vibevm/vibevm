# PROP-047 — the vibeterm `ModelView`, MVC, and the transport seam {#root}

**Status:** DRAFT (v0.1, 2026-07-19) — pre-MVP architectural sketch. **Module:** `vibeterm`. **Genre:**
contract. **Lore:** [`architecture.md`](architecture.md) §2–§4, §7. **Source:**
[`research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md`](../../../research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md)
§2.11, §2.14, §2.1, §8 (deltas D6, D14, D16).

This contract governs the **serialisable `ModelView`** the engine publishes, the **MVC reconciliation**
that keeps the Solid chrome a one-way projection, the **chrome↔engine transport contract**, and the
**shell entity set**.

## 1. The `ModelView` tree {#modelview}

REQ {#tree}. The observable shell state is a serialisable `ModelView` carrying:
`windows[]` → `tabs[] { id: TabId, title, kind, active }` → `panes[] { tabId: TabId, bounds }`, plus
`compact: boolean`, `activeWindow: WindowId`, `activePane: PaneId`, the enabled-actions set with their
addresses + reasons, and the open-modal stack. It carries **no rendering types**.

REQ {#deltas}. Change is delivered as a new immutable `ModelView` plus **event-deltas**
(`opened` / `closed` / `active-changed` / `moved`); there is no in-place mutation and no second mutation
mechanism beyond re-resolution.

REQ {#per-tab-scope}. A per-tab `ModelView` scopes the same shape to a `TabId` — the per-tab AIUI reads
it; the per-tab surface falls out for free.

## 2. The MVC reconciliation {#mvc}

REQ {#single-writer}. The **engine is the single writer** of the `ModelView`. The Solid chrome store is a
**one-way projection** (engine → chrome), rebuilt on re-resolution. The chrome never mutates the
`ModelView`; it dispatches actions and re-resolves.

REQ {#projection}. The projection is a pure function of the `ModelView`; the chrome applies optimistic UI
only for ephemeral state, never for the `ModelView`.

REQ {#ephemeral-state}. Ephemeral chrome state (hover, drag-ghost, in-flight keystrokes, transient focus)
lives **outside** the engine, never crosses the transport seam, and never reaches the AIUI.

## 3. The chrome ↔ engine transport contract {#transport}

REQ {#contract-union}. The chrome↔engine protocol is a versioned, serialisable **discriminated union**
carrying **no Electron-specific types**. It comprises **commands** (`open`, `select`, `split`, `close`,
`move-to-window`, `set-compact`, `set-theme`, `set-locale`) and **events** (`opened`, `closed`,
`active-changed`, `moved`).

REQ {#codec}. A codec encodes/decodes the union to/from the transport frame. A **JSON-Schema** is
generated from the union for cross-language/conformance use. No Electron type appears in the contract.

REQ {#versioning}. The contract carries a **contract-semver**; a version mismatch between the chrome and
the engine is a typed error at handshake, never a silent miscommunication.

REQ {#exchange-model}. The exchange is **hybrid**: an event-stream for tab-lifecycle events + request-
response for commands. Events are monotonic per window; the chrome drains them in order.

REQ {#consistency}. The model is **single-writer main**: the main-process engine is authoritative; the
chrome is a one-way projection (§`#single-writer`). There is no chrome-side write-back of the `ModelView`.

REQ {#transport-adapter}. Electron IPC through a **typed preload bridge** (`contextIsolation: true`) is
one transport adapter. A future external-state sidecar (HTTP/WS/stdio) is another adapter, behind the same
contract — adding it is a new adapter, not a redesign.

## 4. The shell entity set {#entities}

REQ {#entity-ids}. The shell entities — **Window, Tab, Pane, Session, Profile, Theme, Locale** — each
carry a stable **branded id** (`WindowId`, `TabId`, `PaneId`, …) independent of the hosting window. A
`TabId` survives a tear-off (PROP-044 §3, D0).

REQ {#catalogue}. The `action://vibeterm/*` catalogue at pre-MVP: `tab.open`, `tab.select?id`,
`tab.close?id`, `pane.split?target&dir`, `pane.close?id`, `tab.move-to-window?tab&window`,
`view.set-compact?on`, `theme.set?id`, `locale.set?id`, `search.everywhere`, `quit`. Adding a command =
registering an Action; it then appears in the chrome, the keymap, and Search Everywhere with no further
wiring.

## 5. Surfaces {#surfaces}

REQ {#surface-trait}. A `Surface` is the adapter seam: it `present`s a `ModelView` and yields `Event`s.
The Solid chrome is a visual surface; the headless AIUI (PROP-046 §`#aiui`) is a surface whose `present`
is a no-op. No engine code depends on a `Surface` implementation or a rendering type.

## 6. Pre-MVP scope {#pre-mvp}

REQ. The pre-MVP ships: the `ModelView` tree (§`#tree`) with one window + N tabs + a single active pane;
the commands `tab.open` + `tab.select` (create + switch); the events `opened` + `active-changed`; the
contacts-style tab list (chrome); the two launch theme token sets (present, switchable); the en + ru i18n
scaffolding. Split, tear-off, profiles, per-tab networked AIUI are deferred (PROP-044 §7).

## 7. Non-goals {#non-goals}

- Split view / tear-off in the pre-MVP (PROP-044 §4/§5 — the architecture is ready; the M1 build wires
  them, the pre-MVP does not).
- A networked sidecar transport — the contract is ready (§`#transport-adapter`); no sidecar is built.
- Profile restore — the entity exists; restore is deferred (PROP-044 §7).
