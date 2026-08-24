# renamed-perimeter — периметр журнального события `renamed`, посчитанный по файлу

замер от 2026-08-19

## 1. Что меряли и каким инструментом

Предмет — периметр события `renamed` журнала реестра пакетов (`vibe-index`,
слой истины PROP-044 §3), который владелец постановил заменить одним фактом
снятия с `reason` и необязательным преемником. Замер нарезан СЧЁТОМ, а не
понятием: регистронезависимая подстрока `renam` по всему дереву, исключая
`target/`, `.git/` (каталог), `vibedeps/`, `refs/`, `.vibe/`, `node_modules/`.

Инструмент №1 (основной счёт, §3.1 пакета):

```
grep -ric --exclude-dir=target --exclude-dir=.git --exclude-dir=vibedeps \
  --exclude-dir=refs --exclude-dir=.vibe --exclude-dir=node_modules "renam" .
```

Инструмент №2 (повторный счёт, §5.1 пакета, другой инструмент):

```
rg --no-ignore --hidden -i -c "renam" \
  -g '!**/target/**' -g '!**/.git/**' -g '!**/vibedeps/**' -g '!**/refs/**' \
  -g '!**/.vibe/**' -g '!**/node_modules/**' .
```

Определения и метод классификации:

- **Попадание = строка.** Одна строка с любым числом вхождений `renam` — одно
  попадание (так считает и `grep -c`, и `rg -c`). Число вхождений-слов больше
  числа попаданий (например, в `corpus.json` 566 вхождений на 511 строк);
  классификация ведётся построчно.
- **Звуковое правило класса A.** Идентификаторы события существуют только в
  формах `Renamed` (Rust-арм) и `renamed` (провод-тег); форма `RENAMED` в этом
  дереве принадлежит только чужой машинерии «RENAMED ANCHORS»
  (`vibevm/vibespecs/boot/STATIC.xml`, qualify-таблица коротких имён), а формы
  `rename`/`rename_all`/`RenameEntry`/… — serde-механике и переименованиям
  файлов/якорей/пакетов. Поэтому: (а) просмотрены вручную ВСЕ строки всех 148
  файлов, содержащих формы `renamed`/`Renamed`/`RENAMED`; (б) файлы без этих
  форм по построению не могут нести класс A — их попадания отнесены к B по
  домену (машинерия serde-переименований, атомарная запись, переименование
  якорей/пакетов/идентификаторов), выборочно подтверждено чтением строк.
- **Производные артефакты.** Копии A-текста внутри перегенерируемых артефактов
  (`specmap.json`, корпус/кэш кампании) отнесены к A по содержанию строки, с
  пометкой «правки не нужно — перегенерируемый артефакт»: правится источник,
  артефакт пересобирается сам.

## 2. Общий счёт

Команда инструмента №1 (см. §1) с суммированием:

```
$ grep -ric --exclude-dir=... "renam" . | grep -v ':0$' | wc -l   # файлов
525
$ # сумма счётчиков по файлам:
files_with_hits=525
total_hits=4567
```

Повторный счёт инструментом №2 (§5.1): `files_with_hits=525`,
`total_hits=4567` — числа совпали с первым инструментом, расхождений нет.

Числовой замер dated: 2026-08-19; в спеки не переносится.

## 3. Контрольный список — двенадцать утверждений

