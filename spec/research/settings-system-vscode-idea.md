# Comparative research: трёхуровневые системы настроек — VSCode vs IntelliJ (clean-room study for vibevm)

_Status: **IN PROGRESS** (4 research-сабагента запущены 2026-07-16; vibevm-baseline ✓, VSCode/IntelliJ/web в фоне). Шаблон — comparative-research genre (quote-first, two-way gaps, deltas-not-decrees). Заполняется по мере возврата сабагентов → boss synthesis._

**Genre:** comparative research (evergreen study, readable months later without the sources).
**Subject A:** VSCode (snapshot `C:\Users\olegc\git\snapshot\vscode`).
**Subject B:** IntelliJ Platform (snapshot `C:\Users\olegc\git\snapshot\idea`).
**Purpose:** взять лучшее от обеих систем для проектирования трёхуровневых настроек vibevm (L1 user-machine ⊂ L2 repo-shared ⊂ L3 user-project). См. мета-план `../terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md`.

> **Clean-room firewall.** Исходники VSCode/IntelliJ — inspiration-only. Код МОЖЕТ читаться для
> понимания подхода; никакой код не копируется/адаптируется построчно/портируется (license/patent).
> Рабочий метод: study what their code achieves → design STRUCTURALLY DIFFERENT code с тем же
> поведением (та же логика — ок; заимствованное выражение — нет). Та же позиция, что и к
> `eth-sri/type-constrained-code-generation` (90-user.md).

---

## 1. Method (как изучали)

- **4 параллельных research-сабагента** (native Claude, read-only, context-offload): (1) VSCode config
  system internals; (2) IntelliJ settings system; (3) web pain-points (разработчики на API настроек);
  (4) vibevm-current-baseline.
- **Точки входа:** VSCode `src/vs/platform/configuration/`; IntelliJ `com.intellij.openapi.components.*`
  (PersistentStateComponent), `com.intellij.openapi.options.*`, RoamingType, `.idea/`.
- Каждое утверждение о чужом коде — с file:line/class ссылкой; quotes fenced с access date.

_Подробные отчёты сабагентов — в transcript (tasks/…); сюда переносится synthesis._

---

## 2. Subject A — VSCode configuration system

_(заполняется по возврату сабагента; planned structure)_

### 2.1 Уровни + precedence
_Точный список layers и порядок перекрытия (defaults → product → application → user/machine → workspace
→ workspace-folder → language-overrides). ConfigurationScope enum._

### 2.2 Форматы + расположение
_settings.json (user global; workspace `.vscode/settings.json`; multi-root `*.code-workspace`). Что
коммитится (workspace folder), что нет (user global)._

### 2.3 Configuration model internals
_IConfigurationNode, IConfigurationRegistry, ConfigurationModel, overrides (per-language `[markdown]`,
per-resource), layer composition._

### 2.4 Merge semantics
_скаляры last-wins, objects deep-merge?, arrays replace-vs-extend? Точный precedence-resolution._

### 2.5 Schema & validation
_JSON schema contribution (`package.json` `contributes.configuration`), defaults, enum, type, scope,
deprecation, restricted._

### 2.6 Settings Sync (cloud)
_что синхронизирует, merge-conflict resolution, machine-overrides._

### 2.7 Inspect-API (AIUI-relevant)
_`getConfiguration`, `inspect()`, `update()`, `onDidChangeConfiguration`, change granularity._

### 2.8 Что взять / что НЕ взять
_concrete patterns (layers, merge, schema, inspect) + pain points._

---

## 3. Subject B — IntelliJ settings system

_(заполняется по возврату сабагента; planned structure)_

