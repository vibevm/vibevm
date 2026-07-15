# Comparative research: системы настроек VSCode vs IntelliJ — clean-room study for vibevm application/user preferences

_Status: **SYNTHESISED** 2026-07-16 из 4 research-сабагентов (VSCode config, IntelliJ settings, web
pain-points, vibevm-baseline). Comparative-research genre: quote-first, two-way gaps, deltas-not-decrees.
Не ратифицирует собственные предложения — приём downstream в спеках Шага 1._

**Genre:** comparative research (evergreen study, readable months later without the sources).
**Subject A:** VSCode (snapshot `C:\Users\olegc\git\snapshot\vscode`).
**Subject B:** IntelliJ Platform (snapshot `C:\Users\olegc\git\snapshot\idea`).
**Purpose:** взять лучшее от обеих систем для проектирования **настроек приложения/пользователя** vibevm
(Vibe Tree UI: палитры/шрифты/цвета/tier/mode/sort/shape; будущие vibe-app prefs) — трёхуровневых:
L1 user-machine (`~/.vibe/`) ⊂ L2 repo-shared (`.vibe/`, committed) ⊂ L3 user-project (gitignored).
См. мета-план `../terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md`.

> **Scope (owner, 2026-07-16).** Речь о **настройках приложения/пользователя** — НЕ о расширении
> `vibe.toml` (свойства vibe-ПРОЕКТА, package-manifest/deps/build, аналог `pom.xml`). Аналогия:
> IntelliJ `.idea/` (настройки IDE) ≠ `pom.xml` (build); VSCode `.vscode/settings.json` (workspace UI)
> ≠ `package.json` (project). Project-config (`Manifest`/`UserConfig`) — отдельная подсистема, здесь
> только как architectural reference.

> **Clean-room firewall.** Исходники VSCode/IntelliJ — inspiration-only. Код читается для понимания
> подхода; никакой код не копируется/адаптируется построчно/портируется (license/patent). Метод:
> study what their code achieves → design STRUCTURALLY DIFFERENT code. Та же позиция, что к
> `eth-sri/type-constrained-code-generation` (90-user.md).

---

## 1. Method

4 параллельных research-сабагента (native Claude, read-only, context-offload): (1) VSCode config
system internals; (2) IntelliJ settings system; (3) web pain-points; (4) vibevm-baseline. Точки входа:
VSCode `src/vs/platform/configuration/`; IntelliJ `com.intellij.openapi.components.*`
(PersistentStateComponent), `com.intellij.openapi.options.*`, RoamingType, `.idea/`. Каждое
утверждение о чужом коде — с file:line/class ссылкой.

---

## 2. Subject A — VSCode configuration system

### 2.1 Уровни + precedence (7 слоёв, left-to-right deep-merge)

Вычисляется в `Configuration` (`src/vs/platform/configuration/common/configurationModels.ts:725-1153`):

```
defaults ⊂ application ⊂ user(local+remote merged) ⊂ workspace ⊂ workspace-folder ⊂ memory ⊂ (language-override) ⊂ policy
```

- Workspace-consolidated (`:1022-1027`): `default.merge(application, user, workspace, memory)`.
- Folder layer (`:1029-1042`): workspace-consolidated `.merge(folderConfiguration)`.
- Language override применяется сверху на query-path (`:992-994`): `model.override(overrideIdentifier)`.
- **Policy** — последний слой, принудительно переписывает (`:995-1001`): enterprise lockdown.
- `user` уже = `localUserConfiguration.merge(remoteUserConfiguration)` (`:961-972`).

**Resolved value** — не «первый непустой», а **left-to-right deep-merge** (§2.4).

### 2.2 ConfigurationScope (КОРРЕКЦИЯ: 7 членов, не 9)

`configurationRegistry.ts:134-163`: `APPLICATION(1)`, `MACHINE(2)`, `APPLICATION_MACHINE(3)`,
`WINDOW(4)`, `RESOURCE(5)`, `LANGUAGE_OVERRIDABLE(6)`, `MACHINE_OVERRIDABLE(7)`. `OVERRIDE`/`BROWSER`
НЕ существуют. Language-overrides — ключи вида `[markdown]` (regex `OVERRIDE_PROPERTY_REGEX`,
`:981-982`). **Scope→target матрицы** (`workbench/services/configuration/common/configuration.ts:28-34`)
— главный референс дизайна:

