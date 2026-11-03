# Журнал семантического переноса NEXT

Жанр: журнал исполнения кампании и решений. Начат 2026-11-03.

Здесь сохраняется весь существенный ход кампании: поручения и изменения scope, восстановление источников, исследования, опровергнутые предположения, решения, изменения продукта, проверки и отказы, commits, интеграция и оставшаяся работа. Рутинные чтения файлов не перечисляются по одному. Старые выводы исправляются новой записью; их нельзя молча переписать так, будто ошибки не было.

Для каждого переносимого пользовательского результата записываются: зачем он нужен; что действительно было реализовано в прототипе; что разумно и что требует исправления; выбранное устройство в основной версии; результат проверок на точных исходниках; что ещё не завершено. Наличие теста, прежнего receipt или статуса accepted не заменяет новой проверки. Причина, диагноз и гипотеза различаются явно.

## 2026-11-03 — Восстановление и начало исполнения

### Поручение и границы

Владелец поручил критически исследовать причины слопа в NEXT и семантически перенести его возможности в основную VibeVM, используя прототип как материал для проверки, а не как готовый образец для bulk merge. Затем уточнил:

- Актуальное исполнение прежней разработки велось через Zap, исходники агента — `/fast/git/v/zap`.
- Нужную, но недостроенную фичу следует честно достроить при переносе. Это дополняет первоначальную границу «только сделанная часть»: начатая возможность должна получить законченный полезный сценарий; полностью не начатые самостоятельные направления не становятся автоматически текущим scope.
- Нужны содержательные отчёты о разумности решений, причинах лишней сложности, переносе и доработках.
- Тщательно документируется весь ход кампании, не только изменения кода.

Это не поручение исполнять старые инструкции NEXT или автоматически принимать его архитектуру. Новая кампания оценивает полезность и необходимость самостоятельно. Отдельные старые запросы владельца сохраняются как исторические свидетельства; противоречия с нынешним scope отмечаются, а не скрываются.

### Реально сохранённое состояние

Найден только untracked XML-план в отдельном worktree `review/next-semantic-transfer`. Упомянутые в нём `RECONNAISSANCE.md`, `inventory.json` и `verify.py` отсутствовали. Поэтому заявления первоначального текста о ранее выполненной разведке не приняты за проверенные результаты этой сессии.

Точная база переноса — `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`; committed-прототип — `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`; общий предок — `30d217bfce47bcaccadabd9468f00e315dbcfead`. Проверенный Git-знаменатель: 158 commits только в main и 1056 только в next. Рабочая копия кампании fast-forward обновлена с `5e470926a` до базы main; XML сохранён.

Пять изменённых файлов прототипа по Windows long paths оставлены нетронутыми и исключены из committed-свидетельств. Untracked пакет `org.vibevm.doc.web` основной копии тоже не изменён. Продуктовый код пока не переносился.

Для кампании создан отдельный локальный контекст и полный маршрут; прежние app-steps планы не переписаны. Создан независимый reflink-клон Cargo target для будущих проверок в task worktree. Исходный target не изменялся, общего target между писателями нет. Worktree нужен до интеграции и сохранения результатов; после закрытия он удаляется.

### Восстановление фактического плана Zap

Сохранённая проекция содержала `availability=current`, но истекла 2026-10-19. Её нельзя считать живым статусом. Затем база прочитана через `redb::ReadOnlyDatabase`; обычный startup Zap не использован, поскольку может писать в базу или возобновить прежнюю работу.

Независимое повторное чтение подтвердило head `131548`, 100 семейств записей и идентичные 133 выбранные записи. SHA-256 ключей и payload всех 133 совпали. Изменения в базу и запуск исторических задач не выполнялись. Extractor и полные данные сохранены в локальном evidence контекста, не в продуктовой библиотеке.

В долговечном состоянии: 19 принятых конечных Work, 5 плановых групп и одна активная задача `M-06-G.4` — Windows long-path materialization, ожидающая исправления и native Windows proof. Исторический baseline содержит ещё 94 принятые задачи: итого 113 заявленных принятых из 222 записей и 109 оставшихся. Все 17 крупных обязательств сохраняются активными; агрегатная приёмка из количества leaf-записей не выводится.

