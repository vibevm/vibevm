# Пакет P1-O1: импорт вижена как design-документа в диалекте XML (атом A1.9)

Читать сначала, ровно эти файлы: `campaigns/docs-2026-09/findings/PACKET-COMMON.md`
(преамбул; ##subagent-quiet-clause действует), затем этот пакет. Отличия от
преамбула фазы 0: **правки файлов репозитория разрешены ровно в периметре
ниже**; git по-прежнему не трогать (никаких add/commit/stash/checkout/restore);
scratch — `C:\Users\olegc\AppData\Local\Temp\claude\vibe-docs-phase1\P1-O1\`;
отчёт и находка пишутся в `campaigns/docs-2026-09/findings/`.

## Периметр файлов

Создать: `vibevm/vibespecs/design/documentation-vision.xml` (или, при отказе
конвертера, `documentation-vision.md` — причина в отчёт).
Править: `vibevm/vibespecs/design/README.md` — одна строка в списке-индексе
(см. шаг 5). Больше ничего в репозитории не трогать. Центральная сессия
параллельно правит другие файлы дерева — это ожидаемо, не сообщать как
аномалию.

## Сделать

1. Скопировать `C:\Users\olegc\git\v\vibe-docs-vision\VISION.md` в scratch.
   Удалить целиком абзац, начинающийся с `**Провенанс.**` (до пустой строки).
   Больше ничего в тексте не менять: русский язык, цитаты владельца, все
   разделы остаются.
2. Сконвертировать в диалект XML пивотом продукта:
   `target/debug/vibe.exe refactor convert-source --from md --to xml <файл-в-scratch>`
   (сначала `--dry-run`, прочитать отчёт классификации; `--force` — только
   если конвертер называет потери IR-стабильными, и тогда перечислить их в
   отчёте). Использовать готовый `target/debug/vibe.exe`, не `cargo run`.
   Если конвертер отказывает (IR divergence), оставить Markdown и записать
   дословный отказ в отчёт — тогда шаги 3–4 делаются над Markdown в
   грамматике `@fact:ID … @status:stage/state` (образец —
   `vibevm/vibespecs/design/README.md`).
3. Разметить **каждый юнит** документа фактом так, чтобы
   `target/debug/vibe.exe facts check --exhaustive` был зелёным по этому
   файлу: абзацы, пункты списков, ячейки таблиц (кроме заголовочной строки),
   цитаты. Образец XML-формы — `vibevm/vibespecs/design/host-as-package.xml`
   (`<p><name fact="true" status="…">…</name></p>`, группы `<facts>`, ячейки
   `<td><name fact="true" …>…</name></td>`). Схема идентификаторов —
   детерминированная, скриптом (скрипт положить в scratch и приложить к
   отчёту):
   - решения `D-NN`: абзац «**Решение.**» → `D-NN-DECISION`, «**Почему.**» →
     `d-nn-why`, «**Отвергнуто.**» → `d-nn-rejected`, «**Пересмотреть
     когда.**» → `d-nn-revisit`, «**Известное ограничение.**» → `d-nn-limit`,
     остальные юниты секции → `d-nn-<порядковый номер>`;
   - принципы `P-NN` → `P-NN`; цитаты мандата §3 → `mandate-<порядковый>`;
   - остальные секции → `<якорь секции>-<порядковый номер>` (якорь секции —
     её `{#id}`; вложенность через дефис не нужна, только якорь ближайшей
     секции); идентификаторы в нижнем регистре кроме перечисленных
     нормативных; допустимые символы — буквы ASCII, цифры, дефис; не
     начинать с цифры и с `xml`.
   - статус у всех фактов `spec/done`; у документа `<status stage="spec"
     state="done" comment="…">` с текстом: `design rationale behind PROP-057
     — the campaign vision, twelve editions 2026-09-09..11; imported 2026-09-11;
     non-normative, PROP-057 wins where they disagree`.
   - в самом начале документа, сразу после `<status>`, добавить абзац
     `<p><companion-line fact="true" status="spec/done">**Explains:**
     [PROP-057](../common/PROP-057-documentation-packages-and-site.xml) — the
     documentation packages and site contract; the thirty decision records
     D-01…D-30 below are its rationale, and PROP-057 wins where they disagree
     (the spec-genres precedence law). Written in Russian for the owner; the
     norm it explains is English.</companion-line></p>`.
   Если факт-идентификатор конфликтует с именем секции или зарезервированным
   словарём (`spec, title, status, section, p, fact, list, item, table, tr,
   td, fence, quote, facts`) — использовать общую форму `<fact id="…">`.
4. Проверить: `target/debug/vibe.exe facts check --exhaustive` (весь проект;
   в отчёт — строки, относящиеся к новому файлу, и итоговую строку) и
   `target/debug/vibe.exe check --path . --quiet`. Оба должны быть зелёными
   по новому файлу; чужие красные строки, если есть, перечислить в отчёте
   без исправления.
5. В `vibevm/vibespecs/design/README.md` в список-индекс (в конце файла,
   рядом с `idx-tooling-map`) добавить одну строку по образцу соседних:
   `- @fact:idx-documentation-vision [The documentation vision](documentation-vision.xml)
   — the rationale behind PROP-057: thirty decision records (D-01…D-30) on
   documentation as packages, the site, the local reader, localization,
   style and maintenance; written in Russian for the owner; imported
   2026-09-11 from the docs-2026-09 campaign; PROP-057 wins where they
   disagree. @status:spec/done`.

## Самопроверка (обязательно, вывод в отчёт)

- `target/debug/vibe.exe facts check --exhaustive; echo "EXIT=$?"`
- `target/debug/vibe.exe check --path . --quiet; echo "EXIT=$?"`
- `python -c "import xml.etree.ElementTree as ET; ET.parse('vibevm/vibespecs/design/documentation-vision.xml'); print('well-formed')"`
- число фактов в файле и число юнитов без факта (должно быть 0).

## Приёмка боссом

Дифф читается как чужой PR; провенанс-абзаца нет; ни одного упоминания
инструмента-соавтора; все D-NN присутствуют (30 секций `d-01`…`d-30`);
гейты зелёные; коммитит центральная сессия.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O1.md`: команды и их вывод,
классификация конвертера, отклонения, что не сделано и почему. Затем
`echo "TASK-DONE"`.