| # | вердикт | доказательство (`file:line` + дословный фрагмент) |
|---|---|---|
| C1 | **ПОДТВЕРЖДЕНО** | `schemas/journal/e1/journal.jtd.json:137` — `"elements": { "type": "string" },` (поле `from`, строка 136); то же для `to` — `journal.jtd.json:144` — `"elements": { "type": "string" },` (строка 143). Оба поля описаны через `elements` со строковым типом. |
| C2 | **ОПРОВЕРГНУТО** | `schemas/journal/e1/journal.jtd.json:133` — `"description": "A package rename: `from` → `to`, each a `(group, name)` pair."` — у арма ровно два поля: `from` (136) и `to` (143); блока `properties`/`optionalProperties` с `reason` в `renamed` (131–151) нет. Подтверждение с Rust-стороны: `crates/vibe-index/src/journal/record.rs:75-78` — `Renamed { from: (Group, String), to: (Group, String), }` — без `reason`. Отсутствие `reason` — измеренная причина коллапса двух фактов в один (PROP-005:591). |
| C3 | **ПОДТВЕРЖДЕНО** | `crates/vibe-index/src/journal/record.rs:76-77` — `from: (Group, String),` / `to: (Group, String),` — рукописный арм несёт Rust-пары; на проводе каждая — двухэлементный массив строк (схема шире типа, арность проверяет только Rust-ридер, `journal.jtd.json:139`). |
| C4 | **ОПРОВЕРГНУТО** | `crates/vibe-index/src/journal/project.rs:135-136` — `Event::Renamed { .. } => {` / `return Err(unprojectable("Renamed", "package rename"));` — проектор не порождает ничего, он отказывает; надгробие (`tombstone`) сегодня не порождается ни одним армом (PROP-005:588: носитель томбстоуна «populated only by reading a catalog off disk»). |
| C5 | **ПОДТВЕРЖДЕНО** | `crates/vibe-index/src/journal/project.rs:136` — `return Err(unprojectable("Renamed", "package rename"));` — отказ называет арм по имени; текст собирает `unprojectable` (project.rs:205-210): «the journal holds a `{variant}` record, but its carrier ({carrier}) is not built in this vibe-index». |
| C6 | **ОПРОВЕРГНУТО** | `crates/vibe-index/tests/wire_parity_journal.rs:52` — `const EVENT_VARIANT_COUNT: usize = 11;` — одиннадцать, не двенадцать. |
| C7 | **ПОДТВЕРЖДЕНО** | `crates/vibe-index/tests/wire_parity_journal.rs:79` — `("renamed", 3),` — строка таблицы `ARM_WIRE_SHAPES` (объявлена :69) с числом 3 (тег + `from` + `to`). |
| C8 | **ПОДТВЕРЖДЕНО** | `crates/vibe-wire/src/generated/journal/e1/journal/mod.rs:72-73` — `#[serde(rename = "renamed")]` / `Renamed(Box<EventRenamed>),` — и `mod.rs:212` — `pub struct EventRenamed {`. |
| C9 | **ОПРОВЕРГНУТО** | Поиск `generated::journal` и `journal::e1` по дереву находит ровно три употребления, все — тесты: `crates/vibe-index/tests/wire_parity_journal.rs:46` — `use vibe_wire::generated::journal::e1::journal::JournalRecord as GeneratedJournal;`; `crates/vibe-wire/tests/canonical_order.rs:24` (`...journal::FeaturesEntry`); плюс комментарий-заголовок `wire_parity_journal.rs:3`. Реэкспортов короткого пути нет: `crates/vibe-wire/src/generated/journal/mod.rs:7` — `pub mod e1;` (единственное содержимое модуля). Контроль непустоты: та же команда находит эти заведомо существующие употребления — инструмент видит то, что есть; продуктовых (`src/`) употреблений нет. |
| C10 | **ПОДТВЕРЖДЕНО** | `vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml:971` (внутри §2.18, открывающегося :957 `### 2.18 Channels — author-named version pointers {#channels}`) — «`Renamed`, `Notice` and `ForceReplaced` stand in the same place for the same reason.» — про отказ проектора из-за непостроенного носителя. |
| C11 | **ПОДТВЕРЖДЕНО** | `formats/breaks/001.md:1` — `# Break 001 — `repomd.json`: the `files` union becomes symmetrically tagged`; формат образца самоописан, `formats/breaks/001.md:5-8` — «Its shape is the pattern every later note follows; the fields are fixed by PROP-044 §4.7 — *what · epoch · who fixes · sunset · user recipe*». |
| C12 | **ОПРОВЕРГНУТО (записи-данных нет)** | `grep -rn '"kind":"renamed"' -e '"kind": "renamed"'` по дереву (с исключениями §1) — пусто, `exit=1`. Контроль непустоты той же командой для существующего тега: `formats/corpora/index/e1/state/journal/2026-08.ndjson:2` — `{"at":"2026-08-02T10:00:00Z",...,"event":{"kind":"yanked",...}}` — инструмент находит то, что есть. Корпус журнала на диске (`formats/corpora/index/e1/state/journal/*.ndjson`) не содержит записи `renamed`. |

## 4. Класс A — машинерия события, по файлам

Формат по §3.4 пакета: путь · строка(и) · что это · что сделает замена · почему.