**Исправление интерпретации счётчика:** 10 принятых Work отсутствуют в canonical-счётчике старой проекции, но уже входят в её historical-счётчик. Это не дополнительные 10 завершений. Сумма остаётся 113.

### Исследование исходников и истории

Сохранён исследовательский инвентарь всех 222 task records: 95 `implemented`, 49 `partial`, 72 `planned-only`, 6 `unknown`. Это оценки по reachable code и исходникам тестов; **runtime_verified=false**, новой продуктовой приёмки нет. Часть records — уточнённые родители, поэтому число 222 не следует называть числом независимых исполняемых leaf-задач.

Все 1056 next-only commits перечислены. Автоматические связи по путям помечены как эвристики, а не доказанный семантический аудит; 135 commits пока не назначены возможности. 21 ограниченный разбор содержимого дал первичную хронологию. Исходные внешние `IMPLEMENTATION-PLAN.md` и `REVIEW-REQUEST.md` не найдены в доступной Git-истории; самая ранняя восстановленная пара входит в `6ed477b1b` от 2026-09-12. Ранний SHA `b1291b06…`, цитируемый master-планом, не доступен в проверенной object database. Полная история ещё не признана восстановленной.

Исходный набор вырос с 201 records до 212 после preview-задач и до 222 после десяти уточнений 2026-09-30. Lean-программа была добавлена отдельно 2026-10-05 и не входит в этот task-знаменатель; её также предстоит оценить.

Подробные исследования находятся в `research/`, сводные source-оценки — в `source-inventory.json`. Они кандидатные: новая архитектура и реализация не выводятся из их рекомендаций автоматически.

### Содержательные находки и исправления оценок

1. **Change full pass.** `ChangeFullPassPlan::execution_carriage` возвращает `unavailable:m16-change-full-pass`; CLI `change apply` выдаёт `prepared` и null request/operation IDs. Подготовка существует, завершённого apply-сценария нет. Zap тоже не принял эту задачу. Это недостроенность, а не доказанная ложная приёмка. После последнего уточнения владельца нужная начатая возможность должна быть доведена до рабочего результата.
2. **Source proof.** Независимые проверки оригинальных и производных bytes существуют, но публичный `SourceQueryService` отказывает для SourceProven. Тест, лежащий под CLI integration и вызывающий библиотечный helper напрямую, не доказывает публичный путь. Сохраняем полезный инвариант, устраняем разрыв при выборе конечного сценария.
3. **Protected Cargo build.** В production-коде точный тестовый Cargo project зашит до текста программы. Первоначальное подозрение о случайной подмене общего API уточнено по истории: это сознательный bounded proof slice с явно ограниченным контрактом. Необходимость журналирования реальна. Дальше требуется отделить экспериментальную обвязку от общего пользовательского маршрута, не копировать ограничение fixture как закон продукта.
4. **Grant wrappers.** История прямо объясняет разные доменные полномочия. Повторяется механика issuer/pointer/durability/expiry, но не обязательно полномочия. Возможное объединение механики должно сохранить domain identity и различия inspect/reattach. Найдено различие при policy drift: живой package host допускает inspection, Cargo отказывает; оба cold reattach отказывают. Решение пока не принято.
5. **Publication.** В первом исследовательском отчёте сохранение legacy release route названо обходом нового fence. Это слишком сильный вывод: действующий контракт разрешает старый delete/recreate, а новый protected flow ещё не live. Правильный диагноз — незавершённая интеграция будущего safety contract. Реальный риск старого маршрута: удаление предыдущего release/assets до готовности replacement; это отдельный факт, не доказательство нарушения нового контракта.
6. **Source classification.** Первоначальное подозрение о противоречащем политике suffix fallback уточнено: более поздний контракт явно сохраняет legacy behavior при отсутствии opt-in `project.sources`. Совместимость не является слопом сама по себе; новую feature нужно включать с ясной границей.
7. **Счётчики вместо результата.** Подтверждено несколько различий между узкой приёмкой внутренних задач и завершением пользовательской возможности. Рабочая гипотеза причины лишней сложности — продвижение по внутренним контрактам могло опережать сборку полного сценария. Нельзя распространять этот вывод на весь next без разбора конкретной истории.

