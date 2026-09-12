# PACKET-P2-O7 — манифест страниц и `llms`, мета-блок, проверка переводов (A2.14, A2.24, A2.15, A2.23)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммиты пакетов P2-O4 (крейт `vibe-doc`)
и P2-O6 (цитаты, остров, номера блоков) в ветке — проверь `git log
--oneline -16` и прочитай `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O4.md`
и `WORKER-REPORT-P2-O6.md`.

## Читать сначала

1. Этот пакет целиком; отчёты P2-O4 и P2-O6.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A2.14**, **A2.24**, **A2.15**,
   **A2.23**; §2.8 пункты 3–5.
3. Норма: PROP-057 секции `relation` (REL-OFFICIAL-IS-CONVERGENCE,
   REL-DEFAULT-CONVENTION, REL-NO-OFFICIAL-FLAG), `localization` (LOC-MIRROR,
   LOC-NO-REVISION, LOC-EXAMPLE-REF, LOC-DOCUMENTS-MATCH,
   LOC-OFFICIAL-TRANSLATION), `seo` (SEO-LLMS-FILES, SEO-RAW-PROJECTIONS),
   `pipeline` (PIPE-*), `card` (CARD-*); `vibevm/vibespecs/design/documentation-vision.xml`
   D-19 (статусы официальности), D-22 п. 2 и п. 8 (проверка по блокам,
   мета-блок), D-26 (`reviews.toml`), D-27 (никаких версий и отставания),
   F-13 (`llms.txt` как формат со `schema = "none"`), F-44.
4. Код: `crates/vibe-doc/`, `formats/REGISTRY.toml`, `schemas/` (форма
   JTD-схем, `x-wire-order`, `x-empty` — образец `schemas/distribution/e1/…`),
   `xtask/src/codegen`, `crates/vibe-specdoc/src/doc.rs`.
5. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `cargo fmt --all`;
   `unwrap`/`expect` в доменной логике запрещены; ошибки цитируют
   `spec://…`; wire только через схему → `cargo xtask codegen`; гейты в
   приватном `CARGO_TARGET_DIR=<scratch>\target-p2o7`; секреты и `infra/`
   не читать; `git push` не делать.

## Цель — четыре атома, четыре коммита

**A2.14** JTD-схема `schemas/doc_manifest.jtd.json` (пакет: координата,
`title`, `abstract`, `description`, `lang`, издатель, статусы на двух уровнях
— `primary`/`official`/`community` для документации и `official`/`community`
для перевода, вычисленные по D-19; страницы: путь, заголовок, аудитории,
жанр, якоря, резюме = ведущий факт); регистрация в `formats/REGISTRY.toml`
как `doc-manifest` с корпусом; `cargo xtask codegen`; генераторы `llms.txt`,
`llms-full.txt`, `llms-small.txt`, `llms-medium.txt` по бюджетам, в порядке
закона слоёв, на каждый язык; каталог документаций пакета в стиле arXiv;
`llms.txt` — формат со `schema = "none"` (F-13). **Никакого поля
«отставание перевода»** (D-27, седьмая редакция плана снимает его).
Коммит: `feat(doc): derive the page manifest, statuses and llms indexes`.

**A2.24** Манифест страниц получает `reading_time_min` (200 слов/мин для
`ru`, 250 для `en`, иные — 200), `published_at`, `publisher`, `rendered_at`,
`reviewed_at` (из `reviews.toml` пакета, D-26); **ничего** про версии,
отпечатки, хэши источника; JTD и codegen; `llms.txt` печатает время чтения в
строке страницы. Коммит:
`feat(doc): carry reading time and provenance in the page manifest`.

**A2.15** `vibe doc check --translations`: для перевода — источник по
`translates` (store или in-tree); наборы путей и якорей равны (R-18);
`example ref` резолвится в источнике; собственный `example` в переводе —
ошибка; без пинов и «отставания». Коммит:
`feat(doc): check translations page by page`.

**A2.23** `--translations` сверяет и число, и типы блоков каждой страницы с
источником; расхождение — ошибка с первым разошедшимся `pNN` и обеими
страницами. Фикстура: источник и перевод из двух страниц с намеренным
расхождением. Коммит: `feat(doc): check translations block by block`.

## Гейты

`cargo fmt --all --check`, `cargo build -p vibe-doc -p vibe-cli -p vibe-wire`,
`cargo test -p vibe-doc -p vibe-wire`, `cargo clippy` по тронутым, `cargo
xtask check-codegen`, `cargo xtask specmap` (0 suspects), `vibe.exe facts
check --exhaustive` — clean.

## Что не делать

Прозу страниц не менять; PROP-файлы не менять; `wire-derive-baseline.json`
не менять; чужие незакоммиченные файлы не стейджить; других атомов не
открывать.

## Результат

Четыре коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O7.md`
по форме отчёта P2-O1, с примером сгенерированного `llms.txt` для
руководства (первые 20 строк) в отчёте.
