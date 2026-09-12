# WORKER-REPORT-P4-O7 — остров пишет адреса сайта; читатель отдаёт картинки и страницу пакета; голова шаблона правится на месте

Пакет: `campaigns/docs-2026-09/findings/PACKET-P4-O7.md`.
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Три атома сделаны, три коммита. Главное число пакета взято: **660 → 0**.
Ни одна ссылка острова руководства больше не ведёт в форму, которой сайт
не несёт, и строка «citation(s) into documentation this site does not
carry» из вывода линтера **исчезла тоже** (51 → 0) — цитаты теперь
спрашивают резолвер, а резолвер сайт несёт. Линтер зелёный на 13 181
ссылке, 0 битых.

Три результата стоит прочитать отдельно.

1. **Преобразование адреса не нуждается в адресе текущей страницы, и это
   не экономия, а более сильное правило.** Страница, чей исходник лежит в
   `<документ>.xml`, обслуживается из каталога `<документ>/` — то есть
   ровно на один уровень глубже файла, относительно которого написаны её
   ссылки. Значит любой относительный адрес получает ровно один `../`, а
   цель, называющая страницу, меняет `.xml` на слэш. Ни базы, ни написания
   версии, ни `page_path` для этого не нужно — и, в отличие от счёта от
   `page_path`, это остаётся верным под языковым сегментом, за которым
   сайт монтирует адаптацию (D-06). Правка в `build.rs` не понадобилась
   вовсе (раздел «Решение 1»).
2. **Ссылки внутри цитируемого правила — не ссылки страницы.** 45 из 330
   относительных ссылок острова живут в теле `<rule>`: это текст факта,
   вытащенный из спеки, и его `../modules/vibe-mcp/PROP-027-…xml`
   осмысленно рядом с файлом, который это написал, и бессмысленно рядом
   со страницей, которая это цитирует. Они читаются от адреса
   **цитируемого документа** и становятся цитатами. 42 из 45 так и
   разошлись; оставшиеся 3 — пути в репозиторий (`LICENSE.md`,
   `project.rs`, `.md`-файл вне библиотеки) — не получают адреса вовсе.
3. **Вставка в голову вылечена по-настоящему, и тест — байтовый.**
   `relink` больше ничего не вставляет: он переписывает значения у
   элементов, которые шаблон уже объявил, по типу. Тест сравнивает
   последовательность тегов головы до и после вклейки — она обязана
   совпасть, — и отдельно проверяет, что шаблон без головы обе правки
   оставляют холостыми.

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| 1 | `9da13611` | `feat(doc): write island links as site addresses` |
| 2 | `b95b07d3` | `feat(doc): serve the edition's media and the package page` |
| 3 | `1ce25334` | `fix(doc): rewrite the template head in place` |

Между предусловиями пакета и моими коммитами в ветку легли чужие
(P5-O1: `fc488f7d`…`74ec32d0`, `2ad836ee`) — ожидаемо. Чужого
незакоммиченного не стейджил: перед каждым коммитом проверялся
`git status --short`, каждый коммит сделан явной формой
`git commit -m … -m … -- <свои пути>`, новые файлы добавлены поимённо.

## Числа ссылок острова, до и после

Живой прогон над руководством (`vibe doc build` три проекции → сборка
сайта web-пакета → его линтер). Сайт несёт оба написания версии, поэтому
все числа сайта вдвое больше числа на одно дерево.

| Мера | До | После |
|---|---|---|
| «in a form this site does not carry» | **660** | **0** |
| «citation(s) into documentation this site does not carry» | **51** | **0** |
| ссылок проверено | 13 187 | 13 181 |
| битых | 0 | 0 |

Разбор «660» по форме, как его печатал линтер до правки:
`index.xml` (548), `PROP-057-documentation-packages-and-site.xml` (14),
`PROP-024-code-bearing-packages.xml` (14),
`PROP-044-change-native-formats.xml` (12), `PROP-027-mcp-packages.xml` (8),
остальное — хвост из одиночных имён.

Разбор того же на **одном** дереве (330 = 660 / 2), измеренный по байтам
острова с границей `<blockquote class="rule">`:

| Где написана ссылка | Сколько | Во что превратилась |
|---|---|---|
| проза страницы, `../glossary/index.xml#термин` | 274 | `../../glossary/index/#термин` |
| проза страницы, соседние страницы (`first-project.xml`, `../model/two-trees.xml`, …) | 11 | адрес той страницы |
| тело цитируемого правила, `*.xml` спеки | 42 | `<base>resolve/?uri=spec://…` |
| тело цитируемого правила, файл репозитория | 3 | без адреса, `data-address` |

**51 «цитата за пределы библиотеки» стала нулём** не потому, что цитаты
исчезли, а потому, что каждая теперь ведёт в `/doc/resolve/`, который
сайт несёт. Сами адреса не потеряны: они лежат в параметре `uri` и в
`data-uri` блока.

## Атом 1 — решения

### Решение 1 — «на один уровень глубже», и поэтому адрес страницы не нужен

Пакет говорил считать адрес «от адреса текущей страницы (`page_path`)».
Это работает, но требует протащить `page.rel` через `render_page` —
единственную функцию, которая знает и страницу, и проекцию, — а вместе с
ней через `Content` или сигнатуру `to_html_numbered`. На момент начала
работы `crates/vibe-doc/src/build.rs` нёс **незакоммиченную правку
параллельного воркера** (P5-O1, `declared_kind`), и коммит по путям
утащил бы её в мой коммит. Это заставило искать правило, которому адрес
страницы не нужен, — и правило нашлось, оказавшись строго сильнее.

Доказательство в одну строку: страница, чей исходник `<документ>.xml`,
обслуживается из каталога `<документ>/`; значит обслуживаемая страница
стоит **ровно на один уровень глубже** каталога того файла, относительно
которого написаны её ссылки. Отсюда:

- относительный адрес получает ровно один `../`;
- цель, оканчивающаяся на `.xml`, меняет `.xml` на `/` (это и есть адрес
  той страницы, `##SITE-TRAILING-SLASH`);
- всё остальное — путь к файлу рядом со страницами — сохраняет имя.

Проверка на трёх формах пакета: `../glossary/index.xml#term` →
`../../glossary/index/#term`; `reference/tree.xml` → `../reference/tree/`;
`./x.xml` → `../x/`. Каждый из них верен под любой базой, в обоих
написаниях версии **и** под языковым сегментом адаптации — чего счёт от
`page_path` не даёт: `page_path` не знает, стоит ли перед координатой
`ru/`, и под адаптацией ошибся бы на уровень.

Практическое следствие: `build.rs` в атоме 1 не тронут вообще.

### Решение 2 — `index.xml` каталога остаётся страницей `index`

