# Отчёт LP-O2 — учебный путь в читалке: оглавление, переключатель, пейджер {#root}

<status stage="impl" state="done" comment="пакет PACKET-LP-O2.md, 2026-09-25; четыре шага самопроверки из пяти зелёные, пятый — specmap, генерируемый файл вне периметра; один дефект пакета и одно отклонение, оба ниже"/>

## 1. Файлы {#files}

Новые:

| Файл | Что это |
|---|---|
| `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/design/src/components/pager/index.tsx` | компонент пейджера (`Pager`, `PagerLink`, `PagerChapter`) |
| `…/design/src/components/pager/styles.css` | его стили |
| `…/site/src/lib/cards.ts` | страницы документации как карточки, в обоих порядках (вынесено из `view.ts`, см. §6) |
| `…/site/src/reader/contents-view.ts` | что такое «вид оглавления»: два состояния, ключ, штамп на `<html>` |
| `…/site/src/lib/view.test.ts` | юниты `view.ts`: путь на странице пакета и соседи страницы |
| `…/site/tests/learning-path.spec.ts` | e2e: путь, переключатель, пейджер, страница пакета, RU, узкий экран, reduced motion |

Изменённые:

| Файл | Что изменилось |
|---|---|
| `…/design/src/components/contents/index.tsx` | тип `ContentsChapter`, поля `chapters`/`viewLabel`/`pathLabel`/`sectionsLabel`, две панели и сегментный переключатель |
| `…/design/src/components/contents/styles.css` | панели по атрибуту на `<html>`, пилюли переключателя, номер главы (`--accent`, табличные цифры) |
| `…/design/src/index.ts` | экспорт `Pager`, `PagerChapter`, `PagerLink`, `ContentsChapter` |
| `…/design/theme-init.js` | второй IIFE: штамп `data-contents-view` до стилей |
| `…/site/src/lib/contents.ts` | `Contents.chapters`, `learningPath()`, `neighboursOf()`, `chapterTitleOf()`, `servedTitles()` |
| `…/site/src/lib/view.ts` | `PageView.path`, `PackageView.chapters`/`start`; карточки переехали в `cards.ts` |
| `…/site/src/lib/reading.ts` | строки интерфейса пути и двух видов (одно место для двух читалок) |
| `…/site/src/lib/site-language.ts` | 9 русских строк |
| `…/site/src/reader/settings.ts` | пятая настройка чтения `contents`: чтение, запись, сброс, применение, приём от хоста |
| `…/site/src/reader/embedding.ts` | `contents` в патче настроек от хоста |
| `…/site/src/routes/doc/[...path]/index.tsx` | колонка с главами, пейджер, полка по главам, «Start here», компонент `PageOfPackage` |
| `…/site/src/routes/doc/[...path]/styles.css` | `.doc-package__start`, `.doc-chapter`, `.doc-chapter__number` |
| `…/site/src/components/served/index.tsx` | то же для локальной читалки: колонка, `ServedPager`, полка по главам, «Start here» |
| `…/site/src/fixtures/manifest.json` (+ байтовая копия `doc-build/manifest.json`) | две главы, вторая — `appendix` |
| `…/site/src/fixtures/manifest-ru.json` | название первой главы; приложение не названо — это откат на источник |
| `…/site/src/fixtures/README.md` | что объявляют фикстуры и почему библиотека не растёт (§5) |
| `…/site/src/lib/contents.test.ts` | юниты пути и соседей |
| `…/site/src/lib/manifest.test.ts` | тест провода LP-O1 переписан: фикстура теперь объявляет путь, «нет пути» строится удалением таблицы |
| `…/site/tests/furniture.spec.ts` | два теста колонки явно про вид «разделы» (панель `[data-contents-sections]`) |
| `…/site/tests/libraries.spec.ts` | новый тест: пакет без пути — один вид, нет переключателя, нет пейджера |
| `campaigns/docs-2026-09/INTERFACE-COPY.md` | 12 строк (9 новых + `pages.order.layer`, уже жившая в коде) |

## 2. Как реализован вид {#view}

`contents.ts` рядом с `contentsOf()`:

- `learningPath(library, at)` → `PathChapter[] | null`. `null` — «путь не объявлен», и это не пустой
  массив: различие несёт всю развилку интерфейса. Структура пути — всегда исходного издания;
  название главы — издания, которое читают, с откатом на название источника по `id`
  (`chapterTitleOf`, лестница на одну ступень короче, чем у папок: у главы нет каталога, на который
  можно упасть). Номера — место в счёте; `appendix` места не занимает, поэтому глава после приложения
  сохраняет номер, который был бы у неё. Страница главы, которой нет в документации, пропускается —
  как устаревшая закреплённая страница.
