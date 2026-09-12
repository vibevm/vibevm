# PACKET-P2-O6 — цитаты `rule` и рёбра, HTML-остров, номера блоков (A2.11, A2.13, A2.22)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммиты пакетов P2-O2 (`a7bff252`) и
P2-O4 (крейт `vibe-doc`, раннер, `derived`) в ветке — проверь `git log
--oneline -12` и прочитай `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O4.md`
(решения о фикстурах, нормализации и структуре крейта обязательны для тебя).

## Читать сначала

1. Этот пакет целиком; отчёты P2-O2 и P2-O4.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A2.11**, **A2.13**, **A2.22**;
   §2.8 пункты 3–5.
3. Находки: `campaigns/docs-2026-09/findings/A0.6-edges-host-or-engine.md`
   (сканер документов — хостовый, через шов `CodeScanner` движка; движок
   карты не трогать, R-21), `A0.25-block-numbering.md` (`number_blocks` в
   `vibe-doc`, до фильтрации `when`), `A0.22-reader-inventory.md` (что
   ожидает ридер от острова).
4. Норма: PROP-057 секции `pipeline` (PIPE-*), `reader`
   (READER-NUMBERED-BLOCKS и соседи), `observability` (OBS-RULE-EDGE-UNPINNED,
   OBS-*), `invariants` (INV-ANCHORS-IMMUTABLE); PROP-045 §7
   (`DOC-VOCAB-RULE-ADDRESS`); `vibevm/vibespecs/design/documentation-vision.xml`
   D-22 (нумерация `pNN`, F-43), D-27 (никаких пинов и «устарело»).
5. Код: `crates/vibe-doc/` (после P2-O4), `crates/vibe-specdoc/src/doc.rs`
   (`Block::Rule { uri, rev }`, `BlockNode { when, block }`), `crates/vibe-trace/`
   и `xtask/src/specmap*` (как пишется карта, шов сканера), `specmap.json`
   (форма узлов и рёбер), резолвер `spec://` — `crates/vibe-cli/src/commands/explain*`
   и `vibe-workspace` (адреса по store, lock, in-tree, git-источнику).
6. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `cargo fmt --all`;
   `unwrap`/`expect` в доменной логике запрещены; ошибки цитируют
   `spec://…`; гейты в приватном `CARGO_TARGET_DIR=<scratch>\target-p2o6`;
   секреты и `infra/` не читать; `git push` не делать.

## Цель — три атома, три коммита

**A2.11** Резолвер `spec://` по источникам (store, lock, in-tree,
git-источник хоста); при сборке страницы — текст факта по адресу (для
`Rule`); сканер документов пишет рёбра `documents` в карту doc-пакета
**без пина** — только адрес факта (D-27); `vibe doc check --citations`:
неразрешимый адрес или исчезнувший факт — ошибка, ничего больше (ни
ревизий, ни хэшей, ни «устарело»); правила X-029: адрес с многоточием или
`&lt;…&gt;` в прозе — плейсхолдер, `org.acme` — учебная организация,
`spec://org.vibevm.core/vibevm-docs/…` — самоадресация по дереву пакета.
Прогон по руководству: 381 `rule` — все разрешаются (расхождение — в
отчёт, не правка страниц). Коммит:
`feat(doc): resolve rule citations against the current specs`.

**A2.13** `to_html` в `vibe-doc`: секции → `<section id>`, факты →
`<p data-fact="ID" data-status="…">`, списки, таблицы, fence с классом
языка, цитаты; `Rule` → `<a class="rule" href="…" data-uri>` с вложенным
текстом факта и пометкой языка спеки (без `data-rev`, D-27); `Example` и
`ExampleRef` → `<div class="example">` с командой и выводом; `Derived` —
вставленный fence; `Note`, `Figure`, `Prompt` (блок с текстом, needs,
outcome, ассертами); `when` → `data-when`. Никаких скриптов и стилей
внутри острова. Голдены на пакет-фикстуру. Коммит:
`feat(doc): render islands from the pivot so web and local share one content path`.

**A2.22** Детерминированная нумерация блоков после раскрытия `derived`, до
бэкендов (A0.25): HTML — `<a class="p-anchor" id="p07" href="#p07">07</a>`
первым дочерним узлом блока (заголовок сохраняет `id`, номер — атрибутом
`data-p`); `.md` — `[p07]` в начале блока; `.xml` — атрибут `p="7"`;
`llms-full.txt` — как `.md`. Тест: одна фикстура, три проекции, один набор
номеров; повторный рендер — те же номера; `footnotes` не нумеруется.
Коммит: `feat(doc): number blocks at build time across every projection`.

## Гейты

`cargo fmt --all --check`, `cargo build -p vibe-doc -p vibe-cli`, `cargo test
-p vibe-doc` и тронутых крейтов, `cargo clippy` по ним, `cargo xtask
specmap` (0 suspects), `target/debug/vibe.exe facts check --exhaustive` —
clean.

## Что не делать

Прозу страниц не менять; PROP-файлы не менять; движок карты не менять;
`wire-derive-baseline.json` не менять; чужие незакоммиченные файлы не
стейджить. Не открывать других атомов.

## Результат

Три коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O6.md`
по форме отчёта P2-O1, плюс список цитат, которые не разрешились, с
разбором «страница или спека».