Пакет писал в примере `../glossary/#term`. Сайт так не адресует:
`site/src/lib/library.ts::documentOf` снимает с пути манифеста только
расширение, и `docHref` строит `…/<версия>/glossary/index/`. Свернуть
`index` здесь значило бы завести второе мнение о том, откуда сайт отдаёт
страницу, — при том что пакет сам делает адресную карту сайта
авторитетной («Rust обязан давать те же адреса»). Поэтому
`../glossary/index.xml#term` → `../../glossary/index/#term`, и это
подтверждено линтером: 0 битых на 13 181 ссылке.

### Решение 3 — цитата спрашивает резолвер, а не пишет карту сама

Адресная карта `##SITE-MOUNT` детерминированная, и остров **мог** писать
`spec://<группа>/<имя>/<документ>` прямо путём — и писал, в документацию,
которой стоящий перед ним монтаж не несёт (те самые 51). Конвейер этого
знать не может: публичный сайт несёт библиотеку, локальный читатель —
ровно один пакет. Резолвер — единственный адрес, который знает, и он есть
в обоих мирах (`##SEO-MANIFEST-AND-RESOLVER`, развилка F-15).

Форма: `<base>resolve/?uri=<цитата>`, фрагмент — **внутри** параметра,
`#` кодируется как `%23` (иначе браузер оставил бы фрагмент себе и отдал
резолверу половину адреса). Остальное из набора `spec://`-адреса — `:`,
`/`, `@` — не кодируется, чтобы адрес читался в строке состояния.

Два решения внутри формы:

- **адрес абсолютный (с базой), а не относительный.** Пакет писал
  «относительный адрес резолвера `<base>resolve?uri=…`»; сама запись с
  `<base>` — уже абсолютный путь, и это единственный верный вариант:
  остров адаптации публикуется на уровень глубже (`/doc/<язык>/…`), и
  относительный подъём, посчитанный конвейером, попал бы там в
  `/doc/<язык>/resolve/`. База же у `vibe doc build` — параметр прогона, и
  локальный читатель монтируется со своей.
- **со слэшем: `resolve/`, а не `resolve`.** Статический сайт пишет
  резолвер как `dist/doc/resolve/index.html` (P4-O3), то есть адрес
  страницы кончается слэшем; сервер P4-O4 принимает обе формы
  (`rest == ROUTE || rest == ROUTE + "/"`). Со слэшем верны оба.

`Content::link` — прежняя прямая карта — оставлена как есть: это
публичная документированная функция с doctest'ами, единственная
исполняемая запись карты `##SITE-MOUNT` в этом крейте. Остров её больше
не зовёт; удалять её значит потерять запись, а не код.

### Решение 4 — «кто написал ссылку» стало типом, а не догадкой

`html/links.rs`: `Links` с тремя конструкторами — `verbatim()` (словарь
и никакой карты; в этой форме живёт `inline::render`, и тесты словаря
остались словарными), `page(base)` (проза страницы) и
`quoting(base, uri)` (текст цитируемого документа). Это не украшение: что
ЗНАЧИТ относительный адрес — свойство того, кто его написал, а не
свойство Markdown, и до этого атома оба случая молча считались одним.

В режиме цитаты ссылка читается от каталога цитируемого документа, и
результат — снова цитата. `../../common/PROP-024-code-bearing-packages.xml#build`
в тексте факта из `modules/vibe-registry/PROP-002` становится
`spec://org.vibevm.core/vibevm/common/PROP-024-code-bearing-packages#build`
и уходит в резолвер. Путь, выбирающийся за пределы пакета, и файл, который
не спека (`project.rs`, `LICENSE.md`), адреса не получают.

### Решение 5 — адрес, который не удалось поставить, не становится ссылкой

`<a data-address="…">текст</a>`: подпись автора видна, адрес сохранён,
ссылки нет. Это та же идиома, которой остров уже пользуется у правила
(«без базы — `data-uri` и никакого `href`»).

**Имя атрибута выбрано после измерения.** Первая версия называлась
`data-href`, и линтер насчитал 6 «в форме, которой сайт не несёт» вместо
0: его выражение ищет `href` по границе слова, а в `data-href` перед
`href` стоит дефис — небуквенный символ, то есть граница. Переименование
в `data-address` дало 0. Для web-пакета это ничего не меняет (там правка
не нужна), но любой будущий сканер прочитал бы так же.

### Решение 6 — картинка обязана иметь источник

Для `<figure>` `href` не может отсутствовать: `<img>` без `src` — не
честный пробел, а дыра. Поэтому цель, которую сборка поставить не смогла,
у картинки сохраняет написание автора, и только у неё.

### Решение 7 — `media/` не особенный, и это подтверждает голден

Пакет ждал `media/<хэш>.<ext>` → `../../media/<хэш>.<ext>` «по глубине
страницы». В моей форме тот же результат получается сам, если ссылка
написана **относительно файла страницы**, как написаны все остальные
ссылки корпуса: `../media/x.png` со страницы глубины 1 →
`../../media/x.png` от `<издание>/<каталог>/<страница>/` →
`<издание>/media/x.png`. Именно туда P4-O3 раскладывает `media/**`.

Голая `media/diagram.svg` в исходнике страницы значит `<каталог
страницы>/media/diagram.svg` — и ровно это уже записано в `L-01`
линтера: «остров-голден цитирует `media/diagram.svg` относительно
страницы, а конвейер пишет картинки пакета в корень дерева». То есть
ссылка фикстуры была и остаётся неверной в исходнике; правило её не
чинит и не притворяется, что чинит. Юнит-тест на форму `media` есть
(`a_file_beside_the_pages_keeps_its_name_and_takes_the_same_climb`).

### Решение 8 — что не меняется

`.md`- и `.xml`-проекции не тронуты (голдены `guide-every-block.md` и
`.xml` не сдвинулись ни на байт); внешние `http(s)://`, `mailto:`,
адреса, уже написанные от корня сайта (`/doc/…`), и одиночный якорь
(`#p07`) проходят как есть.

### Разница голденов острова, построчно

Два файла, по три строки в каждом:

```
crates/vibe-doc/tests/golden/guide-every-block.html
-      <img src="media/diagram.svg" …/>
+      <img src="../media/diagram.svg" …/>
-      <a class="rule" href="/doc/com.example/subject/latest/common/PROP-001/#A-RULE" …>
+      <a class="rule" href="/doc/resolve/?uri=spec://com.example/subject/common/PROP-001%23A-RULE" …>
-      <a class="rule" href="/doc/com.example/subject/latest/common/PROP-001/#UNRESOLVED" …>
+      <a class="rule" href="/doc/resolve/?uri=spec://com.example/subject/common/PROP-001%23UNRESOLVED" …>
```

`guide-every-block.numbered.html` — те же три строки. `data-uri` у обоих
правил не изменился: адрес цитаты остаётся адресом цитаты.

Ссылка `[a link](/doc/com.example.docs/fixture-manual/latest/guide/every-block/)`
в фикстуре не изменилась — она уже написана от корня сайта.

