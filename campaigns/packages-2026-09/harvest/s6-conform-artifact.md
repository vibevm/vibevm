# S6.1 — артефакт находок conform: замер

Замер выполнялся по дереву worktree `wt/S6-MEASURE` (коммит `5289cdb5`).
Инструменты — только чтение, `grep`, `ls`, `wc`; ни одного `cargo`/`vibe`/`git`.
Поскольку worktree будет удалён, все нужные цитаты вставлены дословно.

**Соглашение о путях.** Бегущий при `cargo xtask conform` движок — это
ВЕНДОР-копия под стеком `rust-ai-native-lang`, на которую указывает
`Cargo.toml:103`. Короткое обозначение ниже:

- `CONF` = `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-conform/src`
  (вендор-копия, **то, что реально компилируется и бежит**);
- `FE` = `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src`
  (фронтенд-драйвер `run_check`/`run_freeze`).

Все цитаты `CONF:…` и `FE:…` даны по этим префиксам.

## 1. ВЕРДИКТ

**ДА С ОГОВОРКАМИ.** Артефакт находок с `file`+`line` на диске существует —
это SARIF-отчёт `target/conform/report.sarif`, который пишет `conform check`
(`FE/lib.rs:171-176`). Но три оговорки делают его непригодным «как есть» для
соединения без явной оговорки свежести (B019-V-RECOMMENDATION):

1. **Он под `.gitignore`** (`target/`), значит отсутствует в свежем клоне — в
   ЭТОМ worktree его прямо сейчас нет (`ls target/conform/` ⇒ No such file).
2. **В нём нет времени прогона** — SARIF намеренно «no wall-clock»
   (`CONF/sarif.rs:12-13`); свежесть можно оценить только по mtime файла, а он
   пересоздаётся каждым `check`.
3. **Единственный ЗАКОММИЧЕННЫЙ conform-артефакт** (`conform-baseline.json`)
   хранит отпечатки `rule|file|carrier` БЕЗ номеров строк
   (`CONF/baseline.rs:22-27`) — то есть коммиченные данные принципиально не
   несут (file,line)-ключа. Соединение по (file,line) возможно только против
   лежалого, некоммиченного SARIF.

## 2. Сверка опорных координат (B1..B5)

| # | утверждение | вердикт | цитата file:line |
|---|---|---|---|
| B1 | `run_conform_check(baseline_rel, scope)` — тонкая обёртка над `rust_ai_native_conform::run_check(&repo_root()?, baseline_rel, scope)` | ПОДТВЕРЖДЕНО | `xtask/src/conform.rs:13-15` (рядом `run_conform_freeze` → `run_freeze`, `xtask/src/conform.rs:17-19`) |
| B2 | в корне лежат `conform.toml` и `conform-baseline.json`, оба под git | ПОДТВЕРЖДЕНО | оба присутствуют в свежем worktree (`conform-baseline.json` 36 B, `conform.toml` 7529 B); ни один не покрыт `.gitignore` (правила `/target/`, `**/target/` — `.gitignore:2-3` — их не задевают; отдельного правила для `conform*` нет) |
| B3 | шаг панели `run_step "cargo xtask conform check" …` стоит в `tools/self-check.sh:325`, комментарий над ним — про «content-addressed fact store», который «re-extracts only changed files» | ПОДТВЕРЖДЕНО | `tools/self-check.sh:325` (`run_step "cargo xtask conform check" cargo xtask conform check`); комментарий `tools/self-check.sh:319-324` («its content-addressed fact store re-extracts only changed files») |
| B4 | движок в `…/core-ai-native/v0.7.0/crates/core-ai-native-conform/src/` (есть `baseline.rs`, `config.rs`, `facts.rs`, `finding.rs`, `lib.rs`); рядом есть Rust-фронтенд `rust-ai-native-conform` | ПОДТВЕРЖДЕНО | директория существует и содержит ровно эти файлы (+`store.rs`, `sarif.rs`, `rules/`); фронтенд найден в ДВУХ стеках: `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform` и `…/rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-conform`. **Оговорка к B4 (см. §9):** указанный в координате каталог `v0.7.0` — устаревший снимок; то, что бежит — вендор-копия `CONF` (v0.8.0-эквивалент) |
| B5 | во всём хосте зависимость на движок качества несёт ровно один крейт — `xtask`; `vibe-trace`/`vibe-cli`/`vibe-mcp` его не знают | ПОДТВЕРЖДЕНО | conform-зависимости только в `xtask/Cargo.toml:22-24` (`rust-ai-native-conform.workspace`, `conform-core.workspace`, `rust-ai-native-conform-frontend.workspace`); workspace-ключи — `Cargo.toml:103-105`. Grep `conform` по `crates/*/Cargo.toml` и корневому `Cargo.toml` (кроме объявления ключей) находок не дал — ни `vibe-trace`, ни `vibe-cli`, ни `vibe-mcp` conform не несут |

