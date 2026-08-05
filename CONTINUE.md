# CONTINUE — cold-resume snapshot (2026-08-05, wind-down №11: B-056 ПОСТРОЕН ЦЕЛИКОМ)

**Не цитируй числа отсюда — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py` ·
`python campaigns/packages-2026-09/tasks/text-stability.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот.

## TL;DR

Владелец задал порядок: **дыры в гейте → реестровая гигиена → B-056 → волна Г**.
Первые три пункта закрыты. **B-056 построен целиком** — четырьмя посадками
дизайна плюс пятой, которой в дизайне не было; попутно закрыт **B-055**.
18 коммитов, панель зелёная с прочитанным хвостом, `gitverse` синхронен.

Главное, что теперь умеет система: **контракт может иметь много источников** —
несколько директив `#source` подряд и/или звёздочку в имени пакета; они
складываются в порядке объявления, сворачиваются **рекурсивно** под уже
существовавшим стражем циклов, и текст каждого узла входит в документ **ровно
один раз**.

## Где стоит работа

- Ветка `main` @ `fff22ff0`. Дерево чистое (кроме `.wt/`, там два чужих
  handle-locked worktree с прошлых сессий).
- Панель зелёная — «self-check: all green», bare-форма, хвост прочитан.
- **`gitverse` синхронен; `github` ОТСТАЁТ** — ssh заворачивается на
  `127.92.0.49` (loopback ⇒ перехват на этой машине). Это **не расхождение**,
  форсить нельзя.
- Реестр: 91 обязательство / 182 drift-вердикта, подтверждённых 11 531.
  Корпус после скана: **272 файла, 13 249 маркеров, 0 неразмеченных, 0 ошибок**,
  `progress check` — clean.
- Активного блокера НЕТ.

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — реестровый долг этого слайса

Он **измерен и назван числом**, не оставлен на догадку
(`tasks/text-stability.py` печатает актуальный список):

1. **10 фактов к перепросуждению** в
   [`spec/design/multiple-sources-and-plugins.md`](spec/design/multiple-sources-and-plugins.md).
   Все десять сдвинулись по одной причине — статус `@spec/plan` → `@impl/done`,
   который посадка и заслужила. Утверждение в каждом не изменилось; **сам факт
   постройки и есть доказательство**, и это самый дешёвый суд в кампании.
2. **Новые факты этого слайса к первому суду**: шесть в PROP-035 §7.3
   (`##SOURCE-SEQUENCE`, `##SOURCE-REPLACE-IS-A-FLAG`,
   `##SOURCE-FACT-OVERRIDE-IS-A-UNION`, `##SOURCE-ONLY-IS-A-DEFINITION`,
   `##SOURCE-RECURSION`, `##SOURCE-GLOB`), две поправки в дизайне
   (`##fold-collision-catcher-was-wrong`, `##recursion-dedup-is-two-things`),
   три в законе транспорта, две в записи B-056 бэклога.
3. **PROP-035 запечатывается КАК ЕСТЬ** — 146 вердиктов побайтно валидны,
   потому что правка только дописывала.

Порядок работ прежний и жёсткий: `vibe progress mirror --campaign …` →
`scan` → вердикты боссом → `merge-verdicts.py` → печать. **Merge и seal никогда
не цепляются одной командой.**

## Что построила эта сессия

### B-056 — пять посадок

1. **Свёртка приняла последовательность** (`crates/vibe-spec/src/merge.rs`):
   `fold_sources(contract, &[sources])`, а `fold_source` остался её вырожденным
   случаем. Все прежние тесты свёртки прошли **без единой правки** — это и был
   смысл сохранённого имени.
