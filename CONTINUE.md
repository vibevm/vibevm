# CONTINUE — cold-resume snapshot (2026-08-05, wind-down №13: бэклог осушается, группа №1 закрыта целиком)

**Не цитируй числа отсюда — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py` ·
`python campaigns/packages-2026-09/tasks/text-stability.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот.

## TL;DR

Сессия шла по новому курсу владельца — **сперва осушить бэклог, к тестам не
подходить**. Закрыто **тринадцать строк**, из них **шесть — измерением**, а не
стройкой: они просили построить построенное. Строк 62 → 50.

**Панель была красной на старте, и структурно.** Гейт свежести карты (B-014,
заведён накануне) падал: `spec/WAL.md` лежит внутри `spec_roots`, wind-down
переписывает его целиком — **последний коммит каждой сессии делал карту
протухшей, и следующая открывалась на красном гейте.** Соседний гейт этот файл
давно изъял по вашему рулингу 2026-07-24; у карты не было ключа изъятия вообще.
Построен, изъяты чекпойнт и компиляторный вывод (643 единицы, **ноль рёбер** —
их никто не цитировал).

**Группа №1 «дисциплина не наведена на себя» закрыта целиком**, включая обе
строки, записанные как «Specified, not built».

## Где стоит работа

- Ветка `main` @ `ff2079e1`, **49 коммитов за сессию**.
- Дерево чистое. `.wt/` пуст — все worktree сняты, все отчёты в архиве.
- **Панель зелёная** — `self-check: all green`, 48 шагов, bare-форма, хвост
  прочитан.
- **`gitverse` синхронен** до `586e7c7`; после него два коммита не разосланы —
  прогнать `cargo xtask mirror`.
- **`github` НЕДОСТИЖИМ** — ssh на `git@github.com` заворачивается на
  `127.92.0.49` (петлевой адрес ⇒ перехват порта 22 на машине). **Это не
  расхождение**, форсить нельзя, и теперь фан-аут говорит это прямо.
- Реестр: 11 641 подтверждённых, **дрейф 177** (был 182), 0 к перепросуждению,
  корпус 274 файла, `progress check` clean.
- Активного блокера НЕТ.

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ

**Пакет для B-045 уже написан и готов к запуску** — он лежит в scratchpad и
воспроизводится по разделу «Как продолжить» ниже. Три доводки грамматики имён,
все измерены, все решения приняты. Если он потерялся — там же сказано, что
измерить и какие решения босс принял.

Дальше по остатку: `AUDIT -01` (дыра покрытия default-path e2e), затем живые
строки бэклога по мере измерения.

## Что закрыто этой сессией

**Тринадцать строк:** B-001, B-002, B-004, B-016, B-021, B-037, B-044, B-051,
B-052, B-053, B-060, B-061 + сужены B-018, B-019.

**Построено:**

| что | суть |
|---|---|
| ключ изъятия карты | перечислимый `spec_exclude` с предупреждениями о мёртвом и невалидном паттерне; движок + 51 вендор-пара |
| `conform-frontend-rust.md` | у пилотного языка появилась спека поверхности, которая была у обеих его проекций |
| дом корневых ключей | описаны один раз в `ENGINE-CONFORM §6`; три спеки цитируют, а не пересказывают |
| no-zombie | проба процесс-таблицы в трёх стеках; растовая **прогнана**, Go/TS собраны и так и названы |
| текст причины отступления | движок + фронтенд; `DeviationStack` вместо счётчика; версия кэша 10 → 11 |
| сканер JTD | схемы читаются; 16 единиц, 7 рёбер; `vibe explain` впервые отвечает про wire-контракты |
| диагноз манифеста | перестал выбрасывать собственную каретку и позицию |
| диагноз зеркал | недостижимый хост больше не объявляется разошедшимся |

## Не-очевидные находки

- **Строка уносит с собой свои цитаты.** Рулинг «строка умирает» не имел
  спутника: удаление B-016/B-021/B-037 оставило висячие адреса в трёх живых
  дизайн-документах. Правило записано: **рулинг закрытой строки живёт в
  коммите, который её закрыл, а не по адресу** — коммит неудаляем, якорь нет.
- **`sync-engines` копирует крейты, но НЕ корневые манифесты.** Вендорённый
  крейт переносит `workspace = true` через копирование и не находит ничего на
  той стороне: три mcp-пакета перестали загружать собственные манифесты.
- **Растяжка `~/.vibe` срабатывает на командах босса.** Панель снимает слепок
  дома в начале и сверяет после тестов; `vibe progress mirror`, запущенный
  рядом, пишет туда кэш — гейт обвинил несуществующий тест. Правило шире, чем
  «не гонять cargo»: под панелью нельзя ни один `vibe`-глагол, пишущий дом.
- **`implements` от голого объявления — ложное покрытие**, и градиент ядовит:
  чем аккуратнее проект объявляет формы заранее, тем больше его накапливает.
  Математики покрытия в индексе нет вовсе — вред доставлялся читателю
  `vibe explain`.
- **Сообщение может быть о правильном событии и называть неправильную
  причину.** Дважды за сессию: манифест (отсутствующее поле как синтаксис) и
  зеркала (недостижимость как расхождение). Второе дороже — оно посылало
  переписывать историю.
- **Бэклог в середине миграции.** Рулинг снял статусы, но строки, закрытые ДО
  него, остались. Живое и историю сейчас не различить; регекс для этого
  ненадёжен (строка, суженная сегодня, читается как закрытая).

## Гейты ловили БОССА, не воркера — пять раз

Параллельный `vibe` под панелью · правка спеки без регенерации карты ·
вендоринг движкового поля без спутника в манифестах · `cargo fmt` после работы
воркера · собственный тест с грубой подстрокой. Все пять — пропущенный шаг,
который сам же и записан.

