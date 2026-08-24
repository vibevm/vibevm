# G2-B057 — долг дисциплины conform в четырёх языковых слотах ai-native

Измерен долг дисциплины (движок `conform`, ENGINE-CONFORM v0.1) на **собственных
исходниках** четырёх авторских пакетных воркспейсов `org.vibevm.ai-native`
(задача B-057: движок раньше гонялся только по хостовым `crates/`, на свои
исходники — никогда). Гонял бинарь `rust-ai-native-conform`, собранный одним
`cargo run` из воркспейса S2, по каждому слоту с **измерительной** политикой
`conform.toml` (все крейты слота в `gated`, `[[rust.exempt]]` не писался,
baseline пустой → каждая находка считается новой, гейт честно падает с
`exit=1`). Замер, а не починка: ни один `.rs` не правился.

Слоты:

- **S1 core** = `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0` (5 крейтов)
- **S2 rust-lang** = `…/rust-ai-native-lang/v0.7.0` (7 крейтов)
- **S3 ts-lang** = `…/typescript-ai-native-lang/v0.6.0` (8 крейтов)
- **S4 go-lang** = `…/go-ai-native-lang/v0.1.0` (8 крейтов)

## 1. Таблица «слот × правило → число находок»

Числа — из строки `conform check: N finding(s) in scope <workspace> ({…})`
(разбивка по правилам в фигурных скобках). Пусто = 0.

| слот              | ambient-env | error-enum-cites-req | error-message-cites-req | file-length | no-unwrap-in-domain | seam-has-doctest | unsafe-gate | **всего** |
|-------------------|------------:|---------------------:|------------------------:|------------:|--------------------:|-----------------:|------------:|----------:|
| S1 core           | 0           | 1                    | 0                       | 1           | 0                   | 3                | 4           | **9**     |
| S2 rust-lang      | 6           | 0                    | 0                       | 1           | 0                   | 20               | 4           | **31**    |
| S3 ts-lang        | 0           | 2                    | 5                       | 0           | 1                   | 45               | 0           | **53**    |
| S4 go-lang        | 2           | 1                    | 3                       | 0           | 1                   | 34               | 0           | **41**    |
| **итого (4 слота)** | **8**     | **4**                | **8**                   | **2**       | **2**               | **102**          | **8**       | **134**   |

Главный сигнал: `seam-has-doctest` даёт **102 из 134** находок (76 %). `ts-lang`
и `go-lang` — почти структурные близнецы (по 8 крейтов одного состава), но
`ts-lang` (53) тяжелее `go-lang` (41) за счёт `seam-has-doctest` (45 vs 34) и
бóльшего числа error-cites-req находок.

## 2. `extracted N file(s)` дословно + контрольное число RP1

Контроль (RP1) — счётчик `find <SLOT>/crates -name '*.rs' -not -path '*/vendor/*' -not -path '*/target/*'` минус файлы под `/generated/` (движок их исключает через `exclude_substrings`).

| слот | дословная строка из лога | extracted | контроль (A − generated) | vendor .rs (не в скане) |
|---|---|---:|---:|---:|
| S1 core | `conform: extracted 49 file(s), 0 cached (producer rust-syn-10).` | 49 | 51 − 2 = **49** | — (нет vendor) |
| S2 rust-lang | `conform: extracted 37 file(s), 0 cached (producer rust-syn-10).` | 37 | 37 − 0 = **37** | 45 |
| S3 ts-lang | `conform: extracted 29 file(s), 0 cached (producer rust-syn-10).` | 29 | 29 − 0 = **29** | 45 |
| S4 go-lang | `conform: extracted 37 file(s), 0 cached (producer rust-syn-10).` | 37 | 37 − 0 = **37** | 45 |

Во всех четырёх слотах `extracted N` **точно** равно контрольному числу. 2
сгенерированных файла в S1 — `core-ai-native-specmap/src/generated/mod.rs` и
`…/specmap/mod.rs` (исключены по `/generated/`). vendor-копии (по 45 `.rs` в
S2/S3/S4) в скан не попали — **ноль**. Подробности и опровержение формулировки —
в RP1 ниже.

## 3. Полные списки `NEW` по слотам

### S1 core — 9 находок (все)

