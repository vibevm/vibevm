# SETTINGS-SYSTEM-META-PLAN v0.1 — настройки приложения/пользователя vibevm (Vibe Tree + будущий vibe app) + UI (как в VSCode/IntelliJ)

_Status: PLANNED · written 2026-07-16, **reframed 2026-07-16 (owner correction — см. §3)** ·
cold-executable: каждый шаг/фаза ends `bash tools/self-check.sh` green; любая граница — безопасная
остановка. **Это мета-план** (последовательность четырёх шагов); детали каждого шага — в его
собственной спеке/подплане (см. §13)._

> **Авторство:** owner-commissioned 2026-07-16. Зонтик над двумя большими работами — (A) системой
> настроек приложения/пользователя и (B) TUI `vibe tree` (PROP-037). Owner переопределил порядок:
> **settings system идёт ПЕРЕД TUI**, потому что TUI (Шаг 3) потребляет её (§9 PROP-037 — экземпляр
> системы, не ad-hoc), а settings UI (Шаг 4) строится на TUI. **AIUI** — «потом»: settings и actions
> AIUI-ready по дизайну, но surface не built.

---

## 2 — Execution record (пополняется при закрытии)

- 2026-07-16: план написан; 4 research-сабагента (VSCode config, IntelliJ settings, web pain-points,
  vibevm-baseline). Вернулись: **vibevm-baseline**, **web pain-points**, **VSCode** (7 уровней
  deep-merge, `inspect()` per-layer provenance, scope→layer матрицы, three-way sync merge, arrays=replace).
  В фоне: IntelliJ. → synthesis в `spec/research/settings-system-vscode-idea.md`.
- 2026-07-16 (позже): **owner reframed scope** — речь о **настройках приложения/пользователя**
  (Vibe Tree UI: палитры/шрифты/цвета/tier/mode/sort/shape; будущие vibe-app prefs), НЕ о расширении
  `vibe.toml` (project-manifest). Хранится в `.vibe/` (repo) + `~/.vibe/` (user) + gitignored
  per-project. План скорректирован (§§3,5,6,7,8,9,10).

---

## 3 — The mandate

Owner (2026-07-16), двумя сообщениями. Существенное verbatim:

> «хранение настроек пользователя … трехуровневыми: настройки пользователя вообще (на этом компьютере
> или в будущем в облаке) + общие настройки репозитория (лежат в репозитории) + настройки пользователя
> под этот конкретный проект (тоже лежат на машине пользователя). Это порядок их перекрытия: обычные
> предпочтения человека перекрываются частными проектными предпочтениями команды, а это всё в свою
> очередь перекрывается файн-тюнингом настроек человека в конкретном проекте. … разработать систему
> хранения настроек, которая будет не хуже, чем в VSCode и IntelliJ IDEA. … Нужно взять лучшее от этих
> систем настроек. … сходить в интернет и понять, на что пользователи обычно жалуются … два компонента
> — система настроек как некий бэкенд. И дальше UI для их изменения и отображения. … план: 1) вначале
> пишем 2 спецификации на систему настроек и на ее UI 2) реализуем систему настроек (без UI) 3)
> Разрабатываем все что мы обсудили про TUI выше 4) На этом TUI делаем UI для настроек. Настройки тоже
> должны быть AIUI ready, конечно (но пока это не разрабатываем).»

> **Owner correction (2026-07-16):** «Нет, я не про те настройки говорю. Имеются в виду настройки Vibe
> Tree, которые можно например положить в .vibe внутри репозитория, и в ~/.vibe в домашнем каталоге.
> vibe.toml это свойства vibe-проекта, а мы говорим о вещах типа настроек шрифтов и цветов и тому
> подобного. У intellij idea например это хранится в .idea, а не в pom.xml Maven например»

### ⚠️ Scope (уточнён) — что конфигурируем

**Объект конфигурации — настройки ПРИЛОЖЕНИЯ/пользователя** (application/user preferences): Vibe Tree
UI (палитра, glyph-vocabulary, rendering tier, display mode, sort, tree shape, fold state, future
fonts/sizes), и будущие vibe-app prefs. **НЕ** расширение `vibe.toml` (это свойства vibe-ПРОЕКТА —
package-manifest/deps/build, аналог `pom.xml` Maven). Аналогия владельца: IntelliJ **`.idea/`**
(настройки IDE) ≠ **`pom.xml`** (build); VSCode **`.vscode/settings.json`** (workspace UI prefs) ≠
**`package.json`** (project). Project-config (`Manifest`/`vibe.toml`, `UserConfig`) — отдельная
существующая подсистема (PROP-000), в этой работе **НЕ трогается**.

