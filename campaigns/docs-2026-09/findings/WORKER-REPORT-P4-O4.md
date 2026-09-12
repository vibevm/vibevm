# WORKER-REPORT-P4-O4 — оболочка внутри `vibe`: встраивание, `vibe doc serve` с оболочкой, паритет, локальный ридер без сети

## Коротко для оркестратора

Четыре атома сделаны; пять коммитов (без push) — четыре атомных и один
`fix` на дыру, которую я нашёл в собственном коде первого коммита (А-5).

Оболочка ридера теперь
живёт в бинарнике: `cargo xtask embed-doc-shell` собирает web-пакет
встраиваемым адаптером, отбирает из выхода то, что читателю действительно
нужно (**129 файлов, 802 900 Б** из 144 и 1,6 МБ), пишет индекс оболочки и
перепинывает `doc-shell.lock`. `vibe doc serve` вклеивает остров в шаблон
маршрута на каждый запрос и считает `sha256` **каждого** инлайн-скрипта
оболочки при старте — их семь, и ни одного внешнего источника в политике.
Вклад оболочки в отладочный бинарник — **827 904 Б** (112 801 280 →
113 629 184).

Одно расхождение с `##LOCAL-CSP` требует подтверждения владельцем
(`style-src 'unsafe-inline'` под настоящей оболочкой) и одно уточнение
формы к `##SEO-MANIFEST-AND-RESOLVER` (поле `media` необязательное). Обе —
в разделе расхождений.

**Что осталось чужой правкой** (названо в разделе «что не сделано»):
оболочка в локальном читателе показывает навигацию и мета-строку
**фикстуры**, с которой её собрали, а не обслуживаемого пакета. Править
web-пакет пакет запретил, а без этого оболочка не может прочитать манифест,
который отдаёт читатель.

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A4.6 | `0cae55ca` | `feat(xtask): embed the docs shell into vibe at release time` |
| A4.7 + хвост `media` | `dd555de5` | `feat(doc): serve the documentation locally with the embedded shell` |
| A4.8 | `e9c51b98` | `test(doc): pin island parity across shell adapters` |
| A4.15 | `408b4b97` | `test(doc): pin the local reader to loopback resources` |
| — (дефект в своём же коде, А-5) | `b1ea7057` | `fix(doc-shell): refuse a drive prefix in a shell asset name` |

Пятый коммит — не атом плана: это дыра в коде первого коммита, найденная
перечитыванием после сдачи (аномалия А-5). Вносить её правкой первого
коммита значило бы переписать три коммита истории — отдельный `fix` честнее
и ничего не переписывает.

Коммит A4.15 пересобран `--amend` сразу после создания: в теле сообщения
две обратные кавычки съела оболочка (`file://` и `vibe` пропали), текст
восстановлен из файла. Ветка не пушилась.

## Где что лежит (для приёмки)

| Требование пакета | Где |
|---|---|
| `cargo xtask embed-doc-shell` | `xtask/src/doc_shell.rs`, подкоманда в `xtask/src/main.rs` |
| крейт с `include_dir` за фичей `embedded-shell` | `crates/vibe-doc-shell/{Cargo.toml,src/lib.rs}` |
| `build.rs`: `rerun-if-changed`, отказ при пустом каталоге, `VIBE_DOC_SHELL_SHA256` | `crates/vibe-doc-shell/build.rs` |
| запасная оболочка без скриптов | `crates/vibe-doc-shell/src/fallback.rs` |
| пин `doc-shell.lock` | `crates/vibe-doc-shell/doc-shell.lock`, модель — `src/pin.rs` |
| шаг панели «оболочка совпадает с пином» | `tools/self-check.sh:688` |
| `vibe doc shell install [--assume-yes]`, `DOC-SHELL.json`, размер+дайджест, `~/.vibe/opt/vibevm/doc-shell/<sha256>/` | `crates/vibe-cli/src/commands/doc/shell.rs` + `shell/fetch.rs` |
| один тип `Shell`, три источника, `provenance()` | `crates/vibe-doc-shell/src/lib.rs` |
| статика оболочки, вклейка острова | `crates/vibe-doc-server/src/routes/statics.rs`, `routes.rs::in_shell` |
| базовый путь `--base`, язык, `--frame-ancestor` | `crates/vibe-doc-server/src/lib.rs` (`Config`, `wearing`), `commands/doc.rs::run_serve` |
| CSP с `sha256` каждого инлайн-скрипта из байтов оболочки | `lib.rs::content_policy` + `vibe-doc-shell/src/template.rs` |
| резолвер `/doc/resolve?uri=spec://…` | `crates/vibe-doc-server/src/routes/resolve.rs` |
| тесты: обход путей, loopback, CSP, `provenance()` в `--json` | `vibe-doc-server/src/{tests.rs,routes/tests.rs}`, `vibe-doc-shell/src/tests.rs`, `vibe doc serve --print-shell` |
| тест паритета + шаг панели | `routes/tests.rs::the_island_is_the_same_bytes_through_both_adapters`, `self-check.sh:700` |
| тест «ни одного внешнего запроса» + шаг панели | `…/web/v0.1.0/site/tests/local-reader.spec.ts`, `self-check.sh:737` |
| поле `media`: схема, codegen, заполнение, тест | `schemas/doc_manifest.jtd.json`, `vibe-doc/src/media/published.rs`, `manifest.rs`, `build.rs`, `manifest/tests.rs` |

