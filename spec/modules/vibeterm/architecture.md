# VibeTerm UI Architecture (lore for the vibeterm PROP family)

**Genre:** design doc (lore) — non-binding rationale and architecture. The normative contracts are the
**vibeterm PROP family** (PROP-044 + siblings, D3); this document explains *why* the system is shaped as
it is. When lore and contract disagree, **the contract wins** (spec-genres). **Self-contained:** this doc
and the vibeterm contracts carry the full system; vibevm's methodology is **provenance** (`vibe-actions`,
`vibe tree`), never a build-dependency (RP-A + the identity-grammar conformance, research D2). **Source:**
[`research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md`](../../../research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md)
(§2 ports/adapts/new, §3 conformance, §8 deltas).

## 0. Thesis

The shell is a **TypeScript re-expression of vibevm's render-free action methodology**, self-contained
under `spec/modules/vibeterm/`. The **Model + the Action Registry *are* the interface**; the Solid **View**
is one optional projection; the **headless AIUI is the reference** surface. Control is semantic
`invoke`; CDP is observation-only. Nothing in the engine imports a rendering type — that invariant
(`#no-render-dep`) is what makes every surface and the AIUI possible.

## 1. Entities & addressing

The shell's entity set: **Window, Tab, Pane, Session/Terminal, Profile, Theme, Locale**. Each carries a
stable branded id (`TabId`, `WindowId`, `PaneId`, …) independent of its host window — `TabId` survives a
tear-off. Behaviour is addressed `action://vibeterm/<name>[?params]`; `(group, name)` is the identity, the
query is not. The catalogue: `tab.open`, `tab.select?id`, `tab.close?id`, `pane.split?target&dir`,
`pane.close?id`, `tab.move-to-window?tab&window`, `view.set-compact?on`, `theme.set?id`, `locale.set?id`,
`search.everywhere`, `quit`.

## 2. The four layers (the MVC, ported + sharpened)

| Layer | Owns | Law |
|---|---|---|
| **vibevm backend** (none, here) | — | vibeterm is self-contained; no vibevm-internal types cross the seam. |
| **Engine** (render-free TS cells) | the tab registry, pane-layout maths, session model, the protocol codec, the Action registry, the `ModelView` | **No Solid/DOM/Electron types.** A dependency-boundary lint on the floor enforces it. The single writer of the `ModelView`. |
| **View** (Solid chrome + the terminal-view pages) | the chrome (rail/list/content) renders the `ModelView`; the terminal views are lean vanilla TS + xterm.js | **No control flow in the View.** The chrome never mutates the `ModelView`; it dispatches actions and re-resolves. |
| **Controller** (the transport + the keymap) | routes chrome intents to `invoke`; routes engine events back as `ModelView` deltas | One `invoke` is the only control path; the keymap binds keys to addresses. |

The MVC cycle: **chrome intent → Controller (transport) → `invoke(addr, params, ctx)` → Action mutates the
engine → engine emits a new immutable `ModelView` + event-deltas → chrome projects it → View renders.**

## 3. The state-container reconciliation (RQ3 — the load-bearing decision)

PROP-039 §3.2 requires the resolved snapshot to be **immutable** and change to come by **re-resolution**;
Solid's `createStore` is a fine-grained **mutable** store. These coexist as follows, with **no double
source of truth**:

- the **engine owns the authoritative immutable `ModelView`** (a frozen, deeply-readable TS object);
- a **Solid store is a one-way projection** engine → chrome, rebuilt on re-resolution (a `createMemo` over
  an engine accessor); the chrome **never mutates the `ModelView`** — it dispatches actions (the AIUI verb)
  and the engine re-resolves;
- **ephemeral chrome state** (hover, drag-ghost, in-flight keystrokes, transient focus) lives **outside**
  the engine, never crosses the seam, never reaches the AIUI.

The engine is the **single writer**; the chrome is a **single reader** that re-projects on change.

## 4. The `ModelView` tree

`windows[] → tabs[] (id, title, kind, active) → panes[] (which tab, bounds)`, plus `compact`,
`activeWindow`, `activePane`, the enabled-actions set with reasons, the open-modal stack. Events
(`opened`/`closed`/`active-changed`/`moved`) are its deltas; change is delivered by re-resolution, not
in-place mutation. The **per-tab ModelView** scopes the same shape to a `TabId` — the per-tab AIUI falls
out for free. The AI **reads** the `ModelView`; the chrome **renders** it; neither touches pixels.

## 5. The AI-UI surface (RQ14 — one surface, not two)

The four verbs — `list_actions(filter?)`, `invoke(addr,args)`, `state() → ModelView`, `search(query,tab?)`
— are a **peer client** of the engine, exposed over the transport. They **fold onto the model plane**:
`state` is the read, `invoke`/`list_actions`/`search` the write. The legacy `--control` terminal plane
stays single-view (frozen, PROP-044 §8); **CDP stays observation-only**. The `vibe aiui` CLI is
target-scoped — it addresses the shell (`invoke vibeterm/*`) and the hosted `vibe tree` (`invoke
vibe.tree/*`) through one verb set.

## 6. Capability/permission for an AI peer (RQ13)

A `Dangerous` action does not run on `invoke` alone for a non-trusted caller. The model: a host-granted
`CallerScope { identity, trust, granted }` flows into every `invoke`; the engine **never trusts a
self-reported scope** (the host grants it). `Dangerous` actions return `ConfirmationRequired` and run only
after a separate `confirm` from a trusted surface. Every `Mutating`/`Dangerous` invoke emits an audit
record. Scope escalation is **REFUSE** (an error), never a warning.

## 7. The chrome ↔ engine transport (RQ15)

A versioned, serialisable **discriminated union** carrying **no Electron types**; a generated JSON-Schema
for cross-language/conformance use; **contract-semver**; **hybrid** exchange (event-stream for tab
lifecycle + request-response for commands); **single-writer main**; the chrome store is a one-way
projection (§3). Electron IPC through a **typed preload bridge** (`contextIsolation: true`) is one
transport adapter — a future external-state sidecar is another, behind the same contract.

## 8. Identity-grammar conformance (RQ12)

The Rust `vibe-actions` and the TS `vibeterm-core` share a normative **identity-grammar spec** — the
address grammar, the `ModelView` schema, the AIUI verb set, the SE provider contract, the i18n key scheme,
the capability lattice — validated by a **conformance golden in CI** on both floors. Shared **grammar**,
not shared **build-dep**; the two AIUIs stay one surface by machine, not by hope.

## 9. What this lore leaves to the contracts (D3)

The contract family fixes the normative REQs; this lore carries only the *why*. The split:
PROP-044 (shell regions/tabs/panes/windows — the existing contract) · **PROP-046** (the action/AIUI core +
the identity-grammar conformance) · **PROP-047** (the `ModelView`/MVC + the transport seam). The
**design system** has its own lore twin ([`design-system.md`](design-system.md)) and its own contract REQs
folded into the family.
