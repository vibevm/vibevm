# WORKER-REPORT-P3-M4 — обязательства документации: документация, факты и прогресс, CLI, MCP, действия, настройки

Пакет: `campaigns/docs-2026-09/findings/PACKET-P3-M4.md` + общая часть
`PACKET-P3-M-COMMON.md` (атом A3.13). Ветка `research-preview-1-docs`, без push.
Дата: 2026-09-12.

## Коротко для оркестратора

Прошёл все четырнадцать документов пакета, проставил **131 пометку** в
**одиннадцати** из них; три документа дали ноль осознанно (см. «Документы с
нулём»). Скрипт — `campaigns/docs-2026-09/findings/P3-M4-mark.py`. Диффа сверх
вставленного атрибута нет: 131 изменённая строка, каждая отличается от прежней
ровно на ` action="continue" actionstage="doc" audience="…"`, проверено
механически. Гейты зелёные. Один коммит. Незакоммиченных моих правок в дереве не
осталось.

По аудиториям (маркеры внутри моих четырнадцати документов, было → стало):

| Аудитория | До | После | Мой вклад |
|---|---|---|---|
| `user` | 5 | 67 | +62 |
| `author` | 13 | 57 | +44 |
| `dev` | 0 | 0 | 0 |
| `agent` | 0 | 26 | +26 |

131 факт, 132 слота аудитории — `READER-NUMBERED-BLOCKS` несёт `user,agent`.
Исходные 5 + 13 — это восемнадцать образцовых пометок в PROP-057, они не
трогались.

## Решения

1. **Критерий применялся дословно: «не сможет пользоваться, не зная».** Помечено
   то, обо что человек этой аудитории спотыкается сам — команда и её
   обязательный результат, формат, который он пишет руками, отказ и его причина,
   умолчание, на которое он полагается, ограничение, которое его остановит.
   Обоснования, отвергнутые варианты, хроники, внутренние инварианты движка,
   имена крейтов и швов — не помечено ни разу.

2. **`dev` — ноль на все четырнадцать документов.** Ни один из них не входит в
   предмет трёх страниц архитектуры руководства (слои крейтов, швы, закон
   wire-форматов, карта трассируемости, правила коммитов). Там, где документ
   говорит о внутренностях vibevm (PROP-039 §1/§12, PROP-040 §12–§13,
   PROP-037 §0–§2), пометки нет вовсе, а не `dev`.

3. **`agent` — там, где поверхность прямо объявлена агентской.** PROP-042
   («`vibe aiui` — the agent-facing command family»), инструменты MCP и
   `query`/`select` из PROP-015, грамматика `tcg_*` из PROP-026, `action://` и
   `ModelView` из PROP-039, машинные проекции сайта (`.md`/`.xml`, `llms*.txt`,
   `manifest.json`, `resolve`) из PROP-057. Это ровно то, что общая часть
   перечисляет как агентское, плюс `READER-NUMBERED-BLOCKS` — номер блока
   `pNN` существует именно чтобы человек и агент цитировали одно место.

4. **Разметка фактов (PROP-043 facts) размечена как `author`, а не `dev`.**
   Подсказка пакета говорит так, и документ сам это подтверждает: слой
   спроектирован как отдельный извлекаемый продукт (§2 `##SEPARABILITY-LAW`),
   а `facts.toml` лежит в корне *пакета*. Это грамматика, которую автор пакета
   пишет руками, — самый плотный по обязательствам документ пакета.

