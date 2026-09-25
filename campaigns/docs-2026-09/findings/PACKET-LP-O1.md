# Пакет LP-O1 — учебный путь: wire, разбор, проверки (opus5, High) — 2026-09-25

##subagent-quiet-clause
«Ты работаешь в СУБАГЕНТСКОМ режиме: твой экранный текст не читает никто,
деливерабл — только артефакты. НЕ пиши на экран ничего сверх предписанного
заданием. Предписанное ОБЯЗАТЕЛЬНО и не отменяется этой клаузой:
heartbeat'ы `echo "PROGRESS: …"` перед каждым шагом, файл отчёта
`campaigns/docs-2026-09/findings/WORKER-REPORT-LP-O1.md` (решения,
отклонения, вывод самопроверки), финальный `echo "TASK-DONE"`. Запрещено:
приветствия, пересказ задачи, промежуточные рассуждения в чат, финальное
резюме сделанного (оно живёт в отчёте, не в чате).»

## Где ты работаешь

- Дерево: `C:\Users\olegc\git\v\vibevm`, ветка `main`, HEAD `b7c28f078`. Пути
  ниже — от этого корня. Оболочка — Git Bash; файлы пиши инструментами
  Write/Edit (PowerShell 5.1 портит UTF-8 без BOM); UTF-8, LF.
- **Git только читающий** (`status`, `diff`, `log`, `show`). Никаких `add`,
  `commit`, `stash`, `checkout`, `restore`, `reset`: ревью и коммиты делает
  центральная сессия.
- Параллельно центральная сессия правит пакеты руководства
  `vibevm/vibepacks/org.vibevm.core/vibevm-docs*/**`. Их не трогай и не
  используй как фикстуры своих тестов.
- `target/` общий и тёплый; ты единственный, кто сейчас собирает Rust.

## Прочитай первым (и только это из правил)

1. `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml` —
   раздел `9.4 The reader`: факты `NAV-PINNED`, `NAV-CHAPTERS`,
   `NAV-CHAPTERS-CHECKED`, `NAV-CHAPTERS-TRANSLATION`, `NAV-CHAPTERS-READER`,
   `READER-SETTINGS` и запись решения `NAV-CHAPTERS-DECISION` с тремя полями.
   Это норма, которую ты реализуешь; она не меняется.
2. `vibevm/vibespecs/common/PROP-048-tokenomics.xml`, `##THE-LAYER-LAW` —
   почему порядок `pages` трогать нельзя.
3. `campaigns/docs-2026-09/findings/LEARNING-PATH-DESIGN.md` §2–§4 — факты и
   развилки, принятые владельцем.
4. Правила стека Rust:
   `vibevm/vibedeps/org.vibevm.ai-native.rust-ai-native-lang/1.0.0/vibevm/vibespecs/boot/20-stack-rust-ai-native-lang.xml`;
   стека TypeScript:
   `vibevm/vibedeps/org.vibevm.ai-native.typescript-ai-native-lang/1.0.0/vibevm/vibespecs/boot/20-stack-typescript-ai-native-lang.xml`.
5. Соседний код, который ты расширяешь (идиома — его, не новая): разбор
   `[navigation]` и проверки `pinned` по адресам ниже.

## Что сделать

Манифест пакета документации получает `[[navigation.chapter]]`:

```toml
[[navigation.chapter]]
id = "start"                 # непустой, уникален среди глав
title = "Getting started"    # непустой
pages = ["start/what-vibevm-is", "start/index"]   # пути документов без .xml, как в pinned
# appendix = true            # необязательно; глава справочных страниц
```

1. **Wire.** `schemas/doc_manifest.jtd.json`: в `navigation` необязательное
   поле `chapters` — массив нового определения `navigation_chapter`
   (`id`, `title`, `pages`; необязательное `appendix`), с `x-wire-order` и
   описаниями, цитирующими `##NAV-CHAPTERS`. `chapters` выдаётся, только
   когда пакет его объявил; `appendix` — только когда `true`. Затем
   `cargo xtask codegen` (перегенерирует
   `crates/vibe-wire/src/generated/doc_manifest/mod.rs` и
   `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/generated/doc-manifest.ts`;
   руками их не править).
2. **Модель и валидация манифеста (`vibe-core`).**
   `crates/vibe-core/src/manifest/document.rs` (поле navigation, строка ~193),
   `crates/vibe-core/src/manifest/document/validation.rs` (проверки
   navigation, строки ~418–455), при нужде
   `crates/vibe-core/src/manifest/package/documentation.rs`. Отказывать с
   сообщением, цитирующим `spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED`
   (или `#NAV-CHAPTERS-TRANSLATION` для перевода), как соседние отказы
   цитируют `#NAV-PINNED`: пустой `id`/`title`; `id` дважды; одна страница
   дважды в объявлении; `pages` не массив строк; `appendix` не булево;
   в переводе (есть `[translates]`) строка главы с `pages`. В переводе строки
   `[[navigation.chapter]]` законны с `id` и `title` без `pages`.
3. **Покрытие по дереву (`vibe check`).**
   `crates/vibe-check/src/checks/doc_package_contract.rs` (рядом с проверкой
   `pinned`, строка ~161): для исходного издания, объявившего главы, —
   страница главы, которой нет в пакете, и страница пакета, не вошедшая ни
   в одну главу, — ошибки со ссылкой на `#NAV-CHAPTERS-CHECKED`. Пакет без
   глав не проверяется этим правилом вовсе.