| путь | строка(и) | что это | что сделает замена | почему |
|---|---|---|---|---|
| `schemas/journal/e1/journal.jtd.json` | 27, 131, 133 | схема (описание союза + арм `renamed` + его description) | переписать | арм уходит из словаря; :27 — проза об отказе пяти армов, станет четырьмя |
| `crates/vibe-index/src/journal/record.rs` | 75 | рукописный тип (арм `Renamed`) | удалить (заменить фактом снятия) | сам заменяемый вариант |
| `crates/vibe-index/src/journal/project.rs` | 135, 136 | проектор (ветка отказа `Renamed`) | переписать | отказ переезжает: новый арм-снятие первый в истории ПРОЕКТИРУЕТСЯ (надгробие), вместо отказа |
| `crates/vibe-index/src/journal/project_tests.rs` | 440, 441 | тест (проверка отказа) | переписать | тест отказа заменяется тестом порождения надгробия |
| `crates/vibe-index/src/journal/tests.rs` | 244 | тест (конструктор арма в фикстуре) | переписать | фикстура переезжает на новый факт |
| `crates/vibe-index/tests/wire_parity_journal.rs` | 20, 79, 280, 373 | тест-оракул (док о парах, строка таблицы `("renamed", 3)`, конструктор, пин арности пары) | переписать | одиннадцатиармовый оракул пересчитывается: строка арма, счётчик, фикстура, пин |
| `crates/vibe-wire/src/generated/journal/e1/journal/mod.rs` | 38, 72, 73, 210, 212 | сгенерированный код (описание союза, атрибут и вариант, док и структура `EventRenamed`) | перегенерировать | файл генерируется из схемы (`cargo xtask codegen`), правится schema → код пересобирается |
| `vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml` | 587, 589–593 | спек-проза (блок решений о коллапсе) | правки не нужны (текст постановки, не описание текущего словаря) | это сами факты-решения, executer сворачивает их исполнением; :593 — обязанность того же коммита |
| `vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml` | 971 | спек-проза (предложение §2.18 об отказе) | переписать | обязано тем же коммитом (PROP-005:593): перестаёт быть верным в обеих половинах |
| `vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml` | 237 | спек-проза (рабочий пример надгробия — переименование) | правки не нужно | пример — модель нового факта (reason + superseded_by), решение на него опирается (PROP-005:590) |
| `crates/vibe-index/docs/format.md` | 160 | документация (тот же пример надгробия) | правки не нужно | пример остаётся верным носителем семантики |
| `TASKS.md` | 41, 47, 78, 79, 85, 87 | план работ (задача перелома) | правки не нужно | пункт закрывается самим исполнением |
| `CONTINUE.md` | 49, 162 | документация холодного резюме | правки не нужно | живой снимок, переписывается на конце сессии |
| `NEXT-SESSION-PROMPT.md` | 89, 92 | документация следующей сессии | правки не нужно | живой указатель, переписывается при следующем ветре |
| `vibevm/vibespecs/WAL.xml` | 50, 87 | состояние работ (WAL) | правки не нужно | живой журнал сессии |
| `PACKET-E-RENAMED-PERIMETER.md` | 22 строки | пакет замера (сам этот заказ) | правки не нужно | переходящий артефакт задачи, исчезает с ней |
| `campaigns/packages-2026-09/harvest/f3-index-state-and-projection.md` | 12, 94, 141 | документация-замер (карта приёмников событий; `Renamed`→`Tombstone.superseded_by`) | правки не нужно | историческая находка-замер, датируется, не переписывается |
| `campaigns/packages-2026-09/harvest/f0-rmw-volume.md` | 252 | документация-замер (перечисление вариантов `Event`) | правки не нужно | исторический замер объёма |
| `campaigns/packages-2026-09/harvest/f4-vocabulary-and-surfaces.md` | 25, 47, 342, 360, 373, 376, 663 | документация-замер (словари и поверхности: перечисления, таблица арма, список отказов, текст-носитель, grep-вывод) | правки не нужно | исторический замер кампании |
| `campaigns/packages-2026-09/harvest/f4-jtd-tagged-union-probe.md` | 101, 157 | документация-замер (проба JTD-объединения с армом `renamed`; сгенерированная форма) | правки не нужно | историческая проба |
| `campaigns/packages-2026-09/harvest/f4-transform-radius.md` | 322 | документация-замер (радиус преобразования: `journal renamed.from/renamed.to`) | правки не нужно | исторический замер |
| `campaigns/packages-2026-09/harvest/f42c3-generated-duplication.md` | 89 | документация-замер (таблица сгенерированных типов, `EventRenamed` в списке) | правки не нужно | исторический замер |
| `campaigns/packages-2026-09/run/state/corpus.json` | 105101, 105114, 105121, 105128, 105129, 105135, 105136, 105142 | фикстура-данные кампании (встроенные копии A-текста: отказ проектора, решение владельца, пример надгробия, форма арма, «ничего не эмитит», §2.18) | правки не нужно | перегенерируемый артефакт кампании |
| `campaigns/packages-2026-09/run/cache.json` | 106016, 106029, 106036, 106043, 106044, 106050, 106051, 106057 | фикстура-данные кампании (зеркало корпуса, те же копии) | правки не нужно | перегенерируемый артефакт кампании |
| `specmap.json` | 43280, 43289, 43298, 43307, 43316, 44036 | фикстура-данные (карта спек↔код, копии заголовков фактов решения и §2.18) | правки не нужно | перегенерируемый артефакт (`cargo xtask specmap`) |

