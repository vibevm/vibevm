# Пакет LP-O3 — отчёт `--chapters --json` как зарегистрированный формат (opus5, High) — 2026-09-25

##subagent-quiet-clause
«Ты работаешь в СУБАГЕНТСКОМ режиме: твой экранный текст не читает никто,
деливерабл — только артефакты. НЕ пиши на экран ничего сверх предписанного
заданием. Предписанное ОБЯЗАТЕЛЬНО и не отменяется этой клаузой:
heartbeat'ы `echo "PROGRESS: …"` перед каждым шагом, файл отчёта
`campaigns/docs-2026-09/findings/WORKER-REPORT-LP-O3.md` (решения,
отклонения, вывод самопроверки), финальный `echo "TASK-DONE"`. Запрещено:
приветствия, пересказ задачи, промежуточные рассуждения в чат, финальное
резюме сделанного (оно живёт в отчёте, не в чате).»

## Где ты работаешь

- Дерево `C:\Users\olegc\git\v\vibevm`, `main`, HEAD не старше `75800885a`.
  Git Bash; Write/Edit; UTF-8, LF. **Git только читающий.**
- Параллельно другой исполнитель правит сайт
  (`vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/**`, кроме
  `generated/`, и `design/src/components/**`). Не трогай их; Rust сейчас
  собираешь только ты.

## Зачем

`vibe doc check --chapters --json` (коммит `75800885a`,
`crates/vibe-doc/src/chapters.rs`) печатает машинный документ через
рукописный `#[derive(Serialize)]` на `Report` и `ForwardLink`. Это
VibeVM-овский формат обмена, а закон проекта для таких — зарегистрированная
JTD-схема и codegen (BACKLOG `B-173`: рукописный wire после базовой линии
держит красной панель релиза). Соседний `vibe doc todo --format json` сделан
правильно — повтори его идиому.

## Прочитай первым

1. `BACKLOG.md`, запись `B-173`.
2. Образец: `schemas/doc_todo.jtd.json`, запись `[format.doc-todo]` и её
   комментарий в `formats/REGISTRY.toml`, использование
   `vibe_wire::generated::doc_todo` в `crates/vibe-doc/src/todo.rs` и
   `todo/report.rs`.
3. `crates/vibe-doc/src/chapters.rs` и `chapters/tests.rs`,
   блок `--chapters` в `crates/vibe-cli/src/commands/doc.rs`.
4. `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml`,
   `##NAV-CHAPTERS-CHECKED` — норма отчёта (замер, не гейт).
5. Правила стека Rust:
   `vibevm/vibedeps/org.vibevm.ai-native.rust-ai-native-lang/1.0.0/vibevm/vibespecs/boot/20-stack-rust-ai-native-lang.xml`.

## Что сделать

1. `schemas/doc_chapters.jtd.json` — документ замера той же формы, что
   печатается сейчас (`chapters`, `pages`, `forward_links` из `{page,
   target}`, `forward_link_count`, `unreadable`, `measured`), с описаниями,
   цитирующими `##NAV-CHAPTERS-CHECKED`, и с `x-wire-order`, как в соседних
   схемах. Если соседние машинные документы `vibe doc` несут конверт или
   `schema_version` — сделай так же и опиши выбор.
2. `formats/REGISTRY.toml`: `[format.doc-chapters]` рядом с `doc-todo`, с
   комментарием по образцу (`recoverable = true` — это чтение дерева).
3. `cargo xtask codegen` — только Rust-выход, если реестр позволяет
   (TypeScript сайту этот документ не нужен); сгенерированное руками не
   править.
4. `chapters.rs` строит и печатает сгенерированный тип; рукописные serde-
   derive с `Report`/`ForwardLink` уходят (человеческая форма `render()` и
   поведение — без изменений, байты JSON — те же или объясни разницу).
   Тесты — на сгенерированном типе.
5. Храповик рукописного wire: найди в `tools/self-check.sh` шаг
   wire-derive и прогони его проверку; счёт `vibe-doc` не должен вырасти
   относительно базовой линии из-за `chapters.rs`.

## Периметр

Можно: `schemas/doc_chapters.jtd.json`, `formats/REGISTRY.toml`,
сгенерированное через codegen, `crates/vibe-doc/src/chapters.rs`,
`crates/vibe-doc/src/chapters/tests.rs`, `crates/vibe-cli/src/commands/doc.rs`
и `crates/vibe-cli/src/commands/doc/tests.rs` (только если меняется вывод),
отчёт. Нельзя: сайт, спеки, пакеты руководства, `specmap.json`, базовые
линии храповиков (не поднимать!), остальные крейты. Нужно больше — стоп и
«дефект пакета» в отчёте.

## Самопроверка (вывод и коды выхода — дословно)

```bash
cargo fmt --all -- --check; echo "EXIT=$?"
cargo xtask codegen; echo "EXIT=$?"
cargo test -p vibe-doc chapters; echo "EXIT=$?"
cargo test -p vibe-cli --bin vibe commands::doc::; echo "EXIT=$?"
cargo clippy -p vibe-doc -p vibe-wire --all-targets -- -D warnings; echo "EXIT=$?"
cargo build -p vibe-cli; echo "EXIT=$?"
target/debug/vibe.exe doc check --chapters --json --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0; echo "EXIT=$?"
# + команда проверки храповика wire-derive из tools/self-check.sh
```

`check-codegen` будет красным только из-за незакоммиченной регенерации —
докажи идемпотентность повторным `codegen`, как в отчёте LP-O1.

## Отчёт

`WORKER-REPORT-LP-O3.md`: файлы, форма схемы, запись реестра, счёт
храповика до и после, вывод самопроверки дословно, отклонения. Затем
`echo "TASK-DONE"`.
