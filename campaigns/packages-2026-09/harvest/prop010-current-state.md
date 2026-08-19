# prop010-current-state — что из PROP-010 уже построено, по фактам

<замер от 2026-08-19; ветку и HEAD не указывай — git запрещён>

## 1. Что меряли и каким инструментом

Предмет — `spec/modules/vibe-registry/PROP-010-local-package-cache.md` (216 строк,
107 фактов `@fact:`; счёт в §2). По каждому факту вынесен ровно один вердикт из
четырёх: **BUILT** (истинно про сегодняшнее дерево, есть исполняющий код,
`file:line` + дословный фрагмент), **PARTLY** (исполнена часть; адрес исполненной
части + чего не хватает), **NOT BUILT** (исполняющего кода нет; прикрыт контролем
непустоты), **NOT-A-BUILD-CLAIM** (факт не утверждает ничего о коде: мотивация,
обоснование, отвергнутая альтернатива, открытый вопрос, надгробие `state="void"`,
запись истории). Политика для пограничных случаев: если мотивационный факт сам
делает проверяемое утверждение о сегодняшнем дереве (например «`--offline` shipped
with PROP-002»), вердикт выносится по этому утверждению; чистые решения-без-артефакта
(«Requirement, not an emergent side effect») — NOT-A-BUILD-CLAIM.

Инструменты — чтение файлов дерева (`crates/vibe-cli`, `crates/vibe-core`,
`crates/vibe-registry`, `crates/vibe-install`, `crates/vibe-workspace`,
`crates/vibe-index`) и адресный `grep` (Git Bash, GNU grep); для каждого
однотипного семейства поисков, вернувшего пустоту, контроль непустоты приложен
дословно в §7. Cargo не запускался, git не запускался (включая read-only).
Цитаты ниже — `file:line` плюс дословный фрагмент; якоря не изобретались: имена
фактов взяты из самого PROP-010.

## 2. Счёт фактов и сходимость

- Шаг 3.1, `grep -c '@fact:' spec/modules/vibe-registry/PROP-010-local-package-cache.md`
  → **107** (совпадает с числом в пакете; расхождений-находок нет).
- Второй, иной способ — `grep -o '@fact:' <тот же файл> | wc -l` → **107**.
  Два инструмента сошлись, поэтому 107 есть свойство предмета, а не одного
  счётчика.
- Перемерено боссом при приёмке третьим заходом: `grep -c '@fact:'` по тому же
  файлу → **107**. Расхождений нет.
- Сходимость вердиктов (таблица §4, 107 строк): **BUILT 17 + PARTLY 26 +
  NOT BUILT 21 + NOT-A-BUILD-CLAIM 43 = 107**. Разницы нет.

Главный вывод счёта: документ прав, что ядра PROP-010 (identity-склад, `vibe cache`,
глобальный `--offline`) не построено, — но неправ в нуле: **17 фактов со статусом
`spec/*` уже истинны про дерево** (§5), и ещё несколько механизмов построены
«не по-PROP-010-ски» — под другими именами и в других файлах.

## 3. Контрольный список — восемь утверждений

| # | утверждение | вердикт | доказательство |
|---|---|---|---|
| C1 | в дереве есть `default_cache_root`, и её реэкспортирует `crates/vibe-registry/src/lib.rs` | **ПОДТВЕРЖДЕНО** | `crates/vibe-registry/src/registry_cache.rs:29` — `pub fn default_cache_root() -> Result<PathBuf, RegistryError> {`; реэкспорт `crates/vibe-registry/src/lib.rs:40` — `pub use git_registry::{GitMonorepoRegistry, RegistryMeta, default_cache_root};` |
| C2 | существует подкоманда `vibe cache` (любая из path/list/add/clean) | **ОПРОВЕРГНУТО** | enum `Command` (`crates/vibe-cli/src/cli.rs:98–302`) не содержит Cache-варианта; контроль A- (§7): `grep -n "Cache" crates/vibe-cli/src/cli.rs` → пусто, тогда как A+ находит `Registry(RegistryArgs)` в `cli.rs:199`. В `RegistrySubcommand` (`cli/registry.rs:19–87`) кэш-подкоманд тоже нет |
| C3 | флаг `--offline` существует хотя бы у одной команды `vibe` | **ПОДТВЕРЖДЕНО** | `crates/vibe-cli/src/cli/pkg.rs:206–207` — `#[arg(long)]` / `pub offline: bool,` в `InstallArgs` (структура с `cli/pkg.rs:101`); док-комментарий: «PROP-030 §3.1: resolve strictly offline» |
| C4 | существует ГЛОБАЛЬНЫЙ `--offline` на корне CLI | **ОПРОВЕРГНУТО** | корневой `Cli` (`crates/vibe-cli/src/cli.rs:59–96`) несёт только `#[arg(long, global = true)]` для `json`, `quiet`, `invoked-by`, `unattended`; `offline` глобальным не является — единственное вхождение поля в `InstallArgs` (контроль B/C §7) |
| C5 | есть переменная окружения `VIBE_OFFLINE`, читаемая кодом | **ОПРОВЕРГНУТО** | контроль B- (§7): `grep -rn "VIBE_OFFLINE" crates/` → пусто, exit 1; B+ тем же способом находит `VIBE_UNATTENDED` (`output.rs:73`) |
| C6 | кэш сегодня адресуется по идентичности `(group, name, version)`, а не по URL реестра | **ОПРОВЕРГНУТО** (для машинно-глобального слоя; см. оговорку) | машинно-глобальный клон-кэш ключёван URL: `crates/vibe-registry/src/git_package_registry/mod.rs:406–408` — «Root of this registry's cache bucket — `<cache_root>/<hash>/`», хэш из `normalize_url` (`mod.rs:227–228`); `VIBEVM-SPEC.md:1548` — «Registry cache lives at `~/.vibe/registries/<canonical-url-hash>/packages/<kind>-<name>/`». Оговорка: извлечённый payload-слой ДЕЙСТВИТЕЛЬНО адресован идентичностью — `fetch.rs:288–291` (`cache_root.join(group).join(name).join("v{version}")`), но он проектный (`.vibe/cache`, `vibe-install/src/plan.rs:109`), а не машинно-глобальный |
| C7 | у `MultiRegistryResolver` есть метод-строитель `with_offline` | **ОПРОВЕРГНУТО** | контроль C- (§7): `grep -rn "with_offline" crates/` → пусто, exit 1; C+ находит соседа `mod.rs:272` — `pub fn with_strict_auth(mut self, strict: bool) -> Self {` |
| C8 | `UserConfig` уже умеет нести секции `[[registry]]` | **ОПРОВЕРГНУТО** | `crates/vibe-core/src/user_config.rs:62–90`: `UserConfig` с `#[serde(deny_unknown_fields)]` несёт только `env` (:78), `install` (:82), `init` (:89); контроль D- (§7): в `user_config.rs` нет ни `[[registry]]`, ни `[[mirror]]` (единственное вхождение слова «registry» — комментарий :40 про `VIBE_REGISTRY_CACHE`). Оговорка: машинно-глобальный файл реестров существует ОТДЕЛЬНО от UserConfig — `~/.vibe/registry.toml`, `GlobalRegistryConfig` (`global_registry.rs:22–49`) |

