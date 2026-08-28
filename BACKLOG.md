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

## Пост-1.0 — мандат-заготовка следующей кампании {#post-1-0}

@fact:POST10-MANDATE **Рулинг владельца 2026-08-20, дословно:** «Корзина 3 и
всё остальное не сделанное нужно будет сделать отдельной кампанией сразу же
после завершения 1.0.0 до конца — но потерять это ни в коем случае нельзя
нужно это записать!!!» Релиз 1.0.0 исполняет
[`campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md`](campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md);
его §4 несёт полный поимённый список невключённого с причинами и
триггерами, ledger-строки — в
[`deferrals.md#release-1-0`](campaigns/packages-2026-09/deferrals.md#release-1-0).
Этот раздел — корневой якорь, переживающий зону кампании: следующая
кампания стартует отсюда. Крупное одной строкой: **фаза T** (тест-рой —
отменена рулингом 2026-08-20, спеки сохраняются), **фаза G полная** (пакет
`org.vibevm.doc`), **подпись пакетов** (B-015, ждёт уведомления),
**wave 3 PROP-044** (ждёт `public = true`), **потребительская сверка
repomd** (B-084-хвост), **CI**, **дистрибутивы Linux/Mac** (владелец, другие
машины), и все живые P2/P3 строки этого файла, не взятые релизным ТЗ.

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
  ([`##WAL-C-MEASURE-WHAT-IS-BUILT-FIRST`](vibevm/vibespecs/WAL.xml)). Completing the
  migration means deleting the pre-ruling closed rows; that is a decision
  about this file's history and it belongs to the owner, not to a regex.

## Открытые решения владельца — пять вопросов {#owner-decisions}

@fact:OWNER-DECISIONS-WHY-THIS-SECTION-EXISTS **Зачем этот раздел.** Пять
вопросов ниже — не работа, а **решения**, которых ждёт работа. Каждый живёт
своей строкой или находкой аудита, и по отдельности они не теряются. Терялась
**группировка**: то, что эти пять — один список, стои́т перед владельцем вместе,
и остальные четыре нельзя забыть, пока решается первый. Группировка жила в
контрольной точке и в файле холодного возобновления, а оба переписываются
целиком — то есть жила до первого завершения сессии. Это тот же закон, который
проект уже держит для находок («не оставляй только в волатильной точке»),
применённый к указателям. @status:impl/done

@fact:OWNER-DECISIONS-THE-FIVE **Пять вопросов, состояние на 2026-08-09:** @status:impl/done

| # | вопрос | где живёт | состояние |
|---|---|---|---|
| @fact:OWNER-Q1 **1** @status:impl/done | формат каталога пакетов: схемы, генерация, эволюция | [`#b-073`](#b-073) | **РАТИФИЦИРОВАН 2026-08-13** («Ратификацию на сам PROP-044 даю»); фазы ТЗ разрешены, исполнение — свежей сессией. Идеология — [`vibevm/vibespecs/common/PROP-044-change-native-formats.xml`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml), ТЗ — [`campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md`](campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md); рамка владельца «среда перманентного перелома» + вердикт отдельно созванного Fable-совета; слово за владельцем |
| @fact:OWNER-Q2 **2** @status:impl/done | недетерминированная запись каталога | [`#b-072`](#b-072) | **всплыл заново** как дефект воспроизводимости; оказался частью вопроса 1 |
| @fact:OWNER-Q3 **3** @status:impl/done | сервер каталога не пишет логи | [`#b-071`](#b-071) | **решён 2026-08-13**: подписчик всегда, уровень по умолчанию warn (ТЗ D11, СТОП снят); стройка — Ф6.2 ТЗ |
| @fact:OWNER-Q4 **4** @status:impl/done | планка доказательства: требовать ли ссылку на код у каждого «проверено» | `AUDIT.md` `2026-08-06-01` (P1, открыта) | **решён 2026-08-13 — двухсортная планка**: per-fact evidence обязателен для утверждений о поведении кода, document-level легален для структурных/декларативных; пересуд host-подмножества ниже планки — будущая кампания (рулинг дописан в находку) |
| @fact:OWNER-Q5 **5** @status:impl/done | соединение движка качества и карты — по данным, с оговоркой о свежести | [`#b-019`](#b-019) часть (в) | **подтверждён 2026-08-13**: движки не сливаются, соединение по данным на запросе; ответ называет, когда получены находки; «отчёта нет» = «не измерено» |

@fact:OWNER-DECISIONS-WHERE-THE-RESEARCH-LIVES **Исследование по вопросу 1
лежит ВНЕ репозитория**, и загрузочная последовательность его не находит:
`C:\Users\olegc\git\v\discovery\vibevm-schema-evolution-discovery\`. Точка
входа — `12-HANDOFF.xml`; его раздел 2 перечисляет обязательное чтение (12
документов, ~12 400 строк) в правильном порядке. Три круга: четыре
веб-исследователя, пять GLM-воркеров, финальное ревью отдельной моделью.
**Эта строка и есть долговечный указатель** — те, что стоя́т в контрольной точке
и в файле холодного возобновления, переписываются целиком и на них полагаться
нельзя. @status:impl/done

@fact:OWNER-DECISIONS-THE-PYRAMID **Место этих пяти в общей вложенности,
записанное здесь, чтобы восстанавливалось без волатильных файлов:** идёт
рефакторинг PROP-043 (волна 2, кампания `packages-2026-09`, фаза E) →
внутри него осушение этого файла и карты развития
([`##MAP-POINTER`](#map)) → внутри осушения эти пять решений → внутри
первого из них подплан по формату каталога. Каждый уровень называет
соседний **в долговечном файле**, поэтому цепочка восстанавливается снизу
вверх чтением, а не памятью. @status:impl/done

## P1 — handled; kept as record {#p1}

*(empty — an open P1 is not in a file, it is in the owner's hands)*

## P2 — the next wave drains from here {#p2}

### B-101 — локальный кэш пакетов ключуется версией и не видит правку источника без бампа {#b-101}

| | |
|---|---|
| @fact:B101-ANCHOR **anchor** | `~/.vibe/cache/<group>/<name>/v<версия>/` + `vibe install`/`update` после правки авторского пакета без смены версии |
| @fact:B101-LOCATOR **locator** | lock-хэш честно пересчитывается (`content_hash` сдвинулся), но fetch видит кэш-хит по версии и материализует СТАРЫЙ снапшот; сайдкар `v1.0.0.sha256` лежит рядом и в сверке не участвует; воспроизведено на волне PROP-049: фрагменты сели в слоты только после ручного `rm -rf` кэша шести пакетов (дважды — второй раз для redbook) |
| @fact:B101-SEVERITY **severity** | P2 — молчаливая несвежесть установленного мира при локальной разработке пакетов; single-dev-цикл «правь пакет → install» тихо не работает |
| @fact:B101-DISPOSITION **disposition** | `open` — кандидат: fetch сверяет сайдкар-хэш кэша с lock-`content_hash` и рефетчит при расхождении (локальные источники — всегда) |
| @fact:B101-FILED **filed by** | волна PROP-049 (snippet-genre), 2026-08-23 |

- @fact:B101-SUT **Суть.** `local_source_freshness` честно предупреждает, но предписанный им же `vibe install --assume-yes` НЕ чинит: кэш отдаёт старые байты под новым замком. Оператор без знания внутренностей ходит по кругу предупреждения.

### B-102 — изолирующий per-edge proof маски строгости — O(рёбра × analyze) {#b-102}

| | |
|---|---|
| @fact:B102-ANCHOR **anchor** | `crates/vibe-install/src/visibility_projection.rs` (`edge_contributes`) |
| @fact:B102-LOCATOR **locator** | на каждое ребро решённого графа клонируется весь VisibilityGraph и гоняется полный `analyze`; на мирах в десятки узлов незаметно, на сотнях станет квадратично-дорогим |
| @fact:B102-SEVERITY **severity** | P3 — корректность доказана голденами; чисто стоимость |
| @fact:B102-DISPOSITION **disposition** | `open` — кандидаты: инкрементальный analyze, батч-доказательство рёбер одной целью, кэш масочных состояний |
| @fact:B102-FILED **filed by** | стройка PROP-050 (W2), 2026-08-23 |

- @fact:B102-SUT **Суть.** Маска строгости считается честно, но самым дорогим из корректных способов. Пока графы маленькие — норм; вырастут — переписать доказательство.

### B-103 — hygiene-линт не называет цену гранта; budget-cap на ленту не реализован {#b-103}

| | |
|---|---|
| @fact:B103-ANCHOR **anchor** | PROP-050 `##VERIFY-LINTS` (полная форма: «each reported with the lane cost the grant admits») + `##CLOSURE-DRIFT-CONTROL` (c): per-lane token budget cap — PROP-048-направление |
| @fact:B103-LOCATOR **locator** | `crates/vibe-check/src/checks/visibility_hygiene.rs` репортит мёртвые записи и отвергнутые гранты без токен-цены; lock несёт lane-байты только диффом (W5b), per-package cost не считается |
| @fact:B103-SEVERITY **severity** | P3 — наблюдаемость есть, экономическая аннотация — нет |
| @fact:B103-DISPOSITION **disposition** | `open` — считать вклад пакета в ленты при материализации, писать в lock, аннотировать линт и `vibe why`; cap — по слову владельца (PROP-048 DIR) |
| @fact:B103-FILED **filed by** | стройка PROP-050 (W5), 2026-08-23 |

- @fact:B103-SUT **Суть.** «Неиспользуемый грант виден» — сделано; «неиспользуемый грант стоит вам N токенов» — ещё нет.

### B-104 — ключ `override` живёт в двух смыслах: легаси `[[override]]`-пины и visibility-таблица {#b-104}

| | |
|---|---|
| @fact:B104-ANCHOR **anchor** | PROP-050 `##OVERRIDE-KEY-COEXISTENCE`; `crates/vibe-core/src/manifest/package/visibility.rs` (ManifestWire untagged union) |
| @fact:B104-LOCATOR **locator** | одно имя ключа — две формы (array-of-tables vs table); одновременное использование — громкая ошибка; читателю манифеста нужно знать различие форм |
| @fact:B104-SEVERITY **severity** | P3 — работает и протестировано; долг ясности схемы |
| @fact:B104-DISPOSITION **disposition** | `open` — ретирование/переименование легаси-пинов отдельной волной (no-legacy право уже есть) |
| @fact:B104-FILED **filed by** | стройка PROP-050 (W1, находка воркера), 2026-08-23 |

- @fact:B104-SUT **Суть.** Соседство осознанное и валидированное, но имя-омоним в схеме — источник будущей путаницы; выпрямить, пока экосистема закрыта.

### B-105 — `when = "concept:X"` — гейт по концепту-интерфейсу, скетч без рулинга {#b-105}

| | |
|---|---|
| @fact:B105-ANCHOR **anchor** | PROP-050 `##CONCEPT-GATE-DIRECTION` |
| @fact:B105-LOCATOR **locator** | сегодня `concepts` питает только словарь пресуппозиционного гейта; загрузочного предиката по концепту нет — только точечный `installed:<группа>/<имя>` |
| @fact:B105-SEVERITY **severity** | P3 — фича-направление |
| @fact:B105-DISPOSITION **disposition** | `open` — ждёт владельческого слова; реализация ляжет в WhenCondition + bootgen conditions одной малой волной |
| @fact:B105-FILED **filed by** | стройка PROP-050 (F7-диалог), 2026-08-23 |

- @fact:B105-SUT **Суть.** Концепт как интерфейс («какой-нибудь пакет мира несёт WAL-дисциплину»), пакет — как реализация; фрагменты смогут привязываться к дисциплине, а не к имени носителя.

### B-077 — TUI пишет путь сохранения настроек трижды, и это единственное, что отделяет его от «доделан» {#b-077}

| | |
|---|---|
| @fact:B077-ANCHOR **anchor** | `spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-037#done-definition` — рулинг владельца 2026-08-06 «доделан = ТОНКАЯ ПОВЕРХНОСТЬ»; найдено аудитом тонкости 2026-08-06 (замер M-TUI-THINNESS) |
| @fact:B077-LOCATOR **locator** | три копии в периметре: `crates/vibe-cli/src/commands/prefs/tui/form/lifecycle.rs:78-89` и `crates/vibe-cli/src/commands/prefs/tui/form/provenance_edit.rs:52` переписывают scope-гейт, `crates/vibe-cli/src/commands/tree/tui/settings.rs:547` переписывает мутацию по dotted-пути; библиотечные оригиналы — `crates/vibe-settings/src/cli/mod.rs:231-244` (`set_key`) и его `set_dotted` на `:247` |
| @fact:B077-SEVERITY **severity** | P2 |
| @fact:B077-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер claudez P2-PREFS)**: три копии заменены библиотечными вызовами; поверхность vibe-settings минимально расширена двумя pub-fn с doctest'ами и spec-метками — `check_writable` (standalone §7-гейт, поднят из `set_key`, одна точка правды) и `set_in_layer` (гейт+dotted-мутация по месту, ядро `set_key`); TUI-семантика рекавери скаляра-на-пути поднята в библиотеку (тихий no-op CLI стал честной записью — строгое исправление, с тестом); +7 тестов и 2 doctest'а; scope-отказ формы = отказу CLI `set` данными (тест эквивалентности). «TUI только рисует» — вторая половина рулинга — теперь держится и здесь |
| @fact:B077-FILED **filed by** | сессия 2026-08-06, аудит тонкости TUI |

- @fact:B077-SUT **Суть, по-простому.** Записать настройку — это библиотечная работа: проверить, что слой вообще доступен для записи, поставить значение по составному пути, сохранить атомарно. Библиотека это умеет, командная строка этим и пользуется. TUI проделывает то же самое сам, тремя отдельными кусками. @status:impl/done
- @fact:B077-WHY-THE-DELETION-TEST-IS-SILENT-ABOUT-IT **Почему проверка удалением про это молчит, и почему это всё равно дефект.** Удали TUI — способность не потеряется: `vibe prefs set` останется. По БУКВЕ теста нарушения нет. Но определение владельца состоит из двух половин, и вторая — «TUI только рисует». Работа, проделанная дважды, эту половину нарушает, и никакая проверка удалением её не увидит, потому что дубль по определению ничего не теряет. @status:impl/done
- @fact:B077-THE-CONTRAST-IS-NEXT-DOOR **Доказательство, что это не вкусовщина, лежит в том же периметре.** Модалка линта зовёт библиотечную `validate` вместо того, чтобы её скопировать; `diff_from_default` тоже вызывается, а не переписан. Поверхность УЖЕ умеет быть тонкой в двух местах и толстая в третьем — значит форма починки известна, её не надо изобретать. @status:impl/done
- @fact:B077-SHAPE-OF-THE-FIX **Форма починки, не построена.** Либо TUI зовёт `run_prefs(PrefsOp::Set { … })`, либо `vibe-settings` выставляет публичный «поставить и сохранить», сворачивающий хостовые шаги. Первое дешевле и ничего не добавляет в библиотеку; второе чище, если у поверхности окажутся свои требования к порядку шагов. Схлопывает три копии, дублированный scope-гейт и собственный `set_dotted` в один шов. @status:spec/plan
- @fact:B077-WHAT-THE-AUDIT-CLEARED **Что аудит рассмотрел и снял — записано, чтобы это не переоткрывали как новую находку.** Пайплайн фильтрации/формы/порядка (`flatten.rs`, `shape.rs`, `sort.rs`) — подготовка к показу над графом, который остаётся в библиотеке, и §1.3 контракта прямо относит фильтрацию, упорядочивание и уплощение к собственному слою модели приложения. Разбор `vibe.tree.*` — проводной формат СОБСТВЕННЫХ ключей поверхности. Обращения к `vibe_actions` (все 42) — санкционированный шов, а не обход: §13.6 контракта перечисляет ровно эти типы как принадлежащие ядру действий. @status:impl/done

### B-076 — гейт трассировки не проверяет, что цитируемый якорь существует, и два хостовых объявления цитируют несуществующий {#b-076}

| | |
|---|---|
| @fact:B076-ANCHOR **anchor** | `##WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS`; найдено 2026-08-06 при замере графа карты под пункт А5b программы |
| @fact:B076-LOCATOR **locator** | `crates/vibe-cli/src/commands/tools.rs:13` и `crates/vibe-workspace/src/tools.rs:16` — оба несут `specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#binaries")`; в `spec/modules/vibe-workspace/PROP-025-*.md` якоря `#binaries` нет (есть `build`, `cross-package`, `dispatch`, `gc`, `history`, `manifest`, `problem`, `root`, `security`, `staleness`, `v1-cut`) |
| @fact:B076-SEVERITY **severity** | P2 — **непокрытая земля, а не лгущий гейт**, см. `##B076-WHY-NOT-P1` |
| @fact:B076-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер claudez P2-MIRROR)**: оба объявления перецелены на существующий `PROP-025#manifest` (реестр — read-сторона поверхности деклараций §2; выбор аргументирован цитатой секции и согласован с якорным рядом `binary.rs`); в `cargo xtask specmap` добавлен resolve-гейт — хостовые рёбра (предикат: URI начинается с namespace САМОЙ карты из `specmap.toml`, не список адресов) обязаны разрешаться в единицу карты, не-хостовые вне юрисдикции молча считаются; идиома ратчета: репорт в обоих режимах, блок только под `--check`, красная строка называет символ/глагол/адрес/файл:строку и учит «чини цитату в коде, не добавляй мёртвый якорь в спеку». Синтетика: мёртвый якорь → CHECK-EXIT=1 дословно; хост: 0 unresolved / 24 вне юрисдикции |
| @fact:B076-FILED **filed by** | сессия 2026-08-06, замер M-A5B |

- @fact:B076-SUT **Суть, по-простому.** Код объявляет, какой пункт спеки он реализует. Два таких объявления называют адрес, которого в спеке нет и никогда не было. Тот, кто пойдёт по адресу, не найдёт ничего, а тот, кто спросит карту «что реализует этот пункт», не увидит этот код — потому что пункта нет. @status:spec/done
- @fact:B076-MEASURED **Измерено по закоммиченной карте, а не предположено.** В карте **955 рёбер**; у 934 из них дальний конец разрешается в единицу спеки, у **21 — нет**, и это **пять различных адресов**. Четыре из пяти — правильные: `ENGINE-CONFORM-v0.1#rules` (11 рёбер), `PROP-014#queries` (6), `PROP-014#index` (1) и полноимённый `…/core-ai-native/mechanisms/PROP-014#addressing-code` (1) адресуют УСТАНОВЛЕННЫЕ пакеты, которые намеренно вне карты проекта — на этом исключении держится байт-воспроизводимость карты. Пятый адрес — хостовый, и он единственный дефект из пяти. @status:spec/done
- @fact:B076-WHY-NOT-P1 **Почему P2, а не P1, и различие здесь несущее.** Гейт `cargo xtask specmap --check` проверяет свежесть карты и ратчет сирот — «публичный элемент в гейтуемом крейте без метки спеки». Он делает ровно то, что заявляет. Он НЕ заявляет, что метка разрешается, и не проверяет этого. То есть это **непокрытая земля**, а не гейт, зелёный про то, что он якобы смотрит, — то же различие, которое уже проведено при закрытии B-057. Если бы гейт утверждал разрешимость, это был бы P1 по `##SEV-GATE-BLINDNESS-IS-P1`. @status:spec/done
- @fact:B076-IT-IS-THE-OTHER-FACE-OF-A-KNOWN-FINDING **Это второе лицо уже записанной находки.** Про `vibe tools` уже сказано, что он отгрузился вообще без документа спеки, и единственной его долговременной записью были одноразовый план и переписанный WAL. Отсюда и адрес: секция, на которую ссылается код, не была написана. Находка не новая по причине — новая по тому, что её теперь видно механически. @status:spec/done
- @fact:B076-SHAPE-OF-THE-FIX **Форма починки — две половины, и вторая важнее.** *(i)* Точечно: переставить оба `scope!` на существующий якорь PROP-025 либо написать секцию, которую они называют, — это решение о том, есть ли у `vibe tools` контракт, а не о том, куда указать строку. *(ii)* Механически: гейт, который отличает неразрешимый ХОСТОВЫЙ адрес от законно чужого. Разделитель уже есть в данных — хостовые адреса начинаются с `spec://org.vibevm.core/vibevm/`, чужие нет, — так что правило выразимо без новых полей. Без второй половины первая чинит специмен и оставляет класс. @status:spec/plan

### B-075 — панель периодически краснеет на чистом дереве, и лечение «перезапусти» обесценивает её отказ {#b-075}

| | |
|---|---|
| @fact:B075-ANCHOR **anchor** | `##WAL-C-INSTRUMENTS-CATCH-WHAT-CARE-DOES-NOT` — «прочитай отказ, никогда не обходи его»; строка заведена ровно затем, чтобы этот совет остался исполнимым |
| @fact:B075-LOCATOR **locator** | `crates/vibe-cli/tests/cli_registry_mgmt.rs` — тест `install_expands_conditional_dependencies_when_predicate_matches`; падает внутри `cargo test --workspace` (шаг 2 `tools/self-check.sh`) на `git clone --recurse-submodules --branch v0.1.0 -- file:///…/org.vibevm_dispatcher.git`, который выходит с кодом 1, напечатав только `Cloning into '…'…` и ни одной строки диагноза |
| @fact:B075-SEVERITY **severity** | P2 |
| @fact:B075-DISPOSITION **disposition** | `open`, **сужена 2026-08-20 (BACKLOG-волна P2, воркер claudez P2-MIRROR)**: (1) отказ теперь ГРОМКИЙ — все фикстурные git-вызовы идут через `run_git`/`run_git_output` (`tests/common/mod.rs`), паника несёт полный argv, exit с расшифровкой NTSTATUS-кодов, ВЕСЬ stdout/stderr (пустой печатается как `<empty>` — пустой stderr и есть симптом), cwd и состояние clone-dest; прод-ошибка `GitError::CommandFailed` уже несла argv+status+stderr; (2) гипотезы «общая фикстура» и «recurse-submodules/отн. пути» ОПРОВЕРГНУТЫ кодом с цитатами (изоляция per-test/per-process полная); (3) стресс 45/45 зелёных прогонов под тройной параллельной нагрузкой + фоновый vibe-index — не воспроизвелось. Живой кандидат один — внешний контеншн ОС; спекулятивные ретраи не ставились (замаскировали бы). Строка ждёт СЛЕДУЮЩЕГО живого красного — он придёт с полной картиной, и тогда решается по данным |
| @fact:B075-FILED **filed by** | сессия 2026-08-06, первый прогон панели после wind-down №17 |

- @fact:B075-SUT **Суть, по-простому.** Панель — единственный пол этого проекта, и она покраснела на дереве, которое предыдущий wind-down оставил зелёным и не тронутым ни одной правкой. Тест прошёл в одиночку и прошёл при полном повторном прогоне СВОЕГО бинаря; красным он был только в общем прогоне воркспейса. То есть отказ не воспроизводится, а лечится перезапуском. @status:impl/done
- @fact:B075-WHY-IT-IS-A-ROW-AND-NOT-A-SHRUG **Почему это строка, а не пожатие плечами.** Мигающий гейт учит читателя перезапускать вместо того, чтобы читать, — а «прочитай отказ, никогда не обходи его» стоит в списке ограничений первой строкой, потому что за прошлую сессию инструменты поймали шесть ошибок, которых не поймала внимательность. Гейт, чей отказ иногда ничего не значит, отравляет именно ту привычку, на которой всё это держится. Настоящий красный, пришедший следом, будет перезапущен. @status:spec/done
- @fact:B075-WHAT-IS-MEASURED **Что измерено, а что нет.** Измерено: одиночный прогон — зелёный; полный прогон бинаря `cli_registry_mgmt` (68 тестов) — зелёный; полная панель следом — **всё зелёно**, включая шаги 3–12, которые в красном прогоне не выполнялись вовсе, потому что панель останавливается на первом отказе. Не измерено: частота. Один красный за один прогон — это не оценка вероятности, и делать вид, что это она, было бы ровно тем неоплаченным числом, против которого написана вся эта кампания. @status:impl/done
- @fact:B075-THE-SHAPE-OF-THE-CAUSE **Форма причины, названная гипотезой, а не диагнозом.** `git clone` вышел с кодом 1, напечатав только строку «Cloning into…», без сообщения об ошибке. Это поведение файловой системы под нагрузкой (клон создаёт много мелких файлов, пока рядом идут шесть десятков других тестов), а не логики: логика не бывает права в одиночку и неправа в толпе. Ближайшие подозреваемые — блокировка файла антивирусом и гонка за временным каталогом. **Диагноз не установлен, и строка существует ровно затем, чтобы следующий красный не был списан на эту гипотезу без проверки.** @status:spec/done
- @fact:B075-SHAPE-OF-THE-FIX **Форма починки, не построена.** Сначала — воспроизведение: прогнать `cargo test --workspace` подряд N раз и получить ЧАСТОТУ; без неё любая починка не проверяема. Потом одно из двух: сделать клон в тесте устойчивым к перегруженной файловой системе (повтор с диагнозом) — либо, если причина внешняя, зафиксировать её в машинных причудах и НЕ трогать тест. Чего делать нельзя ни при каком исходе: помечать тест `#[ignore]`. Тест, который иногда падает, — единственный свидетель; выключить его значит закрыть глаза, а не починить. @status:spec/plan

### B-074 — второй якорь факта в одном абзаце проглатывается молча, и факт теряет адрес {#b-074}

| | |
|---|---|
| @fact:B074-ANCHOR **anchor** | `##WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS` и правило размещения PROP-043 (маркер — первый или последний токен абзаца); найдено 2026-08-06 при разборе «осиротевших вердиктов» |
| @fact:B074-LOCATOR **locator** | `crates/progress-core/src/parse/` — разбор блоков и фактов; `vibe progress check --exhaustive` не сообщает ничего. Специмены (починены тем же заходом): `vibevm/vibespecs/design/loading-and-boot-model.xml`, `vibevm/vibespecs/design/workspace-and-qualified-naming.xml`, `vibevm/vibespecs/design/tui-visual-language.xml` |
| @fact:B074-SEVERITY **severity** | P2 — **граничит с P1**, см. `##B074-WHY-IT-BORDERS-P1` |
| @fact:B074-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер codex P2-CODEGEN)**: чекер `SwallowedAnchor` (Error) в `parse/swallowed.rs` — definition-form токен в continuation-строке тела факта пойман с именем якоря и строкой; без второго лексера (тот же `parse_anchor`, inline-код bleached штатным сканером, fences вне сканирования); 12 граничных тестов; exhaustive по 491 файлу — ноль ложных срабатываний |
| @fact:B074-FILED **filed by** | сессия 2026-08-06, погашение судейского долга |

- @fact:B074-SUT **Суть, по-простому.** Якорь факта — первый токен абзаца. Если два факта написаны подряд без пустой строки между ними, это один абзац: адрес получает первый, а второй становится телом соседа. Его маркер по-прежнему стоит, текст читается неотличимо, гейт молчит — но привязать к нему вердикт больше нельзя. @status:spec/done
- @fact:B074-IT-ALREADY-COST-NINE-DAYS **Это уже случилось и прожило девять дней.** Пять вердиктов в трёх дизайн-документах считались «осиротевшими» с 2026-07-28. Якоря были на месте; потеряна была адресуемость. Диагноз измерителя — «якоря больше нет» — был неверен, и это опаснее неверного счёта: он направляет ремонт не туда. Готовившийся ремонт — подрезать пять вердиктов — уничтожил бы пять действительных суждений ради опрятного числа. @status:spec/done
- @fact:B074-NOTHING-IN-THE-SYSTEM-SAYS-IT **Ни один сигнал не указывает на причину.** `progress check --exhaustive` на этих файлах чист; маркеры корректны; разметка валидна; для человека текст выглядит правильным. Единственным следом были пять записей в колонке, названной для другого отказа. Норма («маркер — первый или последний токен») есть, чекера у неё нет — ровно тот класс, который эта запись и цитирует якорем. @status:spec/done
- @fact:B074-WHY-IT-BORDERS-P1 **Почему граничит с P1 и почему заведено P2.** Адресуемость — фундамент, на котором стоит всё остальное: без адреса факт нельзя ни осудить, ни устареть, ни процитировать. Потеря адреса от пробела — структурная. Заведено P2 потому, что ничего не было потеряно безвозвратно: вердикты пережили девять дней и вернулись целыми, когда вернулся адрес. **Что переведёт в P1:** случай, где адрес потерян и вердикт при этом переписан или удалён, — тогда восстановления не будет. Понижать эту оценку может только владелец. @status:spec/done
- @fact:B074-SHAPE-OF-THE-FIX **Форма починки, не построена.** Дешёвая: `check` сообщает ошибку, когда в одном блоке больше одного якоря факта, — это ровно та же форма, что уже реализована для типизированного якоря (`@fact/code:`), который обязан быть последним фактом своего блока. То есть машинерия есть, ей не хватает одного случая. @status:spec/plan

### B-073 — генерация wire-типов индекса стоит дороже, чем показала таблица: одна форма невыразима, четырнадцать полей меняют вид {#b-073}

| | |
|---|---|
| @fact:B073-ANCHOR **anchor** | рулинг владельца 2026-08-06, пункт А6 программы: «вариант (а) — генерировать». Замер выполнен 2026-08-06 перед постройкой, по стоячему закону «мерить, что уже построено, до того как строить» |
| @fact:B073-LOCATOR **locator** | `crates/vibe-index/src/types/repomd.rs` и `crates/vibe-index/src/types/entry/{mod,aggregate,content,relations}.rs` — 18 публичных типов; генератор и его маршрутизация — `xtask/src/codegen.rs`; схем у индекса нет ни одной (в дереве девять `*.jtd.json`, все чужие) |
| @fact:B073-SEVERITY **severity** | P2 |
| @fact:B073-DISPOSITION **disposition** | `closed` — **закрыта замером 2026-08-20 (марафон, слайс С2): рулинг «вариант (2)» оказался исполнен ещё Ф4.1b (2026-08-15), до самого рулинга.** Мерено по дереву, не по памяти: писатель `crates/vibe-index/src/types/repomd.rs:42` несёт `#[serde(tag = "kind", rename_all = "lowercase")]` на ОБОИХ вариантах; сгенерированный ридер `crates/vibe-wire/src/generated/index/e1/repomd/` существует и тоже тегирован (`mod.rs:44`); паритет писателя и ридера запинен `crates/vibe-index/tests/wire_parity_repomd.rs`; байты манифеста пинует корпус (`crates/vibe-index/tests/golden_corpus.rs` перепроецирует журнал и сравнивает файл-в-файл). Невыразимая форма из `##B073-ONE-FORM-IS-INEXPRESSIBLE` перестала существовать вместе с untagged-вариантом. Хвост «14/21 полей empty-vs-absent» снят той же посадкой схем (у `files` стоит `x-empty: emit`); разрядность `size` — отдельная живая строка [`B-091`](#b-091), решаемая слайсом С2.1 |
| @fact:B073-FILED **filed by** | сессия 2026-08-06, замер под А6 |

- @fact:B073-SUT **Суть, по-простому.** Владелец решил генерировать wire-типы индекса из схем, и причина решения верна: запись индекса читается из ЧУЖОГО реестра, возможно собранного более новым инструментом, — там строгость означает, что новое поле ломает старых клиентов. Но таблица цены, на которую опиралось решение, перечисляла только то, что генерация теряет (методы, spec-ссылки, строгость). Замер нашёл третий пункт, которого в таблице нет: **часть форм в JTD не выражается вовсе, а часть выражается иначе — и это меняет опубликованный формат.** @status:spec/done
- @fact:B073-ONE-FORM-IS-INEXPRESSIBLE **Одна форма невыразима, и это не мелочь.** `RepomdFileEntry` — untagged-объединение: `{"kind":"directory","entries":N}` либо `{"size":N,"sha256":"…"}`. У JTD есть `discriminator`, но он требует тега-свойства в КАЖДОМ варианте, а у файлового варианта его нет. Выразить это в JTD можно только двумя способами, и оба плохи: добавить тег в файловый вариант — смена формата у каждого уже существующего индекса; или описать всё как один тип с необязательными полями — потеря непересекаемости, схема начнёт принимать `{"size":1,"entries":2}`. @status:spec/done
- @fact:B073-FOURTEEN-FIELDS-CHANGE-SHAPE **Четырнадцать полей поменяют вид на проводе.** Сегодня списки и словари несут `skip_serializing_if = "Vec::is_empty"` / `"BTreeMap::is_empty"` — пустое просто отсутствует. Генератор такого не умеет: у него необязательное — это `Option<Box<T>>` со `skip_serializing_if = "Option::is_none"`. Для 14 скалярных `Option`-полей это совпадение, для 14 списочных — нет: либо поле станет являться всегда (пустым), либо тип станет `Option<Vec<…>>`, что не то же самое. **Различие «отсутствует» против «пусто» видит каждый существующий потребитель.** @status:spec/done
- @fact:B073-STRICTNESS-IS-FIFTEEN-PLACES-HERE **Строгость здесь — пятнадцать мест, а не общая цифра.** Программа мерила `deny_unknown_fields` по всем хостовым крейтам (~63). В самих wire-типах индекса их **15**, и все они уходят разом. Это ровно то, что владелец и решил принять ради совместимости вперёд; число названо, чтобы решение принималось по числу, а не по слову. @status:spec/done
- @fact:B073-WHAT-IS-CHEAP **Что при этом ДЕШЕВО, чтобы цена не выглядела запретительной.** Маршрутизация генератора — действительно одна ветка `match` в `xtask/src/codegen.rs` (исключение для одного стема там уже есть). Документация полей переживает генерацию: `metadata.description` становится док-комментарием, проверено на `install_plan`. Скалярные необязательные поля отображаются один в один. То есть дорога только там, где форма типа выходит за пределы того, что JTD умеет описывать. @status:spec/done
- @fact:B073-THE-RESEARCH-2026-08-09 **Исследование 2026-08-09 — три круга, решения нет.** Вопрос вырос: начали с границы генерации, пришли к политике эволюции долговечных форматов вообще (их три — каталог, `vibe.toml`, `vibe.lock`, — и решены они по-разному). Материал ВНЕ репозитория: `C:\Users\olegc\git\v\discovery\vibevm-schema-evolution-discovery\`, точка входа `12-HANDOFF.xml`. Четыре веб-исследователя, пять GLM-воркеров, финальное ревью отдельной моделью. **Что установлено твёрдо:** объединение вариантов у нас полутегированное, а не нетегированное (у «папки» тег есть намеренно, у «файла» нет), и переход на явный тег разблокирует генерацию; полей, где «пусто» неотличимо от «нет поля», **21**, а не 14, и решение принималось по заниженному числу; каталог перечитывается ради перезаписи на шести путях, поэтому терпимость к незнакомым полям **без их перехвата** превратит громкий отказ в тихое стирание чужих данных; ближайший существующий аналог задачи лежал всё время в `refs/src/cargo/` непрочитанным. **Что оспорено:** модель «схема едет с данными» отозвана; сужение объёма работ по сегодняшней надобности признано экономией усилий, а не решением. @status:impl/done

- @fact:B073-SHAPE-OF-THE-ANSWER **Форма ответа, не построена и требует слова владельца.** Три варианта, и выбор между ними — про формат, а не про код: *(1)* генерировать всё, кроме `repomd`, и оставить манифест рукописным с записанной причиной — граница проходит ровно по невыразимой форме; *(2)* сменить формат `RepomdFileEntry` на тегированный и сгенерировать всё — чище, но это несовместимое изменение манифеста, который читает каждый потребитель первым; *(3)* отложить до того, как у индекса появится второй независимый потребитель, потому что до тех пор совместимость вперёд защищает от риска, которого ещё нет. @status:spec/plan

### B-072 — индекс пишет себя недетерминированно, и поэтому «ничего не изменилось» невыразимо {#b-072}

| | |
|---|---|
| @fact:B072-ANCHOR **anchor** | найдено воркером при постройке А1 (автопубликация) 2026-08-06 и отчитано как остаток, а не обойдено; ближайший закон — `@fact:REPOMD-LAST-LAW` PROP-005, порядок записи ради консистентного чтения |
| @fact:B072-LOCATOR **locator** | `crates/vibe-index/src/index/memory.rs` — `write_to` ставит `generated_at: Utc::now()` на каждую запись; следствие видно в `crates/vibe-index/src/publish.rs` (путь `NothingToCommit`) |
| @fact:B072-SEVERITY **severity** | P2 |
| @fact:B072-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С2, воркер R2-IDEMPOTENT): форма (2) из `##B072-SHAPE-OF-THE-FIX` построена.** Замер воркера сперва опроверг устаревший локатор: `Utc::now()` уже был вынесен в CLI-слой (F2-1, отметка едет параметром), но проектор безусловно клал СВЕЖУЮ отметку и на неизменившееся состояние. Починка в `write_to` (memory.rs): scratch-проекция с дисковой отметкой → побайтовое сравнение набора проекции → совпало — не пишется НИЧЕГО (автопубликация даёт `NothingToCommit` по чистому git-статусу), разошлось — полная запись со свежей отметкой; REPOMD-LAST-LAW не тронут, `state/` (журнал) сознательно вне сравнения. Красное: с обезоруженным гейтом падают ровно два инвариантных теста («идентичная мутация ⇒ ноль байтов»). Поле `generated_at` сохранено, семантика не менялась. Хвост-наблюдение воркера: повторный CLI `add` меняет ЖУРНАЛ (вторая запись с новым `indexed_at`) — фолд честно даёт «изменилось»; CLI-дедупликация — отдельный владельческий вопрос, если понадобится |
| @fact:B072-FILED **filed by** | сессия 2026-08-06, постройка А1 |

- @fact:B072-SUT **Суть, по-простому.** Две одинаковые записи индекса дают разные байты, потому что в манифест кладётся текущее время. Значит вопрос «изменилось ли содержимое» на файлах не задать: ответ всегда «да». @status:spec/done
- @fact:B072-WHAT-IT-COSTS-TODAY **Что это стоит уже сегодня.** Автопубликация коммитит каталог после каждой мутации, поэтому повторный идентичный апсерт порождает коммит, у которого меняется одна отметка времени. История индекса засоряется, а `git log` перестаёт быть журналом публикаций. Путь «нечего коммитить» построен и покрыт тестом, но срабатывает только при настоящем перекрытии публикаций — не тогда, когда изменений действительно нет. @status:spec/done
- @fact:B072-WHY-IT-IS-A-ROW **Почему строка, а не попутная починка.** Убрать отметку — это смена того, что видит внешний потребитель индекса, и вопрос к формату: `generated_at` кто-то может читать как «когда это собрано». Решать должен тот, кто знает потребителей, а не тот, кто чинил флаг. @status:spec/done
- @fact:B072-SHAPE-OF-THE-FIX **Форма починки, не построена.** Либо отметка перестаёт входить в хэшируемое содержимое (остаётся, но в отдельном не-версионируемом месте), либо запись становится идемпотентной по содержимому и обновляет отметку только когда что-то реально изменилось. Второе дешевле и сохраняет поле. @status:spec/plan

### B-071 — сервер индекса пишет в трассировку, которую никто не слушает {#b-071}

| | |
|---|---|
| @fact:B071-ANCHOR **anchor** | замер при ревью А1 2026-08-06; ближайший закон — `##WAL-C-SILENCE-IS-THE-DISEASE` |
| @fact:B071-LOCATOR **locator** | `crates/vibe-index/src/server/mod.rs` монтирует `TraceLayer::new_for_http()`; подписчика (`tracing_subscriber`) во всём крейте не ставится нигде, кроме нового пути под флагом `--auto-commit-push` в `crates/vibe-index/src/cli/serve.rs` |
| @fact:B071-SEVERITY **severity** | P2 |
| @fact:B071-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, С3): первая половина — замером, вторая — посадкой R3-LADDER.** Подписчик безусловен с Ф6.2 (D11): `main.rs` ставит его для каждой подкоманды, уровень по умолчанию `warn` (боссов предзамер 2026-08-20; с посадкой лестницы фильтр идёт через `resolve_log_filter` — флаг > `VIBE_INDEX_LOG` > `VIBE_LOG` > файл > warn). Обещание докблока `org_walk.rs:8-9` стало правдой: хелпер `note_skip` пишет SkipNote в отчёт И испускает `tracing::warn!` во всех четырёх дефектных местах пропуска (repo без .git; неудачный tag -l; тег не v<semver>; ошибка build_entry/манифеста); здоровый инкрементальный пропуск сознательно без warn (закомментировано в коде); тест `skips_surface_as_tracing_warns` перехватывает предупреждения scoped-подписчиком |
| @fact:B071-FILED **filed by** | сессия 2026-08-06, ревью А1 |

- @fact:B071-SUT **Суть, по-простому.** Сервер смонтировал слой трассировки HTTP-запросов и ни разу не поставил подписчика. То есть события формируются и выбрасываются: у оператора нет ни строчки лога запросов, хотя код выглядит так, будто есть. @status:spec/done
- @fact:B071-THE-COUPLING-IS-THE-ODD-PART **Странность, которую надо снять, а не унаследовать.** С 2026-08-06 подписчик появляется — но только когда включён `--auto-commit-push`, потому что иначе предупреждение о неудачной публикации было бы невидимым и требование «пишется в warn» ничего бы не значило. Это правильный минимум для той работы и неправильная связь вообще: наблюдаемость сервера не должна зависеть от того, публикует ли он себя сам. @status:spec/done
- @fact:B071-WHY-NOT-FIXED-IN-PASSING **Почему не починено попутно.** Безусловный подписчик меняет то, что сервер пишет в stderr, у КАЖДОГО оператора — это видимое изменение поведения, и его цена (какой уровень по умолчанию, не зашумит ли `TraceLayer` INFO-спанами) решается отдельно от флага публикации. **Цена решена владельцем 2026-08-13: подписчик всегда, уровень по умолчанию warn (ТЗ D11); стройка — Ф6.2 ТЗ.** @status:spec/done
- @fact:B071-A-DOCSTRING-ALREADY-CLAIMED-IT **Мелочь того же рода рядом, и периметр у неё узкий.** `crates/vibe-index/src/scanner/org_walk.rs` в шапке обещает, что пропущенные репозитории идут в `tracing::warn!`; в этом файле вызова нет. Во всём крейте вызовов ровно два, и оба появились 2026-08-06 вместе с автопубликацией (`server/routes/packages.rs`) — то есть на момент написания обещания их было ноль. Обещание без исполнения — ровно тот класс, который эта кампания и ищет, и здесь оно к тому же удваивает первый пункт: даже там, где вызов появился, слушателя без флага нет. @status:spec/done

### B-070 — закрытый словарь видов пакетов живёт в четырнадцати местах, и связывает их только память {#b-070}

| | |
|---|---|
| @fact:B070-ANCHOR **anchor** | `##WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS` и `##WAL-C-ONE-LAW-ONE-IMPLEMENTATION`; всплыло при добавлении шестого вида `lang` 2026-08-06 |
| @fact:B070-LOCATOR **locator** | `crates/vibe-core/src/package_ref/kind.rs` — enum-первоисточник; `crates/vibe-index/src/types/kinds.rs` — вторая копия enum; `schemas/list_report.jtd.json`, `schemas/registry_sync_report.jtd.json`, `crates/vibe-cli/resources/package-tree.schema.v1.json` — три схемы; плюс ~9 рукописных списков в help-текстах, сообщениях об ошибках, `xtask/src/batch_review/refs.rs`, `vibevm/vibespecs/common/PROP-000.xml`, `vibevm/vibespecs/boot/00-core.xml` |
| @fact:B070-SEVERITY **severity** | P2 |
| @fact:B070-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С4, воркер codexrunner R4-VOCAB).** Свежая перепись дала **31 рукописное проявление** (не ~14); один тест `crates/vibe-wire/tests/vocabulary_parity.rs` читает объявленный дом (`formats/vocabularies.json.package_kind`) и сверяет КАЖДУЮ копию поимённо — enum/массивы по составу и порядку, прозу точными фрагментами конкретных сообщений (не поиском значений «где-нибудь»), связь JTD-схем через `x-vocabularies`+`ref`. Проба probe красила тест списком всех отставших. Найденный дрейф исправлен: schema-tree знал будущий `app`, init-prompts не знал `lang`, три help-текста и MCP-шаблон знали четыре вида; плюс три нормативные строки `VIBEVM-SPEC.md` (546/611/1496) отставали с доПРОП-027 времён — выправлены боссом той же посадкой |
| @fact:B070-FILED **filed by** | сессия 2026-08-06, постройка В2 |

- @fact:B070-SUT **Суть, по-простому.** Список видов пакетов закрыт по замыслу, но записан четырнадцать раз. Компилятор держит честными **две** копии (enum-первоисточник и зеркало в индексе — их связывает паритетный тест). Остальные двенадцать при добавлении значения **не ломаются**: они просто продолжают перечислять старый набор. @status:spec/done
- @fact:B070-IT-ALREADY-DRIFTED-BEFORE-ANYONE-NOTICED **Это не гипотеза — дрейф уже случился и прожил незамеченным.** Две из трёх JTD-схем знали **четыре** вида и так и не узнали про `mcp`, который приехал с PROP-027 задолго до этой сессии. Никто не заметил, потому что заметить было нечем. @status:spec/done
- @fact:B070-THE-MEASUREMENT-MISSED-THEM-TOO **Замер тоже их не увидел, и это про периметр.** Перед постройкой я посчитал рукописные списки поиском текста `flow, feat, stack, tool, mcp` — в JSON-схеме тот же список записан массивом `enum`, по имени на строку, и образец до него не дотянулся. Нашла панель: схема дерева упала ровно в тот момент, когда в дереве появился `lang`-пакет. @status:spec/done
- @fact:B070-SHAPE-OF-THE-FIX **Форма починки, не построена.** Один источник и проверка, что остальные с ним согласны: тест, читающий `PackageKind::ALL` и сверяющий его с каждой схемой и каждым прозаическим списком, — либо генерация схем из enum. Дешевле первое: список коротких, а мест много. @status:spec/plan

### B-069 — документация индекса описывает раскладку и маршруты, которых в коде нет {#b-069}

| | |
|---|---|
| @fact:B069-ANCHOR **anchor** | замер M-INDEX 2026-08-06 по вопросам владельца об индексе; программа владельца, пункт Б7 («записать факты, не требующие решения») |
| @fact:B069-LOCATOR **locator** | документация формата описывает файлы `by-name/<kind>/<name>.json` и серверные маршруты, несущие **вид** пакета; код перешёл на `by-name/<name>.json` и на **группу** в маршрутах (`crates/vibe-registry/src/index_client.rs`, `crates/vibe-index/`) |
| @fact:B069-SEVERITY **severity** | P2 |
| @fact:B069-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, С3): замер + одна строка.** Расхождение оказалось уже вылеченным с обеих сторон ещё до строки: PROP-005 перекроен на group-native 2026-05-22 (`##HISTORY-GROUP-NATIVE`), `docs/consumer-protocol.md:115` прямо говорит «there is no `<kind>/` level anywhere», маршруты несут группу. Единственная пережившая ложь — докблок `crates/vibe-index/src/index/mod.rs:7` перечислял живой пайплайн со старым путём `by-name/<kind>/<name>.json` — исправлен (стал историей с указанием перевода на `by-name/<name>.json`); `by_name.rs:9` — честное «Before PROP-008…», оставлено |
| @fact:B069-FILED **filed by** | сессия 2026-08-06, группа Б программы |

- @fact:B069-SUT **Суть, по-простому.** Тот, кто соберёт потребителя индекса по документации, построит нечто, что не работает: он будет запрашивать не те пути и не те маршруты. Ошибка не в коде и не в документе по отдельности — они просто разошлись, и разошлись молча. @status:spec/done
- @fact:B069-SAME-CLASS-AS-THE-DOCBLOCK **Тот же класс, что и обещание в двух домах.** Неделей раньше docblock обещал две поверхности, которых не существовало ни одной (починено 2026-08-06). Обещание, которое код не держит, живёт везде, где его пересказали, и документация формата — самое дорогое место для такого пересказа, потому что по ней строят внешние потребители. @status:spec/done
- @fact:B069-WHY-IT-IS-A-ROW-AND-NOT-A-FIX **Почему строка, а не немедленная починка.** Какая сторона права — вопрос суждения, а не сверки: маршрут по группе может быть намеренным улучшением, которое документация не догнала, либо обратной несовместимостью, которую никто не заметил. Решать это должен тот, кто знает намерение; строка существует, чтобы вопрос не растворился. @status:spec/done

### B-068 — вторая половина рулинга D: забор становится телом факта через `@fact/code` {#b-068}

| | |
|---|---|
| @fact:B068-ANCHOR **anchor** | рулинг владельца 2026-08-06 (вариант D): «по умолчанию любой забор является примером, но его должно быть можно пометить как утверждение, и тогда пусть проверяет»; синтаксис — `@fact/ТИП:ЯКОРЬ`, где следующий за фактом объект названного типа прилепляется к его телу |
| @fact:B068-LOCATOR **locator** | `crates/progress-core/src/parse/facts.rs` — `take_fact_id` читает `@fact:` и `##`, но тип не разбирает; тело факта по-прежнему кончается на пустой строке |
| @fact:B068-SEVERITY **severity** | P2 |
| @fact:B068-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер codex P2-CODEGEN + босс-спека)**: `@fact/code:<ID>` разобран тем же единственным anchor-reader'ом (opener/type/id); закрытый type-set — неизвестный тип = ошибка разбора с именем типа; adjacency-закон (пустые строки ок, любой иной блок или не-последний факт блока — ошибка); забор входит в тело и content_hash факта (правка забора старит факт), оставаясь opaque для сканов маркеров; PAYLOAD_SCHEMA→2 (старый sidecar — cache miss, не неполный warm parse); 167 тестов, exhaustive clean (typed-форм в корпусе ноль — поведение неизменно). Грамматика дописана в PROP-043 §8 `##PARSE-TYPED-FACT-CODE`, суд confirmed, файл запечатан. 372 забора корпуса получили путь стать судимыми утверждениями — opt-in'ом, по умолчанию забор остаётся примером |
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
| @fact:B067-DISPOSITION **disposition** | `planned` — владелец разрешил бамп 2026-08-06; решено было отложить до публикации, см. `##B067-WHY-DEFERRED`; **2026-08-20 поглощена волной 1.0.0** — рулинг «все пакеты бампятся до 1.0.0», исполняет `TZ-RELEASE-1.0` слайс С5 |
| @fact:B067-FILED **filed by** | сессия 2026-08-06, миграция разметки |