## Атом 2 — решения

### Решение 9 — картинки: адрес матчится, а не собирается

`crates/vibe-doc-server/src/routes/media.rs`. Три проверки, и они
сильнее трёх проверок активов оболочки (J-102), потому что путь из байтов
запроса **не собирается вообще**:

1. имя — ровно один обычный сегмент (пусто, `.`, `..`, `/`, `\`, `\0`,
   `:` — отказ; двоеточие по той же причине, по которой его отвергает
   оболочка: на Windows `C:` — префикс тома);
2. адрес обязан быть одним из тех, что несёт **манифест**, то есть одним
   из `vibe_doc::media::slots` — той же функции, из которой
   `vibe doc build` пишет файлы и которой манифест их называет;
3. байты берутся из совпавшего слота, а не из файла, названного запросом.

Тип — по расширению, замкнутой таблицей (`##CARD-MEDIA-SOURCE` допускает
PNG, JPEG, WebP; плейсхолдеры — SVG и составленный PNG). Кэш — навсегда:
имя содержательное, изменившаяся картинка — изменившийся адрес.

**Два написания адреса отвечают.** Манифест, который отдаёт этот
читатель, лежит на `<base>`, поэтому адреса внутри него —
`<base>media/<имя>` (это то, что просил пакет и что показывает карточка).
Но **страница** доезжает до картинок своего издания подъёмом к корню
издания, а локально корень издания — `<base><координата>/<версия>/`.
Одна дорожка, один набор байт, оба адреса; иначе относительные адреса,
которые теперь пишет атом 1, локально бы не резолвились.

### Решение 10 — правило «какие байты у слота» получило один дом

`vibe_doc::media::Slot::render(coordinate, kind, title)`. До этого
трёхветочный `match` по роли жил в `build.rs`; повторить его в сервере
значило положить одно правило в два места, а симптомом расхождения была
бы картинка, которая молча меняется, если читать локально. Тест
сравнивает отданные байты с байтами, которые **написала сборка**, а не со
строкой в тесте.

`build.rs` переведён на эту функцию. Это единственная строка атома в
`build.rs`, и она стала возможна только потому, что к тому моменту
P5-O1 свою работу закоммитил, и файл был чист (см. «Аномалии», А-0).

### Решение 11 — страница пакета: пустой остров, и это конструкция

`<base><координата>/<версия>/` и `<base><координата>/latest/` отвечают
**200** шаблоном оболочки, в дыру которого подставлена пустая строка. За
страницей пакета нет документа — ни один `.xml` в пакете не говорит «это
пакет», — поэтому конвейеру нечего рендерить, а всё, что на ней
показывается (карточка, полка страниц в порядке чтения, `llms.txt`),
оболочка уже строит из `<base>manifest.json` (решение 11 отчёта P4-O6).
Маркер при этом **убирается**: оставить его значило бы показать его
читателю.

`latest` отвечает тем же ответом, а не редиректом: читатель наведён на
один пакет, значит `latest` может быть только этой версией, и два
написания — это один и тот же адрес, ровно как на сайте.

### Решение 12 — дверь `<base>` ведёт на страницу пакета (выбрано и названо)

Было: `302` на первую страницу (решение 6 P4-O4), потому что страницы
пакета не существовало. Стало: `302` на `<base><координата>/<версия>/`.
Дверь значит «эта документация», и теперь у документации есть своя
страница, чтобы это значить; первая глава с неё в один переход. Это же
делает осмысленной строку, которую сервер печатает при старте: она
называет `mount()`, то есть ровно этот адрес, и он теперь отвечает 200.

## Атом 3 — решения

### Решение 13 — `relink` переписывает значения, а не состав головы

`repointed()` находит у уже стоящего `<link>` атрибут `rel="alternate"`,
сверяет `type` с типом проекции и переписывает значения **на месте**,
сохраняя каждый другой байт тега — включая атрибуты каркаса вроде
`:="r5_N"`, которыми помечен каждый элемент резюмируемой головы. Атрибут,
которого в теге нет, не дописывается: длина головы — дело головы, а
значения в ней — дело сервера.

`retitle` так работал и раньше; теперь это сказано в его доке и закрыто
doctest'ом (шаблон без `<title>` возвращается как есть).

### Решение 14 — тест байтовый, и он про закон, а не про случай

`the_head_keeps_every_element_it_had_in_the_order_it_had_them`:
последовательность имён тегов документа до и после `relink` + `retitle`
обязана совпасть (число и порядок), и отдельно проверяется, что
изменились ровно значения — текст `<title>` и адреса трёх `alternate`, —
а `meta description`, таблица стилей и инлайновый скрипт остались
шаблонными. Второй тест: шаблон без головы после обеих правок **равен
себе побайтово**.

### Отклонение: `title` у `alternate` тоже переписывается

Пакет просил менять «только текст `<title>` и `href` трёх `alternate`».
Я переписываю на месте `href` **и** `title`. Причина — атомарность
коммитов, и она механическая: убрать поле `title` из `Alternate` значит в
том же коммите поправить вызовы в `crates/vibe-doc-server/src/routes.rs`,
а `routes.rs` — файл атома 2. При обязательной форме
`git commit -- <пути>` файл целиком попадает в один коммит, так что атомы
2 и 3 пришлось бы слить. Оставить поле неиспользуемым — мёртвый вес в
публичном типе.

Закон, который пакет формулирует, при этом соблюдён полностью: ничего не
вставляется, ничего не удаляется, ничего не переставляется — переписаны
значения элементов, которые шаблон уже объявил. Байтовый тест это и
проверяет. Само значение осмысленно: `title` у `alternate` — это слова,
которые браузер показывает в меню альтернативных версий ДОКУМЕНТА, и
шаблон несёт в них имя фикстуры.

### Заголовки модулей — обновлённые карты решений

- `crates/vibe-doc-shell/src/template.rs` — новый раздел «голова
  правится на месте, никогда не дополняется», с измерением (три
  `<link rel="alternate">` после `</title>` гасят резюмирование и с ним
  весь клиентский слой, молча).
- `crates/vibe-doc-shell/src/lib.rs` — «что он не делает»: каждая правка
  шаблона это ЗАМЕНА значения там, где оно уже стоит.
- `crates/vibe-doc-server/src/lib.rs` — «что он отдаёт»: страница пакета
  как оболочка вокруг пустого острова, картинки карточки, и почему
  резолвер теперь несёт вдвое больше веса.
- `crates/vibe-doc-server/src/routes.rs` — таблица адресов дополнена
  страницей пакета, `latest` и двумя написаниями `media/`.

## Вывод гейтов, дословно

### `cargo fmt --all --check`

```
$ cargo fmt --all --check
FMT_EXIT=0
```

### Тесты трёх крейтов, без фичи и с фичей

