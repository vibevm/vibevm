# Ф1.4 — путь «манифест → запись каталога» и трассировка: замер

**Чем мерил.** Чтение `crates/**` (периметр чтения замера — только `crates/**`;
`vibedeps/**` и `packages/**` не открывались), инструменты — Read/Grep и `rg`
из Git Bash. **Что НЕ запускалось:** `cargo` (запрещён пакетом), `git` (запрещён
пакетом), никакие бинарники дерева. Все утверждения несут `файл:строка`;
рабочее дерево не тронуто, кроме двух файлов периметра записи.

## 1. ВЕРДИКТ

**(i) Есть ли в дереве ОДНО место, куда вписывается проекция манифестного
флага в запись каталога?** — **ДА С ОГОВОРКАМИ**: один *уровень* есть — общие
помощники `mfst::*` в `crates/vibe-index/src/scanner/manifest.rs` — но мест
вписывания **два**: инлайн-литералы `VersionEntry { … }` в
`crates/vibe-index/src/cli/add.rs:84-115` и
`crates/vibe-index/src/scanner/org_walk.rs:202-233`. Скалярные поля
(`license`, `authors`, `description`, `homepage`, `keywords`, `describes`)
каждый литерал отображает **сам по себе**, продублировано (add.rs:96-101 ↔
org_walk.rs:214-219). Новое поле `frozen: Option<bool>` обязано появиться в
обоих литералах + в фиче-строителе `VersionEntry::minimal`
(`crates/vibe-index/src/types/entry/mod.rs:132-170`), либо проектор надо
сначала выделить. Сюда же третий, неконкурирующий путь ingress:
HTTP-`POST /v1/packages` принимает готовую `VersionEntry` снаружи
(`crates/vibe-index/src/server/routes/packages.rs:232`) — туда флаг доедет
автоматически, как только он есть в структуре.

**(ii) Будет ли `warn` из пути чтения каталога виден пользователю сегодня —
без постройки подписчика?** — **ДА С ОГОВОРКАМИ**, но оговорка почти что «НЕТ»:
- **CLI-команды `vibe-index` (`add`, `remove`, `reindex`, `list`, `get`,
  `dump`, `search`, `outdated`, `verify`, …): НЕ виден.** В `main.rs`
  vibe-index подписчика нет вообще (`crates/vibe-index/src/main.rs:6-14` —
  только `eprintln!("error: …")` на ошибке), а rg по всему `crates/` даёт
  ровно две точки установки подписчика (см. §6).
- **`serve`: виден ТОЛЬКО при флаге `--auto-commit-push`.**
  `crates/vibe-index/src/cli/serve.rs:67-73` ставит `tracing_subscriber`
  только внутри `if args.auto_commit_push`; без флага «flag-off path is
  byte-for-byte the old server» (serve.rs:66) — warn теряется.
- **CLI `vibe`: виден** — `init_tracing()` вызывается безусловно в
  `crates/vibe-cli/src/main.rs:52` (EnvFilter `VIBE_LOG`, дефолт `"warn"`,
  writer stderr, main.rs:408-417), и vibe-publish-хук варнит в тот же поток
  (`crates/vibe-publish/src/post_hook.rs:333,346,365`). Но `vibe` **не
  зависит от `vibe-index`** (`crates/vibe-cli/Cargo.toml` — deps содержат
  `vibe-registry`, не `vibe-index`), так что этот подписчик путь
  `Index::load_from` не покрывает.

Что этому мешает (по пунктам): (1) подписчик vibe-index привязан к
`--auto-commit-push`, а не к чтению; (2) `vibe` и `vibe-index` — разные
процессы с разными подписчиками; (3) домашняя форма диагностики vibe-index —
`println!`/`eprintln!`, а не `warn!` (см. §6.3).

## 2. Сверка опорных координат (B1..B6)

