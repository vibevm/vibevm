# WORKER-REPORT-W1-O5 — глиф карточки по виду пакета; подписи колонки «Contents» — из издания читателя

Пакет: `campaigns/docs-2026-09/findings/PACKET-W1-O5.md`.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`.
Веб-пакет остался на `web/v0.1.0` (переезда в `v1.0.0` в дереве нет).
Два атомарных коммита, каждый формой `git commit -m … -m … -- <пути>`, новые
файлы через `git add -- <файл>`. Без `push`, без `cargo`, без единого трейлера и
без упоминания модели или агента в сообщениях. `crates/**` не трогал; чужих
процессов не останавливал — единственный, который я останавливал, это статический
сервер на порту 4307, который сам же и запускал под скриншоты.

## Коммиты

| hash | subject | файлов |
| --- | --- | --- |
| `d77044ba` | `feat(web): draw a package's kind on its card` | 25 |
| `11319f58` | `fix(web): label the contents column in the reader's edition` | 5 |

**Периметр.** Все пути внутри
`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/{design,site}/**`,
`campaigns/docs-2026-09/findings/W1-O5-shots/**` и
`…/web/v0.1.0/specmap.json`. Ни одного попадания в `site/src/generated/`,
`docker/`, `site.example.toml`, `crates/`, `schemas/`, руководство или спеки.
Рабочее дерево чистое (кроме чужого некоммиченного
`findings/WORKER-REPORT-POST-O5.md`).

`specmap.json` — единственный путь вне буквального списка периметра: он лежит в
корне веб-пакета, и гейт `floor` (шаг `typescript-ai-native-specmap --check`)
требует его перегенерации, как только появляется новый файл с `@scope`. Ровно так
же его коммитили W1-O4 и W1-O3B. Перегенерирован
`typescript-ai-native specmap --path <пакет>`; дрейф — ровно одно ребро
(`kind-glyph/index.tsx`).

---

## ГЛАВНОЕ: откуда карточка узнаёт вид — и где пакет расходится с деревом

**Дефект пакета.** Пункт 3 «Читать сначала» говорит: «у проекции уровня 0 — вид её
пакета **из манифеста**; если вид пакета до карточки не доходит — **донеси его из
манифеста**». В манифесте вида нет. Это не «не доезжает до карточки» — этого поля
нет на проводе вообще:

- `schemas/doc_manifest.jtd.json` — у карточки пакета нет члена `kind`
  (слово «kind» встречается там дважды, оба раза в прозе описаний);
- `site/src/generated/doc-manifest.ts` — у `DocPackage` нет `kind`;
- `site/src/lib/manifest.ts` — единственная дверь из байтов в тип — может читать
  только то, что в байтах есть.

Вид у конвейера **есть** и он его тратит:

```rust
// crates/vibe-doc/src/manifest.rs:346
projection: field("kind").as_deref() != Some(DOC_KIND),
```

То есть `[package].kind` прочитан и целиком израсходован на булево `projection`.
Донести слово на провод — правка `schemas/` + `crates/vibe-doc/`, то есть Rust,
`cargo` и периметр, которые пакет мне закрыл. Поэтому я реализовал всё, что
можно реализовать честно, и записал дефект сюда вместо того, чтобы выдумать поле.

**Что в итоге знает карточка.**

`site/src/lib/library.ts` — новая функция `kindOf(card): PackageKind | undefined`,
рядом с `isProjection` и через неё:

```ts
export function kindOf(card: DocPackage): PackageKind | undefined {
  return isProjection(card) ? undefined : "doc";
}
```

- **Издание, которое не проекция, — это пакет вида `doc`.** Это не догадка, а
  определение конвейера, прочитанное с другого конца: билдер помечает рендер
  проекцией ровно тогда, когда отрендеренный пакет — не документация
  (`##LEVEL-ZERO-MARKED`). «Не проекция» и «вид doc» — одно утверждение в двух
  написаниях, поэтому карточка и полка, на которой она стоит, не могут разойтись.
- **У проекции ответ — «не сказано»** (`undefined`), и карточка рисует сегодняшний
  плейсхолдер. Пакет просил именно этого («неизвестный вид — сегодняшний
  плейсхолдер»), просто это сегодня единственная ветка для проекций, а не редкий
  случай.

Дальше вид едет **названным значением**, как все остальные:
`catalogue.ts` → `CatalogueEntry.packageKind`; `view.ts` → `PackageView.packageKind`
и `adaptations[].packageKind`; оттуда в проп `kind` компонентов. Имя `packageKind`,
а не `kind`, потому что `PackageView` уже отвечает на слово `kind` — это его
дискриминант (`"page" | "package" | "catalogue"`).

**Следствие, которое надо видеть боссу:** на сегодняшнем сайте узнаётся **один**
вид из восьми — `doc`. Остальные семь нарисованы, покрыты типом и ждут поля в
манифесте, а не кода. Приёмка «восемь видов узнаются на карточках в обеих темах»
в этом дереве недостижима: ни одна фикстура и ни один реальный манифест не может
объявить вид. Компилятор проверяет полноту набора (`satisfies Record<PackageKind,
…>` — восьмой вид, не нарисованный, валит `tsc`), а глазами в обеих темах
проверен `doc`.

---

## N. Глиф вида — `d77044ba`

### Компонент

`design/src/components/kind-glyph/` — `KindGlyph` + `styles.css`.

- Восемь контуров перенесены из `findings/W1-O5-glyphs.svg` **байт в байт**: те же
  `d`, тот же `<circle>` у `flow`, тот же `<rect>` у `app`, тот же `viewBox`.
- Презентация с элементов снята и живёт в стилях: `fill`, `stroke`, `linecap`,
  `linejoin`, и два именованных токена — `--kind-glyph-size` (по умолчанию `1em`)
  и `--kind-glyph-stroke` (`1.75`, в единицах 24-сетки, чтобы вес держался на
  любом размере). Хост задаёт размер долей своей плитки: `.card__placeholder
  .kind-glyph` и `.package-header__placeholder .kind-glyph` ставят `58%`, поэтому
  один и тот же глиф следует за плиткой через все её ширины (48px на узком, 1.9rem
  рядом с названием, 72px в шапке) без второго числа, которое надо держать в
  синхроне. Прецедент компонентного кастом-проперти в этом пакете —
  `--site-header-h` в `docs-header/styles.css`.
- Цвет — `currentColor`; плитка уже красит себя в `var(--accent)`, поэтому обе темы
  работают через токены и `tokens.css` я не трогал (аудит контраста зелёный,
  «tokens.css writes no colour of its own: OK»).
- Тип `PackageKind` — закрытое объединение восьми слов (VIBEVM-SPEC §4.1),
  экспортируется через шов вместе с компонентом, потому что вид выбирает
  приложение.

### Где рисуется

- `Card` — вместо сегодняшнего символа внутри той же плитки. Проп `kind?`
  отсутствует → плитка ровно та, что была, с `glyph` и `aria-hidden`.
- `PackageHeader` — то же самое, та же доля плитки, чтобы читатель, пришедший с
  полки, узнал шапку как карточку, по которой кликнул.
- `DocCard` — у него **нет плитки**, поэтому глиф встал на строку названия (там же,
  где `Card` держит метку от 768px). Без `kind` карточка байт в байт прежняя.
  **Замечание:** `DocCard` сегодня не используется нигде, кроме шва
  (`design/src/index.ts`); я правил его, потому что пакет назвал его явно.
- Карточки полки «Pages» на странице пакета получают вид **своего пакета**.
  Страница — не пакет, но она страница пакета, и полка отвечает на тот же вопрос,
  что две полки над ней; разная метка читалась бы как различие, которого нет.

### Доступность

Именована **плитка**, а не рисунок: `role="img"` + `title` + `aria-label` со
словом `«<kind> package»` (`kindLabel`, не экспортируется через шов), сам `<svg>`
остаётся `aria-hidden` — одно объявление, а не два. Слово вида не переводится, по
тому же правилу, по которому не переводятся стандинги и `GENERATED`: это словарь
поля, и читатель, сверяющий карточку с пакетом, должен найти в обоих одно слово.
Существительное после него есть потому, что голое «flow» рядом с названием
читается как часть названия. Карточка без вида не объявляет ничего — молчание, а
не «unknown».

### Тесты

- `library.test.ts`: `kindOf` отвечает `doc` документации и `undefined` проекции;
  оба издания фикстурной пары несут `doc` на карточке; у проекции члена
  `packageKind` на карточке **нет вовсе** (а не есть и пустой).
- `catalogue.spec.ts` (браузер, новый тест): на полке две плитки, у первой
  `aria-label="doc package"` и внутри `svg.kind-glyph` с `aria-hidden="true"`;
  шапка страницы пакета несёт ту же метку.
- Полнота восьмёрки — `satisfies Record<PackageKind, () => JSXOutput>`, то есть
  шаг `tsc` гейта.

---

## O. Подписи колонки «Contents» — `11319f58`

Наблюдение W1-O3B подтвердилось на собранном сайте: `titles` в
`site/src/lib/contents.ts` строилась из `source.pages`, поэтому русская колонка
читалась «Every block once» над страницей, чья карточка называется «Каждый блок по
разу».

Теперь подпись берётся у **издания, которое будет обслуживать адрес** — через
`resolvePage`, ту же функцию, через которую резолвится сама страница, поэтому
слово на ссылке и текст за ней не могут разойтись:

```ts
const served = resolvePage(library, at, document);
titles.set(document, (served?.page ?? page).title);
```

Список остался источника (адаптация в работе — не уменьшенное руководство),
порядок, группировка и пины не тронуты. Там, где адаптация страницы не несёт,
стоят слова источника — это ровно тот текст, который отдаст адрес. Правило
записано в шапке файла четвёртым, рядом с тем же правилом для заголовков разделов.

**Тесты.** Два юнита в `contents.test.ts` (лестница в обе стороны на библиотеке,
где адаптация несёт одну страницу из четырёх; то же для припинённой страницы —
«стоять первой» это про порядок и ничего не говорит о словах) и переписанный
браузерный тест `furniture.spec.ts`, который теперь находит обе половины правила
на одном экране: припинённая ссылка по-русски, недостигнутая страница —
по-английски. Комментарий в этом тесте, утверждавший обратное правило, убран.

---

## Самопроверка — вывод дословно

Все три команды пакета запущены из корня веб-пакета **после второго коммита**, на
чистом дереве.

### `pnpm -C … floor` — EXIT=0

```
=== prettier --check (floor perimeter: design/src, site/src) ===
Checking formatting...
All matched files use Prettier code style!

=== tsc --noEmit ===

=== tests (node --test) ===
ℹ tests 119
ℹ suites 9
ℹ pass 119
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 6171.1272

=== eslint (floor perimeter: design/src, site/src) ===

=== typescript-ai-native-conform check ===
typescript-ai-native-conform: policy conform.toml (loaded).
typescript-ai-native-conform: extracted 0 file(s), 130 cached (producer ts-tsc-2).
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
typescript-ai-native-conform: 0 cell(s) gated, 0 exempt — see conform.toml for the why of each.

=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 126 tagged code items, 126 edges, 0 suspects, 126 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).

=== test-gate (xfail-strict) ===
test-gate: running `node --test --test-reporter=tap` over the policy's TS roots …
test-gate: 128 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).

floor: all green (7 step(s) run, 0 disabled by policy).

=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
```

(119 юнит-тестов против 116 до пакета: +3 новых.)

### `pnpm -C … build:static` — EXIT=0

```
build (static): generated 18 page(s), expected 18
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 14 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver, search index over 5 entries
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 48 inline script hash(es) over 117 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
documentation links — every address against the files behind it
    ok        19 page(s), 278 file(s) in the output
    ok        647 link(s) followed, 18 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    L-01      4x — The island golden cites `media/diagram.svg` beside the DOCUMENT, and the pipeline publishes a package's media at the root of its tree under a content name — so the address misses at whatever depth the page is served, and the fixture package carries no such file in either place. SVG is not an allowed medium in this wave (D-20-6). Filed as an island-golden finding; the picture is the only broken one on the fixture page.
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

### `pnpm -C … test:e2e` — EXIT=0 (не в списке пакета, но я добавил браузерные тесты)

```
Running 59 tests using 1 worker
  ok  8 [chromium] › site\tests\catalogue.spec.ts:249:1 › a card carries the mark of its kind, and the page it leads to wears the same one (731ms)
  ok 10 [chromium] › site\tests\furniture.spec.ts:75:1 › a folder is named in the words of the edition being read (1.2s)
  …
  2 skipped
  57 passed (1.1m)
```

Было 56 passed / 2 skipped после W1-O3B, стало 57 / 2: один новый тест (метка
вида) и один прежний переписан под правило подписей. Два `skipped` — прежние
(`local-reader.spec.ts`, навигация реального сервера).

---

## Аномалии

1. **Пакет противоречит дереву по источнику вида** — раздел «ГЛАВНОЕ» выше. Вида
   нет в манифесте; конвейер читает `[package].kind` и целиком тратит его на
   `projection` (`crates/vibe-doc/src/manifest.rs:346`). Семь из восьми глифов на
   сайте сегодня не появятся ни при каких данных. Починка — одно названное поле на
   проводе, рядом с `projection`; это Rust и вне моего периметра.
2. **Счётчик инлайновых вхождений 121 → 117 при тех же 48 уникальных хешах.**
   Своих инлайновых скриптов я не добавлял ни одного. Это сериализация Qwik,
   которая меняется вместе с содержимым страниц (та же природа, что аномалия №1 в
   отчёте W1-O3B); внешних источников не прибавилось («no external source» в той
   же строке).
3. **`prettier --write` по `design/src site/src site/tests` переписал два чужих
   файла** (`site/tests/libraries.spec.ts`, `site/tests/local-reader.spec.ts`) —
   `site/tests/` не входит в периметр гейта prettier, поэтому там был дрейф до
   меня. Оба восстановлены `git checkout --` и в коммиты не попали; проверено, что
   в них нет ни одного изменения содержимого (`git diff --numstat` пуст).
4. **В дереве всё время работал второй воркер** (POST-O5, Rust): его коммиты
   `4d1138ab`, `633856c2`, `efe48cd3` приехали в ветку между и после моих, его
   некоммиченный `findings/WORKER-REPORT-POST-O5.md` лежит в дереве. Пакет
   утверждал, что других воркеров нет. Ни один его файл ни разу не попал в мой
   индекс — оба коммита сделаны явными путями.
5. **Плитка шапки страницы пакета частично уходит за левый край окна на 1440.**
   Это было до меня — сравните `W1-O3B-shots/o-bridge-package-1440-dark.png` с
   `W1-O5-shots/package-1440-dark.png`: раскладка идентична, изменился только
   символ внутри плитки. Не чинил: вне пункта.
6. **Тело страницы на русской половине фикстуры — английское** (остров у
   фикстурной пары один на оба издания). Тоже было до меня, видно на обоих
   снимках; манифест при этом объявляет русское название страницы, и колонка
   теперь показывает именно его.

## Чего не сделано и почему

- **Восемь видов нигде не видно одновременно** — см. аномалию 1. Полнота набора
  держится `satisfies` (шаг `tsc`), а не скриншотом.
- **`site/src/generated/doc-manifest.ts` не тронут** — только читал, как просил
  пакет.
- **Фикстуры не тронуты.** Приписать `"kind"` фикстурному манифесту было бы
  выдумыванием поля, которого нет в схеме, и парсер всё равно бы его не увидел.
- **`tokens.css` / `palette.css` не тронуты** — размер и штрих названы токенами
  самого компонента, а не глобальными; глобальный слой по своему комментарию
  держит только цвет, форму и время, и аудит контраста проверяет, что в нём нет
  ничего своего.
- **`vibe doc build-site` и `cargo` не запускались**; всё измерено на фикстурной
  библиотеке, как в W1-O1/W1-O4/W1-O6/W1-O3B.
- **`tools/parity.mjs` не запускался** (требует собранного дерева чужого
  Astro-сайта). Новых видимых строк чрома я не добавил ни одной: единственный
  новый текст — `aria-label`/`title` плитки, и это словарь поля.
- **Ничего не отправлено (`push`)**, отчёт не коммичен.

## Скриншоты

`campaigns/docs-2026-09/findings/W1-O5-shots/` — 9 файлов.

Коммит N (`d77044ba`):

- `catalogue-1440-{light,dark}`, `catalogue-390-{light,dark}` — полка с глифом
  `doc` на плитке, обе темы, обе ширины;
- `package-1440-{light,dark}`, `package-390-light` — шапка страницы пакета и
  карточки её полок с тем же глифом.

Коммит O (`11319f58`):

- `contents-ru-1440-{light,dark}` — колонка на русской половине: припинённая
  ссылка «Каждый блок по разу» и под заголовком «Справочные страницы» — «Addresses»
  словами источника. До: `W1-O3B-shots/p-contents-ru-1440-light.png`.
