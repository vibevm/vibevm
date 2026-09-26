# Пакет GL-O1 — глоссарий как сущность документации и карточки терминов (opus5, High) — 2026-09-26

##subagent-quiet-clause
«Ты работаешь в СУБАГЕНТСКОМ режиме: твой экранный текст не читает никто,
деливерабл — только артефакты. НЕ пиши на экран ничего сверх предписанного
заданием. Предписанное ОБЯЗАТЕЛЬНО и не отменяется этой клаузой:
heartbeat'ы `echo "PROGRESS: …"` перед каждым шагом, файл отчёта
`campaigns/docs-2026-09/findings/WORKER-REPORT-GL-O1.md` (решения,
отклонения, вывод самопроверки), финальный `echo "TASK-DONE"`. Запрещено:
приветствия, пересказ задачи, промежуточные рассуждения в чат, финальное
резюме сделанного (оно живёт в отчёте, не в чате).»

## Где ты работаешь

- Дерево `C:\Users\olegc\git\v\vibevm`, ветка `main`, HEAD не старше
  `4219d77fb` (включает свёрнутые цитаты, коммит `3f18d825c`). Git Bash;
  Write/Edit; UTF-8, LF. **Git только читающий.**
- Пакет сайта: `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/` (ниже — `web/`).
- Параллельно в другой сессии правят заимствованные примеры переводов
  (`BACKLOG B-176`, чтение исходного издания в сборке `vibe-doc`). Если твоя
  правка пересекается с `build.rs`/`content.rs`, держи её минимальной и
  опиши в отчёте, что и где ты тронул.

## Зачем

Владелец: на десктопе при наведении на ссылку-термин глоссария — всплывающая
карточка с определением (только в десктопной вёрстке, обязательно). И: пусть
глоссарий станет сущностью документации, чтобы фичей пользовалась любая
документация, у которой он есть. Сейчас глоссарий известен конвейеру только
как путь-константа `GLOSSARY_PAGE = "glossary/index.xml"`
(`crates/vibe-doc/src/style/glossary.rs:34`).

## Прочитай первым

1. `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml`,
   §9.4: `##GLOSSARY-DECLARED`, `##GLOSSARY-CHECKED`,
   `##GLOSSARY-TRANSLATION`, `##READER-GLOSSARY-CARD`, запись решения
   `##GLOSSARY-CARD-DECISION` (+ why/rejected/revisit); рядом
   `##READER-RULE-FOLDED` (как сделан предыдущий пакет), `##NAV-CHAPTERS*`
   (как объявлялась прошлая сущность манифеста), `##PIPE-SHELL-PARSES-NOTHING`,
   `##STYLE-LINT`. Норма не меняется.
2. Образец сквозной фичи манифеста — учебный путь (коммит `75800885a`):
   грамматика манифеста в `vibe-core`, проверка в `vibe-check`
   (`doc_package_contract`), перевод в `vibe-doc/src/translations.rs`.
3. `crates/vibe-doc/src/style/glossary.rs` (как сейчас читаются статьи
   глоссария), `html.rs`, `html/inline.rs`, `html/links.rs` (как ссылка
   страницы разрешается в документ и фрагмент), `content.rs`, `build.rs`.
4. Сайт: `web/site/src/reader/toc.ts` (класс `has-sidebar` — это и есть
   «десктопная вёрстка»: ширина окна от 1100 px и колонка оглавления рядом с
   текстом), `web/site/src/reader/rules.ts` (образец всплывающей панели,
   позиционирование, Escape, клик вне), `mount.ts`, `dom.ts`, стили
   `web/design/src/components/prose/styles.css` и
   `web/design/src/components/rule-panel/`, фикстуры
   `web/site/src/fixtures/README.md` (и потолок CSP: фикстурная библиотека
   не может вырасти на страницу).
5. Правила стеков Rust и TypeScript (пути — в `vibevm/vibedeps/…/boot/20-stack-*.xml`).

## Что сделать

