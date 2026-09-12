# PACKET-P2-O3 — индекс, гейт, скаффолд и прогрев для `doc` (A2.3, A2.5, A2.6, A2.7)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммиты пакета P2-O1 (A2.1, A2.2, A2.4)
уже в ветке — проверь `git log --oneline -8` и прочитай
`campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O1.md` (решения и
развилки предыдущего воркера обязательны для тебя).

## Читать сначала

1. Этот пакет целиком; отчёт P2-O1.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A2.3**, **A2.5**, **A2.6**,
   **A2.7** и §2.8 пункты 3–5.
3. Находки: `campaigns/docs-2026-09/findings/A0.21-index-card-fields.md`
   (индекс: обратные запросы на стороне сайта, `[translations]` не хранится),
   `A0.16-check-cell.md` (одна ячейка гейта на правило, образец
   `snippet_presupposition.rs`), `A0.17-intree-to-store.md` (`cache add`,
   замыкание), `A0.8-store-and-sources.md`.
4. Норма: `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml`
   секции `kinds` (KIND-DOC-MUST-DOCUMENT, KIND-DOC-MUST-NOT-EXECUTE,
   KIND-DOC-MAY-DECLARE, KIND-DOC-PAGES-LOCATION), `relation`
   (REL-WARMUP-CLOSURE, REL-INDEX-FIELDS, REL-REVERSE-QUERIES-SITE-SIDE),
   `localization` (LOC-DOCUMENTS-MATCH, LOC-MIRROR, LOC-EXAMPLE-REF),
   `card` (CARD-MEDIA-ROLES, CARD-PLACEHOLDERS-GENERATED); лимиты картинок —
   `vibevm/vibespecs/design/documentation-vision.xml` D-20 и правило R-19
   плана (PNG, JPEG, WebP; SVG запрещён).
5. Эталон doc-пакета: `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/`
   (манифест, README, `vibevm/vibespecs/**`, `AUTHORING.md`) — гейт A2.5
   обязан пропускать его; скаффолд A2.6 делает уменьшенную копию его формы.
6. Код: `crates/vibe-index/` (в т.ч. `src/index/memory/tests.rs`),
   `schemas/index/*.jtd.json`, `crates/vibe-check/`, `crates/vibe-cli/src/commands/init/`,
   `crates/vibe-cli/src/commands/cache/`, `crates/vibe-registry/src/store.rs`.
7. Правила репозитория, названные целиком: Conventional Commits с телом
   «почему»; атом — коммит; **никаких трейлеров и упоминаний моделей**;
   `cargo fmt --all` перед коммитом; `unwrap`/`expect` в доменной логике
   запрещены; ошибки цитируют `spec://…`; wire только через схему →
   `cargo xtask codegen`; секреты и `infra/` не читать; `git push` не делать.

## Цель — четыре атома, четыре коммита, в этом порядке

**A2.3** Индекс: `known()` и CLI `--kind` знают `doc`/`app`; сканер
манифестов и первичная запись индекса несут `title`, `abstract`, `lang`,
`documents`, `documentation`, `translates` (схема `schemas/index/*.jtd.json`
→ codegen → корпус; `wire-diff` только сообщает при `public = false`);
обратные вопросы (`by-documents/{group}/{name}`, `by-translates/…`) — по
A0.21 на стороне сайта, в индексе не делать, если находка так решила.
Тесты в `crates/vibe-index/src/index/memory/tests.rs`. Коммит:
`feat(index): index doc packages with their relations and cards`.

**A2.5** Гейт `vibe check` для `doc`: без `[[documents]]`, `title` или
`abstract` — ошибка; с `[boot_snippet]`, `[[binary]]`, `[[mcp_server]]` —
ошибка; без `README.md` — ошибка; `documentation.primary` в чужую группу —
предупреждение; перевод: `translates` без `lang` — ошибка, `documents`
перевода ≠ `documents` источника — ошибка (источник из store или in-tree;
недоступен — предупреждение с причиной); `[media]`: файл существует,
сигнатура формата PNG/JPEG/WebP, SVG — ошибка, пропорции и размер — по D-20.
Одна ячейка на правило. `vibe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0`
зелёный. Коммит: `feat(check): gate doc packages, translations and media on their contract`.

**A2.6** `vibe init --kind doc`: скаффолд по PLAN A2.6 (манифест с `kind =
"doc"`, `title`, `abstract` с шаблоном четырёх вопросов в комментарии, `lang
= "en"`, `[[documents]]`-заглушка, `README.md`, `vibevm/vibespecs/README.xml`
с одним примером страницы, `specmap.toml`, `media/.gitkeep`); флаг
`--translates <координата>` зеркалит дерево источника из store. По пути
проверь и почини B-134 (`--kind` не действует у `vibe init package`) — это
тот же код. Коммит: `feat(init): scaffold doc packages and their translations`.

**A2.7** `vibe cache add` для doc-пакета добавляет в замыкание координаты из
`[[documents]]` и `[translates]` по их ограничениям версий
(REL-WARMUP-CLOSURE). Коммит:
`feat(cache): warm a doc package's subjects and source so citations resolve offline`.

## Хвост от P2-O5 (тот же коммит, что A2.5, или отдельный `chore(cli)`)

Воркер P2-O5 (`d608a797`, `b9be9f3b`) оставил на границе своего периметра
две правки в `crates/vibe-cli/src`, которые твои: (1) строки
`(expected user|author|dev)` в сообщениях об аудитории устарели — теперь
четыре значения; (2) `crates/vibe-cli/src/commands/progress/grounding.rs`
(около строки 122) выбирает словарь читателя пивота — для пакета вида
`doc` он обязан брать `Vocabulary::Doc` (`from_xml_with`), иначе `vibe
facts check` отказывает первой же странице руководства. После (2) добавь
пакет руководства в наблюдение `facts.toml` так, как решил P2-O5 (X-024:
`[judging] exempt` освобождает и от `--exhaustive`), и убедись, что
`vibe facts check --exhaustive` остаётся clean.

## Гейты каждого атома

`cargo fmt --all --check`, `cargo build --workspace`, тесты тронутых крейтов,
`cargo clippy --workspace --all-targets -- -D warnings` по тронутым крейтам,
`cargo xtask specmap` при новых тегах (ноль suspects), `check-codegen` и
`wire-diff` там, где менялась схема.

## Что не делать

Страницы руководства, зону кампании (кроме отчёта), PROP-файлы,
`VIBEVM-SPEC.md`, `wire-derive-baseline.json` не трогать. Не открывать
других атомов. Развилка без ответа в плане и PROP-057 — стоп на атоме,
описание с рекомендацией, следующий атом, если он не зависит.

## Результат

Четыре коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O3.md`
по форме отчёта P2-O1.
