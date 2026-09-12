# PACKET-P2-O8 — покрытие, плейсхолдеры, `vibe doc` CLI, MCP, скилл (A2.16, A2.17, A2.18, A2.19, A2.20)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммиты пакетов P2-O3 (индекс, гейт,
скаффолд, прогрев), P2-O4 (крейт `vibe-doc`), P2-O6 (цитаты, остров,
номера) и P2-O7 (манифест страниц, `llms`, переводы) в ветке — проверь
`git log --oneline -30` и прочитай их отчёты `WORKER-REPORT-P2-O3/O4/O6/O7.md`.

## Читать сначала

1. Этот пакет целиком; четыре отчёта выше.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A2.16**, **A2.17**, **A2.18**,
   **A2.19**, **A2.20**; §2.8 пункты 3–5; правило R-19 (картинки: PNG,
   JPEG, WebP; SVG запрещён) и R-11 (`DEV-GUIDE.md` правится тем же
   коммитом, что и поверхность).
3. Находки: `campaigns/docs-2026-09/findings/A0.9-skills.md` (`[[skill]]` по
   образцу, строка во встроенный шаблон безусловная), `A0.5-audience-agent.md`,
   `A0.7-http-server.md` (сервер — отдельный крейт `vibe-doc-server`, CSP
   строкой, без CORS, только `127.0.0.1`; в этом пакете `vibe doc serve`
   отдаёт **голые острова**, оболочка — фаза 4).
4. Норма: PROP-043 `AUDIENCE-DOC-USE` (гейт покрытия), PROP-047
   `CMD-REPORT`, `DOC-COVERAGE-RATCHET`; PROP-057 `card` (CARD-PLACEHOLDERS-GENERATED,
   CARD-PREVIEW-COMPOSED, CARD-MEDIA-ROLES), `local` (LOCAL-*), `pipeline`
   (PIPE-*), `site` (SITE-*) в части того, что сервер обязан отдавать;
   `vibevm/vibespecs/design/documentation-vision.xml` §7.4 (скилл), D-20
   (лимиты картинок).
5. Текст скилла руководства (проза центральной сессии, не менять):
   `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/skills/vibevm-docs/SKILL.md`;
   встроенный шаблон скилла `vibevm`: `crates/vibe-mcp/src/skill_template.md`
   (туда — одна строка-указатель на руководство и на `vibe doc`; заодно
   поправь устаревшую фразу про два реестра от `vibe init`, B-136).
6. Код: `crates/vibe-doc/`, `crates/progress-core/src/report.rs`,
   `crates/vibe-cli/src/commands/doc*` (после P2-O4 там уже `vibe doc check`),
   `crates/vibe-mcp/`, `crates/vibe-index/` (образец сервера на axum —
   `A0.7`), `DEV-GUIDE.md` §8.
7. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `cargo fmt --all`;
   `unwrap`/`expect` в доменной логике запрещены; ошибки цитируют
   `spec://…`; новый крейт — по чеклисту §2.8 целиком; гейты в приватном
   `CARGO_TARGET_DIR=<scratch>\target-p2o8`; секреты и `infra/` не читать;
   `git push` не делать.

## Цель — пять атомов, пять коммитов

**A2.16** `vibe doc check --coverage`: через `progress-core` — факты с
`actionstage="doc"` и аудиторией; для каждого требуется входящее ребро
`documents` от страницы, чья аудитория включает ту же; отчёт «покрыто / не
покрыто» по аудиториям; порог 100 % по умолчанию, `--min <процент>` для
промежуточных прогонов. Прогон по руководству — числа в отчёт (они пойдут
в леджер §coverage). Коммит: `feat(doc): gate documentation on spec obligations`.

**A2.17** Детерминированные генераторы: баннер и иконка как SVG из хэша
координаты (палитра и узор — от хэша, глиф — от вида); превью 1200×630 как
растр при сборке (из плейсхолдера, иконки и `title`); картинки из
`[media]` копируются под хэшированными именами; `vibe doc check --media`
дублирует ячейку `vibe check` для собранного пакета. Коммит:
`feat(doc): derive placeholders and link previews from the coordinate`.

**A2.18** `vibe doc build [--path] [--out] [--format html|md|xml] [--lang]`,
`vibe doc check [--examples] [--citations] [--derived] [--translations]
[--coverage] [--media] [--style]` (`--style` появится в A2.25 — оставь
место), `vibe doc manifest [--json] [--llms <tier>] [--lang]`,
`vibe doc serve [--port] [--base] [--lang]` — тонкий адаптер над
библиотекой; сервер — крейт `vibe-doc-server` по §2.8 (axum, `127.0.0.1`,
CSP строкой, без CORS, отдаёт голые острова и проекции). `DEV-GUIDE.md`
§8 — тем же коммитом. Коммит:
`feat(cli): expose the documentation pipeline as vibe doc`.

**A2.19** Документация входит в `explain`/`query`/`select` через карту без
новых инструментов; инструмент `read_doc` (страница по адресу, формат
md/xml, язык) в `vibe-mcp`. Коммит: `feat(mcp): serve documentation pages to agents`.

**A2.20** `[[skill]]`-декларация в doc-пакете ядра уже есть (`vibe.toml`
руководства); одна строка-указатель во встроенном шаблоне `vibevm`; `vibe
skill list` видит скилл руководства после `vibe cache add` (или через
in-tree реестр). Коммит: `feat(skill): point agents from errors to rules and docs`.

## Гейты

`cargo fmt --all --check`, `cargo build -p vibe-doc -p vibe-doc-server -p
vibe-cli -p vibe-mcp`, тесты этих крейтов, `cargo clippy` по ним, `cargo
xtask conform check` по новому крейту, `cargo xtask specmap` (0 suspects),
`vibe.exe facts check --exhaustive` — clean, `vibe doc check --examples
--derived --citations --coverage --media --path
vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0` — вывод дословно в
отчёт (покрытие ниже 100 % не блокирует этот пакет: числа идут в леджер).

## Что не делать

Прозу страниц и скилла не менять; PROP-файлы не менять; оболочку ридера
(фаза 4) не начинать; сервер не слушает ничего, кроме `127.0.0.1`; чужие
незакоммиченные файлы не стейджить.

## Результат

Пять коммитов (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O8.md`
по форме отчёта P2-O1, с таблицей покрытия по аудиториям.
