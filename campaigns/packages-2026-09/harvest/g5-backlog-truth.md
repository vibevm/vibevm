# g5-backlog-truth — измерение 39 «живых» строк бэклога по дереву

_Замер задачи E-G9-BACKLOG-TRUTH, 2026-08-05. Рабочий каталог — корень worktree
`wt/E-G9-BACKLOG-TRUTH`. Гит не запускался. Это измерение по дереву, не решение;
вердикт по каждой строке — `PENDING` (пишет босс). Поиск вёл только по
авторскому дереву: `crates/**`, `xtask/**`, корневые конфиги,
`packages/org.vibevm.ai-native/**` (живые слоты: core-ai-native v0.8.0,
rust-ai-native-lang v0.7.0, typescript-ai-native-lang v0.6.0, go-ai-native-lang
v0.1.0), `packages/org.vibevm.world/**`. В `vibedeps/`, `.vibe/`, `refs/`,
`legacy-spec/`, `.wt/` не заглядывал (копии/история)._

## Итог по четырём статусам

| status | count | ids |
|---|---|---|
| **BUILT** | 10 | B-025, B-026, B-029, B-034, B-035, B-036, B-038, B-039, B-041, B-043 |
| **PARTLY** | 4 | B-001, B-018, B-037, B-047 |
| **NOT-BUILT** | 24 | B-002, B-004, B-008, B-014, B-015, B-016, B-017, B-019, B-020, B-021, B-024, B-032, B-044, B-045, B-046, B-048, B-050, B-051, B-052, B-053, B-059, B-060, B-061, B-062 |
| **NOT-CHECKABLE** | 1 | B-007 |

**Главное.** Десять «живых» строк оказались построенными, и у восьми из них
диспозиция в `BACKLOG.md` при этом стоит `planned`/`open` — то самый класс
«протухшая `disposition` на построенном», из-за которого затевался замер:
**B-029, B-034, B-036, B-038, B-039** держат `planned` на полностью
смонтированных правилах/инвариантах движка; **B-043** держит `open` при
исправленном генераторе реестра; **B-035** держит `planned` при поднятой в
манифест норме паритета; **B-041** держит `planned`, хотя тело самой записи и
дерево говорят «одобрено и интегрировано». B-025 и B-026 (босс-верификация)
перепроверены независимо и **сошлись без расхождений**.

---

## B-001 — link-таблицы PROP-035 §10, непостроенная половина
- **asks** — персистентный on-disk формат link-таблицы и структурный consumer (vtable для §13).
- **status** — PARTLY
- **evidence** — `crates/vibe-spec/src/link_table.rs:43` `build_link_table` (граф, cycle-safe) + `:93` `render` (детерминированный TSV-дамп) + тесты `:141`…`:217`. Документ модуля (`:12-13`) прямо: «This is the core (the graph + a deterministic dump). A persisted on-disk format (a `specmap.json` sibling) and the structural consumer are follow-ups»; `:92` «The on-disk index format later».
- **note** — построено: граф + дамп. НЕ построено: персистентный on-disk формат и structural/JIT consumer (явно «follow-ups»).
- **verdict** — PENDING

## B-002 — budget-строка байндит сгенерированные артефакты наравне с авторскими
- **asks** — уточнить строку `ROW-BUDGET-BOOT-FILE`: сгенерированный boot-артефакт не несёт токен-бюджета.
- **status** — NOT-BUILT
- **evidence** — `packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/authoring-rules.xml:135`: `##ROW-BUDGET-BOOT-FILE Boot file (always loaded) @impl/done | ≤ 500 tokens @impl/done | …`. Строка даёт ОДИН бюджет на «Boot file», без различения авторский/сгенерированный — правка не внесена.
- **note** — это release-event правка спеки (диспозиция `open`, «ждёт release-batch»); измеримый факт — строка не различает.
- **verdict** — PENDING

## B-004 — факт внутри fenced-блока не имеет якоря
- **asks** — факты внутри fenced code-блоков становились адресуемыми/проверяемыми.
- **status** — NOT-BUILT
- **evidence** — `crates/vibe-spec/src/doctree.rs:391` `pub(crate) fn fence_mask(lines: &[String]) -> Vec<bool>` — маска_fence живёт и применяется повсюду: `directives.rs:26,93`, `doctree.rs:82`, `qualify.rs:39,82`, `pipeline.rs:247,321`. Fenced-директивы по-прежнему игнорируются намеренно — якорной модели для fenced-содержимого нет.
- **verdict** — PENDING

