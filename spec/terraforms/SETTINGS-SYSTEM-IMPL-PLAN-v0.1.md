# SETTINGS-SYSTEM-IMPL-PLAN v0.1 — реализация `vibe-settings` crate (Шаг 2 мета-плана)

_Status: PLANNED · written 2026-07-16 · cold-executable: каждая фаза ends `bash
tools/self-check.sh` green + `cargo xtask specmap --check`; любая граница — безопасная остановка.
**Контракт:** [PROP-040](../modules/vibe-settings/PROP-040-settings.md) (каждый REQ — granular
addressable anchor, cite через `specmark`). **Мета-план:** [SETTINGS-SYSTEM-META-PLAN-v0.1](SETTINGS-SYSTEM-META-PLAN-v0.1.md)
Шаг 2. **AIUI surface** — не built (Шаг 4). **Режим:** Spec-Driven Development + AI-Native Rust._

> **SDD-дисциплина (owner reminder, 2026-07-16).** Под каждую фазу — точные спеки: контракт =
> PROP-040 (granular REQs); этот план = фазировка с cite'ом конкретных PROP-040 anchors; код несёт
> спеку через `specmark::scope!` (файл → anchor) + per-fn `#[spec(implements = "spec://…")]`. Если фаза
> требует детали, которой нет в PROP-040 — пишется mechanism-spec перед кодом. AI-Native Rust: cells
> (одна точка регистрации, без sibling-coupling), `thiserror` enums с `#[specmark::spec(implements=…)]`,
> no `unwrap`/`expect` в domain, ≤600-line budget, `conform`+`specmap` green на каждом коммите.

---

## 2 — Execution record (пополняется)

- 2026-07-16: план написан; фаза 2.1 (scaffold) стартовала.
- 2026-07-16: **Шаг 2 EXECUTED** — `vibe-settings` crate целиком (фазы 2.1–2.8),
  floor green throughout. Commit map + свершившиеся predictions — см. §12.

---

## 3 — Mandate reference

Owner (2026-07-16): реализовать `vibe-settings` (Шаг 2 мета-плана) — data layer без UI. Verbatim
essentials и трёхуровневая модель — в
[`SETTINGS-SYSTEM-META-PLAN-v0.1.md#the-mandate`](SETTINGS-SYSTEM-META-PLAN-v0.1.md). Контракт REQs —
в [`PROP-040`](../modules/vibe-settings/PROP-040-settings.md). Каждая фаза ниже доставляет REQs по
anchor'ам.

---

## 5 — Current-state facts (verified)

- `vibe-settings` crate НЕ существует (фаза 2.1 создаёт).
- workspace: 14 members (`Cargo.toml:8-25`), `vibe-actions` — шаблон gated app-layer crate
  (`crates/vibe-actions/Cargo.toml`, `[package] workspace=true`, deps `specmark/thiserror/serde`,
  cell-per-file `src/{action,address,…}.rs`).
- conform.toml: `gated_crates` (`conform.toml:32-43`) включает `vibe-actions`; `vibe-settings` будет
  добавлен (PROP-040 §13 — gated). `conform-baseline.json` EMPTY (zero slack).
- specmap.toml: namespace `vibevm`; vibe-settings будет сканироваться (`scan_roots = ["crates/*"]`);
  PROP-040 anchors резолвятся (`spec/modules/vibe-settings/PROP-040-settings.md`). Зафиксированный
  `specmap.json` устарел (0 units для новых модулей) — **регенерируется** в фазе 2.8 (или раньше при
  первой надобности gate).

---

## 6 — Decisions

- **D1 — Crate layout (cells).** `vibe-settings/src/`: `lib.rs` (scope! `PROP-040#root`, re-exports),
  `schema.rs` (KeyMeta, Scope, registry — REQ §6,§7), `loader.rs` (L1/L2/L3 + path-classifier — §3,§9),
  `resolver.rs` (ResolvedPrefs + deep-merge + inspect — §4,§5), `events.rs` (change-events + applies —
  §10), `persist.rs` (diff-from-default + .gitignore — §6,§9), `error.rs` (thiserror, REQ-citing). Каждая
  cell — свой `scope!` anchor; ≤600 строк.
- **D2 — Frontend-agnostic (PROP-040 §1).** Zero render-deps (no ratatui/crossterm). Deps: `specmark`,
  `thiserror`, `serde`, `toml` (+ `toml_edit` для comment-preserving write в 2.7). Verified conform-gate.
