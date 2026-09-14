# WORKER-REPORT-W1-O2 — версия 1.0.0 у руководства и сайта; вид пакета на проводе

Пакет: `campaigns/docs-2026-09/findings/PACKET-W1-O2.md`. Ворктри
`C:\Users\olegc\git\v\vibevm-docs`, ветка `research-preview-1-docs`.
Три коммита вместо четырёх — M сложен в K, обоснование ниже. `git push` не
делался, `specmap.json` не регенерировался, трейлеров и упоминаний агента в
сообщениях коммитов нет, чужие процессы не останавливались,
`CARGO_TARGET_DIR` не переопределялся. Дерево после работы чистое.

## Коммиты

| хэш | subject | файлов |
| --- | --- | --- |
| `296cfd39` | `feat(doc): carry a package's kind in the documentation manifest` | 13 (+259 −32) |
| `f6cafbe0` | `refactor(doc): publish the manual and the site as 1.0.0` | 396 (+90 −80, из них 372 переименования) |
| `8197d029` | `docs(spec): keep every published number at 1.0.0 and republish in place` | 2 (+3 −2) |

### N2 — `296cfd39`

Четыре двери (J-126) плюс проекция и тесты:

- `schemas/doc_manifest.jtd.json` — новое определение `package_kind`
  (закрытое перечисление восьми видов PROP-000 `##KIND-SET`) и необязательный
  член `doc_package.kind`; `kind` добавлен в `x-wire-order` рядом с
  `projection`.
- `crates/vibe-wire/src/generated/doc_manifest/mod.rs` и
  `…/web/…/site/src/generated/doc-manifest.ts` — перегенерированы
  (`cargo run -p xtask -- codegen`), правок руками нет.
- `crates/vibe-doc/src/manifest.rs` — `Card.kind`, читатель `package_kind(word)`
  по образцу `authorship(word)`, заполнение `DocPackage.kind`. Слово
  `[package].kind` теперь читается ОДИН раз и отвечает на два вопроса: какой
  вид у карточки и является ли рендер проекцией (`projection` считается из того
  же локального значения — поведение бит в бит прежнее).
- `site/src/lib/manifest.ts` — `optionalMember(value, "kind", at, PackageKind)`
  и вынос члена в результат.
- `site/src/lib/library.ts` — `kindOf(card)` читает член; вывод из
  `projection` оставлен ВТОРОЙ половиной для манифестов, написанных до члена,
  ровно как это уже сделано в соседней `isProjection`. **Отклонение от буквы
  пакета, осознанное:** пакет говорит «читать поле вместо вывода из
  `projection`»; поле читается первым, вывод остался только как legacy-ветка,
  иначе уже опубликованный манифест без члена потерял бы метку `doc`. Если
  боссу нужна чистая форма `return card.kind;` — это одна строка.
- `crates/vibe-doc/src/site/level0.rs` **править не потребовалось**: композиция
  уровня 0 уже пишет `kind` в синтезированный `vibe.toml`
  (`site/level0/card.rs:167`, `synthesise`), а пакет без вида пишет туда
  нейтральное `pack`, которое читатель отдаёт как отсутствие. Проверено тестом.

Тесты на разбор поля:

- Rust, `crates/vibe-doc/src/manifest/tests.rs::the_card_carries_the_kind_the_package_calls_itself_by`
  — все восемь видов доезжают; `tool` несёт и `kind`, и `projection`; `pack`
  даёт `None` и остаётся проекцией.
- TS, `site/src/lib/manifest.test.ts::"the kind of the rendered package crosses as itself"`
  — `tool` + `projection` пересекают границу; отсутствие члена permissive;
  девятое слово (`pack`) отклоняется с путём `$.package.kind`.
- TS, `site/src/lib/library.test.ts` — старый тест переписан: карточка носит
  тот вид, который назвал манифест, и ни одного — когда не назвал.

