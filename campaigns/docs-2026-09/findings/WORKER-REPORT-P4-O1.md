# WORKER-REPORT-P4-O1 — пакет сайта, токены и темы, контракт острова

Пакет: `campaigns/docs-2026-09/findings/PACKET-P4-O1.md` (A4.1, A4.10, A4.2, A4.9).
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Четыре атома сделаны, четыре коммита в ветке, незакоммиченных правок у меня
нет. `pnpm floor` — **семь шагов зелёные плюс аудит контраста** (сорок
гейтуемых пар, ноль ниже порога). `pnpm build:static` — **3 страницы**
(ожидалось 3), `pnpm build:embedded` — **1 страница** (ожидалась 1).
`cargo xtask check-codegen` — clean. Шаг панели добавлен и проверен из корня
хоста; всю панель не гонял (её занимает P1-O6).

Два результата, которые стоит прочитать оркестратору отдельно:

1. **Найден и обойдён баг Qwik 2.0.0-beta.43**, стоивший половины времени
   атома A4.2: `<RouterOutlet />` внутри `layout.tsx` уводит SSG в
   **синхронный бесконечный цикл** — ни ошибки, ни вывода, ни одного байта на
   диск, только «SSG render timed out after 30000ms» для каждой страницы.
   Лечение: в layout — `<Slot />`, `<RouterOutlet />` только в `root.tsx`.
   Подробности и метод диагностики — в «Аномалиях», А-1.
2. **`target/debug/vibe.exe check --path <пакет>` красный** и таким останется:
   гейт требует `vibevm/vibespecs/boot/` от **любого** пакета, кроме вида
   `doc`. Это не про вид `app` — ровно так же он отказывает `stack`
   `typescript-ai-native` и `tool` `jtd-codegen`. Дефект продукта, не чинил
   (А-4).

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A4.1 | `4ccc9438` | `feat(web): open the site package under the TypeScript discipline` |
| A4.10 (токены) | `3e0efc08` | `feat(web): lay down the semantic tokens and both themes` |
| A4.10 (компоненты) | `90f21883` | `feat(web): build the shell components on the tokens` |
| A4.2 + A4.9 | `8ed08eca` | `feat(web): fix the island contract against the doc manifest` |

Между моими коммитами в ветку легли чужие (`6923c7ac`, `c3983882`,
`7f919413` и другие) — это ожидаемо, чужого я не стейджил, каждый коммит
делал явной формой `git commit -m … -- <свои пути>`.

## A4.1 — пакет `org.vibevm.doc/web` (`4ccc9438`)

`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/` — новая группа, вид `app`,
UPL-1.0, `[requires]` на `stack:org.vibevm.ai-native/typescript-ai-native`
(`^1.0`). `abstract` отвечает на четыре вопроса карточки D-20. Рабочее
пространство pnpm из двух частей, ровно как требует `##STACK-WORKSPACE`:
`design/` (`@vibe-docs/design`) и `site/` (`@vibe-docs/site`, зависит от
дизайна через `workspace:*`).

**Пины.** Все точные, как просит `##STACK-QWIK` («pinned exactly together
with the versions of Node and pnpm»): `@qwik.dev/core` и `@qwik.dev/router`
`2.0.0-beta.43`, `vite` `8.2.1`, `typescript` `5.9.3`, `prettier` `3.9.6`,
`eslint` `9.39.5`, `typescript-eslint` `8.70.0`, `@types/node` `26.5.1`;
`packageManager: "pnpm@10.33.2"`, `engines.node: "24.18.0"`. Три последних
версии я не выдумывал: поставил сначала каретками, посмотрел, что реально
резолвится, и переписал точными значениями — так пин совпадает с lock-файлом,
а не спорит с ним.

**`@types/node` — добавка к списку пакета.** Списка `typescript`, `prettier`,
`eslint`, `typescript-eslint` мало: `tsconfig` дисциплины несёт
`types: ["node", "vite/client"]` (A0.14 §4, К-5), а `tools/*.mjs` и
`design/audit/contrast.mjs` — обычный Node. Без `@types/node` шаг `tsc`
красный.

**`allowBuilds`.** Норма называет именно `allowBuilds` со значениями `false`,
и pnpm 10.33.2 это понимает (проверено по строкам 138341-138345 и 175891-175898
его собственного `pnpm.mjs`: это общий ключ `pnpm-workspace.yaml`, карта
пакет → булево, а не только про git-зависимости). `sharp` и `@parcel/watcher`
реально в дереве (транзитивные), так что обе записи живые, а не
профилактические.

