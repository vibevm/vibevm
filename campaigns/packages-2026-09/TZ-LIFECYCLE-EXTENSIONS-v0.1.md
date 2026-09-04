# ТЗ: lifecycle-движок и машина расширений — входная точка исполнителя

_STATUS: В РАБОТЕ — R1/R2, R3.1–R3.4, R6.2a/b, R7.1–R7.4, R8.1 и
R8.2a завершены; остальные строки остаются открытыми.
Эта шапка — краткий указатель, не второй журнал. Точная гранулярная сверка,
физический recovery-аудит и действующий atom-level dependency plan:
`LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md`. Спека-закон:
`vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml` (при расхождении
спека главнее этого файла; о расхождении — доложить, не чинить молча).
Очередь уникальных report-derived findings после recovery-аудита:
`RETROSPECTIVE-SPEC-HARVEST-2026-08-27.md`._

Исполнитель: мультиагентная система владельца (модели уровня Opus5/gpt-5.6-sol/glm-5.3).
Этот файл — единственная входная точка. Он самодостаточен в части ПОРЯДКА и ГЕЙТОВ;
вся семантика — в спеке, и спеку читают ЦЕЛИКОМ до первого коммита.

## §0. Как читать и чем работать

1. Прочитать `CLAUDE.md` (правила репозитория) → boot-лейн по его инструкции →
   `vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml` **целиком** (особенно §14
   Reference и §15 Glossary — там точные схемы и словарь) → этот файл целиком.
2. Смежные законы, которые волны трогают по касательной, читать перед своей волной:
   PROP-009 (загрузка), PROP-011 (инкрементальный install), PROP-020 (хуки), PROP-022
   (режимы материализации), PROP-024 (код в пакетах), PROP-025 (бинари), PROP-035
   (компилятор спек), PROP-045 (derived-манифест), PROP-053 (clean). Все — в
   `vibevm/vibespecs/{common,modules/vibe-workspace}/`.
3. Карта живого кода — в самой спеке: `##COMPILER-CODE-MAP` (компилятор) и
   `##MATERIALISE-CODE-MAP` (материализация). CLI-диспетчер: `crates/vibe-cli/src/cli.rs`
   (`enum Command`), аргументы пакетных верб `crates/vibe-cli/src/cli/pkg.rs`
   (`InstallArgs`, `CleanArgs`, `CleanChain` — прецедент цепочки), цепочка clean→install:
   `crates/vibe-cli/src/commands/clean.rs`.

## §1. Непереговорные правила (нарушение = стоп и откат)

- **Атрибуция (Rule 1):** репозиторий человеко-авторский. НИКАКИХ «Generated with…»,
  «Co-Authored-By: <модель>» и любых AI-трейлеров в коммитах, коде, доках.
- **Коммиты:** Conventional Commits, атомарные по смыслу. Если владельческая коробка не
  даёт права коммита — деливерабл = ветка/патчи с теми же сообщениями.
- **Порядок посадки каждого коммита (LANDING-ORDER):** дифф → `cargo fmt --all` (плюс
  СОБСТВЕННЫЙ `cargo fmt` каждого затронутого пакетного воркспейса — хостовый их не видит)
  → `cargo xtask specmap` (если трогались спеки/код с якорями) → стейдж → полная панель
  `CARGO_BUILD_JOBS=4 bash tools/self-check.sh; echo "EXIT=$?"` — вердикт ТОЛЬКО по хвосту
  `all green` + `EXIT=0` (панель обрывается на первом красном) → коммит.
- **Спеки не правит исполнитель.** Статусы фактов PROP-054 (`spec/plan → impl/done`) и
  амендменты чужих PROP (список: `##AMENDMENT-PLAN` в спеке) двигает только владельческая
  сессия. Исполнитель ведёт черновик `campaigns/packages-2026-09/SPEC-DEBT-LIFECYCLE.md`:
  после каждой волны дописывает туда готовый ТЕКСТ амендментов с якорями — и всё.
- **Не трогать никогда:** `formats/EPOCHS.toml`; якоря спек (не переименовывать);
  вендоренные/фрозен-копии (`vibedeps/**`, `**/crates/vendor/**`, фрозен-слоты пакетов
  v0.x) — синк только `cargo xtask sync-engines` из авторской копии; токен-файлы
  `~/.vibe/*.token` (существование проверять можно, читать нельзя); историю git
  (force-push запрещён); версии пакетов не бампать; ничего не публиковать.