## Решения

### Решение 1 — оболочка едет в бинарнике как выборка выхода, а не весь `dist-embedded`

Встраиваемая сборка пишет 144 файла и 1,6 МБ: **14 предрендеренных страниц
фикстуры**, её `sitemap.xml`, её `manifest.json`, шрифты, стили и чанки.
Читателю из этого нужно три вещи: один шаблон маршрута с местом под остров,
`assets/**` и `build/**`. Остальные тринадцать страниц — свойство
библиотеки страниц, с которой собирался пакет, и ни одного запроса к ним не
приходит.

`cargo xtask embed-doc-shell` копирует выборку: шаблон под стабильным именем
`page-template.html`, `assets/**`, `build/**`, `favicon.svg`. **129 файлов,
802 900 Б** — две трети веса оболочки в релизном бинарнике сняты тем, что их
никто не просит.

Шаблон выбирается детерминированно (самый короткий адрес среди страниц,
несущих маркер острова; при равенстве — лексикографически): недетерминированный
выбор менял бы дайджест оболочки между двумя одинаковыми прогонами.

### Решение 2 — `shell.json`: оболочка говорит о себе сама

Оболочка приходит каталогом непрозрачных контент-хэшированных имён. Какой
файл — шаблон, какая строка внутри него — место острова, под какой базовый
путь собраны адреса: это знает **сборка**, и сервер не может восстановить
это, посмотрев на имена. Шаг встраивания пишет `shell/shell.json` (`schema`,
координата, версия, `base`, `island_marker`, `files`, `sha256`), и индекс
едет **внутри** оболочки, а не рядом: скачанная из релиза и вкомпилированная
оболочки отвечают на одни и те же вопросы, и место у них одно.

### Решение 3 — один алгоритм дайджеста, три числа, и кто их сравнивает

`SHELL-PIN` требует, чтобы self-check сверял встроенную оболочку с пином.
Сверять можно только заново **измерив** байты — иначе сравниваются две копии
одного заявления.

- `vibe_doc_shell::digest::of` — единственная реализация (свёртка по дереву:
  имя, разделитель, длина, байты; файлы в порядке путей).
- `cargo xtask embed-doc-shell` зовёт её, когда пишет `shell.json` и пин.
- `vibe doc shell status` зовёт **её же** на байтах, которые бинарник несёт
  прямо сейчас, и печатает три числа: `measured` (замер), `recorded`
  (заявление индекса), `declared` (константа `VIBE_DOC_SHELL_SHA256` из
  `build.rs`) — плюс пин.

`build.rs` **ничего не считает**: он читает `sha256` из индекса и отдаёт
константой. Второй хэшер в build-скрипте был бы вторым мнением о тех же
байтах; измерение в рантайме проверяет оболочку **в том виде, в каком её
несёт бинарник**, что и есть вопрос пина. `declared` сравнивается только для
встроенной оболочки: бинарник, собранный из исходников, не компилирует
внутрь ничего, и требовать от него константу значило бы запретить сборке из
исходников носить настоящую оболочку.

Живой вывод обеих конфигураций — в разделе гейтов.

### Решение 4 — CSP: хэши считаются при старте из байтов оболочки

X-035 просил считать `sha256` инлайн-скриптов из байтов оболочки при старте,
не хардкодить. Сделано: `vibe_doc_shell::template::hashes` разбирает шаблон
ровно тем правилом, каким инлайн-скрипт определяет **браузер** (нет `src`, а
`type` — пустой, `module` или JS-медиатип), и то же правило живёт у
web-пакета в `seo/csp.ts`. `<script type="qwik/state">` и
`<script type="qwik/vnode">` — данные, браузер их не исполняет, политика их
не блокирует, и хэш на них вырастил бы заголовок на число, которого никто не
ищет.

На живом руководстве это **семь** хэшей: тема, отключение восстановления
прокрутки, загрузчик графа бандлов, таблица функций `q:func`, список
preload, инициализатор контейнера и один пустой (`47DEQpj8…` — sha256
пустой строки: у одного скрипта оболочки тело пусто). Столько же, сколько
исполняемых инлайн-скриптов в шаблоне — проверено пересчётом тегов.

### Решение 5 — что сервер правит в шаблоне, и что не правит

Шаблон — предрендеренная страница фикстуры. Сервер меняет в ней ровно то,
что называет СТРАНИЦУ, а не оболочку:

1. остров — на место маркера;
2. `<title>` — на заголовок обслуживаемой страницы;
3. `<link rel="alternate">` — на настоящие `.md`, `.xml`, `llms.txt`.

Всё остальное — голова, чанки, раскладка — байты сайта, нетронутые. Именно
это делает тест паритета осмысленным: он вычитает из отданной страницы две
половины шаблона и сравнивает середину с островом статической сборки.

