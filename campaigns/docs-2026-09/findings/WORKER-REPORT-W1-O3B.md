# WORKER-REPORT-W1-O3B — авторство прозы по данным, две подписи бриджа, припинённые страницы

Пакет: `campaigns/docs-2026-09/findings/PACKET-W1-O3B.md`.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`.
Три атомарных коммита, каждый формой `git commit -F <файл> -- <пути>`, новые файлы
через `git add -- <файл>`. Без `push`, без `cargo`, без трейлеров и без единого
упоминания модели или агента в сообщениях. `crates/**` и спеку, которые в это же
время правил Rust-воркер (POST-O4), не трогал; чужих процессов не останавливал
(единственный процесс, который я останавливал, — статический сервер на порту
4207, который сам же и запускал под скриншоты).

## Коммиты

| hash | subject | файлов |
| --- | --- | --- |
| `34ec6e46` | `feat(web): filter the catalogue by who wrote the prose` | 34 |
| `2abcb058` | `feat(web): show a bridge's maintainer apart from the upstream's author` | 27 |
| `2d99b95c` | `feat(web): list pinned pages first and name sections as the manual does` | 9 |

**Периметр.** `git diff --name-only <sha>~1 <sha>` по трём коммитам даёт 70 путей,
все внутри `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/{site,design}/**`,
`campaigns/docs-2026-09/findings/W1-O3B-shots/**` и `…/web/v0.1.0/specmap.json`.
Ни одного попадания в `site/src/generated/`, `docker/`, `site.example.toml`,
`crates/`, `schemas/`, руководство или спеки. Рабочее дерево чистое.

`specmap.json` — единственный путь вне буквального списка периметра. Он лежит в
корне веб-пакета и его перегенерации требует гейт `floor` (шаг
`typescript-ai-native-specmap --check`), как только появляется новый файл с
`@scope`; ровно так же его коммитил W1-O4. Перегенерирован командой
`typescript-ai-native specmap --path <пакет>` в коммитах N и O.

---

## N. Проекции и авторство по данным — `34ec6e46`

### Главное, чего не было видно из пакета: поля не доезжали до страницы

`site/src/lib/manifest.ts` — единственная дверь из байтов в тип
(`parseDocManifest` → `card()`), и она **молча роняла все три новых поля**:
`authorship`, `projection`, `bridge`. В сгенерированном типе они есть (W1-O3), но
парсер их не читал, поэтому `card.projection` был бы всегда `undefined`, а
`Reflect.get(card, "projection")` W1-O4 не срабатывал никогда — работала только
эвристика. Тот же класс дефекта, что W1-O6 нашёл на `navigation`.

Теперь `card()` читает:

- `authorship` — через новый `optionalMember(...)`: отсутствие разрешено
  («документ ничего не сказал»), четвёртое слово — отказ с именем поля
  (`$.package.authorship`, в тексте перечислены три слова);
- `projection` — через новый `flag(...)`: `true`/`false`/отсутствует, не-булево
  значение — отказ;
- `bridge` (коммит O) — две строки списков и необязательная лицензия.

`isProjection(card)` (`site/src/lib/library.ts`) теперь читает `card.projection`
обычным полем; `Reflect.get` убран, как просил пакет. Эвристика W1-O4 (координата
пакета среди его же `subjects`) осталась **вторым** шагом, с комментарием о том,
что это запасной путь для манифестов, написанных до появления поля, а не для
деплоев.

### Выпадашка и фильтр

- `design/src/components/authorship-filter/` — новый компонент, `<details>` с
  пилюлей и списком, `data-authorship-filter`, записи несут
  `data-authorship-choice="*|human|ai"` и `data-authorship-pill`.
- `site/src/lib/authorship.ts` — **правило** (чистая функция `admitsAuthorship`)
  и три записи меню: «All / Human-authored / AI-generated». `mixed` попадает в
  **обе** названные группы; отсутствующее и неизвестное слово — **ни в одну**,
  только в «all» (PROP-057 `##CARD-AUTHORSHIP`). Примечание под «All» — то самое
  место, где правило о молчании сказано словами.
- `site/src/reader/authorship.ts` — поведение: память (`vibe-doc:authorship`),
  пометка записи и пилюли, сужение полки. Запускается только там, где контрол
  есть на странице (каталог), иначе не регистрирует ничего.
- Карточка: `Card` получил `authorship?`, рисует `data-authorship` и третий бейдж
  (`AuthorshipBadge`: `Human` / `AI` / `Mixed`) — рядом со стандингом и
  `GENERATED`, потому что это три разных вопроса. Поля нет → метки нет.

### Один общий ответ на «что стоит на полке»

Два фильтра сужают одни и те же карточки, поэтому видимость решается в одном
месте — новый `site/src/reader/shelf.ts`: каждый контрол регистрирует своё
«допускаю», карточка стоит, когда её допускают **все**; там же строка «пусто».
`doc-language.ts` переведён на него (его `filterCards` больше не пишет `hidden`
сам). Иначе последний клик побеждал бы: выбор языка молча расширял бы выбранное
авторство.

### Решения, которые пришлось принять самому

1. **Пилюля «anyone», а не «all».** Фильтр языка в том же ряду печатает `ALL`,
   когда ничего не сужает; две пилюли `ALL` рядом — два контрола, которые
   читатель не различает, не открыв. Подписи записей остались как в пакете
   («All / Human-authored / AI-generated»), меняется только слово на закрытой
   пилюле. Видно на `n-authorship-open-1440-light.png` (до правки) и
   `o-bridge-card-1440-light.png` (после).
2. **Слова бейджей (`Human` / `AI` / `Mixed`) не переводятся.** Это словарь поля,
   как `primary` / `official` / `community` и `GENERATED`, которые тоже не в
   таблице чрома: читатель, сверяющий карточку с манифестом, должен найти в обоих
   одно слово. Сама выпадашка — мебель и переведена (шесть новых строк в
   `RUSSIAN_CHROME`).
3. **Свой компонент, а не расширение `LanguageSelector`.** Тот несёт звезду,
   издателя и `hreflang`; здесь — три группы и ни одного адреса. Геометрия
   выписана заново по тем же токенам (кровное родство названо в комментарии
   стилей), потому что общий стиль для двух контролов, различающихся внутри, —
   третье место, куда придётся смотреть при любой правке любого из них.
4. **Метка авторства есть и на карточках страницы пакета** (своя карточка и
   карточки адаптаций) — это свойство карточки, а не двери. Сама выпадашка — только
   на каталоге, как просил пакет.

### Фикстуры

`manifest.json` → `"authorship": "ai"`, `manifest-ru.json` → `"mixed"`. Это
«интересная пара»: «human-authored» меряется на документе, который стоит и в
другой группе. Третье состояние (документ, который ничего не сказал) есть у
второй фикстурной библиотеки `doc-build-pair*/`. `doc-build/manifest.json` — байтовая
копия `manifest.json`, как требует `site/src/fixtures/README.md`, и обновлена
вместе с ним. README дополнен абзацем о том, что и зачем объявляют фикстуры.

### Тесты

- `site/src/lib/authorship.test.ts` (новый, 6 проверок): все четыре состояния
  против трёх групп, включая неизвестное слово из более новой версии конвейера.
- `manifest.test.ts`: слово переживает разбор; четвёртое слово — отказ с именем
  поля; отсутствие — разрешено; `projection` читается как написано и «нет» ≠
  «false».
- `library.test.ts`: `isProjection` верит полю в обе стороны; авторство каждого
  издания едет на его карточку, а библиотека без поля не кладёт на карточку
  ничего.
- `catalogue.spec.ts` (браузер): выпадашка сужает полку, `mixed` виден в обеих
  группах, два фильтра сужают вместе, опустошённая полка говорит, выбор помнится.

---

## O. Две подписи бриджа — `2abcb058`

- `design/src/components/bridge-signatures/` — один компонент на оба места:
  карточку и шапку страницы пакета. Два места, выписанные порознь, — два шанса
  назвать подписи по-разному.
- Подписи: «Bridge maintainer» → `bridge.maintainers`, «Destination author» →
  `bridge.upstream_authors`, «Upstream licence» → `bridge.upstream_license`,
  только когда лицензия приехала. Переводы: «Сопровождает мост», «Автор
  оригинала», «Лицензия оригинала».
- **Пустой список печатается как пустой** (тире). Исчезнувшая строка читалась бы
  как «это не мост» — единственное, чем он не является; а строка, тихо
  заполненная из соседнего списка, и есть тот самый провал, ради которого поле
  существует.
- **`publisher` остался на месте**, над подписями. Группа-издатель — третий факт
  (к кому идти с претензией), и D-19 требует его видимости всегда. Пакет просил
  «у пакета без `bridge` — как сейчас»; у пакета с бриджем подписи **добавлены**,
  а не заменили издателя.
- Ничего не выводится из имён групп: `site/src/lib/bridge.ts` только
  перекладывает `upstream_authors` → `upstreamAuthors` и отдаёт `undefined`, если
  поля нет.
- Шапка страницы пакета и локальный читатель (`components/served/`) получают то
  же самое.

**Дефект, найденный глазами.** Первый рисунок карточки сломал раскладку: с 768 px
`.card__row` — грид, `.card__body` — `display: contents`, поэтому каждый ребёнок
тела обязан быть назван в правиле `grid-column: 2`. Неназванный блок подписей
уехал в первую колонку под значок и утащил за собой всю карточку (видно было на
первом снимке). Правило теперь перечисляет всех детей тела и говорит почему.

**Фикстура.** Мостом объявлен источник (`manifest.json`), адаптация рядом —
не мост, поэтому на одной полке стоят обе формы карточки, а у каждой есть своя
страница пакета. Альтернатива — третья фикстурная библиотека — ломала бы линтер
ссылок: у пакета без дерева нет `llms.txt`, а на него ссылаются и шапка, и
карточка (`copySurfaces` ничего не копирует изданию, у которого нет ни своего
дерева, ни дерева источника). Добавить дерево можно было только правкой
`tools/doc-surfaces.mjs` — вне периметра. Это записано в `site/src/fixtures/README.md`.

**Тесты.** `bridge.test.ts` (новый): оба списка и лицензия едут под именами,
которые читает страница; «не мост» — это `undefined`, а не пустой мост.
`manifest.test.ts`: списки приезжают порознь, пустой остаётся пустым, лицензии
может не быть, сломанный мост — отказ с именем члена. `catalogue.spec.ts`
(браузер): на карточке и в шапке по три строки с ожидаемыми значениями, у
адаптации подписей нет и `publisher` на месте.

---

## P. Припинённые страницы и заголовки разделов — `2d99b95c`

### Что уже было сделано до меня (проверено по коду, а не по отчёту)

W1-O6 приземлил **всю логику** этого пункта:

- `site/src/lib/manifest.ts` — разбор `navigation` (он же чинил то, что парсер
  ронял поле);
- `site/src/lib/library.ts` — `Edition.navigation`;
- `site/src/lib/contents.ts` — `contentsOf()`: пины первыми, в порядке пакета;
  пин, не называющий страницы, молча пропускается; заголовок раздела — издание
  читателя → источник → имя каталога с заглавной; порядок остальных — порядок
  манифеста; список — страницы источника во всех языках;
- `design/src/components/contents/` — колонка, пины вне групп;
- `site/src/lib/contents.test.ts` — 6 проверок на всё перечисленное;
- колонка стоит и на странице документа, и в локальном читателе
  (`components/served/`).

**Чего не хватало: данных.** Ни один манифест ни в одной сборке не объявлял
`navigation` — ни фикстуры, ни их копия в `doc-build/`. Поэтому каждая собранная
страница показывала запасной путь (никаких пинов, заголовок из имени каталога),
и правило «документация сама решает, как выглядит её список страниц» было верным
в коде и невидимым на сайте. Ровно это я и доделал.

### Что сделано

- Фикстурный источник пинит `guide/every-block` и называет раздел `reference`
  («Reference pages»); адаптация называет тот же раздел своими словами
  («Справочные страницы») и не пинит ничего. `doc-build/manifest.json` — байтовая
  копия источника.
- `furniture.spec.ts`: тест колонки теперь читает заявление документации
  (припинённая страница первой и вне групп, заголовок — слова пакета, а не имя
  каталога) вместо запасного пути; второй тест открывает русскую половину и
  находит русский заголовок над тем же разделом и пины, ведущие на русские
  адреса.
- `contents.test.ts`: добавлена проверка средней ступени лестницы, которой не
  было ни у кого, — адаптация, не назвавшая раздел, показывает слова **источника**,
  а не имя каталога.

### Наблюдение (не чинил, это не мой пункт)

Ссылки в колонке несут заголовки страниц **источника** даже там, где адаптация
эту страницу имеет и дала ей своё название: русская колонка читается «Every block
once» над русской страницей, чья карточка на странице пакета называется «Каждый
блок по разу» (`site/src/lib/contents.ts`, карта `titles` строится из
`source.pages`; для сравнения `pageCards` в `view.ts` берёт название из
`resolvePage`, то есть у издания читателя). Пакет говорит только про пины и
заголовки разделов, поэтому я это не менял — но видно это теперь хорошо, снимок
`p-contents-ru-1440-light.png`. Решение владельца/босса.

---

## Самопроверка — вывод дословно

Все четыре команды запущены из корня ворктри в форме, которую требует пакет,
после третьего коммита.

### `pnpm -C … floor` — EXIT=0

```
=== prettier --check (floor perimeter: design/src, site/src) ===
Checking formatting...
All matched files use Prettier code style!