**`tsconfig.json`** — полный пол дисциплины (все одиннадцать флагов GUIDE §1,
`exactOptionalPropertyTypes` сохранён) плюс блоки Qwik: `jsx: "react-jsx"`,
`jsxImportSource: "@qwik.dev/core"`, `module: "esnext"`,
`moduleResolution: "Bundler"`, `lib`, `allowImportingTsExtensions`,
`verbatimModuleSyntax`, `isolatedModules`, `outDir: "tmp"`, `noEmit`,
`skipLibCheck`, `paths`. `typescript-plugin-css-modules` из стартера не взят:
CSS-модулей в пакете нет, а лишняя зависимость — лишний пин.

**Файлы дисциплины** сгенерированы `typescript-ai-native init --namespace
org.vibevm.doc/web` и подправлены (A0.14 §5): `roots`/`scan_roots` указывают
на **два** корня (`design/src`, `site/src`), `exclude_substrings` растёт до
`["/fixtures/", "/generated/"]`, `cells_dir` остаётся закомментированным —
раскладка ячеек описывает домен, разбитый на изолированные единицы с одним
швом, а здесь библиотека компонентов и дерево маршрутов, чьи границы уже
фиксирует фреймворк.

### Решение 1 — резолв eslint-плагина: не подключён, и это решение, а не пропуск

`@org.vibevm/eslint-plugin-ai-native` достижим ровно одним способом —
`file:`-зависимостью на путь, посчитанный от раскладки **этого** чекаута
(`../../../org.vibevm.ai-native/typescript-ai-native-lang/v1.0.0/tools/…`).
Пакет публикуется исходником и собирается где угодно, поэтому такая
зависимость резолвится на одной машине и ни на какой другой. `PROP-057
##STACK-FLOOR` прямо называет этот вопрос («the resolution of the discipline's
eslint plugin **outside the repository layout** is decided when the package is
initialised») — я решил его в пользу переносимости.

Что потеряно: правило `diagnostic-cites-req`, третий канал REQ-цитирующих
диагностик. Что это стоит здесь: ничего измеримого — правило сторожит
диагностики, которые цитируют нарушенный `spec://`, а оболочка сайта их не
порождает: цитаты на странице приходят уже разрешёнными, внутри острова,
который рисует Rust. Запись об этом — в шапке `eslint.config.js`, чтобы
следующий не считал это забывчивостью.

Альтернатива, которую я отверг: `dependenciesMeta.injected` в pnpm. Она,
возможно, и починила бы резолв (`node_modules/.pnpm/node_modules/` несёт
`@typescript-eslint/utils` транзитивно), но не меняет главного — путь наружу
пакета остаётся в `package.json`.

**`pnpm floor` работает из пакета.** Семишаговый пол — это бинарник стека, а
не npm-пакет, поэтому `tools/floor.mjs` ищет его в трёх местах по убыванию
определённости: `$TYPESCRIPT_AI_NATIVE`, `PATH`, слот
`vibevm/vibedeps/…/target/release/` в чекауте выше. Ничего из `package.json`
наружу пакета не указывает; внутри репозитория маршрут A из A0.14 §1 работает
без второго `vibe install`. Отсутствие инструмента — падение с рецептом, не
пропуск.

`DEV-GUIDE.md` §2.6 «Node and pnpm for the site» — тем же коммитом (R-11):
версии, `corepack`, `pnpm install --frozen-lockfile`, `pnpm floor`, форма
вызова из корня хоста (та, что в панели), обе команды сборки и оговорка про
MSYS. §8 не трогал — он в периметре P2-O8; там остался абзац «…join this guide
together with the package, in phase 4», который теперь стоит заменить ссылкой
на §2.6 (мелкая правка для того, кто владеет §8).

## A4.10 — дизайн-система (`3e0efc08`, `90f21883`)

`palette.css` — сырые имена обеих карт, у каждого значения комментарий
«файл-источник: строка» (25 значений: 14 из светлой карты, 11 из тёмной, плюс
радиусы, три тени и `--speed`). Имена подобраны так, чтобы **ни одно сырое имя
не совпадало с семантическим**: `--text-light`, `--faint-light`/`--faint-dark`,
`--cream-dark`, `--accent-raw`/`--accent-hover-raw`/`--accent-soft-raw`,
`--line-dark`/`--line-strong-dark`. Это не стиль: `palette.css` и `tokens.css`
объявляются на одном `:root`, и каскад не умеет различать два `--text`.