## Воркеры опровергли босса пять раз

«T-sem-ярус Rust пуст» (там clippy, чьи диагнозы движок читает фактами) ·
«одна разборка, два потребителя» (Rust парсит **дважды**) · триггер отказа,
промахнувшийся по файлу · защита от переиспользования PID через `start_time`
(сильнее заданного) · `crate_name` для схемы, выбранный измерением
потребителей. Плюс один раз босс поймал себя сам: неверный греп
(`impl<P: …>` не совпал) дал бы ложный вывод о целом классе.

## Карта репозитория (верхний уровень)

- `spec/` — PROP/FEAT-контракты (`common/`, `modules/`), `boot/` (PROP-009),
  `design/`, `terraforms/`, `WAL.md`.
- `campaigns/packages-2026-09/` — активная кампания: `harvest/`, `tasks/`
  (`summary.py`, `drift-registry.py`, `text-stability.py`, `merge-verdicts.py`,
  `evidence/`), `run/` (генерится; `run/mirror/` **gitignored**),
  `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml`.
- `packages/org.vibevm.ai-native/` — дисциплина: `core-ai-native/v0.8.0/`
  (движок, вендорится ×6 = 51 пара), `{rust,go,typescript}-ai-native-lang/`,
  `*-mcp/`. **Крейты стеков тоже вендорятся** — в mcp-пакеты.
- `crates/` — хост, 18 крейтов. `xtask/` — инструментарий.
- Корень: `BACKLOG.md`, `TOOLING-MAP.md`, `AUDIT.md`, `TASKS.md`, `ROADMAP.md`,
  `specmap.json` (под гейтом), `specmap.toml`, `conform.toml`, `schemas/`
  (теперь в карте), `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (байт-идентичны).

## Действующие решения (в силе)

- **План — не источник истины.** Строка умирает вместе с коммитом, делающим её
  неправдой; закрытие есть переезд содержимого в спеку. **И её цитаты уезжают
  с ней.**
- **Перед реализацией плана измеряют, что уже реализовано.** За сессию это
  сэкономило шесть строек; всего за два дня — семнадцать.
- **`implements` — утверждение о коде, который работает** (PROP-014).
- **Один закон — одна реализация**, расхождение двух молчит по природе.
- **Помечать, а не гасить**; **сигнал, а не стена**; **лечи молчание**.
- **Бюджет 600 строк** меряет босс после `cargo fmt`.
- **BUILD-FIRST**; **T/F/G вне добра**; версии не бампать до публикации.
- **Делегация:** claudez-воркеры; вердикты, ревью и коммиты — босс.
- **Роллаут — ТОЛЬКО `cargo xtask mirror`**, fast-forward, никогда `--force`.
- **Движковую правку всегда сопровождает `cargo xtask sync-engines`** —
  отдельным шагом.

## Цепочка последних коммитов

```
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
572f3c1a fix(specmap): an implements edge is a claim about code that runs (B-061)
c9877c14 chore(packages): the three mcp lockfiles record the sysinfo pin
24edd190 fix(packages): a vendored crate's workspace dependency must exist in every workspace carrying it
fbc4a9ed chore(campaign): the three moved citation lines earn their restatements
1b285725 docs: a closed row takes its citations with it, and today it did not
27e9fb41 docs(campaign): the panel's home tripwire fires on the boss's own commands
659d7b3a chore(campaign): five oaths stop drifting because the build made them true
1b06ec66 docs(backlog): the two parity rows die with the builds that made them untrue (B-044, B-053)
60ea56a2 chore(packages): vendor the no-zombie probe and the reason frontend into the mcp copies
5ddf38ce feat(rust-ai-native): the frontend fills the reason the engine learned to carry (B-053)
f3b5574d test(ai-native): the no-zombie oath stops holding on words and asks the OS (B-044)
7d1e95e9 docs(audit): the two PROP-011 refinements are no longer in the same position (-11)
6c97e15b docs(backlog): the map's consumers shipped in wave В, and two rows were asking for them (B-016, B-018)
```

## Ждёт владельца

1. **`github` недостижим — единственное, что требует ВАС.** ssh на
   `git@github.com` уходит на `127.92.0.49`. Диагностика:
   `ssh -vT git@github.com 2>&1 | head -5` и
   `git config --get-regexp 'url\..*\.insteadof'`.
2. **B-050** — dylint для Rust: вопрос nightly-пина. Вы парковали 2026-08-04,
   мандат 2026-08-05 назвал паритет блокирующим для TS и Go. Рулинг, не стройка.
3. **AUDIT `-14`** (P2, новая) — контракт индекса единственный без схемы и без
   гейта кодогенерации: чеканить `index-entry.jtd.json` или записать, почему он
   намеренно prose-first.
4. **AUDIT `-04`** — одно решение про незаконченную TUI-подсистему (41 из 57
   подавлений там), а не 57 суждений.
5. **Миграция бэклога** — удалять ли строки, закрытые ДО рулинга. Это решение
   об истории файла.
6. Прежние: гейт выхода фазы E; **B-007**, **B-015**, **B-017**, **B-020**,
   **B-024**; AUDIT `-06`/`-07`, `-13`, райдер `2026-06-12-01`.

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/text-stability.py
bash tools/self-check.sh              # exit настоящий, хвостом; фоном — bare-форма, БЕЗ параллельных vibe-команд
cargo xtask specmap --check
cargo xtask sync-engines              # после ЛЮБОЙ правки движка или крейта стека
cargo xtask conform check
cargo xtask mirror                    # раскатка, fast-forward-only
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