- **Файловый бюджет:** 600 строк на файл (conform считает и тесты) — новые файлы дробить
  по швам заранее, а не после красного шага.
- **Тестовая дисциплина:** каждый страж доказывается КРАСНЫМ прогоном (обезвредил
  охраняемое — упал ровно свой тест); `cargo test --no-fail-fast` при широких правках;
  красный СУЩЕСТВУЮЩИЙ тест при аддитивной правке — дефект правки, не теста.
- **Зеркала:** только `cargo xtask mirror` (ff-only), и только если владелец дал коробке
  право пуша; иначе — не пушить вовсе.

## §2. Форма каждого шага (инкрементальность — владельческое требование)

Каждый шаг ниже кончается **работающим состоянием**: блок «Демо» — команда и ожидаемый
эффект, проверяемый руками. Шаг не считается сделанным, пока демо не воспроизводится.
Порядок шагов внутри волны обязателен; коммит — на шаг (или мельче, по смыслу).

---

## §3. Волны

### R1 — дифф-материализация (спека §9; крейт `vibe-workspace`)

Зачем первой: сегодня `vibe install` по mutable-источнику сносит слот целиком вместе с
собранным в нём `target/` (спека `##WIPE-TODAY`). Всё, что строят волны дальше, живёт в
слотах — без R1 оно стирается.

- **Ш1.1 — слот-рекорд пишется.** В `vibedeps.rs::materialise_with` (и
  `derived.rs::materialise_with_spec_format`) после копии писать `.vibe-slot.toml` по
  схеме спеки §14.3 из уже возвращаемого футпринта + пофайловых sha256. Читалка с
  `deny_unknown_fields` и схемой-1. Рекорд исключён из хэш-идентичности слота (жанр
  `.vibe-derived.toml`, см. `derived.rs::collect_hash_files`).
  Тесты: запись/чтение round-trip; неизвестное поле — отказ; legacy-слот без рекорда
  получает рекорд при следующей перематериализации.
  **Демо:** `vibe install --path <тест-проект>` → в каждом слоте лежит `.vibe-slot.toml`
  со строками `[[file]]`; `vibe cache …`/хэши не изменились.
  Коммит: `feat(vibe-workspace): the slot record — materialise writes its footprint down`.

- **Ш1.2 — дифф вместо wipe.** При наличии рекорда `materialise_with` выполняет шаги
  §9.3 спеки (1–5) вместо `remove_dir_all`: неизменённый файл НЕ трогается (байты и
  mtime), изменённый пишется, исчезнувший из футпринта удаляется (+пустые родители,
  best-effort), **пути вне рекорда не трогаются никогда**. Без рекорда — прежний
  wipe+copy один последний раз (миграция).
  Red-proof: тест «`target/маркер` внутри слота переживает перематериализацию» обязан
  ПАДАТЬ на коде до Ш1.2; тест «mtime неизменённого файла стабилен».
  **Демо:** руками положить `vibevm/vibedeps/<слот>/target/probe.txt`, поправить один
  файл пакета-источника, `vibe install` → probe.txt жив, поправленный файл обновлён,
  остальные mtime не дрогнули.
  Коммит: `feat(vibe-workspace): materialise becomes a diff — build output survives`.

- **Ш1.3 — хэш-гейт mutable-источников.** В `install.rs` (trust-решение
  `materialise_resolution_with_spec_format`, ветка `dep.source_mutable`): вместо
  безусловной рематериализации — пересчитать content_hash shippable-дерева источника,
  сравнить с `source_hash` рекорда; равенство ⇒ skip (слот fresh). Спека
  `##MUTABLE-GETS-A-GATE`.
  Red-proof: счётчик записанных файлов; на нетронутом источнике = 0.
  **Демо:** дважды подряд `vibe install` по проекту с file://-пакетами → второй прогон
  печатает fresh/skip и не пишет ни файла; `touch` одного файла пакета → перепись ровно
  одного файла слота.
  Коммит: `feat(vibe-install): a mutable source earns its skip by hash, not by wipe`.

