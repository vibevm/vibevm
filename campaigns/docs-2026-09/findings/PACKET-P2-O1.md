# PACKET-P2-O1 — виды `doc` и `app`, wire-словарь, поля манифеста (A2.1, A2.2, A2.4)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `campaigns/docs-2026-09/PLAN.md` — только атомы **A2.1**, **A2.2**, **A2.4**
   (раздел «Фаза 2 — механика») и §2.8 «Чеклист нового крейта» (новых крейтов
   в этом пакете нет, но пункты 3–5 действуют для новых модулей).
3. Находки фазы 0: `campaigns/docs-2026-09/findings/A0.1-kind-census.md`
   (перепись `match` по `PackageKind`), `A0.2-wire-kind-vocabulary.md`
   (словарь wire, корпуса, break-заметки), `A0.3-manifest-strict-fields.md`
   (`ManifestWire`, `deny_unknown_fields`, два `TryFrom`),
   `A0.18-i18n-reuse.md` (`I18nDecl`, теги BCP-47).
4. Норма: `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml`,
   секции `kinds` (KIND-*), `companion`, `relation` (REL-*),
   `localization` (LOC-*), `card` (CARD-*); `REL-FIELD-PLACEMENT` — статус
   `spec/work`, рекомендация в силе: таблицы связи и `[media]` — верхний
   уровень манифеста, `title`, `abstract`, `lang` — в `[package]`.
5. Код: `crates/vibe-core/src/package_ref/kind.rs` (+ `tests.rs`),
   `crates/vibe-core/src/manifest/document.rs`, `crates/vibe-core/src/manifest/package/`,
   `formats/vocabularies.json`, `formats/REGISTRY.toml`, `formats/EPOCHS.toml`
   (`public = false`: break-заметка не обязательна, `wire-diff` только сообщает).
6. Репозиторные правила, которые тебя связывают, названы здесь целиком:
   Conventional Commits с телом «почему»; атомарный коммит на атом; **никаких
   трейлеров и упоминаний моделей в коммитах** (авторство человеческое);
   `cargo fmt --all` перед каждым коммитом; `unwrap`/`expect` в доменной
   логике запрещены; сообщения об ошибках цитируют `spec://…`; новая ветка
   `PackageKind` добавляется в **каждый** `match` осмысленной веткой, не
   `_ =>`; wire меняется только через словарь/схему → `cargo xtask codegen`;
   секреты и `infra/` не читать; `git push` не делать.

## Цель

Три атома фазы 2, три коммита, в этом порядке:

**A2.1** `PackageKind::{Doc, App}` с doc-комментариями по PROP-057 `kinds`;
каждый `match` из переписи A0.1 получает ветку с правильным поведением:
`vibe install` пакета вида `doc` отказывает с подсказкой «read it with
`vibe cache add <coord>` and the local reader» и адресом
`spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-NOT-INSTALLED`;
`app` ведёт себя как `tool` там, где спека не различает, и отказывает в
`vibe bin exec` с адресом `#KIND-APP-VS-TOOL`. Тесты на каждое новое
поведение. Коммит: `feat(core): teach PackageKind the doc and app kinds`.

**A2.2** `formats/vocabularies.json` → `package_kind.enum` += `doc`, `app`;
`cargo xtask codegen`; корпуса с `kind` из A0.2 пополнены; `cargo xtask
check-codegen`, `cargo xtask wire-diff` — прочитать вердикт, break-заметка
`formats/breaks/005.md` только если `wire-diff` её требует. Коммит:
`feat(wire): widen package_kind vocabulary for doc and app`.

**A2.4** Поля манифеста, ровно как в PLAN A2.4: `documents`,
`documentation`, `title`, `abstract` (≤ 1000 знаков), `lang` (BCP-47,
default `en`), `translates`, `translations`, `media`; обязательность и
запреты по виду; ошибки цитируют якоря PROP-057 (`REL-DOCUMENTS-REQUIRED`, `CARD-TITLE`,
`CARD-DESCRIPTION-AND-ABSTRACT`, `LOC-LANGUAGE-FIELD`, `LOC-NO-TRANSLATIONS-TABLE`,
`CARD-MEDIA-SOURCE` — имена проверены);
тесты парсинга и отказов; `vibe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0`
на манифесте руководства должен пройти (этот манифест — эталон полей).
Коммит: `feat(core): carry documentation relations, locales and the card in the manifest`.

## Гейты каждого атома

`cargo fmt --all --check`, `cargo build --workspace`, `cargo test -p vibe-core`
(и крейты, которые атом трогает), `cargo clippy --workspace --all-targets --
-D warnings` по тронутым крейтам, `cargo xtask specmap` если появились новые
`specmark`-теги (ноль suspects). Полную панель не гонять — это делает
центральная сессия.

## Что не делать

Не трогать страницы `vibevm/vibepacks/org.vibevm.core/vibevm-docs/**`, зону
кампании (кроме своего отчёта), `VIBEVM-SPEC.md`, PROP-файлы. Не менять
`wire-derive-baseline.json`. Не открывать других атомов фазы 2. Если атом
упирается в решение, которого нет ни в плане, ни в PROP-057, — остановиться
на нём, описать развилку с рекомендацией и отдать следующий атом, если он не
зависит.

## Результат

Три коммита в ветке `research-preview-1-docs` (без push) и отчёт
`campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O1.md`: хэши и subject'ы;
что сделано по каждому атому (файлы, решения, развилки); вывод гейтов
дословно; что не сделано и почему; аномалии продукта, найденные по пути,
списком без правок; `git status --short` в конце (должен быть чист, кроме
файлов других воркеров, которые были до тебя).
