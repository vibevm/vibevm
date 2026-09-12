# WORKER-REPORT-P2-O6 — цитаты `rule` и рёбра, HTML-остров, номера блоков

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O6.md` (A2.11, A2.13, A2.22).
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Три атома сделаны, тремя коммитами, каждый по своим путям
(`git commit … -- <пути>`). Незакоммиченных правок у меня в дереве нет,
чужих файлов я не стейджил.

**Главная цифра пакета: 381 цитата `rule` в руководстве, разрешились все
381, ноль нерешённых.** Кроме них проверка нашла в прозе ещё восемь
адресов-цитат (все разрешились), две самоадресации, шесть плейсхолдеров и
две учебные — правила X-029 реализованы и работают.

`vibe specmap --path <руководство>` даёт **381 ребро `documents`, 0
suspects, 0 закреплённых рёбер** (`pinned_r` нет ни у одного).

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A2.11 | `51695bdd` | `feat(doc): resolve rule citations against the current specs` |
| A2.13 | `fcc32b96` | `feat(doc): render islands from the pivot so web and local share one content path` |
| A2.22 | `6aa578df` | `feat(doc): number blocks at build time across every projection` |

Плюс один служебный коммит: `cbd3b8e9`
`chore(cargo): record the lockfile for the landed manifests`. `Cargo.lock`
отставал от **уже закоммиченных** манифестов — двух моих (`vibe-doc`,
`vibe-trace`) и одного чужого (`vibe-check` получил `toml` коммитом
`9993148f`). Ни одна версия не сдвинута, ни одна зависимость там не
добавлена: файл лишь записывает то, что закоммиченные `Cargo.toml` уже
говорят, чтобы `cargo build --locked` видел то дерево, которое описывает
ветка. Незакоммиченных чужих правок в нём не было — на момент коммита
`git status` не показывал ни одного изменённого `Cargo.toml`.

Этот отчёт идёт отдельным коммитом `docs(campaign): …`, как отчёты
предыдущих волн: продуктовый атом остаётся одним коммитом и не может
содержать собственный хэш.

## A2.11 — резолвер цитат, рёбра, `--citations` (`51695bdd`)

### Четыре источника, ближний первым

`vibe_doc::citations::sources::SpecSources` резолвит координату
`<group>/<name>` по цепочке: **checkout** (собственная координата
проекта) → **in-tree** (`LocalRegistry` по `vibevm/vibepacks`) → **lock**
(`Lockfile::read` + `vibe_workspace::vibedeps::slot_abs_path`) →
**store** (`vibe_registry::store::entry_dir`). Порядок отвечает на вопрос
«какой экземпляр я получил»: ближний побеждает.

**Ни одна из четырёх раскладок здесь не написана заново.** Каталоговый
реестр и store — одна форма, и её читатель `LocalRegistry`; слот —
`slot_abs_path`; грамматика адреса и инверсия усечения `PROP-042` →
`PROP-042-example-thing.xml` — `vibe_spec::{SpecAddress, FileResolver}`.
Резолвер файла строится через `FileResolver::with_selected_world` ровно с
тем экземпляром, который выбрала цепочка, поэтому «самая свежая
установленная» внутри `vibe-spec` не может подставить другой, а
`@version` в адресе **проверяется**, а не отбрасывается.

Новые зависимости `vibe-doc`: `vibe-spec`, `vibe-core`, `vibe-registry`,
`vibe-workspace`, `semver`. Их называет сама норма — PROP-057
`##LOCAL-WARMUP` перечисляет `lookup`/`list_all`, `Lockfile::read`,
`slot_abs_path`, `LocalRegistry` как источники этой библиотеки.

Мир источников собирает **композиционный корень** (`vibe-cli`), библиотека
не читает окружение: `spec_sources(repo_root, settings_home)` в
`crates/vibe-cli/src/commands/doc.rs`. Store берётся как
`<settings>/cache` тем же хелпером `settings_home`, которым уже пользуется
трипвайр раннера.