## 4. Вердикты по фактам, по разделам

Формат столбцов: `якорь факта` · `раздел` · `вердикт` · `адрес или контроль` · `одна фраза`.
Пути сокращены: `registry_cache.rs` = `crates/vibe-registry/src/registry_cache.rs`;
`git_pkg/` = `crates/vibe-registry/src/git_package_registry/`;
`resolver.rs` = `crates/vibe-cli/src/commands/install/resolver.rs`;
`global_registry.rs` = `crates/vibe-core/src/global_registry.rs`;
`plan.rs` = `crates/vibe-install/src/plan.rs`; `vibedeps.rs` = `crates/vibe-workspace/src/vibedeps.rs`.
«Контроль A/B/C/D/E/F/G» — дословные прогоны в §7.

| якорь факта | раздел | вердикт | адрес или контроль | одна фраза |
|---|---|---|---|---|
| @fact:milestone-line | шапка | NOT-A-BUILD-CLAIM | — | мета-строка этапа документа |
| @fact:status-line | шапка | NOT-A-BUILD-CLAIM | — | статус черновика |
| @fact:related | шапка | NOT-A-BUILD-CLAIM | — | перечень связных PROP |
| @fact:owner-sanction-line | шапка | NOT-A-BUILD-CLAIM | — | правило санкции на правки спека |
| @fact:cache-exists | §1 | BUILT | registry_cache.rs:29 `pub fn default_cache_root()`, git_pkg/mod.rs:136 `let cache_root = default_cache_root()?;` | реестровый кэш есть, install/update идут через него; но env в коде — `VIBE_REGISTRY_CACHE` (registry_cache.rs:30), а не `VIBEVM_REGISTRY_CACHE`, как пишет факт |
| @fact:cache-incidental | §1 | BUILT | контроль A: командного семейства кэша нет; RegistrySubcommand = Sync/Publish/List/Add/SetMirror/Remove/Test/Redirect/RedirectSync/RedirectUpdate/Vendor (cli/registry.rs:19–87) | кэш действительно лишён операторской поверхности — только внутренние бакеты |
| @fact:no-offline-mode | §1 | BUILT | cli/pkg.rs:206–207; plan.rs:194–209 | обе «закрытые» половины истинны: флаг install и freshness-фастпас PROP-011 |
| @fact:no-local-resolve | §1 | BUILT | cli/pkg.rs:206; resolver.rs:419–439 (PROP-030 locals); plan.rs:204–209 | все три названных механизма «сказать resolve-локально» в дереве есть |
| @fact:maven-model | §1 | NOT-A-BUILD-CLAIM | — | модель-метафора |
| @fact:consumers-already-offline | §1 | BUILT | vibedeps.rs:12–14 «`vibedeps/` is committed to the repository — a fresh clone is bootable with no `vibe install`» | потребитель уже полностью офлайн |
| @fact:CACHE-FOR-AUTHORS | §1 | NOT-A-BUILD-CLAIM | — | позиционирование |
| @fact:scaffold-scenario | §1 | NOT-A-BUILD-CLAIM | — | сценарий |
| @fact:accrete-across-projects | §1 | PARTLY | registry_cache.rs:29–34 (общий корень на машину) | клон-кэш аккумулируется межпроектно, но из-за URL-ключа «new work draws from it» только при том же реестре |
| @fact:agent-use-case | §1 | NOT-A-BUILD-CLAIM | — | обоснование ценности |
| @fact:with-without | §1 | NOT-A-BUILD-CLAIM | — | риторика мотивации |
| @fact:CACHE-MACHINE-GLOBAL | §2.1 | PARTLY | registry_cache.rs:29–34 | хранилище на машину с дефолтным путём и env-override есть, но это клон-кэш, а не identity-склад; env назван в документе `VIBEVM_REGISTRY_CACHE`, в коде — `VIBE_REGISTRY_CACHE`; user-config ключ работает через `[env]`-промоушен (user_config.rs:66–78, тест cli_registry_mgmt.rs:299–320) |
| @fact:CACHE-POPULATION-SHARED | §2.1 | PARTLY | git_pkg/fetch.rs:287–291; plan.rs:109 | любой фетч кладёт в общий корень клонов; извлечённый же payload живёт в проектном `.vibe/cache` — «any project resolves from it» только через URL-бакет того же реестра |
| @fact:CACHE-ACCRETIVE | §2.1 | PARTLY | контроль F: авто-выселения нет нигде вне vibe-index rate-limit | версий никто автоматически не выселяет; но это клон-кэш, который при сбое обновления вытирается целиком (git_pkg/fetch.rs:43) — не то хранилище, о котором факт |
| @fact:accretion-why | §2.1 | NOT-A-BUILD-CLAIM | — | обоснование через иммутабельность версий |
| @fact:EXPLICIT-RECLAIM | §2.1 | NOT BUILT | контроль A (+F): `vibe cache clean` нет, evict вне vibe-index пуст | reclaim-поверхности не существует |
| @fact:explicit-not-incidental | §2.1 | BUILT | registry_cache.rs:22–34 | «largely true of `default_cache_root()` already» — да: единая точка, env-override, документирована |
| @fact:SCAFFOLDING-FIRST-CLASS | §2.2 | NOT-A-BUILD-CLAIM | — | декларация требования; исполняемое содержание — в дочерних фактах |
| @fact:NEW-MEMBER | §2.2 | PARTLY | plan.rs:148–149 «Unified resolution (PROP-009 §2.7): the root set is the union» | член с `[requires]` сворачивается в общий граф; «warm cache + --offline» нет — кэш не источник резолва (plan/fetch.rs:23 всегда `source.resolve_and_fetch`) |
| @fact:NEW-PROJECT | §2.2 | PARTLY | global_registry.rs:106–128, 292–306 | новый проект наследует реестры из `~/.vibe/registry.toml`; «never re-downloads» — только пока живы клоны того же URL |
| @fact:mechanism-rests | §2.2 | NOT-A-BUILD-CLAIM | — | сводка трёх решений |
| @fact:GUARANTEE-AND-NAME | §2.2 | NOT-A-BUILD-CLAIM | — | назначение PROP |
| @fact:agent-fast-path | §2.2 | NOT-A-BUILD-CLAIM | — | сценарий ценности |
| @fact:IDENTITY-KEYED | §2.3 | PARTLY | git_pkg/fetch.rs:288–291 `cache_root.join(resolved.group.as_str()).join(&resolved.name).join(format!("v{}", resolved.version))` | извлечённый payload адресован `(group, name, version)` — но в проектном `.vibe/cache` (plan.rs:109), перезаписывается каждым фетчем (fetch.rs:344–348) и не читается как источник; глобальный слой — URL-бакеты (git_pkg/mod.rs:406–408) |
| @fact:REGISTRY-INDEPENDENT | §2.3 | NOT BUILT | контроль: `cache_dir()` = `<cache_root>/<hash(normalize_url)>` (git_pkg/mod.rs:227–230, 406–408) | реестр входит в ключ глобального кэша — тот же пакет из другого реестра ляжет в другой бакет |
| @fact:HASH-INTEGRITY-GATE | §2.3 | PARTLY | git_pkg/fetch.rs:371–431 (gate на фетче против lockfile-pin, `ContentDrift` наверху); shippable.rs:148 `compute_content_hash` | гейт на фетче есть; «a cache entry is valid only if…» неприменимо — записи кэша при чтении не валидируются, кэш не перечитывается |
| @fact:SEQUENCED-AFTER-008 | §2.3 | NOT-A-BUILD-CLAIM | — | заметка-зависимость; сама qualified identity построена: `crates/vibe-core/src/package_ref.rs:45` `pub struct Group(String);` |
| @fact:USER-LEVEL-REGISTRIES | §2.4 | PARTLY | global_registry.rs:22–49 `pub struct GlobalRegistryConfig` с `[[registry]]`/`[[mirror]]`/`[[override]]` в `~/.vibe/registry.toml` | машинно-глобальный дефолт реестров есть и мержится project-first (`merge_effective`, :292); но не в `UserConfig`/`config.toml` — тот с `deny_unknown_fields` и несёт только env/install/init (user_config.rs:62–90, контроль D) |
| @fact:INIT-SEEDS | §2.4 | PARTLY | commands/init/mod.rs:277–296 (по умолчанию `Vec::new()`); global_registry.rs:130–152 `ensure_default_global_registry` | эффект достигнут иначе: init вообще не сеет `[[registry]]`, дефолты живут один раз в глобальном файле; fallback на hardcoded-пару есть (`default_registries`, :106–128) |
| @fact:PROJECTLESS-SOURCE | §2.4 | PARTLY | global_registry.rs:292–306 (глобальный файл — источник резолва) | резолв питается глобальным файлом; но менеджмент-операции (`registry add`/`remove`) пишут в проектный `vibe.toml` (cli/registry.rs:33–40), а `vibe cache add` нет (контроль A) |
| @fact:MEMBER-INHERITS | §2.4 | BUILT | plan.rs:148–149; reinstall.rs:189 «The resolver is built from the workspace root manifest» | член наследует реестры воркспейса — резолвер строится от корня |
| @fact:PROJECT-OVERRIDES | §2.4 | BUILT | global_registry.rs:284–289 «a collision resolves to the project's favour»; user_config.rs:66–68 «a real env-var … wins» | обе полуформулы precedence истинны |
| @fact:halves-of-scaffolding | §2.4 | NOT-A-BUILD-CLAIM | — | связка двух решений |
| @fact:OFFLINE-FLAG | §2.5 | PARTLY | cli/pkg.rs:200–207 `#[arg(long)] pub offline: bool` (InstallArgs) | флаг есть, но только у `vibe install` — не глобальный (корневой `Cli`, cli.rs:59–96) |
| @fact:OFFLINE-LAYERING | §2.5 | NOT BUILT | контроль B: `VIBE_OFFLINE` пуст при живом `VIBE_UNATTENDED`; контроль E: `\[net\]` пуст | ни env-переменной, ни config-ключа; существует лишь зеркалимый паттерн `resolve_unattended` (output.rs:84–86 `cli_flag || env_unattended()`) |
| @fact:OFFLINE-LOCAL-ONLY | §2.5 | PARTLY | global_registry.rs:242–260 `local_only`; multi_registry_resolver/source.rs:70–78 (`url_is_local` — «the `--offline` filter's predicate»); resolver.rs:446–455 | локальные источники (file:// реестры/зеркала/override, embedded, project packages, lock+vibedeps) работают; кэш §2.7 источником не является |
| @fact:OFFLINE-HARD-ERROR | §2.5 | PARTLY | resolver.rs:482–490 bail `--offline: no local registry available to resolve from. …` | жёсткий actionable bail есть, но про отсутствие локального реестра; кэш-мисс-сообщение с именем пакета и рецептом выздоровления не построено |
| @fact:OFFLINE-NO-DEGRADE | §2.5 | PARTLY | flag_tests.rs:99–121 `offline_without_a_local_registry_bails_before_the_network` | для `vibe install` деградации нет — bail до сети; глобальной политики нет |
| @fact:ONLINE-DEFAULT | §2.5 | BUILT | resolver.rs:338–339 `let eff = merge_effective(manifest, global); if args.offline { eff.local_only() } else { eff }` | онлайн — дефолт, флаг чисто аддитивен |
| @fact:RESOLVER-OFFLINE-MODE | §2.6 | PARTLY | resolver.rs:333–340 (офлайн = фильтрация effective-config до резолвера); контроль C: `with_offline` пуст, `with_strict_auth` есть (multi_registry_resolver/mod.rs:272) | офлайн-режим установлен, но на уровне CLI-фильтрации; билдера `with_offline` и чтения кэша резолвером нет |
| @fact:AS-OF-LAST-REFRESH | §2.6 | NOT-A-BUILD-CLAIM | — | оговорка-обоснование; ближайший аналог в дереве — TTL клонов (registry_cache.rs:20 `DEFAULT_FRESHNESS_SECS: u64 = 3600`) |
| @fact:SYNC-COMPANION | §2.6 | BUILT | commands/registry/sync.rs:1–2 «`vibe registry sync` — refresh the per-package clones referenced by the lockfile» | sync реализован; но компаньонство номинальное — оффлайн-резолв клоны удалённых реестров не читает (см. intended-workflow) |
| @fact:intended-workflow | §2.6 | NOT BUILT | resolver.rs:446–450 «a remote github/gitverse walk is dropped — no host is contacted» | `sync` затем `install --offline` не сцепляются: оффлайн отбрасывает именно те реестры, чьи клоны освежил sync |
| @fact:A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY | §2.6 | NOT BUILT | контроль: кэш не источник резолва (plan/fetch.rs:23 `source.resolve_and_fetch(…)` для каждого узла); сбой обновления вытирает клон (git_pkg/fetch.rs:43–52) | кэш-хит ничего не ранжирует — хранилища-источника нет; сегодняшнее поведение противоположно правилу |
| @fact:WHY-THE-CACHE-OUTRANKS-A-SILENT-REGISTRY | §2.6 | NOT-A-BUILD-CLAIM | — | обоснование правила |
| @fact:THE-BEHAVIOUR-THIS-CONTRADICTS-TODAY | §2.6 | BUILT | git_pkg/fetch.rs:20–25 «If `update` fails (origin unreachable, ref missing, etc.), the clone is wiped and we retry via `GitBackend::bootstrap` against the same URL» + код :43, :52 (`fs::remove_dir_all(clone_dir)`) | описание сегодняшнего кода дословно верно |
| @fact:AN-ABSENCE-HAS-THREE-SHAPES-AND-TWO-ANSWERS | §2.6 | PARTLY | error.rs:48–53 (never-there-ответ) | из трёх форм ответа пользователю построена одна; cached- и withdrawn-формы не доведены |
| @fact:ABSENCE-CACHED | §2.6 | NOT BUILT | контроль: каждый узел всегда идёт в `source.resolve_and_fetch` (plan/fetch.rs:23) | кэш не материализует при отсутствии в реестре |
| @fact:ABSENCE-WITHDRAWN | §2.6 | PARTLY | vibe-index/src/journal/record.rs:95–104 (`Buried { name, reason, superseded_by }`); journal/burial_tests.rs:83–101 | надгробие с reason и successor построено на индекс-стороне; резолвер/CLI его не читает — `grep -i tombstone crates/vibe-registry/src` пуст (контроль группы: тот же grep по `crates/vibe-index/src` богат, см. §7-H) |
| @fact:ABSENCE-NEVER-THERE | §2.6 | BUILT | error.rs:48–53 «package `{group}/{name}` is not in the registry … fix: check the spelling» | ответ байт-в-байт той же формы, что у опечатки |
| @fact:THE-LAST-TWO-ARE-DELIBERATELY-INDISTINGUISHABLE | §2.6 | PARTLY | vibe-index/src/journal/record.rs:70–74 (`Removed` — без надгробия); burial_tests.rs:8 («tombstone placed by anything other than a journal fact is erased») | индексная модель даёт неразличимость удаления и небытия; пользовательский путь ответа об отсутствии не построен |
| @fact:THE-CARVE-OUT-FROM-THE-NEVER-SILENT-LAW | §2.6 | NOT-A-BUILD-CLAIM | — | соотношение двух законов проектирования |
| @fact:SKIP-RESOLUTION-SYNERGY | §2.6 | NOT-A-BUILD-CLAIM | — | проектная связка; отсылочный оптимизация уже в дереве: plan.rs:194–209 `Freshness::Fresh => return Ok(Plan::Fresh)` |
| @fact:LOCAL-INDEX-VIEW | §2.7 | NOT BUILT | контроль: в кэше только `meta.toml` на бакет реестра (git_registry.rs:37 «Structure persisted to `<cache_root>/<hash>/meta.toml`») и зарезервированный `freshness_secs` с `#[allow(dead_code)]` (git_pkg/mod.rs:120–124) | вида identity→versions над кэшем нет |
| @fact:layout-open | §2.7 | NOT-A-BUILD-CLAIM | — | надгробие `state="void"` |
| @fact:LAYOUT-EXTRACTED-DIRECTORIES | §2.7 | PARTLY | git_pkg/fetch.rs:288–291 + :350–351 `copy_dir_excluding_git(&clone_dir, &dest_cache)` | извлечённые payload-директории на `(group, name, version)` существуют — но в проектном `.vibe/cache` (plan.rs:109), вытираются и переписываются каждым фетчем (fetch.rs:344–348) и не образуют машинно-глобального склада |
| @fact:WHY-EXTRACTED-AND-NOT-AN-ARCHIVE | §2.7 | NOT-A-BUILD-CLAIM | — | обоснование; посылка «content_hash считается по дереву» верна (shippable.rs:148–149) |
| @fact:WHY-NOT-CLONES | §2.7 | NOT-A-BUILD-CLAIM | — | обоснование; сегодня в глобальном слое построена как раз отвергаемая форма — клоны (git_pkg/mod.rs:424–429) |
| @fact:CLONES-KEEP-THEIR-OWN-JOB | §2.7 | BUILT | git_pkg/mod.rs:406–408 (бакет на реестр, переиспользуется между проектами); vibe-registry/tests/registry_cells_oracle.rs:196 «A second open within the freshness TTL reuses the clone» | клон-кэш реестров существует и переиспользуется; «перестаёт быть load-bearing для доступности» — ещё не случилось |
| @fact:CACHE-FILLS | §2.7 | PARTLY | sync.rs:1–2; git_pkg/fetch.rs:296–352 (клоны + payload как побочный эффект любого фетча) | наполняется побочно от install/update/sync; предпрогрев `vibe cache add` отсутствует (контроль A) |
| @fact:CACHE-COMMANDS | §2.8 | NOT BUILT | контроль A | командного семейства кэша нет |
| @fact:namespace-leaning | §2.8 | NOT-A-BUILD-CLAIM | — | надгробие `state="void"` |
| @fact:NAMESPACE-IS-TOP-LEVEL-VIBE-CACHE | §2.8 | NOT BUILT | контроль A: в `Command` нет Cache-варианта (cli.rs:98–302) | верхнеуровневого `vibe cache` нет |
| @fact:CMD-PATH | §2.8 | NOT BUILT | контроль A | команды нет |
| @fact:CMD-LIST | §2.8 | NOT BUILT | контроль A | команды нет |
| @fact:CMD-ADD | §2.8 | NOT BUILT | контроль A | команды нет |
| @fact:CMD-CLEAN | §2.8 | NOT BUILT | контроль A (+F) | команды нет |
| @fact:SYNC-VENDOR-COMPLEMENT | §2.8 | BUILT | cli/registry.rs:20–21 (Sync), :82–86 (Vendor); vendor.rs:1–17 | обе команды есть и работают как описано |
| @fact:LAYERS-EXPLICIT | §2.9 | NOT-A-BUILD-CLAIM | — | рамочное решение; все три слоя существуют (registry_cache.rs:29; vibedeps.rs:7; manifest/lockfile.rs:410) |
| @fact:LAYER-CACHE | §2.9 | PARTLY | registry_cache.rs:29–34 + git_pkg/fetch.rs:288–291 | машинно-глобальный клон-кэш — источник контента; но URL-ключёванный, а identity-слой извлечения — только проектный |
| @fact:LAYER-VIBEDEPS | §2.9 | BUILT | vibedeps.rs:7 «`<workspace-root>/vibedeps/<group>.<name>/<version>/`», :12–14 (committed), :40 `slot_rel_path` | слой построен; копирование идёт из `cached.cache_dir` фетча — проектного стейджинг-кэша |
| @fact:LAYER-LOCK | §2.9 | BUILT | crates/vibe-core/src/manifest/lockfile.rs:410 `pub const FILENAME: &'static str = "vibe.lock"`; plan.rs:200–209 (fresh lock honoured verbatim) | пиннованная резолюция есть и уважается |
| @fact:OFFLINE-FLOW | §2.9 | PARTLY | материализация копированием из `cached.cache_dir`; resolver.rs:446–455 | «materialises by copying» построено; «resolves `[requires]` against the cache» — нет, оффлайн-резолв идёт по локальным реестрам |
| @fact:SURF-OFFLINE-FLAG | §3 | NOT BUILT | контроль B + cli.rs:59–96 | глобального флага и `VIBE_OFFLINE` нет; есть только install-локальный флаг другого PROP |
| @fact:SURF-CACHE-CMDS | §3 | NOT BUILT | контроль A | нет |
| @fact:SURF-CORE | §3 | NOT BUILT | контроль D: UserConfig = env/install/init + `deny_unknown_fields`; контроль E: `\[net\]` пуст | секций `[[registry]]`/`[[mirror]]` и ключа `[net]` нет |
| @fact:SURF-REGISTRY | §3 | NOT BUILT | контроль C (`with_offline` пуст); identity-склада и локального вида нет (§4: IDENTITY-KEYED, LOCAL-INDEX-VIEW) | ничего из перечисленного в vibe-registry не построено |
| @fact:SURF-CLI | §3 | PARTLY | cli/pkg.rs:206 (провязка флага); resolver.rs:482–490 (actionable bail) | провязка флага install и actionable-оффлайн-bail есть; resolved posture, init-сеяние, cache-команды — нет |
| @fact:SURF-SYNC-VENDOR | §3 | BUILT | cli/registry.rs:20–21, :82–86 | обе команды на месте, «unchanged» |
| @fact:ABANDON-NOT-MIGRATE | §4 | NOT-A-BUILD-CLAIM | — | миграционное решение (кода не требует); посылка подтверждается: существующий кэш ключёван URL (git_pkg/mod.rs:406–408; VIBEVM-SPEC.md:1548) |
| @fact:REPOPULATE | §4 | NOT-A-BUILD-CLAIM | — | план первого запуска на новой раскладке |
| @fact:one-time-cost | §4 | NOT-A-BUILD-CLAIM | — | обоснование цены |
| @fact:ADDITIVE-OTHERWISE | §4 | NOT-A-BUILD-CLAIM | — | обещание совместимости; для построенной части выполнено (resolver.rs:338–339 — сужение только при флаге) |
| @fact:OPEN-LAYOUT | §5 | NOT-A-BUILD-CLAIM | — | надгробие `state="void"` |
| @fact:OPEN-NAMESPACE | §5 | NOT-A-BUILD-CLAIM | — | надгробие `state="void"` |
| @fact:OPEN-STALENESS | §5 | NOT-A-BUILD-CLAIM | — | открытый вопрос |
| @fact:OPEN-EVICTION | §5 | NOT-A-BUILD-CLAIM | — | открытый вопрос |
| @fact:OPEN-SCAFFOLD-UX | §5 | NOT-A-BUILD-CLAIM | — | открытый вопрос |
| @fact:draft2-resolved | §5 | NOT-A-BUILD-CLAIM | — | история черновиков |
| @fact:draft3-resolved | §5 | NOT-A-BUILD-CLAIM | — | история; вложенное «qualified naming … has been implemented» проверено: package_ref.rs:45 |
| @fact:REJ-OFFLINE-DEFAULT | §6 | NOT-A-BUILD-CLAIM | — | отвергнутая альтернатива |
| @fact:REJ-URL-KEYED-INTERIM | §6 | NOT-A-BUILD-CLAIM | — | отвергнутая альтернатива |
| @fact:REJ-REPLACE-VENDOR | §6 | NOT-A-BUILD-CLAIM | — | отвергнутая альтернатива; описание vendor'а соответствует дереву (vendor.rs:1–17) |
| @fact:REJ-PROJECT-SCOPED | §6 | NOT-A-BUILD-CLAIM | — | отвергнутая альтернатива; иронично: извлечённый identity-слой сегодня именно проектный (plan.rs:109) |
| @fact:phases-sequencing | §7 | NOT-A-BUILD-CLAIM | — | заметка последовательности; зависимость закрыта (package_ref.rs:45) |
| @fact:PHASE-1-IDENTITY-CACHE | §7 | NOT BUILT | контроль: identity-склада нет (глобальный слой — URL-бакеты), вида нет (LOCAL-INDEX-VIEW), команд path/list нет (контроль A) | фаза не начата |
| @fact:PHASE-2-USER-REGISTRIES | §7 | PARTLY | global_registry.rs:22–49, :106–152, :292–306 | глобальный файл реестров + project-first override есть; не в `UserConfig`, init не сеет |
| @fact:PHASE-3-OFFLINE | §7 | PARTLY | cli/pkg.rs:206; resolver.rs:339, :482–490; flag_tests.rs:99–121 | install-флаг + `local_only` + bail есть; глобального флага, `VIBE_OFFLINE`, режима резолвера, cache-miss-ошибок нет |
| @fact:PHASE-4-PREWARM | §7 | NOT BUILT | контроль A | `vibe cache add`/`clean` нет |
| @fact:PHASE-5-SCAFFOLDING | §7 | NOT BUILT | контроль: кэш-источник отсутствует (plan/fetch.rs:23) | end-to-end сценарий на кэше невозможен |
| @fact:PHASE-6-DOCS | §7 | NOT BUILT | контроль G: в VIBEVM-SPEC.md `grep -ni 'offline\|vibe cache'` пуст при живом `~/.vibe/registries/` (:961); в `docs/commands/` нет cache-страницы (листинг каталога — 33 команды, кэша нет) | правок спека и docs-страницы нет |
| @fact:HISTORY-DRAFT-1 | §8 | NOT-A-BUILD-CLAIM | — | история версий |
| @fact:HISTORY-DRAFT-2 | §8 | NOT-A-BUILD-CLAIM | — | история версий |

## 5. Что оказалось построено, хотя документ этого не говорит

Статусная строка PROP-010 не содержит ни одного `impl/done`, но **17 фактов со
статусом `spec/*` уже BUILT** — все они описывают существующее дерево и подтверждены
адресами в §4:

1. `@fact:cache-exists` (spec/done) — реестровый кэш существует и питает install/update.
2. `@fact:cache-incidental` (spec/done) — у кэша нет операторской поверхности.
3. `@fact:no-offline-mode` (spec/done) — `--offline` уже поставлен (PROP-002/PROM-030), безусловный ре-резолв уже отменён (PROP-011).
4. `@fact:no-local-resolve` (spec/done) — «resolve-локально» уже выразимо тремя механизмами.
5. `@fact:consumers-already-offline` (spec/done) — `vibedeps/` коммитится, fresh clone бутится без install.
6. `@fact:explicit-not-incidental` (spec/done) — `default_cache_root()` уже почти то, чем PROP-010 его объявляет.
7. `@fact:MEMBER-INHERITS` (spec/done) — unified resolution у корня воркспейса.
8. `@fact:PROJECT-OVERRIDES` (spec/done) — project-first precedence построена (для глобального файла реестров и для `[env]`).
9. `@fact:ONLINE-DEFAULT` (spec/done) — онлайн-дефолт, флаг аддитивен.
10. `@fact:SYNC-COMPANION` (spec/done) — `vibe registry sync` реализован.
11. `@fact:SYNC-VENDOR-COMPLEMENT` (spec/done) — sync + vendor обе существуют.
12. `@fact:THE-BEHAVIOUR-THIS-CONTRADICTS-TODAY` (spec/work) — вытирание клона при сбое обновления существует ровно как описано.
13. `@fact:ABSENCE-NEVER-THERE` (spec/work) — «no such package» уже даёт ответ, неотличимый от опечатки.
14. `@fact:CLONES-KEEP-THEIR-OWN-JOB` (spec/work) — клон-кэш реестров существует и переиспользуется.
15. `@fact:LAYER-VIBEDEPS` (spec/done) — слой vibedeps построен.
16. `@fact:LAYER-LOCK` (spec/done) — слой lockfile построен.
17. `@fact:SURF-SYNC-VENDOR` (spec/done) — surface sync/vendor на месте.

Дополнительно, вне счёта BUILT, два «построено больше, чем кажется из статусов»:

- **Проектный identity-слой извлечения уже существует.** Каждый фетч кладёт
  `.git`-stripped payload в `<workspace-root>/.vibe/cache/<group>/<name>/v<version>/`
  (`git_pkg/fetch.rs:288–291`, `plan.rs:107–109` «Cache layout matches §8.3 … The cache
  lives at the absolute workspace root — one shared cache»). Это ровно форма
  «per-identity extracted directories» из решения владельца 2026-08-19 — только
  проектная, переписываемая и не читаемая как источник. PHASE-1 не начинает с нуля:
  ему есть что поднять и что переосмыслить (перенос уровня с проекта на машину,
  запрет перезаписи, чтение как источника).
- **Машинно-глобальный дефолт реестров уже существует** — `~/.vibe/registry.toml`
  (`GlobalRegistryConfig`, PROP-002 §2.2.2), с сеянием hardcoded-пары на свежей
  машине и project-first мержем. Это функциональный предшественник §2.4 в другом
  файле и с другим именем, чем предлагает PROP-010 (`UserConfig`).

## 6. Что документ обещает и чего нет

Ядро PROP-010 не построено — всё перечисленное NOT BUILT в §4 с контролями §7:

- **Командное семейство `vibe cache`** целиком: `path`/`list`/`add`/`clean`
  (факты CACHE-COMMANDS, NAMESPACE-…, CMD-PATH/LIST/ADD/CLEAN, SURF-CACHE-CMDS, PHASE-4).
- **Глобальный `--offline`**: нет ни глобального флага на корне CLI, ни `VIBE_OFFLINE`,
  ни config-ключа `[net]`, ни resolved-offline-posture (OFFLINE-LAYERING, SURF-OFFLINE-FLAG).
  Существующий `--offline` — флаг `vibe install` из PROP-030/PROP-002, установленный
  до PROP-010 и не совпадающий с его замыслом.
- **Identity-ключёванный машинно-глобальный склад**: глобальный слой ключёван URL
  реестра (REGISTRY-INDEPENDENT, IDENTITY-KEYED, PHASE-1); локального вида
  identity→versions нет (LOCAL-INDEX-VIEW).
- **`MultiRegistryResolver::with_offline`** (RESOLVER-OFFLINE-MODE, SURF-REGISTRY,
  контроль C): резолвер не имеет офлайн-режима; сужение происходит выше него.
- **Кэш-хит как авторитет доступности** (A-CACHE-HIT-…, ABSENCE-CACHED): кэш не
  является источником резолва вообще; при сбое обновления последний локальный
  экземпляр вытирается — поведение, противоположное правилу.
- **Рабочий цикл `sync` → `install --offline`** (intended-workflow): не сцепляется —
  оффлайн отбрасывает удалённые реестры, чьи клоны освежает sync.
- **`[[registry]]`/`[[mirror]]` в `UserConfig` и `[net]`-ключ** (USER-LEVEL-REGISTRIES
  как написано, SURF-CORE, контроль D/E): UserConfig закрыт `deny_unknown_fields`.
- **Ответы об отсутствии** для форм cached/withdrawn (ABSENCE-WITHDRAWN доведён
  только до индекс-модели, до пользователя — нет).
- **PHASE-5 (scaffolding end-to-end) и PHASE-6 (спек+docs)**: не начаты.

Отдельно — расхождение имени: документ дважды пишет `VIBEVM_REGISTRY_CACHE`
(строки 17, 37), код читает `VIBE_REGISTRY_CACHE` (registry_cache.rs:30). Ни одного
чтения `VIBEVM_REGISTRY_CACHE` в дереве нет (grep по crates пуст). Если PROP-010
будет реализовываться, имя в документе следует либо исправить, либо осознанно
ввести второе.

## 7. Контроли непустоты

Каждый NOT BUILT в §4 прикрыт одним из контролов ниже; команды и выводы дословны
(Git Bash, cwd = корень worktree).

**A — семейство команд `vibe cache` (C2, CACHE-COMMANDS, NAMESPACE-…, CMD-*, SURF-CACHE-CMDS, PHASE-4):**

```
$ grep -n "Cache" crates/vibe-cli/src/cli.rs
[exit 1]                              # пусто — Cache-команды нет

$ grep -n "Registry(RegistryArgs)" crates/vibe-cli/src/cli.rs
199:    Registry(RegistryArgs),
[exit 0]                              # контроль: тем же способом команда находится
```

**B — `VIBE_OFFLINE` (C5, OFFLINE-LAYERING, SURF-OFFLINE-FLAG):**

```
$ grep -rn "VIBE_OFFLINE" crates/
[exit 1]                              # пусто

$ grep -n "VIBE_UNATTENDED" crates/vibe-cli/src/output.rs
66:/// Read the `VIBE_UNATTENDED` env-var. Truthy values are `1`,
73:    std::env::var("VIBE_UNATTENDED")
103:    /// `VIBE_UNATTENDED` env-var truthy. Implies skip-all-confirms
156:    /// `VIBE_UNATTENDED` resolves truthy in the environment. Mutating
[exit 0]
```

**C — `with_offline` (C7, RESOLVER-OFFLINE-MODE, SURF-REGISTRY):**

```
$ grep -rn "with_offline" crates/
[exit 1]                              # пусто

$ grep -n "with_strict_auth" crates/vibe-registry/src/multi_registry_resolver/mod.rs
190:    /// Toggled by `MultiRegistryResolver::with_strict_auth`.
272:    pub fn with_strict_auth(mut self, strict: bool) -> Self {
[exit 0]
```

**D — секции `[[registry]]`/`[[mirror]]` в UserConfig (C8, SURF-CORE, USER-LEVEL-REGISTRIES как написано):**

```
$ grep -ni "registry\|mirror" crates/vibe-core/src/user_config.rs
40://! `export VIBE_REGISTRY_CACHE=…` for the value to actually apply.
[exit 0]                              # единственное вхождение — комментарий про env;
                                      # секций нет. Контроль: сам struct несёт env/install/init
                                      # (user_config.rs:78, :82, :89) с deny_unknown_fields (:63)
```

**E — ключ `[net]` (OFFLINE-LAYERING, SURF-CORE):**

```
$ grep -rn "\[net\]" crates/vibe-core/src/
[exit 1]                              # пусто. Контроль: соседний ключ [install]
                                      # тем же способом находится: user_config.rs:82
                                      # `pub install: InstallConfig,`
```

**F — выселение/чистка кэша (EXPLICIT-RECLAIM, CMD-CLEAN, CACHE-ACCRETIVE):**

```
$ grep -rni "evict" crates/ | grep -v "vibe-index"
[exit 1]                              # пусто — вне vibe-index выселения нет нигде

$ grep -rni "evict" crates/vibe-index/src | head -3
crates/vibe-index/src/server/rate_limit.rs:18://! Eviction: when the per-IP map reaches `max_buckets`, idle entries
crates/vibe-index/src/server/rate_limit.rs:21://! evicted to make room. PROP-005 §9 Q10 — this is the v1 surface
crates/vibe-index/src/server/rate_limit.rs:230:                evict_if_full(&mut buckets, self.config.max_buckets, capacity);
                                      # контроль: тот же инструмент находит evict там,
                                      # где он есть (rate-limit IP-пул, не пакетный кэш)
```

**G — правки VIBEVM-SPEC.md и docs (PHASE-6):**

```
$ grep -ni "offline\|vibe cache" VIBEVM-SPEC.md
[exit 1]                              # пусто

$ grep -n "~/.vibe/registries/" VIBEVM-SPEC.md | head -3
961:~/.vibe/registries/
1548:- [ ] Registry cache lives at `~/.vibe/registries/<canonical-url-hash>/packages/<kind>-<name>/` — per-package, not per-monorepo.
                                      # контроль: смежные упоминания кэша находятся;
                                      # листинг docs/commands/ (33 файла) страницы кэша не содержит
```

**H — резолвер не читает надгробия (ABSENCE-WITHDRAWN):**

```
$ grep -i "tombstone\|withdraw" -r crates/vibe-registry/src
[exit 1]                              # пусто

$ grep -il "tombstone" -r crates/vibe-index/src | head -3
crates/vibe-index/src/journal/burial_tests.rs
crates/vibe-index/src/journal/record.rs
crates/vibe-index/src/types/mod.rs
                                      # контроль: тот же grep по vibe-index богат —
                                      # надгробная механика есть, но только индекс-стороне
```

## 8. Что этот замер НЕ установил

- **Поведение во времени не мерялось.** Замер статичен: он говорит, какой код есть
  в дереве, но не прогонял `vibe install --offline` на живом проекте — все выводы о
  поведении выведены из чтения кода и док-комментариев, а не из исполнения
  (Cargo не запускался по условиям пакета).
- **Тесты не выполнялись.** Ссылки на тесты (flag_tests.rs:99, burial_tests.rs:83,
  registry_cells_oracle.rs:196) — это адреса прочитанных утверждений, а не прогоны.
- **`vibe-index`-сервер и индекс-клиент не мерились глубоко.** Для ABSENCE-* фактов
  проверено наличие надгробной модели в журнале/проекции и её отсутствие в
  резолвере; как именно индекс-сервер отвечает на запрос снятого имени по сети —
  не измерено.
- **Хвостые ящики PROP-002/005/008/009/011/030 не ревизовались.** Их факты читались
  ровно настолько, насколько PROP-010 на них опирается; полноценная сверка тех
  документов — отдельная задача.
- **Раскладка на диске реального `~/.vibe/registries/` не осматривалась** (замер
  только по коду); фактическое содержимое кэша на этой машине — вне периметра.
- **Не разрешён вопрос, что имел в виду документ под `VIBEVM_REGISTRY_CACHE`** —
  установлено только, что код это имя не читает; была ли это опечатка документа
  или когда-то переименованная переменная — замер не отвечает.
