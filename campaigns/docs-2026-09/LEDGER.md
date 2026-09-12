# LEDGER — документация 2026-09 {#root}

<status stage="spec" state="work" comment="леджер исполнения кампании documentation по форме campaign-plans; ведётся с 2026-09-11; фаза 0 принята, фаза 1 села, фаза P в работе"/>

Статус: **фаза 0 принята 2026-09-11** (26 спайков, коммитов ноль, внешние
каталоги не тронуты); **фаза 1 села 2026-09-12** — 16 коммитов по карте ниже;
A1.1/A1.2 закоммичены `ed75ce00` под целью сессии владельца («документация
сделана и задеплоена») и ждут его ратификации при слиянии ветки; **фаза P
в работе** — 44 страницы написаны и закоммичены по разделам (карта ниже);
снятие `expect` (PP-C2), инвентарь legacy (PP-C3) и проверки прозы (PP-C1)
у воркеров; следующее: вставка `expect`, прогон промптов P.7b, стиль-ревью
P.8, пакет передачи P.9.

Ветка `research-preview-1-docs` от `main@b1291b06`, тег
`research-preview-1-implementation`; worktree `vibevm-docs`.

## Фаза 0 — находки {#phase-0}

Полные вердикты — `PHASE-0-FINDINGS.md`; сырые находки — `findings/`.

| Спайк | Результат | Решение затронуто | Дата |
|---|---|---|---|
| A0.1 перепись kind | 5 исчерпывающих матчей в продукте, 1 в тесте; `BootCategory::App` уже есть | D-01, D-05 подтверждены | 2026-09-11 |
| A0.2 wire-словарь | ядро и wire — два пути; `known()` и три статических теста растут руками; break-заметки необязательны до `public = true` | D-01 | 2026-09-11 |
| A0.3 манифест | `ManifestWire` + `Manifest` + два `TryFrom`; `min_vibe_version` нигде не проверяется | F-05 переформулирован (владельцу) | 2026-09-11 |
| A0.4 пивот | словарь — параметр читателя; `when` — слот; дискриминатор `title=`; MD-проекция необратима | D-10 уточнено | 2026-09-11 |
| A0.5 аудитория agent | 12 мест правки; `--view doc` — фильтр, не гейт | D-11 подтверждено; PROP-043/047 поправлены | 2026-09-11 |
| A0.6 рёбра | хостовый сканер через шов `CodeScanner`; движок не трогать | D-14, D-17 | 2026-09-11 |
| A0.7 HTTP-сервер | отдельный крейт, CSP строкой, `--frame-ancestor`, без CORS; **P1-кандидат в `vibe-index`** (X-017, владельцу) | D-09 уточнено | 2026-09-11 |
| A0.8 store | API store/lock/реестра; `cache add` лжёт про «bytes untouched» (X-001) | D-01 | 2026-09-11 |
| A0.9 скиллы | `[[skill]]` по образцу; строка во встроенный шаблон безусловная | §7.4 | 2026-09-11 |
| A0.10 Qwik | пины 2.0.0-beta.43 / Vite 8.2.1 / Node 24.18.0 / pnpm 10.33.2; `ssg`; `base: "/"`; слэш обязателен; SSG молчит при недогенерации | D-06, D-12 уточнены | 2026-09-11 |
| A0.11 встраивание | `include_dir` + `build.rs`; оболочка — отдельный релизный актив с `DOC-SHELL.json`; `vibe doc shell install` | D-12 уточнено | 2026-09-11 |
| A0.12 раннер | макет на семи примерах; `VIBE_SETTINGS` + tripwire; без `match`; валидатор JTD в `vibe-doc` | D-10 уточнено | 2026-09-11 |
| A0.13 legacy | 1 184 утверждений, 90.6 % живо — предсказание PHASE-G не подтвердилось | D-15 | 2026-09-11 |
| A0.14 TS floor | floor хостовым бинарником; все семь шагов; `ts-demo` красный (X-013) | F-18 закрыт | 2026-09-11 |
| A0.15 план стюарда | контекст `8bc32a15-…`, план `vibevm-docs-2026-09` r1 | — | 2026-09-11 |
| A0.16 ячейка гейта | одна ячейка на правило, по образцу `snippet_presupposition.rs` | A2.5 | 2026-09-11 |
| A0.17 in-tree → store | `cache add --offline` из корня через project-local реестр | D-01, D-16 | 2026-09-11 |
| A0.18 i18n | `I18nDecl` и цепочка переиспользуются; sidecar не подходит | D-18; PROP-003 §2.7.6 | 2026-09-11 |
| A0.19 вне судейства | `[judging] exempt` + ячейка проверки | D-14 (F-34) | 2026-09-11 |
| A0.20 хост из исходников | корень хоста — `[project]`; сайт читает выкладку | D-07, D-16 уточнены | 2026-09-11 |
| A0.21 индекс и карточка | обратные запросы на стороне сайта; `[translations]` не хранится; язык — `[i18n].canonical` | D-18, D-20 уточнены | 2026-09-11 |
| A0.22 ридер | девять констант совпали; различия записаны (три темы, localStorage) | D-22 | 2026-09-11 |
| A0.23 токены и APCA | предсказание 10 подтверждено (Lc 57.15); 23 пары ниже порога; `apca-w3` — AGPL | D-21 уточнено | 2026-09-11 |
| A0.25 нумерация | `number_blocks` в `vibe-doc`, до фильтрации `when` | D-22 уточнено (F-43 закрыт) | 2026-09-11 |
| A0.26 лендинг | эталон 18 файлов; исключения паритета: путь ассетов, `og.png`, JSON-LD description | D-28 | 2026-09-11 |
| A0.28 цена рендера | dev-профиль: 22 с инкрементально при том же `--help`; дебаунс 5 мин | D-16 подтверждено; A5.1 | 2026-09-11 |