- `contentsOf()` получил поле `chapters` (`ContentsChapter[]`, пустой при отсутствии пути) —
  единственное значение, по которому колонка решает, показывать ли переключатель. `pinned` и
  `sections` не изменились ни в одном байте.
- `servedTitles()` вынесен из `contentsOf()`: три вида задают один вопрос об одних страницах.

Компонент `Contents` рендерит **обе** панели и переключатель над ними; `contents__nav--path`
появляется только при объявленном пути, и каждое правило переключателя привязано к этому классу,
поэтому пакет без пути не задет ни одним из них.

## 3. Переключатель: ключ и механизм до отрисовки {#switch}

- Ключ: `vibe-doc:contents` = `path` | `sections` через `storage.ts`.
- **Пишется только `sections`.** Путь — значение по умолчанию (`##NAV-CHAPTERS-READER`), поэтому
  ОТСУТСТВИЕ атрибута `data-contents-view` и есть путь — ровно как отсутствие `data-theme` есть
  системная тема. Следствие: читатель с выключенными скриптами получает путь, а не обе панели сразу.
- Механизм до первой отрисовки: второй IIFE в `design/theme-init.js`, который уже инлайнится в
  `<head>` перед стилями (`root.tsx`). Выбран он, а не сосед: у CSP тогда один хеш вместо двух, а
  файл уже существует именно для настроек, на которые действует таблица стилей. Его собственный
  заголовок дописан честно — теперь он про две настройки.
- CSS показывает одну панель одним правилом на `:root:not([data-contents-view="sections"])` /
  `:root[data-contents-view="sections"]`; нажатая пилюля выбирается тем же правилом, поэтому панель и
  отметка не могут разойтись на первом кадре.
- `aria-pressed` сервер пишет для вида по умолчанию, а читалка исправляет при сохранённом другом
  выборе: атрибут читают вслух, а не видят, поэтому позднее исправление ничего не стоит — мигать
  нечему. Кнопки в `<head>` ещё не существуют, иначе это сделал бы тот же скрипт.
- Выбор — **пятая настройка чтения** (`##READER-SETTINGS`, решение §4-E дизайна): живёт в
  `ReaderSettings`, пишется и забывается вместе с четырьмя, публикуется хосту через `postMessage`
  и принимается от него. Что такое вид — в `contents-view.ts`, как тема в `theme.ts`; клики
  обрабатывает `settings.ts` там же, где клики темы.
- На узком экране переключатель внутри блока `<details>`: он лежит в `<nav>`, который браузер
  скрывает вместе с закрытым блоком.

## 4. Пейджер, страница пакета, строки {#pager}

**Пейджер.** `neighboursOf(library, at, document)` в `contents.ts`: `null`, когда пути нет или
страница не стоит на пути (последнее `vibe check` не разрешает, но сайт рендерит тот манифест,
который ему дали). Подпись главы появляется только там, где путь переходит в другую главу; для
приложения — название без номера. Адреса соседей — `addressOf(library, lang, document)`, то есть в
языке, который читают, включая страницы с откатом на источник. Слово «Chapter» стоит отдельным
текстовым узлом (`.pager__chapter-word`), потому что таблица языка сайта двигает строки целиком, а
название главы принадлежит пакету. Место — последний блок колонки чтения, после блока
процитированных правил (тест меряет геометрию, а не порядок в разметке). Стрелка сдвигается на 2px;
под `prefers-reduced-motion` сдвиг отменён явно — базовый стиль убирает длительность перехода, что
превратило бы плавный сдвиг в прыжок.

**Страница пакета.** Полка «Pages» при объявленных главах — заголовки глав как дети сетки полки
(`grid-column: 1 / -1`), поэтому сетка остаётся одной и глава из двух карточек не делает ряд своей
ширины. Подпись полки меняется на «In the order of the learning path». `view.start` — первая
страница пути, а без пути первая страница манифеста; на неё ведут и ссылка «Start here» в шапке
(рядом с `llms.txt`), и первая карточка полки «Documentation» (раньше — `view.pages[0]`).

**Строки.** EN — константы в `site/src/lib/reading.ts` (одно место для маршрута и для локальной
читалки, как `MEASURE` и `PLATFORMS`); RU — строки таблицы `RUSSIAN_CHROME` в `site-language.ts`, тем
же механизмом, что соседняя хромированная мебель. В `INTERFACE-COPY.md` — 12 строк в его формате.

## 5. Решения и отклонения {#decisions}