**Расположение (трёхуровневое):**
- **L1 user-machine** — `~/.vibe/` home (глобальные личные prefs: дефолтная палитра, tier и т.п.). Аналог VSCode User settings; IntelliJ Application-level.
- **L2 repo-shared** — `.vibe/` внутри репозитория (коммитятся, командные prefs для проекта: «для этого репо — Catppuccin Mocha»). Аналог VSCode `.vscode/settings.json`; IntelliJ `.idea/` shared (`*.xml`, RoamingType DEFAULT).
- **L3 user-project** — gitignored per-project (`.vibe/*.local.toml` или отдельный personal-файл; личный файн-тюнинг). Аналог IntelliJ `workspace.xml` (personal, RoamingType PER_USER — НЕ коммитится).

**Порядок перекрытия:** L1 ⊂ L2 ⊂ L3 (L3 выигрывает); CLI/env override поверх всего. Прямые аналоги:
VSCode User/Workspace/Workspace-Folder; IntelliJ Application/Project + RoamingType. Источники изучения —
`C:\Users\olegc\git\snapshot\{vscode,idea}` (вне репо, clean-room: study-then-design, не copy code).
Два компонента — backend + UI. AIUI-ready, не built.

---

## 4 — The four steps (последовательность)

| Шаг | Что | Deliverable | Artefact |
|---|---|---|---|
| **0** | **research** (В ПРОЦЕССЕ) | clean-room study VSCode/IntelliJ config systems + web pain-points + vibevm-baseline → synthesis | `spec/research/settings-system-vscode-idea.md` |
| **1** | **2 спецификации** (boss-authored) | (1) settings-system PROP (L1/L2/L3 + merge + schema + inspect-API + AIUI-ready-not-built); (2) settings-UI PROP (TUI surface) | `spec/modules/vibe-settings/PROP-0NN-settings.md` + `…/PROP-0NN-settings-ui.md` |
| **2** | **settings system impl (без UI)** | новый crate/cell: трёхуровневая load + merge + `ResolvedPrefs` + inspect-API; consumed TUI (Vibe Tree) + будущий vibe app | `crates/vibe-settings/` (или cell в vibe-cli) |
| **3** | **TUI** (PROP-037 + visual language) | весь план `TREE-TUI-PLAN-v0.2`; §9 settings-persistence = экземпляр системы (палитра/tier/mode/sort/shape) | `TREE-TUI-PLAN-v0.2.md` + code |
| **4** | **settings UI на TUI** | UI настроек (Шаг 1.2 спека) поверх TUI (Шаг 3) | code + MT |

**AIUI** — после (settings + actions AIUI-ready по дизайну, surface не built).

---

## 5 — Current-state facts (verified; vibevm-baseline сабагент, 2026-07-16; reframed)

**Важно:** vibevm-baseline описывает **project-config** (`Manifest`/`vibe.toml`, `UserConfig`) — это
**ДРУГОЙ** класс настроек (свойства проекта, как `pom.xml`), и он **OUT of scope** для этой работы.
Приведён ниже как референс существующей подсистемы (что НЕ трогаем) + прецедент resolver'а.

**Project-config (существует, OUT of scope):**
- `Manifest` (`crates/vibe-core/src/manifest/document.rs:67`, репо `vibe.toml`, коммитится) + `UserConfig` (`crates/vibe-core/src/user_config.rs:47`, `~/.config/vibe/config.toml`, узкий: `[env]`+`[install]`).
- Spec §9.5 (`VIBEVM-SPEC.md:1072`) объявляет 5-уровневую precedence, но merge-алгоритма НЕТ — только `promote_user_config_env` (`main.rs:246`).
- `vibe show config` (`commands/show/config.rs:20`) — слои side-by-side с provenance, НЕ composite.
- **Precedent resolver'а:** `vibe-publish/src/token.rs:134` `load_token_for_host` — 5-уровневый host-aware resolver с `TokenSource` provenance — готовая модель layered-resolver'а.

**Application/user prefs (НОВАЯ подсистема — НЕ существует):**
- `.vibe/` сегодня — **только cache** (`.vibe/cache/...`, gitignored); никакого prefs-файла.
- `~/.vibe/` сегодня — только `registries/` (cache) + `search-cache/`; никаких prefs.
- TUI §9 PROP-037 планировал ad-hoc `~/.vibe/tree` (не реализовано) → становится **первым экземпляром** новой подсистемы.
- **Gap:** вся трёхуровневая app-prefs подсистема — новая (loader, merge, `ResolvedPrefs`, inspect-API, schema). Project-config переиспользуется только как architectural reference (как устроен layering), не как объект расширения.

