# WORKER-REPORT FB-O1 — главная: карта пунктов меню в супрематизме — 2026-09-26

Рабочее дерево: `C:\Users\olegc\git\v\vibevm\.wt\FB-O1` (detached от `main` @ `3add5a398`).
Git — только чтение; ничего не закоммичено. Все пути ниже — от
`vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/` (далее `web/`), если не сказано иначе.

## 1. Замысел

**Как прочитан супрематизм.** Не «картинки в стиле», а язык, который у семьи
уже есть и который читатель уже видел на Vision и AI-Native Language:
плоские геометрические массы без объёма и без контура — круг, квадрат,
клин, брус, — на нейтральной плоскости; немного сильных цветов при чёрном и
белом; асимметрия, диагональ и разномасштабность вместо сетки одинаковых
карточек; тонкие «чертёжные» линии (пол, риски, крестики) как воздух
композиции. Ключевое — **словарь уже назначен** в `vision/art.tsx`,
`why/ai-native/art.tsx`, `why/vibevm/frieze.tsx`, `why/zap/art.tsx`, и я его не
переназначал: терракотовый круг — человеческое намерение и вообще «своё,
VibeVM»; наклонённый кобальтовый квадрат — машинное намерение; золото — слой
спецификации и дисциплина AI-Native; зелёный — Zap; три минеральных метки
языков (оксидный квадрат, сланцевый круг, бирюзовый треугольник); чистая
краска (`--text`) — детерминированное: страница, коммит, код. Новых цветов
не чеканил — см. §4.

**Как это ложится на всю страницу.** Три слоя:

1. **Земля** (`landing/field.tsx` + `field.css`, `.landing-field`): четыре
   большие плоскости и четыре чертёжных крестика позади всего содержимого
   главной — от hero до карты. Наклонённая терракотовая плоскость за
   созвездием hero (уходит за верхний край), длинный тонкий брус краски в
   пустой правой нижней четверти hero (диагональ фриза Why VibeVM),
   кобальтовый квадрат за вторым рядом карты, золотой диск за первым.
   Интенсивность — 5–8 % смеси токена с прозрачностью, то есть слабее, чем
   уже аудированная подложка `--selection` (14 % терракоты), поэтому ни одна
   текстовая роль не оказывается на фоне хуже измеренного. Плоскости
   абсолютны относительно `.landing-page`, не fixed: они поставлены против
   содержимого (верхние — от верха, нижние — от низа, где карта стоит над
   футером при любой ширине), а не ползут за прокруткой. Hero, карточки и
   плашки непрозрачны и **режут** эти плоскости — этот наплыв и есть
   композиция. Обрезаны `overflow: clip`, так что расширить документ не
   могут.
2. **Карта** (`landing/map.tsx` + `map.css`, `.landing-map`): раздел под
   тремя карточками, над мелкой припиской. Два ряда (band), как две строки
   шапки; в каждом — плашки-плоскости разного размера на 12 колонках.
3. **Рисунки** (`landing/art-tools.tsx`, `art-story.tsx`, `art.tsx`): восемь
   SVG, у каждого — подпись плашки как `role="img"`/`aria-label`, сам SVG
   `aria-hidden`. Цвета только через классы-крючки в `map.css` → токены.

**Движение — только CSS, и его две карты.** Каждый рисующийся/появляющийся
элемент несёт `--at` — своё место в хореографии. Где браузер умеет
scroll-driven animations (`@supports (animation-timeline: view())`: Chromium,
Safari 26+), плашка объявляет `view-timeline: --lm-plate`, и её рисунок
**рисуется по мере входа плашки в окно** — читатель доходит до карты и видит,
как она проявляется; диапазон `entry X% .. entry X%+50%` (не `cover`: хвост
страницы не может прокрутиться дальше футера, а `entry` завершается, как
только плашка вошла целиком). Где не умеет — те же keyframes идут один раз при
загрузке, `--at × 50ms` задержки. Обратимость входа (при прокрутке назад
рисунок «разрисовывается» уходя за нижний край) — свойство view-таймлайна,
осознанное: рисунок следует за читателем. Единственное бесконечное движение
— кольца Zap, один оборот за четыре минуты, как на самой странице Zap.
`prefers-reduced-motion`: всё `animation: none`, конечные состояния,
кольца стоят, hover без сдвига. Ничего не гидратируется, инлайн-скриптов
нет (CSP: 68 хешей — столько же, сколько до правки).

## 2. Форма раздела — и почему не «большие горизонтальные плашки»

Владелец предложил горизонтальные плашки как вариант. Восемь одинаковых
горизонталей — это лента, а не композиция, и очень длинная страница. Вместо
этого — **асимметричная мозаика, разная в двух рядах**, чтобы раздел читался
как одна работа:

- **01 · The software / Софт** — Documentation: башня в 6 колонок и 2 ряда
  (главное направление — самая большая масса, слева); GitHub и GitVerse:
  два малых квадрата справа сверху — **зеркальная пара** (один и тот же
  рисунок отражён; у оригинала квадрат-исходник сплошной, у зеркала —
  полый: это копия); News & support: широкая плашка справа снизу, слова
  слева — рисунок справа (уравновешивает башню).
- **02 · The argument / Смысл** — Vision: одна лента на всю ширину (эссе —
  мировоззрение, которому три продукта — части, ровно как объясняет порядок
  шапки в `chrome.tsx`); под ней **три равные трети** — Why VibeVM, Why Zap,
  AI-Native Language: три акцента под одной крышей (`palette.css` так и
  описывает семью).