### Текст факта и язык спеки

`citations::resolve(uri, &sources)` возвращает `RuleText { uri, anchor,
text, lang, source, path }`. Спека читается **пивотом** (тем же читателем,
против которого её пишет автор), не вторым парсером.

Два решения о том, во что резолвится якорь:

1. **Факт** → его собственное тело. Если у факта есть связанный fence
   (`@fact/code:<ID>`), он приписывается — тело типизированного факта
   продолжается в fence.
2. **Секция** → её **заголовок**, не поддерево. Секция — контейнер;
   вставить её целиком значило бы процитировать главу там, где автор
   просил правило. В корпусе руководства такие адреса есть (например
   `PROP-045#shape`).

`lang` — язык **цитируемого пакета** (`[i18n].canonical` его манифеста,
иначе `en` по `DEFAULT_CANONICAL_LANGUAGE`), никогда не язык страницы:
русская страница цитирует английское правило по-английски и говорит об
этом атрибутом `lang` на ссылке.

### X-029 — три формы прозы, которые не цитаты

`citations::classify(uri, self_coordinate)`:

| класс | что это | сколько в руководстве |
|---|---|---|
| `Citation` | настоящая цитата, обязана разрешиться | 389 (381 из `rule` + 8 из прозы) |
| `Placeholder` | адрес с многоточием или `<…>` | 6 |
| `Teaching` | группа `org.acme` | 2 |
| `SelfAddress` | `spec://org.vibevm.core/vibevm-docs/…` | 2 |

**Находка, которой в X-029 не было:** страница — это XML, поэтому автор,
пишущий `<page>` в прозе, обязан экранировать скобки, и сырые байты несут
`&lt;page&gt;`. Первая версия проверки читала только литеральные `<` и
`>` и приняла два плейсхолдера за цитаты к пакету `&lt;group&gt;`.
Плейсхолдерные метки распознаются в **обоих** написаниях; тест прибит.

Плейсхолдер **побеждает** учебную группу: `spec://org.acme/…/X` нельзя
разрешить именно из-за многоточия, и назвать его учебным значило бы
назвать не ту причину.

Самоадресация резолвится **по дереву пакета**: страница должна
существовать. Фрагмент вида `pNN` сознательно не проверяется — это
позиционный номер, живущий по текущему тексту, а не якорь страницы.

### Рёбра `documents` — хостовая сторона, без пинов

`vibe_doc::citations::edges(package_dir, coordinate)` выдаёт чистое
значение `DocEdge { file, page, line, uri }`; адаптер к движку —
`vibe_trace::docscan::DocScanner`, реализующий движковый `CodeScanner`.
Движок и шесть его вендоренных копий не тронуты, `sync-engines` не
запускался (R-21, PROP-057 `##PIPE-EDGES-HOST-SIDE`).

- Ребро всегда `pinned_r: None` — единственный вход в детекцию suspect'ов
  это пин на ребре, поэтому живая цитата не может «протухнуть» ни при
  каких ревизиях спеки. Проверено на живой карте: `pinned: 0`.
- Хвост ребра — `CodeItem` с сентинелью `crate_name = "<doc>"`, по
  прецеденту JTD-сканера (`"<schema>"`); `fingerprint` отсутствует (у
  страницы нет потока токенов). Wire-схема движка не менялась.
- Символ страницы — её адрес без расширения: `model/boot-lane.xml` →
  `model::boot-lane`.
- **Номер строки берётся из сырых байтов страницы**: IR — документная
  модель, строк в ней нет, а красный гейт, который не может сказать
  «где», приходится грепать. Каждое открытие `<rule` сверяется с адресом,
  которого IR ждёт следующим, поэтому `<rule`, показанный **внутри
  fence** (так делает страница про авторство), не сдвигает строки
  настоящих. Тест на это есть.
