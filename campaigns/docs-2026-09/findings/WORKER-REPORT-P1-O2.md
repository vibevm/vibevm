# WORKER-REPORT-P1-O2 — унаследованные красные гейты

Дата: 2026-09-12. Ветка `research-preview-1-docs`.
Дерево на старте работы: `8f956eeb` (пакет называл `b1291b06`; центральная
сессия за время работы добавила `ed75ce00` и `82620654`, HEAD на момент
самопроверки — `82620654`). Ни один `.rs` этими коммитами не затронут, поэтому
измерения остаются в силе.

## 1. Ratchet рукописных derive

### Как воспроизведён шаг

Скрипт не умеет запускать один шаг: флаги — только `--keep-going`, `--quiet`,
`-h`. Шаг ratchet — третий (`run_step` №3 из 47), первые два занимают ~1 с, так
что `bash tools/self-check.sh` падает на нём за 7–9 с. Подсчёт (строки 309–316
`tools/self-check.sh`):

```
WIRE_DERIVE_PATTERN='^[[:space:]]*#\[derive\([^)]*(Serialize|Deserialize)'
grep -rlE --include='*.rs' "$PAT" "$dir" | grep -v '/generated/' | wc -l
```

по каталогам `crates/*/` и `xtask/`; единица счёта — **файл**, не вхождение.
Ручное повторение подсчёта дало те же шесть превышений, что печатает шаг.

### Какие файлы новые относительно заморозки

Для каждого крейта найден коммит, которым его число было зафиксировано
(перебор истории `wire-derive-baseline.json`), затем множество файлов с
рукописным derive на том коммите (`git grep -l -E … <rev> -- <dir>`)
сопоставлено с сегодняшним. Ни одного файла не исчезло; добавились 12.

| крейт | заморожено в | было → стало | новые файлы |
| --- | --- | --- | --- |
| vibe-cli | `99cbdcd3` (2026-08-20) | 42 → 44 | `show/source_path.rs`, `vvm/store.rs` |
| vibe-core | `51a4846c` (2026-09-09) | 33 → 34 | `manifest/package/embedded_source.rs` |
| vibe-publish | `ee4f7230` (2026-08-17) | 3 → 8 | `git_publish/submodules.rs`, `github_release.rs`, `github_release/asset.rs`, `github_release/model.rs`, `release_manifest.rs` |
| vibe-registry | `ee4f7230` (2026-08-17) | 7 → 8 | `embedded_source.rs` |
| vibe-workspace | `f7b4d6d1` (2026-08-22) | 3 → 4 | `publish/staging.rs` |
| xtask | `ee4f7230` (2026-08-17) | 12 → 14 | `bridge.rs`, `dist/build.rs` |

Два файла существовали до заморозки и получили derive позже:
`crates/vibe-cli/src/commands/vvm/store.rs` — в `40590b5c` (2026-09-11),
`crates/vibe-workspace/src/publish/staging.rs` — в `4efad426` (2026-09-11).
Остальные десять появились вместе со своим derive: `72d866d2`, `4efad426`,
`79bf1fc2`, `40590b5c` (2026-09-10…11).

### Классификация типов

Правило шага: (а) **наш wire** — формат, который читает или пишет другой
процесс/язык → JTD-схема и `cargo xtask codegen`; (б) **не wire** —
конфигурация, CLI-локальная структура, чужой формат → поднять счётчик.