- **Ш1.4 — verify-heal и хуки по факту диффа.** `SlotIntegrity::Verify` сверяет пофайлово
  против рекорда и лечит только разошедшиеся файлы; PROP-020-хуки слота перегоняются
  только когда дифф непустой (`hooks_run.rs`).
  Red-proof: испорченный файл слота лечится один; нетронутый слот не перегоняет хук
  (счётчик HookRunner-сима).
  **Демо:** испортить один файл слота → `vibe install` (verify-режим) → одна строка
  «healed», хук не бежал; поправить источник → хук бежал один раз.
  Коммит: `feat(vibe-workspace): verify heals by the record; hooks rerun only on change`.

- **Ш1.5 — отчёт волны.** Дополнить `SPEC-DEBT-LIFECYCLE.md`: черновики амендментов
  PROP-011 (дифф-рулинг + гейт), PROP-020 §2.1 (reset = record-diff-restore), PROP-022
  §2.2, PROP-045 (манифест поглощён рекордом — с планом миграции transformed-слотов),
  по списку `##AMENDMENT-PLAN`. Кода нет.

### R2 — движок лайфсайкла (спека §3–§6 без agent; новый крейт `vibe-lifecycle` + вербы в `vibe-cli`)

- **Ш2.1 — грамматика `[[extension]]` в манифесте.** `vibe-core` manifest: таблица по
  спеке §14.1 (типы, обязательность, deny unknown, валидация точек; `compiler_internals`
  без `compile:pass` — ошибка, и наоборот). Юниты на каждый ряд таблицы.
  **Демо:** `vibe check` на фикстуре с валидной декларацией молчит; кривое поле — ошибка
  с именем поля.
  Коммит: `feat(vibe-core): the [[extension]] table parses — one grammar for every family`.

- **Ш2.2 — фазовая линейка и вербы.** Крейт `vibe-lifecycle`: таблица фаз
  (`validate…deploy` + clean), построение цепочки, алгоритм спеки `##ENGINE-ALGORITHM`
  (пока без вкладов: все фазы no-op). CLI: вербы-фазы (`vibe build`, `vibe test`, …;
  `vibe install` остаётся прежним вербом И фазой — одна реализация), обобщение
  `CleanChain` до `vibe clean <phase>` с сохранением PROP-053-семантики byte-в-byte
  (существующие e2e `cli_clean_and_world.rs` НЕ трогать — они обязаны остаться
  зелёными).
  **Демо:** `vibe deploy` на prompt-only проекте печатает цепочку из девяти фаз, каждая
  no-op, exit 0; `vibe clean install` работает как раньше.
  Коммит: `feat(cli): the phase line — vibe <phase> runs everything before it`.

- **Ш2.3 — сбор вкладов, порядок, ритуал.** Сбор `[[extension]]` из установленного мира
  (lock-порядок) + хоста; `[[extensions.use]]`/`[extensions].disable`; селекторы
  `applies_to`; печать ритуала до исполнения (`##SURFACE-THE-RITUAL`).
  Тесты: порядок §3.4 (пресет → зависимости по локу → хост); disable глушит ровно одну.
  **Демо:** фикстурный пакет с двумя декларациями → `vibe test` печатает обе в порядке,
  с пакетом-поставщиком.
  Коммит: `feat(vibe-lifecycle): contributions collect in one declared order`.

- **Ш2.4 — конверт и builtin `log`.** Конверт §5 (JSON + Rust-структура), builtin-реестр,
  `log` с плейсхолдерами `{phase}/{project}/{package}`.
  **Демо:** сценарий спеки §10.1 наполовину: строки печатаются в фазовом порядке.
  Коммит: `feat(vibe-lifecycle): the envelope reaches handlers — log proves it`.

- **Ш2.5 — freshness.** `.vibe/lifecycle.toml` по §14.2; фингерпринты; fresh-скип;
  `--force`. `vibe clean` файл НЕ удаляет (юнит).
  **Демо:** второй `vibe build` подряд — все executions `fresh`; `--force` перегоняет.
  Коммит: `feat(vibe-lifecycle): every execution is skip-when-fresh`.