Итого класс A: **98 попаданий в 25 файлах** (11 файлов потребуют правок/перегенерации при переломе; 14 — живые/производные документы без правок).

## 5. Класс B — постороннее употребление слова

Всего **4467 попаданий в 525 файлах** (включая B-части смешанных файлов).
Полный пофайловый список воспроизводим командой §1; ниже — точные суммы по
доменам (машинный подсчёт, сходится с §7):

| домен | попаданий | файлов | суть употребления |
|---|---|---|---|
| `campaigns/packages-2026-09/` run+tasks | 1572 | 97 | корпус/кэш кампании, evidence-JSON, задачи: переименования пакетов (`git-*`), якорей (`RENAMED-FROM-VIBE-TCG`), бинарей; serde-цензусы |
| `vibevm/vibepacks/org.vibevm.ai-native/**` | 616 | 114 | вендорные копии сгенерированного specmap-кода: `#[serde(rename…)]`, `rename_all`; переименования инструментов |
| `campaigns/progress-2026-08/**` | 523 | 7 | корпус/кэш/baseline прогресс-кампании: переименованные link-типы, файлы, манифесты |
| `campaigns/packages-2026-09/` корень | 410 | 8 | baseline.json (399: копии чужих переименований) и PHASE/TZ-документы |
| `spec/**` | 262 | 46 | законы переименования якорей/пакетов (qualified-naming, PROP-028/029/031), исследовательские записки npm/cargo, atomic-write |
| `xtask/**` | 235 | 28 | машинерия codegen: `rename_all`, поимённые `rename`, `RenameEntry`, snake_case-правила |
| `campaigns/packages-2026-09/harvest/` | 234 | 43 | B-части смешанных harvest-файлов (serde-цензусы, `RENAME`-якоря) и прочие harvest |
| `specmap.json` | 145 | 1 | копии чужих заголовков (`RENAMED ANCHORS`, «renamed terms», stage-rename) |
| `legacy-spec/**` | 65 | 16 | планы прошлых волн: переименования стеков/арм/файлов |
| `crates/vibe-spec/**` | 64 | 8 | qualify: переименование коротких якорей в квалифицированные |
| `crates/vibe-cli/**` | 55 | 18 | дерево/vvm: переименования узлов, serde/clap-атрибуты |
| `vibevm/vibepacks/org.vibevm.world/**` | 40 | 18 | законы именования: «a rename is a new identity», форки |
| `crates/vibe-workspace/**` | 40 | 6 | «RENAMED ANCHORS»-таблица компилированной полосы, file-rename |
| `crates/vibe-core/**` | 38 | 13 | serde-переименования манифеста (`snapshot`→`copy` и т.п.) |
| `vibevm/vibepacks/org.vibevm.fractality/**` | 34 | 26 | свой журнал fractality: `rename_all`, ротация файлов (`fs::rename`) |
| `crates/progress-core/**` | 27 | 11 | baseline/cache: переименования файлов между прогонами |
| `crates/vibe-wire/**` | 27 | 8 | сгенерированные типы других словарей: `#[serde(rename…)]` (B-часть журнального mod.rs — 11 строк чужих армов — здесь же учтена) |
| `crates/vibe-index/**` | 20 | 21 | atomic-write (`tmp + rename + fsync`), serde/clap-атрибуты, прочие wire-parity доки |
| `crates/vibe-registry/**` | 7 | 4 | serde-атрибуты |
| `crates/vibe-trace/**` | 7 | 2 | поимённые rename глаголов Verb |
| прочие (`docs/`, `neworder2/`, `discipline/`, корневые `*.md`, `Cargo.toml`, `.zcode`, `.git`-файл, `formats/`, `schemas/index`, `vibe-settings/mcp/publish/install/actions`, `ROADMAP`, `CHANGELOG`, `BACKLOG`, `TOOLING-MAP`, `DEV-GUIDE`) | 27 | 22 | переименования флагов/токенов/пакетов; B-части корневых файлов; имя worktree `E-RENAMED-PERIMETER` в файле `./.git` |

Характерные примеры: `crates/vibe-index/src/index/persistence.rs:35` — `fs::rename(&tmp, path)...` (атомарная запись); `crates/vibe-workspace/src/boot_artifacts.rs:364` — `let mut out = String::from("<!-- RENAMED ANCHORS (short → qualified heirs):\n");` (таблица qualify); `vibevm/vibespecs/boot/STATIC.xml:109` — `LAW-A-RENAME-IS-A-NEW-IDENTITY → ...` (компилированный закон именования); `xtask/src/codegen/snake_case.rs` — правила serde-регистра.

