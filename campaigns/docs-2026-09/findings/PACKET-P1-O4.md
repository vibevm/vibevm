# PACKET-P1-O4 — три унаследованных красных шага панели (фаза 1, хвост)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `tools/self-check.sh` — шапка (комментарий со списком шагов) и сами
   шаги 2, 3 и 6d: `cargo test --workspace`, `cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo xtask wire-diff`.
3. `campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O3.md` §5 — где
   предыдущий воркер увидел эти три красных и почему не чинил.
4. `crates/vibe-cli/src/commands/show/source_path.rs` — тест
   `embedded_root_uses_exact_lock_row_and_threads_offline` и его фикстура
   `bridge_package`.
5. `crates/vibe-core/src/manifest/package/embedded_source.rs` — поля
   `[[package.embedded_source]]` (обязательные `upstream_authors`,
   `upstream_license`, `license_path`, `license_url`).
6. `formats/REGISTRY.toml` и `xtask/src/` в части `wire-diff` — как
   устроен корпус голденов и что означает «drift».

Стоящие правила: коммиты не делать (только правки в дереве; коммитит
центральная сессия); `cargo fmt --all` после правок; ничего, кроме
названных ниже файлов и того, что `wire-diff` регенерирует по своему
контракту, не трогать; секреты и `infra/` не читать; `git` — только
`status --short`, `diff`, `rev-parse HEAD`.

## Цель

Панель `bash tools/self-check.sh` должна пройти шаги 2, 3 и 6d, которые
сейчас красные по причинам, не связанным с кампанией документации
(унаследованы из bridge-кампании, `main@b1291b06`):

1. **`cargo test --workspace`**: тест
   `commands::show::source_path::tests::embedded_root_uses_exact_lock_row_and_threads_offline`
   падает — фикстура lock-файла `bridge_package(..., embedded = true)` не
   несёт `upstream_authors`, обязательное с коммита `7b465809`. Починить
   фикстуру (добавить поле в том виде, в каком его ждёт схема), не тест и
   не схему.
2. **`cargo clippy … -D warnings`**: предупреждения в
   `crates/vibe-install/src/plan/fetch.rs` (около строки 286) и
   `xtask/src/bridge.rs` (около строки 78). Починить по существу
   предупреждения (не `#[allow]`), минимальной правкой.
3. **`cargo xtask wire-diff`**: дрейф корпуса под `formats/corpora/index/e1`.
   Понять, что дрейфует — голдены или эмиттер, — и привести в согласие
   так, как предписывает контракт `wire-diff` (если это регенерация
   голденов командой xtask — выполнить её; если это изменение провода —
   **остановиться** и описать в отчёте, потому что перелом провода — не
   твой мандат).

Затем прогнать панель целиком: `bash tools/self-check.sh`. Ожидание —
зелёная до конца или красная на шаге, который не входит в эти три; в
последнем случае записать шаг, вывод и остановиться.

## Результат

`campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O4.md`:

1. Что изменено, файл за файлом, с одной фразой «почему».
2. Дословный вывод трёх шагов после правки и хвост вывода панели
   (последние 30 строк) с кодом выхода.
3. Что не сделано и почему (в частности, если `wire-diff` оказался
   переломом провода).
4. `git status --short` в конце.

Готово, когда три шага зелёные, `cargo fmt --all --check` чист, отчёт лежит
на месте, и в дереве нет изменений вне названных файлов и регенерированных
голденов.