| # | утверждение | вердикт | цитата file:line |
|---|---|---|---|
| B1 | `epoch: Option<u32>` живёт в `PackageMeta` в `crates/vibe-core/src/manifest/package.rs` сразу после `version` | **ПОДТВЕРЖДЕНО** | `crates/vibe-core/src/manifest/package.rs:84` (`pub version: semver::Version`), затем `:93-94`: `#[serde(default, skip_serializing_if = "Option::is_none")]` / `pub epoch: Option<u32>` — между ними только doc-комментарий `:85-92` |
| B2 | `content_hash` в запись каталога кладут ровно два места: `add.rs:73` и `org_walk.rs:193` | **ПОДТВЕРЖДЕНО С УТОЧНЕНИЕМ** | Производственных вызовов `compute_content_hash` ровно два: `crates/vibe-index/src/cli/add.rs:73` и `crates/vibe-index/src/scanner/org_walk.rs:193` (остальные хиты — определение в `crates/vibe-index/src/content_hash.rs:112-113` и тесты). Уточнение: поле `content_hash` присваивается ещё в двух местах, не считая этих двух — placeholder `"sha256:0"` в фиче-строителе `VersionEntry::minimal` (`crates/vibe-index/src/types/entry/mod.rs:144`) и приём готовой записи по HTTP `POST` (`crates/vibe-index/src/server/routes/packages.rs:232`, тело `Json<VersionEntry>`); оба не «кладут значение из контента» |
| B3 | Клиент индекса не читает `content_hash`: `VersionEntryView` в `wire.rs:30-33` несёт единственное поле `version`, а `rg content_hash crates/vibe-registry/src/index_client/` даёт ноль совпадений | **ПОДТВЕРЖДЕНО С УТОЧНЕНИЕМ** | `crates/vibe-registry/src/index_client/wire.rs:30-33`: `VersionEntryView { pub version: Version }` — единственное поле `:32`. Уточнение: буквально ноль совпадений rg не даёт — их **два**, но оба комментарии, не код: `crates/vibe-registry/src/index_client/mod.rs:7` («Identity (`content_hash`) is verified at …») и `:394` («re-verifies `content_hash` per PROP-002 §2.1» — верификация живёт в резолвере, вне index_client). По существу утверждение верно: ни один view-тип и ни одна строка кода index_client `content_hash` не читает |
| B4 | Сервер загружает каталог с диска ОДИН раз на старте — `serve.rs:77` — и держит его в памяти за `RwLock` в `AppState.index` | **ПОДТВЕРЖДЕНО** | `crates/vibe-index/src/cli/serve.rs:77` `let index = Index::load_from(&args.data_dir)…` (единственный вызов, до цикла `serve`); `crates/vibe-index/src/server/state.rs:30` `pub index: RwLock<Index>`; заливается один раз в `AppState::with_tokens_and_rate_limit` (serve.rs:99-106). Мутации — `state.index.write().await` в `upsert` (`crates/vibe-index/src/server/routes/packages.rs:250`). Уточнение к составу чтения: `Index::load_from` читает `repomd.json` + `by-name/*.json`, а НЕ `primary.jsonl` (`crates/vibe-index/src/index/memory.rs:262-281`) |
| B5 | События трассировки «формируются и выбрасываются»: подписчик ставится НЕ всегда, а только под условием/флагом (решение D11 плана) | **ПОДТВЕРЖДЕНО** | Точное условие: `crates/vibe-index/src/cli/serve.rs:67` `if args.auto_commit_push { … :68-73 установка подписчика … }` — флаг CLI `--auto-commit-push` (объявлен serve.rs:42-43); комментарий serve.rs:62-66: «both the subscriber and the gate are gated on the flag». Весь остальной vibe-index (все CLI-команды кроме serve) подписчика не ставит вовсе. Единственные живые `tracing::warn!` в vibe-index — оба внутри auto-commit-push-пути: `crates/vibe-index/src/server/routes/packages.rs:402` и `:410` |
| B6 | Диагностика `manifest_epoch` живёт в крейте `vibe-check`, метит каждый локально видимый манифест с `[package]` (периметр — корень проекта плюс `packages/**`), выдавая `info` | **ПОДТВЕРЖДЕНО** | Крейт vibe-check: `crates/vibe-check/src/checks/manifest_epoch.rs:34-35` (`pub struct ManifestEpochCheck`), registered id `"manifest_epoch"` — `crates/vibe-check/src/lib.rs:137`. Периметр: `scan_local_packages` — `crates/vibe-check/src/checks/mod.rs:50-74`: корневой `vibe.toml` (`:52-54`) + walkdir по `packages/` c `max_depth(4)` (`:55-72`). Уровень: `report.info(...)` — `manifest_epoch.rs:57`; severity `Info` подтверждена тестом `manifest_epoch.rs:116`. Пропуск без `[package]` — `:49-51`, пропуск непарсящегося — `:45-48` |

## 3. Анатомия PackageMeta

### 3.1. Объявление дословно

`crates/vibe-core/src/manifest/package.rs:75-137` (начало `:75`, конец `:137`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMeta {
    pub name: String,
    /// … (doc)
    pub group: Group,
    pub kind: PackageKind,
    pub version: semver::Version,
    /// `[package].epoch` — … (PROP-044 §6.2, doc :85-92)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<u32>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// … (doc :105-107)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<Purl>,
    /// … (doc :110-112)
    #[serde(default, skip_serializing_if = "PublishPosture::is_default")]
    pub publish: PublishPosture,
    /// … (doc :115-118)
    #[serde(default, skip_serializing_if = "Materialization::is_default")]
    pub materialization: Materialization,
    /// … (doc :121-125)
    #[serde(default, skip_serializing_if = "is_false")]
    pub bridge: bool,
    /// … (doc :128-135)
    #[serde(default, skip_serializing_if = "PackageFormat::is_default")]
    pub format: PackageFormat,
}
```

(doc-строки сокращены пометкой `/// … (doc …)` с номерами; остальные строки —
дословно. Полные doc-комментарии — в самом файле по указанным строкам.)

### 3.2. Поле `epoch` дословно — форма, которой обязан следовать `frozen`

`crates/vibe-core/src/manifest/package.rs:93-94`:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<u32>,
```

То есть: `Option<T>` + `default` + `skip_serializing_if = "Option::is_none"`
— отсутствует ⇒ не пишется (pre-epoch сохраняется, тест
`crates/vibe-core/src/manifest/package/tests.rs:342-352`: сериализация без
`epoch` не содержит строки `epoch`).

### 3.3. Boolean-поля в PackageMeta — прецеденты для `frozen`

`Option<bool>` — **ни одного** в `PackageMeta`. Есть ровно один «голый» bool:

- `bridge: bool` — `crates/vibe-core/src/manifest/package.rs:126-127`:
  ```rust
  #[serde(default, skip_serializing_if = "is_false")]
  pub bridge: bool,
  ```
  Хелпер `is_false` — `package.rs:139-142`:
  ```rust
  /// `skip_serializing_if` helper for boolean fields that default to `false`.
  fn is_false(b: &bool) -> bool {
      !*b
  }
  ```
  Поведение при записи: пишется только `true`; `false` опускается.
- Соседний bool-носитель: `PublishPosture::All(bool)` (untagged-вариант,
  `package.rs:227-236`), опускается целиком, когда `All(true)` — атрибут
  `#[serde(default, skip_serializing_if = "PublishPosture::is_default")]`
  (`package.rs:113-114`).

Вывод для Ф1.4: если `frozen` задуман как `Option<bool>` — следовать форме
`epoch` (`:93-94`); если как bool с семантикой «только явное true значимо» —
в дереве уже есть готовый прецедент `bridge` (+ переиспользуемый хелпер
`is_false`).

### 3.4. Строгость разбора манифеста

`deny_unknown_fields` стоит на ВСЕХ манифестных типах; ключевые:
- `PackageMeta` — `crates/vibe-core/src/manifest/package.rs:76`;
- `Manifest` (корневой документ) — `crates/vibe-core/src/manifest/document.rs:66-67`;
- `Compatibility` — `package.rs:374`; `BootSnippet` — `package.rs:542`;
  `BootSection` — `document.rs:269`; `ProjectSection`/`[project]`-семейство —
  `crates/vibe-core/src/manifest/project.rs:38,65,84,109,348,375`;
  сабмодули `package/*` — `crates/vibe-core/src/manifest/package/{wire.rs:21,39,
  hooks.rs:34, weak_deps.rs:28,52, capabilities.rs:30,174,189,211,236,
  skill.rs:56, binary.rs:37, mcp_server.rs:46}`; `i18n.rs:48`;
  lockfile-типы — `crates/vibe-core/src/manifest/lockfile.rs:78,103,163,254,376`;
  subskill — `crates/vibe-core/src/manifest/subskill.rs:43,77,165,243,272,298`;
  redirect — `crates/vibe-core/src/manifest/redirect.rs:43,145`.

