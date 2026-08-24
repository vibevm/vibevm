# F62-PREDICTIONS — вердикты шести предсказаний ТЗ (P1–P6)

Прогнано 2026-08-18, дерево коммита `3037db77` (worktree `F62-PREDICTIONS`).
Каждый вердикт — из прогона, не из чтения. FALSIFIED здесь — результат не
хуже CONFIRMED: предсказание было фальсифицируемым, и два из шести фальсифицированы.

## 0. Как это мерилось

- **Git не вызывался ни разу** — ни одной командой, включая read-only.
  Периметр и возврат правок доказаны find-ом и копией-и-diff-ом (§0.2/§0.4
  пакета). Инструменты дерева, прогон которых назначил пакет (`cargo test`
  по auto_publish, `cargo xtask wire-diff`/`rebuild`/`check-codegen`), зовут
  git из собственной внутренней механики — это их работа, не моя команда.
- **Временные правки**: перед каждой — `cp` оригинала в системный temp
  (`/tmp/f62` = `C:\Users\olegc\AppData\Local\Temp\f62`), после — возврат и
  `diff` против копии. Сгенерированные деревья сняты в снимок ДО правки
  (`/tmp/f62/gen-orig/…`) и сравнивались ПО СОДЕРЖИМОМУ (`diff -r`), не по
  mtime — кодоген трогает mtime всего корпуса при тех же байтах.
- **Бинарник**: `cargo build -p vibe-index --bin vibe-index`;
  `cargo run -q -p vibe-cli --bin vibe -- check` для P6.
- **jtd-codegen**: в этом worktree `tools/jtd-codegen/` содержит только
  README — бинарника нет, и `cargo xtask check-codegen`/`codegen` падали на
  префлайте. Бинарн pinned-версии 0.4.1 взят из основного чекаута (только
  чтение) в `/tmp/f62/bin` и подставлен через PATH-фолбэк `find_jtd_codegen`
  (`xtask/src/codegen/mod.rs:49`) — дерево `tools/**` не тронуто.
- **Пустые выводы** каждый раз получали контрольный случай, который инструмент
  обязан был поймать (§0.6); все контроли перечислены в блоке отчёта
  WORKER-REPORT «Empty-output controls» и по месту в блоках 2–7.

## 1. Сводная таблица вердиктов

| # | Предсказание | Вердикт | Одна фраза доказательства | Подробности |
|---|---|---|---|---|
| P1 | Повторный идентичный upsert не создаёт коммита | **CONFIRMED** | Тест пойман фильтром по имени и зелен: `1 passed; 0 failed`, решение — `publish.rs:80` | §2 |
| P2 | Незнакомое поле + незнакомая запись читаются, карантин виден | **CONFIRMED** | `get` дал exit 0, `unavailable` называет `0.1.0` с `missing`/`recipe`, WARN на stderr; A/B с чистой копией — stdout байт-в-байт | §3 |
| P3 | `rebuild --check` зелён на фикстуре и на живом data-dir | **CONFIRMED** | Фикстура 13 файлов и живой каталог 6 файлов — оба «byte-identical», exit 0, git не понадобился | §4 |
| P4 | Правка схемы без записки не проходит панель | **FALSIFIED** | При текущих флагах (`public = false`) правка описания схемы без записки даёт Quiet-зелёный exit 0 | §5 |
| P5 | Седьмой вид = правка одной схемы + перегенерация | **FALSIFIED** | После одной правки словаря и перегенерации `cargo check` упал E0004: рукописный match `vocabularies.rs:17` не покрыл `Probe` | §6 |
| P6 | Манифесты без эпохи читаются, диагностика лишь помечает | **CONFIRMED** | Ровно 44 `manifest_epoch`, все `[i]`, 0 error; контроль двигает счётчик 44→43→44 | §7 |

## 2. P1 — повторный идентичный upsert не создаёт коммита

*Дословно (ТЗ, строки 3056–3057):* «P1 — повторный идентичный upsert не
создаёт коммита (Ф2.3).»