| крейт | файл | типы | класс | почему |
| --- | --- | --- | --- | --- |
| vibe-cli | `crates/vibe-cli/src/commands/show/source_path.rs:20,27` | `RootKind`, `SourcePathReport` | б | приватный конверт вывода `vibe show source-path --json` (`ok`/`command` + поля) — CLI-проекция, а не хранимый/обмениваемый формат |
| vibe-cli | `crates/vibe-cli/src/commands/vvm/store.rs:21` | `ActivationJournal` | б | приватный журнал перезаписи указателя `current` внутри install-root; пишет и читает его тот же крейт |
| vibe-core | `crates/vibe-core/src/manifest/package/embedded_source.rs:11,20,28` | `EmbeddedSourceKind`, `EmbeddedSourceAuth`, `EmbeddedSourceDecl` | б | таблица `[[embedded_source]]` авторского TOML-манифеста; продолжение существующей рукописной грамматики манифеста (прецедент `e9d899f6`, `7e928994`) |
| vibe-publish | `crates/vibe-publish/src/github_release.rs:237,274` | `MoveRef`, `CreateRef` | б | тела запросов GitHub REST git-refs — чужой формат |
| vibe-publish | `crates/vibe-publish/src/github_release/asset.rs:94` | `RenameAsset` | б | тело запроса GitHub REST на переименование ассета — чужой формат |
| vibe-publish | `crates/vibe-publish/src/github_release/model.rs:10,37,56,64,84,97,106` | `CreateGithubRelease`, `UpdateGithubRelease`, `GithubMakeLatest`, `GithubRelease`, `GithubReleaseAsset`, `GithubGitObject`, `GithubGitRef` | б | DTO GitHub Releases REST — чужой формат; тот же жанр, что уже засчитанный `github.rs` |
| vibe-publish | `crates/vibe-publish/src/git_publish/submodules.rs:11` | `SubmoduleProvenance` | б | только `Serialize`; попадает в конверт `vibe registry publish --json` (`crates/vibe-cli/src/commands/registry/publish.rs:33,71`) — CLI-проекция |
| vibe-publish | `crates/vibe-publish/src/release_manifest.rs:59,76,88,99,115,124,142` | `DistributionComponentName`, `DistributionComponent`, `DistributionSourceArchive`, `BundleDistributionManifest`, `DistributionAsset`, `PlatformDistributionFragment`, `AggregateDistributionManifest` | **а** | наш wire: `DISTRIBUTION.json` / `DISTRIBUTIONS.json` публикуются как release-ассеты и читаются скриптами `distribution/install/install.sh` и `install.ps1` (другой язык, другой процесс) |
| vibe-registry | `crates/vibe-registry/src/embedded_source.rs:100` | `CacheReceipt` | б | приватная квитанция `source.toml` в кэше пользователя; пишет и читает её тот же крейт (прецедент `f7b4d6d1`) |
| vibe-workspace | `crates/vibe-workspace/src/publish/staging.rs:39` | `SubmoduleProvenance` | б | только `Serialize`; попадает в конверт `vibe workspace publish --json` (`crates/vibe-cli/src/commands/workspace/publish.rs:66`) — CLI-проекция |
| xtask | `xtask/src/bridge.rs:38` | `EmbeddedSourceRow` | б | сопровожденческий инструмент, печатает готовую строку `[[embedded_source]]` для ручной вставки в `vibe.toml`; та же авторская грамматика манифеста |
| xtask | `xtask/src/dist/build.rs:320,329` | `CargoMessage`, `CargoTarget` | б | декодер `cargo --message-format=json` — чужой формат |

### Строки для тела коммита

```
vibe-cli: RootKind + SourcePathReport в crates/vibe-cli/src/commands/show/source_path.rs — приватная проекция вывода `vibe show source-path --json`, не хранимый и не обмениваемый формат.
vibe-cli: ActivationJournal в crates/vibe-cli/src/commands/vvm/store.rs — приватный журнал перезаписи указателя `current` в install-root, который пишет и читает тот же крейт.
vibe-core: EmbeddedSourceKind / EmbeddedSourceAuth / EmbeddedSourceDecl в crates/vibe-core/src/manifest/package/embedded_source.rs — таблица `[[embedded_source]]` авторского TOML-манифеста, продолжение существующей рукописной грамматики манифеста, а не новый JTD-контракт.
vibe-publish: MoveRef + CreateRef в crates/vibe-publish/src/github_release.rs — тела запросов GitHub REST git-refs, чужой формат.
vibe-publish: RenameAsset в crates/vibe-publish/src/github_release/asset.rs — тело запроса GitHub REST на переименование ассета, чужой формат.
vibe-publish: CreateGithubRelease / UpdateGithubRelease / GithubMakeLatest / GithubRelease / GithubReleaseAsset / GithubGitObject / GithubGitRef в crates/vibe-publish/src/github_release/model.rs — DTO GitHub Releases REST, чужой формат.
vibe-publish: SubmoduleProvenance в crates/vibe-publish/src/git_publish/submodules.rs — проекция в JSON-конверт `vibe registry publish --json`, не хранимый формат.
vibe-registry: CacheReceipt в crates/vibe-registry/src/embedded_source.rs — приватная квитанция `source.toml` в кэше пользователя, которую пишет и читает тот же крейт.
vibe-workspace: SubmoduleProvenance в crates/vibe-workspace/src/publish/staging.rs — проекция в JSON-конверт `vibe workspace publish --json`, не хранимый формат.
xtask: EmbeddedSourceRow в xtask/src/bridge.rs — сопровожденческий эмиттер готовой строки `[[embedded_source]]` для ручной вставки в vibe.toml, та же авторская грамматика манифеста.
xtask: CargoMessage + CargoTarget в xtask/src/dist/build.rs — декодер `cargo --message-format=json`, чужой формат.
```