Следствие: добавить `frozen` в `PackageMeta` для СТАРЫХ бинариев `vibe` —
жёсткая ошибка разбора (поле неизвестно). Мест без `deny_unknown_fields` на
манифестных типах не найдено.

### 3.5. Где манифест ЗАПИСЫВАЕТСЯ (сериализуется)

- Функция: `Manifest::write` — `crates/vibe-core/src/manifest/document.rs:307-309`:
  ```rust
  /// Write the manifest to disk, preserving operator comments where the
  /// existing file carries any.
  pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
      write_toml(path, self)
  }
  ```
- `write_toml` — `crates/vibe-core/src/manifest/mod.rs:65-80`: сериализует
  **из структуры** (`toml::to_string_pretty(value)` — `:71`), затем, если файл
  существует, переносит декорации (комментарии/отступы) старого файла поверх
  свежего рендера через `merge_preserving_comments(&existing, &rendered)`
  (`:72-75`). Это НЕ правка документа на месте: старый документ — источник
  только декораций, структура целиком из памяти.
- Производственные вызовы `Manifest::write`: `crates/vibe-install/src/plan.rs:146`,
  `crates/vibe-install/src/apply.rs:109`,
  `crates/vibe-cli/src/commands/init/helpers.rs:300`,
  `crates/vibe-cli/src/commands/install/resolver/git_source_flag.rs:102`,
  `crates/vibe-cli/src/commands/uninstall.rs:144` (плюс workspace-стейджинг
  publish: `crates/vibe-workspace/src/publish/staging.rs:128`). Все строят
  структуру и сериализуют; поле с `skip_serializing_if` доедет до файла
  автоматически, как только оно в `PackageMeta`.

### 3.6. Кто пишет `epoch = 1` в новые манифесты

Ровно **одна** точка в `crates/**`: шаблон `vibe init` (пакетный пресет) —
`crates/vibe-cli/src/commands/init/package.rs:141-149`:

```rust
        let manifest_text = format!(
            "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"{kind}\"\n\
             version = \"{version}\"\nepoch = 1\n{authors_line}\
             license = \"{license}\"\ndescription = \"{description}\"\nformat = \"{format}\"\n\n\
             [boot_snippet]\nsource = \"spec/boot/10-tool-{name}.md\"\ncategory = \"tool\"\nlink = \"{link}\"\n",
            ...
        );
        fs::write(&manifest_path, &manifest_text)?;
```

(`epoch = 1` — `init/package.rs:143`; запись — `fs::write` `:150`.) Других
шаблонов с `epoch = 1` в `crates/**` нет (`rg -n "epoch = 1" crates/` —
хиты только в doc-текстах package.rs:86,90, manifest_epoch.rs и тестовых
строках `manifest_epoch.rs:140`).

## 4. Путь проекции «манифест → VersionEntry»

### 4.0. Исчерпывающий список строителей записи из манифеста

`rg -n "VersionEntry" crates/vibe-index/src` (вывод целиком — в §10) даёт
инлайн-литералы `VersionEntry {` в: `cli/add.rs:84`, `scanner/org_walk.rs:202`,
`types/entry/mod.rs:138` (`VersionEntry::minimal` — фиче-строитель, манифеста
не видит) и тестах (`types/entry/tests.rs:10`, `index/by_name.rs:140`,
`index/memory.rs:322`, `index/inverted.rs:297`, `index/primary.rs:117`).
Производственных проекторов из манифеста — **два**. Сканеры `from_clones.rs` и
`from_github.rs` НЕ строят записи сами — оба делегируют
`scan_org_dir_with_filter` (`crates/vibe-index/src/scanner/from_clones.rs:29`,
`crates/vibe-index/src/scanner/from_github.rs:101`), а драйвер `reindex.rs`
только переносит готовые `report.entries` через `next.upsert(entry.clone())`
(`crates/vibe-index/src/cli/reindex.rs:274,279`). Третий ingress — HTTP
`POST /v1/packages` принимает уже готовую запись (`Json<VersionEntry>` —
`crates/vibe-index/src/server/routes/packages.rs:232`), из манифеста её не
строит. `vibe-publish` строит только крошечный `HookEnvelope {kind,name,
version,registry}` (`crates/vibe-publish/src/post_hook.rs:381-387`) — не
проектор.

### 4.1. Карточка: `vibe-index add` — `crates/vibe-index/src/cli/add.rs`

- Функция и сигнатура: `pub fn run(args: Args) -> Result<()>` —
  `crates/vibe-index/src/cli/add.rs:48`.
- Вход: `mfst::parse_manifest(&manifest_bytes)` (add.rs:64) →
  `mfst::require_package(&manifest)` → `&PackageMeta` (add.rs:65).
- Отображение полей (литерал add.rs:84-115):

