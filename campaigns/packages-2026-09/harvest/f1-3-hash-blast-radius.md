# Ф1.3 — радиус поражения смены вида content-хэша: замер

Замер читает дерево по состоянию ветки `wt/F1-3-PROBE` (HEAD `5289cdb5`).
Периметр чтения — `crates/**` (`vibedeps/**` и `packages/**` не читались: там
нет host-крейтов). Ни одного запуска `cargo`/`vibe`/`git` не делалось; выводы
построены на чтении исходников. Все цитаты — `файл:строка` относительно корня
worktree.

Термин в этом документе: **content-хэш** — значение поля `content_hash` пакета
(идентичность `(group, name, version, content_hash)`, PROP-002 §2.1). Это
НЕ контрольные суммы файлов каталога (`repomd.json::files[*].sha256`,
`sha256_of_bytes`, `primary.rs`), и НЕ хэш текста spec-юнитов в `progress-core`
(`content_hash(text)`, PROP-043). Эти два класса помечены «НЕ ОТНОСИТСЯ» и в
радиус шага не входят.

## 1. ВЕРДИКТ

**ДА С ОГОВОРКАМИ.** При буквальном прочтении посылки — «сменить эмиссию только у
`vibe-index`, больше ничего» — две формы `sha256-tree/1:` и `sha256:` **не
встречаются ни в одном сравнении**: значение из каталога индекса (`VersionEntry`)
вообще не доходит до локфайла и до сравнений, потому что клиент индекса
`wire.rs` не читает поле `content_hash` (см. §5). НО эта посылка **неустойчива**:

1. Две копии tree-walk-алгоритма связаны документированным контрактом
   «MUST stay in lockstep» (`shippable.rs:15-18`, `content_hash.rs:23-28`) — шаг,
   меняющий `vibe-index`, обязан менять и `vibe-registry`, иначе «пакет,
   проиндексированный здесь и материализованный там, хэшируется по-разному».
2. Периметр правки из §8 пакета и сам включает `vibe-registry`. Как только
   меняется реестровая копия — две формы **встречаются ровно в двух
   продакшн-сравнениях**: `vibe-check/src/checks/local_source_freshness.rs:92`
   и `vibe-registry/src/git_package_registry/fetch.rs:385` (оба — свежевычисленный
   хэш против хэша, сохранённого в локфайле). Это ядро риска, см. §4 и §10.
3. Отдельно: «паритет» B5 — это **замороженная константа на стороне `vibe-index`
   только**, а не живая перекрёстная проверка двух реализаций; расхождение
   `vibe-index`/`vibe-registry` этим сторожем поймано НЕ будет (§2, B5).

Итог: в реалистическом сценарии (меняются обе копии алгоритма, как требует
контракт) — формы встречаются, и ровно в двух местах; в буквальном
«только-индекс» — не встречаются, но такой шаг молча ломает контракт и
остаётся незамеченным.

## 2. Сверка опорных координат (B1..B5)

| # | утверждение | вердикт | цитата file:line |
|---|---|---|---|
| B1 | реализаций tree-walk-алгоритма ровно ДВЕ, с одинаковой `SHIPPABLE_EXCLUDES` | ПОДТВЕРЖДЕНО С УТОЧНЕНИЕМ | `vibe-index/src/content_hash.rs:40` (`pub fn compute_content_hash`), `vibe-registry/src/shippable.rs:77` (он же); `SHIPPABLE_EXCLUDES` — `content_hash.rs:28` и `shippable.rs:18` (идентичны). grep `fn compute_content_hash` по `crates/**` = ровно 2 совпадения. Уточнение: B1 говорит про tree-walk, но у content-хэша есть **третий** продюсер `sha256:` — `commit_content_hash` (`fetch.rs:484`), commit-derived, не tree-walk; для in-place пакетов именно он пишет `content_hash` в локфайл (§3). |
| B2 | newtype `ContentHash`, `PREFIX="sha256:"`, проверка префикса на границе десериализации через `serde(try_from/into)` | ПОДТВЕРЖДЕНО | `vibe-core/src/content_hash.rs:42` (`pub struct ContentHash(String)`), `:48` (`PREFIX = "sha256:"`), `:41` (`#[serde(try_from = "String", into = "String")]`), проверка — `parse` `:54-60` через `strip_prefix(Self::PREFIX)`. Всякая десериализация значения с иным префиксом падает в `BadContentHash` (`:56`). |
| B3 | `from_validated` обходит проверку; используется в `record.rs:193` и `update.rs:398` | ПОДТВЕРЖДЕНО | `vibe-core/src/content_hash.rs:72` (`pub fn from_validated(hash: String) -> Self` — без `parse`). grep `from_validated` по `crates/**`: `ContentHash::from_validated` ровно в `vibe-install/src/record.rs:193` и `vibe-cli/src/commands/update.rs:398` (прочие `from_validated` — `PackageName`/`CapabilityNamespace`, не `ContentHash`). |
| B4 | `ContentHash` выводит `PartialOrd, Ord` — сравнение строковое, смена префикса меняет порядок сортировки | ПОДТВЕРЖДЕНО | `vibe-core/src/content_hash.rs:40` (`#[derive(..., PartialOrd, Ord, ...)]`). Но см. §6: нигде по `ContentHash` фактически **не сортируют** и в ключ он не входит — `Ord` латентен. |
| B5 | паритет копий сторожит `content_hash_parity.rs` с `GOLDEN`, фикстура — 6 файлов | ПОДТВЕРЖДЕНО С УТОЧНЕНИЕМ | `vibe-index/tests/content_hash_parity.rs:25` (`GOLDEN = "sha256:e10a49…"`, r=1 verifies); фикстура `crates/vibe-index/fixtures/golden-flow-wal-0.1.0/` — `find -type f` = **6 файлов**. Уточнение (важно): этот тест вызывает **только** `vibe_index::compute_content_hash` (`:23,:41`) и сравнивает с константой `GOLDEN` (`:42-43`). Он **не вызывает** `vibe-registry` и не сравнивает две реализации между собой. Это замораживает `vibe-index` относительно `GOLDEN`, но НЕ ловит расхождение `vibe-index`↔`vibe-registry`. |