```
$ cargo test -p vibe-doc -p vibe-doc-server -p vibe-doc-shell
test result: ok. 554 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.65s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.47s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s
TEST_EXIT=0

$ cargo test -p vibe-doc -p vibe-doc-server -p vibe-doc-shell --features vibe-doc-shell/embedded-shell
test result: ok. 554 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.96s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.17s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.92s
EXIT=0

$ cargo test -p vibe-doc-shell --features vibe-doc-shell/embedded-shell   # фича действительно включена
test tests::the_embedded_shell_wins_over_everything_and_measures_to_its_pin ... ok
```

Тестов вибе-doc было 553, стало 554 (юниты модуля адресов считаются
файлом `links/tests.rs` — 9 тестов — плюс 4 doctest'а; в сумме doctest'ов
62 → 64). В `vibe-doc-server` было 30, стало 34 (картинки, отказы
картинок, страница пакета в обоих написаниях, дверь). В `vibe-doc-shell`
было 17 юнитов, стало 19 (байтовый тест головы и шаблон без головы);
doctest'ов 10 → 11.

### `cargo clippy`

```
$ cargo clippy -p vibe-doc -p vibe-doc-server -p vibe-doc-shell --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.37s
EXIT=0
```

### `cargo xtask conform check`

```
$ cargo xtask conform check --scope crates/vibe-doc
conform: policy conform.toml (loaded).
conform: extracted 8 file(s), 2322 cached (producer rust-syn-11).
conform check: 0 finding(s) in scope crates/vibe-doc ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report.sarif.
conform: 28 crate(s) gated, 7 exempt — see conform.toml for the why of each.

$ cargo xtask conform check --scope crates/vibe-doc-server
conform check: 0 finding(s) in scope crates/vibe-doc-server ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report.sarif.

$ cargo xtask conform check --scope crates/vibe-doc-shell
conform check: 0 finding(s) in scope crates/vibe-doc-shell ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report.sarif.
```

### `cargo xtask specmap`

```
$ cargo xtask specmap
  drift: edges added: 20
specmap: wrote …\specmap.json (7930 spec units, 3600 tagged code items, 3112 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.

$ git checkout -- specmap.json
```

0 suspects; файл возвращён к HEAD, в коммиты не попал.

### `cargo xtask check-codegen`

```
$ cargo xtask check-codegen
xtask check-codegen: clean.
```

Схемы не трогал — это контроль. Первый запуск упал по чужой блокировке
файла, разбор в А-3.

### Живой прогон: три дерева руководства → сборка сайта → линтер

```
$ VIBE_DOC_OUT="<scratch>\manual-html;<scratch>\manual-md;<scratch>\manual-xml" node tools/build.mjs static
build (static): generated 103 page(s), expected 103
build (static): removed dist/q-manifest.json from the output
build (static): 96 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 206 file(s) copied from 3 documentation tree(s) for 1 edition(s) (0 page(s) in a language that does not carry them); 10 written — catalogue, manifests, 2 sitemap part(s) over 50 address(es), resolver
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 216 inline script hash(es) over 716 occurrence(s), no external source
documentation links — every address against the files behind it
    ok        104 page(s), 497 file(s) in the output
    ok        13181 link(s) followed, 54 hreflang pair(s) checked
    ok        51 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    ok        7 outbound link(s) to 1 host(s) the prose cites, fetched by nothing: datatracker.ietf.org (7)
links: green — 13181 checked, 0 broken.
build (static): ok
```

Строк «in a form this site does not carry» и «citation(s) into
documentation this site does not carry» в выводе **нет**: линтер печатает
их только при ненулевом счёте.

До правки тот же прогон давал:

```
    ok        13187 link(s) followed, 54 hreflang pair(s) checked
    note      660 link(s) the pipeline wrote inside an island in a form this site does not carry: index.xml (548), PROP-057-documentation-packages-and-site.xml (14), PROP-024-code-bearing-packages.xml (14), PROP-044-change-native-formats.xml (12), PROP-027-mcp-packages.xml (8)
    note      51 citation(s) into documentation this site does not carry:
              /doc/org.vibevm.ai-native/core-ai-native/latest/mechanisms/PROP-014/
              …
links: green — 13187 checked, 0 broken.
```

### Сборка по умолчанию (фикстура) — не сдвинулась

```
$ node tools/build.mjs static
build (static): generated 19 page(s)…
    ok        646 link(s) followed, 18 hreflang pair(s) checked
    note      1 citation(s) into documentation this site does not carry:
              /doc/com.example/subject/latest/common/PROP-001/
    L-01      4x — The island golden cites `media/diagram.svg` relative to the page, …
links: green — 646 checked, 0 broken.
build (static): ok
```

Числа фикстуры прежние, и это ожидаемо: острова фикстуры в web-пакете —
статические байтовые копии голденов Rust, а web-пакет я только читаю.
Разбор — А-5.

### `vibe doc serve` над руководством

Бинарник на момент прогона — с фичей `embedded-shell`
(`"provenance": "embedded"`, 139 файлов, 796 671 байт). Запросы сделаны
`curl --path-as-is` (без него клиент нормализует `..` сам — А-4).

```
код тип                        размер  адрес
200  image/svg+xml              622     <base>media/e65e5833a9f1d438.svg
200  image/svg+xml              3474    <base>media/2778b64929450ffd.svg
200  image/png                  61685   <base>media/d54e7cc3e2097d60.png
200  image/svg+xml              622     <base>org.vibevm.core/vibevm-docs/0.1.0/media/e65e5833a9f1d438.svg
200  image/png                  61685   <base>org.vibevm.core/vibevm-docs/latest/media/d54e7cc3e2097d60.png
404  application/json                   <base>media/nothing.svg
400  application/json                   <base>media/..%2F..%2Fvibe.toml
400  application/json                   <base>media/%2e%2e
400  application/json                   <base>media/C:
200  text/html; charset=utf-8   35901   <base>org.vibevm.core/vibevm-docs/0.1.0/
200  text/html; charset=utf-8   35901   <base>org.vibevm.core/vibevm-docs/latest/
302  → <base>org.vibevm.core/vibevm-docs/0.1.0/    <base>
302  → <base>org.vibevm.core/vibevm-docs/0.1.0/glossary/index/#manifest
     <base>resolve/?uri=spec%3A%2F%2F…%2Fglossary%2Findex%23manifest
```

Последняя строка — проверка круга: адрес, который теперь пишет остров для
цитаты, ведёт в резолвер, и резолвер приводит на ту страницу с тем
якорем.

### Playwright web-пакета

```
$ pnpm test:e2e
  34 passed (36.5s)
```

`local-reader.spec.ts` — 5 тестов из этих 34, все зелёные; остальные 29 —
чужие спеки сайта. (В отчёте P4-O6 «34 теста» были всей панелью; состав
с тех пор изменился, число совпало случайно.) Первый прогон был красным
по чужой причине — А-2.

