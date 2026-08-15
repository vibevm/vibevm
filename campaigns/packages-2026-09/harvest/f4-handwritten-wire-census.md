# F4-WIRE-CENSUS — перепись рукописного провода и правило белого списка

**Чем мерил:** только читающие команды (`find`, `grep -lzE`, `sed -n`, `ls`,
`awk`/`comm` над временными списками в `/tmp`) в Git Bash на рабочем дереве
`C:\Users\olegc\git\v\vibevm\.wt\F4-WIRE-CENSUS`; правки — только Write.
**Что НЕ запускалось:** ни одной команды `git` (запрещена пакетом), ни одной
команды `cargo` (замер статический; «зелёность» тестов цитируется по месту
в коде, не по прогону). `vibedeps/**`, `packages/**`, `refs/**` не читались и
в подсчёты не входят. **Дата: 2026-08-15.**

## 1. ВЕРДИКТ

**Да — белый список выражается правилом без перечисления имён, и лучший
кандидат — К2 «реестр + маркер»: файл вне `**/generated/**` с рукописным
`derive(Serialize|Deserialize)` обязан нести в себе ровно один тег формата
(`// format: <id>` или атрибут), где `<id>` — существующая запись
`formats/REGISTRY.toml`; если у записи схема — построенный `*.jtd.json`, файл
красный (формат обязан течь через `vibe-wire`).** Реестр форматов уже
существует (`formats/REGISTRY.toml`:20 записей), уже генерирует `FormatId`
в типовую систему («an unregistered format is *inexpressible in the type
system*» — шапка реестра) и уже несёт ось «схема есть/нет» — правило только
джойнит файл с реестром. Белый список становится *данными, распределёнными
по файлам + знаменателем в реестре*, а не списком имён в гейте: файл,
которого ещё нет, попадает под предикат в день появления.

Оговорки: (а) сегодня ни один файл тега не несёт — нужна разовая волна
аннотирования всех 133 файлов переписи (механическая, передаваемая
исполнителю); на время волны переходная ступень — К4 (рехет-счётчик по
крейтам против базовой линии, форма уже доказана соседом
`conform-baseline.json`); (б) правило обязано различать JTD и не-JTD схемы:
`cli-package-tree` зарегистрирован со JSON Schema 2020-12
(`crates/vibe-cli/resources/package-tree.schema.v1.json`), codegen его не
трогает — рукописный эмиттер там законен, без этой оговорки правило поймает
его зря; (в) форма самого гейта — шаг 10d панели (module-perimeter grep с
фильтром строк-комментариев и действующим сообщением), процитирован в §7;
(г) периметр гейта должен включать `xtask/` — иначе `xtask/src/sync_engines.rs`
выпадает (см. U1: записанные «69» — это ровно `crates/**` без xtask).

## 2. Подтверждение-или-опровержение записанного (U1–U5)

**U1 — «рукописный `derive(Deserialize)` вне `generated/**` стоит в 69 файлах»
— ПОДТВЕРЖДЕНО с оговоркой периметра.** Мой счёт: **70** файлов в
`crates/**`+`xtask/**` (включая `tests/**`), из них **69 — ровно `crates/**`
без xtask**:

```
$ grep -vc '^xtask/' /tmp/f4-deser.txt
69
```

70-й файл — `xtask/src/sync_engines.rs:44` (`#[derive(Debug, Deserialize)] ==
pub(crate) struct SyncSet`). Расхождение — не ошибка, а другой периметр:
записанное число верно для `crates/**`; гейт Ф4.3, меряющий и xtask, увидит
70. Многострочных `derive` в дереве нет (см. §3), `serde::`-пути учтены.

**U2 — «`deny_unknown_fields` НЕ эмитится генератором, все вхождения
рукописные» — ПОДТВЕРЖДЕНО, тремя независимыми следами.** (1) Во всех 9
файлах `crates/vibe-wire/src/generated/` — ноль вхождений:

```
$ env LC_ALL=C grep -rc 'deny_unknown_fields' crates/vibe-wire/src/generated/
crates/vibe-wire/src/generated/format_id/mod.rs:0
… (все 9 файлов):0
```

(2) Проект сам записал невозможность, `crates/vibe-wire/src/lib.rs:19-31`:
«None of these types carries `#[serde(deny_unknown_fields)]` … It is not:
**the generator cannot emit it.** Measured 2026-08-06 — no key in any of our
schemas controls it …». (3) Рукописных вхождений атрибута сегодня **48** в
`crates/**`+`xtask/**` вне generated (`grep -rn '#\[serde(deny_unknown_fields)\]'
crates xtask --include='*.rs' | grep -v '/generated/' | wc -l` → `48`) — и
каждое стоит на типе с видимым автором.

**U3 — «шестнадцать `deny_unknown_fields` сняты с типов каталога, а
`package_kind_rejects_unknown` живой и зелёный» — ЧАСТИЧНО ПОДТВЕРЖДЕНО:
сегодняшнее состояние сходится, число 16 сегодняшним деревом непроверяемо.**
В `crates/vibe-index` — ноль вхождений атрибута (`grep -rn … crates/vibe-index
| wc -l` → `0`): с каталога строгость снята, след виден. Тест жив и адресуем:
`crates/vibe-index/src/types/kinds.rs:155-159`:

```rust
#[test]
fn package_kind_rejects_unknown() {
    assert!(<PackageKind as FromStr>::from_str("plugin").is_err());
    assert!(<PackageKind as FromStr>::from_str("").is_err());
}
```

«Зелёный» перепроверен быть не может (cargo запрещён); он следует из
закрытого перечня (`pub enum PackageKind` без fallback-варианта, values через
`FromStr`) и из того, что панель гоняет `cargo test --workspace`
(`tools/self-check.sh:298`). Число **16** проверяемо только git-историей
(git запрещён), но арифметика бьётся: `lib.rs:21` замерила «roughly 63
places» на 2026-08-06; сегодня 48; 63 − 48 = 15 ≈ 16 (±1 поглощается
«roughly» и периметром xtask: без xtask сегодня 46, 63 − 46 = 17). Словари
действительно остались закрытыми: см. U3-тест выше и
`naming_convention_serde_matches_vibe_core_wire` (`kinds.rs:172`).

**U4 — «манифест `vibe.toml` сохраняет строгость осознанно — его парсер несёт
`deny_unknown_fields`» — ПОДТВЕРЖДЕНО.**
`crates/vibe-core/src/manifest/document.rs:64-66`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
```

Осознанность записана рядом с политикой двух классов,
`crates/vibe-wire/src/lib.rs:19-22` («The hand-written types in this project
do the opposite — strictness is the house style there, in roughly 63 places»)
и ruling 2026-08-06, `lib.rs:39-43`: «For a format that arrives from outside,
permissiveness is forward compatibility. The point of this note is that
softness becomes a deliberate choice rather than an accident of tooling».
Строгость несут и секции: `document.rs:202,238,269`; весь манифест-фамилия —
47 из 48 вхождений атрибута сидит в vibe-core/manifest/**, user_config,
global_registry, git_registry, xtask (см. §3 сноску).

**U5 — «в `crates/vibe-wire/src/generated/` лежат ровно те подмодули, что
соответствуют схемам в `schemas/`, плюс `format_id`» — ПОДТВЕРЖДЕНО.**
`ls crates/vibe-wire/src/generated/` → `format_id init_report install_plan
install_report list_report registry_publish_report registry_sync_report
uninstall_report mod.rs`; `ls schemas/` → `init_report.jtd.json
install_plan.jtd.json install_report.jtd.json list_report.jtd.json
registry_publish_report.jtd.json registry_sync_report.jtd.json
uninstall_report.jtd.json` — 7 подмодулей ↔ 7 схем 1:1 по имени, восьмой
`format_id` эмитится из `formats/REGISTRY.toml`, а не из JTD
(`xtask/src/codegen.rs:207-208`: «The vibe-wire generated tree also carries
`format_id`, emitted from `formats/REGISTRY.toml` (PROP-044 §4.1) rather than
a JTD schema»). Оговорка: кроме подмодулей там лежит сгенерированный корень
`mod.rs` (сам синтезируется xtask, `codegen.rs:152-153`) — это не подмодуль,
а модуль-список; утверждение в этой оговорке верно.

## 3. Перепись: три счётчика и как считались

Периметр: все `*.rs` в `crates/**` и `xtask/**` вне `**/generated/**`
(628 − 9 в generated = 619 кандидатов). `schemas/` и `formats/` файлов `.rs`
не содержат (`find schemas formats -name '*.rs'` → пусто). Команды и
дословный вывод:

```
$ find crates xtask -name '*.rs' -not -path '*/generated/*' -print0 \
    | xargs -0 env LC_ALL=C grep -lzE '#\[derive\([^)]*Serialize' | sort > /tmp/f4-ser.txt
$ find crates xtask -name '*.rs' -not -path '*/generated/*' -print0 \
    | xargs -0 env LC_ALL=C grep -lzE '#\[derive\([^)]*Deserialize' | sort > /tmp/f4-deser.txt
$ wc -l < /tmp/f4-ser.txt
122
$ wc -l < /tmp/f4-deser.txt
70
$ sort -u /tmp/f4-ser.txt /tmp/f4-deser.txt | wc -l
133
$ comm -23 /tmp/f4-ser.txt /tmp/f4-deser.txt | wc -l     # только Serialize
63
$ comm -13 /tmp/f4-ser.txt /tmp/f4-deser.txt | wc -l     # только Deserialize
11
```

**Три счётчика: только `Serialize` — 63; только `Deserialize` — 11;
объединение (периметр гейта Ф4.3) — 133** (с обоими — 59 = 122+70−133).

Почему такая форма шаблона:

- **Многострочные `derive`.** `grep -lzE` с `[^)]*` читает файл целиком
  (`-z`), поэтому `#[derive(` и `Serialize` на разных строках ловятся.
  Домер: однострочный `grep -lE` без `-z` даёт те же множества (122/70,
  `comm` пуст) — **сегодня многострочных `derive` в дереве вне generated
  нет ни одного**, но шаблон устойчив к ним и впереди.
- **`serde::Serialize` путём.** Шаблон требует только токен после
  `#[derive(`, поэтому `#[derive(serde::Serialize)]` ловится. Таких файлов
  **8**: `crates/vibe-actions/src/{action,aiui,context,gate,i18n}.rs`,
  `crates/vibe-core/src/manifest/package/tests.rs`,
  `crates/vibe-registry/src/index_client/wire.rs`,
  `crates/vibe-registry/src/multi_registry_resolver/attempt.rs`. ВАЖНО:
  шаблон из плана «`#[derive(Serialize`» в лоб не поймает НИ один из 8 (и
  вообще почти всё — большинство `derive` начинается с `Debug`); гейт обязан
  искать `derive\([^)]*(Serialize|Deserialize)`.
- **Техническая грабля воспроизведения:** в Git Bash `grep -P` падает
  («-P supports only unibyte and UTF-8 locales») — потому ERE + `-z`; `LC_ALL=C`
  обязателен.
- Счётчик `Deserialize` по `crates/**` без xtask = 69 (U1); `tests/**`
  входит в перепись (4 файла с `Deserialize` в `crates/*/tests/` + 1
  `manifest/package/tests.rs` внутри `src`).

## 4. Разбивка по крейтам (таблица А)

Сумма строк = 133 = объединение из §3. Крейты без рукописного derive:
`vibe-check`, `vibe-graph`, `vibe-install`, `vibe-llm`, `vibe-resolver`,
`vibe-settings`, `vibe-spec`, `vibe-test-support`, `vibe-wire` (вне
`src/generated/` у vibe-wire рукописных derive нет вовсе — единственный
крейт, чей wire уже весь сгенерирован).

| крейт | файлов (объединение) | из них `src/` | из них `tests/` |
|---|---|---|---|
| vibe-cli | 41 | 40 | 1 |
| vibe-index | 29 | 27 | 2 |
| vibe-core | 24 | 24 | 0 |
| progress-core | 11 | 11 | 0 |
| vibe-actions | 7 | 7 | 0 |
| vibe-registry | 7 | 6 | 1 |
| vibe-mcp | 5 | 5 | 0 |
| vibe-publish | 3 | 3 | 0 |
| vibe-trace | 3 | 3 | 0 |
| vibe-workspace | 2 | 2 | 0 |
| xtask | 1 | 1 | 0 |
| **итого** | **133** | **129** | **4** |

Примечание: `crates/vibe-core/src/manifest/package/tests.rs` лежит внутри
`src/` (модуль `tests.rs`, не каталог `tests/`) и в графе `tests/` не
считается — он учтён в `src/` и в корзине `test-fixture` (§5).

## 5. Разбивка по назначению (таблица Б)

Корзины старта плюс одна новая — **`shared-domain`** (заведена потому, что
новотипы `vibe-core` — `Group`, `ContentHash`, `CapabilityRef`, `RelPath`,
`SourceUrl` — не описывают ни один формат: они кочуют ВНУТРИ всех форматов
сразу (манифеста, локфайла, записей каталога, отчётов; ср.
`crates/vibe-index/src/types/entry/mod.rs:16` — `use vibe_core::Group`), и
втискивание их в `authored` скрыло бы, что это общий словарь, который
генерированным типам тоже понадобится).

| корзина | файлов | примеры (файл:строка) | законность рукописного derive |
|---|---|---|---|
| wire-out | 77 | `crates/vibe-index/src/types/entry/mod.rs:1-4` («canonical per-version index record… every line of `primary.jsonl`… every `POST /v1/packages` body»); `crates/vibe-cli/src/commands/list.rs:34` (`LockedSubskillJson`); `crates/vibe-index/src/server/routes/packages.rs:36` (`ListResponse`); `crates/vibe-actions/src/aiui.rs:31` (`ActionView`) | сегодня законен, завтра — нет: для форматов с построенной JTD-схемой это миграционный долг перед vibe-wire; для отчётов без схемы — законен, пока схему не построят |
| authored | 22 | `crates/vibe-core/src/manifest/document.rs:65` (`Manifest`); `crates/vibe-core/src/manifest/lockfile.rs:77` (`Lockfile`); `crates/vibe-core/src/user_config.rs:62` (`UserConfig`); `xtask/src/sync_engines.rs:36` (`SyncManifest` ← `sync-engines.toml`); `crates/vibe-index/src/hash_recipe.rs:57` (`Recipe1File` ← `formats/hash_recipes/1.toml`) | законен: формат авторский, `schema = "none"` в реестре — рукописной строгостью (`deny_unknown_fields`, U4) формат и ОПРЕДЕЛЁН |
| internal-state | 14 | `crates/vibe-index/src/index/checkpoint.rs:1-2` («`<data-dir>/state/checkpoint.json`»); `crates/progress-core/src/sidecar.rs:1-3` (sidecar вне репо); `crates/vibe-cli/src/commands/vvm/model.rs:236-239` («`<root>/vibevm/state.toml`»); `crates/vibe-registry/src/search/cache.rs:1-2` («`~/.vibe/search-cache/`») | законен: пишет и читает один и тот же инструмент; внешний потребитель отсутствует по построению |
| third-party-shape | 6 | `crates/vibe-mcp/src/jsonrpc.rs:1` («JSON-RPC 2.0 message shapes per jsonrpc.org»); `crates/vibe-index/src/scanner/from_github.rs:107` (`Repo` ← GitHub API); `crates/vibe-publish/src/github.rs:134` (`RepoResponse`) | законен: чужой протокол описан в чужой спеке; наша JTD-схема поверх чужого формата — двойная работа |
| shared-domain | 6 | `crates/vibe-core/src/content_hash.rs:48` (`ContentHash`); `crates/vibe-core/src/package_ref.rs:43` (`Group`); `crates/vibe-core/src/provenance.rs:1-4` (`SourceUrl`/`TraceId`) | спорен: словарь общий,derive кочует со словарём; при генерации форматов эти типы встанут ВНУТРЬ сгенерированных — вопрос «кто их сериализует» придётся решить |
| test-fixture | 5 | `crates/vibe-cli/tests/cli_search.rs:46` (`SearchQuery` — десериализация вывода собственного CLI); `crates/vibe-core/src/manifest/package/tests.rs:60` (`Wrap`) | законен, но при миграции формата на generated-типы двойники в тестах обязаны перейти на vibe-wire следом, иначе тест и код разъедутся |
| wire-in | 3 | `crates/vibe-registry/src/index_client/wire.rs:17` (`NameEntryView` — клиент индекса); `crates/vibe-cli/src/commands/tree/artifacts.rs:9-13` (`read_index` парсит сгенерированный `INDEX.md` TOML); `crates/vibe-registry/src/git_registry.rs:38` (`RegistryMeta`) | наполовину долг: клиентские виды чужих/сгенерированных файлов — первые кандидаты на потребление vibe-wire, когда у формата появится схема |

ЯВНАЯ сверка суммы: 77 + 22 + 14 + 6 + 6 + 5 + 3 = **133** = объединение §3. ✓

Полное распределение файлов по корзинам (каждый файл ровно один раз):

- **wire-out (77):** vibe-cli/src/commands/{check.rs, init/helpers.rs,
  install/report.rs, list.rs, mcp/install.rs, mcp/mod.rs, mcp/uninstall.rs,
  mcp/upgrade.rs, outdated.rs, prefs/check.rs, prefs/get.rs, prefs/list.rs,
  prefs/migrate.rs, prefs/origins.rs, prefs/set.rs, registry/config/add.rs,
  registry/config/list.rs, registry/config/mirror.rs, registry/config/mod.rs,
  registry/config/remove.rs, registry/config/test.rs, registry/mod.rs,
  registry/publish.rs, registry/redirect/mod.rs, registry/sync.rs,
  registry/vendor.rs, search.rs, search/purl.rs, show/config.rs,
  show/effective.rs, show/features.rs, show/purls.rs, show/subskills.rs,
  specmap.rs, tree/model.rs, tree/tui/model_view.rs, workspace/publish.rs}
  (37); vibe-index/src/{cli/capabilities.rs, cli/get.rs, cli/list.rs,
  cli/outdated.rs, cli/purls.rs, cli/reindex.rs, cli/search.rs, cli/verify.rs,
  index/inverted.rs, journal/record.rs, server/error.rs,
  server/routes/admin.rs, server/routes/capabilities.rs,
  server/routes/health.rs, server/routes/packages.rs, server/routes/purls.rs,
  types/entry/aggregate.rs, types/entry/content.rs, types/entry/mod.rs,
  types/entry/relations.rs, types/kinds.rs, types/repomd.rs} (22);
  vibe-actions/src/{action.rs, address.rs, aiui.rs, context.rs, gate.rs,
  i18n.rs, params.rs} (7); vibe-mcp/src/{agentic.rs, install.rs,
  pkgskill.rs} (3); vibe-trace/src/{search.rs, select.rs, select/parse.rs}
  (3); vibe-publish/src/post_hook.rs (1); vibe-workspace/src/{boot_artifacts.rs,
  tools.rs} (2); vibe-registry/src/multi_registry_resolver/attempt.rs (1);
  progress-core/src/report.rs (1). = 37+22+7+3+3+1+2+1+1 = 77.
- **authored (22):** vibe-core/src/{global_registry.rs, manifest/document.rs,
  manifest/i18n.rs, manifest/lockfile.rs, manifest/package.rs,
  manifest/package/binary.rs, manifest/package/capabilities.rs,
  manifest/package/hooks.rs, manifest/package/mcp_server.rs,
  manifest/package/skill.rs, manifest/package/weak_deps.rs,
  manifest/package/when.rs, manifest/package/wire.rs, manifest/project.rs,
  manifest/redirect.rs, manifest/subskill.rs, user_config.rs} (17);
  vibe-index/src/{hash_recipe.rs, lockfile.rs} (2);
  vibe-registry/src/hash_recipe.rs (1); progress-core/src/scope.rs (1);
  xtask/src/sync_engines.rs (1). = 17+2+1+1+1 = 22.
- **internal-state (14):** progress-core/src/{baseline.rs, cache.rs, doc.rs,
  evidence.rs, journal.rs, model.rs, rollup.rs, sidecar.rs, state.rs} (9);
  vibe-index/src/{index/checkpoint.rs, scanner/org_cache.rs} (2);
  vibe-cli/src/commands/vvm/{model.rs, placer.rs} (2);
  vibe-registry/src/search/cache.rs (1). = 9+2+2+1 = 14.
- **third-party-shape (6):** vibe-index/src/scanner/from_github.rs;
  vibe-registry/src/search/full_scan.rs; vibe-mcp/src/{jsonrpc.rs, lib.rs};
  vibe-publish/src/{github.rs, gitverse.rs}.
- **shared-domain (6):** vibe-core/src/{capability_ref.rs, content_hash.rs,
  package_ref.rs, package_ref/kind.rs, provenance.rs, rel_path.rs}.
- **test-fixture (5):** vibe-cli/tests/cli_search.rs;
  vibe-index/tests/{from_github_e2e.rs, org_cache_e2e.rs};
  vibe-registry/tests/index_search.rs;
  vibe-core/src/manifest/package/tests.rs.
- **wire-in (3):** vibe-registry/src/{index_client/wire.rs, git_registry.rs};
  vibe-cli/src/commands/tree/artifacts.rs.

Пограничные решения (файл → корзина, где неочевидно): `vibe-index/src/journal/record.rs`
→ wire-out (журнал — «TRUTH layer» PROP-044, каталог из него перестраивается
другим процессом: `journal/record.rs:3-5`); `vibe-index/src/index/inverted.rs`
→ wire-out («let consumers fetch … with a single HTTP GET», строки 2-4);
`vibe-index/src/lockfile.rs` → authored (читатель авторского `vibe.lock`,
`lockfile.rs:1-2`); `vibe-cli/src/commands/tree/artifacts.rs` → wire-in
(парсит СГЕНЕРИРОВАННЫЙ файл, `artifacts.rs:12-13`); `vibe-mcp/src/lib.rs`
(`ToolDescriptor`) → third-party-shape (форма протокола MCP,
`lib.rs:1-11`); `progress-core/src/model.rs` → internal-state (сериализация
существует для roundtrip кэша/sidecar, не для внешнего провода).

## 6. Кандидаты в правило белого списка

**К1 — «путь»: белый список = glob-периметр каталогов.**
- *формулировка:* рукописный `derive(Serialize|Deserialize)` вне
  `**/generated/**` легален только в файлах под перечисленными
  glob-каталогами (по корзинам: `manifest/**`, `commands/**`, `server/**`,
  `*/tests/**`, …).
- *механика:* как шаг 11b панели — `grep -rEn --include='*.rs'` по дереву
  минус whitelist-glob'ы (список каталогов живёт в гейте).
- *числа:* чтобы позеленеть сегодня, нужно ≥13 glob'ов, накрывающих все 133
  (по крейтам целиком — 11 позиций таблицы А; по корзинам — ~20):
  пропускает 133, ловит 0.
- *чем ошибается:* пропустит зря — новый рукописный wire-тип внутри уже
  глобнутого каталога (например, новый отчёт в
  `crates/vibe-cli/src/commands/`) молча зелёный; поймает зря — первый файл
  нового легитимного назначения в новом каталоге (например,
  `crates/vibe-check/src/report.rs`) красный, пока список не расширят. По
  существу переносит перечисление из имён файлов в имена каталогов —
  дольше живёт, но остаётся перечислением.

**К2 — «реестр + маркер»: белый список = джойн файла с
`formats/REGISTRY.toml` через тег в файле (РЕКОМЕНДУЕМЫЙ).**
- *формулировка:* файл вне `**/generated/**` с рукописным
  `derive(Serialize|Deserialize)` обязан нести ровно один тег формата в
  файле (`// format: <id>` или атрибут `#[format(id)]`); `<id>` обязан
  существовать в `formats/REGISTRY.toml`; если у записи `schema` —
  существующий `*.jtd.json`, файл красный: формат обязан течь через
  `vibe-wire`; `schema = "none"` или ещё не построенная схема — зелёный.
- *механика:* grep derive → извлечь тег → awk-джойн с REGISTRY.toml →
  проверить существование файла схемы. Двойной знаменатель сам проверяем:
  тег без записи в реестре — красный (неинвентаризированный формат),
  запись без тегов — видна ревью реестра.
- *числа:* сегодня тегов нет — 0/133 проходят (нужна разовая волна
  аннотирования, механическая); после честной аннотации красными станут
  эмиттеры построенных форматов — `commands/list.rs` (cli-list-report),
  `commands/install/report.rs` (cli-install-plan/-report),
  `commands/registry/publish.rs` (cli-registry-publish-report),
  `commands/registry/sync.rs` (cli-registry-sync-report) — то есть правило
  ловит ровно миграционный долг Ф4 и никого больше (~4-8 файлов красные,
  ~125-129 зелёные).
- *чем ошибается:* пропустит зря — файл с ложным тегом `format: manifest`,
  эмитящий list-report (джойн не видит содержания; лечится ревью тега при
  появлении, редким после волны); поймает зря —
  `crates/vibe-cli/src/commands/tree/model.rs` (`cli-package-tree`: схема
  есть, но JSON Schema 2020-12, не JTD; `REGISTRY.toml`: «the codegen
  generator leaves it untouched — its routing keys on the `*.jtd.json`
  suffix») — правило ОБЯЗАНО оговаривать «JTD ⇒ generated, не-JTD ⇒
  рукописно законен», иначе красный зря.

**К3 — «владелец формата»: запрет по крейту-владельцу построенной JTD-схемы.**
- *формулировка:* рукописной derive запрещён во всём крейте-владельце
  формата, чья JTD-схема построена.
- *механика:* gate grep'ает только названные реестром крейты. **Поправка
  босса при перемере: поля `owner` в реестре НЕТ.** Запись несёт ровно шесть
  ключей — `corpus`, `epoch`, `foreign_parsers`, `recoverable`, `schema`,
  `sunset` (`grep -c owner formats/REGISTRY.toml` → 0; перечень ключей снят
  awk-ом по секциям `[format.*]`). Значит владельца пришлось бы либо ЗАВЕСТИ
  новым полем реестра, либо выводить из пути схемы — и это уже не «правило
  поверх готовых данных», а новая графа, которую надо сопровождать. Довод
  против К3 от этого только крепнет, а число ниже (41) верно и снято
  независимо.
- *числа:* владелец всех семи построенных форматов — vibe-cli ⇒ ловит 41
  файл (весь vibe-cli), пропускает 92.
- *чем ошибается:* поймает зря — `vvm/model.rs` и `vvm/placer.rs`
  (внутреннее состояние, не wire), `tree/artifacts.rs` (чтение
  сгенерированного); пропустит зря — эмиттер, вынесенный из крейта-владельца
  в соседний.
  Туп без файловой привязки; годится только как временный зонтичный запрет
  поверх К4.

**К4 — «рехет»: не членство, а дельта против базовой линии.**
- *формулировка:* сегодняшнее число файлов с рукописным derive по крейтам
  (таблица А) замораживается в baseline-файле; красный — любой рост счётчика.
- *механика:* `grep -lzE … | wc -l` по крейту против JSON — точная форма
  уже доказанного соседа: `conform-baseline.json` + `cargo xtask conform
  check` (шаг 5 панели).
- *числа:* пропускает 133/133 сегодня, ловит каждое добавление (по крейту).
- *чем ошибается:* пропустит зря — замена одного файла другим в том же
  крейте (сальдо 0) остаётся зелёной — новый файл проскочит, если
  одновременно удалён старый; поймает зря — перенос файла между крейтами
  (−1/+1) красный, хотя по сути ничего не изменилось. Нулевое смысловое
  содержание (гейт не отличает хорошее добавление от плохого), но нулевое
  же перечисление имён, и подъём базовой линии = обязательный
  человеческий момент.

**Рекомендация:** К2 как целевое правило, К4 как переходная ступень на
время волны аннотирования (133 тега), К1 не рекомендую (перечисление в
другой одежде), К3 — только как временный зонтик. Форма гейта — §7.

## 7. Grep-гейты, которые панель уже держит

Греп-гейты `tools/self-check.sh` (680 строк), форма каждого:

| шаг | строки | что запрещает | периметр | сообщение об отказе |
|---|---|---|---|---|
| 0c `check_instruction_triple` | 208-218 | расхождение триплета CLAUDE/AGENTS/GEMINI.md | три именованных файла; `cmp -s` (не grep) | «CLAUDE.md and $f differ — the instruction files are kept identical; reconcile the hand-edit into all three.» |
| 7 `check_core_slot_is_authored` | 368-377 | гейт замороженного слота | один файл `sync-engines.toml`; `grep -qF` фиксированной строки + подсказка `grep -n` | «\`$CORE_SLOT\` is not a sync-engines.toml source_root. … repoint CORE_SLOT at the authored core-ai-native slot:» |
| 10c `check_mcp_authored_denominator` | 513-530 | несоответствие conform-периметра фактическим authored-крейтам | ВЫВЕДЕННЫЙ знаменатель: awk по `sync-engines.toml` + `ls` + `comm` против `sed` из conform.toml | «\`$slot\` authored crates are {…}, but conform.toml scans {…}. Classify the newcomer — … Neither is optional.» |
| 10d `check_index_clock_gate` | 550-570 | `Utc::now(`/`SystemTime::now(` в модулях записи индекса | ТРИ каталога модулей, рекурсивно: `crates/vibe-index/src/{index,types,journal}` | многострочное, с правилом и рецептом (см. ниже) |
| 11b `check_lane_citations` | 588-602 | `@spec://…/boot/STATIC#` в авторском тексте | каталоги `spec/ packages/ crates/` + `--include='*.md'` + два исключения `grep -v` | «authored text targets a compiled STATIC lane (PROP-035 §11)» |
| 11c `check_member_licence_keys` | 614-638 | член workspace без `license`/`publish` ключа | ВЫВЕДЕННЫЙ знаменатель: `members` читается из корневого Cargo.toml (`sed`+`grep -oE`), затем `grep -qE` по каждому манифесту | «workspace members that do not declare PROP-000 §3 #license: … fix: add \`license-file.workspace = true\` …» |

**Образец для Ф4.3 — шаг 10d, индексные часовые ворота**: периметр по
каталогам модулей («a new file under any of them is covered the day it
lands», строки 542-543), фильтр формы строки перед вердиктом (комментарии
`//` легальны, 545-549), действующее многострочное сообщение с правилом,
спецификацией и рецептом. Целиком:

```bash
check_index_clock_gate() {
  local hits
  hits=$(grep -rnE 'Utc::now\(|SystemTime::now\(' \
      crates/vibe-index/src/index \
      crates/vibe-index/src/types \
      crates/vibe-index/src/journal \
      2>/dev/null \
    | grep -vE ':[0-9]+:[[:space:]]*//')
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    printf 'self-check: the index writer modules call the clock directly.\n' >&2
    printf 'self-check: the rule — time enters at the edge (CLI command or\n' >&2
    printf 'self-check: server mutation event) and never inside index/, types/ or journal/:\n' >&2
    printf 'self-check: one state must produce one byte sequence, or "rebuild and\n' >&2
    printf 'self-check: compare" measures nothing (PROP-044 §4.3, F2-1).\n' >&2
    printf 'self-check: fix: pass the time as an argument — a WriteCtx for\n' >&2
    printf 'self-check: write_to, an `at` for Index::new / VersionEntry::minimal.\n' >&2
    return 1
  fi
  return 0
}
run_step "index clock gate (no Utc::now/SystemTime::now in index/, types/ or journal/)" \
  check_index_clock_gate || OVERALL=$?