## 3. Откуда берётся значение: локфайл и каталог

### 3.1 Локфайл `vibe.lock`

Поле `LockedPackage.content_hash` имеет тип **`ContentHash`** (не `String`):
`vibe-core/src/manifest/lockfile.rs:294`. Значит при **чтении** локфайла
значение всегда проходит `ContentHash::parse` (проверка префикса) —
через `serde(try_from = "String")` (`content_hash.rs:41`→`:104-106`→`:54`).
Локфайл с `sha256-tree/1:` сегодня **не распарсится**.

Цепочка **записи** значения в локфайл (общий случай):

```
vibe-registry::compute_content_hash(&cache_dir)   shippable.rs:77 (->:106 format!("sha256:{hex}"))
        │  (вызывают: git_registry.rs:215, local_registry.rs:211,
        │   multi_registry_resolver/{dispatch.rs:159, redirect_follow.rs:292,
        │   sources.rs:248, sources.rs:315}, git_package_registry/fetch.rs:351)
        ▼
CachedPackage.content_hash : String               vibe-registry/src/lib.rs:205
        │
        ▼
ContentHash::from_validated(c.content_hash.clone())  vibe-install/src/record.rs:193
        │  (БЕЗ parse — обход проверки префикса)
        ▼
LockedPackage.content_hash : ContentHash          vibe-core/src/manifest/lockfile.rs:294
        │
        ▼
Lockfile::write -> write_toml                     vibe-core/src/manifest/lockfile.rs:454-456
```

Зеркальный путь в `vibe-cli` (update): `update.rs:398`
`ContentHash::from_validated(cached.content_hash.clone())`.

**Откуда ЛЕВАЯ (значение) берётся — три продюсера, по условию выбора:**

1. **Локально, по материализованному дереву** (общий случай): реестровый
   резолвер материализует пакет в `cache_dir` и считает
   `vibe-registry::compute_content_hash(&cache_dir)` — tree-walk. Это путь для
   `Registry`/`Git`/`Path`/`Embedded`/`Local` (копирующая материализация).
   Решающий сайт: `git_package_registry/fetch.rs:351`
   `(dest_cache.clone(), compute_content_hash(&dest_cache)?)`.
2. **Принимается как данность, commit-derived** (in-place материализация): для
   пакета с `materialization.is_in_place()` значение считает
   `commit_content_hash(resolved_commit)` — `fetch.rs:341`→`:484-492`,
   `format!("sha256:{hex}")` от SHA-256 коммита. Это **другой алгоритм**
   (не tree-walk), и шаг по «рецепту tree-walk» его по смыслу **не касается** —
   in-place пакеты и после шага несут `sha256:<commit>`. Документация:
   `vibe-registry/src/lib.rs:329-333`.
3. **Переносится из старого локфайла** (инкрементальный in-place): провижн-запись
   берёт `old.content_hash.as_str().to_string()` —
   `vibe-install/src/plan/fetch.rs:125`.

**Условие выбора** между (1) и (2) — `pkg.materialization.is_in_place()`
(`fetch.rs:338`): in-place → `commit_content_hash`; иначе (copy) →
`compute_content_hash(&dest_cache)`.

Вывод: значение в локфайле для общего случая **вычисляется локально по
материализованному дереву** реестровой копией алгоритма, а **не** принимается из
каталога индекса. Между локфайлом и каталогом индекса **нет прямой связи по
`content_hash`** — их пишут разные копии алгоритма (§3.2). Граничный конвертор
«нормализовать вид при записи в локфайл» в коде **отсутствует**: `record.rs:193`
передаёт строку как есть через `from_validated`.

### 3.2 Каталог индекса (`VersionEntry`)

Поле `VersionEntry.content_hash` имеет тип **`String`** (не `ContentHash`):
`vibe-index/src/types/entry/mod.rs:55`. Контроль префикса при десериализации
**отсутствует** — значение с любым префиксом парсится как строка.

Цепочка:

```
vibe-index::compute_content_hash(pkg_root)       content_hash.rs:40 (->:69 format!("sha256:{hex}"))
        │  (эмиттеры: cli/add.rs:73, scanner/org_walk.rs:193)
        ▼
VersionEntry.content_hash : String               vibe-index/src/types/entry/mod.rs:55
        │
        ▼  (серилизация в primary.jsonl / by-name/<name>.json / POST /v1/packages)
каталог индекса на диске и HTTP-ответ сервера
```

