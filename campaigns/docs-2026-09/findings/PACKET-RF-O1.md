# Пакет RF-O1 — свёрнутые цитаты правил: остров и читалка (opus5, High) — 2026-09-26

##subagent-quiet-clause
«Ты работаешь в СУБАГЕНТСКОМ режиме: твой экранный текст не читает никто,
деливерабл — только артефакты. НЕ пиши на экран ничего сверх предписанного
заданием. Предписанное ОБЯЗАТЕЛЬНО и не отменяется этой клаузой:
heartbeat'ы `echo "PROGRESS: …"` перед каждым шагом, файл отчёта
`campaigns/docs-2026-09/findings/WORKER-REPORT-RF-O1.md` (решения,
отклонения, вывод самопроверки), финальный `echo "TASK-DONE"`. Запрещено:
приветствия, пересказ задачи, промежуточные рассуждения в чат, финальное
резюме сделанного (оно живёт в отчёте, не в чате).»

## Где ты работаешь

- Дерево `C:\Users\olegc\git\v\vibevm`, ветка `main`, HEAD не старше
  `c500dcba0`. Git Bash; файлы — Write/Edit; UTF-8, LF.
- **Git только читающий.** Ревью и коммиты — центральная сессия.
- Пакет сайта: `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/` (ниже — `web/`).

## Зачем

Владелец: цитаты из спецификаций в руководствах очень навязчивые. В
официальном руководстве 802 цитаты на 49 страницах, медиана 38 слов, максимум
429. Норма уже написана: цитата правила в HTML-острове — раскрывающийся блок,
изначально свёрнутый до одной строки `▶ spec: <короткое описание>`.

## Прочитай первым

1. `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml`:
   `##READER-RULE-FOLDED` и запись решения `##RULE-FOLDED-DECISION` (с
   `rule-folded-why`, `-rejected`, `-revisit`) — это норма, её не менять;
   `##PIPE-SHELL-PARSES-NOTHING` (почему сворачивает Rust, а не оболочка),
   `##READER-NUMBERED-BLOCKS`, `##READER-META-AND-PRINT`, `##LOC-LANGUAGE-FIELD`.
2. Rust: `crates/vibe-doc/src/html.rs` (`fn rule`, `anchor_line`, `open`,
   `line`, `close`), `crates/vibe-doc/src/html/tests.rs`, `content.rs`,
   `build.rs` (`expect_language` — где пакет называет свой язык), `md.rs`
   (НЕ меняется). Правила стека:
   `vibevm/vibedeps/org.vibevm.ai-native.rust-ai-native-lang/1.0.0/vibevm/vibespecs/boot/20-stack-rust-ai-native-lang.xml`.
3. Сайт: `web/design/src/components/prose/styles.css` (`blockquote.rule`,
   якоря `.p-anchor`, печать), `web/site/src/reader/rules.ts` (панель правила
   по клику на `a.rule`, `quote.closest("blockquote")`),
   `web/site/src/reader/cited-rules.ts`, `web/site/src/reader/mount.ts`,
   соседние модули `reader/*.ts` как образец идиом,
   `web/site/src/fixtures/README.md` (как генерируются фикстуры), тесты
   `web/site/tests/*.spec.ts`. Правила стека TypeScript:
   `vibevm/vibedeps/org.vibevm.ai-native.typescript-ai-native-lang/1.0.0/vibevm/vibespecs/boot/20-stack-typescript-ai-native-lang.xml`.

## Что сделать

### 1. Остров (Rust, `vibe-doc`)

Блок `rule` в HTML-бэкенде выводится так (атрибуты блока — `data-p` и
прочие, что сейчас идут на `blockquote`, — переезжают на `details`):

```html
<details data-p="3" class="rule-fold">
  <summary class="rule-fold__line">
    <a class="p-anchor" id="p03" href="#p03">03</a>
    <span class="rule-fold__mark" aria-hidden="true">▶</span>
    <span class="rule-fold__kind">spec:</span>
    <span class="rule-fold__gist" lang="en">Offline resolution is therefore computed against…</span>
  </summary>
  <blockquote class="rule">
    <a class="rule" href="…" data-uri="…" lang="en">…полный текст, как сейчас…</a>
  </blockquote>
</details>
```

- Описание — чистая функция над текстом правила (inline-разметка снята:
  жирный, код, ссылки → их текст), по норме:
  - жирный лид в начале, 2–8 слов → сам лид (без завершающих `.`/`:`);
  - однословный лид («Decision», «Why» …) → `Лид: ` + первые слова остатка
    (до 5, по правилу ниже);
  - лид длиннее 8 слов → первые 6 слов лида + `…`;
  - нет лида → первые слова текста, не больше 6; хвостовые служебные слова
    (`a an the of to and or in on at by for with from as is are be that this
    which into its their your our`) отбрасываются; `…`, если обрезано;
  - во всём правиле ≤ 10 слов, или текст правила не разрешён → описания нет.
- Нет описания → строка без `spec:`: `<span class="rule-fold__gist
  rule-fold__gist--generic">…</span>` со словами издания: `ru` → `цитата из
  спецификации`, иначе `quote from the specification`. Язык издания — из
  пакета (`##LOC-LANGUAGE-FIELD`); проведи его в рендер без изменения `.md` и
  `.xml`.