=== tsc --noEmit ===

=== tests (node --test) ===
ℹ tests 116
ℹ suites 9
ℹ pass 116
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 6281.4462

=== eslint (floor perimeter: design/src, site/src) ===

=== typescript-ai-native-conform check ===
typescript-ai-native-conform: policy conform.toml (loaded).
typescript-ai-native-conform: extracted 0 file(s), 129 cached (producer ts-tsc-2).
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
typescript-ai-native-conform: 0 cell(s) gated, 0 exempt — see conform.toml for the why of each.

=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 125 tagged code items, 125 edges, 0 suspects, 125 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).

=== test-gate (xfail-strict) ===
test-gate: running `node --test --test-reporter=tap` over the policy's TS roots …
test-gate: 125 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).

floor: all green (7 step(s) run, 0 disabled by policy).
…
=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
```

(116 юнит-тестов против 102 до пакета: +14 новых.)

### `pnpm -C … build:static` — EXIT=0

```
build (static): generated 18 page(s), expected 18
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 14 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver, search index over 5 entries
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 48 inline script hash(es) over 121 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
documentation links — every address against the files behind it
    ok        19 page(s), 275 file(s) in the output
    ok        647 link(s) followed, 18 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    L-01      4x — The island golden cites `media/diagram.svg` … (известное исключение линтера, было до пакета)