**Решение: переключатель — не `TabPills`.** Пакет разрешает («семантику вкладок не копируй, если не
подходит»). Нажатая пилюля должна выбираться таблицей стилей из атрибута на `<html>`, иначе на первом
кадре отмечена не та; `TabPills` отмечает её классом из разметки. Внешне — те же токены и те же
размеры, поэтому читатель встречает один род переключателя.

**Решение: `cards.ts`.** `view.ts` перевалил бюджет 600 строк. Шов выбран по ответственности:
`view.ts` отвечает, что значит один АДРЕС, `contents.ts` — как документация выглядит изнутри как
СПИСОК МЕСТ, а `cards.ts` — о чём каждое из этих мест, в обоих порядках. Импортёры `PageCard`
переставлены на новый модуль, бочки-реэкспорта нет.

**Отклонение (единственное): e2e не меряет «среднюю» страницу пути.** Фикстурное руководство несёт
две страницы, поэтому у его пути есть только два конца. Третья страница была написана
(`reference/versions`, с `.md`/`.xml`/островом и строками в `llms*`) и убрана снова — она ломает
гейт: CSP сайта называет по хешу каждый отдельный инлайн-скрипт, конфигурация nginx не несёт
значение длиннее 4096 байт (`site/src/seo/csp.ts`, `CSP_CONF_LIMIT = 4000`), и замер такой:

| | хешей | байт политики | что делает генератор |
|---|---|---|---|
| два страницы (сейчас) | 64 | 3607 | пишет политику |
| три страницы | 72 | 4039 | политику НЕ пишет — и `csp.test.ts` краснеет |

Запас до потолка был 393 байта ≈ 7 хешей ≈ 3 адреса страниц; одна страница фикстуры — это 4 адреса
(две записи версии × два издания) и 8 хешей. Поэтому фикстурная библиотека не может вырасти на
страницу, пока не решён вопрос потолка (X-044), и это записано в `site/src/fixtures/README.md`.
Случай «у страницы есть сосед с обеих сторон» покрыт юнитами над библиотекой, написанной внутри
теста (`contents.test.ts`: путь из трёх глав, приложение **в середине**, четыре страницы) — там же
покрыты «сосед в той же главе, без подписи» и «страницы нет на пути». В браузере меряются оба конца
пути и переход через шов в обе стороны.

**Дефект пакета: `specmap.json`.** Четыре новых модуля с тегом `@scope` дают 4 новых ребра, и шаг
`typescript-ai-native-specmap --check` требует перегенерировать `specmap.json`. Периметр пакета
прямо запрещает его трогать («Нельзя: … сгенерированные файлы … `specmap.json`»), поэтому файл не
тронут и шаг остаётся красным. Перегенерировать должна центральная сессия:

```
cd vibevm/vibepacks/org.vibevm.doc/web/v1.0.0
C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe specmap --path . --write
```

Четыре модуля, дающие дрейф: `design/src/components/pager/index.tsx`, `site/src/lib/cards.ts`,
`site/src/reader/contents-view.ts`, `site/src/lib/view.test.ts`. (`site/tests/learning-path.spec.ts`
вне `scan_roots` и ребра не даёт.)

**Что ещё стоит знать ревьюеру.** Один юнит-случай, который я написал и затем убрал ради бюджета
600 строк в `contents.test.ts`: «объявленный путь не двигает папки и закрепления» — то же самое
меряется в браузере двумя тестами `furniture.spec.ts` над библиотекой, которая путь объявляет, и в
`view.test.ts` через `view.pages` в порядке манифеста.

## 6. Тесты {#tests}

Юниты (`node --test`, 141 тест в перегоне пола):

- `contents.test.ts` — нумерация глав и приложение без номера (приложение в середине пути, иначе
  «не считается» неотличимо от «стоит последним»); страницы главы в объявленном порядке; «объявил
  путь» против «объявил пустую таблицу» против «не объявлял»; пропуск страницы, которой нет; текущая
  страница в этом виде; названия глав перевода с откатом на источник; перевод, не назвавший ни одной
  главы; адреса и слова издания, которое читают. Соседи: первая страница, последняя, переход между
  главами в обе стороны, приложение, та же глава (без подписи), страница вне пути, RU-подписи.
- `view.test.ts` — полка пакета по главам; `view.pages` в порядке манифеста рядом с ней; первая
  страница пути ≠ первая страница манифеста; названия глав в RU-издании; пакет без пути (главы
  `null`, `start` = первая страница манифеста); соседи на обоих концах пути.
- `manifest.test.ts` — тест провода переписан под фикстуру, которая теперь объявляет путь.