- @fact:B067-WHY-DEFERRED **Почему отложено, а не сделано.** Всё потребление сегодня локальное (`source_kind = "local"`, 36 пакетов из `file://`), а публикация — за границей этой работы. Координата в реестре не переиспользуется, пока туда ничего не публикуется. Бамп же идёт каскадом: `redbook` пинит каждого из 23 членов точной версией, поэтому это не 38 независимых правок, а связная волна. @status:spec/done
- @fact:B067-WHAT-TRIGGERS-IT **Что делает это срочным:** любая попытка опубликовать любой из 38 пакетов. До того момента долг записан и безвреден. @status:spec/done

### B-066 — сервис индекса не умеет публиковать себя сам: флаг есть, работы нет {#b-066}

**Closed 2026-08-06 by `8a8aba12`.** The ruling and its reasoning now live in
`PROP-005 §2.17` — including the two startup refusals the build added (no git
working copy; `state/` not ignored, where `git add -A` would have pushed the
server's own bearer tokens) and the rule that a failed push never turns a
successful write into an error. The private-READ half this row carried is
closed separately: `PROP-002 §2.2.1.1`, commit `ed2ca404`. This line is a
tombstone — process support for whoever walks this file, not project
structure, and it goes when the file does.

### B-065 — образ организации перечитывается на каждой операции, хотя писатель один {#b-065}

**Closed 2026-08-06 by `57d9e8c4`.** The ruling and its reasoning now live in
`PROP-005 §2.8.1`. All five agreed points landed: the image is cached,
`--cache-org` is on by default, the cheap conditional freshness check is what
keeps that default honest rather than an enhancement, `rescan-org` is its own
unconditional verb, and the webhook answer for the fed-image future is
`PROP-005 §2.16`. This line is a tombstone — process support for whoever walks
this file, not project structure, and it goes when the file does.

### B-064 — движок дисциплины знает слово `vibedeps` {#b-064}

| | |
|---|---|
| @fact:B064-ANCHOR **anchor** | замер принадлежности движков по вопросу владельца 2026-08-06; закон переносимости — собственная оговорка движков: «проверяльщик работает на ЛЮБОМ проекте, а не только на том, в котором построен» |
| @fact:B064-LOCATOR **locator** | `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/store.rs:351` и `:434` — литерал `"vibedeps"` в списках пропускаемых каталогов для TypeScript и Go, рядом с универсальными `node_modules`, `target`, `vendor`, `testdata`. Не конфигурируется |
| @fact:B064-SEVERITY **severity** | P2 |
| @fact:B064-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С4, воркер codexrunner R4-VIBEDEPS, посадка 47bcc87e).** Литерал изъят из обоих встроенных списков; взамен — однородный policy-ключ `skip_dirs` (раннее отсечение точным именем компонента, отдельная семантика от `exclude_substrings`/B-059) во всех трёх языковых секциях; наш `conform.toml` несёт `[rust] skip_dirs=["vibedeps"]`, политики стеков замерены как не видящие vibedeps — не тронуты. Красное/зелёное на живых TS/Go деревьях; sync-engines write-through на все 6 vendored-движков (51 пара байт-чисто; «21 копия» строки — устаревшее число, честно названо воркером). Хвост-наблюдение: root-`vibedeps/**` снапшоты обновятся ремaterialise'ом С5 (те же 5 свежих local_source_freshness предупреждений) |
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

