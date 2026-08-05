# CONTINUE — cold-resume snapshot (2026-08-06, wind-down №14: бэклог осушается дальше, заведён P1)

**Не цитируй числа отсюда — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py` ·
`python campaigns/packages-2026-09/tasks/text-stability.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот.

## TL;DR

Курс владельца прежний — **сперва осушить бэклог**. Закрыто **две строки**
(B-008, B-045) и **старейшая находка аудита** (`-01`, открыта с 2026-05-23).
Строк 50 → 48.

**Заведён P1 и он ждёт вас** — треть вердиктов кампании не имеет собственных
доказательств. Подробности ниже, раздел «Ждёт владельца», пункт 1.

**Четыре измерения соврали за сессию, и все четыре — греповые.** Одно чуть не
забраковало верную работу воркера. Все записаны там, где их прочтёт следующий.

## Где стоит работа

- Ветка `main` @ `c38725d8` (+ чекпойнт-коммиты этого wind-down), **11
  коммитов работы за сессию**.
- Дерево чистое, `.wt/` пуст, все отчёты воркеров в архиве, `meta.md` написаны.
- **Панель:** прогон запущен последним действием сессии; **хвост обязан быть
  прочитан до раскатки** — если он не зелёный, чините вперёд, не откатывайте.
- `gitverse` синхронен до `586e7c7` (два коммита прошлой сессии + одиннадцать
  этой не разосланы) — прогнать `cargo xtask mirror`.
- **`github` НЕДОСТИЖИМ** — ssh на `git@github.com` заворачивается на
  `127.92.0.49`. Не расхождение, форсить нельзя. Единственное, что требует
  ваших рук.
- Активного блокера НЕТ.

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ

**Пересканировать корпус кампании.** Эта сессия правила
`spec/modules/vibe-registry/PROP-008-qualified-naming.md` — документ с **92
вердиктами**, — и кэш кампании об этом не знает. Все три measurement-команды
до пересканирования отвечают про кэш, а не про дерево. `text-stability.py`
теперь **сам об этом предупреждает** (это одна из находок сессии), но сделать
скан всё равно надо: `vibe progress scan` / `mirror`, затем пере-суд §2.6 —
там одна изменённая формулировка (`##SHORT-AT-BOUNDARY`) и один новый факт
(`##INSTALLED-STATE-RESOLVES-LOCALLY`).

## Что закрыто этой сессией

**Две строки бэклога и одна находка аудита:**

| что | суть |
|---|---|
| **B-008** | `vibe-index` — единственный из двадцати участников воркспейса без объявления лицензии, при норме в `PROP-000 §3` и «фактe» в леджере владельца. Плюс **чекер**, которого у нормы не было ни в панели, ни в конформе, ни в `vibe check` |
| **B-045** | префикс вида наконец сверяется (`KindMismatch` + мёртвый код возврата 4 ожил); `uninstall`/`update` берут короткое имя из lockfile без сети; redirect-глаголы требование сохранили **с записанной причиной**; цитаты переехали |
| **AUDIT `-01`** | e2e пути по умолчанию: реестр только в машинном доме, `vibe init` без флагов, и проверка «проектный манифест пуст» — она и не даёт тесту стать копией существующего |

**Построено попутно:** цензус поверхностей хоста (B-047, первый пункт);
предупреждение в `text-stability.py`; шаг панели про лицензии.

## Не-очевидные находки

- **Код обещал больше, чем делал, в ЧЕТЫРЁХ местах сразу.** Спека PROP-008,
  докблок `PackageRef`, `docs/commands/uninstall.md` и `update.md` — все
  описывали сверку вида и короткую форму аргумента; типа `KindMismatch` не
  было ни в одном исходнике, а код отбивал ту самую форму, которую доки
  обещали. Стройка сделала все четыре правдой разом.
- **Греп соврал четыре раза, и каждый раз по-своему.** *(i)* `"command":"cargo`
  не совпал, потому что воркер пишет `cd … && cargo …` — чуть не отправил
  отказ верной работе; *(ii)* шаблон без проверки квалификации посчитал
  `flow:org.vibevm.world/wal` неквалифицированным; *(iii)* `grep -P` в этой
  локали ОТКАЗЫВАЕТСЯ работать, пишет об этом в поток, который конвейер
  выбрасывает, и возвращает чистый ноль; *(iv)* шаблон `**Decision:**` нашёл
  24 секции там, где их 146, потому что в этом дереве метка пишется
  `**Decision.**` — с точкой. Первый записан в закон транспорта, остальные — в
  соответствующие строки аудита и бэклога.
