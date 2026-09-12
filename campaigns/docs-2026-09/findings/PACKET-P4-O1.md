# PACKET-P4-O1 — пакет сайта, токены и темы, контракт острова (A4.1, A4.10, A4.2, A4.9)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммиты P2-O6 (`fcc32b96` — HTML-остров,
`6aa578df` — номера блоков) и P2-O7 (`55dfd646` — манифест страниц и схема
`schemas/doc_manifest.jtd.json`) в ветке — проверь `git log --oneline -40`
и прочитай `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O6.md` §A2.13
(форма острова, голдены) и `WORKER-REPORT-P2-O7.md` §A2.14 (что несёт
манифест, два формата, базовый путь и язык). Если в ветке уже есть
`feat(cli): expose the documentation pipeline as vibe doc` (пакет P2-O8),
острова и манифест для проб бери командами `vibe doc build` и `vibe doc
manifest --json`; если нет — из голденов тестов `crates/vibe-doc`.

## Читать сначала

1. Этот пакет целиком; два отчёта выше.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A4.1**, **A4.2**, **A4.9**,
   **A4.10**; §2.3 (особенности машины: Git Bash переписывает POSIX-пути;
   `vibe` из PATH — старый релиз, нужен `target/debug/vibe.exe`); §2.8
   не нужен — крейтов в этом пакете нет; правила **R-09** (никаких внешних
   ресурсов), **R-10** (в пакет — только исходник), **R-11** (`DEV-GUIDE.md`
   тем же коммитом), **R-27** (цвета только через семантические токены);
   развилки **F-14** (TypeScript-типы из JTD через `xtask codegen`), **F-75**
   (никакого Tailwind).
3. Находки: `findings/A0.10-qwik-probe.md` (пины, `ssg`, базовый путь,
   §6 три острых угла, §7 соображения для встроенного адаптера),
   `findings/A0.14-ts-floor.md` (команда floor, семь шагов, флаги `tsconfig`,
   §5 файлы дисциплины — генерировать `init`-ом и подправить), `findings/A0.23-tokens-apca.md`
   (сшивка двух карт, аудит APCA; **файлы `A0.23-tokens/*` в дерево не
   попали — воссоздай `palette.css`, `tokens.css` и аудит по описанию
   находки и по двум файлам-источникам**), `findings/A0.22-reader-inventory.md`
   §1–§2 (что ридер ждёт от острова — только для контракта региона).
4. Решения: `vibevm/vibespecs/design/documentation-vision.xml` D-06
   (адреса `/doc/…`, язык — сегмент), D-12 (Qwik 2.0 beta, один код — два
   адаптера), D-21 (токены, типографика, компоненты), D-22 п. 1 (нумерация
   — в острове, клиент не считает), D-28 (одна pnpm-workspace: `design/` и
   `site/`).
5. Норма: PROP-057 `stack` (STACK-QWIK, STACK-ONE-BASE, STACK-PAGE-COUNT-GATE,
   STACK-BUILD-HYGIENE, STACK-WORKSPACE, STACK-FLOOR, STACK-DESIGN-FLOOR,
   STACK-NODE-SERVER-ONLY), `site` (SITE-MOUNT, SITE-TRAILING-SLASH,
   SITE-ONE-SITE), `reader` (READER-NUMBERED-BLOCKS, READER-SETTINGS),
   `kinds` (KIND-APP-LEAD, KIND-APP-VS-TOOL); PROP-024 §2.2 (в пакете нет
   артефактов сборки).
6. Образцы: `research/ts-demo/` (`eslint.config.js`, `.prettierrc.json`,
   `conform.toml`, `specmap.toml`, скрипт `floor`); `DEV-GUIDE.md` §2 и §8;
   схема `schemas/doc_manifest.jtd.json`; `xtask/src/main.rs` и
   `tools/jtd-codegen/README.md` (как зовётся `jtd-codegen`);
   `tools/self-check.sh` (форма шагов панели для стеков).
7. Источники дизайна, **только чтение** (R-28): светлая карта —
   `C:\Users\olegc\git\talks\2026.08.08-agents-talk\design-system\public\assets\css\anthropic.css`;
   тёмная карта и шрифты — `C:\Users\olegc\git\v\vibevm-org\src\styles\global.css`,
   `C:\Users\olegc\git\v\vibevm-org\public\fonts\*.woff2` (восемь подсетов
   Spectral, Inter, JetBrains Mono — копируются в пакет как ассет-исходник,
   лицензия OFL).
8. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `git push` не делать; чужие
   незакоммиченные файлы не стейджить (в дереве работают другие воркеры:
   P2-O8 в `crates/vibe-doc*`, `crates/vibe-cli`, `crates/vibe-mcp`,
   `DEV-GUIDE.md` §8; разметка спек в `vibevm/vibespecs/common` и
   `modules`); `cargo` — только для `xtask codegen`/`check-codegen`, в
   приватном `CARGO_TARGET_DIR=<scratch>\target-p4o1`, каталог удалить в
   конце; секреты и `infra/` не читать. Сеть разрешена только для
   `pnpm install` из `registry.npmjs.org` внутри пакета; `node_modules/`,
   `dist/`, `server/`, `tmp/` — в `.gitignore` пакета и никогда в коммите;
   `pnpm-lock.yaml` коммитится. На машине стоят Node 24.18.0, pnpm 10.33.2,
   corepack.

## Цель — четыре атома, четыре коммита

**A4.1 Пакет `org.vibevm.doc/web`** —
`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/` (новая группа): `vibe.toml`
(`kind = "app"`, `title`, `abstract` по четырём вопросам карточки D-20,
`description`, `[requires]` на `stack:org.vibevm.ai-native/typescript-ai-native`,
лицензия UPL-1.0, авторы как у руководства), `README.md` (десять строк: что
это, как собрать, два адаптера), `LICENSE.md`; корневой `package.json`
(`"type": "module"`, `packageManager: "pnpm@10.33.2"`, `engines.node
"24.18.0"`, точные пины `@qwik.dev/core` и `@qwik.dev/router`
`2.0.0-beta.43`, `vite 8.2.1`, `typescript`, `prettier`, `eslint`,
`typescript-eslint`; скрипты `floor`, `build:static`, `build:embedded`,
`audit:contrast`), `pnpm-workspace.yaml` (`design`, `site`; `allowBuilds`
с `sharp: false` и `@parcel/watcher: false`), `tsconfig.json` по A0.14 §4
(полный пол дисциплины плюс блоки Qwik, `exactOptionalPropertyTypes`
сохранён), файлы дисциплины по A0.14 §5 (`init`-ом и подправить:
`conform.toml`, `specmap.toml`, baseline и registry, `.prettierrc.json`,
`.prettierignore`, `eslint.config.js`; резолв плагина — как решишь, решение
в отчёт), `specmap.json`; `DEV-GUIDE.md` §2 получает подраздел «Node и
pnpm для сайта» (версии, `corepack`, команда floor из корня хоста через
`target/debug/vibe.exe bin exec typescript-ai-native -- floor` или из
пакета — как получилось в A0.14 §1, `MSYS_NO_PATHCONV=1` при передаче
путей) — тем же коммитом. `target/debug/vibe.exe check --path <пакет>`
зелёный; если гейт отказывает виду `app` — это дефект пакета, в отчёт, не
чинить. Коммит: `feat(web): open the site package under the TypeScript discipline`.

**A4.10 Дизайн-система** — `design/`: `palette.css` (сырые имена обеих
карт с комментарием «файл-источник: строка» у каждого значения),
`tokens.css` (тринадцать семантических имён D-21 плюс `--code-bg`,
`--selection`, радиусы, тени, `--speed`; три блока — `:root,
[data-theme="light"]`, `@media (prefers-color-scheme: dark) {
:root:not([data-theme="light"]) }`, `[data-theme="dark"]` — только через
`var(--сырое-имя)`, ни одного литерала), `fonts.css` (`@font-face` для
Spectral 400/600, Inter, JetBrains Mono, латиница и кириллица раздельно с
`unicode-range`, `font-display: swap`, файлы в `design/fonts/`),
`theme-init.js` (инлайн в `<head>`: читает `localStorage`, ставит
`data-theme` до загрузки стилей), компоненты Qwik на токенах:
`docs-header`, `docs-nav`, `doc-card`, `search-box`, `tag`, `badge`,
`prose`, `footnotes`, `table-scroll` с `breakout`, `fab`, `footer`,
`section-head`, `tab-pills` (переключатель платформ для `data-when`) —
каркас и стили, без поведения ридера (оно в A4.11). Аудит APCA
`design/audit/contrast.mjs`: **собственная реализация опубликованного
алгоритма APCA-W3 (0.1.9), без библиотеки `apca-w3` в дереве** (AGPL,
STACK-DESIGN-FLOOR); пороги по роли: `--text` ≥ 75, `--text-2` и `--text-3`
≥ 60 на каждом фоне (`--bg`, `--bg-raise`, `--bg-sink`, `--code-bg`,
`--selection`, `--accent-soft`), интерактивные контуры (`--accent`,
`--accent-hover` как контур на `--bg`) ≥ 45; `--line`, `--line-strong` и
номера блоков — декоративные, вне гейта, печатаются справочно. Где пара
ниже порога (A0.23 нашёл 23, среди них весь `--text-3` светлой темы и
`--text-2`/`--text-3` тёмной), **подбери новое сырое значение с
минимальным сдвигом по светлоте на том же оттенке** до порога плюс запас
2 Lc, заведи его в `palette.css` новым сырым именем с комментарием
«минт: из `<исходное имя>`, Lc было/стало» и переключи семантический токен
на него; таблица «было → стало, Lc» — в отчёт; дизайн-ревью A4.14
пересмотрит. Аудит — восьмой шаг floor пакета, гейтующий; проверка
согласованности media-блока с `[data-theme="dark"]` — в него же. Коммит:
`feat(web): lay down the semantic tokens and both themes`; компоненты —
вторым коммитом `feat(web): build the shell components on the tokens`.