### Решение 6 — адрес монтирования и дверь ведут на первую страницу

`<base>` и `<base><координата>/<версия>/` раньше отвечали `400`/`404` — в том
числе адрес, который сервер печатает при старте. Читатель наведён на **один**
пакет, поэтому оба адреса значат «эта документация» и оба дают `302` на
первую страницу в порядке закона слоёв. Страница пакета как отдельный рендер
недоступна: её содержимое (полки, карточки, аннотации) оболочка берёт из
библиотеки страниц, вкомпилированной в сборку.

### Решение 7 — `--lang` и `[i18n].preferred`

`##LOCAL-SERVE`: «The language preference comes from the project's
`[i18n].preferred` when present». Читатель обслуживает один пакет, так что
предпочтению не из чего выбирать — оно **заявляет ожидаемый язык**, и
семантика P2-O8 (отказ с координатой адаптации) применяется к нему так же,
как к флагу; флаг перекрывает предпочтение. Корневой `vibe.toml` хоста
`[i18n]` не объявляет, так что на этой машине путь пустой.

### Решение 8 — поле `media` и почему адрес считает Rust

Хвост X-042. Схема `doc_manifest` получила определение `card_media` (`icon`,
`banner`, `preview`) и поле `media` у карточки. Адрес решает **одно** место —
`vibe_doc::media::slots` (модуль `media/published.rs`): объявленная картинка
именуется по своим байтам, сгенерированная — по координате. `build` пишет
файлы по этим адресам, манифест их называет, оболочка показывает названное.
Второй хэшер на TypeScript положил бы одно правило в два языка, и симптомом
расхождения была бы картинка, которая молча перестала грузиться.

Тест сравнивает **множество адресов манифеста** с **множеством файлов,
которые сборка реально написала**, — не со строкой, набранной в тесте.

Живой прогон на руководстве:

```
$ curl -s http://127.0.0.1:8471/doc/manifest.json | … ['package']['media']
{
  "icon": "media/e65e5833a9f1d438.svg",
  "banner": "media/2778b64929450ffd.svg",
  "preview": "media/d54e7cc3e2097d60.png"
}
```

### Решение 9 — скачивание оболочки: шов `Fetcher`, две проверки, каталог по дайджесту дерева

`vibe doc shell install` повторяет **форму** `vibe self install` (тот же
анонимный клиент, те же заголовки, то же ограничение по объявленному
размеру, тот же временный файл, снимаемый при любом исходе, то же правило
«совпасть должны и размер, и дайджест»), но не заимствует код: загрузчик VVM
приватен для store версий, и взять его значило бы втащить модель store в
читателя. Тот же размен, что `##PIPE-CRATES` уже сделал между этими
крейтами.

**Две проверки, не одна.** Сначала архив против `DOC-SHELL.json` (транспорт),
потом — распакованное дерево против **пина** (принадлежность этой сборке).
Релиз может быть целым и при этом нести оболочку, собранную из другого
дерева. Каталог в store называется дайджестом **дерева оболочки** (тем же
числом, что в пине), поэтому поиск нужной оболочки — это подстановка, а не
перебор, и оболочка, байты которой не хэшируются в её собственное имя, не
находится вовсе.

Согласие берётся приёмом `vibe install`: `--assume-yes`, `unattended` и
`--json` — уже заявленный ответ, отсутствие терминала — отказ с рецептом,
иначе вопрос с `default(false)`. Автоматического скачивания нет нигде.

Тесты гоняют `install_into` через `Fetcher`, читающий каталог, который
написал сам тест: единственная команда поверхности, которая пошла бы в сеть,
проверена без сети (релиз не той версии, битый архив, оболочка не по пину,
запись архива, выходящая из каталога).

### Решение 10 — `postMessage`: серверу нечего реализовывать, и есть что не сломать

Контракт P4-O2 (`{theme}`, `{settings}`, `{open: "spec://…"}` внутрь;
`{settings}`, `{openFile}`, `{prompt}` наружу) — разговор хоста со
**страницей**. Ни одно из сообщений не запрос, так что сервер их не видит и
реализовывать ему нечего. Две вещи он мог бы сломать, и обе закрыты:

1. **Страница должна фреймиться** хостом, который её запустил — это
   `--frame-ancestor <origin>` (был и остался параметром запуска, потому что
   origin webview меняется от окна к окну). Проверено тестом
   (`frame-ancestors vscode-webview://abc` в конце строки политики).
2. **Скрипты оболочки должны исполняться** — политика называет каждый по
   хэшу его байтов, а не запрещает все скопом (Решение 4).

Одна половина контракта получила серверный ответ: хост, открывающий адрес
`spec://`, может спросить `<base>resolve?uri=…` и получить `302` на страницу,
вместо того чтобы разбирать координату самому. Оболочка P4-O2 резолвит
`open` собственной картой адресов — картой **фикстуры**, с которой её
собрали (см. «что не сделано»), так что маршрут резолвера — это работающая
половина, пока оболочка не читает манифест читателя.

Записано в заголовке модуля `crates/vibe-doc-server/src/lib.rs`, чтобы
следующий не искал.

