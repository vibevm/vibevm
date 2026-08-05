# g3-f132-schema-debt — measured

Measurement of the F-132 schema-tag debt. Numbers and addresses only; no
recommendations. Working tree is a git worktree of vibevm at
`C:\Users\olegc\git\v\vibevm\.wt\E-G4-F132` (host project). Git was not run.

The premise under audit (recorded verbatim in `conform.toml:122`): the
`vibe-wire` crate is excluded from the conform gate because it is generated
code, and *"the generator input under `schemas/` is the taggable unit
instead."* An earlier debt note named a file `schemas/specmap.jtd.json`; that
file does not exist (see Q1).

---

## Точка A — определение «spec-тега» для JSON-схемы (решение работника)

Термин нигде не определён. Принимаю определение:

> **Spec-тег для JTD-схемы** = присвоенный схеме как целому `spec://…`-адрес,
> детектируемый одним из двух способов: (a) ключ `spec` (или `specmark`) в
> верхнеуровневом блоке `metadata` схемы, значением которого является строка
> `spec://…`; ИЛИ (b) любое литеральное вхождение подстроки `spec://` в теле
> файла схемы.

**Почему это определение:** (1) `spec://…` — единственная форма адреса в
дисциплине (пример по всему дереву — `specmark::scope!("spec://…")`); (2) оба
способа — это то, что сканер specmap (Q4) и движок conform могли бы прочитать,
если бы их научили; (3) они соответствуют двум существующим ролям в модели
движка — «единица спецификации» (markdown-якорь с `spec://`) и «код-тег»
(specmark-атрибут со `spec://`). Подсчёт в Q1 ведётся по обоим; результат
одинаков.

---

## Q1 — что реально лежит в `schemas/`

**0 из 7 схем несут spec-тег** (под любым из двух определений Точки A: ни ключа
`spec`/`specmark` в верхнеуровневом `metadata`, ни подстроки `spec://` нигде в
каталоге нет — `grep -rn "spec://" schemas/` → rc=1).

В каталоге ровно 7 файлов, все формата `.jtd.json` (`ls schemas/` дословно — в
самопроверке ниже):

| Файл | Строк | Верхнеуровневый `metadata` | Ключи верхнеуровневого `metadata` | `spec://` в файле |
|---|---:|:---:|---|:---:|
| `init_report.jtd.json` | 69 | да (стр. 2) | `description`, `rustOptions.package` | нет |
| `install_plan.jtd.json` | 67 | да (стр. 2) | `description`, `rustOptions.package` | нет |
| `install_report.jtd.json` | 50 | да (стр. 2) | `description`, `rustOptions.package` | нет |
| `list_report.jtd.json` | 92 | да (стр. 2) | `description`, `rustOptions.package` | нет |
| `registry_publish_report.jtd.json` | 61 | да (стр. 2) | `description`, `rustOptions.package` | нет |
| `registry_sync_report.jtd.json` | 69 | да (стр. 2) | `description`, `rustOptions.package` | нет |
| `uninstall_report.jtd.json` | 39 | да (стр. 2) | `description`, `rustOptions.package` | нет |
| **итого** | **447** | 7/7 | — | **0** |

Каждый файл имеет верхнеуровневый `metadata` ровно с двумя ключами:
`description` (строка) и `rustOptions.package` (строка вида
`vibe_wire::<модуль>`). Вложенные блоки `"metadata"` (на строках 14, 23, …) —
это JTD per-property `description`-метаданные, не spec-теги. Файла
`specmap.jtd.json` в `schemas/` нет — его здесь быть не должно: `specmap`-схема
живёт в пакете движка (`packages/…/core-ai-native/…/schemas/`), а не в host
`schemas/` (см. Q3).

**Что тут неочевидно.** Файлы НЕ помечены даже намёком на spec-адрес;
единственная «маркировка» в `metadata` — это `rustOptions.package`, т.е. цель
codegen, а не spec-принадлежность. Долг «проставить spec-метки в
`schemas/specmap.jtd.json`» опирается на несуществующий файл и на каталог, в
котором меток нет ни в одном из 7 файлов.

---

## Q2 — что порождается из этих схем

**9 файлов, 420 строк; 7 сгенерированных модулей, каждый связан 1:1 со своей
схемой через `metadata.rustOptions.package` (→ `generated/<имя>/mod.rs`).
Ни один сгенерированный файл `specmark::scope!` не несёт** (`grep -rn
"specmark\|scope!" crates/vibe-wire/` → rc=1).