`tokens.css` — тринадцать семантических имён D-21 (`--bg`, `--bg-raise`,
`--bg-sink`, `--text`, `--text-2`, `--text-3`, `--line`, `--line-strong`,
`--accent`, `--accent-hover`, `--accent-soft`, `--code-bg`, `--selection`) в
трёх блоках (`:root, [data-theme="light"]`; `@media (prefers-color-scheme:
dark) { :root:not([data-theme="light"]) }`; `[data-theme="dark"]`) плюс
отдельный `:root` с радиусами, тенями и `--speed`. **Ни одного литерала** — и
это проверяет сам аудит, а не ревью.

`fonts.css` — восемь `@font-face` (Spectral 400/600, Inter, JetBrains Mono;
латиница и кириллица раздельно, `unicode-range` дословно из источника,
`font-display: swap`), файлы в `design/fonts/` (**476K**, `du -sh`), рядом
`design/fonts/README.md` с явной пометкой, что байты едут под **OFL 1.1**, а
не под UPL пакета.

`theme-init.js` — три состояния темы, а не два: `dark`/`light` штампуются на
корне, `system` не штампует ничего (именно отсутствие атрибута отдаёт решение
media-запросу). Чтение `localStorage` обёрнуто в `try` — в браузере с
запретом хранилища бросает само обращение к свойству.

`base.css` — сброс, типографика страницы на токенах и одно правило
`prefers-reduced-motion`, которое гасит анимацию во всех компонентах сразу.

Компоненты (`90f21883`): `docs-header`, `docs-nav`, `doc-card`, `search-box`,
`tag`, `badge`, `prose`, `footnotes`, `table-scroll` (+ `breakout`), `fab`,
`footer`, `section-head`, `tab-pills` — тринадцать, каркас и стили, без
поведения ридера. Самый содержательный — `prose`: его CSS написан под
разметку, которую он **не рендерит** (остров приходит готовым из Rust), то
есть словарь пивота из §A2.13 отчёта P2-O6 — это словарь этого файла:
`.p-anchor`, `blockquote.rule`, `[data-unresolved]`, `pre.derived`,
`.example`, `aside.note`, `figure`, `.prompt`, `[data-when]`, таблицы.

### Аудит APCA — собственная реализация, сверенная с библиотечной

`design/audit/contrast.mjs` реализует опубликованную форму APCA 0.1.9
(0.0.98G-4g): константы `SA98G` вынесены одним объектом, `sRGBtoY`,
`apcaContrast` с мягким клэмпом чёрного и обеими полярностями, `alphaBlend`
для полупрозрачных фонов. Библиотеки `apca-w3` (AGPL) в дереве нет.

**Реализация сверена с числами A0.23, снятыми библиотечной версией, на шести
независимых парах — совпадение до второго знака:**

| пара | A0.23 (`apca-w3` 0.1.9) | моя реализация |
|---|---|---|
| светлый `--text-3` (`#8E8D85`) на `--bg` (`#FAF9F5`) | 57.15 | **57.15** |
| то же на `--ivory-50` (`#FCFBF8`) | 58.38 | **58.38** |
| светлый `--line` на `--bg` | 14.01 | **14.01** |
| тёмный `--line` на `--bg` | 0.00 | **0.00** |
| светлый акцент `#D97757` на `--bg` | 54.23 | **54.23** |
| тёмный `--accent-hover` `#E08A6D` на `--bg` | −50.52 | **−50.52** |

Матрица гейта: `--text` (≥75), `--text-2`, `--text-3` (≥60) на шести фонах
плюс `--accent`, `--accent-hover` как контур на `--bg` (≥45) = **40
гейтуемых пар** на две темы. `--line`, `--line-strong` и номер блока
(`--text-3` при `opacity .5`) печатаются справочно и в гейт не входят — **6
референсных пар**. Аудит заодно проверяет согласованность media-блока с
`[data-theme="dark"]` (0 расхождений) и отсутствие литералов в `tokens.css`.

### Таблица минта: было → стало, Lc