- **Бюджет длины файла попытался выбрать форму публичного типа.** Воркер
  схлопнул структурный вариант ошибки в строку, чтобы уложить `lib.rs` в 600.
  Замер был верен, вывод — нет. Настоящий шов оказался в самом типе ошибки:
  `SolveError` уехал в свой модуль, `lib.rs` 599 → 523 при базовых **590** —
  то есть файл стоял в десяти строках от бюджета ещё до правки.
- **Инструмент свежести сравнивал два поля внутри кэша.** Правка спеки с 92
  вердиктами дала «0 stale, 0 к пересуду». Замер по дереву: 273 файла из 274
  совпадают с диском, ровно один — нет, и это он.

## Гейты и воркеры

**Воркер опроверг босса один раз:** шестая «мис-цитата» оказалась законной —
`##SHORT-CLI-ONLY` действительно живёт в §2.4 и действительно говорит то, что
цитата утверждает. Пакет считал шесть, их пять.

**Босс не принял работу один раз** — и это первый отказ за кампанию, где
довод воркера был ВЕРЕН, а вывод из него нет. Формулировка отказа назвала
принятое первым, потом ровно один неверный вывод, потом дословный текст для
двух разделов отчёта; доработка вернула всё дословно, включая признание.

## Карта репозитория (верхний уровень)

- `spec/` — PROP/FEAT-контракты (`common/`, `modules/`), `boot/` (PROP-009),
  `design/`, `terraforms/`, `WAL.md`.
- `campaigns/packages-2026-09/` — активная кампания: `harvest/`, `tasks/`
  (`summary.py`, `drift-registry.py`, `text-stability.py`, `merge-verdicts.py`),
  `run/` (генерится; `run/mirror/` **gitignored**), `SUBAGENT-LAUNCHERS.md` +
  `SUBAGENT-MODE.toml`.
- `packages/org.vibevm.ai-native/` — дисциплина: `core-ai-native/v0.8.0/`
  (движок, вендорится ×6 = 51 пара), `{rust,go,typescript}-ai-native-lang/`,
  `*-mcp/`.
- `crates/` — хост, **19 крейтов + xtask = 20 участников воркспейса** (все
  двадцать теперь объявляют лицензию, и это проверяется).