Корпус формата переблагословлён (`VIBE_DOC_BLESS=1 cargo test -p vibe-doc
--test doc_manifest_wire`): в `formats/corpora/doc-manifest/e1/manual.json` и
`adaptation.json` добавился ровно один ключ `"kind": "doc"`. **Эпоха формата
НЕ двигалась** и `SCHEMA_VERSION` остался `1`: член необязательный, реестр
даёт формату `foreign_parsers = "many"` и permissive-чтение (PROP-044 §4.4) —
документ без члена остаётся документом. `maintenance/derived.json` обоих
doc-пакетов сдвинулся на один блок (`reference/machine-formats.xml#jtd-schema:schemas/doc_manifest.jtd.json`)
и принят `--derived --accept`; проза вокруг блока не называет ни полей, ни
чисел, править было нечего.

### K — `f6cafbe0` (вместе с M)

`git mv` обоих каталогов; `version = "1.0.0"` в обоих манифестах и в
`package.json` сайта; `[translates] version = "^1.0"` у `vibevm-docs-ru`;
`[[documents]] version` не тронут — это версия предмета. Запись
`## 1.0.0 — 2026-09-14` в `CHANGELOG.md` руководства (тёмная тема и
переключатель, работающий поиск, метки на карточках, `self`-глаголы, русская
адаптация отдельным пакетом), первая запись переименована в
`## 1.0.0 — the first edition`, а вводный абзац переписан с «по версии» на
«по дате» — иначе файл противоречил бы политике из L.

**M сложен в K** (пакет это разрешает и просит сказать). Причина: разделение
даёт красный промежуточный коммит — после K `tools/self-check.sh` и
`docker/Dockerfile` указывали бы на каталог, которого уже нет. Одна мысль,
один коммит.

### L — `8197d029`

`VERSION-OVERWRITE-POLICY` в PROP-057 рядом с `SITE-VERSION-SHOWS-CURRENT`,
текст дословно из пакета. Статус `impl/done action="continue"
actionstage="doc"`.

**`audience` снят — как пакет и предусмотрел.** С `audience="author"` гейт
покрытия краснеет: `coverage: 537 of 538 obligation(s) told, 99%`,
`UNCOVERED author — …#VERSION-OVERWRITE-POLICY / no page cites it`. Прозу
страницы пишет центральная сессия; до неё `actionstage="doc"` и есть честная
запись долга. После снятия — `537 of 537, 100%`.

PROP-058: `LOOP-MONTHLY` больше не обещает минорную версию — обещает
публикацию под тем же номером с датированной записью в changelog, пока
действует `##VERSION-OVERWRITE-POLICY`. `LOOP-VERSION` — то же самое плюс
судьба снимка поверхности: не «рядом со снимком, который оставила прошлая
смена версии», а один файл, перезаписываемый на месте
(`vibe doc surface --record 1.0.0`).

## Вхождения путей: переписанные и оставленные

### Переписаны (все, что находит grep из пакета) — 12 файлов вне двух каталогов

| правок | файл |
| --- | --- |
| 13 | `README.md` |
| 7 | `DEV-GUIDE.md` (§8 сайта, §7 doc-гейт, docker) |
| 6 | `crates/vibe-doc/src/site/feed/tests.rs` |
| 3 | `tools/self-check.sh` (шаги 4b, 4c, `WEBPKG_DIR`) |
| 3 | `RUNTIME-GUIDE.md` |
| 2 | `crates/vibe-doc-server/src/lib.rs` (два doc-примера) |
| 1 | `xtask/src/doc_shell.rs` (`WEB_PACKAGE`) |
| 1 | `xtask/src/codegen/typescript.rs` (`out_dir`) |
| 1 | `crates/vibe-specdoc/tests/docs_corpus.rs` |
| 1 | `crates/vibe-doc/src/manifest.rs` (doctest) |
| 1 | `crates/vibe-cli/src/commands/doc/site/web.rs` (`WEB_PACKAGE`) |
| 1 | `campaigns/docs-2026-09/PLAN.md` §9, строка `D=…` |

Внутри двух переехавших каталогов:

| правок | файл |
| --- | --- |
| 10 | `…/web/v1.0.0/docker/Dockerfile` |
| 8 | `…/web/v1.0.0/docker/Dockerfile.dockerignore` |
| 6 | `…/vibevm-docs/v1.0.0/…/architecture/traceability.xml` (`<expect>`) |
| 3 | `…/vibevm-docs/v1.0.0/…/authoring/specs-agents-can-cite.xml` (`<expect>`) |
| 2 | `…/web/v1.0.0/docker/compose.yaml` |

Сверх grep'а пакета, чтобы фикстура не противоречила себе:
`feed/tests.rs` — `version = "0.1.0"` → `"1.0.0"` в двух синтетических
манифестах и утверждение `pairs[1].spelled()` → `org.vibevm.core/vibevm-docs@1.0.0`
(иначе тест падал бы от моей же правки фикстуры).

### Оставлены, с причиной

- **Вся история кампании** — `campaigns/docs-2026-09/LEDGER.md`, `JOURNAL.md`,
  `findings/**` (пакеты и отчёты): это история, пакет прямо запрещает.
- `campaigns/docs-2026-09/PLAN.md` строки **1421, 1507, 1543** — описания
  атомов A3.1, A4.1 и волны C в §6 «Фазы»: они говорят, что создавалось тогда,
  а не что где лежит сейчас. Пакет назвал §9, и только §9 — живой гейт. Строка
  1507 к тому же уже была устаревшей до меня: она называет
  `vibevm-docs-ru/v0.1.0`, а адаптация давно в `v1.0.0`.
- `crates/vibe-doc-server/src/routes/resolve.rs:45` — `spec://org.vibevm.core/vibevm-docs@0.1.0/guide/start`
  в `text`-блоке rustdoc: иллюстрация СИНТАКСИСА адреса с версией, не путь. Под
  grep пакета не подпадает и в периметр `crates/**` («только там, где путь
  `v0.1.0` вписан в тест или фикстуру») — тоже нет. Но версия там теперь
  несуществующая: **находка для босса, правка на одно слово.**
- `crates/vibe-doc-shell/src/index.rs:62` — doctest с индексом оболочки
  `{"package":"org.vibevm.doc/web","version":"0.1.0"}`. Та же категория и та же
  находка: реальная сборка теперь пишет туда `1.0.0`.
- `…/web/v1.0.0/site/src/lib/href.test.ts` (строки 39, 43, 47, 148, 151, 153,
  195) — `0.1.0` как сегмент версии во ВХОДНЫХ данных юнит-тестов на сборку
  URL. Версия там — произвольная строка, которую функция переписывает в
  адрес; к путям двух пакетов отношения не имеет.
- `crates/vibe-doc/tests/fixture/translations/adaptation/vibe.toml:35` —
  `version = "^0.1"` в фикстуре переводов, не в боевой адаптации.
- `…/vibevm-docs/v1.0.0/maintenance/surface/1.0.0.json:123` — строка
  `"Default: 0.1.0 for packages"`: это цитата хелпа `vibe`, а не путь.

## Гейты — вывод дословно