```
WORKSPACE_SCOPES = [WINDOW, RESOURCE, LANGUAGE_OVERRIDABLE, MACHINE_OVERRIDABLE]
FOLDER_SCOPES    = [RESOURCE, LANGUAGE_OVERRIDABLE, MACHINE_OVERRIDABLE]
```

Runtime-валидация «target принимает scope» — `ConfigurationEditing.validate` (`configurationEditing.ts:510-581`).

**Mapping к vibevm:** VSCode-`user` ≈ L1; `workspace` ≈ L2; `workspace-folder` ≈ L3. `application`/`policy` —
лишнее (enterprise); `memory` — полезен как in-memory runtime override.

### 2.3 Форматы + расположение

| файл | target | коммитится? | vibevm аналог |
|---|---|---|---|
| `User/settings.json` | USER | НЕТ (вне repo) | L1 `~/.vibe/` |
| `.vscode/settings.json` | WORKSPACE/FOLDER | ДА | L2 `.vibe/` |
| `.code-workspace` | WORKSPACE (multi-root) | ДА (опц.) | L2 container (опц.) |

User-global path НЕ внутри репо — это и делает user-layer per-machine/per-user
(`environment/node/userDataPath.ts:44-73`). File-watching + reload, debounce 100ms
(`browser/configuration.ts:237-275`).

### 2.4 Merge semantics (КРИТИЧНО)

`ConfigurationModel.mergeContents` (`configurationModels.ts:219-229`):

```
для каждого key в target:
  если (isObject(source[key]) && isObject(target[key])) → recurse (deep merge)
  иначе → source[key] = deepClone(target[key])   // REPLACE
```

- **Скаляры:** last-wins.
- **Объекты:** рекурсивный deep-merge.
- **Массивы:** **ЗАМЕНА, не конкатенация** — `types.isObject(array) === false`, попадают в replace-ветку.
  `[a,b]` из L3 полностью заменяет `[a,b,c]` из L2.

Override-блоки с совпадающим набором identifiers deep-merge contents; новые identifiers — добавляются целиком.

### 2.5 Schema & validation

`IConfigurationPropertySchema extends IJSONSchema` (`configurationRegistry.ts:166-271`): VSCode-дополнения
`scope`, `restricted`, `editPresentation`, `policy`, `ignoreSync`, `disallowSyncIgnore` + JSON-Schema
`type/default/enum/pattern/min-max/items/markdownDescription/deprecationMessage`. Schema-bucket routing
по scope (`:337-343`, 7 singletons). Per-target schemas регистрируются под URI
`vscode://schemas/settings/{user,workspace,folder,…}` (`common/configuration.ts:18-23`). **`restricted`**
— value только из trusted sources; в untrusted workspace НЕ читается из workspace/folder
(`checkAndFilterPropertiesRequiringTrust :1324-1336`) — security-паттерн для vibevm (L2 из чужого clone).

### 2.6 Defaults

Built-in (`DefaultConfiguration`, `configurations.ts:20-80`) из registry `property.default`. Default-overrides
через `registerDefaultConfigurations` (`:427-481`) — object-defaults deep-merge с per-key source-tracking.
Extension defaults: `contributes.configuration` + `contributes.configurationDefaults` (scope-guard).

### 2.7 Settings Sync (cloud)

THREE-WAY per-key merge (`settingsMerge.ts:104-251`): fast-paths (no-op/push/pull/adopt) → both-diverged
(6 случаев added/removed/updated × local/remote) → **конфликт = same-key same-time divergent**
(`handleConflict :145-154`); non-conflicting auto-merge. Comment/order preservation через кастомный AST
(`parseSettings :559-656`). **Machine-overrides (КРИТИЧНО):** MACHINE-scoped ключи **ИСКЛЮЧАЮТСЯ из sync**
через `getIgnoredSettings` (`userDataSync.ts:54-69`); паттерн **ignore-and-preserve**
(`updateIgnoredSettings :70-102`): remote-ignored пишутся в local до merge, local-ignored реинжектятся
после — облако никогда не затирает machine-ключ. `disallowSyncIgnore` для control-plane; `-key`-prefix
форсирует sync. Profiles (`IUserDataProfile`) — bundle ресурсов как unit.

