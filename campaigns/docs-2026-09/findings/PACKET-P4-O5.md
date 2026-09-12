# PACKET-P4-O5 — лендинг на Qwik и тест паритета (A4.16, A4.17)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммиты P4-O1 (`4ccc9438`, `3e0efc08`,
`90f21883`, `8ed08eca`) в ветке — проверь `git log --oneline -60` и
прочитай `campaigns/docs-2026-09/findings/WORKER-REPORT-P4-O1.md` целиком
(решения 1–4, аномалии А-1 и А-2 обязательны: `<Slot />` в layout,
порядок сборок в `tools/build.mjs`, счёт страниц).

## Читать сначала

1. Этот пакет целиком; отчёт P4-O1.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A4.16**, **A4.17**; правила
   **R-09**, **R-10**, **R-27**, **R-28** (`vibevm-org` только читается);
   развилки **F-38** (имена краулеров — по документации провайдера в день
   сборки, источник и дата в отчёт), **F-75** (без Tailwind).
3. Находки: `findings/A0.26-landing-inventory.md` **целиком** — таблица
   переноса (`i18n.ts`, компоненты и анимации, мета-теги `BaseLayout.astro`,
   корневые файлы `public/`, адреса `sitemap.xml` и `feed.xml`), эталонная
   сборка, острые места переноса; `findings/A0.23-tokens-apca.md` §1
   (какие значения лендинга стали токенами).
4. Решения: `vibevm/vibespecs/design/documentation-vision.xml` **D-28**
   (лендинг переезжает один к одному; список машинных файлов корня), D-24
   (тег Umami — тот же, значение website-id — из конфигурации сайта; в
   этом пакете — из одной переменной конфигурации с пустым значением по
   умолчанию и без тега при пустом), D-21 п. 6 (типографика).
5. Норма: PROP-057 `site` (SITE-ONE-SITE — корневые файлы генерирует та же
   сборка; SITE-ANALYTICS), `seo` (SEO-ROBOTS, SEO-SITEMAP, SEO-LLMS-FILES,
   SEO-STRUCTURED-DATA, SEO-CHARSET-AND-REDIRECTS, SEO-INDEXNOW), `stack`
   (STACK-ONE-BASE, STACK-PAGE-COUNT-GATE, STACK-BUILD-HYGIENE).