- **D3 — `vibe prefs` CLI split.** Logic (get/set/list/origins/migrate) в `vibe-settings::cli` cell;
  surface (clap wiring, output formatting) в `vibe-cli/src/commands/prefs/`. PROP-040 §8.
- **D4 — specmap regen.** `cargo xtask specmap` (write) в фазе 2.8 → `chore(specmap)` коммит;
  manual `--check` в каждой фазе (gate НЕ в self-check для vibevm crates).
- **D5 — `vibe tree` §9 не трогается в Шаге 2.** Поглощение ad-hoc `~/.vibe/tree` в общую систему —
  Шаг 3 (TUI P9a). Шаг 2 = standalone data layer + CLI.

---

## 7 — Predictions (falsifiable)

- **P1** — трёхуровневая модель выражается чистыми функциями над TOML (`load → merge → inspect`) без
  runtime mutation; resolver — immutable snapshot per resolve. Falsifiable: resolve мутирует состояние.
- **P2** — deep-merge alg покрывает scalar/object/array** без** special-casing per key (кроме opt-in
  `merge` strategy для arrays). Falsifiable: ключ требует bespoke merge-кода.
- **P3** — inspect() round-trips provenance: `inspect(k).origin == layer_that_set_it`. Falsifiable:
  origin не совпадает с источником.
- **P4** — каждая cell проходит conform zero-findings (baseline EMPTY) и specmap-clean (scope! → anchor).

---

## 8 — Phases (каждая ends floor-green + `cargo xtask specmap --check`)

| Фаза | REQ § (PROP-040 anchors) | Deliverable | Делегат/Boss |
|---|---|---|---|
| **2.1** scaffold | `#root`, §12,§13 | crate `vibe-settings` (Cargo.toml + `lib.rs` scope!); workspace members/default/deps; conform `gated_crates`. Floor: `cargo build -p vibe-settings` + self-check | **Boss** |
| **2.2** loaders | `#locations`, `#gitignore`(classifier part), §3 | `loader.rs` — L1/L2/L3 TOML parse, missing=Ok(Default), path-classifier (layer by filename), role-marker. `error.rs` thiserror | **Delegate** |
| **2.3** resolver | `#merge`, `#resolver`, §4,§5 | `resolver.rs` — deep-merge (scalar last-wins, objects deep-merge, arrays replace + opt-in) + `inspect(key)` per-layer provenance + `get`/`get-section`. Golden unit-тесты | **Boss** (architecture) + delegate (alg) |
| **2.4** schema | `#schema`, `#scope-meta`, §6,§7 | `schema.rs` — KeyMeta, Scope enum, registry, writable-layer matrix, `set()` refuses wrong layer, unknown-key warning, deprecation `replaced_by` | **Boss** (scope-policy) + delegate |
| **2.5** events | `#events`, §10 | `events.rs` — change-event `{affected_keys, source_layer}`, `affects(prefix)`, `applies`, file-watch reload | **Delegate** |
| **2.6** CLI | `#show-origins`, §8 | `vibe prefs {get,set,list,check,migrate}` + `--show-origins`/`--layer`; logic в `vibe-settings::cli`, surface в `vibe-cli/commands/prefs/` | **Delegate** (cell) + boss (wiring) |
| **2.7** persist | `#schema`(diff part), `#gitignore`, §6,§9 | `persist.rs` — diff-from-default (non-default only, collapse-to-empty) + `.gitignore` auto-gen в `vibe init` для `*.local.toml` | **Delegate** |
| **2.8** finalize | §13,§14 | end-to-end golden (3-layer resolve/inspect/set/origins/scope-refusal/deprecation-migrate); specmap regen + `chore(specmap)` commit; self-check all green | **Boss** |

**Dependency notes:** 2.1 → все. 2.2 (loaders) + 2.4 (schema) → 2.3 (resolver merge над typed values).
2.3 → 2.5 (events над resolver). 2.3/2.4 → 2.6 (CLI над resolver+schema). 2.4 → 2.7 (diff-from-default
нужен KeyMeta.default). 2.8 — финал.

---

## 9 — Risks & fallbacks

- **R1 — `toml` vs `toml_edit`.** Read-only parse — `toml`; comment-preserving write (2.7) — `toml_edit`.
  → 2.2 `toml`, 2.7 добавляет `toml_edit`.
- **R2 — deep-merge edge cases** (mixed scalar/object at same path). → typed error citing
  `PROP-040#merge-algorithm`, не panic; golden-тест на каждый кейс.