**Этот номер был занят дважды, и вот маршрут для тех, кого сюда привело не
то.** С 2026-08-15 по 2026-08-19 якорь `#b-056` несла ещё одна строка —
живая, про тип, которым пишет наш собственный писатель (`u64` против `uint32`
в JTD). Она перенумерована в [`B-091`](#b-091); номер `B-056` остаётся за
закрытой строкой выше, потому что именно её называют восемнадцать авторских
файлов, включая `crates/vibe-spec/**` и `PROP-035` §7.3. Ссылка на `B-056`,
написанная до 2026-08-19 и целившая в **живую** развилку — таких три, все
датированные записи: могильник фазы Ф4 в свёрнутом плане change-native и две
находки `harvest/` — приземляется сюда, и её настоящий адрес `B-091`.

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
| @fact:B007-ANCHOR **anchor** | the question is about `vibevm/vibespecs/common/**` and `vibevm/vibespecs/modules/**` as a genre, not about one anchor. The rule it would satisfy is `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root` |
| @fact:B007-LOCATOR **locator** | 153 sections in `vibevm/vibespecs/common/` + `vibevm/vibespecs/modules/**` carry a bolded **Decision** label; 4 carry all four fields |
| @fact:B007-SEVERITY **severity** | P2 |
| @fact:B007-DISPOSITION **disposition** | `closed` — **рулинг владельца 2026-08-20 (дословно): «в PROP файлах должна быть ADR часть, отдельных ADR-файлов не нужно»** — жанр решён: решения живут секцией внутри своего PROP, отдельный ADR-жанр не заводится; норма записана в PROP-000 §13 `##DECISIONS-LIVE-INSIDE-PROPS` |
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
  `When to revisit`): `vibevm/vibespecs/common` + `vibevm/vibespecs/modules` **153 → 4**; all of `spec/`
  **157 → 7**; the `fractality` specspace **34 → 14**; this campaign's own
  records **15 → 8**. The practice is adopted at roughly **41 %** in the sibling
  project and **4.6 %** in the host's PROP/FEAT tree. Counted 2026-07-31.
