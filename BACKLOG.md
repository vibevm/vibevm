# BACKLOG — what the mega-refactor found and did not do {#root}

_Created 2026-07-26 by owner directive. **Findings raised during the PROP-043
Progress-Control programme that are neither the campaign's own work nor an
emergency collect here, and the next wave of work drains from this file.**_

**Not `TASKS.md`.** That file is a live checklist for one work-slice — items
that are commits waiting to be made. This one is the opposite genre: findings
nobody is working on yet, kept so the decision to work on them can be taken
deliberately later. Two different questions, two files, by owner ruling.

---

## What this file is, against the three that resemble it {#boundaries}

| file | holds | drained by |
|---|---|---|
| @fact:REL-TASKS `TASKS.md` | the current slice's checklist — each item is a commit | itself, as work lands |
| @fact:REL-DEFERRALS `campaigns/<id>/deferrals.md` | **one campaign's** tails; dies with the zone | the next campaign's mandate (`campaign-plans` law) |
| @fact:REL-AUDIT `AUDIT.md` | the periodic health sweep; an append-only **trend** | re-judged at the next audit |
| @fact:REL-BACKLOG **this file** | product-shaped findings the programme surfaced and deliberately did not act on | the next wave of work, after the programme reaches its end |

- @fact:TASKS2-OUTLIVES-THE-ZONE **It lives at the repository root because a campaign zone is
  disposable.** `ZONE-LIFETIMES` says `run/` is throwaway after close-out and
  wave 1's already is. A finding about where the product should go outlives the
  campaign that noticed it.
- @fact:TASKS2-GENRE **Genre: forward-looking, non-binding, drained by a later mandate.** Not
  a contract, not a checkpoint, not a health record. `spec-genres`' map does not
  carry this genre — the row is owed, alongside the documentation row Phase G
  adds.

## Карта развития — порядок осушения этого файла {#map}

@fact:MAP-POINTER **Как записи этого файла складываются в развитие системы — [`TOOLING-MAP.md`](TOOLING-MAP.md)** (рядом, корень репозитория): четыре плоскости инструментария с измеренным состоянием каждой, хребет зависимостей, предложение волн, десять развилок владельца, пять наблюдаемых вех. Карта — производное: записи здесь и рулинги владельца побеждают её везде, где разойдутся. Одобрена владельцем 2026-08-02 («мне нравится этот документ») с рамкой: **действуем внутри идущего рефакторинга (PROP-043, волна 2 — кампания packages-2026-09; фаза D → E/T/F/G), чего не хватает — откладывается на потом**; карта — форма осушения, не параллельный процесс.

@fact:MAP-WAVES-DIGEST Волны одной строкой (полные составы — в карте): **А** — детерминированная загрузка (B-011 самым высоким приоритетом → B-006/B-031/B-028); **Б** — паритет гейтов и новые классы правил (B-029/033/034/039/030 под циклом B-035 → B-036/037/038 → B-025/026); **В** — карта и её потребители (B-013 done 2026-08-03 → одна смена формата B-019а+B-016.1+B-017 → B-018 → B-020/021); **Г** — хост догоняет дисциплину (B-040, B-005, spec-метки схем) — оппортунистически. Вне волн: B-042 (далёкое будущее), B-015 (запаркована), B-043.

## The three severities {#severity}

The scale is **P1 / P2 / P3**, taken from the `health-audit` flow rather than
invented. One severity vocabulary in the project, not two.

| | meaning | routing |
|---|---|---|
| @fact:SEV-P1 **P1** | security, data loss, structural integrity — **or a gate reporting green while not looking** | **stops the wave, reaches the owner the same session.** It never enters this file as a plan; it appears only afterwards, as record |
| @fact:SEV-P2 **P2** | a real gap with no emergency in it: a missing surface, a feature the corpus assumes and the code lacks, a mechanism specified and unbuilt | **this file.** Drains into the next wave |
| @fact:SEV-P3 **P3** | noted, no action planned | recorded here as `accepted`, so it is not rediscovered as new |

- @fact:SEV-REVIEWER-IS-AN-AGENT **«Reviewer» here means the boss *agent*, not the owner.** That is
  fine for classifying, and **not** fine for two things: **severity moves up
  freely and down only through the owner** (an agent may escalate to P1, never
  downgrade from it), and **every P2/P3 filed during a wave is reported to the
  owner at the time**, not merely written here — otherwise the agent deciding
  «this is a finding, not work» is the agent that wants to move on.
- @fact:SEV-ASSIGNED-BY-REVIEWER **Severity is the reviewer's call, never a worker's.** A cheap model
  calling something critical is noise, and a scale anyone may set is not a
  scale. A worker **reports the observation**; the reviewer classifies it.
- @fact:SEV-WORKER-MAY-INTERRUPT **One exception, running the other way:** a worker that believes it has
  found something genuinely alarming — a credential in source, an auth bypass, a
  gate that is lying — **stops its own packet and says so immediately**. The
  classification stays the reviewer's; the *interruption* needs no permission.
- @fact:SEV-P1-IS-NEVER-FILED **A P1 is never «filed».** That is the whole point of the split: one
  class of finding is not allowed to become a line in a list. If it is here, it
  is here as history, with what was done.
- @fact:SEV-GATE-BLINDNESS-IS-P1 **A gate that reports green because it is not looking is P1**, not P2.
  This programme found that shape three times — a floor gating a frozen slot, a
  parser blind to units the grammar allows, a sync check covering four of seven
  workspaces. Each was green and each was wrong, and a green panel that says
  nothing about coverage is a structural-integrity failure, not a gap.

## What an entry carries {#entries}

An **id**, the **`spec://…#ANCHOR`** it came from where one exists, a one-line
**locator**, a **severity**, a **disposition** (`open` · `planned` · `done` ·
`accepted`), and the **campaign or session** that filed it.

- @fact:ENTRY-CITES-NEVER-RESTATES **Cite the anchor; never restate the fact.** The same law Phase G's
  documentation runs on, for the same reason: a restated fact is a second
  statement of one truth with its own writer, and this programme has found that
  shape seven times.
- @fact:ENTRY-PREFER-GENERATED **Prefer generated over hand-maintained.** Where a finding is already
  carried by a marker — `action="rework"`, `stage="idea"`, an `#[ignore]`d test
  bound by its `verifies` edge — **the marked corpus is the source and this file
  quotes a query, not a copy.** A hand-maintained backlog is a derived value
  with its own writer, which is the defect class this programme keeps paying for.