## 3. Артефакт: путь, писатель, формат, git-статус

Conform кладёт на диск ТРИ объекта. Только первый (SARIF) несёт находки с
(file,line); остальные — либо отпечатки без строк, либо факты (входы правил).

### 3.1. `target/conform/report.sarif` — отчёт находок (то, что нужно слайсу)

- **путь:** `target/conform/report.sarif` (от корня репозитория).
- **кто пишет:** `run_check` — рендер SARIF и запись:
  `FE/lib.rs:171` `let report = sarif::render(&rule_refs, &findings);`
  `FE/lib.rs:172` `let sarif_path = root.join("target").join("conform").join("report.sarif");`
  `FE/lib.rs:176` `std::fs::write(&sarif_path, &report)?;`
- **формат:** SARIF 2.1.0, pretty-JSON; рендерер — `CONF/sarif.rs:23` `pub fn render(rules, findings) -> String`. На каждую находку: `ruleId`, `level:"error"`, `message.text`, `partialFingerprints`, `locations[0].physicalLocation.artifactLocation.uri` (=file), `region.startLine` (=line), `properties.vibevmConform/evidence`, `properties.vibevmConform/status`; для принятого отступления — `suppressions[{kind:"inSource",justification}]` (`CONF/sarif.rs:33-77`). Сериализуемый тип — `Finding` (§4).
- **под git?** НЕТ. Покрыт правилом `.gitignore:3` (`**/target/`), плюс `.gitignore:2` (`/target/`).
- **существует прямо сейчас в этом worktree?** НЕТ (`ls target/conform/report.sarif` ⇒ No such file or directory; `target/conform/` отсутствует).

### 3.2. `conform-baseline.json` — замороженный ракет (коммичен)

- **путь:** `conform-baseline.json` (корень репозитория).
- **кто пишет:** `run_freeze` — только эта команда переписывает baseline:
  `FE/lib.rs:250` `let fps = baseline::freezeable(&findings);`
  `FE/lib.rs:251` `let body = serde_json::json!({ "schema": 1, "findings": fps });`
  `FE/lib.rs:255` `std::fs::write(&path, text)…`. Команда `check` baseline только ЧИТАЕТ (`FE/lib.rs:178` `baseline::load(…)`).
- **формат:** JSON `{ "schema": 1, "findings": [<строка-отпечаток>, …] }`. Сериализуемый тип — `Baseline { schema: u32, findings: Vec<String> }` (`CONF/baseline.rs:22-27`); `findings` — это отпечатки (`Vec<&str>` из `freezeable`, `CONF/baseline.rs:98-107`), НЕ объекты `Finding`.
- **под git?** ДА. Отдельного правила в `.gitignore` нет; файл присутствует в свежем worktree.
- **существует прямо сейчас?** ДА. Содержимое целиком (4 строки, 36 байт):
  ```
  {
    "findings": [],
    "schema": 1
  }
  ```

### 3.3. `target/conform/facts/<id>-<ver>/<sha256>.json` — кэш фактов (входы правил, НЕ находки)

- **путь:** `target/conform/facts/<frontend_id>-<frontend_version>/<content_hash>.json`.
- **кто пишет:** `Store::for_rust` ставит root `target/conform/facts` (`CONF/store.rs:61,63`); запись слота — `CONF/store.rs:178` `std::fs::write(&slot, serde_json::to_string(&fresh)?)?` (слот — `CONF/store.rs:92` `fn slot`).
- **формат:** JSON `Vec<Fact>` (нормализованная модель исходника, входы правил), НЕ находки.
- **под git?** НЕТ (`.gitignore:2-3`, `target/`).
- **существует прямо сейчас?** НЕТ (весь `target/conform/` отсутствует).