- @fact:B007-RE-MEASURED **Re-counted 2026-08-06, and the host figure has
  tripled.** `vibevm/vibespecs/common` + `vibevm/vibespecs/modules` **146 → 12**; all of `spec/`
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
  examples** (under `*/vibedeps/org.vibevm.world.decision-records/` and
  `flow-comparative-research/`, ×2 vendoring packages) — **0 authored**; the
  specspace's own authored decision blocks are 9, in a three-label dialect,
  none complete. So the honest comparison is «nobody authors the four-field
  form anywhere except this campaign's own plans», and the question is again
  *whether to adopt*, not «why is the PROP tree the outlier». Full measurement
  and the four costed options:
  `campaigns/packages-2026-09/harvest/d10-adr-genre-proposal.md`; the
  campaign's recommendation there is **B + A′** (four-field inside the owning
  section, forward-only, backfill only `vibevm/vibespecs/common/`, close `spec/decisions/`
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
| @fact:B015-DISPOSITION **disposition** | `closed` — **рулинг владельца 2026-08-20: «только от порчи»** — программа подписи (аутентичность) не заводится; `SHA256SUMS.txt` дистрибутива остаётся честным чеком целостности, его дисклеймер — постоянная правда, не времянка |
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
  2-ter. **Часть 2 ЗАКРЫТА 2026-08-06 (`fc7495f2`).** Второй уровень построен: `vibe select --where "…"` плюс MCP-инструмент `select` — семь предикатов через И, `scope:` по префиксу адреса, `has:`/`lacks:` по виду ребра, `depth:0..3` ненаправленным обходом, число шагов на каждом попадании, версия грамматики в ответе, неизвестный предикат и пустой запрос — ошибки с именем токена. Стоит ПОВЕРХ простого уровня и структурно не может его утянуть: грамматика в своём модуле за своей точкой входа. Контракт — `PROP-015 §2.2.2`, рассуждение — [`vibevm/vibespecs/design/map-query-language.xml`](vibevm/vibespecs/design/map-query-language.xml). **Замер, которого запись знать не могла:** канонический вопрос уровня неотвечаем без `scope:` — `lacks:verifies` в одиночку даёт 5742 из 5825, и сузить это нечем, потому что `kind` не несёт НИ ОДНА единица спеки в этом дереве, а `uri` точный. Открытыми у B-018 остаются части 3 (фрагменты по отпечатку, едет с B-016.2) и та половина части 4, что зависит от B-016.1.

  2-bis. **Часть 2 СУЖЕНА 2026-08-06 (`471e3b1b`): простой уровень построен, открыт только язык запросов.** Владелец постановил строить ОБА уровня; первый — три фильтра через И под жёстким потолком — построен как способность в разделяемом крейте с CLI и MCP поверх, контракт в `PROP-015 §2.2.1`. Вариант *(i)* из записи выше и есть построенное. Осталось *(ii)* — обход графа (глубина, «нет ребра такого-то вида»), тот самый, что отвечает на «какие правила никто не проверяет», и он **встаёт поверх первого уровня, а не заменяет его**: по рулингу владельца простой уровень ПОСТОЯННЫЙ, и в библиотеке он отдельная точка входа именно затем, чтобы сломанная грамматика не могла его утянуть. Грамматику придётся версионировать — это цена, названная заранее.
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
- @fact:B019-B-DESIGN **Дизайн (б) написан 2026-08-06: [`vibevm/vibespecs/design/command-nodes.xml`](vibevm/vibespecs/design/command-nodes.xml)**, на замере M-B019B (архив `cache/agents/sorted/M-B019B/`). Два измерения срезали цену. **Первое:** `item_kind` объявлен СВОБОДНОЙ строкой в схеме (`specmap.jtd.json:92-94`), тогда как все четыре его соседа несут `enum`, и **ни одно место продакшн-кода не сопоставляется по нему** — значит новый вид есть новое ЗНАЧЕНИЕ открытого поля, а не смена формата, какой была (а). **Второе:** у `explain` нет замкнутого множества видов цели вообще — `explain.rs:199-204` проверяет один строковый префикс, а `explain_symbol` матчит `symbol` и вида не спрашивает; поэтому узел с символом-путём вызова (`vibe install`) отвечается существующей машинерией БЕЗ единой правки `explain`. Опознание — по клэповскому `derive(Subcommand)`, а не по авторской метке: метка, без которой подкоманду можно добавить, есть норма без чекера. Сканер сегодня к `derive` слеп (у его читателя атрибутов ровно две руки), и это единственная добавляемая способность. Строка «(б) — с нуля» тем самым уточняется: с нуля здесь экстрактор, а не формат и не потребитель.
- @fact:B019-B-SLICE-1-LANDED **Срез 1 пункта (б) ПОСТРОЕН 2026-08-06.** В карте **56 узлов вида `command`** — `vibe` 29, `vibe-index` 14, `xtask` 13, — у каждого символом стоит путь вызова (`vibe install`), спан варианта и отпечаток по потоку токенов. Опознание — по клэповскому `derive(Subcommand)`, обе записи; сканер научился читать `derive`, чего не умел. Формат не тронут: вид элемента — открытая строка, и `explain` отвечает на `vibe install` существующим путём по символу, без единой правки. Осталось: **срез 2** (вложенность) и **срез 3** (приёмка `explain` тестом) — оба в [`command-nodes.xml`](vibevm/vibespecs/design/command-nodes.xml) `#cut`. Строка живёт, пока не закрыты они и пункт (в).
- @fact:B019-B-WHAT-THE-BUILD-COST **Что стройка стоила и чему научила — число приёмки поймало дефект, которого не увидело ни одно ревью.** Оба крейта, `vibe-cli` и `vibe-index`, объявляют `pub enum Command`; джойн искал перечисление по имени типа по всему воркспейсу, и `find` отдал обоим корням одно и то же — карта утверждала `vibe-index agentic` и `vibe-index term`, двадцать девять несуществующих команд. Джойн стал крейт-локальным, тест на два одноимённых перечисления в разных крейтах доказан падающим до правки. **Само число ошибалось трижды** (29 → 43 → 71 → 56), и каждый раз причиной был периметр измерения, а не поиск: цензус мерил поверхность одного бинаря, воркер — каталог `crates/`, карта — воркспейс, где нашёлся третий бинарь `xtask`. Одно чтение между ними дало **0**, и это была протухшая сборка, а не логика (`cargo clean -p core-ai-native-specmap` вернул 56).
- @fact:B019-A-COUNT-MOVED **Число в локаторе (а) сдвинулось, и не от регресса.** Запись говорит «915 из 915»; замер 2026-08-06 — **916 отпечатков из 932 элементов**. Выросло и то и другое: сканер JTD (B-060, построен 2026-08-05) добавил в карту виды `schema` (7) и `schema-def` (9), а отпечатка они не несут, потому что отпечаток считается по потоку токенов Rust. То есть «916 из 932» — это «все, у кого он определён», а не «шестнадцать потеряли». Числу в локаторе верить нельзя; мерить по дереву.
- @fact:B019-V **(в) — что имеется в виду, и вопрос границы систем (решить ДО реализации — требование владельца).** В кодовой базе два независимых движка: **conform** (гейт качества кода: прогоняет правила, находит нарушения) и **specmap** (карта связей «код ↔ спека»). Данные о «вариантах ошибок» — какие enum-варианты с какими текстами ошибок существуют и на какие требования ссылаются — **уже извлекаются конформом** для двух его правил. Карта этих данных не видит: это два разных графа двух разных подсистем. Вопрос: чьей частью становится узел «вариант ошибки»? Три варианта: **(1)** specmap извлекает сам — дублирование экстракции в двух движках, две правды об одном; **(2)** specmap читает данные conform'а — новая зависимость между сознательно разделёнными движками; **(3)** не сливать данные вовсе, объединять на этапе запроса — инструмент B-018 показывает и карту, и находки conform'а рядом. Склонность исследования — (3) при наличии B-018, иначе (1) с выносом общей экстракции в разделяемую библиотечку; окончательное решение — первый шаг реализации этой части.
- @fact:B019-V-THE-OWNER-CHOSE-THREE-AND-THE-TOOL-NOW-EXISTS **Владелец выбрал (3) 2026-08-06, а 2026-08-06 же появился инструмент, которого вариант ждал.** Простой уровень поиска по карте построен (программа, пункт А5), и его результат по построению несёт поле «откуда взят» — ровно затем, чтобы второй поставщик добавлялся значением, а не ломал каждого читателя. Дверь, которую вариант (в) требовал оставить открытой, открыта. @status:impl/done
- @fact:B019-V-THE-QUESTION-THAT-IS-NOW-CONCRETE **Вопрос, который рулинг не закрыл, потому что тогда он был неконкретен: КТО из потребителей понесёт обе зависимости.** Рулинг решал границу между ДВИЖКАМИ и решил её правильно — данные остаются каждый у своего. Но соединение «в момент запроса» требует, чтобы соединяющий дотянулся до обоих, и вот это измерено 2026-08-06: во всём хосте зависимость на движок качества несёт **ровно один крейт — `xtask`**, инструмент разработчика, который вдобавок намеренно выведен из-под дисциплинарных гейтов. Ни `vibe-trace`, ни `vibe-cli`, ни `vibe-mcp` его не знают. @status:spec/done
- @fact:B019-V-THREE-WAYS-TO-JOIN-AND-WHAT-EACH-COSTS **Три способа соединить, и цена у каждого своя.** *(1)* Дать крейту трассировки зависимость на движок качества — соединение честное и типизированное, но продуктовая поверхность впервые потянет за собой гейт-движок, а «сознательно разделённые» системы окажутся связаны у потребителя. *(2)* Соединять там, где обе уже есть, то есть в `xtask`, — новой связи нет, но `xtask` это developer tooling, а `vibe query` — продукт; поверхность оказалась бы не там, где живёт способность. *(3)* Соединять по ДАННЫМ, а не по коду: находка несёт `file` и `line`, узел карты несёт `file` и `line`, и этого достаточно — ни одному движку не нужно знать о другом. **Цена (3) названа честно: отчёт движка качества — артефакт на диске, и он свеж ровно настолько, насколько недавно его прогнали, тогда как карта строится свежей на каждый запрос.** Смешивать в одном ответе свежее и лежалое, не сказав об этом, — та же болезнь, что и всё остальное в этой кампании. @status:spec/done
- @fact:B019-V-RECOMMENDATION **Рекомендация босса — подтверждена владельцем 2026-08-13.** (3) с обязательной оговоркой: ответ, содержащий находки, называет, когда эти находки были получены, и отсутствие отчёта — это «не измерено», а не «нарушений нет». Так соединение не заводит ни одной новой связи между системами, которые владелец разделял, а единственная его слабость — разная свежесть двух половин — становится видимой вместо того, чтобы быть предположенной. Владелец, дословно: «движки conform и specmap это разные оси, не сливать, подтверждаю». Не построено. @status:spec/plan

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
- @fact:B046-NO-TYPED-MARKER **Чего в рельсах НЕТ — и это несущий пробел, которого запись не видела.** Отличить «AI-Native языковой стек» от любого другого пакета механически сегодня **нечем**. Поле `kind` — закрытое перечисление `flow | feat | stack | tool | mcp` (`crates/vibe-core/src/package_ref.rs:45-49`), и `kind = "stack"` несут **девять** канонических манифестов, из которых языковых стеков — три: рядом стоят семейные агрегаторы `rust-ai-native` / `typescript-ai-native` / `go-ai-native` (PROP-028, `kind = "stack"` ровно так же) и тест-фикстуры. Единственное сегодняшнее отличие — суффикс имени `-lang`, то есть **текст, а не объявление**. Ближайший механический признак — что у агрегатора нет `[boot_snippet]`, а у языкового стека есть с `category = "stack"`, — это вывод по косвенному, а не декларация, и норма без чекера дрейфует ровно так же ([`##WAL-C-A-NORM-WITHOUT-A-CHECKER-DRIFTS`](vibevm/vibespecs/WAL.xml)). **Поэтому у развилки появился нулевой вопрос, который решается ДО выбора из трёх вариантов: чем пакет объявляет, что он — AI-Native язык.** Любой из трёх вариантов без ответа на него будет угадывать по имени.
- @fact:B046-AUTONOMY-HOLDS **Закон автономности держится механически (проверено 2026-08-06).** Единственная зависимость каждого из трёх языковых стеков на `org.vibevm.*` — ровно `core-ai-native` (`rust-ai-native-lang/v0.7.0/vibe.toml:16` и близнецы); секций `[recommends]`/`[suggests]` нет ни у одного; на уровне `Cargo.toml` все три вендорят одни и те же четыре движковых крейта и ничего общего сверх них. Предпосылка варианта (2) «автономность не тронута по построению» — верна по факту, а не по намерению.
- @fact:B046-PRICE-OF-A-TOOL **Цена «добавить один MCP-тул» измерена по образцу: ~44 строки в одном файле, ноль правок диспетчера.** Самый простой существующий тул `explain` — `crates/vibe-mcp/src/tools.rs:487-529` (43 строки вместе с дескриптором; тело `run()` — 14 строк делегирования) плюс одна строка регистрации в `default_tools()` (`tools.rs:50`); роутинг по имени через `BTreeMap`, и док файла фиксирует прямо: «A new tool is a new cell added here, not an edit to the dispatcher» (`tools.rs:42-43`). **Оговорка, снимающая обманчивость этой дешевизны:** дёшев только каркас. Составного запроса «перечисли установленные языки и их поверхности» в коде нет — его надо писать, и он упирается в `##B046-NO-TYPED-MARKER`. Заодно измерено, что тул такого рода уже был запланирован и не построен: `tools.rs:8` несёт комментарий «Subsequent slices add `list_capabilities` … once `vibe-llm` is real».
- @fact:B046-RELATED **Смежность.** B-018 (агентские инструменты vibe — ростер-половина варианта (2) ложится туда естественно); B-047 (норма поверхностей — агрегатор обязан быть тонкой поверхностью над разделяемой логикой); PROP-026-грамматика (единая грамматика инструментов — то, что делает композицию дешёвой).

### B-047 — норма поверхностей: логика в разделяемом крейте, CLI и MCP — тонкие поверхности над ней {#b-047}

| | |
|---|---|
| @fact:B047-ANCHOR **anchor** | критика владельца 2026-08-02 (дословно): «Нужен какой-то код, доступный из разных поверхностей. MCP — одна поверхность, инструменты командной строки — другая. У нас постоянно в коде недостаточный уровень абстракции, всё прибивается гвоздями к конкретной реализации… логика, общая между MCP и CLI должна быть сформулирована абстрактно в какой-то библиотеке или крейте, чтобы ее переиспользовали разные поверхности» |
| @fact:B047-LOCATOR **locator** | стеки норму уже держат: логика в bridge/engine-крейтах (`rust-ai-native-tcg-bridge` и близнецы, conform/specmap-движки), CLI-бинари — первая поверхность, MCP-серверы — вторая (описания инструментов буквально «= `rust-ai-native init`»); проверить и довести хостовую сторону: vibe-mcp (четыре продуктовых тула) против CLI-паритета, B-018-инструменты — с рождения двумя поверхностями |
| @fact:B047-SEVERITY **severity** | P2 |
| @fact:B047-DISPOSITION **disposition** | `planned` — решение владельца 2026-08-02 (та же директива, что B-046). **Путевая половина (`##B047-THE-EXPENSIVE-ROW`) закрыта 2026-08-20** (марафон, слайс С4, воркер R4-B047-PATHS — первая живая задача лейна codexrunner/gpt-5.6-sol): общий `vibe_core::machine_json_path` (дом — rel_path.rs), ~30 машинных JSON/MCP путевых поверхностей унифицированы на POSIX-вид, человеческий вывод не тронут, узкий `files_written_posix` снят; паритет-тест `cli_mcp_path_parity.rs` с честным красным (временный legacy-прогон: `spec\\flows…` ≠ `spec/flows…`). Остальное (дом нормы поверхностей, аудит-программа) — пост-1.0 |
| @fact:B047-FILED **filed by** | рулинг истории OracleRegistry/F-210, 2026-08-02 |

- @fact:B047-SUT **Суть, по-простому.** Стоячая норма: пользовательская способность живёт в разделяемой библиотеке; CLI и MCP — тонкие поверхности над ней, ни одна не является «основой»; новая способность рождается минимум с двумя поверхностями или с записанной причиной, почему одной хватит. Работа записи: (1) аудит «где прибито гвоздями» — обход поверхностей хоста и стеков с таблицей «способность → логика-крейт → CLI → MCP → дыра»; (2) доводка найденных дыр (первый известный кандидат: карт-инструменты vibe — CLI-половина есть, MCP-половина — B-018); (3) поднять норму в спеку дисциплины owner-approved диффом (дом — решить при аудите; кандидат — ENGINE-CONFORM/GUIDE-семья рядом с четырёхслойной моделью SPEC/ENGINE/DRIVER/DEPLOYMENT, чьим уточнением норма и является: DRIVER — это не один бинарь, а набор тонких поверхностей над ENGINE).
- @fact:B047-CENSUS-BUILT **Пункт (1) закрыт 2026-08-06: цензус построен и лежит в [`campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md`](campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md).** Двадцать девять команд верхнего уровня, у девятнадцати логика в крейте, у **десяти** её дом — сам `vibe-cli` (`init`, `list`, `aiui`, `term`, `frame`, `show`, `tree`, `self`, `vars`, `version`); из пяти MCP-тулов норму держат **два** (`explain`, `agentic_explain` — один крейт, одна функция на обе поверхности), у **двух** CLI-близнеца нет вовсе (`read_subskill`, `materialise_subskill`), и **один** (`query_package`) делит с `vibe list` тип данных, но не логику показа.
- @fact:B047-THE-HOLE-IS-A-PATTERN **Дыра `list ↔ query_package` оказалась не парой, а узором (замер 2026-08-06).** Один и тот же `lockfile.packages`/`LockedPackage` ради ПОКАЗА пакета обходят как минимум шесть команд `vibe-cli` помимо этих двух — `outdated`, `show effective`, `show purls`, `show features`, `show subskills`, `tree`, — и каждая строит свой пер-пакетный вывод руками. Трёхполевая проекция сабскилла `{path, delivery, describes}` написана в дереве **трижды**: `commands/list.rs:90-98`, `vibe-mcp/src/tools.rs:103-115` и `commands/show/subskills.rs:52-69`.
- @fact:B047-THE-EXPENSIVE-ROW **Самая дорогая строка расхождения — не форма, а ЗНАЧЕНИЕ.** Из четырнадцати общих полей одиннадцать сериализуются по-разному при одинаковом имени (CLI через `skip_serializing_if` — поле пропадает; MCP через `json!` — всегда `null`/`[]`), и это стиль. Но `files_written` считается по-разному по существу: `commands/list.rs:86` отдаёт `to_string_lossy()` без нормализации, `vibe-mcp/src/tools.rs:119` — `to_string_lossy().replace('\\', "/")`. **На Windows две поверхности одной способности печатают разные пути.** Правка меняет опубликованный JSON-вывод CLI, поэтому это не попутный фикс, а решение со своей ценой.
- @fact:B047-THE-CANDIDATE-HOME-DOES-NOT-EXIST **Кандидат на дом нормы из пункта (3) опровергнут измерением, и это меняет цену пункта.** Запись предлагала посадить норму «рядом с четырёхслойной моделью SPEC/ENGINE/DRIVER/DEPLOYMENT» в семье `ENGINE-CONFORM`/`GUIDE-*`. Такой модели **в пакете дисциплины нет**: слов `DRIVER` и `DEPLOYMENT` во всём `vibevm/vibepacks/org.vibevm.ai-native/` — ноль вхождений. Ближайшее, что там есть, — двухчастное расщепление «core ships prompt content and neutral engine crates / the runnable half ships in each stack» (`core-ai-native/v0.8.0/README.md:19,21-24`), без слоёв и без имён.
- @fact:B047-WHERE-THE-MODEL-ACTUALLY-LIVES **Где четырёхслойная модель живёт на самом деле — и почему это отдельная находка.** Её единственная сжатая формулировка стоит в `vibevm/vibespecs/WAL.xml` (`##WAL-C-PERIMETER`): «SPEC в `core-ai-native`, ENGINE в его крейтах, DRIVER в CLI стека, DEPLOYMENT у потребителя». Кроме неё — два кампанийных документа (`PHASE-C-BATCH-PLAN.md §4.5`, `PHASE-C-RESUME.md`), план кампании и около десяти harvest-разборов, пересказывающих её прозой. **Ни один из этих домов не долговечен и ни один не проверяется:** WAL переписывается целиком каждым wind-down и выведен из корпуса кампании конструктивно (`progress.toml`), `campaigns/**` исключён структурно, harvest — свидетельства, а не контракты. То есть несущий архитектурный закон проекта пересказан не менее двенадцати раз и не имеет ни одного верифицируемого дома — ровно тот класс «одна истина, много домов», который эта программа и убирает. Пункт (3) поэтому распадается надвое: **сперва дать модели дом, потом уточнять её нормой о поверхностях.** Дом — по-прежнему решение владельца, но список кандидатов теперь другой: `00-MANIFESTO.xml` (языко-независимое ядро, где уже живёт лексика слоёв), `mechanisms/MCP-CORE-v0.1.md` (механизм одной из двух поверхностей), либо новый механизм-документ.
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
| @fact:B050-ANCHOR **anchor** | `GUIDE-AI-NATIVE-RUST.xml` `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` (:72) — клауза «custom clippy lints name the rule and the remedy»; та половина третьего канала, которую батч 3 не построил |
| @fact:B050-LOCATOR **locator** | цензус `harvest/e13-r2-custom-lints-census.md` §Q3/Q8: `dylint`, `declare_lint!`, `LateLintPass`, `rustc_private` — 0 в исходниках; `rust-toolchain.toml:2` пинит `channel = "stable"` и это ЕДИНСТВЕННЫЙ toolchain-файл дерева; на машине владельца (замер 2026-08-04) нет ни nightly, ни `cargo-dylint` |
| @fact:B050-SEVERITY **severity** | P3 |
| @fact:B050-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-04: «добавить в BACKLOG.md с низким приоритетом»**, то есть dylint сейчас НЕ строим и обещание гайда НЕ снимаем; вопрос про nightly возвращается вместе с этой строкой |
| @fact:B050-FILED **filed by** | босс-дизайн батча 3 волны Б, 2026-08-04 (`vibevm/vibespecs/design/new-rule-classes.xml` §3) |

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


### B-091 — язык схем не выражает тип, которым пишет наш собственный писатель {#b-091}

| | |
|---|---|
| @fact:B091-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/common/PROP-044#machinery`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#machinery) §4.2 — JTD выбран **за бедность**, чтобы агент не мог выразить опасное; это первый случай, когда бедность режет по живому полю |
| @fact:B091-LOCATOR **locator** | `crates/vibe-index/src/types/repomd.rs:45` — `File { size: u64, … }`; схема `schemas/index/e1/repomd.jtd.json`, поле `size`, вынужденно `uint32` |
| @fact:B091-SEVERITY **severity** | P2 |
| @fact:B091-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С2.1, воркер R2-WIRE64 + боссов хвост)**: правило «целые шире 32 бит едут десятичной строкой» внесено в PROP-044 §4.2b (`##M-WIDE-INTEGERS-AS-STRINGS`); `size` — строка на проводе (писатель `wire_decimal_u64` с каноном: цифры, без знака — включая `+`, боссова заплатка, — без ведущих нулей; сгенерированный ридер `String`), схема истинна, корпус переведён, перелом — `formats/breaks/003.md`; красное: старая числовая форма отвергается ридером громко (`the_generated_reader_refuses_the_old_numeric_size_form`); замер воркера: других >32-битных полей в схемных типах нет, у Timestamp — RFC3339 |
| @fact:B091-FILED **filed by** | фаза Ф4.1b кампании packages-2026-09, посадка схемы манифеста каталога, 2026-08-15 — **заведена под номером `B-056`, перенумерована 2026-08-19** (`##B091-THE-RENUMBERING`) |