- @fact:ENTRY-NO-SILENT-DELETION An entry leaves only by changing disposition, never by deletion. A
  backlog that forgets is indistinguishable from one that was never right.
  **SUPERSEDED 2026-08-05** by the owner's ruling recorded at
  [`##B062-WHAT-REPLACES-THE-MARKER`](#b-062): a row now dies with the commit
  that makes it untrue, and the commit is what remembers.
- @fact:ENTRY-THE-FILE-IS-MID-MIGRATION **How to read this file today, and it is
  not yet the way the ruling intends.** The ruling removed statuses from
  planning documents and made deletion the closure — but the rows closed
  BEFORE it were closed the old way, by flipping a field that has since been
  stripped. So the file currently holds live rows and finished history side by
  side **with nothing distinguishing them**, which is a weaker state than
  either the old convention or the new one. Measured 2026-08-05: of 50 rows,
  **13 carry no sign of closure anywhere in their body** and the rest read as
  done — but that count comes from matching closure words in prose, and prose
  is not a status: a row narrowed today reads «построена» about the third of
  it that shipped and is still live. **Treat the count as a lower bound on the
  live set, never as the set itself**, and measure a row against the tree
  before acting on it — which is the standing law anyway
  ([`##WAL-C-MEASURE-WHAT-IS-BUILT-FIRST`](spec/WAL.md)). Completing the
  migration means deleting the pre-ruling closed rows; that is a decision
  about this file's history and it belongs to the owner, not to a regex.

## P1 — handled; kept as record {#p1}

*(empty — an open P1 is not in a file, it is in the owner's hands)*

## P2 — the next wave drains from here {#p2}

### B-068 — вторая половина рулинга D: забор становится телом факта через `@fact/code` {#b-068}

| | |
|---|---|
| @fact:B068-ANCHOR **anchor** | рулинг владельца 2026-08-06 (вариант D): «по умолчанию любой забор является примером, но его должно быть можно пометить как утверждение, и тогда пусть проверяет»; синтаксис — `@fact/ТИП:ЯКОРЬ`, где следующий за фактом объект названного типа прилепляется к его телу |
| @fact:B068-LOCATOR **locator** | `crates/progress-core/src/parse/facts.rs` — `take_fact_id` читает `@fact:` и `##`, но тип не разбирает; тело факта по-прежнему кончается на пустой строке |
| @fact:B068-SEVERITY **severity** | P2 |
| @fact:B068-DISPOSITION **disposition** | `open` — синтаксис согласован с владельцем, реализация не начата |
| @fact:B068-FILED **filed by** | сессия 2026-08-06, после миграции разметки |

- @fact:B068-SUT **Суть, по-простому.** Утверждение внутри огороженного блока сегодня не принадлежит ни одному факту: **372 забора несут ноль фактов**, тогда как все 7255 текстовых блоков несут их все. Значит ложь внутри забора нельзя ни осудить, ни сделать устаревшей — что дважды за неделю и произошло. Пометка делает забор телом факта, и весь существующий механизм (вердикт, устаревание, печать) начинает работать без изобретения нового. @status:spec/done
- @fact:B068-ONLY-ONE-TYPE-EARNS-ITS-PLACE **Реализуется ровно один тип — `code`, и это измерено, а не предположено.** Изображений в корпусе **ноль**; из 908 строк таблиц 891 уже внутри тела факта, из 96 цитат — 84. Заборы — единственный вид блока, выпадающий целиком. `@fact/image` не откладывается, а не имеет предмета. @status:spec/done
- @fact:B068-UNKNOWN-TYPE-IS-AN-ERROR **Неизвестный тип — ошибка разбора, а не молчаливый пропуск.** Иначе грамматика обещает то, чего не умеет, и `@fact/image` завтра пройдёт молча. @status:spec/done
- @fact:B068-MARKING-COSTS-DEBT **Цена, которую надо назвать заранее:** каждый помеченный забор — новый факт без вердикта, то есть судейский долг в момент пометки. Помечать волнами и судить тем же заходом. Кандидатов немного: 19 блоков-диаграмм плюс quick-start блоки. @status:spec/done

### B-067 — версии 38 пакетов не забамплены после смены синтаксиса разметки {#b-067}

| | |
|---|---|
| @fact:B067-ANCHOR **anchor** | закон именования: координата `name@version` не переиспользуется для другого содержимого. Миграция разметки 2026-08-06 изменила содержимое 38 пакетов, не тронув их версии |
| @fact:B067-LOCATOR **locator** | `packages/org.vibevm.*/**` — 38 пакетов с изменённым содержимым при прежних версиях; `vibe.lock` фиксирует их как `source_kind = "local"` |
| @fact:B067-SEVERITY **severity** | P2 — блокирует публикацию, не блокирует работу |
| @fact:B067-DISPOSITION **disposition** | `open` — владелец разрешил бамп 2026-08-06; решено отложить до публикации, см. `@fact:B067-WHY-DEFERRED` |
| @fact:B067-FILED **filed by** | сессия 2026-08-06, миграция разметки |

- @fact:B067-WHY-DEFERRED **Почему отложено, а не сделано.** Всё потребление сегодня локальное (`source_kind = "local"`, 36 пакетов из `file://`), а публикация — за границей этой работы. Координата в реестре не переиспользуется, пока туда ничего не публикуется. Бамп же идёт каскадом: `redbook` пинит каждого из 23 членов точной версией, поэтому это не 38 независимых правок, а связная волна. @status:spec/done
- @fact:B067-WHAT-TRIGGERS-IT **Что делает это срочным:** любая попытка опубликовать любой из 38 пакетов. До того момента долг записан и безвреден. @status:spec/done

### B-066 — сервис индекса не умеет публиковать себя сам: флаг есть, работы нет {#b-066}

| | |
|---|---|
| @fact:B066-ANCHOR **anchor** | рулинг владельца 2026-08-06: «пользователь должен мочь вручную ЗАПУСКАТЬ реиндекс, но проводить процесс републикации сервер должен мочь сам… для этого уже есть флаг, нужно заставить его заработать» |
| @fact:B066-LOCATOR **locator** | `crates/vibe-index/src/cli/serve.rs:39` объявляет `--auto-commit-push`, и `:58` выбрасывает его одной строкой `let _ = args.auto_commit_push; // parked until slice 9.` Формат прямо фиксирует ручной шаг: «Operators commit + push the non-state/ content themselves (v0)» |
| @fact:B066-SEVERITY **severity** | P2, **блокер по слову владельца** — чинить до фазы T |
| @fact:B066-DISPOSITION **disposition** | `open` — заведено 2026-08-06 |
| @fact:B066-FILED **filed by** | замер M-INDEX по вопросам владельца об индексе, 2026-08-06 |

- @fact:B066-SUT **Суть, по-простому.** Индекс — это набор файлов, которые надо положить туда, где их видно по HTTP. Сервис умеет всё остальное: принять запись с авторизацией, атомарно записать файлы, пересчитать манифест, проверить целостность. **Не умеет только донести результат до хостинга** — закоммитить и запушить. Это единственная ручная дыра во всём сценарии публикации.
- @fact:B066-TARGET-IS-A-SETTING **Куда публиковать — настройка пользователя, а не константа (рулинг владельца 2026-08-06).** GitHub/`vibespecs` каноничен для проекта VibeVM как явления; для сервиса индекса это лишь одна из возможных целей, и **приватный репозиторий — законный случай**.
- @fact:B066-PRIVATE-BREAKS-READING-TOO **Приватный индекс ломает не только публикацию, но и чтение — и это в объёме работы.** Потребитель забирает `by-name/<имя>.json` обычным HTTP-запросом **без авторизации** (`crates/vibe-registry/src/index_client.rs:186-218`). В приватном репозитории такой запрос откажет. Владелец 2026-08-06: аутентификация нужна — где ssh-ключи, там по ключам; для остальных случаев что-нибудь простое, токен в настройках, **проверить, нет ли уже готового механизма**. **Умеет ли клиент авторизоваться — НЕ ИЗМЕРЕНО**, и мерить это первым шагом: от ответа зависит, работа это «добавить пуш» или «добавить пуш и аутентифицированное чтение».
- @fact:B066-WHAT-IS-ALREADY-BUILT **Чего строить НЕ надо — измерено 2026-08-06.** Сборка индекса с GitHub построена и покрыта тестами; инкрементальный режим построен (контрольная точка по каждому репозиторию: последний коммит и список тегов, перевалкиваются только изменившиеся); пер-пакетные `add`/`remove` построены. Сервер целиком построен: чтение, запись, bearer-авторизация, ограничение частоты, метрики. **141 тест, ноль отложенных, ноль заглушек кроме трёх названных.**
- @fact:B066-THREE-STUBS **Заглушек в крейте ровно три**, и две из них не про это: сборка **напрямую с GitVerse** (их API не даёт перечислить организацию — принципиальное ограничение, обход через зеркало), **остановка сервера на Windows** (печатает PID для ручного убийства), и эта.

### B-065 — образ организации перечитывается на каждой операции, хотя писатель один {#b-065}

| | |
|---|---|
| @fact:B065-ANCHOR **anchor** | предложение владельца 2026-08-06 (флаг для многоворкерного случая, кэш образа организации по умолчанию) плюс возражение босса, владельцем принятое |
| @fact:B065-LOCATOR **locator** | `crates/vibe-index/src/index/checkpoint.rs` — контрольная точка есть и работает; `crates/vibe-index/src/cli/reindex.rs:140-190` — инкрементальный режим её читает и пишет. Чего нет: образ организации в памяти между операциями и дешёвая проверка свежести вместо полного перечисления |
| @fact:B065-SEVERITY **severity** | P2 |
| @fact:B065-DISPOSITION **disposition** | `open` — форма согласована с владельцем 2026-08-06; имя флага выбрано им же: `--cache-org`, включён по умолчанию |
| @fact:B065-FILED **filed by** | разговор владельца об индексе, 2026-08-06 |

- @fact:B065-SUT **Суть, по-простому.** Чтобы понять, что изменилось, индекс перечисляет организацию. Для локальных клонов это чтение каталога — копейки; для гитхаба — обращение к их API на каждую операцию. Владелец предложил держать образ организации в памяти и перечитывать только по явной команде или на старте.
- @fact:B065-THE-AXIS-IS-NOT-THE-TOPOLOGY **Возражение босса, принятое владельцем: ось не та.** Посылка «между операциями организацию менять некому» **неверна уже сегодня, и не из-за соседних воркеров**: разработчик, публикующий пакет, создаёт репозиторий и пушит тег **напрямую в гит-хост**, минуя сервис индекса. Образ протухает при ОДНОМ воркере так же, как при десяти. Настоящая ось — **«все ли изменения идут через индекс»**, а не «сколько воркеров». Имя `--cluster` обещало бы защиту не от того риска; имя выбрано владельцем — см. [`##B065-THE-FLAG-IS-NAMED`](#b-065).
- @fact:B065-THE-FLAG-IS-NAMED **Имя флага — рулинг владельца 2026-08-06: `--cache-org`, включён по умолчанию.** Слово выбрано; ось, записанная выше, остаётся той же. Заметить стоит одно: **имя называет механизм (кэш образа), а не допущение** — а согласованная ось была именно про допущение. Это не противоречие, но у него есть жёсткое следствие, и оно уже согласовано пунктом *(3)* ниже: раз кэш включён по умолчанию, **дешёвая проверка свежести перестаёт быть улучшением и становится условием корректности умолчания**. Без неё «включён по умолчанию» означало бы ровно то допущение, которое владелец отверг — «организацию помимо меня никто не меняет». С ней умолчание честно: образ кэшируется, но не считается истиной без вопроса хосту.
- @fact:B065-AGREED-SHAPE **Согласованная форма, пять пунктов.** *(1)* Кэш образа — да, перечисление на каждую операцию убрать. *(2)* Флаг — `--cache-org`, включён по умолчанию ([`##B065-THE-FLAG-IS-NAMED`](#b-065)). Допущение, которое умолчанием **не** принимается, — «организация может измениться помимо меня»; оно верно сегодня, и держит его не имя флага, а пункт *(3)*. *(3)* Вместо полного перечисления — **дешёвая условная проверка свежести**, которую гит-хосты умеют и которая не требует обхода. *(4)* `rescan-org` явным глаголом — безоговорочно. *(5)* Настоящий ответ для будущего веб-интерфейса — **вебхуки**: тогда образ авторитетен потому, что его кормят, а не потому, что мы предположили.
- @fact:B065-WEBHOOKS-AND-THE-GUIDE **Вебхуки и гайд (рулинг владельца 2026-08-06).** Запланировать обработку вебхуков; возможно, поверх GitHub Actions. Написать пользовательский гайд по настройке, и **хранить его в наших спецификациях, а не в документации**, меняя вместе со свойствами вебхуков. Причина держать его в спеках: гайд описывает настройку механизма, свойства которого определяем мы; лежащий рядом с контрактом меняется вместе с ним, а лежащий в `docs/` дрейфует — эта сессия намерила два независимых экземпляра ровно такого дрейфа.
- @fact:B065-PER-PACKAGE-PATH-EXISTS **Для веб-интерфейса перечисление вообще не нужно.** `add` вставляет одну запись из манифеста пакета, `remove` удаляет версию или пакет — оба построены и покрыты тестами. «Нажали мышкой — добавился пакет» должно звать их точечно, а не `reindex`.

### B-064 — движок дисциплины знает слово `vibedeps` {#b-064}

| | |
|---|---|
| @fact:B064-ANCHOR **anchor** | замер принадлежности движков по вопросу владельца 2026-08-06; закон переносимости — собственная оговорка движков: «проверяльщик работает на ЛЮБОМ проекте, а не только на том, в котором построен» |
| @fact:B064-LOCATOR **locator** | `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/store.rs:351` и `:434` — литерал `"vibedeps"` в списках пропускаемых каталогов для TypeScript и Go, рядом с универсальными `node_modules`, `target`, `vendor`, `testdata`. Не конфигурируется |
| @fact:B064-SEVERITY **severity** | P2 |
| @fact:B064-DISPOSITION **disposition** | `open` — **владелец 2026-08-06: «нужно чинить, починим ещё до перехода на фазу T»** |
| @fact:B064-FILED **filed by** | замер M-ENGINES, 2026-08-06 |

- @fact:B064-SUT **Суть, по-простому.** Движки дисциплины намеренно ничего не знают о vibevm: **ноль зависимостей на хостовые крейты**, связь строго односторонняя, вся проектная специфика вынесена в файл политики, который пишет потребитель. Это выдержано — кроме одного слова. Имя нашего каталога установленных пакетов вшито в обход как константа.
- @fact:B064-EFFECT **Последствие мизерное, но это протечка.** У чужого проекта каталог с буквальным именем `vibedeps` молча не просканируется. Чинится либо настраиваемым ключом, либо просто удалением — `vendor` в том же списке уже стои́т.
- @fact:B064-COST **Цена.** Правка авторского движка тянет `cargo xtask sync-engines` — перевендоривание в 21 копию. Поэтому не чинилось попутно.
- @fact:B064-THE-ANSWER-IT-CAME-FROM **Ответ, ради которого это мерилось (вопрос владельца).** `conform` и `specmap` — **свойства дисциплины, не внутренности vibevm**. Живут в пакете ядра дисциплины; не зависят от хоста ничем; их собственные комментарии заявляют переносимость; три языковых пакета уже являются посторонними потребителями, каждый вендорит движки и строит поверх свой фронтенд; в дереве лежит настоящий чужой проект `research/rust-demo/`, держащий дисциплинарный пол этим же движком. **Чужие ими пользоваться могут — это замысел, и он выдержан с точностью до этой одной константы.**

### B-063 — валидация разметки не стоит ни в одном гейте, а гайд владельца утверждал, что стоит {#b-063}

**Closed 2026-08-06.** The step is built: `tools/self-check.sh` runs
`vibe progress check --exhaustive` and the panel goes red on an unmarked
fact. Both open questions the row carried are answered in the step's own
comment, where whoever changes it will read them: the zone comes from the
host `progress.toml` (no `--campaign`, which is correct inside a campaign
and outside one), and the user-home hazard was MEASURED rather than
assumed — 169 files snapshotted by content, both forms of the verb run,
nothing moved. Proven not blind on a probe file: the plain form printed
`clean (276 files)`, the exhaustive form found the fact and exited 1.
The guide's line, which said the opposite, is corrected in the same
landing.

This line is a tombstone — process support, not project structure.

### B-062 — четыреста размеченных фактов вне корпуса: маркер стоит, вердикта нет {#b-062}

**Closed.** The ruling and its reasoning live in `ff2079e1`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-059 — исключения конформа сопоставляются не с тем путём, который конформ печатает {#b-059}

**Closed.** The ruling and its reasoning live in `0f12992e`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-058 — производные сущности без гейта свежести: `vibedeps/` и `specmap.toml` {#b-058}

**Closed.** The ruling and its reasoning live in `836cf5a2`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-057 — движок дисциплины не наведён сам на себя: конформ не гоняется по исходникам пакетов {#b-057}

**Closed.** The ruling and its reasoning live in `f7ffb5e5`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-056 — множественное наследование контрактных документов и плагинная форма `#source` {#b-056}

**Closed.** The ruling and its reasoning live in `77224fcf`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-003 — the Go floor gates a directory named `dirty` {#b-003}

**Closed.** The ruling and its reasoning live in `0998f319`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-005 — `mirror --check` tests equality where the flow specifies ancestry {#b-005}

**Closed.** The ruling and its reasoning live in `0998f319`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-006 — the highest-priority boot lane carries four normative snippets twice {#b-006}

**Closed.** The ruling and its reasoning live in `9f79acf1`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-007 — do the specs owe ADRs, and in what form? {#b-007}

| | |
|---|---|
| @fact:B007-ANCHOR **anchor** | the question is about `spec/common/**` and `spec/modules/**` as a genre, not about one anchor. The rule it would satisfy is `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root` |
| @fact:B007-LOCATOR **locator** | 153 sections in `spec/common/` + `spec/modules/**` carry a bolded **Decision** label; 4 carry all four fields |
| @fact:B007-SEVERITY **severity** | P2 |
| @fact:B007-DISPOSITION **disposition** | `open` — **filed at owner request, 2026-07-31**, as a question to answer rather than work to schedule |
| @fact:B007-FILED **filed by** | the packages-actualization campaign, Phase D, wave 7 |

- @fact:B007-THE-QUESTION **The question, in the owner's framing.** Should the specifications
  carry Architecture Decision Records — and if so, **how**: as a section inside
  the PROP/FEAT that owns the decision, as a separate `spec/decisions/` genre, or
  as the four-field block the `decision-records` flow already prescribes? This is
  a **spec-genre design question**, and answering it decides how much work the
  `decision-records` host obligation actually is.
- @fact:B007-WHAT-IS-MEASURED **What is measured, so the question starts from facts.** Sections
  carrying a bolded `Decision` against those carrying all four fields
  (`Decision` · `Why` · `Considered and rejected` · `Revisit when` /
  `When to revisit`): `spec/common` + `spec/modules` **153 → 4**; all of `spec/`
  **157 → 7**; the `fractality` specspace **34 → 14**; this campaign's own
  records **15 → 8**. The practice is adopted at roughly **41 %** in the sibling
  project and **4.6 %** in the host's PROP/FEAT tree. Counted 2026-07-31.
- @fact:B007-RE-MEASURED **Re-counted 2026-08-06, and the host figure has
  tripled.** `spec/common` + `spec/modules` **146 → 12**; all of `spec/`
  **151 → 15**. So the completeness rate in the PROP/FEAT tree moved from
  **4.6 %** to **8.2 %** in five days, and the complete records are no longer
  one file: `PROP-000`, `PROP-018` and `PROP-024` each now carry at least one,
  where only `PROP-000` did. Nobody was asked to do this — the form is being
  adopted where sections are being written, which is evidence for the
  recommendation this row already carries (four-field **inside** the owning
  section, forward-only) and against minting a separate `spec/decisions/`
  genre. The owner's question is unchanged; its starting facts are not.
- @fact:B007-THE-LABEL-HAS-A-PERIOD **Reproducing that count needs one thing
  written down, because getting it wrong costs the whole answer.** The bolded
  label in this tree is overwhelmingly **`**Decision.**` — with a period**:
  122 occurrences against 25 of `**Decision:**`. A pattern written for the
  colon form finds **24** sections where there are 146, i.e. it reports the
  practice as six times rarer than it is, and it reports it *confidently*.
  That mistake was made on the way to the figures above. Same for the other
  three labels; match `[.:]?` on all four.
- @fact:B007-CENSUS-CORRECTION **The sibling-adoption premise is withdrawn — corrected the same day
  by the D10 proposal pass.** The fractality «14 complete records» are, by file,
  **8 files carrying all four fields, all 8 vendored copies of the
  `decision-records` flow's own template, protocol, boot snippet and worked
  examples** (under `*/vibedeps/flow-decision-records/` and
  `flow-comparative-research/`, ×2 vendoring packages) — **0 authored**; the
  specspace's own authored decision blocks are 9, in a three-label dialect,
  none complete. So the honest comparison is «nobody authors the four-field
  form anywhere except this campaign's own plans», and the question is again
  *whether to adopt*, not «why is the PROP tree the outlier». Full measurement
  and the four costed options:
  `campaigns/packages-2026-09/harvest/d10-adr-genre-proposal.md`; the
  campaign's recommendation there is **B + A′** (four-field inside the owning
  section, forward-only, backfill only `spec/common/`, close `spec/decisions/`
  explicitly in the genre table).
- @fact:B007-WHY-IT-IS-A-QUESTION-NOT-A-TASK **Why it is a question and not a task.** «Add the missing fields to
  153 sections» is the wrong shape twice over. Most of those decisions are not
  reopenable, so a revisit condition on them would be ceremony; and the four-field
  block is not obviously the right ADR form for a specification, which already
  states rationale in prose. **What is owed first is the genre decision**, and
  `spec-genres`' own map does not carry an ADR row today.
- @fact:B007-WHAT-IT-UNBLOCKS **What it unblocks.** The largest single host obligation this phase
  surfaced ([`PHASE-D-HOST-OBLIGATIONS.md`](campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md)).
  It cannot be sized, let alone scheduled, until this is answered.

### B-009 — the wind-down's push step contradicts the rollout two host documents standardise {#b-009}

**Closed.** The ruling and its reasoning live in `ae26bcca`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-010 — a check verb that writes, and a `--campaign` flag that selects state rather than scope {#b-010}

**Closed.** The ruling and its reasoning live in `c9cdf39d`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-011 — marker stripping in the boot compiler needs an aliasing design first {#b-011}

**Closed.** The ruling and its reasoning live in `c9cdf39d`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-012 — PROP-014's specified-not-built mechanism set: research feasibility {#b-012}

**Closed.** The ruling and its reasoning live in `eccb1499`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-013 — the specmap schema-bump path is broken before anyone needs it {#b-013}

**Closed.** The ruling and its reasoning live in `dd02f1e2`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-014 — the committed host specmap.json drifts with no freshness gate {#b-014}

**Closed.** The ruling and its reasoning live in `f7ffb5e5`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-015 — программа безопасности runtime-канала: запротоколирована и запаркована до уведомления владельца {#b-015}

| | |
|---|---|
| @fact:B015-ANCHOR **anchor** | тема §2.8.4 PROP-014 (specmap); полное досье — `campaigns/packages-2026-09/harvest/d14-b012-part-A.md`, раздел A5 |
| @fact:B015-LOCATOR **locator** | подписи нет нигде в дереве (единственная crypto-зависимость — sha2 для контент-хэшей); две уже шипящиеся дороги «текст пакета → контекст агента» перечислены ниже |
| @fact:B015-SEVERITY **severity** | P2 |
| @fact:B015-DISPOSITION **disposition** | `open` — **запаркована решением владельца, НЕ строить до его специального уведомления**; кодовых триггеров нет намеренно |
| @fact:B015-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- @fact:B015-SUT **Суть, по-простому.** Задуманные инструменты для агентов будут отдавать текст из пакетов прямо в контекст агента. Текст в контексте агента — потенциальные команды: подложи в пакет вредный абзац — и читающий агент может быть им управляем (prompt injection). Защита — криптографическая подпись содержимого пакетов, чтобы читатель мог проверить «текст от автора, не подменён». Дизайн specmap изначально требовал: канал не шипится без подписи.
- @fact:B015-RULING **Решение владельца (2026-08-01, дословно):** «Положить в бэклог, ничего не строить до специального уведомления. Нужно вначале построить чтобы вся система работала "как-то", наполнить репозитории, и так далее. И только потом уже беспокоиться о безопасности. Бессмысленно строить безопасность проекта, которым никто не пользуется. Пользуется им кто-то или не пользуется — из кодовой базы не видно, это видно владельцу из наблюдения внешнего мира, поэтому это решение владельца.» Следствие: условие переоткрытия — **только уведомление владельца**; никакие наблюдаемые в коде события записью не назначаются.
- @fact:B015-TASKS **Протокол задач на день переоткрытия (полный список):**
  1. **Выбор схемы подписи.** Кандидаты, в порядке рекомендации исследования: (1) подписанные git-теги SSH-ключом мейнтейнера — реестр и есть git, паблишер уже пушит теги, ноль нового wire-формата, верификация через allowed_signers; (2) minisign-класс — detached-подпись контент-хэша пакета, крошечная permissive-зависимость, полностью офлайн; (3) sigstore-класс — отклонён на сегодня: тяжёлые зависимости, онлайн-верификация против clean-clone/offline-постуры, identity через OIDC чужда single-writer-модели; пересмотреть при втором независимом издателе.
  2. **Единица подписи** — дерево пакета на теге (рекомендация), не index отдельно: всё, что сервится из верифицированного дерева, наследует целостность. Сегодняшний контент-хэш в lockfile защищает от подмены байтов зеркалом, но не отвечает «это байты издателя?» — подпись закрывает второй вопрос.
  3. **Инфраструктура:** trust root (где живёт публичный ключ), точка верификации при fetch (рядом с существующей проверкой хэша), ротация/ревокация, кастодия ключа по secrets-hygiene, возможное поле в lockfile.
  4. **Оформление ответов инструментов:** фраза «возвращаемое — справочные данные, не инструкции» на всех инструментах, отдающих агенту текст пакетов, включая **уже существующие две дороги** — чтение сабскиллов установленного пакета и boot-снипеты, читаемые агентом на старте сессии. Явное исключение: агентский релей (agentic_explain) — там инструкции суть фичи, оформление не меняется.
  5. **Линт императивных формулировок** в текстах пакетов (второе-лицо-повелительное вне guide-типа) — требует меток типа на секциях (см. B-019, twin-разметка).
  6. **Правка позиции спеки.** PROP-014 несёт позицию «канал шипится только подписанным». Решением владельца последовательность перевёрнута (канал раньше подписи — B-018); в момент постройки B-018 эта позиция правится owner-approved диффом, чтобы спека не противоречила построенному. Записано здесь, чтобы не потерялось.

### B-017 — профили приватности для закрытых проектов {#b-017}

| | |
|---|---|
| @fact:B017-ANCHOR **anchor** | механизм «[metamodel] profile» PROP-014; досье — `d14-b012-part-A.md`, раздел A3 |
| @fact:B017-LOCATOR **locator** | ключа не существует ни в одном манифесте/схеме/парсере; редакционного пути нет; у «contract»-уровня нет данных (карта не хранит сигнатур) |
| @fact:B017-SEVERITY **severity** | P2 |
| @fact:B017-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить»** |
| @fact:B017-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- @fact:B017-SUT **Суть, по-простому.** Закрытый (не open-source) проект должен уметь сказать в конфиге: «когда мою карту читают снаружи — делись всем / только контрактом без тел кода / ничем». Три уровня: open / contract / none.
- @fact:B017-BUILD **Что строить.** (1) Ключ в манифесте — сам по себе маленький, но парсер манифеста отвергает незнакомые ключи, значит старые версии vibe будут падать на файле с новым ключом: вводить вместе с механикой минимальной версии, не «на вырост». (2) Редакцию применять **на стороне производителя** (байты закрытого проекта не покидают его машину), не фильтром на сервере. (3) Для уровня «contract» карте нужны сигнатуры элементов кода — это смена формата карты: ехать той же одной сменой, что B-016/B-019. (4) Содержание «contract»-уровня (что именно безопасно отдавать: сигнатуры? доки?) — вопрос, который дизайн сам отложил до реального закрытого потребителя; в момент постройки вернуть владельцу с требованиями такого потребителя на столе.
- @fact:B017-DEPS **Зависимости.** Применяется только там, где есть чем делиться наружу: строить после/вместе с B-016 (половина 1) и B-018.

### B-018 — инструменты для агентов (MCP), широкий вариант — высокий приоритет владельца {#b-018}

| | |
|---|---|
| @fact:B018-ANCHOR **anchor** | механизмы «runtime exposure» PROP-014; досье — `d14-b012-part-A.md`, раздел A4 |
| @fact:B018-LOCATOR **locator** | **замер 2026-08-05: три части из четырёх ПОСТРОЕНЫ волной В.** Часть 1 — `vibe explain` есть и в CLI (строит карту СВЕЖЕЙ в памяти, и его собственный `--help` называет канонический вопрос дословно), и в MCP (`ExplainMcpTool`, `crates/vibe-mcp/src/tools.rs:485-494`, зарегистрирован — `lib.rs:486` это утверждает тестом). Часть 3 — фрагменты: `crates/vibe-trace/src/fragment.rs`. Часть 4 — чужие пакеты: `crates/vibe-trace/src/foreign.rs`. **Открыта ровно часть 2 — ПОИСК по карте**, и она отложена самим владельцем 2026-08-04 (развилка №6 не взята) |
| @fact:B018-SEVERITY **severity** | P2 |
| @fact:B018-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить, причем с высоким приоритетом и в широком варианте (вместе с объяснением чужих пакетов)»** |
| @fact:B018-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- @fact:B018-SUT **Суть, по-простому.** Дать AI-агенту спрашивать работающий vibe: «объясни это требование», «что реализует эту команду», «покажи фрагмент», «поищи по карте» — и не только про свой проект, но и про **установленные пакеты**. Это центральная фича всего сюжета «поделиться картой».
- @fact:B018-PARTS **Четыре части, в порядке постройки.**
  1. **Перенос «объясни» в агентский интерфейс vibe** — легко: все швы готовы, в стековых серверах есть три рабочих образца этой же формы. **Цена уточнена измерением (2026-08-04, волна В, владелец: «заноси, делай»):** «все швы готовы» верно про СТЕКИ — их MCP тонко оборачивает CLI-функцию, которая строит карту в памяти и рендерит (`rust-ai-native-cli/src/trace.rs:8`, обёртка `rust-ai-native-mcp/src/tools_discipline.rs:207`). У ХОСТА этой способности нет вовсе: его MCP несёт четыре инструмента (`query_package`, `read_subskill`, `materialise_subskill`, `agentic_explain` — `crates/vibe-mcp/src/tools.rs`) и картой не занимается; движок карты сегодня тянет только `xtask`. То есть работа — не «обернуть готовое», а «дать хосту способность». **Но связывание заново не нужно:** корневой `Cargo.toml:102` уже объявляет `specmap-core` рабочей зависимостью через вендор-копию, так что рельс проложен и новой связи хоста с пакетом не возникает. Форма — по норме поверхностей ([B-047](#b-047)): способность в разделяемом крейте, CLI и MCP — тонкие поверхности над ней, а не две копии. Карта строится СВЕЖЕЙ в памяти, как у стеков («explain answers for the tree as it is, never for a stale committed artefact»); чтение карт УСТАНОВЛЕННЫХ пакетов — это часть 4, не эта.
  2. **Поиск по карте.** Дизайн не определил язык запросов — сначала спроектировать (заготовка v0: точный URI + имя символа + фильтр по типу, жёсткий потолок размера ответа), положить в спеку owner-диффом, потом кодить. **Отложено владельцем 2026-08-04 («положить в бэклог со средним приоритетом») — развилка №6 карты НЕ взята; часть 1 при этом построена и живёт.** Что стоит помнить к моменту возврата: точечный `explain` уже отвечает, значит недостающее — именно ПОИСК, и его форма зависит от того, каких вопросов агенту не хватило на практике. Два рассмотренных варианта записаны, чтобы не изобретать заново: *(i)* три независимых фильтра (точный URI · подстрока имени символа · тип элемента), комбинируемые через И, плюс жёсткий потолок числа результатов — парсить нечего, ломаться нечему, расширяется добавлением полей; *(ii)* то же плюс обход графа (глубина N и «нет ребра такого-то типа»), что сразу отвечает на «какие правила никто не проверяет», но заводит грамматику, которую придётся версионировать. Первый — рекомендация босса на момент отложения.
  3. **Фрагменты по отпечатку** — вместе с B-016 (половина 2).
  4. **Ответы про установленные пакеты** («объяснение чужих пакетов»). Сегодня чужие секции сознательно не попадают в карту проекта — на этом исключении держится воспроизводимость карты (байт-в-байт проверка). Ломать исключение нельзя; строить **вторую, некоммитимую** карту-резолвер, собираемую в момент запроса из установленных пакетов. Кормится из B-016 (половина 1).
- @fact:B018-SECURITY **Безопасность.** Осознанно строится ДО подписи — перепоследовательность зафиксирована решением владельца в [B-015](#b-015): безопасность паркуется до его уведомления. В момент постройки этой записи позиция спеки «канал шипится только подписанным» правится owner-approved диффом (см. B-015, задача 6), чтобы построенное не противоречило написанному.
- @fact:B018-CANONICAL-QUERY **Канонический запрос (владелец, 2026-08-02) — ОТВЕЧЕН.** «Какой тест проверяет это правило спеки?»: агент даёт `spec://…#якорь`, получает verifies-рёбра с file:line. Запись говорила «хостовый vibe-mcp не умеет вовсе» — измерено 2026-08-05: умеет, `ExplainMcpTool` служит инструмент `explain`, и хостовый CLI несёт `vibe explain` собственной способностью, а не делегацией (делегирующий алиас `vibe trace` существует отдельно и ведёт в установленный стек). Приёмочный пример постройки, назначенный этой записью, выполнен.

### B-019 — отпечатки кода + узлы «команда» и «вариант ошибки» в карте {#b-019}

| | |
|---|---|
| @fact:B019-ANCHOR **anchor** | механизм «edge model nodes» PROP-014; досье — `d14-b012-part-B.md`, раздел B2 |
| @fact:B019-LOCATOR **locator** | **замер 2026-08-05: (а) ПОСТРОЕНА** — `CodeItem` несёт `fingerprint: Option<Box<String>>` в форме `tok1:<sha256>` по ПОТОКУ ТОКЕНОВ (док-комментарии считаются кодом, обычные комментарии и пробелы — нет), и в закоммиченной карте он стоит на **915 элементах из 915**; узла «команда» по-прежнему нет (`item_kind` в живой карте: mod/fn/enum/struct/impl/trait, и ничего больше); извлечение «вариантов ошибок» существует в соседней подсистеме conform, не в карте |
| @fact:B019-SEVERITY **severity** | P2 |
| @fact:B019-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить. Это должна быть алгоритмическая фича, без использования LLM. Все части — а, б, в»** |
| @fact:B019-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- @fact:B019-SUT **Суть, по-простому.** Три доделки самой карты, все чисто алгоритмические (владелец: без LLM). **(а)** Отпечаток (хэш) на каждом элементе кода — чтобы карта замечала «код под этим требованием изменился, пересмотри связь»; сегодня она слепа к изменениям кода. **(б)** Узел «команда» — чтобы `vibe install` был сущностью карты, а не только функцией: ответ «что реализует vibe install» становится возможен напрямую. **(в)** Узел «вариант ошибки» — чтобы каждая ошибка была узлом карты и вела к своему требованию.
- @fact:B019-A **(а) — ПОСТРОЕНА, и развилка выбрана в пользу рекомендации.** Хэшируется **поток токенов**, не текст: `tok1:<sha256>`, форматонезависимо — то есть прогон `cargo fmt` отпечаток не двигает, а правка док-комментария двигает (док считается кодом, обычный комментарий нет). Развилка, которую эта запись оставляла владельцу, тем самым закрыта выбором, а не решением на бумаге. Остаётся парная половина со стороны СПЕКИ — метки-редакции на секциях (~80 секций, на которые ссылаются сообщения об ошибках, + правило «новые секции сразу с меткой»): она к отпечаткам кода не сводится и живёт своим сроком.
- @fact:B019-B **(б) — с нуля.** В дереве нет ни определения «команды», ни экстрактора, ни потребителя. Определить, что считается командой (поверхность CLI-подкоманд), написать экстрактор, добавить тип узла (та же одна смена формата), научить «объясни» принимать команду как цель.
- @fact:B019-B-DESIGN **Дизайн (б) написан 2026-08-06: [`spec/design/command-nodes.md`](spec/design/command-nodes.md)**, на замере M-B019B (архив `cache/agents/sorted/M-B019B/`). Два измерения срезали цену. **Первое:** `item_kind` объявлен СВОБОДНОЙ строкой в схеме (`specmap.jtd.json:92-94`), тогда как все четыре его соседа несут `enum`, и **ни одно место продакшн-кода не сопоставляется по нему** — значит новый вид есть новое ЗНАЧЕНИЕ открытого поля, а не смена формата, какой была (а). **Второе:** у `explain` нет замкнутого множества видов цели вообще — `explain.rs:199-204` проверяет один строковый префикс, а `explain_symbol` матчит `symbol` и вида не спрашивает; поэтому узел с символом-путём вызова (`vibe install`) отвечается существующей машинерией БЕЗ единой правки `explain`. Опознание — по клэповскому `derive(Subcommand)`, а не по авторской метке: метка, без которой подкоманду можно добавить, есть норма без чекера. Сканер сегодня к `derive` слеп (у его читателя атрибутов ровно две руки), и это единственная добавляемая способность. Строка «(б) — с нуля» тем самым уточняется: с нуля здесь экстрактор, а не формат и не потребитель.
- @fact:B019-B-SLICE-1-LANDED **Срез 1 пункта (б) ПОСТРОЕН 2026-08-06.** В карте **56 узлов вида `command`** — `vibe` 29, `vibe-index` 14, `xtask` 13, — у каждого символом стоит путь вызова (`vibe install`), спан варианта и отпечаток по потоку токенов. Опознание — по клэповскому `derive(Subcommand)`, обе записи; сканер научился читать `derive`, чего не умел. Формат не тронут: вид элемента — открытая строка, и `explain` отвечает на `vibe install` существующим путём по символу, без единой правки. Осталось: **срез 2** (вложенность) и **срез 3** (приёмка `explain` тестом) — оба в [`command-nodes.md`](spec/design/command-nodes.md) `#cut`. Строка живёт, пока не закрыты они и пункт (в).
- @fact:B019-B-WHAT-THE-BUILD-COST **Что стройка стоила и чему научила — число приёмки поймало дефект, которого не увидело ни одно ревью.** Оба крейта, `vibe-cli` и `vibe-index`, объявляют `pub enum Command`; джойн искал перечисление по имени типа по всему воркспейсу, и `find` отдал обоим корням одно и то же — карта утверждала `vibe-index agentic` и `vibe-index term`, двадцать девять несуществующих команд. Джойн стал крейт-локальным, тест на два одноимённых перечисления в разных крейтах доказан падающим до правки. **Само число ошибалось трижды** (29 → 43 → 71 → 56), и каждый раз причиной был периметр измерения, а не поиск: цензус мерил поверхность одного бинаря, воркер — каталог `crates/`, карта — воркспейс, где нашёлся третий бинарь `xtask`. Одно чтение между ними дало **0**, и это была протухшая сборка, а не логика (`cargo clean -p core-ai-native-specmap` вернул 56).
- @fact:B019-A-COUNT-MOVED **Число в локаторе (а) сдвинулось, и не от регресса.** Запись говорит «915 из 915»; замер 2026-08-06 — **916 отпечатков из 932 элементов**. Выросло и то и другое: сканер JTD (B-060, построен 2026-08-05) добавил в карту виды `schema` (7) и `schema-def` (9), а отпечатка они не несут, потому что отпечаток считается по потоку токенов Rust. То есть «916 из 932» — это «все, у кого он определён», а не «шестнадцать потеряли». Числу в локаторе верить нельзя; мерить по дереву.
- @fact:B019-V **(в) — что имеется в виду, и вопрос границы систем (решить ДО реализации — требование владельца).** В кодовой базе два независимых движка: **conform** (гейт качества кода: прогоняет правила, находит нарушения) и **specmap** (карта связей «код ↔ спека»). Данные о «вариантах ошибок» — какие enum-варианты с какими текстами ошибок существуют и на какие требования ссылаются — **уже извлекаются конформом** для двух его правил. Карта этих данных не видит: это два разных графа двух разных подсистем. Вопрос: чьей частью становится узел «вариант ошибки»? Три варианта: **(1)** specmap извлекает сам — дублирование экстракции в двух движках, две правды об одном; **(2)** specmap читает данные conform'а — новая зависимость между сознательно разделёнными движками; **(3)** не сливать данные вовсе, объединять на этапе запроса — инструмент B-018 показывает и карту, и находки conform'а рядом. Склонность исследования — (3) при наличии B-018, иначе (1) с выносом общей экстракции в разделяемую библиотечку; окончательное решение — первый шаг реализации этой части.

### B-020 — объяснения человеческим языком через внешние LLM {#b-020}

| | |
|---|---|
| @fact:B020-ANCHOR **anchor** | механизм «LLM as renderer» PROP-014; досье — `d14-b012-part-B.md`, раздел B3 |
| @fact:B020-LOCATOR **locator** | команда «объясни» отвечает детерминированным шаблоном; слот под второго производителя текста в кэше готов; LLM-клиента у vibe нет (крейт-заглушка) |
| @fact:B020-SEVERITY **severity** | P2 |
| @fact:B020-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить. Я думаю построить лайтовый клиент для внешних нелокальных LLM, который будет через них строить такие объяснения. Возможно это будет fractality, с этим нужно разобраться позднее»** |
| @fact:B020-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- @fact:B020-SUT **Суть, по-простому.** Команда «объясни» сегодня отвечает сухим шаблоном («такая-то функция реализует такой-то пункт»). Фича: опционально та же информация пересказывается внешней LLM человеческой прозой — «эта команда устроена так потому-то, вот решения, вот известные отступления».
- @fact:B020-DIRECTION **Направление владельца.** Лайтовый клиент к внешним нелокальным LLM (не встроенный движок); возможный носитель — fractality (воркер дергает внешнюю модель); разобраться позднее, в момент постройки.
- @fact:B020-BUILD **Что строить и что помнить.** (1) Сначала — текст в данных: сегодня ответ «объясни» несёт только имена и пути, без текста спеки и без документации кода; LLM было бы не из чего писать. Зависимость: включить текст документации и секций в ответ (кандидат ближайшего рабочего среза, дёшево, формат карты не меняется). (2) Второй «производитель текста» встаёт в готовый слот кэша; в ключ кэша добавляется идентификатор модели. (3) Шаблонный режим остаётся навсегда — инструмент обязан быть полноценным без LLM (инвариант дизайна). (4) Проза — только презентационный слой поверх детерминированных данных; сами данные карты LLM не трогает. (5) Ключи/креды внешних LLM — по secrets-hygiene.

### B-022 — исследование: механизмы кэша объяснений (LEDGER-INTENT), можно ли реализовать {#b-022}

**Closed.** The ruling and its reasoning live in `0c9c97dc`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-023 — исследование: синтаксический уровень для JS/TS и Python-фронтенд {#b-023}

**Closed.** The ruling and its reasoning live in `0c9c97dc`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-024 — исследование: не вытесняют ли маркеры @stage/state lifecycle-статусы specmap {#b-024}

| | |
|---|---|
| @fact:B024-ANCHOR **anchor** | вопрос владельца 2026-08-01 к тексту EDGE-MODEL-EDGES (партия 1d): «не устарела ли вообще вся эта система с появлением синтаксиса вида @status:doc/done? Там же тоже есть свой tombstone» |
| @fact:B024-LOCATOR **locator** | две параллельные системы: kind-line-статусы specmap (`planned`/`disputed`; `ratified` — отсутствие, `retired` — tombstone; парсер готов, носителей 0 из 5 266) и хостовые маркеры PROP-043 `@stage/state` (весь корпус размечен; `void` — их tombstone) |
| @fact:B024-SEVERITY **severity** | P2 |
| @fact:B024-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01 (вторая сессия): «предлагаю запланировать в бэклог свести стадии жизненного цикла в specmap к аналогичным в progress»** — направление выбрано: сводим словарь specmap к словарю progress (derive, not declare); исследовательская часть сужается до механики (как выводить; что делать с `disputed`, у которого аналога нет) |
| @fact:B024-FILED **filed by** | вопрос владельца 2026-08-01, зафайлен как исследование; повышен до `planned` его же решением в тот же день |

- @fact:B024-SUT **Суть, по-простому.** В проекте два способа сказать «в каком состоянии кусок спеки». Маркеры `@stage/state` — прогресс каждого факта (насколько сделано: spec/impl/doc × done/work/…), живут на всём корпусе, `void` — их могильный камень. Статусы specmap — контрактное состояние секции для машины трассировки (`planned` — задумано, `disputed` — оспорено парой, `retired` — второй могильный камень), задуманы, чтобы управлять рёбрами графа (заморозка связей в спорные секции, отдельный учёт planned в покрытии — механики не построены), и не носятся ни одной секцией. **Два tombstone на одно понятие — реальная дупликация**; `planned` перекрывается со стадиями маркеров; уникален только `disputed` (пара конфликтующих секций аналога в маркерах не имеет).
- @fact:B024-QUESTION **Вопрос исследования.** Может ли машина трассировки **читать хостовые маркеры** вместо собственной параллельной системы (derive, not declare): `void` ⇒ retired, стадия/state ⇒ planned-эквивалент, а `disputed` — единственное, что останется собственным словарём specmap? Если да — kind-line-статусы сокращаются до `disputed`, и разметка B-019(а)-twin (метки ~80 секций) дешевеет. Если нет — записать, почему двум системам жить (разные предметы: прогресс факта ≠ контрактный статус юнита), и развести их словари явно.

### B-025 — находки гейта: помечать признанные отступления, а не гасить {#b-025}

**Closed.** The ruling and its reasoning live in `245aedd6`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-026 — ингест SARIF: диагнозы чужих линтеров становятся фактами гейта {#b-026}

**Closed.** The ruling and its reasoning live in `245aedd6`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-027 — аудит маркеров у «Specified, not built»: смысл против буквы {#b-027}

**Closed.** The ruling and its reasoning live in `245aedd6`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-028 — грамматика spec://-адресов: пакет публикует подмножество того, что реализует хост {#b-028}

**Closed.** The ruling and its reasoning live in `93d92ec9`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-029 — ключ гейта: нейтральное/пер-языковое имя вместо растового на всех {#b-029}

**Closed.** The ruling and its reasoning live in `1ef63a37`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-030 — проверка «ассерция соответствия присутствует»: построить для Go, обследовать Rust/TS {#b-030}

**Closed.** The ruling and its reasoning live in `1ef63a37`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-031 — корень vibevm становится полноценным пакетом: fully-qualified адресация без исключений {#b-031}

**Closed.** The ruling and its reasoning live in `1ef63a37`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-033 — Go: выделенное правило «ошибка шва цитирует REQ» по образцу растовых {#b-033}

**Closed.** The ruling and its reasoning live in `f882cd46`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-034 — инвариант «каждая единица кода под гейтом или исключена» для Go и TypeScript {#b-034}

**Closed.** The ruling and its reasoning live in `f882cd46`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-035 — паритет-аудит языковых стеков: TS и Go не слабее Rust, или причина записана {#b-035}

**Closed.** The ruling and its reasoning live in `f882cd46`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-036 — conform-правило «инварианты не тонут в середине файла» {#b-036}

**Closed.** The ruling and its reasoning live in `1f048058`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-038 — pending-карточки правил обретают карточки и чекеры: R-060 и closed-vocabulary-naming {#b-038}

**Closed.** The ruling and its reasoning live in `1f048058`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-039 — смонтировать R-001 (FlagSites) на TypeScript-гейт; обследовать Go {#b-039}

**Closed.** The ruling and its reasoning live in `1f048058`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-040 — рефакторинг-обзор собственных швов: полный scaffold-B на нашем коде {#b-040}

**Closed.** The ruling and its reasoning live in `1f048058`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-041 — карта развития инструментария: от реестра дыр к системе {#b-041}

**Closed.** The ruling and its reasoning live in `1f048058`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-046 — мультиязычная композиция: агент собирает несколько AI-Native языков в одном проекте {#b-046}

| | |
|---|---|
| @fact:B046-ANCHOR **anchor** | директива владельца 2026-08-02 (по истории OracleRegistry, дословно): «должен быть понятный способ которым AI агент должен собрать процесс использования нескольких языков в одном проекте одновременно. Может это отдельный MCP+CLI, может еще что-то… Сделать общий реестр может быть стоит, но этот реестр должен работать на основе autodiscovery подключенных AI-Native языков, не нарушая их автономность когда они установлены по-отдельности» |
| @fact:B046-LOCATOR **locator** | сегодня агент мультиязычного проекта подключает N серверов руками (по одному на язык); суверенитет держится (MCP-SOVEREIGNTY, PROP-027), композиционного слоя нет; рельсы autodiscovery уже существуют — lockfile знает установленные пакеты, `[[mcp_server]]`-таблицы объявляют серверы (PROP-027), `[[binary]]` — CLI-бинари (PROP-025), `vibe mcp` их уже читает |
| @fact:B046-SEVERITY **severity** | P2 |
| @fact:B046-DISPOSITION **disposition** | `planned` — **«Такое стоит сразу класть в бэклог»** (владелец, 2026-08-02) |
| @fact:B046-FILED **filed by** | рулинг истории OracleRegistry/F-210, 2026-08-02 |

- @fact:B046-SUT **Суть, по-простому.** Суверенитет языков сохраняется (каждый стек автономен, установлен по-отдельности — работает сам по себе), но над ним появляется понятный способ собрать мультиязычный проект: агент одним жестом узнаёт, какие AI-Native языки подключены, и получает их поверхности. Это НЕ возврат удалённого OracleRegistry (тот был прибит к одной топологии в хосте) — это композиция поверх суверенных.
- @fact:B046-OPTIONS **Варианты (решить при проектировании, владельцу):** **(1)** отдельный тонкий MCP+CLI-агрегатор («ai-native-workspace»): autodiscovery по lockfile → релей к пер-языковым серверам/бинарям; одна точка подключения для агента, ноль собственной логики; **(2)** конвенция discovery-манифеста без нового сервера: `vibe` отдаёт агенту ростер подключённых стеков и их поверхностей (расширение `vibe mcp` / B-018-инструментов), агент подключает языки сам; **(3)** гибрид: (2) как основа + (1) как опциональная обёртка для хостов, где число серверов ограничено. Заготовка рекомендации: начать с (2) — рельсы готовы (lockfile + `[[mcp_server]]` + `[[binary]]`), автономность не тронута по построению; (1) добавлять по реальной боли одного-подключения.
- @fact:B046-AUTONOMY **Закон автономности (владелец, дословно в anchor):** реестр/агрегатор работает ТОЛЬКО autodiscovery-путём по установленному; стек, поставленный в одиночку, не знает об агрегаторе и не зависит от него; отсутствие агрегатора ничего не ломает.
- @fact:B046-RAILS-CONFIRMED **Утверждение о готовых рельсах перемерено 2026-08-06 и держится — но не целиком.** Обход установленного по lockfile действительно построен и работает в обе стороны: `crates/vibe-workspace/src/bins.rs:253` `collect_mcp_servers` и `:184` `collect_binaries` перебирают `lockfile.packages`, читают манифест слота и собирают объявленные серверы и бинари; их читают `vibe mcp status`/`install` (`commands/mcp/mod.rs:146`, `mcp/install.rs:418`) и `vibe bin` (`commands/bin.rs:16,45,66,82`). Реальных объявлений: `[[mcp_server]]` — **4** канонических манифеста, `[[binary]]` — **19** заголовков в **8**. Ростер-поведение тоже есть, но узкое: `vibe mcp status` перечисляет установленные MCP-серверы с состоянием артефакта — и это CLI-отчёт человеку, не тул для агента, и он про *серверы*, а не про *языки*.
- @fact:B046-NO-TYPED-MARKER **Чего в рельсах НЕТ — и это несущий пробел, которого запись не видела.** Отличить «AI-Native языковой стек» от любого другого пакета механически сегодня **нечем**. Поле `kind` — закрытое перечисление `flow | feat | stack | tool | mcp` (`crates/vibe-core/src/package_ref.rs:45-49`), и `kind = "stack"` несут **девять** канонических манифестов, из которых языковых стеков — три: рядом стоят семейные агрегаторы `rust-ai-native` / `typescript-ai-native` / `go-ai-native` (PROP-028, `kind = "stack"` ровно так же) и тест-фикстуры. Единственное сегодняшнее отличие — суффикс имени `-lang`, то есть **текст, а не объявление**. Ближайший механический признак — что у агрегатора нет `[boot_snippet]`, а у языкового стека есть с `category = "stack"`, — это вывод по косвенному, а не декларация, и норма без чекера дрейфует ровно так же ([`##WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS`](spec/WAL.md)). **Поэтому у развилки появился нулевой вопрос, который решается ДО выбора из трёх вариантов: чем пакет объявляет, что он — AI-Native язык.** Любой из трёх вариантов без ответа на него будет угадывать по имени.
- @fact:B046-AUTONOMY-HOLDS **Закон автономности держится механически (проверено 2026-08-06).** Единственная зависимость каждого из трёх языковых стеков на `org.vibevm.*` — ровно `core-ai-native` (`rust-ai-native-lang/v0.7.0/vibe.toml:16` и близнецы); секций `[recommends]`/`[suggests]` нет ни у одного; на уровне `Cargo.toml` все три вендорят одни и те же четыре движковых крейта и ничего общего сверх них. Предпосылка варианта (2) «автономность не тронута по построению» — верна по факту, а не по намерению.
- @fact:B046-PRICE-OF-A-TOOL **Цена «добавить один MCP-тул» измерена по образцу: ~44 строки в одном файле, ноль правок диспетчера.** Самый простой существующий тул `explain` — `crates/vibe-mcp/src/tools.rs:487-529` (43 строки вместе с дескриптором; тело `run()` — 14 строк делегирования) плюс одна строка регистрации в `default_tools()` (`tools.rs:50`); роутинг по имени через `BTreeMap`, и док файла фиксирует прямо: «A new tool is a new cell added here, not an edit to the dispatcher» (`tools.rs:42-43`). **Оговорка, снимающая обманчивость этой дешевизны:** дёшев только каркас. Составного запроса «перечисли установленные языки и их поверхности» в коде нет — его надо писать, и он упирается в `##B046-NO-TYPED-MARKER`. Заодно измерено, что тул такого рода уже был запланирован и не построен: `tools.rs:8` несёт комментарий «Subsequent slices add `list_capabilities` … once `vibe-llm` is real».
- @fact:B046-RELATED **Смежность.** B-018 (агентские инструменты vibe — ростер-половина варианта (2) ложится туда естественно); B-047 (норма поверхностей — агрегатор обязан быть тонкой поверхностью над разделяемой логикой); PROP-026-грамматика (единая грамматика инструментов — то, что делает композицию дешёвой).

### B-047 — норма поверхностей: логика в разделяемом крейте, CLI и MCP — тонкие поверхности над ней {#b-047}

| | |
|---|---|
| @fact:B047-ANCHOR **anchor** | критика владельца 2026-08-02 (дословно): «Нужен какой-то код, доступный из разных поверхностей. MCP — одна поверхность, инструменты командной строки — другая. У нас постоянно в коде недостаточный уровень абстракции, всё прибивается гвоздями к конкретной реализации… логика, общая между MCP и CLI должна быть сформулирована абстрактно в какой-то библиотеке или крейте, чтобы ее переиспользовали разные поверхности» |
| @fact:B047-LOCATOR **locator** | стеки норму уже держат: логика в bridge/engine-крейтах (`rust-ai-native-tcg-bridge` и близнецы, conform/specmap-движки), CLI-бинари — первая поверхность, MCP-серверы — вторая (описания инструментов буквально «= `rust-ai-native init`»); проверить и довести хостовую сторону: vibe-mcp (четыре продуктовых тула) против CLI-паритета, B-018-инструменты — с рождения двумя поверхностями |
| @fact:B047-SEVERITY **severity** | P2 |
| @fact:B047-DISPOSITION **disposition** | `planned` — решение владельца 2026-08-02 (та же директива, что B-046) |
| @fact:B047-FILED **filed by** | рулинг истории OracleRegistry/F-210, 2026-08-02 |

- @fact:B047-SUT **Суть, по-простому.** Стоячая норма: пользовательская способность живёт в разделяемой библиотеке; CLI и MCP — тонкие поверхности над ней, ни одна не является «основой»; новая способность рождается минимум с двумя поверхностями или с записанной причиной, почему одной хватит. Работа записи: (1) аудит «где прибито гвоздями» — обход поверхностей хоста и стеков с таблицей «способность → логика-крейт → CLI → MCP → дыра»; (2) доводка найденных дыр (первый известный кандидат: карт-инструменты vibe — CLI-половина есть, MCP-половина — B-018); (3) поднять норму в спеку дисциплины owner-approved диффом (дом — решить при аудите; кандидат — ENGINE-CONFORM/GUIDE-семья рядом с четырёхслойной моделью SPEC/ENGINE/DRIVER/DEPLOYMENT, чьим уточнением норма и является: DRIVER — это не один бинарь, а набор тонких поверхностей над ENGINE).
- @fact:B047-CENSUS-BUILT **Пункт (1) закрыт 2026-08-06: цензус построен и лежит в [`campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md`](campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md).** Двадцать девять команд верхнего уровня, у девятнадцати логика в крейте, у **десяти** её дом — сам `vibe-cli` (`init`, `list`, `aiui`, `term`, `frame`, `show`, `tree`, `self`, `vars`, `version`); из пяти MCP-тулов норму держат **два** (`explain`, `agentic_explain` — один крейт, одна функция на обе поверхности), у **двух** CLI-близнеца нет вовсе (`read_subskill`, `materialise_subskill`), и **один** (`query_package`) делит с `vibe list` тип данных, но не логику показа.
- @fact:B047-THE-HOLE-IS-A-PATTERN **Дыра `list ↔ query_package` оказалась не парой, а узором (замер 2026-08-06).** Один и тот же `lockfile.packages`/`LockedPackage` ради ПОКАЗА пакета обходят как минимум шесть команд `vibe-cli` помимо этих двух — `outdated`, `show effective`, `show purls`, `show features`, `show subskills`, `tree`, — и каждая строит свой пер-пакетный вывод руками. Трёхполевая проекция сабскилла `{path, delivery, describes}` написана в дереве **трижды**: `commands/list.rs:90-98`, `vibe-mcp/src/tools.rs:103-115` и `commands/show/subskills.rs:52-69`.
- @fact:B047-THE-EXPENSIVE-ROW **Самая дорогая строка расхождения — не форма, а ЗНАЧЕНИЕ.** Из четырнадцати общих полей одиннадцать сериализуются по-разному при одинаковом имени (CLI через `skip_serializing_if` — поле пропадает; MCP через `json!` — всегда `null`/`[]`), и это стиль. Но `files_written` считается по-разному по существу: `commands/list.rs:86` отдаёт `to_string_lossy()` без нормализации, `vibe-mcp/src/tools.rs:119` — `to_string_lossy().replace('\\', "/")`. **На Windows две поверхности одной способности печатают разные пути.** Правка меняет опубликованный JSON-вывод CLI, поэтому это не попутный фикс, а решение со своей ценой.
- @fact:B047-THE-CANDIDATE-HOME-DOES-NOT-EXIST **Кандидат на дом нормы из пункта (3) опровергнут измерением, и это меняет цену пункта.** Запись предлагала посадить норму «рядом с четырёхслойной моделью SPEC/ENGINE/DRIVER/DEPLOYMENT» в семье `ENGINE-CONFORM`/`GUIDE-*`. Такой модели **в пакете дисциплины нет**: слов `DRIVER` и `DEPLOYMENT` во всём `packages/org.vibevm.ai-native/` — ноль вхождений. Ближайшее, что там есть, — двухчастное расщепление «core ships prompt content and neutral engine crates / the runnable half ships in each stack» (`core-ai-native/v0.8.0/README.md:19,21-24`), без слоёв и без имён.
- @fact:B047-WHERE-THE-MODEL-ACTUALLY-LIVES **Где четырёхслойная модель живёт на самом деле — и почему это отдельная находка.** Её единственная сжатая формулировка стоит в `spec/WAL.md` (`##WAL-C-PERIMETER`): «SPEC в `core-ai-native`, ENGINE в его крейтах, DRIVER в CLI стека, DEPLOYMENT у потребителя». Кроме неё — два кампанийных документа (`PHASE-C-BATCH-PLAN.md §4.5`, `PHASE-C-RESUME.md`), план кампании и около десяти harvest-разборов, пересказывающих её прозой. **Ни один из этих домов не долговечен и ни один не проверяется:** WAL переписывается целиком каждым wind-down и выведен из корпуса кампании конструктивно (`progress.toml`), `campaigns/**` исключён структурно, harvest — свидетельства, а не контракты. То есть несущий архитектурный закон проекта пересказан не менее двенадцати раз и не имеет ни одного верифицируемого дома — ровно тот класс «одна истина, много домов», который эта программа и убирает. Пункт (3) поэтому распадается надвое: **сперва дать модели дом, потом уточнять её нормой о поверхностях.** Дом — по-прежнему решение владельца, но список кандидатов теперь другой: `00-MANIFESTO.md` (языко-независимое ядро, где уже живёт лексика слоёв), `mechanisms/MCP-CORE-v0.1.md` (механизм одной из двух поверхностей), либо новый механизм-документ.
- @fact:B047-RELATED **Смежность.** B-018 (первый потребитель нормы), B-046 (агрегатор обязан ей подчиняться), B-035 (паритет-аудит — та же таблично-обходная механика).

### B-048 — TS-floor: prettier/eslint-шаги обходят fixtures пакета (двойник B-003) {#b-048}

**Closed.** The ruling and its reasoning live in `68106a1c`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-049 — Rust-floor обретает floor_disable (близнец Go/TS-механизма) {#b-049}

**Closed.** The ruling and its reasoning live in `e4314e83`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-050 — типо-аварный вехикл кастомных линтов для Rust: dylint-библиотека и её toolchain {#b-050}

| | |
|---|---|
| @fact:B050-ANCHOR **anchor** | `GUIDE-AI-NATIVE-RUST.md` `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` (:72) — клауза «custom clippy lints name the rule and the remedy»; та половина третьего канала, которую батч 3 не построил |
| @fact:B050-LOCATOR **locator** | цензус `harvest/e13-r2-custom-lints-census.md` §Q3/Q8: `dylint`, `declare_lint!`, `LateLintPass`, `rustc_private` — 0 в исходниках; `rust-toolchain.toml:2` пинит `channel = "stable"` и это ЕДИНСТВЕННЫЙ toolchain-файл дерева; на машине владельца (замер 2026-08-04) нет ни nightly, ни `cargo-dylint` |
| @fact:B050-SEVERITY **severity** | P3 |
| @fact:B050-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-04: «добавить в BACKLOG.md с низким приоритетом»**, то есть dylint сейчас НЕ строим и обещание гайда НЕ снимаем; вопрос про nightly возвращается вместе с этой строкой |
| @fact:B050-FILED **filed by** | босс-дизайн батча 3 волны Б, 2026-08-04 (`spec/design/new-rule-classes.md` §3) |

- @fact:B050-SUT **Суть, по-простому.** Гайд обещает три канала структурной диагностики; два построены (ошибки цитируют REQ; отчёты в SARIF), третий — свои линты — построен батчем 3 только для TypeScript (плагин `@typescript-eslint`, вехикл был уже в дереве). У Rust вехикл ровно один — `dylint`, чья библиотека линкуется с потрохами компилятора через `#![feature(rustc_private)]` и не собирается на stable, а stable мы пиним сознательно. Что при этом НЕ является пробелом: грамматика `violates REQ …; fix surface: …` у Rust соблюдается уже сегодня — единственный рендерер `req_message` и 19 мест его вызова в conform-движке, то есть слой «своих проверок с правильной формой сообщения» существует. Пробел ровно один и он назван: у Rust нет вехикла, ВИДЯЩЕГО ТИПЫ (conform читает синтаксис). Стройка, когда до неё дойдут руки: крейт линт-библиотеки с СОБСТВЕННЫМ nightly-пином внутри (рабочее пространство остаётся на stable) + шаг флора `cargo dylint` с рецептом установки при отсутствии инструмента — та же форма, в какой у Go живёт `staticcheck`, а у TS `eslint`.
- @fact:B050-RELATED **Смежность.** Go-половина того же канала: гайд Go не называет вехикла вообще («custom checks emit the same grammar»), а естественный носитель — свой `analysis.Analyzer` по образцу уже вызываемых флором `staticcheck`/`exhaustive`; едет этой же строкой. Встречная половина контура — B-026 (SARIF-ингест чужих линтеров). Паритет-ось — B-035: пробел записан причиной и маршрутом, не молчанием (`##PARITY-GAP-IS-NEVER-SILENT`).

### B-043 — генератор реестра может выдать один id двум кластерам {#b-043}

**Closed.** The ruling and its reasoning live in `e4314e83`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-055 — вторая директива `#source` в документе проглатывается молча {#b-055}

**Closed.** The ruling and its reasoning live in `bc88e530`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-054 — файл тестов прогресс-команды стоит в тринадцати строках от бюджета {#b-054}

| | |
|---|---|
| @fact:B054-ANCHOR **anchor** | нет — найдено попутно при посадке B-010; ближайший закон — правило гейта `file-length` (`discipline://rust-ai-native-lang/guide#surface-form`, бюджет 600 строк) |
| @fact:B054-LOCATOR **locator** | `crates/vibe-cli/src/commands/progress/tests.rs` — **587 строк** после `cargo fmt` на HEAD этой записи (было 574 до B-010, который добавил тест разбора нового флага) |
| @fact:B054-SEVERITY **severity** | P3 |
| @fact:B054-DISPOSITION **disposition** | `accepted` — не нарушение и не долг: гейт зелёный. Записано, чтобы следующая правка этого файла не превратилась в внезапно-красную панель у того, кто её сделает |
| @fact:B054-FILED **filed by** | волна Г, посадка B-010, 2026-08-04 |

- @fact:B054-SUT **Суть.** Файл в тринадцати строках от блокирующего бюджета. Любая следующая правка — добавление одного теста — уронит панель у автора правки, и он потратит время на выяснение, при чём тут длина файла. Разрез по швам ответственности стоит дёшево сегодня и дорого в момент срабатывания.
- @fact:B054-WHY-NOT-NOW **Почему не сейчас.** Разрез файла тестов — не задача той работы, которая его обнаружила; делать его попутно значит смешать в одной посадке фикс поведения и рефакторинг чужого файла. Ждёт первой же работы, которая тронет этот файл по существу.
- @fact:B054-THE-CLASS **Класс, а не случай.** Это второй раз за одну сессию: `xtask/src/mirror.rs` и `go-ai-native-cli/src/floor.rs` оба перевалили бюджет ПОСЛЕ форматирования, хотя воркеры мерили до него. Мерить длину надо после `cargo fmt`, и мерит её босс — у воркера этого шага нет.


### B-042 — тестовая кодовая база для TCG-замеров: далёкое будущее, сейчас не строим {#b-042}

| | |
|---|---|
| @fact:B042-ANCHOR **anchor** | `TCG-ORACLE-GO-v0.1.md` `##QUANTITIES-ARE-CAMPAIGN-MEASURED` — аннотация 2026-08-02 несёт это решение прямо в тексте; тот же вопрос ждёт Rust/TS-замеров семьи F-215 |
| @fact:B042-LOCATOR **locator** | bench-станки всех трёх стеков готовы и параметризованы на корпус потребителя; Go-корпуса нет; растовый и TS-корпуса в `research/tcg-bench/` малы (9 и 7 кейсов) |
| @fact:B042-SEVERITY **severity** | P3 |
| @fact:B042-DISPOSITION **disposition** | `accepted` — **решение владельца 2026-08-02 (дословно): «создание тестовой кодовой базы, на которой мы будем делать замеры — это какая-то работа на далекое будущее. Например, тестовый код можно было бы сгенерировать через LLM или фаззером. Прямо сейчас мы такую базу делать не будем»** |
| @fact:B042-FILED **filed by** | рулинг предъявления F-167, 2026-08-02 |

- @fact:B042-SUT **Суть.** Замеры производительности TCG-оракулов требуют представительной кодовой базы-корпуса. Решено: не строить сейчас; направление на будущее — генерация корпуса LLM'ом или фаззером. Запись существует, чтобы отсутствие Go-замеров не переоткрывалось как новая находка.
- @fact:B042-STANDING-ANSWER **Стоячий ответ (владелец, 2026-08-02, второй раз):** «замеров нет и нескоро будет, нужно положить куда-нибудь в роадмап и больше не кошмарить меня вопросами "почему нет замеров"». Исполнено: все три complete-цели стеков (rust/go/ts) несут аннотацию «posted, not yet measured» с именем своего bench-станка и ссылкой на эту запись (батч D33); карта развития несёт тот же стоячий ответ. **Вопросы вида «почему нет замеров» владельцу больше не задаются — ответ здесь.**