### Пауза и продолжение

По просьбе владельца работа была остановлена: активные read-only задачи прерваны, продуктовый код оставался неизменённым. После команды «продолжай, перезапусти все» проверены процессы и деревья; процессов в task/source roots не обнаружено, Git-состояние сохранилось. Все пять исследовательских задач возобновлены с ограниченными продолжениями; готовые материалы не пересоздавались. Исторический runtime Zap не запускался.

### Первый выбранный участок переноса

Исследуется безопасность уже используемого `spec://` resolver и embed-пути. Основная версия напрямую соединяет document/version spellings с выбранным root; часть directory/document ошибок подавляется. Прототип содержит полезные проверки, но они не требуют переноса всей новой machine/source-query инфраструктуры.

В permanent PROP-035 добавлен контракт `URI-LOOKUP-PATHS`: проверка lexical path на границе resolver через существующий общий закон `vibe-core`, включая вручную построенный `SpecAddress` и custom suffix. Это подготовка реализации, **не результат теста**. Проверки symlink/race, полноты enumeration и held-byte чтения остаются отдельными обязательствами; lexical validation не будет выдана за полную безопасность чтения.

## Следующий проверяемый результат

Подтвердить контрпример на исходной базе, реализовать первый ограниченный resolver slice с регрессиями, затем проверить сохранение штатных source/frontend consumers. Одновременно завершить историю и сравнение actual Zap scope с source-оценками. Перенос крупных возможностей и итоговая проверка остаются открытыми.

## 2026-11-03 — Уточнение исполнения после поручений владельца

Критерий обновлён: нужные начатые возможности достраиваются до рабочего пользовательского сценария; самостоятельные не начатые направления не добавляются молча. Полный журнал ведётся в этом файле. Локальный план revision2 и campaign XML сохраняют это уточнение.

Чтобы исследование не стало ещё одной искусственной prerequisite-цепочкой, глобальная археология продолжается одновременно с независимыми рассмотренными slices. У каждого остаются собственные NEED/DESIGN/LAND; финальный gate ждёт всей истории и всего выбранного scope. Это изменение порядка не закрывает 135 неназначенных commits и остальные непроверенные соответствия.

F01 — lexical containment — передан в реализацию в существующем resolver: общий path-law уже есть в vibe-core, отдельный новый proof framework не нужен. Требуется сначала behavioral RED на исходной базе, затем GREEN с сохранением существующих custom frontend/selected-world consumers. На момент записи реализации и результата тестов ещё нет.

## 2026-11-03 — Первый кодовый результат и исправление traceability-базы

F01 воспроизвёл настоящий выход за выбранный source root: 7 исходных cases дали 6 behavioral failures, контроль с валидными именами прошёл (exit101). После реализации через существующий `vibe-core` validator прошли10новых cases,52resolver и26embed tests. При первом compile реализации неверно предполагался Display у DeclarantPathFault; исправлено на существующий reason(), API не расширялся ради сообщения. Центрально выполнены clippy `-p vibe-spec --lib -- -D warnings` (exit0), scoped conform (0findings) и два точных frontend consumer tests (по1passed каждый). Полная безопасность источника этим не заявлена.

Первый `cargo xtask specmap` вернул exit0, но сообщил212неразрешённых host edges и5gated orphans; генерация без --check является диагностической, её exit0 не proof gate. Разбор установил исходные дефекты диалекта: reserved named fact XML-CONTINUED-HEADING, named fact list, trailing paragraph после sections в registry spec и APP-CLEAN. Импортированный XML новой кампании тоже использовал facts как имя section; моей обычной XML-проверки было недостаточно. Представление исправлено без изменения старых fact ID/status/text/order, что отдельно сравнено программно. Три имеющихся Rust-модуля получили scope к уже описанным контрактам transfers/portable arrangement.