### 2.8 Inspect-API (AIUI-relevant) — КЛЮЧЕВОЙ паттерн

`IConfigurationService` (`configuration.ts:152-207`): `getValue<T>`, `updateValue`, **`inspect<T>(key)
→ IConfigurationValue<T>`**, `onDidChangeConfiguration`. **inspect return shape EXACT** (`:83-107`):
per-layer поля `defaultValue/applicationValue/userValue/userLocalValue/userRemoteValue/workspaceValue/
workspaceFolderValue/memoryValue/policyValue` + **`value`** (resolved) + per-layer sub-objects
`IInspectValue<T> = { value?, override?, overrides? }` + `overrideIdentifiers?`. **Это ровно то, что
нужно AIUI:** «effective value + какой слой установил + есть ли override». Change-event гранулярный:
`IConfigurationChangeEvent.affectedKeys: Set` + prefix `affectsConfiguration(section)`
(`configurationModels.ts:1176-1242`) — подписчик фильтрует свои ключи/namespace.

### 2.9 Что взять / НЕ брать (см. §7 deltas)

---

## 3. Subject B — IntelliJ settings system

### 3.1 Уровни + precedence (3 уровня, НО без межуровневого слияния!)

Три уровня (Application / Project / Module), каждый — отдельный `ComponentManager` со своим
`ComponentStoreImpl`. Уровень фиксируется **при регистрации сервиса**, не runtime. Физическое
расположение: App → `<configDir>/options/*.xml` (`ApplicationStoreImpl.kt:120`); Project →
`<project>/.idea/*.xml` (`ProjectStoreImpl.kt:328`); Module → `<name>.iml` (`ModuleStoreImpl.kt:98`).

**Межуровневого слияния НЕ существует.** `ComponentStoreImpl.registerComponent:522` регистрирует компонент
ровно в ОДНОМ хранилище; проект НЕ перекрывает приложение. `PropertiesComponent.getInstance()` vs
`getInstance(project)` — два независимых экземпляра, «победитель забирает всё» в зависимости от
вызова.

> **Главное архитектурное расхождение для vibevm.** vibevm хочет L1⊂L2⊂L3 с выигрышем L3; IJ
> принципиально не имеет лестницы переопределения. **Брать физическое разделение уровней IJ, но
> наслаивать явный resolver** (Scheme pattern, §3.6).

### 3.2 RoamingType (КОРРЕКЦИЯ: современный enum)

`RoamingType.java:18-70`: **`DISABLED / LOCAL / PER_OS / DEFAULT`** (не устаревший `DEFAULT/PER_USER/
DISPERSED/DEV`). Предикаты: `isRoamable()` = `DEFAULT || PER_OS`; `canBeMigrated()` = `!= DISABLED`;
`isOsSpecific()` = `PER_OS`.

**Путь-макрос превосходит объявление.** `getEffectiveRoamingType` (`ComponentInfo.kt:132-149`)
принудительно `DISABLED` для всего по `$WORKSPACE_FILE$`/`$CACHE_FILE$`/`other.xml`. Автор не может
случайно сделать `workspace.xml` shared — классификатор принудительный. Только `PER_OS` меняет путь
→ `options/<os>/` (`storeUtil.kt:183-192`).

### 3.3 Shared vs Personal — прямой аналог L2/L3

| файл | личный? | механизм |
|---|---|---|
| `workspace.xml` (`$WORKSPACE_FILE$`) | **ДА** | принудительный DISABLED — **прямой аналог L3** |
| `shelf/`, `dataSources.local.xml` | ДА | `.idea/.gitignore` + `.local.xml` suffix |
| `misc.xml`, `vcs.xml`, `modules.xml` | НЕТ (shared) | обычный путь |
| `*.iml` (`$MODULE_FILE$`) | НЕТ | теперь устаревший мост |
| `runConfigurations/*.xml` | НЕТ | `isShared=true` |