| манифест → запись | источник → приёмник | цитата |
|---|---|---|
| `pkg.kind` → `kind` | через `mfst::package_kind` | add.rs:68 → :86 |
| `pkg.group` → `group` | клон | add.rs:69 → :87 |
| `pkg.name` → `name` | клон | add.rs:70 → :88 |
| `pkg.version` → `version` | клон | add.rs:71 → :89 |
| `pkg.license` → `license` | клон | add.rs:96 |
| `pkg.authors` → `authors` | клон | add.rs:97 |
| `pkg.description` → `description` | клон | add.rs:98 |
| `pkg.homepage` → `homepage` | клон | add.rs:99 |
| `pkg.keywords` → `keywords` | клон | add.rs:100 |
| `pkg.describes` → `describes` | `map(\|p\| p.to_string())` | add.rs:101 |
| `manifest.compatibility` → `compatibility` | `mfst::compatibility_from` | add.rs:102 → manifest.rs:77-82 |
| `manifest.provides` → `provides` | `mfst::provides_from` | add.rs:103 → manifest.rs:84-88 |
| `manifest.requires` → `requires` | `mfst::requires_from` | add.rs:104 → manifest.rs:95-113 |
| `manifest.requires_any` → `requires_any` | `mfst::requires_any_from` | add.rs:105 → manifest.rs:115-121 |
| `manifest.obsoletes` → `obsoletes` | `mfst::obsoletes_from` | add.rs:106 → manifest.rs:123-127 |
| `manifest.conflicts` → `conflicts` | `mfst::conflicts_from` | add.rs:107 → manifest.rs:129-133 |
| `manifest.features` → `features` | `mfst::features_from` | add.rs:108 → manifest.rs:135-140 |
| `pkg_root` (ФС) → `subskills` | `mfst::collect_subskills` | add.rs:109 → manifest.rs:199-227 |
| `manifest.i18n` → `i18n` | `mfst::i18n_from` | add.rs:110 → manifest.rs:142-147 |
| `manifest.boot_snippet` → `boot_snippet` | `mfst::boot_snippet_from` | add.rs:111 → manifest.rs:149-154 |
| `manifest.origin` → `workspace_origin` | `mfst::workspace_origin_from` | add.rs:95 → manifest.rs:159-167 |

- Поля записи НЕ из манифеста: `content_hash` — хэш каталога манифеста
  (`compute_content_hash(pkg_root)` add.rs:73); `source_ref` — флаг или
  `v{version}` (add.rs:74); `source_url` — флаг или композ из
  `repomd`-настроек (add.rs:75-77, `compose_default_repo_url` add.rs:126-136);
  `resolved_commit` — флаг CLI (add.rs:93); `registry` — из индекса
  (add.rs:94); `files_count` — обход walkdir (add.rs:78-82); `indexed_at` —
  `Utc::now()` (add.rs:113); `indexed_by` — версия бинария (add.rs:114);
  `schema_version` — константа (add.rs:85).

### 4.2. Карточка: орг-сканер — `crates/vibe-index/src/scanner/org_walk.rs`

- Функция и сигнатура: `fn build_entry(repo: &Path, repo_name: &str, tag:
  &str, version: Version, opts: &FromClonesOptions) -> Result<VersionEntry>` —
  `crates/vibe-index/src/scanner/org_walk.rs:172-178`.
- Вход: чекаут тега во временном каталоге → `mfst::parse_manifest` (org_walk.rs:190) →
  `mfst::require_package` (org_walk.rs:191).
- Отображение — литерал org_walk.rs:202-233; **скалярная часть — построчный
  дубликат add.rs**:

| манифест → запись | источник → приёмник | цитата |
|---|---|---|
| `pkg.kind` → `kind` | `mfst::package_kind` | org_walk.rs:197 → :204 |
| `pkg.group` → `group` | клон | org_walk.rs:205 |
| `pkg.name` → `name` | клон | org_walk.rs:206 |
| `version` (из git-тега!) → `version` | параметр, НЕ манифест | org_walk.rs:207 |
| `pkg.license`/`authors`/`description`/`homepage`/`keywords`/`describes` | клоны | org_walk.rs:214-219 |
| `manifest.compatibility`/`provides`/`requires`/`requires_any`/`obsoletes`/`conflicts`/`features` | те же `mfst::*` | org_walk.rs:220-226 |
| `&snapshot` (ФС) → `subskills` | `mfst::collect_subskills` | org_walk.rs:200 → :227 |
| `manifest.i18n` → `i18n`; `manifest.boot_snippet` → `boot_snippet`; `manifest.origin` → `workspace_origin` | те же `mfst::*` | org_walk.rs:228-229, :213 |

- Поля НЕ из манифеста: `content_hash` — хэш снапшота тега (org_walk.rs:193);
  `source_url` — композ из опций (`source_url_for` org_walk.rs:209, :242-252);
  `source_ref` — сам тег (org_walk.rs:210); `resolved_commit` —
  `git_cli::resolve_commit` (org_walk.rs:194, :211); `registry` — из опций
  (org_walk.rs:212); `files_count` — обход (org_walk.rs:195, :230);
  `indexed_at` — ЕДИНЫЙ на прогон из опций, не `now()` (org_walk.rs:231,
  FromClonesOptions.indexed_at :36); `indexed_by` — `opts.generator`
  (org_walk.rs:232).
- Расхождение с add.rs: **version берётся из git-тега, не из манифеста**
  (`parse_v_tag`, org_walk.rs:145,237-240); `indexed_at` детерминирован.

### 4.3. Один проектор или несколько?

**Проектора-функции «манифест → VersionEntry» НЕТ.** Есть общий слой
помощников по СЕКЦИЯМ (`scanner/manifest.rs`: `compatibility_from` :77,
`provides_from` :84, `requires_from` :95, `requires_any_from` :115,
`obsoletes_from` :123, `conflicts_from` :129, `features_from` :135,
`i18n_from` :142, `boot_snippet_from` :149, `workspace_origin_from` :159,
`collect_subskills` :199) — но скалярные `[package]`-поля каждый литерал
мапит сам, идентично и дважды. **Мест вписывания проекции нового флага —
два** (add.rs:84-115 и org_walk.rs:202-233; форма — `frozen: pkg.frozen,`),
плюс обязательное третье касание — инициализация в `VersionEntry::minimal`
(entry/mod.rs:138-169), иначе конструкция не соберётся. `deny_unknown_fields`
на `VersionEntry` (entry/mod.rs:38) не влияет на Rust-сборку литералов, но
см. §9 про старых читателей.

## 5. Доезжает ли epoch до каталога

**НЕТ. Никаким путём.** Команда `rg -n "epoch" crates/vibe-index/src` —
**пустой вывод** (0 совпадений; см. §10). Цепочка обрывается в двух местах
одновременно:

1. В самой записи: `VersionEntry` (crates/vibe-index/src/types/entry/mod.rs:43-121)
   поля `epoch` не имеет — от `schema_version: u32` (:44) до
   `indexed_by: String` (:120) его нет.
2. В проекторах: оба литерала читают из `PackageMeta` только
   `kind/group/name/version/license/authors/description/homepage/keywords/describes`
   (add.rs:68-71,96-101; org_walk.rs:197,205-206,214-219) — `pkg.epoch` не
   трогают.

