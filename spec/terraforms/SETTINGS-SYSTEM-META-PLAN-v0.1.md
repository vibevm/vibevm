# SETTINGS-SYSTEM-META-PLAN v0.1 — трёхуровневые настройки vibevm + UI (как в VSCode/IntelliJ)

_Status: PLANNED · written 2026-07-16 · cold-executable: каждый шаг/фаза ends `bash
tools/self-check.sh` green; любая граница — безопасная остановка. **Это мета-план** (последовательность
четырёх шагов); детали каждого шага — в его собственной спеке/подплане (см. §13)._

> **Авторство:** owner-commissioned 2026-07-16. Это зонтик над двумя большими работами —
> (A) системой настроек и (B) TUI `vibe tree` (PROP-037). Owner переопределил порядок: **settings
> system идёт ПЕРЕД TUI**, потому что TUI (Шаг 3) потребляет общую settings system (§9 PROP-037
> становится экземпляром system, не ad-hoc `~/.vibe/tree`), а settings UI (Шаг 4) строится на TUI.
> **AIUI** — «потом»: settings и actions AIUI-ready по дизайну, но surface не built.

---

## 2 — Execution record (пополняется при закрытии)

- 2026-07-16: план написан; 4 research-сабагента запущены (VSCode config, IntelliJ settings, web
  pain-points, vibevm-baseline). vibevm-baseline ВЕРНУЛСЯ (см. §5). Остальные в фоне → synthesis в
  `spec/research/settings-system-vscode-idea.md`.

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

Решённые fork'и (owner, 2026-07-16): уровни = L1 user-machine ⊂ L2 repo-shared ⊂ L3 user-project
(L3 выигрывает) — прямые аналоги VSCode User/Workspace/Workspace-Folder и IntelliJ Application/Project
+ RoamingType (shared DEFAULT vs personal PER_USER). Источники изучения —
`C:\Users\olegc\git\snapshot\{vscode,idea}` (вне репо, clean-room: study-then-design, не copy code).
Два компонента — backend + UI. AIUI-ready, не built.

---

## 4 — The four steps (последовательность)

| Шаг | Что | Deliverable | Artefact |
|---|---|---|---|
| **0** | **research** (В ПРОЦЕССЕ) | clean-room study VSCode/IntelliJ config systems + web pain-points + vibevm-baseline → synthesis | `spec/research/settings-system-vscode-idea.md` |
| **1** | **2 спецификации** (boss-authored) | (1) settings-system PROP (L1/L2/L3 + merge + schema + inspect-API + AIUI-ready-not-built); (2) settings-UI PROP (TUI surface) | `spec/modules/vibe-settings/PROP-0NN-settings.md` + `…/PROP-0NN-settings-ui.md` |
| **2** | **settings system impl (без UI)** | новый crate/cell: трёхуровневая load + merge + `ResolvedConfig` + inspect-API; consumed install/resolver/LLM/TUI | `crates/vibe-settings/` (или cell в vibe-core) |
| **3** | **TUI** (PROP-037 + visual language) | весь план `TREE-TUI-PLAN-v0.2`; §9 settings-persistence = экземпляр системы (палитра/tier/mode/sort/shape) | `TREE-TUI-PLAN-v0.2.md` + code |
| **4** | **settings UI на TUI** | UI настроек (Шаг 1.2 спека) поверх TUI (Шаг 3) | code + MT |

**AIUI** — после (settings + actions AIUI-ready по дизайну, surface не built).

---

## 5 — Current-state facts (verified at authoring; vibevm-baseline сабагент, 2026-07-16)

