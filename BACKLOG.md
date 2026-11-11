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

### B-242 — `vibe validate` на голом дереве пакета пишет `.vibe/lifecycle.*` в само дерево пакета {#b-242}

| | |
|---|---|
| @fact:B242-ANCHOR **anchor** | правило отката `lease_root` в `crates/vibe-orchestrator/src/install/lease.rs:20-37` (spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM); тесты, закрепляющие поведение, — `crates/vibe-cli/tests/cli_lifecycle_lease.rs:274,296,369` |
| @fact:B242-LOCATOR **locator** | codkit challenge 2026-11-09: `vibe validate` в каталоге пакета вне какого-либо workspace берёт аренду жизненного цикла с корнем в самом каталоге (обнаружение workspace не удалось — откат на выбранный корень) и создаёт там `.vibe/lifecycle.*`; проверка манифеста, по смыслу только читающая, оставляет состояние в исходниках пакета, и оно попадает в его дерево (и в публикацию, если его не исключить) |
| @fact:B242-SEVERITY **severity** | P2 — читающая команда загрязняет дерево пакета; автор пакета вынужден чистить или игнорировать `.vibe/` |
| @fact:B242-DISPOSITION **disposition** | `open` — решение владельца: (a) `vibe validate` на голом дереве пакета делает только читающую проверку манифеста и никогда не входит в аренду жизненного цикла; (b) поправить PROP-054, чтобы дерево вне любого workspace получало собственный дом состояния на корень в пользовательском каталоге состояния |
| @fact:B242-FILED **filed by** | worker, codkit challenge, 2026-11-09 |

### B-241 — `vibe app list` в проекте показывает только домашний каталог проекта; общего вида с глобальными приложениями нет {#b-241}

| | |
|---|---|
| @fact:B241-ANCHOR **anchor** | `crates/vibe-cli/src/commands/app/home.rs` (`select`, `choose`), `crates/vibe-cli/src/commands/app.rs` (`AppCommand::List`) |
| @fact:B241-LOCATOR **locator** | codkit challenge 2026-11-09 (`campaigns/corporate-env-2026-11/LOG.md`): внутри каталога с `vibe.toml` `vibe app list` перечисляет только `vibevm/vibeopt`; глобальные приложения видны лишь при `-g`, который переключает дом, а не добавляет его. Проектный список не говорит, что в пользовательском доме есть установки, и вид «оба дома сразу» (как `vibe shell` кладёт на PATH обе `opt/bin`) отсутствует |
| @fact:B241-SEVERITY **severity** | P2 — пользователь в проекте не видит, что глобальная команда (например `codkit`) установлена и лежит на PATH, и ищет её не там |
| @fact:B241-DISPOSITION **disposition** | `open` — решить форму: строка-подсказка о глобальных установках в проектном списке, или флаг вида «оба дома» с колонкой дома |
| @fact:B241-FILED **filed by** | worker, codkit challenge, 2026-11-09 |

### B-240 — у платформы нет оси libc: на musl-хосте выбирается glibc-вариант {#b-240}

| | |
|---|---|
| @fact:B240-ANCHOR **anchor** | выбор варианта установщика по `platform`/`arch` в `crates/vibe-app` (`HostFacts`, `select`) |
| @fact:B240-LOCATOR **locator** | codkit challenge 2026-11-09: на musl-системе (Alpine) vibe определяет платформу `linux` и берёт glibc-вариант; пакет codkit поэтому кладёт musl-сборку под отдельную платформу `linux-musl`, которую нужно выбрать руками (`--platform linux-musl`) |
| @fact:B240-SEVERITY **severity** | P2 — на musl-хосте установка по умолчанию даёт нерабочий бинарник, и каждый пакет изобретает своё имя платформы для musl |
| @fact:B240-DISPOSITION **disposition** | `open` — добавить определение libc хоста и ось (`libc = "gnu" \| "musl"`) в выбор варианта, с откатом на вариант без оси |
| @fact:B240-FILED **filed by** | worker, codkit challenge, 2026-11-09 |

### B-239 — `needs = ["network"]` отказывает под `--offline`, даже когда все входы шага лежат в drop {#b-239}

| | |
|---|---|
| @fact:B239-ANCHOR **anchor** | проверка `needs` шага установщика (`need_unmet`) в `crates/vibe-app/src/steps` |
| @fact:B239-LOCATOR **locator** | codkit challenge 2026-11-09: шаг с `needs = ["network"]` (соглашение `org.vibevm.apps/mingw-w64`) отказывает под `--offline` с `need_unmet`, хотя каждый его вход уже лежит в `[artifacts] packages` drop или в кэше и сеть не понадобится; пакет codkit поэтому не объявляет потребность в сети вовсе |
| @fact:B239-SEVERITY **severity** | P2 — честное объявление потребности в сети ломает офлайн-установку из drop в закрытой сети; авторы пакетов вынуждены её не объявлять |
| @fact:B239-DISPOSITION **disposition** | `open` — считать `network` выполненной, когда каждый вход шага разрешается локально (drop, кэш, путь пакета), до запуска шага |
| @fact:B239-FILED **filed by** | worker, codkit challenge, 2026-11-09 |

### B-238 — часть тестов vibe-app, vibe-registry и vvm красная на настоящем Windows {#b-238}

| | |
|---|---|
| @fact:B238-ANCHOR **anchor** | `crates/vibe-app` (14 тестов), `crates/vibe-registry` (4 теста), тесты источников релизов в `crates/vibe-cli/src/commands/vvm` (2 теста) |
| @fact:B238-LOCATOR **locator** | прогон на Windows 11 2026-11-09 (корпоративная кампания, `campaigns/corporate-env-2026-11/LOG.md`): эти тесты падают на Windows одинаково на закоммиченном дереве до работы и после неё; ещё 1–2 loopback-HTTP теста vibe-registry сбрасываются под параллельной нагрузкой и проходят в один поток. Интеграционные CLI-наборы собираются только под Unix, поэтому на Windows их заменял сквозной прогон |
| @fact:B238-SEVERITY **severity** | P2 — сквозной сценарий закрытой сети на Windows проходит, но набор тестов на Windows не зелёный и не прогоняется регулярно, так что регрессии Windows ловятся только вручную |
| @fact:B238-DISPOSITION **disposition** | `open` — разобрать по причинам (Unix-допущения тестов против настоящих дефектов), починить настоящие дефекты, сделать Windows-прогон регулярным |
| @fact:B238-FILED **filed by** | центральная сессия, корпоративная кампания, 2026-11-09 |

### B-237 — откат прерванной операции не переживает падение процесса {#b-237}

| | |
|---|---|
| @fact:B237-ANCHOR **anchor** | `crates/vibe-workspace/src/install/rollback.rs` (`InstallRollback`), `crates/vibe-workspace/src/boot_artifacts/inverse.rs` (`BootImages`) |
| @fact:B237-LOCATOR **locator** | отмена до записи lock восстанавливает слоты, boot и agent-файлы из журнала в памяти; если процесс упадёт посреди восстановления, следующий запуск `vibe` его не продолжит. Тот же класс: если упадёт сам `vibe`, helper-процесс native-плагинов завершается по EOF stdin, но его scratch-каталог остаётся |
| @fact:B237-SEVERITY **severity** | P3 — окно узкое (падение во время самого отката), обычная отмена и ошибки восстанавливаются полностью |
| @fact:B237-DISPOSITION **disposition** | `open` — решение владельца 2026-11-08: durable журнал отката «не сейчас, а когда-нибудь в будущем» |
| @fact:B237-FILED **filed by** | центральная сессия VIBE-NEEDS, 2026-11-08 |

### B-236 — битый receipt package-skills обрывает регенерацию boot, а не шаг recover {#b-236}

| | |
|---|---|
| @fact:B236-ANCHOR **anchor** | `crates/vibe-orchestrator/src/world/package_skill.rs:59` — `lower_project_skill_bindings` читает `.vibe/package-skills.toml` при планировании presets |
| @fact:B236-LOCATOR **locator** | тест `cli_lifecycle_package::lifecycle_package::failure::package_binding_failure_stops_every_later_contribution` красный с `3d2a5d8a1` (управляемая активация): повреждённый receipt останавливает регенерацию boot, вместо того чтобы упасть шагом recover в фазе package |
| @fact:B236-SEVERITY **severity** | P2 — отказ правильный, но не в той фазе (PROP-054 `#FAILURE-BY-PHASE`); пользователь видит другую ошибку и другую точку восстановления |
| @fact:B236-DISPOSITION **disposition** | `fixed` 2026-11-08 — планирование presets (`crates/vibe-orchestrator/src/world/package_skill.rs`) сначала судит существующий receipt через `check_project_skill_receipt` (`vibe-agent-projection`); нечитаемый receipt не обрывает планирование и регенерацию boot, а планирует только строку recover, которая в фазе package читает тот же receipt и падает с тем же отказом, останавливая все последующие вклады (PROP-054 `#FAILURE-BY-PHASE`); тест не ослаблен |
| @fact:B236-FILED **filed by** | центральная сессия VIBE-NEEDS, 2026-11-08 (финальный гейт) |

### B-235 — `vibe remap bind-source` не проверен на локальном Git-источнике {#b-235}

| | |
|---|---|
| @fact:B235-ANCHOR **anchor** | `crates/vibe-cli/tests/cli_remap_bind_source.rs` — `only_explicit_new_binding_records_newly_observed_git_commit` (`#[ignore]`) |
| @fact:B235-LOCATOR **locator** | `bind-source` всегда работает в контролируемом транспорте, а тот допускает только HTTPS/SSH (и http после FMT-066); тестовый `git+file://` источник отвергается до контакта |
| @fact:B235-SEVERITY **severity** | P3 — поведение для реальных HTTPS/SSH источников покрыто, не покрыта только запись наблюдённого коммита для новой привязки |
| @fact:B235-DISPOSITION **disposition** | `fixed` 2026-11-08 — тест снова включён: связываемый источник раздаётся как dumb-HTTP Git (bare-клон + `update-server-info`) через `crates/vibe-cli/tests/common/http_dir.rs`; контролируемый транспорт допускает `http://` (FMT-066), исходные утверждения сохранены, плюс проверка, что HTTP-эндпоинт действительно опрошен |
| @fact:B235-FILED **filed by** | центральная сессия VIBE-NEEDS, 2026-11-08 |

### B-234 — `vibe update --all` не поднимает ни одной версии {#b-234}

| | |
|---|---|
| @fact:B234-ANCHOR **anchor** | `crates/vibe-cli/src/commands/update/whole.rs` — «`--all` re-resolves the whole declared graph through the install substrate» |
| @fact:B234-LOCATOR **locator** | проект с `"org.example/p" = "^0.1.0"` в lock на 0.1.0, в registry появилась 0.1.1: `vibe update --all --json` отвечает `packages_resolved: 0`, `unchanged: true`; `vibe update org.example/p` поднимает до 0.1.1. Воспроизводится на установленной 1.0.11 и на текущем дереве |
| @fact:B234-SEVERITY **severity** | P2 — `--help` обещает «Update every package in the lockfile», а команда молча ничего не делает и сообщает успех |
| @fact:B234-DISPOSITION **disposition** | `fixed` 2026-11-08 (FMT-068) — `--all` и голый `vibe update` идут через scoped-путь с корнями = все объявленные корни workspace (`crates/vibe-cli/src/commands/update/all.rs`); install-подложка остаётся только для возобновления парковки, пустого/отсутствующего lock и незалоченного объявленного корня. Тесты: `crates/vibe-cli/tests/cli_update_all.rs` |
| @fact:B234-FILED **filed by** | центральная сессия VIBE-NEEDS, 2026-11-08 (найдено при разборе теста отмены update) |


### B-123 — `vibe cache add` печатает «bytes untouched», хотя запись store заменена по хэшу {#b-123}

| | |
|---|---|
| @fact:B123-ANCHOR **anchor** | `crates/vibe-cli/src/commands/cache/add.rs:70` (`was_present` считается до выборки) против `crates/vibe-registry/src/store/refresh.rs:24` (`insert_current_at` заменяет запись мутабельного источника по хэшу) |
| @fact:B123-LOCATOR **locator** | `vibe cache add --offline <координата>` из корня с in-tree пакетом после правки его содержимого: строка отчёта «already present — bytes untouched (write-once)», а sidecar `v<версия>.sha256` и байты записи уже новые |
| @fact:B123-SEVERITY **severity** | P2 — вывод CLI лжёт о сделанном; данные верны |
| @fact:B123-DISPOSITION **disposition** | `open` — считать `was_present` после выборки или печатать «refreshed by hash» |
| @fact:B123-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.8, A0.20; DEFERRALS X-001), 2026-09-11 |

- @fact:B123-SUT **Суть.** Сайт документации на строку отчёта не опирается; человек, прогревающий store руками, — опирается и делает неверный вывод «ничего не обновилось».

### B-124 — `judging-debt.py` цитирует якоря долга по старому адресу PROP-043 {#b-124}

| | |
|---|---|
| @fact:B124-ANCHOR **anchor** | `campaigns/packages-2026-09/tasks/judging-debt.py`, докстринг: `PROP-043 §10.1`, `##DEBT-IS-A-LIST-NOT-A-RATIO`, `##DEBT-MUST-BE-ASKABLE` |
| @fact:B124-LOCATOR **locator** | якоря живут в `vibevm/vibespecs/modules/vibe-progress/PROP-047-progress-campaigns.xml` §6.2 после разделения 2026-08-22; старый адрес — tombstone |
| @fact:B124-SEVERITY **severity** | P3 — стале-ссылка в скрипте чужой зоны; поведение не задето |
| @fact:B124-DISPOSITION **disposition** | `open` — правится попутно при внедрении `[judging] exempt` (docs-2026-09, фаза 2), тем же коммитом |
| @fact:B124-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.19; DEFERRALS X-002), 2026-09-11 |

### B-125 — `vibevm/vibedeps/.gitignore`, который пишет `vibe`, не в истории {#b-125}

| | |
|---|---|
| @fact:B125-ANCHOR **anchor** | `crates/vibe-workspace/src/vibedeps/build_ignore.rs` — файл «Managed by vibe» появляется untracked после `vibe bin build` и `vibe cache add` из корня хоста |
| @fact:B125-LOCATOR **locator** | `git status` после любой из этих команд в чистом чекауте; в той же сессии файл позже исчез (удалён пробой воркера) |
| @fact:B125-SEVERITY **severity** | P3 — вопрос владельцу: файл должен быть в истории или в `.gitignore` корня; рендерер сайта делает `git pull` в выкладке и не должен спотыкаться об untracked-файл |
| @fact:B125-DISPOSITION **disposition** | `open` — решает владелец |
| @fact:B125-FILED **filed by** | кампания docs-2026-09, фаза 0 (журнал J-025; DEFERRALS X-005), 2026-09-11 |

### B-126 — схема `install_plan` требует `plans`, а `vibe install --json` печатает `packages` {#b-126}

| | |
|---|---|
| @fact:B126-ANCHOR **anchor** | `schemas/install_plan.jtd.json` (поле `plans`) против `crates/vibe-cli/src/commands/install/report.rs:150` (эмиттер пишет `packages`) |
| @fact:B126-LOCATOR **locator** | документ `install:plan` потока `vibe install --json` невалиден по собственной схеме; сгенерированный тип `install_plan` в `vibe-cli` не используется, поэтому `cargo xtask check-codegen` расхождения не видит |
| @fact:B126-SEVERITY **severity** | P2 — контракт `--json` нарушен; граничит с «гейт не смотрит», но `check-codegen` разрешимость эмиттера не заявляет (то же различие, что у `##B076-WHY-NOT-P1`) |
| @fact:B126-DISPOSITION **disposition** | `open` — либо эмиттер через сгенерированный тип, либо схема по эмиттеру плюс тест «каждый документ `--json` валиден по своей схеме» |
| @fact:B126-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.12; DEFERRALS X-008), 2026-09-11 |

- @fact:B126-SUT **Суть.** Раннер примеров документации сверяет `--json` со схемой по карте фикстуры и до исправления помечает этот документ «не проверен», не «прошёл».

### B-127 — ключи конверта `invoked_by` и `unattended` не объявлены ни в одной JTD-схеме {#b-127}

| | |
|---|---|
| @fact:B127-ANCHOR **anchor** | `crates/vibe-cli/src/output.rs:471`, `:487` — конверт получает ключи при `--invoked-by` и `--unattended` |
| @fact:B127-LOCATOR **locator** | любой `--json`-документ с `--invoked-by` невалиден по своей схеме (`additionalProperties` закрыты); `VIBE_INVOKED_BY` в окружении даёт тот же эффект молча |
| @fact:B127-SEVERITY **severity** | P2 — контракт `--json` неполон; PROP-000 §16 требует схему на каждый wire |
| @fact:B127-DISPOSITION **disposition** | `open` — объявить конверт одной общей схемой или `optionalProperties` в каждой |
| @fact:B127-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.12; DEFERRALS X-009), 2026-09-11 |

### B-128 — `install:closure-diff` собирается вручную и схемы не имеет {#b-128}

| | |
|---|---|
| @fact:B128-ANCHOR **anchor** | `crates/vibe-cli/src/commands/install/closure_diff.rs:195`, `:208` |
| @fact:B128-LOCATOR **locator** | документ потока `vibe install --json` вне реестра форматов и без JTD-схемы |
| @fact:B128-SEVERITY **severity** | P2 — машинный вывод вне контракта JTD (PROP-000 §16, PROP-044) |
| @fact:B128-DISPOSITION **disposition** | `open` — схема + регистрация в `formats/REGISTRY.toml` + генерируемый тип |
| @fact:B128-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.12; DEFERRALS X-010), 2026-09-11 |

### B-129 — `research/ts-demo` красный на HEAD по трём шагам floor {#b-129}

| | |
|---|---|
| @fact:B129-ANCHOR **anchor** | `research/ts-demo/`: `prettier` (`src/main.ts:11`, 81 символ), `eslint` (плагин `@org.vibevm/eslint-plugin-ai-native` через `file:`-симлинк не резолвит `@typescript-eslint/utils`), `specmap` (orphan `compose`); `vibe.lock` схемы 6 при требуемой 7; `tsconfig.json` ниже пола GUIDE §1 на семь флагов без `deviates` |
| @fact:B129-LOCATOR **locator** | `typescript-ai-native floor --path research/ts-demo` на `b1291b06`; панель `tools/self-check.sh` этот floor не гоняет, поэтому зелёная панель об этом молчит |
| @fact:B129-SEVERITY **severity** | P2 — долг стека TypeScript, не кампании; boot-сниппет стека называет полом четыре флага, GUIDE — двенадцать (расхождение нормы) |
| @fact:B129-DISPOSITION **disposition** | `open` — чинить резолв плагина до открытия web-пакета документации (A4.1 решает временный обход) |
| @fact:B129-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.14; DEFERRALS X-013, X-014), 2026-09-11 |

### B-130 — `min_vibe_version` нигде не проверяется; реальный гейт старого `vibe` — `deny_unknown_fields` {#b-130}

| | |
|---|---|
| @fact:B130-ANCHOR **anchor** | `crates/vibe-core/src/manifest/` — `ManifestWire` с `deny_unknown_fields`; `min_vibe_version` — транзит в индекс и post-publish-хук, ни одного сравнения с версией `vibe` |
| @fact:B130-LOCATOR **locator** | пакет с новым ключом манифеста отказывает старому `vibe` парсером, а не сообщением «нужна версия ≥ X»; поле декларативно |
| @fact:B130-SEVERITY **severity** | P2 — обещание манифеста не подкреплено кодом; станет заметно с первыми doc-пакетами (новые ключи `title`, `abstract`, `[[documents]]`, `[documentation]`, `[translates]`, `[media]`) |
| @fact:B130-DISPOSITION **disposition** | `open` — проверять поле до разбора остальных ключей и падать с понятным сообщением; вопрос владельцу о версии `vibe` (1.1.0) — в вопросах гейта фазы 0 кампании |
| @fact:B130-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.3, F-05), 2026-09-11 |

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
| @fact:B092-CODE-OPACITY-CORRECTION **коррекция 2026-11-04** | Историческая политика выше сохранена как запись принятого тогда решения. Её часть про inline-код теперь отменена: действующие `PARSE-MARKDOWN-FIRST`/`PARSE-FACT-ANCHORS` в PROP-043 требуют непрозрачности кода. Дубликаты определяются только по разобранным настоящим определениям; цитата в code span не создаёт адрес. Настоящие дубликаты по-прежнему запрещены. Проверены parser/conversion/facts/CLI и старый кеш; стираемый payload3 заменяется перепарсингом в4 без изменения tracked cache2 и verdict maps. Подробности: [F07](campaigns/next-semantic-transfer-2026-11/features/F07-inline-anchor-opacity.md) и FMT-013. |

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

### B-110 — clean-chain остаётся вторым composer default lifecycle поверх одного planner {#b-110}