Каталог пишет **только** индексная копия алгоритма
(`vibe-index::compute_content_hash`). Это значение **никуда дальше не
передаётся** в смысле локфайла/реестра: клиент индекса `wire.rs` его не читает
(§5). Поэтому смена эмиссии индекса изолирована внутри подсистемы индекса — до
тех пор, пока не меняется реестровая копия.

## 4. Все строковые сравнения хэша

Методика: `rg` по `crates/**` шаблонов `content_hash\s*(!=|==)`, `(!=|==)\s*[^/]*content_hash`,
`starts_with\("sha256`, `contains\("sha256`, плюс структурное `assert_eq!` над
`Lockfile`/`LockedPackage` (транзитивно через `derive(PartialEq)` в
`lockfile.rs:77`). После фильтрации не-релевантных классов (repomd-контрольные
суммы; `progress-core` text-hash) получен исчерпывающий список ниже. Колонки по
вопросу 3 пакета.

### 4.1 ПРОДАКШН-код (не-тест) — места, где формы могут встретиться

| файл:строка | что с чем сравнивается | откуда ЛЕВАЯ | откуда ПРАВАЯ | сломается? | почему |
|---|---|---|---|---|---|
| `vibe-check/src/checks/local_source_freshness.rs:92` | `fresh != pkg.content_hash.as_str()` — пересчитанный хэш источника против записи локфайла | `fresh` = `vibe_registry::compute_content_hash(&src)` (recompute, `:75`, import `:20`) | `pkg.content_hash.as_str()` — поле `ContentHash` из прочитанного локфайла (`:40` `Lockfile::read`) | **ДА** (при смене реестровой эмиссии или при рассинхроне «старый локфайл ↔ новый эмиссер») | строковое `!=`. Если эмиссер стал `sha256-tree/1:`, а локфайл (существующий) несёт `sha256:` — всегда `!=` → ложный warning «источник изменился» на каждом local-пакете. |
| `vibe-registry/src/git_package_registry/fetch.rs:385` | `Some(expected) if expected == content_hash` — cross-source content_hash gate (зеркало принимается, если его свежий хэш совпал с пином) | `expected` = параметр `expected_hash: Option<&str>` (`:278`) — **пин из локфайла** (`vibe-cli/src/commands/install/resolver.rs:67-68,87,142`) | `content_hash` = свежевычисленный: copy → `compute_content_hash(&dest_cache)` (`:351`), in-place → `commit_content_hash` (`:341`) | **ДА** для copy-пути (tree-walk ↔ пин локфайла); **НЕТ** для in-place (обе стороны commit-derived `sha256:`) | при расхождении форм copy-гейт **никогда** не совпадает → зеркало всегда бракуется как «disagreeing», integrity-gate молча отключается (см. тест `fetch/tests.rs:362 returns_last_attempt_when_no_match`). |

### 4.2 Тесты — `.starts_with("sha256:")` над РЕАЛЬНЫМ выводом `compute_content_hash`

| файл:строка | что с чем | ЛЕВАЯ | ПРАВАЯ | сломается? | почему |
|---|---|---|---|---|---|
| `vibe-index/src/content_hash.rs:143` | `h.starts_with("sha256:")` | `h` = `compute_content_hash` (`:142`) | литерал | **ДА** | эмиссия; периметр правки |
| `vibe-index/src/content_hash.rs:144` | `h.len() == 7 + 64` | `h` = `compute_content_hash` | литерал длины | **ДА** | `sha256-tree/1:` даёт 13+64≠71; периметр |
| `vibe-index/src/content_hash.rs:85` | `h == "sha256:e3b0c44…"` (empty-dir golden) | `compute_content_hash` пустого каталога (`:81`) | литерал | **ДА** | эмиссия; периметр |
| `vibe-publish/tests/post_hook.rs:154-158` | `body["content_hash"].starts_with("sha256:")` | тело POST к индексу, хэш из `vibe_registry::compute_content_hash` (`post_hook.rs:28,:185`) | литерал | **ДА** | **вне периметра** (vibe-publish) → это хвост Q8 |
| `vibe-cli/tests/cli_live_e2e.rs:255,256` | `pkg.content_hash.starts_with("sha256:")` | resolved-пакет из реестра (реестровая compute) | литерал | **ДА** (условно) | **вне периметра**, но тесты `#[ignore]` (`cli_live_e2e.rs:106,151,197`) — в дефолтном `cargo test` не бегут |
| `vibe-registry/tests/registry_cells_oracle.rs:71,194` | `cached.content_hash.starts_with("sha256:")` | `CachedPackage` (реестровая compute) | литерал | **ДА** | периметр |
| `vibe-registry/src/git_package_registry/fetch/tests.rs:55` | `cached.content_hash.starts_with("sha256:")` | copy-вычисление (`compute_content_hash`) | литерал | **ДА** | периметр |
| `vibe-registry/src/git_package_registry/fetch/tests.rs:268` | то же | copy-вычисление | литерал | **ДА** | периметр |
| `vibe-registry/src/git_package_registry/fetch/tests.rs:406` | `cached.content_hash.starts_with("sha256:")` (посль `bogus_pin`) | copy-вычисление | литерал | **ДА** | периметр |
| `vibe-registry/src/git_package_registry/fetch/tests.rs:536` | `cached.content_hash.starts_with("sha256:")` | **in-place** = `commit_content_hash` (comment `:535` «commit-derived, not a tree walk») | литерал | **НЕТ** | шаг не меняет commit-derived продюсер |
| `vibe-registry/src/local_registry.rs:429` | `cached.content_hash.starts_with("sha256:")` | реестровая compute | литерал | **ДА** | периметр |
| `vibe-registry/src/multi_registry_resolver/sources/tests.rs:295` | `cached.content_hash.starts_with("sha256:")` | реестровая compute | литерал | **ДА** | периметр |
| `vibe-registry/src/multi_registry_resolver/tests.rs:127` | `cached.content_hash.starts_with("sha256:")` | реестровая compute | литерал | **ДА** | периметр |
| `vibe-registry/src/shippable.rs:75` | доктест `hash.starts_with("sha256:")` | `compute_content_hash` (доктест) | литерал | **ДА** | периметр |

