# WORKER-REPORT-P2-O12 — `wire-diff` был красен из-за унификации фич, а не из-за журнала

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O12.md`.
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

В выводах гейтов ниже абсолютный префикс рабочего дерева заменён на
`<repo>`; больше в цитатах ничего не изменено.

## Коротко для оркестратора

Лид пакета подтвердился, но не там, где его ожидали. Дело не в том, что
`zlib-rs` включается в графе `vibe-cli` и не включается в графе `xtask`
— на HEAD он включается **в обоих** (`zip` висит и на `xtask`). Дело в
том, что `vibe-index` **вообще не выбирает бэкенд**: `flate2 = "1"` с
фичами по умолчанию. Бэкенд выбирает граф сборки, а фичи только
складываются, поэтому один и тот же исходник кодировал одни и те же
5976 байт двумя разными кодеками.

Причина одной фразой: **`flate2` выбирает DEFLATE-бэкенд cfg-каскадом
по фичам, унифицированным по всему графу сборки, а `vibe-index` их не
закреплял — поэтому закоммиченный `primary.jsonl.gz` был функцией от
того, кто слинковал крейт, а не от журнала.**

Закреплено: `flate2` у `vibe-index` теперь
`default-features = false, features = ["zlib-rs"]`. Каталог
перепроецирован из **неизменённого** журнала. `cargo xtask wire-diff`
зелёный, `rebuild --check` зелёный, весь `cargo test -p vibe-index`
зелёный.

## Причина с доказательством

`flate2 1.1.9`, `src/ffi/mod.rs` — бэкенд выбирается каскадом
`any_c_zlib` > `zlib-rs` > `miniz_oxide`:

```
#[cfg(feature = "any_c_zlib")]                                   -> C zlib
#[cfg(all(not(feature = "any_c_zlib"), feature = "zlib-rs"))]    -> zlib-rs
#[cfg(all(not(feature = "any_zlib"), feature = "miniz_oxide"))]  -> miniz_oxide
```

Корневой `Cargo.toml:242` закрепляет
`zip = { version = "4", default-features = false, features = ["deflate-flate2-zlib-rs"] }`.
`zip` висит и на `crates/vibe-cli`, и на `xtask`, значит в этих графах
`flate2/zlib-rs` включён и ветка `miniz_oxide` гасится. А в графе, где
`vibe-index` собирается сам по себе, `zip` нет — и до правки включался
только `rust_backend`/`miniz_oxide`.

Измерено на этом дереве (до правки):

```
$ cargo tree -e features -i flate2 -p vibe-index
flate2 v1.1.9
├── flate2 feature "any_impl"
│   ├── flate2 feature "miniz_oxide"
│   │   └── flate2 feature "rust_backend"
│   │       └── flate2 feature "default"
│   │           └── vibe-index v1.0.0 (<repo>\crates\vibe-index)
...
$ cargo run -p vibe-index --bin vibe-index -- rebuild formats/corpora/index/e1 --check
rebuild --check: the catalog at `formats/corpora/index/e1` is byte-identical to
its journal's projection (16 file(s)); ...
```

```
$ cargo tree -e features -i flate2 -p xtask
...
│   ├── flate2 feature "any_zlib"
│   │   └── flate2 feature "zlib-rs"
│   │       └── zip feature "deflate-flate2-zlib-rs" (*)
...
$ cargo xtask wire-diff
rebuild: differs `primary.jsonl.gz`
rebuild: differs `repomd.json`
Error: rebuild --check: 2 drift item(s) ...
```

Это и есть доказательство: **один и тот же код проекции**
(`vibe_index::rebuild::check_catalog` — `xtask rebuild` только обёртка
над ним) даёт противоположный вердикт по одному и тому же каталогу в
зависимости от того, какие фичи `flate2` унифицировал граф. Содержимое
совпадает (распакованный `.gz` байт в байт равен `primary.jsonl`,
5976 Б), расходится только поток DEFLATE: `miniz_oxide` даёт 1879 Б,
`zlib-rs` — 1891 Б.

Контрольный опыт после закрепления `zlib-rs` (то же дерево, тот же
журнал, тот же уровень 6): одиночный граф `vibe-index` **перестал**
соглашаться со старым каталогом и дал ровно те же две позиции дрейфа,
что и `xtask`. То есть флип воспроизводится в обе стороны и объясняется
только бэкендом.

Отброшенные версии:

- **Версии кодеков.** `flate2 1.1.9`, `miniz_oxide 0.8.9`, `zlib-rs 0.6.7`,
  `zip 4.6.1` — одинаковы в `Cargo.lock` на `cb041f5d` и на HEAD.
- **Строка `zip` в корневом манифесте.** На `cb041f5d` она байт в байт
  та же, что на HEAD, и `zip` уже висел на `xtask`. Значит гейт был
  красен **с самого блессинга** `cb041f5d`, а не со вчерашних коммитов —
  совпадает с тем, что зелёный `wire-diff` после него нигде не записан.
- **Уровень сжатия и заголовок.** Не двигались: уровень 6, `mtime=0`,
  без имени файла; заголовок закоммиченного файла (`flags 0, mtime 0,
  xfl 0, os 255`) — ровно то, что пишет `flate2`.
- **Путь записи.** Блессинг и проверка идут через один и тот же
  `Index::write_to`; разницы «мутация на месте против scratch» нет.

## Вопрос нормы не возник

Проверка сравнивает ровно то, что по норме и должна: `primary.jsonl.gz`
— это часть **проекции** (PROP-044 §3), её размер и sha256 записаны в
`repomd.json`, и обе величины обязаны быть функцией журнала. Дефект был
не в гейте, а в том, что проекция функцией журнала не была. Поэтому это
исполнение, а не пересмотр нормы: закрепляем кодек и перепроецируем.
Режим починки в `rebuild` не добавлялся.

Почему именно `zlib-rs`, а не `rust_backend` (который сохранил бы 1879):
**фичи только складываются**. Пин `default-features = false,
features = ["rust_backend"]` не отнял бы `flate2/zlib-rs`, который
включает `zip` в тех же графах, и по каскаду выше зашёлся бы всё равно
`zlib-rs` — то есть пин был бы фикцией, а расхождение осталось бы. Пин
`zlib-rs` переживает унификацию: фича включена в **любом** графе,
содержащем `vibe-index`, ветка `miniz_oxide` гасится всегда. Остаточный
риск — если кто-нибудь когда-нибудь включит `flate2/zlib`,
`flate2/zlib-ng` или `flate2/cloudflare_zlib` (`any_c_zlib` бьёт
`zlib-rs`); сегодня в дереве таких включений нет (`vibe-index` —
единственный крейт воркспейса с прямой зависимостью от `flate2`), а
если появятся — `golden_corpus` и `wire-diff` покраснеют громко, что и
есть их работа.

## Что изменено

Один коммит, явной формой, только свои пути.

| Коммит | Subject |
|---|---|
| `c6d8c662` | `chore(corpora): reproject the index catalog on a pinned DEFLATE backend` |

- `crates/vibe-index/Cargo.toml` — `flate2 = { version = "1",
  default-features = false, features = ["zlib-rs"] }` плюс комментарий с
  разбором каскада и того, почему выбор кодека не может принадлежать
  потребителю.
- `crates/vibe-index/src/index/primary.rs` — только доки: в модуль и в
  `gzip_deterministic` добавлен четвёртый пункт детерминизма (один
  кодек), и на тест `gzip_is_deterministic` повешен комментарий о том,
  что самосогласованность внутри одной сборки — слабая половина
  свойства и именно она пропустила этот дефект.
- `formats/corpora/index/e1/primary.jsonl.gz` — 1879 → 1891 Б.
- `formats/corpora/index/e1/repomd.json` — ровно две строки: `size` и
  `sha256` записи `primary.jsonl.gz`. `generated_at`, `package_count`,
  `version_count` и все прочие записи файлов не двигались.

Журнал `formats/corpora/index/e1/state/journal/**` не тронут. `Cargo.lock`
не изменился (`zlib-rs 0.6.7` уже был ребром `flate2` в локе — новых
крейтов и сети не потребовалось).

Каталог перепроецирован **тем же проектором**, что читает гейт: replay →
fold → `write_to` в каталог корпуса. Поскольку у `rebuild` нет и не
должно быть режима починки, для одного прогона был заведён временный
тестовый файл `crates/vibe-index/tests/tmp_reproject_corpus.rs`,
выполнявший эти три вызова; он **удалён сразу после прогона** и не
попал ни в индекс, ни в коммит.

## Выводы гейтов

`cargo xtask wire-diff` — **зелёный, exit 0**:

```
rebuild --check: the catalog at `<repo>\formats/corpora/index/e1` is byte-identical to its journal's projection (16 file(s)); no fact lives in the derived artifact (PROP-044 ##FORBID-SECRET-TRUTH).
wire-diff: REPORTING (green, exit 0) — the watched wire surface shifted vs the commit while `public = false`:
wire-diff: shift classes — schema: 0, corpus: 0, other formats: 1
  wire-diff: shifted `formats/REGISTRY.toml`
wire-diff: the pre-publication regime — breaking is free and unmigrated; a break note under `formats/breaks/` is OPTIONAL until the owner declares the first public presentation (D13, PROP-044 §7).
wire-diff: green here does NOT mean «nothing changed» — it means the change is allowed unannounced. The paths above ARE different from the commit.
```

`corpus: 0` — мой сдвиг закоммичен. Оставшийся `formats/REGISTRY.toml`
— незакоммиченная правка параллельного воркера (запись
`[format.doc-site-config]`), не моя.

`cargo test -p vibe-index` — **весь пакет зелёный**, включая
`golden_corpus` и `rebuild_cli`; ни одного `failed` ни в одной цели:

```
     Running tests\golden_corpus.rs
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
     Running tests\rebuild_cli.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.67s
```

(Аномалия 1 из P2-O11 — что `golden_corpus` краснеет во время чужой
перегенерации кодогена — в этот прогон не сработала: чужая правка
кодогена ещё не дошла до генерируемых wire-типов, которые читает этот
тест.)

`cargo fmt -p vibe-index --check` — **чисто**, пустой вывод, exit 0.

`cargo xtask check-codegen` — **красен, и это не моё**. Названы ровно три
файла:

```
crates/vibe-wire/src/generated/format_id/mod.rs
crates/vibe-wire/src/generated/format_id/properties.rs
crates/vibe-wire/src/generated/mod.rs
```

Все три — чужие и в моём коммите не участвуют. Дрейф ровно один: во всех
трёх не зарегистрирован `doc_site_config`, которого требуют
**незакоммиченные** `schemas/doc_site_config.jtd.json` (untracked) и
правка `formats/REGISTRY.toml` параллельного воркера. Мои четыре пути
входами кодогена не являются вовсе, так что этот красный к P2-O12
отношения не имеет и погаснет, когда тот воркер прогонит
`cargo xtask codegen` и закоммитит результат.

Сетевые тесты `vibe-cli` не гонялись.

## Чего не сделал и почему

- **Не добавлял golden-тест на точные байты `gzip_deterministic`.** После
  пина граф тестов `vibe-index` совпадает с графом `xtask`, поэтому
  `golden_corpus` наконец ловит смену бэкенда сам — отдельный
  байтовый golden дублировал бы его. Ограничился комментарием на
  существующем тесте.
- **Не чинил `check-codegen`.** Причина целиком в чужих незакоммиченных
  файлах; регенерация застейджила бы `crates/vibe-wire/**` параллельного
  воркера.
- **Не трогал корневой `Cargo.toml`.** Строка `zip` осталась как была:
  закрепления кодека у `vibe-index` достаточно, а сужать чужую
  зависимость без нужды — лишний сдвиг.

## Отклонение от пакета

Пакет предполагал два действия — закрепить бэкенд и перебластить корпус —
и назвал для корпуса коммит `chore(corpora): reproject the index catalog …`.
Я сложил оба в **один** коммит с этим subject'ом: по отдельности первый
из них оставил бы дерево красным (пин без перепроекции — это ровно
сегодняшний дрейф), а второй без первого не имел бы смысла. Прецедент
такой атомарности в дереве есть: `cb041f5d` тоже вёз изменение кода и
перегенерацию корпуса одним коммитом.