- @fact:B091-THE-FACT **Факт, измеренный, а не вычитанный.** У JTD (RFC 8927) **нет 64-битного целого вовсе**: запинённый `jtd-codegen 0.4.1` отвергает и `uint64`, и `int64` как `InvalidType`. Писатель при этом объявляет `size: u64`. Значит схема, описывающая наш собственный формат, не может описать тип, которым он пишется.
- @fact:B091-WHY-IT-MATTERS **Почему это не косметика.** Сегодня схема говорит `uint32`. Это ИСТИННОЕ утверждение ниже 2³² и ЛОЖНОЕ на нём и выше: наш писатель способен породить документ, который наша же схема объявляет невалидным. Пойман не отказ читателя (он-то как раз громкий и честный), а то, что **описание формата расходится с форматом**.
- @fact:B091-WHAT-IS-SETTLED **Что уже решено и не пересматривается.** Режим отказа: размер ≥ 2³² роняет сгенерированный ридер громко, а не усекает молча — закон 1 PROP-044 соблюдён, неверный ответ за верный не выдаётся ни при каком значении.
- @fact:B091-OPTIONS **Три выхода, все измерены, ни один не бесплатен.** *(а)* Оставить `uint32` и объявить потолок свойством формата — тогда честнее сузить и КОД до `u32`, чтобы писатель не мог породить непредставимое, а отказ переехал с читателя на писателя, то есть с потребителя на нас. *(б)* Кодировать размер строкой — провод меняется, зато диапазон неограничен; в допубликационном режиме D13 перелом бесплатен и безмиграционен. *(в)* Пустая схема `{}` — **отвергнута замером**: генератор выдаёт `Option<Value>`, теряя И тип, И обязательность поля.
- @fact:B091-WHY-NOT-NOW **Почему не сейчас.** Каждый выход трогает либо опубликованный провод, либо продуктовый тип, и оба — за пределами слайса, который это нашёл. Записано, чтобы решение было принято отдельно и осознанно, а не попутно.
- @fact:B091-THE-CLASS **Класс, а не случай.** Это первое место, где выбор «JTD за бедность» столкнулся с типом, который дерево уже использует. Оно не последнее: всякое поле шире 32 бит — `u64`, `i64`, время как эпоха в миллисекундах — придёт к той же развилке. Ответ стоит дать один раз и общий.
- @fact:B091-THE-RENUMBERING **Почему у строки сменился номер, и почему сменился именно у неё.** С 2026-08-15 номер `B-056` носили ДВЕ строки: закрытая — про множественное наследование контрактных документов — и эта. Один якорь `#b-056` стоял на обеих, так что ссылка попадала на первую по файлу, то есть на закрытую, объявляющую себя закрытой; а ратифицированная спека ([`PROP-005` §2.12](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#types)) указывала этой координатой именно на живую развилку. **Двигали ту, за которую никто не держится, и это счёт, а не вкус:** имя `B-056` называют **24** авторских файла (генерируемые кэши и карта не в счёте), из них **восемнадцать** — закрытую строку, включая семь мест `crates/vibe-spec/**`, `PROP-035` §7.3 и дизайн волны Г; живую называют шесть, и лишь **три** из них живые указатели — этот файл, `PROP-005` §2.12 и докблок `crates/vibe-index/src/types/mod.rs`, все три перенацелены той же посадкой. Ни один из одиннадцати якорей этой строки не цитируется вне `BACKLOG.md`; шесть имён `##B056-…`, которые цитируются снаружи (`-RULED`, `-COST`, `-ODR-PARALLEL`, `-ORDER-AND-RECURSION`, `-REPLACE-BECOMES-A-FLAG`, `-GLOBS-DEGRADE-NATURALLY`), принадлежат закрытой строке и **не разрешались уже до этой правки** — они умерли, когда та строка была сведена к могильнику. Перенумерование живой не отняло у них ничего.
- @fact:B091-WHAT-THE-RENUMBERING-DELIBERATELY-LEAVES-ALONE **Что намеренно НЕ тронуто, чтобы отсутствие правки не читалось как недосмотр.** Три оставшихся упоминания живой строки — датированные записи, а не указатели: могильник фазы Ф4 в свёрнутом плане change-native и две находки `harvest/` (`plan-mortality-d`, `prop005-drift-b`). Датированная запись говорит, что было верно на её дату, и переписывать её задним числом значит стирать замер; вместо этого маршрут назад поставлен там, куда такая ссылка приземляется, — в могильник [`B-056`](#b-056) выше по файлу. **И вопрос разрядности эта правка не решает** — она чинит адрес, а не спор: развилка `u64` против `u32` остаётся владельческой ровно в том виде, в каком записана выше.

### B-095 — правило «шире 32 бит — строкой» встретило счётчики, и нужен владельческий рулинг о границе класса {#b-095}

| | |
|---|---|
| @fact:B095-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/common/PROP-044#machinery`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#machinery) `##M-WIDE-INTEGERS-AS-STRINGS` — общее правило, отчеканенное на `repomd.size` |
| @fact:B095-LOCATOR **locator** | admin-счётчики `crates/vibe-index/src/server/routes/admin.rs` (`uptime_seconds`, `requests_total`, `mutations_total` — Rust `u64`, JSON numbers, домены НЕограничены); count-поля CLI/HTTP-конвертов (`returned`/`offset`/`limit`/`hit_count`/`update_available` — Rust `usize`/`u32`-класс, домены структурно малы); схемы говорят `uint32` с записанным расхождением — по ДОправильному прецеденту |
| @fact:B095-SEVERITY **severity** | P2 |
| @fact:B095-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (рекомендация босса принята владельцем «должно быть сделано»; воркер codex P2-FSYNC)**: граница класса — честность домена, не Rust-тип (ADR-part `##WIDE-INTEGERS-CLASS-BOUNDARY` в PROP-044 §4.2b). Исполнение: 3 админ-счётчика (`uptime_seconds`/`requests_total`/`mutations_total`) → канонические десятичные строки через `wire_decimal`, схемы string; 22 счётных поля обеих семей конвертов → честный `uint32` с проверенной конверсией `wire_count::checked_u32` (никаких молчаливых `as`; переполнение — громкая ошибка: CLI `WireCountOverflow`, HTTP 400/500 в `ErrorResponse`); deviation-описания схем умерли; довесок — axum-404 отвечает тем же `ErrorResponse`-конвертом (E2E). 375 тестов vibe-index зелёные; wire-diff честно доложил 11 schema + 1 corpus сдвигов (public=false, reporting)  |
| @fact:B095-FILED **filed by** | марафон 1.0.0, ревью посадки R2-ENVELOPES-HTTP, 2026-08-20 |

- @fact:B095-SUT **Суть.** Правило `##M-WIDE-INTEGERS-AS-STRINGS` читается буквально по Rust-типу: «поле шире 32 бит едет строкой». Конверты B-079 несут два разных подкласса таких полей: (а) **неограниченные тоталы** — `requests_total`/`uptime_seconds` долгоживущего сервера honestly перерастут 2³²; (б) **структурно малые счётчики** — `hit_count`/`offset`/`limit`, чей домен ограничен размером ответа. Обе посадки отгрузили uint32-с-записанным-расхождением (стиль СТАРОГО repomd) — то есть правило в его собственный первый месяц не применено к десятку новых полей.
- @fact:B095-FORK **Развилка (владельцу).** *(а)* строгое правило по Rust-типу: все такие поля — строкой (единообразно; уродует счётчики; ломает свежие конверты — бесплатно при public=false); *(б)* уточнить правило: строкой едут поля с НЕограниченным доменом (size/total/epoch-время), структурно-ограниченные счётчики законно `uint32` с громким отказом за пределом — тогда admin-тоталы конвертируются, счётчики остаются; *(в)* сузить Rust-типы счётчиков до `u32` и снять вопрос на стороне писателя. Вариант (б) требует правки текста правила в PROP-044 — владельческий акт.
- @fact:B095-ALSO **Той же строкой.** Пустой axum-404 на незнакомом маршруте против общего `ErrorResponse` зарегистрированных обработчиков — унификация требует server-правки; named, not masked (отчёт R2-ENVELOPES-HTTP §Отклонения-3).

### B-096 — не-TUI поверхность `vibe prefs` живёт с пустой схемой {#b-096}

| | |
|---|---|
| @fact:B096-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/PROP-041#cli`](spec/modules/vibe-cli/PROP-041-prefs-tui.md) — контракт CLI-поверхности prefs |
| @fact:B096-LOCATOR **locator** | `prefs ui` строит схему из `TreeSettings`; общий CLI-загрузчик (`list`/`get`/`check`) оставляет `Schema::new()` — на чистых настройках `list` = «0 keys», `get vibe.tree.palette` не видит built-in default `rose-pine`, `check` диагностирует собственный объявленный ключ TUI как `unknown setting` |
| @fact:B096-SEVERITY **severity** | P2 |
| @fact:B096-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер claudez P2-PREFS)**: `prefs::load` строит схему вызовом той же `build_schema()` (`tree/tui/settings.rs`, теперь pub(crate) и документирована как общесхемная точка всего CLI) — одна точка истины, без второго списка ключей; все шесть не-TUI сабкоманд получили схему через `Loaded` без правок в себе; `prefs ui` перестал строить дубликат. Живой transcript: чистый temp — list 6 ключей с default-origin, get видит `rose-pine`, check чист по объявленному ключу и ловит опечатку; 4 новых юнита; строка ALPHA-NOTES снята |
| @fact:B096-FILED **filed by** | марафон 1.0.0, пре-прогон ручных тестов, 2026-08-20 |

### B-097 — `vibe tree --quiet` не quiet {#b-097}

| | |
|---|---|
| @fact:B097-ANCHOR **anchor** | `vibe tree --help`: «Reduce output to a single summary line» |
| @fact:B097-LOCATOR **locator** | наблюдение пре-прогона: `--quiet` печатает то же 51-строчное plain-дерево, exit 0 — флаг объявлен и не применён |
| @fact:B097-SEVERITY **severity** | P3 |
| @fact:B097-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (та же посадка P2-PREFS)**: `--quiet` даёт одну summary-строку (`plain::summary_line`: project/packages/roots), гасит plain/TUI/vibeterm; `--json` остаётся json (clap `conflicts_with`, живьём exit 2); юнит на форму строки; строка ALPHA-NOTES снята |
| @fact:B097-FILED **filed by** | марафон 1.0.0, пре-прогон ручных тестов, 2026-08-20 |

### B-098 — origins-поверхность prefs: контракт называет флаг, бинарь даёт сабкоманду {#b-098}

| | |
|---|---|
| @fact:B098-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/PROP-041#cli`](spec/modules/vibe-cli/PROP-041-prefs-tui.md) — форма `--show-origins` |
| @fact:B098-LOCATOR **locator** | `vibe prefs --show-origins` → exit 2 (`unexpected argument`); фактическая поверхность — `vibe prefs show-origins [key]`, работает корректно |
| @fact:B098-SEVERITY **severity** | P3 |
| @fact:B098-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, босс)**: истинной признана сабкоманда (живёт, работает, гибче — принимает опциональный ключ); четыре цитаты флаг-формы в PROP-040 (§8 заголовок, `##show-origins-req`, `##prefs-command`) и PROP-041 (`##provenance-view`) переписаны на `vibe prefs show-origins [key]`, якоря не тронуты; суд confirmed×3, оба файла запечатаны; строка ALPHA-NOTES снимается этой же посадкой |
| @fact:B098-FILED **filed by** | марафон 1.0.0, пре-прогон ручных тестов, 2026-08-20 |

### B-099 — help `vibe prefs set` несёт устаревший phase-lore {#b-099}

| | |
|---|---|
| @fact:B099-ANCHOR **anchor** | `vibe prefs set --help` |
| @fact:B099-LOCATOR **locator** | текст «basic write; phase 2.7 enriches with diff-from-default + comment-preserve» — а enriched-персистентность уже реализована и вызывается этим же кодом |
| @fact:B099-SEVERITY **severity** | P3 |
| @fact:B099-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (та же посадка P2-PREFS)**: help говорит правду об enriched write (diff-from-default + comment-preserve + role-marker через atomic rewrite), сверено по `vibe-settings/src/persist/mod.rs` |
| @fact:B099-FILED **filed by** | марафон 1.0.0, пре-прогон ручных тестов, 2026-08-20 |

### B-100 — `vibe progress --campaign <id>` трактует голый id как путь от cwd и молча создаёт мусорную зону {#b-100}

| | |
|---|---|
| @fact:B100-ANCHOR **anchor** | `vibe progress mirror --campaign packages-2026-09` (из корня репозитория) |
| @fact:B100-LOCATOR **locator** | справка обещает «campaigns/\<id\>/», но значение резолвится как путь относительно cwd: голый `packages-2026-09` создал `\<root\>/packages-2026-09/run/mirror` (492 файла) вместо ошибки «зона не найдена»; рабочее написание — полный путь `campaigns/packages-2026-09` |
| @fact:B100-SEVERITY **severity** | P3 |
| @fact:B100-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-22 (CLI-волна границы facts/progress, `##BOUNDARY-CLI`)**: общий резолвер campaign-аргумента (`commands/progress/grounding.rs::resolve_campaign`) резолвит голое имя через `campaigns/\<id\>/`, несуществующий id — громкая ошибка со списком существующих зон и без единой записи на диск (тестом проверено отсутствие мусорного каталога); путь с разделителем — прежнее поведение |
| @fact:B100-FILED **filed by** | замер §5a PROP-045, 2026-08-22 |

- @fact:B100-SUT **Суть.** Создание state-зоны — побочный эффект записи; неверно разрешённое значение флага не должно материализовывать новую зону молча. Обнаружено при подготовке суждения: mirror отработал «успешно» в свежесозданный мусорный каталог в корне хоста, и только несовпадение пути выдало промах. Мусор удалён руками в той же сессии.

### B-078 — «у JTD нет nullable» оказалось ложью, и записанное расхождение журнала стало разрешимым {#b-078}

| | |
|---|---|
| @fact:B078-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/common/PROP-044#laws`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#laws) — закон 1: описание формата не вправе расходиться с форматом; и §4.2, где схема названа описанием ПРОВОДА |
| @fact:B078-LOCATOR **locator** | `schemas/journal/e1/journal.jtd.json`, `definitions.event.mapping.removed.optionalProperties.version` — против `schemas/list_report.jtd.json:56-62`, где `"nullable": true` уже применён |
| @fact:B078-SEVERITY **severity** | P2 |
| @fact:B078-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С2; босс до красного + воркер codexrunner R2-JOURNAL-STRICT до зелёного).** Схема: `removed.version` — `properties` + `nullable:true` (провод писателя не изменился ни байтом — он ВСЕГДА писал `"version": null`, корпусный шард это нёс). Боссово красное вскрыло дефект глубже строки: serde на плоском `Option` молча превращает отсутствующий ключ в `None` — required-nullable в генерате не был строгим. Воркер построил общий закон: ветка `RequiredNullable` эмитит `deserialize_with = behaviour::required_nullable::deserialize` (значение→Some, null→None, отсутствие ключа→ошибка), общий хелпер с юнитами; замер нашёл ДЕВЯТЬ таких полей (journal.version, list_report.boot_snippet, шесть index-cli), все писатели проверены безусловными — закон применён ко всем. `formats/breaks/004.md`; красное→зелёное `the_old_absent_version_form_is_refused`; golden_corpus зелёный. `##B078-THE-CLASS` исполнен буквально: «утверждение о пределе языка меряется как всякое другое» |
| @fact:B078-FILED **filed by** | фаза Ф4.2b кампании packages-2026-09, посадка Ф4.2b-5 (`e4b46885`), 2026-08-17 |

- @fact:B078-THE-FACT **Факт, найденный воркером, а не перечитыванием.** Схема журнала утверждала дословно: «JTD (RFC 8927) has no nullable type and no present-but-null form». Это ложь: `nullable` — штатный флаг JTD, и `schemas/list_report.jtd.json` его УЖЕ использует на поле `boot_snippet`, а запинённый `jtd-codegen 0.4.1` честно эмитит на него `Option<…>` БЕЗ skip-атрибута, то есть ровно форму «ключ есть всегда, значение бывает `null`».
- @fact:B078-HOW-IT-SURFACED **Как всплыло.** Инвентарь пакета Ф4.2b-5 перечислял два класса опциональных полей (скаляр и структура) и не знал третьего. Воркер нашёл его в дереве, поверил дереву против пакета (§0.9 транспортного закона) и обработал по общему правилу шага. Ни одно перечитывание схемы этого бы не дало: утверждение выглядело обоснованным, а опровержение лежало в соседнем файле.
- @fact:B078-WHY-IT-MATTERS **Почему это не косметика.** Пока схема говорит `optionalProperties`, она ШИРЕ типа: допускает отсутствующий ключ, которого наш писатель не пишет никогда, и сгенерированный читатель при обороте теряет `null`. Записанное «recorded, not resolved» держалось на несуществующем пределе языка.
- @fact:B078-THE-FIX **Правка, которая делает схему истинной.** `version` переезжает из `optionalProperties` в `properties` с `"nullable": true`. После этого схема описывает писателя дословно, а сгенерированный тип получает `Option<Version>` без skip — то есть провод сгенерированного и рукописного совпадает побайтово там, где сегодня расходится.
- @fact:B078-WHY-NOT-NOW **Почему не сделано попутно.** Перенос поля между формами задевает ветку слоя преобразований, обрабатывающую required-nullable, и меняет провод формата журнала. Оба — предмет отдельного шага с собственным красным доказательством, а не хвоста чужой посадки.
- @fact:B078-THE-CLASS **Класс, а не случай.** Проверить стоит КАЖДОЕ место, где записано «язык этого не выражает»: два таких утверждения в этом дереве уже проверены — B-091 (64-битного целого у JTD действительно нет; строка носила номер `B-056`, когда это писалось) подтвердилось, это — нет. Утверждение о пределе языка есть утверждение и меряется так же, как всякое другое.

### B-079 — семь конвертов ответа `vibe-index` не описаны схемой и не стоят в реестре форматов {#b-079}