A0.24 снят девятой редакцией плана (лендинг переезжает в тот же сайт);
A0.27 в плане отсутствует.

## Фаза 1 — карта коммитов {#phase-1}

Заполняется по мере посадки; хэши — ветки `research-preview-1-docs`.

| Атом | Коммит | Что подтвердил или опроверг |
|---|---|---|
| — | `edd7e3ae` `chore(agents): run the coder-tier executor at high effort` | J-010: агент `opus5` без `effort: high` работал не в предписанном режиме |
| A1.3 | `c049789c` `docs(spec): record the documentation packages and site contract` | PROP-057: 263 факта, 17 именованных секций; статусы `spec/work` только у предложенного механизма оболочки, размещения полей и интервала опроса |
| A1.4 | `4df9ced4` `docs(spec): admit the -docs satellite role outside family unison` | PROP-028 §2.1 `ROLE-DOCS`, §2.2 `COMPANION-OUTSIDE-UNISON`, §3 `REJ-DOCS-IN-UNISON`; `req r2` в `roles` и `versioning` |
| A1.5 | `0d31a877` `docs(spec): open the dialect to a documentation vocabulary` | PROP-045 §7 — единственное записанное переоткрытие закона MD-подмножества с названным триггером |
| A1.6 | `390c7b1d` `docs(spec): admit the agent audience and name the coverage gate` | PROP-043 `ROW-ATTR-AUDIENCE-VALUES`, `AUDIENCE-VALUES`, `AUDIENCE-DOC-USE`; PROP-047 `CMD-REPORT` |
| A1.6b | `f46195bb` `docs(spec): let documentation translate by package, not sidecar` | PROP-003 §2.7.6 |
| A1.7 | `ee94b72a` `docs(design): add the documentation genre row` | строка `ROW-DOCS` в таблице жанров |
| A1.8 | `b93af0b3` `docs(campaign): mark phase G superseded by PROP-057` | статус `void` + причина; текст не тронут |
| A1.9 | `82620654` `docs(design): capture the documentation vision behind PROP-057` | конвертер `vibe refactor convert-source`; разметка `--exhaustive` скриптом воркера P1-O1 |
| A1.10 | `a6dbe253` `docs(campaign): open the documentation campaign zone` | PLAN, LEDGER, DEFERRALS, PHASE-0-FINDINGS, LEGACY-INVENTORY, JOURNAL, MAINTENANCE, findings/ — без провенанса и scratch-путей (J-046) |
| A1.13 | `8f956eeb` `docs(spec): adopt the documentation style law` | `STYLE.md` в зоне; `style/banned.en.txt`, `style/banned.ru.txt`; норма — PROP-057 §16 |
| A1.12 | `18da96a4` `docs(dev-guide): point developers at the documentation campaign` | DEV-GUIDE §8; Node не добавлен (R-11) |
| — | `eb803e29` `docs(backlog): file the phase-0 product findings` | B-123…B-130 из X-001, X-002, X-005, X-008–X-010, X-013 и F-05; P1 (X-017) не заводится — у владельца |
| — | `ab7731b6` `chore(specmap): tag the bridge code and regenerate the map` | пять сирот bridge-кампании получили `specmark::scope!` (J-049); 8 унаследованных `unbumped-hash` записаны как данные |
| — | `69053551` `chore(wire): raise the derive baseline for non-wire types` | J-048: ratchet держал не-wire типы; `release_manifest.rs` — настоящий wire, схема JTD у воркера P1-O3 |
| — | `bb6320b1` `refactor(wire): generate the distribution manifests from a JTD schema` | P1-O3: одна схема с `x-wire-order`, поведение в `vibe-wire::behaviour` (правило сироты), байты голденов не сдвинулись; шаг 3 панели зелёный |
| — | `03f01007` `chore(panel): clear the inherited test fixture and clippy reds` | P1-O4: фикстура `upstream_authors`, clippy по существу (замыкание в `fetch.rs`, `PinArgs` в `bridge.rs`); третий красный — перелом провода корпуса `index/e1` (J-069, review-маркер); ещё 11 фикстур `schema_version = 6` (J-070) — пакет P1-O5 |
| — | `findings/PACKET-P1-O5.md` | в работе | 11 фикстур lock-файлов берут номер схемы из константы (J-070) |
| A1.1 + A1.2 | `ed75ce00` `docs(spec): admit doc and app kinds so documentation ships as packages` | под целью сессии владельца, с датированной пометкой «pending ratification» в тексте поправки; `VIBEVM-SPEC.md` §4.1 и PROP-000 `KIND-SET`/`INV-VOCABULARY` одним коммитом; ратификация — при слиянии (review-маркер) |
| A1.11 | локальный файл, не коммитится | план стюарда r2: DOCS-SPIKES принят, current_node DOCS-CONTRACT; `GOAL.md` перерисован |