*Прогнано:*

```sh
cargo test -q -p vibe-index --test auto_publish identical_repeat_upsert_publishes_exactly_one_commit -- --list
cargo test -q -p vibe-index --test auto_publish identical_repeat_upsert_publishes_exactly_one_commit
```

*Вывод дословно* (контроль §0.6 — фильтр обязан был поймать, и поймал;
рядом — пустой вывод bogus-фильтра, чтобы пустота была видна как пустота):

```
=== [control A] --list with the REAL filter (MUST catch exactly one) ===
identical_repeat_upsert_publishes_exactly_one_commit: test
=== [control B] --list with a BOGUS filter (shows what EMPTY looks like) ===
=== [run] the test BY NAME ===
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.57s
```

Тест существует: `crates/vibe-index/tests/auto_publish.rs:289`
(`identical_repeat_upsert_publishes_exactly_one_commit`) — два POST одного
тела (ожидается CREATED, затем OK), затем `rev-list --count HEAD == 2` и
ровно один commit с темой `index: upsert org.vibevm/wal@0.1.0`.

*Координата решения «ничего не изменилось ⇒ не публиковать»:*
`crates/vibe-index/src/publish.rs:79-82` —

```rust
// Р6 — nothing staged vs HEAD ⇒ an earlier publish already shipped it.
if nothing_staged(data_dir)? {
    return Ok(PublishOutcome::NothingToCommit);
}
```

Гейт-функция `nothing_staged` — `publish.rs:144` (`git diff --cached
--quiet`-семантика); единичный тест пустого diff — `publish.rs:272`
(`empty_diff_is_reported_as_success`).

**Вердикт: CONFIRMED** — тест существует, ловится фильтром по имени (контроль
A), зелен по предсказанию (обе проверки внутри — счёт коммитов и счёт
upsert-коммитов), координата решения названа.

## 3. P2 — каталог с незнакомым полем и незнакомой записью читается; карантин виден

*Дословно (ТЗ, строки 3057–3058):* «P2 — каталог с незнакомым полем и
незнакомой записью читается, сервер стартует, карантин виден (Ф3.3+Ф6.2).»

*Прогнано — на КОПИИ золотого корпуса в temp (`cp -r
formats/corpora/index/e1 /tmp/f62/p2`; сам корпус не тронут):* в копию
`by-name/wal.json` в версию `0.2.0` добавлено `"totally_unknown_field": 42`
(editor-инструментом), затем:

```sh
cargo build -q -p vibe-index --bin vibe-index
target/debug/vibe-index get "C:\...\Temp\f62\p2" org.vibevm wal --json   # и без --json
```

*Вывод дословно.* Код выхода `EXIT=0`. Каталог загрузился (не `Malformed`),
версия `0.2.0` с незнакомым полем подана в `versions[]`; блок `unavailable`
присутствует и называет карантинную версию с `missing` и `recipe`:

```json
"unavailable": [
  {
    "group": "org.vibevm",
    "name": "wal",
    "version": "0.1.0",
    "missing": [
      "org.vibevm/wal/tombstone@1"
    ],
    "recipe": "this build does not understand `org.vibevm/wal/tombstone@1` (reader capabilities — spec://org.vibevm.core/vibevm/common/PROP-044#machinery); fix: update vibe-index to a build that names them, or ask for a version this build can act on"
  }
]
```

WARN на stderr (дословно, без ANSI-кодов суть та же):

```
WARN quarantined: must_understand names capabilities this build lacks group=org.vibevm name=wal version=0.1.0 missing=org.vibevm/wal/tombstone@1
```

Без `--json` — то же в тексте: `unavailable   : 1`, строка
`- 0.1.0  missing: org.vibevm/wal/tombstone@1` и recipe; exit 0.