### 4.3 Тесты — сравнение с локфайлом (по посылке локфайл остаётся `sha256:` → НЕ ломаются)

| файл:строка | что с чем | ЛЕВАЯ | ПРАВАЯ | сломается? | почему |
|---|---|---|---|---|---|
| `vibe-cli/tests/cli_pkg_cycle.rs:128` | `lock.packages[0].content_hash.starts_with("sha256:")` | локфайл (после реального `vibe install`) | литерал | **НЕТ** | локфайл остаётся `sha256:` (если реестровая эмиссия не менялась); вне периметра |
| `vibe-cli/tests/cli_registry_mgmt.rs:189` | `entry.content_hash.starts_with("sha256:")`, где `entry = &lock.packages[0]` (`:186`) | локфайл | литерал | **НЕТ** | тот же аргумент; вне периметра |

### 4.4 Тесты — точное `==` над синтетикой/фикстурой (НЕ ломаются)

| файл:строка | что с чем | ЛЕВАЯ | ПРАВАЯ | сломается? | почему |
|---|---|---|---|---|---|
| `vibe-index/tests/server_e2e.rs:362` | `body["content_hash"] == "sha256:wal0.2.0"` | ответ сервера, VersionEntry засеяно `format!("sha256:{name}{version}")` (`server_e2e.rs:40`) | литерал | **НЕТ** | синтетический round-trip; `compute_content_hash` не участвует |
| `vibe-registry/tests/index_fast_path.rs:132` | mock-JSON `"content_hash": "sha256:0000"` | мок каталога | литерал | **НЕТ** | мок; старая форма всё равно принимается |
| `vibe-mcp/tests/tools_oracle.rs:316` | `out["content_hash"] == "sha256:deadbeef"` | чтение `LOCKFILE_FIXTURE` (`tools_oracle.rs:29`, `:309`) | литерал | **НЕТ** | фикстура локфайла остаётся `sha256:`; вне периметра |

### 4.5 Тесты — структурное/транзитивное сравнение (через `derive(PartialEq)`)

| файл:строка | что с чем | ЛЕВАЯ | ПРАВАЯ | сломается? | почему |
|---|---|---|---|---|---|
| `vibe-core/src/manifest/lockfile/tests.rs:93,106,178,261` | `assert_eq!(lf, back)` — round-trip parse→write→read целого `Lockfile` | распарсенный TOML | перечитанный TOML | **НЕТ** | обе стороны — фикстура `sha256:abc`; `compute_content_hash` не участвует; периметр |
| `vibe-registry/src/git_package_registry/fetch/tests.rs:405` | `assert_ne!(cached.content_hash, bogus_pin)`, `bogus_pin = "sha256:000…"` (`:395`) | copy-вычисление | литерал | **НЕТ** | `assert_ne` держится при любой форме (новая форма тем более ≠ `sha256:000…`) |

### 4.6 НЕ ОТНОСИТСЯ (исключено из радиуса шага)

- `progress-core/**` (`cache.rs:193,221`, `sidecar.rs:230`, `seal.rs:152,275`,
  `baseline.rs:298`, `baseline/project.rs:263`) — это **другой** `content_hash`:
  SHA-256 текста spec-юнита (`parse/mod.rs:42`, `doc.rs:92`, PROP-043). К
  пакетному content-хэшу отношения не имеет.
- Контрольные суммы файлов каталога: `sha256_of_bytes`
  (`vibe-index/src/index/persistence.rs:60-61,:120`), поля
  `RepomdFileEntry.sha256` (`types/repomd.rs:53`), `primary.rs:190,192` — это
  per-file чексуммы, отдельная схема от пакетного `content_hash`.

Полнота: других операторов сравнения пакетного content-хэша
(`==`/`!=`/`starts_with`/`contains`/структурное `assert_eq`/как ключ коллекции)
по `crates/**` нет — grep по перечисленным шаблонам пуст сверх вышеприведённого.

## 5. vibe-check и клиент индекса

### 5.1 `vibe-check/src/checks/lockfile_files.rs`