### 3.1 Уровни + precedence (Application / Project / Module)
### 3.2 **Shared vs personal (RoamingType)** — прямой аналог L2/L3
_DEFAULT (shared/roaming) vs PER_USER (local) vs DISPERSED/DEV. Что коммитить в `.idea/`, что нет
(`workspace.xml` personal; `*.iml`/`runConfigurations` shared)._
### 3.3 Форматы (XML, service-based)
### 3.4 PersistentStateComponent (`@State`, `@Storage`, roamingType)
### 3.5 Configurable API (Settings UI) — для UI-спеки Шага 1.2
_applicationConfigurable / projectConfigurable EP, ConfigurableV2, displayName, createComponent,
isModified, apply, reset._
### 3.6 Schemes (named selectable sets)
### 3.7 Settings Sync (cloud)
### 3.8 Programmatic access (AIUI-relevant)
### 3.9 Что взять / что НЕ взять

---

## 4. Web — pain points (разработчики на settings API)

_(заполняется по возврату сабагента; planned structure)_

### 4.1 VSCode pain points
### 4.2 IntelliJ pain points
### 4.3 Cross-cutting (hot-reload vs restart, array-merge ambiguity, secrets, migration, discoverability)
### 4.4 CLI/TUI-specific layering (git, cargo, npm, starship, helix — как решают precedence без GUI)
### 4.5 Топ-10 design imperatives для vibevm (выведенные из чужих ошибок)

---

## 5. vibevm-current-baseline (✓ заполнено, сабагент 2026-07-16)

vibevm уже имеет **2 из 3 уровней**, но без layering-merge:

- **Единой `Config` struct НЕТ.** `Manifest` (`crates/vibe-core/src/manifest/document.rs:67`, репо
  `vibe.toml`, коммитится, L2) + `UserConfig` (`crates/vibe-core/src/user_config.rs:47`,
  `~/.config/vibe/config.toml`, L1 — **узкий**: только `[env]` + `[install].slot_integrity`).
- **L3 (user-project) ОТСУТСТВУЕТ полностью.** Некуда положить machine-specific project-настройки без
  загрязнения коммитящегося `vibe.toml`.
- **Spec §9.5** (`VIBEVM-SPEC.md:1072`) объявляет 5-уровневую precedence (CLI > `VIBE_*` env > project
  `vibe.toml` > user config > built-in defaults), но **merge-алгоритма НЕТ** — только точечный
  env-fallback `promote_user_config_env` (`main.rs:246`).
- **`vibe show config`** (`commands/show/config.rs:20`, `ConfigReport`) показывает слои side-by-side с
  provenance, но **НЕ composite** — registries/overrides только из Manifest.
- **Precedent resolver'а:** `vibe-publish/src/token.rs:134` `load_token_for_host` — рабочий 5-уровневый
  host-aware resolver с `TokenSource` provenance — готовая модель layered-resolver'а (изолирован от
  основного config-story).

**Точный gap для L1/L2/L3:** (1) создать L3 (gitignored `.vibe/config.toml` или `vibe.local.toml`);
(2) расширить L1 секциями-зеркалами Manifest (registries/mirrors/llm/active); (3) `ResolvedConfig`
resolver (L1+L2+L3 + per-field provenance), `ConfigReport` → presentation; (4) синхронизировать
stale doc-comment `user_config.rs:18-24`.

---

## 6. Two-way gaps (где vibevm отстаёт / где ведёт)

_(заполняется после synthesis §2–§4)_

### 6.1 Где vibevm отстаёт (trail)
### 6.2 Где vibevm может вести (lead) — spec-driven, AIUI-ready, clean TOML

---

## 7. Numbered roadmap deltas

_(заполняется после synthesis; каждый delta → приоритет + target home в spec tree; исследование
ратифицирует — приём downstream)_

---

## Re-fetch list (для обновления вместо переписывания)

- VSCode snapshot: `C:\Users\olegc\git\snapshot\vscode` (local mirror; upstream microsoft/vscode).
- IntelliJ snapshot: `C:\Users\olegc\git\snapshot\idea` (local mirror; upstream JetBrains/intellij-community).
- Доступ: 2026-07-16.
- Pain-points: web search (июль 2026); GitHub issues microsoft/vscode `area:config`,
  JetBrains/intellij-community settings.