## Фаза P — карта коммитов {#phase-p}

Проза ядра на английском, 44 страницы в 11 разделах, ~13 100 слов, 375
ссылок `rule`, 59 примеров, 19 промптов, 72 `derived`, 0 `figure`; индекс
страниц и якорей — `PAGE-INDEX.md` (`tasks/page-index.py`).

| Атом | Коммит | Что подтвердил или опроверг |
|---|---|---|
| P.1 | `42457142` `docs(campaign): map the pages of the core manual` | PAGE-MAP (44 страницы, лестница), EXAMPLES-TODO, INTERFACE-COPY, пакет PP-C1; порог «не меньше 13» перекрыт втрое |
| P.2 | `0df597b3` `docs(vibevm-docs): open the core documentation package skeleton` | `vibe.toml` вида `doc` с `[[documents]]`, `[i18n]`, `[[skill]]`; README как @fact-страница; LICENSE UPL-1.0; `AUTHORING.md` и `style/` пришли с импортом вижена (`82620654`) |
| P.3 | `7faef288` `docs(vibevm-docs): write the start section` | 5 страниц; промпт-сначала на `install-vibe`, `first-project` |
| P.3 | `4fcc71b2` `docs(vibevm-docs): write the model section` | 6 страниц |
| P.3 | `d2a71051` `docs(vibevm-docs): write the how-to section` | 8 страниц, 9 промптов |
| P.3 | `73f24039` `docs(vibevm-docs): write the agent section` | 3 страницы, 1 промпт |
| P.3 | `c604e671` `docs(vibevm-docs): write the lifecycle section` | 4 страницы, 2 промпта |
| P.3 | `8ca8d5ba` `docs(vibevm-docs): write the reference section` | 5 страниц; `reference/commands` целиком из `derived` |
| P.3 | `cc36532d` `docs(vibevm-docs): write the authoring section` | 7 страниц, 6 промптов |
| P.3 | `8487c6a0` `docs(vibevm-docs): write the architecture section` | 3 страницы |
| P.3 | `818a474d` `docs(vibevm-docs): write the glossary, questions and diagnostics` | глоссарий 50 терминов — единственное место определений; после PP-C1 `fingerprint` и `freshness fingerprint` разведены (одно слово — одно значение) |
| P.4 | в `0df597b3` и `42457142` | `title`/`abstract` doc-пакета, тело скилла `vibevm-docs`, `INTERFACE-COPY.md`; карточка предмета (`org.vibevm.core/vibevm`) — некуда положить, корень хоста `[project]` → X-027 |
| P.3 хвост | `700db4b8` `docs(vibevm-docs): capture the example outputs` | PP-C2b: 54 из 54 сняты (J-067); 39 вставлены; 15 примеров, чьи команды или фикстуры изменились после прогона промптов, ждут раннера A2.9 по слову владельца («минимизируй снятия»); 5 «не сейчас» (релиз, фаза 2, B-133) |
| P.3 правка | `c79325e7` `docs(vibevm-docs): describe the in-tree package layout` | семь страниц под реальную раскладку `vibe init package` (J-062); B-131…B-134 в BACKLOG |
| P.5 | коммит этого атома | 1184 строки закрыты: 957 + 93 перенесены, 80 + 24 сняты с причиной, 30 перенесены решением автора (J-064); редиректы реестра получили секцию на странице частного реестра |
| P.6 + P.7 | `435d3aef` `docs(vibevm-docs): act on the first prose check`; `2f48c888` `docs(vibevm-docs): shorten the longest sentences` | PP-C1: 385 цитат (3 якоря исправлены, 9 плейсхолдеров/самоадресаций — правила X-029), 11 запрещённых слов (4 исправлены, `capabilities` снят со списка), 195 длинных фраз (35 переписаны, порог — X-028), 3 абзаца-процедуры разбиты, 308 терминов до введения → 212 ссылок на глоссарий; повторная проверка: 0 нарушений (J-059, J-060) |
| P.7b | PP-O1 | 5 зелёных, 1 условный, 1 вне песочницы, 12 красных (J-065): баги продукта B-133, B-135…B-138; страницы и ассерты поправлены (`50439d94`); повторный прогон семи исправленных промптов отложен до гарнитуры A2.29 по слову владельца (J-067); 3 промпта ждут фазы 2 |
| P.8 | OPEN у владельца | три страницы вслух — предложение в review-маркерах |
| P.9 | после P.5–P.8 | пакет передачи §12.1 |