## 4. Тип находки — дословно

Бегущий тип — из вендор-копии (v0.8.0-эквивалент); `CONF/finding.rs:29-57` +
enum `CONF/finding.rs:87-98`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub rule: &'static str,
    pub file: String,
    pub line: u32,
    pub message: String,
    /// Why the rule exists — the axiom trace rendered into SARIF.
    pub why: &'static str,
    /// Stable identity for the baseline: `rule|file|carrier`.
    pub fingerprint: String,
    // --- B-025 (mark, don't suppress): a deviation no longer vanishes ---
    pub status: FindingStatus,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingStatus {
    Live,
    DeviationAcknowledged { reason: Option<String> },
}
```

Построчный разбор:

- `rule: &'static str` → статичный идентификатор правила (напр. `"unsafe-gate"`). Идентификатор правила — ДА. Как (file,line)-ключ — не ключ, но группирует.
- `file: String` → репо-относительный путь с прямыми слешами. **Это первая половина (file,line)-ключа.** Годится.
- `line: u32` → 1-номерная строка. **Это вторая половина (file,line)-ключа.** Годится.
- `message: String` → человекочитаемый текст (по грамматике `req_message`, ссылается на `discipline://…`). Человекочитаемый текст — ДА.
- `why: &'static str` → почему правило существует (выводится в SARIF `shortDescription`).
- `fingerprint: String` → стабильный идентификатор `rule|file|carrier` (`CONF/finding.rs:38`). Именно он замораживается в baseline (БЕЗ `line`).
- `status: FindingStatus` → `Live` либо `DeviationAcknowledged{reason}` (B-025 «помечать, не гасить»); в SARIF становится `properties.vibevmConform/status` + `suppressions`.
- `evidence: String` → компактный рендер факта, породившего находку (для визуализатора).

**Вывод §4:** находка несёт и `file`, и `line`, и идентификатор правила, и
человекочитаемый `message`. (file,line)-ключ присутствует и годен. Отпечаток
`fingerprint` этот ключ НЕ содержит (там `rule|file|carrier`, без строки) —
поэтому baseline-данные (§3.2) для соединения не годятся, годится только SARIF.

## 5. Время прогона

**Ни один артефакт не несёт времени прогона.** Доказательства:

- SARIF намеренно без wall-clock: `CONF/sarif.rs:12-13`
  «Byte-stable minimal SARIF 2.1.0: stable ordering (findings are pre-sorted),
  **no wall-clock**, no absolute paths.» В JSON-документе SARIF
  (`CONF/sarif.rs:78-89`) полей времени нет — только статичные `$schema`,
  `version:"2.1.0"`, `tool.driver{name,version:"0.1.0"}`.
- `conform-baseline.json` хранит только `schema` и `findings` (отпечатки),
  поля времени нет (`CONF/baseline.rs:22-27`).
- Кэш фактов ключируется хэшем содержимого, не временем (`CONF/store.rs:92` `fn slot`,
  ключ `<id>-<ver>/<content_hash>`).

**Что остаётся вместо него:** только mtime файла `target/conform/report.sarif`.
Это слабый сигнал — файл пересоздаётся каждым `check`, а сам лежит под
`target/` и отсутствует в свежем клоне. Оговорка свежести, которую владелец
назвал обязательной (B019-V-RECOMMENDATION, 2026-08-13: «ответ называет, КОГДА
получены находки, а отсутствие отчёта — это „не измерено“»), из артефакта
удовлетворена быть не может — её нужно строить отдельно (см. §9).

## 6. Базовая линия против отчёта

`conform-baseline.json` — это **ЗАМОРОЖЕННЫЙ РАКЕТ отпечатков принятого долга,
не отчёт последнего прогона.**

По коду:

- Это ракет «только уменьшается»: `CONF/baseline.rs:10-11`
  «`conform-baseline.json`: frozen pre-existing findings, by fingerprint. The
  file only shrinks.»