- **R3 — path-classifier vs declaration conflict.** → filename wins (PROP-040 §9 `#path-classifier`);
  unit-тест что `settings.local.toml` всегда L3.
- **R4 — conform zero-slack.** → ≤600/cell, no unwrap, `#[specmark::spec]` на thiserror; conform check
  на каждом commit.
- **R5 — specmap stale.** → regen в 2.8; manual `--check` в каждой фазе.

---

## 10 — Quick-start

```sh
git log --oneline -1                              # сверить
bash tools/self-check.sh                          # floor GREEN
cargo build -p vibe-settings                      # crate под изменением
cargo test -p vibe-settings                       # layering + merge + inspect unit tests
cargo xtask conform check && cargo xtask specmap --check
```

---

## 11 — Whole-step acceptance (Шаг 2)

```sh
bash tools/self-check.sh; echo "EXIT=$?"           # 0
cargo test -p vibe-settings                        # all green
vibe prefs --show-origins                          # provenance (после 2.6)
cargo xtask specmap && cargo xtask conform check   # PROP-040 anchors scope!'d, zero findings
# vibe tree §9 поглощается в Шаге 3 (P9a), НЕ здесь
```

---

## 12 — Execution ledger

_Заполняется._

**Шаг 2 EXECUTED (2026-07-16) — `vibe-settings` crate целиком, floor green throughout.**

Commit map (11 коммитов `e53da83`→`02bee60`):
- `e53da83` docs(terraform): impl-plan v0.1
- `16a7d40` feat phase 2.1 — scaffold + workspace wiring
- `7af6ea1` feat phase 2.2 — loader cell (L1/L2/L3 + path-classifier)
- `9741cee` feat phase 2.4 — schema cell (KeyMeta/Scope/Schema + validation)
- `eb68bff` feat phase 2.3 — resolver cell (ResolvedPrefs + deep-merge + inspect)
- `43532fd` feat phase 2.5 — events cell (change-events + applies + Watcher trait)
- `a83a79c` feat phase 2.6 — `vibe prefs` CLI (logic cell + vibe-cli wiring)
- `12c0884` feat phase 2.7 — persist (diff-from-default + comment-preserve + gitignore)
- `4567662` fix — spec-tag public schema types (specmap-clean)
- `941bcd9` chore — specmap regen (PROP-040 units)
- `02bee60` test — end-to-end golden (3-level resolve/inspect/scope-refusal/migrate)

**Predictions — все подтверждены:** P1 (pure resolve, immutable snapshot) ✓;
P2 (deep-merge без per-key special-casing) ✓; P3 (inspect.origin roundtrips
provenance, golden-tested) ✓; P4 (каждая cell conform 0-findings + specmap-clean)
✓ для vibe-settings (138 PROP-040 units, 0 orphans).

**AI-Native Rust:** cells (loader / schema / resolver / events / cli / persist),
каждая `specmark::scope!` + per-fn `#[spec(implements)]`; `thiserror` enums с
`#[specmark::spec]`; no `unwrap`/`expect` в domain; ≤600/cell (dir-module split
где потребовалось); doctests на каждом public seam (34 doc + 87 unit + 2 e2e);
conform 0 findings.

**Делегирование:** 8 ячеек реализованы native Claude subagents под boss-спеками
(REQ-anchor + сигнатуры + acceptance-test + target), каждая — boss review (diff
как PR + re-verify tests/conform) + commit. Boss держал architecture (crate
layout, resolver design, specmap discipline) + integration + финальный golden.

**Тесты/acceptance:** `cargo test -p vibe-settings` — 87 unit + 34 doc + 2 e2e
green; `cargo test -p vibe-cli` green; `bash tools/self-check.sh` all green;
`vibe prefs --help` — все 6 subcommands; end-to-end set→get→list→origins→
check→migrate round-trip verified.

**Known pre-existing (НЕ Шаг 2):** `cargo xtask specmap --check` всё ещё fail из-за
33 orphans в `vibe-spec` (PROP-035 provisional) + 1 в `vibe-resolver` — separate
debt, не блокирует settings-system (gate advisory для vibevm crates, impl-plan
D4). `vibe tree` §9 → экземпляр системы в Шаге 3 (TUI P9a), НЕ здесь (DEF-2.1).

---

## 13 — Deferrals

- **DEF-2.1** — `vibe tree` §9 → экземпляр системы (Шаг 3 TUI P9a).
- **DEF-2.2** — cloud sync L1 (PROP-040 §14, design-for).
- **DEF-2.3** — schemes (PROP-040 §14, отдельный PROP).