5. **Помечены и кебабные якоря** (PROP-040 `precedence-law`, `schema-first`,
   `prefs-command`, …; PROP-041 `tree-widget-req`, `write-layer-choice`, …).
   Регистр из PROP-043 `##DECISION-TWO-REGISTERS` — сигнал нормативности для
   Markdown-якорей, и эти два документа его просто не применяли: их кебабные
   единицы несут «REQ {#…}» и нормативны буквально. Критерий — потребность
   читателя, а не написание имени.

6. **Ретированные разделы не помечались.** В PROP-026 `##RETIRED-SECTIONS-KEPT`
   объявляет §3–§5 проектной записью ушедшей топологии — оттуда ноль пометок,
   хотя тексты несут `impl/done`. То же соображение сняло §0 PROP-037 (аудит
   тонкости) и §14 PROP-057 (кухня разработчиков документации).

7. **Объём.** 131 пометка на 1201 факт `done` в пакете — 10,9 %. По документам
   разброс 0 %…16,4 % (таблица ниже). Четыре документа вышли за 15 %: PROP-015
   (17,1 %), PROP-042 (16,4 %), PROP-036 (16,3 %), PROP-027 (15,3 %). Это
   документы, почти целиком состоящие из командной поверхности — список глаголов,
   их флаги и их отказы; урезать их до 15 % значило бы оставить какой-то из
   глаголов нерассказанным. Ориентир в 5–15 % считался ориентиром, а не гейтом;
   если центральная сессия решит иначе, кандидаты на снятие названы в «сомнениях»
   со звёздочкой.

## Отклонения

Нет. Единственная правка файлов — вставка строки
` action="continue" actionstage="doc" audience="…"` сразу после `status="…"`.
Ни один другой байт не изменён: ни текст, ни переносы, ни отступы, ни порядок
фактов, ни имена якорей; новых фактов не добавлено.

## Гейты

### 1. `target/debug/vibe.exe facts check --exhaustive`

| | файлов | предупреждений | вердикт |
|---|---|---|---|
| до правок | 380 | 24 | `progress check: clean` |
| после правок | 372 | 24 | `progress check: clean` |

Восемь файлов ушли из корпуса **не из-за меня**: соседний воркер добавил в
`facts.toml` исключение `vibevm/vibepacks/org.vibevm.core/vibevm-docs/*/examples/**`
(фикстуры примеров руководства). Мои правки — только вставка атрибута в уже
наблюдаемые файлы, они не могут менять счётчик файлов. Предупреждений как было
24, так и осталось; выход 0 в обоих случаях.

### 2. `target/debug/vibe.exe progress report --view doc --audience <A>`

Счёт `<marker …>` внутри моих четырнадцати документов (глобальные числа растут и
от соседних воркеров, поэтому изолировано по `path=`):

| Аудитория | До | После |
|---|---|---|
| `user` | 5 | 67 |
| `author` | 13 | 57 |
| `dev` | 0 | 0 |
| `agent` | 0 | 26 |

Пофайлово после правок: PROP-057 — 8 `user` / 25 `author` / 4 `agent`;
PROP-043 (facts) — 24 `author`; PROP-036 — 13 `user`; PROP-037 — 15 `user`;
PROP-042 — 2 `user` / 8 `agent`; PROP-015 — 7 `user` / 1 `author` / 6 `agent`;
PROP-027 — 5 `user` / 6 `author`; PROP-026 — 5 `agent`; PROP-039 — 1 `author` /
3 `agent`; PROP-040 — 11 `user`; PROP-041 — 6 `user`.

### 3. `python campaigns/docs-2026-09/tasks/zone-gate.py`

```
zone gate: clean
```

выход 0.

### 4. `git diff --stat` по моим спекам

```
 .../PROP-057-documentation-packages-and-site.xml   | 36 ++++++++--------
 .../vibe-actions/PROP-039-action-system.xml        |  8 ++--
 .../modules/vibe-cli/PROP-036-package-tree.xml     | 26 ++++++------
 .../modules/vibe-cli/PROP-037-tree-tui.xml         | 30 +++++++-------
 .../modules/vibe-cli/PROP-042-aiui-observation.xml | 20 ++++-----
 .../modules/vibe-facts/PROP-043-facts-markup.xml   | 48 +++++++++++-----------
 .../modules/vibe-mcp/PROP-015-mcp-integration.xml  | 28 ++++++-------
 .../modules/vibe-mcp/PROP-026-tcg-tool-family.xml  | 10 ++---
 .../modules/vibe-mcp/PROP-027-mcp-packages.xml     | 22 +++++-----
 .../modules/vibe-settings/PROP-040-settings.xml    | 22 +++++-----
 .../modules/vibe-settings/PROP-041-settings-ui.xml | 12 +++---
 11 files changed, 131 insertions(+), 131 deletions(-)
```

131 изменённая строка = 131 пометка. Пары «минус/плюс» здесь — это правка строки
на месте, а не перезапись: отдельной проверкой снят `git diff -U0` по моим
одиннадцати файлам и для каждой пары показано, что удаление вставленного
атрибута из новой строки даёт прежнюю строку байт в байт (131/131, отклонений 0).

## Таблица пометок

### `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml` (18 новых)

| Якорь | Аудитории | Почему |
|---|---|---|
| `KIND-APP-VS-TOOL` | author | выбирая вид, автор обязан знать границу `app`/`tool` |
| `LEVEL-ZERO` | author | любой опубликованный пакет получает страницу сайта даром |
| `REL-OFFICIAL-IS-CONVERGENCE` | author | официальность = оба ребра; одно ребро — community |
| `REL-DEFAULT-CONVENTION` | author | без `[documentation]` действует соглашение `<name>-docs` |
| `LOC-LANGUAGE-FIELD` | author | язык перевода — `[i18n].canonical`, поля `lang` нет |
| `LOC-DOCUMENTS-MATCH` | author | `documents` перевода обязан совпасть; `vibe check` откажет |
| `LOC-OFFICIAL-TRANSLATION` | author | три условия официальности перевода |
| `SITE-MOUNT` | user | детерминированная карта `spec://` → адрес сайта |
| `SITE-VERSION-SHOWS-CURRENT` | user | адрес с версией показывает текущее; вечных ссылок нет |
| `READER-NUMBERED-BLOCKS` | user,agent | номер `pNN` — общее место цитаты человека и агента |
| `SEO-RAW-PROJECTIONS` | agent | каждая страница отдаётся как `.md` и `.xml` |
| `SEO-LLMS-FILES` | agent | `llms.txt`, `llms-full.txt` и бюджетные варианты |
| `SEO-MANIFEST-AND-RESOLVER` | agent | манифест страниц и резолвер `spec://` |
| `OBS-RULE-EDGE-UNPINNED` | author | `rule` цитирует живьём; исчезнувший якорь валит сборку |
| `OBS-COVERAGE-GATE` | author | обязательства и `vibe doc check --coverage` |
| `STYLE-SOURCE-LANGUAGE` | author | источник английский, остальные языки — адаптации |
| `STYLE-PROMPT-FIRST` | author | контракт блока `prompt` и обязательный `assert` |
| `STYLE-LINT` | author | `vibe doc check --style` — гейт страницы |

### `vibevm/vibespecs/modules/vibe-facts/PROP-043-facts-markup.xml` (24, все `author`)

| Якорь | Почему |
|---|---|
| `STATUS-ELEMENT` | единственный элемент и две его формы |
| `POINT-SELF-CLOSING` | незакрытый точечный маркер — ошибка `check` |
| `FENCE-AWARE` | внутри заборов и кода разметка не распознаётся |
| `VOCAB-CLOSED` | словари закрыты; чужое значение — ошибка с подсказкой |
| `MULTI-MARKERS` | один статусный маркер, action-маркеров сколько угодно |
| `ACTIONSTAGE-NARROWS` | что именно сужает `actionstage` |
| `AUDIENCE-DOC-USE` | `actionstage="doc"` + аудитория = обязательство документации |
| `SHORTHAND-FORMS` | сокращённая форма `@status:<stage>/<state>` |
| `SHORTHAND-BARE` | голое сокращение подразумевает `state="work"` |
| `PLACE-DOCUMENT` | где стоит документный маркер |
| `PLACE-SECTION` | где стоит секционный маркер |
| `PLACE-PARAGRAPH` | где стоит маркер абзаца |
| `PLACE-LIST-ITEM` | каждый элемент списка — своя единица |
| `NO-ORPHAN-MARKER` | одинокий маркер между абзацами — ошибка `check` |
| `FACT-ANCHOR-SYNTAX` | `@fact:<ID>` первым токеном |
| `ANCHORED-WHEN-MARKED` | размеченная единица обязана нести якорь |
| `FACT-ID-GRAMMAR` | грамматика `<ID>` и адрес `spec://…#<ID>` |
| `ROLLUP-DOWNWARD` | маркер узла покрывает неразмеченных потомков |
| `ROLLUP-UPWARD` | статус неразмеченного узла — худшее из детей |
| `REQUIRES-GRAMMAR` | грамматика `@requires:` и её XML-эквивалент |
| `REQUIRED-ARTIFACT-KINDS` | закрытый словарь артефактов закрытия |
| `CONFIG-FILE` | `facts.toml` в корне пакета |
| `INCLUDE-STYLE` | глобы include-стиля: что наблюдается |
| `BOUNDARY-CLI` | `vibe facts check [--exhaustive]` — дом линтера |

### `vibevm/vibespecs/modules/vibe-cli/PROP-036-package-tree.xml` (13, все `user`)

| Якорь | Почему |
|---|---|
| `TREE-ANSWER` | что вообще отвечает `vibe tree` |
| `TREE-READ-ONLY` | ничего не меняет — гарантия, на которую полагаются |
| `TREE-INPUTS` | что читает и откуда (`--path`, умолчание) |
| `OUT-TUI` | на tty по умолчанию интерактив |
| `OUT-JSON` | машинная поверхность |
| `OUT-PLAIN` | не-tty и `--plain` дают ASCII-дерево |
| `ROW-PER-PACKAGE` | строка = пакет, порядок колонок |
| `EFFECTIVE-FROM-ARTIFACTS` | `load` читается из закоммиченных артефактов |
| `STATIC-SIZE-INDICATOR` | бюджет статической полосы в статус-строке |
| `JSON-CONTRACT` | `--json` валиден против поставляемой схемы |
| `DIAG-STALE-ARTIFACTS` | диагноз с рецептом `vibe reinstall` |
| `DIAG-ROOT-DRIFT` | lock отстал от `[requires.packages]` |
| `DAG-RENDERING` | что значит `(*)` и почему ветка не разворачивается |

### `vibevm/vibespecs/modules/vibe-cli/PROP-037-tree-tui.xml` (15, все `user`)

| Якорь | Почему |
|---|---|
| `TIER-DETECTION` | определение тира, умолчание Tier 3, `vibe.tree.tier` |
| `THREE-SHAPES` | три формы дерева и умолчание |
| `THREE-MODES` | три режима отображения |
| `MODE-SELECT-REQ` | режим выбирается меню F3 и сохраняется |
| `F-KEY-SCHEME` | вся раскладка F1–F6 |
| `TREE-KEYS-REQ` | стрелки, `Space`, `Enter`, `Shift`+стрелки |
| `FOCUS-GROUPS-REQ` | `Tab`/`Shift+Tab` между группами фокуса |
| `MODAL-STACK-REQ` | `Esc` снимает модалку; на дне — подтверждение выхода |
| `F2-SORT-MENU-REQ` | F2 — сортировка, содержимое зависит от режима |
| `F1-SEARCH-REQ` | F1 — Search Everywhere |
| `QUIT-CONFIRM-REQ` | как вообще выйти из приложения |
| `SETTINGS-PERSISTENCE` | где лежит состояние UI и что при порче файла |
| `COPY-PROVIDERS-REQ` | «что вижу, то и копирую» |
| `COPY-FLOW-REQ` | F6 и `Shift+F6` |
| `PNG-RESERVED` | выбор PNG упирается в «coming soon» |

### `vibevm/vibespecs/modules/vibe-cli/PROP-042-aiui-observation.xml` (10)

| Якорь | Аудитории | Почему |
|---|---|---|
| `AIUI-FAMILY` | agent | `vibe aiui` — агентское семейство, сигнатура `render` |
| `RENDER-VERB-SEMANTICS` | agent | что делает `render` и его умолчания |
| `SNAPSHOT-FORMATS` | agent | два формата снимка, одна схема на все планы |
| `FMT-CELLS` | agent | форма JSON, которую агент разбирает |
| `KEY-SCRIPT-GRAMMAR` | agent | грамматика `--send` |
| `SIDE-EFFECT-KEYS-REFUSED` | agent | `F4`/`F6` отвергаются, а не выполняются |
| `TERMINAL-VERBS` | agent | глаголы живой сессии терминала |
| `MODEL-VERB` | agent | `vibe aiui state` — план модели |
| `TERM-LAUNCHER` | user | что запускает `vibe term` |
| `IN-PLACE-UPGRADE` | user | `vibe tree` внутри vibeterm не открывает второе окно |

### `vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml` (14)

| Якорь | Аудитории | Почему |
|---|---|---|
| `SURFACE-SERVER` | user | `vibe mcp serve` отдаёт состояние lock-файла инструментами |
| `SURFACE-INSTALL` | user | одна команда вместо правки пяти конфигов |
| `TOOL-QUERY-PACKAGE` | agent | что возвращает инструмент и что он read-only |
| `TOOL-READ-SUBSKILL` | agent | читает байты при любом режиме доставки |
| `TOOL-MATERIALISE-SUBSKILL` | agent | единственный пишущий; без `force` не перезапишет |
| `MAP-QUERY-ANSWERS-A-DIFFERENT-QUESTION` | agent | когда `query`, а когда `explain` |
| `SELECT-SEVEN-PREDICATES-JOINED-BY-AND` | agent | весь язык `select` без операторов |
| `SELECT-AN-UNKNOWN-PREDICATE-IS-AN-ERROR` | agent | ошибка называет токен; пустой запрос отвергнут |
| `AGENT-SET` | user | какие агенты поддержаны |
| `CONFIG-PATH` | user | какой файл правится и почему не `settings.json` |
| `CONFIG-MERGE` | user | чужие ключи и их порядок сохраняются |
| `SKILL-MANIFEST` | user | install пишет ещё и `SKILL.md` |
| `LIFECYCLE-MATRIX` | user | все глаголы идемпотентны, есть `--dry-run` |
| `INCLUDE-SELECTIVE` | author | `include` в `[[skill]]` и поведение без него |

### `vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml` (11)

| Якорь | Аудитории | Почему |
|---|---|---|
| `MCP-KIND-DEF` | author | что такое пакет вида `mcp` |
| `TABLE-ONLY-IN-KIND` | author | `[[mcp_server]]` легален только здесь |
| `KIND-PROMISES-SERVER` | author | вид без сервера отвергается |
| `SERVER-IS-BINARY` | author | `binary` обязан ссылаться на `[[binary]]` того же манифеста |
| `ARGS-CLOSED-SET` | author | только `{project_root}`; чужой токен отвергнут |
| `EXACT-PIN-LAW` | author | все зависимости пакета `mcp` — точные `=X.Y.Z` |
| `REG-PACKAGE-DISCOVERY` | user | установленные `mcp`-пакеты сами попадают в конфиги |
| `REG-MANAGED-SIDECAR` | user | чужие серверы оператора не трогаются |
| `REG-PROJECT-SCOPE` | user | регистрация только проектная |
| `REG-STATUS` | user | несобранный артефакт и рецепт `vibe bin build` |
| `CONSENT-GATE-INHERITED` | user | чужая группа требует `--assume-yes` |

### `vibevm/vibespecs/modules/vibe-mcp/PROP-026-tcg-tool-family.xml` (5, все `agent`)

| Якорь | Почему |
|---|---|
| `TOOLS-NEW-HOME` | инструменты живут в пер-языковых `mcp`-пакетах |
| `FOUR-TOOLS` | какие четыре инструмента вообще есть |
| `PARAM-LANGUAGE` | обязательный параметр и ошибка со списком поддержанных |
| `PARAMS-PASSTHROUGH` | остальные параметры идут насквозь |
| `ENRICHED-RESPONSES` | что лежит в `structuredContent` |

### `vibevm/vibespecs/modules/vibe-actions/PROP-039-action-system.xml` (4)

| Якорь | Аудитории | Почему |
|---|---|---|
| `ADDRESS-GRAMMAR` | agent | форма `action://<group>/<name>[?params]` |
| `MODEL-VIEW-DEF` | agent | сериализуемое состояние UI вместо пикселей |
| `AIUI-REFERENCE` | agent | `list_actions` / `invoke` / `state` / `search` |
| `I18N-FALLBACK-LAW` | author | пакет может привезти `locales/<lang>.ftl`, приоритеты явные |

### `vibevm/vibespecs/modules/vibe-settings/PROP-040-settings.xml` (11, все `user`)

| Якорь | Почему |
|---|---|
| `app-prefs-not-project` | что здесь хранится и чем это не `vibe.toml` |
| `L1-USER-MACHINE` | путь и роль слоя L1 |
| `L2-REPO-SHARED` | L2 коммитится — командные настройки |
| `L3-USER-PROJECT` | L3 в gitignore — личная донастройка |
| `precedence-law` | закон приоритета слоёв целиком |
| `MERGE-ARRAYS` | массив заменяется, а не склеивается |
| `missing-is-default` | отсутствующий или битый файл — не ошибка |
| `schema-first` | неизвестный ключ громко предупреждает, а не молчит |
| `prefs-command` | вся поверхность `vibe prefs` |
| `gitignore-autogen` | `vibe init` сам закрывает личный файл от коммита |
| `no-secrets-in-committed` | секреты запрещены, `vibe prefs check` откажет |

### `vibevm/vibespecs/modules/vibe-settings/PROP-041-settings-ui.xml` (6, все `user`)

| Якорь | Почему |
|---|---|
| `tree-widget-req` | левая панель и её клавиши |
| `tree-context` | без проекта видны только страницы L1 |
| `write-layer-choice` | в какой слой пишется правка и когда отказ |
| `provenance-view` | «откуда взялось это значение» |
| `provenance-edit` | правка конкретного слоя и сброс к нижнему |
| `settings-search` | поиск настроек по ключу, имени, описанию, синонимам |

## Документы с нулём пометок

| Документ | `done` | Почему ноль |
|---|---|---|
| `modules/vibe-progress/PROP-047-progress-campaigns.xml` | 64 | Инструмент кампаний рефакторинга самого vibevm. Мандат документа (`##CAMPAIGN-LAYER-MANDATE`, распоряжение владельца 2026-08-22) прямо говорит: этот слой дозревает, пока не станет достаточно хорош, чтобы показать миру. Рассказывать его руководству рано; единственный кусок, который наружу уже нужен — список обязательств `--view doc --audience` — помечен в PROP-043 `##AUDIENCE-DOC-USE` и PROP-057 `##OBS-COVERAGE-GATE`. |
| `modules/vibe-progress/PROP-043-progress-markup.xml` | 1 | Надгробие после разделения facts/progress: один факт-указатель на два новых дома. Указатель — не обязательство перед читателем. |
| `modules/vibe-progress/OWNER-GUIDE.xml` | 70 | Русскоязычный гайд владельца по тому же инструменту кампаний (жанр — guide, не контракт), и его собственный документный маркер несёт `action="drift"`. Предмет тот же, что у PROP-047, — ноль по той же причине. |

## Сомнения — рассмотрено и не помечено

Звёздочкой помечены кандидаты на **снятие** пометки, если центральная сессия
решит держаться 15 % жёстко.

| Документ | Якорь | Аудитория, если бы | Причина не помечать |
|---|---|---|---|
| PROP-057 | `OBS-MAINTENANCE-TOOLS` | author | `vibe doc todo` / `surface` / `diff` — «кухня разработчиков документации», §14 сама так их зовёт |
| PROP-057 | `OBS-NOT-JUDGED` | dev | механизм `[judging] exempt` в `facts.toml` хоста — внутренняя настройка корпуса |
| PROP-057 | `STYLE-STE`, `STYLE-READER`, `STYLE-REGISTER`, `STYLE-HUMOUR` | author | норма прозы *нашего* руководства (`AUTHORING.md`), не закон для чужих `doc`-пакетов; мехчасть закрыта `STYLE-LINT` |
| PROP-057 | `SITE-TRAILING-SLASH` | user | адрес со слэшем — следствие `SEO-RAW-PROJECTIONS` и `SITE-MOUNT` |
| PROP-057 | `READER-SETTINGS`, `READER-FOR-AGENT`, `READER-LANGUAGE-SWITCH-KEEPS-PLACE` | user | поведение читалки; ничто из этого читателя не останавливает |
| PROP-043 (facts) | `DECISION-TWO-REGISTERS` | author | соглашение об именовании, не отказ инструмента ⭐ |
| PROP-043 (facts) | `FENCE-IS-AN-EXAMPLE-UNTIL-MARKED` | author | типизированный `@fact/code:` — продвинутая форма, без неё живётся |
| PROP-043 (facts) | `STATE-VOID`, `STAGE-ORDER`, `TABLE-ADDRESSING`, `CONFIG-EXCLUDE`, `DEFAULT-EXCLUDES`, `NESTED-OWNS-SUBTREE` | author | второй эшелон грамматики; помечать весь словарь значило бы попросить руководство процитировать его целиком |
| PROP-047 | `CMD-REPORT`, `DOC-COVERAGE-RATCHET` | dev | см. «Документы с нулём»: сам слой ещё не показывают |
| PROP-036 | `RES-GIVEN-PATH`, `RES-LAST-PROJECT`, `RES-FOLDER-PICKER` | user | разрешение проекта нужно в основном GUI-лаунчеру из другого репозитория |
| PROP-036 | `COL-LOAD`, `COL-TRANSITIVE`, `COL-CONDITION`, `COL-STATIC`, `LOAD-*` | user | колонки покрыты `ROW-PER-PACKAGE` + `EFFECTIVE-FROM-ARTIFACTS`, прозой вокруг цитаты |
| PROP-037 | `FOUR-TIERS` | user | сам перечень тиров; отказ и ручка закрыты `TIER-DETECTION` |
| PROP-037 | `MARKDOWN-EXPORT-REQ`, `COPY-DEST-REQ`, `DETAIL-CARD-REQ`, `MODE-TREE/SUBTABLES/TABS-REQ` | user | детали, не мешающие пользоваться; `THREE-MODES` и `COPY-FLOW-REQ` держат целое |
| PROP-042 | `PROP-SCOPE`, `RENDER-DETERMINISTIC`, `UNKNOWN-KEY-ERROR`, `SESSION-DEFAULT`, `APP-RESOLUTION` | agent/user | ⭐ `SESSION-DEFAULT` и `UNKNOWN-KEY-ERROR` — самые близкие к порогу; `RENDER-DETERMINISTIC` — забота голденов, не агента |
| PROP-015 | `SERVER-FRESH-LOCKFILE`, `TOOL-FAILURE-RENDERING`, `CONFIG-WINDOWS-SHIM`, `AGENT-PRESENCE`, `MAP-QUERY-THREE-FILTERS-AND-A-CEILING` | agent/user | ⭐ последний — самый близкий к порогу: потолок выдачи агент увидит сам, он о нём сообщает |
| PROP-015 | `VERB-INSTALL/STATUS/UPGRADE/UNINSTALL` | user | четыре глагола покрыты `LIFECYCLE-MATRIX`, руководство раскроет прозой |
| PROP-027 | `SERVER-NAME`, `VIBE-FREE-SERVING`, `PIN-VALIDATION`, `REG-UNINSTALL`, `COMPOSITION-LAW`, `KIND-REGISTER` | author/user | ⭐ `SERVER-NAME` и `VIBE-FREE-SERVING` ближе всего к порогу; `PIN-VALIDATION` — форма отказа для уже помеченного `EXACT-PIN-LAW` |
| PROP-026 | `LANGUAGE-COMPAT-PARAM`, `NG-NO-AUTODETECT` | agent | перекрываются `PARAM-LANGUAGE` |
| PROP-026 | всё из §3–§5 | — | `##RETIRED-SECTIONS-KEPT`: ушедшая топология, проектная запись |
| PROP-039 | `CONTEXT-INTROSPECTABLE`, `SE-RANKING-LAW`, `SE-PROVIDERS-SHIP`, `SE-TABS-LAW` | agent/user | поведение Search Everywhere уже обязательно через PROP-037 `##F1-SEARCH-REQ` |
| PROP-040 | `scope-metadata`, `scope-matrix` | user | `scope` объявляет тот, кто заводит ключ, а ключи сегодня заводит только vibevm |
| PROP-040 | `merge-algorithm`, `null-semantics`, `deprecation`, `applies`, `show-origins-req`, `file-watch`, `path-classifier`, `restricted-l2`, `diff-from-default`, `vs-project-config` | user | ⭐ `null-semantics`, `deprecation` и `show-origins-req` ближе всего к порогу; `show-origins` назван внутри `prefs-command` |
| PROP-041 | `form-per-type`, `apply-indicator`, `validation-feedback`, `lint-all`, `declarative-pages` | user | форма и обратная связь; ⭐ `validation-feedback` ближе всего к порогу |

## Факты, уже несущие `action=` / `actionstage=`

Решение по ним — центральной сессии; я их не трогал.

| Документ | Якорь | Что стоит | Прошёл бы критерий? |
|---|---|---|---|
| PROP-043 (facts) | `AUDIENCE-VALUES` | `action="continue" actionstage="impl"` | **Да, `author`.** Это сам словарь аудиторий (`user` / `author` / `dev` / `agent`, умолчание `dev`) — без него автор не поставит ни одной пометки. `actionstage="impl"` здесь про недоделанную поддержку `agent` в коде, а не про документацию; строго по PROP-043 `##MULTI-MARKERS` на одну единицу можно повесить несколько action-маркеров, так что doc-обязательство добавляется, не вытесняя impl-задачу. |
| PROP-043 (facts) | `ROW-ATTR-AUDIENCE-VALUES` | `action="continue" actionstage="impl"` | **Да, `author`**, по той же причине — это ячейка таблицы атрибутов с тем же словарём. Если центральная сессия примет предыдущую строку, эту разумно пометить вместе с ней; если нет — достаточно одной. |

Прочие восемнадцать фактов PROP-057 уже несут `actionstage="doc"` — это образец
кампании, он не требует решения.

## Числа по документам

| Документ | фактов `done` | помечено мной | доля | всего doc-пометок |
|---|---|---|---|---|
| `common/PROP-057-documentation-packages-and-site.xml` | 249 | 18 | 7,2 % | 36 (с образцом) |
| `modules/vibe-facts/PROP-043-facts-markup.xml` | 158 | 24 | 15,2 % | 24 |
| `modules/vibe-progress/PROP-047-progress-campaigns.xml` | 64 | 0 | 0 % | 0 |
| `modules/vibe-progress/PROP-043-progress-markup.xml` | 1 | 0 | 0 % | 0 |
| `modules/vibe-progress/OWNER-GUIDE.xml` | 70 | 0 | 0 % | 0 |
| `modules/vibe-cli/PROP-037-tree-tui.xml` | 151 | 15 | 9,9 % | 15 |
| `modules/vibe-cli/PROP-036-package-tree.xml` | 80 | 13 | 16,3 % | 13 |
| `modules/vibe-cli/PROP-042-aiui-observation.xml` | 61 | 10 | 16,4 % | 10 |
| `modules/vibe-mcp/PROP-015-mcp-integration.xml` | 82 | 14 | 17,1 % | 14 |
| `modules/vibe-mcp/PROP-027-mcp-packages.xml` | 72 | 11 | 15,3 % | 11 |
| `modules/vibe-mcp/PROP-026-tcg-tool-family.xml` | 46 | 5 | 10,9 % | 5 |
| `modules/vibe-actions/PROP-039-action-system.xml` | 51 | 4 | 7,8 % | 4 |
| `modules/vibe-settings/PROP-040-settings.xml` | 73 | 11 | 15,1 % | 11 |
| `modules/vibe-settings/PROP-041-settings-ui.xml` | 43 | 6 | 14,0 % | 6 |
| **Итого** | **1201** | **131** | **10,9 %** | **149** |

## Что не сделано

Ничего из пакета не пропущено. Вне пакета осталось одно решение, которое я
принять не вправе: две строки таблицы «Факты, уже несущие `action=`» —
`AUDIENCE-VALUES` и `ROW-ATTR-AUDIENCE-VALUES` в PROP-043 (facts). Оба
проходят критерий как `author`, но несут чужой `actionstage="impl"`, и общая
часть пакета велит отдать такие случаи центральной сессии.