```

Для К2 вторая половина образца — философия выведенного знаменателя шага 11c
(«The member list is read from the workspace manifest rather than restated
here, so a crate added tomorrow is covered without anyone remembering this
step exists», строки 611-613): реестр форматов играет ту же роль, что
`members = [...]` в Cargo.toml.

## 8. Дыры и неожиданности

1. **Семь построенных форматов — один потребитель.** `grep -rln 'vibe_wire'
   crates xtask --include='*.rs'` → только `crates/vibe-cli/src/commands/
   init/helpers.rs` (+ тест самого vibe-wire и `xtask/src/codegen.rs`).
   `vibe list` шлёт машинный JSON из рукописных типов
   (`crates/vibe-cli/src/commands/list.rs:34,41` — `LockedSubskillJson`,
   `JsonEntry`) при живой построенной `schemas/list_report.jtd.json` и
   сгенерированном подмодуле `list_report`. То же — install/report.rs,
   registry/publish.rs, registry/sync.rs. Это и есть красная зона Ф4.3.
2. **Реестр называет семь несуществующих файлов схем.** `find schemas -name
   '*.jtd.json'` → только 7 плоских; записанные в REGISTRY.toml
   `schemas/index/e1/{entry,repomd,by_name,by_cap,by_purl}.jtd.json`,
   `schemas/journal/e1/journal.jtd.json`, `schemas/hello/e1/hello.jtd.json`
   отсутствуют. Реестр честно комментирует это как план («Schema paths for
   not-yet-built formats … recorded here as-is (Ф4.1 / Ф3.1 / Ф6.1)») — но
   правило «схема построена ⇒ generated» обязано проверять СУЩЕСТВОВАНИЕ
   файла схемы, а не просто запись.
3. **`cli-package-tree` — не-JTD схема.** JSON Schema 2020-12 живёт в
   `crates/vibe-cli/resources/package-tree.schema.v1.json`, codegen её не
   трогает (routing по суффиксу `*.jtd.json`, `xtask/src/codegen.rs:81-88`);
   эмиттер `commands/tree/model.rs` рукописен — законно, но без оговорки
   «JTD vs не-JTD» любое схемное правило ловит его зря.
4. **Шаблон гейта из плана в лоб почти слеп.** «`#[derive(Serialize`» не
   матчит ни `#[derive(Debug, …, Serialize)]` (почти все 133 файла), ни
   `#[derive(serde::Serialize)]` (8 файлов, §3). Нужен
   `#\[derive\([^)]*(Serialize|Deserialize)` (в Git Bash — `grep -lzE` с
   `LC_ALL=C`; `-P` падает по локали).