| | |
|---|---|
| @fact:B079-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/common/PROP-044#our-formats`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#our-formats) — `##FMT-UNINVENTORIED`: «The seven CLI `--json` reports (scripts and agents parse them) … each is "something a foreign parser reads", each gets a registry entry, a schema, an epoch and a corpus» |
| @fact:B079-LOCATOR **locator** | `crates/vibe-index/src/cli/{get,list,search,capabilities,purls,outdated,verify}.rs` — рукописные `Envelope`/`Row`-типы; в `formats/REGISTRY.toml` нет ни одной записи `cli-*` для `vibe-index` (реестр несёт семь `cli-*` отчётов `vibe` и `cli-package-tree`) |
| @fact:B079-SEVERITY **severity** | P2 |
| @fact:B079-DISPOSITION **disposition** | `closed` — **закрыта целиком 2026-08-20 (марафон, слайс С2.5, две посадки).** CLI-половина (e3ec088e, воркер claudez R2-ENVELOPES-CLI + боссов хвост wire_decimal): 7 конвертов `--json` — схемы/реестр/8 корпусов/parity. HTTP-половина (воркер codexrunner R2-ENVELOPES-HTTP, yolo+quiet): **10 новых конвертов** `/v1/**` (list≠search структурно, общий DeleteResponse с required-nullable version, оба health-состояния, настоящий error-корпус из ApiError; сырые file-маршруты честно переиспользуют существующие `index-*` форматы, gzip признан transport-encoding, `/metrics` — Prometheus-текст вне JSON-инвентаря) + 11 корпусов + 5 parity-тестов включая клиентскую связку (`SearchResults`/`PurlLookupResults` читают серверные корпуса). Незамаскированные границы обеих посадок вынесены в [`B-095`](#b-095): ширина count/total-полей против правила строк и пустой axpower-404 fallback |

| @fact:B079-FILED **filed by** | фаза Ф6.2 кампании packages-2026-09, нарезка Ф6.2b, 2026-08-17 |

- @fact:B079-THE-FACT **Факт, измеренный замером Ф6.2 §4, а не выведенный.** Ни один конверт ответа `vibe-index` — ни CLI `--json`, ни HTTP `/v1/**` — не входит в `formats/REGISTRY.toml` и не имеет схемы: ни JTD, ни JSON Schema. При этом их читают чужие: клиент `vibe-registry` расшифровывает HTTP-ответы собственными view-типами (`crates/vibe-registry/src/index_client/wire.rs`), а `--json`-отчёты по построению адресованы скриптам и агентам.
- @fact:B079-WHY-IT-MATTERS **Почему это не косметика.** Контракт называет такую поверхность форматом ПРЯМЫМ ТЕКСТОМ, и весь смысл реестра — что незарегистрированный формат обязан быть невыразим в системе типов (§4.1). Здесь он не просто выразим — он живёт и растёт: **Ф6.2b добавляет ему поле `unavailable`**, то есть поверхность, которую никто не инвентаризовал, расширяется тем же приёмом, которым была написана.
- @fact:B079-WHY-NOT-NOW **Почему не сделано попутно, и это довод, а не отговорка.** Описать схемой ОДНО новое поле внутри рукописного конверта значит завести ВТОРУЮ законную запись одной поверхности — ровно то, что дерево запрещает («одна вещь пишется одним способом»). Описать все семь целиком — отдельная работа, и §8a контракта маршрутизирует её владельцу: «absent → this task mints a format, which is an owner-visible act — stop and surface».
- @fact:B079-WHAT-THE-GATE-SAID **Что при этом сказал гейт, и почему это ценно.** Рехет `wire-derive` покраснел ровно на этом росте (`vibe-index` 24 → 25 файлов с рукописным serde) и предложил ДВА рецепта: описать схемой либо поднять число, назвав тип. Выбран второй — но выбор пришлось произнести, и именно это гейт покупает: рост рукописного провода перестал быть бесшумным.
- @fact:B079-THE-CLASS **Класс, а не случай.** Тот же вопрос стоит над HTTP-ответами сервера (`server/routes/**`) — они тоже рукописные и тоже незарегистрированные, и их читает наш собственный клиент. Ответ стоит дать один раз и общий: инвентарь поверхностей ОТВЕТА, а не по одному конверту за фазу.

### B-080 — резолвер выбирает версию, которой сборка пользоваться не может {#b-080}

| | |
|---|---|
| @fact:B080-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/common/PROP-044#machinery`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#machinery) — §4.5: читатель, которому не хватает возможностей, карантинит запись, и отказ всплывает В ТОЧКЕ ПРИМЕНЕНИЯ |
| @fact:B080-LOCATOR **locator** | `crates/vibe-registry/src/index_client/wire.rs` (`VersionEntryView` — одно поле `version`), `crates/vibe-registry/src/index_client/mod.rs` (`list_versions` отдаёт весь список), `crates/vibe-registry/src/git_package_registry/lookup.rs` (выбор `Latest`/`Req` по этому списку) |
| @fact:B080-SEVERITY **severity** | P2 |
| @fact:B080-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (слово владельца «сделать тест/пример» исполнено тем же вечером; воркер claudez P2-MIRROR)**: красный тест сперва ДОКАЗАЛ слепоту (Latest вернул карантинную 2.0.0 с `must_understand=["b080-test-capability"]`), затем починка: реестр возможностей читателя переехал в общий дом `vibe-core::capabilities` (честный `UNDERSTOOD=[]`; vibe-index переведён на него дёшево, 13 quarantine-тестов зелёные без правки), `VersionEntryView` вырос до `IndexVersion` с декларациями (старые файлы читаются как прежде), выбор идёт через `pick_version`: карантин → прежняя лестница по выжившим; warn называет пакет/версию/возможность, `AllVersionsUnusable` называет лучшую пропущенную и чего не хватило (отказ в точке применения, §4.5), multi-registry walk на ней останавливается; capability никогда не маскирует честное `NoMatchingVersion`. 250 тестов vibe-registry зелёные |
| @fact:B080-FILED **filed by** | фаза Ф6.2 кампании packages-2026-09, замер перед нарезкой Ф6.2c, 2026-08-17 |

- @fact:B080-THE-FACT **Факт, измеренный, а не заподозренный.** Слово `must_understand` не встречается во всём `crates/vibe-registry/**` ни разу (греп с контрольной проверкой: тот же шаблон ловит десятки хитов в `vibe-index`). Клиентский `VersionEntryView` несёт РОВНО ОДНО поле — `version`. Значит клиент физически не может увидеть объявление, ради которого оно на проводе есть.
- @fact:B080-WHERE-THE-WRONG-ANSWER-IS-MADE **Где именно рождается неверный ответ.** `list_versions` берёт СЫРОЙ `by-name/<name>.json` и отдаёт все версии подходящей группы; резолвер выводит «latest» сам — новейшая без `pre` из этого списка. Карантинная версия в списке есть (файл честно её несёт), фильтра нет, и выбор падает на неё.
- @fact:B080-WHY-THE-EARLIER-READING-WAS-WRONG **Почему прежняя формулировка мимо.** Записанное подозрение звучало как «клиент читает слепой агрегат `latest_stable` из файла». Он его не читает вовсе — поля нет у view. Слепота не в АГРЕГАТЕ, а в ВЫБОРЕ: клиент считает «latest» самостоятельно и потому наследует слепоту не от файла, а от собственного незнания предиката.
- @fact:B080-WHY-SERVER-SPEECH-DOES-NOT-FIX-IT **Почему речь сервера этого не лечит.** Ф6.2c учит говорить ВЫЧИСЛЯЮЩИЕ ответ маршруты. Клиент по быстрому пути читает не их, а сырой файл (`by-name/{name}`), который по решению Р55.1 остаётся дословным и честным. То есть сколько бы сервер ни объяснял, этот путь клиента его не слышит.
- @fact:B080-WHAT-THE-FIX-COSTS **Чего стоит починка, и почему она не хвост.** Чтобы клиент спросил предикат, реестр возможностей ЧИТАТЕЛЯ (`UNDERSTOOD` и `is_usable`, сегодня в `crates/vibe-index/src/index/quarantine.rs`) обязан стать видимым и `vibe-registry` — то есть переехать в дом, который видят оба. Это решение об архитектуре крейтов, а не правка глагола: `vibe-registry` сегодня от `vibe-index` не зависит вовсе, и заводить такое ребро ради предиката — отдельный разговор.
- @fact:B080-TRIGGER **Триггер.** Первая работа, берущаяся за клиентскую сторону карантина, ЛИБО первая реальная возможность в `UNDERSTOOD` (пока набор пуст, дефект достижим только через рукотворный каталог; с первой же настоящей возможностью он становится живым).
- @fact:B080-THE-CLASS **Класс, а не случай.** Всякий раз, когда предикат ЧИТАТЕЛЯ живёт в одном крейте, а читателей несколько, вопрос «чей это дом» встаёт заново. Ответ стоит дать один раз и общий — вместе с B-079, где тот же вопрос стоит об инвентаре поверхностей ответа.

### B-081 — словарь видов живёт не в одном месте: у него второй рукописный экземпляр {#b-081}

| | |
|---|---|
| @fact:B081-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/common/PROP-044#agents`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#agents) — G9: «a vocabulary exists in exactly one schema; both wire sides, Rust types, docs and prose lists are generated from it» |
| @fact:B081-LOCATOR **locator** | `crates/vibe-wire/src/behaviour/vocabularies.rs:17-26` (`as_str` — match без wildcard) и `:33-43` (`known()` — `[PackageKind; 6]` хардкодом), против единственного объявленного дома `formats/vocabularies.json` (узел `package_kind`) |
| @fact:B081-SEVERITY **severity** | P2 |
| @fact:B081-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 той же посадкой R4-VOCAB, БЕЗ нарушения Р14/Р24**: генератор по-прежнему не эмитит поведение — тихую копию `known()` (и громкую `as_str`) пинит паритет-тест против объявленного дома; седьмое значение в доме теперь красит тест с именем каждой отставшей копии, т.е. P5-сценарий «правка одной схемы» стал ловимым за один прогон. Замер-сюрприз: второго enum-зеркала в vibe-index уже нет (re-export generated shared PackageKind) — прежний parity retired честно |
| @fact:B081-FILED **filed by** | сверка предсказаний P1–P6, кампания packages-2026-09, 2026-08-17 |

- @fact:B081-THE-FACT **Факт, добытый пробой, а не чтением.** Предсказание P5 гласило: «добавление 7-го вида = правка ОДНОЙ схемы + перегенерация». Проба добавила седьмое значение `probe` в `formats/vocabularies.json` и только туда, прогнала `cargo xtask codegen` (exit 0, изменился ровно ОДИН сгенерированный файл, ветки `Unknown` целы) — и `cargo check` упал: `error[E0004]: non-exhaustive patterns: &shared::PackageKind::Probe not covered` на `vocabularies.rs:17`. Одной правки НЕ хватает.
- @fact:B081-TWO-COPIES-NOT-ONE **Экземпляров два, и они разного класса.** `as_str` — **громкий**: компилятор ловит пропуск немедленно, и это худший исход из хороших. `known()` — **тихий**: он хардкодит длину `[PackageKind; 6]`, седьмой вид просто не попадёт в него, сборка останется зелёной, а справка, сводные таблицы и сообщения CLI молча потеряют вид. Тихая копия и есть настоящий дефект.
- @fact:B081-WHY-IT-IS-NOT-A-TAIL **Почему это не хвост чужой посадки.** Очевидная починка — «пусть генератор эмитит `as_str` и `known()`» — прямо противоречит уже принятым решениям Р14 и Р24 этой же кампании: генератор эмитит ФОРМУ, а не ПОВЕДЕНИЕ, и на этом основании ему запрещено выводить `Default` и `is_empty`. Значит вопрос не «как быстро починить», а «где честный дом отображения вид→строка, если сгенерированный serde-слой уже его несёт».
- @fact:B081-WHAT-IS-ALREADY-TRUE **Что уже верно и переписывать не надо.** Открытость словаря цела: `Unknown(String)` сохраняется, незнакомое значение проезжает провод дословно. Ломается не терпимость, а утверждение «в одном месте».
- @fact:B081-THE-CLASS **Класс, а не случай.** Ровно та болезнь B7, которую фаза Ф4.2c закрывала структурно: словарь с двумя записями, совпадающими по случайности. Здесь она вернулась этажом выше — в рукописном слое поведения, который Ф4.2c сама и завела рядом со сгенерированным деревом.

### B-082 — гейт окна перелома обещает больше, чем меряет {#b-082}

| | |
|---|---|
| @fact:B082-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/common/PROP-044#machinery`](vibevm/vibespecs/common/PROP-044-change-native-formats.xml#machinery) — §4.7: «while it is closed, CI refuses any change under `schemas/**` or `formats/**`» |
| @fact:B082-LOCATOR **locator** | обещание — `formats/EPOCHS.toml:18-19` и сообщение самого гейта `xtask/src/wire_diff.rs:263`; измеряемое — `corpus_shift` (`xtask/src/wire_diff.rs:122-130`), чей `git diff` идёт по ОДНОМУ пути `formats/corpora/` |
| @fact:B082-SEVERITY **severity** | P2 |
| @fact:B082-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер codex P2-FSYNC)**: щуп сдвига расширен до обещанного — `git diff … HEAD -- schemas/ formats/`; три класса путей (schema/corpus/other-formats) с честными счётчиками в каждом говорящем вердикте и отдельным ремонтным классом «schema changed without a corpus move»; Quiet-строка говорит правду о наблюдаемом периметре; чистый классификатор затестирован таблично. Живая приёмка воркера: правка description → вердикт называет схему и класс; байт-восстановление по SHA-256 → Quiet. Ложная зелень дня флипа `public` закрыта ДО флипа — как и требовала строка |
| @fact:B082-FILED **filed by** | сверка предсказаний P1–P6, кампания packages-2026-09, 2026-08-17 |

- @fact:B082-THE-FACT **Факт, измеренный прогоном и перемеренный боссом.** Проба временно изменила ОДНО описание в `schemas/hello/e1/hello.jtd.json` без записки перелома и прогнала `cargo xtask wire-diff`: exit 0, вердикт Quiet. Босс перемерил чтением: слово `schemas` встречается в `wire_diff.rs` РОВНО ОДИН раз — в тексте сообщения (`:263`), и ни разу в щупе; аргумент `git diff` — единственный путь `formats/corpora/`.
- @fact:B082-WHY-IT-MATTERS **Почему это не педантизм.** Гейт СООБЩАЕТ, что закрытое окно отвергает изменения под `schemas/**`, и оператор, прочитавший сообщение, будет полагаться именно на это. Правка схемы, не дошедшая до перегенерации корпуса, невидима щупу при ЛЮБЫХ значениях флагов — то есть обещание, на которое опираются, в машине не реализовано. Это тот же класс, что «инструмент мерит не ту величину», только на этаж выше: инструмент меряет УЖЕ, чем заявляет.
- @fact:B082-WHY-IT-IS-QUIET-TODAY **Почему сегодня тихо.** В допубликационном режиме (`public = false`) любой сдвиг даёт максимум отчётный вердикт, поэтому расхождение не может проявиться отказом. Оно проявится в день, когда владелец переключит `public = true`, — и проявится как ложная зелень, худший вид.
- @fact:B082-TWO-HONEST-EXITS **Два честных выхода, оба стоят решения.** *(а)* Расширить щуп на `schemas/**` и `formats/**` — тогда сообщение станет правдой, но всякая правка описания схемы начнёт требовать записки. *(б)* Сузить ОБЕЩАНИЕ до того, что меряется («сдвиг байтов корпуса»), — тогда правда восстанавливается без ужесточения, но закрытое окно перестаёт быть тем, чем его объявляет PROP-044 §4.7. Выбор между «привести машину к прозе» и «привести прозу к машине» здесь не косметический: он решает, что означает «период стабильности».

### B-083 — `[[registry]].index_url` описан спекой, не существует в манифесте, а секция строгая {#b-083}

| | |
|---|---|
| @fact:B083-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor`](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#form-factor) — `##INDEX-URL-CONFIG` (TOML-пример) и `##INDEX-URL-DEFAULT` («Default: `<registry-url>/index`») |
| @fact:B083-LOCATOR **locator** | `crates/vibe-core/src/manifest/project.rs:109` (`#[serde(deny_unknown_fields)]`) и `:110` (`RegistrySection` — поля `name`/`url`/`ref`/`naming`/`auth`/`token_env`/`enabled`, без `index_url`); единственный существующий источник — `crates/vibe-registry/src/index_client/mod.rs` (`index_url_for` → `VIBEVM_INDEX_URL_<REGISTRY>`) |
| @fact:B083-SEVERITY **severity** | P2 |
| @fact:B083-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С3.1, воркер R3-INDEXURL + боссов хвост).** Ключ `index_url: Option<String>` в `RegistrySection` (одним типом — проектная И машинная колонки, `deny_unknown_fields` сохранён); лестница `resolve_index_url`: env → ключ → умолчание `<url>/index`, ровно `"none"` на явной ступени выключает индекс намеренно (раньше `none` в env «работал» лишь аварией URL-разбора); резолвер прикрепляет клиент по лестнице. Красное: дословный TOML-блок спеки падал разбором до правки (текст ошибки в отчёте воркера), живёт после. Боссов хвост: enum обогащён провенансом ступени (`IndexUrlSource`) и обе env-only поверхности vibe-cli (`search`, `search --purl`) переведены на лестницу — Absent на УГАДАННОМ умолчании больше не шумит в unreachable (реестр без индекса — не ошибка), а `--full-scan`/`unconfigured` ветки живы через `Disabled`/тихий фолл-даун. Статусы PROP-005 подняты той же посадкой. Остаток (не блокер): `vibe-publish post_hook` держит приватную env-only копию — операторский контекст, кандидат на общий хелпер при следующей правке того крейта |
| @fact:B083-FILED **filed by** | полный проход PROP-005 против дерева, кампания packages-2026-09, 2026-08-18 |

- @fact:B083-THE-FACT **Факт, перемеренный боссом после воркера.** Слово `index_url` в `crates/vibe-core/src` встречается **ноль** раз; `RegistrySection` несёт `deny_unknown_fields`. Значит TOML-блок из спеки — не «пример, который пока не действует», а **отказ разбора**: пользователь, скопировавший его в свой `vibe.toml`, получит ошибку загрузки манифеста.
- @fact:B083-WHY-IT-IS-NOT-COSMETIC **Почему это не косметика.** Спека несла `@status:impl/done` на конфигурационной поверхности, которой нет, — то есть худший класс устаревания: не «отстала», а «утверждает построенное». Читатель планирует вокруг такого утверждения, а не перепроверяет его.
- @fact:B083-THE-SUBSTITUTE-IS-WEAKER **Заменитель существует и он слабее по существу.** `VIBEVM_INDEX_URL_<REGISTRY>` — переменная окружения: она пошелловая и попрогонная, не едет ни с проектом, ни с локфайлом, и потому не закрывает требование, а лишь делает его отсутствие незаметным на одной машине.
- @fact:B083-TWO-HONEST-EXITS **Два честных выхода, и оба принадлежат владельцу.** *(а)* Завести ключ в `RegistrySection` (и тогда пример становится правдой, а env остаётся переопределением). *(б)* Понизить требование: объявить, что расположение индекса — операторская настройка среды, а не свойство проекта, — и тогда из спеки уходит `index_url`, а не статус. Выбор решает, ЧЬЯ это настройка: проекта или машины.

### B-084 — `repomd.json` объявлен единственной точкой доверия, и ни один потребитель по нему ничего не сверяет {#b-084}

| | |
|---|---|
| @fact:B084-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout`](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#layout) — `##REPOMD-TRUST-POINT`; рядом `#trust` `##TWO-INTEGRITY-LAYERS` |
| @fact:B084-LOCATOR **locator** | `crates/vibe-registry/src/index_client/` — ноль вхождений `sha256`/`etag` (контроли: `crates/vibe-registry/src` → 43, `crates/vibe-index/src/index` → 39); потребительский путь `index_client/mod.rs:293` идёт за `by-name/<name>.json` напрямую; сверка живёт только в операторском глаголе `crates/vibe-index/src/cli/verify.rs` |
| @fact:B084-SEVERITY **severity** | P2 |
| @fact:B084-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С3.2)**: `##REPOMD-TRUST-POINT` сужен до существующего — точка доверия оператора (`vibe-index verify`), потребительская половина названа отложенной прямо в факте и стоит в леджере (`deferrals.md#release-1-0`, вместе с вопросом дома предиката B-080); соседние честные факты `##NO-SHIPPED-CONSUMER-VERIFIES…`/`##WHAT-IS-NEVERTHELESS-PROTECTED…` не тронуты — они и были правдой |
| @fact:B084-FILED **filed by** | полный проход PROP-005 против дерева, кампания packages-2026-09, 2026-08-18 |