---

## 6 — Decisions (предварительные, финализируются в Шаге 1 спеке после research synthesis)

- **D1 — Три уровня + precedence (app-prefs).** L1 user-machine (`~/.vibe/`, напр. `~/.vibe/settings.toml`) ⊂ L2 repo-shared (`.vibe/settings.toml` в репо, коммитится) ⊂ L3 user-project (`.vibe/settings.local.toml`, gitignored). L3 выигрывает; CLI/env override поверх всего. Merge-семантики (scalar last-wins, objects deep-merge, **arrays replace** — как VSCode `ConfigurationModel.merge`, см. research §2.4) — в спеке Шага 1.
- **D2 — `ResolvedPrefs` resolver.** Единый resolve-вход → composed L1+L2+L3 с **per-field provenance** (как `TokenSource`/VSCode `inspect()`). Заменяет ad-hoc чтение; `vibe show config`-аналог для app-prefs (`vibe prefs --show-origins`) показывает источник каждого значения.
- **D3 — НОВАЯ подсистема app-prefs**, параллельная project-config (НЕ расширение `UserConfig`/`Manifest`). Собственный crate/cell, собственный schema, собственная трёхуровневая load. Project-config не меняется.
- **D4 — L3 gitignored by default.** `.vibe/settings.local.toml` (personal per-project); `vibe init` генерирует `.gitignore`-entry (pain-point IntelliJ `workspace.xml` «keeps popping up»). Marker в header-comment каждого файла про роль (L1/L2/L3).
- **D5 — schema-first + validation + inspect-API (best of VSCode).** TOML schema с валидацией (unknown-key warning, не silent — pain VSCode 1.5/1.6). `inspect(key) → { value, l1, l2, l3, default, origin }` (VSCode `IConfigurationValue<T>`). Гранулярный change-event (`affectedKeys` + prefix `affectsConfiguration`, VSCode §8) — для reactive TUI + AIUI. `scope`-метаданные per-key (machine-only? overridable-from-L2?) — VSCode scope→layer матрица (research §1). Deprecation `replaced_by` + `vibe prefs migrate` (pain VSCode 1.6).
- **D6 — backend и UI раздельно.** Шаг 2 (backend) не зависит от UI; Шаг 4 (UI) потребляет inspect-API. AIUI-ready: inspect/discover programmatic surface обязателен в Шаге 2.
- **D7 — clean-room study.** VSCode/IntelliJ код — inspiration-only (study → design структурно-различного кода), не copy-line-by-line (license/patent). Синтез → `spec/research/settings-system-vscode-idea.md`.

Rejected (recorded): расширять `UserConfig`/`Manifest` (это project-config — другой объект); всё в один файл (нет L3/personal); JSON (vibevm — TOML; TOML beats JSON comments и YAML whitespace); policy/application enterprise-слои VSCode (лишнее); multi-root `.code-workspace` (лишнее).

---

## 7 — Predictions (falsifiable)

- **P1** — app-prefs подсистема — **новая**, параллельная project-config; `vibe.toml`/`UserConfig` не меняются несовместимо. Falsifiable: project-config schema тронута.
- **P2** — `ResolvedPrefs` resolver покрывает TUI (Vibe Tree) + будущий vibe app **без** per-consumer чтения слоёв (единый resolve-вход). Falsifiable: consumer читает сырой слой.
- **P3** — TUI §9 settings-persistence становится **экземпляром** подсистемы (палитра/tier/mode/sort/shape/fold — все через `ResolvedPrefs`, <100 строк TUI-side). Falsifiable: TUI дублирует layering-логику.
- **P4** — pain-points VSCode/IntelliJ/web (precedence confusion, what-to-commit `.idea`/`.vibe`, schema gaps, array-merge trap, sync conflicts) **адресованы** явно в спеке (каждая → design-imperative, см. web research топ-10). Falsifiable: известная боль не отражена.

---

## 8 — Phases

Каждый шаг разбит на фазы; каждая ends `bash tools/self-check.sh` green + `cargo xtask specmap --check`.

