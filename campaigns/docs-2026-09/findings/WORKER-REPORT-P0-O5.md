# WORKER-REPORT-P0-O5 — Qwik 2.0 beta, проба вне репозитория (спайк A0.10)

Дата: 2026-09-11. Дерево: `b1291b06`. Находка: `findings/A0.10-qwik-probe.md`.
Прототип: `<scratch>/vibe-docs-phase0\P0-O5\qwik-probe\`.
Запуск: `cd qwik-probe/site && pnpm build`, затем
`cd .. && node serve-static.mjs site/dist 4181` → `http://127.0.0.1:4181/doc/guide/`.

## Решения

1. **Версия каркаса.** `pnpm create qwik@latest` даёт Qwik **1.20.0**, двойка
   живёт под тегом `beta`. Взят точный пин `create-qwik@2.0.0-beta.43`,
   шаблон `empty`. Все версии в находке выписаны из `pnpm-lock.yaml` с номерами
   строк.
2. **Рабочее пространство.** Два пакета, как задано: `design/`
   (`@vibe-docs/design` — токены `tokens.css` + три компонента) и `site/`
   (`@vibe-docs/site` — маршруты, адаптер). `design` экспортирует **исходники**
   (`"exports": {".": "./src/index.ts"}`), отдельная сборка библиотеки не
   понадобилась — оптимизатор Qwik обрабатывает `.tsx` из symlink'а pnpm.
3. **Статический адаптер.** Конфиг `site/adapters/ssg/vite.config.ts` написан
   руками по шаблону из
   `@qwik.dev/core/dist/starters/adapters/ssg/adapters/ssg/vite.config.ts`,
   потому что `qwik add ssg --yes` всё равно требует TTY (см. «Отклонения»).
4. **Базовый путь.** Проверены оба варианта: `base: "/"` с каталогом
   `src/routes/doc/` (принят) и `base: "/doc/"` (разобран, показана механика
   двойного префикса). Ответ на вопрос пакета — base один на всё приложение.
5. **Маршруты прототипа.** `/` и `/ru/` — лендинг; `/doc/`, `/doc/guide/`,
   `/doc/ru/guide/` — документация; плюс два измерительных маршрута:
   `/doc/plain/` (тот же текст без островов страницы) и `/bare/` (со своим
   `layout!.tsx`, вообще без островов) — чтобы вес сериализации измерялся
   разностью, а не на глаз.
6. **Свой шрифт.** `@font-face` с `url("/fonts/vibe-serif.woff2")` и файл-
   заглушка в `public/fonts/` — чтобы проверить, что шрифт едет со своего
   origin и что происходит с его путём при непустом base.

## Отклонения

1. **`qwik add ssg --yes` не сработал** — CLI печатает план и ждёт
   подтверждения в TTY, в неинтерактивной среде возвращает 0 и ничего не
   создаёт. Конфиг адаптера и скрипт `build.server` дописаны руками по
   официальному шаблону, байт в байт. Записано как острый угол.
2. **`pnpm install` вернул код 1** на чистой установке
   (`ERR_PNPM_IGNORED_BUILDS: @parcel/watcher@2.6.0, sharp@0.34.5`). Добавлен
   явный `allowBuilds` со значениями `false` для обоих (нужны только
   оптимизации картинок и вотчеру; сборка проходит полностью). Записано как
   острый угол.
3. **Живой браузер запустить не удалось.** Панель Browser отказывает в
   навигации к `127.0.0.1`/`localhost` (запросы до сервера не доходят — в его
   логе их нет), расширение Chrome не подключено
   (`list_connected_browsers` → `[]`). Поднять `.claude/launch.json` в рабочем
   дереве запрещено правилом «без правок файлов репозитория». Вместо записи
   сетевой панели снят **полный набор адресов**, объявленных разметкой и графом
   модулей, и проигран `fetch`'ем по статическому серверу; отдельно доказано,
   что в чанках нет ни одного абсолютного `import`/`fetch`. Ограничение явно
   названо в находке (§4 и §9.1). Перенос `#p12` переключателем языка поэтому
   подтверждён по выпущенному чанку и разметке, а не кликом.
4. **Переменная окружения с путём оказалась ловушкой.** `VD_BASE=/doc/` в Git
   Bash доехало до SSG как `/Program%20Files/Git/doc/` (MSYS2 переписывает
   POSIX-подобные значения). Первая попытка варианта B из-за этого дала ноль
   страниц; после перехода на голый сегмент (`VD_BASE=doc`) вариант B собрался
   и был разобран честно. Обе половины записаны: и настоящая механика base, и
   сама ловушка.