- **Единой `Config` struct НЕТ.** Два независимых типа: `Manifest` (`crates/vibe-core/src/manifest/document.rs:67`, репо-`vibe.toml`, коммитится) + `UserConfig` (`crates/vibe-core/src/user_config.rs:47`, `~/.config/vibe/config.toml`).
- **2 из 3 уровней уже есть:** L1 (`UserConfig`, **узкий** — только `[env]` + `[install].slot_integrity`); L2 (`Manifest`/`vibe.toml`, полнофункциональный, коммитится, с валидацией). **L3 (user-project) ОТСУТСТВУЕТ полностью.**
- **Spec §9.5** (`VIBEVM-SPEC.md:1072`) объявляет 5-уровневую precedence (CLI > `VIBE_*` env > project `vibe.toml` > user config > built-in defaults), но **merge-алгоритма НЕТ** — только точечный env-fallback.
- **`promote_user_config_env`** (`crates/vibe-cli/src/main.rs:246`) — единственный L1→env мост (live env выигрывает; provenance в `PROMOTED_FROM_USER_CONFIG`).
- **`vibe show config`** (`crates/vibe-cli/src/commands/show/config.rs:20`, `ConfigReport`) показывает слои side-by-side с provenance, но **НЕ composite** — registries/overrides только из Manifest.
- **Precedent: `vibe-publish/src/token.rs:134`** `load_token_for_host` — рабочий 5-уровневый host-aware resolver с `TokenSource` provenance — готовая модель layered-resolver'а (пока изолирован от основного config-story).
- **TUI §9 settings** — ad-hoc `~/.vibe/tree` (планировалось, не реализовано) → становится экземпляром общей системы в Шаге 3.

---

## 6 — Decisions (предварительные, финализируются в Шаге 1 спеке после research synthesis)

- **D1 — Три уровня + precedence.** L1 user-machine (`~/.config/vibe/config.toml`, расширенный) ⊂ L2 repo-shared (`vibe.toml`, коммитится) ⊂ L3 user-project (**НОВЫЙ**, gitignored, `.vibe/config.toml` или `vibe.local.toml`). L3 выигрывает; CLI/env override поверх всего. Точные merge-семантики (scalar last-wins, objects deep-merge, arrays replace-or-append) — в спеке Шага 1.
- **D2 — `ResolvedConfig` resolver.** Единый resolve-вход (`Config::resolve()`) → composed L1+L2+L3 с **per-field provenance** (как `TokenSource`/`ConfigReport`, но composing). Заменяет «каждый consumer читает один слой». `ConfigReport` понижается до presentation-вьюхи над `ResolvedConfig`.
- **D3 — L1 расширяется** секциями-зеркалами Manifest (registries/mirrors/llm/active), чтобы user-level defaults были first-class («add mirror для registry X для меня на этой машине» без дублирования в каждый `vibe.toml`).
- **D4 — L3 — новый gitignored слой.** Для machine-specific project-настройки (локальный mirror, dev-token, локальный source-pin) без загрязнения коммитящегося `vibe.toml`. `[[override]]` остаётся коммитящимся (L2).
- **D5 — schema-first + validation + inspect-API.** TOML schema с валидацией (unknown-key warning, не silent), inspect-API для AIUI (`config.get(...)`, `inspect(key) → value + origin layer`, `on_change`) — аналог VSCode `inspect()`. Лучшее от VSCode (layers/scopes/inspect) + IntelliJ (RoamingType shared/personal, Schemes, Configurable-EP для UI) — после research synthesis.
- **D6 — backend и UI раздельно.** Шаг 2 (backend) не зависит от UI; Шаг 4 (UI) потребляет inspect-API. AIUI-ready: inspect/discover programmatic surface обязателен в Шаге 2.
- **D7 — clean-room study.** VSCode/IntelliJ код — inspiration-only (study → design структурно-различного кода), не copy-line-by-line (license/patent). Синтез → `spec/research/settings-system-vscode-idea.md` (comparative-research genre).

Rejected (recorded): оставить два независимых типа (нет layering → боль); всё в один файл (нет L3/personal); JSON (vibevm — TOML).

---

## 7 — Predictions (falsifiable)