### `vibe doc check` по руководству

```
$ vibe doc check --path <руководство> --examples --citations --coverage --media --style --min 100
examples: 55 matched, 0 captured, 0 re-blessed, 0 differ, 0 failed, 7 skipped, 0 unreadable page(s)
citations: 759 rule(s), 767 address(es) to resolve, 2 self-address(es), 0 unresolved, 9 placeholder(s), 2 teaching, 0 unreadable page(s)
style: 48 of 48 page(s) clean, 100% (threshold 100%), 0 error(s), 119 warning(s), 0 unreadable page(s) [en]
media: 0 declared (none), 0 error(s), 0 warning(s), 3 role(s) generated
coverage: 516 of 516 obligation(s) told, 100% of 571 audience pair(s) (threshold 100%), 0 unreadable page(s)
EXIT=0
```

С добавленным `--derived` прогон **красный**, и по чужой причине:

```
  CHANGED reference/commands.xml#cli-help:vibe doc --help — `vibe doc --help`
derived: 71 unchanged, 1 moved
error: a `derived` reference no longer builds to what the record holds (violates spec://org.vibevm.core/vibevm/common/PROP-045#ROW-DOCVOCAB-DERIVED-CHECK; fix: read what moved, update the prose around it, and re-run with --accept)
```

Разбор — А-6.

## Аномалии

### А-0. `build.rs` был занят чужой незакоммиченной работой, и это изменило конструкцию атома 1

На старте `git status` показывал незакоммиченную правку P5-O1 в
`crates/vibe-doc/src/build.rs` (`declared_kind`). Обязательная форма
`git commit -- <пути>` берёт файл из рабочего дерева целиком, то есть
любой мой коммит с этим путём утащил бы чужую работу под моим сообщением.
Вариантов «взять только свои куски» при этой форме нет:
`git apply --cached` кладёт в индекс, а `git commit -- <путь>` индекс
игнорирует; временно снимать чужую правку — гонка с живым воркером.

Поэтому атом 1 был спроектирован так, чтобы `build.rs` не понадобился
вовсе, — и именно этот поиск дал правило «на один уровень глубже»,
которое оказалось строго сильнее счёта от `page_path` (решение 1). К
моменту атома 2 P5-O1 свою работу закоммитил (`fc488f7d`…`74ec32d0`), и
единственная строка в `build.rs` — перевод на `Slot::render` — сделана
уже по чистому файлу.

Вывод для будущих пакетов: список «чужого периметра» в пакете назывался
`crates/vibe-doc/src/site/**`, а воркер работал шире. Периметр стоит
проверять `git status`, а не только читать.

### А-1. `cargo build --features vibe-doc-shell/embedded-shell` отрапортовал успех, оставив бинарник без оболочки

Первая попытка собрать `vibe` с фичей упала: `error: failed to remove
file target/debug/vibe.exe … Access is denied (os error 5)` — файл держал
**чужой** `vibe doc build-site` (J-105: чужие процессы не трогал, ждал).
После того как чужой процесс закончился, повторные запуски той же команды
завершались `Finished`, пересобирая `vibe-cli`, но печатая
`Fresh vibe-doc-shell` — и получавшийся бинарник отвечал
`"provenance": "fallback"`. То есть **гейт был бы пройден по зелёному
выводу на бинарнике без оболочки**: страница пакета отдавалась запасным
шаблоном (608 байт вместо 35 901), и живая проверка меряла не то.

Лечение: остановить **свои** процессы, державшие `vibe.exe`, тронуть
`crates/vibe-doc-shell/src/lib.rs` и пересобрать. После этого
`--print-shell` даёт `embedded`, 139 файлов, и тот же адрес отдаёт
35 901 байт. Дешёвый признак, которым стоит начинать любую живую проверку
оболочки: `vibe doc serve --print-shell` до первого запроса.

### А-2. `pnpm test:e2e` был красным из-за мусора в `site/dist`, оставленного моей же сборкой

Первый прогон панели: 5 зелёных, 29 красных — «element(s) not found»,
«Expected 200, Received 404». Причина не в коде: перед ним я собирал сайт
над деревьями руководства (103 страницы), а затем — фикстурную сборку (19
страниц), и `tools/build.mjs static` **не чистит `dist`**. Линтер при
этом печатает свои числа (19 страниц, 233 файла) и зовётся зелёным, так
что загрязнение ему невидимо.

После `rm -rf site/dist` и пересборки — `34 passed`. Правило для
следующих прогонов: сборка над другой библиотекой требует чистого `dist`,
и это стоит либо автоматизировать в сборке, либо записать рядом с
`VIBE_DOC_OUT`.

### А-3. `cargo xtask check-codegen` упал один раз по чужой блокировке

```
Error: publishing complete generated tree …\crates/vibe-wire/src/generated
Caused by: installing fresh generated tree … failed: Access is denied. (os error 5); restored the complete old tree
```

Повтор — `clean`. Дерево генератор восстановил сам (`git status` после
прогона чист по этому пути). Схем я не трогал.

### А-4. HTTP-клиент .NET нормализует `..` в адресе до отправки

`Invoke-WebRequest` на `<base>media/%2e%2e` дал `302` на страницу пакета:
клиент сам свернул `…/media/..` в `<base>`. Сервер этот адрес не видел.
С `curl --path-as-is` тот же адрес даёт `400`, как и юнит-тест
(`a_picture_that_was_never_published_is_refused_and_never_looked_for`).
Проверки обхода нужно делать клиентом, который не «улучшает» путь.

### А-5. Фикстурные копии острова в web-пакете разошлись с голденами Rust

`site/src/fixtures/island.html` и
`site/src/fixtures/doc-build/com.example.docs/fixture-manual/0.1.0/guide/every-block/index.html`
— байтовые копии голденов `guide-every-block.html` /
`…numbered.html` (решение 6 отчёта P4-O6, сверено тогда `diff`).
Голдены сдвинулись, копии — нет, и web-пакет я только читаю. Практически
это значит, что фикстурная сборка сайта всё ещё показывает `L-01` (4
попадания) и одну «цитату за пределы библиотеки»; перекопировать два
файла — и оба исчезнут. Это отдельная правка в чужом периметре.

### А-6. `vibe doc check --derived` красный по чужой правке CLI

Запись `maintenance/derived.json` руководства не знает подкоманды
`vibe doc build-site`, которую добавил P5-O1 (`fc488f7d`…). Проверка
честно говорит `1 moved`, и единственное расхождение — `vibe doc --help`.
Принимать (`--accept`) не стал: это правка пакета руководства, вызванная
чужой фичей, и принимать её должен тот, кто фичу сделал. Все остальные
оси `vibe doc check` зелёные (прогон выше, EXIT=0).

## Что не сделано и почему