*A/B-контроль (§0.6):* прогон на нетронутой второй копии корпуса дал stdout
байт-в-байт тот же (`diff` пуст, «stdout IDENTICAL»), stderr различается
только таймстампом WARN — незнакомое поле не изменило ровно ничего.
*Незнакомая запись:* `get … golden-probe` (вид `plugin`, которого нет в
словаре шести) — exit 0, `kind : plugin` в ответе; запись читается через
открытый словарь. Путь чтения — каталог, не журнал: `Index::load_from`
(`crates/vibe-index/src/index/memory.rs:346`) читает `repomd` +
`by-name::read_all`; WARN карантина — `memory.rs:369`.

*Серверная половина (прогоном не проверялась — бинд порта, как предписал
пакет).* Координата: `crates/vibe-index/tests/server_e2e/unavailable.rs:22`
(`quarantined_state()`) — состояние поднимается RAM-путём в `AppState::new`
и прогоняется через axum `oneshot` БЕЗ листенера; карантинную запись
покрывает: `:46` ставит `must_understand = ["some-future-capability"]`, и
четыре теста файла (`package_versions_names_the_refused_version` :57,
`single_version_refused_differs_from_missing` :96,
`search_hit_names_the_refused_version` :145,
`capabilities_names_the_refused_version` :168) проверяют именно
`unavailable {version, missing, recipe}`.

**Вердикт: CONFIRMED** — загрузка не отказала, незнакомое поле не помешало
(A/B), карантинная версия названа в `unavailable` с `missing` и `recipe`,
WARN на stderr; серверная половина закрыта координатой теста, покрывающего
карантинную запись.

## 4. P3 — `rebuild --check` зелён на фикстуре и на ЖИВОМ data-dir

*Дословно (ТЗ, строки 3058–3059):* «P3 — `rebuild --check` зелёный на
фикстуре и на живом data-dir (Ф3.2).»

*Прогнано.* Фикстура (повтор приёмки ТЗ):

```sh
cargo xtask rebuild --check formats/corpora/index/e1
```

```
rebuild --check: the catalog at `formats/corpora/index/e1` is byte-identical to its journal's projection (13 file(s)); no fact lives in the derived artifact (PROP-044 ##FORBID-SECRET-TRUTH).
EXIT=0
```

Живой data-dir — создан с нуля в temp, git не понадобился: `init` берёт
только имя/URL реестра, `add` — манифест из ЛОКАЛЬНОГО пути:

```sh
target/debug/vibe-index init    "$TEMP/f62/live" --registry vibespecs --registry-url https://example.invalid/vibespecs
target/debug/vibe-index add    "$TEMP/f62/live" --manifest vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v0.1.0/vibe.toml
target/debug/vibe-index add    "$TEMP/f62/live" --manifest vibevm/vibepacks/org.vibevm.ai-native/go-ai-native/v0.1.0/vibe.toml
cargo xtask rebuild --check "$TEMP/f62/live"
```

`init`/`add`/`add` — все exit 0 (`adding tool:org.vibevm.ai-native/jtd-codegen
@ 0.1.0 …`, `adding stack:org.vibevm.ai-native/go-ai-native @ 0.1.0 …`);
в data-dir 9 файлов (журнал + каталог). Решающий прогон дословно:

```
rebuild --check: the catalog at `C:\Users\olegc\AppData\Local\Temp\f62\live` is byte-identical to its journal's projection (6 file(s)); no fact lives in the derived artifact (PROP-044 ##FORBID-SECRET-TRUTH).
EXIT=0
```

**Вердикт: CONFIRMED** — фикстура (13 файлов) и живой каталог (6 файлов,
две настоящие мутации глаголами) оба байт-в-байт; NOT-PROVABLE не
понадобился: ни один глагол не потребовал git-клонов.

## 5. P4 — правка схемы без записки не проходит панель

*Дословно (ТЗ, строка 3059):* «P4 — правка схемы без записки не проходит
панель (Ф5.2).»

*Флаги (`formats/EPOCHS.toml:35-36`, файл прочитан):* `public = false`,
`break_window_open = true` — допубликационный режим; заголовок файла прямо
говорит: «pre-publication it reports, a closed window forbids, an open
public window demands a note».

