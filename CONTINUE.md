# CONTINUE — cold-resume snapshot (2026-08-16, wind-down №24)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

Сессия закрыла **Ф4.0, Ф4.1 и Ф4.2a**: генератор научился ходить по
подкаталогам и раскладывать выход по пути, общий словарь получил один дом с
транзитивной подстановкой, **все шесть схем каталога описаны**, и механизм
впервые в своей истории правит содержимое собственного выхода.

Двадцать пять коммитов, панель зелёная на каждой посадке, оба зеркала
синхронны на `3f56405`.

**Главное для решений — не число схем, а то, что у каждой есть ОРАКУЛ
ПРОВОДА.** Он строит полностью заполненный рукописный тип, гоняет через JSON
в сгенерированный и обратно, сравнивает значения — и потому ловит то, чего
не видно в диффе: пропущенное поле, пропущенное плечо объединения, потерю на
третьем уровне вложенности через общий фрагмент. Тринадцать красных
доказательств за фазу; часть добрана боссом после срезов, и добранные дважды
оказались сильнее заказанных.

Владелец не заблокирован ничем. Следующий вход — **остаток Ф4.2**: семь
преобразований из восьми (боксирование построено).

## Где стоит работа

- Ветка `main`, дерево чистое, `origin` ahead/behind = 0/0; HEAD `3f564059`.
  **Раскатано** (`cargo xtask mirror` — gitverse ok, github ok).
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`,
  реальный код выхода 0, 51 шаг** (прогон на последней кодовой посадке).
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых ОЖИДАЕМЫ и
  объяснены ниже; за двадцать шесть посадок подряд не сдвинулись.
- Судейство: **0 неосуждённых, 0 осиротевших**; 34 файла stale. Корпус 281
  файл, 13788 маркеров, 0 неразмеченных.
- Карта: 6061 спек-юнит / **1038** tagged / 976 рёбер / 0 подозрений / 0 сирот.
- Воркеров нет, `.wt/`-worktree'ов нет, дерево чисто. Два worktree под
  `~/.fractality/runs/**` — чужие, к кампании отношения не имеют.
- Логи, отчёты и `meta.md` с вердиктами ревью — в `cache/agents/sorted/{F4-CODEGEN,
  F4-WIRE-CENSUS,F4-VOCAB,F40-CODEGEN-PATHS,F41A-VOCAB,F41A2-TRANSITIVE,
  F41B1-ENTRY,F41B2-SCHEMAS,F42A-BOXING,F41B3-JOURNAL}/`.

## Блокер и действие человека

**Блокера нет.** За владельцем три вопроса, ни один не держит главную полосу:

1. **S2** — переименование живых `_`-репозиториев в org `vibespecs` (org-права).
2. **Ж8** — что означает `--full` для записи, которую скан больше не видит.
3. **B-056** (новое) — язык схем не выражает тип, которым пишет наш писатель:
   у JTD нет 64-битного целого, а `Repomd.File.size` объявлен `u64`. Значит
   наш писатель способен породить документ, который наша же схема объявляет
   невалидным. Три выхода измерены и записаны в `BACKLOG.md`; каждый трогает
   либо опубликованный провод, либо продуктовый тип.

## Что сделала эта сессия — по существу, а не по списку

**Замер под Ф4 опроверг шесть записанных утверждений, и три из шести
изменили ПОРЯДОК шагов, а не их объём.** Самое дорогое: схемы Ф4.1 легли бы в
подкаталоги, которых сканер не видит (`read_dir` + `is_file`), — и ничего бы
не покраснело, потому что фаза, не сгенерировавшая ничего, не даёт диффа, а
`check-codegen` сравнивает именно дифф. Зелёная панель поверх пустоты была в
одном замере от нас.

**Механизм трижды пришлось положить раньше того, что его требует** — и каждый
раз это устанавливалось пробой, а не рассуждением: спуск в подкаталоги до
схем; транзитивная подстановка до составных фрагментов; боксирование до
журнала. Последнее замкнулось буквально: проба уронила `large_enum_variant` на
настоящей записи, преобразование село, и на настоящем журнале линт не
сработал.

**G9 оказался нарушен ДО фазы, а не ею.** `package_kind` был дословно записан
в двух существующих схемах, а межфайловых ссылок у JTD нет — `ref` разрешается
только внутри документа, и висячая ссылка роняет генератор ПАНИКОЙ без
диагностики. Разделение словаря стало восьмым механизмом нашего слоя, и он же
валидирует ссылки сам, до вызова бинаря.

**Словарей-дефектов оказалось пять, а не один.** К записанному `BindingSite`
добавились `PackageKind`, `NamingConvention`, `DeliveryMode`, `SourceKind`; и
у `PackageKind` паритет-теста **нет** вопреки записанному — три теста рядом с
ним проверяют только его самого.

**Язык схем дважды не выразил того, чем пишет код:** 64-битное целое (B-056) и
кортеж у `Renamed`. Оба записаны честно — с тем, что схема говорит, чего
сказать не может, и где живёт проверка.

## Что решено НЕ делать, и это решение, а не забывчивость

**Печать суда (`seal`) не поставлена** — как и на прошлом сворачивании.
`spec/modules/vibe-index/PROP-005-package-index.md` несёт двинувшиеся факты; не
напечатать стоит одной проверки в будущем, напечатать ошибочно — ложного
«проверено», и прецедент отката есть (`498e8c8b`). Закрывает S7.

**B-056 не решён на месте.** Каждый выход трогает либо провод, либо
продуктовый тип — это решение владельца, а не слайса, который его нашёл.

**Аннотации политики написаны, но ничем не читаются.** `x-vocabulary`,
`x-empty`, `x-default`, `x-rust-type` живут в схемах и словнике; их
потребитель — остаток Ф4.2. Это не машинерия впрок: схема есть единственный
законный дом политики, и написать схему дважды дороже. Но факт, что механизма
за ними пока нет, назван в каждом отчёте — и назван здесь.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`** — до-эпоховые манифесты под `packages/`.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.

**Рост любого из них — находка.** Регрессию поймал именно этот счётчик.

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **остаток Ф4.2**: семь преобразований из
восьми.

**Первое дело — РАСКОЛ.** `xtask/src/codegen/mod.rs` — **586 из 600**, а
каждое преобразование требует проводки в `generate_into`. Прецедент раскола в
этом же каталоге есть дважды.

**Страховочная сеть уже стоит:** шесть оракулов в
`crates/vibe-index/tests/wire_parity_*.rs` сравнивают ПРОВОД. Преобразования
меняют форму типов в Rust и обязаны не трогать байты — оракулы это и меряют,
и переживают все преобразования по построению.

**Порядок посадки:** применить дифф → `cargo fmt --all` → `cargo xtask specmap`
→ панель → коммит. Слайс, трогающий `generated/**`, садится **стейдж → панель
→ коммит**: `check-codegen` не видит untracked.

## Неочевидные находки этой сессии (сверх документов)

**Фикстура, которая не может упасть, — на боссовой стороне тоже.** Первый
прогон clippy по сгенерированному объединению дал ноль линтов, и принять это
за ответ значило бы сертифицировать регрессию: нагрузка пробы была
четырёхполевой заглушкой. На нагрузке настоящего размера линт сработал.

**`cargo fmt` без `--check` выходит с нулём и когда результат не чист.** Файл,
вырезанный текстом, начался с пустой строки; `cargo fmt --all` до него дошёл,
пустоту не убрал, а `--check` на ней упал — панель умерла на третьем шаге из
51. Лечится шапкой `//!` первой строкой.

**Отчёт, написанный до гейтов, спасается от среза — и тем же движением может
соврать.** Второй срез пощадил отчёт, но тот утверждал «clippy exit 0», а
воркер до clippy не дошёл. Перемер дал 101. Правило уточнено: писать раньше,
но каждую строку приёмки начинать словом PENDING.

**Убитая задача не убивает воркера** — подтверждено дважды за сессию, логи
росли на 62 и 5 КБ ПОСЛЕ «убийства».

**Закрытый периметр обязан предусматривать файл, который предписанный им же
раскол создаст** — иначе два пункта пакета сталкиваются. Воркер это заметил
сам и попросил вето.

**Специмап индексирует JTD-схемы рекурсивно** — новая схема даёт запись карты,
а определения внутри схемы дают свои. Отсюда: словник в `formats/` вне его
периметра, и переименованием это не лечится (проверено; `check-codegen` от
переименования падает — имя словника несущее).

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-index/PROP-005`, `PROP-002`,
  `PROP-008`, `spec/common/PROP-029`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (**шестнадцать** находок:
  четыре добавлены этой сессией под Ф4), `SUBAGENT-LAUNCHERS.md` (§8 — **45**
  размеченных фактов; `SUBAGENT-MODE.toml` = `claudez`), `tasks/*.py`, `run/`.
- `formats/` — `REGISTRY.toml` (20 записей), **`vocabularies.json` (18
  фрагментов, новое)**, `hash_recipes/1.toml`, `breaks/001.md`.
- `schemas/` — семь CLI-отчётов в корне плюс **`index/e1/` (пять схем) и
  `journal/e1/` (одна)**.
- `crates/` — 19 крейтов + `xtask`. Предмет ближайших шагов:
  `xtask/src/codegen/` (`mod.rs` 586, `format_id.rs` 245, `vocabulary.rs` 427
  + два файла тестов, `postproc.rs` 193 + тесты), `crates/vibe-wire/src/generated/`
  (19 файлов `mod.rs`, 10 подмодулей верхнего уровня),
  `crates/vibe-index/tests/wire_parity_*.rs` (шесть оракулов).
- Корень: `BACKLOG.md` (**+B-056**), `AUDIT.md`, `TASKS.md`,
  `NEXT-SESSION-PROMPT.md`, `specmap.json`, `conform.toml`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). `2026-08-06-01` (P1) — «ruled — re-judgement campaign pending»,
её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках и в ТЗ)

- **PROP-044 ратифицирован**; терминология §2b обязательна.
- **Каталог — проекция журнала**, починка идёт в одну сторону.
- **Словарь живёт в ОДНОМ доме**, подстановку делает наш слой, и она
  транзитивна: фрагмент объявляет свои зависимости, резолвер замыкает и
  отказывает на цикле.
- **Схема описывает ПРОВОД.** Боксирование — деталь представления в Rust и
  поэтому НЕ аннотация схемы, а безусловное преобразование генератора.
- **Маршрут выхода ключится ДОМОМ схемы и её путём**, а не стемом: каталог
  несёт эпоху, и `e1/entry` с будущим `e2/entry` обязаны остаться разными
  модулями.
- **Оракул сравнивает провод, а не типы** — типы расходятся намеренно до
  конца Ф4.2.
- **Ф4.3 садится рехетом (К4)**, целевое правило К2 «реестр + маркер» —
  именованный дефер с триггером «посадка Ф4.2»; периметр — 133 файла.
- **S1**: `RepoNotVisible`; «already exists» — позднее свидетельство.
- **S6**: свежесть сайдкаром; движок дисциплины не трогается.
- Допубликационный режим (D13). Делегирование по умолчанию; ревью, вердикты,
  спеки, планы и коммиты — никогда не делегируются. Раскатка только
  `cargo xtask mirror`. Никогда `git add -A`. Печать — только за проверенное.

## Последние коммиты (свежие сверху)

```
3f564059 docs(campaign): six schemas of six, and the rule the second kill sharpened
e0a2248c feat(index): the journal's eleven events get a schema — six of six
06875323 feat(xtask): the generator starts editing what it generates
b33db823 docs(campaign): the journal waits for boxing, and the reason was measured
456c4ea9 feat(index): the catalog manifest gets a schema, and JTD shows its floor
b000e51f feat(index): three catalog surfaces get schemas, each with its own oracle
33b4afff docs(launchers): write the self-verify from what a change touches
a0538f84 feat(index): the entry schema exists, and an oracle proves it complete
eb96dc24 docs(campaign): transcribing thirty fields needs an oracle, not a reader
9d8b3201 docs(launchers): a closed perimeter must anticipate the split it orders
71e94e8f feat(xtask): a shared fragment brings its own dependencies
dd9744c9 docs(campaign): sharing has to reach composites, not just word lists
c7faa208 feat(xtask): a wire vocabulary gets one home and the layer puts it there
890ce966 docs(campaign): the schemas cannot precede the references they use
ecab4857 docs(campaign): the first step of phase 4 records what it cost
f1ba88e0 feat(xtask): a schema whose name cannot be a module is refused by name
33b9ce59 feat(xtask): codegen descends into schema directories and mirrors them
c715a73b docs(campaign): the vocabulary gets one home and our layer puts it there
b556e06c docs(harvest): the one-schema-per-vocabulary law is already broken
9c1ce20d refactor(xtask): the registry emitter stops sharing a file with the driver
a7a276f1 docs(campaign): phase 4 is cut against the tree, not against the record
450fa1e8 docs(harvest): four more dictionaries agree only by luck
c01c1599 docs(harvest): the whitelist becomes a predicate, not a list of names
004c13c1 docs(harvest): the schema scan cannot see where the registry points
51006ecf docs(harvest): the probe that read clean could not have failed
d25f954a docs(harvest): the generator's shape is measured, not assumed
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста
cargo xtask specmap --check
cargo xtask check-codegen         # git diff против ИНДЕКСА — untracked не видит
cargo test -p vibe-index --test wire_parity_entry --test wire_parity_by_name \
  --test wire_parity_inverted --test wire_parity_repomd --test wire_parity_journal
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