1. **`--derived` не принят** (А-6) — чужая правка CLI, чужая приёмка.
2. **Фикстуры web-пакета не перекопированы** (А-5) — web-пакет в этом
   пакете только читается. После перекопирования из фикстурной сборки
   уйдут `L-01` (4x) и последняя строка «1 citation(s) into documentation
   this site does not carry».
3. **`<base><координата>/latest/<документ>/` по-прежнему 404.** Пакет
   просил `latest` только для страницы пакета, и сделано ровно это.
   Ассиметрия («полка по `latest` открывается, а страница с неё — нет»)
   названа здесь как кандидат в следующий атом; относительные адреса
   острова от этого не зависят — они работают в обоих написаниях по
   построению.
4. **Сборка каждого коммита по отдельности не проверялась.** В дереве всё
   время лежит незакоммиченная работа P5-O1 (сейчас — десять файлов в
   `site/src/**`), а проверка потребовала бы `git checkout` на коммит,
   то есть трогать их файлы. Независимость проверена по составу: атом 1
   не трогает ни `build.rs`, ни сервер, ни оболочку; атом 2 зовёт
   `template::relink` с той же сигнатурой, что в HEAD (поле `title` у
   `Alternate` сохранено именно ради этого — см. отклонение в атоме 3);
   атом 3 трогает только крейт оболочки и не зависит от атомов 1–2.
5. **Всю панель `tools/self-check.sh` не гонял** — только гейты пакета.
6. **Линтер ссылок не научен считать `data-address`** — и не должен: это
   не ссылка. Но стоит помнить, что его выражение ищет `href` по границе
   слова, поэтому имя атрибута с дефисом перед `href` он читает как
   ссылку (решение 5).

## Состояние дерева на момент сдачи

- `git status --short` по моим путям — пусто: всё в трёх коммитах.
  В дереве остаётся незакоммиченная работа P5-O1 в
  `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/**` (10 файлов).
- **Бинарник дерева возвращён в состояние без фичи**: `cargo build -p
  vibe-cli`, `--print-shell` отвечает `"provenance": "fallback"` — как
  просил пакет и как собирает панель.
- `crates/vibe-doc-shell/shell/` не тронут (собран P4-O6, в `.gitignore`).
- `site/dist` оставлен чистой фикстурной сборкой (19 страниц), на которой
  панель Playwright зелёная.
- `specmap.json` возвращён к HEAD.
- Своих процессов не осталось; чужие не останавливал (J-105) — ждал.
  Три дерева руководства из scratch удалены.

## Хвост — `latest` как алиас обслуживаемой версии

Один атом, один коммит: **`985e2a56` `feat(doc): serve latest as an alias
of the served version`**.

### Выбор: 200, а не 302 — и почему

Пакет хвоста разрешал оба ответа и просил назвать выбранный. Выбран
**тот же ответ, что у нумерованного адреса**, а не редирект на него, по
трём причинам в порядке веса:

1. **Сайт отвечает 200 на обоих написаниях.** P4-O6 мерил это числом: 96
   островов = 48 страниц × 2 написания версии. Читатель, который на том
   же адресе даёт редирект, показывает локально не то, что показывает
   веб, — а весь смысл общей адресной карты в том, что ссылка работает в
   обоих мирах одинаково.
2. **Переключатель версий оболочки (P4-O2) предлагает именно `latest`.**
   Редирект вернул бы номер в адресную строку, и переключатель читался бы
   как кнопка, которая ничего не делает: читатель нажал «latest», а адрес
   остался прежним.
3. **Относительные адреса острова (атом 1) верны под обоими написаниями
   по построению.** Уводить читателя из выбранного им написания нечем
   оправдать: выигрыша нет, а плата — лишний круг на каждый ресурс под
   `latest`.

### Решение 15 — перевод делается один раз, до всех дорожек

`routes::as_numbered` заменяет ведущий `<координата>/latest/` на
`<координата>/<версия>/` сразу после снятия базы, и дальше **ни одна
дорожка не знает, что у версии два имени**. Поэтому под `latest`
отвечает не «четыре формы, которые кто-то не забыл перечислить», а всё,
что отвечает вообще: страницы, обе проекции, починка потерянного слэша,
картинки карточки, страница пакета, резолвер и отказы.

Побочный эффект — упрощение: `routes/media.rs` носил второе написание
сам (`latest_prefix` в `requested`), и эта ветка исчезла; условие
страницы пакета из атома 2 свернулось с двух сравнений до одного.
`Reader::latest_prefix()` остался, и теперь у него ровно один читатель —
сам переводчик.

`latest` читается **только на позиции версии**: `…/0.1.0/latest/` —
по-прежнему 404 с именем `latest.xml`, потому что там стоит документ, а
не версия. Это отдельный тест.

### Решение 16 — тест формулирует закон, а не таблицу статусов

`latest_answers_as_the_version_it_stands_for_on_every_address`: для
восьми форм сравниваются **статус, тип, `Location` и байты** двух
написаний. Список ожидаемых статусов был бы согласен с читателем, который
случайно прав на тех четырёх формах, которые кто-то выписал; равенство —
нет.

Рядом сторож `the_picture_the_alias_case_names_is_one_this_package_publishes`:
имя картинки в списке форм сверяется с тем, что сборка действительно
пишет. Он **сработал при первом прогоне** — я взял хэш наугад, и без
сторожа случай «картинка под `latest`» тихо сравнивал бы два одинаковых
404 и проходил бы всегда.

### Живой прогон: обе колонки совпали на всех формах

`vibe doc serve` над руководством, оболочка настоящая
(`"provenance": "embedded"`), запросы `curl --path-as-is`:

```
совпало | нумерованный                             | latest                                   | адрес
same    | 200 text/html; charset=utf-8 44005       | 200 text/html; charset=utf-8 44005       | start/index/
same    | 200 text/markdown; charset=utf-8 5506    | 200 text/markdown; charset=utf-8 5506    | start/index.md
same    | 200 application/xml; charset=utf-8 3668  | 200 application/xml; charset=utf-8 3668  | start/index.xml
same    | 308                                      | 308                                      | start/index
same    | 200 text/html; charset=utf-8 35901       | 200 text/html; charset=utf-8 35901       | (страница пакета)
same    | 200 image/svg+xml 622                    | 200 image/svg+xml 622                    | media/e65e5833a9f1d438.svg
same    | 308                                      | 308                                      | llms.txt
same    | 404 application/json 123                 | 404 application/json 123                 | model/nope/
```

Обе починки слэша ведут на **нумерованный** адрес — перевод происходит
раньше починки, то есть канонической формой читателя остаётся номер:

```
0.1.0   start/index  308 -> <base><координата>/0.1.0/start/index/
latest  start/index  308 -> <base><координата>/0.1.0/start/index/
```

### Находка: `<координата>/<версия>/llms*.txt` — тупик в обоих написаниях

