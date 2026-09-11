# Пакет P1-O3: манифест дистрибуции как JTD-схема (закрыть ratchet derive в `vibe-publish`)

Читать сначала, ровно эти файлы: `campaigns/docs-2026-09/findings/PACKET-COMMON.md`
(преамбул; ##subagent-quiet-clause действует), затем этот пакет, затем
`campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O2.md` §1 (классификация,
строка `release_manifest.rs` — класс (а)). Правки файлов репозитория разрешены
ровно в периметре ниже; git не трогать (только читающие команды); scratch —
`<scratch>\vibe-docs-phase1\P1-O3\`; отчёт —
`campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O3.md`. Центральная сессия
параллельно правит `vibevm/vibepacks/org.vibevm.core/vibevm-docs/**` и
`campaigns/**` — не трогать и не сообщать как аномалию.

## Что случилось

Шаг «wire-derive ratchet» панели `tools/self-check.sh` остаётся красным для
одного крейта: `vibe-publish` — 8 файлов с рукописными
`#[derive(Serialize/Deserialize)]` против 7 в `wire-derive-baseline.json`.
Восьмой — `crates/vibe-publish/src/release_manifest.rs` (типы
`DistributionComponentName`, `DistributionComponent`,
`DistributionSourceArchive`, `BundleDistributionManifest`,
`DistributionAsset`, `PlatformDistributionFragment`,
`AggregateDistributionManifest`): это **наш wire** — `DISTRIBUTION.json` и
`DISTRIBUTIONS.json` публикуются как релизные активы и читаются скриптами
`distribution/install/install.sh` и `install.ps1` и командой `vibe self`.
Лечение по правилу шага: описать формат JTD-схемой, сгенерировать типы
`cargo xtask codegen`, перевести код на сгенерированные типы. Поднимать
базовую линию для wire нельзя.

## Периметр файлов

Создать: одну или две схемы в `schemas/` (по образцу соседних `*.jtd.json`),
запись формата в `formats/REGISTRY.toml` (по образцу соседних записей; закон
PROP-044 §6 — `formats/EPOCHS.toml` держит `public = false`,
`break_window_open = true`, break-заметка не обязательна, но одна короткая
`formats/breaks/NNN.md` по образцу соседних приветствуется, если соседи её
пишут для новых форматов).
Править: `crates/vibe-publish/src/release_manifest.rs` (рукописные derive
уходят; типы либо становятся `pub use` из `vibe_wire::generated::…`, либо
тонкими обёртками без собственных derive), `crates/vibe-wire/src/generated/**`
(**только** через `cargo xtask codegen`), потребители типов в `vibe-publish`,
`vibe-cli` (`self`/`vvm`) и `xtask/src/dist/**` — ровно настолько, чтобы
компилировалось; `wire-derive-baseline.json` — **не трогать**.
Не менять: байтовую форму `DISTRIBUTION.json`/`DISTRIBUTIONS.json` (порядок
полей, имена, `deny_unknown_fields`, `schema_version = 1`), скрипты
`distribution/install/*`, поведение `vibe self install/update/bootstrap`.

## Сделать

1. Прочитать `release_manifest.rs` целиком и найти все места, где эти типы
   сериализуются или десериализуются (`grep -rn "DistributionAsset\|AggregateDistributionManifest\|PlatformDistributionFragment\|BundleDistributionManifest" crates xtask`).
   Снять **голдены до правки**: в scratch собрать по одному образцу каждого
   JSON-документа существующими тестами или из `xtask/src/dist` (если тесты
   уже держат образцы — использовать их).
2. Написать JTD-схему(ы) по фактической форме документов (RFC 8927: `properties`,
   `optionalProperties`, `enum`, `elements`, `values`; `additionalProperties`
   не включать, чтобы сохранить `deny_unknown_fields`); посмотреть, как
   соседние схемы выражают `#[serde(rename_all)]` и переименования, и как
   `formats/vocabularies.json` описывает enum-словари, если
   `DistributionComponentName` — enum.
3. `cargo xtask codegen`, затем `cargo xtask check-codegen` — зелёный.
4. Перевести `release_manifest.rs` на сгенерированные типы; сохранить
   публичный API крейта (имена типов, методы), например через `pub use` и
   `impl` блоки; если сгенерированный тип не может нести метод — обёртка без
   derive. Никаких `_ =>`, никаких изменений поведения.
5. `cargo fmt --all`, `cargo build -p vibe-publish -p vibe-cli -p xtask`,
   `cargo clippy -p vibe-publish -p vibe-cli --all-targets -- -D warnings`,
   `cargo test -p vibe-publish -p vibe-cli --quiet`, `cargo xtask wire-diff`
   (читать как данные, приложить вывод).
6. Голдены после правки — побайтово те же, что до (шаг 1); приложить `diff`.
7. `bash tools/self-check.sh` до первого падения: шаг ratchet должен быть
   зелёным. Если падает дальше — записать шаг и вывод, не чинить.

## Самопроверка (обязательно, вывод в отчёт)

- `cargo xtask check-codegen; echo EXIT=$?`
- `cargo xtask wire-diff 2>&1 | tail -20`
- `cargo test -p vibe-publish -p vibe-cli --quiet 2>&1 | tail -5`
- `bash tools/self-check.sh 2>&1 | grep -E "ratchet|failed|passed" | head`
- `git status --short` (свои файлы перечислить; чужие не трогать)

## Приёмка боссом

Дифф читается как чужой PR; байтовая форма релизных манифестов не изменилась
(голдены); ни одной рукописной derive в `release_manifest.rs`; схемы
зарегистрированы; гейты зелёные. Коммитит центральная сессия.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O3.md`: схемы, решения по
форме, голдены до/после, выводы самопроверки, что не сделано и почему. Затем
`echo "TASK-DONE"`.