Размер плашки — данное записи меню (`plate: "tall" | "small" | "wide" |
"third" | "band"` в `menu.ts`), а раскладку делает `grid-auto-flow: dense`:
CSS не знает имён плашек, и девятая запись встанет сама, того размера, что
объявит. Планшет (≤ 900 px): первый ряд переобъявлен — Documentation
горизонтальной плашкой во всю ширину (рисунок слева), пара зеркал 3+3, News
во всю ширину (рисунок справа); второй ряд держит форму на 6 колонках
(трети — 2+2+2). Телефон (≤ 600 px): одна колонка, но с ритмом — башня с
рисунком сверху, пара зеркал 3+3 бок о бок, News горизонталью, Vision с
рисунком над словами, три продукта — горизонталями, где рисунок чередует
сторону (зигзаг через `:nth-child(odd)`); словам на телефоне — широкая
колонка (1.15fr/0.85fr).

Плашка целиком — ссылка (как карточки News & support): одна цель, одна
остановка, одно кольцо фокуса в радиусе плашки. Имя ссылки — только имя
пункта (`aria-labelledby` на `<h3>`), стрелка нарисована CSS
(`content: "→" / ""`), поэтому список ссылок читается как шапка.
`<ul>`/`<li>` — читателю экранной программы говорят, сколько мест в ряду.

## 3. Один список для шапки и карты

`site/src/landing/menu.ts` — `landingMenu(locale, here)`: два ряда записей
`{ id, label, href, offSite, current, plate }` и то же по именам
(`entries`). Шапка (`chrome.tsx`) рендерит ряды из него (те же классы
`landing-nav__row--tools/--story`, тот же `rel="noopener"` у внешних, тот же
`aria-current`); футер берёт из `entries` в своём прежнем порядке; карта
рендерит те же ряды. Слова карты (`mapK`, `mapTitle`, `mapLede`,
`mapRows`, `mapPlates: Record<MenuId, {body, alt}>`) — в `i18n.ts`, по тем же
id; рисунок — `art.tsx` c исчерпывающим `switch` по `MenuId`. Итог: новый
пункт меню без текста плашки или без рисунка — ошибка компиляции, а не дыра
на карте. Тест `site/tests/landing-map.spec.ts` читает оба списка со
собранной страницы и требует поэлементного равенства (label, href, rel) и
равной формы (длины рядов = длины лент).

## 4. Иллюстрация каждого пункта

| Пункт | Рисунок | Цвета |
| --- | --- | --- |
| Documentation | страница — высокая плоскость краски с короткими строками цвета фона, вторая страница позади (16 %), терракотовый номер блока поверх угла, длинная тонкая диагональ — линия чтения | `--text`, `--bg`, `--accent` |
| GitHub | граф коммитов: ствол с тремя пришпиленными узлами, терракотовая ветка уходит вправо и вливается обратно, сплошной квадрат — исходник | `--text`, `--accent`, `--bg-raise`, `--line-strong` |
| GitVerse | тот же граф, отражённый `matrix(-1 0 0 1 240 0)`; квадрат полый — копия | те же |
| News & support | вещание: терракотовый круг с ореолом, три дуги (rr 56/92/128, рисуются наружу, убывающая плотность), два приёмника — сплошная и полая наклонённая плоскости, пунктир ответа назад к кругу (поддержка) | `--accent`, `--text`, `--line-strong` |
| Vision | сжатый `IntentField`: круг человека с ореолом, наклонённый кобальтовый квадрат с волосками, золотая дуга спецификации, золотые рёбра к общему полу через узлы (кобальтовый квадратик, терракотовая точка) | `--accent`, `--intent-machine`, `--accent-gold`, `--text-2` |
| Why VibeVM | сжатый фриз: восемь наклонённых прямоугольников дрейфа, терракотовый клин входит справа сверху (`lm-enter`), брус-противовес, решётка: корень, два квадрата, четыре пина | `--text-3`, `--text`, `--accent`, `--bg-raise` |
| Why Zap | орбитальная карта: звёзды, четыре пунктирных кольца (вращаются), ядро с ореолом, узлы (полые/заполненные), один терракотовый узел — проект в семье, траектория со стрелкой | `--accent-zap`, `--accent`, `--zap-star`, `--bg-raise` |
| AI-Native Language | сжатая `Projection`: золотое ядро с ореолом, три луча на оксидный квадрат, сланцевый круг, бирюзовый треугольник, у каждого четыре золотые скобки; золотой пол с риской-выходом, пунктирные спуски | `--accent-gold`, `--mark-rust/ts/go`, `--text` |

Каждая плашка несёт словесное описание рисунка (`alt`, EN и RU) в
`aria-label`.

## 5. Новые токены и контраст

**Новых токенов нет.** Палитра семьи уже содержит всё, чем говорит
супрематизм этой страницы (терракота, кобальт, золото, зелень Zap, три
метки языков, краска/фон обеих тем); чеканить новый оттенок означало бы
выдать плашке подпись другого автора. Полупрозрачные плоскости земли —
`color-mix()` существующих токенов на ≤ 8 %, что ниже аудированной
`--selection`; плашки на 90 % `--bg-raise`, чтобы земля просвечивала.
`node design/audit/contrast.mjs` — без изменений, зелёный (вывод в §9).

## 6. Что стало с паритетом лендинга

Все три гейта сравнивают главную с Astro-эталоном; новый раздел меняет её
намеренно, поэтому у каждого гейта появилось именное правило, а не
исключение «на главной всё можно»:

