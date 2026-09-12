# PACKET-P2-O11 — пол conform после волны фазы 2: десять файлов, выросших за бюджет

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: ветка `research-preview-1-docs` на
`6d95657a` или новее (`git log --oneline -5`).

## Почему пакет существует

Панель `tools/self-check.sh` (шаг 5, `cargo xtask conform check`) красна
27 новыми находками против пустой базовой линии `conform-baseline.json`.
Разбор центральной сессии по `git show b1291b06:<файл> | wc -l`:
**десять файлов кампания сама вырастила за бюджет 600 строк** (правило
`file-length`, `discipline://rust-ai-native-lang/guide#surface-form`),
**семнадцать находок унаследованы с `main`** и решаются владельцем
отдельно. Каждый воркер фазы гонял `conform check` только по своим новым
путям («по своим путям ноль»), поэтому рост старых файлов никто не
заметил. Твоя работа — вернуть десять файлов в бюджет **без изменения
поведения**.

## Читать сначала

1. Этот пакет целиком.
2. `conform.toml` (политика; таблица `[[rust.exempt]]` — ничего в ней не
   менять); `conform-baseline.json` — пустой, **не замораживать**
   (`conform freeze` — решение владельца, не твоё).
3. Норма: `vibevm/vibedeps/org.vibevm.ai-native.rust-ai-native-lang/1.0.0/vibevm/vibespecs/rust/GUIDE-AI-NATIVE-RUST.xml`
   — разделы `surface-form` (бюджет файла, «ячейки» модульного зерна) и
   `bans-and-escape-hatches`; как в этом дереве уже раскладывают файлы —
   соседи по каждому крейту (`crates/vibe-specdoc/src/xml_doc_tests.rs`
   как сосед-тест, `crates/vibe-cli/src/commands/doc/tests.rs` как
   `tests.rs` внутри каталога модуля): следуй конвенции **того крейта**,
   который правишь, не заводи третью.
4. `tools/self-check.sh` — шаги 1–5 (форма гейтов), чтобы гонять их
   по отдельности.

## Десять файлов (строк на базе `b1291b06` → сейчас)

| Файл | База | Сейчас |
|---|---|---|
| `crates/progress-core/src/model.rs` | 569 | 601 |
| `crates/progress-core/src/scope.rs` | 439 | 612 |
| `crates/vibe-cli/src/commands/progress/tests.rs` | 599 | 602 |
| `crates/vibe-index/src/index/memory/tests.rs` | 537 | 705 |
| `crates/vibe-index/src/scanner/manifest.rs` | 516 | 680 |
| `crates/vibe-specdoc/src/xml_doc.rs` | — (новый) | 625 |
| `crates/vibe-specdoc/src/xml_doc_tests.rs` | — (новый) | 737 |
| `crates/vibe-specdoc/src/xml_in.rs` | 540 | 662 |
| `crates/vibe-specdoc/src/xml_out.rs` | 488 | 654 |
| `xtask/src/codegen/optional_shapes/tests.rs` | 567 | 605 |

`crates/vibe-core/src/manifest/package.rs` (602 → 639) был за бюджетом
уже на базе и **не твой**: он идёт в список владельца вместе с
унаследованными.

## Что сделать

Каждый файл — разложить по швам ответственности на ячейки модульного
зерна, каждая ≤ 600 строк с запасом (целься в ≤ 500, чтобы следующая
правка не вернула находку): тесты — в соседний тестовый модуль по
конвенции крейта (по темам, не «первая половина / вторая половина»);
большие `impl`-блоки и группы свободных функций — в подмодули с именем
по ответственности (`xml_in/attributes.rs`, `scanner/manifest/relations.rs`
и подобные — имена выбери по содержимому). Правила:

- **поведение не меняется**: ни одной правки логики, ни одного нового
  теста, ни одного удалённого; публичный API крейта тот же (новые
  `pub(crate)` / `pub(super)` допустимы ровно там, где их требует
  разрез; новых `pub` наружу крейта — нет);
- `#[spec(...)]`-теги едут вместе со своими элементами; `use`-импорты
  сужены до нужного в каждой ячейке; `cargo fmt --all`;