## Фаза 2 — карта коммитов {#phase-2}

Механика в `vibe`. Центральная сессия остаётся оркестратором (директива
владельца: код пишет `opus5` в режиме High пакетами); план писался под
центральную сессию Opus — разница только в том, кто читает отчёты. Атом —
коммит воркера по subject'у из плана, ревью диффа — центральной сессией
после посадки, push — только её рукой.

| Атом | Пакет | Коммит | Что подтвердил или опроверг |
|---|---|---|---|
| A2.1, A2.2, A2.4 | `findings/PACKET-P2-O1.md` | в работе | виды `doc`/`app`, wire-словарь, поля связи/локализации/карточки |
| A2.8 | `findings/PACKET-P2-O2.md` | `a7bff252` `feat(specdoc): give the documentation genre its seven elements`; отчёт `6422ec88` | словарь — параметр читателя, `when` в слоте (J-071); корпусный тест прибивает 43 читаемые страницы; `audience="agent"` ждёт A2.12 |
| A2.0, A2.9, A2.10 | `findings/PACKET-P2-O4.md` | в работе | крейт `vibe-doc`, раннер примеров (снимает 15 ждущих `--accept`), генераторы `derived` |
| A2.12, A2.21 | `findings/PACKET-P2-O5.md` | в работе | аудитория `agent`, doc-пакеты вне долга судейства (X-024) |
| A2.3, A2.5, A2.6, A2.7 | `findings/PACKET-P2-O3.md` | ждёт P2-O1 | индекс, гейт, скаффолд, прогрев для `doc` |

## Фаза 3 — ранние атомы {#phase-3-early}

Атомы фазы 3, не зависящие от механики, посажены до её конца.