- **Ш2.6 — script/binary хендлеры.** Провода §14.4: `VIBE_CONTEXT`/`VIBE_REPLY` для
  script (интерпретаторная лестница PROP-020 переиспользуется — `hooks.rs`), stdin/stdout
  для binary (PROP-025 `artifact`+build-if-missing, БЕЗ консент-промпта — печатаем и
  строим: `##INSTALL-IS-CONSENT`). `[hooks]`-таблица пакета читается как сахар двух
  `slot:`-вкладов — существующие пакеты с хуками работают без правок.
  Red-proof: падение script non-zero валит фазу по `##FAILURE-BY-PHASE`.
  **Демо:** фикстурный скрипт пишет файл-маркер в scratch; `vibe extensions`-прообраз
  (лог) показывает исполнение.
  Коммит: `feat(vibe-lifecycle): script and binary handlers ride the old wires`.

- **Ш2.7 — kind-пресеты и реестр.** Пресеты как данные (§4.5): rust-стек биндит
  `phase:build`→`cargo build`, `phase:test`→`cargo test` (через script/binary-жанр самого
  стек-пакета — vibe не хардкодит cargo; для хост-репо достаточно фикстурного пресета);
  верб `vibe extensions [--json]` — реестр §3.5 `##OBS-REGISTRY`.
  **Демо:** `vibe extensions --json` листает вклады с поставщиками; `vibe build` в
  rust-фикстуре запускает сборку.
  Коммит: `feat(cli): vibe extensions — the machine is a query surface` (+ отдельный
  коммит пресетов).

- **Ш2.8 — e2e владельческого сценария §10.1 целиком** (установка фикстуры → `vibe test`
  → обе строки в порядке → fresh на повторе → disable глушит одну → реестр видит всё).
  Коммит: `test(cli): the phase-announcer plugin — the commissioning scenario is green`.

### R3 — IR-рефакторинг компилятора (спека §7.5; крейт `vibe-spec`)

Гейт всей волны: **байт-идентичность** — после КАЖДОГО шага перегенерация boot-лейнов
хоста даёт `git diff --exit-code` по `vibevm/vibespecs/boot/` и по слотовым generated
(закоммиченный лейн — и есть оракул). Плюс весь тест-сьют vibe-spec зелёный.

- **Ш3.1 — типы уровней и пасс-скелет.** Явные типы Source/Document/Closure/Lane/Emitted
  (§7.4.1), трейт пасса (имя, уровень in/out), менеджер как список. Пока НИЧЕГО не
  переносить — типы + менеджер + один тривиальный identity-пасс с юнитами.
  **Демо:** `cargo test -p vibe-spec` зелёный; лейны байт-те же.
- **Ш3.2 — перенос фаз по одной.** Порядок: `parse` → `close` (topo/merge/embed-комплекс
  можно дробить на под-пассы `merge`/`embed`) → `qualify` → `absorb` → `link` →
  `assemble` → `emit:*` (пер-артефактные имена). Каждый перенос — отдельный коммит со
  своим byte-identity прогоном. `compile_static_inner` в конце — тонкая обёртка над
  декларированным списком пассов.
  **Демо после каждого коммита:** лейны байт-идентичны; в конце — `compile_static*`
  публичные сигнатуры не изменились (внешние вызыватели не тронуты).
- **Ш3.3 — verifier-скелет.** Инварианты уровней (`DuplicateId`-гейт, ацикличность,
  целостность маркеров) как проверки «после пасса», пока включаются только в тестах.
- **Ш3.4 — трассировка.** `--trace-compile`/`[compile] trace`: снапшоты
  `.vibe/trace/<run>/<seq>-<pass>-<kind>_<scope>_<artifact>-<ordinal>.json`
  (точный full/short Windows-safe codec, retention и бюджет — в
  `COMPILER-IR-TRACE-ARCHITECTURE-v0.1.md`) + таблица таймингов (`##OBS-TRACE`).
  **Демо:** `vibe install --trace-compile` на фикстуре → каталог с пронумерованными
  снапшотами; дифф двух соседних показывает работу qualify.
  Коммиты волны: `refactor(vibe-spec): …the <pass> phase becomes a pass` (серия),
  `feat(vibe-spec): --trace-compile — print-after-all for the lane`.

### R4 — staged-ярус компилятора (спека §7.2; `vibe-spec` + `vibe-workspace`)