```
error-enum-cites-req  crates/core-ai-native-mcp/src/error.rs:29 — thiserror enum `McpCoreError` carries no #[spec] REQ edge
file-length           crates/core-ai-native-conform/src/rules/go.rs:1 — 612 lines exceeds the 600-line file budget
seam-has-doctest      crates/core-ai-native-mcp/src/server.rs:21 — public seam trait `Transport` has no compiled doctest
seam-has-doctest      crates/core-ai-native-mcp/src/toolset.rs:76 — public seam trait `Tool` has no compiled doctest
seam-has-doctest      crates/core-ai-native-specmap/src/scanner.rs:23 — public seam trait `CodeScanner` has no compiled doctest
unsafe-gate           crates/core-ai-native-mcp/src/capture.rs:142 — `unsafe` (block) outside a designated audit crate
unsafe-gate           crates/core-ai-native-mcp/src/capture.rs:158 — `unsafe` (block) outside a designated audit crate
unsafe-gate           crates/core-ai-native-mcp/src/capture.rs:200 — `unsafe` (block) outside a designated audit crate
unsafe-gate           crates/core-ai-native-mcp/src/capture.rs:211 — `unsafe` (block) outside a designated audit crate
```

### S2 rust-lang — 31 находка (все)

```
ambient-env      crates/rust-ai-native-env-audit/src/lib.rs:80  — `env::set_var()` …
ambient-env      crates/rust-ai-native-env-audit/src/lib.rs:89  — `env::remove_var()` …
ambient-env      crates/rust-ai-native-env-audit/src/lib.rs:96  — `env::var_os()` …
ambient-env      crates/rust-ai-native-env-audit/src/lib.rs:107 — `env::set_var()` …
ambient-env      crates/rust-ai-native-env-audit/src/lib.rs:109 — `env::remove_var()` …
ambient-env      crates/rust-ai-native-tcg/src/bench.rs:148     — `env::var()` …
file-length      crates/rust-ai-native-conform-frontend/src/lib.rs:1 — 601 lines exceeds the 600-line file budget
seam-has-doctest crates/rust-ai-native-conform/src/lib.rs:22   — pub seam fn `load_config`
seam-has-doctest crates/rust-ai-native-conform/src/lib.rs:29   — pub seam fn `load_config_or_default`
seam-has-doctest crates/rust-ai-native-conform/src/lib.rs:125  — pub seam fn `run_check`
seam-has-doctest crates/rust-ai-native-conform/src/lib.rs:217  — pub seam fn `run_freeze`
seam-has-doctest crates/rust-ai-native-specmap/src/lib.rs:22   — pub seam fn `run_specmap`
seam-has-doctest crates/rust-ai-native-specmap/src/lib.rs:54   — pub seam fn `run_gate`
seam-has-doctest crates/rust-ai-native-tcg-bridge/src/client.rs:23 — pub seam trait `Transport`
seam-has-doctest crates/rust-ai-native-tcg-bridge/src/lib.rs:146 — pub seam fn `resolve_rust_analyzer`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:40  — pub seam struct `Policy`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:65  — pub seam struct `WireFinding`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:76  — pub seam struct `EnrichedValidate`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:202 — pub seam fn `enrich_validate`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:266 — pub seam struct `BrandedNewtype`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:312 — pub seam struct `ScopeAnswer`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:322 — pub seam fn `seam_file_for`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:352 — pub seam fn `finalise_completions`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:386 — pub seam fn `spawn_oracle`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:395 — pub seam fn `read_content_from`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:410 — pub seam fn `validate_exit_code`
seam-has-doctest crates/rust-ai-native-tcg/src/lib.rs:418 — pub seam fn `parse_position`
unsafe-gate      crates/rust-ai-native-env-audit/src/lib.rs:80  — `unsafe` (block) outside a designated audit crate
unsafe-gate      crates/rust-ai-native-env-audit/src/lib.rs:89  — `unsafe` (block) outside a designated audit crate
unsafe-gate      crates/rust-ai-native-env-audit/src/lib.rs:107 — `unsafe` (block) outside a designated audit crate
unsafe-gate      crates/rust-ai-native-env-audit/src/lib.rs:109 — `unsafe` (block) outside a designated audit crate
```

### S3 ts-lang — 53 находки (показаны 40 из 53)

