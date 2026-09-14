# PACKET-W1-O5 — сайт: глиф карточки по виду пакета (web, без Rust)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией. Коммитишь **только** формой `git commit -m … -- <пути>`, новые файлы — `git add --
<файл>`; никогда `git add -A`, никогда голый `git commit`, никаких трейлеров `Co-Authored-By` и никаких
упоминаний модели или агента в сообщениях коммитов — авторство репозитория человеческое (PROP-000
`#commits`). `git push` не делаешь. `cargo` не запускаешь. Никакие процессы, которых ты не запускал, не
останавливать.

## Зачем

Замечание владельца 11 (`findings/OWNER-REVIEW-2026-09-14.md`): иконка на карточке должна отражать вид
пакета — flow как flow, doc как doc. Глифы нарисованы центральной сессией:
`campaigns/docs-2026-09/findings/W1-O5-glyphs.svg` — восемь `<symbol>` 24×24 (`kind-flow`, `kind-feat`,
`kind-stack`, `kind-tool`, `kind-mcp`, `kind-lang`, `kind-doc`, `kind-app`), штрих `currentColor`.

## Читать сначала, ровно эти файлы

1. `campaigns/docs-2026-09/findings/W1-O5-glyphs.svg` — глифы.
2. `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/src/components/card/**`, `doc-card/**`,
   `package-header/**` (плейсхолдер «один символ по виду пакета»), `badge/**`, `design/src/index.ts`,
   `design/tokens.css`.
3. `site/src/lib/view.ts`, `lib/library.ts`, `site/src/routes/doc/index.tsx`, `routes/doc/[...path]/index.tsx`
   (где карточкам передаётся вид пакета; у doc-пакета вид `doc`, у проекции уровня 0 — вид её пакета из
   манифеста; если вид пакета до карточки не доходит — донеси его из манифеста, `site/src/generated/doc-manifest.ts`
   только читать), `site/src/components/catalogue/**`.
4. `README.md` web-пакета, `site/README.md`, `package.json`, тесты компонентов карточки.

## Сделать — один атомарный коммит

Глифы — в дизайн-систему одним компонентом `KindGlyph` (`design/src/components/kind-glyph/`): инлайновый
SVG по `kind`, восемь символов из файла (перенеси контуры как есть, размер и штрих — токенами
дизайн-системы, `aria-hidden`, вид пакета — в `title`/`aria-label` карточки, а не глифа). Карточки полок
(`Card`/`DocCard`) и заголовок страницы пакета показывают глиф своего вида вместо сегодняшнего символа;
неизвестный вид — сегодняшний плейсхолдер. На узких экранах — как сейчас (W1-O1 выровнял глиф с
названием на широких). Обе темы. Скриншоты каталога 1440 и 390 в
`campaigns/docs-2026-09/findings/W1-O5-shots/`.
Коммит: `feat(web): draw a package's kind on its card`.

Общее: никаких инлайновых скриптов и внешних ресурсов; стиль кода и докстрингов — как у соседей.

## Периметр файлов

`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/**` и `design/**` (кроме `site/src/generated/**`,
`docker/**`, `site.example.toml`), `campaigns/docs-2026-09/findings/W1-O5-shots/**`. (Если к запуску пакета
web-пакет уже переехал в `v1.0.0` — W1-O2 — работай там; центральная сессия скажет при запуске.)

## Самопроверка (обязательно, вывод в отчёт)

```
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 floor
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:static
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:embedded
```

## Приёмка боссом

Один коммит; восемь видов узнаются на карточках в обеих темах; неизвестный вид не ломает карточку;
гейты зелёные; скриншоты.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-W1-O5.md` (не коммитить; скриншоты — коммитить): хэш и
subject, откуда карточка узнаёт вид, вывод гейтов дословно, аномалии.
