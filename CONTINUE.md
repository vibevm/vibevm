# CONTINUE — cold-resume snapshot (2026-08-17, wind-down №26)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

Длинная рабочая сессия: **шесть из семи шагов блока Ф4.2b**, семнадцать
коммитов, шесть делегированных пакетов — шесть приняты первым проходом, ноль
циклов доработки. Слой преобразований кодогенерации построен на девять
десятых: из сгенерированного дерева ушли `Option<Box<…>>` (было 76), camelCase
(81), `HashMap` (11); пришли открытые словари (11 из 14 копий), канонический
порядок ключей, политика пустого и строгость по реестру. Аннотаций долга
больше нет: `x-vocabulary` 5, `x-empty` 31, `x-default` 21 — все сайты закрыты.

**Седьмой шаг замерен и нарезан, но не начат** — это ровно то место, откуда
продолжать.

Ценность сессии не только в коде. **Дважды правило спасло шаг там, где
измерение солгало**, и оба раза цена ошибки была бы молчаливым переломом
провода. Плюс исправлено утверждение о ПРЕДЕЛЕ ЯЗЫКА, на котором держалось
записанное «неразрешимо».

Панель зелёная на каждой из шести посадок; `vibe check` не сдвинулся ни разу.
Владелец не заблокирован ничем.

## Где стоит работа

- Ветка `main`, дерево чистое, HEAD — сворачивающие коммиты этой сессии.
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`**,
  реальный код выхода 0 (последний прогон на посадке `0fd7ce2d`).
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых ОЖИДАЕМЫ и не
  двигались тридцать три посадки подряд.
- Судейство: **0 неосуждённых, 0 осиротевших**; 34 файла stale. Корпус 281
  файл, 13788 маркеров, 0 неразмеченных.
- Карта: 6061 спек-юнит / 1038 tagged / 976 рёбер / 0 подозрений / 0 сирот.
- Воркеров нет, `.wt/` пуст, ветки `wt/*` удалены. Один worktree под
  `~/.fractality/runs/**` — чужой, не трогать.
- Логи, отчёты, пакеты и `meta.md` с вердиктами — в
  `cache/agents/sorted/F42B{1..6}-*/`; **нарезанный, но не запущенный пакет
  седьмого шага — `cache/agents/sorted/F42B7-DOMAIN-TYPES/`**.

## Блокер и действие человека

**Блокера нет.** За владельцем три вопроса, ни один не держит главную полосу:

1. **S2** — переименование живых `_`-репозиториев в org `vibespecs` (org-права).
2. **Ж8** — что означает `--full` для записи, которую скан больше не видит.
3. **B-056** — у JTD нет 64-битного целого, а `Repomd.File.size` объявлен
   `u64`. Три выхода измерены и записаны в `BACKLOG.md`.

## Что сделала эта сессия — по существу

**Шесть пассов слоя, каждый отдельным коммитом:**

| шаг | коммит | что построено |
|---|---|---|
| Ф4.2b-1 | `50b0aa35` | открытие словаря по `x-vocabulary`: 11 открыто, 3 закрыто |
| Ф4.2b-2 | `21b0ac94` | snake_case полей: 81 → 0; 308 тождественных rename снято |
| Ф4.2b-3 | `17193dd8` | `HashMap` → `BTreeMap`: канонический порядок ключей |
| Ф4.2b-4 | `ab9edff6` | политика пустого по `x-empty`: 31 сайт, 15 схлопываний |
| Ф4.2b-5 | `e4b46885` | формы опционального: `Option<Box<…>>` 76 → 0 |
| Ф4.2b-6 | `0fd7ce2d` | строгость по реестру: правило в машине, ноль байт диффа |

**Дважды правило спасло шаг, где измерение солгало.** *(i)* Пакет b-2 говорил
«снять все 309 полевых rename» — по замеру исключений ноль. Исключение есть:
свойство схемы зовётся `ref`, это ключевое слово Rust, генератор эскейпит
идентификатор в `ref_`, и rename — единственный носитель провода `"ref"`. Его
снятие уронило бы `registry_sync_report`, у которого НЕТ оракула. Скрипт, на
котором стояло «ноль исключений», **не мог сработать ни на одном поле** —
проверено прямо, скормив ему случай, который он обязан пометить. *(ii)* Пакет
b-5 перечислял два класса опционального поля; в дереве их три — обязательный
член с `"nullable": true` приходит как `Option<Box<T>>` без skip.

**Утверждение о пределе языка оказалось ложью.** Схема журнала записала, что у
JTD нет `nullable` и нет формы «present-but-null», и на этом основании
расхождение `removed.version` было подшито «recorded, not resolved». `nullable`
— штатный флаг RFC 8927, и соседняя схема того же дерева его уже применяет.
Исправлено на месте (`351a9594`), разрешимая правка — `BACKLOG.md` **B-078**.
Класс: проверять надо КАЖДОЕ такое утверждение; соседнее (B-056, «у JTD нет
64-битного целого») проверено и подтвердилось.

**Красное доказательство берётся там, где оно честно бывает.** Для порядка
ключей красного нет вовсе: против `HashMap` тест падал бы вероятностно, а
мерцающее красное — не доказательство. Красное дал КОМПИЛЯТОР: поле,
переданное в параметр `&BTreeMap`, до пасса не собиралось (E0308). Для снятия
crate-wide `allow(non_snake_case)` красное — clippy на 81 сайте, ровно по числу
измеренных camelCase-полей.

## Что решено НЕ делать, и это решение

**Печать суда (`seal`) не поставлена** — четвёртое сворачивание подряд.
Закрывает S7.

**B-078 не починен попутно** — перенос поля между формами меняет провод
журнала и задевает ветку слоя; это шаг со своим красным доказательством, а не
хвост чужой посадки.

**Независимая полоса (S1/S6) не шла параллельно фазе** — и это измерено, а не
осторожность: S6 заводит запись в `formats/REGISTRY.toml`, которая
перегенерирует `format_id/mod.rs` — файл, который держит дерево Ф4.2b.
Периметры записи пересекаются.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`** — до-эпоховые манифесты под `packages/`.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.

