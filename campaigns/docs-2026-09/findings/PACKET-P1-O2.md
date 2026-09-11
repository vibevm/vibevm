# Пакет P1-O2: унаследованные красные гейты — wire-derive ratchet и пять сирот карты

Читать сначала, ровно эти файлы: `campaigns/docs-2026-09/findings/PACKET-COMMON.md`
(преамбул; ##subagent-quiet-clause действует), затем этот пакет. Отличия от
преамбула фазы 0: правки файлов репозитория разрешены ровно в периметре
ниже; git по-прежнему не трогать (никаких add/commit/stash/checkout/restore;
читающие `git log`, `git diff`, `git status`, `git blame` можно); scratch —
`<scratch>\vibe-docs-phase1\P1-O2\`; отчёт и
находка — в `campaigns/docs-2026-09/findings/`. Центральная сессия параллельно
правит другие файлы дерева — это ожидаемо. Ещё один воркер параллельно
пишет `vibevm/vibespecs/design/documentation-vision.xml` и одну строку в
`vibevm/vibespecs/design/README.md` — не трогать.

## Что случилось

На ветке `research-preview-1-docs` (от `main@b1291b06`) кампания документации
не меняла ни одного `.rs`-файла, но два гейта красные унаследованно:

1. `bash tools/self-check.sh` падает на шаге «wire-derive ratchet»: число
   файлов с рукописными `#[derive(Serialize/Deserialize)]` превышает
   замороженное в `wire-derive-baseline.json` — `vibe-publish` (см. лог),
   `vibe-registry` 8 против 7, `vibe-workspace` 4 против 3, `xtask` 14
   против 12. Лог шага: `<scratch>/…/self-check.txt` не нужен — воспроизведи
   сам.
2. `cargo xtask specmap --check` падает на ratchet сирот: пять публичных
   элементов в гейтуемых крейтах без метки спеки —
   `crates/vibe-core/src/manifest/package/embedded_source.rs:9,17,27`
   (`EmbeddedSourceKind`, `EmbeddedSourceAuth`, `EmbeddedSourceDecl`) и
   `crates/vibe-publish/src/git_publish/submodules.rs:10,19`
   (`SubmoduleProvenance`, `inspect`). Все пять пришли коммитами кампании
   bridge-пакетов (`72d866d2`, `4efad426`, `7b465809`), спека которой —
   `vibevm/vibespecs/modules/vibe-registry/PROP-023-bridge-packages.xml`
   (секции `classes` 2.2, `maintainer-model` 2.4) и
   `PROP-021-submodule-sources.xml` (`fetch`, `source-abstraction`).

## Периметр файлов

Править: `wire-derive-baseline.json` (там, где он лежит — найди `find . -name
wire-derive-baseline.json -not -path "*/target/*"`), два файла с сиротами выше
(только добавление меток спеки), `specmap.json` (только через `cargo xtask
specmap`). Ничего больше. Если для меток нужен `use` макроса — по образцу
соседних файлов того же крейта.

## Сделать

1. **Ratchet derive.** Воспроизвести шаг: прочитать заголовок
   `tools/self-check.sh` (первые ~120 строк, там описаны шаги и флаги) и
   запустить либо один шаг, если скрипт это умеет, либо весь скрипт до первого
   падения (шаг третий, секунды). Для каждого крейта сверх базовой линии
   перечислить файлы с рукописными derive (сам шаг печатает или считает их —
   найди, как он считает, и повтори подсчёт руками), найти, какие файлы
   **новые** относительно момента заморозки (`git log --oneline --
   <baseline>` даёт коммит заморозки; `git log --diff-filter=A -- <файл>` —
   когда файл появился). Каждый новый тип классифицировать по правилу из
   сообщения шага: (а) **наш wire** — формат, который читает или пишет другой
   процесс/язык (тогда правильное лечение — JTD-схема и `cargo xtask codegen`,
   **этого не делать**, только записать в отчёт как долг с адресом типа);
   (б) **не wire** — конфигурация, CLI-локальная структура, чужой формат
   (тогда поднять счётчик крейта в базовой линии и написать для тела коммита
   одну строку на тип: `<крейт>: <тип> в <файл> — <что это>`). Если хотя бы
   один тип — (а), базовую линию для него всё равно **не** поднимать; отчёт
   скажет, что шаг остаётся красным по этой причине.
2. **Сироты карты.** Для каждого из пяти элементов выбрать якорь спеки,
   который его описывает (читать PROP-023 §2.2, §2.4 и PROP-021; если
   элемент описан в другом месте — найти по `grep -rn "<имя>" vibevm/vibespecs`),
   и поставить метку так, как это делают соседние тэгированные элементы того
   же крейта (посмотри `grep -rn "spec(" crates/vibe-core/src/manifest/package/*.rs`
   и `crates/vibe-publish/src/git_publish/*.rs`, включая `specmark::scope!`).
   Якорь должен существовать (`cargo xtask specmap --check` резолвит хостовые
   рёбра). Если для элемента нет подходящего якоря — не выдумывать: оставить
   без метки и записать в отчёт кандидат-раздел, куда факт надо бы добавить.
3. `cargo build -p vibe-core -p vibe-publish`, `cargo fmt --all -- --check`,
   `cargo clippy -p vibe-core -p vibe-publish --all-targets -- -D warnings`,
   `cargo test -p vibe-core -p vibe-publish --quiet`, затем
   `cargo xtask specmap` и `cargo xtask specmap --check` (нужно: `clean`, `0
   orphan(s)` или меньше пяти с объяснением каждого оставшегося).
4. Повторно прогнать шаг ratchet derive (или скрипт до первого падения) и
   приложить вывод.

## Самопроверка (обязательно, вывод в отчёт)

- `cargo xtask specmap --check 2>&1 | tail -8`
- шаг ratchet derive (как в п. 1) — вывод целиком
- `cargo fmt --all -- --check; echo EXIT=$?`
- `git status --short` (ожидаемо: базовая линия, два `.rs`, `specmap.json`,
  плюс чужие изменения других воркеров — их перечислить, не трогать)

## Приёмка боссом

Дифф читается как чужой PR; каждая поднятая цифра базовой линии имеет строку
для тела коммита; каждая метка спеки указывает на существующий якорь, чей
текст действительно описывает элемент; ни одного `_ =>`, ни одной правки
поведения кода. Коммитит центральная сессия.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O2.md`: таблица типов с
классификацией и строками для коммита, таблица сирот с якорями, выводы
самопроверки, что не сделано и почему. Затем `echo "TASK-DONE"`.