`…/0.1.0/llms.txt` отвечает `308` на `…/0.1.0/llms.txt/`, а тот — `404`
(«this documentation carries no page `llms.txt.xml`»). Это **было так и
до хвоста**: локальный читатель отдаёт агентские файлы только в корне
монтажа (`<base>llms.txt` → `200 text/plain`, 17 900 байт), а под
координатой их нет. Сработала общая развилка `page()`: адрес с точкой,
чьё расширение не `.md` и не `.xml`, считается «страницей, потерявшей
слэш».

Алиас этот адрес **не чинит и не ломает** — он делает оба написания
одинаковыми, как и просил пакет («отвечают тем же, что нумерованный
адрес»). Но у `latest` ответ поменялся с честного `404` на `308` в тот же
`404`, то есть на один прыжок длиннее, и это стоит знать.

Чинить в этом атоме не стал: A4.5 говорит, что **сайт** копирует
`llms*.txt` дерева в `/doc/…/<версия|latest>/`, то есть правильный ответ —
отдавать четыре тира и под координатой тоже, и это решение о том, где
живут агентские поверхности локально (`##SEO-LLMS-FILES`), а не о
написании версии. Правка на три строки: `machine_file` должен принимать
имя и после снятия `reader.prefix`. Кандидат в следующий атом; тест
алиаса на форму `llms.txt` уже стоит и переживёт починку — он сравнивает
две колонки, а не статус.

`manifest.json` под координатой — та же форма и тот же тупик, но его
сайт под изданием и не публикует (A4.5: «не копируется; сайт пишет его
сам»), так что чинить там нечего — разве что отвечать честным 404.

### Гейты хвоста, дословно

```
$ cargo fmt --all --check
FMT_EXIT=0

$ cargo clippy -p vibe-doc -p vibe-doc-server -p vibe-doc-shell --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.33s
CLIPPY_EXIT=0

$ cargo test -p vibe-doc -p vibe-doc-server -p vibe-doc-shell
test result: ok. 554 passed; 0 failed; …
test result: ok. 5 passed; …
test result: ok. 3 passed; …
test result: ok. 5 passed; …
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 19 passed; …
test result: ok. 64 passed; …   (doctest vibe-doc)
test result: ok. 6 passed; …    (doctest vibe-doc-server)
test result: ok. 11 passed; …   (doctest vibe-doc-shell)
EXIT=0

$ cargo test -p vibe-doc -p vibe-doc-server -p vibe-doc-shell --features vibe-doc-shell/embedded-shell
… те же девять строк, 37 в vibe-doc-server …
EXIT=0

$ cargo xtask conform check --scope crates/vibe-doc-server
conform check: 0 finding(s) in scope crates/vibe-doc-server ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report.sarif.

$ cargo xtask conform check --scope crates/vibe-doc
conform check: 0 finding(s) in scope crates/vibe-doc ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report.sarif.

$ cargo xtask specmap
specmap: wrote …\specmap.json (7930 spec units, 3600 tagged code items, 3112 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.
$ git checkout -- specmap.json
```

Тестов в `vibe-doc-server` было 34, стало **37**: закон алиаса, сторож
имени картинки и «`latest` читается только на позиции версии».

### Playwright хвоста

```
$ npx playwright test -c site/tests/playwright.config.ts site/tests/local-reader.spec.ts
  5 passed (14.8s)
```

Полная панель (`pnpm test:e2e`) на момент хвоста красная не по моей
причине и **не воспроизводимо моим кодом**: P5-O1 держит в дереве 17
изменённых и 3 новых файла web-пакета (включая `tools/build.mjs`,
фикстуры и `.gitignore`), суд — над собранным `site/dist`, а статическая
сборка `vibe` не зовёт вообще (в `tools/build.mjs` единственный
`spawnSync` — это `vite`). Мой диффы этого хвоста — только Rust. Зелёной
я видел эту панель час назад на `34 passed`; сейчас в ней 38 зелёных и
новые чужие спеки (`libraries.spec.ts`), падающие на чужом незакоммиченном
коде. Спека, которая меряет мой периметр, зелёная.

### А-7. Осиротевший превью-сервер моего же прогона держал порт

После красного `pnpm test:e2e` остался `node serve.mjs 4173` без
родителя, и следующий запуск Playwright отказался стартовать
(«already used»). Процесс мой (время создания совпадает с моим прогоном,
цепочка родителей мертва) — остановлен. Чужих процессов не трогал:
J-105 про чужие, а не про сирот своего прогона.

### А-6 закрыта чужой рукой

Запись `derived` руководства перезаписана коммитом `c719ce29`
(`docs(vibevm-docs): re-record the vibe doc help after the registry
builder landed`) — тем, кто добавил `vibe doc build-site`, как и
следовало. Проверка после хвоста:

```
$ vibe doc check --path <руководство> --derived
derived: 72 unchanged, 0 moved
EXIT=0
```

Пункт 1 раздела «Что не сделано» тем самым снят.

### Состояние дерева после хвоста

- `git status --short` по `crates/**` — пусто: всё в коммите `985e2a56`.
- Бинарник дерева **снова без фичи** (`cargo build -p vibe-cli`,
  `--print-shell` → `"provenance": "fallback"`).
- `specmap.json` возвращён к HEAD.
- Своих процессов не осталось (`vibe.exe` — 0), чужих не останавливал.

## Хвост 2 — агентские файлы под изданием

Один атом, один коммит: **`3c96b78c` `feat(doc): serve the agent files
under the edition too`**. Это закрывает находку хвоста 1.

### Проверка по `tools/build.mjs`: `manifest.json` под издание сайт кладёт

Пакет хвоста просил свериться. Ответ — **да, кладёт**, и это две разные
руки в одной сборке:

- `tools/doc-surfaces.mjs::copySurfaces` под `base` издания копирует
  `<документ>.md`, `<документ>.xml`, четыре имени из `LLMS_FILES` и
  `media/**` — но `manifest.json` в этом цикле нет;
- `tools/build.mjs` (строки «The page manifest of each edition, at both
  spellings of the version») **пишет** `${base}manifest.json` отдельно,
  пересериализуя манифест, который написал конвейер.

Проверено и по выходу: в `dist/doc/<координата>/0.1.0/` лежат
`index.html`, `llms.txt`, `llms-full.txt` (фикстура несёт два тира из
четырёх) и `manifest.json`. Код, который его пишет, **закоммичен** — это
не незакоммиченная работа P5-O1 (её диф `tools/build.mjs` этих строк не
трогает).

Отсюда: под изданием читатель отдаёт **пять** файлов — четыре тира и
манифест. Таблица A4.5 в отчёте P4-O3 («`manifest.json` не копируется»)
верна про `copySurfaces` и неполна про сборку целиком; строчку стоит
поправить, когда до неё дойдут руки.

### Решение 17 — одно снятие префикса, и адрес перестаёт быть двумя адресами