2. **Конвейер передаёт ВСЕ `#source`** в порядке объявления
   (`pipeline.rs`) — **этим закрыт [B-055](BACKLOG.md#b-055)**. Недостижимый
   источник называет СЕБЯ, а не seed и не первого.
3. **Закон циклов дотянут до рёбер `#source`** тем же обходчиком
   (`use_graph.rs`: один `visit`, одна карта цветов, один `is_contract`), и
   свёртка стала **рекурсивной** под ним — со **стражем включения**, которого в
   дизайне не было (см. ниже).
4. **Резолвер перечисляет по образцу** (`resolver/glob.rs`): звёздочка в ИМЕНИ
   пакета, сортировка по паре (имя, каталог слота), членство = имя совпало И
   документ на месте, пустой набор законен.
5. **Столкновение определений между источниками** (`pipeline/fold.rs`) —
   посадка сверх дизайна, см. ниже.

### Не-очевидные находки прогона — три, и все против ожидания

- **Дедупликация узла ≠ дедупликация текста.** Обход честно посещает общий
  источник один раз, но свёртка — это текстовое включение: в ромбе тело общего
  источника входило в документ ДВАЖДЫ. Для прозы безобидно, **для фактов
  смертельно** — обычная композиция двух плагинов над общей базой падала бы на
  дубле якоря. Понадобился отдельный **страж включения**. Нашёл воркер,
  измерив собственный тест ромба и записав `count == 2` честно, вместо того
  чтобы подогнать ожидание.
- **Гейт уникальности НЕ МОЖЕТ поймать два источника, объявивших одну
  секцию** — он намеренно терпит повтор заголовка (в слитом виде тот
  неотличим от законного `:add`-склеивания), и провенанс к его запуску уже
  потерян. Ловить обязана свёртка. Тоже находка воркера, измерением гейта
  против фразы дизайна.
- **Докблок резолвера обещал больше, чем делал код**: «не-образец раскрывается
  сам в себя» — а код уходил в `vibedeps/` и для адреса на собственное дерево
  проекта возвращал ПУСТО. Это класс B-055 в новом месте.

### Находки про сам транспорт (записаны в `SUBAGENT-LAUNCHERS.md`)

- **Статус-опрос воркера врёт в опасную сторону:** stream-json пишет в лог и
  промпт, а в пакете строка завершения стоит инструкцией — grep находит её с
  первой секунды. По нему живой воркер выглядит закончившим, и естественный
  следующий шаг (`-c`) даёт двух писателей на одно дерево. Настоящие сигналы:
  нотификация харнесса + файл отчёта; для liveness — grep по **tool-call**
  воркера (`"command":"echo \"PROGRESS`), который пакет подделать не может.
- **Объём лога — телеметрия размышления, а не активность** (строка на токен).
- **Пакет-продолжение, написанный из ревью-заметки, теряет boilerplate:**
  клаузула сердцебиения выпала, и воркер десять минут выглядел как зависший.

## Карта репозитория (верхний уровень)

- `spec/` — PROP/FEAT-контракты (`common/`, `modules/`), `boot/` (PROP-009),
  `design/` (рационали; `multiple-sources-and-plugins.md` — дизайн B-056 с
  двумя записанными опровержениями), `terraforms/`, `WAL.md`.
- `campaigns/packages-2026-09/` — активная кампания: `harvest/`, `tasks/`
  (`summary.py`, `drift-registry.py`, `text-stability.py`, `merge-verdicts.py`,
  `evidence/`), `run/` (генерится; `run/mirror/` **gitignored**),
  `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml` (закон транспорта).
- `packages/org.vibevm.ai-native/` — дисциплина: `core-ai-native/v0.8.0/`
  (движок, вендорится ×6), `{rust,go,typescript}-ai-native-lang/`, `*-mcp/`.
  У всех семи живых слотов свой `conform.toml`. `fractality/` — специспейс.
- `crates/` — хост. **`vibe-spec` перестроен этой сессией:** `merge.rs` (510),
  `use_graph.rs` (590), `resolver.rs` (549) + `resolver/glob.rs` (475),
  `pipeline.rs` (400) + `pipeline/{fold,tests,fold_tests,collision_tests}.rs`.
- Корень: `BACKLOG.md` (до B-059), `TOOLING-MAP.md`, `AUDIT.md` (его активная
  часть — durable home находок аудита), `TASKS.md`, `ROADMAP.md`,
  `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (байт-идентичны, гейт 0c).

## Действующие архитектурные решения (в силе)

- **Один закон — одна реализация.** Один хэш — один калькулятор; один закон
  циклов — один обходчик; одно место знает, что такое ребро `#source`.
  Расхождение двух реализаций одного закона **молчит по природе**.
- **Дедупликация обхода и дедупликация текста — РАЗНЫЕ вещи**, и вторая нужна
  отдельным стражем включения.
- **Ловить надо там, где жив провенанс.** Пост-слияночный гейт видит документ,
  из которого уже вытравлено, кто что принёс.
- **Долг называют числом, а не прячут** в реечный файл и не замалчивают.
- **Помечать, а не гасить**; **сигнал, а не стена**; **лечи молчание, а не
  состояние**.
- **Бюджет 600 строк — нейтральный ключ конформа**, считает ЛЮБОЙ файл,
  тестовые в том числе; меряет только босс и только после `cargo fmt`.
- **Коммитнутая карта проекта байт-воспроизводима.**
- **BUILD-FIRST**; **T/F/G вне добра**; публикация — после рефакторинга; версии
  не бампать; **«замеров нет»** — стоячий ответ.
- **Делегация:** claudez-воркеры (GLM-5.2), закон транспорта
  `SUBAGENT-LAUNCHERS.md`; **вердикты, ревью и коммиты — босс**.
- **Роллаут — ТОЛЬКО `cargo xtask mirror`** (fast-forward, никогда `--force`).

## Цепочка последних коммитов

```
fff22ff0 chore(campaign): rescan and mirror after B-056, with the registry debt named
ced98a6e docs(campaign): the packet written from a review note drops the boilerplate
ee11d0c7 docs(tasks): the fifth landing ticked — the collision the design did not foresee
5254f0cf fix(vibe-spec): two sources defining one section is an error the gate cannot see
9c65dfa3 docs(tasks): B-056's four landings ticked, the fifth named
bc88e530 docs(backlog): B-056 closed with what the build measured
3c53a7ea docs(spec): PROP-035 §7.3 describes a sequence of sources, not one
72bb79ad docs(design): the B-056 design's plan facts are built facts now
38eba517 feat(vibe-spec): a #source glob reaches the fold, through one edge law
f736d72f docs(design): two claims of the B-056 design that the build refuted
bb1fd8eb feat(vibe-spec): the fold recurses under the cycle guard, with an include guard
77224fcf docs(backlog): B-055 closed — the silence on a second #source is gone
1124cd4e feat(vibe-spec): the resolver enumerates a package-name glob, sorted
68f03853 fix(vibe-spec): every declared #source reaches the fold, in declaration order
eb9c8842 docs(campaign): the transport law learns that its own status poll can lie
87bb2bc8 feat(vibe-spec): the fold takes a sequence of sources, not exactly one
16dd103a feat(vibe-spec): the cycle law reaches #source edges — one walker, two edge sets
f6d37c9d docs(tasks): hygiene ticked, B-056 becomes the slice in flight
4ec8d660 docs(wal): session-end checkpoint — wind-down №10
a1dec8bf docs(continue): cold-resume checkpoint — both gate holes closed, hygiene closed whole
b670b2a2 chore(campaign): refresh the zone after the hygiene pass
ab7d2262 docs(campaign): registry hygiene closed — 272 files, nothing stale, nothing unjudged
c8a5e365 docs(campaign): three never-judged files judged and sealed
ea2d6f93 docs(campaign): the stale set reaches zero
f73a08d7 docs(campaign): twenty-two verdicts judged, fifteen files sealed
```

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
python campaigns/packages-2026-09/tasks/text-stability.py   # что реально сдвинулось
bash tools/self-check.sh              # exit настоящий, хвостом; фоном — bare-форма
cargo test -p vibe-spec               # 206 юнитов после B-056
cargo xtask conform check             # длины файлов + правила дисциплины
cargo xtask sync-engines              # вендор ×6 после движковых правок
cargo run -q -p vibe-cli --bin vibe -- install --assume-yes   # рематериализация
cargo xtask mirror                    # раскатка, fast-forward-only
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