- `tools/parity.mjs` — **D-40** (`where: "text"`): фрагменты карты
  вычисляются из таблицы `landing/i18n.ts` (kicker, заголовок, строка,
  имена рядов, `body` каждой плашки), как `NEWS_FRAGMENTS` для D-38. Имена
  плашек — это ярлыки шапки, объяснённые там, где появились (D-10, D-34,
  D-38, Why-страницы); множество фрагментов не считает слово дважды.
  `alt` — атрибуты, в текст не входят.
- `tools/layout-parity.mjs` — **L-11** (`where: "sections"`): секции
  сравниваются один к одному по порядку; лишняя секция сборки допускается,
  только если стоит **после** последней секции эталона и несёт заголовок
  карты (`mapTitle` любого языка, читается из i18n). Доля высоты секции
  теперь считается от суммы **сравниваемых** секций (`px` вместо доли от
  `main`) — иначе карта, дописанная после hero и карточек, уменьшила бы
  долю каждой из них на свою высоту и красила бы гейт за то, что ничего не
  сдвинулось. Отрицательный контроль (Zap против AI-Native) остаётся
  чутким: лишние секции AI-Native не носят заголовок карты → находка.
- `tools/visual-classify.mjs` — слой **`− map`**: скрывает `.landing-field`
  и `.landing-map` (как слой `− entry` скрывает дверь в эссе) — иначе одна
  плоскость земли за созвездием посчитала бы пятую часть пикселей hero
  «другими» за сдвиг цвета на несколько единиц. Таблица и пояснения отчёта
  получили колонку.
- `tools/visual-parity.mjs` — без правок: он «свидетельство, не вердикт»
  (по его же шапке).

**Единственный Astro-эталон на этой машине** — `C:\Users\olegc\git\v\vibevm-org\dist`
(сборка 2026-09-11, без Why-страниц, без `llms-full.txt`). Против него гейты
красны ещё **до** моих правок (baseline в `web/tmp/baseline/*`: parity —
37 unexplained, в т. ч. `disambiguatingDescription`, ярлыки Why в шапке,
приписка про Phala; layout — hero «78.4 % → 74.2 %» и «0 секций в эталоне» у
всех Why; classify — 100 % у Why). Поэтому полный зелёный прогон здесь
невозможен; проверено то, что можно: строки `home-en`/`home-ru` и `/`, `/ru/`
до и после (§9) — новые фрагменты идут как D-40, лишняя секция — как L-11,
доли hero/карточек не разъезжаются, residual hero с `− map` возвращается к
базовому. Центральной сессии: гнать гейты против свежего эталона с
Why-страницами.

## 7. Файлы

Новые: `site/src/landing/menu.ts`, `map.tsx`, `map.css`, `field.tsx`,
`field.css`, `art.tsx`, `art-tools.tsx`, `art-story.tsx`;
`site/tests/landing-map.spec.ts`.
Изменённые: `site/src/landing/i18n.ts` (тип `MapPlate`, поля `map*`, EN/RU),
`chrome.tsx` (ряды и футер из `menu.ts`), `landing.tsx` (`LandingField`,
`LandingMap`), `tools/parity.mjs` (D-40), `tools/layout-parity.mjs` (L-11,
доли), `tools/visual-classify.mjs` (слой `− map`), `specmap.json`
(перегенерирован: +6 рёбер).
Не тронуты: тексты и порядок hero/карточек/приписки, `design/*`, страница
News & support (см. §8).

## 8. Отклонения и решения, о которых стоит знать

- **News & support не перерисован.** Пакет разрешал привести саму страницу к
  языку карты, «если служит целому». Не служит: страница — «стойка» дома в
  домашних цветах, её пять карточек и тексты — владельца; холст с вещанием
  там был бы второй парадной. Её супрематистская иллюстрация живёт на
  главной, наравне со всеми (прямое требование владельца выполнено).
- **Обратимый scroll-driven вход** (описан в §1) — свойство таймлайна, не
  дефект; при загрузке с прокруткой посередине страницы плашки в окне уже
  дорисованы (`animation-fill-mode: both`).
- **Полностраничные скриншоты** сняты с принудительно завершёнными
  рисунками (тот же набор правил, что у `prefers-reduced-motion`): снимок
  всей страницы делается с окном в верхнем положении, и плашки ниже сгиба
  честно стояли бы «до входа». Снимки карты (`map-*`) — без подмены: окно
  1440×1900, прокрутка к разделу, все плашки вошли.
- Копия конфига Playwright на 4273 лежала в игнорируемом `web/tmp/` и
  удалена по окончании; скрипт съёмки — `web/tmp/shots.mjs` (игнорируется).
- Полный e2e-прогон: 254 passed, 5 skipped — пять пропусков в
  `local-reader.spec.ts` (строки 252–397) существовали до правки и не
  касаются главной.
- `README.md` пакета и `chrome.css` не трогал: композиция шапки не менялась,
  только источник её строк.

## 9. Самопроверка — вывод и коды выхода дословно

Все команды — из `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0` рабочего дерева.
Полные логи: `web/tmp/floor.log`, `contrast.log`, `build-2.log`,
`e2e-full.log`, `build-embedded.log`, `after/*.txt`, `baseline/*.txt`
(каталог `tmp/` игнорируется git).

### 9.1 `TYPESCRIPT_AI_NATIVE="C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe" node tools/floor.mjs --keep-going; echo "EXIT=$?"`