Литералы `sha256:00`/`sha256:abc` (`:154,:211,:250`) встречаются **только в
тестовых фикстурах** локфайла. Продакшн-метод `run` (`:24-113`) **не парсит, не
сравнивает, не печатает и не конструирует** хэш: он проверяет лишь наличие
каталогов `vibedeps/<group>.<name>/<version>/` (`:46-47`) и ловит
orphan-слоты/`embedded`-записи. Литерал `sha256:` здесь — инертный наполнитель,
нужный лишь чтобы тестовый `vibe.lock` прошёл `ContentHash::parse` (поле
обязательное). **К радиусу шага отношение не имеет.**

### 5.2 `vibe-check/src/checks/local_source_freshness.rs`

Это **главное опасное место** (см. §4.1). Продакшн-логика:
- `:75` `let fresh = match compute_content_hash(&src)` — пересчёт через
  `vibe_registry::compute_content_hash` (import `:20`);
- `:92` `if fresh != pkg.content_hash.as_str()` — **строковое** сравнение
  пересчитанного хэша с записью локфайла.

Решающие строки дословно:

```rust
// local_source_freshness.rs:75
let fresh = match compute_content_hash(&src) {
    Ok(h) => h,
    Err(e) => { /* warn … ; continue */ }
};
// local_source_freshness.rs:92
if fresh != pkg.content_hash.as_str() {
    report.warn( CheckId::LocalSourceFreshness, /* … "its local source changed since install
        (recorded content_hash {}, source now {fresh}) — run `vibe install --assume-yes`" */ );
}
```

Тесты этого файла от смены эмиссии **не падают** (обе стороны — одна и та же
функция): `:185` записывает в локфайл `compute_content_hash(&src)`, `:75`
пересчитывает её же → равенство сохраняется; тест `:207` пишет литерал
`"sha256:deadbeef"` и ожидает дрейф — дрейф и остаётся. **Ловится только
продакшн-поведение** на существующем локфайле, что тесты не моделируют.

### 5.3 Клиент индекса `vibe-registry/src/index_client/wire.rs`

Клиент **не читает `content_hash` из каталога**. Wire-типы:
`VersionEntryView { version }` — единственное поле (`wire.rs:30-33`);
`NameEntryView`/`PackageEntryView` (`:17-28`), `SearchHit` (`:55-67`),
`PurlLookupHit` (`:83-89`) поля `content_hash` **не имеют**. grep `content_hash`
по `index_client/**` = 0 совпадений. Значит `VersionEntry.content_hash`
(будь то `sha256:` или `sha256-tree/1:`) **не доходит** ни до локфайла, ни до
реестра, ни до какого-либо сравнения через клиент. Это и даёт «НЕТ» в
буквальном сценарии «только-индекс» (§1). Значение проходит как `String`
(`VersionEntry.content_hash: String`, `entry/mod.rs:55`), не через `ContentHash`.

## 6. Порядок и упорядоченные коллекции

`ContentHash` выводит `PartialOrd, Ord` (`content_hash.rs:40`) — порядок
строковый, и смена префикса формально его меняет. Поиск фактического
использования:

- `rg "ContentHash[,>]|BTreeMap<ContentHash|HashMap<ContentHash|sort_by_key.*content_hash"` по `crates/**` → только импорт (`record.rs:9`), `From<ContentHash>` (`content_hash.rs:95`) и декларация поля (`lockfile.rs:294`). **Ни одного использования `ContentHash` как ключа `HashMap`/`BTreeMap`/`BTreeSet` или в `sort_by_key`.**
- `LockedPackage` лежит в `Vec` (`lockfile.rs:83`); поиск/удаление — по `(group, name)` (`:459-478`), **не** по хэшу; дубликаты детектятся через `BTreeSet<(&Group, &String)>` от `(group, name)` (`:441-450`).
- `VersionEntry::sort_key()` возвращает `(&Group, &str, &Version)` — `(group, name, version)` (`entry/mod.rs:174-176`), **без** `content_hash`.

**Вывод:** нигде по `ContentHash` не сортируют и в упорядоченный ключ он не входит.
Вывод `Ord` — **латентный** риск (если будущий код станет сортировать по хэшу,
смена префикса перевернёт порядок), но сегодня он не exercised. Смена префикса
на текущний порядок коллекций **не влияет**.

## 7. Список A (парсинг) и список B (эмиссия)

### Список A — файлы с литералом `sha256:`, которые сломались бы, если бы `ContentHash::parse` ПЕРЕСТАЛ принимать `sha256:`

Ответ по гипотезе пакета: **ни один не сломался бы**, потому что старую форму
продолжают принимать. Но периметр, который надо держать принимаемым, таков
(rg `sha256:` по `crates/**`, без `vibedeps`/`packages`):

- Тестовые фикстуры локфайла (значение парсится в `ContentHash`):
  `vibe-check/src/checks/lockfile_files.rs:154,211,250`;
  `vibe-check/src/checks/local_source_freshness.rs:170,207,227,260`;
  `vibe-cli/tests/{tree_fixture.rs:96,104,112,123,131,139; cli_redirect.rs:217}`;
  `vibe-cli/src/commands/short_name.rs:251`;
  `vibe-core/src/manifest/lockfile/tests.rs:35,53,168,213,249,285`;
  `vibe-workspace/src/{freshness.rs:323,345,355,566; bins.rs:363}`;
  `vibe-mcp/tests/tools_oracle.rs:29`; `vibe-install/tests/incremental_in_place.rs:125`;
  `vibe-index/src/lockfile.rs:70`.
