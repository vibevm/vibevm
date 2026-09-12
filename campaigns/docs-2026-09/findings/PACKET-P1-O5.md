# PACKET-P1-O5 — фикстуры lock-файлов берут номер схемы из константы (хвост фазы 1)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O4.md` §4.1 — список
   точек: 11 фикстур в дереве пишут `schema_version = 6` при
   `CURRENT_SCHEMA_VERSION = 7` (с коммита `72d866d2`); каскад роняет ~8
   целей `cargo test --workspace`, в том числе `crates/vibe-cli/tests/cli_facts.rs:129`,
   `crates/vibe-workspace` `bins::tests::*`, `crates/vibe-index/tests/golden_corpus.rs`
   (последний имеет и вторую причину — перелом провода, не твой мандат).
3. `crates/vibe-core/src/manifest/lockfile.rs` — где объявлена
   `CURRENT_SCHEMA_VERSION` и как читается lock-файл старой схемы.
4. Правила: Conventional Commits с телом «почему»; один коммит; **никаких
   трейлеров и упоминаний моделей**; `cargo fmt --all`; ничего вне тестов и
   фикстур не менять; секреты и `infra/` не читать; `git push` не делать.

## Цель

Один коммит: каждая тестовая фикстура lock-файла, пишущая литерал
`schema_version = 6`, берёт номер из `CURRENT_SCHEMA_VERSION` (через
`format!`/константу теста, а не новый литерал `7`), чтобы следующий подъём
схемы не ронял их снова. Продуктовый код не трогать. Если какая-то фикстура
намеренно тестирует **старую** схему (миграцию), оставить литерал и назвать
её в отчёте. `cargo test --workspace --no-fail-fast` после правки: красными
могут остаться только цели, чья причина — корпус `index/e1` (перелом
провода, J-069); их перечислить дословно.

Коммит: `test: take the lock schema version from the constant in every fixture`.

## Что не делать

Не трогать `formats/`, `crates/vibe-index/src`, `Cargo.toml`, `Cargo.lock`,
страницы, зону кампании (кроме отчёта). Не чинить корпус `index/e1`. В дереве
работает другой воркер (P2-O1) — его незакоммиченные файлы в `crates/vibe-core`,
`crates/vibe-wire`, `crates/vibe-cli/src`, `xtask/src`, `formats/`, `schemas/`
не трогать и не стейджить; стейджи только свои тестовые файлы.

## Результат

Коммит (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O5.md`:
список точек с решением по каждой; дословный хвост `cargo test --workspace
--no-fail-fast`; что осталось красным и почему; `git status --short`.