```
=== prettier --check (floor perimeter: design/src, site/src) ===
Checking formatting...
All matched files use Prettier code style!

=== tsc --noEmit ===

=== tests (node --test) ===
✔ a build told nothing publishes no analytics tag and claims one domain (1.1088ms)
✔ the featured coordinates arrive as a list, however they were spaced (0.1537ms)
✔ the analytics property and the host it reports to arrive separately (0.1183ms)
✔ an address keeps no trailing slash whichever way it was written (0.0937ms)
✔ each of the three themes arrives as itself (0.1017ms)
✔ a theme nobody defined is the site's own (0.1521ms)
▶ who wrote the prose, as a shelf is narrowed by it
  ✔ admits everything under the entry that narrows nothing (0.6845ms)
  ✔ puts each named word in its own group (0.2229ms)
  ✔ puts a document written by both hands in both groups (0.7372ms)
  ✔ leaves a document that says nothing out of both named groups (0.1104ms)
  ✔ treats a word this build does not know as a document that said nothing (0.0803ms)
  ✔ offers everything first, then the two named groups (0.7572ms)
✔ who wrote the prose, as a shelf is narrowed by it (3.4747ms)
▶ a bridge's two authorships, as a page is handed them
  ✔ hands over both lists and the licence, under the names a page reads (2.1613ms)
  ✔ answers nothing at all for a package that is not a bridge (0.4687ms)
✔ a bridge's two authorships, as a page is handed them (3.613ms)
▶ the manual's pages as the column lists them
  ✔ groups them by folder, in the order the manifest gave them (1.8929ms)
  ✔ puts the pinned pages first and outside their folders (0.1864ms)
  ✔ passes over a pin that names no page of this documentation (0.181ms)
  ✔ shows a folder under the title the reader's edition gives it (0.7164ms)
  ✔ falls to the source's words before it falls to the folder's name (0.2792ms)
  ✔ marks the page the reader is on, and only that one (0.1944ms)
  ✔ names a page in the words of the edition that will serve it (0.4183ms)
  ✔ names a pinned page the same way (0.2055ms)
  ✔ lists the source's pages under the adaptation's addresses (0.2627ms)
✔ the manual's pages as the column lists them (5.2809ms)
▶ the learning path the documentation declared
  ✔ numbers the chapters in order and leaves an appendix unnumbered (0.4872ms)
  ✔ holds the pages the author named, in the order they were named (0.1716ms)
  ✔ tells a documentation that declared none from one that declared nothing (0.1854ms)
  ✔ passes over a chapter's page that names no page of this documentation (0.0988ms)
  ✔ marks the page the reader is on in this view too, and only that one (0.1532ms)
  ✔ names a chapter in the reader's edition's words, falling back to the source's (0.1191ms)
  ✔ takes the source's path whole when the translation names no chapter (0.1632ms)
✔ the learning path the documentation declared (1.5932ms)
▶ where the path leads from one page of it
  ✔ offers only the way on from the first page (0.1945ms)
  ✔ offers only the way back from the last (0.09ms)
  ✔ names the chapter only where the path crosses into another one (0.1235ms)
  ✔ answers nothing for a page the declared path does not reach (0.0711ms)
  ✔ leads to the addresses and the words of the edition being read (0.0836ms)
  ✔ captions a crossing in the words of the edition being read (0.1414ms)
✔ where the path leads from one page of it (0.8597ms)
✔ href owns the leading slash so JSX never writes one (0.7766ms)
✔ a documentation address ends in a slash, with and without a language (0.1048ms)
✔ a projection is the page's path as a file, so it loses the slash (0.1402ms)
✔ the language segment is told from a group by the dot, not by a list (0.6254ms)
✔ what is not an address parses as none of them (0.1711ms)
✔ parse and print are each other's inverse (0.1689ms)
✔ a package's own address is the page's address one segment short (0.1475ms)
✔ one parser reads both addresses and says which it found (0.1964ms)
✔ the served path reads back as the segments the route matched (0.2089ms)
✔ a citation carries the version and no language (0.1979ms)
✔ no source outside href.ts writes a documentation path as a literal (24.0167ms)
✔ a click on a block number is the block's address (1.2272ms)
✔ a click on a quoted rule carries the spec address, not the page link (0.1351ms)
✔ an ordinary link stays an ordinary link (0.1068ms)
✔ prose is not an intent (0.1054ms)
✔ the walk gives up before it leaves the island (0.1026ms)
▶ the library a build renders from
  ✔ is the trees' when the deployment named trees (3.3034ms)
  ✔ is the package's own fixture pair when it named none (0.9337ms)
✔ the library a build renders from (4.7568ms)
▶ as many libraries as there are source documentations
  ✔ gives two documentations a library each (2.4637ms)
  ✔ attributes an adaptation to the source it names, not to the first one (2.5432ms)
  ✔ serves each library under its own coordinate, the adaptation behind its source's (2.3698ms)
  ✔ keeps one catalogue per language when two libraries share one (1.8592ms)
  ✔ is a library of its own when the source of an adaptation is absent (0.7077ms)
✔ as many libraries as there are source documentations (10.1753ms)
▶ the addresses a library declares
  ✔ gives a tree's documentation its package page and one page each (0.7057ms)
  ✔ serves an adaptation under the source's coordinate with its language in front (0.3158ms)
  ✔ writes every page at both spellings of the version (0.4634ms)
✔ the addresses a library declares (1.6091ms)
▶ the island behind each address
  ✔ is the bytes of that page's own index.html, at both spellings (1.2771ms)
  ✔ is the source's own when the adaptation has not reached the page (1.3727ms)
  ✔ never reads one library's page into another library's address (2.6837ms)
✔ the island behind each address (5.4167ms)
▶ the package page of a library out of a tree
  ✔ shows the tree's own card and its pages (1.2807ms)
  ✔ answers from the library the address names, over many (1.8663ms)
✔ the package page of a library out of a tree (3.2082ms)
▶ the door
  ✔ lists every edition of every library, source before adaptation (1.8106ms)
  ✔ features by the documentation's coordinate, adaptations included (1.9439ms)
  ✔ tells a rendering of a package from a documentation about one (0.1773ms)
  ✔ wears the kind the manifest names, and none when it names none (0.426ms)
  ✔ carries each edition's own authorship onto its card (1.7745ms)
✔ the door (6.2401ms)
✔ the source manifest parses (1.4937ms)
✔ the adaptation parses and is a page short of its source (0.4091ms)
✔ an absent optional field stays absent rather than becoming undefined (0.2901ms)
✔ who wrote the prose survives the crossing from bytes to type (0.6643ms)
✔ a fourth authorship is refused, and none at all is not (0.4974ms)
✔ a bridge's two authorships arrive apart and stay apart (0.9961ms)
✔ the level-zero mark is read as written, and absence is not false (0.3611ms)
✔ the kind of the rendered package crosses as itself (0.3141ms)
✔ a value outside a closed vocabulary is refused by name (0.1874ms)
✔ the failure names the path, not just the fact of failing (0.3485ms)
✔ anything that is not a manifest is not a manifest (0.1539ms)
✔ a declared learning path crosses, and no path stays no path (0.6331ms)
✔ a malformed chapter is refused by the path of the field that failed (0.6652ms)
✔ a word from a page's title finds that page (1.6196ms)
✔ a word from a page's abstract finds that page (0.164ms)
✔ a title outranks an abstract for the same word (0.1092ms)
✔ a second word narrows rather than widens (0.4414ms)
✔ the coordinate a page is published under finds it (0.1543ms)
✔ a Russian word finds a Russian edition, by title and by abstract (0.1462ms)
✔ a word nothing carries finds nothing (0.6273ms)
✔ a query too short to mean anything is not answered (0.1217ms)
✔ the number of answers is bounded (0.1846ms)
✔ anything that is not an index is not one (0.2495ms)
✔ an index that is one is read whole (0.3359ms)
✔ the two languages are the two the site is published in (1.0178ms)
✔ every row reads both ways, so a switch back is not a reload (0.2103ms)
✔ no two rows share a translation (0.0985ms)
✔ a string the chrome does not carry is left alone (0.0827ms)
✔ the browser's first known preference decides, in its own order (1.4001ms)
✔ the script the design system ships declares its own default (0.7465ms)
✔ a configured theme replaces the declaration and nothing else (0.2066ms)
✔ a script that says nowhere to put the default is refused (0.4242ms)
▶ the learning path on a package's own page
  ✔ shelves the pages by chapter, in the order of the path (4.8743ms)
  ✔ opens the documentation at the first page of the path (1.0125ms)
  ✔ names the chapters in the words of the edition being read (0.612ms)
  ✔ leaves a documentation that declared no path exactly as it was (1.8736ms)
  ✔ ends each page of the path with the neighbours it has (2.2319ms)
  ✔ shows no neighbours at all on a page of a documentation with no path (1.3165ms)
✔ the learning path on a package's own page (13.0598ms)
✔ a data block is not a script, and an external one is not inline (1.3347ms)
✔ a `>` inside an attribute does not end the tag (0.1179ms)
✔ the hash is over the bytes, and the policy names it once (0.756ms)
✔ the policy names no host at all (0.1769ms)
✔ the serving fragment carries the policy on documents and nowhere else (0.3498ms)
✔ a policy that cannot be quoted is refused (0.324ms)
✔ a policy the server cannot parse becomes no policy, loudly (0.9611ms)
✔ the generator accepts the policy this package's build wrote (0.5444ms)
✔ the compose file is the two containers and the volume between them (0.6007ms)
✔ the service names and the port are placeholders the owner replaces (0.2005ms)
✔ the image pins the Node the package's engines name (0.1003ms)
✔ the image pins the compiler the workspace manifest declares (0.1694ms)
✔ the renderer renders and the serving image holds nothing (0.1515ms)
✔ the build context is named in, never filtered out (0.1254ms)
✔ the shipped configuration carries the defaults and no live value (0.1858ms)
✔ a page of the local reader declares no head of its own (0.9279ms)
✔ a local page publishes none of the public head (1.1954ms)
✔ every pattern the embedded gate uses finds the tag it names (0.3591ms)
✔ the card of a page is the address its own manifest names (0.5368ms)
✔ a manifest that names none falls through to what the build found (0.0766ms)
✔ a map the environment did not carry is no map at all (0.4119ms)
✔ the builder's own directory is not part of the domain (0.6527ms)
✔ the optimizer's manifest is answered 404 wherever it appears (0.122ms)
✔ the immutable cache names the three content-addressed directories (0.1811ms)
✔ the unhashed root files are not cached for a year (0.0891ms)
✔ every text format the site publishes declares utf-8 (0.1626ms)
✔ every redirect this container writes is relative (0.1229ms)
✔ no rule sends a reader to a plaintext address (0.0839ms)
✔ the policy is included from the file the build generates (0.4876ms)
✔ the inherited rules of the landing are still here (0.3125ms)
✔ a location that sets a header of its own keeps the security ones (0.2673ms)
ℹ tests 141
ℹ suites 12
ℹ pass 141
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 366.1738

=== eslint (floor perimeter: design/src, site/src) ===

=== typescript-ai-native-conform check ===
typescript-ai-native-conform: policy conform.toml (loaded).
typescript-ai-native-conform: extracted 178 file(s), 0 cached (producer ts-tsc-2).
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
typescript-ai-native-conform: 0 cell(s) gated, 0 exempt — see conform.toml for the why of each.

=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 174 tagged code items, 174 edges, 0 suspects, 174 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).

=== test-gate (xfail-strict) ===
test-gate: running `node --test --test-reporter=tap` over the policy's TS roots …
test-gate: 153 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).

floor: all green (7 step(s) run, 0 disabled by policy).
EXIT=0
```