- Пишется ТОЛЬКО командой `freeze` (`FE/lib.rs:250-255`), и пишет туда
  `freezeable(&findings)` — отсортированные дедуплицированные отпечатки LIVE-находок
  (`CONF/baseline.rs:98-107`), принимая отступления (B-025, `CONF/baseline.rs:100-101`).
  `check` его не пишет, только читает для диффа (`FE/lib.rs:178-179`).
- Команда `check` отдельно рапортует «new» (мимо baseline) и «stale»
  (`FE/lib.rs:179-188`) и падает на любом new (`FE/lib.rs:207-209`).

По содержимому — файл целиком (4 строки, `wc -l` = 4, `wc -c` = 36):

```
{
  "findings": [],
  "schema": 1
}
```

`findings` сейчас пуст: либо долг действительно нулевой, либо (правдоподобнее,
учитывая «file only shrinks») он был сведён к нулю ранее. Формально — замороженный
список ноль отпечатков; это НЕ снимок последнего прогона (в прогоне находок может
быть сколько угодно, но все они либо `Live`+`new`/`stale`, либо
`DeviationAcknowledged`, и ни одна не оседает в baseline, пока её не «freeze»-нут).

**Вывод §6:** для слайса это решающий факт — **`check` ничего не пишет в git,
только `freeze`**, и `freeze` пишет отпечатки без строк. Отчёт с (file,line)
порождается исключительно `check` как побочный эффект в лежалый `target/`.

## 7. Приёмная поверхность и её провенанс

Слайс S6 должен показывать находки рядом с узлами карты в «простом уровне
поиска по карте» (программа А5 = `vibe query`) и в `explain`.

| поле | значение |
|---|---|
| **поверхность** | Простой уровень поиска — `vibe query` (A5A-MAPSEARCH) и надстройка `vibe select` (E-A5B-QUERYLANG, графовый обход поверх простого уровня). `explain` — отдельная поверхность (см. ниже). |
| **где реализована** | Способность — в крейте `vibe-trace`: `vibe query` → `crates/vibe-trace/src/search.rs:211` `pub fn query(root, filters)`, чистое ядро `search.rs:171` `pub fn search(map, filters)`; `vibe select` → `crates/vibe-trace/src/select.rs:331` `pub fn query`, ядро `select.rs:147` `pub fn select`. CLI (`crates/vibe-cli/src/cli/{query,select}.rs`) и MCP — тонкие поверхности, «build no map of its own» (`crates/vibe-cli/src/cli/query.rs:14`, `select.rs:15`). Карта строится свежей на каждый запрос (`search.rs:211-214`, `select.rs:331` → `specmap_core::index::build`). |
| **что возвращает** | `SearchOut { hits: Vec<Hit>, total_matching, limit }` (`search.rs:121-130`); каждый `Hit` (`search.rs:54-75`) несёт `source, name, kind, file, line, uri?, crate_name?`. `vibe select` оборачивает тот же `Hit`: `SelectHit { #[serde(flatten)] hit: Hit, depth }` (`select.rs:62-69`), импортируя `Hit, HitSource` из `search` (`select.rs:44-46`). |
| **есть ли провенанс-поле** | **ДА (структурированное)** — `Hit.source: HitSource` (`search.rs:57`), где `HitSource` (`search.rs:41-48`) — enum `{Spec, Code}`. Документация называет его **задокументированной точкой расширения для второго поставщика данных**: `search.rs:37-40` «Where a `Hit` was taken from — its provenance (Р6). Two sources feed the map today; **a future code-quality engine joins at query time as a new variant here**, flowing through the same `Vec<Hit>` and renderers. The enum is the documented extension point for the second data provider.» И ещё `search.rs:13-15` «Each hit carries its source (`HitSource`) so a future second data provider can join at query time (Р6) — nothing of that engine is built today, only the door stays open.» |

**Про `explain` — оговорка:** у `explain` структурированного провенанс-поля на
каждый hit НЕТ. `explain` возвращает `Explain{Text(String)|Json(Value)}`
(`crates/vibe-trace/src/lib.rs:62-68`), рёбра несут `file:line`
(`lib.rs:210-212`), но провенанс там — лишь текстовая строка, различающая
«fresh build» и «carried foreign map» (V6-FOREIGN-EXPLAIN): `lib.rs:85-86`
«one provenance line marks that the data came from a carried map, not a fresh
build.» То есть дверь под второго поставщика (`HitSource`) есть в `vibe
query`/`select`, но НЕ в `explain` — соединение через `explain` потребует
отдельной работы.

