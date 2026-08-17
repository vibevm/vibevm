# CONTINUE — cold-resume snapshot (2026-08-17, wind-down №27)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

**Блок Ф4.2b закрыт целиком — семь шагов из семи.** Слой преобразований
кодогенерации дорос до **девяти пассов**. Затем фаза Ф4.2c была замерена,
отсужена (Р24–Р27), нарезана на четыре шага, и два из них сели. Двенадцать
коммитов, три посадки кода, панель зелёная на каждой, `vibe check` не сдвинулся
ни разу.

Ценность сессии не только в коде. **Снята записанная проблема, которой не
существовало пять сессий:** «пин генератора не сходится с артефактами» —
бинарь отвечает `jtd-codegen 0.4.1`, и ОН ЖЕ пишет в шапку `for Rust v0.2.1`,
потому что это версия Rust-**таргета**. Две величины разных вещей сравнивались
как одна. Это тот же класс, что прошлосессионный «пустой вывод скрипта есть
доказательство»: **инструмент мерил не ту вещь**.

**Шлюз GLM лежал весь день** (HTTP 529, три пробы). Один делегированный пакет,
оба прогона убиты шлюзом; написанный воркером пасс хорош и оставлен, хвост
забран боссом — по калькуляции, а не по нетерпению.

## Где стоит работа