`$WORKSPACE_FILE$` javadoc (`StoragePathMacros.java:21-25`): *«contains settings local to a specific
environment that should NOT be passed to other team members»*. **Разделяемое по умолчанию; локальное
требует явного opt-in через макрос.** Соглашение `.local.xml` suffix (dataSources.xml shared +
dataSources.local.xml personal).

**Принадлежность элемента, не компонента** (`RunManagerImpl.kt:245-267`): два scheme-manager'а
(`workspaceSchemeManager` isShared=false + `projectSchemeManager` isShared=true); чекбокс «Share»
переносит элемент между ними. Внутри одной функции одни ключи shared, другие personal — именно то,
что требует реальное перекрытие L2⊂L3.

### 3.4 Форматы

XML/JDOM (`<application><component name="X">…</component></application>`). `XmlSerializer` → `BeanBinding`
(bean-conventions, `@Attribute/@Property/@OptionTag`). **Только non-default значения сериализуются**
(`PersistentStateComponent.java:24-27`, `SkipDefaultValuesSerializationFilters`) → маленькие файлы,
чистые diff, тривиальный reset.

### 3.5 PersistentStateComponent

`getState()` → POJO (null = не сохранять); `loadState()` при init И при внешнем изменении файла →
**live reload**; хуки `noStateLoaded()`, `initializeComponent()`. `@State(name, storages, reloadable,
defaultStateAsResource, category, perClient, exportable)` (`State.java:17`) — НЕТ поля roamingType
(роуминг per-`@Storage`). `@Storage(value, roamingType=DEFAULT, deprecated, exclusive, exportable)`
(`Storage.java:18`). Lifecycle: `doInitComponent:578` → parse → `loadState:644`; `doSave:310` →
`commitComponent:450`; reload при внешнем изменении (`reload:827`, `analyzeExternalChangesAndUpdateIfNeeded`).

### 3.6 Schemes — ПАТТЕРН РЕЗОЛЬВЕРА L1⊂L2⊂L3

`SchemeManager<T, MUTABLE_T>` + `SchemeProcessor`. Триплет **name+parent+mutability**:
`getName()` (non-localized id); parent (`Keymap.getParent()`, `AbstractColorsScheme.parent_scheme`);
mutability — runtime-flag (`canModify`/`isReadOnly`), membership в `readOnlyExternalizableSchemes`.

**Diff-only persistence** (главное достижение): пользовательская схема хранит ТОЛЬКО различия против
bundled parent (`AbstractColorsScheme.writeAttribute :682-690`, `optimizeAttributeMap:697`); если правки
возвращаются к bundled → файл УДАЛЯЕТСЯ (`SchemeManagerImpl.isEqualToBundledScheme:572-606`). Preset =
`(parent_id, diff_map)`.

**Один resolver с fallback-chain** (ШАБЛОН vibevm): `ProjectInspectionProfileManager.getCurrentProfile:243-264`
— флаг `useProjectProfile` выбирает хранилище; `getProfile(name) = projectSchemeManager.findSchemeByName(name)
?: InspectionProfileManager.getInstance().getProfile(name, …)` (`:267-269`). НЕ два параллельных типа —
один resolver.

Bundled vs User: bundled из classloader-resource (read-only); «Save As…» разветвляет editable clone
`_@user_<basename>` (`Scheme.EDITABLE_COPY_PREFIX`). Lazy load + «current-by-name» (`currentPendingSchemeName`).

### 3.7 Configurable EP (Settings UI) — для UI-спеки Шага 1.2

Контракт (`UnnamedConfigurable.java`): `createComponent()` → `isModified()` (дешёвый, частый) →
`apply()` (бросает `ConfigurationException`) → `reset()` → `disposeUIResources()`. EPs
(`PlatformExtensionPoints.xml`): `applicationConfigurable`, `projectConfigurable` (area=IDEA_PROJECT,
dynamic=true), `groupConfigurable`. `ConfigurableEP` schema: `instance`/`provider`, `displayName`/`key`+`bundle`,
`id`, `parentId`/`groupId`, `dynamic`, `groupWeight`, `nonDefaultProject`. **Класс лениво создается при
первом открытии** — метаданные в XML, чтобы дерево рендерилось без загрузки классов страниц.
**`SearchableConfigurable.getId()` == XML `id`** — join-key. **`searchableOptions.xml` build-time index**
— полный программный реестр каждой страницы — introspection surface для AIUI.