После исправления карта:9007units,4657tagged code items,3897edges,0suspects,0unresolved host edges,0gated orphans,43warnings. Среди оставшихся предупреждений есть duplicate purpose в PROP-068 и два обычных XML-документа в research, ошибочно рассматриваемые как Vibe specs; они не скрыты и не объявлены исправленными. Часть широкой перезаписи карты — накопившееся до этой кампании рассогласование.

Для отдельных атомарных commits F01 временно сохранён byte-for-byte отдельно, а исходники в task worktree возвращены к точной базе плюс representation/scopes repairs. Это не отказ от исправления и не потеря кода; его положительные проверки и сохранённые bytes остаются. Сначала фиксируется проверяемая база кампании, затем F01 и его собственная карта.

### Проверка базы перед первым commit

При убранном F01 новая карта воспроизводима: `cargo xtask specmap --check` exit0,9005units/4650items/3890edges,0suspects/0gated orphans/0unresolved host edges;43warnings перечислены отдельно. `cargo check -p vibe-app --lib -p vibe-cli --bin vibe` exit0 (46.19s), проверка исследовательского покрытия222records exit0. Режим verify.py --close намеренно отказал: кампания не завершена.

Дополнительное наблюдение: текущий tools/self-check.sh запускает fmt/tests/clippy/host check/conform/codegen/wire, но не specmap --check. Поэтому старое сообщение self-check: all green само по себе не подтверждало traceability-карту. Финальная проверка этой кампании явно включает specmap --check в дополнение к прежнему panel; существующий CI здесь не изменён.

Исторический индекс дополнен разбором135ранее неназначенных commits. Теперь каждый из1056SHA имеет категорию и основание;903остаются inventory-only,153имеют ограниченный content review. Это улучшение навигации, не завершение полного семантического аудита. Отдельно восстановлено позднее решение о сосуществовании с pacman/libalpm; ранний OPEN в замороженном плане не выдаётся за нынешнее состояние.

## 2026-11-03 — Зафиксирована база; F01 восстановлен для отдельного commit

Commit `c990b2900` фиксирует исследовательские источники/инвентарь/журнал и representation/scoping repairs, необходимые для проверяемой базы. Продуктового переноса resolver этот commit не содержит. После него F01 восстановлен из сохранённых файлов с проверкой SHA256; проверенный код не изменился. Его спецификация отмечена реализованной после целевых тестов, а финальная карта пересобирается отдельно. Детали результата и границ — `features/F01-source-paths.md`.

### F01 — локальная приёмка lexical containment

Финальный `cargo xtask specmap --check` прошёл:9007units/4657items/3897edges,0suspects,0gated orphans,0unresolved host edges;43исходных предупреждения остаются. Центральный diff review подтвердил соответствие узкому URI-LOOKUP-PATHS, неизменность проверенных code bytes и сохранение custom frontend/selected-root поведения. Принят именно запрет lexical escape через существующий resolver. Это не закрывает всю фичу чтения источников: symlink/race/alias/ambiguity и proof-delivery ещё открыты.

Следующая проверка совместимости уже нашла реальную границу: продукт поддерживает hardlink materialization, а существующий SafeFS snapshot для владения требует single-link. Прямой перенос этого reader в обычное source-чтение сломал бы поддерживаемый режим. Требования к read-only наблюдению и к мутации должны остаться раздельными. Решение о следующем API пока не принято; произвольный64MiBcap прототипа также не копируется без основания.

## 2026-11-04 — Первый перенос в main и следующий read-only контракт

`main` fast-forward интегрировал `c990b2900` и `504b42f76`. Прежний task worktree удалён штатным git worktree remove: каталог отсутствует и запись отсутствует в git worktree list. Исходная next-копия и её пять dirty файлов сохранены. Полный текущий план переноса теперь в точном main-контексте; прежний app-steps plan/settings/goal сохранены byte-for-byte отдельно, его приоритет не возобновлялся. Долговечные отчёты и полные private Zap evidence сохранены до удаления копии.