- Ветка `main`, дерево чистое, HEAD — сворачивающие коммиты этой сессии.
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`**,
  реальный код выхода 0 (последний прогон на посадке `dca804db`).
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых ОЖИДАЕМЫ и не
  двигались тридцать шесть посадок подряд.
- Судейство: **0 неосуждённых, 0 осиротевших**; 34 файла stale. Корпус 281
  файл, 13788 маркеров, 0 неразмеченных.
- Карта: 6061 спек-юнит / 1038 tagged / 976 рёбер / 0 подозрений / 0 сирот.
- Воркеров нет, `.wt/` пуст, ветки `wt/*` удалены. Два worktree под
  `~/.fractality/runs/**` — чужие, не трогать.
- Логи, пакеты и `meta.md` с вердиктом — в
  `C:\Users\olegc\git\v\cache\agents\sorted\F42B7-DOMAIN-TYPES\`. **Архив живёт
  ВНЕ чекаута** (§5 запускалок): путь `cache/agents/...` в тексте — это
  `git/v/cache/...`, а не подкаталог репозитория. Прошлый снимок дал повод
  искать его под корнем — искать не там.

## Блокер и действие человека

**Блокера нет.** За владельцем те же вопросы, ни один не держит главную полосу:
**S2** (org-права), **Ж8** (`--full` и исчезнувшая запись), **B-056** (у JTD нет
64-битного целого) и рядом **B-078** (разрешимая правка провода журнала).

Одно операционное, не блокирующее: **шлюз z.ai отдавал 529 весь день**. Если
делегирование понадобится — проверить пустой пробой до нарезки пакета.

## Что сделала эта сессия — по существу

| шаг | коммит | что построено |
|---|---|---|
| Ф4.2b-7 | `c2c240b3` | доменные типы по `x-rust-type`, оба плеча; **блок Ф4.2b закрыт** |
| — | `e8d8238f` | отказ `check-codegen` перестал знать одну причину из двух |
| Ф4.2c-1 | `95feb37f` | трейт-этаж: 74 типа получили `Debug, Clone, PartialEq, Eq` |
| Ф4.2c-2 | `dca804db` | имя варианта по `x-rust-variants`; `KindName0` → `KindSlashName` |

Плюс замер Ф4.2c (`d20ad5f2`), решения Р23–Р27 и нарезка (`a4c7b85c`,
`051d37c2`, `b54205d4`), три новых факта запускалок (`a01b39e8`), поправки по
факту в план (`9c27a1ff`, `64547bf5`, `7352052f`).

## Три вещи, которые эта сессия установила

**1. Инструмент может мерить не ту величину, и его вывод читают как ответ.**
Пин генератора «не сходился» пять сессий, потому что версия инструмента
сравнивалась с версией Rust-таргета. Настоящее сведение сильнее любой строки и
уже стояло в панели: зелёный `check-codegen` утверждает побайтовое равенство
артефактов сегодняшней эмиссии. Гейт на версию при этом **не построен
намеренно** — дом пина запрещает пересказывать число в дереве потребителя и
прямо объявляет, что CI сборку не принуждает.

**2. Пасс снимает РОВНО то, что обязан.** Доменные типы снимают импортные items,
осиротевшие ИХ подстановкой, — не «подчищают импорты». Открытие словаря снимает
серде-пару, а не всю строку derive. Один закон, два слоя.

**3. Дифф, прочитанный без прогона, есть свидетельство о намерении, а не о
поведении.** Воркер отдал пасс, чьи собственные тесты его реализации не
удовлетворяли: сопоставитель объявления не узнавал НИ ОДНОГО объявления (шесть
красных), а две формы вывода не пережили бы `cargo fmt --check`. Всё это
невидимо при чтении и мгновенно под `cargo test`.

## Что решено НЕ делать, и это решение

**Гейт версии генератора не строится** — см. выше; это записанная позиция дома
пина, а не забывчивость.

**Реэкспорт не режется на два шага** — орфанное правило не даёт: перенос метода
без реэкспорта оставил бы поведение в двух определениях, реэкспорт без переноса
не собрался бы вовсе.

**Печать суда (`seal`) не поставлена** — пятое сворачивание подряд. Закрывает S7.

**Независимая полоса (S1/S6) не шла параллельно фазе** — измерено: S6 заводит
запись в `formats/REGISTRY.toml`, а она перегенерирует `format_id/mod.rs`.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`** — до-эпоховые манифесты под `packages/`.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.

**Рост любого из них — находка.**

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **Ф4.2c-3: реэкспорт рукописных типов, слой
поведения в `vibe-wire`, рантаймное ребро и `--kind`. ОДНИМ коммитом.**

Замер сделан целиком —
[`harvest/f42c-reexport-radius.md`](campaigns/packages-2026-09/harvest/f42c-reexport-radius.md),
решения записаны как Р24–Р27 в §7 плана. Коротко, что в них:

- **Р24** — трейт-этаж эмитится безусловно (`Debug, Clone, PartialEq, Eq`),
  `Default` в него НЕ входит: «есть ли у типа осмысленное пустое значение» —
  суждение, а не форма, поэтому `Default` живёт рукописным impl'ом. **Сделано
  в Ф4.2c-1.**
- **Р25** — `ValueEnum` не переезжает: `clap` не выводит его по варианту с
  нагрузкой, а `PackageKind` открыт. `--kind` берёт строку и разбирается в
  открытый тип, ветка `Unknown` печатает то, чего требует Б.1. **Цена названа:
  shell-completion по флагу теряется.**
- **Р26** — методы и трейт-impl'ы переезжают В `vibe-wire`: орфанное правило.
  15 inherent-методов, `Display` ×2, `FromStr` ×1, плюс `Default`-impl'ы.
  `vibe-wire` перестаёт быть «только сгенерированным», и его шапка обязана
  сказать это тем же коммитом.
- **Р27** — имя варианта объявляется схемой картой по wire-значению. **Сделано
  в Ф4.2c-2.**

**Периметр Ф4.2c-3 обязан назвать и то, что сломает:**
`crates/vibe-index/src/types/entry/tests.rs` (281 строка, стоит на `PartialEq`
и `Debug`), 16 сайтов `::default()` по воркспейсу на семи подструктурах,
`crates/vibe-index/Cargo.toml` (dev → runtime, ПЛЮС комментарий, который
перестанет быть правдой), шапка `crates/vibe-wire/src/lib.rs`. Команда замера
разлёта — `cargo check --workspace --all-targets` — входит в пакет.

## Неочевидные находки этой сессии (сверх документов)

**Отказ шлюза — третий способ завершения прогона.** Не завершение и не kill:
`result`-событие приходит (процесс кончился, `-c` безопасен немедленно), но
несёт `terminal_reason: api_error` и `api_error_status: 529`, а harness печатает
голый exit 1. Забирает он ровно ХВОСТ — верификацию, которую пакет кладёт
последней.

**Умерший транспорт переоткрывает калькуляцию, а не переотправляет пакет.**
Провалившийся прогон меняет ОСТАВШУЮСЯ задачу, значит «делегировать или взять»
спрашивается заново про остаток.

**Слот пасса бывает вынужденным.** Трейт-этаж: `strictness` якорится на
нетронутую строку derive ⇒ после него; `open_vocabulary` обязан увидеть
расширенную ⇒ до него. Между ними ровно один слот.

**Охват переименования решает область видимости имени.** Имя ТИПА уникально в
файле ⇒ переименование файловое. Имя ВАРИАНТА принадлежит своему enum'у ⇒
только внутри объявления. Первую реализацию поймал тест, написанный по
настоящей форме объединения.

**Описание схемы едет в док-комментарий и проходит через те же переименования,
что и код.** Процитировать в описании имя, которое пасс переименует, — выпустить
док, называющий несуществующее.

**Отказ, называющий только авторскую схему, отправляет автора не в тот файл** —
общий фрагмент в неё ПОДСТАВЛЕН. Тот же дефект, что рецепт, чинящий не то;
починен дважды за день (у `check-codegen` и у доменных типов).

**Windows иногда держит сгенерированный файл замапленным** (`os error 1224`) —
это не логический отказ, повторить прогон.

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-index/PROP-005`, `PROP-002`,
  `PROP-008`, `spec/common/PROP-029`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (**восемнадцать** находок;
  у `f4-transform-radius` §5 и §10 стоят поправки, у `f4-codegen-mechanism` §7
  — тоже), `SUBAGENT-LAUNCHERS.md` (§8 — **53** размеченных факта;
  `SUBAGENT-MODE.toml` = `claudez`), `tasks/*.py`, `run/`.
- `formats/` — `REGISTRY.toml` (20 записей), `vocabularies.json` (18
  фрагментов), `hash_recipes/1.toml`, `breaks/001.md`.
- `schemas/` — семь CLI-отчётов в корне плюс `index/e1/` (пять) и
  `journal/e1/` (одна). `schemas/hello/` НЕ существует — дыра, которую пасс
  строгости называет по имени в выводе прогона.
- `crates/` — 19 крейтов + `xtask`. Предмет ближайших шагов:
  **`xtask/src/codegen/`** — драйвер `mod.rs`, `layout`, `vocabulary`,
  `format_id`, и девять пассов: `postproc` (боксирование), `snake_case`,
  `ordered_maps`, `empty_policy`, `optional_shapes`, `strictness`,
  `domain_types` (+ `rulings`, `variants`), `derive_floor`, `open_vocabulary`;
  `crates/vibe-wire/src/generated/` (19 файлов), `crates/vibe-wire/tests/`
  (четыре проверки артефакта), `crates/vibe-index/tests/wire_parity_*.rs`
  (пять оракулов), `crates/vibe-index/src/types/**` (предмет Ф4.2c-3).
- Корень: `BACKLOG.md`, `AUDIT.md`, `TASKS.md`, `NEXT-SESSION-PROMPT.md`,
  `specmap.json`, `conform.toml`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). `2026-08-06-01` (P1) — «ruled — re-judgement campaign pending»,
её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках и в ТЗ)

- **PROP-044 ратифицирован**; терминология §2b обязательна.
- **Схема описывает ПРОВОД.** Аннотации политики говорят о ФОРМАТЕ;
  боксирование, канонический порядок, snake_case и трейт-этаж — безусловные
  преобразования: у них ровно один законный ответ.
- **Генератор эмитит ФОРМУ и никогда ПОВЕДЕНИЕ** — эта линия провела границу
  трейт-этажа (`Default` осталась за ней).
- **Имя, которое выбирает человек, ОБЪЯВЛЯЮТ, а не выводят**; ключ — wire-значение.
- **Слой ключится ДОМОМ схемы** и связывает весь слой целиком.
- **Порядок пассов — закон**, и слот нового пасса выводится из ограничений.
- **Сшивка по МНОЖЕСТВУ значений, а не по имени** — имя минтит генератор.
- **У каждого пасса сторож** — счёт сайтов, постусловие или отказ.
- **Каталог — проекция журнала**, починка идёт в одну сторону.
- **Ф4.3 садится рехетом (К4)**; периметр — 133 файла.
- **S1**: `RepoNotVisible`; «already exists» — позднее свидетельство.
  **S6**: свежесть сайдкаром; движок дисциплины не трогается.
- Допубликационный режим (D13). Делегирование по умолчанию; ревью, вердикты,
  спеки, планы и коммиты — никогда не делегируются. Раскатка только
  `cargo xtask mirror`. Никогда `git add -A`. Печать — только за проверенное.

## Последние коммиты (свежие сверху)

```
7352052f docs(campaign): a variant's scope is its enum, and the test said so
dca804db feat(xtask): a variant's identifier is declared, never derived
64547bf5 docs(campaign): the pass had one slot and the neighbour was over-removing
95feb37f feat(xtask): every generated type carries the same trait floor
b54205d4 docs(campaign): the re-export is one commit because it cannot be two
051d37c2 docs(campaign): four rulings the re-export cannot be cut without
d20ad5f2 docs(harvest): a re-export moves the trait floor, not only the fields
9c27a1ff docs(campaign): two versions of two things were read as one
e8d8238f fix(xtask): the drift refusal names the cause that would launder itself
a01b39e8 docs(launchers): a dead gateway is neither a completion nor a kill
a4c7b85c docs(campaign): the annotation must resolve without the file it lands in
c2c240b3 feat(xtask): the schema names the Rust type its scalars bind
171c4dc8 docs(handoff): the entry prompt starts from the domain types
7805bf16 docs(continue): cold-resume checkpoint
2ea19cb3 docs(wal): session-end checkpoint
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
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста
cargo xtask specmap --check
cargo xtask check-codegen         # git diff против ИНДЕКСА — untracked не видит
cargo test -p xtask               # 188 тестов слоя преобразований
cargo test -p vibe-wire           # четыре проверки артефакта + полнота реестра
cargo test -p vibe-index --test wire_parity_entry --test wire_parity_by_name \
  --test wire_parity_inverted --test wire_parity_repomd --test wire_parity_journal \
  --no-fail-fast
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