### 0. Доводка описаний свёрнутых цитат (мелкое, первым)

Описание свёрнутой цитаты (`crates/vibe-doc/src/html/rule.rs`, функция
описания) на настоящем руководстве иногда начинается с номера раздела
(`6.2 vibe.toml is the most expensive…`, `8.1 The ABI is C +…`) или несёт
якорную разметку заголовка (`REQ {#provenance-edit}. From the provenance
view…`). Убирай из описания ведущий номер раздела (цифры с точками и
пробел после них) и токены `{#…}`; всё остальное — как есть. Юнит-тесты на
оба случая; число цитат с описанием и с общей подписью на руководстве — в
отчёт до и после.

### 1. Сущность «глоссарий» (Rust)

- `vibe-core`: грамматика манифеста пакета `kind = "doc"` — таблица
  верхнего уровня `[glossary]` с `page` (путь документа без расширения, как
  у `pinned` и глав). Неизвестные поля — ошибка, как у соседей.
- `vibe check` (`##GLOSSARY-CHECKED`): страница существует; каждая статья
  (раздел верхнего уровня страницы) имеет `title` и начинается с абзаца.
- `vibe doc check --translations` (`##GLOSSARY-TRANSLATION`): перевод без
  `[glossary]` при объявленном у источника или с другой страницей — проблема
  отчёта.
- Линтер стиля: вместо константы — объявленный глоссарий; без объявления
  проверок терминов нет. Константу `GLOSSARY_PAGE` убери, если больше нигде
  не нужна.
- Нужен ли `[glossary]` в JTD-проводе манифеста страниц
  (`schemas/doc_manifest.jtd.json`)? Только если сайту он действительно
  нужен; реши и объясни в отчёте.

### 2. Остров (Rust, HTML)

- Ссылка на статью объявленного глоссария (цель — страница глоссария,
  фрагмент — id существующей статьи) получает `data-gloss="<id>"` и
  `aria-describedby="gloss-<id>"`; остальное в ссылке без изменений.
- В конце острова — один скрытый блок с определениями терминов, на которые
  ссылается ЭТА страница (без повторов, в порядке первого появления):

  ```html
  <aside class="gloss-defs" hidden data-gloss-defs>
    <div class="gloss-def" id="gloss-manifest" data-gloss="manifest">
      <p class="gloss-def__term">manifest</p>
      <p class="gloss-def__text">…первый абзац статьи, inline-разметка и ссылки разрешены как в теле страницы…</p>
    </div>
  </aside>
  ```

  Ссылки внутри определения разрешаются относительно ТЕКУЩЕЙ страницы (как
  всё в её острове). Не нумеруется (не блок потока).
- На самой странице глоссария — ни атрибутов, ни блока. Без объявленного
  глоссария — ничего.
- `.md`, `.xml`, `llms*` — байт в байт как были.
- Тег нормы на новом коде — в идиоме крейта (`##GLOSSARY-*`,
  `##READER-GLOSSARY-CARD`). Юнит- и golden-тесты.

### 3. Карточка (сайт)

- Модуль `reader/glossary-card.ts` (по образцу соседей, подключить в
  `mount.ts`, тег `@scope …#READER-GLOSSARY-CARD`). Работает, только когда
  страница в десктопной вёрстке (`has-sidebar` на месте) и
  `matchMedia("(hover: hover) and (pointer: fine)")` верно — проверять в
  момент наведения, а не один раз при загрузке.
- Наведение или фокус с клавиатуры на `a[data-gloss]` → через ~350 мс
  карточка: термин и определение (клон содержимого `#gloss-<id>`), рядом со
  ссылкой — под ней, а если места нет — над ней; ссылку не закрывает, в окно
  укладывается. Держится, пока указатель на ссылке или на карточке (короткая
  пауза на переход), закрывается при уходе указателя, Escape, прокрутке,
  потере фокуса. Одна карточка на странице, переиспользуется. Карточка
  `aria-hidden="true"` (описание читателю экрана даёт `aria-describedby`).
  Ссылки внутри карточки кликабельны.