4. **Перевод (`vibe doc check --translations`).**
   `crates/vibe-doc/src/translations.rs`: строка главы перевода, чей `id`
   источник не объявляет, — ошибка со ссылкой на `#NAV-CHAPTERS-TRANSLATION`.
5. **Манифест страниц (`vibe-doc`).** `crates/vibe-doc/src/manifest.rs`:
   `navigation()` (строки ~499–533) читает главы и переносит их как есть;
   `layer::order` и порядок `pages` не меняются. Комментарий у строк
   ~239–245 («A projection that also reordered the list would give the
   site two orders…») переписать: второй порядок теперь объявленный и
   намеренный, `pages` по-прежнему закон слоёв (`##NAV-CHAPTERS`).
6. **Замер ссылок вперёд (`vibe doc check`).** Когда пакет объявил главы,
   `vibe doc check` в обычном прогоне печатает отчёт: каждая ссылка со
   страницы на другую страницу этого пакета, которая на пути стоит позже,
   кроме ссылок в главы с `appendix = true`; ссылки на якоря той же страницы
   и на глоссарий в приложении не считаются. Отчёт **не меняет код выхода**
   (`##NAV-CHAPTERS-CHECKED`); в `--json` — отдельное поле с парами
   «страница → цель» и счётчиком. Разбор ссылок — существующей моделью
   конвейера (ссылки в страницах — Markdown `[текст](../раздел/страница.xml#якорь)`
   и относительные в той же папке), не новым регэкспом, если модель есть.
   Имя селектора и место в выводе — по конвенции соседних отчётов
   `vibe doc check`; опиши выбор в отчёте.
7. **TS-парсер сайта.**
   `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/lib/manifest.ts`,
   функция `navigation()` (~477–494): читать необязательное `chapters` с той
   же строгостью, что `sections` (ошибка — `{ok:false, error}`, не
   исключение), и юнит-тест рядом с существующими тестами парсера.
8. **Теги прослеживаемости.** Новые публичные элементы в gated-крейтах
   помечай так же, как помечены соседние (`#[spec…]`/scope-марки на
   `NAV-CHAPTERS*`); `specmap.json` **не** перегенерировать.

## Периметр

Можно менять: `schemas/doc_manifest.jtd.json`; сгенерированные файлы — только
через `cargo xtask codegen`; `crates/vibe-core/src/manifest/**`;
`crates/vibe-check/src/checks/doc_package_contract*`,
`crates/vibe-check/src/checks/doc_translation*`; `crates/vibe-doc/src/**`,
`crates/vibe-doc/tests/**`; `crates/vibe-cli/src/cli/doc.rs` и
`crates/vibe-cli/src/commands/doc*` — только если отчёту нужен флаг или
вывод; `…/web/v1.0.0/site/src/lib/manifest.ts` и его тест; отчёт
`campaigns/docs-2026-09/findings/WORKER-REPORT-LP-O1.md`.

Нельзя: `vibevm/vibespecs/**`, пакеты руководства, компоненты и маршруты
сайта, фикстуры сайта (`site/src/fixtures/**`), `specmap.json`,
`facts.toml`, любые файлы вне периметра. Если без них не обойтись — остановись
и опиши в отчёте как дефект пакета.

## Тесты (обязательны, по соседству с существующими)

- `vibe-core`: каждый отказ из п.2 и законная строка перевода без `pages`.
- `vibe-check`: страница главы вне пакета; страница пакета вне глав;
  пакет без глав не задет.
- `vibe-doc`: главы переносятся в манифест, порядок `pages` не изменился;
  неизвестный `id` главы перевода — ошибка; отчёт ссылок вперёд: ссылка
  назад не считается, ссылка вперёд считается, ссылка в `appendix` не
  считается, код выхода не меняется.
- TS: `chapters` разбирается; неверная форма — ошибка с путём поля.

## Самопроверка (вывод и коды выхода — в отчёт дословно)

```bash
cargo fmt --all -- --check; echo "EXIT=$?"
cargo xtask check-codegen; echo "EXIT=$?"
cargo test -p vibe-core --lib manifest; echo "EXIT=$?"
cargo test -p vibe-check doc_; echo "EXIT=$?"
cargo test -p vibe-doc; echo "EXIT=$?"
cargo clippy -p vibe-core -p vibe-check -p vibe-doc --all-targets -- -D warnings; echo "EXIT=$?"
cargo build -p vibe-cli; echo "EXIT=$?"
cargo xtask specmap --check; echo "EXIT=$?"
cd vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 && TYPESCRIPT_AI_NATIVE="C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe" node tools/floor.mjs --keep-going; echo "EXIT=$?"
```

`specmap --check` уже красный до тебя: `unbumped-hash` у
`PROP-019#release-production`, добавленные единицы `PROP-059#binary-selection`,
`#mode-transition` и восемь `PROP-057#NAV-CHAPTERS*`/`nav-chapters-*`,
«сироты» `vibe_cli::cli::run::RunArgs` и
`vibe_cli::commands::application::distribution::codec::source_differs`. Твоя
задача — не добавить **новых** сирот и новых дрейфов, кроме твоих рёбер к
`NAV-CHAPTERS*`; перечисли в отчёте всё, что он печатает. Если тестовый
фильтр выбрал ноль тестов — это не зелёный прогон: подбери фильтр,
выбирающий твои тесты, и напиши какой.

## Отчёт

`WORKER-REPORT-LP-O1.md`: файлы; семантика как реализована (сообщения
отказов дословно, имя и форма отчёта ссылок вперёд, поле JSON); решения и
отклонения; вывод самопроверки дословно; что не сделано и почему. Затем
`echo "TASK-DONE"`.