### 9.2 `node design/audit/contrast.mjs; echo "EXIT=$?"`

```
APCA audit — both themes, thresholds by role

--- light ---
  --text         on --bg           Lc=  99.99  min= 75  EXCELLENT
  --text         on --bg-raise     Lc= 101.22  min= 75  EXCELLENT
  --text         on --bg-sink      Lc=  93.44  min= 75  EXCELLENT
  --text         on --code-bg      Lc=  92.46  min= 75  EXCELLENT
  --text         on --selection    Lc=  77.10  min= 75  PASS
  --text         on --accent-soft  Lc=  93.80  min= 75  EXCELLENT
  --text-2       on --bg           Lc=  85.17  min= 60  EXCELLENT
  --text-2       on --bg-raise     Lc=  86.40  min= 60  EXCELLENT
  --text-2       on --bg-sink      Lc=  78.63  min= 60  EXCELLENT
  --text-2       on --code-bg      Lc=  77.64  min= 60  EXCELLENT
  --text-2       on --selection    Lc=  62.28  min= 60  PASS
  --text-2       on --accent-soft  Lc=  78.98  min= 60  EXCELLENT
  --text-3       on --bg           Lc=  85.17  min= 60  EXCELLENT
  --text-3       on --bg-raise     Lc=  86.40  min= 60  EXCELLENT
  --text-3       on --bg-sink      Lc=  78.63  min= 60  EXCELLENT
  --text-3       on --code-bg      Lc=  77.64  min= 60  EXCELLENT
  --text-3       on --selection    Lc=  62.28  min= 60  PASS
  --text-3       on --accent-soft  Lc=  78.98  min= 60  EXCELLENT
  --accent       on --bg           Lc=  54.23  min= 45  GOOD
  --accent-hover on --bg           Lc=  65.03  min= 45  EXCELLENT
  --accent-gold  on --bg           Lc=  77.81  min= 45  EXCELLENT
  --accent-zap   on --bg           Lc=  47.13  min= 45  PASS
  --intent-machine on --bg           Lc=  80.13  min= 45  EXCELLENT
  --line         on --bg           Lc=  14.01  decorative, not gated
  --line-strong  on --bg           Lc=  27.12  decorative, not gated
  block number   on --bg           Lc=  45.41  decorative, not gated

--- dark ---
  --text         on --bg           Lc= -98.11  min= 75  EXCELLENT
  --text         on --bg-raise     Lc= -97.35  min= 75  EXCELLENT
  --text         on --bg-sink      Lc= -98.33  min= 75  EXCELLENT
  --text         on --code-bg      Lc= -98.33  min= 75  EXCELLENT
  --text         on --selection    Lc= -95.98  min= 75  EXCELLENT
  --text         on --accent-soft  Lc= -95.98  min= 75  EXCELLENT
  --text-2       on --bg           Lc= -64.31  min= 60  PASS
  --text-2       on --bg-raise     Lc= -63.54  min= 60  PASS
  --text-2       on --bg-sink      Lc= -64.52  min= 60  PASS
  --text-2       on --code-bg      Lc= -64.52  min= 60  PASS
  --text-2       on --selection    Lc= -62.17  min= 60  PASS
  --text-2       on --accent-soft  Lc= -62.17  min= 60  PASS
  --text-3       on --bg           Lc= -64.18  min= 60  PASS
  --text-3       on --bg-raise     Lc= -63.42  min= 60  PASS
  --text-3       on --bg-sink      Lc= -64.40  min= 60  PASS
  --text-3       on --code-bg      Lc= -64.40  min= 60  PASS
  --text-3       on --selection    Lc= -62.05  min= 60  PASS
  --text-3       on --accent-soft  Lc= -62.05  min= 60  PASS
  --accent       on --bg           Lc= -47.20  min= 45  PASS
  --accent-hover on --bg           Lc= -50.52  min= 45  GOOD
  --accent-gold  on --bg           Lc= -57.38  min= 45  GOOD
  --accent-zap   on --bg           Lc= -69.64  min= 45  EXCELLENT
  --intent-machine on --bg           Lc= -60.57  min= 45  EXCELLENT
  --line         on --bg           Lc=   0.00  decorative, not gated
  --line-strong  on --bg           Lc=   0.00  decorative, not gated
  block number   on --bg           Lc= -22.55  decorative, not gated

=== pairs: gated=46, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
EXIT=0
```