- Только `rule` чеканит ребро. Адрес в прозе проверяется (мёртвый адрес
  мёртв везде), но `documents` — это **объявление**, и объявляет его
  элемент `rule`.

### Карта doc-пакета

Заведён `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/specmap.toml`:

- `spec_roots = []` — **несущая пустота**: страницы написаны словарём
  документации (`<rule>`, `<example>`, `<prompt>`), движковый диалект
  закрыт и их не знает, и страница внутри `spec_roots` была бы отвергнута
  как нарушение диалекта. Их читает хостовый сканер, который команда
  `vibe specmap` подмешивает к встроенному набору через `CompositeScanner`
  (только для пакетов вида `doc`).
- `max_connections_per_item = 0` — гейт `overloaded-item` выключен.
  Он спрашивает, не реализует ли один элемент кода слишком много точек
  спеки; страница — не элемент кода, и страница, цитирующая восемь правил,
  это жанр, работающий как задумано. На пороге 3 он давал **42
  предупреждения** и хоронил под собой четыре осмысленных.
- `[[external_specs]]` на три пространства имён (хост, `core-ai-native`,
  `addressable-specs`) — только для **разрешения адресов**; их узлы в
  карту пакета не попадают, но не дают рёбрам болтаться.

`package.specmap.json` закоммичен — так же, как у `fractality`
(единственный прецедент в дереве). Гейта, который держал бы его свежим,
нет; см. «Что не сделано», п. 3.

## A2.13 — HTML-остров (`fcc32b96`)

`vibe_doc::html::to_html(doc, &Content) -> String`.

**Никаких скриптов и стилей — это контракт, а не предосторожность.**
Параметры запуска приходят в оболочку неисполняемым блоком
`<script type="application/json">`, который эмитит **оболочка**, и именно
поэтому CSP не нуждается в `'unsafe-inline'`. Фикстура голдена цитирует в
прозе и `<script>`, и `<style>`; тест доказывает, что ни один не выжил как
разметка. Остров не несёт и обвязки страницы: ни `<html>`, ни `<head>`, ни
навигации, ни мета-блока.

Форма:

| конструкция | HTML |
|---|---|
| документ | `<article class="doc-page" data-status data-audience>` |
| секция | `<section id="…">` + заголовок своей глубины |
| факт | `data-fact` + `data-status` на блоке, который его несёт |
| `when` | `data-when`; блок **сохраняется**, не выбрасывается |
| `rule` | `<blockquote class="rule">` с `<a class="rule" href data-uri lang>` внутри |
| `example` / `example ref` | `<div class="example">` с командой и выводом |
| `derived` | `<pre class="derived" data-derived data-ref>` |
| `note` | `<aside class="note" data-note>` |
| `figure` | `<figure><img><figcaption>` |
| `prompt` | `<div class="prompt">` + текст, needs, outcome, ассерты |

**Почему `<a class="rule">` завёрнут в `<blockquote>`, а не стоит блоком.**
Пакет требует `<a class="rule" href data-uri>` с вложенным текстом факта —
он есть. Но A2.22 требует, чтобы первым дочерним узлом блока был
`<a class="p-anchor">`, а **вложенный `<a>` внутри `<a>` — невалидный
HTML**. Обёртка-цитата разрешает конфликт и заодно говорит правду: цитата
правила — это цитата.

`data-rev` нет нигде, `~rN` из атрибута не доходит до острова — цитата
живая и беспиновая (D-27). Проверено тестом на `#A~r7`.

Блок, текст которого эта сборка не достала, помечается
`data-unresolved="true"` и показывает свой адрес. Отказываться рендерить
страницу значило бы сообщать автору по одному дефекту за прогон; место,
где отсутствующий текст — это провал, — проверки, а не рендерер.

### Инлайн-Markdown — закрытый словарь из пяти конвенций