Итог для калибровки: чтобы новое манифестное поле «доехало до записи»,
нужно: (а) поле в `VersionEntry` + `minimal`; (б) строка в add.rs; (в) строка
в org_walk.rs. Никакой готовой трубы нет — `epoch`, положенный Ф1.2, до
каталога сегодня не доехал.

## 6. Трассировка в пути чтения

### 6.1. Все места установки подписчика в `crates/**`

Ровно **два** (`rg -n "tracing_subscriber|set_global_default|EnvFilter|fmt\(\)\.init|try_init" crates/`):

1. `crates/vibe-cli/src/main.rs:408-417` — `init_tracing()`, вызывается
   безусловно из `main` (`main.rs:52`):
   ```rust
   fn init_tracing() {
       use tracing_subscriber::{EnvFilter, fmt};

       let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
       let _ = fmt()
           .with_env_filter(filter)
           .with_target(false)
           .with_writer(std::io::stderr)
           .try_init();
   }
   ```
   Условие: нет (все команды `vibe`). Дефолтный уровень — `warn` (через
   EnvFilter `"warn"`), env-переключатель `VIBE_LOG`, writer stderr.
2. `crates/vibe-index/src/cli/serve.rs:67-73` — только при флаге
   `--auto-commit-push` (объявление :42-43):
   ```rust
   if args.auto_commit_push {
       let filter = tracing_subscriber::EnvFilter::try_from_default_env()
           .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
       let _ = tracing_subscriber::fmt()
           .with_env_filter(filter)
           .with_writer(std::io::stderr)
           .try_init();
       crate::publish::preflight(&args.data_dir)?;
   }
   ```
   Условие: `args.auto_commit_push`. Дефолтный уровень `warn`, env —
   `RUST_LOG` (`try_from_default_env`), writer stderr. Причина привязки —
   сам код: «its WARN logs must be observable — so both the subscriber and
   the gate are gated on the flag» (serve.rs:62-66).

`vibe-settings`-хиты по слову «subscriber» — не трассировка, это свой
`EventEmitter` (`crates/vibe-settings/src/events/mod.rs:240-316`).

### 6.2. Зависимости от tracing / tracing-subscriber

- Корневой workspace: `tracing = "0.1"`, `tracing-subscriber = { version =
  "0.3", features = ["env-filter"] }` — `Cargo.toml:125-126`.
- `tracing` (без subscriber) зависят: `vibe-graph` (Cargo.toml:17),
  `vibe-publish` (:19), `vibe-resolver` (:18), `vibe-registry` (:18),
  `vibe-install` (:25), `vibe-mcp` (:26), `vibe-cli` (:55), `vibe-core`
  (:23), `vibe-index` (:37).
- `tracing-subscriber` — ровно два: `vibe-cli` (Cargo.toml:56) и
  `vibe-index` (Cargo.toml:38). Ни один крейт-библиотека не ставит
  подписчика сама (правильно: подписчик — дело бинария).

### 6.3. Главный вопрос: виден ли `tracing::warn!` из `Index::load_from`?

Путь чтения: `Index::load_from` (`crates/vibe-index/src/index/memory.rs:262-281`)
→ `repomd::read` (:263) + `by_name::read_all` (:264) →
`by_name::parse`/`serde_json::from_slice` (`crates/vibe-index/src/index/by_name.rs:66-68`)
→ внутри `PackageEntry.versions: Vec<VersionEntry>` (aggregate.rs:32), каждый
декодируется строгим `VersionEntry` (`deny_unknown_fields`,
entry/mod.rs:38). `primary.jsonl` этим путём НЕ читается (см. §9).

- **(а) CLI-команды `vibe-index` (`add`,`remove`,`reindex`,`list`,`get`,
  `dump`,`search`,`outdated`,…): НЕ ВИДЕН.** `crates/vibe-index/src/main.rs:6-14`
  подписчика не ставит (там только `eprintln!("error: {e}")` :10 наErr);
  единственная установка подписчика в крейте — serve.rs:67-73 (см. выше).
  warn из load-пути будет сформирован и выброшен.
- **(б) `serve`: ВИДЕН ТОЛЬКО С `--auto-commit-push`.** `Index::load_from`
  вызывается на serve.rs:77 — ПОСЛЕ блока установки подписчика (:67-75),
  поэтому при включённом флаге warn из чтения виден (stderr); без флага
  подписчика нет — не виден.
- Дополнительно: **CLI `vibe`** подписчика ставит всегда (main.rs:52), и
  `vibe-publish` уже варнит в этот поток (`crates/vibe-publish/src/post_hook.rs:333,
  :346, :365` — `warn!(target: "vibe_publish::post_hook", …)`), но vibe не
  зависит от vibe-index, так что на load-путь vibe-index это не влияет.

### 6.4. Домашняя форма диагностики vibe-index (как предупредить, чтобы не быть чужеродным)

Счётчик §7: `rg -c "warn!|eprintln!|println!" crates/vibe-index/src` — 17
файлов, 71 строка-совпадение суммарно (main.rs:1, capabilities.rs:4,
org_walk.rs:1, add.rs:1, packages.rs:2, verify.rs:9, outdated.rs:4,
remove.rs:1, stop.rs:4, purls.rs:4, reindex.rs:13, init.rs:1, serve.rs:2,
search.rs:5, list.rs:5, dump.rs:2, get.rs:12; сумма 71).

Разбивка по макросам:
- `warn!` — **2 вызова** (третий хит — doc-строка `org_walk.rs:9`), оба в
  auto-commit-push сервера: `crates/vibe-index/src/server/routes/packages.rs:402`
  ```rust
  tracing::warn!(
      error = %e,
      "auto-commit-push failed after a successful mutation; \
       the write stands and the index retries on the next mutation"
  );
  ```
  и `packages.rs:410` `tracing::warn!(error = %e, "auto-commit-push task join failed");`
- `eprintln!` — **3 строки**: `main.rs:10` `eprintln!("error: {e}");`,
  `serve.rs:122-128` (баннер запуска), `serve.rs:141`
  `eprintln!("vibe-index: SIGINT received, shutting down");`