До минта гейтуемых пар ниже порога было **20** (у A0.23 — 23 при других
порогах: там `--line`/`--line-strong` гейтовались, а акценты нет). Минт по
правилу пакета — тот же оттенок и та же насыщенность, минимальный сдвиг по
светлоте до порога плюс 2 Lc, считая по **худшему** фону роли (для всех
текстовых ролей это `--selection`):

| тема | токен | сырое было | сырое стало | worst \|Lc\| было → стало | порог | шагов (0,1 % L) |
|---|---|---|---|---|---|---|
| светлая | `--text-2` | `--muted` `#63625B` | `--muted-docs` `#4F4E49` | 54.08 → 62.28 | 60 | 75 |
| светлая | `--text-3` | `--faint-light` `#8E8D85` | `--faint-light-docs` `#4F4E49` | 34.26 → 62.28 | 60 | 241 |
| тёмная | `--text-2` | `--dim` `#A8A197` | `--dim-docs` `#BEB9B1` | 49.12 → 62.17 | 60 | 94 |
| тёмная | `--text-3` | `--faint-dark` `#6F695E` | `--faint-dark-docs` `#BDB9B1` | 21.86 → 62.05 | 60 | 315 |
| тёмная | `--accent` | `--accent-raw` `#D97757` | `--accent-docs` `#DC8264` | 43.29 → 47.20 | 45 | 32 |

**Главное для дизайн-ревью A4.14: вторичный и третичный текст схлопнулись.**
В светлой теме оба минта дали `#4F4E49`, в тёмной `#BEB9B1` и `#BDB9B1` —
различить их глазом нельзя. Это не ошибка подбора, а следствие самих порогов:
`--text-2` и `--text-3` держатся на **одном** пороге 60 над **одним и тем же**
худшим фоном, а значит и минимальное значение, закрывающее порог, у них одно.
Палитра теряет третий уровень иерархии. Три выхода, любой — решение владельца,
не моё:

1. опустить порог третичного текста (D-21 отдельно его не называет — REVIEW
   самой A0.23);
2. сузить набор фонов для третичного (убрать `--selection`/`--accent-soft`);
3. осветлить `--selection` в светлой теме — но одна эта мера не спасает:
   светлый `--faint-light` не добирает до 60 даже на белом.

Сырые имена оставлены раздельными именно поэтому: когда ревью разведёт роли,
поменяется одно имя, а не структура файла.

Второй вопрос к ревью помельче: **тёмный `--accent` отминчен** (`#D97757` →
`#DC8264`, 32 шага, ΔL 3,2 %), потому что пакет гейтует акцент как
интерактивный контур (≥45), а исходный давал 43.29. Цена — «один акцент на
обе темы» (D-21-5) в тёмной карте больше не буквальная: светлая держит
`#D97757`, тёмная показывает `#DC8264`. Если брендовая буквальность важнее
контура — контур в тёмной теме должен рисоваться `--accent-hover` (−50.52,
проходит), и тогда минт откатывается.

## A4.2 — контракт острова и данных (`8ed08eca`)

### Решение 2 — TypeScript-типы из JTD: цель в `xtask codegen`

`xtask/src/codegen/typescript.rs`: таблица целей (схема → каталог → имя файла),
пока одна — `schemas/doc_manifest.jtd.json` →
`…/web/v0.1.0/site/src/generated/doc-manifest.ts`. Генерация идёт через тот же
`Vocabularies::resolve`, что и Rust-ветка, то есть из **того же** разрешённого
документа: иначе два языка описывали бы два разных контракта. `check-codegen`
диффит и файл TypeScript тоже — контракт с гейтом с одной стороны не контракт.

Восемь Rust-проходов постобработки к выходу **не** применяются: они кодируют
нашу wire-политику для Rust-читателей. Но **один** проход есть, и он не вкусовой:
`jtd-codegen` печатает закрытый словарь как `enum`, а пакет компилируется под
`erasableSyntaxOnly`, который `enum` запрещает (TS1294). Проход переписывает
каждый в стирающуюся пару того же имени:

```ts
export const Audience = { Agent: "agent", … } as const;
export type Audience = (typeof Audience)[keyof typeof Audience];
```

Член не вида `Имя = "строка"` останавливает прогон с названным именем — тихо
пропустить его значило бы вернуть в файл синтаксис, который потом отвергнет
`tsc` без всякого объяснения. Отминчено четыре словаря: `Audience`,
`DocumentationStatus`, `PageGenre`, `TranslationStatus`.