```
$ grep -rn "vibevm-docs/v0.1.0\|web/v0.1.0" . --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=vibedeps --exclude-dir=campaigns
(пусто)

$ cargo build -p vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.29s

$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0 --citations --derived --coverage --media --style --min 100
citations: 790 rule(s), 798 address(es) to resolve, 2 self-address(es), 0 unresolved, 9 placeholder(s), 2 teaching, 0 unreadable page(s)
style: 49 of 49 page(s) clean, 100% (threshold 100%), 0 error(s), 128 warning(s), 0 unreadable page(s) [en]
media: 0 declared (none), 0 error(s), 0 warning(s), 3 role(s) generated
  user    313/313 (100%)
  author  220/220 (100%)
  dev     19/19 (100%)
  agent   40/40 (100%)
coverage: 537 of 537 obligation(s) told, 100% of 592 audience pair(s) (threshold 100%), 0 unreadable page(s)
derived: 76 unchanged, 0 moved

$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0 --examples
examples: 52 matched, 0 captured, 0 re-blessed, 3 differ, 0 failed, 7 skipped, 0 unreadable page(s)
error: documented examples do not match the product: 3 diverged, 0 could not run, 0 page(s) unreadable (violates spec://org.vibevm.core/vibevm/common/PROP-057#INV-EXAMPLES-RUN; fix: repair the product, or the page — never the comparison)
   ↑ КРАСНОЕ ДО МЕНЯ, разбор ниже

$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0 --citations --derived --translations --style --min 100
citations: 790 rule(s), 791 address(es) to resolve, 2 self-address(es), 0 unresolved, 9 placeholder(s), 2 teaching, 0 unreadable page(s)
translations: adapting org.vibevm.core/vibevm-docs found in the project's own in-tree registry, 49 page(s), 0 problem(s), 0 unreadable page(s)
style: 49 of 49 page(s) clean, 100% (threshold 100%), 0 error(s), 33 warning(s), 0 unreadable page(s) [ru]
derived: 76 unchanged, 0 moved

$ target/debug/vibe.exe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0
vibe check: clean — every check passed against `…\vibevm\vibepacks\org.vibevm.core\vibevm-docs\v1.0.0`

$ cargo test -p vibe-doc
test result: ok. 567 passed; 0 failed; …
test result: ok. 5 passed; 0 failed; …
test result: ok. 3 passed; 0 failed; …
test result: ok. 5 passed; 0 failed; …
test result: ok. 64 passed; 0 failed; …   (doc-тесты)

$ pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 floor
ℹ tests 120
ℹ fail 0
test-gate: 129 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).
=== pairs: gated=40, reference=6; below threshold=0 ===

$ pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 build:static
build (static): generated 18 page(s), expected 18
links: green — 647 checked, 0 broken.
build (static): ok

$ pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 build:embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): ok

$ docker compose -f docker/compose.yaml config -q      # из …/web/v1.0.0
(пусто — зелено; Docker Compose v5.0.2)

$ cargo run -p xtask -- check-codegen
xtask check-codegen: clean.

$ cargo xtask specmap --check
Error: `…\specmap.json` is out of date relative to the tree.
  drift: units added: 1
   ↑ ОЖИДАЕМО: один добавленный юнит — это факт `VERSION-OVERWRITE-POLICY` из L.
     Перемещённые doc-страницы дрейфа не дали: в `specmap.json` путей обоих
     пакетов нет вообще (0 вхождений и старой, и новой формы) — `vibe explain`
     и `vibe select` сканируют дерево живьём. Карту, как велит пакет, НЕ
     регенерировал.

$ target/debug/vibe.exe doc manifest --path …/vibevm-docs/v1.0.0 --json | head
  "version": "1.0.0",
  …
  "kind": "doc",
```

## Аномалии

1. **`--examples`: 3 расхождения, красные ДО моей работы.** Расходятся
   `architecture/traceability.xml#explain`, `#select` и
   `authoring/specs-agents-can-cite.xml#explain`. Мой rewrite сделал ПУТИ в
   `<expect>` правильными — `v1.0.0` теперь совпадает с обеих сторон. Остаётся
   единственная разница: фактический вывод несёт ЕЩЁ и строки русской
   адаптации (`vibevm-docs-ru/v1.0.0/…`, плюс третья страница
   `model::packages-and-kinds`), а голдены перечисляют только английские.
   Доказательство, что это не от переезда: в `HEAD` (до моих коммитов) в
   `traceability.xml` было **ноль** вхождений `vibevm-docs-ru`, а сама
   адаптация лежала в `v1.0.0` и её страницы цитируют `KIND-SET` — то есть те
   же три примера расходились и раньше, теми же строками. Не переблагословил
   намеренно: инструмент сам говорит, что «re-blessing a divergence is a
   decision somebody takes, not a side effect of running a check»
   (`--accept --force`), и решение «руководство показывает шесть строк вместо
   трёх» — это правка прозы, отдельная мысль и не мой пакет.