## B-007 — должны ли спеки нести ADR, и в какой форме
- **asks** — решение о жанре ADR (раздел в PROP/FEAT, отдельный `spec/decisions/`, или четырёхполевый блок).
- **status** — NOT-CHECKABLE
- **evidence** — `glob spec/decisions/**/*.md` → No files found. Жанр ADR не принят: ни каталога `spec/decisions/`, ни принятой формы.
- **note** — строка по своей диспозиции (`open`, «filed at owner request as a question to answer rather than work to schedule») просит решение владельца о жанре, а не код. Измерить можно только отсутствие принятой формы — она отсутствует. Сама работа — это рулинг, не постройка.
- **verdict** — PENDING

## B-008 — один крейт воркспейса не объявляет лицензию
- **asks** — `crates/vibe-index/Cargo.toml` получает `license-file.workspace = true`.
- **status** — NOT-BUILT
- **evidence** — `crates/vibe-index/Cargo.toml` (строки 1-51): полного package-метаданные (`authors`, `description`, `homepage`, …), но НИ ОДНОГО ключа `license`/`license-file`; строка 7 — `publish = false`. У всех остальных крейтов — `license-file.workspace = true`.
- **verdict** — PENDING

## B-014 — коммитнутый host-`specmap.json` дрейфует без гейта свежести
- **asks** — гейт свежести для хостового индекса (регенерация, byte-compare или staleness-предупреждение).
- **status** — NOT-BUILT
- **evidence** — `tools/self-check.sh:415-442`: единственные specmap-шаги панели — пакетные self-trace'ы `rust-ai-native-specmap --gate (… pkg self-trace)` по слотам пакетов. Хостового `specmap.json` они не касаются. `grep` по `crates/`+`tools/` для `host specmap.*(check|regener|stale|byte)` → ни одного шага регенерации/сверки хостового индекса (матчи — только «built FRESH in memory» в `vibe-trace` и пакетные self-trace'ы).
- **verdict** — PENDING

## B-015 — программа подписей runtime-канала
- **asks** — криптографическая подпись содержимого пакетов (схема подписи + верификация).
- **status** — NOT-BUILT
- **evidence** — `grep -i "minisign|sigstore|verify_signature|signed.*tag|cosign" -g '!**/{vibedeps,.vibe,refs,legacy-spec}/**'` → 23 матча, и ВСЕ — спеки/документы/json (`PROP-002`, `PROP-019`, `PROP-014`, `LEDGER-INTENT`, `ROADMAP`, `BACKLOG`, harvest, evidence, `specmap.json`). Кода подписи/верификации нет; единственная crypto-зависимость — `sha2` для контент-хэшей.
- **note** — намеренно запаркована владельцем («не строить до специального уведомления»; условие переоткрытия — только уведомление владельца). Дерево подтверждает: подписи нет.
- **verdict** — PENDING

## B-016 — карта в составе пакета + получение фрагментов по отпечатку
- **asks** — (1) пакет возит готовую карту + читатель на стороне потребителя; (2) хранилище хэш→исходный текст.
- **status** — NOT-BUILT
- **evidence** — `…/core-ai-native/v0.8.0/spec/mechanisms/PROP-014-specmap-bidirectional-traceability.md:58` `##DISTRIBUTION-RIDES-THE-EXISTING-REGISTRY` «Specified, not built (→ B-016): no package ships an index … and there is no fetch-by-content-hash path — `content_hash` hashes, it does not retrieve.» `@impl/plan`. Ни один `vibe.toml` не перечисляет `specmap.json` в payload; хранилища хэш→текст нет.
- **verdict** — PENDING

## B-017 — профили приватности для закрытых проектов
- **asks** — ключ `[metamodel] profile` (`open`/`contract`/`none`) в манифесте/схеме/парсере.
- **status** — NOT-BUILT
- **evidence** — `…/PROP-014-…:241` `##RUNTIME-PROFILES` «Specified, not built (→ B-017): `[metamodel]` is in no manifest, no schema and no parser; the three profile values have no representation.» `@impl/plan`. `grep "\\bprofile\\b"` по движку даёт только `FileMetrics`/ATLAS-профили из чужой дисциплины — `[metamodel]` отсутствует.
- **verdict** — PENDING