- Константы/доктесты, парсящие `sha256:`: `content_hash_parity.rs:25` (`GOLDEN`);
  `vibe-core/src/content_hash.rs:36,37` (доктест `ContentHash::parse`).
- Синтетические `content_hash` в структурах/JSON (парсятся `String`, не `ContentHash`, но семантически): `vibe-index/tests/{server_writes.rs:29; server_e2e.rs:40; auto_publish.rs:69}`; `vibe-index/src/index/{primary.rs:123; memory.rs:328; inverted.rs:303; by_name.rs:146}`; `vibe-index/src/types/entry/{mod.rs:144; tests.rs:16}`; `vibe-registry/tests/index_fast_path.rs:132`; `vibe-install/tests/incremental_in_place.rs:85`; `vibe-cli/src/commands/progress_evidence.rs:121,123`.

Чтобы шаг был корректен, `ContentHash::parse` должен **продолжать принимать `sha256:`**
(старые локфайлы) **и** начать принимать `sha256-tree/1:`. Места проверки
«принимает ли parse новый префикс» — только доктест `content_hash.rs:36-38`.

### Список B — файлы, которые сломаются, если эмиссия `compute_content_hash` (обеих копий tree-walk) сменится на `sha256-tree/1:`, а остальное останется

| путь | сколько вхождений | тест/продакшн | чинится правкой ожидания или требует логики |
|---|---|---|---|
| `crates/vibe-index/tests/content_hash_parity.rs` | 1 (`:25` `GOLDEN`) | тест | ожидание (пересчитать `GOLDEN`); но см. §2 B5 — это не кросс-проверка |
| `crates/vibe-index/src/content_hash.rs` | 3 (`:85` golden, `:143` starts_with, `:144` len) | тест | ожидание (новый префикс + длина 13+64) |
| `crates/vibe-registry/src/shippable.rs` | 1 (`:75` доктест) | доктест | ожидание |
| `crates/vibe-registry/tests/registry_cells_oracle.rs` | 2 (`:71,194`) | тест | ожидание |
| `crates/vibe-registry/src/local_registry.rs` | 1 (`:429`) | тест | ожидание |
| `crates/vibe-registry/src/multi_registry_resolver/sources/tests.rs` | 1 (`:295`) | тест | ожидание |
| `crates/vibe-registry/src/multi_registry_resolver/tests.rs` | 1 (`:127`) | тест | ожидание |
| `crates/vibe-registry/src/git_package_registry/fetch/tests.rs` | 3 (`:55,268,406` — copy-путь) | тест | ожидание. `:536` (in-place/commit-derived) **НЕ ломается** |
| **`crates/vibe-publish/tests/post_hook.rs`** | 1 (`:157`) | тест | ожидание — **вне периметра правки** (хвост, см. §8) |
| `crates/vibe-cli/tests/cli_live_e2e.rs` | 2 (`:255,256`) | тест (`#[ignore]`) | ожидание — вне периметра, не бегут в дефолте |

Эмиттеры, чей вывод меняет вид (не assertions, а источники нового значения):
`vibe-index/src/cli/add.rs:73`, `vibe-index/src/scanner/org_walk.rs:193`,
`vibe-index/src/content_hash.rs:69`, `vibe-registry/src/shippable.rs:106`.

Отдельно от Списка B — **продакшн-логика, требующая правки, а не ожидания** (не
падают как тесты, но молча ломают поведение на существующих локфайлах):
`local_source_freshness.rs:92` (нужна нормализация одной из сторон перед `!=`)
и `fetch.rs:385` (нужна нормализация перед гейтом `==`).

## 8. Крейты вне периметра правки

Периметр шага — `vibe-index`, `vibe-registry`, `vibe-core`. Остальные 16 крейтов
workspace (`progress-core, vibe-actions, vibe-check, vibe-cli, vibe-graph,
vibe-install, vibe-llm, vibe-mcp, vibe-publish, vibe-resolver, vibe-settings,
vibe-spec, vibe-test-support, vibe-trace, vibe-wire, vibe-workspace`).
Тесты какого из них упадут от смены эмиссии?

- **`vibe-publish`** — ДА. `crates/vibe-publish/tests/post_hook.rs:154-158`:
  ```rust
  assert!( body["content_hash"].as_str().unwrap().starts_with("sha256:") );
  ```
  `body` — это POST к индексу; хэш считает `vibe_registry::compute_content_hash`
  (`post_hook.rs:28` import, `:185` вызов). При смене реестровой эмиссии
  `starts_with("sha256:")` → false → тест падает. **Единственный
  не-ignored крейт вне периметра, падающий гарантированно.**
- **`vibe-cli`** — условно. `cli_live_e2e.rs:255,256` упали бы, но они
  `#[ignore]` (`cli_live_e2e.rs:106,151,197` — live network). В дефолтном
  `cargo test --workspace` не бегут. `cli_pkg_cycle.rs:128` и
  `cli_registry_mgmt.rs:189` читают **локфайл** (остаётся `sha256:`) — не падают.