Пивот сознательно не моделирует инлайн-грамматику
(`##INLINE-STAYS-MARKDOWN`), поэтому бэкенд, печатающий юнит дословно,
показал бы читателю `**это**` и `` `то` ``. `html::inline::render`
переводит ровно пять конвенций — код-спан, ссылка, автоссылка, `strong`,
`em` — и экранирует всё остальное. **Код-спан связывает первым**, и его
содержимое больше не разбирается: на этом держится руководство, полное
процитированной разметки. Незакрытый маркер остаётся текстом.

Это единственное место во всём атоме, которого пакет прямо не просил;
причина — без него «один путь контента для веба и локального читателя»
не выполняется ни для одного из двух.

### Голдены

Фикстура — **настоящий пакет** `crates/vibe-doc/tests/fixture/manual/`
(kind `doc`, манифест, одна страница со всеми блоками жанра по разу), а не
экранированный литерал: голден, снятый с дерева, можно открыть, прочитать
и поправить. Голдены — `crates/vibe-doc/tests/golden/`, снимаются
`VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test island`.

## A2.22 — номера блоков в трёх проекциях (`6aa578df`)

`vibe_doc::numbering::{BlockPath, Numbering, number_blocks, expand_derived}`.

| проекция | как выглядит номер |
|---|---|
| HTML | `data-p="7"` на блоке + `<a class="p-anchor" id="p07" href="#p07">07</a>` первым дочерним узлом |
| `.md` | `[p07]` в начале блока |
| `.xml` | атрибут `p="7"` |

Тест `crates/vibe-doc/tests/projections.rs`: одна фикстура, три проекции,
**один набор номеров** (сравниваются множества чисел, а не написания —
каждая проекция пишет номер так, как хочет её носитель); повторный
рендер — те же номера; секция `footnotes` не нумеруется ни в одной
проекции.

Три решения и их причины:

1. **Нумерация до фильтрации `when`** (PROP-057 `##PIPE-NUMBERING`).
   Иначе `p12` в баг-репорте значил бы одно на Windows и другое на Linux.
   Пропуски в конкретной сборке — принятая цена.
2. **Номер — не состояние.** `number_blocks` — чистая функция, `Numbering`
   — значение рядом с документом, никогда не поле IR: все законы пивота
   сформулированы как равенство `SpecDoc`, и номер внутри вошёл бы в
   `PartialEq`. `to_xml`/`to_markdown` пивота не тронуты и `Numbering` не
   видят.
3. **`pNN` позиционен**, и это принято: вставка абзаца двигает номера
   ниже, как ссылка на строку файла после правки. Устойчивая адресация —
   именованный якорь.

**Две уступки HTML, а не вкусу:** список может содержать только `<li>`, а
таблица — только caption/colgroup/rows, поэтому якорь списка едет в голову
первого пункта, а якорь таблицы — в `<caption>`. Всё остальное получает
его первым дочерним узлом. `<pre>` получает якорь **без окружающих
пробелов**: там пробел — это содержимое.

### Почему `.md` и `.xml` написаны в `vibe-doc`, а не взяты у пивота

Две независимые причины, каждая достаточная:

1. `Numbering` в пивот не входит по норме, а номер нужен в обеих
   проекциях.
2. Пивот проецирует **источник**: `rule` — адресом, `derived` — строкой
   происхождения, `example ref` — фразой «скопируется при проекции». Это
   правильно для того, чем оно является (форма, через которую хостовые
   сканеры читают страницу), и неправильно для читателя, которому нужен
   текст правила, сгенерированный вывод и заимствованный пример.

Написания, общие с пивотом (`@fact:<ID>`, `@status:<stage>/<state>`, формы
fence), сделаны **одинаковыми сознательно**: один проект — один диалект
Markdown.

**`.xml` сохраняет адрес, а не подставляет текст** — в отличие от острова
и Markdown. Агенту, читающему дерево, нужен адрес: он резолвит цитату сам,
а подставленная копия — ровно то, чего весь этот конвейер избегает.
Писатель эмитит **родовые** написания диалекта (`<section id=…>`,
`<fact id=…>`), а не элементо-именные формы, которые может написать автор:
у проекции нет авторских байтов, которые надо сохранить, и одно написание
— это на один закон меньше.