links: green — 647 checked, 0 broken.
build (static): ok
```

### `pnpm -C … build:embedded` — EXIT=0

```
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): ok
```

### `pnpm -C … test:e2e` — EXIT=0

```
Running 58 tests using 1 worker

  ok  1 [chromium] › site\tests\catalogue.spec.ts:48:1 › the door offers three shelves and opens on one with something on it (853ms)
  ok  2 [chromium] › site\tests\catalogue.spec.ts:67:1 › a shelf with nothing on it says so rather than vanishing (576ms)
  ok  3 [chromium] › site\tests\catalogue.spec.ts:80:1 › the shelf a reader is on is in the address and is remembered (708ms)
  ok  4 [chromium] › site\tests\catalogue.spec.ts:101:1 › the language filter works inside the shelf a reader is on (1.3s)
  ok  5 [chromium] › site\tests\catalogue.spec.ts:127:1 › the shelf narrows to who wrote the prose, both hands in both groups (973ms)
  ok  6 [chromium] › site\tests\catalogue.spec.ts:162:1 › a bridge names its maintainer and the upstream's author apart (1.4s)
  ok  7 [chromium] › site\tests\catalogue.spec.ts:206:1 › the two filters narrow together, and a shelf they empty says so (1.4s)
  ok  8 [chromium] › site\tests\furniture.spec.ts:43:1 › the manual's pages stand in a column, grouped by their folders (395ms)
  ok  9 [chromium] › site\tests\furniture.spec.ts:73:1 › a folder is named in the words of the edition being read (564ms)
  …
  2 skipped
  56 passed (1.1m)