## Расхождения с предложенным механизмом PROP-057 §12 (для подтверждения владельцем)

**Р-1. `style-src` получает `'unsafe-inline'` под настоящей оболочкой.**
`##LOCAL-CSP` пишет политику дословно, и в ней `style-src 'self'`. Каркас
инлайнит таблицу стилей каждого компонента в документ, а настройки чтения
пишут меру и кегль **атрибутом** `style` — который хэш покрыть не может,
потому что хэш называет содержимое элемента, а у атрибута его нет.
Публичная сборка сайта пришла к тому же (`site/src/seo/csp.ts`:
`style-src 'self' 'unsafe-inline'`). Под **запасной** оболочкой политика
остаётся дословно нормой — её таблица стилей это файл, и тест сравнивает
строку байт в байт с константой `CSP_WITHOUT_FRAME_ANCESTORS`.
*Что подтвердить:* правку `##LOCAL-CSP` — `style-src 'self' 'unsafe-inline'`
с записанной причиной, либо решение снять инлайн-стили в web-пакете (это
правка адаптера, не читателя).

**Р-2. Поле `media` в манифесте — необязательное, не обязательное.**
Пакет просил «поле `media` с тремя адресами». Сделано обязательным оно быть
не может по двум независимым причинам, и обе стоит записать:
(а) реестр форматов объявляет `doc-manifest` `foreign_parsers = "many"`, то
есть чтение **разрешительное** (PROP-044 §4.4) — новый обязательный член
делает невалидным каждый уже опубликованный документ;
(б) читатель манифеста в web-пакете (`site/src/lib/manifest.ts`) собирает
`DocPackage` литералом, и обязательное поле уронило бы `tsc` — а пакет
править запрещено.
Каждая сборка конвейера поле пишет, и всегда все три адреса; отсутствие
означает «документ старше поля», никогда «у пакета нет картинки». Это
записано в описании поля в схеме.
*Что подтвердить:* формулировку; если владелец хочет обязательность —
это отдельный атом с правкой `manifest.ts` и решением по уже опубликованным
документам.

**Р-3. Оболочка кладётся выборкой, а не целиком.** `##SHELL-XTASK-EMBED`
говорит «places the result … in a directory the crate includes». Кладётся не
весь результат, а шаблон + `assets/**` + `build/**` + `favicon.svg`
(Решение 1). Это исполнение, а не пересмотр: остальное — предрендеренные
страницы фикстуры, к которым читатель не обращается.

**Р-4. Релизный актив оболочки не производится.** `##SHELL-RELEASE-ASSET`
описывает `vibevm-doc-shell-<version>.zip` и `DOC-SHELL.json` рядом с
`DISTRIBUTIONS.json`. **Читающая** половина сделана целиком (форма манифеста,
обе проверки, распаковка); **пишущая** — нет: это шаг публикации релиза, а
публиковать и деплоить пакет запретил (R-24), и расширение счёта «двенадцать
активов» в `xtask/src/dist/release.rs` — отложенное X-022 с явным адресом в
фазу 5.

## Вывод гейтов, дословно

### `cargo fmt --all --check`

Гоняется по всему дереву, в котором одновременно работает чужой воркер
(см. аномалию А-1), поэтому форматирование проверено по своим крейтам и
файлам:

```
$ cargo fmt -p vibe-doc-shell -p vibe-doc-server -p xtask --check
(пустой вывод, код 0)
$ rustfmt --edition 2024 <11 своих файлов в vibe-cli и vibe-doc> && повтор --check
(пустой вывод, код 0)
```

### Сборка без фичи и с фичей

```
$ cargo build -p vibe-doc-shell -p vibe-doc-server -p vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 43.53s

$ cargo xtask embed-doc-shell
embed-doc-shell: pnpm.cmd --dir …\org.vibevm.doc\web\v0.1.0 build:embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): ok
embed-doc-shell: re-pinned …\crates/vibe-doc-shell\doc-shell.lock
embed-doc-shell: 129 file(s), 802900 byte(s) under …\crates/vibe-doc-shell\shell
embed-doc-shell: template com.example.docs/fixture-manual/0.1.0/guide/every-block/index.html
embed-doc-shell: sha256 d700bac7b1d395ce1c44cb181f12abf7c1ee668b093ea7930367adce59eba570
embed-doc-shell: build `vibe` with `--features vibe-doc-shell/embedded-shell` to carry it

$ cargo build -p vibe-doc-shell -p vibe-doc-server -p vibe-cli --features vibe-doc-shell/embedded-shell
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.94s
```

### Тесты трёх крейтов