- **P1** — трёхуровневая модель выражается **переиспользованием** существующих `Manifest`/`UserConfig` + одним новым L3-слоем, без переделки L2 (vibe.toml). Falsifiable: L2 schema меняется несовместимо.
- **P2** — `ResolvedConfig` resolver покрывает install/resolver/LLM/TUI consumer'ы **без** per-consumer чтения слоёв (единый resolve-вход). Falsifiable: consumer всё ещё читает сырой слой.
- **P3** — TUI §9 settings-persistence становится **экземпляром** общей системы (<100 строк TUI-side, остальное — reuse). Falsifiable: TUI дублирует layering-логику.
- **P4** — pain-points VSCode/IntelliJ (precedence confusion, what-to-commit, schema gaps, sync conflicts) **адресованы** явно в спеке (каждая → design-imperative). Falsifiable: известная боль не отражена.

---

## 8 — Phases

Каждый шаг разбит на фазы; каждая фаза ends `bash tools/self-check.sh` green + `cargo xtask specmap --check`.

**Шаг 0 — research (В ПРОЦЕССЕ, NO code commits):**
- 4 сабагента: VSCode config system, IntelliJ settings, web pain-points, vibevm-baseline (✓ вернулся).
- **Ф0.1** synthesis → `spec/research/settings-system-vscode-idea.md` (comparative-research: quote-first, two-way gaps, numbered deltas).
- **Ф0.2** deferral: cloud sync (L1 «в будущем в облаке») — design-for, не built.

**Шаг 1 — 2 спецификации (boss-authored):**
- **Ф1.1** settings-system PROP (`spec/modules/vibe-settings/PROP-0NN-settings.md`): L1/L2/L3 модель, precedence, merge-семантики, schema, validation, `ResolvedConfig` + inspect-API, AIUI-ready-not-built. Каждый REQ гранулярный addressable anchor.
- **Ф1.2** settings-UI PROP (`spec/modules/vibe-settings/PROP-0NN-settings-ui.md`): TUI surface — дерево настроек, поиск, edit per-key с provenance, layered-view («откуда значение»), validation feedback.

**Шаг 2 — settings system impl (без UI):**
- Фазы TBD после спеки (ожидаемо): L3 loader + gitignore; `ResolvedConfig` resolver + provenance; расширение L1; миграция consumers на resolve-вход; schema/validation; inspect-API + change-events; unit-тесты layering (golden).

**Шаг 3 — TUI (см. `TREE-TUI-PLAN-v0.2`, мета-план ниже в этом файле):**
- Визуальный язык → компоненты → tree-widget → trees-everywhere → keymap → card → copy; §9 = экземпляр системы.

**Шаг 4 — settings UI на TUI:**
- Фазы TBD: дерево настроек widget, edit-form per-type, provenance-view, validation-rendering, search.

---

## 9 — Risks & fallbacks

- **R1 — L2 schema regression.** Расширение L1/L3 не должно ломать коммитящийся `vibe.toml`. → спека требует preserve-compat; `merge_preserving_comments` (toml_edit) сохраняется.
- **R2 — per-field provenance cost.** Resolve с provenance на каждое значение — накладные. → provenance ленивый (вычисляется в inspect, не при каждом get).
- **R3 — secrets в L3/L1.** API-key уже через `api_key_env` (env-var name, не значение). → L3/L1 наследуют тот же invariant; `vibe show config` redacts.
- **R4 — convergence pain (VSCode «какой слой выигрывает»).** → явная documented layer-order + `vibe config --show-origins` (показ источника каждого значения) как first-class command.
- **R5 — what-to-commit confusion (IntelliJ `.idea`).** → L3 gitignored по умолчанию + явный marker; документ «что коммитить» в спеке.
- **R6 — schema gaps (молчаливые опечатки).** → unknown-key warning, не silent; schema-first.
- **R7 — clean-room fence.** VSCode/IntelliJ — огромные; риск accidental copy. → study → design notes → свой код; три-сессии firewall (study → redesign → implement).

---

## 10 — Non-goals

- **Cloud sync L1** — design-for в спеке, не built (owner: «в будущем в облаке»).
- **AIUI surface** — AIUI-ready по дизайну (inspect/discover), не built.
- **Полная замена `vibe show config`** — понижается до presentation над `ResolvedConfig`, не удаляется.
- **Migration существующих `vibe.toml`** — L2 не меняется несовместимо.

---

## 11 — Quick-start для исполняющей сессии