- имена модулей и файлов — по ответственности, не по номеру части;
- **число тестов на крейт до и после равно** — сними `cargo test -p <крейт>`
  до правок и после, числа `passed` в отчёт парами;
- один коммит на крейт (пять коммитов): `refactor(progress): split the
  model and scope files into cells`, `refactor(cli): split the progress
  tests by topic`, `refactor(index): split the memory tests and the
  manifest scanner into cells`, `refactor(specdoc): split the
  documentation genre files into cells`, `refactor(xtask): split the
  optional-shape tests by topic` — subject'ы можешь уточнить, тело —
  «почему» (бюджет файла дисциплины, поведение не менялось, число
  тестов).

## Что не трогать

Семнадцать унаследованных находок и их файлы: `crates/vibe-agent-projection/src/pkgskill.rs`,
`crates/vibe-check/src/checks/bridge_provenance.rs`,
`crates/vibe-core/src/manifest/document/tests.rs`,
`crates/vibe-core/src/manifest/package.rs`,
`crates/vibe-core/src/manifest/lockfile.rs`,
`crates/vibe-core/src/manifest/package/embedded_source.rs`,
`crates/vibe-core/src/manifest/package/skill.rs`,
`crates/vibe-orchestrator/src/dispatch/native_all_owner_tests/support.rs`,
`crates/vibe-orchestrator/src/world/mod.rs`,
`crates/vibe-registry/src/embedded_source.rs`,
`crates/vibe-cli/src/commands/vvm/error.rs`,
`crates/vibe-cli/src/commands/vvm/placer.rs` — это код `main`, его
судьбу решает владелец. `conform.toml`, `conform-baseline.json`,
`specmap.json` (после `cargo xtask specmap` верни файл к HEAD — карту
перегенерирует интегратор одним прогоном на волну), PROP-файлы, страницы
руководства, web-пакет.

Параллельно в дереве работает P4-O4: `crates/vibe-doc-server/**`, новый
`crates/vibe-doc-shell/**`, `xtask/src/main.rs` и новый модуль xtask,
`crates/vibe-cli/src/commands/doc*`, `crates/vibe-doc/src/build.rs`,
`schemas/doc_manifest.jtd.json`, кодген `crates/vibe-wire/src/generated/**`,
`Cargo.toml`/`Cargo.lock` рабочего пространства. Ничего из этого не
править и не стейджить; для раскладки `optional_shapes/tests.rs` тебе
достаточно `xtask/src/codegen/optional_shapes/mod.rs` (или как там
объявлен модуль) — `xtask/src/main.rs` не трогать.

## Гейты (вывод дословно в отчёт)

`cargo fmt --all --check`; `cargo build -p progress-core -p vibe-index
-p vibe-specdoc -p vibe-cli -p xtask`; `cargo test` по тем же пяти
пакетам (числа до/после); `cargo clippy` по ним `--all-targets -- -D
warnings`; `cargo xtask conform check` — ожидаемый итог: **ровно 17
новых находок, все из списка «не трогать», ни одной в десяти файлах и в
ячейках, на которые они разложены** (полный список находок — в отчёт);
`cargo xtask specmap` — 0 suspects, 0 сирот (файл вернуть к HEAD);
`cargo xtask check-codegen` — clean (кодген ты не трогал — это
контроль). Всё в приватном `CARGO_TARGET_DIR` (см. запуск); в конце
удалить, `df -h .` до и после — в отчёт.

## Правила репозитория

Conventional Commits с телом «почему»; **коммитить только явной формой
`git commit -m … -- <свои пути>`**, новые файлы сначала `git add -- <файл>`;
никаких трейлеров и упоминаний моделей; `git push` не делать; секреты и
`infra/` не читать; чужие незакоммиченные файлы не стейджить; отчёт
`campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O11.md` (не коммитить
— вносит центральная сессия): по файлу — что куда уехало и строки
после, числа тестов парами, вывод гейтов дословно, что не сделано и
почему. Ни одного пути вне репозитория, IP и секретов в отчёте.