**Рост любого из них — находка.**

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **Ф4.2b-7, доменные типы по `x-rust-type`**.

**Замер сделан целиком, решение записано (Р22 в §7 плана), пакет нарезан:**
`cache/agents/sorted/F42B7-DOMAIN-TYPES/2026-08-17-cut-not-run-packet.md`.
Его можно отправлять воркеру как есть, сверив координаты с деревом.

Коротко, что в нём: у `x-rust-type` два плеча, и какое — решает ФОРМА
определения (псевдоним берёт аннотацию правой частью, структура/enum — именем);
четыре аннотации уже удовлетворены, корень журнала `Journal`→`JournalRecord` —
единственное расхождение имени, три скалярных фрагмента (`group`, `version`,
`timestamp`) не удовлетворены; аннотация `group` **сама дефектна** (называет
имя своего псевдонима вместо цели) и правится на `vibe_core::Group`; фрагмент
`timestamp` переводится на `"type": "timestamp"` (решение Р11).

**Радиус ВПЕРВЫЕ выходит за `generated/**`** — периметр обязан это назвать:
`crates/vibe-index/tests/wire_parity_journal.rs:42` (импорт `Journal`), плюс
`crates/vibe-wire` получает `vibe-core`, `semver`, `chrono`, то есть
`Cargo.toml` **и `Cargo.lock`**. Цикла нет, гейта на рёбра зависимостей нет —
оба факта измерены.

**Порядок посадки:** применить дифф → `cargo fmt --all` → `cargo xtask specmap`
→ **стейдж** → панель → коммит. Слайс, трогающий `generated/**`, садится
именно так: `check-codegen` не видит untracked.

## Неочевидные находки этой сессии (сверх документов)

**Пакет не проходит как аргумент.** `claudez -p "$(cat …)"` роняется на
`Argument list too long` уже при 34 КБ, и отказ приходит от ОС строкой
запускалки — в JSONL нет ни одного события, называющего пакет. Форма: копия в
корень worktree плюс короткий указатель с копией закрывающих клауз.

**Опасен не `cd`, а каждая команда после него.** Голый `cd` в начале
read-only замера увёл рабочий каталог, и следующей командой был `git apply`
диффа воркера — он ушёл в worktree. Спасла атомарность `git apply` (все хунки
отказали), а не дисциплина; три `cp` рядом такой защиты не имели.

**Периметр обязан называть и то, что правка СЛОМАЕТ.** Схлопывание коллекций
уронило тест, посаженный двумя шагами раньше; закрытый список его не назвал, и
воркеру пришлось выбирать между двумя инструкциями одного пакета.

**Команда замера разлёта окупается дважды.** `cargo check --workspace
--all-targets` не только называет сломанное — его молчание про остальные крейты
ДОКАЗАЛО свойство, которое пакет только утверждал прозой.