5. **Сеть шла через зеркало** `npm-mirror.gitverse.ru` (настройка машины), с
   503 и ретраями; на состав lock-файла это не повлияло — версии и integrity
   в находке указаны как есть.

## Вывод самопроверки (дословно)

### 1. `pnpm build` в scratch-проекте

```
$ cd .../P0-O5/qwik-probe/site && pnpm build
EXIT=0
--- last 5 lines ---
==============================================
✓ Built server (ssr) modules
✓ Type checked
✓ Lint checked

```

(последняя строка пустая; полный лог — `/tmp/p0o5-final.log`. Строка SSG в
этом же прогоне: `- Generated: 7 pages`.)

### 2. `grep -rho 'https\?://[^"'"'"' )]*' dist | sort -u`

```
http://localhost/`,o=new
http://www.sitemaps.org/schemas/sitemap/0.9
http://www.w3.org/1999/xhtml`,Er=`http://www.w3.org/2000/svg`,Dr=`http://www.w3.org/1998/Math/MathML`,Or=`qRender`,kr=`q:id`,A=`q:props`,Ar=`q:seq`,jr=`q:seqIdx`,Mr=`q:p`,Nr=`q:ps`,Pr=`:`,Fr=`:on`,Ir=`:onIdx`,Lr=`:onFlags`,Rr=`:cursorBoundary`,zr=`:`,Br=`dangerouslySetInnerHTML`,Vr=100,j=e=>{try{return!!e&&typeof
http://www.w3.org/1999/xlink`;case`xml:base`:case`xml:lang`:case`xml:space`:return`http://www.w3.org/XML/1998/namespace`;default:return
http://www.w3.org/2000/svg
https://json.schemastore.org/web-manifest-combined.json
https://qwikdev-build-v2.qwik-8nx.pages.dev/docs/errors/#q${e}`
https://vibevm.example/
https://vibevm.example/</loc></url>
https://vibevm.example/bare/
https://vibevm.example/bare/</loc></url>
https://vibevm.example/doc/
https://vibevm.example/doc/</loc></url>
https://vibevm.example/doc/guide/
https://vibevm.example/doc/guide/</loc></url>
https://vibevm.example/doc/plain/
https://vibevm.example/doc/plain/</loc></url>
https://vibevm.example/doc/ru/guide/
https://vibevm.example/doc/ru/guide/</loc></url>
https://vibevm.example/ru/
https://vibevm.example/ru/</loc></url>
EXIT=0
```

Ожидание пакета выполнено с двумя оговорками, разобранными в находке §4:
`sitemaps.org`/`w3.org` — пространства имён; `https://vibevm.example/*` — наш
собственный `origin` в canonical и sitemap; `json.schemastore.org` — поле
`$schema` в `public/manifest.json` из стартера; `qwikdev-build-v2.qwik-8nx.pages.dev` —
адрес внутри шаблона **сообщения об ошибке** в ядре. Ни один из них не
является запросом: абсолютных `import`/`fetch` в `dist/build/*.js` — ноль.

### 3. `git -C C:/Users/olegc/git/v/vibevm-docs status --short`

```
 M .claude/agents/opus5.md
EXIT=0
```

Пусто, кроме единственного ожидаемого пакетом исключения. Ни одного байта в
репозиторий не записано; git-команд, кроме `status`, не выполнялось.

## Что не сделано и почему

1. **Клик в настоящем браузере** (перенос `#p12`, работа панели настроек
   чтения, счётчик сетевых запросов). Причина — отклонение 3: браузера,
   способного дойти до локального сервера, в песочнице нет. Оставлено открытым
   вопросом №1 в находке с точным сценарием проверки.
2. **MDX** — не проверялся: пакет его не требовал. Поскольку документацию,
   скорее всего, понесёт именно MDX (`qwikRouter()` принимает `mdx`/
   `mdxPlugins`, зависимости `@mdx-js/mdx` и `rehype-*` уже стоят), это
   записано открытым вопросом №4.
3. **Второй, «встроенный» адаптер** — по условию пакета не делался; вместо
   него собран список требований к статическому серверу (находка §7) и
   отдельно отмечено, что монтирование документации под `/doc/` встроенным
   сервером требует отдельной сборки с `base: "/doc/"` и маршрутами в корне
   `src/routes/`.
4. **Причина скачка `pnpm --version`** (10.33.2 снаружи рабочего пространства,
   11.24.0 внутри) не установлена: ни `packageManager`, ни `.npmrc`, ни
   `engines` не найдены. Открытый вопрос №3.
5. **Дефектов пакета не обнаружено.** Правил, которых не хватало бы для
   задачи, не встретилось; секретов не встречено.