5. **`xtask/` — слепая зона периметра `crates/**`.** Единственный рукописный
   derive вне crates — `xtask/src/sync_engines.rs:36,44` (парсер авторского
   `sync-engines.toml`, строгий: `deny_unknown_fields` на обеих структурах).
   Записанное U1 «69» именно поэтому 69, а не 70.
6. **Один формат — два рукописных парсера.** `hash_recipe.rs` существует в
   vibe-index и vibe-registry; шапка: «This module is duplicated
   verbatim-in-intent in \`vibe-registry\`; the two MUST stay in lockstep
   (PROP-005 §3.2) … The parity test \`tests/content_hash_parity.rs\` gates
   that divergence» (`crates/vibe-index/src/hash_recipe.rs:19-23`). Формат
   `formats/hash_recipes/1.toml` описан данными, но парсеры — две
   независимые руки.
7. **Один словарь — два крейта.** `NamingConvention` в
   `vibe-index/src/types/kinds.rs:87` зеркалирует vibe-core:
   «Mirrors \`vibe-core::manifest::NamingConvention\` — the same four
   variants and the same wire strings» (kinds.rs:83-85); связь держит тест
   `naming_convention_serde_matches_vibe_core_wire` (kinds.rs:172). При
   генерации index-схем этот словарь — кандидат на shared-domain.
8. **Строгость авторских форматов неоднородна.** Манифест строг (U4),
   `sync-engines.toml` строг (`xtask/src/sync_engines.rs:37,45`),
   `registry.toml` строг (`git_registry.rs:39`) — но `progress.toml` мягкий:
   `crates/progress-core/src/scope.rs:41,52` — `ProgressSection`/`ScopeConfig`
   без `deny_unknown_fields`.
9. **Тестовые двойники формата.** 5 test-fixture файлов десериализуют вывод
   собственных CLI/сервера своими типами (`crates/vibe-cli/tests/
   cli_search.rs:46` и др.) — при миграции формата на generated они обязаны
   перейти на vibe-wire следом, иначе разъедутся с продакшн-типами.
10. **Записанное «roughly 63 places» устарело.** `lib.rs:21` (замер
    2026-08-06) против сегодняшних 48 атрибутов `deny_unknown_fields`:
    дельта 15 ≈ 16 снятий U3 — числа согласуются, но записанное число без
    даты-и-периметра в реестре строгости снова уйдёт; сам модуль
    `deny_unknown_fields` стоило бы взять в перепись Ф4.x.

## 9. Как воспроизвести этот замер

Только читающие команды, пути от корня рабочего дерева; временные списки
в `/tmp` (вне репо). Git Bash, обязателен `LC_ALL=C` (`grep -P` падает:
«-P supports only unibyte and UTF-8 locales»).

```bash
# 1. Периметр: .rs вне generated (619 кандидатов; schemas/ и formats/ .rs не содержат)
find crates xtask -name '*.rs' -not -path '*/generated/*' | wc -l   # 628-9