## B-018 — инструменты для агентов (MCP), широкий вариант
- **asks** — (1) перенос «объясни» в агентский интерфейс vibe; (2) поиск по карте; (3) фрагменты по отпечатку; (4) ответы про установленные пакеты.
- **status** — PARTLY
- **evidence** — часть 1 ПОСТРОЕНА: `crates/vibe-mcp/src/tools.rs:50` `ExplainMcpTool` (`:487-529`, «the MCP face of `vibe explain`… build the specmap fresh in memory») поверх общего крейта `crates/vibe-trace/src/lib.rs` (`vibe_trace::explain`, «build the index FRESH in memory») + CLI `vibe explain`.
- **note** — построено: часть 1 (explain). НЕ построено: часть 2 (поиск по карте — отложен владельцем 2026-08-04), часть 3 (фрагменты по отпечатку — это B-016 половина 2), часть 4 (объяснение чужих/установленных пакетов — вторая некоммитимая карта-резолвер; текущий explain строит карту только дерева THIS проекта).
- **verdict** — PENDING

## B-019 — отпечатки кода + узлы «команда» и «вариант ошибки» в карте
- **asks** — (а) `content_hash` на `CodeItem`; (б) узел «команда»; (в) узел «вариант ошибки».
- **status** — NOT-BUILT
- **evidence** — `…/PROP-014-…:198` `##EDGE-MODEL-NODES` «Specified, not built (→ B-019): `CodeItem` carries no content hash … and there are no derived `Command` or `ErrorVariant` node views. (`ErrorVariant` exists as a conform **fact** … which is a different graph.)» `@impl/plan`. (`content_hash` в `specmap.jtd.json:41` — на `SpecUnit`, не на `CodeItem`.)
- **verdict** — PENDING

## B-020 — объяснения человеческой прозой через внешние LLM
- **asks** — лайтовый клиент к внешним нелокальным LLM для рендера explain прозой.
- **status** — NOT-BUILT
- **evidence** — `crates/vibe-llm/src/lib.rs:1-9`: «**STATUS: M0 stub.** Concrete providers (Anthropic, OpenAI, OpenRouter, Ollama) land in the v1.5 LLM milestone … this crate is a deliberate placeholder». Реального LLM-клиента нет; `tools.rs:8` упоминает «once `vibe-llm` is real».
- **verdict** — PENDING

## B-021 — пороговые предупреждения: перегруженные связи и длинные секции
- **asks** — (1) multiplicity-lint (число рёбер на элемент кода + порог); (2) предупреждение о длинных секциях.
- **status** — NOT-BUILT
- **evidence** — `…/PROP-014-…:190` `##RULE-MULTIPLICITY-LINT` «Specified, not built (→ B-021): no checker in any layer counts edges per item; `vibe check`'s checks do not include a multiplicity lint.» `@impl/plan`. `grep "multiplicity|long.section|section.length"` по `core-ai-native-specmap/src` → ни одного правила.
- **verdict** — PENDING

## B-024 — вывести lifecycle-статусы specmap из progress-маркеров
- **asks** — машина трассировки читает хостовые `@stage/state`-маркеры вместо собственной параллельной kind-line системы.
- **status** — NOT-BUILT
- **evidence** — `…/core-ai-native-specmap/src/mdspec.rs:40` `fn parse_kind_line` по-прежнему парсит СОБСТВЕННЫЕ статусы `planned`/`disputed` (`:88` «expected `planned` or `disputed(#anchor)`»), `retired` — tombstone; `@impl/plan`. `grep "derive.*status|from.*progress|stage.*state|@stage"` по specmap-движку → ничего (вывода из progress-маркеров нет).
- **note** — направление выбрано владельцем (сводить к progress-словарю), механика вывода не построена; две параллельные системы сосуществуют.
- **verdict** — PENDING

## B-025 — находки гейта: помечать признанные отступления, а не гасить
- **asks** — статус-поле у находки; правило штампует статус вместо фильтра по `in_deviation`; baseline/ratchet учитывает «признанные» отдельно; SARIF несёт статус.
- **status** — BUILT
- **evidence** — `…/core-ai-native-conform/src/finding.rs:85-95` `enum FindingStatus { Live, DeviationAcknowledged { reason } }` + поле `status` на `Finding` (`:50`); комментарий `// B-025 (mark, don't suppress)` в `finding.rs:39` и `sarif.rs:52`; SARIF несёт статус и `suppressions` (`sarif.rs:52-71`, `status_name` `:101`); `baseline.rs:79-80` `diff` фильтрует acknowledged из `new`, `baseline.rs:98-101` `freezeable` исключает их; тест `baseline.rs:161` `acknowledged_findings_are_gate_inert` + `:190-191` подтверждает, что правило штампует `DeviationAcknowledged`.
- **note** — независимая перепроверка СОШЛАСЬ с босс-верификацией (без расхождений): enum, комментарий, SARIF, baseline — всё на месте.
- **verdict** — PENDING