- Неразрешённая цитата: `data-unresolved="true"` на `details`, тело — адрес,
  как сейчас.
- `lang` на `rule-fold__gist` — язык спецификации (как на `a.rule`), у
  общей подписи — язык издания.
- `.md`, `.xml`, `llms*` — байт в байт как были. Тег нормы на новом коде —
  `READER-RULE-FOLDED` в идиоме крейта.
- Юнит-тесты функции описания (все ветки, включая «Decision:», обрезку
  служебных слов, ≤ 10 слов, русскую общую подпись) и тест острова;
  обновить затронутые golden-тесты острова.

### 2. Читалка (сайт, `web/`)

- Стили в `design/src/components/prose/styles.css` (только токены, без
  литералов цвета): строка свёрнутой цитаты — одна строка, мелкий кегль,
  `--text-3`, слева акцентная граница как у `blockquote.rule`, фон
  `--bg-raise`, радиус как у цитаты; `spec:` моноширинным акцентом; описание
  обрезается многоточием в одну строку; родной маркер `summary` спрятан
  (`list-style: none`, `::-webkit-details-marker`); ▶ поворачивается на 90°
  при раскрытии (`--speed`; без анимации при `prefers-reduced-motion`);
  видимое кольцо фокуса; hover. Раскрытая цитата — прежний `blockquote.rule`
  под строкой, без двойных рамок и лишних отступов. Номер блока (`.p-anchor`)
  стоит на полях как у остальных блоков и виден в свёрнутом состоянии —
  проверь селекторы якорей, которые рассчитывают на `[data-p] > .p-anchor`.
- Печать: модуль `reader/*.ts` по образцу соседей — на `beforeprint`
  раскрыть все закрытые `details.rule-fold` (запомнив, какие), на
  `afterprint` свернуть их обратно; подключить в `mount.ts`; тег
  `@scope …PROP-057#READER-RULE-FOLDED`.
- Проверь, что панель правила (`rules.ts`) и список «Rules this page cites»
  (`cited-rules.ts`) работают на новой структуре.
- Фикстуры острова перегенерируй тем путём, что описан в
  `site/src/fixtures/README.md`. Потолок CSP фикстуры (README) не превышать.
- `web/specmap.json` перегенерируй сам: `C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe specmap --path <web>`
  (без флагов пишет файл).

### 3. Тесты

- e2e: цитата свёрнута по умолчанию (видна строка с ▶ и `spec:`, текст
  правила скрыт); клик раскрывает; Enter на сфокусированной строке
  раскрывает и сворачивает; номер блока виден в свёрнутом состоянии;
  событие `beforeprint` раскрывает все цитаты, `afterprint` возвращает;
  панель правила открывается кликом по раскрытому тексту; нет
  горизонтального скролла на 390, 834, 1440; при reduced motion нет
  перехода. Существующие тесты, которые опирались на `blockquote.rule` как
  на блок с `data-p`, поправь явно.

## Периметр

Можно: `crates/vibe-doc/**`, `crates/vibe-cli/**` только если без этого не
провести язык издания, `web/site/src/**`, `web/site/tests/**`,
`web/design/src/components/**`, `web/specmap.json`, фикстуры через конвейер,
отчёт. Нельзя: спеки, пакеты руководства, `BACKLOG.md`, корневой
`specmap.json`, базовые линии храповиков. Нужно больше — стоп и «дефект
пакета» в отчёте.

## Самопроверка (вывод и коды выхода — в отчёт дословно)

```bash
# до правок: снимок .md руководства текущим бинарём
target/debug/vibe.exe doc build --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0 --out <scratch>/md-before --format md
# после правок:
cargo fmt --all -- --check; echo "EXIT=$?"
cargo test -p vibe-doc; echo "EXIT=$?"
cargo clippy -p vibe-doc --all-targets -- -D warnings; echo "EXIT=$?"
cargo build -p vibe-cli; echo "EXIT=$?"
target/debug/vibe.exe doc build --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0 --out <scratch>/md-after --format md; diff -r <scratch>/md-before <scratch>/md-after; echo "MD-DIFF EXIT=$?"
# число свёрнутых цитат и общих подписей в HTML обоих изданий (EN и RU)
cd vibevm/vibepacks/org.vibevm.doc/web/v1.0.0
TYPESCRIPT_AI_NATIVE="C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe" node tools/floor.mjs --keep-going; echo "EXIT=$?"
node design/audit/contrast.mjs; echo "EXIT=$?"
node tools/build.mjs static; echo "EXIT=$?"
node node_modules/@playwright/test/cli.js test -c site/tests/playwright.config.ts; echo "EXIT=$?"
node tools/build.mjs embedded; echo "EXIT=$?"
```

Все e2e, не выборка: на выходе — число прошедших и упавших.

## Отчёт

`WORKER-REPORT-RF-O1.md`: файлы; разметка острова; функция описания и её
ветки; как проведён язык издания; стили и печать; числа по руководству (EN и
RU: свёрнутых цитат, с описанием, с общей подписью) и 15 случайных строк
описаний; вывод самопроверки дословно; отклонения и что не сделано. Затем
`echo "TASK-DONE"`.