```
error-enum-cites-req    crates/typescript-ai-native-extract-bridge/src/lib.rs:37   — thiserror enum `BridgeError` carries no #[spec] REQ edge
error-enum-cites-req    crates/typescript-ai-native-tcg-bridge/src/lib.rs:76       — thiserror enum `TcgBridgeError` carries no #[spec] REQ edge
error-message-cites-req crates/typescript-ai-native-extract-bridge/src/lib.rs:37   — `BridgeError::NodeMissing` display text cites no spec:// REQ
error-message-cites-req crates/typescript-ai-native-extract-bridge/src/lib.rs:45   — `BridgeError::TypescriptUnresolvable` …
error-message-cites-req crates/typescript-ai-native-extract-bridge/src/lib.rs:53   — `BridgeError::ExtractorFailed` …
error-message-cites-req crates/typescript-ai-native-extract-bridge/src/lib.rs:61   — `BridgeError::Protocol` …
error-message-cites-req crates/typescript-ai-native-tcg-bridge/src/lib.rs:110      — `TcgBridgeError::Io` …
no-unwrap-in-domain     crates/typescript-ai-native-tcg/src/bench.rs:128           — `.expect()` in domain logic
seam-has-doctest  crates/typescript-ai-native-conform-frontend/src/lib.rs:25  — pub seam struct `TsTscFrontend`
seam-has-doctest  crates/typescript-ai-native-conform/src/lib.rs:143          — pub seam fn `run_check`
seam-has-doctest  crates/typescript-ai-native-conform/src/lib.rs:206          — pub seam fn `run_freeze`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:30   — pub seam enum `BridgeError`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:71   — pub seam enum `RawFact`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:132  — pub seam struct `RawMarker`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:142  — pub seam struct `FileRecord`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:153  — pub seam fn `parse_ndjson`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:181  — pub seam fn `materialise_extractor`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:198  — pub seam fn `extract_tree`
seam-has-doctest  crates/typescript-ai-native-extract-bridge/src/lib.rs:229  — pub seam fn `conform_facts`
seam-has-doctest  crates/typescript-ai-native-specmap-scan/src/lib.rs:158    — pub seam fn `records_to_index`
seam-has-doctest  crates/typescript-ai-native-specmap-scan/src/lib.rs:173    — pub seam struct `RecordsScanner`
seam-has-doctest  crates/typescript-ai-native-specmap-scan/src/lib.rs:214    — pub seam struct `TsOrphan`
seam-has-doctest  crates/typescript-ai-native-specmap-scan/src/lib.rs:225    — pub seam fn `orphans`
seam-has-doctest  crates/typescript-ai-native-specmap/src/lib.rs:22          — pub seam fn `run_specmap_typescript`
seam-has-doctest  crates/typescript-ai-native-specmap/src/lib.rs:67          — pub seam fn `run_gate`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:35       — pub seam fn `verbatim_free`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:46       — pub seam fn `materialise_oracle`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:70       — pub seam enum `TcgBridgeError`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:122      — pub seam struct `Diagnostic`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:132      — pub seam struct `InitResult`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:140      — pub seam struct `ValidateResult`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:149      — pub seam struct `SymbolInfo`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:158      — pub seam struct `BrandedType`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:166      — pub seam struct `ScopeResult`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:175      — pub seam struct `CompletionEntry`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:186      — pub seam struct `CompleteResult`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:192      — pub seam struct `TypeResult`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:199      — pub seam struct `Position`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:206      — pub seam struct `WireError`
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:215      — pub seam struct `ResponseFrame`
```

Хвост (41–53, все того же правила `seam-has-doctest`, кратко `файл:строка`):

```
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:227
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:244
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/lib.rs:263
seam-has-doctest  crates/typescript-ai-native-tcg-bridge/src/transport.rs:28
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:33
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:68
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:77
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:87
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:137
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:178
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:235
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:267
seam-has-doctest  crates/typescript-ai-native-tcg/src/lib.rs:308
```

### S4 go-lang — 41 находка (показаны 40 из 41)