**Шаг 0 — research (В ПРОЦЕССЕ, NO code commits):**
- 4 сабагента: VSCode config (✓), IntelliJ settings (в фоне), web pain-points (✓), vibevm-baseline (✓).
- **Ф0.1** synthesis → `spec/research/settings-system-vscode-idea.md` (comparative-research: quote-first, two-way gaps, numbered deltas). VSCode: 7 уровней deep-merge, inspect(), scope-матрицы, three-way sync. Web: топ-10 design imperatives. IntelliJ: `.idea/` structure, RoamingType, Configurable-EP.
- **Ф0.2** deferral: cloud sync (L1 «в будущем в облаке») — design-for (ignore-and-preserve для machine-ключей, VSCode §7), не built.

**Шаг 1 — 2 спецификации (boss-authored):**
- **Ф1.1** settings-system PROP (`spec/modules/vibe-settings/PROP-0NN-settings.md`): L1/L2/L3 app-prefs модель, precedence, merge (arrays=replace), schema, validation, `ResolvedPrefs` + inspect-API + change-events, scope-метаданные, AIUI-ready-not-built. Каждый REQ гранулярный addressable anchor.
- **Ф1.2** settings-UI PROP (`spec/modules/vibe-settings/PROP-0NN-settings-ui.md`): TUI surface — дерево настроек, поиск, edit per-key с provenance, layered-view («откуда значение»), validation feedback.

**Шаг 2 — settings system impl (без UI):**
- Фазы TBD после спеки (ожидаемо): L1/L2/L3 loaders (`~/.vibe/`, `.vibe/`, `.vibe/*.local.toml`); `ResolvedPrefs` resolver + provenance; schema/validation; inspect-API + change-events; `.gitignore` generation; unit-тесты layering (golden, 3 уровня × merge-семантики). Consumer: TUI (Vibe Tree §9), будущий vibe app.

**Шаг 3 — TUI (см. `TREE-TUI-PLAN-v0.2`, мета-план ниже в этом файле):**
- Визуальный язык → компоненты → tree-widget → trees-everywhere → keymap → card → copy; §9 = экземпляр системы (палитра/tier/mode/sort/shape через `ResolvedPrefs`).

**Шаг 4 — settings UI на TUI:**
- Фазы TBD: дерево настроек widget, edit-form per-type, provenance-view, validation-rendering, search.

---

## 9 — Risks & fallbacks

- **R1 — путаница `.vibe/` cache vs prefs.** Сегодня `.vibe/` = cache; новые prefs-файлы не должны конфликтовать. → отдельный prefs-файл (`.vibe/settings.toml`) vs cache-dir (`.vibe/cache/`); спека фиксирует layout.
- **R2 — per-field provenance cost.** → provenance ленивый (в inspect, не при каждом get) — как VSCode `ConfigurationInspectValue`.
- **R3 — secrets в prefs.** Prefs — UI (палитры/шрифты), секретов не несут; но спека запрещает `[secret]`-секции в committed `.vibe/settings.toml` (pain VSCode/IntelliJ keystore-leak).
- **R4 — convergence pain (VSCode «какой слой выигрывает»).** → явная documented layer-order + `vibe prefs --show-origins` (источник каждого значения, file:line) — pain VSCode 1.1.
- **R5 — what-to-commit confusion (IntelliJ `.idea`).** → L3 gitignored by default + `.gitignore` auto-gen by `vibe init` + marker в header; документ «что коммитить» — pain IntelliJ 2.1/2.3.
- **R6 — schema gaps / array-merge trap.** → unknown-key warning; **arrays replace by default**, merge opt-in per-key with declared strategy — pain dotnet #118204 / VSCode.
- **R7 — clean-room fence.** VSCode/IntelliJ — огромные; риск accidental copy. → study → design notes → свой код; три-сессии firewall (study → redesign → implement).
- **R8 — collision с project-config framing.** Команда может спутать `.vibe/settings.toml` (app-prefs) с `vibe.toml` (project). → явное именование + документ + `vibe prefs` vs `vibe show config` (project) — разные команды.

---

## 10 — Non-goals

- **Cloud sync L1** — design-for (ignore-and-preserve для machine-prefs), не built (owner: «в будущем в облаке»).
- **AIUI surface** — AIUI-ready (inspect/discover), не built.
- **Трогать project-config** (`vibe.toml`/`Manifest`/`UserConfig`) — НЕ в scope; это отдельная подсистема (свойства проекта, как `pom.xml`).
- **Multi-root workspaces / `.code-workspace`-аналоги** — лишнее для vibevm.

---

## 11 — Quick-start для исполняющей сессии