- @fact:B084-WHAT-IS-EXPOSED-IS-METADATA **Что именно обнажено — метаданные в пути, а не содержимое.** `content_hash` сверяется с фактически скачанными байтами в момент забора независимо от того, как выбрана версия ([PROP-005 §2.3](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#truth)), поэтому подменённый индекс способен НАПРАВИТЬ потребителя не туда, но не способен заставить его поставить непроверенные байты. Подменённый же `by-name`-файл читается как пришёл.
- @fact:B084-THE-ZERO-IS-MEASURED **Ноль измерен, а не предположен.** Тот же греп по соседним домам даёт 43 и 39 попаданий — значит молчание принадлежит клиенту, а не инструменту. Это ровно то требование к пустому выводу, которое кампания оплатила на собственной приёмке.
- @fact:B084-THE-VERB-IS-A-DIFFERENT-PARTY **Глагол `verify` — другая сторона в другое время.** Он сверяет каталог у ОПЕРАТОРА, на его диске, по его расписанию; утверждение же спеки — про ПОТРЕБИТЕЛЯ в момент установки. Считать одно исполнением другого значит закрыть требование чужой работой.
- @fact:B084-WHY-IT-BLOCKS-THE-SIGNING-STORY **Почему это держит и подпись.** `##repomd-pattern-heritage` объявляет манифест-с-контрольными-суммами путём к GPG без переархитектуры; путь этот стоит на том, что потребитель ЧИТАЕТ манифест. Пока не читает — подпись подписывала бы документ, которого никто не открывает.

### B-085 — HTTP-триггер переиндексации обещан тремя якорями, не построен, и на него указывает документированный хук {#b-085}

| | |
|---|---|
| @fact:B085-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http`](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#http) — блок маршрутов; плюс `#reindex` `##TWO-REINDEX-MODES` и `##TRIGGER-HTTP`; потребитель обещания — `#wire-up` `##WIRE-POST-RECEIVE` |
| @fact:B085-LOCATOR **locator** | `crates/vibe-index/src/server/mod.rs:54-107` — единственный `Router::new()` крейта, 16 путей, админ-поверхность одна (`:105` `GET /v1/admin/status`); `crates/vibe-index/src/server/routes/admin.rs` — единственный обработчик `status`; хит `admin/reindex` по `crates/` — ноль |
| @fact:B085-SEVERITY **severity** | P2 |
| @fact:B085-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С3.3): ручка отозвана.** `##TRIGGER-HTTP` переписан отзывом (операторский глагол; DoS-довод §2.16 распространён на админ-токен), `##TWO-REINDEX-MODES` и `##THE-ADMIN-SURFACE-IS-ONE-ROUTE` фиксируют развилку решённой, §11-хук переведён с `curl POST` на вызов CLI-глагола (ssh/локально), устаревший комментарий `server/mod.rs:104` («reindex POST lands in slice 6») заменён отзывом; возврат маршрута — по нужде первого оператора (`deferrals.md#release-1-0`) |
| @fact:B085-FILED **filed by** | полный проход PROP-005 против дерева, кампания packages-2026-09, 2026-08-18 |

- @fact:B085-WHY-IT-IS-NOT-A-DOC-BUG **Почему это не документационная опечатка.** §11 спеки даёт оператору готовый `post-receive`-хук, который делает `curl -X POST …/v1/admin/reindex`. Оператор, исполнивший рецепт, получает молча ничего не делающий хук: сервер ответит 404, `curl -sf` промолчит, `|| echo` напечатает «non-fatal». То есть обещание не просто не исполнено — оно ВСТРОЕНО в рабочий рецепт, который выглядит настроенным и не работает. Это ровно та форма, которую PROP-005 §2.16 сама называет худшей: «hook, который настроен и молча не приходит, выглядит как hook, которому нечего делать».
- @fact:B085-THE-CODE-SAYS-IT-WAS-DEFERRED **Дерево само помнит отсрочку.** `server/mod.rs:104` несёт комментарий «Admin (read-only in slice 5; reindex POST lands in slice 6)». Слайс 6 закрылся — с write-маршрутами `/v1/packages` и авторизацией, — а reindex не приехал и никто этого не заметил, потому что спека утверждала обратное.
- @fact:B085-TWO-HONEST-EXITS **Два честных выхода.** *(а)* Построить маршрут — он дёшев (обработчик поверх существующего `reindex`-кода) и делает документированный хук правдой. *(б)* Отозвать: объявить, что переиндексация — операторский глагол, а не сетевая операция, убрать маршрут из §2.10, переписать §11-хук на вызов CLI по ssh или на host-native планировщик. Довод за (б) не в экономии: сетевой триггер тяжёлой перестройки — это DoS-рычаг, который §2.16 уже отказалась давать вебхукам по этой самой причине, и давать его админ-токену стоит решать сознательно.

### B-086 — машины приоритетов конфигурации у `vibe-index` нет, а спека описывает четырёхуровневую {#b-086}

| | |
|---|---|
| @fact:B086-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#config`](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#config) — `##CONFIG-PRECEDENCE`; смежно `#cli` `##CLI-SURFACE` (`--data-dir` / `$VIBE_INDEX_DATA_DIR` / умолчание `./vibe-index-data`) |
| @fact:B086-LOCATOR **locator** | греп `config.toml` по `crates/vibe-index/src` → **0** (контроль: `checkpoint.json` тем же грепом → хиты в `index/checkpoint.rs`, `scanner/org_cache.rs`, `scanner/org_walk.rs`); греп `VIBE_INDEX` → ровно один хит `scanner/git_cli.rs:19` (`VIBE_INDEX_GIT`); `data_dir` — обязательный позиционный во всех 15 глаголах |
| @fact:B086-SEVERITY **severity** | P2 |
| @fact:B086-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (марафон, слайс С3.4, воркер R3-LADDER + боссов хвост): машина построена, дефект-середина устранена.** `crates/vibe-index/src/config.rs`: четыре ступени (флаг → env `VIBE_INDEX_*` → строгий `<data-dir>/state/config.toml` → умолчание), каждое разрешение несёт значение И источник; видимое — глагол `vibe-index config <data-dir> [--json]` с `[source: …]` на каждом члене; неизвестный ключ файла — громкий отказ с перечнем известных (ровно анти-«середина» из `##B086-THE-MIDDLE-STATE-IS-THE-DEFECT`). Обе блудные переменные вписаны в семью: `VIBE_INDEX_GIT` — член, `VIBE_LOG` — записанный легаси-синоним НИЖЕ `VIBE_INDEX_LOG` (узкое бьёт широкое) с сохранённым EnvFilter-языком. Подключены 4 показательных члена (log-level, git, api-base, dump-format); ~17 остальных — механическое продолжение по шаблону, с двумя владельческими развилками (булев диалект значений; имена для коллизии `--limit`) — перечислены в отчёте R3-LADDER и LOG ТЗ. `data-dir` в лестницу не входит по `##CLI-SURFACE` (позиционный; файл живёт внутри него). Красное: файл с `dump-format=json` до правки игнорировался, после — применяется с названным источником; env бьёт файл, флаг бьёт env — живыми прогонами и тестами |
| @fact:B086-FILED **filed by** | полный проход PROP-005 против дерева, кампания packages-2026-09, 2026-08-18 |

- @fact:B086-THE-MIDDLE-STATE-IS-THE-DEFECT **Дефект — не отсутствие лестницы, а середина.** Ни один из четырёх уровней, кроме первого, не существует, и при этом две переменные окружения ЕСТЬ (`VIBE_INDEX_GIT`, `VIBE_LOG`) — но они не члены никакой цепочки и спекой не названы. Оператор, читающий §3.5, ищет `state/config.toml` и `VIBE_INDEX_*`-семейство; оператор, читающий код, находит две несвязанные переменные. Оба описания неполны, и ни одно не даёт предсказать поведение.
- @fact:B086-THE-DATA-DIR-CASE-IS-THE-SAME-QUESTION **`--data-dir` — та же развилка, а не отдельная.** Спека обещала флаг, переменную и умолчание; дерево требует позиционный аргумент. Позиционный — защищает от запуска не над тем каталогом (умолчание `./vibe-index-data` тихо создало бы индекс где угодно), и это довод ЗА сегодняшнюю форму. Но тогда норма должна сказать именно это, а не описывать удобство, которого нет.
- @fact:B086-WHY-A-LADDER-COSTS-MORE-THAN-IT-LOOKS **Почему «просто построить лестницу» — не очевидный выход.** Четырёхуровневый резолвер требует, чтобы КАЖДЫЙ флаг с умолчанием знал своё имя переменной и свой ключ в файле, и чтобы `--help` умел показать, откуда взялось действующее значение; без последнего лестница добавляет способ ошибиться, не добавляя способа проверить. Именно это делает вопрос владельческим, а не хвостом чьей-то посадки.

### B-087 — атомарная запись не синхронизирует каталог, а протокол это обещает {#b-087}

| | |
|---|---|
| @fact:B087-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#persistence`](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml#persistence) — `##ATOMIC-WRITE-PROTOCOL` шаг 4 `##AW-FSYNC-DIR` |
| @fact:B087-LOCATOR **locator** | `crates/vibe-index/src/index/persistence.rs:25-37` — весь протокол: `create_dir_all` → tmp → `write_all` → `sync_all` (ФАЙЛА) → `rename`; шага «fsync каталога» нет. Все четыре `sync_all` крейта — файловые: `index/persistence.rs:33`, `journal/store.rs:56`, `lockfile.rs:82`, `server/auth.rs:116` |
| @fact:B087-SEVERITY **severity** | P2 |
| @fact:B087-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер codex P2-FSYNC)** консервативной веткой: код догнал ратифицированный протокол — `fsync_parent_dir` в `index/persistence.rs` после rename (POSIX всерьёз, ошибки наружу; Windows no-op с причиной — ровно как разрешает скобка самого шага), все production-писатели проекций идут через один `atomic_write`. Замер воркера сузил locator: journal — append-in-place без tmp→rename (протокол его не описывает), lockfile/auth `sync_all` — тестовые fixtures на read-only путях. PROP-005 `##AW-FSYNC-DIR` → impl/done, факт-гэп переписан записью закрытия, суд confirmed×2, запечатано |
| @fact:B087-FILED **filed by** | полный проход PROP-005 против дерева, кампания packages-2026-09, 2026-08-18 |

- @fact:B087-WHAT-THE-MISSING-STEP-BUYS **Что именно покупает пропущенный шаг.** `fsync` файла делает долговечными БАЙТЫ; долговечной ЗАПИСЬ каталога (то, что имя указывает на новый inode) делает `fsync` каталога. Без него на POSIX возможен исход, при котором после сбоя питания новые байты целы, а имя не указывает ни на них, ни на прежнюю версию. Окно узкое и реальное; именно поэтому шаг и был записан в протокол четвёртым, а не забыт.
- @fact:B087-WHY-IT-MATTERS-MORE-HERE-THAN-USUAL **Почему здесь это весит больше обычного.** Каталог — проекция, и его потеря лечится перестройкой из журнала; но журнал пишется ТЕМ ЖЕ семейством операций (`journal/store.rs:56` — тоже только файловый `sync_all`). То есть отсутствие шага задевает и слой, восстановлением из которого чинится всё остальное.
- @fact:B087-THE-HONEST-NARROWING **Как выглядит честное сужение, если выбран второй путь.** Не «удалить шаг 4», а сказать вслух: атомарность здесь — свойство `rename` в пределах живой системы, а не гарантия переживания внезапной потери питания; кто нуждается во втором, монтирует данные соответствующе. Удалённый без такой фразы шаг превращает гарантию в фольклор — она продолжит жить в головах, не имея дома.

### B-088 — «спека не ссылается на ЗАКРЫТЫЙ план» держится процедурой, чекера нет {#b-088}