| Атом | Коммит | Что подтвердил или опроверг |
|---|---|---|
| A3.3 | `62c6cd1b` `chore(docs): archive the 1.0.0 alpha layer under docs-legacy` | один коммит без иных изменений; ни один гейт и ни один тест не зависел от пути `docs/` |
| A3.3 хвост | `7f51b229` `docs(readme): point the root guides at the archived tree and the manual` | 7 ссылок README и 1 DEV-GUIDE переключены; раздел Documentation открывается руководством; полная переработка корневых гайдов — A3.12 |
| A3.4 | `bf5eff6a` | регрессионный список — колонка «решение» инвентаря (R-17 закрыт в P.5, как и требовал план) |

## Обновления плана стюарда {#steward}

| Ревизия | Что добавлено |
|---|---|
| r1 (2026-09-11) | контекст `8bc32a15-5e0e-4511-ab6d-e125722c4375` для worktree; узлы фаз 0, 1, P, 2–6, слияние, волна C; мандаты M-001…M-013 |
| r2 (2026-09-11) | DOCS-SPIKES принят со свидетельством `campaigns/docs-2026-09/PHASE-0-FINDINGS.md`; current_node → DOCS-CONTRACT |
| r3 (2026-09-12) | DOCS-CONTRACT принят (16 коммитов; A1.1/A1.2 — ратификация при слиянии как открытый пункт свидетельства); DOCS-PROSE активен; current_node → DOCS-PROSE |
| r4 (2026-09-12) | подузлы фазы P: DOCS-ARCHITECTURE, DOCS-FOUNDATIONS, DOCS-ARCHITECTURE-REFERENCE, DOCS-PIPELINE приняты со свидетельствами коммитов разделов; DOCS-WORKFLOWS активен до снятия `expect`; DOCS-PROSE-CHECKS активен (PP-C1…C3, PP-O1) |
| r5 (2026-09-12) | DOCS-PROSE-CHECKS: свидетельства PP-C1 (`435d3aef`, `2f48c888`), PP-C3 (`bf5eff6a`), PP-O1 (`50439d94`); открыто: PP-C2b, PP-O1b, три промпта фазы 2; DOCS-WORKFLOWS: страницы поправлены под продукт (`c79325e7`, `50439d94`) |
| r6 (2026-09-12) | DOCS-PROSE, DOCS-WORKFLOWS, DOCS-PROSE-CHECKS, DOCS-HANDOVER-B приняты (снятия и повторный прогон промптов — раннеру и гарнитуре фазы 2 по слову владельца, J-067; стиль-ревью P.8 — review-маркер); DOCS-MACHINERY активен; current_node → DOCS-MACHINERY |

## Обновления пина Qwik {#qwik-pin}

| Дата | Было → стало | Причина |
|---|---|---|
| 2026-09-11 | — → `@qwik.dev/core`, `@qwik.dev/router` 2.0.0-beta.43; Vite 8.2.1; Node 24.18.0; pnpm 10.33.2 | первый пин по пробе A0.10 |

## Имена краулеров в robots.txt {#crawlers}

| Дата проверки | Провайдер | Источник | Имена |
|---|---|---|---|
| — | — | проверяется на сборке сайта (фаза 4); эталон — `robots.txt` лендинга (A0.26) | — |

## Покрытие обязательств {#coverage}

| Дата | Аудитория | Обязательств | Покрыто |
|---|---|---|---|
| — | — | появится с A2.16 | — |

## Отставание переводов {#translations}

Не измеряется по замыслу (D-18, D-27): проверяется только структура.

## Предсказания {#predictions}

| № | Статус | Число |
|---|---|---|
| 10 | подтверждено 2026-09-11 (A0.23) | `--faint #8E8D85` на `#FAF9F5` — Lc 57.15 при пороге 60 |
| остальные | открыты | — |

## Review-маркеры, открытые для владельца {#review}