## B-026 — ингест SARIF: диагнозы чужих линтеров становятся фактами гейта
- **asks** — SARIF-парсер, маппинг диагноз→`Fact`, точка входа, словарь цитирования.
- **status** — BUILT
- **evidence** — `…/core-ai-native-conform/src/sarif.rs:166` `pub fn ingest(text: &str) -> Vec<Fact>` (точь-в-точь адрес босса); `Fact::LintDiagnosis` (`facts.rs:246`); `Fact::cites_lint(tool, rule_id, suppressed)` — словарь `check:{tool,id,status}` (`facts.rs:370`); `load_reports` — точка входа (`sarif.rs:264`); отдельный тест `tests/sarif_ingest.rs` существует + док-тест `sarif.rs:138-165`.
- **note** — независимая перепроверка СОШЛАСЬ с босс-верификацией (без расхождений): `ingest` на `sarif.rs:166`, тест `tests/sarif_ingest.rs` — ровно как у босса.
- **verdict** — PENDING

## B-029 — нейтральное/пер-языковое имя ключа гейта вместо растового
- **asks** — нейтральный/пер-языковый ключ единиц гейта (старый `gated_crates` — alias совместимости).
- **status** — BUILT
- **evidence** — `…/core-ai-native-conform/src/config.rs:9-17` «The v2 surface (B-029 + B-034, design `gate-parity-config.xml`)»: пер-языковые секции `[rust].gated`/`[go].gated`/`[typescript].gated` (RustConfig:206, GoConfig:274, TsConfig:339), единица = crate/package/cell; старые плоские ключи — громкие tombstone (`Option<Value>`, `config.rs:72-88`, `gated_crates:76`), `Config::load`+`tombstones::check` reject'ит их с подсказкой (`:430-437`). Ростеры читают `config.rust.gated` и т.д. (rust `lib.rs:69`).
- **note** — диспозиция `planned` (с текстом «исполняется этой стройкой»), но дерево показывает v2-поверхность построенной.
- **verdict** — PENDING

## B-032 — протокол гранулярности планирования (FEAT-файлы как единицы)
- **asks** — протокольный абзац «как выбрать медиум» + форма «план ссылается на FEAT-файлы».
- **status** — NOT-BUILT
- **evidence** — `grep "when-to-propose|FEAT.*file|granularity|which.*medium|compose.*FEAT"` по `packages/org.vibevm.world` → единственный матч — нерелевантное «per-change granularity» в `atomic-commits/ATOMIC-COMMITS-PROTOCOL.md:74`. В флоу `campaign-plans`/`spec-tree-layout` протокола выбора медиума нет.
- **note** — это новая норма на стыке двух пакетов (release event), не построена.
- **verdict** — PENDING