Поднято в `wire-derive-baseline.json`: `vibe-cli` 42 → 44, `vibe-core` 33 → 34,
`vibe-publish` 3 → **7**, `vibe-registry` 7 → 8, `vibe-workspace` 3 → 4,
`xtask` 12 → 14.

### Долг: шаг остаётся красным по vibe-publish

`vibe-publish` измеряется как 8, а поднят только до 7: пятый новый файл —
класс (а), и по правилу пакета его в базовую линию не включаем. Адрес долга:

- **Тип(ы):** `BundleDistributionManifest`, `PlatformDistributionFragment`,
  `AggregateDistributionManifest`, `DistributionComponent`,
  `DistributionComponentName`, `DistributionAsset`,
  `DistributionSourceArchive`.
- **Файл:** `crates/vibe-publish/src/release_manifest.rs:59–150`.
- **Лечение:** описать форматы схемами JTD (рядом с `schemas/*.jtd.json`,
  жанр `registry_publish_report.jtd.json`) и выполнить `cargo xtask codegen`;
  после этого поднять `vibe-publish` до 8 не потребуется — счётчик упадёт до 7
  сам, потому что сгенерированный код живёт под `**/generated/**`.
- **Почему (а):** модульная докстрока файла сама называет их «Wire contracts
  shared by the four independent vibevm distribution builds … strict JSON
  contracts with deterministic serializers»; `DISTRIBUTION.json` и
  `DISTRIBUTIONS.json` публикуются как release-ассеты и читаются
  `distribution/install/install.sh:7` (`VIBEVM_DISTRIBUTIONS_ASSET`) и
  `install.ps1`, то есть другим языком; PROP-019 §2.12 `COLD-START-PATH`
  прямо описывает это как «each bounded script downloads `DISTRIBUTIONS.json`».
  Это ровно определение нашего wire из PROP-044 §2 закон 5.

Работа по этому долгу в периметр пакета не входит (пакет запрещает
`cargo xtask codegen`), поэтому шаг 3 self-check остаётся красным
**одной** строкой про `vibe-publish` вместо шести крейтов.

## 2. Сироты карты

Оба файла получили файловую метку `specmark::scope!` — так же, как соседние
тэгированные файлы тех же крейтов (`crates/vibe-core/src/manifest/package/*.rs`
используют ровно эту форму; `crates/vibe-publish/src/git_publish/process.rs`
тоже). Тело кода не изменено.

| элемент | файл:строка | якорь | почему этот якорь |
| --- | --- | --- | --- |
| `EmbeddedSourceKind` | `crates/vibe-core/src/manifest/package/embedded_source.rs:13` | `spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-021#source-abstraction` | `FORM-DEPENDENCY-DECLARED` перечисляет ровно эти поля: «a portable name, public credential-free HTTPS URL, full immutable commit, expected `sha256:` source-tree hash, optional ref hint, and upstream licence provenance» |
| `EmbeddedSourceAuth` | `…embedded_source.rs:23` | тот же | «public credential-free HTTPS URL» — v1-поза «public, credential-free», которую кодирует enum |
| `EmbeddedSourceDecl` | `…embedded_source.rs:31` | тот же | там же; поля объявления перечислены пофамильно |
| `SubmoduleProvenance` | `crates/vibe-publish/src/git_publish/submodules.rs:14` | `spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-023#classes` | `CLASS-SUBMODULE`: «Registry publication deliberately flattens a clean populated gitlink into ordinary package files, removes `.gitmodules`, and reports `submodule <path> vendored at <sha>`» — это и есть пара (path, commit) |
| `inspect` | `…submodules.rs:26` | тот же | функция собирает те самые gitlink'и до уплощения, чтобы отчёт был правдив |