```
ambient-env           crates/go-ai-native-extract-bridge/src/lib.rs:189 — `env::var()` …
ambient-env           crates/go-ai-native-tcg-bridge/src/lib.rs:146     — `env::var()` …
error-enum-cites-req  crates/go-ai-native-extract-bridge/src/lib.rs:44  — thiserror enum `BridgeError` carries no #[spec] REQ edge
error-message-cites-req crates/go-ai-native-extract-bridge/src/lib.rs:44 — `BridgeError::GoMissing` …
error-message-cites-req crates/go-ai-native-extract-bridge/src/lib.rs:52 — `BridgeError::ExtractorFailed` …
error-message-cites-req crates/go-ai-native-extract-bridge/src/lib.rs:60 — `BridgeError::Protocol` …
no-unwrap-in-domain   crates/go-ai-native-tcg/src/bench.rs:148          — `.expect()` in domain logic
seam-has-doctest  crates/go-ai-native-conform-frontend/src/lib.rs:29  — pub seam struct `GoExtractFrontend`
seam-has-doctest  crates/go-ai-native-conform/src/lib.rs:159         — pub seam fn `run_check`
seam-has-doctest  crates/go-ai-native-conform/src/lib.rs:217         — pub seam fn `run_freeze`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:37   — pub seam enum `BridgeError`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:69   — pub seam enum `RawFact`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:139  — pub seam struct `RawMarker`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:150  — pub seam struct `FileRecord`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:161  — pub seam fn `parse_ndjson`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:188  — pub seam fn `go_binary`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:195  — pub seam fn `materialise_extractor`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:221  — pub seam fn `extract_content`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:260  — pub seam fn `extract_tree`
seam-has-doctest  crates/go-ai-native-extract-bridge/src/lib.rs:291  — pub seam fn `conform_facts`
seam-has-doctest  crates/go-ai-native-specmap-scan/src/lib.rs:137    — pub seam fn `records_to_index`
seam-has-doctest  crates/go-ai-native-specmap-scan/src/lib.rs:152    — pub seam struct `RecordsScanner`
seam-has-doctest  crates/go-ai-native-specmap-scan/src/lib.rs:202    — pub seam struct `GoOrphan`
seam-has-doctest  crates/go-ai-native-specmap-scan/src/lib.rs:214    — pub seam fn `orphans`
seam-has-doctest  crates/go-ai-native-specmap/src/lib.rs:25          — pub seam fn `run_specmap_go`
seam-has-doctest  crates/go-ai-native-specmap/src/lib.rs:70          — pub seam fn `run_gate`
seam-has-doctest  crates/go-ai-native-tcg-bridge/src/client.rs:21    — pub seam trait `Transport`
seam-has-doctest  crates/go-ai-native-tcg-bridge/src/lib.rs:145      — pub seam fn `resolve_gopls`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:38   — pub seam struct `Policy`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:74   — pub seam struct `WireFinding`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:86   — pub seam struct `EnrichedValidate`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:163  — pub seam fn `enrich_validate`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:243  — pub seam struct `BrandedType`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:251  — pub seam fn `brands_of`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:276  — pub seam struct `ScopeAnswer`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:313  — pub seam fn `seam_file_for`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:341  — pub seam fn `finalise_completions`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:380  — pub seam fn `spawn_oracle`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:389  — pub seam fn `read_content_from`
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:404  — pub seam fn `validate_exit_code`
```

41-я (хвост):

```
seam-has-doctest  crates/go-ai-native-tcg/src/lib.rs:412  — pub seam fn `parse_position`
```

## 4. Точки уточнения

### RP1 — доходит ли скан до вендор-копий

**Ответ числом: нет, не доходит.** Во всех трёх языковых слотах `extracted N`
**точно** совпал с числом `.rs` вне vendor/target (минус generated): S2 37=37,
S3 29=29, S4 37=37. Vendor-копии дают по **45 `.rs`** на слот и внесли в скан
**ноль** — расхождения с контрольным числом нет, поэтому назвать «разницу числом
и объясняющий файл» не требуется: разница = 0.

**Но формулировка босса неточна, и это ценнее согласия, поэтому прямым текстом:**
утверждение «`crates/*` не видит `crates/vendor/**`, потому что у `vendor` нет
`Cargo.toml`» описывает не тот механизм. В движке **две** разные процедуры, и
проверка `Cargo.toml` сидит только в одной:

- **Классификация юнитов** `rust_units` (`config/coverage.rs:96-110`) — да,
  проверяет `Cargo.toml` у каждого непосредственного подкаталога `crates/`
  (строка 103: `e.path().join("Cargo.toml").exists()`). Поэтому `crates/vendor`
  юнитом НЕ становится и в проверку «каждый крейт gated или exempt» не попадает.
- **Файловый сканер** `workspace_sources` (`store.rs:229-287`) — **не
  проверяет** `Cargo.toml`. Для глоба `crates/*` он добавляет **каждый**
  подкаталог `crates/`, включая `vendor` (строки 238-242), и обходит в нём
  `src/` и `tests/`. Если бы в `crates/vendor/` лежал собственный `src/`, его
  `.rs` попали бы в скан под юнит-именем `vendor` — без всякого `Cargo.toml`.