2. **`cargo test -p vibe-specdoc --test docs_corpus` — красный, тоже до меня.**
   `docs_corpus_shape_is_counted`: `the pages cite 781 rules / left: 790 / right: 781`.
   Считал по коммитам: `<rule ` в руководстве — 790 в рабочем дереве, 790 в
   `HEAD`, `HEAD~1`, `HEAD~2`, `HEAD~3`. Константу не двигал: она затем и
   стоит, чтобы заметить сдвиг формы корпуса, и кто-то должен посмотреть,
   откуда взялись девять правил.
3. **`vibe facts check --exhaustive` — `2086 error(s), 25 warning(s)`,
   до меня и не про меня.** Ни `VERSION-OVERWRITE-POLICY`, ни `LOOP-VERSION`,
   ни `LOOP-MONTHLY` в списке ошибок не встречаются; ошибки в
   `vibevm/vibepacks/org.vibevm.ai-native/**`. В требуемый пакетом список
   гейтов эта команда не входит; отмечаю, потому что §9 плана требует от неё
   нуля, а `tools/self-check.sh` гоняет её шагом 11.
4. **Чужой процесс ломал прогон примеров.** Tripwire песочницы
   (`PROP-057#PIPE-EXAMPLE-RUNNER`) шесть раз из семи ловил изменения в
   `~/.zap/build/next-rust/**` — посторонняя cargo-сборка, которую я не
   запускал и, по пакету, не останавливал. Прогонял с повтором до тихой
   попытки. Это НЕ дефект примеров и не дефект дерева; но гейт примеров на
   этой машине флапает, пока рядом идёт любая cargo-сборка.
5. **`pnpm` пришлось перелинковать.** После `git mv` каталога `node_modules`
   переехал вместе с ним, но на Windows pnpm держит junction'ы с АБСОЛЮТНЫМИ
   путями, и `floor` упал на `Cannot find module …/v1.0.0/node_modules/prettier/bin/prettier.cjs`.
   Вылечено `CI=true pnpm -C …/web/v1.0.0 install --frozen-lockfile --offline`
   (319 пакетов, `reused 319, downloaded 0`); `pnpm-lock.yaml` не изменился.
   `CI=true` понадобился только чтобы pnpm снёс несогласованный `node_modules`
   без TTY-подтверждения. **Босу на заметку:** тот же шаг понадобится на любой
   машине, где каталог сайта переезжал.
6. **`cargo xtask check-codegen` один раз упал** на
   `Access is denied. (os error 5)` при подмене каталога
   `crates/vibe-wire/src/generated` — виндовый файловый лок, старое дерево
   инструмент восстановил сам. Повтор сразу дал `clean`.
7. `Dockerfile.dockerignore` лежит не «в корне репозитория», как говорит пакет,
   а в `…/web/v1.0.0/docker/` рядом с `Dockerfile` (пути ВНУТРИ него —
   от корня репозитория, потому что контекст сборки это корень). Противоречия
   с деревом нет, это разночтение формулировки; переписан по месту.

## Что не сделано и почему

- **`git push`** — запрещён пакетом.
- **`specmap.json`** — не регенерировал, запрещён пакетом; `--check` красный
  на один добавленный юнит, см. выше.
- **Переблагословление трёх примеров** — см. аномалию 1: расхождение досталось
  в наследство, а его закрытие — решение о прозе руководства, а не побочный
  эффект переезда.
- **Константа 781 в `docs_corpus.rs`** — см. аномалию 2.
- **Страница руководства про `VERSION-OVERWRITE-POLICY`** — прозу пишет
  центральная сессия; факт стоит `impl/done` + `actionstage="doc"` без
  `audience`, то есть долг записан и гейт покрытия зелёный.
- **`docker build`** — не гонял, серверная выкладка за центральной сессией;
  `docker compose config -q` прогнан и зелёный.
- **`@0.1.0`-упоминания в `resolve.rs` и `index.rs`** — вне grep'а и вне
  периметра пакета; выписаны выше как находка.