- **`vibe-check`** — тесты **не падают** (см. §5.2: обе стороны — одна функция),
  но именно здесь живёт **продакшн-поломка** `local_source_freshness.rs:92`
  (не тестовый, а поведенческий отказ).
- **`vibe-mcp`** — нет. `tools_oracle.rs:316` читает фикстуру `sha256:deadbeef`.
- **`vibe-install`** — нет. `incremental_in_place.rs:85` (`"sha256:feedface"`) и
  `:125` — литералы; `record.rs:193` — `from_validated` без assertion на префикс.
- **`vibe-workspace`** — нет. `freshness.rs:323,345,355,566`, `bins.rs:363` —
  литералы в фикстурах, без assertion на compute.
- **`vibe-wire`** — нет. `generated/install_plan/mod.rs:21` — комментарий
  док-строки.
- Прочие (`progress-core, vibe-actions, vibe-graph, vibe-llm, vibe-resolver,
  vibe-settings, vibe-spec, vibe-test-support, vibe-trace`) — операторов
  сравнения пакетного content-хэша не содержат (grep пуст).

Итого хвост, который исторически доставался боссу: **`vibe-publish`
(гарантированно)** + `vibe-cli` live-тесты (только если запускать `--ignored`).
Плюс **поведенческий** (не тестовый) отказ в `vibe-check`.

## 9. Ловушка порядка сортировки в фикстуре