# 2. Три счётчика
find crates xtask -name '*.rs' -not -path '*/generated/*' -print0 \
  | xargs -0 env LC_ALL=C grep -lzE '#\[derive\([^)]*Serialize' | sort > /tmp/f4-ser.txt
find crates xtask -name '*.rs' -not -path '*/generated/*' -print0 \
  | xargs -0 env LC_ALL=C grep -lzE '#\[derive\([^)]*Deserialize' | sort > /tmp/f4-deser.txt
wc -l < /tmp/f4-ser.txt                                            # 122
wc -l < /tmp/f4-deser.txt                                          # 70
sort -u /tmp/f4-ser.txt /tmp/f4-deser.txt | wc -l                  # 133 (объединение)
comm -23 /tmp/f4-ser.txt /tmp/f4-deser.txt | wc -l                  # 63 (только Serialize)
comm -13 /tmp/f4-ser.txt /tmp/f4-deser.txt | wc -l                  # 11 (только Deserialize)
grep -vc '^xtask/' /tmp/f4-deser.txt                                # 69 (U1, crates/**)

# 3. Контроль однострочности derive (многострочных нет: вывод пуст)
find crates xtask -name '*.rs' -not -path '*/generated/*' -print0 \
  | xargs -0 env LC_ALL=C grep -lE '#\[derive\([^)]*Serialize' | sort > /tmp/f4-ser1.txt