Первой строкой файла печатается `/** @scope … */`, и адрес берётся не с
потолка, а из `metadata.spec.implements` самой схемы — иначе orphan-ratchet
карты справедливо ругается на экспортируемые интерфейсы без цитаты (я это
увидел: 14 ORPHAN до маркера, 0 после).

`site/src/generated/` выведен из prettier, eslint и conform (его форма —
форма генератора, а переформатировать его значит ломать дифф), но **остаётся**
под `tsc` и под картой: типы проверяются и трассируются.

### Решение 3 — базовый путь: одна конфигурация, две сборки

`##STACK-ONE-BASE` требует, чтобы статическая сборка шла с `base: "/"`, а
встраиваемая — с `base: "/doc/"` и маршрутами документации в корне. Сделано
буквально и без второго кода:

| | `site/vite.config.ts` | `site/vite.config.embedded.ts` |
|---|---|---|
| `base` | `/` | `/doc/` |
| `routesDir` | по умолчанию `src/routes` | `src/routes/doc` |
| остров | фикстурный HTML через `define` | `<!--vibe-doc-island-->` через `define` |
| выход | `dist` / `server` | `dist-embedded` / `server-embedded` |

Разница между сборками — **значение**, а не ветка кода: `island-source.ts`
читает `__VIBE_ISLAND_HTML__`, который каждая конфигурация подставляет своим
`define`. Маршруты, компоненты, стили и входная точка — один исходник.
Лендинг исключается из встраиваемой сборки тем, **где лежат файлы**
(`src/routes/layout.tsx` вне `src/routes/doc`), без флага и без условия.

Адреса в JSX не пишутся руками: `site/src/lib/href.ts` — `href()` (владеет
ведущим слэшем и базой), `docPath()`/`docHref()` (строит адрес из координаты),
`projectionHref()` (`.md`/`.xml` рядом файлом), `parseDocAddress()` (читает
адрес обратно). Языковой сегмент отличается от группы **детерминированно, без
списка языков**: у группы всегда есть точка (reverse-DNS), у языкового тега её
нет никогда. Прямой литерал `/doc/…` вне `href.ts` ловит тест
`site/src/lib/href.test.ts` (сканирует `.ts`/`.tsx` обоих корней) — правило
eslint потребовало бы того самого плагина, который в пакет не берётся.

**Остров.** `site/src/components/island/index.tsx` — регион с `data-island` и
`dangerouslySetInnerHTML`; Qwik не превращает его в дерево компонентов и
никогда не перерисовывает. Обработчик ровно один, на регионе: `islandTarget()`
(чистая функция, пять тестов) превращает кликнутый узел в намерение —
`anchor` (номер блока), `rule` (адрес `spec://` из `data-uri`), `link`, `none`
— и результат пока только записывается на регион как `data-island-intent`.
Поведение ридера (A4.11) найдёт делегирование уже разведённым.

**Контракт данных.** `site/src/lib/manifest.ts` — единственное место, где
`unknown` становится `DocManifest`, и он **смотрит**, а не утверждает:
`as DocManifest` был бы весь контракт, сведённый к заявлению. Ошибка — это
значение с путём (`$.pages[0].summary`), а не исключение; закрытые словари
проверяются по самому словарю; отсутствующее необязательное поле остаётся
отсутствующим (`exactOptionalPropertyTypes` этого и требует).

### Решение 4 — как считаются страницы

`tools/build.mjs <static|embedded>`: клиентская сборка базовой конфигурацией,
затем сборка адаптером, затем гейт. Число из вывода генератора
(`- Generated: N page`) сверяется с числом, которое даёт **манифест**
(`site/src/fixtures/manifest.json`, `pages.length`), плюс — только для
статической сборки — число лендинговых маршрутов, найденных на диске
(`index.tsx` вне `src/routes/doc`). Два независимых счёта одного и того же:
если генератор тихо перестанет печатать страницы, они разойдутся. Коду выхода
не доверяем — `##STACK-PAGE-COUNT-GATE` ровно об этом. Отсутствие строки
`Generated` считается нулём, потому что при нулевом результате генератор её и
не печатает.

Гигиена выхода: `dist/q-manifest.json` (54 кБ метаданных оптимизатора, которые
никто не запрашивает) удаляется после сверки; `site/public/manifest.json`
переписан без чужого `$schema` на `schemastore.org` и без несуществующих
иконок.

## A4.9 — шаг панели

