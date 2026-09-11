# WORKER-REPORT-P0-O3

Пакет: P0-O3 «HTTP-сервер локального читателя и встраивание оболочки»
(спайки A0.7 и A0.11). Дата: 2026-09-11. Дерево: `b1291b06`.

Находки:
- `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.7-http-server.md`
- `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.11-shell-embedding.md`

Пробные крейты (вне репозитория):
`<scratch>/vibe-docs-phase0\P0-O3\`
— `probe/` (сервер + фича `embedded-shell`), `embedcmp/` (сравнение
библиотек встраивания), `sizecmp/{base,idir,rembed}/` (размер и время
сборки). У каждого свой `CARGO_TARGET_DIR` в scratch; `cargo` в дереве
репозитория не запускался.

## Решения

1. **Сервер — отдельный тонкий крейт `vibe-doc-server`**, не модуль внутри
   `vibe-doc`. Из `vibe-index` переиспользуется форма (роутер, тип ошибки,
   `oneshot`-тесты), а не код: его `AppState` держит `RwLock<Index>`, и
   зависимость притащила бы `reqwest`, `flate2`, `vibe-wire`. Модуль внутри
   `vibe-doc` затащил бы axum + hyper + tower в генератор сайта и в MCP
   (замер чистой сборки этого набора — 18,7 с). Раскладка: `vibe-doc`
   (логика) → `vibe-doc-shell` (байты оболочки, крейт назван D-12 п. 1) →
   `vibe-doc-server` (axum).
2. **Встраивание — `include_dir` 0.7.4**, а не `rust-embed` 8.12.0. Решающий
   факт: `rust-embed` в debug-сборке читает файлы с диска в рантайме (если
   не включён `debug-embed`) — измерено, бинарник не самодостаточен. Плюс
   4 транзитивных крейта против 23 и 2,95 с чистой сборки против 6,53 с.
   Обе библиотеки MIT, copyleft в графах нет. Альтернатива приемлема:
   `rust-embed` с обязательными `features = ["debug-embed",
   "deterministic-timestamps"]`.
3. **`build.rs`-гейт обязателен независимо от выбора библиотеки**: ни
   `include_dir`, ни `rust-embed` не отличают пустой каталог от полного
   (обе молча собираются с нулём файлов) и ни одна не заказывает пересборку
   при изменении каталога. `cargo:rerun-if-changed=shell` + проверка на
   `index.html` + понятное сообщение — макет в находке A0.11.
4. **Оболочка в релизе — отдельный актив `vibevm-doc-shell-<version>.zip`
   с собственным манифестом `DOC-SHELL.json`**, а не новое поле в
   `DISTRIBUTIONS.json`: `AggregateDistributionManifest` и
   `PlatformDistributionFragment` объявлены `deny_unknown_fields`, а
   `validate_schema` принимает ровно версию 1 — новое поле сломало бы
   `self update` у всех установленных `vibe 1.0.0`. Структура релизного ZIP
   тоже закрытое множество, положить файл внутрь бандла нельзя.
5. **Оболочка кладётся в `~/.vibe/opt/vibevm/doc-shell/<sha256>/`** — рядом
   с `build/` и `src/`, а не внутрь каталога экземпляра: экземпляры
   неизменяемы (PROP-019 §2.4). Пин (D-12 п. 5) живёт двумя слоями:
   константа `VIBE_DOC_SHELL_SHA256` в бинарнике и `DOC-SHELL.lock.toml`,
   записанный в экземпляр во время стейджинга, то есть при его рождении.

## Что найдено сверх задания (важно)

**Обход путей в HTTP-сервере `vibe-index`.** Маршруты
`/v1/index/by-name/{name}`, `by-cap/{slug}`, `by-purl/{slug}`
(`crates/vibe-index/src/server/routes/index_files.rs:80-126`) джойнят
percent-декодированный захват с `data_dir` без проверки. axum-роутер
сопоставляет маршрут по сырому пути, а экстрактор `Path` декодирует `%2F`
уже после матчинга, поэтому один сегмент может содержать разделитель.
На идентичном по форме маршруте в пробном крейте с той же версией axum
(0.8.9) запрос `/by-name/..%2Fsecret.txt` вернул **200 и содержимое файла
вне корня**. В `vibe-index` выход ограничен расширением (обработчик требует
суффикс `.json` / `.jsonl` до джойна), то есть читается любой `*.json`
относительно data-dir — но это всё равно выход за корень, а сервер может
быть запущен не только на петле (`--bind` принимает любой адрес).

На живом `vibe-index` не подтверждено: бинарника
`target/debug/vibe-index.exe` в дереве нет, а `cargo build` внутри
репозитория пакетом запрещён. Проверка — один тест на
`oneshot(req(GET, "/v1/index/by-name/..%2F..%2Fsecret.json"))` в
`crates/vibe-index/tests/server_e2e.rs`. Кандидат в `BACKLOG.md`, severity
P1 по форме дерева. Локальный читатель эту форму повторять не должен —
рабочие механизмы (ручной лексический join и `ServeDir`) измерены и описаны
в A0.7.

## Отклонения

Отклонений от пакета нет. Два уточнения о ходе работы:

- Пакет допускал сетевую установку пакетов внутри scratch-проекта, и она
  понадобилась: `rust-embed` и `include_dir` в локальном кэше cargo
  отсутствовали (`axum 0.8.9` и `tower-http 0.6.11` были). Установлены
  только в scratch-проект, глобально ничего не ставилось, PATH и реестр не
  трогались.
- Пакет просил макеты `build.rs` и `lib.rs` «не больше 60 строк каждый».
  `build.rs` — 31 строка, приведён целиком. Файл `probe/src/shell.rs` — 72
  строки; в находку вошли строки 1-57, а функция `safe_join` (строки 59-72)
  приведена целиком в находке A0.7, чтобы не дублировать и уложиться в лимит.
- Дерево репозитория не изменялось, git не использовался, кроме
  `git status --short`.

## Вывод самопроверки

### 1. Сборка пробного крейта без фичи (каталога `shell/` нет)

```
### SC1a: cargo build (no feature, no shell dir)
   Compiling docshell-probe v0.0.0 (<scratch>/vibe-docs-phase0\P0-O3\probe)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.14s