### 9.3 `node tools/build.mjs static; echo "EXIT=$?"` (хвост; полный лог — `tmp/build-2.log`, 449 строк, из них ~400 — проверка ссылок)

```
build (static): generated 28 page(s), expected 28
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 14 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver, search index over 5 entries
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 13 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 68 inline script hash(es) over 177 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
build (static): no trace of the previous framework — 4 mark(s) looked for, none found
documentation links — every address against the files behind it
links: green — 1142 checked, 0 broken.
build (static): ok
EXIT=0
```

(Baseline той же сборки до правки, `tmp/baseline/build.log`: те же 28 страниц,
те же 68 хешей CSP, `links: green — 1126 checked` — новые 16 ссылок — это
восемь плашек × два языка.)

### 9.4 `node node_modules/@playwright/test/cli.js test -c tmp/playwright.config.ts; echo "EXIT=$?"` (копия конфига на 4273; строки нового спека и итог)

```
  ok  45 [chromium] › site\tests\landing-map.spec.ts:102:3 › / maps every header destination, in the header's order (136ms)
  ok  46 [chromium] › site\tests\landing-map.spec.ts:102:3 › /ru/ maps every header destination, in the header's order (152ms)
  ok  47 [chromium] › site\tests\landing-map.spec.ts:134:3 › / keeps the map under the content and the small print last (141ms)
  ok  48 [chromium] › site\tests\landing-map.spec.ts:134:3 › /ru/ keeps the map under the content and the small print last (151ms)
  ok  49 [chromium] › site\tests\landing-map.spec.ts:168:3 › / names each plate once and hides its drawing (137ms)
  ok  50 [chromium] › site\tests\landing-map.spec.ts:168:3 › /ru/ names each plate once and hides its drawing (145ms)
  ok  51 [chromium] › site\tests\landing-map.spec.ts:206:3 › / gives a plate a visible focus ring (141ms)
  ok  52 [chromium] › site\tests\landing-map.spec.ts:206:3 › /ru/ gives a plate a visible focus ring (154ms)
  ok  53 [chromium] › site\tests\landing-map.spec.ts:225:3 › / announces Zap on the map and does not install it (126ms)
  ok  54 [chromium] › site\tests\landing-map.spec.ts:225:3 › /ru/ announces Zap on the map and does not install it (132ms)
  ok  55 [chromium] › site\tests\landing-map.spec.ts:239:5 › / does not scroll sideways at 1440px (116ms)
  ok  56 [chromium] › site\tests\landing-map.spec.ts:239:5 › / does not scroll sideways at 834px (108ms)
  ok  57 [chromium] › site\tests\landing-map.spec.ts:239:5 › / does not scroll sideways at 390px (94ms)
  ok  58 [chromium] › site\tests\landing-map.spec.ts:239:5 › /ru/ does not scroll sideways at 1440px (116ms)
  ok  59 [chromium] › site\tests\landing-map.spec.ts:239:5 › /ru/ does not scroll sideways at 834px (115ms)
  ok  60 [chromium] › site\tests\landing-map.spec.ts:239:5 › /ru/ does not scroll sideways at 390px (113ms)
  ok  61 [chromium] › site\tests\landing-map.spec.ts:257:3 › / stands still for a reader who asked (104ms)
  ok  62 [chromium] › site\tests\landing-map.spec.ts:257:3 › /ru/ stands still for a reader who asked (121ms)
  ok  63 [chromium] › site\tests\landing-map.spec.ts:284:3 › / draws the map for a reader who scrolls to it (2.0s)
  ok  64 [chromium] › site\tests\landing-map.spec.ts:284:3 › /ru/ draws the map for a reader who scrolls to it (2.0s)
  …
  5 skipped
  254 passed (1.1m)
EXIT=0
```