`machine_file` снимает `reader.prefix` с адреса перед тем, как сверить
имя. Всё остальное — прежнее: те же пять имён, те же байты, тот же
рендер. `latest` не стоил ничего: адрес приходит сюда уже переведённым в
номер (`as_numbered`, хвост 1), так что три написания одного файла
(`<base>`, `<base><координата>/<версия>/`, `…/latest/`) — это один
ответ, а не три реализации.

Локально пять файлов одинаковы в корне и под изданием — потому что
читатель наведён на **один** пакет, и «поверхности издания» и
«поверхности монтажа» здесь одно и то же. На сайте это два разных файла
(`/doc/manifest.json` — каталог всех изданий, `<издание>/manifest.json` —
страницы одного), и это не расхождение: локальный читатель несёт ровно
одно издание, так что его каталог и есть его издание.

### Решение 18 — починка слэша спрашивает, страница ли это, а не похоже ли

Развилка `page()` срабатывала на **любом** адресе с точкой, чьё
расширение не `.md` и не `.xml`, и отправляла запрос на файл, которого
читатель не отдаёт, на прыжок дальше — чтобы отказать там:
`…/llms.txt` → `308` → `…/llms.txt/` → `404` про страницу
`llms.txt.xml`. Теперь `308` выдаётся, только если пакет действительно
несёт `<адрес>.xml`; иначе — `404` там, где спросили, с именем в
сообщении. Набор страниц уже читался строкой ниже, так что лишнего
чтения нет — только порядок.

Это та самая находка хвоста 1 («тупик в обоих написаниях»), и закрыта
она с двух сторон: адрес `llms*.txt` под изданием теперь **отвечает**, а
имя, которого нет, отказывает без прыжка.

### Решение 19 — тестовая ячейка разрезана по шву «адреса одного издания»

`conform check` поймал файл: `routes/tests.rs` дорос до 615 строк при
бюджете 600. Разрез — по предмету, а не по размеру: в
`routes/tests/editions.rs` уехало всё про **адреса одного издания в обоих
написаниях версии** — страница пакета, закон алиаса, сторож имени
картинки, «`latest` читается только на позиции версии», агентские файлы
под изданием и отказ без прыжка. Харнесс (`package`, `reader`,
`get_path`, `get_bytes`) остался в родителе; ни один тест не переписан.
Родитель — 449 строк, ячейка — 185.

### Живой прогон

`vibe doc serve` над руководством, оболочка настоящая, `curl --path-as-is`:

```
файл             корень                              <версия>                            latest
manifest.json    200 application/json 32855          200 application/json 32855          200 application/json 32855
llms.txt         200 text/plain 17900                200 text/plain 17900                200 text/plain 17900
llms-small.txt   200 text/plain 127469               200 text/plain 127469               200 text/plain 127469
llms-medium.txt  200 text/plain 509607               200 text/plain 509607               200 text/plain 509607
llms-full.txt    200 text/plain 915341               200 text/plain 915341               200 text/plain 915341
nothing.txt      404                                 404                                 404
vibe.toml        404                                 404                                 404
```

Байты `manifest.json` во всех трёх адресах сверены `cmp` — **identical**.

Отказ без прыжка и сохранённая починка:

```
0.1.0/nothing.txt  -> 404 redirect=''
latest/nothing.txt -> 404 redirect=''
0.1.0/start/index  -> 308 <base><координата>/0.1.0/start/index/
latest/start/index -> 308 <base><координата>/0.1.0/start/index/
страница latest/start/index/          200 text/html 44005
картинка latest/media/<хэш>.svg       200 image/svg+xml 622
```

### Гейты хвоста 2, дословно

```
$ cargo fmt --all --check
FMT_EXIT=0

$ cargo clippy -p vibe-doc -p vibe-doc-server -p vibe-doc-shell --all-targets -- -D warnings
CLIPPY_EXIT=0

$ cargo test -p vibe-doc -p vibe-doc-server -p vibe-doc-shell
test result: ok. 554 passed; … / 5 / 3 / 5 / 39 passed / 19 / 64 / 6 / 11
EXIT_A=0

$ cargo test … --features vibe-doc-shell/embedded-shell
те же девять строк, 39 в vibe-doc-server
EXIT_B=0

$ cargo xtask conform check --scope crates/vibe-doc-server
  conform: NEW file-length crates/vibe-doc-server/src/routes/tests.rs:1 — violates REQ discipline://rust-ai-native-lang/guide#surface-form: 615 lines exceeds the 600-line file budget; fix surface: split along the file's responsibility seams into module-grain cells
conform check: 1 finding(s) in scope crates/vibe-doc-server ({"file-length": 1}), 0 frozen in baseline, 1 new
… после разреза:
conform check: 0 finding(s) in scope crates/vibe-doc-server ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report.sarif.

$ cargo xtask conform check --scope crates/vibe-doc
conform check: 0 finding(s) in scope crates/vibe-doc ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report.sarif.

$ cargo xtask specmap
specmap: wrote …\specmap.json (7930 spec units, 3600 tagged code items, 3112 edges, 0 suspects, 31 warnings).
$ git checkout -- specmap.json

$ npx playwright test -c site/tests/playwright.config.ts site/tests/local-reader.spec.ts
  5 passed (14.6s)
```

Тестов в `vibe-doc-server` было 37, стало **39**: агентские файлы под
изданием в обоих написаниях и отказ без прыжка. Плюс два адреса
добавлены в закон алиаса хвоста 1 (`llms.txt` и `nothing.txt`), и его
комментарий про «расхождение с сайтом» снят — расхождения больше нет.

### Состояние дерева после хвоста 2

- `git status --short` по `crates/**` — пусто: всё в коммите `3c96b78c`.
- Бинарник дерева **без фичи** (`--print-shell` → `"provenance":
  "fallback"`).
- `specmap.json` возвращён к HEAD.
- Своих процессов не осталось (`vibe.exe` — 0), чужих не останавливал.

### Что осталось после всех трёх хвостов

1. **Фикстурные копии острова в web-пакете** всё ещё байтовые копии
   старых голденов (А-5) — чужой периметр.
2. **Строчка A4.5 в отчёте P4-O3** неполна: `manifest.json` под издание
   сайт кладёт, просто не в `copySurfaces`.
3. **Полная панель Playwright** красна на чужих спеках поверх чужой
   незакоммиченной работы; спека моего периметра зелёная.

## R-25

Путей вне репозитория в отчёте нет: scratch обозначен `<scratch>`, база
читателя — `<base>`, каталог руководства — `<руководство>`. Секретов,
токенов, `~/.vibe/*` и `infra/` не читал и не касался. IP-адресов в
тексте нет; локальный порт не назван. Ничего на сервере и ничего в сети
(R-24, R-09) — все прогоны локальные, web-пакет собирался из уже
установленного `node_modules`.