Реальная причина, по которой vendor невидим: у `crates/vendor` **нет своих
`src/` и `tests/`** — вендор-крейты лежат на уровень глубже
(`crates/vendor/<name>/src/…`), а сканер читает только `crate_dir/src` и
`crate_dir/tests`. Я проверил: `crates/vendor/src` и `crates/vendor/tests`
**не существуют** ни в одном из S2/S3/S4. Итог: вывод босса верен (vendor не
сканируется), обоснование — нет; cargo-манифест тут ни при чём для файлового
ската, он важен только для классификации юнитов.

### RP2 — что здесь ложная находка

Моё мнение (решает босс). Не чинил ничего.

1. **`seam-has-doctest` по протокольным DTO в `*-tcg-bridge` — главный подозреваемый
   (102 из 134 находок всего, и бóльшая часть именно в tcg-bridge).** Правило
   требует compiled-доктеста «с каноническим построением и использованием» на
   каждом `pub` «шве». Но в `tcg-bridge/src/lib.rs` под удар попадают
   data-переносчики JSON-протокола: `Diagnostic`, `Position`, `ResponseFrame`,
   `ScopeResult`, `CompletionEntry`, `WireError`, `BrandedType`, `SymbolInfo`,
   `InitResult` и т.п. Это serde-DTO без сколько-нибудь осмысленного «шовного»
   поведения; доктест на них — косметика. Похоже, правило (или фронтендовская
   классификация «что есть шов») разносит флажок на **каждый** `pub`-item, а не
   на истинные абстракции-швы. Это шум, а не долг — но подтверждать надо
   проверкой, как фронтенд решает, что pub-item «шов».

2. **`ambient-env` + `unsafe-gate` по `rust-ai-native-env-audit` (S2, 6+4=10).**
   Крейт `_env-audit` по имени и смыслу — это и есть «audit home»: его работа —
   держать env-мутации и `unsafe` за безопасной API. Правила
   `UnsafeGate`/`AmbientEnv` умеют исключать `audit_crates`; в хостовой политике
   env-audit прямо назван audit-домом. Измерительный `conform.toml` положил его
   в `gated`, а не в `audit_crates` — и правила бьют по единственному крейту,
   который существует, чтобы их удовлетворять. Артефакт классификации, не долг.
   Все 6 `ambient-env` и все 4 `unsafe-gate` в S2 — в одном файле
   `rust-ai-native-env-audit/src/lib.rs`.

3. **`ambient-env` / `no-unwrap-in-domain` в `bench.rs`** — S2
   `rust-ai-native-tcg/src/bench.rs:148` (чтение env), S3 и S4
   `*-tcg/src/bench.rs` (`.expect()`). Бенчмарк, читающий флажок-toggle через
   env, или `.expect()` в бенче — идиоматика и явно не «доменная логика» и не
   composition root. Правила нацелены на продукт-домен; на бенчмарках это
   похоже на перелёт. (Скажем, 3 находки.)

Что, на мой взгляд, **настоящий** долг (не ложное): `file-length`
(`rules/go.rs` 612, `conform-frontend/lib.rs` 601 — реально превышают бюджет
600); `error-enum-cites-req`/`error-message-cites-req` по `BridgeError`/
`TcgBridgeError`/`McpCoreError` (именно те error-слои, на которые правило
нацелено); `unsafe-gate` по `core-ai-native-mcp/src/capture.rs` (mcp не audit,
так что unsafe там легитимно флажкуется — чинить переносом в audit или
`#[spec(deviates)]`).

### RP3 — умолчание бюджета длины

**`max_file_lines` по умолчанию = 600.** Источник: `Config::default` в
`core-ai-native-conform/src/config.rs:165` (`max_file_lines: 600`); тот же 600
заassertнут в доктесте `config.rs:58` (`assert_eq!(cfg.max_file_lines, 600)`).
**Совпадает** с 600 в шаблоне пакета. Поэтому обе `file-length`-находки
настоящие: 612 и 601 строк действительно превышают 600.

## 5. Побочные эффекты прогона (на заметку боссу)

Каждый `check` пишет SARIF-отчёт в `<СЛОТ>/target/conform/report.sarif`, а
сборка бинаря — артефакты в `target/` воркспейса S2. Всё под `target/`
(gitignored), это нормальное поведение движка, **не правки исходников**. Ни один
`conform-baseline.json` в слотах не создавался, `freeze` не запускался.