- Компонент дизайн-системы `web/design/src/components/glossary-card/`
  (стили; только токены: `--bg-raise`, `--line`, `--radius-md`,
  `--shadow-card`, `--text`, `--text-2`), ширина до ~24rem, кегль чуть
  меньше текста, термин выделен; без анимации при `prefers-reduced-motion`.
- В десктопной вёрстке ссылки-термины можно пометить пунктирным
  подчёркиванием, чтобы было видно, что у них есть карточка — реши по месту
  и опиши.

### 4. Фикстуры и тесты

- Фикстурная библиотека объявляет глоссарий без новой страницы (потолок
  CSP): одна из существующих страниц фикстурного руководства становится
  глоссарием, другая ссылается на её статьи; перевод объявляет то же.
  Генерация — путём из README фикстур; пакет без глоссария в фикстурах
  сохрани (откат).
- e2e: 1440×900 — наведение показывает карточку с термином и определением
  после паузы, уход закрывает, Escape закрывает, фокус с клавиатуры
  открывает; карточка не перекрывает ссылку и не даёт горизонтального
  скролла; 834 (без `has-sidebar`) — карточки нет; телефон 390 с эмуляцией
  касания — карточки нет, тап ведёт в глоссарий; на странице глоссария
  карточек нет; при reduced motion нет перехода. Юниты там, где логика
  отделима.

## Периметр

Можно: `crates/vibe-core/**` (грамматика манифеста), `crates/vibe-check/**`,
`crates/vibe-doc/**`, `crates/vibe-cli/**` при необходимости,
`schemas/doc_manifest.jtd.json` + codegen только если решишь, что сайту это
нужно, `web/site/src/**`, `web/site/tests/**`,
`web/design/src/components/**`, фикстуры через конвейер, `web/specmap.json`
(перегенерируй: `C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe specmap --path <web>`),
отчёт. Нельзя: спеки, пакеты руководства (для проверки на настоящем
руководстве — копия в scratch с добавленным `[glossary] page =
"glossary/index"` в обоих манифестах), `BACKLOG.md`, корневой
`specmap.json`, базовые линии храповиков.

## Самопроверка (вывод и коды выхода — в отчёт дословно)

```bash
cargo fmt --all -- --check; echo "EXIT=$?"
cargo test -p vibe-core -p vibe-check -p vibe-doc; echo "EXIT=$?"
cargo clippy -p vibe-core -p vibe-check -p vibe-doc --all-targets -- -D warnings; echo "EXIT=$?"
cargo build -p vibe-cli; echo "EXIT=$?"
# на копии руководства с [glossary] в обоих изданиях:
target/debug/vibe.exe check --path <scratch-en>; echo "EXIT=$?"
target/debug/vibe.exe check --path <scratch-ru>; echo "EXIT=$?"
target/debug/vibe.exe doc check --style --translations --path <scratch-ru>; echo "EXIT=$?"
# .md руководства до и после — без разницы; число ссылок с data-gloss и блоков gloss-defs в HTML (EN, RU)
cd vibevm/vibepacks/org.vibevm.doc/web/v1.0.0
TYPESCRIPT_AI_NATIVE="C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe" node tools/floor.mjs --keep-going; echo "EXIT=$?"
node design/audit/contrast.mjs; echo "EXIT=$?"
node tools/build.mjs static; echo "EXIT=$?"
node node_modules/@playwright/test/cli.js test -c site/tests/playwright.config.ts; echo "EXIT=$?"
node tools/build.mjs embedded; echo "EXIT=$?"
```

## Отчёт

`WORKER-REPORT-GL-O1.md`: файлы; грамматика и проверки; разметка острова;
решение про провод манифеста; карточка (условия десктопа, тайминги,
позиционирование, закрытие); числа по руководству; вывод самопроверки
дословно; отклонения. Затем `echo "TASK-DONE"`.