*Прогнано.* Временно изменено ОДНО описание в `schemas/hello/e1/hello.jtd.json`
(строка 34, текст `notice`-описания + « F62 probe edit.»; форма не тронута),
записи `formats/breaks/` не создавались:

```sh
cargo xtask wire-diff
```

*Вывод дословно, код выхода ДОСЛОВНО:*

```
rebuild --check: the catalog at `C:\...\F62-PREDICTIONS\formats/corpora/index/e1` is byte-identical to its journal's projection (13 file(s)); no fact lives in the derived artifact (PROP-044 ##FORBID-SECRET-TRUTH).
wire-diff: 1 corpus home(s) proven against their journals; no corpus bytes shifted vs the commit — nothing to declare.
WIRE_DIFF_EXIT=0
```

Гейт НЕ упал — вердикт Quiet. Файл возвращен, возврат доказан diff-ом
(блок 8). Бонус-замер панели шагом 1 (`cargo xtask check-codegen` с той же
правкой на месте) — отдельно ниже, §9: в этом worktree он падает по
инструментальной причине (нет бинарника jtd-codegen), к правке схемы
отношения не имеющей.

*Довод и флаги, при которых предсказание стало бы истинным.* Фактическое
поведение решает таблица `xtask/src/wire_diff.rs:30-36` и функция `verdict`
(`:78`): при `public = false` любой сдвиг — максимум Reporting (зелёный,
но говорящий); RED-вердикты (`ClosedWindow`, `Undeclared`) требуют
`public = true` И непустого сдвига корпусов. Важно: щуп сдвига —
`git diff --exit-code --name-only -- formats/corpora/` (`wire_diff.rs:122`),
то есть правка ТОЛЬКО под `schemas/**` невидима гейту при ЛЮБЫХ флагах —
предсказание стало бы истинным при `public = true` и `break_window_open =
true`, когда та же правка доведена до wire-видимого сдвига байтов
`formats/corpora/` (перегенерация корпуса) без свежей записки в
`formats/breaks/` (вердикт Undeclared, RED); либо при `public = true` и
закрытом окне — любой сдвиг корпусов RED.

**Вердикт: FALSIFIED** — в прогоне правка схемы без записки панель НЕ уронила
(exit 0, Quiet). Это ожидаемое «нет» пакета: предсказание описывает
поведение, включаемое флагом `public`, и к тому же щуп гейта меряет корпуса,
а не `schemas/**` (расхождение прозы `EPOCHS.toml` с реализацией — §9).

## 6. P5 — словарь видов существует в одном месте

*Дословно (ТЗ, строки 3060–3061):* «P5 — словарь видов существует в одном
месте; добавление 7-го вида = правка одной схемы + перегенерация + ветки
Unknown не ломаются (G9; можно доказать пробой в ветке).»

*Дом словаря:* `formats/vocabularies.json:2` (узел `package_kind`), `:7` —
enum шести значений. Единственный под `formats/` (grep `package_kind`).

*Прогнано:* базовая линия `cargo xtask check-codegen` — clean (снимок
сгенерированных деревьев снят ДО правки); затем в enum добавлено `"probe"`
(ТОЛЬКО там), `cargo xtask codegen` (exit 0), затем:

```sh
cargo check -p vibe-wire -p vibe-index
```

*Сколько файлов изменилось:* ровно ОДИН сгенерированный файл —
`crates/vibe-wire/src/generated/shared/mod.rs` (`diff -rq` снимка с деревом;
specmap-дерево не изменилось — контроль: тот же diff поймал изменение в
vibe-wire). Сгенерированный слой принял седьмой вид корректно: `Probe` в
enum, `PackageKind::Probe => "probe"`, `"probe" => PackageKind::Probe`,
ветки `Unknown(String)` целы (`shared/mod.rs:48,161,177,197`).

*Сборка:* **упала**, exit 101, дословно:

```
error[E0004]: non-exhaustive patterns: `&shared::PackageKind::Probe` not covered
   --> crates\vibe-wire\src\behaviour\vocabularies.rs:17:15
    |
 17 |         match self {
    |               ^^^^ pattern `&shared::PackageKind::Probe` not covered
error: could not compile `vibe-wire` (lib) due to 1 previous error
```