```
$ cargo test -p vibe-doc-shell -p vibe-doc-server
test result: ok. 28 passed; 0 failed; …   (vibe-doc-server, lib)
test result: ok. 16 passed; 0 failed; …   (vibe-doc-shell, lib)
test result: ok.  6 passed; 0 failed; …   (vibe-doc-server, doctests)
test result: ok.  9 passed; 0 failed; …   (vibe-doc-shell, doctests)

$ cargo test -p vibe-doc
test result: ok. 435 passed; 0 failed; …  (lib)
test result: ok.   5 passed; 0 failed; …  (doc_manifest_wire)
test result: ok.   3 passed; …  ok. 5 passed; …  ok. 28 passed; …  (интеграционные)
test result: FAILED. 52 passed; 1 failed; …  (doctests — см. аномалию А-2, не моя)

$ cargo test -p vibe-cli -p vibe-doc-server -p vibe-doc-shell     # финальный прогон по коммитам
81 тестовых бинарников, 1205 passed, 2 failed
test result: ok. 744 passed; 0 failed; …                   (vibe-cli, lib)
…
test which_reports_the_direct_source_executable_without_an_active_version ... FAILED
test ls_on_a_fresh_root_still_identifies_the_direct_source_execution ... FAILED
test result: FAILED. 2 passed; 2 failed; …
error: test failed, to rerun pass `-p vibe-cli --test vvm`
```

Две красные — **следствие правила пакета о приватном каталоге сборки**, не
правок (аномалия А-7 ниже). На панели, которая собирает в `<репозиторий>/target/`,
они зелёные.

С фичей `embedded-shell` крейт оболочки зелёный отдельно:

```
$ cargo test -p vibe-doc-shell --features embedded-shell
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok.  7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### `cargo clippy`

```
$ cargo clippy -p vibe-doc-shell -p vibe-doc-server -p vibe-cli --all-targets -- -D warnings
(ни одной строки error/warning, код 0)
```

### `cargo xtask conform check`

По новому крейту и по каждому своему периметру — ноль находок:

```
$ cargo xtask conform check --scope crates/vibe-doc-shell
conform check: 0 finding(s) in scope crates/vibe-doc-shell ({}), 0 frozen in baseline, 0 new
$ cargo xtask conform check --scope crates/vibe-doc-server
conform check: 0 finding(s) in scope crates/vibe-doc-server ({}), 0 frozen in baseline, 0 new
$ cargo xtask conform check --scope crates/vibe-cli/src/commands/doc
conform check: 0 finding(s) in scope crates/vibe-cli/src/commands/doc ({}), 0 frozen in baseline, 0 new
$ cargo xtask conform check --scope xtask/src/doc_shell.rs
conform check: 0 finding(s) in scope xtask/src/doc_shell.rs ({}), 0 frozen in baseline, 0 new
$ cargo xtask conform check --scope crates/vibe-doc
conform check: 0 finding(s) in scope crates/vibe-doc ({}), 0 frozen in baseline, 0 new
```

По всему дереву гейт красный **чужими** находками (аномалия А-1):

```
$ cargo xtask conform check
conform check: 75 finding(s) in scope <workspace> (…), 0 frozen in baseline, 17 new
Error: conform: 17 new finding(s) against the baseline
```

Семнадцать новых — `crates/vibe-core/src/manifest/package/embedded_source.rs`,
`crates/vibe-core/src/manifest/lockfile.rs`,
`crates/vibe-core/src/manifest/package/skill.rs`,
`crates/vibe-cli/src/commands/vvm/placer.rs` и соседние: периметр
bridge-коммитов, не мой (аномалия А-3). Три находки, появившиеся от моих
правок (`content_policy`, `Provenance`, `Report` без доктеста), закрыты
доктестами в этом же заходе — до них было 20.

### `cargo xtask specmap`

```
$ cargo xtask specmap
  drift: edges added: 32
  drift: edges removed: 2