- Корень: `BACKLOG.md` (48 строк), `TOOLING-MAP.md`, `AUDIT.md`, `TASKS.md`,
  `ROADMAP.md`, `specmap.json` (под гейтом), `specmap.toml`, `conform.toml`,
  `schemas/`, `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (байт-идентичны).

## Действующие решения (в силе)

- **План — не источник истины.** Строка умирает вместе с коммитом, делающим её
  неправдой; закрытие есть переезд содержимого **и цитат** в спеку.
- **Перед реализацией плана измеряют, что уже реализовано.** За три дня это
  остановило девятнадцать строек построенного.
- **Греп — измерение, которое врёт в обе стороны.** Не нашёл — тоже не факт.
- **Бюджет 600 строк — нейтральный ключ.** Он не выбирает форму типа; мерит
  босс и только после `cargo fmt`.
- **`implements` — утверждение о коде, который работает.**
- **Один закон — одна реализация**; расхождение двух молчит по природе.
- **Помечать, а не гасить**; **сигнал, а не стена**; **лечи молчание**.
- **BUILD-FIRST**; **T/F/G вне добра**; версии не бампать до публикации.
- **Делегация:** claudez-воркеры; вердикты, ревью и коммиты — босс.
- **Роллаут — ТОЛЬКО `cargo xtask mirror`**, fast-forward, никогда `--force`.
- **После правки движка или крейта стека — `cargo xtask sync-engines`**
  отдельным шагом. *(Эта сессия движков не трогала — только хостовые крейты.)*

## Цепочка последних коммитов

```
c38725d8 fix(campaign): the stability report says when its zero is about the cache, not the tree
4b52b5f0 docs(audit): the doc sweep's blocker is discharged, and its shape changed (-10)
64d66c64 feat(vibe-resolver): the kind prefix is finally checked, and two verbs stop demanding a group (B-045)
1892b8af docs(campaign): the host's surfaces are measured — ten capabilities have no home outside the CLI (B-047)
9218605a docs(audit): a third of the campaign's verdicts carry no evidence of their own (P1)
d3c77ff4 docs(backlog): the ADR practice tripled in five days while the row waited (B-007)
0092e7b2 docs(audit): the doc-requalification sweep is four times what the row claimed (-10)
bd90da81 docs(campaign): a grep that finds nothing is not a measurement either
e1373944 test(vibe-cli): the default install path gets the e2e it never had (AUDIT -01)
3e04256e build(self-check): the licence norm gets the checker it never had
4db4fcc6 fix(vibe-index): the one crate that declared no licence joins its eighteen siblings
a66b87a5 docs(wal): session-end checkpoint — wind-down №13
a9a3fa74 docs(continue): cold-resume checkpoint — the backlog drains, group one closes
ff2079e1 docs(backlog): say how to read a file that is mid-migration
586e7c7a fix(xtask): the mirror fan-out stops calling an unreachable host a divergence
30128855 style(specmap): rustfmt the JTD scanner's tests, authored and vendored
bf81a16e chore(campaign): the generated-code rule is re-judged against the build that finished it
0f12992e docs(specmap): the generated-code rule stops describing an unbuilt half (B-060)
0e990eee fix(specmap): the seven wire contracts enter the map, and the exemption becomes true
e9e60b94 chore(packages): vendor the JTD scanner across the six copies
e0fa42f2 feat(specmap): the designated taggable unit becomes readable — a JTD scanner (B-060)
d605214c docs(audit): the doc requalification sweep is smaller and not independent (-10)
e3a009fc docs(audit): the dead-code row is one subsystem ruling, not 57 judgements (-04)
27c26979 fix(vibe-core): the manifest parse error stops discarding its own diagnosis (AUDIT -15)
f55c906f chore(campaign): the implements rule earns three verdicts and the index note is corrected
```

## Ждёт владельца

1. **AUDIT `2026-08-06-01`, P1 — НОВОЕ, и это главное.** Треть вердиктов
   кампании (**4 151 из 11 862**) имеет единственным доказательством абзац,
   разделяемый с другими вердиктами. У `PROP-008` — **90 якорей на один
   абзац**, и в нём была ложная фраза: «kind validated (KindMismatch)» при
   отсутствующем типе. Тот же смысл кампания судила `drift` четыре раза на
   пакетной стороне, с указанием файла и строки. Три вопроса в `AUDIT.md`:
   считать ли такие вердикты в итог `confirmed`; требовать ли хотя бы одну
   ссылку на каждый `confirmed` факт (это переведёт ~4 151 в «непроверено»
   разом); пересуживать ли 90 якорей `PROP-008` сейчас.
2. **`github` недостижим** — единственное, что требует ваших рук.
   `ssh -vT git@github.com 2>&1 | head -5` и
   `git config --get-regexp 'url\..*\.insteadof'`.
3. **B-050** — dylint для Rust, вопрос nightly-пина. Рулинг, не стройка.
4. **AUDIT `-14`** — контракт индекса единственный без схемы и без гейта
   кодогенерации: чеканить `index-entry.jtd.json` или записать, почему
   prose-first.
5. **AUDIT `-04`** — одно решение про незаконченную TUI-подсистему (41 из 57
   подавлений там), а не 57 суждений.
6. **AUDIT `-10`, переформулирована** — свип оказался не «исправить 169
   ошибок»: доки описывали ЗАКОННУЮ форму. Вопрос теперь редакторский: где
   документация учит полной форме, а где короткая — честный пример. 27 файлов.
7. **Миграция бэклога** — удалять ли строки, закрытые ДО рулинга.
8. Прежние: гейт выхода фазы E; **B-007** (жанр ADR — премиса пересчитана,
   доля утроилась), **B-015**, **B-017**, **B-020**, **B-024**;
   AUDIT `-06`/`-07`, `-11`, `-13`, райдер `2026-06-12-01`.

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/text-stability.py   # теперь предупреждает, когда его ноль про кэш
bash tools/self-check.sh              # exit настоящий, хвостом; фоном — bare-форма, БЕЗ параллельных vibe-команд
cargo xtask specmap --check
cargo xtask sync-engines              # после ЛЮБОЙ правки движка или крейта стека
cargo xtask conform check
cargo xtask mirror                    # раскатка, fast-forward-only
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