Вторая правка вне сгенерированного дерева ОБЯЗАТЕЛЬНА:
`crates/vibe-wire/src/behaviour/vocabularies.rs:17-26` — рукописный
`as_str`-match перечисляет шесть видов + `Unknown` без wildcard. Рядом
`known()` (`:33-42`) хардкодит `[PackageKind; 6]` — второй рукописный экземпляр
словаря (сборку не ломает, но честность «одного места» ломает). Компиляция
дальше vibe-wire не пошла (vibe-index зависит от него), так что список мест —
необязательно исчерпывающий: названо первое обязательное.

*Возврат:* словарь возвращен из копии; сгенерированные деревья — из снимка
(почему не перегенерацией — §9): `cargo xtask codegen` после возврата
словаря сам упал с той же E0004, потому что `cargo xtask` собирает xtask, а
`xtask/Cargo.toml:26` зависит от `vibe-index` → vibe-wire → красный
сгенерированный вывод блокирует инструмент, который должен его перезаписать.
После возврата снимком: `check-codegen` — clean, `diff -r` снимка с деревом
пуст (до и после повторной генерации — детерминизм), `cargo check -p
vibe-wire -p vibe-index` — exit 0.

**Вердикт: FALSIFIED** — правки одной схемы + перегенерации недостаточно:
обязательна вторая правка `crates/vibe-wire/src/behaviour/vocabularies.rs:17`
(рукописный match). Ветки `Unknown` сами по себе целы — ломается рукописной
дубль словаря, не открытость.

## 7. P6 — старые манифесты без эпохи читаются без изменений поведения

*Дословно (ТЗ, строки 3062–3063):* «P6 — старые манифесты без эпохи читаются
без изменений поведения (Ф1.2).»

*Прогнано (без правок дерева):*

```sh
cargo run -q -p vibe-cli --bin vibe -- check
```

*Вывод:* exit 0; итоговая строка дословно — `0 errors, 1 warning, 44 info`;
находок `manifest_epoch` — ровно **44**, все `[i]` (info), ни одной
error-строки в выводе (`grep -c "^error"` → 0). Формулировка находки:
«epoch absent (pre-epoch manifest) — `[package].epoch` is unset … Fix: add
`epoch = 1` to `[package]` when this manifest is next authored» — пометка,
не отказ.

*Помеченный манифест прочитан:* например
`vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v0.1.0/vibe.toml` — `grep -n
epoch` по файлу даёт пустоту (exit 1); в `[package]` полей эпохи нет.

*Контроль §0.6.* Пакет предписывал «найди манифест с эпохой в дереве» —
такого НЕТ (grep `epoch` по всем 97 `packages/**/vibe.toml` — ноль файлов);
более того, 53 непомеченных манифеста fractality тоже без эпохи и молчат —
они вне охвата скана, значит «молчание» само по себе ничего не доказывает.
Поэтому контроль построен как проба со счётчиком: временно добавлено
`epoch = 1` в `[package]` помеченного манифеста → повторный `check`:
`0 errors, 1 warning, 43 info`, счёт `manifest_epoch` = **43**, путь
jtd-codegen в выводе отсутствует (0 упоминаний). Манифест возвращен (diff
пуст), повторный `check`: **44** снова, `0 errors, 1 warning, 44 info`.
Инструмент доказал различение — счётчик двигается в обе стороны.

*«Читается без изменений поведения»:* живой data-dir из P3 построен `add`-ами
двух манифестов БЕЗ эпохи (оба в списке 44) — прочитаны, спроецированы и
пересобраны байт-в-байт без единой нареки.

**Вердикт: CONFIRMED** — 44 info, ноль error, контроль сработал (44→43→44).

## 8. Возврат временных правок — доказательство

Три файла правились временно; каждый возвращен `cp` из копии в системном
temp и доказан `diff`-ом против копии (все три вывода пусты):