```

Было 55 passed / 2 skipped после W1-O6, стало 56 / 2: четыре новых теста (два на
авторство, один на бридж, один на русские заголовки колонки) и один прежний тест
колонки переписан под данные. Два `skipped` — те же, что и раньше
(`local-reader.spec.ts`, навигация реального сервера).

---

## Аномалии

1. **Счётчик инлайновых скриптов 47 → 48, при тех же 121 вхождении.** Своих
   инлайновых скриптов я не добавлял ни одного: во всём пакете их два
   (`root.tsx` — тема, `routes/doc/[...path]/index.tsx` — отмена восстановления
   прокрутки), плюс блоки `application/ld+json`, которые гейт не хеширует.
   Инвентаризация выхода показывает, что все 48 уникальных тел — это два
   авторских, один `URLSearchParams` (был до пакета) и **45 фреймворковых**:
   `(window._qwikEv…)`, `document["qFuncs_…"]` и списки предзагрузки модулей,
   то есть сериализация Qwik, которая меняется вместе с содержимым страниц. Два
   новых компонента дали ещё один такой блок. Внешних ресурсов не прибавилось
   («no external source» в той же строке).
2. **Раскладка карточки сломалась на первом рисунке подписей бриджа** (см. O) —
   найдено скриншотом, починено в стилях карточки, а не в компоненте подписей.
3. **Заголовки страниц в колонке — источника, а не издания читателя** (см. P).
   Не чинил: вне пункта.
4. **Форма коммита.** Пакет пишет `git commit -m … -- <пути>`; сообщения
   многоабзацные, поэтому — `git commit -F <файл> -- <пути>`, как делал W1-O6.
   Явные пути и отсутствие `git add -A` соблюдены; ни один файл Rust-воркера,
   работавшего в том же дереве, ни разу не попал в индекс.
5. **Три коммита Rust-воркера (POST-O4) приехали в ветку** между моими — его
   правки в `crates/vibe-registry/**` уже закоммичены им самим и в моих коммитах
   не участвуют.

## Чего не сделано и почему

- **Третьей фикстурной библиотеки-моста нет** — вместо неё мостом объявлен
  источник фикстурной пары. Причина в разделе O: у библиотеки без дерева нет
  `llms.txt`, а линтер ссылок считает это красным; дерево можно добавить только
  правкой `tools/doc-surfaces.mjs`, которая вне периметра.
- **Заголовки страниц в колонке не тронуты** (см. P, «Наблюдение»).
- **Слова бейджей авторства не переведены** — по правилу «словарь поля не
  переводится», как у стандингов и `GENERATED`.
- **Выпадашка авторства только на каталоге.** На странице пакета карточки метку
  носят, но фильтра там нет: пакет просит контрол «на каталоге, рядом с
  вкладками», а второй контрол на странице с тремя полками — отдельное решение.
- **`vibe doc build-site` не запускался**, `cargo` не запускался: всё измерено на
  фикстурной библиотеке, как и в W1-O1/W1-O4/W1-O6.
- **`tools/parity.mjs` не запускался** (требует собранного дерева чужого
  Astro-сайта). Новые видимые строки, для которых там могло бы не быть правила:
  «Who wrote the prose», «All», «Human-authored», «AI-generated», «Bridge
  maintainer», «Destination author», «Upstream licence», «Reference pages».
- **Ничего не отправлено (`push`)**, отчёт не коммичен.

## Скриншоты

`campaigns/docs-2026-09/findings/W1-O3B-shots/` — 16 файлов, закоммичены с N, O и
P соответственно.

- `n-catalogue-marks-{1440}-{light,dark}` — полка с метками `AI` и `MIXED`;
- `n-authorship-open-1440-{light,dark}`, `n-authorship-open-390-light` — открытая
  выпадашка (сняты до правки пилюли, поэтому на них закрытая пилюля ещё `ALL`);
- `n-authorship-human-1440-{light,dark}` — выбрано «human-authored»: стоит одна
  карточка, и она `MIXED`;
- `o-bridge-card-1440-{light,dark}` — карточка моста рядом с карточкой без моста
  (здесь же видно пилюлю `ANYONE`);
- `o-bridge-package-{1440-light,1440-dark,390-light}` — шапка страницы пакета;
- `o-no-bridge-package-1440-light` — страница пакета без моста, как было;
- `p-contents-{1440-light,1440-dark}` — колонка с припинённой страницей первой и
  заголовком «Reference pages»;
- `p-contents-ru-1440-light` — та же колонка на русской половине.
