# PACKET-P2-O2 — словарь документации в пивоте (A2.8)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `campaigns/docs-2026-09/PLAN.md` — атом **A2.8** (раздел «Фаза 2 —
   механика») и §2.8 пункты 3–5.
3. `campaigns/docs-2026-09/findings/A0.4-pivot-extension-points.md` — где в
   пивоте (`xml_in`, `xml_out`, `md_out`, IR) живут точки расширения, как
   устроен диспетчер «литерал против именованной секции», флаг жанра.
4. Норма: `vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml` §7
   `documentation-vocabulary` (DOC-VOCAB-*, строки таблицы
   ROW-DOCVOCAB-*) — точный список элементов, их атрибутов, MD-проекций и
   проверок; `DOC-VOCAB-DISCRIMINATOR` (элемент словаря никогда не несёт
   `title=`), `DOC-VOCAB-WHEN-SLOT`, `DOC-VOCAB-MD-ONE-WAY`.
5. Живой корпус для round-trip-тестов: 44 страницы
   `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/**/*.xml`
   — используют `example`/`run`/`expect`/`stderr`, `rule ref`, `derived
   kind ref`, `note kind`, `prompt id` с `needs`/`outcome`/`assert`, `when`
   на секциях; `figure` в корпусе нет — покрыть юнит-тестом.
6. Репозиторные правила, которые тебя связывают: Conventional Commits с
   телом «почему»; один атомарный коммит; **никаких трейлеров и упоминаний
   моделей в коммитах**; `cargo fmt --all` перед коммитом; `unwrap`/`expect`
   в доменной логике запрещены; ошибки цитируют `spec://…`; секреты и `infra/`
   не читать; `git push` не делать.

## Цель

Один атом, один коммит: варианты IR `Block::Example { id, fixture, lang,
exit, when, run, expect, stderr }`, `Block::ExampleRef { id }`, `Block::Rule
{ uri, rev }`, `Block::Derived { kind, reference }`, `Block::Note { kind,
body }`, `Block::Figure { src, alt, caption }`, `Block::Prompt { id, text,
needs, outcome, asserts }`; атрибут `when` на секциях и на
`Example`/`Note`/`Prompt`. `xml_in` принимает их **только в режиме жанра
документации** (флаг загрузчика по виду пакета `doc`, F-08 плана), иначе —
громкая ошибка как для чужого элемента с адресом
`spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-BY-KIND`; `xml_out`
пишет их детерминированно (байт-идемпотентный round-trip XML→IR→XML на всех
44 страницах — это тест); `md_out` проецирует по таблице PROP-045 §7
(`example` → соседние fence `sh` и `output` (+ `stderr`); `rule` → строка
цитаты с адресом; `derived` → fence с пометкой «generated from …»; `note` →
абзац с меткой вида; `figure` → изображение с подписью; `prompt` → fence
`prompt`, список «needs», абзац «outcome», список ассертов; `when` → пометка
блока); тест «вне режима — ошибка». Существующие документы и их проекции не
меняются ни на байт — это тоже тест (голдены пивота, если есть).

Коммит: `feat(specdoc): give the documentation genre its seven elements`.

## Гейты

`cargo fmt --all --check`, `cargo build --workspace`, `cargo test -p <крейт
пивота>` и всех зависимых, `cargo clippy --workspace --all-targets -- -D
warnings` по тронутым крейтам, `cargo xtask specmap` при новых тегах (ноль
suspects), `target/debug/vibe.exe facts check --exhaustive` — по-прежнему
clean (325 файлов).

## Что не делать

Страницы руководства не править (если round-trip находит в них дефект —
записать в отчёт, не чинить). Не трогать `vibe-core`, wire, манифест —
это пакет P2-O1, идущий параллельно; если вам нужно одно и то же место,
остановись и опиши. Не открывать других атомов.

## Результат

Коммит в ветке `research-preview-1-docs` (без push) и отчёт
`campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O2.md`: хэш и subject;
решения (особенно форма флага жанра и то, как `when` живёт в IR); вывод
гейтов дословно; дефекты страниц, найденные round-trip'ом; что не сделано и
почему; `git status --short` в конце.