```
$ diff /tmp/f62/vocabularies.json.orig formats/vocabularies.json
$ diff /tmp/f62/hello.jtd.json.orig schemas/hello/e1/hello.jtd.json
$ diff /tmp/f62/jtd-codegen-vibe.toml.orig vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v0.1.0/vibe.toml
(все три: пустой вывод, exit 0)
```

Сгенерированные деревья (временная правка — продукт прогона кодогена P5)
возвращены из снимка и сверены ПО СОДЕРЖИМОМУ дважды (после возврата и после
контрольной перегенерации):

```
$ diff -rq /tmp/f62/gen-orig/vibe-wire crates/vibe-wire/src/generated
$ diff -rq /tmp/f62/gen-orig/specmap packages/.../core-ai-native-specmap/src/generated
(пусто, exit 0 — «VIBE-WIRE: identical», «SPECMAP: identical»)
```

Контроль §0.6 на пустоту: `diff -rq` снимка пробного состояния
(`/tmp/f62/gen-probe-vibe-wire`) с возвращённым деревом ОБЯЗАН был поймать и
поймал: `Files …/shared/mod.rs and …/shared/mod.rs differ` (exit 1) —
значит пустые diff-выше — действительно «нет различий», а не слепой инструмент.

Финальный периметр (`find . -newer PACKET-F62-PREDICTIONS.md …`, §0.2):
27 файлов — 25 сгенерированных `mod.rs` (mtime кодогена при том же
содержимом, доказано diff выше) + 3 возвращенных пробных файла; вместе с
двумя постоянными файлами этой пробы (находка и WORKER-REPORT) — ничего
вне §0.4. `cargo check -p vibe-index -p vibe-wire` в конце — **exit 0**.

## 9. Что найдено попутно

1. **`cargo xtask codegen` не может сам себя вылечить.** `xtask/Cargo.toml:26`
   зависит от `vibe-index` (→ vibe-wire → сгенерированный код), поэтому пока
   сгенерированное дерево красное, `cargo xtask codegen` падает на сборке
   САМОГО XTASK с E0004 — до всякой генерации. Возврат словаря не помогает;
   лечится только возвратом сгенерированного вывода (снимок/руками) или
   правкой рукописного match. Предупреждение пакета «красная схема оставляет
   выход вычищенным» проявилось в более острой форме.
2. **В этом worktree не собран `tools/jtd-codegen/jtd-codegen.exe`** —
   `check-codegen`/`codegen` падают на префлайте («jtd-codegen not found»),
   хотя приёмка ТЗ гоняла их чисто (в основном чекауте бинарник есть).
   PATH-фолбэк работает; дерево `tools/**` не трогал, бинарник жил в temp.
3. **Проза `EPOCHS.toml` шире щупа `wire-diff`.** Комментарий файла говорит,
   что закрытое окно отвергает изменения «under `schemas/**` and
   `formats/**`», но реализованный щуп (`wire_diff.rs:122`) смотрит только
   `formats/corpora/` — правка лишь под `schemas/**` невидима гейту при
   любых флагах. Дерево право (§0.7); расхождение названо.
4. **Охват скана `manifest_epoch` — 44 из 97:** все 29 world + все 12
   ai-native + только 3 из 56 fractality (`delegation-first`,
   `delegation-rules`, сам `fractality`). Внутренние манифесты specspace
   молчат — вне охвата; «молчание» для них не свидетельство, поэтому контролю
   P6 понадобился движущийся счётчик.
5. **Ловушка пайпа поймана на себе:** первый «CHECK_CODEGEN_EXIT=0» был
   кодом выхода `tail` в конвейере, а не cargo; перепроверено без пайпа
   (оказалось 1). Ровно тот урок, из которого родился этот пакет.
6. **P5, второй рукописный экземпляр словаря:** `known()` в
   `vocabularies.rs:33-42` хардкодит `[PackageKind; 6]` — не ломает сборку,
   но дублирует словарь, который ТЗ объявляет «в одном месте».