- `println!` — **68 строк** (71 минус 3 eprintln; паттерн `println!`
  матчит и строки с `eprintln!`): все человекочитаемые выводы CLI.

Форма «неполадка, которая не ошибка» в этом крейте сегодня — вообще не
макрос: сканер собирает **`SkipNote`** (`org_walk.rs:55-60`, пушатся на
каждый пропуск :102,112,136,146,155) и репортит их вызывающему. Т.е. новое
предупреждение «пропускаю запись с незнакомой возможностью» органичнее
всего либо (1) `tracing::warn!` — но тогда нужно, чтобы подписчик стоял на
пути чтения (сегодня его нет, §6.3), либо (2) `eprintln!` в духе
`serve.rs:141`, либо (3) канал в стиле SkipNote, если чтение должно
остаться чистым. Решение за планом; в дереве прямого прецедента «warn из
load-пути, видимый в CLI» НЕТ.

## 7. Поверхности, показывающие версию

Замер, ничего не менял. «Фикс.» = фиксированный набор полей (добавление поля
= правка структуры строки/конвертера); «Целиком» = сериализует всю
`VersionEntry` — новое поле доедет само.

### vibe-index CLI

| поверхность | точка рендера | форма |
|---|---|---|
| `vibe-index list` (text) | `crates/vibe-index/src/cli/list.rs:102-117` (`group/name @ latest`, описание) | Фикс. (`PackageRow` list.rs:45-55) |
| `vibe-index list --json` | list.rs:86-100 (Envelope → PackageRow) | Фикс. |
| `vibe-index get` (text) | `crates/vibe-index/src/cli/get.rs:119-141` (`render_text`: group/name/kind/latest stable/versions/`- {version} (commit …)`/content_hash/source_url) | Фикс. |
| `vibe-index get --json` | get.rs:100-112, `GetEnvelope.versions: Vec<&VersionEntry>` (get.rs:40) | **Целиком** (поле доедет само) |
| `vibe-index dump --format jsonl` | `crates/vibe-index/src/cli/dump.rs:41-52` (`serde_json::to_string(entry)` :43) | **Целиком** |
| `vibe-index dump --format json` | dump.rs:54-71 (`"entries": entries` :65) | **Целиком** |
| `vibe-index search` (text/json) | `crates/vibe-index/src/cli/search.rs:76-89` / :54-75 (`HitRow` :39-47, показывает `latest_stable`) | Фикс. |
| `vibe-index outdated` (text/json) | `crates/vibe-index/src/cli/outdated.rs:96-115` / :84-95 (`Row` :38-46, installed/latest) | Фикс. |
| `vibe-index capabilities` (text/json) | `crates/vibe-index/src/cli/capabilities.rs:68-81` — `{}:{} @ {version}` (:77); Row :37/:49 (`version: e.version.clone()`) | Фикс. |
| `vibe-index purls` (text/json) | `crates/vibe-index/src/cli/purls.rs:70-82` — kind/name/version/binding_site (:81); Row :37/:57 | Фикс. |

### HTTP-сервер vibe-index

| поверхность | точка рендера | форма |
|---|---|---|
| `GET /v1/packages` (list/search) | `crates/vibe-index/src/server/routes/packages.rs:90-155`; `PackageRow` :42-52, `SearchHit` :62-71 | Фикс. |
| `GET /v1/packages/{group}/{name}` | packages.rs:157-175; `PackageVersionsResponse` :177-185 — верх фиксирован, но `versions: Vec<VersionEntry>` (:184) | Верх фикс., версии **целиком** |
| `GET /v1/packages/{group}/{name}/{version}` | packages.rs:187-211 — `Result<Json<VersionEntry>, _>` (:190) | **Целиком** |
| `POST /v1/packages` (upsert) | packages.rs:229-279 — принимает `Json<VersionEntry>` (:232), отвечает `UpsertResponse` :219-227 (фикс.) | приём **целиком**; ответ фикс. |
| Статика: `GET /primary.jsonl[.gz]`, `/repomd.json`, `/by-name/*` | `crates/vibe-index/src/server/routes/index_files.rs:17-29` (+ дальше по файлу) — файл с диска байт-в-байт | **Целиком** (байты с диска) |

### vibe CLI (`vibe`)

Замечание: `vibe` рендерит **локфайл** (`vibe.lock`, `LockedPackage` —
`crates/vibe-core/src/manifest/lockfile.rs:255-…`), а НЕ запись каталога;
в каталог ходит через `vibe-registry` view-типы (только version, §8).

| поверхность | точка рендера | форма |
|---|---|---|
| `vibe list` | `crates/vibe-cli/src/commands/list.rs:41-76` — `JsonEntry` (версия :82); текст — тот же файл далее | Фикс. (локфайл-поля) |
| `vibe show` (subskills/purls/features/effective) | `crates/vibe-cli/src/commands/show/*` — читают локфайл (`show/effective.rs:47-51,116` — `entry.version` в URI `:116`); `show/config.rs:277,304` — версия ПРОЕКТА, не пакета | Фикс. (локфайл-поля) |
| `vibe tree` | `crates/vibe-cli/src/commands/tree/build.rs:266-286` — `Package { version: p.version.to_string() … }` (:271, из локфайла) | Фикс. |
| `vibe search` | `crates/vibe-cli/src/commands/search.rs:93,361,389` (`latest_stable`), :406 | Фикс. (зеркалит SearchHit сервера) |
| `vibe search purl` | `crates/vibe-cli/src/commands/search/purl.rs:38,202` | Фикс. |
| `vibe outdated` | `crates/vibe-cli/src/commands/outdated.rs:85-110` (installed/latest/status; `probe_latest` через резолвер :165-178) | Фикс. |
| `vibe install/update/reinstall` отчёты | `crates/vibe-cli/src/commands/install/report.rs:20-26`, `update.rs:163,214,243,393`, `reinstall.rs:230` (версии разрешённых пакетов) | Фикс. |
| `vibe mcp` (список серверов) | `crates/vibe-cli/src/commands/mcp/mod.rs:132,160` (version строки) | Фикс. |
| `vibe show effective` URI | `show/effective.rs:116,145` — `spec://{group}/{name}/{version}` | Фикс. |