- **Ш4.1 — четыре канонические позиции** как transform-пассы; активация хостом
  (`[[extensions.use]]` для compile-точек; auto=false закон §3.3); заголовок
  `<!-- vibe:transforms … -->` в артефактах; правило оракула §7.3 (byte-stable сравнение
  делается ДО трансформов).
- **Ш4.2 — builtin `xml-minify`** по алгоритму `##TEST-XML-MINIFY` (безопасность по
  форме родителя; комментарии/CDATA нетронуты) + e2e: включён → сжат и записан в хедер;
  выключен → байт-идентичен прежнему.
- **Ш4.3 — lane analyzer** (`##OBS-LANE-ANALYZER`): `vibe extensions analyze [--json]` —
  байты/токены по нодам, пакетам, дельтам пассов (связка PROP-048).
  **Демо волны:** включить минификатор в фикстуре → STATIC.xml меньше, `analyze`
  показывает дельту пасса; выключить → всё как было.
  Коммиты: `feat(vibe-spec): tier-1 staged transforms…`, `feat(vibe-spec): the xml-minify
  test vehicle…`, `feat(cli): vibe extensions analyze — the lane on the scales`.

### R5 — нативный ярус (спека §8; новый крейт `vibe-ext` + загрузчик)

- **Ш5.1 — крейт `vibe-ext`**: типы Context/Reply, макрос `vibe_extension!` (четыре
  C-символа, catch_unwind, (де)сериализация). Собственный юнит-сьют.
- **Ш5.2 — загрузчик** (libloading; семь шагов §14.4 `##REF-WIRE-NATIVE`; кэш по пути;
  паника плагина = провал execution, vibe жив — red-proof паникующим фикстурным
  плагином).
- **Ш5.3 — сборка в build-фазе** (`crate_dir` → cargo build в слоте; PROP-025-жанр;
  инкрементальность подтверждается mtime-законом R1); generated `.gitignore` в корне
  vibedeps (`**/target/`, `**/node_modules/`); prebuilt-загрузка по платформенному ключу
  (`##PREBUILT-CLOSED`) — на этом боксе демо только windows-x86_64, остальные ветки
  юнитами.
- **Ш5.4 — bootstrap-order** (`##BOOTSTRAP-ORDER`): install без собранного трансформа
  компилирует без него, метит `transforms-pending`, build дособирает и перекомпилирует.
  Red-proof: хедер-метка появляется и исчезает.
- **Ш5.5 — паритет-e2e §10.2**: минификатор нативной cdylib-фикстурой; байты выхода ==
  builtin-варианту (это и есть ассерт).
  **Демо волны:** `vibe build` собирает фикстурное расширение в слоте; повторный
  `vibe build` — cargo no-op; `vibe install` перекомпилировал лейн; вывод == builtin.
  Коммиты: `feat(vibe-ext): …`, `feat(vibe-lifecycle): the native loader…`,
  `feat(vibe-lifecycle): natives build in-slot…`, `test(vibe-spec): native minify parity`.

### R6 — pass-ярус (спека §7.4; `vibe-spec` + `vibe-core`)

- **Ш6.1 — валидация флага** `compiler_internals` (§14.1-строки; red-proof: `pass` без
  флага — отказ с ремедиацией).
- **Ш6.2 — IR-провод**: сериализация уровней (фризовка скелетов §14.6 из R3-типов;
  `ir_schema = 1`), manifest-гейт схемы у плагина.
- **Ш6.3 — позиции пассов** (`after`/`before`/`replace` по именам встроенных; frontend
  по расширению — адресная грамматика принимает зарегистрированное расширение как
  .md/.xml; backend по имени `emit:*`).
- **Ш6.4 — verifier включён** между плагин-пассами (невыключаем из манифеста; red-proof:
  фикстурный пасс, ломающий уникальность якорей, падает СВОИМ именем).
- **Ш6.5 — e2e**: фронтенд `.txt` (строка = параграф, титул из имени файла) — документ
  пакета входит в лейн и адресуется `spec://…`; JSON-бэкенд лейна; `vibe extensions`
  показывает internals-вклады.
  **Демо волны:** фикстурный пакет с `.txt`-доком и флагом → `vibe install` → нода в
  STATIC.xml, `vibe show lane --json` (бэкенд) отдаёт лейн машинно; без флага — отказ.
  Коммиты: `feat(vibe-spec): the pass tier — full IR behind one conspicuous flag` (+
  дробление по шагам).