| | |
|---|---|
| @fact:B088-ANCHOR **anchor** | `campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md` §11.3 — сам план называет этот пробел и объявляет его дефером; запись здесь существует ровно потому, что план подлежит смерти, а пробел — нет |
| @fact:B088-LOCATOR **locator** | ссылок на план в `spec/**` **четыре**, а не две: `vibevm/vibespecs/common/PROP-044-change-native-formats.xml:11` (`##PURPOSE`), `:536` (`##SOURCES`), `:3` (комментарий элемента `<status>`) и `vibevm/vibespecs/research/schema-evolution-2026-08/README-PROVENANCE.xml:22`; панель `tools/self-check.sh` шага, который бы их заметил, не содержит |
| @fact:B088-SEVERITY **severity** | P3 |
| @fact:B088-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (партия P3)**: триггер сработал (change-native план «ИСПОЛНЕНО И СВЁРНУТО» 2026-08-18) — реестр `campaigns/CLOSED-PLANS.toml` (schema 1, первый жилец) + шаг панели `spec does not link to a closed plan` (тупой fixed-string греп path+basename по spec/**, `vibevm/vibespecs/WAL.xml` исключён по построению, отсутствие реестра — само красное); ТРИ жанра различены по форме: обе живые ссылки (из записанных четырёх две умерли эволюцией раньше) пере-словлены боссом в жанр-3 — имя+дата, без пути. Доказано срабатыванием на хосте: синтетическая ссылка → красный с file:line и коучингом, чистое дерево → зелёный. Примечание посадки: код гейта — воркер codex, принят диффом двух файлов; его самовольный коммит/панель/правки боссовых зон отвергнуты и записаны инцидентом лейна |
| @fact:B088-FILED **filed by** | §11 плана change-native форматов, вынесено из умирающего документа, 2026-08-18 |

- @fact:B088-WHY-A-NORM-WITHOUT-A-CHECKER-DRIFTS **Почему процедуры мало.** Стоячий закон проекта — план временный, спека постоянна, и утверждения цитируют якоря спек, а не строки плана. Сегодня этот закон исполняется тем, что кто-то о нём помнит в момент закрытия кампании. Правило без чекера есть ЖЕЛАНИЕ — собственный закон дисциплины, — и первым же признаком дрейфа станет спека, ссылающаяся на файл, которого больше нет: ссылка не сломается громко, она просто перестанет разрешаться у читателя, который пойдёт по ней через полгода.
- @fact:B088-THE-GATE-IS-CHEAP **Форма гейта.** Реестр закрытых планов (или маркер закрытия в шапке самого файла плана) → греп по `spec/**` на путь каждого закрытого плана → красный на любом хите. Стоимость — десяток строк в панели.
- @fact:B088-THE-GATE-MUST-DISTINGUISH-ACTIVE-FROM-CLOSED **Разделяющая клауза, без которой гейт вреден.** Ссылки на АКТИВНЫЙ план законны и нужны: их держат `vibevm/vibespecs/WAL.xml` и `CONTINUE.md`, и запрещать их значило бы запретить чекпойнту называть работу, которая идёт. Значит гейт различает два состояния плана, а не ищет слово «campaigns». И у него есть исключение по построению: `vibevm/vibespecs/WAL.xml` — летучий чекпойнт, а не контракт, и меряется другим мерилом.
- @fact:B088-THE-COUNT-WAS-ALREADY-WRONG-BEFORE-THE-GATE-EXISTED **Пробел уже стоил числа, и это лучший довод за гейт из возможных.** Сам §11.2 плана говорит «убираются ОБЕ ссылки», называя `##PURPOSE` и `##SOURCES`. Греп по `spec/**` даёт **четыре**: к двум названным добавляются комментарий элемента `<status>` в шапке того же PROP-044 и markdown-ссылка из `vibevm/vibespecs/research/schema-evolution-2026-08/README-PROVENANCE.xml`. Сессия, исполнившая §11.2 буквально, убрала бы две, оставила бы две и объявила план несвязанным — то есть норма без чекера уже промахнулась на собственном единственном применении, ещё до того, как её применили.
- @fact:B088-THE-GATE-DISTINGUISHES-THREE-THINGS-NOT-TWO **И различать гейту нужно ТРИ вещи, а не две.** Кроме «активный против закрытого» есть третий жанр: **датированное упоминание провенанса** — «этот контракт вводился в работу вместе с тем планом», — которое остаётся правдой и после смерти плана и удалять которое значит стирать историю. Разделяющий вопрос: ведёт ли текст читателя ТУДА ЗА содержанием (живой указатель — умирает вместе с планом) или сообщает, ОТКУДА взялось это (провенанс — переживает его). Оговорка, которая делает разницу материальной: markdown-ссылка провенанса становится битой в тот день, когда файл плана действительно удалят, поэтому провенанс-упоминание должно ссылаться на коммит, а не на путь.
- @fact:B088-WHY-P3 **Почему P3, а не P2.** Пробел не может выстрелить, пока ни один план волны не закрыт; в день закрытия он выстрелит четырьмя ссылками, все из которых уже названы поимённо в строке `locator`. Опасность не в первом случае, а в десятом, когда планов закрыто пять и никто уже не помнит, какие спеки на них ссылались.

### B-090 — `mirror --check` объявляет «all targets in sync» над хостом, который отстал {#b-090}

| | |
|---|---|
| @fact:B090-ANCHOR **anchor** | [PROP-016](vibevm/vibespecs/common/PROP-016-source-mirrors.xml) и флоу `source-mirrors`: `##EVERY-HOST-IS-A-DOWNSTREAM-READ-REPLICA`, `##HISTORY-REACHES-A-HOST-ONLY-THROUGH-THE-FAN-OUT` |
| @fact:B090-LOCATOR **locator** | `xtask/src/mirror/mod.rs:319-345` — `verify()` кладёт в `drift` только `Drift` и `Missing`; `Behind` печатает строку на `:326` и в вердикт НЕ входит, поэтому управление доходит до `:344` `println!("mirror --check: all targets in sync.")` и функция возвращает `Ok`. Классификация — `xtask/src/mirror/probe.rs:65-79`: неравный предок ⇒ `Behind`, док называет это «healthy» |
| @fact:B090-SEVERITY **severity** | P2 |
| @fact:B090-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер claudez P2-MIRROR)** консервативной веткой развилки: exit-семантика не тронута (`Behind` не краснеет — его чинит обычный fast-forward), но хвост честен всегда — `check_tail` (чистая fn, 5 таблично-оффлайн тестов, инвариант «хвост никогда не содержит „in sync“ над Behind/Drift»); Behind-хвост: `local ahead -- N target(s) behind, fast-forward needed`; смешанный Drift+Behind называет оба класса; док «healthy» скоупирован до exit-вердикта. Вторая ветка (краснеть на Behind) остаётся доступной владельцу одной правкой check_tail |
| @fact:B090-FILED **filed by** | раскатка посадки B-089, наблюдено живьём 2026-08-18 |

- @fact:B090-OBSERVED-LIVE-NOT-INFERRED **Наблюдено, а не выведено.** Прогон 2026-08-18 напечатал `BEHIND gitverse at becbcd9` и следующей же строкой `mirror --check: all targets in sync.` при локальном `02c4902` и коде выхода **0**. Причина затем прочитана в коде, а не угадана по выводу: обе стороны — печать строки и вычисление вердикта — сходятся в `verify()`, и `Behind` в вердикт не попадает по построению.
- @fact:B090-THE-TAIL-IS-THE-VERDICT **Почему это весит больше, чем кажется.** Дисциплина этого репозитория читает вердикт ИЗ ХВОСТА — так предписано и панели, и всякому гейту, потому что строка выше хвоста может принадлежать шагу, который ничего не решает. Здесь хвост утверждает то, чего не утверждает ни одна строка над ним. Гейт, чей последний вывод противоречит собственной строке, хуже отсутствующего: отсутствующий не даёт ложной уверенности.
- @fact:B090-WHERE-IT-BITES **Где выстреливает.** Быстрый старт WAL и рецепт завершения сессии оба гоняют `cargo xtask mirror --check` и записывают «зеркала синхронны». Хост, отставший на два коммита, при этом даёт зелёный хвост — то есть чекпойнт фиксирует состояние, которого нет, и следующая сессия строит на нём. Ровно тот жанр, что B-082: гейт обещает больше, чем меряет.
- @fact:B090-BOTH-READINGS-ARE-DEFENSIBLE **Почему это развилка, а не дефект.** `Behind` действительно здоровое состояние: хост не разошёлся, его чинит обычный fast-forward, и краснеть на нём значило бы краснеть между посадкой и раскаткой всегда. Но тогда вердикт обязан сказать «ни один хост не РАЗОШЁЛСЯ» — это правда и это другое утверждение, — и вынести число отставших в хвост, чтобы хвост нёс весь ответ. Выбор между «сузить обещание» и «краснеть на отставании» владельческий, потому что второе меняет, когда раскатка обязана случиться.
- @fact:B090-WHAT-IS-NOT-BROKEN **Что при этом работает верно и трогать не надо.** Недоступный хост даёт громкий отказ: тот же прогон при закрытом соединении вышел с кодом 1 и назвал причину, отделив «хост не ответил» от «хост разошёлся» — то есть различение недостижимости и дрейфа уже построено и правильно. Речь только о `Behind`.

### B-092 — цитата факта, набранная формой определения, заводит вторую координату с тем же именем {#b-092}

| | |
|---|---|
| @fact:B092-ANCHOR **anchor** | флоу `addressable-specs`: `##EACH-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR` — у факта ровно один авторитетный якорь. Формы намеренно разные: **определение** — тот префикс, которым открывается всякая размеченная строка этого файла; **цитата** — двойная решётка с именем (`##ИМЯ`) |
| @fact:B092-LOCATOR **locator** | `BACKLOG.md`, три места, каждое — ячейка `severity`/`disposition`, у которой уже есть СОБСТВЕННОЕ определение: `:177` (`B076-SEVERITY` цитирует `B076-WHY-NOT-P1`), `:209` (`B074-SEVERITY` → `B074-WHY-IT-BORDERS-P1`), `:319` (`B067-DISPOSITION` → `B067-WHY-DEFERRED`) |
| @fact:B092-SEVERITY **severity** | P3 |
| @fact:B092-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (та же посадка P2-CODEGEN + босс)**: настоящая цена уплачена — закон `DuplicateId` расширен (обе строки в сообщении; definition-form в inline-коде считается координатой — ровно класс этой строки; `##ИМЯ`-цитата и fences — нет), и три места в BACKLOG переведены боссом на цитатную форму `##ИМЯ` (uniq-d-греп теперь пуст). Известная граница, записанная честно: `BACKLOG.md` вне наблюдаемого корпуса `progress.toml` (include-only) — чекер стережёт корпус, BACKLOG проверяется прямым парсом/грепом; включение BACKLOG в корпус = судейская экономика, решение владельца |
| @fact:B092-FILED **filed by** | починка координаты B-091 (бывшей B-056), 2026-08-19 — найдено тем же грепом на дубли, которым проверялась собственная правка |

- @fact:B092-THE-FACT **Факт, найденный контролем, а не глазом.** Рецепт: `grep -oE '@fac[t]:[A-Za-z0-9_-]*' BACKLOG.md | sort | uniq -d` — класс символов в середине слова стоит там намеренно, чтобы сам рецепт не попал в собственную выборку, и это первое следствие дефекта, который он ищет. Он даёт ровно три имени; тот же счёт на версии из `HEAD` даёт те же три — значит они не заведены правкой, которая их нашла. Ни одно из трёх не является вторым определением: все три — **ссылки**, набранные формой определения, вида «см. <префикс-определения>`B074-WHY-IT-BORDERS-P1`» там, где адресуемость предписывает `##B074-WHY-IT-BORDERS-P1`.
- @fact:B092-WHY-IT-MATTERS **Почему это не опечатка.** Две формы существуют затем, что делают разное: по определению инструмент ЗАВОДИТ адрес, по цитате — РАЗРЕШАЕТ его. Цитата, набранная формой определения, объявляет один id дважды, и всякий скан, считающий определения, видит две координаты там, где предмет один. Это тот же класс, что B-091, одним уровнем ниже: адрес разрешается не туда, куда целил автор, и молча.
- @fact:B092-NOTHING-COVERS-THIS-FILE **Почему этого никто не поймал: файл не покрыт НИ ОДНИМ из двух инструментов, и это измерено, а не выведено из их молчания.** Судимый корпус кампании его не видит — среди 281 вида в зеркале вида `BACKLOG.md` нет, при контроле на заведомо присутствующем `PROP-005`. И карта спеков его фактов не несёт: греп по `specmap.json` даёт **ноль** вхождений `B090-ANCHOR`, `B091-THE-RENUMBERING` и `B092-THE-FACT`, тогда как `PROP-005` встречается 451 раз; пятнадцать вхождений строки `BACKLOG.md` в карте — упоминания пути в текстах ДРУГИХ документов, а не юниты этого файла. **Поэтому «`specmap --check` чист» о двойной координате не говорит ничего** — гейт чист по тому же праву, по какому чист любой гейт над файлом, которого он не читает. Это была первая формулировка этой строки, и замер её опроверг.
- @fact:B092-WHAT-IS-NOT-ESTABLISHED **Чего замер НЕ установил — чтобы строка не читалась сильнее, чем измерено.** Во что двойное имя обходится на практике, не показано: не найдено ни одного потребителя, который разрешал бы эти имена и получал не то. Показано ровно две вещи — что дубли есть и что их не читает ни один из двух инструментов, которыми этот репозиторий меряет разметку. Отсюда и `severity`: это непокрытая земля, а не лгущий гейт.
- @fact:B092-THE-CLASS **Класс, а не случай.** Три штуки в одном файле — не три опечатки, а форма, которую легко набрать не ту, и проверять стоит каждый размеченный файл, а не этот. Поэтому содержание строки — не «поправить три префикса», а «завести счёт»: `uniq -d` по определениям каждого файла стоит десятка строк, ловит класс целиком и заодно отвечает на вопрос, который эта строка оставила открытым.

### B-093 — `xtask codegen` стирает своё дерево до записи, и оборванный прогон оставляет воркспейс несобираемым {#b-093}

| | |
|---|---|
| @fact:B093-ANCHOR **anchor** | флоу `tool-design-lessons`: `##NEVER-OVERWRITE-A-FILE-THAT-MAY-BE-IN-USE` — «write a new instance directory and flip a pointer». Урок собственный, оплаченный, и здесь нарушен |
| @fact:B093-LOCATOR **locator** | `cargo xtask codegen` (`xtask/src/codegen/mod.rs`, `run_codegen`), выходные каталоги `crates/vibe-wire/src/generated/` и `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/**/src/generated/`; воспроизведение — любой обрыв процесса между очисткой и записью |
| @fact:B093-SEVERITY **severity** | P2 |
| @fact:B093-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер codex P2-CODEGEN)** ровно формой собственного урока: `output_tree.rs` — генерация в sibling `.new-<pid>-<nanos>` на том же томе, публикация только после успеха ВСЕХ групп двумя rename (`live→old`, `new→live`; откат old при падении второго; неоднозначное восстановление никогда не удаляет данные — честная ошибка с именами полных кандидатов), stale-debris убирается следующим прогоном. Живая приёмка: настоящий SIGPIPE (`\| head -3`, PIPESTATUS=101) оставил оба live-дерева нетронутыми (46 файлов, 0 hash-diff), повторный прогон убрал debris и прошёл; 4 lifecycle-юнита; на хосте `check-codegen: clean` через новый путь. Бонус: `GenerationGroup` снял `#[allow(too_many_arguments)]` |
| @fact:B093-FILED **filed by** | посадка факта снятия (`buried`), 2026-08-19 — поймано на себе и воспроизведено намеренно |
| @fact:B093-COST-PAID **что стоило** | один обрыв, одно восстановление, один цикл «почему codegen не может починить то, что сам сломал» |

- @fact:B093-THE-FACT **Факт, воспроизведённый намеренно, а не выведенный из одного случая.** `cargo xtask codegen 2>&1 | head -3` завершается с кодом **101** (SIGPIPE: `head` закрывает канал, процесс умирает на середине), и после этого `git status` по `crates/vibe-wire/` показывает **22 файла** в состоянии `D` — всё сгенерированное дерево, кроме одного каталога, удалено. Замер повторён специально: до обрыва статус показывал одну изменённую строку, после — двадцать две.
- @fact:B093-WHY-IT-IS-NOT-JUST-CLUMSINESS **Почему это дефект инструмента, а не оператора.** Разрушение необратимо СВОИМИ ЖЕ средствами: `xtask` зависит от `vibe-wire`, а `vibe-wire` без своего сгенерированного дерева не компилируется, поэтому повторный `cargo xtask codegen` падает с `could not compile vibe-wire (lib)` — генератор не может восстановить то, что сам стёр. Выход существует ровно один и лежит вне инструмента: `git restore crates/vibe-wire/src/generated/`, а затем регенерация.
- @fact:B093-THE-AMPLIFIER **Усилитель, из-за которого это P2, а не P3.** Воркерам git запрещён абсолютно — транспортный закон кампании не даёт им ни одной git-команды, включая read-only. Значит делегированный исполнитель, чей `codegen` оборвался (обрыв канала, kill, `529` шлюза — все три уже наблюдались в этой кампании), остаётся с несобираемым деревом и **без единственного пути восстановления**. Он не сможет ни починить, ни честно доложить причину, потому что его собственный инструмент к тому моменту тоже не запускается.
- @fact:B093-THE-CLASS **Класс, а не случай.** Пострадавший приём — «очистить каталог, затем записать» — не уникален для `codegen`: всякий шаг, переписывающий каталог целиком, имеет ту же форму. Урок `##NEVER-OVERWRITE-A-FILE-THAT-MAY-BE-IN-USE` написан ровно про это и говорит, что делать: писать в новый каталог и переставлять указатель, после чего обрыв оставляет прежнее дерево нетронутым. Проверить стоит каждый такой шаг, а не только этот.
- @fact:B093-WHY-THE-CATALOG-DOES-THE-SAME-AND-IS-FINE **Возражение, которое читатель выдвинет первым, и ответ на него.** Тот же приём стоит внутри `Index::write_to` (`crates/vibe-index/src/index/memory.rs`): каталог `by-name` очищается перед перезаписью, и комментарий там называет это «scorched-earth» прямо. Разница не в аккуратности, а в наличии **слоя истины**: каталог есть проекция журнала, поэтому оборванная запись чинится повторным прогоном любой мутации или `rebuild`, и порядок «истина раньше производного» специально построен так, чтобы обрыв оставлял журнал без каталога, а не наоборот. У сгенерированного кода такого слоя нет — за ним стоит только git, до которого воркеру ходу нет. **Стирание перед записью безопасно ровно настолько, насколько восстановим стёртый артефакт**, и это разделяющий вопрос для каждого места этого класса. @status:spec/plan
- @fact:B093-THE-INTERIM-RULE **Правило до починки, стоящее ноль.** Вывод `codegen` не пропускают через `head`, `grep -m`, `tail -n` или любой другой рано закрывающийся канал — только в файл с последующим чтением файла. Это тот же приём, что предписан для панели по другой причине (`cmd | tail; echo $?` печатает код ПАЙПА), и здесь цена ошибки выше: там инструмент врал о результате, здесь он ломает дерево.

### B-094 — дымовой список подкоманд `vibe-index` рукописен и проверяется в одну сторону {#b-094}

| | |
|---|---|
| @fact:B094-ANCHOR **anchor** | `vibevm/vibespecs/design/command-nodes.xml` `#cut` — ruling соседнего места: распознавание идёт по derive самого clap, «потому что пометка, без которой можно добавить подкоманду, есть норма без чекера». Здесь та же норма и тот же дефект |
| @fact:B094-LOCATOR **locator** | `crates/vibe-index/tests/help_smoke.rs:11` — `const SUBCOMMANDS`; обе проверки (`root_help_lists_every_subcommand`, `every_subcommand_renders_help`) обходят ЭТОТ список и утверждают лишь «перечисленное есть в help» |
| @fact:B094-SEVERITY **severity** | P3 |
| @fact:B094-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-20 (BACKLOG-волна P2, воркер claudez P2-FSYNC)** дешевле, чем строка думала: clap уже был обычной зависимостью, а CLI — в lib-поверхности; `cli::command()` (CommandFactory) отдаёт то самое дерево, из которого рендерится help, тест итерирует `get_subcommands()` с derive-вычислимым исключением `!is_hide_set()` и проверяет ОБА включения + непустоту набора; рукописная `const SUBCOMMANDS` удалена. Приёмка срабатыванием: фиктивная `b094-probe` вошла в дым БЕЗ единой ручной правки списков (при старом const прогон прошёл бы, её не заметив), удалена бесследно |
| @fact:B094-FILED **filed by** | посадка глагола `yank`, 2026-08-19 |

- @fact:B094-THE-FACT **Факт, установленный срабатыванием, а не чтением.** Список проверяется включением в одну сторону: `SUBCOMMANDS ⊆ help`. Обратное включение — `help ⊆ SUBCOMMANDS` — не проверяет никто, поэтому подкоманда, появившаяся в `--help` и не попавшая в список, гейту невидима. Именно это и произошло с `yank`: он собрался, зарегистрировался, вышел в `--help`, и **ни один тест не покраснел**; строка в списке добавлена рукой постфактум.
- @fact:B094-WHY-IT-MATTERS **Чего гейт не делает при этом.** Второй тест, `every_subcommand_renders_help`, гоняет `<sub> --help` по тому же списку — то есть новая подкоманда не получает даже дымовой проверки своего help'а. Ценность теста тем самым не растёт с ростом CLI, хотя ровно для этого он и написан («the regression gate that every later slice's CLI addition must keep green» — его собственная шапка).
- @fact:B094-THE-FIX **Форма починки, и почему она не однострочная.** Вывести список из `clap::CommandFactory` (`Cli::command().get_subcommands()`), после чего оба теста покрывают ЛЮБУЮ будущую подкоманду без правки. Интеграционный тест линкуется с крейтом снаружи и видит только его публичный API плюс dev-зависимости, поэтому `clap` придётся объявить в `[dev-dependencies]` — а объявление зависимости двигает `Cargo.lock`. Отсюда `open`, а не «сделано попутно».
- @fact:B094-THE-CLASS **Класс, а не случай.** Всякий рукописный перечень, дублирующий то, что программа и так знает о себе, дрейфует — вопрос лишь в том, заметят ли. Проверить стоит каждое такое место: соседний `vibe-cli` держит одноимённый инвариант (`every_subcommand_renders_help`), и вопрос, выведен он или тоже написан рукой, этой строкой НЕ отвечен.

### B-042 — тестовая кодовая база для TCG-замеров: далёкое будущее, сейчас не строим {#b-042}

| | |
|---|---|
| @fact:B042-ANCHOR **anchor** | `TCG-ORACLE-GO-v0.1.xml` `##QUANTITIES-ARE-CAMPAIGN-MEASURED` — аннотация 2026-08-02 несёт это решение прямо в тексте; тот же вопрос ждёт Rust/TS-замеров семьи F-215 |
| @fact:B042-LOCATOR **locator** | bench-станки всех трёх стеков готовы и параметризованы на корпус потребителя; Go-корпуса нет; растовый и TS-корпуса в `research/tcg-bench/` малы (9 и 7 кейсов) |
| @fact:B042-SEVERITY **severity** | P3 |
| @fact:B042-DISPOSITION **disposition** | `accepted` — **решение владельца 2026-08-02 (дословно): «создание тестовой кодовой базы, на которой мы будем делать замеры — это какая-то работа на далекое будущее. Например, тестовый код можно было бы сгенерировать через LLM или фаззером. Прямо сейчас мы такую базу делать не будем»** |
| @fact:B042-FILED **filed by** | рулинг предъявления F-167, 2026-08-02 |

- @fact:B042-SUT **Суть.** Замеры производительности TCG-оракулов требуют представительной кодовой базы-корпуса. Решено: не строить сейчас; направление на будущее — генерация корпуса LLM'ом или фаззером. Запись существует, чтобы отсутствие Go-замеров не переоткрывалось как новая находка.
- @fact:B042-STANDING-ANSWER **Стоячий ответ (владелец, 2026-08-02, второй раз):** «замеров нет и нескоро будет, нужно положить куда-нибудь в роадмап и больше не кошмарить меня вопросами "почему нет замеров"». Исполнено: все три complete-цели стеков (rust/go/ts) несут аннотацию «posted, not yet measured» с именем своего bench-станка и ссылкой на эту запись (батч D33); карта развития несёт тот же стоячий ответ. **Вопросы вида «почему нет замеров» владельцу больше не задаются — ответ здесь.**

### B-106 — доисторическая гниль ссылок: 116 файлов несут md-ссылки на никогда-не-существующие/переехавшие цели {#b-106}

| | |
|---|---|
| @fact:B106-ANCHOR **anchor** | свип конверсии K3/K4 (`vibevm/vibespecs/terraforms/CONVERT-SOURCE-APPLICATION-v0.1.xml` ##K2-INVENTORY) — остаток «dangling» после условной перезаписи `.md`→`.xml` |
| @fact:B106-LOCATOR **locator** | 116 файлов с md-ссылками, чьи цели не резолвятся ни в одной форме; выборочная проверка пяти против дофлипового HEAD — все мертвы и там (жанры: пути md-эпохи `spec/flows/wal/*`, кросс-пакетные прозаические указатели `spec/rust/GUIDE-…`, ротация версий vibedeps `…/0.8.0/…`, никогда не построенные `spec/refactorings/*.prompt.md`); полный список — лог свипа сессии 2026-08-24 |
| @fact:B106-SEVERITY **severity** | P3 |
| @fact:B106-DISPOSITION **disposition** | `open` — флип не создал ни одной новой висячей ссылки; чинить гниль — отдельная волна (переадресация на живые дома или датированное provenance-упоминание по имени) |
| @fact:B106-FILED **filed by** | посадка конверсии корпуса, 2026-08-24 |

### B-107 — три exclude-рулинга facts.toml мертвы: пакетный корпус фактов сжался до `.md`-остатка после XML-конверсии {#b-107}

| поле | значение |
|---|---|
| @fact:B107-ANCHOR **anchor** | `facts.toml` `exclude` (рулинги F-071 cards/INDEX, F-080 legacy-projections, F-096 DISCOVERY-PROMPT) против include-глобов `vibevm/vibepacks/**/*.md` |
| @fact:B107-LOCATOR **locator** | панель, шаг `vibe facts check --exhaustive`: три предупреждения «matched no observed file»; корпус наблюдает 98 файлов — пакетные спеки стали `.xml` (K-волны), а include пакетов остался `*.md`-only, так что и исключать нечего; сами файлы живы (`…/cards/INDEX.xml`, `legacy-projections/*.xml`) |
| @fact:B107-SEVERITY **severity** | P2 |
| @fact:B107-DISPOSITION **disposition** | `closed` — **закрыта 2026-08-27 (`f8f197cd`, `c195eae1`)**: корпус наблюдает Markdown и XML в живой раскладке, все 502 судимые записи перенесены один-к-одному (98 прежних расширений + 404 XML), зеркало/кэш/состояние сходятся на 508 файлах, а каждый из шести exclude-паттернов снова матчится хотя бы один раз. 19 548 verdicts и 42 318 evidence-строк сохранены; migration/stability-харнессы 18/18 и 15/15 |
| @fact:B107-FILED **filed by** | посадка переезда раскладки (PROP-052 R6), 2026-08-24 |

### B-108 — contention-тест package-skill запускает таймер до того, как parent разрешает старт {#b-108}

| поле | значение |
|---|---|
| @fact:B108-ANCHOR **anchor** | `crates/vibe-mcp/src/pkgskill/receipt/concurrency_tests.rs` — `two_child_reconciles_converge_from_one_baseline` |
| @fact:B108-LOCATOR **locator** | каждый child пишет `ready-*.marker`, затем на строках 194–198 запускает собственный 10-секундный `BUDGET` ожидания общего `go.marker`; parent ждёт оба ready последовательно и пишет go только на строках 131–133. Под тяжёлой параллельной сборкой ранний child может исчерпать бюджет до release. Наблюдено 2 падения примерно в 19 полных прогонах при R8.1 portability repair; отдельные тёплые повторы затем зелёные, product-law тесты не падали |
| @fact:B108-SEVERITY **severity** | P3 |
| @fact:B108-DISPOSITION **disposition** | `open` — test-harness flake, не product-дефект. Сделать barrier без pre-release wall-clock бюджета либо передавать child exit-reason по каналу, который parent действительно читает; текущий stdout child направлен в null/libtest capture и скрывает причину exit 101 |
| @fact:B108-FILED **filed by** | независимая приёмка R8.1 receipt portability, 2026-08-27 |

### B-109 — Rust R-001 умеет охранять только один registry-файл одного crate {#b-109}

| поле | значение |
|---|---|
| @fact:B109-ANCHOR **anchor** | `conform.toml` `[rust]` R-001 — «cell constructors appear only in the selection registry»; реализация `core-ai-native-conform/src/rules/structure.rs::FlagSites` |
| @fact:B109-LOCATOR **locator** | `conform.toml:28-29` несёт ровно одну пару `registry_file` + `registry_gated_crate`; `core-ai-native-conform/src/config.rs:226-228` моделирует оба члена как `Option<String>`, а `rust-ai-native-conform/src/lib.rs` строит из них ровно один `FlagSites` |
| @fact:B109-SEVERITY **severity** | P3 |
| @fact:B109-DISPOSITION **disposition** | `open` — обобщить Rust-конфигурацию до списка пар registry-file/gated-crate (с громкой миграцией старой одиночной формы), затем перевести `vibe-package-source/src/cells.rs` с временного crate-local exact-set/source-fence RED на общий conform/SARIF-гейт. До этого A15a держит старый гейт на `vibe-cli/src/registry.rs` и отдельно механически охраняет новый registry внутри crate; продуктовая семантика покрыта, долг — в единстве инструмента |
| @fact:B109-FILED **filed by** | R7.4 A15a extraction `vibe-package-source`, 2026-08-28 |

- @fact:B109-ONE-PAIR-CANNOT-EXPRESS-TWO-HOMES **Почему это ограничение движка, а не неверный config.** После выноса install/source-ячеек существуют два законных дома: `vibe-cli/src/registry.rs` всё ещё строит publish-cell, а `vibe-package-source/src/cells.rs` строит resolver/provider cells. Переставить единственную пару на новый crate означает перестать охранять старый; оставить её на CLI означает, что новый дом общий движок не видит. Никакое значение двух строк TOML не выражает обе пары одновременно.
- @fact:B109-INTERIM-IS-MECHANICAL-BUT-NOT-UNIFORM **Временная защита реальна, но не равна закрытию.** Новый crate проверяет точный набор зависимостей и сканирует production-source: каждый разрешённый cell-конструктор обязан жить в `cells.rs`, появление его в другом файле краснит тест. Мутация с конструктором в `source.rs` этот RED убила. Однако результат не входит в общий `conform` finding/SARIF и требует отдельного знания, поэтому долг остаётся открытым.
- @fact:B109-FIX-SHAPE **Форма будущей починки.** Нужна коллекция типизированных пар, а не два параллельных массива: каждая запись атомарно связывает crate с единственным registry-файлом, валидатор отказывает на дубликате crate/пути и строит один `FlagSites` на запись. Старые scalar-ключи должны умереть громко с точным migration hint; молча принимать обе формы означало бы вернуть два источника истины.