### MCP-инструменты (`crates/vibe-mcp/src/tools.rs`)

| инструмент | точка рендера | форма |
|---|---|---|
| `read_lockfile_entry` (lookup установленного пакета) | `crates/vibe-mcp/src/tools.rs:172-187` — `json!({ … "version": entry.version.to_string() … })` (:175); описание инструмента :124 перечисляет поля | Фикс. (ручной json! из `LockedPackage`) |
| `read_subskill` | tools.rs:273, :412 — `join(format!("v{}", entry.version))` в пути (версия в пути, не в выводе) | — |

Что трогать при добавлении одного поля статуса замороженности (D14: каждая
поверхность с версией обязана показывать статус): «Целиком»-поверхности —
ничего (поле доедет); «Фикс.»-поверхности — по одному конвертеру/строке на
каждую строку таблицы выше (в vibe-семействе — только если поле протащат в
`LockedPackage` и резолвер; в MCP-инструменте — руками в json! tools.rs:172-187).

## 8. Клиентские view-типы

B3 — см. §2. Полный список типов `crates/vibe-registry/src/index_client/wire.rs`:

```rust
// wire.rs:17-21
#[derive(Debug, Deserialize)]
pub(super) struct NameEntryView {
    #[serde(default)]
    pub packages: Vec<PackageEntryView>,
}

// wire.rs:23-28
#[derive(Debug, Deserialize)]
pub(super) struct PackageEntryView {
    pub group: Group,
    #[serde(default)]
    pub versions: Vec<VersionEntryView>,
}

// wire.rs:30-33
#[derive(Debug, Deserialize)]
pub(super) struct VersionEntryView {
    pub version: Version,
}

// wire.rs:44-52
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchResults {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub hit_count: usize,
    #[serde(default)]
    pub hits: Vec<SearchHit>,
}

// wire.rs:55-67
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: PackageKind,
    pub name: String,
    #[serde(default)]
    pub latest_stable: Option<Version>,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub matched_tokens: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

// wire.rs:71-79
#[derive(Debug, Clone, Deserialize)]
pub struct PurlLookupResults {
    #[serde(default)]
    pub purl: String,
    #[serde(default)]
    pub hit_count: usize,
    #[serde(default)]
    pub hits: Vec<PurlLookupHit>,
}

// wire.rs:83-89
#[derive(Debug, Clone, Deserialize)]
pub struct PurlLookupHit {
    pub kind: PackageKind,
    pub name: String,
    pub version: Version,
    pub binding_site: BindingSite,
}
```

(плюс enum `BindingSite` wire.rs:92-99 — к записям каталога отношения не имеет.)

**Сломает ли клиента появление новых полей?** — **НЕТ, ни одно.** Ни один
view-тип не несёт `deny_unknown_fields` (атрибутов строгости на
`NameEntryView`/`PackageEntryView`/`VersionEntryView` нет вовсе — wire.rs:17,
:23,:30 — только `#[derive(Debug, Deserialize)]`); serde по умолчанию
игнорирует неизвестные поля. Doc подтверждает намерение: «Only the fields the
resolver's version selector needs are read; **the rest of the on-disk shape
is tolerated**» (wire.rs:14-16); для SearchResults — «Extra fields on the
wire … are tolerated silently» (wire.rs:35-39); тест
`name_entry_view_extracts_candidate_groups` (wire.rs:201-222) прямо
декодирует JSON с полями `name`/`indexed_at`, отсутствующими во view.

- Новые `must_understand`/`yanked`/`frozen` в `VersionEntry`: декодирование
  не сломается; **но** клиент их молча не заметит — `VersionEntryView`
  выбирает версии только по `version` (wire.rs:32), т.е. yanked/frozen-версия
  останется кандидатом резолвера, пока view не расширен. Это
  семантический риск (silent-ignore), не поломка.
- `tombstone` в `NameEntry` (`crates/vibe-index/src/types/entry/aggregate.rs:69-76`):
  клиента не сломает (`NameEntryView` читает только `packages`, wire.rs:19-20),
  но tombstone-пакеты будут выглядеть живыми кандидатами — снова
  silent-ignore. Единственное обязательное поле — `group`
  (`PackageEntryView` wire.rs:25, без `default`); `versions` дефолтится к
  пустому (wire.rs:26-27).

## 9. Дыры и неожиданности

1. **`deny_unknown_fields` на `VersionEntry` — форвард-совместимости нет.**
   `crates/vibe-index/src/types/entry/mod.rs:38`. Любое НОВОЕ поле записи
   (`frozen`, `must_understand`, `yanked`), будучи однажды записанным в
   каталог, сделает строку НЕЧИТАЕМОЙ для СТАРЫХ бинариев vibe-index: parse
   валится на первой же такой строке — `crates/vibe-index/src/index/primary.rs:92-97`
   (`serde_json::from_str(line).map_err(… Malformed … line {lineno})`, тест
   `malformed_line_surfaces_with_lineno` primary.rs:228-235) и
   `crates/vibe-index/src/index/by_name.rs:66-68,94` (`out.push(parse(&bytes)?)` —
   `?` прерывает ВЕСЬ `read_all`). Смягчители: (а) все Option-поля со
   `skip_serializing_if` не пишутся, пока `None` — старые читатели видят
   старую форму, пока флаг не установлен; (б) НО при первом `frozen = true`
   весь каталог становится нечитаемым для старых версий. План («читатель
   каталога пропускает запись с незнакомой возможностью с warn-логом»)
   обязан учесть, что сегодня пропуск-with-warn в пути чтения НЕ существует —
   вместо него жёсткий `Error::Malformed` на весь файл.
2. **`Index::load_from` читает НЕ `primary.jsonl`.** B4 подразумевает
   «каталог с диска»; фактически — `repomd.json` + `by-name/*.json`
   (`crates/vibe-index/src/index/memory.rs:263-264`). `primary::read`
   (`primary.rs:75-82`) в дереве вызовов не значится ни разу (rg — единственный
   потребитель его тесты; см. §10). «Читатель каталога», которому учат
   пропускать записи, — это `by_name::parse`/`read_all`, а не primary.
   (`vibe-index verify` проверяет только sha256/размеры файлов против
   `repomd.json` — `crates/vibe-index/src/cli/verify.rs:73-118`.)