`expand_derived` заменяет один блок одним блоком, поэтому нумерация до и
после раскрытия совпадает — это доказано тестом, а не предположено, и
именно это делает порядок нормы («раскрыть, потом пронумеровать»)
бесплатным.

## Расхождения с текстом пакета и нормой

1. **`READER-NUMBERED-BLOCKS` против `PIPE-NUMBERING` внутри PROP-057.**
   Первый перечисляет **заголовок** среди нумеруемых блоков; второй прямо
   говорит «list items, table cells, the children of `prompt` and
   **headings** are not». Реализовано по `PIPE-NUMBERING`: это норма
   именно той функции, которую я писал, она написана по A0.25 (где
   заголовки исключены с обоснованием «у них своя адресация — якорь»), и в
   IR заголовок вообще не блок — он `Section.title`, и `number_blocks`
   его не встречает. Формулировку `READER-NUMBERED-BLOCKS` стоит
   поправить; PROP-файлы мне трогать запрещено.
2. **`data-p` против «номер идёт атрибутом `data-p`» у заголовка.** План
   A2.22 пишет «заголовок сохраняет `id` из `{#id}`, номер идёт атрибутом
   `data-p`». Прочитано так: `id="p07"` живёт на самом якоре
   (`<a class="p-anchor">`), а номер как атрибут — на элементе блока; у
   секции остаётся её именованный `id` и никакого номера (п. 1).
3. **`footnotes` не нумеруется** — в IR нет рода блока `footnotes`, есть
   только возможная **секция** с таким id. Реализовано как «блоки секции
   с `id="footnotes"` не нумеруются»; константа
   `numbering::UNNUMBERED_SECTION`. Другого предмета у этого правила
   сегодня нет.
4. **Инлайн-Markdown в острове** — пакет его не просил (см. A2.13). Без
   него остров показывает читателю сырые звёздочки.

## Аномалии, найденные по пути (правок не делал)

1. **Движок не чеканит узел для факта внутри ячейки таблицы.** Карта
   doc-пакета несёт четыре `dangling-edge`:

   ```
   howto/install-a-package.xml:54  → …/PROP-008#ROW-QUALIFIED-KIND-BEHAVIOUR
   howto/update-packages.xml:48    → …/PROP-002#ROW-GS-BRANCH-MEANING
   model/packages-and-kinds.xml:48 → …/PROP-008#ROW-SHORT-BEHAVIOUR
   model/registries.xml:32         → …/PROP-002#ROW-GS-BRANCH-MEANING
   ```

   **Якоря существуют** — например
   `<ROW-QUALIFIED-KIND-BEHAVIOUR fact="true" …>` в
   `vibevm/vibespecs/modules/vibe-registry/PROP-008-…xml:82`, внутри
   `<td>`, — и мой резолвер их находит (`--citations` зелёный). Не
   находит их движковый `mdspec`: в хостовой карте на всё дерево ровно
   **один** узел с якорем `ROW-*`. То есть факт в ячейке таблицы не
   становится spec unit. Это свойство движка (R-21, правка — только в
   авторскую копию), предшествующее этому пакету; кандидат в `BACKLOG.md`
   строкой `docs:`.
2. **`overloaded-item` бессмыслен для страниц** — см. выше; выключен
   конфигом пакета, но порог движка стоит перекалибровать, если doc-пакеты
   станут обычным делом.
3. **Доставка карты пакета не автоматизирована** (подтверждает открытый
   вопрос 3 из A0.6): `install` не зовёт `vibe specmap`, ни один слот
   `vibedeps/**` не несёт `package.specmap.json`. Карта руководства
   свежая на момент коммита; ничто не держит её свежей.

## Вывод гейтов, дословно