```sh
git log --oneline -1                              # сверить status line
bash tools/self-check.sh                          # floor GREEN
# прочитать исследования:
sed -n '1,80p' spec/research/settings-system-vscode-idea.md   # synthesis (после Ф0.1)
# текущая модель:
sed -n '1,40p' crates/vibe-core/src/user_config.rs            # L1 (узкий)
sed -n '60,120p' crates/vibe-core/src/manifest/document.rs    # L2 (Manifest)
sed -n '1070,1090p' VIBEVM-SPEC.md                             # §9.5 precedence (spec-only)
```

---

## 12 — Whole-campaign acceptance

```sh
bash tools/self-check.sh; echo "EXIT=$?"           # 0
cargo test -p vibe-settings                        # layering + provenance + merge unit tests green
vibe config --show-origins                         # provenance каждого значения (L1/L2/L3/CLI/env)
vibe show config                                   # presentation над ResolvedConfig
# TUI (Шаг 3): см. TREE-TUI-PLAN-v0.2 acceptance
# settings UI (Шаг 4): MT sign-off владельцем
cargo xtask specmap && cargo xtask conform check   # REQ-traceability: каждый anchor scope!'d
```

---

## 13 — Review points

- **RP1 — L3 file name/location** (`.vibe/config.toml` vs `vibe.local.toml`): предлагается в Ф0.1 synthesis; owner rules.
- **RP2 — merge-семантики** (arrays replace vs append; nested objects deep-merge depth): предлагается в Ф1.1 спеке; owner rules (это контракт).
- **RP3 — UI scope Шага 4** (полный edit-form vs view+edit-in-`$EDITOR`): предлагается в Ф1.2 спеке; owner rules.

---

## 14 — Execution ledger

_Заполняется исполняющей сессией._

---

## 15 — Deferrals ledger

- **DEF-1** — cloud sync L1 (Settings Sync как VSCode/IntelliJ) · owner · design-for в спеке, не built.
- **DEF-2** — AIUI settings surface · owner · AIUI-ready (inspect/discover), surface built с AIUI.
- **DEF-3** — Schemes (named selectable sets, как IntelliJ ColorSchemes/CodeStyle) · owner · если нужно для TUI-palette/profiles — кандидат на отдельный PROP.

---

## 16 — МЕТА-ПЛАН TUI (Шаг 3, перенос из plan-сессии — для durable)

> Полный план Шага 3 (TUI PROP-037 + visual language) — в `TREE-TUI-PLAN-v0.1.md` → обновить до
> **v0.2** (visual language как primary axis, 11 фаз P0–P10). Краткая сводка здесь для cold-resume:

**Visual language vision (design-doc `spec/design/tui-visual-language.md`):**
- Unicode box-drawing со скруглениями `╭╮╰╯` (не ASCII `+-|`); fold `▾▸`, DAG `↩`, flags `●○`; braille/blocks для индикаторов.
- **5 палитр** (data-driven `Palette` trait, semantic role tokens): Rosé Pine cosmic-violet (сохранить ТОЧНО) + Catppuccin Mocha/Macchiato/Frappé/Latte (light). Активная палитра → settings (Шаг 2).
- Rendering tiers + degradation: truecolor→256→16; rounded→unicode→ASCII. Detection: crossterm + `$COLORTERM`/`$TERM`.
- Окна/модалки — красивые (solid panel, rounded, title-chip, padding, shadow), не fallback-вид.
- Только символы для основного UI (переносимость SSH/tmux, без Sixel/Kitty); `ratatui-image` readiness зарезервирован.

**Фазы TUI (P0–P10):** P0 prep · P1 visual-language spec · P2 palette/glyph/tier system · P3 ui-foundation (ui::Window, state/ split, ModalStack) · P4 tree-widget+pipeline+3-shapes · P5 trees-everywhere · P6 keymap+actions+quit · P7 components+F2+ComingSoon · P8 detail-card · P9a settings-через-систему · P9b copy · P10 discipline+sign-off.

**Подробный план** — в `~/.claude-glm/plans/` (session) до записи `TREE-TUI-PLAN-v0.2.md`; копия в git-истории этой сессии.