**A4.2 Контракт острова и данных** — `xtask codegen` получает цель
TypeScript для `schemas/doc_manifest.jtd.json` (F-14; `jtd-codegen
--typescript-out`), выход — `site/src/generated/doc-manifest.ts`,
`cargo xtask check-codegen` ловит дрейф; маршруты `site/src/routes/doc/…`
по D-06 с языковым сегментом (`/doc/<группа>/<имя>/<версия>/<документ>/`
и `/doc/<lang>/…`), завершающий слэш обязателен (SITE-TRAILING-SLASH,
угол 1 A0.10); базовый путь — из одной конфигурации (`STACK-ONE-BASE`:
статический адаптер `base: "/"`, встроенный — `base: "/doc/"` без
маршрутов лендинга; флаг сборки, а не второй код); остров — компонент
`Island`, вставляющий готовый HTML из конвейера Rust без реактивности Qwik
(`dangerouslySetInnerHTML` над регионом с `data-island`), обработчики
событий — делегированием на регионе (пока только каркас: один слушатель,
без фич A4.11); `<Link>` и абсолютные пути в JSX префиксуются базовым
путём через одну функцию `href()` — прямой литерал `/doc/…` в JSX
ловит правило eslint или тест. Два адаптера собираются: `pnpm build:static`
пререндерит фикстурную страницу с островом (число страниц из вывода
генератора сверяется с числом маршрутов — STACK-PAGE-COUNT-GATE, никакого
доверия коду выхода); `pnpm build:embedded` даёт шаблон маршрута с местом
под остров. Гигиена сборки: `q-manifest.json` удаляется из выхода,
`public/manifest.json` без чужого `$schema`. Коммит:
`feat(web): fix the island contract against the doc manifest`.

**A4.9 Floor в панели** — `tools/self-check.sh` получает шаг «web-пакет:
floor» по образцу шагов стеков (семь шагов дисциплины плюс аудит
контраста), из корня хоста; шаг красный — панель красная. Тем же коммитом,
что A4.2, или отдельным `test(web): add the site floor to the panel`.

## Гейты (вывод дословно в отчёт)

`pnpm install --frozen-lockfile` в пакете; `pnpm floor` (семь шагов) и
`pnpm audit:contrast` зелёные; `pnpm build:static` и `pnpm build:embedded`
с числом страниц; `grep -rn "#[0-9a-fA-F]\{3,8\}" design/src site/src` —
пусто (литералы только в `palette.css`); `cargo xtask check-codegen`;
`cargo xtask specmap` (0 suspects); `target/debug/vibe.exe check --path
<пакет>`; `bash tools/self-check.sh` — **только** шаг web-пакета, если
панель целиком занята другим воркером (P1-O6); `du -sh` каталога шрифтов
(в отчёт — красная линия больших блобов); `df -h` до и после, приватный
`CARGO_TARGET_DIR` удалён.

## Что не делать

Прозу страниц не менять; PROP-файлы и вижен не менять; крейты `vibe-doc*`
не трогать; поведение ридера (A4.11), оглавление и таблицы (A4.12),
лендинг (A4.16), SEO (A4.4) — не начинать; Tailwind — нет; внешние CDN,
шрифты по сети, аналитика — нет; `vibevm-org` и `design-system` — только
чтение.

## Результат

Четыре коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P4-O1.md`
по форме отчёта P2-O1: решения (резолв eslint-плагина, форма конфигурации
базового пути, как считаются страницы), таблица токенов «было → стало» с
Lc, вывод гейтов, аномалии беты Qwik, что не сделано и почему. Никаких
путей вне репозитория, IP и секретов в отчёте (R-25) — источники дизайна
называй по имени файла, не по абсолютному пути.
