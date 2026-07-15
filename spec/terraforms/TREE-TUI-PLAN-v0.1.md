# TREE-TUI-PLAN v0.1 — the `vibe tree` TUI as a real application

_Status: PLANNED · written against tree `6473ecb` · cold-executable: every phase
ends with `bash tools/self-check.sh` green; any phase boundary is a safe stop.
The **contract** is [PROP-037](../modules/vibe-cli/PROP-037-tree-tui.md); this
plan is the recipe that executes it, phase by phase, each phase citing the
PROP-037 REQ anchors it delivers._

---

## 2 — Execution record (prepended at close)

_Empty at authoring._

---

## 3 — The mandate

Owner (2026-07-15), commissioning a large TUI expansion. Verbatim essentials:

> "Их нужно все запланировать и начать делать … максимально отделяй логику
> работы vibevm от интерфейса приложения … архитектурно выдели слои. Самый
> простой паттерн — MVC … абстрагировать все UI примитивы … твоя библиотека
> компонентов … F-клавиши для основных действий … выпадающие модальные окна
> должны перекрывать друг друга в модальном порядке … настройки сохраняй в
> ~/.vibe/tree … F6 универсальный шорткат копирования (Markdown / PNG,
> буфер / файл) … деревья во всех режимах, а не плоские списки."

Follow-up (2026-07-15): "Если у нас нет спецификации на vibe tree, её нужно
написать. И при программировании начать использовать AI Native Rust … эти фичи
должны быть максимально гранулярны и адресуемы." → the contract PROP-037 (each
feature a granular addressable REQ) is the answer; the code follows AI-Native
Rust and cites those anchors.

Answered forks (owner, 2026-07-15): the tree **shape** is an F2 setting (default
= members-as-roots + full subtrees) and — the load-bearing insight — **the tree
is a widget fed by a configurable filter pipeline**, the shapes/modes being
pipeline configs (D1). Components **wrap rat-widget**, extend in its idiom where
it lacks, drop to ratatui-core only as a last resort (D2). Unbuilt features get a
**standard `ComingSoon` modal**; PNG is reserved behind it (D3).

Scope questions resolve against PROP-037 + this mandate.

---

## 5 — Current-state facts (verified at authoring)

- The `vibe tree` TUI ships today (PROP-036 §2.11, PROP-037 supersedes the
  sketch): `crates/vibe-cli/src/commands/tree/tui/` — `mod.rs` (rat-salsa
  `run_tui`), `state.rs` (`App` + fold/flatten), `render.rs` (status/table/
  footer/tabs), `input.rs` (keys + the resize→`Changed` fix), `modal.rs` (the
  card), `modes.rs` (the flat sub-tables/tabs builders — **to be replaced by
  trees**, PROP-037 §4). Model/view/controller are **not** yet separated — P1's
  job.
- Stack: `rat-salsa 4.0` + `rat-widget 3.2` over `ratatui-core/widgets/crossterm`
  (all permissive). rat-widget provides menu / button / text-input / tabbed /
  popup / msgdialog — the wrap targets (D2).
- The model builder `build.rs` produces `PackageTree` (the vibevm boundary,
  PROP-037 §1.2) — reuse as-is.
- Discipline: `vibe-cli` is a gated crate; conform baseline EMPTY (zero slack);
  `bash tools/self-check.sh` is the floor.

---

## 6 — Decisions

- **D1 — Tree = widget + configurable filter pipeline** (PROP-037 §3). The three
  shapes (§3.3) and the modes (§4) are pipeline configs, not bespoke renderers.
  Default shape = members-as-roots + full subtrees; user-selectable on F2 + saved.
  Rejected: per-mode custom renderers (the flat lists shipped today) — they don't
  generalize and the owner explicitly wants trees everywhere.
- **D2 — Components wrap rat-widget** (PROP-037 §2.1): wrap → extend-in-idiom →
  ratatui-core last. One `ui::` facade per component. Rejected: pure hand-roll
  (more code, reinvents scroll/edit/focus) and raw rat-widget at call sites (no
  single sync point).
- **D3 — Standard `ComingSoon` modal** (PROP-037 §2.10) for every unbuilt
  feature; PNG + Search Everywhere reserved behind it. Lets P3 wire all F-keys
  early.
- **D4 — Four-layer MVC** (PROP-037 §1): vibevm boundary / Model / View (+theme)
  / Controller (keymap registry + modal stack). Styling and vibevm logic are
  each walled off. Rejected: the current mixed `tui/` (model+render+input in one)
  — the owner's core complaint.
- **D5 — F-key scheme + mode-aware keymap registry** (PROP-037 §5); footer writes
  `Shift` as `↑`. Supersedes the letter shortcuts (the earlier "don't touch keys"
  is retired by this redesign).
- **D6 — AI-Native Rust + granular addressable REQs** (PROP-037 §11, owner
  directive): every file `scope!`s a PROP-037 anchor; the REQ is the work unit.

---

## 7 — Predictions

- **P1** — the Tree-widget + filter pipeline (D1) expresses all 3 modes × 3
  shapes with **zero** per-mode bespoke render code. Falsifiable: a mode needs
  rendering outside the pipeline.