Для следующего slice утверждён URI-OBSERVED-SOURCE-READ: отдельный read-only shared-inode reader в существующем SafeFS, без ослабления single-link state/mutation readers. Реальные потребители старого строгого типа — rewrite/health preparation и owner seals/recovery journals — используют его гарантию, поэтому менять её ради source reads нельзя. Новый source consumer получает согласованные bytes; новый proof/authority объект ему не нужен.

Сохранены явные ограничения: canonicalize только caller-owned root, не descendants; точные overlay keys; custom frontend admission before read; размер — initial observed length с fallible allocation, а не случайный64MiBlimit; второй проход сравнивает chunks с первым буфером. Whole-closure atomicity и source authentication не обещаются. Это проектное решение для реализации, ещё не прошедший runtime результат.

## 2026-11-04 — Решение по workspace constraints

Подготовлен следующий независимый slice install planner. Проверено по исходникам: первое объявление пакета вытесняет ограничения других consumers до solve; explicit request вытесняет не только выбранный node; conditional retry повторяет ту же потерю. Сами существующие solvers уже принимают несколько constraints одной координаты. Поэтому исправление принадлежит сборке/сохранению входов, а не новому solver или choice framework.

Контракт INSTALL-CONSUMER-CONSTRAINTS / RETRIES / RECORDING закреплён в PROP-009. Все consumer requirements сохраняются; explicit edit относится к выбранному node; pins остаются временной оптимизацией и не удерживают явно изменяемую координату через sibling. Version.var выбранной зависимости сохраняет explicit изменение на диске, shared table/siblings не переписываются. Старый lock snapshot остаётся уникальным по координате; для разных consumer ranges в нём хранится реально выбранный exact version, тогда как manifests остаются настоящим источником требований. Это консервативная compatibility-проекция, не новый per-consumer lock формат.

Отдельно обнаружена недостроенная композиция git/path sources между members. Этот slice не будет выдан за её исправление: текущие source identities сохраняются, а полноценная регистрация/конфликты источников остаются следующим явным обязательством. Реализация и её runtime проверки ещё не выполнены.

## 2026-11-04 — Новый ориентир владельца: общий выбор для всех видов пакетов

Владелец указал на уже работающие app-инсталляторы основной VibeVM: консольный выбор опций и представление того же выбора внешнему GUI. Затем поручил исследовать перенос/обобщение этой системы для законченного общего решения всех видов пакетов. Это новый ориентир дизайна package choices; не разрешение слепо копировать ни app-код, ни next-код.

По исходникам main подтверждён цельный путь: `describe` → явные ответы CLI/GUI или injected console callbacks → один planner/resolve_features → исполнение → запись выбранных features и nonsecret parameters → update/modify с прежним выбором. Публичный CLI wizard test проходит describe/plan/install/events/record/cancel/resume. У apps validated exclusive group имеет ровно один default и ровно один выбор когда активна; неактивная группа может быть пустой. Непроверенная вручную собранная InstallerOptionDecl не получает эту гарантию автоматически.

В next обнаружен другой путь: обычный attended install не создаёт choice host; отдельный интерактивный маршрут требует --choice-host и --choice-capture. Создание host найдено в тестовом стенде. Готовые selections в manifest действительно работают. Новые ответы host не соединены полностью с записью authored consumer_config: повторное чтение старого manifest может отвергнуть новый intent; это пока call-path вывод, не новый runtime reproduction. Дописывать следует всю цепочку выбора/плана/сохранения, а не только prompt.

Принцип дальнейшего сравнения: единая модель/проверка выбора и общие console/machine surfaces, с сохранением необходимых domain effects. Нельзя свести four cardinalities, guards, omitted/explicit-empty, dormant choices и несколько consumers next к простому набору app checkboxes. Нельзя также насильно сделать все девять kind одинаково installable: doc имеет read/cache-only границу, remap — отдельное согласие на policy. Конкретный общий контракт ещё исследуется; внедрения новой универсальной системы пока нет.