**Клауза про boot работает.** За шесть запусков ни один воркер не читал
статическую полосу (2186 строк): пакет говорит, какое чтение нужно, а какое он
сделал избыточным.

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-index/PROP-005`, `PROP-002`,
  `PROP-008`, `spec/common/PROP-029`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (семнадцать находок; у
  `f4-transform-radius` §5 стоит поправка о трёх ложных утверждениях),
  `SUBAGENT-LAUNCHERS.md` (§8 — **50** размеченных фактов;
  `SUBAGENT-MODE.toml` = `claudez`), `tasks/*.py`, `run/`.
- `formats/` — `REGISTRY.toml` (20 записей), `vocabularies.json` (18
  фрагментов), `hash_recipes/1.toml`, `breaks/001.md`.
- `schemas/` — семь CLI-отчётов в корне плюс `index/e1/` (пять) и
  `journal/e1/` (одна). `schemas/hello/` НЕ существует — дыра, которую пасс
  строгости теперь называет по имени в выводе прогона.
- `crates/` — 19 крейтов + `xtask`. Предмет ближайших шагов:
  **`xtask/src/codegen/`** — драйвер `mod.rs`, `layout`, `vocabulary`,
  `format_id`, и семь пассов: `postproc` (боксирование), `snake_case`,
  `ordered_maps`, `empty_policy`, `optional_shapes`, `strictness`,
  `open_vocabulary`; `crates/vibe-wire/src/generated/` (19 файлов),
  `crates/vibe-index/tests/wire_parity_*.rs` (шесть оракулов),
  `crates/vibe-wire/tests/` (три проверки артефакта).
- Корень: `BACKLOG.md`, `AUDIT.md`, `TASKS.md`, `NEXT-SESSION-PROMPT.md`,
  `specmap.json`, `conform.toml`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). `2026-08-06-01` (P1) — «ruled — re-judgement campaign pending»,
её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках и в ТЗ)

- **PROP-044 ратифицирован**; терминология §2b обязательна.
- **Схема описывает ПРОВОД.** Аннотации политики говорят о ФОРМАТЕ;
  боксирование, канонический порядок и snake_case — безусловные преобразования:
  у них ровно один законный ответ.
- **Слой ключится ДОМОМ схемы** и связывает весь слой целиком; дома называют
  своего владельца там же, где названы их каталоги.
- **Порядок пассов — закон:** формно-привязанные работают, пока файл ещё есть
  выход генератора; открытие словарей идёт последним.
- **Сшивка по МНОЖЕСТВУ значений, а не по имени** — имя минтит генератор.
- **У каждого пасса сторож счёта** — он делает пропуск объединений названным
  правилом, а не совпадением.
- **Обязательность ограничивает политику пустого:** у обязательной коллекции
  `omit` незаконен.
- **Каталог — проекция журнала**, починка идёт в одну сторону.
- **Ф4.3 садится рехетом (К4)**; периметр — 133 файла.
- **S1**: `RepoNotVisible`; «already exists» — позднее свидетельство.
  **S6**: свежесть сайдкаром; движок дисциплины не трогается.
- Допубликационный режим (D13). Делегирование по умолчанию; ревью, вердикты,
  спеки, планы и коммиты — никогда не делегируются. Раскатка только
  `cargo xtask mirror`. Никогда `git add -A`. Печать — только за проверенное.

## Последние коммиты (свежие сверху)

```
57ef3c1b docs(campaign): a rule that changes no bytes, and the one still unbuilt
0fd7ce2d feat(xtask): reader strictness comes from the format registry
ae3c8182 docs(campaign): three corrections the tree made to the step's inventory
351a9594 fix(schemas): the limit that made the gap unresolvable does not exist
e4b46885 feat(xtask): the optional shapes lose the box the generator adds
5c10f1b3 docs(launchers): a perimeter names what the change breaks
4b18e207 docs(campaign): the compiler measured what the prose asserted
ab9edff6 feat(xtask): an empty collection obeys the schema, not the generator
a61de3f8 docs(campaign): requiredness decides what an empty collection may do
17193dd8 feat(xtask): the wire's maps become the ordered kind
273499b2 docs(launchers): an empty output is a claim, not a proof
52d08d90 docs(campaign): the count was wrong and the rule outlived it
21b0ac94 feat(xtask): a rename survives only where it carries the wire
5f29b733 docs(launchers): what follows a stray cd is what writes
66793708 docs(campaign): the layer stitches on values, not on names
50b0aa35 feat(xtask): the schema decides which vocabularies open
42052926 docs(launchers): a big packet cannot travel as an argument
1bb14227 docs(wal): session-end checkpoint
d53e4aa6 docs(continue): cold-resume checkpoint
80acdd9d docs(handoff): the entry prompt starts from the vocabulary pass
4a23fc98 docs(launchers): a worktree hands the worker more than the packet accounted for
d592f622 docs(campaign): the transformation layer is cut against seven refutations
6d399715 docs(harvest): the transformation layer is sized against the tree, not the list
440c45fc refactor(xtask): the layout rules stop sharing a file with the driver
3c28481a docs(handoff): the entry prompt starts from the transformation layer
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста
cargo xtask specmap --check
cargo xtask check-codegen         # git diff против ИНДЕКСА — untracked не видит
cargo test -p xtask               # 165 тестов слоя преобразований
cargo test -p vibe-wire           # три проверки артефакта + полнота реестра
cargo test -p vibe-index --test wire_parity_entry --test wire_parity_by_name \
  --test wire_parity_inverted --test wire_parity_repomd --test wire_parity_journal
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