comm -13 /tmp/f4-ser1.txt /tmp/f4-ser.txt

# 4. serde::-пути внутри derive (8 файлов)
find crates xtask -name '*.rs' -not -path '*/generated/*' -print0 \
  | xargs -0 env LC_ALL=C grep -lzE '#\[derive\([^)]*serde::(Serialize|Deserialize)'

# 5. deny_unknown_fields: рукописные / в generated
env LC_ALL=C grep -rn '#\[serde(deny_unknown_fields)\]' crates xtask --include='*.rs' \
  | grep -v '/generated/' | wc -l                                   # 48
env LC_ALL=C grep -rc 'deny_unknown_fields' crates/vibe-wire/src/generated/  # все :0

# 6. Таблица А (по крейтам, src/tests)
cat /tmp/f4-union.txt 2>/dev/null || sort -u /tmp/f4-ser.txt /tmp/f4-deser.txt > /tmp/f4-union.txt
sed 's|^crates/||; s|^xtask/|XTASK/|' /tmp/f4-union.txt \
  | awk -F/ '{k=($1=="XTASK")?"XTASK":$1; if ($0 ~ /\/tests\//) t[k]++; else s[k]++}
       END {for (k in s) printf "%s src=%d tests=%d\n", k, s[k], t[k]+0;
            for (k in t) if (!(k in s)) printf "%s src=0 tests=%d\n", k, t[k]}'

# 7. Реестр и схемы
ls formats/ && ls schemas/ && ls crates/vibe-wire/src/generated/
env LC_ALL=C grep -rln 'vibe_wire' crates xtask --include='*.rs'    # потребители generated
```

Замер семантических корзин (таблица Б) воспроизводится только чтением
шапок файлов (команда вида `sed -n '1,14p' <файл>` по списку §5) — это
суждение, а не grep; списки файлов по корзинам приведены в §5 для сверки.