---

## 6. Калибровка и посадка (второй заход)

Первый замер был нарочно грубым (все крейты в `gated`) и дал 134 находки вместо
ожидавшегося «почти нуля». Замысел политики оттого поменялся: хостовая политика
устроена как «расширяемый по мере готовности список» — крейт входит в `gated`,
когда уже чист. Поэтому посадка гейтит чистое, а остальное объявляет
`[[rust.exempt]]` С ПРИЧИНОЙ, чтобы долг был **назван**, а не заморожен молча.
Измерительные `conform.toml` заменены посадочными; код не правился.

**Принцип разбиения.** Крейт с хотя бы одной находкой gated-зависимого правила
(`seam-has-doctest`, `pub-doctest`, `error-enum-cites-req`,
`error-message-cites-req`, `no-unwrap-in-domain`, `ambient-env`) переезжает в
`exempt`; без таких — остаётся в `gated`. `rust-ai-native-env-audit` (S2)
помещён в `audit_crates` (он и есть аудит-дом для env/unsafe), а не в `exempt` —
после этого у него gated-находок не осталось, поэтому он остался в `gated`
(одновременно в `audit_crates`).

### 6.1 Таблица «слот → gated / exempt / остаток / заморожено»

«Остаток» — находки после калибровки (только правила, не зависящие от `gated`:
`file-length`, `unsafe-gate`). «Заморожено» — то же число, записанное в baseline.

| слот | gated | exempt | audit_crates | остаток после калибровки | заморожено в baseline |
|---|---:|---:|---:|---:|---:|
| S1 core        | 3 | 2 | — | 5 | 5 |
| S2 rust-lang   | 3 | 4 | 1 (`rust-ai-native-env-audit`) | 1 | 1 |
| S3 ts-lang     | 1 | 7 | — | 0 | 0 |
| S4 go-lang     | 1 | 7 | — | 0 | 0 |
| **итого**      | **8** | **20** | **1** | **6** | **6** |

Ни в одном слоте `gated` не оказался пуст (минимум 1 крейт — `*-cli`/`*-conform`).
Контроль: post-freeze `check` по всем четырём слотам дал **`exit=0`** (`0 new`).

> **Колонка «заморожено» относится ко ВТОРОМУ заходу и была изменена третьим.**
> Босс отклонил заморозку четырёх `unsafe`-находок ядра по закону «помечать, а
> не гасить»: они оформлены признанным отступлением в коде и остались видимыми
> находками. Итоговое состояние, проверенное боссом на живом дереве:
> **S1 core — заморожена 1** (не 5), S2 — 1, S3/S4 — 0, **итого 2** (не 6).
> «Остаток после калибровки» не изменился: находок по-прежнему 6, из них
> четыре теперь gate-inert. Подробности — в блоке про третий заход ниже.

### 6.2 Поимённый список замороженных находок (6 шт., ≤10 — все)

S1 core — `conform-baseline.json`, 5 fingerprint(s) frozen:

- `file-length` — `crates/core-ai-native-conform/src/rules/go.rs:1` (612 строк > 600)
- `unsafe-gate` — `crates/core-ai-native-mcp/src/capture.rs:142` (`unsafe`-block вне audit-крейта)
- `unsafe-gate` — `crates/core-ai-native-mcp/src/capture.rs:158`
- `unsafe-gate` — `crates/core-ai-native-mcp/src/capture.rs:200`
- `unsafe-gate` — `crates/core-ai-native-mcp/src/capture.rs:211`

S2 rust-lang — 1 fingerprint frozen:

- `file-length` — `crates/rust-ai-native-conform-frontend/src/lib.rs:1` (601 строка > 600)

S3 ts-lang — **0** заморожено (baseline пустой, `{"schema":1,"findings":[]}`).
S4 go-lang — **0** заморожено (baseline пустой).

Строки `unsafe-gate` (142/158/200/211) — из первого замера; baseline хранит
fingerprint правилом+файлом+`block#N` (без номера строки), проверочная строка
взята из лога первого прогона.

### 6.3 Про 102 находки `seam-has-doctest`