## 8. Карта зависимостей (conform | specmap)

Workspace-ключи объявлены в корневом `Cargo.toml`: conform — `:103-105`,
specmap — `:108-109` (обе через `path = "…/rust-ai-native-lang/v0.7.0/crates/…"`).

**conform (движок качества)** — ровно ОДИН потребитель в хосте:

| crate | на что зависит | цитата |
|---|---|---|
| `xtask` | `rust-ai-native-conform` | `xtask/Cargo.toml:22` |
| `xtask` | `conform-core` (= `core-ai-native-conform`) | `xtask/Cargo.toml:23` |
| `xtask` | `rust-ai-native-conform-frontend` | `xtask/Cargo.toml:24` |

(ключи: `Cargo.toml:103` `conform-core`, `:104` `rust-ai-native-conform-frontend`, `:105` `rust-ai-native-conform`.)

**specmap (карта)** — ТРИ потребителя в хосте:

| crate | на что зависит | цитата |
|---|---|---|
| `xtask` | `specmap-core` | `xtask/Cargo.toml:19` |
| `xtask` | `rust-ai-native-specmap` | `xtask/Cargo.toml:20` |
| `vibe-cli` | `specmap-core` | `crates/vibe-cli/Cargo.toml:39` |
| `vibe-trace` | `specmap-core` | `crates/vibe-trace/Cargo.toml:15` |

(ключи: `Cargo.toml:108` `specmap-core`, `:109` `rust-ai-native-specmap`.)

**Вывод §8 (он же будущий grep-гейт слайса):** conform сегодня держит один
крейт — `xtask`; specmap — `xtask`, `vibe-cli`, `vibe-trace`. Ни `vibe-mcp`,
ни `vibe-trace` conform не знают; `vibe-trace` (где живёт приёмная поверхность
§7) держит ТОЛЬКО specmap и conform НЕ держит. Поэтому маршрут (3) — соединение
по данным в момент запроса — не порождает новой cargo-связи между движками: conform
оставлен зависимостью одного `xtask`, а данные (SARIF) заносятся значением в
`HitSource`, не типом.

## 9. Дыры и неожиданности

1. **B4 указывает на устаревший снимок, а не на бегущий код (главная неожиданность).**
   В дереве ДВЕ версии движка: `core-ai-native/v0.7.0/…` (прочитанная по
   координате B4) и `core-ai-native/v0.8.0/…`. В v0.7.0 у `Finding` ШЕСТЬ полей,
   НЕТ `freezeable` и НЕТ `Store::for_rust`; в v0.8.0 (и в вендор-копии `CONF`,
   которая реально компилируется по `Cargo.toml:103`) — ВОСЕМЬ полей (добавлены
   `status`/`evidence`, B-025) и есть `freezeable`/`for_rust`. То есть каталог
   `v0.7.0` — устаревший авторский снимок; вендор-синк-гейт
   (`tools/self-check.sh:327-332`) держит вендор-копии байт-в-байт с v0.8.0, а
   v0.7.0 в этот синк не входит. Все цитаты типов/артефактов выше даны по `CONF`
   (v0.8.0-эквивалент) — то есть по тому, что бежит.

2. **Коммиченный артефакт не несёт (file,line).** `conform-baseline.json` хранит
   отпечатки `rule|file|carrier` (`CONF/finding.rs:38`, `CONF/baseline.rs:18`),
   где строки НЕТ — есть «carrier» вроде `block#0`. Значит единственные
   коммиченные conform-данные принципиально не могут служить (file,line)-ключом.
   Соединение возможно только против `target/conform/report.sarif`, а его в
   клоне нет.

3. **Ни одного времени прогона нигде.** SARIF «no wall-clock» по дизайну
   (`CONF/sarif.rs:12-13`); baseline и fact-store — тоже без времени. Оговорка
   свежести владельца (B019-V-RECOMMENDATION) опирается на факт, которого в
   данных просто нет: его придётся завести (хотя бы как mtime-пробу файла
   SARIF + явное «нет файла ⇒ не измерено»). Это, пожалуй, единственная
   настоящая ДЫРА — остальное работой.