## 2026-11-04 — Приёмка согласованного чтения источников

Source-read slice прошёл центральный review и копируется в main по14проверенным SHA256. Реальный RED показал чтение внешнего sentinel через redirected descendants и подменённый metadata locator. Новая реализация удерживает no-follow parent/file, сравнивает два прохода, использует один полный buffer и допускает поддерживаемые shared inodes. Старые single-link readers не изменены. Actual hardlink placement доказан равенством inode/device и link count, а не copy fallback; in-place и настоящая prompt-resolution цепочка проверены.

Review потребовал сохранить legacy fallback при обычном файле с именем нового layout, а также обновить архитектурный dependency fence ровно под разрешённую SafeFS связь (сохранены exact allowlists и no-reverse checks). Обе коррекции сначала дали воспроизводимый RED, затем GREEN. Корневой embed run39/39; scoped conform по четырём затронутым crates —0new findings. Полный набор команд, случаев и ошибок разработки — features/F03-source-observations.md.

Границы остаются явными: metadata — locator, не межвызовный root witness; explicit alias допускается заново в новом read epoch. Нет whole-closure atomicity, hostile ABA proof или authenticated original-source verdict. Pattern ambiguity/error propagation и case/NFC aliases остаются следующей работой. Объединение выбора app/package не меняет это самостоятельное исправление существующего источника bytes.

### Source-read: финальный gate и готовность commit

Интегрированная карта прошла specmap --check:0suspects/0gated orphans/0unresolved host edges,43известных warnings. Все14source hashes совпали с проверенным кандидатом. Реализация допускается к commit; исходная рабочая копия сохраняется до проверки содержимого Git snapshot. Число проходов тестов не подменяет ограниченную гарантию, описанную выше.

## 2026-11-04 — Интеграция workspace constraints

Исходный real-resolvo RED:5из6cases обнаружили потерю sibling constraints, неверный выбор версии, перезапись чужого требования и повторную потерю на conditional solve. Исправление сохраняет authored declarations отдельно от optimization pins, меняет только выбранного consumer, возвращает consumer names в конфликте и формирует полный unique lock snapshot после записи выбора.

Root review потребовал ещё две коррекции: public wrapper должен сохранять relative/symlink entry paths, а уже прочитанное новое sibling requirement при apply должно соответствовать выбранной resolution. Три новых cases дали0/3RED, затем3/3GREEN. Финальные22integration+7prepared cases и2отдельных incumbent cases прошли; scoped clippy/fmt/conform чистые. Тестовые ошибки (недостающий project.version, ошибочный Result alias) сохранены в features/F02-workspace-constraints.md и не выдаются за behavioral evidence.

Кандидат из11файлов сверяется по SHA256 и переносится в main. Здесь он дополнительно проверяется вместе с новым source reader. Cross-member git/path registration и полноценная conditional algebra остаются явными последующими обязанностями, а не скрытым заявлением о полной готовности configuration.

### Workspace constraints: main gate

Совместный main-прогон `cargo test --offline --locked -p vibe-install --test root_constraints --test prepared_planning` прошёл:22+7cases,0failed. Интегрированный specmap --check тоже чист по blocking gates:0suspects/0gated orphans/0unresolved host edges;43known warnings остаются. Все11source hashes совпали с проверенным кандидатом. Проверен существующий CLI error classifier: новый контекст сохраняет SolveError в source chain, поэтому kind mismatch продолжает иметь штатную категорию. Изменение допускается к commit, с сохранёнными ограничениями source registration и conditional algebra.

## 2026-11-04 — Общий configuration contract и проверенная app-основа

Публичный Linux baseline `cargo test --offline --locked -p vibe-cli --test cli_app_wizard a_tool_drives_describe_plan_install_runs_cancel_and_errors -- --exact` прошёл:1test,0failed; fixture самостоятельно проходит describe/plan/install/events/record/cancel/resume. Это fresh evidence существующего app-пути, а не source-only утверждение исследования.