```sh
git log --oneline -1                              # сверить status line
bash tools/self-check.sh                          # floor GREEN
# прочитать исследования:
sed -n '1,80p' spec/research/settings-system-vscode-idea.md   # synthesis (после Ф0.1)
# project-config (РЕФЕРЕНС, не объект работы — не трогать):
sed -n '60,120p' crates/vibe-core/src/manifest/document.rs    # Manifest (vibe.toml)
sed -n '1,40p' crates/vibe-core/src/user_config.rs            # UserConfig (project user-config)
# .vibe/ сегодня = cache ( НЕ prefs):
ls -la .vibe/ 2>/dev/null                                      # cache only
ls -la ~/.vibe/ 2>/dev/null                                    # registries cache only
```

---

## 12 — Whole-campaign acceptance

```sh
bash tools/self-check.sh; echo "EXIT=$?"           # 0
cargo test -p vibe-settings                        # layering + provenance + merge unit tests green
vibe prefs --show-origins                          # provenance каждого значения (L1/L2/L3/CLI/env)
# TUI (Шаг 3): см. TREE-TUI-PLAN-v0.2 acceptance
# settings UI (Шаг 4): MT sign-off владельцем
cargo xtask specmap && cargo xtask conform check   # REQ-traceability: каждый anchor scope!'d
```

---

## 13 — Review points

- **RP1 — L3 file name/location** (`.vibe/settings.local.toml` vs отдельный `~/.vibe/projects/<hash>/…`): предлагается в Ф0.1 synthesis; owner rules.
- **RP2 — merge-семантики** (arrays replace-default + opt-in merge-strategy per-key; nested deep-merge): предлагается в Ф1.1 спеке; owner rules (контракт).
- **RP3 — UI scope Шага 4** (полный edit-form vs view+edit-in-`$EDITOR`): предлагается в Ф1.2 спеке; owner rules.
- **RP4 — namespace app-prefs vs project-config** (`.vibe/settings.toml` + `vibe prefs` vs `vibe.toml` + `vibe show config`): owner confirm именование.

---

## 14 — Execution ledger

_Заполняется исполняющей сессией._

---

## 15 — Deferrals ledger

- **DEF-1** — cloud sync L1 (Settings Sync как VSCode/IntelliJ) · owner · design-for (ignore-and-preserve machine-prefs), не built.
- **DEF-2** — AIUI settings surface · owner · AIUI-ready (inspect/discover), surface built с AIUI.
- **DEF-3** — Schemes (named selectable pref-sets, как IntelliJ ColorSchemes / VSCode Profiles) · owner · кандидат на отдельный PROP (palette-profiles для TUI).
- **DEF-4** — per-language / per-resource override dimension (VSCode `[lang]`) · owner · если vibevm захочет per-filetype overrides.

---

## 16 — МЕТА-ПЛАН TUI (Шаг 3, перенос из plan-сессии — для durable)

> Полный план Шага 3 (TUI PROP-037 + visual language) — в `TREE-TUI-PLAN-v0.1.md` → обновить до
> **v0.2** (visual language как primary axis, 11 фаз P0–P10). Краткая сводка здесь для cold-resume:

**Visual language vision (design-doc `spec/design/tui-visual-language.md`):**
- Unicode box-drawing со скруглениями `╭╮╰╯` (не ASCII `+-|`); fold `▾▸`, DAG `↩`, flags `●○`; braille/blocks для индикаторов.
- **5 палитр** (data-driven `Palette` trait, semantic role tokens): Rosé Pine cosmic-violet (сохранить ТОЧНО) + Catppuccin Mocha/Macchiato/Frappé/Latte (light). Активная палитра → **app-prefs подсистема** (Шаг 2), не ad-hoc.
- Rendering tiers + degradation: truecolor→256→16; rounded→unicode→ASCII. Detection: crossterm + `$COLORTERM`/`$TERM` (→ pref с override).
- Окна/модалки — красивые (solid panel, rounded, title-chip, padding, shadow), не fallback-вид.
- Только символы для основного UI (переносимость SSH/tmux, без Sixel/Kitty); `ratatui-image` readiness зарезервирован.

**Фазы TUI (P0–P10):** P0 prep · P1 visual-language spec · P2 palette/glyph/tier system · P3 ui-foundation (ui::Window, state/ split, ModalStack) · P4 tree-widget+pipeline+3-shapes · P5 trees-everywhere · P6 keymap+actions+quit · P7 components+F2+ComingSoon · P8 detail-card · P9a settings-через-систему (ResolvedPrefs) · P9b copy · P10 discipline+sign-off.

**Подробный план** — в `~/.claude-glm/plans/hashed-questing-beaver.md` (session) до записи `TREE-TUI-PLAN-v0.2.md`.