Сборка и тесты гонялись с приватным каталогом сборки
(`CARGO_TARGET_DIR=<scratch>/target-p2o6`) — дерево делят несколько
воркеров, и общий `target/debug/vibe.exe` они перелинковывают.

```
$ cargo fmt --all --check
(exit 0)

$ cargo build -p vibe-doc -p vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.14s

$ cargo test -p vibe-doc
running 158 tests
test result: ok. 158 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 3 tests      (tests/island.rs)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 5 tests      (tests/projections.rs)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 18 tests     (doctests)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p vibe-trace
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p vibe-cli --bins commands::doc
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 719 filtered out

$ cargo test -p vibe-cli --bins commands::specmap
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 720 filtered out

$ cargo test -p vibe-specdoc --test docs_corpus
test quarantined_pages_still_carry_exactly_the_recorded_defect ... ok
test every_page_is_refused_by_the_spec_reader ... ok
test docs_corpus_shape_is_counted ... ok
test non_canonical_pages_are_exactly_the_recorded_ones ... ok
test docs_xml_to_ir_to_xml_is_byte_idempotent ... ok
test doc_genre_is_not_round_trippable_through_markdown ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo clippy -p vibe-doc -p vibe-trace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.27s

$ cargo xtask conform check   # фильтр по моим путям
(ноль строк: ни одной находки в crates/vibe-doc/**, crates/vibe-trace/**,
 crates/vibe-cli/src/{cli,commands}/doc.rs, crates/vibe-cli/src/commands/specmap.rs)
conform: 26 crate(s) gated, 7 exempt
  — общий счётчик гейта красен предсуществующими чужими находками
    (progress-core/src/model.rs, vibe-specdoc/src/xml_doc.rs,
    vibe-cli/src/commands/vvm/error.rs и др.); первая версия
    citations.rs была 756 строк и попала в file-length — разбита на
    citations/{scan,anchor,sources}.rs, html.rs — на html/emit.rs.

$ cargo xtask specmap
  drift: edges added: 4
specmap: wrote …\specmap.json (7930 spec units, 3462 tagged code items,
         3007 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) …

$ vibe doc check --citations --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
citations: 381 rule(s), 389 address(es) to resolve, 2 self-address(es),
           0 unresolved, 6 placeholder(s), 2 teaching, 0 unreadable page(s)
(exit 0)

$ vibe specmap --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
wrote …\package.specmap.json under spec://org.vibevm.core/vibevm-docs/ —
  0 spec units, 43 tagged code items, 381 edges, 0 suspects, 4 warnings
  note: local specmap.toml namespace `vibevm-docs` is not globally unique;
        remapped to the package coordinate `org.vibevm.core/vibevm-docs`
  — четыре предупреждения разобраны в «Аномалиях», п. 1; закоммиченная
    карта совпадает с перегенерированной байт в байт.

$ vibe facts check --exhaustive
progress check: clean (380 files, 24 warning(s))
  — 380 файлов вместо 325 базовой линии: соседний воркер добавил страницы
    руководства в `include` файла facts.toml коммитом `153e68d9`. Ни один
    из моих файлов в корпус не входит (фикстура лежит под
    crates/vibe-doc/tests/, а глобы facts.toml прибиты к корню дерева).
```

## Список цитат, которые не разрешились

**Пустой.** Все 381 `rule` и все 8 адресов-цитат из прозы разрешились;
разбора «страница или спека» не потребовалось.

Первый прогон дал две строки `UNRESOLVED`, и обе оказались **дефектом
проверки, а не страницы**:

```
agent/how-agents-read-this-manual.xml:21 — spec://org.vibevm.core/vibevm-docs/&lt;page&gt;#&lt;anchor&gt
authoring/specs-agents-can-cite.xml:12   — spec://&lt;group&gt;/&lt;name&gt;[@&lt;version&gt
```

Обе — плейсхолдеры, написанные с экранированными угловыми скобками
(страница есть XML). Правило X-029 распознавало только литеральные `<` и
`>`. Исправлено в классификаторе, не на страницах; прозу я не трогал.