- **P2** — the wrap-rat-widget strategy (D2) covers the base component set
  (Window/Menu/Button/RadioGroup/TextField) with **no** drop to ratatui-core.
  Falsifiable: a base component needs bare ratatui-core.
- **P3** — the P1 refactor preserves today's behavior: the existing tests +
  golden pass unchanged, floor green (a pure refactor, no feature change).
- **P4** — the modal stack + keymap registry make each later menu **additive** (a
  registry entry + a modal), with no dispatch rewrite across P3–P5.

---

## 8 — Phases

Each phase ends floor-green; each cites the PROP-037 REQs it delivers.

**Phase 0 — spikes (NO commits).** rat-widget component coverage (menu / button /
text-input / radio-ish / popup stacking) for D2; the Tree-widget + filter
pipeline shape (§3.2) on the existing data; `arboard` clipboard; the
`~/.vibe/tree` JSON round-trip; the modal-stack draw+input order. Findings fold
into the decisions.

**Phase 1 — the foundation** (§1, §2.1–§2.2, §3.1–§3.2, §5.1, §6). The four-layer
split (Model / View / Controller modules), the component-library skeleton + the
`Theme`, the `Tree` widget + the filter/shape pipeline, the mode-aware keymap
registry, and the modal stack. **Refactor the existing TUI onto it with no
behavior change** (P3 prediction: tests + golden unchanged). The big enabling
phase.

**Phase 2 — trees everywhere + settings** (§3.3, §4.1–§4.4, §5.3, §9). The three
tree shapes as pipeline configs; sub-tables = stacked trees; tabs = per-tab tree;
`Shift`+arrows tab switch; `~/.vibe/tree` persistence + restore-on-launch. Proves
D1's abstraction.

**Phase 3 — menus, modals, quit-confirm** (§2.4–§2.10, §5.2, §6, §7). The
`ComingSoon` modal; F3 mode menu; F2 sort menu (groups / radio, mode-dependent);
F1 → ComingSoon; Esc quit-confirm; the mode-aware footer. Wire **all** F-keys
(stubs where unbuilt).

**Phase 4 — the detail card redesign** (§2.9, §8). The real form: paper
background, bold headers, `esc [x]`, spacing, wrapping, per-line copy.

**Phase 5 — the copy system** (§2.8, §10). Per-screen copy providers; F6 / ↑F6;
the copy-settings modal; Markdown export; clipboard / file + the file-path modal
(modal stack). PNG → ComingSoon (§10.4).

**Later (deferred, §15):** PNG rasterization; Search Everywhere; PlantUML /
Mermaid formats.

---

## 9 — Risks & fallbacks

- **R1 — rat-widget lacks a needed widget** → extend in its idiom (D2 tier 2);
  detection at P0/P1; fallback ratatui-core (tier 3), recorded.
- **R2 — the filter pipeline can't express a shape cleanly** → the shape is the
  falsifier for P1; if a shape needs bespoke code, the abstraction is re-cut in
  §3 before P2 proceeds.
- **R3 — clipboard/file I/O on Windows** (arboard, path handling) → P0 spike;
  fallback OSC-52 for clipboard if arboard misbehaves.
- **R4 — the P1 refactor regresses behavior** → the existing tests + golden are
  the guard (P3); refactor is not "done" until they pass unchanged.
- **R5 — PS5.1 UTF-8 / CRLF** (machine) → Edit/Write only; heredoc commits.

---

## 10 — Non-goals

Per PROP-037 §12: no localization (English-only, i18n indirection only); PNG +
Search Everywhere reserved behind ComingSoon; no settings-editor screen; the TUI
only (`--json`/`--plain` stay the machine/fallback surfaces).

---

## 11 — Quick-start for the executing session

```sh
git log --oneline -1                 # 6473ecb — matches the status line
bash tools/self-check.sh             # floor GREEN before Phase 0
cargo build -p vibe-cli              # ./target/debug/vibe tree — the app under change
# read the contract first:
sed -n '1,60p' spec/modules/vibe-cli/PROP-037-tree-tui.md
```

---

## 12 — Whole-campaign acceptance

```sh
bash tools/self-check.sh; echo "EXIT=$?"                 # 0
cargo test -p vibe-cli                                    # engine + pipeline unit tests green
# every PROP-037 REQ anchor the code implements is scope!-cited (specmap clean):
cargo xtask specmap && cargo xtask conform check
# the manual test MT-02 (the TUI app) is signed off by the owner
```

---

## 13 — Review points

- **RP1 — the four-layer boundaries** (module names + what may depend on what):
  executor proposes at P1 open; owner rules if it diverges from PROP-037 §1.
- **RP2 — the `ui::` component API surface** (the facade signatures): proposed at
  P1; owner may steer (it is the library everything reuses).

---

## 14 — Execution ledger

_Filled by the executing session._

---

## 15 — Deferrals ledger

- **DEF-1** — PNG export (rasterization: font + image crates; a designed card
  image) · owner · deferred behind `ComingSoon` (PROP-037 §10.4) until its spike.
- **DEF-2** — Search Everywhere (F1) · owner · deferred behind `ComingSoon`
  (§7.3); design TBD.
- **DEF-3** — PlantUML / Mermaid copy formats · owner · later additions to §10.2.