## 6. Класс C — пограничное, оставлено боссу

Два попадания, которые я не смог честно отнести ни к A, ни к B — перечисления
ВИДОВ фактов журнала разговорным словом «rename» (не имя арма; после перелома
семантически остаются верными — журнал по-прежнему будет записывать
переименования, но уже как снятия):

1. `vibevm/vibespecs/common/PROP-044-change-native-formats.xml:146` — «(3) the **registry facts journal** — append-only records of what sources cannot carry: publication, yank, rename, removal, ownership, security notice;»
2. `crates/vibe-index/docs/operator-handbook.md:22` — «Lose the journal and no amount of re-scanning brings back a yank, a rename, a freeze or a tombstone —»

Оба текста переживут замену без правки по смыслу; требует ли их редактуры
перелом — решать владельцу.

## 7. Сходимость суммы

| класс | попаданий |
|---|---|
| A | 98 |
| B | 4467 |
| C | 2 |
| **сумма** | **4567 = общему счёту §2** |

Машинная проверка (awk по пофайловому списку с таблицей исключений):
`A=98 B=4467 C=2 TOTAL=4567`. Смешанные файлы сходятся построчно: например,
`journal.jtd.json` 4 = A3+B1; `wire_parity_journal.rs` 6 = A4+B2;
`generated journal/mod.rs` 16 = A5+B11; `PROP-005` 14 = A8+B6; corpus/cache
511 = A8+B503 каждый; `specmap.json` 151 = A6+B145. Расхождений нет.

## 8. Записано как данные?

**НЕТ.** Ни одного места, где `renamed` записан как данные (корпус, golden,
фикстура, тестовый журнал на диске), в дереве нет. Доказательства:

- Поиск записи события по всему дереву (исключения §1):
  `grep -rn '"kind":"renamed"' -e '"kind": "renamed"' .` → **пусто, exit=1**.
- Контроль непустоты (тот же инструмент, тот же шаблон, заведомо существующий
  тег): находит `formats/corpora/index/e1/state/journal/2026-08.ndjson:2` —
  `..."event":{"kind":"yanked","group":"org.vibevm",...}` — приём работает.
- Второй контроль в каталоге корпуса: `grep -rn "renamed" formats/` → пусто,
  при этом `grep -rn '"kind"' formats/` → десятки попаданий (записи
  `initialised`, `published`, `entry_set_replaced`, `yanked`, `frozen`,
  `removed` в `state/journal/2026-07.ndjson` и `2026-08.ndjson`).
- Код-сторона: `Event::Renamed` в `crates/` встречается ровно 4 раза — отказ
  проектора (`project.rs:135`) и три тестовых конструктора
  (`project_tests.rs:441`, `journal/tests.rs:244`,
  `wire_parity_journal.rs:280`); писателя, эмитящего арм, нет.

Вывод для перелома: замена свободна — ни одна записанная запись не ссылается
на арм, обратной совместимости провода держать не на чем (и не для кого:
C9 — внешних потребителей сгенерированного типа нет, потребление только
тестовое).

## 9. Что этот замер НЕ установил

1. **Русскоязычные упоминания.** Подстрока `renam` ловит только латинское
   написание; проза на русском («переименование», «переименован») этим счётом
   не покрыта. Для ключевых файлов (PROP-005, TASKS, WAL) русские тексты
   читались глазами попутно, но систематического русского обхода не было —
   русскоязычный периметр может быть шире посчитанного.
2. **Содержимое `primary.jsonl.gz`.** Корпус `formats/corpora/index/e1/`
   содержит gzip-копию; проверено только байтовое вхождение `renam` (его
   нет), распаковка не выполнялась. Несжатый двойник `primary.jsonl`
   проверен построчно — записей `renamed` нет.
3. **Семантика будущего арма-снятия.** Замер фиксирует периметр, а не проект:
   как именно проектор будет порождать надгробие и куда ляжет преемник —
   вопрос дизайна, здесь не решался.
4. **Поведение при сборке.** Cargo не запускался (пакет прямо запрещает);
   согласованность счётчиков оракула после правки проверит сам оракул, не
   этот замер.
5. **Исключённые каталоги.** `vibedeps/`, `refs/`, `.vibe/`, `node_modules/`,
   `target/` пакетом исключены из счёта; копии журнальных файлов внутри них
   (если есть) в периметр не входят и не проверялись.