Пропущенные пять — `local-reader.spec.ts:252, 284, 325, 374, 397`
(существовавшие до правки `test.skip`, не о главной). Все `chrome.spec.ts`
(композиция шапки на пяти ширинах и двух языках, порядок табуляции),
`news.spec.ts` (первый ряд шапки заканчивается каналами) и
`disambiguation.spec.ts` (приписка последняя в `main`) — зелёные.

### 9.5 `node tools/build.mjs embedded; echo "EXIT=$?"` (хвост)

```
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): no trace of the previous framework — 4 mark(s) looked for, none found
build (embedded): ok
EXIT=0
```

### 9.6 `C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe specmap --path .`

```
  drift: edges added: 6
typescript-ai-native-specmap: wrote .\specmap.json (0 spec units, 174 tagged code items, 174 edges, 0 suspects, 174 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).
SPECMAP_EXIT=0
```

### 9.7 Гейты паритета против `C:/Users/olegc/git/v/vibevm-org/dist` (устаревший эталон, см. §6) — до и после

`node tools/parity.mjs <ref>` — до: `parity: RED — 37 unexplained difference(s), 57 deliberate.` (EXIT=1);
после: `parity: RED — 37 unexplained difference(s), 83 deliberate.` (EXIT=1).
Те же 37 необъяснённых до и после (Why-страницы, `llms-full.txt`,
`disambiguatingDescription`, ярлыки Why в шапке, приписка про Phala, ссылки
каналов в `llms.txt` — всё существовало до правки); +26 объяснённых — это
D-40 (13 фрагментов × 2 языка). Выдержка `--- page /` после:

```
    D-40      text added: «Where to go»
    D-40      text added: «Everything here, on one plane.»
    D-40      text added: «Everything the header links to, drawn large enough to see without reading the header. Pick one.»
    D-40      text added: «The software»
    D-40      text added: «The manual for vibe and the packages it installs — read online, page by page, in English and in Russian.»
    D-40      text added: «The canonical repository: the code, the issues, the releases.»
    D-40      text added: «The same source, mirrored on GitVerse.»
    D-40      text added: «Release news on Telegram, a chat for questions and bug reports, the community on Reddit — and the creator on X.»
    D-40      text added: «The argument»
    D-40      text added: «The essay behind everything on this site: two sources of intention, an expensive probabilistic layer over a cheap deterministic one, and traceable edges between them.»
    D-40      text added: «The product thesis: discipline you can install. Specs, flows and skills as versioned, pinned packages — and the context an agent boots from, computed from them.»
    D-40      text added: «Announced for October 2026: a local workspace that puts coding agents, their questions and their worktrees on one map. A declaration of intent, not a release.»
    D-40      text added: «One discipline over Rust, TypeScript and Go: the code stays ordinary, and the strictness lives around it — in gates a machine runs.»
```