EXIT=0
```

### 2. Сборка с фичей и пустым каталогом (сообщение дословно)

```
### SC1b: cargo build --features embedded-shell (empty shell dir)
   Compiling docshell-probe v0.0.0 (<scratch>/vibe-docs-phase0\P0-O3\probe)
error: failed to run custom build command for `docshell-probe v0.0.0 (<scratch>/vibe-docs-phase0\P0-O3\probe)`

Caused by:
  process didn't exit successfully: `<scratch>/vibe-docs-phase0/P0-O3/target\debug\build\docshell-probe-aa892dddf3c56e88\build-script-build` (exit code: 1)
  --- stdout
  cargo:rerun-if-changed=shell
  cargo:rustc-env=DOCSHELL_DIR=<scratch>/vibe-docs-phase0\P0-O3\probe\shell

  --- stderr
  error: feature `embedded-shell` is on, but the built doc shell is missing: `<scratch>/vibe-docs-phase0\P0-O3\probe\shell\index.html` does not exist.
  note: the shell is produced by `cargo xtask embed-doc-shell` (pnpm build with the embeddable adapter).
  fix: run `cargo xtask embed-doc-shell` before a release build, or build without `--features embedded-shell` to get the fallback bare shell.
EXIT=101
```

### 3. Сборка с фичей и заполненным каталогом

```
### SC1c: cargo build --features embedded-shell (populated shell dir)
   Compiling docshell-probe v0.0.0 (<scratch>/vibe-docs-phase0\P0-O3\probe)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.03s
EXIT=0
```

### 4. Заголовки пробного сервера

```
### SC2: curl -sI http://127.0.0.1:51585/
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
content-security-policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
x-content-type-options: nosniff
content-length: 103
date: Fri, 11 Sep 2026 20:22:33 GMT

EXIT=0
```

### 5. Файлы находок

```
### SC3: ls findings
findings/A0.11-shell-embedding.md
findings/A0.7-http-server.md
EXIT=0
```

### 6. Состояние рабочего дерева

```
### SC4: git status --short
 M .claude/agents/opus5.md
EXIT=0
```

Единственное изменение — `.claude/agents/opus5.md`, которое пакет назвал
чужим. Ни одного файла репозитория работа не трогала.

## Что не сделано и почему

- Обход путей в `vibe-index` не подтверждён на живом бинарнике: его нет в
  `target/`, а сборка в дереве репозитория запрещена пакетом. Доказательство
  получено на идентичном по форме маршруте в пробном крейте с той же версией
  axum.
- Размер собранной оболочки Qwik не оценён: web-пакета ещё нет, измерена
  только накладная стоимость кода библиотеки встраивания (+1 КиБ к
  `include_str!`).
- Наличие и пины Node/pnpm для `cargo xtask embed-doc-shell` не проверялись:
  пакет этого не просил, и установка чего-либо глобально запрещена.
- Секретов не встречено; каталоги из R-28 и `infra` не открывались.