Эти **102** находки `seam-has-doctest` (+ `error-*`, `no-unwrap`, `ambient-env`
по ним же) **НЕ заморожены в baseline** — они названы **причиной освобождения** в
`[[rust.exempt]]` каждого крейта. То есть долг остался **видимым**: baseline не
его прячет, exempt-таблица явно перечисляет, какой крейт сколько должен и по
каким правилам. Когда крейт обретёт доктесты на швах и ссылки на требования в
слое ошибок, он переезжает из `exempt` в `gated` — и его находки станут уже
новыми против пустого для него baseline. Это и есть «расширяемый по мере
готовности список».

### 6.4 Оформление отступлений `unsafe` в `capture.rs` (третий заход)

Второй заход заморозил четыре `unsafe-gate`-находки по
`crates/core-ai-native-mcp/src/capture.rs` (блоки 142/158/200/211) в baseline
ядра. Это противоречит закону проекта «ПОМЕЧАТЬ, А НЕ ГАСИТЬ»: признанное
отступление обязано оставаться видимой находкой со штампом «отступление
признано», а не исчезать в реечном файле. Заморозка прячет; отметка объявляет.

Поэтому четыре `unsafe` оформлены признанным отступлением в коде: на каждую
несущую функцию (`redirect_stderr_to` и `restore_stderr` в unix- и
windows-платформенных модулях `capture.rs`) навешан атрибут, уже
практикуемый в дереве. Образец, по которому равнялся (RP-A):
`crates/vibe-publish/src/redirect_sync.rs:245` и `crates/vibe-resolver/src/activation.rs:270` —

```rust
#[spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "<одна фраза: почему unsafe здесь законен — по смыслу // SAFETY:>"
)]
```

Атрибут импортируется `use specmark::spec;` (зависимость `specmark` у
`core-ai-native-mcp` уже была — `specmark::scope!` использовался в файле и
раньше), новой зависимости не добавлено. `// SAFETY:`-комментарии оставлены
на месте: они для человека, атрибут для машины. Причины (`reason`) написаны
своими словами по смыслу уже стоящих `// SAFETY:` — по одной на функцию.

После правки baseline ядра перезаписан (`freeze`), и движок сам выбросил
признанные отступления из реечного файла (`freezeable` исключает
acknowledged, `baseline.rs:223`). В `conform-baseline.json` ядра осталась
**ровно одна запись**:

```json
{
  "schema": 1,
  "findings": [
    "file-length|crates/core-ai-native-conform/src/rules/go.rs"
  ]
}
```

Сами четыре `unsafe` остались **видимыми** находками: `conform check` по-прежнему показывает `{"file-length": 1, "unsafe-gate": 4}` (5 finding(s)), но
статус `unsafe-gate` теперь `deviation-acknowledged`, и они gate-inert — `0 new`, `exit=0`. Отступление на виду, baseline его не прячет; в реечном файле
остался только настоящий неоформленный долг — длина `rules/go.rs`.

Сборка ядра после правки зелёная: `cargo fmt`, `cargo check --workspace
--all-targets`, `cargo test --workspace`, `cargo clippy --workspace
--all-targets -- -D warnings` — все `exit=0` (включая тест
`capture::tests::capture_guard_end_to_end`, так что атрибуты не изменили
поведение захвата stderr).

Замечание о вендор-копиях: правка `capture.rs` — это АВТОРСКИЙ исходник движка;
его шесть байт-копий в других пакетах теперь разошлись. Это ожидаемо,
синхронизацию копий делает босс отдельной командой; копии не трогались.

### 6.4 Остаточный риск посадки — назван воркером, не найден ревью

Половина `capture.rs` под `#[cfg(unix)]` **на этой машине компилятором не
проверяется** — сборка её вырезает. Значит атрибуты `#[spec(deviates)]` в
unix-ветке не прошли того же контроля, что windows-ветка. Риск снижен тремя
опорами, но не снят:

1. атрибут в unix-ветке написан по тому же компилируемому образцу, что и в
   windows-ветке, и они текстуально совпадают по форме;
2. windows-ветка того же файла прошла `check` + `test` + `clippy` полностью;
3. движок конформа читает **оба** модуля синтаксическим разбором, а не через
   cargo, и после правки перевёл все четыре находки `unsafe-gate` в
   «отступление признано» — то есть атрибут распознан и в unix-ветке тоже.

Прямая проверка требует сборки под unix и в эти четыре команды не входила.
Записано здесь, чтобы следующий читатель не принял зелёную панель за
доказательство того, чего она не проверяла.