specmap: wrote …\specmap.json (7930 spec units, 3574 tagged code items, 3092 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.
```

**0 suspects.** `specmap.json` я вернул в исходное состояние и **не
коммитил**: дрейф смешан — мои файлы и файлы чужого воркера (`progress-core`,
`vibe-index`, `vibe-specdoc` в момент разрезания на ячейки). Регенерация —
обычным `chore(specmap)` центральной сессией после волны, как `6d95657a`.

### `vibe facts check --exhaustive`

```
$ target/debug/vibe.exe facts check --exhaustive
progress check: clean (376 files, 24 warning(s))
```

### Панель

Три новых шага после шага 8b (floor web-пакета), все после того, как
`WEBPKG_DIR` уже определён:

| Строка | Шаг | Что делает |
|---|---|---|
| 688 | `the reader's shell matches its pin` | `vibe doc shell status`; на панельной сборке (без фичи) печатает запасную оболочку и зелёный |
| 700 | `island parity across the shell adapters` | именованный прогон теста A4.8 |
| 737 | `the local reader loads nothing from outside (browser)` | Playwright по `local-reader.spec.ts`; пропуск с причиной без Node или без бинарника |

```
$ grep -c '^run_step ' tools/self-check.sh
53          (было 50 после P4-O1)
$ bash -n tools/self-check.sh
(пусто, код 0)
```

### Floor web-пакета (после новой спеки и перегенерации типа)

```
$ typescript-ai-native floor --path vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
ℹ tests 29   ℹ pass 29   ℹ fail 0
typescript-ai-native-conform check: 0 finding(s) …, 0 new
typescript-ai-native-specmap --check: clean (… 0 suspects …)
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).
```

### Живой прогон

```
$ vibe.exe doc shell status            # сборка с фичей
shell: embedded (129 file(s), 802900 byte(s))
  built from org.vibevm.doc/web@0.1.0 at base /doc/
  measured  d700bac7b1d395ce
  recorded  d700bac7b1d395ce
  declared  d700bac7b1d395ce
  pinned    d700bac7b1d395ce
  the embedded shell matches the pin
EXIT=0

$ vibe-bare.exe doc shell status       # сборка без фичи
shell: fallback (0 file(s), 0 byte(s))
  the bare shell: typography and no scripts, which is what a build without Node ships
  `vibe doc shell install` fetches the shell this version was built with, and asks first
  a downloaded shell would be read from …\.vibe\opt\vibevm/doc-shell\d700bac7…
EXIT=0

$ vibe.exe doc serve --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --port 8471 --no-derived
$ netstat -ano | grep -E "TCP.*:8471 "
  TCP    127.0.0.1:8471         0.0.0.0:0              LISTENING       132116

$ curl -s -i http://127.0.0.1:8471/doc/ | head -3
HTTP/1.1 302 Found
location: /doc/org.vibevm.core/vibevm-docs/0.1.0/agent/how-agents-read-this-manual/
content-security-policy: default-src 'self'; script-src 'self' 'sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=' 'sha256-G1UEPJq9aJ7BX5q267nQJREuy4qJjZQUIgEbNky2cLk=' 'sha256-IPzPVHrDtvoI1ZQWhlKXR/V3yFmutugyWf9ip46eo3I=' 'sha256-Qtlx8JSU3navKLsuoanWCLoJ805RRVIZMXVEC7w+9iw=' 'sha256-iFcibSERrHqwA1Grz7KJQqO4v0iSI+KA6xf5ZqlIgTE=' 'sha256-jnN3Ms0euIKZzQsp58bwn8fOtY8ic7FGIw/FWVB9Yho=' 'sha256-vw6GxSSQaCsd1Z5NLQRJqMTb8Q/nP7OwuWIUYmev9eA='; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'

$ curl -s -I <страница> | head -5
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
cache-control: no-store
content-security-policy: … (та же, семь sha256, ноль внешних источников)
x-content-type-options: nosniff
content-length: 72447

# внешних источников в политике:
$ … | grep -coE "https?:|//[a-z]"   →  0

# страница несёт остров, заголовок своей страницы, скрипты оболочки
<title :="r5_0">How an agent reads this manual</title>
src="/doc/build/q-DZYfwtOh.js"
<link rel="alternate" type="text/markdown" href="/doc/org.vibevm.core/vibevm-docs/0.1.0/agent/how-agents-read-this-manual.md" …>
<link rel="alternate" type="application/xml" href="…/how-agents-read-this-manual.xml" …>
<link rel="alternate" type="text/plain" href="/doc/llms.txt" …>
# ни одного src=/href= со схемой:
$ grep -oE '(src|href)="https?://[^"]*"' страница  →  пусто

# статика оболочки
$ curl -s -I http://127.0.0.1:8471/doc/build/q-DZYfwtOh.js | head -3
HTTP/1.1 200 OK
content-type: text/javascript; charset=utf-8
cache-control: public, max-age=31536000, immutable

# резолвер
$ curl -s -i "…/doc/resolve?uri=spec://org.vibevm.core/vibevm-docs/agent/how-agents-read-this-manual" | head -2
HTTP/1.1 302 Found
location: /doc/org.vibevm.core/vibevm-docs/0.1.0/agent/how-agents-read-this-manual/

# обход путей, три написания
/doc/…/0.1.0/../../../../etc/passwd   -> 404
/doc/…/0.1.0/..%2F..%2Fsecret         -> 400
/doc/assets/..%5Csecret               -> 404
```

### A4.15 — браузерный прогон

```
$ VIBE_BIN=… playwright test -c site/tests/playwright.config.ts site/tests/local-reader.spec.ts
Running 3 tests using 1 worker
local-reader: 90 request(s) from 127.0.0.1:55664
  ok 1 … every request a served page makes goes back to the loopback (2.0s)
local-reader: 178 request(s) over two pages from 127.0.0.1:55664
  ok 2 … a second page of the manual loads with nothing from outside, and no policy blocks it (1.9s)
local-reader: static output made 1 request(s) from (file)
  ok 3 … the static build opened as a file makes no request of its own (1.8s)
  3 passed (11.0s)
```

**Число перехваченных запросов и их хосты:** 90 на одной странице, 178 на
двух; **единственный хост — `127.0.0.1:<порт этого прогона>`**. У статического
вывода, открытого как `file://`, — 1 запрос, схема `file:`, ни одного
сетевого. Тега аналитики нет ни там, ни там; консоль без нарушений политики.

## Размеры

| Что | Значение |
|---|---|
| `du -sh crates/vibe-doc-shell/shell` | **1.1M** (файлов 130 вместе с `shell.json`; полезная нагрузка 129 файлов, 802 900 Б) |
| Выход встраиваемой сборки целиком | 144 файла, 1.6M (из них берётся 129) |
| `vibe.exe` (debug) без фичи | **112 801 280 Б** |
| `vibe.exe` (debug) с `embedded-shell` | **113 629 184 Б** |
| Вклад оболочки | **+827 904 Б** (+0,73 %) |

Красная линия больших блобов: каталог оболочки **не коммитится**
(`/crates/vibe-doc-shell/shell/` в `.gitignore`), в историю уходит только пин;
релизный бинарник растёт на 0,8 МБ.

## R-25

Скан отчёта перед сдачей:

```
$ grep -nE '([0-9]{1,3}\.){3}[0-9]{1,3}|xray|reality|x-ui|:3300|/opt/' <отчёт> \
    | grep -vE '127\.0\.0\.(1|11)\b'
58:| `vibe doc shell install …`, `~/.vibe/opt/vibevm/doc-shell/<sha256>/` | … |
```

Одно попадание, и оно не инфраструктурное: `~/.vibe/opt/vibevm/doc-shell/` —
раскладка store версий VVM, процитированная дословно из
`##SHELL-INSTALL-COMMAND` и из самого пакета. Это путь в домашнем каталоге
продукта, не на сервере. Ни адресов, ни портов, ни имён VPN-компонентов, ни
путей сервера в отчёте нет; единственные IP — `127.0.0.1` локального
читателя. Путей вне репозитория в отчёте тоже нет (проверено отдельным
грепом по `C:\Users`, `AppData`, `/home/`).

## Диск

```
до работы:                    C:  3.7T  3.5T  236G  94% /c
на пике (перед уборкой):      C:  3.7T  3.5T  187G  96% /c
после удаления своего target: C:  3.7T  3.5T  236G  94% /c
```

Приватный `CARGO_TARGET_DIR` вырос до **46G** и удалён вместе с
каталогом пробы `PathBuf::push` (А-5). Свободное место вернулось к
исходному **236G** ровно; ошибок линковки `link.exe … 1108/1140/1201` за
всю работу не было.

## Аномалии

**А-1. Дерево перестраивалось чужим воркером три часа подряд.** В
`crates/progress-core`, `crates/vibe-index`, `crates/vibe-specdoc` и
`xtask/src/codegen` шло разрезание файлов на ячейки (коммиты `c349d2e7`,
`f8ee39ac`, `140ca684`, `e34fa5ae`, `32c4a55c`). Пока оно шло, `cargo check`
по этим крейтам давал от 3 до 15 ошибок «cannot find function … in this
scope» — модуль вынесен, `mod`/`use` ещё не дописан. Это блокировало
`cargo xtask codegen` (xtask зависит от `vibe-index`) примерно на три часа.
Ничего не чинил и ничего их не стейджил; просто ждал и перепроверял. Метод
на будущее: `cargo check -p <их крейт>` в цикле, а не «пересобрать всё».

**А-2. Доктест `vibe_doc::surface::fields::lock_fields` красный, и это не
моё.** `cargo test -p vibe-doc --doc` падает на
`crates/vibe-doc/src/surface/fields.rs:64`:
`assertion failed: fields.iter().any(|f| f == "package")`.
Проверено экспериментом: я откатил **все** свои правки в `vibe-doc`, схеме,
сгенерированных типах и корпусах (`git checkout --` по девяти файлам),
прогнал доктесты — падение то же; затем восстановил правки. Соседний
`manifest_fields` зелёный, так что дело в форме отказа serde для
`vibe_core::manifest::Lockfile` (два члена: `meta` обязателен, поэтому
«missing field `meta`» приходит раньше «unknown field»), а не в разборе
списка имён. Периметр P2-O10 (`vibe doc surface`, коммит `907948a2`);
кандидат в `BACKLOG.md` как P2.

**А-3. `conform check` по всему дереву красный семнадцатью чужими
находками.** Периметр bridge-коммитов (`7b465809` и соседи):
`embedded_source.rs`, `lockfile.rs`, `skill.rs` без доктестов,
`vvm/placer.rs:344` с `.expect()` в доменной логике. Не мой периметр, не
трогал. Свои три (появились от моих правок) закрыл доктестами.

**А-4. В отданной странице маркер острова встречается ещё раз — внутри
сериализованного состояния каркаса.** `<script type="qwik/state">` несёт
пропсы компонента, среди них строку `<!--vibe-doc-island-->`. Вклейка
заменяет **первое** вхождение (дыра в DOM идёт раньше состояния, которое
каркас пишет в конец), поэтому страница верна. Но это значит: если оболочка
перерисует остров на клиентской навигации, она возьмёт плейсхолдер и покажет
пустоту. Заменить и в состоянии нельзя — это JSON-строка, а остров несёт
кавычки. Практически невидимо, потому что клиентская навигация в локальном
читателе и так ведёт по адресам фикстуры (см. «что не сделано»); чинится
тем же самым — оболочка должна брать остров у читателя, а не из сборки.

**А-5. Дыра в собственном коде, найденная перечитыванием после коммита:
`PathBuf::push` на Windows заменяет путь, а не продолжает его.** Проверка
имени в `Shell::asset` отбрасывала `..`, `.`, `\`, `\0` и пустой сегмент —
и пропускала `C:`. Замерено пробой, не предположено:

```
          C:  ->  C:
  C:/Windows  ->  C:Windows
        /etc  ->  C:\shellroot\sub\etc
         sub  ->  C:\shellroot\sub\sub
```

(`PathBuf::from(r"C:\shellroot\sub")`, затем `push` по сегментам.) То есть
`GET <base>C:/Windows/win.ini` в лане **store** выходил из корня оболочки в
каталог, относительный текущему на томе. Встроенная лана не задета —
там точное совпадение по списку времени компиляции, — и лана распаковки
архива тоже: `fetch::plain_relative` префикс диска уже отбрасывала.

Закрыто пятым коммитом: имя теперь отбрасывает и двоеточие (ни один файл
бандлера его не несёт), а сверх имени проверяется сам **стык** —
`path.starts_with(root)` после сборки. Два вопроса, два ответа: один читает
запрос, другой читает, что из него сделала платформа.

**А-7. Правило приватного `CARGO_TARGET_DIR` красит два теста VVM, и это
свойство правила, а не кода.** `crates/vibe-cli/tests/vvm.rs`:
`which_reports_the_direct_source_executable_without_an_active_version` и
`ls_on_a_fresh_root_still_identifies_the_direct_source_execution` ждут, что
`vibe self which` / `ls` опознают «прямой запуск из исходников». Опознание —
`provenance::executable_source_root` → `source::find_source_root`, который
идёт по **предкам исполняемого файла** и ищет каталог с `Cargo.toml` **и**
`crates/vibe-cli/`:

```rust
start.ancestors().find(|dir| {
    dir.join("Cargo.toml").is_file() && dir.join("crates").join("vibe-cli").is_dir()
})
```

Бинарник, собранный в каталог вне репозитория, таких предков не имеет, и
`vibe self which` отвечает «no active version». Диагностика: сообщение теста
называет полный путь бинарника, и он ведёт в приватный каталог сборки.

Практический вывод для кампании: **правило диска и эти два теста
несовместимы**. Пакет требует приватный `CARGO_TARGET_DIR` вне дерева; эти
два теста требуют бинарник под `<репозиторий>/target/`. Панель собирает в
дерево, поэтому она зелёная. Следующему воркеру: либо гонять `--test vvm`
отдельно из дерева, либо считать эти две красные ожидаемыми и называть их в
отчёте — как здесь. Кандидат в `BACKLOG.md` (P3): тест мог бы пропускаться с
причиной, когда `CARGO_TARGET_DIR` уводит бинарник из дерева, вместо того
чтобы падать на утверждении о провенансе.

**А-6. Порядок сборок web-пакета подтверждён ещё раз.** Предупреждение
адаптера «Qwik Router SSG was skipped… Build with the Qwik CLI or
`createBuilder(config).buildApp()`» печатается на **каждой зелёной** сборке
(находка P4-O1 А-2). `cargo xtask embed-doc-shell` зовёт `pnpm build:embedded`,
то есть рабочую последовательность пакета, и не «чинит» совет адаптера.

## Что не сделано и почему

1. **Оболочка в локальном читателе показывает хром фикстуры.** Навигация,
   мета-строка, переключатель версий, блок «для агента» и адреса ссылок
   берутся из библиотеки страниц, **вкомпилированной в сборку web-пакета**
   (`site/src/fixtures/manifest.json`), а не из пакета, который отдаёт
   читатель. Заголовок вкладки и три `<link rel="alternate">` сервер правит
   (Решение 5); остальное править нечем: чтобы оболочка читала манифест
   читателя, нужен запрос `<base>manifest.json` при старте маршрута — это
   правка `site/src/routes/doc/[...path]/index.tsx` и `lib/view.ts`, а
   **web-пакет пакет править запретил**. Следующий атом: оболочка читает
   манифест по адресу своей базы (и заодно поле `media`, которое теперь
   есть). Оценка: один атом в web-пакете, без Rust.
2. **Страница пакета (`<base><координата>/<версия>/`) не рендерится** —
   по той же причине; вместо неё `302` на первую страницу (Решение 6).
3. **Релизный актив оболочки не производится** — Р-4 выше: это шаг
   публикации (R-24), расширение счёта активов — X-022, фаза 5.
4. **`specmap.json` не перегенерирован в коммит** — дрейф смешан с чужим
   (гейт specmap выше). 0 suspects проверено.
5. **`cargo fmt --all --check` и `cargo test --workspace` целиком не
   гонялись** — в дереве чужая незавершённая работа (А-1, А-3); проверено по
   своим крейтам и файлам.
6. **Предсказание 6 кампании.** «Паритет адаптеров сломается на первом
   изменении оболочки, если тест не сравнивает остров байт в байт». Тест
   сравнивает байт в байт — вычитанием шаблона из отданной страницы, а не
   поиском подстроки. Число для отчёта фазы 6: **1** тест, сравнение
   **побайтное**, на момент сдачи расхождений **0**.