### 3.8 Settings Sync (cloud)

Категории = `SettingsCategory` enum (`UI, KEYMAP, CODE, TOOLS, SYSTEM, PLUGINS, OTHER`). Исключение —
**ФИЛЬТР, не блэклист** (`SettingsSyncFiltering.kt:16,49`): синхронизируется iff
`category != OTHER && category enabled && roamingType.isRoamable`. **Git-based 3-way merge** (JGit,
`GitSettingsLog.kt:550,604`): `SettingsProvider.mergeStates(base, older, newer)`; при реальном конфликте
побеждает newer-timestamp; opaque XML = LWW; orchestrator `SettingsSyncBridge.mergeAndPush:491` **никогда
не запрашивает пользователя**. Per-device = UUID в `RoamingType.DISABLED` (`SettingsSyncLocalSettings.kt:22`).
Per-OS = storage bin `mac/windows/linux`.

### 3.9 Programmatic API (AIUI-relevant) — BOTTLENECK

**Три несвязанных пути, без unified introspection:** (1) `PersistentStateComponent` via getService (POJO,
надо знать класс); (2) `PropertiesComponent` (flat string map, **нет enum-API**); (3) Configurable EP
enumeration. **Унифицированной introspection НЕТ** — agent/test/CLI не может единообразно перечислить
настройки. Лучший dump = `ExportSettingsAction` (`.zip` XML per-component). **CLI `--set-key` НЕТ.**
→ vibevm: спроектировать ОДНО типизированное перечисляемое дерево настроек с первого дня.

### 3.10 Что взять / НЕ брать (см. §7)

---

## 4. Web — pain points (разработчики на settings API) + design imperatives