Якорь `PROP-021#source-abstraction` уже используется соседями того же смысла
(`crates/vibe-registry/src/embedded_source.rs:9`,
`crates/vibe-cli/src/commands/show/source_path.rs:4`); `PROP-023#maintainer-model`
уже используется в `crates/vibe-check/src/checks/bridge_provenance.rs:10`.
Оба якоря резолвятся: `resolve gate — 0 unresolved host edge(s)`.

Рассмотренный и отклонённый вариант для `embedded_source.rs`:
`PROP-023#maintainer-model` (§2.4) описывает только разделение
`upstream_authors` / `upstream_license` — подмножество файла; файловая метка
должна покрывать весь файл, поэтому взят `PROP-021#source-abstraction`.
Элементов без якоря не осталось — кандидат-разделов для дописывания нет.

## 3. Отклонения

- **Дерево отличалось от объявленного в пакете.** Пакет называет HEAD
  `b1291b06`, фактический HEAD на старте — `8f956eeb`, на финише — `82620654`
  (центральная сессия коммитила параллельно, в том числе
  `vibevm/vibespecs/design/documentation-vision.xml`). `.rs`-файлы не
  затронуты, поэтому классификация и подсчёты не пересматривались.
- **Пакет описывает четыре красных крейта, фактически их шесть.**
  Дополнительно превышали базовую линию `vibe-cli` (44 против 42) и
  `vibe-core` (34 против 33). Оба обработаны по тому же правилу; периметр
  файлов пакет задаёт как «`wire-derive-baseline.json` … и два файла с
  сиротами», и правки ограничены ровно этим — поднятие ещё двух чисел
  остаётся внутри того же одного файла базовой линии.
- **`specmap.json` уже был изменён в рабочем дереве до начала работы**
  (4607 вставок / 1133 удаления) — регенерация, сделанная кем-то другим на
  промежуточном состоянии `documentation-vision.xml`, и на момент старта она
  уже была устаревшей (`units added: 442`). Правка через `cargo xtask specmap`
  перезаписала файл целиком, как и предписано периметром.
- **Первый вызов `cargo xtask specmap` упал** с `os error 1224`
  («file with a user-mapped section open») — `specmap.json` был открыт другим
  процессом. Повторный вызов прошёл; поведение кода не менялось.
- **Дефект пакета:** заголовочный абзац обещает «отчёт и находка — в
  `campaigns/docs-2026-09/findings/`», но задание не определяет ни спайка, ни
  имени файла находки (`A0.N-<slug>.md`), а раздел «Отчёт» требует все
  результаты именно в отчёте. Выбрано консервативное толкование: отдельный
  файл находки не создавался, всё содержательное — здесь.

Ничего не установлено глобально; секретов не открывал и не встречал;
`C:\Users\olegc\git\infra\**` не читал; git-команд, изменяющих состояние, не
выполнял.

## 4. Вывод самопроверки

### `cargo xtask specmap --check 2>&1 | tail -8`

```
specmap --check: clean (7930 spec units, 3413 tagged code items, 2966 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.
```

Код выхода 0. Пять сирот закрыты: `0 gated orphan(s)`.

### Шаг ratchet derive — `bash tools/self-check.sh` (падает на шаге 3/47)

До правки:

```
=== [3/47 00:08:39] wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json) ===
self-check: `vibe-cli` carries 44 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 42. …
self-check: `vibe-core` carries 34 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 33. …
self-check: `vibe-publish` carries 8 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 3. …
self-check: `vibe-registry` carries 8 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 7. …
self-check: `vibe-workspace` carries 4 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 3. …
self-check: `xtask` carries 14 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 12. …
self-check: `wire-derive ratchet (…)` failed (exit 1, step 3/47, 9s)
```

(строки рецепта, повторяющиеся дословно для каждого крейта, здесь сжаты
многоточием; полный вывод —
`<scratch>\vibe-docs-phase1\P1-O2\selfcheck-before.txt`)

После правки — целиком, дословно:

```
=== [1/47 00:19:36] the floor builds every live package workspace ===
self-check: the floor builds all 7 live package workspace(s) under vibevm/vibepacks/org.vibevm.ai-native/.
self-check: ✓ [1/47] the floor builds every live package workspace (1s)

=== [2/47 00:19:37] instruction files identical (CLAUDE.md = AGENTS.md = GEMINI.md) ===
self-check: ✓ [2/47] instruction files identical (CLAUDE.md = AGENTS.md = GEMINI.md) (0s)

=== [3/47 00:19:37] wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json) ===
self-check: `vibe-publish` carries 8 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 7. the rule — handwritten wire grows only through a
self-check: named decision, never silently (PROP-044 §2 law 5: a handwritten
self-check: parser or writer of our own format is the mechanism by which the
self-check: other four bans break unnoticed). fix: describe the format as a schema
self-check: and run `cargo xtask codegen`; if the new type is NOT our wire (a
self-check: config, a CLI-local struct, a foreign format), raise the `vibe-publish` count
self-check: in wire-derive-baseline.json in the same commit and say in the commit body what the type is.
self-check: `wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json)` failed (exit 1, step 3/47, 7s)
```

Код выхода скрипта 1 — ожидаемо, по единственной оставшейся причине (раздел
«Долг» выше).

### Прочие команды пункта 3 задания

```
$ cargo build -p vibe-core -p vibe-publish
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.31s

$ cargo clippy -p vibe-core -p vibe-publish --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.55s

$ cargo test -p vibe-core -p vibe-publish --quiet
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 188 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.46s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

$ cargo xtask specmap
  drift: units added: 442
  drift: edges added: 2
specmap: wrote C:\Users\olegc\git\v\vibevm-docs\specmap.json (7930 spec units, 3413 tagged code items, 2966 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.
```

### `cargo fmt --all -- --check; echo EXIT=$?`

```
EXIT=0
```

### `git status --short`

```
 M campaigns/docs-2026-09/JOURNAL.md
 M crates/vibe-core/src/manifest/package/embedded_source.rs
 M crates/vibe-publish/src/git_publish/submodules.rs
 M specmap.json
 M wire-derive-baseline.json
?? campaigns/docs-2026-09/PAGE-MAP.md
?? campaigns/docs-2026-09/findings/PACKET-P1-O2.md
?? vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/LICENSE.md
?? vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/README.md
?? vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibe.toml
?? vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/
```

Моё: `crates/vibe-core/src/manifest/package/embedded_source.rs`,
`crates/vibe-publish/src/git_publish/submodules.rs`,
`wire-derive-baseline.json`, `specmap.json`, плюс этот отчёт.
Чужое, не тронуто: `campaigns/docs-2026-09/JOURNAL.md`,
`campaigns/docs-2026-09/PAGE-MAP.md`,
`campaigns/docs-2026-09/findings/PACKET-P1-O2.md` (сам пакет),
`vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/**`.
`vibevm/vibespecs/design/documentation-vision.xml` соседний воркер писал
параллельно; на момент финиша он закоммичен (`82620654`) и в списке не виден.

## 5. Что не сделано и почему

- **JTD-схемы для дистрибутивных манифестов** — пакет прямо запрещает
  `cargo xtask codegen`. Долг адресован выше; из-за него шаг 3 self-check
  остаётся красным (одна строка, `vibe-publish` 8 против 7).
- **Шаги self-check после третьего не выполнялись.** Заголовок скрипта
  (строки 84–92) настаивает: красный прогон ничего не говорит о шагах
  после красного — они не запускались. `--keep-going` не использовался,
  потому что он потянул бы полный `cargo test --workspace` и все пакетные
  наборы, что выходит за периметр задания.
- **Расхождений с решениями вижена (`D-NN`) не обнаружено** — задание их не
  цитирует, а найденные факты ни одному не противоречат. `REVIEW:` нет.
- **Предупреждение для босса перед коммитом:** `specmap.json` зависит от
  всего дерева спек, а центральная сессия коммитит параллельно
  (`documentation-vision.xml`, пакет `vibevm-docs`). Если между этим отчётом и
  коммитом дерево спек изменится, `cargo xtask specmap` надо прогнать ещё раз
  непосредственно перед коммитом.