`node tools/layout-parity.mjs <ref>` — до: `layout parity: RED — 23 unexplained finding(s) over 24 comparison(s).`,
в т. ч. пять о главной:

```
- `/ @ desktop` — section 1 «Install the context your agents run on.»: takes 78.4% of the page in the reference, 74.2% here
- `/ @ mobile` — section 1 «Install the context your agents run on.»: takes 65.4% of the page in the reference, 61.6% here
- `/ru/ @ desktop` — section 1 «Установите контекст для вашего агента»: takes 76.5% of the page in the reference, 71.5% here
- `/ru/ @ tablet` — section 1 «Установите контекст для вашего агента»: takes 65.7% of the page in the reference, 62.1% here
- `/ru/ @ mobile` — section 1 «Установите контекст для вашего агента»: takes 64.6% of the page in the reference, 60.5% here
```

после: `layout parity: RED — 18 unexplained finding(s) over 24 comparison(s).` —
все 18 о Why-страницах, которых в эталоне нет («0 section(s) in the
reference»); о главной ни одной находки, карта объяснена:

```
- L-11 `/ @ desktop` — section 4 «Everything here, on one plane.» stands after the reference's 3
- L-11 `/ @ tablet` — section 4 «Everything here, on one plane.» stands after the reference's 3
- L-11 `/ @ mobile` — section 4 «Everything here, on one plane.» stands after the reference's 3
- L-11 `/ru/ @ desktop` — section 4 «Весь сайт на одной плоскости.» stands after the reference's 3
- L-11 `/ru/ @ tablet` — section 4 «Весь сайт на одной плоскости.» stands after the reference's 3
- L-11 `/ru/ @ mobile` — section 4 «Весь сайт на одной плоскости.» stands after the reference's 3
Negative control: comparing two different pages produces 1 finding(s), so the measurement is awake.
```

(Пять «пропавших» находок о доле hero — следствие нормировки долей по
сумме сравниваемых секций, а не по `main`: приписка и поля больше не в
знаменателе. Это то, что гейт и хотел измерять — пропорцию композиции.)

`node tools/visual-classify.mjs <ref>` — главная, до / после:

```
до:    | `/`    | 11.19% | +10px | 7.23%  |          7.23% | 7.11% | 5.68% | 1.20% |
       | `/ru/` | 12.10% | +10px | 7.61%  |          7.61% | 7.47% | 5.80% | 1.32% |
после: | `/`    | 24.52% | +10px | 20.72% | 20.72% | 7.23% | 7.11% | 5.68% | 1.20% |
       | `/ru/` | 25.44% | +10px | 21.10% | 21.10% | 7.61% | 7.47% | 5.80% | 1.32% |
                                            − entry  − map   + type + tones  residual
```

Слой `− map` возвращает hero ровно к базовым 7.23 % / 7.61 %; residual
1.20 % / 1.32 % не изменился. Вердикт обоих прогонов `RED` — только из-за
шести Why-страниц (100 %), которых в этом эталоне нет.

## 10. Скриншоты

`campaigns/docs-2026-09/findings/FB-O1-shots/` (20 PNG, 7.4 МБ; порт 4273,
Chromium, device scale 1, снято `web/tmp/shots.mjs` над `site/dist`).
*Примечание центральной сессии:* в репозиторий снимки не вошли (крупные
двоичные файлы) и сохранены только локально у центральной сессии; ниже —
что на них.

- `landing-{en,ru}-1440-{dark,light}.png` — первый экран 1440×900 (земля за
  созвездием, брус, крестики).
- `landing-{en,ru}-390-{dark,light}.png` — первый экран телефона 390×844.
- `landing-{en,ru}-full-{dark,light}.png` — вся главная 1440 целиком:
  hero → карточки → карта → приписка → футер (рисунки в конечном
  состоянии, см. §8).
- `landing-{en,ru}-full-390-{dark,light}.png` — вся главная телефона
  целиком.
- `map-{en,ru}-1440-{dark,light}.png` — раздел карты, как его видит
  дошедший до него читатель: окно 1440×1900, все плашки вошли, все
  рисунки дорисованы самим scroll-driven входом.

## 11. Приёмка центральной сессией — 2026-09-26

- Ревью диффа и снимков: принято. Одна правка текста: подводка карты
  переписана проще и без слова «шапка» — EN «Every place the menu at the
  top leads to, drawn large. Pick one.», RU «Всё, куда ведёт меню наверху,
  — крупно и в картинках. Выбирайте.» (D-40 читает строки из таблицы, так
  что выдержка §9.7 выше показывает прежнюю формулировку).
- Коммит перенесён на `main` @ `cf4a9f802` (глоссарий и свёрнутые
  цитаты) без конфликтов; `specmap --check` на объединённом дереве —
  clean, 177 рёбер.
- Гейты на объединённом дереве: floor — all green (7 шагов), контраст —
  EXIT=0, static — `links: green — 1170 checked, 0 broken`, e2e —
  265 passed, 5 skipped (прежние пропуски `local-reader`), 0 failed.
- Собственные снимки центральной сессии: 834 EN светлая и RU тёмная (раскладка
  планшета, без горизонтальной прокрутки), 1440 верх страницы (созвездие
  дорисовывается своей прежней анимацией за ~4 с), наведение на плашку Zap,
  телефон 390; при `reduced motion` — 0 анимаций.