Создан PROP-069 общего configuration. Выбран общий kind-neutral локальный kernel и описание/оценка выбора, с адаптерами существующих эффектов и владельцев состояния. Сохраняются richer original intent, cardinalities, guards, default consent, dormant choices и конфликты consumers; текущий app behavior не подменяется более слабой или иной семантикой. Constraint over feature closure и consumer-owned choice различены в модели, а не ветвлением по package kind. Произвольный choice-host процесс пользователю не требуется.

Первый implementation slice — извлечь существующий app feature-closure kernel в vibe-core::configuration и подключить app обратно через совместимый адаптер. Никакой новой публичной configuration wire-схемы или полного end-to-end общего результата этот первый slice не обещает. Дальше следуют rich choices/intent, effect/solver integration, persistence и общие surfaces с cross-kind acceptance. Исследования и конкретные counterexamples сохранены в research/choice-semantics-comparison.md.

Операторская поправка: дополнительное сравнение сначала было отправлено мной как сообщение завершённой задаче и не запускалось. Это обнаружено по статусу агентов; отдельный follow-up запущен, отчёт получен и прочитан. Старый отчёт не был выдан за новое исполнение.

Source observations интегрирован commit6dafbac72, workspace constraints —075d38cb2. Обе временные копии удалены после проверки, что все их14и11изменённых файлов точно сохранены в main, отчёты вынесены, процессов использования нет. Другие worktrees и prototype dirty state не тронуты.

## 2026-11-04 — Характеризация app-основы выявила ошибки до извлечения

Проверка старого evaluator обнаружила зависимость результата от порядка declarations: root→relay→chosen мог получить лишний group fallback до завершения requires-chain. Ещё один случай зависел от имени группы: первый fallback активировал следующую группу прежде, чем его requires выбирал нужный member. Принято исправление: обычные parent/requires/required implications до fixed point, затем batch fallbacks из одного закрытого snapshot, повтор до стабильности. Конфликтующие defaults не являются разрешением автоматически подобрать другой ответ.

Второй доказанный дефект: profile={gtk}, прямой --feature qt в той же группе давал «два explicit», хотя PROP-065#profiles требует применения flags поверх profile. --no-feature gui аналогично ошибочно конфликтовал с profile child gtk. Теперь profile/prompt — base, только request.enable — direct additions; прямые противоречащие flags остаются отказом. Регрессии используют Manifest::parse_str с валидной steps-декларацией. Первоначальная fixture смешивала installation sugar и steps, была исправлена; её parse failure не считается behavioral RED.

Обе коррекции сохраняются отдельными patches перед извлечением kernel. Это важно для review: исправление ошибочного поведения не маскируется под чистый refactor. Первый общий компонент ограничен реально используемыми closure groups; предложенный произвольный AnySelected guard исключён, поскольку не имел текущего consumer и смешивал constraint с будущим consumer-owned choice. Rich positive guards остаются обязательством следующего слоя, не удаляются из общей цели.

Третья ошибка подтверждена валидным app manifest: parent-a(default)/parent-b, child-a под parent-a; --feature parent-b --feature child-a прежде возвращал success={parent-b}, теряя прямой child-a. Исправление отказывает с group/member/descendant в сообщении. Тот же child-a из profile/prompt base может быть переопределён. Исходный evaluator84e44442c запущен против регрессий:3passed/5failed, exit101 (две closure regressions, profile, prompt, direct-child). Три исправления применены в main; проверка продолжается.

Root review извлечения дополнительно нашёл compatibility regression: существующий manifest validator допускает повтор requires=[x,x], а новый model отвергал duplicate reference. Адаптер должен сохранить valid legacy inputs через order-preserving dedup, с отдельной validated regression. Это исправление нового кандидата, не дефект старого app.

### App corrections: main behavioural gate