Три якоря, которые PP-C1 нашёл битыми (`PROP-054#WHY-C-ABI`,
`PROP-054#WASM`, ещё один `WHY-C-ABI`), сегодня разрешаются — их починили
в фазе P, и мой прогон это подтверждает.

## Что не сделано и почему

1. **`specmap.json` (хостовая карта) не закоммичен.** Гейт выполнен —
   0 suspects, 0 неразрешённых хостовых рёбер, — но регенерация в общем
   дереве втягивает дрейф параллельных воркеров. Файл возвращён к HEAD
   (`git checkout -- specmap.json`), как это сделали P2-O2 и P2-O4.
   **Интегратору:** после посадки всех пакетов фазы — один прогон
   `cargo xtask specmap` и один коммит `specmap.json`.
2. **Инъекция doc-сканера в хостовую карту (`xtask/src/specmap.rs`) не
   делалась.** PROP-057 `##PIPE-EDGES-HOST-SIDE` требует её в обеих точках
   xtask, «чтобы гейт покрытия не был зелёным от пустоты», — но это гейт
   покрытия, то есть **A2.16**, отдельный атом. Сегодня инъекции там и не
   нужно: хостовый `spec_roots = ["vibevm/vibespecs"]`, а руководство
   лежит под `vibevm/vibepacks/`, то есть в хостовую карту не попадает
   вовсе. Пакет прямо сказал писать рёбра **в карту doc-пакета**, что и
   сделано.
3. **Хостовый файл политики скана страниц не заводился.** Та же норма
   говорит, что политика живёт в хостовом файле, а не в `specmap.toml`
   (из-за `deny_unknown_fields`). Для карты **пакета** предмет этой
   политики — «где лежат страницы», и ответ даёт раскладка PROP-052
   (`pages::SPEC_ROOT`), а не конфиг. Файл понадобится, когда сканер
   пойдёт в хостовую карту (A2.16).
4. **`bash tools/self-check.sh` целиком не гонялся** — по постоянному
   указанию оркестратора (панель гонит `cargo run -p vibe-cli` и
   перелинковывает общий `target/`). Гейты панели, относящиеся к моим
   атомам, прогнаны их собственными командами (выше).
5. **Проза страниц, PROP-файлы, движок карты и
   `wire-derive-baseline.json` не менялись.** Чужие незакоммиченные файлы
   не стейджились; `git push` не делался.
6. **Шаг панели для `--citations` не добавлен** — пакет его не просил, а
   `tools/self-check.sh` трогают другие воркеры этой фазы. Команда для
   шага, если он понадобится:
   `vibe doc check --citations --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0`.

## `git status --short` на момент сдачи

Моих в списке ровно один: этот отчёт — он уходит следующим коммитом.
Больше в дереве не изменено ничего: параллельные воркеры к этому моменту
закоммитили своё (`09afc33a`, `93b0d7ff`, `153e68d9`, `9993148f`), а
`Cargo.lock` ушёл коммитом `cbd3b8e9` (разбор выше).

В течение работы в дереве были чужие незакоммиченные правки —
`crates/vibe-index/**`, `crates/vibe-cli/tests/**`,
`crates/vibe-cli/src/commands/init/**`, `crates/vibe-wire/**`,
`formats/**`, `xtask/src/codegen/**`, `specmap.json` и другие. Я к ним не
прикасался и не стейджил. Одно касание всё же было: **`cargo fmt --all`**
(гейт пакета формулирован как `--all`) мог переформатировать чужие файлы,
находившиеся в работе. Правок смысла это не вносит и в мои коммиты не
попало.

Каждый из трёх продуктовых коммитов сделан формой
`git commit … -- <пути>` (новые файлы предварительно добавлены `git add`
по тем же путям — `git commit -- <пути>` не берёт неотслеживаемые файлы);
проверено `git show --name-only` по каждому: чужих путей нет.