6. Источник содержания, **только чтение** (R-28): `C:\Users\olegc\git\v\vibevm-org\`
   — `src/i18n.ts` (строки обеих локалей дословно), `src/components/**`
   (анимации `draw`/`pop`/`pulse`, `prefers-reduced-motion`),
   `src/layouts/BaseLayout.astro` (мета-теги один к одному), `src/pages/**`,
   `public/*` (`robots.txt`, `llms.txt`, `llms-full.txt`, `sitemap.xml`,
   `feed.xml`, ключ-файл IndexNow, `og.png`, `favicon.svg`),
   `scripts/build-llms-full.mjs`, `nginx.conf` (только чтобы знать
   редиректы). Эталонная сборка Astro для сравнения — **в копии в
   scratch-каталоге**, не в самом репозитории (A0.26 §2 говорит, как она
   делалась).
7. Код пакета: `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/` — `design/`
   (токены, `docs-header`, `footer`, `theme-init.js`, `fonts.css`),
   `site/src/routes/layout.tsx`, `site/src/routes/index.tsx` и
   `ru/index.tsx` (заглушки P4-O1 — заменить), `site/src/lib/href.ts`,
   `tools/build.mjs`, `site/README.md`.
8. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `git push` не делать; чужие
   незакоммиченные файлы не стейджить (в дереве работают другие воркеры:
   P2-O8 в `crates/**` и `DEV-GUIDE.md`; **P4-O2 в том же web-пакете** —
   его периметр `site/src/routes/doc/**`, `site/src/reader/**`,
   `site/src/lib/**`, `site/src/tests/**`, компоненты ридера в `design/`;
   `site/src/routes/layout.tsx` — его, тебе туда только добавить
   лендинговые пункты навигации одной правкой в конце работы, если
   нужно); `cargo` не запускать; сеть — только `pnpm install` из
   настроенного npm-реестра и документация провайдеров краулеров (F-38);
   `node_modules/`, `dist*/`, `server*/`, `tmp/` — в `.gitignore`, никогда в
   коммите; секреты и `infra/` не читать; **картинки и шрифты — исходники
   из `vibevm-org/public/`, суммарно не больше 2 МБ, в отчёт `du -sh`**.

## Твой периметр файлов

`site/src/routes/index.tsx`, `site/src/routes/ru/**`, `site/src/routes/404*`,
`site/src/routes/en/**` (редирект `/en/` → `/`), `site/src/landing/**`
(новое: `i18n.ts`, компоненты страницы), `design/src/components/hero/**`,
`design/src/components/dep-graph/**`, `design/src/components/capability-card/**`,
`site/public/**` (`og.png`, `favicon.svg`, ключ-файл IndexNow — как файлы;
`robots.txt`, `llms.txt`, `llms-full.txt`, `sitemap.xml`, `feed.xml` —
генерируются сборкой, не лежат в `public/`), `tools/root-files.mjs`
(новое: генерация корневых файлов, вызывается из `tools/build.mjs`
статической сборки), `tools/parity.mjs` (новое), `site/src/config.ts`
(новое: домен, website-id Umami, ключ IndexNow — значения из окружения
сборки с безопасными пустыми умолчаниями), `site/README.md` (раздел о
лендинге), `tools/build.mjs` (только вызов генерации корневых файлов и
счёт лендинговых страниц).

## Цель — два атома, два коммита

**A4.16 Лендинг.** Маршруты `/` и `/ru/` на компонентах `design/`: шапка и
футер — те же, что у документации (навигация получает пункт
«Documentation»/«Документация» на `/doc/`); hero (eyebrow с точкой,
заголовок Spectral с `<em>`-акцентом, лид с мандатным описателем в
`<strong>`, кнопки GitHub и GitVerse, пилюля Early Access и команда
установки), граф зависимостей как Qwik-компонент с теми же анимациями
`draw`/`pop`/`pulse` и `prefers-reduced-motion`, ряд из трёх карточек
способностей, `404`; тексты — из `i18n.ts` дословно, обе локали;
мета-теги `BaseLayout.astro` один к одному (`canonical`, `hreflang`
en/ru/x-default, `og:*`, `twitter:*`, JSON-LD `SoftwareApplication` +
`WebSite`, `theme-color`, preload шрифтов по локали); корневые файлы
генерируются сборкой: `robots.txt` (ASCII-only, allow-лист по F-38 с
проверкой имён по документации провайдеров в день сборки — источники и
дата в отчёт, `Sitemap:` на `/sitemap.xml` и `/doc/sitemap.xml`),
`llms.txt` (абзац дизамбигуации дословно, разделы как были, плюс ссылка на
`/doc/llms.txt`), `llms-full.txt` по образцу `build-llms-full.mjs`,
`sitemap.xml`, `feed.xml`, ключ-файл IndexNow тем же именем, `og.png`,
`favicon.svg`, шрифты по прежним путям `/fonts/*.woff2` (копии или
симлинки сборки из `design/fonts/` — не дублировать байты в репозитории);
тег Umami — из `site/src/config.ts`, только при непустом website-id и
только в статической сборке. Без Tailwind. Коммит:
`feat(web): port the landing onto the shared design system`.

**A4.17 Тест паритета.** `tools/parity.mjs`: сравнивает эталонную сборку
Astro (путь к ней — аргумент; сама сборка делается в scratch-копии
`vibevm-org` командой из A0.26 §2) и выход `pnpm build:static`: множества
адресов (`/`, `/ru/`, `/404.html`, `/robots.txt`, `/llms.txt`,
`/llms-full.txt`, `/sitemap.xml`, `/feed.xml`, ключ-файл IndexNow,
`/og.png`, `/favicon.svg`, `/fonts/*`) равны с точностью до заранее
перечисленных добавлений (`/doc/…`, `/en/`); для HTML — `lang`, `<title>`,
`meta description`, `canonical`, все `hreflang`, `og:*`, `twitter:*`,
JSON-LD (нормализованный), видимый текст (нормализованный по пробелам),
тег Umami; для `robots.txt` и ключ-файла — байт в байт; для `llms.txt` —
абзац дизамбигуации дословно и все ссылки эталона на месте; для
`sitemap.xml` — те же адреса плюс `/doc/…`. Отчёт скрипта: каждая разница
— либо починена, либо записана как осознанное отличие с причиной (список
осознанных отличий — в самом скрипте, с причинами). Зелёный отчёт — гейт
переключения домена (A5.6). Коммит:
`test(web): pin landing parity against the Astro build`.

## Гейты (вывод дословно в отчёт)

`pnpm floor` — зелёный; `pnpm build:static` — число страниц сходится
(лендинговые маршруты + фикстурные страницы документации); `node
tools/parity.mjs <эталон> dist` — зелёный, вывод в отчёт целиком;
`grep -rn "#[0-9a-fA-F]\{3,8\}" design/src site/src` — пусто; `grep -rl
"https://" dist` — только `vibevm.org`, `github.com/vibevm`,
`gitverse.ru/vibevm` и адреса из эталона; `du -sh site/public design/fonts`
— в отчёт; `bash tools/self-check.sh` — только шаг web-пакета.

## Что не делать

Ридер, оглавление, страницы документации — периметр P4-O2; крейт
оболочки, сервер, развёртывание — нет; Tailwind — нет; внешние CDN и
шрифты по сети — нет; `vibevm-org` не править (R-28); PROP-файлы и вижен
не менять; `cargo` не запускать; значения website-id и ключа IndexNow не
искать в чужих файлах и не выдумывать — они приходят из окружения сборки,
в репозитории только пустые умолчания (значение website-id владелец
перенесёт в конфигурацию A5.1).

## Результат

Два коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P4-O5.md`
по форме отчёта P4-O1: таблица переноса «было в Astro → стало в Qwik» по
A0.26, список осознанных отличий паритета с причинами, имена краулеров с
источниками и датой проверки, вывод гейтов дословно, два скриншота
лендинга (1440 и 390 px, светлая и тёмная темы) в
`campaigns/docs-2026-09/findings/P4-O5-shots/` (PNG, суммарно не больше
2 МБ) — если Playwright уже стоит в пакете после P4-O2; иначе без
скриншотов, с пометкой. Никаких путей вне репозитория, IP и секретов в
отчёте (R-25).