`tools/self-check.sh`, шаг 8b: `typescript-ai-native floor --path <пакет>` из
корня хоста плюс `node <пакет>/design/audit/contrast.mjs` — семь шагов
дисциплины и аудит контраста одним шагом, красный шаг красит панель. Бинарник
берётся из собранного слота хоста (маршрут A по A0.14 §1), отсутствие
бинарника или `node_modules` — падение с рецептом, не пропуск: «пол, который
считает только то, о чём ему сказали, не может ошибиться» — та же логика, что
у шага 0b. `grep -c '^run_step '` вырос с 49 до **50**.

## Вывод гейтов, дословно

```
$ pnpm install --frozen-lockfile
Scope: all 3 workspace projects
Lockfile is up to date, resolution step is skipped
Already up to date

Done in 390ms using pnpm v10.33.2
FROZEN_EXIT=0
```

```
$ pnpm run floor:discipline          (семь шагов)
✔ the failure names the path, not just the fact of failing (0.2044ms)
ℹ fail 0
=== test-gate (xfail-strict) ===
test-gate: 17 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).
EXIT=0
```

```
$ pnpm audit:contrast                 (восьмой шаг, хвост)
  --accent       on --bg           Lc= -47.20  min= 45  PASS
  --accent-hover on --bg           Lc= -50.52  min= 45  GOOD
  --line         on --bg           Lc=   0.00  decorative, not gated
  --line-strong  on --bg           Lc=   0.00  decorative, not gated
  block number   on --bg           Lc= -22.55  decorative, not gated

=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
EXIT=0
```

```
$ node tools/build.mjs static
- Generated: 3 pages
build (static): generated 3 page(s), expected 3
build (static): removed dist/q-manifest.json from the output
build (static): ok
STATIC_EXIT=0

$ node tools/build.mjs embedded
- Generated: 1 page
build (embedded): generated 1 page(s), expected 1
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): ok
EMBEDDED_EXIT=0
```

Встраиваемый выход проверен по содержанию, а не по коду выхода:

```
$ find site/dist-embedded -name index.html
site/dist-embedded/com.example.docs/fixture-manual/0.1.0/guide/every-block/index.html
$ grep -c "vibe-doc-island" site/dist-embedded/.../index.html
1
$ grep -o 'q:base="[^"]*"' site/dist-embedded/.../index.html
q:base="/doc/build/"
```

и статический — тоже:

```
$ grep -c "p-anchor" site/dist/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/index.html
22
$ ls site/dist/q-manifest.json
ls: cannot access '…/site/dist/q-manifest.json': No such file or directory
$ grep -c "schemastore" site/dist/manifest.json
0
```

```
$ grep -rn "#[0-9a-fA-F]\{3,8\}" design/src site/src
GREP_EXIT=1 (1 = no match = clean)
```

```
$ cargo xtask check-codegen
  - schemas/doc_manifest.jtd.json → …/site/src/generated/doc-manifest.ts
xtask check-codegen: clean.
CHECKCODEGEN_EXIT=0
```

```
$ typescript-ai-native specmap --path <пакет>
typescript-ai-native-specmap: wrote …/specmap.json
  (0 spec units, 29 tagged code items, 29 edges, 0 suspects, 29 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).
SPECMAP_EXIT=0
```

(29 warnings — это «цитата ведёт наружу пакета»: все `@scope` указывают на
PROP-057 в хостовом дереве, а `[[external_specs]]` я сознательно не завёл, см.
шапку `specmap.toml`. Ratchet — 0 orphan, 0 suspects.)

```
$ cargo xtask specmap --check
Error: `…/specmap.json` is out of date relative to the tree.
  drift: unbumped-hash: `spec://…/PROP-000#token-secrecy` …
  … (90 строк, все — unbumped-hash)
  drift: edges added: 62
SPECMAP_CHECK_EXIT=1
```

**Красный — не мой.** Все 90 строк дрейфа — `unbumped-hash` на спек-документах
(PROP-000, PROP-018, PROP-022, PROP-025 и т. д.), то есть правки разметки,
которые прямо сейчас делает соседний воркер; слова `xtask` в логе нет ни разу,
suspects и orphan'ов нет. Хостовый `specmap.json` я **не перестраивал**: это
общий файл, и перестройка вшила бы в него чужое незакоммиченное состояние.

```
$ target/debug/vibe.exe check --path vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
vibe check: 1 finding
  [E]  [boot_directory] vibevm/vibespecs/boot — vibevm/vibespecs/boot/ is missing …