3. **Дублированная скалярная проекция.** Скалярные `[package]`-поля
   мапятся в запись в двух независимых литералах (add.rs:96-101 ↔
   org_walk.rs:214-219) — общего `pkg_meta → VersionEntry`-проектора нет;
   Ф1.4 либо правит оба места (+`minimal`), либо сначала выделяет
   проектор. Забытый второй литерал = флаг доезжает через `add`, но теряется
   при `reindex`.
4. **`version` в орг-сканере берётся из git-тега, не из манифеста**
   (org_walk.rs:145,207): манифестная `version` в этом пути вообще не
   читается. Любая проекция «манифестных» полей в org_walk обязана это
   помнить.
5. **Подписчик vibe-index привязан к `--auto-commit-push`**, а не к чтению:
   warn из `Index::load_from` невидим во всех CLI-командах vibe-index и в
   `serve` без флага (§6.3). План-посылка D11 «формируются и выбрасываются»
   подтверждена ровно в этом виде.
6. **Счётчик §7 по паттерну `println!` матчит и `eprintln!`-строки**
   (подстрока): 71 строка = 68 println + 3 eprintln. Для чистого счёта
   `println!` нужно вычитание или негативный просмотр назад (rg его не
   умеет). Аналогично `warn!` даёт 3 хита, из них 1 — doc-строка
   (org_walk.rs:9); реальных вызовов 2.
7. **`manifest_epoch`-диагностика не видит манифесты глубже 4 уровней**
   внутри `packages/` (`crates/vibe-check/src/checks/mod.rs:58` —
   `max_depth(4)`); глубже лежащий `vibe.toml` не метится. Для текущей
   схемы `packages/<group>/<name>/v<version>/vibe.toml` (4 уровня — как в
   тесте manifest_epoch.rs:108-110) хватает впритык.
8. **`vibe init` пишет `epoch = 1` ровно в одном пресете** — пакетном
   (`crates/vibe-cli/src/commands/init/package.rs:143`); прочие пресеты
   init (project-семейство, `init/helpers.rs`, `init/prompts.rs`) `epoch` не
   пишут — но они и не `[package]`, так что это не дыра, а граница.
9. **Клиент индекса игнорирует новые поля молча** (§8): поломки нет, но
   `yanked`/`frozen`/`tombstone` без расширения view-типов не окажут на
   резолвер никакого эффекта — поле есть на диске, семантики нет.

## 10. Как воспроизвести этот замер

Все команды — из корня рабочего дерева, читающие; ничего не меняют.

1. Анатомия `PackageMeta` и B1:
   `rg -n "epoch" crates/vibe-core/src/manifest/package.rs`
   (строки 85-94); файл целиком — Read `crates/vibe-core/src/manifest/package.rs`.
2. «epoch не доезжает до каталога» (§5):
   `rg -n "epoch" crates/vibe-index/src` → пусто (exit 1).
3. Исчерпывающий список строителей записи (§4.0):
   `rg -n "VersionEntry" crates/vibe-index/src` → литералы в
   `cli/add.rs:84`, `scanner/org_walk.rs:202`, `types/entry/mod.rs:138` (+5 тестовых);
   делегирование сканеров: `rg -n "compute_content_hash|scan_org_dir|build_entry" crates/vibe-index/src`.
4. Подписчики трассировки / B5 (§6.1):
   `rg -n "tracing_subscriber|set_global_default|EnvFilter|fmt\(\)\.init|try_init" crates/`
   → ровно serve.rs:68-73 и vibe-cli main.rs:409-416 (+doc-хит
   `show/config.rs:120`).
5. Домашняя форма диагностики (§6.4):
   `rg -c "warn!|eprintln!|println!" crates/vibe-index/src` (17 файлов, 71
   строк); разбивка: `rg -n "warn!" crates/vibe-index/src` (2 вызова + 1 doc),
   `rg -n "eprintln!" crates/vibe-index/src` (3).
6. B3 / клиентские view (§8):
   `rg -n "content_hash" crates/vibe-registry/src/index_client/` → 2 хита,
   оба комментарии (`mod.rs:7`, `mod.rs:394`); код — Read
   `crates/vibe-registry/src/index_client/wire.rs`.
7. B4 (§2): Read `crates/vibe-index/src/cli/serve.rs` (:77, :99-106) и
   `crates/vibe-index/src/server/state.rs` (:30).
8. B6 (§2): `rg -n "epoch" crates/vibe-check/src`;
   Read `crates/vibe-check/src/checks/manifest_epoch.rs`;
   периметр — `rg -n -A 25 "fn scan_local_packages" crates/vibe-check/src/checks/mod.rs`.
9. Кто пишет `epoch = 1` (§3.6): `rg -n "epoch = 1" crates` → шаблон
   `crates/vibe-cli/src/commands/init/package.rs:143`.
10. Запись манифеста (§3.5):
    `rg -n "pub fn (write|to_toml|serialise|serialize|render|save)\b" crates/vibe-core/src/manifest`
    → `document.rs:307`; `rg -n -A 16 "fn write_toml" crates/vibe-core/src/manifest/mod.rs`;
    вызовы: `rg -n "\.write\(&|manifest\.write|Manifest::write" crates`.
11. Путь чтения (§6.3, §9.1-9.2):
    `rg -n "primary::read|by_name::read|primary::parse|by_name::parse" crates/vibe-index/src`
    → единственный потребитель `by_name::read_all` в `memory.rs:264`;
    Read `crates/vibe-index/src/index/by_name.rs:57-97`,
    `crates/vibe-index/src/index/primary.rs:84-101`.
12. Поверхности версии (§7): Read `crates/vibe-index/src/cli/{list,get,dump,search,outdated}.rs`,
    `crates/vibe-index/src/server/routes/{packages,index_files}.rs`;
    `rg -n "latest_stable|\.versions|VersionEntry" crates/vibe-cli/src/commands crates/vibe-mcp/src`.
13. Зависимости tracing (§6.2): `rg -n "^tracing" crates/*/Cargo.toml` +
    `rg -n "tracing" Cargo.toml` (корень, строки 125-126).