| поле | значение |
|---|---|
| @fact:B110-ANCHOR **anchor** | [PROP-054 `##ENGINE-ALGORITHM`](vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml#ENGINE-ALGORITHM) и R7.4 A12/A15b закон «один shared application/command path» |
| @fact:B110-LOCATOR **locator** | обычные phase-глаголы теперь идут через `vibe-orchestrator/src/command.rs::{lease_default_lifecycle,prepare_default_lifecycle,PreparedDefaultLifecycle::run}`; `vibe clean <phase>` всё ещё вручную собирает chain/metadata/prelude/trace/`PhaseRun` в `crates/vibe-cli/src/commands/lifecycle.rs::execute` |
| @fact:B110-SEVERITY **severity** | P2 |
| @fact:B110-DISPOSITION **disposition** | `open` — сознательно НЕ расширять A15c: MCP не экспонирует clean, а корректное схождение требует отдельного типизированного post-wipe handoff, не forgeable `adopt_clean_epoch`. Названный будущий атом **CLEAN-COMMAND-CONVERGENCE** обязан посадить clean-prefix/rows/notices + строгую post-wipe selection reload в shared command path до появления любой второй clean-поверхности. До него source REDы пинят живой `request.steps() → step_name` маршрут, единый family-classifier и существующие clean goldens |
| @fact:B110-FILED **filed by** | независимый Opus review R7.4 A15b + центральная приёмка, 2026-08-28 |

- @fact:B110-ONE-PLANNER-TWO-COMPOSERS **Точная граница долга.** Алгоритмический planner/runner не раздвоен: обе ветки вызывают один `vibe_orchestrator::run_phases`, один world planner и один `classify_outcome`. Дублирован верхний composer — вычисление chain/metadata, выбор identity, trace-owner и упаковка `PhaseRun`. Поэтому это не второй lifecycle engine, но это второй путь, на котором будущая правка порядка может не совпасть с обычными verbs.
- @fact:B110-WHY-NOT-ADOPT-NOW **Почему не добавлен удобный setter.** Clean выбирает identity до wipe, не открывает trace до wipe, несёт untracked clean rows, затем строго перезагружает selection из того же snapshot под тем же lease. Публичный `adopt_clean_epoch(Vec<_>, PreparedSelection)` позволил бы поверхности подделать уже доказанные chain/rows/root. Будущий атом сначала проектирует закрытый тип результата pre-wipe epoch и только его принимает shared command; A15c не получает ни clean-грамматики, ни обходного конструктора.
- @fact:B110-INTERIM-RATCHETS **Что не даёт долгу тихо расти.** `lifecycle_default_path_tests.rs` читает настоящий clean composer и требует `request.steps()` перед `steps.iter().map(step_name)`, ровно один `PhaseRun` site в CLI (только clean) и ровно один InstallBarrier-vs-Lifecycle classifier для обеих веток. `cli_lifecycle` отдельно прогоняет clean-only, clean-build, consent, failure-after-wipe и path precedence. Эти гейты не закрывают архитектурное раздвоение, но превращают сегодняшние известные совпадения в механику, а не память.

### B-111 — selected-world prompt resolver хранит workspace root, который закрытый selected path не читает {#b-111}

| поле | значение |
|---|---|
| @fact:B111-ANCHOR **anchor** | [PROP-054 `##H-AGENT`](vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml#H-AGENT) — prompt обязан разрешаться через provider-self + exact lock-selected world без slot/workspace fallback |
| @fact:B111-LOCATOR **locator** | `crates/vibe-lifecycle/src/agent/resolver.rs`: `SelectedWorldPromptResolver` хранит `workspace_root` и передаёт его вторым аргументом `FileResolver::with_selected_world`; но `FileResolver::spec_root` при `selected = Some(_)` отвечает self через `self_root`, чужой coordinate через `selected`, а отсутствующий coordinate громко отказывает — `ws_root` используется только `package_slot`-веткой при `selected = None`, которая здесь недостижима |
| @fact:B111-SEVERITY **severity** | P3 |
| @fact:B111-DISPOSITION **disposition** | `open` — отдельным API-cleanup атомом выразить закрытый selected resolver без fallback-root: либо отдельный constructor/type `FileResolver` без `ws_root`, либо zero-state `SelectedWorldPromptResolver`; удалить вводящий в заблуждение обязательный root из hosted/CLI constructors и mutation-доказать, что wrong workspace root больше не является скрытой authority. Не делать `ws_root` реальным fallback: это нарушит selected-world закон |
| @fact:B111-FILED **filed by** | A15c independent review + central accepted-wisdom harvest, 2026-08-28 |

- @fact:B111-NO-PRODUCT-BUG **Это не текущий product bug.** Все функциональные ветки отвечают правильно: self берётся из exact provider root, embed — из selected map, unselected отказывает. Долг в типе authority: публичный constructor требует значение, которое не влияет на выбранную семантику, а rustdoc называет его load-bearing. Такая ложная зависимость заставляет следующий surface искать и переносить root без доказуемой причины.
- @fact:B111-FIX-MUST-KILL-THE-MUTATION **Приёмка будущего атома.** Подмена переданного workspace root на collision tree сегодня не меняет ни один resolver test; после починки такого входа не должно существовать. Existing REDы на provider-self, fresher unselected slot, selected cross-package embed и unselected refusal остаются зелёными, а source fence подтверждает отсутствие slot scan/fallback в selected type.

### B-112 — foundational layout в VIBEVM-SPEC описывает доисторическое дерево {#b-112}

| поле | значение |
|---|---|
| @fact:B112-ANCHOR **anchor** | [`VIBEVM-SPEC.md` §4.2 `#directory-layout`](VIBEVM-SPEC.md#directory-layout) против [PROP-052](vibevm/vibespecs/common/PROP-052-directory-layout.xml) и живого `vibe_core::layout` |
| @fact:B112-LOCATOR **locator** | диаграмма всё ещё обещает `spec/boot/{00-core.md,90-user.md,INLINE.md,INDEX.md}`, `spec/WAL.md`, корневой `vibedeps/flow-wal/0.3.0` и project-local `.vibe/cache`; живое дерево — `vibevm/vibespecs/boot/{00-core.xml,90-user.xml,STATIC.xml,INDEX.md}`, `vibevm/vibespecs/WAL.xml`, `vibevm/vibedeps/<group>.<name>/<version>/`, а machine package store находится в settings/cache, не под проектом. Сама следующая проза повторяет «`spec/` fixed» и ссылается на мёртвый `spec/common/PROP-024…md` |
| @fact:B112-SEVERITY **severity** | P2 |
| @fact:B112-DISPOSITION **disposition** | `open` — отдельный present-truth атом должен вывести диаграмму из `vibe_core::layout`/PROP-052 либо заменить её адресуемым указателем, затем пройти все foundational path examples в VIBEVM-SPEC, boot 00-core и runtime docs. Не чинить одну картинку изолированно: соседние §4.3/§4.6/glossary тоже несут старые `spec/`/WAL/effective-cache формы |
| @fact:B112-FILED **filed by** | R7.5-P0 independent spec audit + central wisdom harvest, 2026-08-28 |

- @fact:B112-WHY-NOT-IN-P0 **Почему не исправлено вместе с R7.5.** P0 меняет evidence/optional-provider/vocabulary authority; layout-долг охватывает всю foundational spec и runtime documentation. Частичная замена четырёх путей сделала бы диаграмму правдоподобнее, но не истиннее, и спрятала бы blast radius, который аудит уже назвал.

### B-113 — read-only facts scanner не различает Windows junction/reparse directory {#b-113}

| поле | значение |
|---|---|
| @fact:B113-ANCHOR **anchor** | [PROP-054 `##FACT-QUERY-CONTRACT`](vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml#FACT-QUERY-CONTRACT) и R7 architecture §2.1 — requirements читает только выбранные host/package source roots и не превращает metadata query в произвольный filesystem crawl |
| @fact:B113-LOCATOR **locator** | `crates/vibe-facts/src/scan.rs::collect_spec_documents`: `DirEntry::file_type().is_symlink()` честно пропускает обычную symlink, но Windows junction/reparse directory может выглядеть как `is_dir()` и рекурсия пойдёт наружу; общий детектор уже существует как `vibe_safefs::ensure_no_follow_walk` (`component.rs`) |
| @fact:B113-SEVERITY **severity** | P2 |
| @fact:B113-DISPOSITION **disposition** | `open` — закрыть до P3 requirements surface: провести каждую директорию/документ через единый no-follow/reparse preflight либо capability-relative reader, не копируя `FILE_ATTRIBUTE_REPARSE_POINT`; сохранить обычный moved-clone in-place slot и добавить Windows junction/injected-reparse RED. Корень, который caller объявляет trusted source root, остаётся отдельной authority и не должен случайно запрещать законный in-place slot |
| @fact:B113-FILED **filed by** | центральная приёмка R7.5 P2/A1 `vibe-facts` scanner, 2026-08-28 |

- @fact:B113-NOT-A-REGRESSION **Почему A1 принят, а долг всё равно P2.** Удалённый CLI-private scanner тоже рекурсировал через `path.is_dir()` и имел ту же junction-щель; A1 улучшил обычные symlink files/dirs и сделал split-brain/identity общими, но не заявлял новую Windows reparse механику. Новый public query расширяет число callers, поэтому известную щель нельзя переносить через P3 как «старое поведение».
- @fact:B113-FIX-SHAPE **Почему не писать четвёртый OS helper.** `vibe-safefs` уже проверяет symlink + `FILE_ATTRIBUTE_REPARSE_POINT` на каждом компоненте и умеет инъекционные race REDы. Локальный `cfg(windows)` бит в `vibe-facts` снова создаст две containment-грамматики; допустим либо прямой lower dependency, либо маленький общий read-only capability API, но не копия константы.

### B-114 — `vibe check` шумит двумя warning'ами на здоровом дереве {#b-114}

| поле | значение |
|---|---|
| @fact:B114-ANCHOR **anchor** | [`tool-design-lessons` `##NEVER-LET-A-PACKAGES-IDENTITY-INCLUDE-BUILD-ARTIFACTS`](vibevm/vibedeps/org.vibevm.world.tool-design-lessons/1.0.0/vibevm/vibespecs/boot/70-flow-tool-design-lessons.xml) — идентичность пакета есть его источник, а не то, что рядом собралось |
| @fact:B114-LOCATOR **locator** | (1) правило `local_source_freshness`: `content_hash` локального package source считает и git-ignored файлы. Запуск бандленного `test_render_goal.py` изнутри `vibevm/vibepacks/org.vibevm.world/multi-user-planning/v0.2.1/**/scripts/` создаёт `__pycache__/` (закрыт своим `.gitignore`), после чего host `vibe check` заявляет «its local source changed since install» и советует `vibe install --assume-yes`. (2) правило `boot_directory` помечает `vibevm/vibespecs/boot/.vibe-boot-artifacts.lock` как non-spec-source файл, хотя он генерируемый и закрыт `.gitignore:19 **/.vibe-boot-*` |
| @fact:B114-SEVERITY **severity** | P3 |
| @fact:B114-DISPOSITION **disposition** | `open` — оба правила должны считать периметр так же, как его считает git: исключать ignored пути из `content_hash` и из инвентаря boot-директории. Пока не закрыто, ложный freshness-warning ловится ровно тем, что его вызвало, — не запускать тесты пакета внутри его собственного source tree, либо чистить `__pycache__` после. **Частичный дрейн (T10B-приёмка, 2026-08-29):** третье тело того же якоря — байт-карта слота в `cli_spec_format` считала персистентный `.vibe-boot-artifacts.lock` частью идентичности пакета и роняла полигон-тест; `generated()` теперь признаёт ровно этот файл engine-owned (закон — `boot_dir_accepts_only_the_exact_persistent_lock` в vibe-check), residue-lock вычищен из слота `org.vibevm.world.git-practices` |
| @fact:B114-FILED **filed by** | центральная приёмка R4.1 T6b, 2026-08-29 |

- @fact:B114-WHY-IT-MATTERS **Почему это не косметика.** Warning советует конкретное действие — `vibe install --assume-yes`, — которое переустановит слот из-за байткода Python. Совет, срабатывающий от артефакта сборки, обучает игнорировать всю категорию; именно так настоящий дрейф источника однажды пройдёт незамеченным.

### B-115 — центральные диспозиции T6b живут только в неотслеженном `cache/` {#b-115}

| поле | значение |
|---|---|
| @fact:B115-ANCHOR **anchor** | [`decision-records` `##GOVERNING-SPEC-SECTION-IS-THE-RECORD`](vibevm/vibedeps/org.vibevm.world.decision-records/1.0.0/vibevm/vibespecs/boot/25-flow-decision-records.xml) и `##REASON-IS-LOST-UNLESS-WRITTEN-DOWN` |
| @fact:B115-LOCATOR **locator** | При заведении `cache/r4-t6b-reds-claudez/REVIEW-NOTES-t6b-round1.md` был единственным носителем нескольких решённых развилок T6b. Самая нагруженная: `std::error::Error::source()` у непрозрачной `TransformCompileError` возвращает `Some(private fault)`, а не `None`; довод — приватный источник не публикует именуемую таксономию, а молчаливый обрыв стандартной цепочки противоречит замороженному typed-source дизайну. Теперь ruling перенесён в отслеживаемый ABI §6.3; cache-путь остаётся историческим locator |
| @fact:B115-SEVERITY **severity** | P2 |
| @fact:B115-DISPOSITION **disposition** | `closed` — четырёхполевая запись решения поднята в управляющий якорь [ABI §6.3](campaigns/packages-2026-09/R4-TRANSFORM-PLAN-ABI-v0.1.md): непрозрачная публичная ошибка сохраняет приватный типизированный `source()`, потому что dyn-источник не публикует именуемую таксономию, а `None` молча рвёт стандартную причинную цепочку. Cache-отчёт остаётся только историческим evidence и больше не является единственным носителем rationale |
| @fact:B115-FILED **filed by** | центральная приёмка R4.1 T6b, 2026-08-29 |

- @fact:B115-THE-TRAP **В чём ловушка.** Мутация M1 делает `source() → None` и валит ровно один тест с сообщением «does not terminate the source chain». Тест защищает решение, но не объясняет его: следующая сессия, увидев RED, узнает *что* нельзя, но не *почему* — и первый же аргумент «непрозрачность лучше» переоткроет вопрос. Тест — это защёлка, запись решения — иммунитет.
- @fact:B115-DRAINED **Осушено FINAL-археологией (2026-09-08).** Развилка T6c о границе свидетеля lane уже жила в [ABI §6.4](campaigns/packages-2026-09/R4-TRANSFORM-PLAN-ABI-v0.1.md) (`4f2acb42`); оставшийся T6b ruling о приватном typed source теперь записан рядом в §6.3 с решением, причиной, отклонённой альтернативой и последствием. `cache/r4-t6b-reds-claudez/REVIEW-NOTES-t6b-round1.md` сохраняется по M-012 как история коррекции, не как авторитет.

### B-117 — путь reached-субъекта не проверяется на контракт `paths` в production {#b-117}

| поле | значение |
|---|---|
| @fact:B117-ANCHOR **anchor** | [ABI §5](campaigns/packages-2026-09/R4-TRANSFORM-PLAN-ABI-v0.1.md) — `declared_path` есть то, с чем матчится измерение `paths`; ядро компилирует его глобы с `require_literal_separator: true` (`crates/vibe-extension-registry/src/selector.rs`), поэтому написание разделителя семантично |
| @fact:B117-LOCATOR **locator** | `SpecAddress::parse` (`crates/vibe-spec/src/address.rs:100-128`) отвергает пробелы, пустую authority и пустой сегмент, но **не** обратный слэш: `doc_path` собирается как `doc_segs.join("/")`, а сам сегмент может содержать `\`. `DocumentSubject::reached` (`ir/subject.rs:150`) копирует его без проверки. Предикат `path_is_forward_slashed` enforced в трёх местах — `ir/artifact.rs:466` (пути строк плана), `wire.rs:192` (декодирование носителя) и `verify.rs:265` (верификатор) — и ни одно не покрывает reached-субъект, рождённый живьём в discovery: верификатор к тому же под `#[cfg(test)]`, то есть в production выключен |
| @fact:B117-SEVERITY **severity** | P2 |
| @fact:B117-DISPOSITION **disposition** | `closed` — закрыт посадкой T8 (`99e52760`): admission-гейт (`transform/selector_admission.rs`) отвергает backslashed `declared_path` **до** любого матчинга, типизированно (`TransformError::Selector { BackslashedDeclaredPath }`) и безусловно по authored-измерениям — malformed-значение отвергается независимо от того, спросит ли его сегодняшнее измерение, ровно как три существующие границы. Живой reached-путь покрыт e2e через построенный `#use`-мир (`schedule_selector_worlds.rs`), RED доказан мутацией №5 пакета; гарантия утверждена под production-конструкцией (`without_verify_each`), потому что test-only верификатор перехватывал бы субъект слоем раньше в чужой семье. Узкая правка грамматики `SpecAddress` сознательно НЕ сделана — публичный тип со своим радиусом; если владелец захочет запретить `\` уже в разборе, это отдельный атом с собственным RED |
| @fact:B117-FILED **filed by** | центральная приёмка R4.1 T7, 2026-08-29 |

- @fact:B117-WHY-P2 **Почему P2, хотя щель ещё не жива.** Симптом — молчаливый: трансформация с authored `paths` просто никогда не применится к документу с обратным слэшем, без единого сообщения. Это ровно тот класс тихой аппроксимации, ради устранения которого в этом же атоме `Option` был расщеплён на `Unclaimed`/`Undetermined`; оставить его незаписанным значило бы починить одну половину проблемы и забыть вторую.

### B-116 — `TransformBehaviorError::WrongStage` рендерит неграмматичный артикль {#b-116}

| поле | значение |
|---|---|
| @fact:B116-ANCHOR **anchor** | [PROP-054 `##TRANSFORM-PLAN-IDENTITY`](vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml#TRANSFORM-PLAN-IDENTITY) — типизированный отказ поведения виден пользователю через публичную цепочку ошибок |
| @fact:B116-LOCATOR **locator** | `crates/vibe-spec/src/compiler/transform/behavior.rs`: шаблон сообщения даёт «refusing a Emitted invocation» для стадий с гласной. Рядом, но отдельно: тестовые машинки `FailingSource`/`FailingEmitted` в `schedule_execution_vehicles.rs` фабрикуют отказ через `self.wrong_stage(<своя же стадия>)`, отчего текст становится самопротиворечивым — «declares Emitted, refusing a Emitted invocation» |
| @fact:B116-SEVERITY **severity** | P3 |
| @fact:B116-DISPOSITION **disposition** | `open` — убрать артикль из шаблона (стадия и так в `{:?}`), а машинкам дать честный собственный вариант ошибки поведения вместо чужого `WrongStage`. Правка трогает production-строку, поэтому идёт со своим RED на рендер |
| @fact:B116-FILED **filed by** | центральная приёмка R4.1 T6b, 2026-08-29 |

### B-118 — 102 неразмеченных факта валят `vibe facts check --exhaustive` на полной панели {#b-118}

| поле | значение |
|---|---|
| @fact:B118-ANCHOR **anchor** | панель `tools/self-check.sh`, шаг «markup validation» — факт без маркера не судится никем; шаг существует ровно затем, чтобы такой факт не жил между границами фаз |
| @fact:B118-LOCATOR **locator** | 102 ошибки `[unmarked]` в файлах, которых кампания LIFECYCLE не касалась: `vibevm/vibepacks/org.vibevm.world/multi-user-planning/{v0.1.0,v0.2.0,v0.2.1}/**/user-local-state.xml` (в т.ч. ЗАМОРОЖЕННЫЕ слоты v0.1.0/v0.2.0), `vibevm/vibespecs/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml`, `PROP-053-clean-verb.xml`. Последние правки — `a42968a9` (planning) и `eb9103ab` (clean verb), обе до 2026-08-29; T10B-панель это только вскрыла (52/54 шага зелёные, красный ровно этот) |
| @fact:B118-SEVERITY **severity** | P2 — гейтит когерентную полную панель R4 (задача #7) и каждую следующую полную панель |
| @fact:B118-DISPOSITION **disposition** | `closed` (2026-08-30, R4-гейт) — обе половины задренированы применением УЖЕ записанных решений: (1) замороженные слоты v0.1.0/v0.2.0 multi-user-planning добавлены в перечислимый exclude `facts.toml` по владельческой политике 2026-07-26 («superseded slot покидает корпус» — тот же прецедент, что core-ai-native/v0.7.0 и redbook/v0.1.0; перечень ручной, steward-волна просто не добавила свои); (2) 34 живых Cell-юнита размечены по конвенции корпуса (стили PROP-038: каждая data-ячейка — свой маркер, стемы строк расширены), пакет пере-материализован, lock hash следует источнику. `facts check --exhaustive`: clean, 523 файла, 0/0. Было: `open` — два решения, оба не для попутной правки: (1) разметить факты в живых источниках (v0.2.1 + два PROP) — контент-авторинг, рядом с владельческим; (2) для v0.1.0/v0.2.0 решить ПРИНЦИП: замороженные слоты либо исключаются из зоны `--exhaustive` (правка зоны/конфига чекера), либо размечаются задним числом — редактирование замороженной истории, что само по себе против правила неизменности слотов. Принцип выбирает владелец; дренаж — до задачи #7 |
| @fact:B118-FILED **filed by** | центральная приёмка R4.1 T10B (end-to-end панель), 2026-08-29 |

### B-119 — hoisted-фикстура несёт тело shared-пакета дважды в корневой полосе {#b-119}

| поле | значение |
|---|---|
| @fact:B119-ANCHOR **anchor** | PROP-038 §2.4 — hoisted single copy: «hoisted to the global root STATIC.md and linked once, its local zones left a #use marker» |
| @fact:B119-LOCATOR **locator** | наблюдение R4.3-приёмки (`tests_analyze_parity`, фикстура: root→{a,b} static, a/b→c static): в записанной корневой XML-полосе тело `c-flow` встречается ДВАЖДЫ — hoisted-копия в корне плюс одна зонная копия — при ожидаемой одной. Паритет analyze==written при этом точен (пин зелёный); вопрос к самой гибридной композиции |
| @fact:B119-SEVERITY **severity** | P3 — форма редкая (два прямых static-требования корня к пакетам с общим static-потомком); байты стабильны и детерминированы |
| @fact:B119-DISPOSITION **disposition** | `open` — выяснить, законна ли зонная копия при hoisted-корневой (возможно, B-006-жанр пропуска dedup для этой формы); паритетный пин снял численную претензию сознательно, чтобы не замораживать поведение, которым тест не владеет |
| @fact:B119-FILED **filed by** | центральная приёмка R4.3, 2026-08-30 |

### B-120 — safefs-примитив дайджеста отказывает каждому реальному Cargo-артефакту {#b-120}

| поле | значение |
|---|---|
| @fact:B120-ANCHOR **anchor** | закон safefs «второе имя = второй владелец» против реальности Cargo: release-раскладка хардлинкует `target/<profile>/<bin>` к `deps/<bin>-<hash>` |
| @fact:B120-LOCATOR **locator** | `vibe_safefs::Project::digest_file_in` (и каждый read-примитив safefs) отказывает файлу с >1 hardlink; поймано вживую e2e-тестом R8-CARGO («has 2 names»). Обход в `mechanism/cargo.rs::digest_produced_file`: собственный стриминговый дайджест после containment-доказательства и отказа финальному симлинку; причина записана на месте вызова |
| @fact:B120-SEVERITY **severity** | P3 — обход корректен для своего домена доверия (build.rs — и так произвольный код; корень engine-owned); вопрос — нужен ли safefs отдельный build-artifact-примитив рядом с публикационным |
| @fact:B120-DISPOSITION **disposition** | `open` — решить владельцу safefs-закона: (а) примитив `digest_build_artifact_in` с ослабленным именным законом для engine-owned корней, либо (б) обход остаётся локальной нормой механизменных провайдеров и фиксируется правилом |
| @fact:B120-FILED **filed by** | центральная приёмка R8-CARGO, 2026-08-30 |

## B-121 — третий жанр лаунчеров (PROP-019 VVM-шимы) не несёт маркера

| поле | значение |
|---|---|
| @fact:B121-WHAT **what** | §7.1 называет два жанра лаунчеров (deploy:vibe-bin и PROP-025 project-shim), и оба теперь имеют точные маркеры-константы в `vibebin/launcher.rs`. Но репозиторий РЕАЛЬНО пишет третий жанр — PROP-019 VVM-шимы (`commands/vvm/env.rs`, `posix_shim()`/`cmd_shim()`): POSIX-тело несёт только комментарий `# vibevm (VVM) shim — …`, `.cmd`-тело — вообще ничего |
| @fact:B121-EFFECT **effect** | закон коллизии vibe-bin видит VVM-шим как «unmarked user file» — отказывает (ничего не перезаписывается, безопасно), но отказ не может НАЗВАТЬ происхождение файла; человек у отказа не узнаёт, что файл — VibeVM-ов |
| @fact:B121-SEVERITY **severity** | P3 — поведение безопасно-консервативное; страдает только диагностика |
| @fact:B121-DISPOSITION **disposition** | `open` — дать VVM-шимам маркер жанра `vibevm-launcher genre=vvm-shim owner=spec://…PROP-019#…` тем же трафаретом; атом, трогающий `vvm/env.rs`, добавляет и распознавание в `classify()` |
| @fact:B121-FILED **filed by** | центральная приёмка R8-VIBE-BIN, 2026-08-30 |

## B-122 — legacy `[[binary]]` нельзя назвать в `[[deploy.target]]`: validate_plane не видит lowering

| поле | значение |
|---|---|
| @fact:B122-WHAT **what** | `vibe_core::manifest::plane::validate_plane` строит множество artifact-id только из `[artifacts]` (build+package outputs); §7.0.7-lowering legacy `[[binary]]` происходит ПОЗЖЕ, в `vibe_orchestrator::dispatch::mechanism::lower_binaries`. Манифест с `[[binary]] name="helper"` + `[[deploy.target]] artifact="helper"` падает на валидации («names no declared artifact output»), хотя lowering произвёл бы ровно этот артефакт |
| @fact:B122-EFFECT **effect** | legacy-путь не может задеплоить свой бинарь через новый деплой-план без переписывания в `[[artifacts.build]]`; обход — авторская `[[artifacts.build]]`-строка (что и есть миграционная рекомендация) |
| @fact:B122-SEVERITY **severity** | P3 — legacy-совместимость сужена, но прямой (новый) путь полноценен; сообщение об ошибке корректно называет чинящий шаг |
| @fact:B122-DISPOSITION **disposition** | `open` — либо validate_plane учитывает lowering-проекцию (vibe-core узнаёт о ней — расширение периметра ядра), либо PROP-054-преемник фиксирует «[[binary]] не сочетается с [[deploy.target]]; мигрируйте на [[artifacts.build]]» как правило |
| @fact:B122-FILED **filed by** | отчёт воркера R8-VIBE-BIN §0.9, зафиксировано центральной приёмкой 2026-08-30 |

## B-131 — оффлайн-установка после `cache add` не резолвит `latest` без клона реестра

| поле | значение |
|---|---|
| @fact:B131-WHAT **what** | `vibe cache add --offline org.vibevm.world/wal --path <хост>` кладёт `wal@1.0.0` в машинный store, но `vibe install org.vibevm.world/wal --path <свежий проект> --offline` (и с пином `@1.0.0`) падает: «`org.vibevm.world/wal@latest` is not resolvable offline — … not in the machine store». Резолвер сначала спрашивает у настроенных реестров (`github.com/vibespecs`, `gitverse.ru/vibespecs` из машинного `registry.toml`), что такое `latest`; без их клона это невозможно, и store не опрашивается. Сообщение об ошибке советует ровно то, что уже сделано (`vibe cache add`) |
| @fact:B131-EFFECT **effect** | сценарий «прогреть store и уйти в оффлайн» (PROP-010 §2.8) не работает для проекта, чьи реестры ни разу не клонировались; после одной онлайн-установки работает. Документация (`howto/work-offline`) описывает контракт; фикстуры примеров обходят это локальным `file://` реестром |
| @fact:B131-SEVERITY **severity** | P2 — контракт оффлайна нарушен в реальном случае «новая машина, store перенесён», сообщение вводит в заблуждение |
| @fact:B131-DISPOSITION **disposition** | `open` — оффлайн-резолюция должна перечислять версии из store (и `file://` зеркал) до отказа; сообщение — называть, чего именно не хватает (клона реестра) |
| @fact:B131-FILED **filed by** | кампания документации, снятие примеров PP-C2 §5a и проба центральной сессии, 2026-09-12 |

## B-132 — клон реестра в глубоком каталоге рвётся на `fatal: '$GIT_DIR' too big`

| поле | значение |
|---|---|
| @fact:B132-WHAT **what** | при `VIBE_SETTINGS` в каталоге длиной ~130 символов `vibe install org.vibevm.world/wal` (онлайн) падает не по сети, а внутри `git clone --recurse-submodules --no-checkout --no-tags`: назначение `<settings>/registries/<hash>/packages/org.vibevm.world.wal/clone` вместе с `.git/…` пробивает буфер Git для Windows (MSYS) — `fatal: '$GIT_DIR' too big`, статус 128 |
| @fact:B132-EFFECT **effect** | пользователь с длинным домашним путём (корпоративные профили, `VIBE_SETTINGS` в глубокой папке) не может установить ни одного пакета; ошибка выглядит как поломка git, а не как ограничение длины пути |
| @fact:B132-SEVERITY **severity** | P3 — воспроизводится только при глубоком `VIBE_SETTINGS`; обход — короткий каталог |
| @fact:B132-DISPOSITION **disposition** | `open` — укоротить раскладку кэша реестров (хэш вместо `packages/<coord>/clone`), передавать `-c core.longpaths=true` на Windows (как делает F19 у fractality), и в сообщении называть длину пути |
| @fact:B132-FILED **filed by** | кампания документации, PP-C2 §5b, 2026-09-12 |

## B-133 — `registry publish --dry-run` требует publish-токен и режет букву диска как хост

| поле | значение |
|---|---|
| @fact:B133-WHAT **what** | `vibe registry publish <пакет> --dry-run` при реестре-каталоге Windows (`C:/…/registry`) падает до какой-либо записи: «publish refused: no token available for host `C`». Хост берётся наивным разрезом URL, и `C:` читается как `scheme://host`; кроме того, `--dry-run` требует токен вообще |
| @fact:B133-EFFECT **effect** | репетиция публикации в локальный реестр невозможна; на настоящем хосте `--dry-run` без токена тоже отказывает, хотя ничего не пишет. Примеры `howto/publish-a-package` ждут починки |
| @fact:B133-SEVERITY **severity** | P3 — публикация в GitHub с токеном работает; страдают репетиция и локальные реестры |
| @fact:B133-DISPOSITION **disposition** | `open` — разбирать адрес реестра как URL или путь до извлечения хоста; `--dry-run` не должен требовать токен, а сообщать, что его нет |
| @fact:B133-FILED **filed by** | кампания документации, PP-C2 §5c, 2026-09-12 |

## B-134 — `vibe init package … --kind <kind>` не действует

| поле | значение |
|---|---|
| @fact:B134-WHAT **what** | `vibe init package org.acme/review-flow --kind flow`, `vibe init --kind flow package org.acme/flow-a` и `vibe init package org.acme/flow-b --kind lang` все создают слот `vibevm/vibepacks/org.acme/<имя>/v0.1.0/vibe.toml` с `kind = "tool"` и сниппетом `10-tool-<имя>.md`; `--help` обещает «Package kind: flow, feat, stack, tool, mcp, lang. Default: tool» |
| @fact:B134-EFFECT **effect** | автор правит вид в манифесте руками; имя файла сниппета остаётся `10-tool-…` |
| @fact:B134-SEVERITY **severity** | P3 — обход тривиален, но флаг лжёт |
| @fact:B134-DISPOSITION **disposition** | `open` — прокинуть `--kind` в шаблон слота (манифест, имя и категория сниппета) или снять флаг из `--help` |
| @fact:B134-FILED **filed by** | кампания документации, проба центральной сессии 2026-09-12 (три формы вызова) |

## B-135 — `vibe uninstall` оставляет пустой каталог `vibevm/vibedeps/<пакет>/`

| поле | значение |
|---|---|
| @fact:B135-WHAT **what** | `vibe uninstall org.vibevm.world/wal --path hello-vibe --assume-yes` печатает «removed its vibedeps/ slot», но `vibevm/vibedeps/org.vibevm.world.wal/` остаётся пустым каталогом; `test ! -e vibevm/vibedeps/org.vibevm.world.wal` красный |
| @fact:B135-EFFECT **effect** | «нет и следа» после удаления не выполняется буквально; скрипты и ассерты документации, проверяющие отсутствие каталога, ложно красные |
| @fact:B135-SEVERITY **severity** | P3 — пустой каталог безвреден, но противоречит отчёту команды |
| @fact:B135-DISPOSITION **disposition** | `closed` — 2026-10-04: удаление транзитивного замыкания убирает пустые контейнеры пакетов через нерекурсивный `remove_dir`, сохраняя другие версии и неучтённые файлы. Проверено тестом `empty_containers_removed_but_untracked_sibling_preserved` на Windows и Linux и свежими копиями FPF GOLDEN: остатки пакетов/контейнеров 0/0, 1/1 и 28/28. |
| @fact:B135-FILED **filed by** | кампания документации, прогон промптов PP-O1 (`remove-a-package`) и проба центральной сессии, 2026-09-12 |

## B-136 — `vibe outdated` требует `[[registry]]` в манифесте, которого `vibe init` не пишет

| поле | значение |
|---|---|
| @fact:B136-WHAT **what** | свежий проект от `vibe init` не несёт `[[registry]]`; `vibe install` резолвит через машинный `registry.toml`, а `vibe outdated` отказывает: «no registry configured. Add a `[[registry]]` entry to `vibe.toml`…». Установленный скилл `vibevm` при этом утверждает, что `vibe init` записал два реестра (`crates/vibe-mcp/src/skill_template.md`) |
| @fact:B136-EFFECT **effect** | три источника расходятся: `install`, `outdated`/`registry list` и скилл; пользователь свежего проекта не может спросить об обновлениях без ручной правки манифеста |
| @fact:B136-SEVERITY **severity** | P2 — базовый сценарий «есть ли новее» не работает из коробки, скилл вводит агента в заблуждение |
| @fact:B136-DISPOSITION **disposition** | `open` — `outdated` читает те же слои реестров, что `install`; текст скилла приводится к факту |
| @fact:B136-FILED **filed by** | кампания документации, PP-O1 (`update-packages`, аномалия A2) и проба 2026-09-12 |

## B-137 — `vibe registry test` называет несуществующий SSH-адрес достижимым

| поле | значение |
|---|---|
| @fact:B137-WHAT **what** | `vibe registry add acme git@github.com:acme-specs …` и затем `vibe registry test` → `reachable (auth=none)` для вымышленной организации, хотя `--help` определяет `reachable` как «org URL responded» и предусматривает `unreachable` |
| @fact:B137-EFFECT **effect** | диагностика не диагностирует: опечатка в адресе или отсутствующая организация обнаружатся только при первой установке |
| @fact:B137-SEVERITY **severity** | P3 |
| @fact:B137-DISPOSITION **disposition** | `open` — для SSH-адресов зондировать хост и организацию (например, `git ls-remote` на стаб или организационный URL), либо честно печатать `unprobed` |
| @fact:B137-FILED **filed by** | кампания документации, PP-O1 (`private-registry`, аномалия A5), 2026-09-12 |

## B-138 — `vibe scrape` на проекте без кода: дефолтный контракт непроходим, `--output` не принимает `..`

| поле | значение |
|---|---|
| @fact:B138-WHAT **what** | (1) `vibe scrape --plan` без контракта отвечает «not a Vibe project: default scrape contract … is absent», хотя проект — проект; (2) дефолтный контракт от `contract init` ставит `modified = "refuse"` и `proof = "contract-assertion-v1"`, у которого нет baseline, так что каждый файл vibe «unknown» и план блокируется 23 находками на чистом `vibe init` + `vibe install`; (3) единственный healthcheck по умолчанию — `cargo`, и без `Cargo.toml` контракт не проходит `check`; (4) `--output ../x` отвергается как «dot component» вместо нормализации |
| @fact:B138-EFFECT **effect** | сценарий «убрать vibe из готового проекта» требует трёх ручных правок контракта, о которых не говорит ни `--help`, ни сообщения; страница руководства их описывает |
| @fact:B138-SEVERITY **severity** | P2 — флагманская операция недоступна без чтения спецификации |
| @fact:B138-DISPOSITION **disposition** | `open` — сообщение об отсутствующем контракте должно предлагать `contract init`; для файлов, записанных самим vibe (слоты, boot-артефакты), baseline известен из lock и `.vibe-boot-artifacts.lock`, а не «unknown»; healthcheck по умолчанию — по обнаруженному инструменту проекта; `--output` нормализует путь |
| @fact:B138-FILED **filed by** | кампания документации, PP-O1 (`scrape`, аномалия A8) и пробы 2026-09-12 |

## B-139 — `--profile` у `vibe deploy` не объяснён, схема профиля недостижима из `--help`

| поле | значение |
|---|---|
| @fact:B139-WHAT **what** | `vibe deploy --help` называет `--profile`, но ни он, ни `vibe package --help` не говорят, как объявить профиль: таблицы `[[artifacts.build]]`, `[[deploy.target]]` с `mechanism`/`config`, `[deploy.profiles.<имя>]` и `default_profile` известны только из тестов (`crates/vibe-cli/tests/cli_deploy.rs`) |
| @fact:B139-EFFECT **effect** | агент по промпту «покажи план деплоя профиля local» сделал шесть попыток угадать схему и сдался; руководство теперь показывает таблицы, но продукт сам себя не объясняет |
| @fact:B139-SEVERITY **severity** | P3 — после появления справочника манифеста (A2.13) закрывается документацией; `--help` всё равно должен ссылаться на страницу |
| @fact:B139-DISPOSITION **disposition** | `open` — в `--help` команд lifecycle добавить адрес страницы руководства или краткую схему |
| @fact:B139-FILED **filed by** | кампания документации, PP-O1 (`build-package-deploy`, аномалия A9), 2026-09-12 |

## B-140 — у `vibe bin list` и `vibe tools` нет `--path`, `vibe bin exec` глотает его молча; `--quiet` не действует у части команд

| поле | значение |
|---|---|
| @fact:B140-WHAT **what** | `vibe bin list --path X` → exit 2 («unexpected argument»), `vibe tools --path X` → то же, тогда как почти все команды проекта принимают `--path`; `vibe bin exec … --path X` принимает и игнорирует; `vibe registry list --quiet` и `vibe mcp status` печатают полный вывод при документированном «single summary line» |
| @fact:B140-EFFECT **effect** | скрипты и агенты, привыкшие к `--path`, получают разное поведение от команд одного продукта; документация вынуждена описывать исключения |
| @fact:B140-SEVERITY **severity** | P3 |
| @fact:B140-DISPOSITION **disposition** | `open` — единый набор глобальных флагов на всех командах проекта (`--path`, `--quiet`, `--json`) с одной семантикой |
| @fact:B140-FILED **filed by** | кампания документации, PP-O1 (аномалии A10, A11) и проба 2026-09-12 |

## B-141 — словарь `boot_snippet.category` не знает видов `feat`, `lang`, `mcp`

| поле | значение |
|---|---|
| @fact:B141-WHAT **what** | `[boot_snippet].category` принимает `foundation, flow, stack, tool, app, user-override`, тогда как `[package].kind` — `flow, feat, stack, tool, mcp, lang` (и с PROP-057 `doc`, `app`); автор `feat`- или `lang`-пакета обязан выбрать чужую категорию (в прогоне PP-O1 агент выбрал `app` для feat и `foundation` для lang) |
| @fact:B141-EFFECT **effect** | порядок чтения в boot-лейне для feat/lang задаётся случайной категорией; руководство (`authoring/write-a-lang-package`) советует `stack` как ближайшую по смыслу |
| @fact:B141-SEVERITY **severity** | P3 — словари должны совпадать или категория должна выводиться из вида |
| @fact:B141-DISPOSITION **disposition** | `open` — PROP-009/PROP-049: либо категория по умолчанию выводится из `kind`, либо словарь категорий расширяется теми же именами |
| @fact:B141-FILED **filed by** | кампания документации, PP-O1 аномалия A13, 2026-09-12 |

## B-142 — `vibe list` и `vibe tree` расходятся о загрузочном сниппете пакета

| поле | значение |
|---|---|
| @fact:B142-WHAT **what** | после установки `org.vibevm.world/wal` `vibe list` печатает `BOOT SNIPPET  —`, а `vibe tree --plain` показывает тот же пакет как `dynamic`-запись лейна |
| @fact:B142-EFFECT **effect** | два обзора одного проекта противоречат друг другу; страница `howto/install-a-package` ссылается на `vibe tree` |
| @fact:B142-SEVERITY **severity** | P3 |
| @fact:B142-DISPOSITION **disposition** | `open` — колонка `BOOT SNIPPET` у `vibe list` читает то же поле lock-файла, что и `tree` |
| @fact:B142-FILED **filed by** | кампания документации, PP-O1 аномалия A15, 2026-09-12 |

## B-143 — `vibe scrape contract check` на Rust-проекте: «no applicable required check» и отказ от hard-linked `cargo.exe`

| поле | значение |
|---|---|
| @fact:B143-WHAT **what** | на проекте с `Cargo.toml` и `Cargo.lock` дефолтный контракт (`[[healthcheck]] kind = "cargo"`) даёт два блокера: `health-no-applicable-required-check: the health panel has no applicable required check` и `health-preparation-failed: reading health asset C:\Users\<user>\.cargo\bin\cargo.exe stably: … has 14 names (hard link); refusing to treat it as exclusively owned` — rustup ставит `cargo.exe` жёсткими ссылками на каждой Windows-машине |
| @fact:B143-EFFECT **effect** | scrape не проходит health-проверку ни на одной машине с rustup под Windows; примеры `lifecycle/scrape` отложены (`examples/deferred.toml`) |
| @fact:B143-SEVERITY **severity** | P2 — флагманская операция недоступна на основной платформе разработчика |
| @fact:B143-DISPOSITION **disposition** | `open` — для инструментов из PATH проверять стабильность содержимого (хэш) вместо эксклюзивного владения; «required after check» дефолтного контракта должен быть применим, когда `Cargo.toml` есть |
| @fact:B143-FILED **filed by** | кампания документации, PP-C2c-подготовка и прогон раннера P2-O4 §3, 2026-09-12 |

## B-144 — движок карты не чеканит spec unit для факта внутри `td`

| поле | значение |
|---|---|
| @fact:B144-WHAT **what** | факт с `fact="true"` в ячейке таблицы (`<td><ID fact="true" …>`) получает якорь и резолвится читателем адресов, но движок карты (`specmap`) не заводит для него spec unit; ребро `documents` на такой якорь помечается `dangling-edge`, хотя якорь существует — 4 таких ребра в карте руководства (PROP-045 строки `ROW-*`, PROP-057 таблицы) |
| @fact:B144-EFFECT **effect** | карта врёт о покрытии табличных фактов; гейт покрытия (A2.16) недосчитает обязательства, записанные строками таблиц |
| @fact:B144-SEVERITY **severity** | P3 — обход: цитировать секцию, а не строку; но табличные факты — законная форма PROP-045 |
| @fact:B144-DISPOSITION **disposition** | `open` — движок карты чеканит юнит для любого элемента с `fact="true"`, включая ячейки таблиц (авторский движок, R-21: правка в движке, не в вендоре) |
| @fact:B144-FILED **filed by** | кампания документации, P2-O6 (A2.11), 2026-09-12 |

## B-145 — `vibe check` требует boot-каталог от всякого пакета, кроме `doc`

| поле | значение |
|---|---|
| @fact:B145-WHAT **what** | проверка `boot_directory` (`crates/vibe-check/src/checks/boot_directory.rs`) освобождает ровно один вид — `doc` — и отказывает любому другому пакету без `vibevm/vibespecs/boot/`: `app` `org.vibevm.doc/web`, `stack` `typescript-ai-native`, `tool` `jtd-codegen` |
| @fact:B145-EFFECT **effect** | `vibe check --path <пакет>` красный для пакетов, которым boot-каталог не нужен по норме; гейт web-пакета кампании не может опираться на него |
| @fact:B145-SEVERITY **severity** | P2 — обход: хостовый `vibe check --path .` зелёный; но вид `app` по норме `KIND-APP-VS-TOOL` не участвует в boot ни одного проекта |
| @fact:B145-DISPOSITION **disposition** | `open` — освободить `app` по той же норме, что `doc`, и решить, требует ли гейт boot-каталог от пакетов без boot-сниппета вообще |
| @fact:B145-FILED **filed by** | кампания документации, P4-O1 (A4.1), 2026-09-12 |

## B-146 — Qwik 2.0.0-beta.43 недетерминированно собирает цепочку layout'ов лендинга

| поле | значение |
|---|---|
| @fact:B146-WHAT **what** | web-пакет `org.vibevm.doc/web`: в двух сборках подряд `dist/index.html` лендинга получил документационную шапку поверх своей (два `<header class="docs-header">`), в том числе на дереве без незакоммиченных правок; следующая сборка тех же файлов дала одну; `layout-landing!.tsx` уже в top-форме (известное лечение) |
| @fact:B146-EFFECT **effect** | тест паритета лендинга краснеет на 33 отличия без изменений в исходниках; выкладка может унести дефектную страницу, если паритет гонялся на предыдущей сборке |
| @fact:B146-SEVERITY **severity** | P2 — обход: паритет и линтер ссылок над той же сборкой, что уходит в выкладку; кандидат на причину — инкрементальный кэш конфигурации маршрутов между клиентской и адаптерной сборками |
| @fact:B146-DISPOSITION **disposition** | `open` — воспроизвести на чистом `node_modules/.vite`; при третьем воспроизведении сменить пин Qwik |
| @fact:B146-FILED **filed by** | кампания документации, P4-O5 (A4.16) и P4-O3 (A4.4), 2026-09-12 |

## B-147 — голден острова ссылается на картинку по адресу, которого конвейер не пишет

| поле | значение |
|---|---|
| @fact:B147-WHAT **what** | голден `crates/vibe-doc` (страница `guide/every-block` фикстурного пакета) цитирует `media/diagram.svg` относительно страницы; `vibe doc build` кладёт медиа пакета в корень дерева под хэшированными именами (`media/<хэш>.<ext>`), а SVG не входит в разрешённые носители волны |
| @fact:B147-EFFECT **effect** | картинка острова битая в любой раскладке сайта; линтер ссылок web-пакета держит для неё исключение `L-01` (8 попаданий на двух написаниях версии) |
| @fact:B147-SEVERITY **severity** | P3 — фикстура; настоящих пакетов с картинками в страницах пока нет |
| @fact:B147-DISPOSITION **disposition** | `open` — решить форму адреса картинки в странице (относительно корня дерева, по хэшу) и переписать голден; снять `L-01` |
| @fact:B147-FILED **filed by** | кампания документации, P4-O2 (A4.12) и P4-O3 (A4.4), 2026-09-12 |

## B-148 — `vibe init` не пишет реестры по умолчанию, хотя норма это обещает

| поле | значение |
|---|---|
| @fact:B148-WHAT **what** | `vibe init <имя>` создаёт `vibe.toml` только с таблицей `[project]` (проба 2026-09-12: десять файлов, ни одной строки `[[registry]]`), тогда как PROP-000 `##INIT-DEFAULT-REGISTRY` и PROP-008 `##DEFAULT-TRUSTED-REGISTRIES` (оба `spec/done`, обязательства `user`) обещают два доверенных корня в каждом новом манифесте; подсказка `--no-registry` в `vibe init --help` говорит о секции `[registry]`, которой команды реестра не читают (`vibe registry list` знает только `[[registry]]`) |
| @fact:B148-EFFECT **effect** | первый `vibe install` в новом проекте не находит реестра, пока пользователь не добавит его `vibe registry add` или не заведёт `~/.vibe/registry.toml`; руководство (`model/registries`, `howto/use-a-private-registry`) описывает продукт как есть и цитирует норму как обещание |
| @fact:B148-SEVERITY **severity** | P2 — обход: `vibe registry add`; норма и продукт расходятся на поверхности, которую видит каждый новичок |
| @fact:B148-DISPOSITION **disposition** | `open` — либо `vibe init` пишет `[[registry]]` с двумя корнями (и `--no-registry` его подавляет), либо норма переписывается под машинный `registry.toml`; решение владельца |
| @fact:B148-FILED **filed by** | кампания документации, первая полная сверка (A6.5), 2026-09-12 |

## B-149 — `vibe init package` пишет в заголовок сниппета удвоенную группу

| поле | значение |
|---|---|
| @fact:B149-WHAT **what** | `vibe init package org.acme/review-notes` создаёт `vibevm/vibespecs/boot/10-tool-review-notes.md` с первой строкой `<!-- vibe:static org.org.acme/review-notes — boot snippet -->`: группа `org.acme` получает второй префикс `org.` (видно в голдене примера `convert` страницы `authoring/specs-agents-can-cite`, снятом раннером с настоящего бинарника) |
| @fact:B149-EFFECT **effect** | комментарий-маркер сниппета называет несуществующую координату; сам сниппет и манифест верны, так что дефект косметический, но он попадает в каждый новый пакет и в документацию, которая снимает вывод дословно |
| @fact:B149-SEVERITY **severity** | P3 — обход: поправить строку руками; ни один гейт по ней не читает |
| @fact:B149-DISPOSITION **disposition** | `open` — найти шаблон сниппета в `vibe init package` и убрать лишний префикс; голден примера перебласти тем же коммитом |
| @fact:B149-FILED **filed by** | кампания документации, первая полная сверка (A6.5), 2026-09-12 |

## B-150 — docs: навигация сайта и ридера идёт в порядке манифеста, а манифест — по закону слоёв

| поле | значение |
|---|---|
| @fact:B150-WHAT **what** | `vibe doc manifest` упорядочивает страницы по закону слоёв (стабильное раньше изменчивого): первыми идут `agent/*` и `architecture/*`, `start/what-vibevm-is` стоит 33-й из 49, `start/first-project` — 46-й; навигация сайта и локального ридера берёт порядок манифеста как есть (`site/src/lib/view.ts`), и десять самых частых терминов впервые встречаются на страницах для агентов и разработчиков, а не на страницах новичка (аудит P6-C1, пп. 6–7) |
| @fact:B150-EFFECT **effect** | человек, открывший навигацию, видит сначала «How an agent reads this manual» и «How vibe is built»; маршрут новичка спрятан; для `llms.txt` тот же порядок верен |
| @fact:B150-SEVERITY **severity** | P2 — обход: ссылки со страницы-маршрута `start/index` (поставлены 2026-09-12); правка — атом в web-пакете |
| @fact:B150-DISPOSITION **disposition** | `open` — навигация для людей упорядочивает разделы по маршруту читателя (`start`, `model`, `howto`, `agent`, `authoring`, `lifecycle`, `reference`, `diagnostics`, `faq`, `glossary`, `architecture`), неизвестные разделы — после, по алфавиту; порядок манифеста для агентов не меняется; решение владельца — держать список в web-пакете или объявлять в манифесте документации |
| @fact:B150-FILED **filed by** | кампания документации, месячная петля A6.3 (аудит P6-C1), 2026-09-12 |

## B-151 — docs: двадцать пять полей манифеста и лока не названы ни на одной странице

| поле | значение |
|---|---|
| @fact:B151-WHAT **what** | снимок поверхности (`vibe doc surface`) знает 221 поле `vibe.toml` и 55 полей `vibe.lock`; нигде в руководстве не встречаются: манифест — `artifacts.build[].workdir`, `boot.default_link`, `boot_snippet.concepts`, `deploy.target[].depends_on`, `embedded_source[].license_path`, `embedded_source[].license_url`, `llm.default_model`, `llm.default_provider`, `mechanism[].config_schema`, `package.spec_format`, `project.spec_format`, `recommends`, `suggests`; лок — `meta.active_features`, `meta.virtual_capabilities[]` (с `emitted_at`, `trace_id`), `package[].admitted_by`, `package[].embedded_source[].license_file_sha256`/`license_path`/`license_url`/`tree_oid`, `package[].subskills_active[]` (с `cache_files`), `package[].via_override` (аудит P6-C1, п. 2) |
| @fact:B151-EFFECT **effect** | читатель справочника не найдёт этих полей; `vibe doc diff` при смене версии назовёт страницы справочника, но не подскажет, что поле никогда не было описано |
| @fact:B151-SEVERITY **severity** | P2 — таблицы `reference/manifest` и `reference/lock-file` |
| @fact:B151-DISPOSITION **disposition** | `open` — строки в двух таблицах с `rule` на факт каждого поля; поля без факта в спеке — сначала факт, потом строка |
| @fact:B151-FILED **filed by** | кампания документации, месячная петля A6.3 (аудит P6-C1), 2026-09-12 |

## B-152 — docs: 45 подкоманд второго уровня не упомянуты в прозе

| поле | значение |
|---|---|
| @fact:B152-WHAT **what** | из 93 подкоманд `vibe <команда> <подкоманда>` 45 существуют в руководстве только в авто-выводе `--help` родителя: `self` 10 из 15, `progress` 9 из 10, `aiui` 11 из 11, `facts` 5 из 9, `doc` 4 из 9, `registry` 3 из 11, `prefs` 2 из 7, `extensions` 1 из 2 (аудит P6-C1, п. 1); третий уровень `--help` не выводится — X-046 |
| @fact:B152-EFFECT **effect** | читатель узнаёт о подкоманде из списка, но не о том, когда она нужна; `vibe doc diff` относит смену флага такой подкоманды к «нужна новая страница» |
| @fact:B152-SEVERITY **severity** | P3 — семействами: `self`, `facts`, `registry`, `doc` — проза на существующих страницах; `aiui`, `progress` — инструменты разработчика, одна страница в `architecture/` или блоки `derived` второго уровня |
| @fact:B152-DISPOSITION **disposition** | `open` — решается в недельных петлях по семейству за петлю |
| @fact:B152-FILED **filed by** | кампания документации, месячная петля A6.3 (аудит P6-C1), 2026-09-12 |

## B-153 — index: маршруты `vibe-index` джойнят percent-декодированный захват с `data_dir`

| поле | значение |
|---|---|
| @fact:B153-WHAT **what** | маршруты `/v1/index/by-name/{name}`, `by-cap/{slug}`, `by-purl/{slug}` (`crates/vibe-index/src/server/routes/index_files.rs:80-126`) склеивают percent-декодированный захват с `data_dir` без проверки; на идентичном маршруте с той же axum 0.8.9 `..%2Fsecret.txt` вернул файл вне корня (200); на живом бинарнике не подтверждено |
| @fact:B153-EFFECT **effect** | любой файл, до которого дотягивается процесс индекс-сервера, читается по HTTP; сайт документации `vibe-index` не использует, риск — при публикации индекс-сервера наружу |
| @fact:B153-SEVERITY **severity** | P1 — безопасность; чинить до публикации индекс-сервера наружу: лексический `safe_join` или `ServeDir`, тест `oneshot(GET /v1/index/by-name/..%2F..%2Fsecret.json)` |
| @fact:B153-DISPOSITION **disposition** | `open` — отдельный PR вне кампании документации; владелец извещён на гейте фазы 0 и в HANDOVER-B §6 |
| @fact:B153-FILED **filed by** | кампания docs-2026-09, фаза 0 (A0.7; DEFERRALS X-017), найдено 2026-09-11, строка заведена при закрытии леджера 2026-09-12 |

## B-154 — codegen: первый `float` в JTD-схемах снимет `Eq` со всех генерируемых типов

| поле | значение |
|---|---|
| @fact:B154-WHAT **what** | `xtask/src/codegen/derive_floor.rs` выводит `Eq` на весь набор, и первая дробная метрика его снимет; поэтому «тики на тысячу слов» записаны как `tics_per_100k_words` (`uint32`), а отчёт делит обратно |
| @fact:B154-EFFECT **effect** | первая схема, которой нужна дробь без потерь, либо ломает пол derive для всех типов, либо повторяет обходной путь целыми |
| @fact:B154-SEVERITY **severity** | P3 — решить пол derive для типов с `f64` (отдельный набор derive для схем с дробями) при первой настоящей дробной метрике |
| @fact:B154-DISPOSITION **disposition** | `open` |
| @fact:B154-FILED **filed by** | кампания docs-2026-09 (P2-O10, аномалия 2; DEFERRALS X-047), 2026-09-12 |

## B-155 — tests: сетевые `redbook_polygon_*` делают `cargo test -p vibe-cli` часовым

| поле | значение |
|---|---|
| @fact:B155-WHAT **what** | `crates/vibe-cli/tests/cli_spec_format.rs::redbook_polygon_*` запускают `vibe install` по сети, 10–15 минут на прогон; полный `cargo test -p vibe-cli` не заканчивается за час, панель `tools/self-check.sh` наследует эту стоимость |
| @fact:B155-EFFECT **effect** | панель гоняется реже, чем нужно, и точечные гейты заменяют её по необходимости, а не по выбору |
| @fact:B155-SEVERITY **severity** | P2 — офлайновая фикстура реестра для этих тестов или флаг окружения, отделяющий сетевые тесты от пола |
| @fact:B155-DISPOSITION **disposition** | `open` |
| @fact:B155-FILED **filed by** | кампания docs-2026-09 (P2-O11, аномалия 3; DEFERRALS X-048), 2026-09-12 |

## B-156 — tests: `golden_corpus.rs` читает wire-типы из рабочего дерева

| поле | значение |
|---|---|
| @fact:B156-WHAT **what** | `vibe-index/tests/golden_corpus.rs` читает сгенерированные wire-типы из рабочего дерева, поэтому чужой кодген в соседнем периметре красит его на любой ревизии (1879 против 1891 байт) |
| @fact:B156-EFFECT **effect** | ложный красный при параллельной работе двух воркеров; правило J-101 (парные замеры последовательно, без чужого кодгена в дереве) обходит, не лечит |
| @fact:B156-SEVERITY **severity** | P3 — голден сравнивает закоммиченные типы, не рабочее дерево, или тест помечает чужой кодген причиной |
| @fact:B156-DISPOSITION **disposition** | `open` |
| @fact:B156-FILED **filed by** | кампания docs-2026-09 (P2-O11, аномалия 1; DEFERRALS X-049), 2026-09-12 |

## B-157 — tests: два теста `vvm.rs` ждут бинарник под `target/` дерева

| поле | значение |
|---|---|
| @fact:B157-WHAT **what** | `which_reports_the_direct_source_executable…` и `ls_on_a_fresh_root…` в `crates/vibe-cli/tests/vvm.rs` ждут бинарник под `<репозиторий>/target/`: `find_source_root` идёт по предкам исполняемого файла, и приватный `CARGO_TARGET_DIR` вне дерева красит их |
| @fact:B157-EFFECT **effect** | воркер с приватным каталогом сборки получает два красных теста без дефекта в продукте |
| @fact:B157-SEVERITY **severity** | P3 — тест пропускается с причиной, когда бинарник вне дерева, или `find_source_root` принимает корень явно |
| @fact:B157-DISPOSITION **disposition** | `open` |
| @fact:B157-FILED **filed by** | кампания docs-2026-09 (P4-O4, А-7; DEFERRALS X-050), 2026-09-12 |

## B-158 — doc site: билдер читает `primary.jsonl`, а не `primary.jsonl.gz`

| поле | значение |
|---|---|
| @fact:B158-WHAT **what** | реестровый билдер читает несжатый `primary.jsonl`: цена бэкенда DEFLATE в графе конвейера выше выигрыша на каталоге в килобайты |
| @fact:B158-EFFECT **effect** | при каталоге больше мегабайта каждый опрос индекса тянет лишние байты |
| @fact:B158-SEVERITY **severity** | P3 — перейти на `.gz`, когда каталог перерастёт мегабайт; бэкенд `flate2` уже прибит к `zlib-rs` (J-069) |
| @fact:B158-DISPOSITION **disposition** | `open` |
| @fact:B158-FILED **filed by** | кампания docs-2026-09 (P5-O1, «не сделано» 4; DEFERRALS X-057), 2026-09-12 |

## B-159 — web: каждый `<script>` из `DocumentHead.scripts` несёт пустой атрибут `:`

| поле | значение |
|---|---|
| @fact:B159-WHAT **what** | в отрендеренном HTML сайта каждый скрипт головы страницы заканчивается атрибутом с именем `:` и пустым значением: `<script type="application/ld+json" q:container="html" :>` и `<script defer src="/u/s.js" … data-host-url="https://vibevm.org" :>`; так сериализует `@qwik.dev/router` 2.0.0-beta.43 для записей `scripts` без `key` |
| @fact:B159-EFFECT **effect** | браузеры атрибут игнорируют, скрипты грузятся; это мусор в разметке, который ловит любой строгий HTML-валидатор, и он не наш, а фреймворка |
| @fact:B159-SEVERITY **severity** | P4 — проверить при пересмотре пина Qwik (X-020, X-034): задать `key` каждой записи или переписать записи через `props`, если стабильная 2.0 не уберёт атрибут сама |
| @fact:B159-DISPOSITION **disposition** | `open` |
| @fact:B159-FILED **filed by** | кампания docs-2026-09, выкладка A5.6 (2026-09-12) |

## B-160 — vvm: `--offline` парсится, но не доходит до `vibe self`

| поле | значение |
|---|---|
| @fact:B160-WHAT **what** | корневой `--offline` принят разбором и виден в `vibe self … --help`, но `main.rs` не передаёт его в `VvmEnv`, а `output::Context` его не несёт; `vibe --offline self update` идёт в сеть и успешно обновляется (измерено воркером POST-O2 на изолированном корне) |
| @fact:B160-EFFECT **effect** | скрипт, который запретил сеть, всё равно получает скачивание; честных офлайн-отказов для `update`, `install stable` и `reinstall` нет |
| @fact:B160-SEVERITY **severity** | P2 — одно поле в корне композиции плюс три места вызова в `bundle.rs`; чинить отдельным PR |
| @fact:B160-DISPOSITION **disposition** | `closed` — POST-O3 (`a031e804`, `09f647d7`), проза `75be0ac5`^ (2026-09-14) |
| @fact:B160-FILED **filed by** | кампания docs-2026-09, ревью POST-O2 (2026-09-14) |

## B-161 — registry: один HTTP 429 от raw-хоста стоит целого клона

| поле | значение |
|---|---|
| @fact:B161-WHAT **what** | быстрый путь чтения по HTTPS на неожиданный статус, включая 429, падает в штатную лестницу `git archive` → clone → fetch → checkout → submodule: пять запусков git и около 20 с из 90 с фазы разрешения на графе redbook; ретрая с задержкой нет |
| @fact:B161-EFFECT **effect** | одно ограничение частоты у хоста удлиняет холодную установку на пятую часть |
| @fact:B161-SEVERITY **severity** | P2 — короткий backoff-ретрай на 429/5xx до падения в git; решение за владельцем, поведение при неожиданном статусе задумано как «спроси git» |
| @fact:B161-DISPOSITION **disposition** | `closed` — POST-O4 (`b709a02e`, `87799488`), проза `54038d89`^ (2026-09-14) |
| @fact:B161-FILED **filed by** | кампания docs-2026-09, отчёт POST-O1 «Аномалии» п. 1 (2026-09-14) |

## B-162 — registry: `index_client` открывает соединение на каждый запрос индекса

| поле | значение |
|---|---|
| @fact:B162-WHAT **what** | после POST-O1 чтения файлов идут через один клиент с пулом, а `index_client` по-прежнему строит клиент на вызов; на реестре со статическим индексом на том же raw-хосте это 196 из 196 оставшихся рукопожатий за прогон при 190 запросах индекса |
| @fact:B162-EFFECT **effect** | холодная фаза разрешения платит TLS-рукопожатие за каждый запрос индекса |
| @fact:B162-SEVERITY **severity** | P2 — один клиент на процесс для индекса; рядом — параллельный веер `PERF-RAYON-FANOUT` PROP-002 §2.12, всё ещё `spec/done` |
| @fact:B162-DISPOSITION **disposition** | `open` |
| @fact:B162-FILED **filed by** | кампания docs-2026-09, отчёт POST-O1 «Аномалии» п. 2 (2026-09-14) |

## B-163 — docs: страница публикации и таблица ошибок расходятся о повторной публикации версии

| поле | значение |
|---|---|
| @fact:B163-WHAT **what** | `howto/publish-a-package` говорит, что публикация существующей версии заменяет её (PROP-002 `PUBLISH-MUTABLE-VERSIONS`); `diagnostics/errors` для `publish refused: tag … already exists` советует «поднимите версию; опубликованные версии не двигаются» (PROP-002 `IDENTITY-CONSEQUENCE`); адаптация повторяет обе строки |
| @fact:B163-EFFECT **effect** | читатель получает два ответа на один вопрос; какое из двух поведений у публикатора на текущем `main`, не проверено |
| @fact:B163-SEVERITY **severity** | P3 — сверить с кодом публикатора и оставить одну формулировку на обеих страницах и в адаптации |
| @fact:B163-DISPOSITION **disposition** | `open` |
| @fact:B163-FILED **filed by** | кампания docs-2026-09, волна C, зеркалирование таблицы ошибок (2026-09-14) |

## B-164 — web: лендинг держится на условии в корневом layout вместо route groups

| поле | значение |
|---|---|
| @fact:B164-WHAT **what** | `routes/layout.tsx` рендерит шапку и подвал документации только когда `docSegments(pathname) !== null`, потому что `layout-landing!.tsx` («top» layout, deprecated) в закреплённой бете Qwik не останавливает цепочку layout'ов; форма, на которую указывает предупреждение сборки, — route groups |
| @fact:B164-EFFECT **effect** | условие держится, но каждая сборка печатает deprecated-предупреждение десятками строк в логе рендера, а следующий маршрут вне `/doc/**` повторит ту же ловушку двух шапок |
| @fact:B164-SEVERITY **severity** | P3 — перестроить дерево маршрутов вокруг route groups вместе с пересмотром пина Qwik (X-020, X-034, B-159) и снять условие |
| @fact:B164-DISPOSITION **disposition** | `open` |
| @fact:B164-FILED **filed by** | кампания docs-2026-09, ревью W1-O1 (2026-09-14) |

## B-165 — spec: PROP-002 не описывает быстрый путь чтения по HTTPS

| поле | значение |
|---|---|
| @fact:B165-WHAT **what** | POST-O1 (`b058098f`, `818247cd`) научил бэкенд читать один файл по HTTPS с GitHub и GitVerse до `git archive`, но PROP-002 §2.12 (`PERF-FETCH-FILE`) по-прежнему описывает чтение одного файла как `git archive`; POST-O4 положил рядом факт о повторах (`RAW-READ-BACKOFF`), и норма о повторе стоит раньше нормы о самом пути |
| @fact:B165-EFFECT **effect** | руководство цитирует правило о повторах, а правила о быстром пути, его хостах, авторитетности промаха (тег/SHA против ветки) и токене-в-заголовке нет; `raw_http.rs` на 605 строках против бюджета 600 — адаптеры хостов просятся в `raw_http/hosts.rs` |
| @fact:B165-SEVERITY **severity** | P2 — один факт `RAW-READ-FAST-PATH` в §2.12 по тексту докстрингов `raw_http.rs` (таблица хостов, промах авторитетен только для тега или SHA, токен только заголовком), правка `PERF-FETCH-FILE`, ребро `implements` из `raw_http.rs`; вместе с ним — вынос адаптеров в `hosts.rs` |
| @fact:B165-DISPOSITION **disposition** | `closed` — POST-O5 (`633856c2`, `4d1138ab`), проза `d2e786b2`^ (2026-09-14) |
| @fact:B165-FILED **filed by** | кампания docs-2026-09, ревью POST-O4 (2026-09-14) |

## B-166 — registry: `shell.rs` сверх бюджета строк; rustdoc-ссылки в `//!` подмодулей разрешаются в области родителя

| поле | значение |
|---|---|
| @fact:B166-WHAT **what** | `crates/vibe-registry/src/git_backend/shell.rs` — 767 строк против совещательного бюджета 600 (сверх бюджета ещё до POST-O1); в `raw_http.rs`, `raw_http/retry.rs` и `raw_http/hosts.rs` ссылки rustdoc в `//!`-документации подмодулей (`super::absence_is_authoritative`, `MAX_ATTEMPTS`, `PAUSE_BUDGET`, `retry`) разрешаются в области родителя и дают четыре предупреждения `unresolved link`; `cargo doc` не гейт и выходит 0 |
| @fact:B166-EFFECT **effect** | файл-хаб бэкенда git растёт с каждым пакетом; предупреждения rustdoc копятся и однажды спрячут настоящее |
| @fact:B166-SEVERITY **severity** | P3 — вынести классификацию ошибок и рендер argv из `shell.rs` в свои файлы; ссылки в `//!` писать путём от крейта (одна правка на четыре места) |
| @fact:B166-DISPOSITION **disposition** | `open` |
| @fact:B166-FILED **filed by** | кампания docs-2026-09, ревью POST-O5 (2026-09-14) |

## B-167 — docs: три золотых примера и константа корпуса отстали от адаптации

| поле | значение |
|---|---|
| @fact:B167-WHAT **what** | `vibe doc check --examples` над руководством даёт три расхождения: вывод `vibe explain` и `vibe select` теперь несёт `documents`-рёбра страниц `vibevm-docs-ru` (появились с волны C); `cargo test -p vibe-specdoc --test docs_corpus` ждёт 781 цитируемое правило при 791 в дереве; rustdoc-примеры в `crates/vibe-doc-server/src/routes/resolve.rs:45` и `crates/vibe-doc-shell/src/index.rs:62` пишут координату `@0.1.0`, которой после W1-O2 нет |
| @fact:B167-EFFECT **effect** | полный гейт `--examples --min 100` красный; тест корпуса красный; примеры в докстрингах называют несуществующую версию |
| @fact:B167-SEVERITY **severity** | P2 — переблагословить три золотых (решение, а не автоматика), поднять константу корпуса до текущего числа, поправить две координаты в докстринках |
| @fact:B167-DISPOSITION **disposition** | `closed` под ратификацию 2026-09-14: константа корпуса поднята до 791 (`25775078`), обе координаты в докстринках — `1.0.0` (`90dbbc13`), три золотых переблагословлены с вывода продукта (`20855535`; при отказе владельца — revert одного коммита) |
| @fact:B167-FILED **filed by** | кампания docs-2026-09, ревью W1-O2 (2026-09-14) |

## B-168 — обновление изменяемого источника оставляет неполный кэш при Windows sharing violation {#b-168}

| поле | значение |
|---|---|
| @fact:B168-WHAT **what** | `insert_current_at` в `crates/vibe-registry/src/store/refresh.rs` удаляет старую запись через `remove_dir_all` до подготовки новой. При OS32 удаление останавливается посередине: остаются несколько файлов и старый `v1.0.0.sha256`, но отсутствует manifest. |
| @fact:B168-EVIDENCE **evidence** | 2026-09-12: scoped offline update `flow:org.vibevm.world/multi-user-planning` из `vibevm-next` отказал на общем store; уцелели только три `skills/steward-*/SKILL.md`. Установленный слот и lock не изменились. Владелец блокирующего handle не установлен; наличие MCP-процесса не доказывает его причастность. |
| @fact:B168-SEVERITY **severity** | P2 — повреждается восстанавливаемая производная запись, последующие потребители могут получить missing-manifest вместо старого целого пакета. Исходник и установленный пакет сохранны. |
| @fact:B168-DISPOSITION **disposition** | `open` — подготовить и проверить замену до удаления старой записи; при отказе переключения сохранить целую старую запись либо явно восстановить её. Проверить сценарий удерживаемого Windows handle и согласованность sidecar; не выдавать частичное удаление за отсутствие побочных эффектов. |
| @fact:B168-WORKAROUND **workaround** | Для scoped update использовать процессный `VIBE_SETTINGS` во временном каталоге и `VIBEVM_USER_CONFIG`, указывающий на действующий config, затем удалить только этот временный каталог. Повреждённую старую запись восстанавливать добавлением отсутствующих файлов лишь после совпадения старого sidecar и всех уцелевших файлов с независимо подготовленным старым источником; уцелевшие файлы и sidecar не переписывать. |
| @fact:B168-RECOVERY **incident recovery** | Старый sidecar `sha256:35abb3312ba052fc158070c9cf4cd133d6c0326b74c2ff7f9f03c316f712af1b` не совпал с независимо подготовленным main. Поэтому восстановление неизвестного старого снимка не заявляется. Кэш заново собран из 27 файлов действующего источника main, чей hash `sha256:e1a193828f417c3ddcf1581285dc0166ee9d900ab1ad859402624bc39ad2a051` совпал с main lock: 24 отсутствующих файла добавлены, 3 уцелевших проверены и сохранены, после полного побайтового сравнения обновлён sidecar. NEXT получил новую версию содержимого через изолированный scoped update. |
| @fact:B168-FILED **filed by** | проверка публикации multi-user-planning 1.0.0, 2026-09-12; исправление продукта не включено в обновление пакета и не запускает NEXT |

## B-169 — документация публикации отрицает поддержанное обновление той же версии {#b-169}

| поле | значение |
|---|---|
| @fact:B169-WHAT **what** | `docs/commands/registry-publish.md` обещает отказ при существующем теге и отдельные push main/tag. Реализация `crates/vibe-publish/src/git_publish.rs` сохраняет parent main, атомарно обновляет main и выбранный изменяемый тег с точными leases; идентичный повтор не меняет refs. |
| @fact:B169-SEVERITY **severity** | P3 |
| @fact:B169-DISPOSITION **disposition** | `open` — синхронизировать pipeline, ошибки и описание повторной публикации с текущей реализацией и её тестами. Рабочий путь подтверждён публикацией multi-user-planning 1.0.0, remote commit `0a99e63b7898723057ef262c2ac90b0dcbb846c3`, с побайтовым сравнением всех 28 файлов. |
| @fact:B169-FILED **filed by** | проверка публикации multi-user-planning, 2026-09-12 |

## B-170 — registry publish dry-run всё ещё проверяет репозиторий через API {#b-170}

| поле | значение |
|---|---|
| @fact:B170-WHAT **what** | CLI help для `registry publish --dry-run` обещает отсутствие API calls, но `crates/vibe-publish/src/orchestrator.rs` вызывает `repo_exists` до проверки `config.dry_run`. Путь с `--registry` требует действительный token даже для предварительного просмотра. |
| @fact:B170-EVIDENCE **evidence** | 2026-09-13, проверка публикации исходного пакета: dry-run с процессным фиктивным token отказал с ошибкой scope; тот же dry-run с обычным механизмом credentials успешно сообщил план создания `example-source-package`, не создавая репозиторий. Это не свидетельство отсутствия прав у действующего пользователя. |
| @fact:B170-SEVERITY **severity** | P3 — неверное обещание dry-run; опубликованные refs и пакет не меняются. |
| @fact:B170-DISPOSITION **disposition** | `open` — либо убрать сетевую проверку из dry-run, либо явно документировать read-only API probe и требования к credentials. Исправление продукта выделено в отдельную задачу. |

## B-171 — host traceability map has existing orphan and freshness debt {#b-171}

| field | value |
|---|---|
| @fact:B171-WHAT **what** | A host specmap regeneration finds five untagged existing items: `EmbeddedSourceKind`, `EmbeddedSourceAuth`, `EmbeddedSourceDecl` in `crates/vibe-core/src/manifest/package/embedded_source.rs`, and `SubmoduleProvenance`, `inspect` in `crates/vibe-publish/src/git_publish/submodules.rs`. Neither source file changed in the native application installation work. |
| @fact:B171-EVIDENCE **evidence** | 2026-09-16: regeneration reports 7,614 spec units, 3,422 tagged code items, 2,975 edges, zero suspects and zero unresolved host edges, but five gated orphans and 500 added units relative to the older committed map. It also reports unchanged-revision drift at `PROP-023#maintainer-model` and `PROP-020#trust-gate`. The generated map spans changes beyond the application task, so its prior committed bytes were retained. 2026-09-25: the same regeneration also owes the learning path of PROP-057 — eight units (`NAV-CHAPTERS`, `-CHECKED`, `-TRANSLATION`, `-READER`, `-DECISION` and the record's three fields) and three scope edges from `75800885a` — plus `PROP-059#binary-selection` and `#mode-transition`, drift at `PROP-019#release-production`, and two orphans, `vibe_cli::cli::run::RunArgs` and `vibe_cli::commands::application::distribution::codec::source_differs`; the committed bytes were kept again by this rule. |
| @fact:B171-SEVERITY **severity** | P2 — existing host traceability debt; no application runtime failure or new application orphan was observed. |
| @fact:B171-DISPOSITION **disposition** | `open` — reconcile the existing source/spec revisions and missing tags, then regenerate and check the complete host map in a dedicated follow-up. Do not add exemptions or silently include unrelated unaccepted source facts in an application feature commit. |
| @fact:B171-FILED **filed by** | Native user-application installation acceptance, 2026-09-16. |

## B-172 — conform accepts an unmatched scope as a green result {#b-172}

| field | value |
|---|---|
| @fact:B172-WHAT **what** | `rust-ai-native conform check --scope` advertises a crate name, but `core-ai-native-conform::finding::Report::scoped` filters repository-relative file paths with `starts_with`. `--scope vibe-core` selects no files and exits successfully even when the full check reports findings in `crates/vibe-core/`. |
| @fact:B172-EVIDENCE **evidence** | 2026-09-17 command-progress acceptance: named scopes `vibe-core` and `vibe-cli` reported zero; the full check exposed new progress-module findings, which were then handled directly. The canonical Rust stack and installed projection have identical scope-filter sources. |
| @fact:B172-SEVERITY **severity** | P2 — a documented invocation can falsely certify a change. |
| @fact:B172-DISPOSITION **disposition** | `open` — align the advertised grammar with crate resolution and refuse unknown or unmatched scopes. Until fixed, use a repository-relative prefix such as `--scope crates/vibe-core/` and verify the selected coverage. No discipline-engine changes are included in the command-progress feature. |

## B-173 — release floor is blocked by post-baseline handwritten wire {#b-173}

| field | value |
|---|---|
| @fact:B173-WHAT **what** | The final `tools/self-check.sh` release panel stops at the wire-derive ratchet before product tests: 13 `Serialize`/`Deserialize` files landed after the current baseline across `progress-core`, `vibe-cli`, `vibe-core`, `vibe-doc-shell`, `vibe-doc`, `vibe-registry`, and `xtask`. Several are strict VibeVM-owned durable or exchanged formats, notably the user-application index/management/context/reply and distribution descriptors, so raising every count as “non-wire” would be false. |
| @fact:B173-EVIDENCE **evidence** | 2026-09-17 CLI-progress release gate, `tools/self-check.sh` step 3/53: current/baseline counts are `progress-core` 12/11, `vibe-cli` 48/44, `vibe-core` 37/35, `vibe-doc-shell` 3/absent, `vibe-doc` 3/2, `vibe-registry` 9/8, `xtask` 15/14. The CLI-progress diff itself adds no serde derive. Exact post-baseline files were enumerated against commit `10f37bd21`; affected progress tests, strict clippy, and specmap are independently green. |
| @fact:B173-SEVERITY **severity** | P1 — blocks the mandatory release panel and therefore tag/release publication; the current CLI-progress product change is not the source of the drift. |
| @fact:B173-DISPOSITION **disposition** | `open` — classify all 13 files. Move VibeVM-owned formats to registered JTD/codegen; raise the baseline only for proven config, CLI-local, foreign-response, or test-only shapes, naming each decision. Then rerun the complete panel from step 1. |
| @fact:B173-FILED **filed by** | CLI-PROGRESS-ALL final release gate, 2026-09-17. |

## B-174 — a multi-line table cell in the planning flow blinds two whole-corpus gates {#b-174}

| field | value |
|---|---|
| @fact:B174-WHAT **what** | `vibevm/vibepacks/org.vibevm.world/multi-user-planning/v1.0.0/vibevm/vibespecs/flows/multi-user-planning/interruption-recovery.xml` writes the text of its `<td>` cells across several lines from line 30 on. The facts reader refuses the file — «a `<td>` cannot hold `|` or a newline» — and every reader of the whole corpus stops there: `vibe facts check --exhaustive` at the repository root, and `vibe doc check --coverage`. |
| @fact:B174-EVIDENCE **evidence** | 2026-09-25 on `main`: both commands exit 1 with that message and judge nothing else. The file last changed in `800d7d87e` (2026-09-12). A copy of the host spec tree alone passes the facts check clean (91 files, six old `FoldLossy` warnings); the four documentation obligations `PROP-057#NAV-CHAPTERS*` added that day were matched to their citing pages by hand, because the coverage gate could not run. |
| @fact:B174-SEVERITY **severity** | P2 — two gates have reported nothing for a whole corpus for two weeks while looking like a tool failure; no product runtime is affected. |
| @fact:B174-DISPOSITION **disposition** | `open` — keep each cell's text on one line (or turn the table into a list) in the canonical package, reinstall its projection, and rerun both gates at the root. The edit changes the package's content hash, so it is a commit of its own and not part of a documentation feature. |
| @fact:B174-FILED **filed by** | DOCS-LEARNING-ORDER (M-015), 2026-09-25. |

## B-175 — a registry count in a wire test missed the fourteenth CLI report {#b-175}

| field | value |
|---|---|
| @fact:B175-WHAT **what** | `crates/vibe-wire/tests/requirements_report_wire.rs:102` counts the `cli-*` rows of `formats/REGISTRY.toml` and asserts 13, to prove that the requirements report did not join the CLI report family. `53f1521a5` (2026-09-17) registered `[format.cli-application-report]`, the fourteenth such row, and did not move the count. The test `the_format_is_inventoried_under_a_surface_neutral_id` has failed since. |
| @fact:B175-EVIDENCE **evidence** | 2026-09-25 on `main` at `13da768aa`: `cargo test -p vibe-wire --test requirements_report_wire` gives 6 passed, 1 failed, `left: 14, right: 13`. The count was last set in `9cac46d13` (2026-09-09), an ancestor of `53f1521a5`. The distribution workflow does not run tests by default, so a release is not blocked. |
| @fact:B175-SEVERITY **severity** | P3 — every run of the whole `vibe-wire` suite is red for a reason that no product behaviour carries; the property that the test protects still holds. |
| @fact:B175-DISPOSITION **disposition** | `closed` — 2026-09-30, before the 1.0.1 release: the count is gone. The test keeps refusing a `cli-requirements-report` row and now pins the section heading that `[format.requirements-report]` stands under, so the `cli-*` family can grow without touching it. |
| @fact:B175-FILED **filed by** | DOCS-LEARNING-ORDER (M-015), 2026-09-25. |

## B-176 — a translation's borrowed examples render as empty frames {#b-176}

| field | value |
|---|---|
| @fact:B176-WHAT **what** | A translation must not author examples and points at the source's with `example ref` (PROP-057 `##LOC-EXAMPLE-REF`). The island and Markdown renderers resolve such a reference through `Content::examples` (`crates/vibe-doc/src/html.rs:275`, `md.rs:197`), but the build never fills that map: `crates/vibe-doc/src/build.rs:162` sets `examples: BTreeMap::new()`. Every borrowed example therefore renders as a frame that holds only its block number, marked `data-unresolved="true"`. |
| @fact:B176-EVIDENCE **evidence** | 2026-09-25: `vibe doc build --format html` of `org.vibevm.core/vibevm-docs-ru` at `d5f74e5af` leaves 62 unresolved example references on 34 pages; the English source has none. The live page `https://vibevm.org/doc/ru/org.vibevm.core/vibevm-docs/latest/start/what-vibevm-is/` shows block 02 as an empty frame where the English page shows `vibe --version` and its expected output. `vibe doc check --translations` finds the source's examples, so the source is reachable from a translation; only the build does not ask for them. |
| @fact:B176-SEVERITY **severity** | P2 — every example of the Russian manual is invisible to its readers, while every check stays green. |
| @fact:B176-DISPOSITION **disposition** | `closed` — 2026-09-26 (`9b8cd2489`, `9905a71c0`): the build, `vibe doc build-site`, the local reader and the agent fill each page's borrowed examples from the source page at the same address, found by the lookup `vibe doc check --translations` makes (`translations::adapted`). The bodies are keyed by page and then by id, because eight ids of the English manual repeat across pages and 23 of the 62 Russian references name one of them; a map keyed by bare id would have shown another page's command and reported no gap. The HTML and Markdown projections draw the body; the XML projection keeps `example ref`, as it keeps every address. A fixture pair whose two pages borrow one id with different bodies pins it. The Russian html build now has 0 unresolved frames and 62 resolved ones, the English pages come out byte-identical, and a host-only `vibe doc build-site` render leaves no gap in the three host versions. A build now also counts the gaps it leaves (`9905a71c0`); the site check that fails on a gap is [B-177](#b-177). vibevm.org shows the fix after the next release of vibe and the site image built with it. |
| @fact:B176-FILED **filed by** | DOCS-LEARNING-ORDER (M-015), found in the full-site preview, 2026-09-25. |

## B-177 — a gap cannot fail the site build while the documentation image shares a step with the product images {#b-177}

| field | value |
|---|---|
| @fact:B177-WHAT **what** | B-176 asked for a site check that fails when a published tree carries `data-unresolved`. `vibe doc build-site` runs inside `docker/vibevm-doc/Dockerfile`, and `.github/workflows/docker-images.yml` builds `vibevm`, `vibevm-dev`, `vibevm-zap` and `vibevm-doc` in one step and publishes them together. A render that exited non-zero on a gap would therefore withhold the product's images over a documentation defect, which PROP-057 `##INV-NO-RELEASE-GATE` rules out. The render measures instead (`9905a71c0`): each version's gaps are counted by kind, and a gap in the host's own documentation prints as a `warn` line. |
| @fact:B177-EVIDENCE **evidence** | 2026-09-26: the workflow step «Build all four images locally» runs the four `docker build` calls in one `set -euo pipefail` script, and «Publish mutable 1.0.0 and latest tags» comes after it; the site image renders with `RUN vibe doc build-site --config /site.toml --out /out --web /web`. Each kind of gap already has a check that fails on it — `vibe doc check --citations`, `--derived` and `--translations` — and since B-176 the translation check and the build find the source through one function. |
| @fact:B177-SEVERITY **severity** | P3 — every kind of gap is gated by a check and counted by the build; what is missing is a red at the site's own build. |
| @fact:B177-DISPOSITION **disposition** | `open` — an owner decision, because the change is to CI: build and publish `vibevm-doc` in a job of its own, then let `vibe doc build-site` exit non-zero when a host documentation has a gap. The host renders from the checkout with every input in hand, so a gap there is a defect; a registry package's gap stays the page `##SITE-RENDER-IDEMPOTENT` asks for. |
| @fact:B177-FILED **filed by** | the B-176 fix, 2026-09-26. |

## B-178 — the site's level-0 manifest keeps only the tables a hand-kept list names {#b-178}

| field | value |
|---|---|
| @fact:B178-WHAT **what** | `vibe doc build-site` renders every package through a level-0 copy whose `vibe.toml` it writes itself (`crates/vibe-doc/src/site/level0/card.rs`, `synthesise`), and that copy keeps only the tables named in `CARRIED`. `[glossary]` was not on the list, so vibevm.org resolved no glossary link and carried no definition, while `vibe doc build` and the local reader showed both (fixed in `d954a09f4`). Nothing ties the list to the manifest grammar: the next table the pipeline reads will reach the local reader and not the site, and no test will fail. |
| @fact:B178-EVIDENCE **evidence** | 2026-09-26: the candidate image built from `384e4e87e` served `start/what-vibevm-is` of the manual with no `data-gloss` link and no definitions block, and the rollout's smoke check stopped before the switch. A host-only `vibe doc build-site --no-web` at `d954a09f4` renders 12 glossary links and one definitions block on that page in both editions. |
| @fact:B178-SEVERITY **severity** | P3 — the one known instance is fixed and the rollout's smoke check now looks for glossary links; the list can drift again with the next table. |
| @fact:B178-DISPOSITION **disposition** | `open` — derive the check from the grammar: a test that walks every table `vibe-core` accepts in a documentation manifest and requires each to be on `CARRIED` or on a named list of tables the level-0 view drops on purpose, each with its reason. |
| @fact:B178-FILED **filed by** | DOCS-PUBLISH-2 (M-023), found by the rollout's smoke check, 2026-09-26. |

## B-179 — the machine store accepts and keeps a package entry that holds only its manifests {#b-179}

| field | value |
|---|---|
| @fact:B179-WHAT **what** | 44 entries of the owner's store `~/.vibe/cache/` (30 `org.vibevm.world`, 11 `org.vibevm.ai-native`, 3 `org.vibevm.fractality`, all `v1.0.0`) hold nothing but the `vibe.toml` files of the host checkout's in-tree packages, working-tree bytes rather than the published ones. Every `vibe install` of one of them on this machine materialises manifest-only slots and then fails compiling the boot lane (`document boot/30-flow-atomic-commits not found`). Three defects let such an entry in and keep it: the shippable walk swallows every walk error (`crates/vibe-registry/src/shippable.rs:38`, `.filter_map(\|e\| e.ok())`); the content hash is computed over the same short tree, so the sidecar agrees with it (`crates/vibe-registry/src/store.rs:401-413`, inserted through `crates/vibe-registry/src/local_registry.rs:196-205`); and a first install, with no lock pin, verifies an already-present entry against nothing (`crates/vibe-install/src/plan/fetch.rs:22-25`, `crates/vibe-registry/src/git_package_registry/fetch.rs:397-409`), while `vibe cache check` compares each entry with its own sidecar only (`crates/vibe-cli/src/commands/cache/check.rs:139-167`). |
| @fact:B179-EVIDENCE **evidence** | 2026-09-26: the entries' directories date from 2026-09-25 19:42 UTC; each file count equals the number of `vibe.toml` files in `vibevm/vibepacks/<group>/<name>/v1.0.0` (1 for most, 32 for `delegation-rules` and `fractality`, whose nested slots are manifest-only too); the sidecar of `git-atomic-commits` equals the hash of its one manifest. The published registry copies are complete. A fresh isolated store (`VIBE_SETTINGS` in a temporary folder) installs `org.vibevm.world/redbook` cleanly, 27 packages. The statement that produced the manifest-only source was not reproduced: `vibe doc build-site` with host, registry and both sources, and two `vibe cache add` probes, all insert complete entries (investigation `W3`, 2026-09-26). |
| @fact:B179-SEVERITY **severity** | P2 — one bad insert breaks every later install of that package on the machine, and no check reports it; the source and the registry stay intact. |
| @fact:B179-DISPOSITION **disposition** | `open` — propagate walk errors instead of swallowing them; refuse an insert whose tree lacks files its manifest or boot snippet names; carry the registry index's `content_hash` as the expected hash when no lock pin exists; give `vibe cache check` a rung that compares an entry with the registry, and let `--repair` re-fetch a mismatch; mark entries inserted from an in-tree source, so a consumer outside that project never receives one checkout's working copy as the published package. |
| @fact:B179-WORKAROUND **workaround** | Remove each manifest-only `v1.0.0/` directory and its sibling `v1.0.0.sha256` (a directory whose whole subtree holds only `vibe.toml`), then re-fetch from a folder that is not a vibevm project, so the projectless resolver reads `~/.vibe/registry.toml` rather than the checkout's in-tree packages. |
| @fact:B179-FILED **filed by** | DOCS-FIRST-PROJECT (M-024), found walking the first-project tutorial, 2026-09-26. |
| @fact:B179-REPAIRED **owner's store** | Repaired 2026-09-26 by the workaround, with the owner's consent: the 44 entries moved to `~/.vibe/cache-quarantine-20260926/` rather than deleted, then re-fetched one by one from the default registry outside any project; `vibe cache check` answers 60 ok, 0 mismatched, and no entry holds only manifests. The product defect stays open. |

## B-180 — five tests of `vibe-cli` fail on `main` {#b-180}

| field | value |
|---|---|
| @fact:B180-WHAT **what** | `cargo test -p vibe-cli --no-fail-fast` on `main` gives 1284 passed and 5 failed, in four targets. (1) `command_progress::tests::top_level_policy_inventory_matches_clap`: clap exposes a `run` verb that `TOP_LEVEL_COMMANDS` in `crates/vibe-cli/src/command_progress.rs` does not list. (2) Two tests of `cli_lifecycle_golden`: the `verify` phase reports `ok` where the golden pins `no-op`, and the `status` key of the `verification` object moved. (3) `cli_progress_sidecar::sidecar_absent_runs_cold` and (4) `cli_scrape_environment::execute_and_recover_without_home_refuse_bounded`: command progress narration, `[start] …` / `[done] …`, now reaches a stream both tests pin as free of it. |
| @fact:B180-EVIDENCE **evidence** | 2026-09-26, at `aa44b6e12`, in a clean worktree with its own target directory (investigation `TF`); none of the four targets is touched by the change that ran them. The release workflow runs without tests by default, so no release was blocked. |
| @fact:B180-SEVERITY **severity** | P3 — the suite is red for reasons that are each one small drift; the narration in (3) and (4) may also be a behaviour change in what a script reads. |
| @fact:B180-DISPOSITION **disposition** | `open` — add `run` to the inventory; decide whether `verify`'s `ok` is the intended result and re-pin or fix; find which change routed the narration into the pinned stream, and either keep narration out of it or re-pin with the reason. |
| @fact:B180-FILED **filed by** | DOCS-FIRST-PROJECT (M-024), found while gating the tree and init fixes, 2026-09-26. |

## B-181 — the redbook's packages still describe the layout from before the tree moved into `vibevm/` {#b-181}

| field | value |
|---|---|
| @fact:B181-WHAT **what** | 28 packages of `org.vibevm.world` (the redbook and its members) carry about 170 mentions of the pre-PROP-052 `spec/` layout: `spec/WAL.md`, `spec/flows/<flow>/…`, `spec/common/PROP-000`, `spec/boot/…`. Some sit in text an agent reads: the `wal` boot snippet says the flow owns «the protocol files under `spec/flows/wal/`», the `wal-status` skill says «Read `spec/WAL.md` end to end» and points at `spec/flows/wal/WAL-PROTOCOL.md`, and `WAL-PROTOCOL.xml` names `vibedeps/flow-wal/<version>/spec/flows/wal/`. The `addressable-specs` layout diagram draws a `spec/` tree. The current layout is `vibevm/vibespecs/` (the WAL `vibevm/vibespecs/WAL.xml`), which the same flows also name in other places. |
| @fact:B181-EVIDENCE **evidence** | 2026-09-26: a grep of `vibevm/vibepacks/org.vibevm.world/*/v1.0.0` for `` `spec/ ``, `spec/flows/`, `spec/WAL`, `spec/boot/`, `spec/common/` and `spec/modules/` outside `spec://` addresses: licensing 14, wal 13, addressable-specs 13, health-audit 8, git-attribution-policy 8, conflict-protocol 7, sixteen more with 1 to 6 each. The `vibe init` template had the same staleness and was fixed in `78415ee85`. |
| @fact:B181-SEVERITY **severity** | P2 — a new project's agent reads two layouts for one project, and may create a `spec/` tree the tools do not read. |
| @fact:B181-DISPOSITION **disposition** | `open` — one sweep over the redbook's texts that rewrites every path to the current layout (package-relative paths to `vibevm/vibespecs/…`, a consumer's paths to `vibevm/vibedeps/<group>.<name>/<version>/…`), leaving `spec://` addresses alone; then republish the changed packages, which is the owner's call. |
| @fact:B181-FILED **filed by** | DOCS-FIRST-PROJECT (M-024), found walking the first-project tutorial, 2026-09-26. |

## B-182 — `vibe list` misaligns its table and drops the group {#b-182}

| field | value |
|---|---|
| @fact:B182-WHAT **what** | `vibe list` pads its header to the longest package name but prints each row unpadded, so the columns drift; and the `NAME` column shows `redbook` rather than `org.vibevm.world/redbook`, which is ambiguous as soon as two groups publish one name. |
| @fact:B182-EVIDENCE **evidence** | 2026-09-26, a project with the redbook's 27 packages: the header reads `KIND  NAME                      VERSION  ROLE     BOOT SNIPPET`, the rows read `flow  redbook  1.0.0    package  —` (investigation `W1`). |
| @fact:B182-SEVERITY **severity** | P3 — the data is right and `--json` is unaffected; the table looks broken, so the first-project tutorial shows `vibe tree --plain` instead. |
| @fact:B182-DISPOSITION **disposition** | `open` — pad every row to the header's column widths and print the full coordinate; re-pin the manual's `vibe list` goldens. |
| @fact:B182-FILED **filed by** | DOCS-FIRST-PROJECT (M-024), 2026-09-26. |

## B-183 — `vibe install` without a terminal fetches the whole closure before it refuses {#b-183}

| field | value |
|---|---|
| @fact:B183-WHAT **what** | Without `--assume-yes` and without a terminal to ask on, `vibe install` resolves and fetches every package of the plan, and only then fails with `no TTY available for confirmation; re-run with --assume-yes`. It could know that before the first fetch. |
| @fact:B183-EVIDENCE **evidence** | 2026-09-26, `vibe install org.vibevm.world/redbook` from a script on a cold store: exit 1 after 1 min 48 s and 27 fetched packages; the project was left untouched (investigation `W1`). |
| @fact:B183-SEVERITY **severity** | P3 — nothing is damaged and the store keeps the download; a scripted first run wastes two minutes and reads as a failure. |
| @fact:B183-DISPOSITION **disposition** | `open` — detect the missing terminal before fetching; print the resolved plan, if it can be computed without fetching, and the `--assume-yes` hint. |
| @fact:B183-FILED **filed by** | DOCS-FIRST-PROJECT (M-024), 2026-09-26. |

## B-184 — the boot lane reads the git practices twice {#b-184}

| field | value |
|---|---|
| @fact:B184-WHAT **what** | In a project with the redbook, the root static lane already splices the four static git practices, and `INDEX.md` also names `vibevm/vibedeps/org.vibevm.world.git-practices/1.0.0/vibevm/vibespecs/boot/STATIC.xml` (`STATIC.md` under the mixed target) as a static entry: a file byte-identical in size to the root lane. An agent that follows the lane reads the same rules twice at every session start. |
| @fact:B184-EVIDENCE **evidence** | 2026-09-26, `spec_format = "xml"`: root `STATIC.xml` 37 217 B, git-practices' `STATIC.xml` 37 217 B, 75 matches of the same headings in each; the lane is 32 files and about 188 KB in all. An agent in the `W1` run named the duplicate unprompted. |
| @fact:B184-SEVERITY **severity** | P2 — about 37 KB, some nine thousand tokens, of every session start is spent twice, in every project that installs the redbook. |
| @fact:B184-DISPOSITION **disposition** | `open` — a normal package's own compiled lane must not enter the consumer's `INDEX.md` when its contributions are already spliced into the consumer's static lane; find which of the two the loading model intends and remove the other. |
| @fact:B184-FILED **filed by** | DOCS-FIRST-PROJECT (M-024), 2026-09-26. |

## B-185 — a documentation package cannot declare the addresses that only illustrate {#b-185}

| field | value |
|---|---|
| @fact:B185-WHAT **what** | The citation check tells an illustrative `spec://` address from a citation by constants in `crates/vibe-doc/src/citations.rs`: the teaching group `org.acme` and, since `9a41d8d1e`, the official manual's tutorial projects `hello-vibe` and `hello-vibevm`. Any other documentation package whose tutorial shows a reader's project address has no way to say so, and its check turns red. |
| @fact:B185-EVIDENCE **evidence** | 2026-09-26: `tutorials/ai-native-rust.xml` printed `spec://hello-vibevm/modules/calculator/PROP-001#division-by-zero` twice and the check reported both as `names no package`; a package group may be a single dotless segment, so «no dot means a project» is not a safe rule. |
| @fact:B185-SEVERITY **severity** | P3 — the official manual is served by the constants; another author hits the same wall. |
| @fact:B185-DISPOSITION **disposition** | `open` — let a documentation package declare its teaching namespaces in its manifest (for example `[citations] teaching = [...]`), and move the manual's two names there. |
| @fact:B185-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-186 — the documentation examples' sandbox inherits `VIBEVM_HOME` {#b-186}

| field | value |
|---|---|
| @fact:B186-WHAT **what** | `VIBEVM_HOME` is not among the variables the examples sandbox clears (`crates/vibe-doc/src/examples/sandbox.rs`, `CLEARED`), so a machine that sets it turns `reference/settings-and-environment.xml#vars` red: the page's golden shows the sandbox's value, the run prints the machine's. |
| @fact:B186-EVIDENCE **evidence** | 2026-09-26, on the owner's machine, `vibe doc check --examples` of `org.vibevm.core/vibevm-docs` (investigation `W4`). |
| @fact:B186-SEVERITY **severity** | P3 — one example, one machine configuration; the gate is wrong, not the page. |
| @fact:B186-DISPOSITION **disposition** | `open` — add `VIBEVM_HOME` to the cleared set, beside the other variables that point vibe at a machine's own folders. |
| @fact:B186-FILED **filed by** | DOCS-FIRST-PROJECT (M-024), 2026-09-26. |

## B-187 — AI-Native Rust tools misread a project run from an absolute path or a renamed checkout {#b-187}

| field | value |
|---|---|
| @fact:B187-WHAT **what** | Two tools of `org.vibevm.ai-native/rust-ai-native` depend on where the project sits rather than on the project. The MCP server, registered by `vibe mcp install` with an absolute `--path`, runs the specmap engine under that path, which renames every symbol and reports the committed `specmap.json` as stale. And `conform` names a single-crate project's crate after the checkout's folder, so a clone into another folder answers «no crate directory matches it». |
| @fact:B187-EVIDENCE **evidence** | 2026-09-26, investigation `W2` in a sandbox `hello-vibevm`: the agent never used the connected 18-tool server for that reason, and the checkout-name defect reproduced by hand. |
| @fact:B187-SEVERITY **severity** | P2 — the server an install registers does not work on the project it was registered for, and a clone under another name breaks the gate. |
| @fact:B187-DISPOSITION **disposition** | `open` — resolve symbols relative to the crate root whatever the working path; name a single crate from its `Cargo.toml` package name, not from the folder. |
| @fact:B187-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-188 — installing AI-Native Rust leaves the reader three more commands and four unnamed prerequisites {#b-188}

| field | value |
|---|---|
| @fact:B188-WHAT **what** | `vibe install org.vibevm.ai-native/rust-ai-native` copies source and stops. The tools need `vibe bin build` (142 crates, about 370 MB in two slots that share no `target/`), the server needs `vibe mcp install` (which, bare, writes into five agents at user scope), and the skills need `vibe skill install`; the install's output names none of them. The checks call `rustfmt`, `clippy`, `cargo-nextest` and, for the type oracle, `rust-analyzer`, and nothing checks for them before `floor` fails. The specmap engine's default roots also index the generated boot lane and the WAL, so 82 of 91 units in a fresh project are the redbook's text under the project's name, and its first-run warning («a bare parent directory scans nothing») is wrong for a single-crate layout. |
| @fact:B188-EVIDENCE **evidence** | 2026-09-26, investigation `W2`: `vibe bin list` showed five tools `not built` after the install; `floor` passed only after `cargo-nextest` existed; `spec_exclude` for `boot/` and `WAL.*` was added by hand. |
| @fact:B188-SEVERITY **severity** | P2 — every newcomer walks into each of these, and the advanced tutorial has to teach around them. |
| @fact:B188-DISPOSITION **disposition** | `partly resolved` 2026-11-10 (APP-VERSIONS P7e) — the first half is delivered and is the acceptance PROP-065 `#ordinary-consumer` measures that decision by: `vibe bin build` and `vibe mcp install` are declared `slot:post-install` contributions of the packages that own them (`rust-ai-native-lang`, `rust-ai-native-mcp`), so both appear in `vibe install --plan-only` with their titles before anything runs, the four prerequisites are `needs` that refuse the installation up front naming the missing one instead of surfacing as a failed `floor`, and the registration is `optional = true` so writing into the user's agents cannot fail the install. **Still open**: `vibe skill install` is not declared (its receipt format and ownership rules want their own pass); `rust-ai-native init` still does not check its toolchain; `vibevm/vibespecs/boot/**` and `WAL.*` are still not excluded from the default spec roots; the bare-parent-directory warning's condition is still wrong. |
| @fact:B188-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-189 — the manual's `package` example no longer matches `vibe package` {#b-189}

| field | value |
|---|---|
| @fact:B189-WHAT **what** | `lifecycle/build-package-deploy.xml#package` in `org.vibevm.core/vibevm-docs` expects `→ build: no-op` and `→ verify: no-op`. In the `hello-deploy` fixture, `vibe package --path hello-deploy --assume-yes` now prints `→ build: ok` and `→ verify: ok`: the fixture's declared `build:cargo` artifact is built during `package`. The golden dates from `7c56a0037` (2026-09-14). |
| @fact:B189-EVIDENCE **evidence** | 2026-09-26, `vibe doc check --examples` of the manual: one example differs. It differs the same way with `VIBEVM_HOME` unset and in a clean worktree at `78415ee85`, so it predates the tutorial work that found it. |
| @fact:B189-SEVERITY **severity** | P3 — the examples gate runs by hand before a release, and until then it reads red on `main`. |
| @fact:B189-CAUSE **cause** | `b02e260c9` (2026-09-17, «feat(cli): make long waits observable») made a phase without contributions end `ok` when it has a built-in binding: build, when the manifest declares build targets (`crates/vibe-orchestrator/src/dispatch/mod.rs`), and verify, once the evidence boundary fires. Cargo already built the artifact during `package` when the golden was captured; only the reported status changed. PROP-054 `#INVOKE-RUNS-PRIORS` keeps `no-op` for a phase with no contributions and no built-in binding, so the new status is the intended one. |
| @fact:B189-DISPOSITION **disposition** | `resolved` 2026-09-26 — the golden was re-blessed with this cause. Left over: `b02e260c9` changed stdout although its changelog line says it did not; two tests of `crates/vibe-cli/tests/cli_lifecycle_golden.rs` stay red for the same reason (item 2 of `B-180`); the `NoOp` comment in `crates/vibe-orchestrator/src/values.rs` is out of date; and the build step never reports `fresh`, so an up-to-date target still prints `build: ok`. |
| @fact:B189-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-190 — `rust-ai-native floor` reports specification drift as a fresh project {#b-190}

| field | value |
|---|---|
| @fact:B190-WHAT **what** | When the map check inside `floor` fails, `floor` prints only ``floor: `specmap` failed (fresh project? run `rust-ai-native specmap` once to mint the index)``. After a revision bump the index exists and is committed; the real cause is drift, which `specmap --check` run alone prints line by line, naming every suspect item. The hint sends the reader to the wrong problem at the moment the link between rule and code matters most. |
| @fact:B190-EVIDENCE **evidence** | 2026-09-26, investigation `W5`, step 6 of the advanced tutorial: two revision bumps, five suspects, and the `floor` message above. |
| @fact:B190-SEVERITY **severity** | P2 — the one diagnostic the discipline's central loop depends on is misleading. |
| @fact:B190-DISPOSITION **disposition** | `open` — carry the drift lines of `specmap --check` through `floor`, and offer the fresh-project hint only when no index exists. The tutorial teaches `specmap --check` meanwhile. |
| @fact:B190-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-191 — the AI-Native boot texts name paths from before the tree moved into `vibevm/` {#b-191}

| field | value |
|---|---|
| @fact:B191-WHAT **what** | The boot snippets of `org.vibevm.ai-native/core-ai-native` and `org.vibevm.ai-native/rust-ai-native-lang`, which every session of an installing project reads, send the agent to `spec/00-MANIFESTO.xml`, `spec/rust/GUIDE-AI-NATIVE-RUST.xml`, `spec/cards/INDEX.xml` and similar. The packages keep those files under `vibevm/vibespecs/`. The same defect in the redbook is `B-181`. |
| @fact:B191-EVIDENCE **evidence** | 2026-09-26, the two boot files of a sandbox project after `vibe install org.vibevm.ai-native/rust-ai-native` (investigation `W5`). The agent there found the files by searching; a weaker reader follows the path and finds nothing. |
| @fact:B191-SEVERITY **severity** | P3 — a strong agent recovers, and the cost is turns. |
| @fact:B191-DISPOSITION **disposition** | `open` — rewrite the paths in both boot snippets, and sweep the family's other texts for the old layout in the same change. |
| @fact:B191-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-192 — a project's specifications may cite addresses that resolve nowhere, and nothing says so {#b-192}

| field | value |
|---|---|
| @fact:B192-WHAT **what** | `vibe check` passes a project whose specifications cite `spec://…` addresses that no unit carries. Two ways lead there. An agent guesses the namespace: in a walk of the first-project tutorial, Codex wrote every cross-reference as `spec://hello-vibevm/PROP-001#…`, while the project's index minted `spec://project/modules/hello-vibevm/PROP-001#…`. And the namespace moves under the citations: without a `specmap.toml` the map engine mints `spec://project/…` (`core-ai-native-specmap/src/config.rs`, the default `namespace = "project"`), and `rust-ai-native init` writes `namespace = "<project name>"`, so every citation written before the discipline was set up dies at that moment. |
| @fact:B192-EVIDENCE **evidence** | 2026-09-26, investigation `W1`, step 5 (Codex 0.152.1, `gpt-5.6-sol`): `vibe explain "spec://hello-vibevm/PROP-001#invocation"` → `no spec unit with URI … in the index`, and `vibe check` clean. The namespace move: the default in `config.rs` against the `specmap.toml` that `rust-ai-native init` wrote in investigation `W5`. |
| @fact:B192-SEVERITY **severity** | P3 — the citations read as prose to a person and lead nowhere for a machine, and no check tells either of them. |
| @fact:B192-DISPOSITION **disposition** | `open` — resolve a project's own `spec://` citations in `vibe check` and warn on the dead ones, as the host's gate does for its own addresses (`B-076`); have `rust-ai-native init` rewrite or report the `spec://project/` citations it is about to orphan. The advanced tutorial tells the reader to replace them by hand meanwhile. |
| @fact:B192-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-193 — the map check fails on a Windows checkout of `specmap.json` and does not say why {#b-193}

| field | value |
|---|---|
| @fact:B193-WHAT **what** | `specmap --check` compares the index it would write with `specmap.json` byte for byte. With `core.autocrlf = true`, the default of Git for Windows, any checkout of the file writes CRLF line endings, and the check fails with ``.\specmap.json` is out of date relative to the tree`` and no drift line; inside `floor` the same failure reads «fresh project?» (`B-190`). The content is identical: a rebuild writes the same index with LF, and `git status` stays clean throughout. |
| @fact:B193-EVIDENCE **evidence** | 2026-09-26, the sandbox of investigation `W5`: after `git checkout -- specmap.json` the file held 685 CR bytes and the check failed; with `specmap.json text eol=lf` in `.gitattributes` the same checkout kept LF and the check was clean. |
| @fact:B193-SEVERITY **severity** | P2 — on the most common Windows setup, the discipline's gate goes red after an ordinary git operation, with a message that points elsewhere. |
| @fact:B193-DISPOSITION **disposition** | `open` — compare the parsed index, or normalise line endings before comparing; have `rust-ai-native init` add `specmap.json text eol=lf` to `.gitattributes`; name the cause when only the bytes differ. The advanced tutorial tells Windows readers to add the attribute meanwhile. |
| @fact:B193-FILED **filed by** | DOCS-AI-NATIVE-TUTORIAL (M-025), 2026-09-26. |

## B-194 — `vibe registry add` cannot add a local folder, and its refusal speaks of publishing {#b-194}

| field | value |
|---|---|
| @fact:B194-WHAT **what** | `vibe registry add my-vault file:///C:/…/vibevm/vibepacks` fails with «publish refused: cannot derive an organization segment from `file:///…`»: the command requires a host and an organisation (`crates/vibe-cli/src/commands/registry/config/add.rs:58-61`) and borrows the publisher's error (`crates/vibe-publish/src/lib.rs:143`). A hand-written `[[registry]]` with the same URL works everywhere else, the manual's own root fixture included. The command also has no way to write the machine file `~/.vibe/registry.toml`, only a project's manifest. `model/registries.xml` tells the reader to add a registry with it, and `vibe workspace publish` names it in its own refusal. |
| @fact:B194-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, steps 4b–4c. |
| @fact:B194-SEVERITY **severity** | P2 — the machine-wide local registry the tutorial teaches can only be written by hand. |
| @fact:B194-DISPOSITION **disposition** | `open` — accept a `file:///` root (no organisation to derive), add a machine scope that writes `~/.vibe/registry.toml`, and word the refusal as the command's own. The Obsidian tutorial writes the entry by hand meanwhile. |
| @fact:B194-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-195 — `--registry` and `[[registry]] url` take opposite forms of a local address {#b-195}

| field | value |
|---|---|
| @fact:B195-WHAT **what** | `vibe install --registry` takes a folder's path and fails on a `file:///` URL with a raw operating-system error (`os error 123`, `os error 3`); a `[[registry]] url` requires the `file:///` form and sends a bare path to the git back end. A reader who learns one form and applies it to the other gets an error that names neither rule. |
| @fact:B195-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, step 4a. |
| @fact:B195-SEVERITY **severity** | P3 — one form works in each place; the cost is a confusing message. |
| @fact:B195-DISPOSITION **disposition** | `open` — accept both forms in both places, or refuse the wrong one with a sentence that names the right one. The Obsidian tutorial states the difference in its edge cases. |
| @fact:B195-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-196 — `vibe reinstall --force` leaves the lock's content hash stale {#b-196}

| field | value |
|---|---|
| @fact:B196-WHAT **what** | After a package's source changed, `vibe reinstall --force --assume-yes` copies the new bytes into `vibevm/vibedeps/` and updates the slot's `source_hash`, but `vibe.lock` keeps the old `content_hash`. The project then holds two records that disagree, and `vibe check` and `vibe validate` are both clean. A later verified install from the lock would meet a fingerprint that no longer matches. |
| @fact:B196-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, steps 4d and 6 (the lock still `f5199e11…`, the slot `43239f8a…`). |
| @fact:B196-SEVERITY **severity** | P2 — a lock that no longer describes what is installed, reported by nothing. |
| @fact:B196-DISPOSITION **disposition** | `done — landed on main 2026-11-02 as fix(reinstall): a forced reinstall refuses bytes the lock does not pin` — the lock pins bytes: `vibe reinstall --force` refuses with `lock_hash_mismatch` when a refetch, from any source kind, hashes to anything but the locked `content_hash`, before anything reaches `vibedeps/`; `vibe update` is what re-records. `vibe check` gains the `lock_slot_hash` error for a slot record that disagrees with its lock row. |
| @fact:B196-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-197 — a user-scope skill install writes into the real home on Windows, whatever `HOME` says {#b-197}

| field | value |
|---|---|
| @fact:B197-WHAT **what** | With `HOME` and `USERPROFILE` pointed at a sandbox, `vibe skill install --scope user` still plans `C:/Users/<name>/.claude/skills/…`, `.config/opencode/skills/…` and `.agents/skills/…`: the user-scope paths go through `dirs::home_dir()` (`crates/vibe-agent-projection/src/agents.rs`), which on Windows asks the operating system and ignores the environment. Helpers that take an injected home exist (`agents/home_paths.rs`) and this command does not use them. |
| @fact:B197-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, step 3 (`vibe skill install --dry-run --scope both`). |
| @fact:B197-SEVERITY **severity** | P2 — no sandbox, the documentation's own example runner included, can contain a user-scope projection on Windows. |
| @fact:B197-DISPOSITION **disposition** | `open` — route the user scope through the injectable home, honouring `VIBE_SETTINGS` or an explicit override. The tutorial uses `--scope project`. |
| @fact:B197-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-198 — `vibe list` prints no boot snippet for any package {#b-198}

| field | value |
|---|---|
| @fact:B198-WHAT **what** | The BOOT SNIPPET column of `vibe list` shows `—` for a package whose `[boot_snippet]` is declared and whose snippet stands in `INDEX.md`. The installer never writes the lock's `boot_snippet` field (`crates/vibe-core/src/manifest/lockfile.rs:371`), and `crates/vibe-cli/src/commands/list.rs:205` falls back to `—`. The manual's example in `model/packages-and-kinds.xml` shows the same dash. |
| @fact:B198-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, step 3 (`vibe list` after installing `org.example/my-skills`). |
| @fact:B198-SEVERITY **severity** | P3 — a column that has never carried a value. |
| @fact:B198-DISPOSITION **disposition** | `open` — record the snippet at install, or drop the column. |
| @fact:B198-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-199 — an install from a local folder waits for every remote registry listed before it {#b-199}

| field | value |
|---|---|
| @fact:B199-WHAT **what** | A package that only a local `file:///` registry holds still costs a round trip to each remote registry the walk meets first: 5.9 s with the local entry last in `~/.vibe/registry.toml`, 1.4 s with it first, 0.15 s with `--offline`. Re-resolving an in-tree package made nine resolver calls of about half a second each; with `--offline` each took no time. |
| @fact:B199-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, steps 3 and 4c. |
| @fact:B199-SEVERITY **severity** | P3 — seconds per install, and a network dependency for work that needs none. |
| @fact:B199-DISPOSITION **disposition** | `done — landed on main 2026-11-02 as perf(registry): ask each remote registry about a package once per run` — a per-run negative cache in the resolver: a remote's «not here» (missing package, or a walked-past public 401 / 403) is replayed for the rest of the run. Local-first was ruled out: PROP-002 `##REGISTRY-WALK-ORDER` makes array order absolute. The tutorial still puts the local entry first, which skips the remotes entirely. |
| @fact:B199-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-200 — publishing: a rehearsal that needs a token, and a GitVerse the CLI does not publish to {#b-200}

| field | value |
|---|---|
| @fact:B200-WHAT **what** | `vibe registry publish <slot> --dry-run` loads a token and calls the host's API (it fails with «token lacks `repo:create` permission» against an organisation the token cannot write), while `vibe workspace publish --dry-run` needs neither. Publishing to GitVerse is a stub in the CLI (`crates/vibe-cli/src/commands/registry/publish.rs:135-146`) though `howto/publish-a-package.xml` says GitVerse is known, and `vibe registry publish` reads only the project's `[[registry]]` list, never `~/.vibe/registry.toml`. |
| @fact:B200-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, step 7; the registries survey of the same day. |
| @fact:B200-SEVERITY **severity** | P2 — the manual's publishing page promises a harmless rehearsal and a second host. |
| @fact:B200-DISPOSITION **disposition** | `open` — make the rehearsal offline, implement or remove the GitVerse adapter, read the machine list; until then correct `howto/publish-a-package.xml` (see also `B-133`). |
| @fact:B200-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-201 — the authoring pages disagree with what the scaffold writes {#b-201}

| field | value |
|---|---|
| @fact:B201-WHAT **what** | `authoring/write-a-flow.xml` writes the snippet at `vibevm/vibespecs/boot/review-notes.xml`, while `vibe init package` writes `vibevm/vibespecs/boot/10-<kind>-<name>.md` and points `[boot_snippet].source` there; the same page says a package is `simple` by default, while the scaffold writes `format = "normal"`. `start/first-project.xml` says `.vibe-slot.toml` names each file `copied` or `converted`; on the Markdown lane each `[[file]]` carries only a path and a hash. `howto/set-up-a-workspace.xml` and `howto/publish-a-package.xml` do not say that `vibe workspace publish` needs a `[workspace]` table and a `[[registry]]` in the root manifest. |
| @fact:B201-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, steps 3 and 6. |
| @fact:B201-SEVERITY **severity** | P2 — a reader who follows the pages to the letter ends with a snippet the manifest does not name. |
| @fact:B201-DISPOSITION **disposition** | `open` — correct the three pages against the scaffold, or change the scaffold where the page states the intended behaviour. |
| @fact:B201-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-202 — `vibe init` ends with a hint in a retired syntax {#b-202}

| field | value |
|---|---|
| @fact:B202-WHAT **what** | `vibe init` and `vibe init package` end with «install packages with `vibe install <kind>:<name>` (e.g. flow:wal)», a coordinate form the resolver no longer takes, and the package scaffold reports «Done. Project `org.example/my-skills`» about a package. |
| @fact:B202-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`, steps 1 and 3. |
| @fact:B202-SEVERITY **severity** | P3 — the first advice a new project gets is wrong. |
| @fact:B202-DISPOSITION **disposition** | `done` — landed on main 2026-11-03 as `fix(init): today's install hint, --link honoured, --translates parsed and escaped`. |
| @fact:B202-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-203 — PROP-030 still names the retired `packages/` folder {#b-203}

| field | value |
|---|---|
| @fact:B203-WHAT **what** | `PROP-030 ##LOCAL-AUTO-OPEN` says a project carrying `<project_root>/packages/` gets it opened as a local registry; the code opens `vibevm/vibepacks/` (`crates/vibe-core/src/layout.rs`, `crates/vibe-package-source/src/project_local.rs`). The manual's registries page quotes the rule and now says which folder vibe really opens. |
| @fact:B203-EVIDENCE **evidence** | 2026-09-27, the registries survey and the sandbox walk of `DOCS-OBSIDIAN-TUTORIAL`. |
| @fact:B203-SEVERITY **severity** | P3 — a normative rule that describes the layout before the move into `vibevm/`. |
| @fact:B203-DISPOSITION **disposition** | `open` — amend the rule by the owner's hand. |
| @fact:B203-FILED **filed by** | DOCS-OBSIDIAN-TUTORIAL (M-051), 2026-09-27. |

## B-204 — the example runner's tripwire fires on another session's hook {#b-204}

| field | value |
|---|---|
| @fact:B204-WHAT **what** | `vibe doc check --examples` failed with «the runner changed state outside its sandbox: changed: home:steward/contexts/<id>/records» after all 61 examples passed: a pre-compaction hook of another agent session wrote a record under `~/.vibe/steward/` during the run. |
| @fact:B204-EVIDENCE **evidence** | 2026-09-27, the first full manual check of the batch, while three agents worked in the background. |
| @fact:B204-SEVERITY **severity** | P3 — a false red for the gate whenever agents work beside it. |
| @fact:B204-DISPOSITION **disposition** | `open` — run the examples gate only when no other session is live, or keep session records out of `~/.vibe`; widening the tripwire is forbidden by the runner's own rule and needs the owner. |
| @fact:B204-FILED **filed by** | DOCS-PUBLISH-4 (M-055), 2026-09-27. |

## B-205 — a merge mode is lost when a Markdown heading converts to XML {#b-205}

| field | value |
|---|---|
| @fact:B205-WHAT **what** | A heading suffix `:replace` or `:add` (PROP-035 §7.3) does not survive conversion: `vibe refactor convert-source --to xml --dry-run` shows `## Tests before fixes {#tests-first} :replace` becoming `{#tests-first}`, and the installed `details.xml` has no trace of it. The pivot's `Section` carries no field for it (`crates/vibe-specdoc/src/doc.rs:88-103`) and `split_anchor` drops what follows `}` (`crates/progress-core/src/parse/units.rs:46-57`). Under `spec_format = "xml"` the compiler then reads the merge as `:add`, and a redeclared fact fails with «id TESTS-FIRST is declared twice in the merged document». |
| @fact:B205-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-MD-XML-TUTORIAL` (`a-convert-dry-run-source`, `a-cat-source-xml`, `e-install-xml-replace`). |
| @fact:B205-SEVERITY **severity** | P2 — `#INHERITANCE-PARITY` promises a format-blind inheritance machinery, and one of its modes is dropped by the conversion itself. |
| @fact:B205-DISPOSITION **disposition** | `open` — carry the merge mode through the pivot and both printers. The tutorial avoids merge modes and says the merged result does not compile under `xml` today. |
| @fact:B205-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-206 — the XML static lane refuses a source that shares an anchor with its contract {#b-206}

| field | value |
|---|---|
| @fact:B206-WHAT **what** | Under `spec_format = "xml"`, `emit:static-xml` stops with «fact id @fact:org-acme--review--root is defined twice» when a `#source` merge shares any anchor with its contract, two `{#root}` titles included: both qualify to the same label (PROP-035 `#PIPE-QUALIFY`). The same package compiles under the default target. The failed install leaves the slot on disk, `vibe.lock` unwritten and no `STATIC.xml`, and its message names `vibevm/vibespecs/boot/INLINE.md`, the file's name before 2026-07-16. |
| @fact:B206-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-MD-XML-TUTORIAL` (`a-install`, `c-install-xml-disjoint`, `a-ls-vibedeps`, `a-cat-lock`, `b-cat-static-md`). |
| @fact:B206-SEVERITY **severity** | P2 — the target the manual recommends rejects the inheritance the specification promises, and a failed install is not atomic. |
| @fact:B206-DISPOSITION **disposition** | `done — landed on main 2026-11-02 as fix(install): a failed install rolls back and the XML lane compiles a shared-anchor merge` — the XML target keeps a repeated heading label on the first heading and emits later ones unanchored (PROP-035 `##XML-CONTINUED-HEADING`), so both targets define the same labels; the apply keeps an undo log from the first slot change to the lock write and restores slots and `vibe.toml` on failure (PROP-009 `##INSTALL-ROLLBACK`); the refusal names `STATIC.xml` / `STATIC.md`. The tutorial's disjoint-anchors edge case can now be relaxed. |
| @fact:B206-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-207 — the shorthand status vocabulary is not closed {#b-207}

| field | value |
|---|---|
| @fact:B207-WHAT **what** | `vibe facts check` passes `@status:spec/finished` and `@status:spek/done` as clean: the shorthand is not read as a marker when its values are unknown, while the element form `state="finished"` is refused with the value named. PROP-043 `#VOCAB-CLOSED` and `authoring/facts-and-status-markers.xml` say every vocabulary is closed. |
| @fact:B207-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-MD-XML-TUTORIAL` (`h-facts-check-bad-shorthand`, `h-facts-check-bad-state-element`). |
| @fact:B207-SEVERITY **severity** | P2 — a misspelt status in the form most authors type passes the gate silently. |
| @fact:B207-DISPOSITION **disposition** | `open` — refuse an `@status:` token whose values are outside the vocabulary. The tutorial names the vocabulary and the gap. |
| @fact:B207-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-208 — every ordinary Markdown file converts with a «loss» {#b-208}

| field | value |
|---|---|
| @fact:B208-WHAT **what** | The reverse projection of `vibe refactor convert-source` appends one blank line to a Markdown file, so an ordinary file comes back `ir-stable-loss` and the command refuses it without `--force`. The loss is only that line; `--force` has become the everyday form of the command. |
| @fact:B208-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-MD-XML-TUTORIAL` (`c2-convert-plain`, the tutorial's `convert-dry-run` example). |
| @fact:B208-SEVERITY **severity** | P3 — the honesty check cries wolf on every file. |
| @fact:B208-DISPOSITION **disposition** | `open` — print without the trailing blank line, or treat a final blank line as insignificant. The tutorial shows the diff and explains it. |
| @fact:B208-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-209 — the compiled lane's header comments show labels in escaped form {#b-209}

| field | value |
|---|---|
| @fact:B209-WHAT **what** | `STATIC.xml` explains its qualified labels in comments as `<origin-slug>-%2D<original>`, because XML forbids `--` inside a comment, while the elements below carry `--`. The first lines an agent reads describe a form that appears nowhere else in the file. |
| @fact:B209-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-MD-XML-TUTORIAL` (`t18-cat-static-xml`; the tutorial's `static-xml` example). |
| @fact:B209-SEVERITY **severity** | P3 — misleading guidance at the top of the agent's first file. |
| @fact:B209-DISPOSITION **disposition** | `open` — describe the label without two hyphens in a row, for example with a placeholder word, or state the escape in the comment. The tutorial explains it. |
| @fact:B209-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-210 — dotted heading ids pass the facts check and resolve nowhere {#b-210}

| field | value |
|---|---|
| @fact:B210-WHAT **what** | `{#verification.timeout}` passes `vibe facts check`, but no address reaches it: a dot in a fragment is a tree path (`crates/vibe-spec/src/address.rs:219`, `doctree.rs:263-276`). The addressable-specs protocol still shows such an id as an example. |
| @fact:B210-EVIDENCE **evidence** | 2026-09-27, the sandbox walk of `DOCS-MD-XML-TUTORIAL` (`d-install-dotted-embed`, `d2-install-tree-path-embed`). |
| @fact:B210-SEVERITY **severity** | P3 — an id that looks valid and cannot be cited. |
| @fact:B210-DISPOSITION **disposition** | `open` — refuse a dot in a heading id at the check, and correct the protocol's example. |
| @fact:B210-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-211 — `vibe init package` ignores `--link` and contradicts its help on `--format` {#b-211}

| field | value |
|---|---|
| @fact:B211-WHAT **what** | `vibe init package … --link static` still writes `link = "dynamic"`. The format-help mismatch recorded here was corrected during root declaration selection on 2026-10-02: help now reports the existing `normal` default. |
| @fact:B211-EVIDENCE **evidence** | 2026-09-27, the sandbox walks of `DOCS-MD-XML-TUTORIAL` (`f-cat-link-static`, `f-cat-defaults`) and `DOCS-OBSIDIAN-TUTORIAL`. |
| @fact:B211-SEVERITY **severity** | P3 — two flags that do not do what their help says. |
| @fact:B211-DISPOSITION **disposition** | `done` — landed on main 2026-11-03 as `fix(init): today's install hint, --link honoured, --translates parsed and escaped`. |
| @fact:B211-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-212 — PROP-035 lags the compiler it describes {#b-212}

| field | value |
|---|---|
| @fact:B212-WHAT **what** | PROP-035 `#IR-XML-FUTURE` and `#OPEN-XML-FRONTEND` still call the XML front end future, while PROP-045 and the code convert both ways today; `#DIRECTIVE-MANIFEST-AGREE` has no check in `vibe check`; and no real package under `vibevm/vibepacks/` has a `contract/` folder, only the test fixtures do. |
| @fact:B212-EVIDENCE **evidence** | 2026-09-27, the authorities map and the sandbox walk of `DOCS-MD-XML-TUTORIAL`. |
| @fact:B212-SEVERITY **severity** | P3 — a normative text that trails the product. |
| @fact:B212-DISPOSITION **disposition** | `open` — reconcile PROP-035 with PROP-045 by the owner's hand, and add the agreement check. |
| @fact:B212-FILED **filed by** | DOCS-MD-XML-TUTORIAL (M-052), 2026-09-27. |

## B-213 — audit native client configuration isolation in remaining Windows tests {#b-213}

| field | value |
|---|---|
| @fact:B213-WHAT **what** | `UserScratch` relocates Vibe settings/cache, but Windows `dirs` resolves client home/config through KnownFolders rather than HOME/USERPROFILE overrides. A user-scope MCP subprocess fixture is therefore not isolated by those environment variables. The global MCP fixtures found during agent-selection work were replaced with in-process dispatch over injected temporary client directories; remaining client-config test surfaces need an explicit audit. |
| @fact:B213-EVIDENCE **evidence** | 2026-10-02: `dirs-5.0.1/src/win.rs`, `vibe-test-support/src/isolate.rs`, and the retired global cases in `crates/vibe-cli/tests/cli_application.rs`. The replacement `global_mcp_dispatch/tests.rs` proves all destination paths are inside its temporary root and verifies retention, rollback and drift there. No assumption about restoring unknown prior real client state is safe. |
| @fact:B213-SEVERITY **severity** | P1 — an unisolated test can read or modify the developer's client configuration. |
| @fact:B213-DISPOSITION **disposition** | `open` — inventory the remaining tests that reach native user-scope client paths; inject directory authority or otherwise prove isolation before executing them. Extend the user-home tripwire beyond Vibe settings to cover client configs without printing their contents. The current global MCP fixtures are already migrated. |
| @fact:B213-FILED **filed by** | Global MCP agent/server selection review, 2026-10-02. |

## B-214 — legacy nested init interpolates metadata into TOML {#b-214}

| field | value |
|---|---|
| @fact:B214-WHAT **what** | Legacy `vibe init package` scaffolding inserts authors, license and description directly into quoted TOML templates. Valid prose containing quotes, backslashes or newlines can produce an invalid manifest. New root project/package declarations serialize typed manifests safely, but the legacy nested generic/doc template writers still require migration. |
| @fact:B214-EVIDENCE **evidence** | 2026-10-02 source review: `commands/init/package.rs::create_package_dirs_from_fields` manually quotes authors and prose; `commands/init/doc.rs::manifest_text` formats metadata similarly. Root-role integration tests prove quoted authors/license/description safely roundtrip through the new typed root writer. |
| @fact:B214-SEVERITY **severity** | P2 — otherwise valid metadata can leave a broken nested scaffold. |
| @fact:B214-DISPOSITION **disposition** | `done` — closed on main 2026-11-02 by `fix(init): the nested forms write free-text fields as TOML literals`. `c403cc7f2` writes description, licence and authors through the TOML writer, `50ecc85a0` validates the whole candidate before the first write, and this change routes the doc writer's remaining title and translation fields through the same writer and adds the quote/backslash/newline round-trip regressions for a nested `tool` and `doc` package in `crates/vibe-cli/tests/cli_init_kinds.rs`. |
| @fact:B214-FILED **filed by** | Init root declaration/group review, 2026-10-02. |

## B-215 — duplicate test cfg attributes block all-target CLI clippy {#b-215}

| field | value |
|---|---|
| @fact:B215-WHAT **what** | The existing global MCP test modules repeat cfg(test) at both the module declaration and file level. Strict all-target CLI clippy rejects duplicated_attributes although ordinary compilation and tests accept them. |
| @fact:B215-EVIDENCE **evidence** | 2026-10-02: cargo clippy -p vibe-cli --all-targets -- -D warnings reports duplicate attributes in global_mcp_dispatch/tests.rs:1 and tests/progress.rs:1; these files are unchanged by the init role feature. |
| @fact:B215-SEVERITY **severity** | P3 — inherited test-only lint debt. |
| @fact:B215-DISPOSITION **disposition** | `closed` — redundant file attributes removed; parent test-only boundaries retained. Full Linux workspace all-target Clippy and full seven-step gate passed during 1.0.3 distribution validation (2026-10-02). |
| @fact:B215-FILED **filed by** | Init root declaration/group verification, 2026-10-02. |

## B-216 — the old `vibe-user-applications/1` index is refused, not migrated {#b-216}

| field | value |
|---|---|
| @fact:B216-WHAT **what** | The clean application subsystem (PROP-059, 2026-10-30) reads only `vibe-user-applications/3`. A settings root carrying the 1.0.7 index (`/1`, written by the retired installer child-process protocol) makes every global application command fail with "application index … is unreadable: protocol … is not …/3; fix: … remove it to start an empty inventory". The owner's own machine holds such an index with `zap` installed through the retired bridge model. |
| @fact:B216-EVIDENCE **evidence** | `~/.vibe/applications/index.json` on the owner's Linux box, 2026-10-30; `crates/vibe-app/src/index.rs::from_wire` protocol check. The owner ruled on 2026-10-19 that the new model carries no compatibility requirement. |
| @fact:B216-SEVERITY **severity** | P2 — one-time manual step on upgrade for anyone who installed an application with 1.0.7. |
| @fact:B216-DISPOSITION **disposition** | `accepted` — no migration by owner ruling; the error names the remedy. The 1.0.8 changelog carries the upgrade note. |
| @fact:B216-FILED **filed by** | APP-CLEAN campaign, 2026-10-31. |

## B-217 — `org.vibevm.zap/zap` lost its application declaration {#b-217}

| field | value |
|---|---|
| @fact:B217-WHAT **what** | The in-tree registry copy of `org.vibevm.zap/zap` declared `[application_source]`, `installer_package`, `commands` and `[application.distribution]`, all retired with the installer-options model. The copy now keeps only `[application] id = "zap"` and no `[[installer_options]]`, so `vibe install -g org.vibevm.zap/zap` refuses it as not globally installable until the package (in its own repository) declares installer options — most naturally a `source_build` row per platform whose recipe runs its existing Node installer. |
| @fact:B217-EVIDENCE **evidence** | `vibevm/vibepacks/org.vibevm.zap/zap/v1.0.0/vibe.toml` after commit `af6dfb36a`; the owner's installed `zap` is in the `/1` index of B-216. |
| @fact:B217-SEVERITY **severity** | P2 — the owner's own product is not installable through the new model until its package is re-declared. |
| @fact:B217-DISPOSITION **disposition** | `open` — owner decision in the zap repository; the documentation-site package (`org.vibevm.doc/web`) was converted in this campaign and is the worked example. |
| @fact:B217-FILED **filed by** | APP-CLEAN campaign, 2026-10-31. |

## B-218 — a forked child briefly holds the application mutation lock {#b-218}

| field | value |
|---|---|
| @fact:B218-WHAT **what** | `SettingsRoot::lock` takes a `try_lock` on `applications/.mutation.lock` and refuses when held. In one test process, a child spawned by another thread inherits the lock file descriptor between `fork` and `exec` (the descriptor is close-on-exec, the lock is per open-file-description), so a back-to-back `lock()` in a different test can see a stale holder for a few milliseconds. The `vibe-app` storage unit tests retry only that refusal; product code never retries. |
| @fact:B218-EVIDENCE **evidence** | P4 executor report, 2026-10-30: intermittent `Locked` in `storage` unit tests, 25/25 green with the test-only retry; `crates/vibe-app/src/storage/relocate.rs::tests::retry_locked`. |
| @fact:B218-SEVERITY **severity** | P3 — test-only; a real second `vibe` process holding the lock is refused correctly. |
| @fact:B218-DISPOSITION **disposition** | `done` — closed on main 2026-11-02 by `fix(app): the mutation lock waits out a child's fork window`. `ae9a782d4` gives `SettingsRoot::lock` a 50 ms retry window (the owner's B-228 choice, recorded in PROP-059 `#index`), and this change removes the test-only `retry_locked` helper and the "one mutation per test" split it forced in the `vibe-app` unit tests. |
| @fact:B218-FILED **filed by** | APP-CLEAN campaign, 2026-10-31. |

## B-219 — native-fixture tests fail after a `cargo build --workspace` {#b-219}

| field | value |
|---|---|
| @fact:B219-WHAT **what** | The native-loader fixture tests in `vibe-native-loader`, `vibe-install` and `vibe-lifecycle` locate their `cdylib` fixture by asserting exactly one `libvibe_native_loader_*fixture.so` across `target/debug/` and `target/debug/deps/`. `cargo test` builds the fixtures into `deps/` only, but a preceding `cargo build --workspace` uplifts them into `target/debug/` as well, so the next `cargo test --workspace` fails 11 tests with "expected exactly one … found [deps/…, …]". The tests are sensitive to build history, not to code. |
| @fact:B219-EVIDENCE **evidence** | 2026-10-31 `tools/self-check.sh` on `a435a9abf`: step 2 failed with 11 such tests after the P6 executor's `cargo build --workspace`; removing the uplifted copies and rerunning the three packages was green. The archived prototype met the same class (its `fix(test)` commits of 2026-10-20 about fixture linkage). |
| @fact:B219-SEVERITY **severity** | P2 — the release gate goes red from an ordinary developer action; the fixtures themselves are correct. |
| @fact:B219-DISPOSITION **disposition** | `open` — make the lookup deterministic (prefer `deps/`, or the newest artifact) in the shared fixture helper instead of asserting uniqueness; until then, delete `target/debug/libvibe_native_loader_*fixture.so` before `cargo test --workspace`. |
| @fact:B219-FILED **filed by** | APP-CLEAN campaign close-out, 2026-10-31. |

## B-220 — hold a global application at its version {#b-220}

| field | value |
|---|---|
| @fact:B220-WHAT **what** | `vibe app update` and `vibe app outdated` (PROP-065 `#outdated`) follow the registry; there is no way to say "keep Vim at 9.2.1167 until I say otherwise" the way `apt-mark hold` or `brew pin` do. A held installation would be reported by `outdated` as held, skipped by a bulk update, and updated only by naming it explicitly. |
| @fact:B220-EVIDENCE **evidence** | Owner question of 2026-10-31 ("мы что-то еще забыли обсудить?"), answered during the APP-STEPS campaign; not in PROP-059 or PROP-065. |
| @fact:B220-SEVERITY **severity** | P3 — a convenience over the existing per-version installation keys. |
| @fact:B220-DISPOSITION **disposition** | `done` — the owner's standing grant of 2026-10-31 (add what is foundational); specified in PROP-065 `#update-and-rollback`, landed in APP-STEPS Q3 (`50a6685cc`: `vibe app hold | unhold`, `outdated` reports held, `vibe update -g --all` skips held). |
| @fact:B220-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-221 — keep the previous version hidden for rollback {#b-221}

| field | value |
|---|---|
| @fact:B221-WHAT **what** | An application update installs the new version and removes the old one. With hidden installations (PROP-059 `#hidden-installations`) the old version could instead stay hidden for one update cycle, so `vibe app rollback PKG` is just `expose` of the previous key plus `app use`, with no download and no rebuild. The next successful update would remove it. |
| @fact:B221-EVIDENCE **evidence** | Owner question of 2026-10-31; the index already keys installations by exact version, and alternatives already let two versions coexist. |
| @fact:B221-SEVERITY **severity** | P3 — source builds taking minutes make rollback-by-reinstall expensive, but nothing is wrong today. |
| @fact:B221-DISPOSITION **disposition** | `done` — the owner's standing grant of 2026-10-31; the previous version is kept hidden by default, PROP-065 `#update-and-rollback`, landed in APP-STEPS Q3 (`50a6685cc`: `vibe app rollback`, `--purge-previous`, autoremove of kept versions). |
| @fact:B221-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-222 — export and import the set of global applications {#b-222}

| field | value |
|---|---|
| @fact:B222-WHAT **what** | A project records its applications in `vibe.toml` / `vibe.lock`, but the global home has no portable description of "what I have installed, with which installer option, features and parameters". A Brewfile-like `vibe app export > apps.toml` and `vibe app import apps.toml` would move a user's toolset to another machine through the ordinary registry resolution, choosing the matching platform rows. |
| @fact:B222-EVIDENCE **evidence** | Owner question of 2026-10-31; the index record already holds every field such an export needs. |
| @fact:B222-SEVERITY **severity** | P3 — convenience; the project scope covers the reproducible case today. |
| @fact:B222-DISPOSITION **disposition** | `done` — landed on main 2026-11-01 as `feat(app): vibe app export and import carry the user's applications and launcher arrangement to another machine`, accepted by the owner the same day: `vibe app export` writes the `vibe-apps/1` set (installations with their installer, features, parameters, licence, mark and hold, then collections, pins and launcher overrides unless `--installations-only`); `vibe app import` installs it through the ordinary resolution and `install -g` row by row with `--dry-run`, `--registry`, `--accept-license` and `--skip-installed`, refuses per row, and answers `vibe-app-import/1`; PROP-059 `#portable-set`. |
| @fact:B222-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-223 — payload paths print with mixed separators on Windows {#b-223}

| field | value |
|---|---|
| @fact:B223-WHAT **what** | Removal plans and uninstall summaries on Windows print the owned payload as `C:\…\opt\apps/vim/org.vibevm.apps.vim@1.1.0` — the library root with backslashes, the per-application part joined with forward slashes. The path works; the display is inconsistent with every other path vibe prints. |
| @fact:B223-EVIDENCE **evidence** | MT-04 automated run of 2026-10-31 on `base` (`uninstall --plan-only`, `app autoremove --plan-only`, the uninstall summary line). |
| @fact:B223-SEVERITY **severity** | P3 — cosmetic. |
| @fact:B223-DISPOSITION **disposition** | `done` — landed in APP-STEPS Q4 (`f28a385a2`): the library-relative directory is joined through `Path` one component at a time (`vibe-app` `launch::library_path`), so every platform prints native separators. |
| @fact:B223-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-224 — launcher metadata for a desktop start menu {#b-224}

| field | value |
|---|---|
| @fact:B224-WHAT **what** | A launcher (PROP-059 `#launch-points`) carries `name`, `description`, `executable`, `args`, `working_directory`, one `icon` path, `terminal` and `default`; `[application] icon` names one image. A desktop client building a start menu and a taskbar from `vibe app list --json` and `vibe app launchers --json` needs three explicit author decisions the manifest cannot express today. (1) `kind = "gui" \| "tui" \| "command"` on the launcher: a TUI is a different thing from a plain console command and the client opens it through a terminal it starts; the kind is declared by the package author, never inferred from `terminal`. (2) `menu = true` on the launcher: an explicit opt-in to the start menu, off by default for every launcher of every kind (GUI included — a package that ships a hundred executables must not flood the menu); a suite of ten TUI utilities and one central one lists only the central one, the rest stay reachable but hidden. (3) `[application] menu_group = "…"`: the human-readable group name the menu shows (the way "Adobe Master Collection" is a group), independent of the package name and not the description. Supporting fields: `categories` and `keywords` for grouping and search; an icon set by format and size (`svg`, `png` sizes, `ico`); optionally `mime_types`; and `vibe app launchers --all --json` across every installed application so the menu is one query. Generation of system `.desktop` / `.lnk` files stays a later, separate option. |
| @fact:B224-EVIDENCE **evidence** | Owner, 2026-10-31: a start menu and taskbar in the desktop client; one application installs both console utilities (no shortcut) and graphical ones (shortcut); TUI launchers optionally get a shortcut opened through a terminal; the TUI mark, the menu opt-in and the group name are explicit author decisions ("отметка о том, что является TUI — наше явное решение как автора пакета", "должно быть наше явное решение включить лаунчер в меню Пуск", "явно указать имя группы ярлыков"). The existing `terminal` flag and `public_commands` already separate console utilities from launchers, so the gap is metadata, not mechanism. |
| @fact:B224-SEVERITY **severity** | P2 — blocks the owner's desktop client from presenting installed applications well; no correctness defect. |
| @fact:B224-DISPOSITION **disposition** | `accepted` — the owner specified the three decisions on 2026-10-31 and then ordered it into 1.0.8 ("сними тэг 1.0.8 и доделай это"); the icon-set shape (SVG source, PNG by pixel size, `ico` / `icns` containers, `symbolic`, `maskable`, client-side size choice, launcher → application → client fallback, `vibe check` rules) is specified in PROP-059 `#menu`; APP-STEPS phase Q5: PROP-059 `#launch-points` amendment, manifest fields with validation (`kind`, `menu`, `menu_group`, `categories`, `keywords`, icon set), `application_launchers` / `application_list` / `application_describe` schemas and codegen, `--all`, tests, Vim and documentation-site manifests updated (console Vim: `kind = "tui"`, gVim: `kind = "gui"`, `menu = true`; group "Vim"). |
| @fact:B224-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-225 — lifecycle verbs have no `-g` and misname the home they searched {#b-225}

| field | value |
|---|---|
| @fact:B225-WHAT **what** | Every `vibe app` verb selects the project home when the working directory is a project and the user home otherwise, and `-g` / `--global` (declared once on `vibe app`, so every subcommand has it) chooses the user home; but inside a project a verb aimed at a globally installed application failed with `not_installed: global application … is not installed`, which named the wrong home and gave no hint. |
| @fact:B225-EVIDENCE **evidence** | MT-05 Q4 scenario on `flash`, 2026-10-31: `vibe app hold org.vibevm.apps/vim` from `~/git/vibevm-app` refused with that message while `app list -g` showed the installation; the same command from `~` succeeded. `crates/vibe-cli/src/commands/app/lifecycle.rs` resolves the home through the project-or-user rule; the error text is fixed to "global". |
| @fact:B225-SEVERITY **severity** | P2 — a desktop client or a script running inside a project cannot reach the user's applications with these verbs. |
| @fact:B225-DISPOSITION **disposition** | `done` (the flag already existed; the message was the defect) — landed in APP-STEPS Q5b: every application verb that selects a home takes `-g` / `--global` with the `app list` semantics (the flag is global on `vibe app`), and `not_installed` names the home it searched (`the user home` / `the project home <path>`) and, for a project home, hints `use -g for the user home`; `cli_app_modify.rs` holds the regression test. |
| @fact:B225-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-226 — edit `HKCU\Environment` through the registry API, not PowerShell {#b-226}

| field | value |
|---|---|
| @fact:B226-WHAT **what** | The Windows user-environment persister (`vibe-cli` `commands/vvm/env.rs`, shared by `vvm env` and the application `env` step) reads and writes `HKCU\Environment` by spawning Windows PowerShell and parsing its output, then relies on it for the `WM_SETTINGCHANGE` broadcast. A native implementation (`RegOpenKeyExW` / `RegQueryValueExW` / `RegSetValueExW` / `RegDeleteValueW` keeping `REG_SZ` versus `REG_EXPAND_SZ`, `SendMessageTimeoutW` for the broadcast) needs no child process, no execution policy and no quoting, and runs in microseconds instead of hundreds of milliseconds per value. |
| @fact:B226-EVIDENCE **evidence** | MT-04 env-step run on `base`, 2026-10-31: under a reduced PATH the step failed with `spawning powershell to edit the user environment: program not found`; fixed for 1.0.8 by resolving PowerShell and `cmd.exe` through `%SystemRoot%` (`vibe-app` `installer/programs.rs`), which removes the PATH dependency but keeps the subprocess. |
| @fact:B226-SEVERITY **severity** | P3 — correctness is restored by the path fix; this is robustness and speed. |
| @fact:B226-DISPOSITION **disposition** | `done` — landed on `main` 2026-11-02 as `feat(vvm): the Windows user environment is edited through the registry API`, accepted by the owner 2026-11-01: `vibe-cli` reads, writes and deletes `HKCU\Environment` values with `RegQueryValueExW` / `RegSetValueExW` / `RegDeleteValueW`, keeping `REG_SZ` versus `REG_EXPAND_SZ`, and broadcasts with `SendMessageTimeoutW` (`windows-sys` 0.61 with the `Win32_Foundation`, `Win32_System_Registry` and `Win32_UI_WindowsAndMessaging` features, Windows only); the pure PATH transformation and its tests are unchanged, and `VIBE_TEST_HKCU=1` runs a scratch-value round trip on a Windows host. |
| @fact:B226-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-227 — refresh the registry cache for the user home {#b-227}

| field | value |
|---|---|
| @fact:B227-WHAT **what** | `vibe registry sync` refreshes the clones a project's `vibe.lock` references and refuses outside a project ("no `vibe.toml`"). A user who installs applications globally has no way to refresh the registry cache under the user settings root before the implicit-pull TTL (one hour) expires, and a desktop client has no "refresh catalog" verb. `vibe registry sync -g` (or `sync` outside a project) should refresh the user home's cached clones and the index handshake. |
| @fact:B227-EVIDENCE **evidence** | MT-04 / MT-05 runs of 2026-10-31 right after publishing `vim@1.1.2`: a fresh home resolved `1.1.1` from a stale index copy, and `vibe registry sync` could not be used from the home directory. |
| @fact:B227-SEVERITY **severity** | P3 — the TTL bounds the staleness; a client can wait or clear `<home>/registries/`. |
| @fact:B227-DISPOSITION **disposition** | `done` — landed on `main` 2026-11-01 as `feat(registry): refresh the user home, publish into the index checkout, trust repomd` (the commit after `0999cc819`), accepted by the owner the same day: `vibe registry sync -g` (and the plain form outside a project) refreshes the user home's cached clones and index handshakes; `vibe registry publish --index-checkout` adds the release to a local index checkout and commits it (`--push-index` pushes); the index client checks a `by-name` file against `repomd.json` and falls back to tag enumeration on a mismatch. |
| @fact:B227-FILED **filed by** | APP-STEPS campaign, 2026-10-31. |

## B-228 — the application mutation lock leaks into a forked child for an instant {#b-228}

| field | value |
|---|---|
| @fact:B228-WHAT **what** | `SettingsRoot::lock()` (`vibe-app` `settings.rs`) takes `applications/.mutation.lock` with `File::try_lock`, an open-file-description lock. A child spawned while the lock is held shares that description between `fork` and `exec` (the descriptor is close-on-exec, so the window closes at `exec`), and a second lock attempt in that window is refused as `locked` although the holder has already released it. One vibe process takes the lock once, so the CLI never observes it; a test process that locks the same home twice around a child spawn does. |
| @fact:B228-EVIDENCE **evidence** | `steps/work.rs` unit tests, 2026-10-31: two `clean_work` calls on one home intermittently returned `locked` while a sibling test's child was being spawned; the test now uses one home per lock acquisition and says why. |
| @fact:B228-SEVERITY **severity** | P3 — no user-visible effect today; a hazard for any future in-process second acquisition (a long-lived desktop client, a daemon). |
| @fact:B228-DISPOSITION **disposition** | `done` — landed on `main` 2026-11-02 as `fix(app): the mutation lock waits out a child's fork window`. The owner chose the retry (2026-11-01): `SettingsRoot::lock()` retries `try_lock` every 2 ms for at most 50 ms before refusing `locked`; how children are spawned is unchanged. A `settings.rs` test keeps a child holding the lock's file, shows that a zero window refuses and that the retry succeeds, and the `clean_work` test locks one home twice again. |
| @fact:B228-FILED **filed by** | kept-work-directories change (`c22f72bb8`), 2026-10-31. |

## B-229 — a translated doc scaffold writes the source's name unescaped into its README and XML example {#b-229}

| field | value |
|---|---|
| @fact:B229-WHAT **what** | `vibe init package … --kind doc --translates <group>/<name>@<version>` only splits `--translates` on `/` and never parses it as a coordinate, so the source's name reaches the scaffold's title. Since `8ca42c901` the TOML values go through the TOML writer, but the same title is still pasted as written into the generated README and into the XML example page, where a quote or an angle bracket breaks the page. |
| @fact:B229-EVIDENCE **evidence** | Review of `crates/vibe-cli/src/commands/init/doc.rs` during the B-214 close-out, 2026-11-02. Reaching the writer needs a readable source directory under that name, so ordinary use is unaffected. |
| @fact:B229-SEVERITY **severity** | P3 — needs a source package whose directory name carries the character; a broken example page, nothing else. |
| @fact:B229-DISPOSITION **disposition** | `done` — landed on main 2026-11-03 as `fix(init): today's install hint, --link honoured, --translates parsed and escaped`. |
| @fact:B229-FILED **filed by** | installer-package wave close-out (H-1), 2026-11-02. |


## B-230 — optional preserved collection readers accept undeclared null {#b-230}

| field | value |
|---|---|
| @fact:B230-WHAT **what** | A nonnullable JTD optional collection with x-empty=preserve generates Option<Vec/Map> using ordinary serde, so explicit JSON null is admitted as absence even though the schema declares only absent or collection. This keeps None versus empty but has a reader/schema admission mismatch. |
| @fact:B230-EVIDENCE **evidence** | Common-configuration candidate exposed the distinction. Existing application_describe.forceArguments is optional nonnullable in schemas/application_describe.jtd.json and Option<Vec<String>> with ordinary serde in generated/application_describe/mod.rs; preserve tests in xtask/src/codegen/empty_policy/tests.rs assert that generated shape. requirements_report and doc_manifest also use this policy. Root inspected these source shapes; no broad old-client runtime/compatibility matrix has been run. |
| @fact:B230-SEVERITY **severity** | P3 — invalid-by-schema null aliases absence in existing readers; valid declared values remain distinguishable. Compatibility needs review before changing old admission. |
| @fact:B230-DISPOSITION **disposition** | open; deliberately not tightened globally during common app integration. New presence-sensitive common arrays use supported required-nullable collection refs, preserving null versus [] and rejecting missing required fields in their new contract. Review old schemas/consumers before either declaring nullability or narrowing readers. Inline required-nullable collections additionally hit an existing empty-policy generation limitation; the equivalent shared collection-ref route is supported, so no general emitter expansion is needed for the current consumer. |
| @fact:B230-FILED **filed by** | Semantic transfer campaign,2026-11-04. |


## B-231 — `vibe self import --use` writes the PATH line into the shell profile without asking {#b-231}

| field | value |
|---|---|
| @fact:B231-WHAT **what** | `vibe self import <exe> --use` writes the shims and vibe's managed PATH block into the shell's profile (zsh `~/.zshrc`, bash `~/.bashrc`, fish `config.fish`, else `~/.profile`, chosen by `$SHELL`) on every unix, with no consent asked and no flag to leave the profile alone. VibeVM's first start on macOS runs that import, so the folder is on PATH before the person is asked: the consent of Settings → vibe's *Add to PATH* (`vibe self doctor --fix --yes`, vibevm-app R130, I-2044) then only covers a line that was later removed. Also: `vibe self doctor --json` names no profile and reads only the calling process's PATH, so a client started from the Finder must repeat vibe's `$SHELL` rule and read the managed block itself. |
| @fact:B231-EVIDENCE **evidence** | vibevm-app's R130 agent, 2026-11-04: in a temp home `vibe 1.0.11 self import … --use` wrote `.zshrc` at once; read in the source at /fast/git/vibevm. Recorded fixtures in vibevm-app `crates/core/tests/fixtures/vibe-path/`. |
| @fact:B231-SEVERITY **severity** | P2 — consent to change a shell profile is implied rather than asked on the first import; nothing breaks. |
| @fact:B231-DISPOSITION **disposition** | `candidate — to be considered and decided later in vibe` (the owner, 2026-11-04). Options to weigh: a `--no-profile` (or consent) switch for `self import`, the profile and its block named in the doctor's JSON. |
| @fact:B231-FILED **filed by** | vibevm-app build side (session MVP), 2026-11-04. |


## B-232 — a project install names no source before it; no per-requirement pin; no preferred hosting {#b-232}

| field | value |
|---|---|
| @fact:B232-WHAT **what** | Three needs of VibeVM's Library, to be built in vibe: (1) a plan of a project's install without installing (`install --plan-only` or `--dry-run` for project packages) that names the registry and the source each package will come from — today `--plan-only` works only with `--global` (applications), there is no `--dry-run`, and `install:plan` carries `packages[{package, version}]` with no source; the source is known only after the install, from `vibe.lock` (`registry`, `source_url`, `source_ref`, `resolved_commit`, `source_kind`); (2) pinning a registry to one requirement (today only the project's `[[registry]]` order exists); (3) a resolution order by a preferred hosting (a package published on GitHub and mirrored on GitVerse is taken from the hosting the person prefers — in VibeVM by the home: ru → GitVerse first, world → GitHub). |
| @fact:B232-EVIDENCE **evidence** | vibevm-app's R137 check on vibe 1.0.11, 2026-11-04, in a temp home: `install org.vibevm.world/wal --plan-only --json` in a project → "Usage: vibe install --global --plan-only …"; `install --json` → `install:plan`, `install:closure-diff`, `install` with no source; the lock row names `registry = "vibespecs"`. vibevm-app's I-2075: the application does not choose or know an install's source until vibe can say it. |
| @fact:B232-SEVERITY **severity** | P1 — **high priority by the owner's word (2026-11-04)**: "will be installed from <source>", "Take it from here" and the preferred hosting in the Library wait for these three. |
| @fact:B232-DISPOSITION **disposition** | `open — high priority (the owner, 2026-11-04)`; when vibe has them, vibevm-app adds the three (R137 §4, I-2075). |
| @fact:B232-FILED **filed by** | vibevm-app build side (session MVP), 2026-11-04. |

## B-233 — окружения на основе существующего мета-дистрибутива {#b-233}

| field | value |
|---|---|
| @fact:B233-WHAT **what** | Архитектурно проверить гипотезу владельца: подготовка Docker и других окружений должна опираться на существующий мета-дистрибутив `vibe install -g`, расширенный на другие корни установки и точки монтирования, вместо второй независимой системы установки. HOME, target rootfs и исполнитель не следует отождествлять. Пользовательская цель: сборка понятных контейнеров из готовых пакетов через общие GUI/CLI/агентные инструменты; долгосрочно — ОС с Vibe как пакетным менеджером, одинаково доступная людям и агентам. Дословные слова владельца сохранены в заметке и комментарии PROP-059. |
| @fact:B233-EVIDENCE **evidence** | Предложение владельца2026-11-04; существующий мета-дистрибутив описан в PROP-059#purpose, структурированные шаги — в PROP-065. Уже есть пользовательские/проектные homes и библиотеки хранения; их достаточность для Docker/rootfs не установлена. Подробные вопросы и ожидаемый результат: [архитектурная заметка](campaigns/next-semantic-transfer-2026-11/research/environment-targets-metadistribution-question.md). |
| @fact:B233-SEVERITY **severity** | P2 — серьёзный открытый архитектурный вопрос перед проектированием окружений; не аварийный дефект. |
| @fact:B233-DISPOSITION **disposition** | `open — architecture investigation`; гипотеза не объявлена утверждённым решением. Сопоставить с M-14/M-15 полного плана, явно отделив reuse, необходимые расширения и backend-specific поведение. Не новая задача в знаменателе222 и не основание считать Docker уже готовым. |
| @fact:B233-FILED **filed by** | По предложению владельца,2026-11-04. |


## B-254 — an installed application records no source {#b-254}

| field | value |
|---|---|
| @fact:B254-WHAT **what** | `vibe install -g` of an application from a registry writes no global `vibe.lock`, and `applications/index.json` records no source of the installation, so a client cannot say where an installed application came from ("Installed from <registry/url@ref>"). A project's `vibe.lock` does record `registry`, `source_url`, `source_ref`, `resolved_commit` for a project package. |
| @fact:B254-EVIDENCE **evidence** | vibevm-app's R137b probe of vibe 1.0.11, 2026-11-04, in a temp home: `install -g` from a local registry; no global lock written, the index without a source. vibevm-app's `ApplicationFacts.installed.source` waits for it (I-1246). |
| @fact:B254-SEVERITY **severity** | P2 — the Library's "Installed from" line on an application's page is hidden until vibe records it; related to B-232 (the install's source, high priority). |
| @fact:B254-DISPOSITION **disposition** | `candidate — to be considered and decided later in vibe` (the owner's rule of 2026-11-04 for vibe's needs found while building the app). |
| @fact:B254-FILED **filed by** | vibevm-app build side (session MVP), 2026-11-04. |


## 2026-11-07 · the owner's ruling relayed from vibevm-app's audit (notes/66 there)

- **P1 · Remove `org.vibevm.world.zap` and `org.vibevm.zap.zap` from the public registry.** The repositories under `github.com/vibespecs` go, then their versions are **yanked from the registry's index** (`anarchic/vibespecs` and its GitHub mirror) and the index is rebuilt, so `vibe` stops offering packages that no longer download. The owner's ruling of 2026-11-07: everything of vibevm-app and Zap is closed software; `vibe` itself stays open. The `vibe-index` yank-then-reindex procedure is this repository's; the repository removals are the owner's hand.


## B-243 — `vibe tool px exec` does not exclude a re-entry when resolving its command {#b-243}

| field | value |
|---|---|
| @fact:B243-WHAT **what** | `vibe tool px exec -- <name>` takes the first `PATH` match for `<name>` with no exclusion (`crates/vibe-cli/src/commands/tool/which.rs`). An application package whose public command is `<name>` and whose launcher runs `vibe tool px exec -- <name>` therefore resolves its own `vibe-app-proxy` copy and recurses without end, because PROP-059 `#path-directories` puts vibe's link directories and `opt/bin` ahead of the system directories. The neighbouring `command:NAME` launcher resolution (PROP-065 `#launcher-command`) deliberately never chooses the running proxy's own image; `exec` has no equivalent guard. |
| @fact:B243-EVIDENCE **evidence** | Demonstrated 2026-11-09 while building `org.vibevm.pxworld/pxnode`: a shim `node` earlier on `PATH` than the real one was chosen (`running .../shadow/node through the vibe-managed px on 127.0.0.1:38113`). The pxworld packages avoid it by naming their commands `pxnode` / `pxnpm` / `pxnpx` / `pxpip`, never the command they wrap, and both READMEs say why. |
| @fact:B243-SEVERITY **severity** | P2 — no shipped package hits it, and the wrapper convention documents the rule; a third-party wrapper that ignores the convention gets an unbounded loop instead of a refusal. |
| @fact:B243-DISPOSITION **disposition** | `candidate` — give `exec` the same self-exclusion the launcher resolution has (skip a candidate that is a `vibe-app-proxy` copy in a vibe-owned shim or link directory), or detect the loop and refuse by name. PROP-068 `#proxy-tool` would gain the rule. |
| @fact:B243-FILED **filed by** | Corporate-env campaign, CORP-FMT-038 verification, 2026-11-09. |


## B-244 — an index fetch through an unreachable px has no timeout {#b-244}

| field | value |
|---|---|
| @fact:B244-WHAT **what** | With `[proxy] mode = "px"` configured and the upstream unreachable, `vibe install -g` and `vibe app installers` without `--offline` hang on the built-in registries' index fetch with no progress and no timeout — the operation never reports that the route is dead. px itself starts and answers; the `CONNECT` to the upstream is what stalls. |
| @fact:B244-EVIDENCE **evidence** | Observed twice independently, 2026-11-09, while verifying `org.vibevm.pxworld/pxnode` and `/pxpip` against the repository's fake px (whose `--proxy` names a host that does not exist): past 120 s with no output; `--offline` returns at once. |
| @fact:B244-SEVERITY **severity** | P3 — a misconfigured proxy is the cause, and `vibe dl --test-proxy` diagnoses it; but the symptom a user meets first is a silent hang rather than a named failure. |
| @fact:B244-DISPOSITION **disposition** | `candidate` — give the index/metadata reads the same bounded timeout the probe uses, and classify a stalled `CONNECT` as the proxy failure it is (PROP-068 `#proxy-doctor` already has the vocabulary). |
| @fact:B244-FILED **filed by** | Corporate-env campaign, CORP-FMT-038 verification, 2026-11-09. |


## B-245 — `vibe self px status` reports the companion store under `$HOME`, not the settings root {#b-245}

| field | value |
|---|---|
| @fact:B245-WHAT **what** | With `$VIBE_SETTINGS` pointing at an isolated settings root, `vibe self px status` still prints the px companion store under `$HOME/.vibe/opt/…`. The lookup itself is unaffected when `[proxy] px_path` names an executable, so the report and the behaviour can disagree. |
| @fact:B245-EVIDENCE **evidence** | Observed 2026-11-09 while verifying `org.vibevm.pxworld/pxpip` in an isolated home (`VIBE_SETTINGS` + `HOME` both redirected). |
| @fact:B245-SEVERITY **severity** | P3 — cosmetic in the reported path; it would mislead anyone diagnosing a store that is not where the report says. |
| @fact:B245-DISPOSITION **disposition** | `candidate` — resolve the store root through the same settings root every other report uses (PROP-019 `#companion-tools`). |
| @fact:B245-FILED **filed by** | Corporate-env campaign, CORP-FMT-038 verification, 2026-11-09. |

## B-246 — `cargo xtask dist px` refuses the Windows px archive's own licence {#b-246}

| field | value |
|---|---|
| @fact:B246-WHAT **what** | The px windows-amd64 zip ships `LICENSE.txt` with CRLF line ends (1101 bytes); `xtask/dist/px.toml` pins `license_sha256` of the GitHub-raw LF form (1080 bytes). `stage_with_archive` prefers the in-archive licence, so `cargo xtask dist px --target x86_64-pc-windows-msvc` always fails with `px LICENSE does not match the pinned SHA-256` unless `--license` names the LF copy. |
| @fact:B246-EVIDENCE **evidence** | Corporate test build on base, 2026-11-09 (deterministic). |
| @fact:B246-SEVERITY **severity** | P2 — blocks the documented release step for the Windows px companion. |
| @fact:B246-DISPOSITION **disposition** | `fixed` 2026-11-10 — `stage_with_archive` (`xtask/src/dist/px_tool.rs`) folds CRLF to LF (`lf_line_ends`) before measuring the licence against `license_sha256` **and** writes that LF form into `tools/px/LICENSE.txt`, so the staged licence hashes to the pin whichever copy the build read; `xtask/dist/px.toml` says the pin is of the LF form. Nothing is skipped: a lone CR is content, and a one-byte content change is still refused by digest, now with both digests named. Tests: `the_crlf_spelling_of_the_pinned_licence_is_staged_as_the_pinned_lf_text`, `folding_line_ends_touches_nothing_but_a_crlf_pair` |
| @fact:B246-FILED **filed by** | Corporate-env campaign, Windows test-build walk, 2026-11-09. |

## B-247 — a single-platform release drop cannot carry `DISTRIBUTIONS.json` {#b-247}

| field | value |
|---|---|
| @fact:B247-WHAT **what** | With `<drop>/v1.0.12/DISTRIBUTIONS.json` holding only the Windows target, `vibe self update` and `vibe self reinstall` refuse: `aggregate must contain each supported distribution target exactly once`. Without the file they succeed (`verified against its own embedded manifest only`). `docs/admin/closed-network.md` §3.2 shows the aggregate in the drop. |
| @fact:B247-EVIDENCE **evidence** | Corporate walk on base, 2026-11-09. |
| @fact:B247-SEVERITY **severity** | P2 — an air-gapped Windows-only site cannot follow the guide as written. |
| @fact:B247-DISPOSITION **disposition** | `fixed` 2026-11-10 — the complete five-target matrix is now the *publishing* rule only (`AggregateDistributionManifest::validate`, still enforced by `from_platforms`, `to_json_bytes`, `from_json_slice` and so by `xtask dist` and `dist gitverse-publish`); a consumer uses `validate_for_target` / `from_json_slice_for_target` and needs the running host's target exactly once (`crates/vibe-wire/src/behaviour/release_manifest.rs`, `crates/vibe-cli/src/commands/vvm/bundle/selection.rs`). Safe because nothing authenticates the aggregate as a whole — no signature, no outer digest; it is the root of trust and everything it covers it covers per entry (asset name/size/SHA-256, cross-checked against the bundle's embedded manifest), so an entry gains nothing from its neighbours. Still refused: a duplicate target (`TargetDuplicate`), an unsupported target (`UnsupportedTarget`), an empty aggregate, and an aggregate without the host's target (`TargetAbsent`, which names it). Spec fact `PROP-019#RELEASE-SOURCE-PARTIAL-AGGREGATE` and `docs/admin/closed-network.md` §3.2 say so. Tests: `a_consumer_accepts_an_aggregate_holding_its_own_target_alone` (vibe-publish), `a_drop_whose_aggregate_holds_only_this_host_installs_and_is_still_verified` + `a_drop_whose_aggregate_omits_this_host_is_refused_by_name` (vibe-cli) |
| @fact:B247-FILED **filed by** | Corporate-env campaign, Windows test-build walk, 2026-11-09. |

## B-248 — no local verb produces `DISTRIBUTIONS.json` and `TOOLS.json` {#b-248}

| field | value |
|---|---|
| @fact:B248-WHAT **what** | `xtask dist finalize` builds both only from fragments downloaded from a GitHub draft release; a local or test build has no pipeline path to them (the corporate test build synthesised them from its own fragments). |
| @fact:B248-EVIDENCE **evidence** | Corporate test build on base, 2026-11-09. |
| @fact:B248-SEVERITY **severity** | P3. |
| @fact:B248-DISPOSITION **disposition** | `candidate` — `cargo xtask dist aggregate --out-dir DIR` from fragments on disk. |
| @fact:B248-FILED **filed by** | Corporate-env campaign, Windows test-build walk, 2026-11-09. |

## B-249 — reports ignore the env-registry test seam {#b-249}

| field | value |
|---|---|
| @fact:B249-WHAT **what** | With `VIBEVM_ENV_REGISTRY_SUBKEY` set, `self apply --show` and `self doctor` still print `HKCU\Environment\Path` while writes go to the override key. |
| @fact:B249-EVIDENCE **evidence** | Corporate walk on base, 2026-11-09. |
| @fact:B249-SEVERITY **severity** | P3 — misleading only in sandboxed runs. |
| @fact:B249-DISPOSITION **disposition** | `candidate` — print the effective key. |
| @fact:B249-FILED **filed by** | Corporate-env campaign, Windows test-build walk, 2026-11-09. |

## B-250 — every first search of a fresh monorepo clone warns that its index is stale {#b-250}

| field | value |
|---|---|
| @fact:B250-WHAT **what** | `vibe search` against a freshly cloned monorepo registry prints `WARN … the index of layout registry … is stale (the journal changed since the index was written); computing it in memory`, although the mirror wrote a current index. |
| @fact:B250-EVIDENCE **evidence** | Corporate walk on base and the mirror check here, 2026-11-09. |
| @fact:B250-SEVERITY **severity** | P3 — noise a corporate user will report on day one. |
| @fact:B250-DISPOSITION **disposition** | `candidate` — compare against the journal state the clone carries, or demote to a note. |
| @fact:B250-FILED **filed by** | Corporate-env campaign, Windows test-build walk, 2026-11-09. |

## B-251 — `self apply` in px mode finishes quietly when px was not installed {#b-251}

| field | value |
|---|---|
| @fact:B251-WHAT **what** | The release zip carries no px (px is a separate companion asset, PROP-019 `#companion-tools`). When a user unpacks only the vibe zip and runs `vibe self apply` with a profile that sets `[proxy] mode = "px"`, the only trace is one grey line `○ Installing px · skipped: no px-…zip beside the binary`; the summary still reads `proxy mode: px` and the command exits 0. The first network command then fails. `vibe self px status` for a missing px also ends with `✓ px`. |
| @fact:B251-EVIDENCE **evidence** | The owner missed the step in the corporate delivery, 2026-11-09; reproduced on base (unpack the zip only → `self apply` → `self px status` says not installed → `vibe self px install --from <dir>` fixes it). |
| @fact:B251-SEVERITY **severity** | P2 — the corporate first-run path breaks silently. |
| @fact:B251-DISPOSITION **disposition** | `done` — closed on main 2026-11-09 by `fix(self): apply ensures px when the profile selects it`. `vibe self apply` runs a px step after the self-import and whether or not it ran (`crates/vibe-cli/src/commands/vvm/site/steps.rs` `px_step`, `bundle/px_lane.rs` `ensure_px_for_apply`), so the repeated apply the row describes now installs px from the archive the operator just copied in; the store addressed is the one under the install root the applied profile chooses, not the process's. A px still absent is `px: NOT INSTALLED` in the summary, a warning naming the asset, the store and `vibe self px install --from <directory-or-zip>`, and the report's new `px` member (`vibe-self-apply/1` `px.outcome` / `px.fix`), at exit 0 — the profile was applied. `vibe self px status` now reports and then refuses a px mode with no px anywhere (non-zero, the `self doctor` convention), while a store without the pinned copy is still a plain fact when `px_path` or PATH answers. Law: PROP-019 `##COMPANION-TOOLS-APPLY`. Not taken: the release README's mention of the px asset beside the zip (documentation, left for the release-notes pass). |
| @fact:B251-FILED **filed by** | Corporate-env campaign, owner feedback on the delivery, 2026-11-09. |

## B-252 — a profile-guided band of the system prompt prefix {#b-252}

| field | value |
|---|---|
| @fact:B252-WHAT **what** | Beyond boot, an agent reads specs on demand (PROP/FEAT documents, package docs). A profile of which documents real sessions read — collected by a read hook (PROP-064) or, at no cost, from the session transcripts already kept on the machine — could feed one more cached band after the unconditional INDEX band of PROP-076, so frequently read documents arrive in the cached prefix instead of by tool call. The analogy is PGO: a committed profile (a sorted list of paths, never session content), rebuilt by an explicit command, stale only in the sense of suboptimal. |
| @fact:B252-LIMITS **limits** | Unlike PGO the change is not semantics-preserving: added text changes the agent's behaviour, so the band is additive only (no LTO-style removal of boot text that seems unused — its use cannot be observed) and needs a with/without behaviour comparison, which is a paid, owner-authorised run. Inclusion is a cost rule, not a frequency cut: a document in the prefix is paid as a cache read on every turn of every session, a document read on demand once, so the threshold depends on read probability against turns per session. Order the band by path, never by frequency, or a small shift reorders bytes and breaks the cache. Place it before the OS-gated band so machines on different OSes share it; it is the fourth cache breakpoint (prefix, INDEX, profile, last message), which is Anthropic's limit. Central sessions and quiet-clause workers read different sets: profile the central session only, or keep exactly two profiles — never one per task kind. Personal layers (`90-user`, override) never enter a profile. `vibe check` warns on a stale profile, it does not fail. |
| @fact:B252-SEVERITY **severity** | P3 — an optimisation idea; nothing is broken without it. |
| @fact:B252-DISPOSITION **disposition** | `candidate` — a separate campaign after SYS-PROMPT: it needs PROP-076's bands and breakpoints (P3–P5) and the real cache-hit metrics of its SYS-ADAPTERS phase to prove the band saves rather than costs. Not part of PROP-076. |
| @fact:B252-FILED **filed by** | Owner idea in the SYS-PROMPT discussion, 2026-11-10. |

## B-253 — Node.js built from source as an installer option {#b-253}

| field | value |
|---|---|
| @fact:B253-WHAT **what** | `org.vibevm.node/node` publishes only upstream's prebuilt archives, so nvm's `install -s` (build from source) and `-j <N>` (build parallelism) have nothing to map to (PROP-075 `#VIBENVM-MAPPING`). A source-build installer option, selected with `--installer`, would build a version from the release's source tarball, verified against its `SHASUMS256.txt`, on hosts and architectures with no prebuilt archive; the parallelism would be a declared option parameter, not a shim flag. |
| @fact:B253-SEVERITY **severity** | P3 — prebuilt archives cover every supported target; source builds serve rare hosts. |
| @fact:B253-DISPOSITION **disposition** | `candidate` — owner decision 2026-11-10: recorded, not done now; until then `-s`/`-b`/`-j` stay owner-approved exclusions of the vibenvm differential suite. |
| @fact:B253-FILED **filed by** | Owner, APP-VERSIONS P7 exclusion review, 2026-11-10. |

## B-258 — PROP-020 `#trust-gate` is unreached on the path every command takes {#b-258}

| field | value |
|---|---|
| @fact:B258-WHAT **what** | On the canonical install path a `[hooks]` entry is lowered to a synthetic `slot:` script contribution and run through the lifecycle handler engine, which consults **no** `HookPolicy` and asks nothing — `crates/vibe-install/src/apply.rs` says so in as many words («install itself is consent, so this path has no `HookPolicy` or prompt surface»), and nothing in `vibe-cli` ever constructs one. PROP-020 `#trust-gate`'s allow-list (`DEFAULT_ALLOWED_GROUPS`), its attended consent question and its unattended refusal are therefore reached only by the legacy `apply` / `apply_with_spec_format` entry points, which no command uses: every published package that declares `[hooks]` runs its hook unasked. |
| @fact:B258-EVIDENCE **evidence** | Measured while implementing PROP-065 `#ordinary-trust` (APP-VERSIONS P7e, `campaigns/app-versions-2026-11/LOG.md`, 2026-11-10) — the fact was first written claiming the two gates were behaviourally identical, the measurement falsified that, and the correction is recorded in the fact as residue: «PROP-020 `#trust-gate` is unreached on the path every command takes, which is a defect of that gate and not of this one». |
| @fact:B258-SEVERITY **severity** | P2 — a documented trust gate no command reaches reads as protection that is not there, and the claim is in a spec a reader trusts. |
| @fact:B258-DISPOSITION **disposition** | `candidate` — not closable silently. Gating `[hooks]` would newly **ask**, and unattended newly **refuse**, for every published package that declares one, so it needs an owner decision and a migration note. **The second half of this disposition is done (2026-11-10):** the two allow-lists `#ordinary-trust` used to union are now one mechanism — `vibe_core::trust`, base list `["org.vibevm.*"]`, with the site profile's `[trust]` table and `agents.toml`'s `trusted_groups` / `untrusted_groups` as its other layers — and `vibe-workspace`'s `DEFAULT_ALLOWED_GROUPS` no longer exists, so the two gates read the same policy and cannot diverge again (owner decision, `AV-FMT-104`, `PROP-020#ALLOW-LIST-SUCCESSOR`). What remains is exactly the original finding: the policy the `[hooks]` gate now shares is still **not consulted** on the path every command takes. |
| @fact:B258-FILED **filed by** | APP-VERSIONS P7e and its tail, 2026-11-10. |

## B-259 — three tests flake under parallel load {#b-259}

| field | value |
|---|---|
| @fact:B259-WHAT **what** | `vibe-orchestrator`'s `source_and_prebuilt_compose_through_phase_and_slot_with_one_image_machine` and `vibe-cli`'s `cli_registry_mirror_templates::a_selected_template_writes_exactly_the_expected_snapshot_tree` fail intermittently when the suite runs with several test threads, and pass when rerun on their own. Neither failure has been attributed yet: a shared temp or cache path, a wall-clock or ordering assumption, and contention on one host resource are all still open. Third, seen 2026-11-11: `vibe-cli`'s `cli_dl::a_stop_file_cancels_the_transfer_removes_the_partial_and_reaps_px` failed once with `px left running` (crates/vibe-cli/tests/cli_dl.rs:508) during a full `--test-threads=8` run while a cold workspace build ran beside it; it passed three times in a row alone. The reap is probably asserted before the child's exit is observable under load — the assertion wants a bounded wait, not a single check. Fourth, seen 2026-11-11: `vibe-registry`'s `tests/proxy_route.rs` failed one of its five cases during a `-p vibe-registry -p vibe-core --test-threads=4` run that shared the host with a cold build, and passed five-for-five in three consecutive reruns on its own. Those cases bind loopback listeners and a proxy, so the shared host resource is a port or an accept deadline. |
| @fact:B259-EVIDENCE **evidence** | Seen while gating the APP-VERSIONS P7e tail with `--test-threads=6`, 2026-11-10; both pass in isolation. `proxy_route` measured 2026-11-11 while gating the `exclude_hosting` enumeration fix, which touches nothing that test exercises. |
| @fact:B259-SEVERITY **severity** | P3 — no product behaviour is implicated, but a flaky gate teaches contributors to rerun rather than to read, which is how a real regression gets waved through. |
| @fact:B259-DISPOSITION **disposition** | `candidate` — find the shared resource in each (not the timing) and make the test own it, rather than serialising the suite. |
| @fact:B259-FILED **filed by** | APP-VERSIONS P7e tail, 2026-11-10. |

## B-260 — the `xtask` optional-shapes census is red on main {#b-260}

| field | value |
|---|---|
| @fact:B260-WHAT **what** | `cargo test -p xtask codegen::optional_shapes::tests::stitch::the_real_vocabulary_home_walks_to_its_sites` fails: «the original pre-portable census is unchanged — left: 105, right: 104» (`xtask/src/codegen/optional_shapes/tests/stitch.rs:253`). The test walks `formats/vocabularies.json`, excluding the portable, full-import and named later fragments, and asserts the remaining baseline still declares exactly 104 optional sites; a fragment added to that baseline since makes 105. Re-confirmed 2026-11-11 on an unmodified checkout of `d0b9fa1c5` (formats/ and xtask/ clean), filed briefly under a duplicate id (since reused for another entry) and folded back here. |
| @fact:B260-EVIDENCE **evidence** | Verified red at `54f7574e5` on 2026-11-10 (run before touching anything, and unaffected by the P7e tail, which adds no vocabulary fragment). |
| @fact:B260-SEVERITY **severity** | P3 — a census, not a behaviour: nothing the product does depends on the number. But it is a red gate on main, so it hides the next real drift of the same shape. |
| @fact:B260-DISPOSITION **disposition** | `candidate` — find the fragment that joined the baseline, decide whether it belongs in the baseline or in the `later` list beside `source_evidence` and `hosting_view`, and move the number or the fragment accordingly. Do **not** simply bump 104 to 105: the exclusion lists are the thing the test is about. |
| @fact:B260-FILED **filed by** | APP-VERSIONS P7e tail, 2026-11-10. |

## B-261 — `vvm/bundle.rs` sits exactly on the 600-line conform budget {#b-261}

| field | value |
|---|---|
| @fact:B261-WHAT **what** | `crates/vibe-cli/src/commands/vvm/bundle.rs` is **600** lines against `conform.toml`'s `max_file_lines = 600`, so the next edit of any size fails the gate and whoever makes it pays for the split. Its seams are already visible in the file: bootstrap and activation (`activate_install`, `bootstrap_with_downloader`), the running-binary and release-version reads (`running_binary_record`, `binary_release_version`), the `latest` rebuild and newest-release move (`rebuild_latest`, `move_to_newest_release`), and the remote half (`remote_context`, `fetch_aggregate`, `install_release_version`, `install_selected`, `finish_install`) — and the directory beside it already holds a dozen cells to join. |
| @fact:B261-EVIDENCE **evidence** | Measured 2026-11-10 (`wc -l`), with conform reporting 0 new findings — the file is at the budget, not over it. |
| @fact:B261-SEVERITY **severity** | P3 — nothing is broken, and the cost is a tax on the next contributor rather than on a user. |
| @fact:B261-DISPOSITION **disposition** | `candidate` — split along the remote seam (`bundle/remote.rs`) before the next feature edit, as its own commit, so the split and the change are not read as one diff. |
| @fact:B261-FILED **filed by** | APP-VERSIONS P7e tail, 2026-11-10. |

## B-262 — `vibe install -g` of an application ignores the declared version file {#b-262}

| field | value |
|---|---|
| @fact:B262-WHAT **what** | `vibe install -g org.vibevm.node/node` in a directory whose `.nvmrc` says `20.18.0` installs the **newest** release instead, measured: it installed 23.1.0. PROP-075 `#VERSION-FILE-FORMATS`' precedence puts the nearest declared version file directly below an explicit requirement and above the slot default, and every other consumer of that precedence (`vibe app env --project`, the prompt hook, `vibe app exec`) honours it — only the install path never consults it. So the one command a user types when a project's file says what it wants is the one command that ignores it. |
| @fact:B262-EVIDENCE **evidence** | Measured 2026-11-11 while building `org.vibevm.node/vibenvm` (APP-VERSIONS P7f): nvm's bare `nvm install` reads `.nvmrc`, and the shim can only reach that behaviour by composing two commands — `vibe app env --project --json` for the `requested` value, then `vibe install -g …@=<requested>`. Recorded in `PROP-075#P7F-AS-BUILT` (4). |
| @fact:B262-SEVERITY **severity** | P2 — a documented precedence that one surface silently does not follow, on the surface where a wrong answer costs a download and a wrong runtime. |
| @fact:B262-DISPOSITION **disposition** | `done` — landed 2026-11-11 as `feat(app): a version file's value selects the way every switcher's does` (`AV-FMT-106`), which carries this reader because the two are the same rule: once a version file's value means what a switcher's means, an install with no version is one more surface that reads it. The chicken-and-egg did not arise: the discovery is the one an activation already uses, the home's own `vibe-app-version-files/1` sidecar of what every **installed** application declares, so the file is consulted exactly when the home already has a provider of that package's slot — and a first install of a package the home never held resolves as it always did. The deciding file and the requirement it produced are one stderr line in every mode; a spelling carrying an `@` is never second-guessed. `#VERSION-FILE-FORMATS`' precedence paragraph now says so. |
| @fact:B262-FILED **filed by** | APP-VERSIONS P7f, 2026-11-11. |

## B-263 — `vibe app versions` has no version-pattern filter {#b-263}

| field | value |
|---|---|
| @fact:B263-WHAT **what** | `vibe app versions <target>` filters by `--channel`, by `--installed` and by count (`--limit` / `--all`), but not by a version requirement: `vibe app versions 'org.vibevm.node/node@22'` is accepted and lists every version anyway, and `vibe app versions 'node@22' --installed` is refused as an unknown target. nvm's `ls <pattern>` and `ls-remote <pattern>` are exactly that filter, and it is the natural question of a list of 400 releases. |
| @fact:B263-EVIDENCE **evidence** | Measured 2026-11-11 (APP-VERSIONS P7f): the vibenvm shim answers `nvm ls 22` with the single version the requirement resolves to, because resolving is the only thing vibe offers and filtering in the shim would be version logic the shim must not hold. `PROP-075#P7F-AS-BUILT` (4). |
| @fact:B263-SEVERITY **severity** | P3 — the information is reachable (the full listing is there), the ergonomics are not. |
| @fact:B263-DISPOSITION **disposition** | `done` — landed 2026-11-11 as `feat(cli): vibe app versions takes a version pattern` (`AV-FMT-106`). Not a flag but a positional `[PATTERN]`, read by the one rule `#VERSION-FILE-VALUE` gives a version file's value — a complete version is itself, a partial one a prefix, a channel name that channel — over the installed listing and the available one, the human rows and `--json`, applied before the limit. The `@<req>` the target spelling already parsed is the same filter, so both symptoms the row reported are gone; a spelling that is neither is the new refusal `bad_version_pattern`. `#VERSIONS-LIST-OUTPUT` amended. |
| @fact:B263-FILED **filed by** | APP-VERSIONS P7f, 2026-11-11. |

## B-264 — a version file can be written only by a command that also changes state {#b-264}

| field | value |
|---|---|
| @fact:B264-WHAT **what** | `--write-version-file` exists on `vibe install -g` and `vibe app use` and nowhere else, so "record the version this shell resolved into the project's file" cannot be asked for on its own. `vibe app env`, the one command whose whole job is resolving for a shell, cannot write it. nvm's `use --save` changes the shell and the file and **not** the global default; the vibenvm shim has to go through `vibe app use` and therefore moves the slot default as a side effect nvm does not have. |
| @fact:B264-EVIDENCE **evidence** | Measured 2026-11-11 (APP-VERSIONS P7f) and recorded in `PROP-075#P7F-AS-BUILT` (4). The suite does not catch it: in `cases/15-version-file.toml` the version being saved is already the default, so the extra effect is invisible there. |
| @fact:B264-SEVERITY **severity** | P2 — a shell-local act silently becomes a machine-global one, and the suite's own coverage cannot see it. |
| @fact:B264-DISPOSITION **disposition** | `done` — landed 2026-11-11 as `feat(cli): vibe app env records the version it resolved` (`AV-FMT-106`). The decision the row asked for, taken: the flag needs **exactly one** `--use SLOT=VER`, so whose file is written is said and never guessed, and the five shapes that cannot answer for it (no `--use`, several, `--unset`, `SLOT=none`, a `--use` that resolved to nothing installed) are usage refusals naming the flag. It changes the shell and the file and no slot default, which is nvm's `use --save` exactly. The written path is stderr, because `app env`'s stdout is what a shell evaluates, and `--json` carries it as the new optional `versionFile` of `vibe-app-env/1`. `#VERSION-FILE-WRITE` amended. |
| @fact:B264-FILED **filed by** | APP-VERSIONS P7f, 2026-11-11. |

## B-265 — a version file written by `--write-version-file` does not pin the version it recorded {#b-265}

| field | value |
|---|---|
| @fact:B265-WHAT **what** | `#VERSION-FILE-WRITE` writes the **resolved** version into the file, and `#VERSION-FILE-FORMATS` reads the file's line as a **requirement** — so a `.nvmrc` holding `22.9.0` selects 22.11.0 once that is installed. Measured: `vibe app env --project` in a directory whose `.nvmrc` says `22.9.0`, with 22.9.0 and 22.11.0 both installed, selects **22.11.0**. The whole reason `--write-version-file` records the resolved version rather than the channel is that "a version file that says `lts` tells the next reader nothing about what was chosen" — and reading it back as a caret range gives that away again. nvm's prefix match pins it. |
| @fact:B265-EVIDENCE **evidence** | Measured 2026-11-11 (APP-VERSIONS P7f). `PROP-075#P7F-AS-BUILT` (4). The differential suite does not catch it: when `cases/15-version-file.toml` reads the file back, 22.9.0 is the only version installed. |
| @fact:B265-SEVERITY **severity** | P2 — a pin file that does not pin, on the surface whose entire purpose is reproducibility between machines. |
| @fact:B265-DISPOSITION **disposition** | `done` — landed 2026-11-11 as `feat(app): a version file's value selects the way a switcher's does` (`AV-FMT-106`), on the owner's requirement that the answer be general for any SDK switcher with nvm parity proven. The **pin** reading, taken as the general rule and stated as a decision fact, `PROP-075#VERSION-FILE-VALUE`: a complete version in a version file selects exactly that version, a partial one is a prefix, a channel keeps its channel meaning, every other spelling keeps its requirement meaning — both formats, every reader. `--write-version-file` keeps writing the plain resolved version, which now round-trips to exactly itself, so the second option's cost (bytes other tools read) is not paid. The first option's cost **is** paid and is recorded rather than hidden: a `.nvmrc` holding `22.9.0` used to select the newest 22.x and a `22.9` reached into 22.10. The CLI spelling is deliberately unchanged — `@22.9.0` is still a requirement and `@=22.9.0` the pin — so nothing recorded in a manifest, a lock or an index changes meaning. |
| @fact:B265-FILED **filed by** | APP-VERSIONS P7f, 2026-11-11. |

## B-278 — the vibenvm shim dropped every flag written after the version {#b-278}

| field | value |
|---|---|
| @fact:B277-WHAT **what** | `nvm use 22 --save` exited **0** and wrote nothing. Every option loop in `vibenvm.sh` ended `*) break` on the first non-option, so the version stopped the scan and every flag behind it was discarded — silently, with a success status, on the one command whose whole observable effect is the file it writes. Upstream nvm accepts a flag on either side of the version for most branches, and the branches where it does not say so with exit 6; it never drops one. |
| @fact:B277-EVIDENCE **evidence** | Measured 2026-11-11 in the corporate-network proof (`nodecorp/followup.log` §4): `nvm use 22 --save` → `[exit=0]`, then `ls -la` shows an empty directory. The audit the fix demanded found three more divergences in the same loops: `--reinstall-packages-from <v>` was accepted where nvm refuses the space spelling, a repeated `--save` was last-one-wins where nvm exits 6, and an unknown long option was ignored by `ls`, `ls-remote`, `version-remote` and `exec` where nvm refuses it. |
| @fact:B277-SEVERITY **severity** | P1 — a flag silently ignored with exit 0 on a state-changing command, and nine passing differential cases above it. |
| @fact:B277-DISPOSITION **disposition** | `done` — fixed 2026-11-11 in `org.vibevm.node/vibenvm@1.1.2`. The discipline is now stated rather than guessed: `PROP-075#VIBENVM-ARGUMENT-ORDER` records nvm's rule branch by branch from the pinned `nvm.sh` v0.41.0 — `install`'s two passes with exit 6 for a selecting flag written after the version, the whole-list scans of `use` / `which` (last version wins) and of `ls` / `ls-remote` / `version` / `version-remote` (first pattern wins), the stop-at-the-version of `run` / `exec`, and the one-version counts of `uninstall` / `reinstall-packages`. **Why nine cases missed it**: not one asked for a flag after the version. `conformance/vibenvm/cases/17-flags-after-the-version.toml` now asks each wish in both positions with the version file in the observation record. |
| @fact:B277-FILED **filed by** | the corporate-network proof, 2026-11-11. |

## B-279 — `nvm use default` was refused although `default` is the slot default {#b-279}

| field | value |
|---|---|
| @fact:B279-WHAT **what** | `nvm use default` answered "`default` is not installed; run `nvm install default`" and exited 3 with the slot default set, and so did `which` / `run` / `exec` / `version` for the same spelling. `PROP-075#VIBENVM-MAPPING` already decided that `default` **is** the slot default while named user aliases stay deferred; the shim handed the word to vibe as a requirement, where it reads as a channel nothing declares. |
| @fact:B279-EVIDENCE **evidence** | Measured 2026-11-11 in the corporate-network proof (`nodecorp/followup.log` §3), immediately after `vibe app use node org.vibevm.node/node@22.23.4` reported the default was set. |
| @fact:B279-SEVERITY **severity** | P2 — the one built-in alias nvm users type most, refused on every subcommand that takes a version. |
| @fact:B279-DISPOSITION **disposition** | `done` — fixed 2026-11-11 in `org.vibevm.node/vibenvm@1.1.2` and recorded as `PROP-075#VIBENVM-BUILT-IN-ALIASES`. `default` is one more spelling of the shim's single translation table, resolved to `=<the slot's default version>` out of vibe's own `vibe app versions <slot> --installed --json` answer — free of a registry and of the network — so it works wherever a version does and nothing downstream knows it was a word; with no default set it is a refusal that names how to set one. An `lts/` **pattern** became an LTS selection in the same place, as it is for nvm. `conformance/vibenvm/cases/18-default-and-channels.toml` asks each subcommand for it by name. |
| @fact:B279-FILED **filed by** | the corporate-network proof, 2026-11-11. |

## B-268 — a `monorepo` registry contributed no directory root, so no template was ever discovered {#b-268}

| field | value |
|---|---|
| @fact:B268-WHAT **what** | `vibe app versions <coordinate> --channel lts` refused with `unknown_channel … channels: (none)` while `vibe app versions node --installed --all` showed `[lts, lts/krypton]` in the same home — and the version list was right in both, which is what made it hard to see. The roots enumeration that decides where a template is discovered knew only `localdir`, though its own doc comment named the `monorepo` working copy; so a home whose only `[[registry]]` is a mirrored monorepo — the corporate shape — had no directory root at all, and every listing fell back to the published version union: no channel table, every row `published`, no template provenance and no mirror-snapshot line. `nvm ls-remote --lts` was the user-visible face of it, answering `N/A` with exit 3. |
| @fact:B268-EVIDENCE **evidence** | Measured 2026-11-11 against `~/corp/admin/vibevm-repo` with the network denied (`nodecorp/followup.log` §2, and the before/after pair run directly against the proof's home). The differential suite **cannot** catch it: its sandbox B publishes into a `localdir` registry, so the registry type the corporate shape uses is structurally outside its reach. |
| @fact:B268-SEVERITY **severity** | P1 — channels, origins and provenance absent exactly in the closed-network deployment the campaign exists to serve, reported as a package that declares no channels rather than as a failure. |
| @fact:B268-DISPOSITION **disposition** | `done` — fixed 2026-11-11 and recorded as `PROP-075#VERSIONS-CHANNEL-DISCOVERY`. Which sources have a directory root is now the registry layer's own decision, taken per variant beside the variants (`RegistrySource::directory_root`), so a kind of registry cannot be left out of it again. Fixing it uncovered a second defect inside the first, and that one is the lesson: a monorepo root is a capture the registry **owns**, so the enumeration was handing back a path its own resolver's drop deleted before the caller read it — same symptom, right version list. The roots now travel with the lease that keeps them on disk, as `LayoutRoot` already does (PROP-073 `#NATIVE-PREPARATION`). Covered by `crates/vibe-cli/tests/cli_app_versions_monorepo.rs` (the listing over a real local monorepo with no `--registry` on the command line), two unit tests over the three variants, and the corporate proof's own re-run. |
| @fact:B268-FILED **filed by** | the corporate-network proof, 2026-11-11. |

## B-269 — a per-package git registry still contributes no channel discovery {#b-269}

| field | value |
|---|---|
| @fact:B269-WHAT **what** | `B-268` closed the `monorepo` half of channel discovery: `localdir` and `monorepo` registries now both offer a directory root, and a per-package **git** registry still offers none, by design — its packages are repositories and there is no tree on disk to read manifests off. So a home whose only registry is a per-package git host sees the same degraded listing B-268 described: the version union, with no channel table, no origins and no template provenance. Nothing measured it, because no deployment in this repository is shaped that way. |
| @fact:B269-EVIDENCE **evidence** | Read off the code while fixing B-268, 2026-11-11: `RegistrySource::directory_root` answers `None` for `RegistrySource::Git`, and `vibe app versions` then falls back to the published versions. The natural answer is the one `--installed` already uses — the template cache, which an install primes — consulted when no root yields a template; whether that is right, or whether a git registry should expose a snapshot of its own, is the decision this row holds. |
| @fact:B269-SEVERITY **severity** | P3 — the public registry is this shape, and the degraded answer is a correct version list **since B-296**; until then it was not, because the template revisions were in it. What is still missing is the channel table, the origins and the template provenance. |
| @fact:B269-DISPOSITION **disposition** | `open` — deliberately out of scope of the corporate-proof fix, which closed the shape the proof measured. Narrowed 2026-11-11 by `B-296`, which made the version list itself right on this shape by reading the index row's `template` flag; the channel/origin/provenance half, which needs the template document and not just the row, is what this row still holds. |
| @fact:B269-FILED **filed by** | the corporate-network proof, 2026-11-11. |

## B-270 — every command re-captures the whole monorepo in a closed network {#b-270}

| field | value |
|---|---|
| @fact:B270-WHAT **what** | In the corporate shape — one `type = "monorepo"` registry, which is a mirrored repository of every package — **each `vibe` invocation captures the repository again**: a `git clone --bare` of the mirror plus a checkout, into a per-process `tempfile::TempDir` that is deleted when the process exits. The capture is shared correctly *within* one operation (`shared_snapshot`, PROP-073 `#NATIVE-PREPARATION`), and not at all between them. Every `nvm` command is one or more `vibe` commands, so a shell session that a user experiences as a version manager pays a full repository capture per command. |
| @fact:B270-EVIDENCE **evidence** | Observed 2026-11-11 while fixing `B-268`. With `VIBE_LOG=vibe_registry=debug`, one `vibe app versions <coordinate> --channel lts` logged `capturing monorepo snapshot` **twice** — once for the roots enumeration and once for the published fallback — and after the fix logs it **once**, because the roots enumeration now finds the template and the fallback is not reached. The corporate offline proof shows the cost directly: each step of its `nvm` session takes tens of seconds against a 16 MiB mirror bundle, and `vibe install -g …@lts --plan-only` walks the whole captured checkout (96 packages) with its file descriptors deep inside `org.vibevm.world/health-audit/v1.0.0/` while doing it. The wall-clock figures of that run are not quoted as a measurement: it ran beside the `vibe-cli` suite at load average 30, and a clean measurement is the first thing this row needs. |
| @fact:B270-SEVERITY **severity** | P2 — it is a user-visible cost on exactly the deployment the APP-VERSIONS campaign exists to serve, and it grows with the mirror rather than with the question asked. Nothing is incorrect, which is why it is a backlog row and not a defect. |
| @fact:B270-DISPOSITION **disposition** | `open`. Two shapes suggest themselves and neither is chosen here: keep the capture in the registry cache keyed by `(url, ref, commit)` so a second process reuses it, which is what the `localdir` path effectively has; or let a `monorepo` source answer a read-only question (version list, template discovery) from a bare clone without a working-tree checkout. The first needs a freshness rule and a cleanup owner; the second needs the template layer to read a tree through git rather than through the filesystem. A measurement on an idle machine comes first. |
| @fact:B270-FILED **filed by** | the corporate-network proof, 2026-11-11. |

## B-271 — a Qwen Code output style set in the harness's own settings is invisible to the launcher {#b-271}

| field | value |
|---|---|
| @fact:B271-WHAT **what** | Qwen Code's identity sentence has two variants, and which one a session gets depends on whether an output style is active: with one, the sentence points at the style instead of at software engineering. `vibe agent qwen`'s carrier reads `--output-style <name>` off the launch's own command line and composes the styled sentence for it, but a style set in `general.outputStyle` in the operator's own `~/.qwen` settings is **not** read — doing so would mean reimplementing another program's settings precedence, which `PROP-076#SYS-QWEN` refuses on the same grounds that keep the whole base prompt out of vibe. Such a session therefore receives Qwen Code's **unstyled** identity sentence. |
| @fact:B271-EVIDENCE **evidence** | Measured 2026-11-11 on Qwen Code 0.27.2 (`getDefaultCoreIdentitySentence`, `packages/core/src/core/prompts.ts:100-107`): the styled and unstyled sentences differ in their closing clause only, and upstream's own test «keeps the style section under a `QWEN_SYSTEM_IDENTITY_MD` override» (`prompts.test.ts:849-860`) asserts that an override owns the sentence verbatim while the style's own `# Output Style: <name>` section still lands. So the cost is one clause of one sentence and the style itself is unaffected. |
| @fact:B271-SEVERITY **severity** | P3 — one clause of one sentence, in a prompt whose style section is still delivered; nothing silently replaces the harness's own instructions, which is the failure class the carrier is gated against. |
| @fact:B271-DISPOSITION **disposition** | open. Two shapes have a chance of being exact rather than a guess: ask the installed package's **own** settings loader for the effective style inside vibe's sampling helper (which already runs that package's code, so it would be the harness answering about itself rather than a reimplementation), or wait for a documented way to ask the program for the style of the session it is about to start. Reading the two settings files directly is explicitly **not** one of them. |
| @fact:B271-FILED **filed by** | sys-prompt P10, the Qwen identity carrier, 2026-11-11. |

## B-272 — Qwen Code's identity carrier is unrun on Windows and macOS {#b-272}

| field | value |
|---|---|
| @fact:B272-WHAT **what** | `vibe agent qwen`'s carrier needs two machine answers that differ per platform: the installed npm package behind the resolved `qwen` program, and the Node that package's code runs under. Both name the Windows and POSIX layouts (`<prefix>/lib/node_modules/<pkg>` with `<prefix>/bin/node`; a `qwen.cmd` shim beside its own `node_modules` with `<prefix>/node.exe`; `node` on `PATH` last) and both are unit-tested over fixture trees — but **no Qwen Code session has been started through the carrier on either platform**. |
| @fact:B272-EVIDENCE **evidence** | The offline whole-request proof and the live check of 2026-11-11 (`campaigns/sys-prompt-2026-11/live/qwen-identity-prove.sh`, `qwen-identity-live-check.sh`) both ran on Linux only; the proof needs `bwrap` and a pseudo-terminal, neither of which is a Windows mechanism. `PROP-076#SYS-QWEN`, residual risk (4). |
| @fact:B272-SEVERITY **severity** | P3 — the failure mode is a **soft launch**: a lookup that answers nothing starts Qwen Code unchanged behind the loud warning, so the worst outcome is the `read` tier the harness shipped with. |
| @fact:B272-DISPOSITION **disposition** | open. What it needs is one real launch per platform with the carrier delivering: `vibe prompt flags --harness qwen` naming a file inside the settings tree is enough to prove the lookups, and a session that quotes the delivered tail marker is enough to prove the slot. The byte-level proof does not have to be repeated there — it is a property of the harness, not of the host. |
| @fact:B272-FILED **filed by** | sys-prompt P10, the Qwen identity carrier, 2026-11-11. |

## B-273 — a `monorepo` registry could not be captured on Windows at all {#b-273}

| field | value |
|---|---|
| @fact:B273-WHAT **what** | On Windows, every `vibe install` from a `type = "monorepo"` registry failed with `✗ source_policy_failed git operation failed … git 'admitted native Git operation' exited with status 128`, once per snapshot candidate, with no stderr. The controlled child reads neither system nor global configuration (`GIT_CONFIG_NOSYSTEM`), so Git for Windows' own `core.longpaths` could not reach it, and its staging checkout lives under the operation's acquisition root — `<settings>/registries/.native/vibe-native-stage-…/checkout/`, some ninety characters before the repository's own deepest path. A mirrored monorepo carries whole dependency trees, so the capture overflowed `MAX_PATH` (260) and Git exited 128 with `Filename too long`. It was never a regression: the same failure reproduces on the 1.0.12 build from `8b8cfce52`, and the earlier "it worked" run had no `site-profile.toml` at all, so it resolved from per-package git registries over the network and never touched the monorepo. |
| @fact:B273-EVIDENCE **evidence** | Measured on the Windows host 2026-11-11 against the corporate mirror bundle (`C:\Users\olegc\vibe-wintest\wc2\mono.git`, 5439 entries, deepest 183 characters). `clone --bare` and the `ls-remote` after it both succeed; the staging `reset --hard`, run by hand with the child's exact recipe and environment, dies with `fatal: cannot create directory at '…/core-ai-native-specmap/src/generated/specmap': Filename too long`, exit 128, and with `-c core.longpaths=true` checks out all 5439 entries and exits 0. The whole repro and the fixed build's `qwen-code` / `opencode` / `node@=24.22.2` installs are in the same day's Windows transcript. |
| @fact:B273-SEVERITY **severity** | P1, release-blocking — the closed-network deployment the APP-VERSIONS campaign exists to serve could not install anything on Windows, and the diagnosis named source policy rather than a path length. |
| @fact:B273-DISPOSITION **disposition** | `done` — fixed 2026-11-11. The controlled child's whole configuration recipe is now `owned_config(windows)`, a pure function of the platform, and on Windows alone it carries `core.longpaths=true`. Covered by a unit test over both polarities of that function (`crates/vibe-registry/src/git_backend/shell/native_admission/context.rs`) and by the Windows proof. The two findings the same investigation produced are `#b-274` and `#b-275`. |
| @fact:B273-FILED **filed by** | the Windows corporate-mirror proof, 2026-11-11. |

## B-274 — an undiagnosable exit 128: the failure published no stderr it was allowed to {#b-274}

| field | value |
|---|---|
| @fact:B274-WHAT **what** | `B-273`'s refusal read `git 'admitted native Git operation' exited with status 128`, then the fixed sentence `direct Git operation failed; redirects and ambient credential routes are disabled`, under `fix: re-run the command by hand and read the stderr below` — with no stderr below it and no argv anywhere. Git had said `fatal: … Filename too long`. The suppression itself is right where it was written: a remote's stderr is untrusted and may reflect authorization. But that reasoning cannot reach an operation that carries no such context — a local repository is read through the filesystem with no credential, header, proxy or TLS input, and an absent remote is an operation on objects the process already owns (`rev-parse`, `read-tree`, `reset --hard`, `fsck`: every step of the staging checkout). |
| @fact:B274-EVIDENCE **evidence** | Finding `B-273` took a hand-rebuilt child with the same recipe and environment on the affected host — not something a user of a closed network can do — because the product's own diagnosis contained no fact about the failure. |
| @fact:B274-SEVERITY **severity** | P2 — nothing is incorrect, and every Windows monorepo failure was unactionable. |
| @fact:B274-DISPOSITION **disposition** | `done` — fixed 2026-11-11. An absent remote and a local repository publish Git's own stderr, bounded to its trailing 4096 bytes on a character boundary, because Git writes the `fatal:` that ends an operation last; a silent failure still answers the fixed sentence, and an HTTPS or SSH contact still publishes none of its stderr. The argv is logged the way a transfer already logs its own. Covered by two unit tests in the same file. |
| @fact:B274-FILED **filed by** | the Windows corporate-mirror proof, 2026-11-11. |

## B-275 — the native preparation's Git 2.53 floor is not checked on the path that uses it {#b-275}

| field | value |
|---|---|
| @fact:B275-WHAT **what** | `NativeState::preflight` refuses a Git older than 2.53 (`native preparation requires Git 2.53 or newer`, `supported_version`), and nothing on the monorepo capture path calls it: the Windows host ran the whole native preparation — clone, `ls-remote`, fetch, stage, checkout — on **git 2.52.0.windows.1**, successfully. Either the floor is real and this path skips it, or the floor is higher than the recipe actually needs and says so to users who could be served. |
| @fact:B275-EVIDENCE **evidence** | Read off the code while fixing `B-273`, and measured: `git --version` on the Windows host is 2.52.0.windows.1, and the fixed build installs `qwen-code`, `opencode` and `node@=24.22.2` from the mirror through the native path on it. `crates/vibe-registry/src/git_backend/shell/native_admission.rs` — `preflight` at the `NativeState` seam, `supported_version` beside it. |
| @fact:B275-SEVERITY **severity** | P3 — the observable behaviour is the permissive one, and it works; the risk is a version-dependent recipe failing later with a worse message than the floor would have given. |
| @fact:B275-DISPOSITION **disposition** | `open` — deliberately out of scope of a release-blocking fix. What it needs first is the decision the row holds: which exact options in the bounded recipe need 2.53, and therefore whether the capture path owes a preflight or the floor owes a correction. |
| @fact:B275-FILED **filed by** | the Windows corporate-mirror proof, 2026-11-11. |

## B-276 — unrelated credential and proxy configuration refused a registry on disk {#b-276}

| field | value |
|---|---|
| @fact:B276-WHAT **what** | A `[[registry]]` whose url is a path or a `file:` url was refused with `native context unsupported: warm transport settings require a separately admitted proxy, credential, header or TLS context` because the invocation's `~/.gitconfig` carried `credential.<url>.*` entries — for a different host entirely. Every corporate workstation has them: a credential helper for the company's own forge, and often an `http.proxy` too. A repository Git reads through the filesystem makes no contact, so none of that configuration applies to it and none can be smuggled in through it. The answer also depended on the spelling: `C:/srv/mono.git` carries a colon and no scheme and was read as an SSH `host:path`, which skipped the check, while the same mirror as `file:///C:/srv/mono.git` — or any POSIX path — was refused. |
| @fact:B276-EVIDENCE **evidence** | Read off `capture_for_request` while fixing `B-273`, then measured as a test: the exact refusal reproduces over a local bare repository with `credential.https://forge.invalid.provider` and `http.proxy` in the selected home (`crates/vibe-registry/src/git_backend/shell/native_admission/local_registry_tests.rs`, which fails with that sentence on the pre-fix tree). |
| @fact:B276-SEVERITY **severity** | P1 for the deployment it hits — a closed corporate network is exactly where both the local mirror and the credential helpers for the company forge are present — and it had not yet been reported, because the Windows spelling of the url happened to skip the check. |
| @fact:B276-DISPOSITION **disposition** | `done` — fixed 2026-11-11. The capture asks whether the selected address contacts anything at all (`local_repository_for`, the reading `LocalRepository` already does) and validates the invocation's transport context only when it does. The refusal is intact for an address that contacts a host, and tested over all four spellings on both platforms. |
| @fact:B276-FILED **filed by** | the Windows corporate-mirror proof, 2026-11-11. |
## B-277 — the routed git classifications still report without their errand {#b-277}

| field | value |
|---|---|
| @fact:B277-WHAT **what** | The registry git failure explanation (`crate::git_failure`) re-frames exactly three classifications — the unclassified `CommandFailed`, `HttpRedirected` and `TlsTrust`. The other transport answers (`AuthFailed`, `RepoNotFound`, `NetworkUnreachable`, `RefNotFound`, `ArchiveUnsupported`) keep their one-line message and so still report without the operation in words, the registry alias, the real URL, the package or the allowed-registries line. A 401 against a corporate mirror is therefore still read as a bare `HTTPS authentication failed … for <url>`. |
| @fact:B277-EVIDENCE **evidence** | Deliberate, and pinned by `git_failure::tests::the_funnel_leaves_the_routed_classifications_alone`. Those variants are not prose: the resolver walk routes on them — `AuthFailed` is the public-401 walk-past (`multi_registry_resolver/walk.rs`, `source_candidates/inventory.rs`), `RepoNotFound` becomes `UnknownPackage` (`git_package_registry/lookup.rs`), `ArchiveUnsupported` selects the clone fallback — and several of the matches that depend on them live in `git_backend/shell/native_admission/**`, which this work was explicitly scoped out of. Wrapping them would change resolution, not only its wording. |
| @fact:B277-SEVERITY **severity** | P2 — the worst case (an intercepted external host) is covered; the remaining half is a smaller, still-confusing message on the paths an operator hits next. |
| @fact:B277-DISPOSITION **disposition** | `open` — the shape that closes it without touching resolution is an **optional context attachment on `RegistryError::Git`** (`Git { source, context: Option<Box<…>> }`), so every `matches!(…, RegistryError::Git(GitError::X { .. }))` becomes `RegistryError::Git { source: GitError::X { .. }, .. }` and the routed identity survives verbatim. That is ~50 call sites across `vibe-registry`, `vibe-cli` and `vibe-install`, including files owned by a parallel worker; it wants its own commit, taken when no one else is in `native_admission/**`. |
| @fact:B277-FILED **filed by** | corporate-env registry-diagnostics slice, 2026-11-11. |
## B-283 — a Git for Windows global known-hosts file is no longer read at all {#b-283}

| field | value |
|---|---|
| @fact:B283-WHAT **what** | The MSYS OpenSSH client Git for Windows bundles reports its compiled-in `GlobalKnownHostsFile /etc/ssh/ssh_known_hosts`. That path has no drive, so it is not absolute on Windows, and the capture now lets it select **no trust** rather than refuse the context. A site that deploys its host keys into `<GitPrefix>/etc/ssh/ssh_known_hosts` — the one place that client itself would find them — therefore gets none of them, and must repeat the file under `UserKnownHostsFile` for `vibe` to see it. Resolving it properly means reading that client's MSYS mount table (`<GitPrefix>/etc/fstab` plus its built-in map), which the capture neither reads nor witnesses. |
| @fact:B283-EVIDENCE **evidence** | The decision and its reasoning are in `trust_files` in `crates/vibe-registry/src/git_backend/shell/native_admission/ssh_config/values.rs`; `msys_rootless_global_trust_defaults_select_nothing_rather_than_refusing` pins the behaviour. `PROP-073#NATIVE-WINDOWS-LAW` states it. |
| @fact:B283-SEVERITY **severity** | P3 — Git for Windows ships no such file, so the common case loses nothing, and the failure mode is an explicit host-key refusal at connect time, never a silent acceptance. |
| @fact:B283-DISPOSITION **disposition** | open — deliberately not mapped. Doing it right means witnessing the mount table as an input of the capture, which is a larger piece of work than the `__PROGRAMDATA__` fix this was found beside. |
| @fact:B283-FILED **filed by** | the Win32-OpenSSH `__PROGRAMDATA__` fix, 2026-11-11. |

## B-284 — `${NAME}` in an SSH path is matched case-sensitively, which Windows is not {#b-284}

| field | value |
|---|---|
| @fact:B284-WHAT **what** | `expand_path` resolves `${NAME}` with `input.environment.get(&name)`, an exact match. Windows compares environment variable names without case, and the captured snapshot keeps whatever spelling the invocation used, so a configuration writing `${ProgramData}` against a captured `PROGRAMDATA` (or the reverse) refuses with "SSH path expansion variable is unavailable". The crate already has the case-insensitive lookup this wants, twice: `variable_in` in `vibe-package-source`, and the new `program_data` beside this code. |
| @fact:B284-EVIDENCE **evidence** | `crates/vibe-registry/src/git_backend/shell/native_admission/ssh_config/values.rs`, the `$` branch of `expand_path`. Found while adding the `__PROGRAMDATA__` token expansion, which does compare without case. |
| @fact:B284-SEVERITY **severity** | P3 — it refuses rather than mis-resolves, and `${…}` in an ssh_config path is rare; but the refusal names a variable the caller can see is set. |
| @fact:B284-DISPOSITION **disposition** | open — out of the scope of the `__PROGRAMDATA__` fix, which only had to stop the refusal that broke `vibe install -g` over SSH on Windows. The fix is to fold the lookup to the same case rule the rest of the Windows path already uses. |
| @fact:B284-FILED **filed by** | the Win32-OpenSSH `__PROGRAMDATA__` fix, 2026-11-11. |

## B-280 — four registry-opening commands carry no source policy at all {#b-280}

| field | value |
|---|---|
| @fact:B280-WHAT **what** | `vibe outdated`, `vibe registry vendor`, `vibe cache add` and `vibe registry test` build their resolver with `MultiRegistryResolver::open` / `open_refreshing` and never call `with_source_policy_provider`, so the effective `[source_policy]` is the empty default for them. They therefore still enumerate — and contact — a registry whose hosting `exclude_hosting` names, which the enumeration skip of `PROP-073#EXCLUDED-REGISTRY-ENUMERATION` closes everywhere else. They also build no `HttpAdmission`, so the refusal does not catch them either. |
| @fact:B280-EVIDENCE **evidence** | Read off the construction sites while fixing the enumeration skip: `crates/vibe-cli/src/commands/outdated.rs:114`, `crates/vibe-registry/src/vendor.rs` via `commands/registry/vendor.rs:139`, `crates/vibe-cli/src/commands/cache/add.rs:367`, `crates/vibe-cli/src/commands/registry/config/test.rs:86`. `vibe install`, `vibe update`, `vibe app versions` and the template layer go through `vibe_package_source::factory`, which does attach the provider; `vibe search` and both `vibe registry sync` paths were given the filter explicitly in this change. |
| @fact:B280-SEVERITY **severity** | P2 — pre-existing (none of these four ever had a policy), and three of them are read-only reporting; but `vibe outdated` on a corporate machine will still reach an excluded public host, which is exactly what the operator asked not to happen. |
| @fact:B280-DISPOSITION **disposition** | open — deliberately out of scope. The right fix is not a fifth call site for the same filter but routing these four through `vibe_package_source::source_operation`, so they inherit the policy *and* the admission the way install does; that is a wider change than the enumeration skip and touches commands this work was not scoped to. |
| @fact:B280-FILED **filed by** | the `exclude_hosting` enumeration fix, 2026-11-11. |

## B-281 — a resolver built from already-opened sources skips the policy filter {#b-281}

| field | value |
|---|---|
| @fact:B281-WHAT **what** | `Catalog::split` decides the source-policy skip from the *declared* rows, so a `MultiRegistryResolver` built by `MultiRegistryResolver::new` — which is handed already-constructed `RegistrySource` owners instead of declarations — walks every source it was given, whatever the policy says. `disabled_registries()` is empty for such a resolver, so the walk report cannot name the skip either. |
| @fact:B281-EVIDENCE **evidence** | `crates/vibe-registry/src/multi_registry_resolver/catalog.rs`, `Catalog::opened` sets `opened` eagerly and `demand` returns it untouched. Nothing in production uses that constructor today — its only callers are `crates/vibe-install/src/record/provenance/tests.rs:172` and `multi_registry_resolver/catalog_tests.rs:70` — which is why it was left as it is. |
| @fact:B281-SEVERITY **severity** | P3 — unreachable from any command; a seam, not a defect, and the HTTP admission still refuses an excluded host underneath it. |
| @fact:B281-DISPOSITION **disposition** | open — the fix is to make `opened` lazy like the declared path, filtering the handed-in sources by `RegistrySource::url()` at first demand, so one rule covers both constructors. Left undone because it changes when a directly-constructed resolver attaches its progress transfers, which is worth its own commit rather than riding along with the enumeration fix. |
| @fact:B281-FILED **filed by** | the `exclude_hosting` enumeration fix, 2026-11-11. |

## B-282 — the `-v` walk narrates which registry, not which step inside it {#b-282}

| field | value |
|---|---|
| @fact:B282-WHAT **what** | `vibe -v install` now says which registry it is asking and at which URL, and why one was skipped, but a single walk step is still one line: it does not distinguish the `ls-remote` that lists versions from the index probe, the archive read of `vibe.toml`, or the redirect probe that follows a stub tag. When a step hangs or fails, `-v` names the registry but not which of its four round trips was in flight. |
| @fact:B282-EVIDENCE **evidence** | `crates/vibe-registry/src/multi_registry_resolver/walk/negative.rs` — `ask_remote` emits one detail and then runs the whole `lookup` closure, whatever that closure does. The per-step facts exist a layer down in `git_package_registry` and `index_client`, which have no `ProgressTask`. |
| @fact:B282-SEVERITY **severity** | P3 — the diagnosability gap the owner reported (`-v` printed nothing at all about resolution) is closed; this is the next notch of detail. |
| @fact:B282-DISPOSITION **disposition** | open — it wants the observation seam threaded into `GitPerPackageRegistry` and the index client, which is a wider plumbing change than the walk-level narration this commit added. |
| @fact:B282-FILED **filed by** | the `exclude_hosting` enumeration fix, 2026-11-11. |
## B-285 — `--json` carries no `resolved_from`, so a machine reader never sees the short name it typed {#b-285}

| field | value |
|---|---|
| @fact:B285-WHAT **what** | When an application command resolves a bare `node` to `org.vibevm.node/node`, the decision is one line on stderr. A `--json` caller gets the qualified coordinate inside the envelope and no record that it asked with a short name, nor which candidate set the answer came out of. The natural shape is a `resolved_from` member on the envelopes that already name a package — the install report, the plan, `describe`, `versions`. |
| @fact:B285-EVIDENCE **evidence** | `crates/vibe-cli/src/commands/app/short_name.rs` prints the provenance with `Context::decision`, deliberately on stderr so `--json`'s stdout stays one envelope. |
| @fact:B285-SEVERITY **severity** | P3 — the stderr line is present in every mode, so no information is lost to a reader that reads both streams; only a reader of stdout alone is short. |
| @fact:B285-DISPOSITION **disposition** | open — deliberately out of scope. Adding the member means editing several JTD schemas, running `cargo xtask codegen` and filing one `CORP-FMT-0NN` record per envelope; that is a format change of its own and should be decided as one rather than ridden in on a convenience. |
| @fact:B285-FILED **filed by** | the application short-name boundary, 2026-11-11. |

## B-286 — `vibe shell --use` and `vibe app exec` still take a slot id only {#b-286}

| field | value |
|---|---|
| @fact:B286-WHAT **what** | `vibe app use`, `vibe app alternatives` and `vibe app env --use` now read a bare installed package name as the slot that package provides (`PROP-059#SHORT-NAME-SLOT-PRECEDENCE`). `vibe shell --use SLOT=VER` and `vibe app exec SLOT[@VER]` take the same kind of argument and did not get the alias, so the four spellings of one idea disagree. |
| @fact:B286-EVIDENCE **evidence** | `crates/vibe-cli/src/commands/app/exec.rs` and the `--use` handling of `vibe shell`; neither passes its slot word through `app::short_name::slot_target`. |
| @fact:B286-SEVERITY **severity** | P3 — nothing regressed, and the slot id every caller writes today still works; it is an inconsistency in how far the convenience reaches. |
| @fact:B286-DISPOSITION **disposition** | open — `exec` resolves its target inside `vibe-app` and `vibe shell` composes its own homes, so each needs its own small boundary rather than the shared one the other three share. |
| @fact:B286-FILED **filed by** | the application short-name boundary, 2026-11-11. |

## B-287 — `vibe app stop`, `pin`, `unpin` and `launcher` take a launcher key with no short form {#b-287}

| field | value |
|---|---|
| @fact:B287-WHAT **what** | These four take a **launcher key** (`<group>/<name>@<version>#<launcher id>`) or, for `stop`, a process id — not a package coordinate — so the short-name boundary deliberately leaves them alone. A key is long and its package half would resolve like any other short name; a user who can write `vibe app start node` cannot write `vibe app pin node#main`. |
| @fact:B287-EVIDENCE **evidence** | `crates/vibe-cli/src/commands/app/catalog.rs` and `runtime.rs`; `vibe_app::split_launcher_key` is where the package half is already separated. |
| @fact:B287-SEVERITY **severity** | P3 — a convenience, and the version in the key makes it a different decision from a package short name (which version's launcher?). |
| @fact:B287-DISPOSITION **disposition** | open — it needs a rule for the version half before the name half is worth resolving, which is a design question rather than a boundary call. |
| @fact:B287-FILED **filed by** | the application short-name boundary, 2026-11-11. |

## B-288 — a short name naming a remap package still falls through to the application path {#b-288}

| field | value |
|---|---|
| @fact:B288-WHAT **what** | `vibe install -g <group>/<name>` reads the kind off the manifest the registry would serve, so a `kind = "remap"` package installs as an offer without the operator knowing (`PROP-066#package`). That probe parses the pkgref itself and needs a group, so a bare `vibe install -g intranet` answers `None`, falls through to the application path, and fails there with "the package declares no `[application]`" instead of installing the offer. |
| @fact:B288-EVIDENCE **evidence** | `crates/vibe-cli/src/commands/remap/packages.rs::claims` calls `PackageRef::parse(raw).ok()?` and then `resolver.manifest_of`, both before `crates/vibe-cli/src/commands/app/global.rs` qualifies the spelling. |
| @fact:B288-SEVERITY **severity** | P3 — a bare name has never worked on this path at all, so nothing regressed; it is a corner the application boundary did not reach. |
| @fact:B288-DISPOSITION **disposition** | open — the fix is to qualify once, above the kind probe, which means moving the boundary out of `app::global::install` into the dispatcher so both kinds share it. Deliberately not taken here: that dispatcher also routes `mcp:` and project installs, and moving the boundary is a change to all three rather than to the one this work was asked for. |
| @fact:B288-FILED **filed by** | the application short-name boundary, 2026-11-11. |

## B-289 — a `.gitattributes` inside a submodule of a vibe-owned clone still converts {#b-289}

| field | value |
|---|---|
| @fact:B289-WHAT **what** | A working copy vibe reads as data is checked out byte-exact by two levers: `-c core.autocrlf=false -c core.eol=lf`, which Git exports to its own children through `GIT_CONFIG_PARAMETERS` and so reaches a submodule checkout, and a vibe-owned `$GIT_DIR/info/attributes`, which is per-repository and does **not**. A `.gitattributes` saying `* text=auto eol=crlf` *inside a submodule* of a registry monorepo or a package clone therefore still converts that submodule's files, and anything hashed out of them mis-hashes exactly as the top level used to. |
| @fact:B289-EVIDENCE **evidence** | `crates/vibe-registry/src/git_backend/verbatim.rs` (the two levers and why `GIT_ATTR_SOURCE` was not used: it is Git 2.40 and its value depends on the repository's object format); `crates/vibe-registry/src/git_backend/shell/checkout.rs::sync_submodules`. Measured on Linux: the `-c` flags do reach the submodule checkout, an in-submodule `.gitattributes` does not. |
| @fact:B289-SEVERITY **severity** | P2 — it needs a registry monorepo or a package whose submodule carries an EOL attribute, which is rare; where it happens the symptom is the same silent hash mismatch, which is why it is not P3. |
| @fact:B289-DISPOSITION **disposition** | open — the fix is to walk `git submodule status --recursive`, install the attributes in each module's git dir and renormalise it, or to use `GIT_ATTR_SOURCE` with the repository's own empty-tree id once the Git floor allows. Deliberately not taken with the release fix: both add a traversal to every clone, and the reported defect is at the top level. |
| @fact:B289-FILED **filed by** | the byte-exact registry checkout, 2026-11-11. |

## B-290 — the legacy clone path has no attribute admission for a registry working copy {#b-290}

| field | value |
|---|---|
| @fact:B290-WHAT **what** | Under the controlled transport a payload that selects `ident`, `working-tree-encoding` or an external `filter` is refused before its stage is used (PROP-073 `#NATIVE-MATERIALISATION`). The incumbent `ShellGit` clone path — the one a profile without `[source_policy]` exclusions takes — has no such probe for a **registry** working copy: an `ident`-marked file in a mirrored monorepo is expanded on checkout and then fails the hash it was supposed to answer, with no explanation naming the attribute. The byte-exact checkout deliberately does not paper over it: `info/attributes` unsets `text`/`eol` only, precisely so these three stay visible to whoever probes them. |
| @fact:B290-EVIDENCE **evidence** | `crates/vibe-registry/src/git_backend/shell/checkout.rs` (no probe); `crates/vibe-registry/src/git_backend/shell/native_admission/materialisation.rs` (the probe the controlled path runs); `crates/vibe-registry/src/git_backend/verbatim.rs` for why the attributes are left alone. |
| @fact:B290-SEVERITY **severity** | P3 — it predates this work and needs a registry that marks its own files `ident` or routes them through a filter, which a mirror has no reason to do. |
| @fact:B290-DISPOSITION **disposition** | open — the probe exists and is reusable; what it needs is a home on the incumbent path that does not turn every clone into an extra `check-attr` walk. |
| @fact:B290-FILED **filed by** | the byte-exact registry checkout, 2026-11-11. |

## B-291 — the project install boundary applies no trust filter, to aliases or to names {#b-291}

`vibe install -g qwen` asks the trust policy about an alias target's group
before picking it (`#SHORT-NAME-TRUST`): installing an application runs that
author's code, so a group nobody trusted is listed and never chosen. The
**project** boundary — `vibe-cli::commands::short_name`, which qualifies
`vibe install wal` — applies no trust filter at all, and this landing did not
add one for aliases either, because filtering the offered spellings while
leaving the real names unfiltered at the same boundary would be incoherent.
Whether a project dependency should be trust-narrowed is a decision about the
exact candidates too, not about aliases, and it is the owner's to take.
Severity **P3**.

## B-292 — a per-package registry's alias table is fetched unvouched {#b-292}

`IndexClient::alias_table` GETs `<index>/aliases.toml` so `vibe registry
mirror` can carry a per-package registry's spellings across.
`repomd.json` does not vouch for it — correctly, because the manifest
describes what the WRITER writes and the table is what the writer was told —
so the by-name freshness check that catches a stale cache (`by_name_is_current`)
has no counterpart here. A cache serving an old table makes a mirror carry an
old row, which the merge rule then records as current. The fix is either a
digest of the table inside `repomd.json`'s own body (not its `files` map) or a
conditional request; both are a format question. Only the mirror path is
affected: a RESOLVING client never fetches this file, it reads the alias off
the `by-name` file it already gets. Severity **P3**.

## B-293 — the mirror report says nothing about aliases in `--json` {#b-293}

A `vibe registry mirror` run reports its alias decisions — rows dropped
because this mirror does not hold their package, rows kept because the mirror
owner edited them, rows withdrawn upstream — on **stderr** only. The
`registry-mirror-report` envelope (`schemas/registry_mirror_report.jtd.json`)
carries none of them, so a machine reader sees a silent merge. Three fields
(`aliases_copied`, `aliases_dropped`, `aliases_kept_local`) would be additive
and epoch-neutral; they were left out of CORP-FMT-043 to keep that landing to
one wire schema. Severity **P3**.

## B-294 — the template-snapshot mirror test is time-dependent and flakes {#b-294}

`cli_registry_mirror_templates::a_selected_template_writes_exactly_the_expected_snapshot_tree`
asserts that an identical second `vibe registry mirror` run reports
`index_refreshed: false`. It does not, reliably: under `--test-threads=4`
on a loaded machine it failed 2 runs in 8 while passing 3/3 at
`--test-threads=1`.

**Cause, measured.** The refresh verdict is
`!facts.is_empty() || !files.is_empty() || !removed.is_empty() || state != Current`
(`crates/vibe-publish/src/layout_publish/index_plan.rs`), and `files`
holds only the batch files whose bytes DIFFER from the target's. An
instrumented run of the failing case logs

```
PROBE facts=0 files=["_index/mirrors/src.toml",
  "_index/templates/org.vibevm.node/node/snapshot.toml",
  "_index/templates/org.vibevm.node/node/versions.json",
  "_index/templates/org.vibevm.node/node/digests/22.23.4",
  "_index/templates/org.vibevm.node/node/digests/24.22.2"] removed=0 state=Current
```

against `files=[]` on a passing one. `crates/vibe-cli/src/commands/registry/mirror/templates.rs`
stamps `record.sync.vibe` and `record.sync.last = now` on the snapshot
record **unconditionally**, where `now` is `Utc::now()` at second
granularity — so two runs that straddle a second boundary write
different `snapshot.toml` bytes, and the assertion fails. The sibling
`mirror_record` already guards its own `sync.last` behind a `changed`
flag; the snapshot path does not.

**Fix direction:** stamp the snapshot's `[sync] last` only when the
snapshot actually changed, the way `history::mirror_record` does, or
freeze the clock in the test. Note the test's own comment claims the
identical re-run is the end-to-end closure of `#P4-AS-BUILT` amendment
6 — so the assertion is deliberate and should be kept, with the writer
fixed rather than the assertion loosened. Severity **P2** (a flaky gate
hides real regressions).

Independently reproduced by the byte-exact registry checkout work (2026-11-11): on pristine `main` at `b89e8f673`, 10 repeats of `cli_registry_mirror_templates` under `--test-threads=4` failed once and passed alone, so the flake predates both that change and the alias table.

---

## B-300 — `vibe app versions --installed` cannot carry the `in drop` mark {#b-300}

The `in drop` mark of PROP-075 `#VERSIONS-LIST-OUTPUT` (landed 2026-11-12,
`AV-FMT-107`) needs a manifest to learn which files this platform's installer
option downloads. `--installed` deliberately answers from this home and the
template cache alone — no registry root, no version records — so it has no
manifest for any row and marks nothing. `--in-drop` with `--installed` is
therefore refused at the parser, which is honest but is a hole in a grid a user
can see.

**The fix that is available and was not taken.** An installed version's own
rendered manifest is already on this machine, in its library directory, and the
index record names the option that was chosen (`Installation::option`,
`platform`, `arch`) — so the names could be read with no registry and no
network at all, which would also make the mark *more* exact than the
registry path's (the selected option is recorded rather than re-derived). What
stopped it in this change was scope: the probe is one seam (`Domains::in_drop`)
fed by the registry adapter, and a second source would have meant a second
feeding path through the same seam for the one flag combination that is least
useful — an installed version is already installed, so "is its payload in the
drop" answers nothing a user acts on. Severity **P3** (a refused flag
combination with a clear message, not a wrong answer).

## B-301 — a `template_optional` installer option can be marked `in drop` although a full render would drop it {#b-301}

PROP-075 `#TEMPLATE-TWO-STAGE`'s skeleton render substitutes every digest
variable with the placeholder, and `template_optional` drops an option only on
a `MissingDigest` error — which a skeleton therefore never raises
(`crates/vibe-template/src/render.rs`, `render_options`). So where upstream
publishes no file for this platform at some version, the listing still sees the
option, computes its file name and may mark the row, while the install that
follows would find that option dropped.

**Why it was left.** It cannot produce a wrong mark in practice: the reason the
full render drops the row is that upstream's digest document names no such
file, which is also the reason nobody could have put a file of that name in the
drop. Closing it properly would mean reading the digest document — exactly the
cost the mark exists to avoid — so the honest alternatives are to leave it, or
to teach `template_optional` a second, digest-free criterion. Recorded so that
a future reader of a surprising mark finds the mechanism rather than hunting
it. Severity **P3**.

## B-296 — a template revision was listed and selectable on every indexed registry {#b-296}

| field | value |
|---|---|
| @fact:B296-WHAT **what** | `vibe app versions org.vibevm.node/node` answered five rows against the public `vibespecs` registry, of which `1.1.0 (published)` and `1.0.0 (published)` were the template's own revisions and not versions of Node.js at all. The same two rows were **selectable**: `@^1`, `@1.1.0` and `@~1.0` each resolved to the template document, `vibe outdated` could report one as an available update, and the depsolver enumerated them as ordinary candidates. The directory-shaped path never had the defect — `discover_in_directory` splits the manifests on disk and `union_versions` removes the revisions — so the two registry shapes disagreed about what the same package's versions were. |
| @fact:B296-EVIDENCE **evidence** | Measured 2026-11-11 against the real public registry with a `debug` build. `VersionEntryView` (`crates/vibe-registry/src/index_client/wire.rs`), the registry client's hand-written partial decode of a `by-name` row, carried `version` and `must_understand` and no `template` field, so serde dropped a flag the index had been writing since AV-FMT-003 — confirmed present on `by-name/node.json`, `by-name/opencode.json` and `by-name/qwen-code.json` in `vibespecs-index-work`. Six revision rows across three packages; none outranked its package's newest produced version, which is the only reason `install -g <package>` had not yet installed a template. |
| @fact:B296-SEVERITY **severity** | P1 — wrong version domain on the registry shape every public user has, and a selection that would have landed on the template itself for any package whose revisions outrank its produced versions (`org.vibevm.nvm/nvm`: revisions `1.x`, versions `0.41.x`). |
| @fact:B296-DISPOSITION **disposition** | `done` — fixed 2026-11-11 and recorded as `PROP-075#TEMPLATE-REVISION-NOT-A-VERSION`. The flag is read where it arrives and the split is applied once in `list_candidates_with_route`, above both the listing and the selector, so they cannot disagree; discovery, a mirror and a lock each keep seeing revisions because each reads one by identity rather than selecting it. Covered by two unit tests over the decode and two end-to-end tests over a mock index (`crates/vibe-registry/tests/index_fast_path.rs`), and verified against the public registry. |
| @fact:B296-FILED **filed by** | the `org.vibevm.nvm/nvm` template package, 2026-11-11. |

## B-297 — a template whose produced versions download nothing could not be installed {#b-297}

| field | value |
|---|---|
| @fact:B297-WHAT **what** | `[template.digests]` was a required member of `[template]`, and `TemplateLayer::full` fetched that version's digest document as its **first act** — before it knew whether the manifest named a single `{sha256:…}` or `{size:…}` variable. So a template package whose produced versions carry no downloaded input at all, and whose payload is a file already in the template's own tree, could not be installed: it had to declare a per-version digest document that does not exist, and the install failed fetching it. The *listing* worked, because a domain renders nothing, which is what made the gap easy to miss. |
| @fact:B297-EVIDENCE **evidence** | Measured 2026-11-11 with a two-file local registry: a template declaring `[template.versions] kind = "list"` with one item, one `kind = "copy"` installer step reading a file out of the package tree, and no digest variable anywhere. `vibe app versions` answered `0.41.0 (template)`; `vibe install -g 'org.example/tool@=0.41.0' --plan-only` answered `✗ registry_failed … the digest document of \`org.example/tool\` could not be fetched from \`https://nothing.invalid/v0.41.0/SHASUMS256.txt\``. That repro is now the committed test `a_template_that_downloads_nothing_needs_no_digest_document`. |
| @fact:B297-SEVERITY **severity** | P2 — it blocked a whole genre of template package (a tool whose release is a script the package itself carries), and `org.vibevm.nvm/nvm` is the first one asked for: nvm-sh publishes no checksum document per release, so the nvm shim could not be a template package at all. |
| @fact:B297-DISPOSITION **disposition** | `done` — fixed 2026-11-11 and recorded as `PROP-075#TEMPLATE-NO-DIGEST` with the format record AV-FMT-033. `[template.digests]` is optional; a template that declares none may not name a digest variable, refused **by name at the variable** (only the variable's site knows it is used) and at resolution rather than at install; `full` makes no request. The lock's `digest_source_sha256` and the mirror snapshot's `digest_url` / `digest_sha256` become optional — **absent, never an empty string**, because `#TEMPLATE-LOCK-RECORD`'s alarm compares recorded with computed and an invented value would hide a real change. The lock pays one epoch for that (10 → 11, `DIGESTLESS_TEMPLATE_SCHEMA_VERSION`, pinned by name); the ladder gained a rung and no lock written before this changes a byte. The installed-application record needed nothing: its digest members have been optional since AV-FMT-017. |
| @fact:B297-FILED **filed by** | the `org.vibevm.nvm/nvm` template package, 2026-11-11. |

## B-298 — a template package published to a per-package registry had no installable version {#b-298}

| field | value |
|---|---|
| @fact:B298-WHAT **what** | The consequence of `B-269` its own row did not state. A per-package git registry contributes no directory root, so no template was ever discovered there — and a template package published to the public `vibespecs` registry therefore offered **only its revision tags**, which `B-296` has now correctly removed from the version domain. The package was visible, carried a `by-name` record, and had no installable version at all: every produced version lived behind a template the registry shape could not read. |
| @fact:B298-EVIDENCE **evidence** | Measured 2026-11-11 against the real public registry: `vibe install -g org.vibevm.node/node@lts --plan-only` answered `✗ source_policy_failed  no version of \`org.vibevm.node/node\` matches \`lts\``, although the template declares the `lts` channel and the same command succeeds against a `localdir` or `monorepo` registry. The three template packages in public are each saved by their concrete publications — `#TEMPLATE-CONCRETE-WINS` carries them, not their templates. |
| @fact:B298-SEVERITY **severity** | P1 for a package that publishes no concrete version, which is exactly what `org.vibevm.nvm/nvm` is. |
| @fact:B298-DISPOSITION **disposition** | `done` — fixed 2026-11-11 and recorded as `PROP-075#TEMPLATE-IN-A-REPO`. The index row says which published version is the active revision, the revision's `vibe.toml` is read once at its tag as text, and the four call sites of `#TEMPLATE-DOMAIN-SEAM` are served from one module beside the registry. Discovery costs the index read plus one file read — not one per tag — and a checkout happens only to render. Proven against the real public registry: `vibe app versions org.vibevm.node/node` answers 409 rows with origins and channels, and `vibe install -g org.vibevm.node/node@lts` installs 26.13.1, a version that exists only in upstream's index document, with `node -v` answering `v26.13.1`. A per-package registry **without** an index stays degraded and says so, because a tag list cannot tell a revision from a version. |
| @fact:B298-FILED **filed by** | the `org.vibevm.nvm/nvm` template package, 2026-11-11. |

## B-299 — two vibe homes on one machine edit the same two shell profiles {#b-299}

| field | value |
|---|---|
| @fact:B299-WHAT **what** | `vibe install -g` of an installer option carrying `path_dirs` runs PROP-065 `#env-step` in its finish phase, which writes a managed block into the **real** `~/.profile` and `~/.bashrc` — even when the invocation is otherwise isolated with `VIBE_SETTINGS` pointing at a throwaway root. The block names that root's `env.sh`, so two vibe homes on one machine write and remove each other's lines in the same two files. An install in one home concurrent with an uninstall in another can leave a block naming a root that no longer exists, or lose a block that should have stayed. |
| @fact:B299-EVIDENCE **evidence** | Observed 2026-11-11 while proving the Node template's `source` installer option: an install under `VIBE_SETTINGS=/fast/tmp/node-src-tmp/…` added `# >>> vibe applications environment …` to the operator's own `~/.bashrc` and `~/.profile`. Restored byte-identically by the matching uninstall, which is what makes the race the finding rather than the write: the write is deliberate and documented, the absence of a per-home lock on those two files is not. |
| @fact:B299-SEVERITY **severity** | P3 — it needs two homes installing or uninstalling at once on one machine, which is a developer's and a test harness's situation rather than a user's. Nothing is lost that `vibe self doctor` cannot restate. |
| @fact:B299-DISPOSITION **disposition** | open — the fix is a lock around the profile edit keyed on the file rather than on the home, plus the question of whether a non-default `VIBE_SETTINGS` should edit the operator's profile at all. The second half is the real decision: an isolated root arguably has no business in `~/.bashrc`, and every test that sets `VIBE_SETTINGS` is currently writing there. |
| @fact:B299-FILED **filed by** | the Node template's source installer option, 2026-11-11. |