EXIT=1
```

Дефект гейта, не пакета — см. А-4. Хостовая проверка при этом зелёная:

```
$ target/debug/vibe.exe check --path . --quiet
vibe check: 0 errors, 2 warnings, 0 info
EXIT=0
```

(обе warning — `wal_wellformed` на `vibevm/vibespecs/WAL.xml`, они были в
дереве до меня).

Шаг панели, прогнанный из корня хоста ровно так, как его зовёт `self-check.sh`
(всю панель не гонял — её занимает P1-O6):

```
$ bash -n tools/self-check.sh
SYNTAX_EXIT=0
$ grep -c '^run_step ' tools/self-check.sh
50
$ vibevm/vibedeps/…/typescript-ai-native.exe floor --path vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
floor: all green (7 step(s) run, 0 disabled by policy).
FLOOR_EXIT=0
$ node vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/audit/contrast.mjs
=== pairs: gated=40, reference=6; below threshold=0 ===
AUDIT_EXIT=0
```

Диск и размеры:

```
$ du -sh vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/fonts
476K
$ df -h /c          (до снятия приватного CARGO_TARGET_DIR)
C:              3.7T  3.5T  195G  95% /c
$ du -sh <scratch>/target-p4o1
2.8G
$ df -h /c          (после)
C:              3.7T  3.5T  195G  95% /c
```

Приватный `CARGO_TARGET_DIR` удалён (`Test-Path` → `False`); `dist/`,
`dist-embedded/`, `server/`, `server-embedded/`, `node_modules/.cache/`,
`target/` и `tmp/` пакета удалены; `node_modules/` оставлен — он нужен шагу
панели и лежит в `.gitignore`.

## Аномалии, найденные по пути (правок в чужом не делал)

**А-1. Qwik 2.0.0-beta.43: `<RouterOutlet />` внутри `layout.tsx` — вечный
синхронный цикл.** Симптом: каждая страница падает по
`SSG render timed out after 30000ms`, при этом ни одной ошибки, ни байта в
`dist` (каталоги маршрутов создаются пустыми), воркер не отвечает и не
падает. Диагностика заняла время, поэтому записываю метод: (1) `--debug` у
`run-ssg.js` показал, что дерево маршрутов обходится правильно
(`3 routes queued`), значит дело в рендере; (2) прямой запуск воркера
worker-thread'ом с настоящим `workerData` воспроизвёл зависание и показал, что
даже вставленный в воркер `setInterval` не срабатывает — то есть цикл событий
**заблокирован**, а не ждёт промис; (3) `Get-Process node | Sort CPU` показал
+13,6 с CPU за 5 с настенного времени — цикл горячий. Причина: в Qwik Router
дочерний маршрут приходит в layout через `<Slot />`, а `<RouterOutlet />`
означает «сюда подставить сматченное дерево» и принадлежит только `root.tsx`;
outlet внутри layout просит роутер отрисовать дерево внутри себя самого.
Бета делает это молча и бесконечно. Апстримного тикета не заводил (сеть
разрешена только для `pnpm install`) — кандидат в `E-BUG` или в апстрим.

**А-2. Порядок сборок — не вкусовщина, и совет самого адаптера ведёт в стену.**
Каждая сборка печатает:
«Qwik Router SSG was skipped: the `ssr` environment was built directly instead
of through the Vite app builder… Build with the Qwik CLI or
`createBuilder(config).buildApp()`». Я последовал совету и получил на каждой
странице `Code(Q14) … Cannot resolve symbol s_… in {…}`: при `build.ssr: true`
в конфиге адаптера клиентское окружение не пересобирается, и SSG сверяется с
**устаревшим** `q-manifest.json`. Рабочая последовательность — та, что у
`qwik build`: клиентская сборка базовой конфигурацией, затем сборка
адаптером. Предупреждение при этом печатается на каждой **зелёной** сборке и
является шумом. Записано в `site/README.md`, чтобы следующий не «починил» это
обратно.

**А-3. `vibe check --path <пакет>` требует boot-каталог от всякого пакета,
кроме `doc`.** Проверка `boot_directory`
(`crates/vibe-check/src/checks/boot_directory.rs:28-36,58`) освобождает ровно
один вид — `doc` — а прочим печатает «run `vibe init` if it disappeared».
Отказывает не только моему `app`:

```
$ …/vibe.exe check --path vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native/v1.0.0   → EXIT=1
$ …/vibe.exe check --path vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0            → EXIT=1
$ …/vibe.exe check --path vibevm/vibepacks/org.vibevm.world/campaign-plans/v1.0.0             → EXIT=0  (несёт boot-сниппет)
```

То есть это не «гейт отказывает виду `app`», а «гейт считает всякий пакет
проектом». Для `app` основание освободить ровно то же, что у `doc`, и оно
записано в норме: `##KIND-APP-VS-TOOL` — «an `app` runs nowhere in a consumer
project and is built and deployed on its own». Правку не делал (не мой
периметр и прямой запрет пакета); кандидат в `BACKLOG.md` как P2.