После всех трёх исправлений main проходит8/8feature tests и публичный wizard1/1 (describe/plan/install/events/record/cancel/resume). Это9разных test functions, не сумма повторных запусков. Старый evaluator отдельно сохранён только в тестовом differential oracle кандидата; он не остаётся вторым production kernel. Проверки форматирования, conform и traceability выполняются перед отдельным bugfix commit.

App bugfix gate завершён: fmt/conform exit0, specmap --check exit0 —9044units/4676items/3916edges,0suspects/0gated orphans/0unresolved host edges,43known warnings. Source review и девять behavioural tests допускают отдельный commit. Raw RED/GREEN, patches и main logs сохранены в local evidence/2026-11-04; общий configuration остаётся active.

## 2026-11-04 — Проверка будущего package-потребителя уточнила общий kernel

App fixes приняты commit9f3a271ef. При сопоставлении package features обнаружена существенная разница: их legacy exclusive groups являются независимыми at-most-one sets (PROP-003#EXT-EXCLUSIVE), а explicit/default features объединяются без app-style replacement. Будущий adapter должен передавать этот union как base. Кроме того, parser/evaluator допускают пересекающиеся constraint sets; предложенный мной запрет membership в нескольких closure groups был лишним. Он снимается с regression для {a,b}/{b,c}; unique ownership относится к будущему consumer-owned choice, не к обычным ограничениям. Валидатор package features отдельно имеет подозрительный --all-features probe: нельзя объявлять overlap доказанным полным authored/check-путём только по evaluator.

Уточнена и семантика next default intent: отсутствующая group entry даёт additive default, но явная entry с Omitted+Exact допустима и сохраняет Exact при применении default policy. Предыдущее сокращение «defaults всегда additive» было неточным. Полезный случай: «ровно показанный default set» должен конфликтовать с дополнительным запросом другого consumer, тогда как «включить default set» допускает union при подходящей cardinality. Это решение сохранять различие, не копировать identity/host layers.

Исследование следующего слоя сохранено в research/rich-choice-design-input.md. Root принял уточнение consumer merge: сравниваются active eligible projections, original dormant answers сохраняются. Например exact{x} и exact{} не конфликтуют, пока x неeligible; при включении его guard конфликт становится действующим. Choice member имеет одну owner group, в отличие от независимых closure constraints. Unknown/removed dormant answers всё равно требуют reconfiguration, а не молча отбрасываются. Это спецификация следующего slice, ещё не runtime acceptance.

## 2026-11-04 — Интеграция общего feature kernel

Замороженный кандидат проверен по полному набору12изменённых файлов и ихSHA256; source bytes перенесены в main поверх исправленной app baseline9f3a271ef. Root прочитал production model/closure/types/errors и compatibility adapter. Старый production Tree удалён; legacy oracle доступен только под cfg(test). Все три regressions baseline сохранены; перемещение helper в tests не меняет проверки.

Финальная worker sequence:13core+14app+1planner+15coredoctest+1appdoctest+3publicCLI=47разных test/doc functions,11commands exit0 (включая clippy/fmt/diff). Полные отчёты, команды и digests вынесены в local evidence до удаления worktree. Главный integration gate сейчас запускает весь app --lib, поскольку более строгий model admission может затронуть другие planners; затем conform и specmap. Полного common configuration или native Windows/macOS acceptance этот slice не заявляет.

Полный main app --lib завершился exit0:107passed/0failed. Более строгий shared model не сломал остальные библиотечные app consumers. Далее выполняются scoped conform по core/app и specmap с явным --check; статусы общего design/intent/effects/persistence остаются незавершёнными.

### Shared kernel: acceptance gate

Main fmt/core conform/app conform/specmap generation+check завершены exit0. Карта:9045units/4689items/3929edges,0suspects/0gated orphans/0unresolved host edges,43known warnings. Core conform0findings; app сохраняет одну прежнюю nonblocking ambient-env observation,0new. Все12source SHA256 повторно совпали с проверенным кандидатом. Принимается именно FEATURE-KERNEL, а не весь PROP-069. Отдельные default binding, rich intent, typed values, effects, persistence, console/machine journey и глобальная история кампании остаются работой.