### 4.1 VSCode pain points (топ)
1. **Precedence confusion** — 4 уровня, docs двусмысленны; issue [#228983](https://github.com/microsoft/vscode/issues/228983) — workspace НЕ перебивает user (баг); language-overrides `[python]` — ещё слой. [SO 67520898](https://stackoverflow.com/questions/67520898).
2. **`.vscode/settings.json` затирает личные настройки контрибьюторов** — friction. [Joplin forum](https://discourse.joplinapp.org/t/43184).
3. **Extension `inspect()/update()` баги** — `inspect()` ломается для resource/language sections; [#40489](https://github.com/microsoft/vscode/issues/40489).
4. **Settings Sync конфликты** — «Cannot resolve merge conflicts» ([#104626](https://github.com/microsoft/vscode/issues/104626)), Windows vs Linux не мёрджится ([#111482](https://github.com/microsoft/vscode/issues/111482)).
5. **JSON schema gaps → молчаливые опечатки** — unknown keys не валидируются, `get('typoKey')` молча `undefined`. [vscode-docs #322](https://github.com/vicrosoft/vscode-docs/issues/322).
6. **Deprecated ненадёжно флагаются** — миграции меняют поведение тихо; [#204061](https://github.com/microsoft/vscode/issues/204061).
7. **`machine`/`machine-overridable` scope** — решение, но мало кто знает (discoverability).
8. **Atomic write ломает symLinks** ([#195539](https://github.com/microsoft/vscode/issues/195539)).
9. **Application scope / enterprise policies** — «Managed by Organization» (policy-слой).

### 4.2 IntelliJ pain points (топ)
1. **`.idea/` — что коммитить** — вечная путаница. Официально: коммитить всё КРОМЕ `workspace.xml`, `usage.statistics.xml`, `shelf/`. [JB guide](https://intellij-support.jetbrains.com/hc/en-us/articles/206544839) (126 комментариев жалоб). Цитата: *«`.idea` directory caused us never-ending headaches, so our solution was to leave it out of version control completely»*.
2. **CodeStyle/inspection profiles — shared by NAME, не по значению** — *«references the configuration by name, but the actual configuration is not there … later someone changes the current style profile and it won't be re-shared»* (Igor Santos, JB). Inspect Code использует свой профиль.
3. **`workspace.xml` «keeps popping up»** — [SO 19973506](https://stackoverflow.com/questions/19973506); шумные diff'ы, merge-конфликты.
4. **Settings Sync неполный** — keymap не синхронизируется; нельзя одновременно Settings Repository + cloud sync; кросс-IDE ненадёжен; Snap auto-update затирал настройки.
5. **PersistentStateComponent миграции ломаются** — «inconsistent», «loadState not being called»; 2026.1 EAP «Many unexpected breaking changes» ([JB platform](https://platform.jetbrains.com/t/3644)).
6. **Settings UI медленный** — серое окно до 2 минут.

### 4.3 Cross-cutting
1. **Array merge ambiguity** — эталон: dotnet/runtime [#118204](https://github.com/dotnet/runtime/issues/118204) (цитата): *«I expect `["C"]`… In reality `["C","B"]`… feels like a trap»*. .NET мёрджит по индексу, не заменяет. **Ни одна система не делает merge массивов интуитивно по умолчанию.**
2. **Hot-reload vs restart** — VSCode часть настроек требует Reload Window (нет индикатора каких); IntelliJ editorconfig/code-style часто требует переоткрытия.
3. **Секреты в настройках** — `.idea/`/`.vscode/` скрытый вектор; IDEA ≤13 `dataSources.ids` содержал DB-пароли; AI-тулы читают `.env`.
4. **Defaults overriding by `null`** — «как вернуть default?» часто требует удалить ключ, не поставить null.
5. **Migration/deprecation UX** — серый text ненадёжен; нужен явный `config doctor`.
6. **Расширения засоряют user settings** — Ownership непонятен; нужен namespacing + separation.

### 4.4 CLI/TUI-specific layering (как git/cargo/helix/lazygit/starship/editorconfig решают)
- **Git** — canonical «last value wins» (`system < global < local < worktree`) + `includeIf`. Боль: порядок — если `[user]` после `[includeIf]`, перебивает conditional. [git-config docs](https://git-scm.com/docs/git-config).
- **Cargo** — **cleanest**: `.cargo/config.toml` (repo) переопределяет `~/.cargo/config.toml` (user), key-by-key merge. [Cargo config](https://doc.rust-lang.org/cargo/reference/config.html). Ближайший positive-pattern для vibevm.
- **Helix** — `-c` flag ПОЛНОСТЬЮ заменяет, не слоит. [#5919](https://github.com/helix-editor/helix/issues/5919). CLI override ≠ layer — частая ошибка.
- **Lazygit** — правильная иерархия (repo переопределяет global; parent-dir подхватываются). [#3293](https://github.com/jesseduffield/lazygit/issues/3293).
- **Starship** — НЕТ нативного layering (один файл; direnv-костыли). Negative-pattern.
- **`.editorconfig`** — cascade по directory, `root = true` останавливает подъём. Боль: `root=true` **молча** тенит repo-root конфиг. [roslyn #56142](https://github.com/dotnet/roslyn/issues/56142).

### 4.5 Топ-10 design imperatives для vibevm (из чужих ошибок)

1. **Layer-order = law.** Зафиксируй `policy/env/CLI > user-machine > repo-shared > user-project > default` в одном абзаце в начале docs и в binary. Никакой двусмысленности (VSCode 4.1).
2. **`vibe prefs --show-origins`** — всегда показывать источник каждого значения с file:line (git `--show-origin` + VSCode `inspect()`).
3. **Array replace by default, merge opt-in per-key with declared strategy.** Никогда не мёрджь массивы молча по индексу (dotnet #118204).
4. **Schema-first, TOML.** Unknown/deprecated → loud warning at boot (VSCode 4.1.5/1.6). TOML beats JSON (comments) и YAML (Norway problem).
5. **Three files, three roles, explicit markers.** `~/.vibe/` (L1) / `.vibe/settings.toml` (L2 committed) / `.vibe/settings.local.toml` (L3 gitignored). Никогда не клади personal в committed-файл (`.vscode/settings.json` 4.1.2, `workspace.xml` 4.2.3).
6. **Gitignore `*.local.toml` auto-generated by `vibe init`.** Не полагайся на юзера (workspace.xml «keeps popping up»).
7. **Secrets out of committed config** (`env:` / keychain / local-file). Pre-commit refusal (keystore-leak).
8. **Deprecation = first-class migration** (`replaced_by` in schema + `vibe prefs migrate`). Никаких «greyed-out в JSON» (VSCode 4.1.6, IntelliJ PersistentStateComponent 4.2.5).
9. **Declarative `applies = live | reload | restart` per setting.** TUI показывает (hot-reload 4.3.2).
10. **CLI flag layers, never replaces; cascade shadows warned.** `--config` flag добавляет слой, не заменяет (Helix `-c` 4.4.3); любое переопределение репортится (editorconfig `root=true` 4.4.6).

---

## 5. vibevm-current-baseline (✓ сабагент; reframed — project-config OUT of scope)

**Важно:** описанное ниже — **project-config** (`Manifest`/`vibe.toml`, `UserConfig`) — ДРУГОЙ класс
настроек (свойства проекта, как `pom.xml`), **OUT of scope**. Приведён как architectural reference
(что НЕ трогаем) + precedent resolver'а.

**Project-config (существует, НЕ трогаем):**
- `Manifest` (`crates/vibe-core/src/manifest/document.rs:67`, репо `vibe.toml`, committed) + `UserConfig` (`user_config.rs:47`, `~/.config/vibe/config.toml`, узкий: `[env]`+`[install]`).
- Spec §9.5 (`VIBEVM-SPEC.md:1072`) объявляет 5-уровневую precedence, но merge-алгоритма НЕТ — только `promote_user_config_env` (`main.rs:246`).
- `vibe show config` (`commands/show/config.rs:20`) — слои side-by-side с provenance, НЕ composite.
- **Precedent resolver'а:** `vibe-publish/src/token.rs:134` `load_token_for_host` — 5-уровневый host-aware resolver с `TokenSource` provenance.

**Application/user prefs (НОВАЯ подсистема — НЕ существует):**
- `.vibe/` сегодня — только cache (`.vibe/cache/...`, gitignored); никаких prefs.
- `~/.vibe/` сегодня — только `registries/` + `search-cache/`; никаких prefs.
- TUI §9 PROP-037 планировал ad-hoc `~/.vibe/tree` (не реализовано) → становится **первым экземпляром** новой подсистемы.
- **Gap:** вся трёхуровневая app-prefs подсистема — новая.

---

## 6. Two-way gaps (где vibevm отстаёт / где может вести)

### 6.1 Где vibevm отстаёт (trail)
- Нет layered app-prefs подсистемы вообще (VSCode/IntelliJ/Cargo/lazygit имеют).
- Нет merge-алгоритма (даже project-config §9.5 — spec-only).
- Нет inspect-API / unified introspection (VSCode `inspect()` есть; IntelliJ — bottleneck).
- Нет schema/validation (VSCode есть; IntelliJ — нет).

### 6.2 Где vibevm может вести (lead)
- **Unified typed introspection с первого дня** — то, чего НЕТ ни у VSCode (extension-host-only `getConfiguration`), ни у IntelliJ (три несвязанных пути). AIUI-ready: `get(path) → {label, type, value, level, roaming}` + `set(path, value)` без знания классов.
- **TOML, не XML/JSON** — clean, comments, no Norway-problem (IntelliJ застрял в XML).
- **Символьный TUI settings-UI** — VSCode/IntelliJ Settings UI — тяжёлые GUI; vibevm делает лёгкий TUI (мгновенный, ищущий).
- **Явный layer-order law + `--show-origins`** — VSCode precedence confusion избегается.
- **App-prefs ≠ project-config** — чёткое разделение (`.vibe/` vs `vibe.toml`), которого IntelliJ смешивает (`.idea/` содержит и build-связанное, и personal).
- **Clean-room design** — не тащить legacy (`.iml` deprecation, `badWorkspaceComponents` blacklist).

---

## 7. Numbered roadmap deltas (→ priority + target home; приём downstream в спеках)

> Каждый delta — предложение; исследование ратифицирует, приём — в PROP Шага 1.

- **D-01 [P0, settings-system PROP]** Layered deep-merge `L1 ⊂ L2 ⊂ L3` (L3 wins): scalar last-wins, objects deep-merge, **arrays replace by default** (VSCode §2.4). merge-strategy opt-in per-key (imperative 3).
- **D-02 [P0, settings-system PROP]** `ResolvedPrefs` resolver + `inspect(key) → { value, l1, l2, l3, default, origin }` per-field provenance (VSCode `IConfigurationValue<T>` §2.8) — КЛЮЧЕВОЙ AIUI-API.
- **D-03 [P0, settings-system PROP]** `vibe prefs --show-origins` command (file:line per value) — imperative 2.
- **D-04 [P0, settings-system PROP]** Schema-first TOML + validation; unknown/deprecated → loud warning at boot — imperative 4.
- **D-05 [P0, settings-system PROP]** Three files, three roles: `~/.vibe/` (L1) / `.vibe/settings.toml` (L2 committed) / `.vibe/settings.local.toml` (L3 gitignored) — imperative 5; `.idea/`/`$WORKSPACE_FILE$` analogue (IntelliJ §3.3).
- **D-06 [P0, settings-system PROP]** `.gitignore` для `*.local.toml` auto-gen by `vibe init` — imperative 6; path-macro-beats-declaration classifier (IntelliJ §3.2) — L3 mechanically forced, не convention.
- **D-07 [P0, settings-system PROP]** `scope`-metadata per-key (machine-only / overridable-from-L2 / roamable) + scope→layer matrix (VSCode §2.2) — drives где ключ можно установить и синхронизировать.
- **D-08 [P1, settings-system PROP]** Deprecation `replaced_by` + `vibe prefs migrate` (imperative 8); diff-from-default serialization (IntelliJ §3.4) — маленькие файлы, тривиальный reset.
- **D-09 [P1, settings-system PROP]** Declarative `applies = live | reload | restart` per setting (imperative 9); granular change-event `affectedKeys` + prefix `affectsConfiguration` (VSCode §2.8) для reactive TUI.
- **D-10 [P1, settings-system PROP]** Secrets out of committed config; `restricted`-gating для untrusted L2 (VSCode §2.5) — L2 из чужого clone.
- **D-11 [P1, settings-system PROP]** CLI flag **layers**, never replaces; cascade shadows warned (imperative 10, Helix `-c` 4.4.3, editorconfig `root=true` 4.4.6).
- **D-12 [P1, settings-system PROP]** Unified typed introspection: ОДНО перечисляемое дерево `get(path)/set(path)` (того, чего НЕТ у IntelliJ §3.9) — AIUI-ready с первого дня.
- **D-13 [P1, settings-system PROP]** Scheme model для named pref-sets (palette-profiles): name+parent+mutability, diff-only persistence, один resolver с fallback-chain (IntelliJ §3.6) — preset = `(parent_id, diff_map)`.
- **D-14 [P2, deferral DEF-1]** Cloud sync L1: three-way merge + **ignore-and-preserve** для machine-prefs (VSCode §2.7); category-filter + `isRoamable()`-predicate (IntelliJ §3.8); per-device UUID DISABLED; per-OS bin. Design-for, не built.
- **D-15 [P2, settings-UI PROP]** Configurable-EP-style registry для settings-UI: declarative pages (`id`, `displayName`, `parentId`, lazy instance, `groupWeight`) + build-time searchable-index (IntelliJ §3.7) — introspection surface.
- **D-16 [P2, settings-UI PROP]** Configurable lifecycle contract: cheap `isModified` / `apply` (throws) / `reset` (IntelliJ §3.7) — для TUI forms.

---

## Re-fetch list (для обновления вместо переписывания)

- VSCode snapshot: `C:\Users\olegc\git\snapshot\vscode` (local mirror; upstream microsoft/vscode). Доступ 2026-07-16.
- IntelliJ snapshot: `C:\Users\olegc\git\snapshot\idea` (local mirror; upstream JetBrains/intellij-community). Доступ 2026-07-16.
- Pain-points: web search июль 2026; GitHub issues microsoft/vscode (`area:config`), JetBrains/intellij-community settings.
- Key sources: dotnet/runtime #118204 (array-merge trap); JB `.idea` guide + 126 comments; VSCode #228983 (precedence bug), #40489 (inspect API), #104626/#111482 (sync conflicts); git-config / cargo-config / helix #5919 / lazygit #3293 / editorconfig roslyn #56142; IntelliJ 2026.1 breaking changes (JB platform forum).