## B-034 — инвариант «каждая единица под гейтом или исключена» для Go и TS
- **asks** — пер-языковый инвариант (Go=package, TS=cell) + пер-языковые гейт-списки.
- **status** — BUILT
- **evidence** — инвариант вызывается в каждом фронтенде: rust `validate_against_tree` (`rust-ai-native-conform/src/lib.rs:130`), TS `validate_typescript_against_tree` (`typescript-…/lib.rs:146`), Go `validate_go_against_tree` (`go-…/lib.rs:162`), в `run_check`/`run_freeze`; тесты отказа: TS `lib.rs:244` `validate_refuses_an_unclassified_ts_cell`, Go `lib.rs:255` `validate_refuses_an_unclassified_go_package`. Пер-языковые `gated`/`[[<lang>.exempt]]` — в config (B-029).
- **note** — диспозиция `planned` (fork #2 решён 2026-08-04), по дереву — построено.
- **verdict** — PENDING

## B-035 — паритет-аудит стеков: TS/Go не слабее Rust или причина записана
- **asks** — систематическое сравнение + достроить слабины или записать причиной + поднять принцип в спеку.
- **status** — BUILT
- **evidence** — принцип поднят в манифест: `…/core-ai-native/v0.8.0/spec/00-MANIFESTO.md:97` `##PARITY-ACROSS-PROJECTIONS` (`@impl/done`) и `:103` `##PARITY-GAP-IS-NEVER-SILENT` (`@spec/done`); три гайда цитируют (`GUIDE-AI-NATIVE-GO.xml:311,313` и TS/RUST). Слабины закрыты: seam-error ×3 (`TsSeamErrorCitesReq`, `GoSeamErrorCitesReq`), `floor_disable` ×3 (B-049), conformance ×3, flag-sites ×3 (B-039).
- **note** — это аудит-принцип; остаток (Go-floor `./...`-residual, близнец B-048) записан причиной и маршрутом, не молчит — ровно то, чего требует `##PARITY-GAP-IS-NEVER-SILENT`. Диспозиция `planned` — норма по дереву поднята.
- **verdict** — PENDING

## B-036 — правило «инварианты не тонут в середине файла»
- **asks** — правило: комментарий-инвариант в средней трети длинного файла → предупреждение.
- **status** — BUILT
- **evidence** — `…/core-ai-native-conform/src/rules/position.rs:74` `struct InvariantCommentPosition` (id `invariant-comment-position`, `:88`), middle-third-логика (`:119-149`), маркеры `INVARIANT:/WARNING:/PANICS:/MUST:/NEVER:`, `SAFETY:` исключён; СМОНТИРОВАНО во всех трёх ростерах (rust `lib.rs:85`, TS `:73`, Go `:89`); root-ключи конфига `invariant_comment_markers`+`invariant_comment_min_file_lines` (`config.rs:118,123`, default 5 маркеров/120 строк). `Fact::InvariantComment` (`facts.rs:195`).
- **note** — диспозиция `planned`; по дереву правило построено и смонтировано.
- **verdict** — PENDING

## B-037 — слой кастомных REQ-цитирующих линтов (dylint / typescript-eslint)
- **asks** — Rust: dylint-библиотека линтов; TS: пакет правил `@typescript-eslint`.
- **status** — PARTLY
- **evidence** — TS-половина ПОСТРОЕНА: `…/typescript-ai-native-lang/v0.6.0/tools/eslint-plugin-ai-native/src/diagnostic-cites-req.ts` (+ test `…/test/diagnostic-cites-req.test.ts`, `createRule`/`@typescript-eslint`). Rust-половина НЕ построена: `grep "dylint|declare_lint|rustc_private"` по авторскому дереву → 0 в исходниках (только спеки/campaigns); `rust-toolchain.toml` пинит `stable`.
- **note** — построено: TS custom-lint (eslint-plugin-ai-native). НЕ построено: Rust dylint (это отдельная запись B-050).
- **verdict** — PENDING

## B-038 — pending-карточки обретают карточки и чекеры: R-060 и closed-vocabulary-naming
- **asks** — R-060 карточка+чекер; rule-closed-vocabulary-naming (R3-004) карточка+чекер.
- **status** — BUILT
- **evidence** — карточки существуют: `…/rust-ai-native-lang/v0.7.0/spec/cards/rule-declared-test-matrices.md` и `…/rule-closed-vocabulary-naming.xml`. Чекер R-060 `DeclaredTestMatrices` (`rules/matrices.rs:78`, id `declared-test-matrices`) смонтирован во всех трёх ростерах (rust `lib.rs:97`, TS `:77`, Go `:93`). Чекер R3-004 (вычисляемые имена, fork #1) `CellNameIsComputed` (`rules/naming.rs:86`) смонтирован в rust (`lib.rs:78`) и Go (`:85`).
- **note** — построены оба чекера, заявленные первыми. Оставшиеся три половины R3-004 (закрытый словарь / один референт / нет синонимов) — отдельная запись B-052. Диспозиция `planned` — чекеры по дереву построены.
- **verdict** — PENDING

## B-039 — смонтировать R-001 (FlagSites) на TS-гейт; обследовать Go
- **asks** — добавить TS-гейту конфиг-ветку, смонтировать FlagSites; то же для Go.
- **status** — BUILT
- **evidence** — TS: `[typescript].composition_root` (`config.rs:354`) + правило `rules::TsFlagSites` монтируется при `Some` (`typescript-…/lib.rs:67-69`); тесты `demo_config_mounts_flag_sites_and_validates_green` (`:292`) и `flag_sites_is_unmounted_without_a_composition_root` (`:327`). Go: `[go].registry_pkg` (`config.rs:296`) + `rules::GoFlagSites` при `Some` (`go-…/lib.rs:76-78`). `Fact::TsEnvRead` (`facts.rs:165`).
- **note** — диспозиция `planned`; и TS, и Go flag-sites по дереву смонтированы (обследовано и закрыто).
- **verdict** — PENDING

## B-041 — карта развития инструментария
- **asks** — design-карта развития инструментария (механизмы/состояния/порядок/развилки).
- **status** — BUILT
- **evidence** — `TOOLING-MAP.md` существует в корне (Glob → `TOOLING-MAP.md`). Тело записи `##B041-DRAFT`: «Черновик написан, одобрен и интегрирован (2026-08-02) … Владелец: 'мне нравится этот документ'», живёт рядом с бэклогом + раздел-указатель `#map` в шапке `BACKLOG.md`.
- **note** — диспозиция `planned`, но и тело записи, и дерево говорят «одобрено и интегрировано» — диспозиция отстаёт.
- **verdict** — PENDING

## B-043 — генератор реестра может выдать один id двум кластерам
- **asks** — уникальное сопоставление кластер→прежний id + громкий отказ при коллизии вместо тихого дубля.
- **status** — BUILT
- **evidence** — `campaigns/packages-2026-09/tasks/drift-registry.py:557-562` `carry_ids`: жадное ОДНОЗНАЧНОЕ сопоставление с пометкой занятости — `taken_p, taken_c = set(), set()`, `:559` `if j < 0.5 or pi in taken_p or ci in taken_c: continue`, `:561-562` `.add(pi)/.add(ci)`. Сценарий REPO (два кластера от разбиения наследуют один прежний id) в текущем коде невозможен: как только кластер A занял `pi` (prior id), кластер B его не получает. Минтинг идёт над `spent` (`build:587-591`, `next_free` пропускает занятые).
- **note** — это fix-шейп B-043 (вариант 1: «жадное по пересечению якорей с пометкой занятости прежнего id»). Буквального «громкого отказа при коллизии» нет — коллизия исключена дизайном (однозначное сопоставление), что удовлетворяет суть («рефьюз лучше двойника»: дублей нет). Диспозиция `open` и REPO-текст «дефект остался» — расходятся с деревом; стол диспозиция протухла, босс судит.
- **verdict** — PENDING

## B-044 — no-zombie тест: процесс-таблица подтверждает смерть ребёнка оракула
- **asks** — по тесту на стек: поднять оракул, убить обёртку, опросить процесс-таблицу с дедлайном.
- **status** — NOT-BUILT
- **evidence** — `grep -i "sysinfo|no.zombie|process.*table.*assert"` → единственная настоящая process-table-ассерция (sysinfo-проба с дедлайном) — у fractality: `packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-pod/tests/loopback.rs` (+ `supervise.rs`, `worker_env.rs`). TCG-оракулы трёх стеков (`rust/go/ts-ai-native-tcg-bridge/.../live_oracle.rs`, `oracle.rs`, `client.rs`) содержат kill-on-drop/`try_wait` МЕХАНИКУ, но не sysinfo-ассерцию смерти ребёнка.
- **note** — механика убийства построена, тест-ассерция «ребёнок действительно умер» — только у fractality и про чужого (pod), не про оракул.
- **verdict** — PENDING

## B-045 — qualified-naming: kind-валидация, short-имена, четыре мис-цитаты
- **asks** — (а) тип `KindMismatch`+сверка+exit-код 4; (б) short-имена для uninstall/update; (в) правка четырёх мис-цитат §2.4→§2.6.
- **status** — NOT-BUILT
- **evidence** — `crates/vibe-cli/src/exit_code.rs:24` `pub const TYPE_MISMATCH: u8 = 4;` — единственное упоминание, мёртв (больше нигде не используется); `KindMismatch` — только док-коммент `crates/vibe-core/src/package_ref.rs:437`, реального типа/варианта нет. (б) `uninstall.rs:153` и `update.rs:445` всё ещё зовут `require_group` (short отбивается). (в) `uninstall.rs:152`/`update.rs:444`/`redirect/mod.rs:125` всё ещё цитируют «PROP-008 §2.4».
- **note** — ни (а), ни (б), ни (в) не выполнены.
- **verdict** — PENDING

## B-046 — мультиязычная композиция (агент собирает несколько AI-Native языков)
- **asks** — агрегатор/discovery-слой поверх суверенных стеков (MCP+CLI или конвенция через `vibe mcp`).
- **status** — NOT-BUILT
- **evidence** — `grep -i "ai-native-workspace|multi.?lang.*(registry|aggregat)|aggregat.*stack|autodiscover.*lang"` → 17 матчей, все — `vibe.toml` (семья пакетов, PROP-028 package-families), specs, campaigns, TOOLING-MAP/BACKLOG (обсуждение). Крейта/инструмента-агрегатора `ai-native-workspace` нет; композиционного слоя autodiscovery над стеками нет.
- **verdict** — PENDING

## B-047 — норма поверхностей: логика в разделяемом крейте, CLI/MCP — тонкие поверхности
- **asks** — (1) аудит «где прибито гвоздями»; (2) доводка дыр; (3) поднять норму в спеку.
- **status** — PARTLY
- **evidence** — норма ПРИМЕНЕНА в первом потребителе: explain-способность в общем крейте `crates/vibe-trace` (`vibe_trace::explain`) с двумя тонкими поверхностями — CLI `vibe explain` и MCP `ExplainMcpTool` (`crates/vibe-mcp/src/tools.rs:487`,descr `:476`). НО: `grep "thin surface|shared (crate|library).*CLI|two surfaces|surface norm"` по `core-ai-native/v0.8.0/spec` → No matches found — норма НЕ поднята в спеку дисциплины; аудит-таблицы поверхностей нет.
- **note** — построено: норма соблюдена в новой explain-способности (побочный эффект B-018). НЕ построено: перекрёстный аудит поверхностей и подъём нормы owner-диффом в спеку (собственные delivers B-047).
- **verdict** — PENDING

## B-048 — TS-floor: prettier/eslint обходят fixtures пакета (двойник B-003)
- **asks** — заскоупить/отфильтровать prettier- и eslint-шаги от fixture-деревьев.
- **status** — NOT-BUILT
- **evidence** — `…/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-cli/src/floor.rs:79,82` `prettier --check .` и `:142,145` `eslint .` — оба ходят по `.` без скоупа на roots/без пост-фильтра фикстур. `is_disabled` (`:78,141`) — это `floor_disable`, не fixture-фильтр. tests-шаг заскоуплен (`:125-133`), prettier/eslint — нет; `.prettierignore` отсутствует.
- **verdict** — PENDING

## B-050 — типо-аварный вехикл кастомных линтов для Rust (dylint + nightly)
- **asks** — крейт dylint-библиотеки линтов + шаг флора `cargo dylint`.
- **status** — NOT-BUILT
- **evidence** — `grep "dylint|declare_lint|rustc_private"` по авторским исходникам → 0; `rust-toolchain.toml:2` пинит `channel = "stable"` и это единственный toolchain-файл. dylint-крейта нет, шага флора нет.
- **note** — диспозиция `planned` низким приоритетом (владелец: «добавить с низким приоритетом»).
- **verdict** — PENDING

## B-051 — у пилотного языка нет документа поверхности конформа
- **asks** — авторить `conform-frontend-rust.xml` по образцу Go/TS-близнецов.
- **status** — NOT-BUILT
- **evidence** — `glob **/conform-frontend-rust.xml` → No files found. Go/TS-документы существуют (`go-ai-native-lang/v0.1.0/spec/go/tools/conform-frontend-go.md`, `typescript-…/spec/typescript/tools/conform-frontend-typescript.md`), растового близнеца в живом rust-стеке v0.7.0 нет.
- **verdict** — PENDING

## B-052 — три непостроенные половины R3-004 (закрытый словарь, один референт, нет синонимов)
- **asks** — словарь структурных токенов ДАННЫМИ; реестр имён контрактной поверхности; детектор синонимов/затенения.
- **status** — NOT-BUILT
- **evidence** — `rules/naming.rs:86` содержит ТОЛЬКО композицию (`CellNameIsComputed`, первая половина R3-004, `@impl/done`). Закрытого словаря токенов данными/константой в `naming.rs` нет; реестра имён контрактной поверхности нет (только `in_src`/`is_lib_root`); детектора синонимов/затенения нет. Карточка `rule-closed-vocabulary-naming.xml` несёт раздел «specified, not built» для этих трёх.
- **note** — построена 1 из 4 половин R3-004 (композиция, она же B-038); эти три — отдельной записью честно помечены `@spec/done` «specified, not built».
- **verdict** — PENDING

## B-053 — текст причины отступления не доходит до находки у Rust
- **asks** — довести `reason` до трёх растовых фактов (`UnsafeUse`/`UnwrapUse`/`EnvRead`).
- **status** — NOT-BUILT
- **evidence** — `…/core-ai-native-conform/src/facts.rs:58-102`: `UnsafeUse`/`UnwrapUse`/`EnvRead` несут только `in_deviation: bool` (без `reason`), тогда как `TsUnsafe`/`GoUnsafe` несут `reason: Option<String>` (`:117,139`). Док `finding.rs:78-83` прямо: «The Rust facts carry only the boolean … Plumbing the reason text … into the three Rust fact variants is a measured, recorded leftover».
- **note** — сознательное «не сейчас» (диспозиция `planned`, P3), измерено при стройке B-025.
- **verdict** — PENDING

## B-059 — исключения конформа сопоставляются не с тем путём
- **asks** — сопоставлять исключение с repo-relative путём, который печатается в находке.
- **status** — NOT-BUILT
- **evidence** — `…/core-ai-native-conform/src/store.rs:271` `let rel_in_crate = path.strip_prefix(&crate_dir)...`; `:272` `rel_fwd = rel_in_crate…` (`src/lib.rs`); `:273` `if exclude.iter().any(|s| rel_fwd.contains(s.as_str()))` — исключение сопоставляется с путём ВНУТРИ крейта, тогда как в находку (`:277-281`) кладётся `file` — путь от корня репо (`crates/foo/src/lib.rs`). Два строковых пространства у одного ключа, ровно как в локаторе.
- **verdict** — PENDING

## B-060 — назначенная единица разметки не размечена и не читается: схемы JTD
- **asks** — JTD-схемы несут spec-адрес + JSON-сканер их читает.
- **status** — NOT-BUILT
- **evidence** — `grep "spec://"` по `**/schemas/*.jtd.json` → только description-проза (`specmap.jtd.json:17,23` объясняют формат URI), не теги-адреса; 0 из 7 схем несут spec-адрес. JSON-сканера нет: `…/core-ai-native-specmap/src/scanner.rs:23` `trait CodeScanner`, единственные impl — `RustScanner` (`:35`) и `CompositeScanner` (`:55`); `JsonScanner` отсутствует.
- **verdict** — PENDING

## B-061 — `implements`-ребро от объявления заявляет покрытие, которого нет (шов Watcher)
- **asks** — новый глагол ребра или правило («`implements` на трейте без прод-реализации — находка»).
- **status** — NOT-BUILT
- **evidence** — `crates/vibe-settings/src/events/mod.rs:444` `pub trait Watcher` несёт `implements = "spec://…/PROP-040#file-watch"` (`:442` на трейте, `:450` на методе). `impl Watcher for` совпадает ровно дважды, оба тестовые: `events/tests.rs:250` (`Noop`) и док-пример (`mod.rs:419`). Док `mod.rs:393`: «`impl Watcher for` matches exactly two sites in the whole tree» — прод-реализации нет; ребро всё ещё заставляет карту считать REQ покрытым.
- **note** — это решение о словаре карты (3 варианта в `##B061-FIX-SHAPE`), не принято; докблок приведён к измеренному, дефект стоит.
- **verdict** — PENDING

## B-062 — четыреста размеченных фактов вне корпуса: маркер стоит, вердикта нет
- **asks** — расширить корпус наблюдения на документы, несущие статусы (или снять статусы / третий режим).
- **status** — NOT-BUILT
- **evidence** — `progress.toml:83-91` `include = [ "spec/boot/[0-9]*.md", "spec/common/**/*.md", "spec/design/**/*.md", "spec/manual-tests/**/*.md", "spec/modules/**/*.md", "packages/org.vibevm.world/**/*.md", "packages/org.vibevm.ai-native/**/*.md" ]`. Ни один из четырёх названных документов не покрывается: `BACKLOG.md` и `TOOLING-MAP.md` (корень, вне `spec/`/`packages/`), `campaigns/…` (`campaigns` — структурное исключение движка `scope.rs`, и не в include), `PHASE-T-SPEC.md` (вне glob'ов). Маркеры на них стоят, вердиктов не получают.
- **note** — это развилка владельца о ЦЕНЕ расширения корпуса (решение, не правка), но измеримый факт — документы по-прежнему не наблюдаются, статусы не проверяются.
- **verdict** — PENDING