Дерево `crates/vibe-wire/src/`: `lib.rs` (31), `generated/mod.rs` (13 —
агрегатор, синтезируется xtask'ом), и 7 модулей `generated/<имя>/mod.rs`.

Связка схема → модуль (имя схемы ↔ `rustOptions.package` ↔ путь модуля):

| Схема | `metadata.rustOptions.package` | Сгенерированный модуль | Строк |
|---|---|---|---:|
| `init_report.jtd.json` | `vibe_wire::init_report` | `generated/init_report/mod.rs` | 58 |
| `install_plan.jtd.json` | `vibe_wire::install_plan` | `generated/install_plan/mod.rs` | 47 |
| `install_report.jtd.json` | `vibe_wire::install_report` | `generated/install_report/mod.rs` | 35 |
| `list_report.jtd.json` | `vibe_wire::list_report` | `generated/list_report/mod.rs` | 88 |
| `registry_publish_report.jtd.json` | `vibe_wire::registry_publish_report` | `generated/registry_publish_report/mod.rs` | 50 |
| `registry_sync_report.jtd.json` | `vibe_wire::registry_sync_report` | `generated/registry_sync_report/mod.rs` | 67 |
| `uninstall_report.jtd.json` | `vibe_wire::uninstall_report` | `generated/uninstall_report/mod.rs` | 31 |
| (агрегатор) | — | `generated/mod.rs` | 13 |
| (корень крейта) | — | `lib.rs` | 31 |

Связка подтверждена дословно: `generated/mod.rs` (стр. 6-7) — *"Each submodule
is generated by `jtd-codegen` from the matching `*.jtd.json` schema under
`schemas/`"*; `lib.rs` (стр. 4-6, 26-30) указывает на `schemas/` как на source
of truth.

**Что тут неочевидно.** Расхождение версий генератора в комментариях: заголовок
сгенерированного файла — *"Code generated by jtd-codegen for Rust v0.2.1"*
(`generated/init_report/mod.rs:1`), тогда как `lib.rs:14` и
`xtask/src/codegen.rs:171` ссылаются на *jtd-codegen 0.4.1*. На измерение связи
схема↔модуль это не влияет; зафиксировано как наблюдение.

---

## Q3 — чем порождается

**Генератор найден в дереве: `cargo xtask codegen` (и `check-codegen`),
реализован в `xtask/src/codegen.rs`; вызывает локально-вендоренный бинарник
`jtd-codegen` из `tools/jtd-codegen/`.**

Точки вызова/определения (`путь:строка`):

- `xtask/src/main.rs:323` — `Cmd::Codegen => run_codegen()`
- `xtask/src/main.rs:324` — `Cmd::CheckCodegen => run_check_codegen()`
- `xtask/src/codegen.rs:98` — `pub(crate) fn run_codegen()` (точка входа)
- `xtask/src/codegen.rs:19` — `fn find_jtd_codegen()`; локальная копия
  `root.join("tools").join("jtd-codegen").join(exe)` (`codegen.rs:25`)
- `xtask/src/codegen.rs:106` — сканирует ДВА дома схем: host `schemas/` и
  `specmap_schema_dir(&root)` (схема specmap внутри пакета движка,
  `codegen.rs:51-54`)
- `xtask/src/codegen.rs:148` — `fn generate_into()`; цикл запуска `jtd-codegen`
  по схемам — `codegen.rs:176-190`
- `xtask/src/codegen.rs:237` — `fn run_check_codegen()` (codegen + `git diff
  --exit-code` по сгенерированным деревьям)

Команды поиска и их результат: `grep -rn "codegen\|jtd-codegen\|schemas"
xtask/` → совпадения сосредоточены в `xtask/src/codegen.rs` и
`xtask/src/main.rs` (вывод приведён вEXEC-трассе); `find tools -type d -name
jtd-codegen` → `tools/jtd-codegen` (содержит `first-run.ps1`, `first-run.sh`,
`self-check.sh`, подкаталог `jtd-codegen/`). Ручной запуск `jtd-codegen`
напрямую нигде не зафиксирован — всё идет через xtask.

**Что тут неочевидно.** Host-схемы (`schemas/`) — лишь один из двух входов
генератора; второй вход (`specmap_schema_dir`) — это `specmap`-схема внутри
`packages/…/core-ai-native/…/schemas/`. Поэтому отсутствие `specmap.jtd.json` в
host `schemas/` корректно: эта схема принадлежит пакету движка, не host'у.

---

## Q4 — какие расширения принимает сканер specmap

**2 типа файлов: `.rs` и `.md`. `.json` среди них НЕТ.**

Сканер строит `specmap.json` композицией двух сканеров; каждый сканер владеет
своим обходом и хардкодит свой фильтр расширений (центральной таблицы
«расширение→обработчик» нет — см. Q5). Движок, который собирает host,
разрешается через `Cargo.toml:108-109` в
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-specmap/`
(далее `$ENG`).

Множество расширений задано дословно в трёх местах (одно и то же сравнение):

- `$ENG/src/rscan.rs:333` —
  `if path.extension().and_then(|e| e.to_str()) != Some("rs") {` → код-сканер
  читает ТОЛЬКО `.rs`
- `$ENG/src/mdspec.rs:422` —
  `if path.extension().and_then(|e| e.to_str()) != Some("md") {` →
  markdown-сканер читает ТОЛЬКО `.md`
- `$ENG/src/mdspec.rs:498` — то же `!= Some("md")` (второй обход, для
  `root_spec_docs`)
- (параллельно) `$ENG/src/ratchet.rs:78` — `!= Some("rs")` → orphan-ratchet
  пере-обходит только `.rs`

**Ответ числом:** 2 расширения (`.rs`, `.md`); `.json` отсутствует. Политики
`specmap.toml` (`scan_roots`, `spec_roots`, `root_spec_docs`) задают **каталоги**
для обхода, а не расширения; расширения захардкожены в сканерах.

**Что тут неочевидно.** `.json` в дереве сканером не читается вообще — ни как
источник spec-единиц, ни как источник code-тегов. `specmap.json` на выходе —
это артефакт (результат), а не вход.

---

## Q5 — цена обучения сканера читать `metadata` JSON-схемы как spec-тег

**Диспетчеризация по типу файла ЕСТЬ — структурная, не табличная: один сканер
на язык, каждый хардкодит своё расширение в собственном обходе.** Естественное
гнездо для JSON-случая — новый `impl CodeScanner` (трейт + `CompositeScanner`
существуют именно для этого; doc `scanner.rs:1-9` называет
`typescript-ai-native-specmap-scan` как такой же per-language сканер).
**Честной единой оценки в строках НЕ получается** — см. Точку B ниже; даю точки
вставки и вилку.

Точки вставки (`путь:строка`, движок `$ENG`):

1. **Новый модуль-сканер** (напр. `$ENG/src/jsonscan.rs`), реализующий трейт
   `CodeScanner` (`scanner.rs:23`): обход `scan_roots`/`spec_roots` для
   `.jtd.json`, парсинг JSON `metadata`, извлечение spec-адреса, эмиссия
   `CodeItem`/`Edge`. Образец — обход+фильтр `rscan.rs:320-346` (~15 строк) +
   `record_item`/`record_edge` `rscan.rs:44-76`. Оценка объёма новой
   функции: ~50-80 строк, **но вилка зависит от Точки B**.
2. **Включение в композицию.** `index.rs:57-58` (`build`), `index.rs:341`
   (`write`), `index.rs:368` (`check`) захардкожены в `&RustScanner`; замена на
   `CompositeScanner::new(vec![&RustScanner, &JsonSchemaScanner])` — 3 места,
   ~1-3 строки на каждое (`scanner.rs:45-53`).
3. **Host-политика.** `specmap.toml` `scan_roots = ["crates/*", "xtask"]`
   (`specmap.toml`, блок `scan_roots`) — НЕ включает `schemas/`. Даже при
   наличии JSON-сканера каталог не сканируется; нужно добавить `schemas` (1
   строка конфига, не код).
4. **Паритет ratchet.** `$ENG/src/ratchet.rs:70-90` пере-обходит только `.rs`
   независимо от индекса; если JSON-теги должны быть видны ratchet'у — нужен
   параллельный обход, ~15-20 строк. Условно: `vibe-wire` уже `exempt` и в
   `specmap.toml`, и в `conform.toml`, а схемы — не pub-поверхность крейта,
   поэтому ratchet для них, возможно, не требуется (зависит от решения босса о
   том, что считать «taggable unit»).

Сколько строк СЕЙЧАС в затрагиваемых функциях: `scanner.rs` — 78 строк (весь
файл), `rscan.rs` — 430, `mdspec.rs` — ~510, `index.rs` — ~470, `ratchet.rs` —
~270.

## Точка B — почему единой оценки в строках не сходится

После чтения кода: модель данных движка не даёт «тегу JSON-схемы» однозначного
дома. Конкретно:

- `CodeItem` (`$ENG/src/generated/specmap/mod.rs:36`) НЕ имеет поля
  spec-адреса. Спец-принадлежность кода в модели — это **`Edge`**
  (`generated/specmap/mod.rs`, struct `Edge`): ребро `fromSymbol`(код)→`uri`
  (`spec://…`) с `verb` (`implements`/`verifies`/`deviates`), порождаемое
  specmark-атрибутами (`#[spec]`/`#[verifies]`/`scope!`) — см. `rscan.rs:44-76`,
  `rscan.rs:85-134`.
- `fingerprint` (`CodeItem.fingerprint`, и `fingerprint.rs:14`) определён как
  хэш **`to_token_stream().to_string()`** — т.е. syn token-stream Rust-кода. У
  JSON-файла нет syn-потока; поле `endLine`/`line` — также code-span-семантика.
  Поля `Option`, и doc явно допускает «scanner that cannot leaves it absent» —
  но политики fingerprint'а для не-кода нет.

Поэтому «схема несёт spec-тег» может означать две несовместимые вещи, и выбор
меняет набор файлов и необходимость новой структуры:

- **(a) Item + Edge** (путь rscan): синтетический `symbol` (напр.
  `rustOptions.package`) + `Edge` на `spec://`URI из `metadata`. Требует решения
  о fingerprint'е для JSON и о том, что есть «symbol» схемы. Меняет
  `jsonscan.rs` + 3 точки в `index.rs` + `specmap.toml`.
- **(b) Spec-единица** (путь mdspec): схема как единица спецификации. Но
  единицы приходят из markdown-якорей через сугубо markdown-парсер
  (`mdspec.rs`, заголовки/`<a id>`); JSON `metadata` — не markdown-заголовок,
  нужен принципиально иной парсер и, вероятно, новое поле/структура в модели
  единиц.

**Чего не хватает для оценки:** (1) решения item-vs-unit (его модель не
навязывает); (2) политики fingerprint'а для не-code файлов; (3) решения, должен
ли ratchet видеть такие теги. Без (1) число «строк для добавления»
расходится в 1.5-2 раза между двумя путями и в принципе меняет затрагиваемый
набор файлов. Адреса, из-за которых оценка не сходится:
`generated/specmap/mod.rs:36` (нет поля адреса в `CodeItem`), `rscan.rs:44-76`
(тег = Edge, не поле Item), `fingerprint.rs:14` (fingerprint = syn-поток),
`mdspec.rs:200-220` (единицы = markdown-stem, иной парсер).

---

## Q6 — кто ещё читает эти схемы

**Широко: 102 упоминания строки `schemas/` по ~61 файлу (31 проза `.md` + 30
код/конфиг `.rs`/`.toml`/`.sh`/`.ps1`) — но с шумом: ~20 из 30 «код»-файлов это
vendored-копии движка, говорящие о СОБСТВЕННОМ `schemas/` пакета, а не о 7
host-схемах. Узко (настоящие потребители host-схем в коде): 6 файлов, из них
контент схем читает 1 (генератор). Имена схем поимённо — 18 упоминаний в 7
файлах, ВСЁ проза, 0 в коде.**

Разбор (`grep -r …`, исключая `vibedeps/`, `refs/`, `.vibe/`, `target/`,
`campaigns/`, `.wt/`):

- **Имена 7 схем поимённо** (`init_report.jtd.json` и т.д.): 18 упоминаний в 7
  файлах — `docs/commands/{init,install,list,registry-publish,registry-sync,uninstall}.md`,
  `docs/lockfile-format.md`. **0 в коде** (код обращается к схемам только через
  glob `*.jtd.json`, не по именам).
- **Строка `schemas/`**: 102 упоминания / ~61 файл. После исключения vendored
  `packages/…/core-ai-native-specmap/src/*` (чужой `schemas/` движка)
  host-потребители в коде/конфиге =
  - `xtask/src/codegen.rs`, `xtask/src/main.rs` — генератор (читает схему);
  - `crates/vibe-wire/src/lib.rs`, `crates/vibe-wire/src/generated/mod.rs` —
    комментарии (указывают на `schemas/` как source of truth);
  - `tools/self-check.sh` — gate 6b (`check-codegen`, см. `self-check.sh:334-338`);
  - `conform.toml` — формулировка изъятия (стр. 122).
- **Токен `jtd`**: ~45 код-файлов / ~29 prose-файлов (широко, включая vendored
  копии; точное host-только число не снималось — большинство совпадений в
  `packages/…/core-ai-native-specmap` относятся к JTD вообще, не к host-схемам).

**Что тут неочевидно.** Единственный код, который РАЗБИРАЕТ содержимое схем —
генератор (`xtask/src/codegen.rs` через бинарник `jtd-codegen`). Остальные
5 host-потребителей ссылаются на `schemas/` текстуально (комментарии, политика,
gate). Поимённо схемы упомянуты только в пользовательской документации
wire-форматов (`docs/commands/*.md`).

---

## Q7 — цена альтернативы (правка только формулировки в `conform.toml`)

**Оперативная формулировка — 1 строка (`conform.toml:122`, строка `reason =`).
Весь блок изъятия — 3 строки (`conform.toml:120-122`). Параллельное
утверждение-предпосылка долга есть ещё в одном файле — `specmap.toml`
(комментарий к `exempt`) — правка только `conform.toml` его не затронет.**

Изъятие `vibe-wire` в `conform.toml`:

- `conform.toml:120` — `[[rust.exempt]]`
- `conform.toml:121` — `unit = "vibe-wire"`
- `conform.toml:122` — `reason = "generated code (JTD-schema codegen output),
  excluded by PROP-014 §2.3; the generator input under schemas/ is the taggable
  unit instead"`

Чтобы переформулировать (например, убрать претензию «the generator input under
schemas/ is the taggable unit instead» или заменить её), трогается **1 строка**
(122). Перестройка всего блока — **3 строки** (120-122).

Отдельно: есть второй, МЕХАНИЧЕСКИЙ механизм изъятия сгенерённого кода —
`conform.toml:22`, `exclude_substrings = ["/generated/"]`. Это он фактически
вырезает `crates/vibe-wire/src/generated/**` из сканирования; `[[rust.exempt]]
unit = "vibe-wire"` снимает требование «every crate gated or exempt». Правка
формулировки (`:122`) этого механизма не касается.

**Образцы «исключено, и вот почему, и вот маршрут» в этом же файле — есть.**
Все 6 блоков `[[rust.exempt]]` (`conform.toml:112-135`) одинаковой формы
(`unit` + `reason`), и несколько причин явно дают маршрут/альтернативу. Два
дословно:

1. `vibe-test-support` (`conform.toml:128-130`):
   ```
   [[rust.exempt]]
   unit = "vibe-test-support"
   reason = "test-only support crate — consumed exclusively as a dev-dependency, ships in no binary; the Class-F/G seam and REQ-edge gates describe a product surface it does not have (DRIFT-020)"
   ```
2. `vibe-graph` (`conform.toml:112-114`):
   ```
   [[rust.exempt]]
   unit = "vibe-graph"
   reason = "M0 stub, no code yet — the task-graph runner per VIBEVM-SPEC §5 is unbuilt; nothing to gate until it lands"
   ```
   (ср. также `vibe-llm` `:116-118`, `xtask` `:124-126`, `vibe-spec` `:132-134`
   — той же формы).

Параллель в `specmap.toml` (другой файл): блок `exempt` с комментарием
*"vibe-wire — generated code; the JTD schema is the taggable unit"* — та же
предпосылка долга. Если «править только `conform.toml`», эта строка-двойник в
`specmap.toml` останется.

---

## Что осталось неизмеренным

- **Какую версию `core-ai-native-specmap` соберёт произвольный потребитель**
  (не этот host): измерялся только host-путь разрешения (`Cargo.toml:108-109`
  → v0.7.0 vendored). У других потребителей (fractality, mcp) путь другой, но
  логика фильтра расширений в движке одинакова во всех копиях (`.rs`/`.md`),
  поэтому на ответ Q4 это не влияет.
- **Точное host-only число совпадений токена `jtd`** (Q6): снято широкое число
  (45 код / 29 проза) с шумом vendored-копий; узкое число «кто упоминает именно
  7 host-схем» дано по именам файлов (0 в коде, 7 в прозе), что сильнее и
  снимает неоднозначность.
- **Поведение `jtd-codegen` при наличии ключа `metadata.spec`**: не запускалось
  (код не менялся, бинарник не вызывался напрямую). Измерение Q1/Q2 — по
  фактическому содержимому файлов, не по гипотетическому прогону.