e2e (`site/tests/learning-path.spec.ts`, 13 тестов): вид «путь» по умолчанию; приложение без номера
в колонке; переключатель (`role=group`, подпись, `aria-pressed`, видимое кольцо фокуса); переключение
в «разделы» и обратно; выбор переживает перезагрузку и стоит **на первом кадре** (атрибут на `<html>`
и `display: none` панели пути, замеренные в `requestAnimationFrame`, и он не отменён читалкой позже);
`href` пейджера на обоих концах; подпись главы только на переходе; пейджер после блока правил;
«Start here» и полка по главам на странице пакета; RU-читатель (главы, кнопки, слова пейджера,
подпись «Глава 1 · …», «Начать отсюда»); нет горизонтального скролла на 390/834/1440; переключатель
внутри блока на телефоне и работает оттуда; стрелка шагает на 2px и стоит на месте под reduced motion
(с проверкой, что наведение всё-таки случилось — по границе карточки).

`libraries.spec.ts` — новый тест «пакет без объявленного пути показывает один вид и не несёт
пейджер», в той же сборке, где соседняя документация путь объявляет.

`furniture.spec.ts` — два теста колонки поправлены явно: они про вид «разделы», локаторы привязаны к
панели `[data-contents-sections]`, содержимое списка не изменилось.

## 7. Самопроверка — вывод дословно {#verify}

```
$ cd vibevm/vibepacks/org.vibevm.doc/web/v1.0.0
$ TYPESCRIPT_AI_NATIVE=".../target/debug/typescript-ai-native.exe" node tools/floor.mjs --keep-going
=== prettier --check (floor perimeter: design/src, site/src) ===
=== tsc --noEmit ===
=== tests (node --test) ===
ℹ tests 141
ℹ pass 141
ℹ fail 0
=== eslint (floor perimeter: design/src, site/src) ===
=== typescript-ai-native-conform check ===
typescript-ai-native-conform: policy conform.toml (loaded).
typescript-ai-native-conform: extracted 0 file(s), 165 cached (producer ts-tsc-2).
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
typescript-ai-native-conform: 0 cell(s) gated, 0 exempt — see conform.toml for the why of each.
=== typescript-ai-native-specmap --check ===
`C:\Users\olegc\git\v\vibevm\vibevm\vibepacks\org.vibevm.doc\web\v1.0.0\specmap.json` is out of date relative to the tree.
  drift: edges added: 4
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
floor: `specmap` FAILED
=== test-gate (xfail-strict) ===
test-gate: running `node --test --test-reporter=tap` over the policy's TS roots …
test-gate: 153 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
Error: floor: 1 step(s) failed: specmap
EXIT=1
```

```
$ node design/audit/contrast.mjs
=== pairs: gated=46, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
EXIT=0
```

```
$ node tools/build.mjs static
build (static): generated 26 page(s), expected 26
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 14 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver, search index over 5 entries
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 11 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 64 inline script hash(es) over 165 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
build (static): no trace of the previous framework — 4 mark(s) looked for, none found
documentation links — every address against the files behind it
    ok        27 page(s), 342 file(s) in the output
    ok        1035 link(s) followed, 34 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        32x github.com — the canonical source repository, linked by the landing
    ok        31x gitverse.ru — the source mirror, linked by the landing
    L-01      4x — The island golden cites `media/diagram.svg` beside the DOCUMENT, … (известная находка острова, не моя)
links: green — 1035 checked, 0 broken.
build (static): ok
EXIT=0
```

```
$ node node_modules/@playwright/test/cli.js test -c site/tests/playwright.config.ts
  5 skipped
  190 passed (49.4s)
EXIT=0
```

Прошло 190, упало 0, пропущено 5 — все пять пропусков это `local-reader.spec.ts`, который сам
пропускается без собранного `vibe` и печатает причину; это состояние машины, а не мой тест. Из 190
тринадцать — новый `learning-path.spec.ts` (отдельный перегон: `13 passed (5.4s)`).

```
$ node tools/build.mjs embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): no trace of the previous framework — 4 mark(s) looked for, none found
build (embedded): ok
EXIT=0
```

## 8. Чего не сделано {#not-done}

1. **`specmap.json` не перегенерирован** — дефект пакета, §5. Единственный красный шаг пола.
2. **«Средняя» страница пути не меряется в браузере** — отклонение, §5: потолок CSP не даёт вырастить
   фикстурную библиотеку; случай покрыт юнитами.
3. **Главы в двух манифестах настоящего руководства** — это шаг 4 маршрута дизайна (центральная
   сессия), не мой: периметр запрещает пакеты руководства.
4. Git только читающий: ничего не добавлено в индекс и не закоммичено. `site/tests/chrome.spec.ts`
   я задел `prettier --write` по каталогу и вернул в байтовое состояние HEAD.