Алгоритм (`content_hash.rs:41-53`, копия `shippable.rs:78-90`):
собрать `Vec<PathBuf>` → `files.sort()` (`:48`/`:85`) → **потом** нормализация
`\`→`/` (`:53`/`:90`) только для хэш-входа. `files.sort()` сортирует
`OsStr`-пути **с нативным разделителем**: на Windows — `\` (0x5C), на Unix —
`/` (0x2F). Нормализация порядка **не пересчитывает**.

Шесть путей фикстуры `crates/vibe-index/fixtures/golden-flow-wal-0.1.0/`
(`find -type f`), в forward-slash форме:

```
README.md
boot/10-flow-wal.md
spec/flows/wal/WAL-PROTOCOL.md
spec/flows/wal/morning-routine.md
spec/flows/wal/session-end-hook.md
vibe.toml
```

### (а) Есть ли в текущей фикстуре пара с переворотом порядка «до» vs «после» нормализации? — НЕТ.

Порядок может перевернуться только если в какой-то паре решающий (первый
различающийся) байт — разделитель у одного и не-разделитель у другого. Разбор
всех решающих байтов по парам:

| пара | общий прекс | решающий байт (A / B) | разделитель участвует? |
|---|---|---|---|
| `README.md` vs `boot/…` | — | `R`(0x52) / `b`(0x62) | нет |
| `boot/…` vs `spec/…` | — | `b`(0x62) / `s`(0x73) | нет |
| `spec/…` vs `vibe.toml` | — | `s`(0x73) / `v`(0x76) | нет |
| `README.md` vs `vibe.toml` | — | `R`(0x52) / `v`(0x76) | нет |
| три файла под `spec/flows/wal/` | `spec/flows/wal/` (15 байт, одинаков включая оба разделителя) | `W`(0x57) / `m`(0x6D) / `s`(0x73) | нет (разделители на одинаковых позициях, сравнение решается буквами имени) |

Ни в одной паре решающий байт не есть разделитель. Переворота нет →
фикстура **случайно безопасна** (не упражняет ловушку), и её GOLDEN-хэш
совпадает на Windows и Linux.

### (б) Точное условие переворота (через коды символов)

Порядок двух путей P, Q переворачивается между «до нормализации» (Windows, `\`=0x5C) и
«после нормализации» (`/`=0x2F) тогда и только тогда, когда в первой
различающейся позиции i (P[0..i] == Q[0..i]) мультимножество {P[i], Q[i]} равно
{ **разделитель**, **c** }, где `c ∈ (0x2F, 0x5C)`, т.е. `c ∈ {0x30..0x5B}` —
ASCII-цифра `0`–`9` (0x30–0x39), заглавная буква `A`–`Z` (0x41–0x5A) или
`[` (0x5B). Ибо `0x2F < c < 0x5C`, поэтому с разделителем `0x2F` порядок один
(`/` < c), а с `0x5C` — обратный (`\` > c). Практически: каталог, чьё имя —
префикс соседней записи, у которой следующий байт — цифра или заглавная буква.

### (в) Конкретная пара, на которой переворот проявляется (побайтово)

Добавить в фикстуру файл **`spec/flows/wal0.md`** — соседний с каталогом `wal/`
на уровне `spec/flows/`. Тогда два листовых пути:
`P = spec/flows/wal/morning-routine.md` и `Q = spec/flows/wal0.md`. Общий префикс
`spec/flows/wal` = 14 байт (индексы 0–13). Решающая позиция — **индекс 14**:

```
индекс:           0  1  2  3  4  5  6  7  8  9 10 11 12 13 | 14
P[0..15]:         s  p  e  c  /  f  l  o  w  s  /  w  a  l | '/'  (норм) или '\'  (win)
Q[0..15]:         s  p  e  c  /  f  l  o  w  s  /  w  a  l | '0'
                 73 70 65 63 2F 66 6C 6F 77 73 2F 77 61 6C | см. ниже
```

Побайтовое сравнение в позиции 14:

| представление | байт P[14] | байт Q[14] | порядок |
|---|---|---|---|
| после нормализации (`/`) | `0x2F` | `0x30` (`'0'`) | `0x2F < 0x30` → **P < Q** → `wal/morning-routine.md` идёт раньше `wal0.md` |
| до нормализации, Windows (`\`) | `0x5C` | `0x30` (`'0'`) | `0x5C > 0x30` → **P > Q** → `wal\morning-routine.md` идёт позже `wal0.md` |

Порядок **переворачивается**. Поскольку в хэш байты подаются в порядке
`files.sort()` (нормализация порядка не меняет), на Windows и Linux
последовательность `(rel || 0x00 || file_bytes || 0x00)` для этой пары
поступит в разном порядке → **разный итоговый SHA-256** → GOLDEN-хэш фикстуры
станет платформенно-зависимым (на Windows ≠ на Linux), и
`content_hash_parity.rs` (если бы GOLDEN был один) не мог бы одинаково
выполняться на обеих ОС. Нынешняя фикстура этой пары не содержит, поэтому
кросс-платформенность GOLDEN держится случайно.

## 10. Вывод для стройки

**Громко и первым — два продакшн-сравнения, где формы встретятся.**
`vibe-check/src/checks/local_source_freshness.rs:92` (`fresh != pkg.content_hash.as_str()`)
и `vibe-registry/src/git_package_registry/fetch.rs:385`
(`Some(expected) if expected == content_hash`) — оба строково сравнивают
свежевычисленный tree-walk-хэш с хэшем, лежащим в локфайле. При посылке пакета
(локфайл остаётся `sha256:`, эмиссер стал `sha256-tree/1:`) первое даёт
**ложный warning «источник изменился» на каждом local-пакете**, второе —
**молча отключает cross-source integrity-gate** (зеркало всегда бракуется). Это
тот самый «неверный ответ, выглядящий верным», который нельзя вылечить
перекачиванием; правится **логикой** (нормализация одной из сторон перед
сравнением), а не правкой ожидания.

**Что делает шаг работой, а не риском:** (1) смена эмиссии изолирована —
клиент индекса `wire.rs` не читает `content_hash`, и каталог индекса не кормит
локфайл, поэтому «только-индексная» смена ни с чем не сталкивается; (2) `Ord` у
`ContentHash` латентен — нигде не сортируют и в ключ не кладут (§6), так что
порядок коллекций не затронут; (3) `commit_content_hash` (in-place) — отдельный
продюсер, шагом не трогается; (4) большинство тестовых падений (Список B) —
правка ожидания, и почти все внутри периметра; (5) `lockfile_files.rs` хэш не
трогает вовсе.

**Три вещи, которые требуют внимания владельца, а не механики:**
- **Посылка «локфайл = `sha256:`, индекс = `sha256-tree/1:`» внутренне напряжена.**
  При `from_validated` без нормализации (`record.rs:193`) локфайл просто
  наследует вид реестровой эмиссии: если меняется `vibe-registry` (контракт
  «lockstep» этого требует) — локфайл тоже станет `sha256-tree/1:`, что
  противоречит посылке. Чтобы посылка держалась, нужна явная нормализация на
  границе записи локфайла — а тогда именно она и порождает два сравнения выше.
- **B5 не ловит расхождение копий.** `content_hash_parity.rs` замораживает
  только `vibe-index` относительно `GOLDEN`; живой перекрёстной проверки
  `vibe-index`↔`vibe-registry` нет. Шаг, меняющий одну копию, пройдёт этот
  сторож (после обновления `GOLDEN`) и молча нарушит контракт
  «проиндексированный тут = материализованный там».
- **Хвост вне периметра:** `vibe-publish` (`post_hook.rs:157`) падает
  гарантированно; `vibe-cli` live-тесты (`cli_live_e2e.rs:255,256`) — при
  `--ignored`. Их надо править вместе со шагом.

## 11. Как воспроизвести этот замер

- Прочитать `crates/vibe-index/src/content_hash.rs`.
- Прочитать `crates/vibe-registry/src/shippable.rs`.
- Прочитать `crates/vibe-core/src/content_hash.rs`.
- Выполнить `rg -n "fn compute_content_hash" crates/`.
- Выполнить `rg -n "from_validated" crates/`.
- Выполнить `rg -n "sha256:" crates/`.
- Выполнить `rg -n "content_hash\s*(!=|==)" crates/`.
- Выполнить `rg -n 'starts_with\("sha256' crates/`.
- Выполнить `rg -n "content_hash" crates/vibe-registry/src/index_client/`.
- Прочитать `crates/vibe-check/src/checks/local_source_freshness.rs`.
- Прочитать `crates/vibe-check/src/checks/lockfile_files.rs`.
- Прочитать `crates/vibe-install/src/record.rs` (строки 160–213).
- Прочитать `crates/vibe-registry/src/git_package_registry/fetch.rs` (строки 335–395, 478–493).
- Прочитать `crates/vibe-cli/src/commands/install/resolver.rs` (строки 60–150).
- Выполнить `find crates/vibe-index/fixtures/golden-flow-wal-0.1.0/ -type f`.
- Выполнить `rg -n "ContentHash[,>]|BTreeMap<ContentHash|sort_by_key" crates/`.