**А-4. Реестр npm на этой машине — зеркало.** `pnpm install` ходил на
`https://npm-mirror.gitverse.ru/repository/registry_npmjs_org/…`, а не прямо
на `registry.npmjs.org` (это машинная настройка, я её не менял и не читал
секретов). На воспроизводимость это не влияет — `pnpm-lock.yaml`
зафиксирован и `--frozen-lockfile` зелёный, — но формулировку «сеть только к
`registry.npmjs.org`» в будущих пакетах стоит читать как «только к настроенному
npm-реестру». Отдельно: зеркало отвечало 503 на части запросов и сборка
lock-файла заняла 1 м 30 с с ретраями.

**А-5. `q-manifest.json` попадает в публичный выход всегда.** Подтверждаю
находку A0.10: адаптер кладёт 54 кБ метаданных оптимизатора в `dist`, ни одна
страница их не просит. Удаляю в `tools/build.mjs`; nginx-правило из
`##STACK-BUILD-HYGIENE` (404 на него) — дело атома развёртывания.

## Что не сделано и почему

- **Поведение ридера (A4.11), оглавление и таблицы (A4.12), лендинг (A4.16),
  SEO и корневые машинные файлы (A4.4/A4.5)** — не начинал, запрещено
  пакетом. `site/src/routes/index.tsx` и `ru/index.tsx` — честно помеченные
  заглушки: они существуют, чтобы у статической сборки были лендинговые
  маршруты и чтобы счёту страниц было в чём ошибаться.
- **Ни одна страница не открыта в браузере.** Панель Browser к локальному
  серверу не пускает, расширения нет (то же ограничение, что у A0.10 §9).
  Проверено то, что проверяемо без браузера: разметка выхода, число страниц,
  наличие острова и плейсхолдера, базовый путь в `q:base`. Клик по `.p-anchor`,
  переключатель тем и `prefers-reduced-motion` живым браузером **не
  проверены** — это работа A4.11 с Playwright.
- **Инлайн-скрипт темы требует хэша в CSP.** `theme-init.js` вставляется в
  `<head>` инлайном (пакет требует именно этого, и иначе FOUC), а значит
  CSP сайта обязана нести его `sha256`, либо скрипт станет блокирующим
  `<script src>` в `<head>`. Решение — за атомом развёртывания; записал в
  комментарии `root.tsx`.
- **Хостовый `cargo xtask specmap` не перестраивал** — см. вывод гейтов.
- **`DEV-GUIDE.md` §8** не трогал (периметр P2-O8); абзац про «Node… join this
  guide… in phase 4» теперь стоит заменить ссылкой на §2.6.
- **`typescript-plugin-css-modules`, `eslint-plugin-qwik`, Playwright,
  vitest** — не подключал: пакет их не называет, а лишний пин в бете дорог.
  Компонентных тестов нет по той же причине, что в A0.14 §4 (К-3): `node
  --test` не исполняет `.tsx`, а vitest потребовал бы отключить два шага пола.
- **Схлопывание `--text-2`/`--text-3`** не «починено» подгонкой: правило минта
  применено буквально, а вопрос вынесен в ревью A4.14 (см. таблицу выше).

## `git status --short` на момент сдачи (мои пути)

```
$ git status --porcelain -- vibevm/vibepacks/org.vibevm.doc xtask tools/self-check.sh
(пусто)
```

Сам этот отчёт оставлен **незакоммиченным** — по заведённому в кампании
порядку его вносит центральная сессия вместе с записью в леджер (как
`WORKER-REPORT-P1-O6.md`, который тоже лежит нетронутым).