### R7 — create и агентный ярус (спека §6.4–§6.5; `vibe-llm` + `vibe-lifecycle` + `vibe-mcp`)

- **Ш7.1 — провайдер-шов** `vibe-llm` реальный: `LLMProvider` + одна реализация
  «openai-совместимый HTTP endpoint» (конфиг `[llm]` в `~/.vibe/config.toml`; ключ —
  по пути к токен-файлу, содержимое не читать в логи). Тесты — мок-транспортом.
- **Ш7.2 — agent-хендлер, CLI-режим**: промт-адрес → резолв из мира → провайдер →
  outputs-контракт → валидация артефактов. Без провайдера — отказ с ремедиацией.
- **Ш7.3 — hosted-режим**: детект (`VIBE_INVOKED_BY`/env-жанр `cli.rs`), outbox-файлы и
  fenced-блок по §14.5, статус `delegated`, идемпотентный resume той же фазой.
  Red-proof: невыполненный контракт再-паркуется, выполненный — закрывается.
- **Ш7.4 — MCP-поверхность**: `lifecycle_run`/`lifecycle_tasks` над теми же файлами
  (жанр `vibe-mcp/src/tools.rs`, оракул-тесты как у `agentic_explain`).
- **Ш7.5 — нейтральный внешний work-loop substrate (owner ruling 2026-08-27,
  reaffirmed 2026-08-28).**
  Lifecycle остаётся пассивным framework, не кодинговым агентом: structured
  verification evidence, exact tree/run identity, CLI/MCP control/read surfaces и
  опциональный read-only adapter требований: full `spec://…#fact` address;
  authoring status, consumer adoption, edge provenance/provider freshness и
  typed gap/staleness observations — отдельные поля, не heuristic `unmet`.
  Status-bearing основа — `vibe-specdoc` + `vibe-facts`; current specmap лишь
  optional relation provider, compiler IR не подменяет fact status. Никаких
  встроенных Plan/Act policy, автоматического
  `create→verify→create`, выбора следующей задачи или LLM-зависимости. В PROP-054 —
  ненормативный PDSA reference scenario внешнего агента; референсная реализация самого
  агента — отдельная будущая кампания. Red-proof: fake external orchestrator использует
  только machine reports/tasks/facts; отсутствие IR adapter и LLM не меняет обычный
  lifecycle.
  **Демо волны:** в терминале с мок-провайдером `vibe create` производит файл по
  контракту; с `VIBE_INVOKED_BY=claude-code` — печатает `vibe-agent-tasks`-блок, повторный
  запуск после ручного изготовления файла продолжает цепочку.
  Коммиты: `feat(vibe-llm): the provider seam gets a real provider`,
  `feat(vibe-lifecycle): create — the agent handshake`.

### R8 — package/build/deploy (спека §4.2 + принятый successor-дизайн)

Цели deploy уже выбраны владельцем; `##OPEN-DEPLOY-TARGETS` не является
развилкой. Полная форма и порядок — в
`BUILD-PACKAGE-DEPLOY-ARCHITECTURE-v0.1.md` и
`SPEC-DEBT-LIFECYCLE-R7-R8.md`: artifact records/DAG, один общий mechanism
registry, Cargo commissioning, полностью статический skill, Agent Plugin 1.0,
Claude/Codex/OpenCode projections, profiles/intent/receipt/recovery,
`deploy:vibe-bin`, plugin replacement и Windows zip. Отдельный обязательный
atom `R8-PLATFORM-APPLICABILITY` добавляет first-class `when`/`os` к
`[[artifacts.package]]` и `[[deploy.target]]`: vocabulary ровно
`windows | linux | macos` переиспользует loading-model OS probe; неактивные
package targets ничего не производят, неактивные deploy targets не входят в
profile closure/collision checks, а active→inactive dependency отказывает с
именем обоих рядов. Parse/write, human/JSON plan и skip evidence входят в тот
же atom. Сегодняшняя подробная схема в successor-дизайне — подготовленная
hypothesis, не implementation freeze: когда atom станет current, обязательны
cold re-read актуальной grammar/engine, повторный design review и улучшенная
freeze до первого code edit. Будущий VibeVM OS остаётся
compatibility horizon, не текущей системной мутацией.