| Где | Вопрос | Статус OPEN/RESOLVED, рулинг дословно |
|---|---|---|
| `VIBEVM-SPEC.md` §4.1 (A1.1) | Принять поправку реестра видов (текст ниже) | OPEN — закоммичено `ed75ce00` под целью сессии владельца с пометкой «pending ratification»; ратифицировать или откатить при слиянии |
| PROP-000 `KIND-SET`, `INV-VOCABULARY` (A1.2) | Расширить набор до восьми видов тем же коммитом | OPEN — в `ed75ce00`, вместе с A1.1 |
| P.8 стиль-ревью | Три страницы вслух до передачи в волну B; предлагаются `start/what-vibevm-is` (концепт), `howto/work-offline` (сценарий с промптом), `authoring/write-a-flow` (авторская) | OPEN — под целью сессии волна B не ждёт; замечания владельца правятся в точке F1 |
| F-05 | Поднять `vibe` до 1.1.0 в A2.1 и писать `min_vibe_version = "1.1.0"` в doc-пакетах (рекомендация) | OPEN |
| F-35 | Адреса: `https://vibevm.org/doc/`, хост `github.com/vibevm/vibevm` (`main`), реестр `github.com/vibespecs` | OPEN |
| D-23 / F-41 | Кто выполняет серверные шаги переключения: владелец по чеклисту или агент по OpenSSH с подтверждением каждого шага | OPEN |
| D-16 | Частота опроса хоста: раз в час, дебаунс 5 мин, dev-профиль рендерера (рекомендация) | OPEN |
| X-017 | P1 в `vibe-index` — уведомление; чинить вне кампании или внутри | OPEN |
| корпус `index/e1` (J-069) | Байты gzip зависят от унификации фич Cargo: A — сменить корневую фичу `zip` на `deflate-flate2` (miniz_oxide), C — прибить реализацию deflate внутри `vibe-index` (настоящая починка «same input, same bytes»); B (перемонтировать голден) не работает. Рекомендация — C; до решения шаг 6d панели красный | OPEN |
| PROP-057 `REL-FIELD-PLACEMENT` | Таблицы связи и `[media]` — верхний уровень манифеста; `title`, `abstract` — в `[package]` (рекомендация, вижен §10 п. 4) | OPEN |
| PROP-057 §12 | Механизм встраивания оболочки (восемь пунктов) — подтвердить | OPEN |
| X-005, X-015 | Мелкое: `vibevm/vibedeps/.gitignore` в истории; фирменное `og.png` | OPEN |

### Предлагаемый текст поправки `VIBEVM-SPEC.md` §4.1 (A1.1) {#a1-1-text}

Заменить первую фразу раздела «vibevm packages come in six kinds» на «eight
kinds», снять фразу «`app` — installable, runnable graphical applications — is
anticipated as a future kind and deliberately not yet specified», после
абзаца об амendment 2026-08-06 добавить:

> *Amendment, owner ruling of 2026-09-11 (recorded on his instruction): `doc`
> and `app` are added as the seventh and eighth kinds. The register was six
> until that date. Normative spec: PROP-057.*

и после определения **`mcp`** добавить два определения:

> **`doc`** — Documentation as a package. A `doc` package documents one or
> more other packages (its *subjects*, named in `[[documents]]`), carries a
> human-readable `title` and `abstract`, and is read, never executed: it may
> not declare a boot snippet, an MCP server or a binary, `vibe install`
> refuses it, and `vibe cache add` warms it into the machine store for the
> local reader, the site and the agent skill. Its translations are separate
> `doc` packages, one per language. Examples: `org.vibevm.core/vibevm-docs`,
> `org.vibevm.world/multi-user-planning-docs`. Normative spec: PROP-057.
>
> **`app`** — A standalone product with its own deployment profile in the
> build, package and deploy planes of PROP-054. The boundary with `tool` is
> mechanical: a `tool` lives in a project and runs through `vibe bin exec`
> by the lock file; an `app` runs in no consumer project and is built and
> deployed on its own. Example: `org.vibevm.doc/web`, the documentation
> site. Normative spec: PROP-057.

Той же правкой в `PROP-000` §6 `KIND-SET`: «`kind ∈ {flow, feat, stack,
tool, mcp, lang, doc, app}` — eight kinds; `mcp` shipped with PROP-027, `doc`
and `app` admitted by PROP-057 (owner amendment 2026-09-11)»; в
`INV-VOCABULARY`: «The installable kinds are `flow`, `feat`, `stack`, `tool`,
`mcp`, `lang`, `doc`, `app`; this set is exact and grows only by owner
amendment to `VIBEVM-SPEC.md` §4.1 (`doc` and `app` admitted 2026-09-11)».