4. **`check` не пишет в git, `freeze` — пишет отпечатки.** Из `FE/lib.rs`
   однозначно: `check` пишет только SARIF в `target/` (`:172-176`), `freeze`
   пишет отпечатки в baseline (`:250-255`). Шаг панели
   (`tools/self-check.sh:325`) гоняет именно `check`, то есть на CI порождается
   SARIF, который тут же живёт в лежалом `target/` и исчезает. Любой потребитель
   соединения должен либо сам прогонять `check`, либо читать SARIF из
   переживший-прогон `target/`.

5. **Соединение архитектурно предзаложено — это не новый шов.** `HitSource`
   (`search.rs:41-48`) с комментарием «a future code-quality engine joins at
   query time as a new variant here» — это и есть дверь, которую вариант (3)
   ждал (B019-V-THE-OWNER-CHOSE-THREE). Добавить conform-находки = добавить
   вариант `HitSource` + наполнить `Vec<Hit>` теми же `file`/`line`, что уже
   несёт карта. Рендереры текст/JSON (`search.rs:308-384`) переключаются по
   `hit.source` — то есть новая варианта потребует одной ветки в рендере.

6. **`explain` провенанса не имеет.** Дверь `HitSource` есть только у
   `vibe query`/`select`; `explain` (`vibe-trace/src/lib.rs:136`) вернёт
   `Explain` без поля поставщика. Если слайс хочет находки и в `explain`, это
   отдельный механизм, не бесплатное следствие.

## 10. Как воспроизвести этот замер

Каждое число — один глагол. Находясь в корне worktree:

- `ls -la conform.toml conform-baseline.json` → оба присутствуют, размеры 7529 / 36.
- `wc -l -c conform-baseline.json` → `4 36`.
- `cat conform-baseline.json` → `{ "findings": [], "schema": 1 }`.
- `ls target/conform/` → No such file (подтверждает: SARIF/fact-store отсутствуют).
- `grep -n "conform" .gitignore` → пусто (правил для `conform*` нет); `grep -n target .gitignore` → `:2 /target/`, `:3 **/target/`.
- `sed -n '319,325p' tools/self-check.sh` → шаг `cargo xtask conform check` + комментарий про fact store.
- `sed -n '13,19p' xtask/src/conform.rs` → обёртки `run_conform_check`/`run_conform_freeze`.
- `sed -n '270,292p' xtask/src/main.rs` → `ConformCmd::{Check,Freeze}` (Check «emit SARIF … gate»; Freeze «Rewrite the baseline»).
- `grep -n "report.sarif\|fs::write" vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs` → `:172` (путь SARIF), `:176` (запись), `:255` (запись baseline).
- `sed -n '23,98p' vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-conform/src/sarif.rs` → рендер SARIF, «no wall-clock», поля включая `startLine`.
- `sed -n '29,98p' …/vendor/core-ai-native-conform/src/finding.rs` → тип `Finding` (8 полей) + `FindingStatus`.
- `sed -n '22,27p;98,107p' …/vendor/core-ai-native-conform/src/baseline.rs` → `Baseline{schema,findings:Vec<String>}` + `freezeable`.
- `grep -rni "conform" Cargo.toml xtask/Cargo.toml crates/*/Cargo.toml` → conform только в `xtask/Cargo.toml:22-24` (+ объявление ключей в `Cargo.toml:103-105`).
- `grep -rni "specmap" crates/*/Cargo.toml xtask/Cargo.toml` → specmap в `xtask`, `vibe-cli`, `vibe-trace`.
- `sed -n '37,75p' crates/vibe-trace/src/search.rs` → `HitSource` (дверь второго поставщика) + `Hit{source,…,file,line}`.
- `find vibevm/vibepacks/org.vibevm.ai-native -name baseline.rs` → две версии (v0.7.0 без `freezeable`, v0.8.0 + вендор-копии с `freezeable`).
- `grep -rn "fn freezeable" packages/` → только в v0.8.0 и вендор-копиях, НЕ в v0.7.0 (доказывает §9.1).