## §4. Сквозные гейты (каждый коммит, каждая волна)

- На каждом атоме — только точные affected tests/check/clippy. Полная панель
  `CARGO_BUILD_JOBS=4 bash tools/self-check.sh; echo "EXIT=$?"` запускается на
  неизменившемся дереве в конце связного batch и в финале эпика; её зелёный
  ХВОСТ обязателен. Панель до последней правки — не про то дерево.
- `cargo xtask specmap` — 0 unresolved, 0 suspects (21 warning — стоячие, их не плодить).
- Byte-identity лейнов (R3+): `git diff --exit-code vibevm/vibespecs/boot/` после
  перегенерации без включённых трансформов.
- Оба владельческих сценария (§10 спеки) зелёными с R2.8/R4.2 и навсегда.
- `vibe check` хоста: 0 errors (1 стоячий warning wal_wellformed не в счёт).

## §5. Мины (оплачены этим репозиторием — не переплачивать)

- Занятый бокс: `CARGO_BUILD_JOBS=4`; полный параллелизм линкеров даёт 0xc0000142.
- Windows-локи user-mapped section (1224) на STATIC/INDEX и Access denied (5) на свапе
  generated — транзиент: снести `generated.new-*`, перегнать.
- `check-codegen` диффит против git-ИНДЕКСА — свежий генерат требует `git add` до шага.
- `cargo xtask codegen` НЕ пропускать через `head` (SIGPIPE убивает 22 файла; лечение
  `git restore crates/vibe-wire/src/generated/`).
- Пакетные воркспейсы форматируются СВОИМ `cargo fmt`; синк-таргеты (mcp, vendor) руками
  не правятся — только авторская копия + `cargo xtask sync-engines`.
- `vibe progress scan --campaign …` — ПИШУЩИЙ верб (прунит out-of-scope записи с
  вердиктами). Исполнителю не запускать вовсе; замер долга — `judging-debt.py`.
- Пустой/обрезанный вывод инструмента — утверждение: скорми случай, который инструмент
  ОБЯЗАН различить, прежде чем читать ноль доказательством.
- Всякое число, на котором стоит решение, меряется двумя способами.
- Глубокие пути Windows: `git -c core.longpaths=true` при worktree-операциях.
- Правки файлов — editor-инструментами; PowerShell 5.1 портит UTF-8 no-BOM.

## §6. Зависимости и параллелизм волн

Историческая wave-формула была слишком грубой; действуют atom-level зависимости
ledger'а. R1 → (R2, R3 core). R6.2a/b следуют за R3.3 и предшествуют R3.4 — этот
порядок уже выполнен. R4.0 следует после коллектора R2 и typed compiler core;
R4.1–R4.3 используют один его kernel. R5 phase-native требует R2+R4, а native
compiler path также R6.2. R6.3–R6.5 требуют R3/R5/R6.2; R7.1–R7.4 требуют
R2, не R5. R7.5 следует за нейтральными R7.4-адаптерами, читает status-bearing
`vibe-specdoc`/`vibe-facts` и может опционально обогащать ответ current-specmap
relations, не превращая ни один provider в lifecycle dependency.
R8 artifact records/DAG могут идти после R2 параллельно, но mechanism
world/selection обязаны дождаться R4.0 и расширить тот же kernel — второй
collector запрещён. `R8-PLATFORM-APPLICABILITY` зависит от artifact DAG и
deploy engine, но не от provider replacement; он обязан использовать ту же
OS-probe semantics, что boot loading, без environment-selected profile.
Внутри каждого атома зависимости и конфликтные manifest /
install / compiler периметры сверяются с ledger перед fan-out.

## §7. Definition of Done эпика

Все восемь волн посажены; панель зелёная; оба сценария §10 зелёные; лейны без трансформов
байт-стабильны; `SPEC-DEBT-LIFECYCLE.md` полон (по волне на секцию) и передан владельцу;
статусы фактов спек не тронуты исполнителем; ничего из §1 не нарушено. Финальное слово —
владельческая инспекция.
