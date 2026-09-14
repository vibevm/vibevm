# PACKET-POST-O2 — `vibe self` на бинарной установке: `update` двигает версию вперёд, `reinstall` перекачивает текущую

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией и ещё одним воркером: коммитишь **только** формой `git commit -m … -- <пути>`, новые
файлы — `git add -- <файл>` перед этим; никогда `git add -A`, никогда голый `git commit`. `git push` не делаешь.
Общий `target/` занят другим Rust-воркером — работай в приватном каталоге сборки: перед любым `cargo`
ставь в окружении процесса `CARGO_TARGET_DIR=<scratch>\post-o2-target` (путь скажет запускающий), и в конце
работы удали этот каталог целиком (холодная сборка `vibe-cli` — десятки гигабайт; места хватает).
Правила, которые тебя связывают: R-12 — не читать `~/.vibe/*.token`; никакие процессы, которых ты не
запускал, не останавливать; `bash tools/self-check.sh` не гонять; в `~/.vibe/opt` владельца не писать —
любые живые пробы только с `VIBEVM_INSTALL_ROOT=<scratch>\post-o2-root` (изолированный корень; в конце
удалить его и вычистить из постоянного пользовательского PATH запись `…\post-o2-root\opt\bin`, которую
бутстрап туда допишет — проверь `[Environment]::GetEnvironmentVariable('Path','User')`).

## Решение владельца (2026-09-14, дословно по смыслу)

«Старую бинарную версию обновляет `reinstall`, а `update` продвигает версии вперёд». Это меняет норму
`##CMD-UPDATE` в PROP-019 («an installed binary refreshes its own version number and never jumps to
another») — по слову владельца, факт переписывается по новому смыслу, не обходится.

## Читать сначала, ровно эти файлы

1. `vibevm/vibespecs/common/PROP-019-version-manager.xml` — `##CMD-INSTALL`, `##CMD-UPDATE` (строки ~88–96),
   `##SEL-LATEST`, `##SEL-STABLE` (~144–145), §2.2 «surface».
2. `crates/vibe-cli/src/cli/vvm.rs` — clap-определения подкоманд `self` (`update`, `install`, их флаги,
   тексты справки).
3. `crates/vibe-cli/src/commands/vvm/mod.rs` — диспетчер подкоманд и `run_install_cmd` (~174–260): ветвление
   для бинарного исполнения (`Origin::Binary`): явный тег → `bundle::install_binary_version`, `latest` →
   `VvmError::BinaryFetchUnavailable`, остальное — сборка из исходников.
4. `crates/vibe-cli/src/commands/vvm/bundle.rs` — `run_update`/`update_binary`, `install_binary_version`,
   `install_selected`, `GITHUB_RELEASE_ROOT`, `read_aggregate`, `select_platform`, трейт `Downloader`
   (~378) и `HttpDownloader`, `cache_busted`.
5. `crates/vibe-cli/src/commands/vvm/bundle/archive.rs` — `install_bundle`, смысл `force`/`reused`.
6. `crates/vibe-cli/src/commands/vvm/bundle/tests.rs` (+ `network_tests.rs`) — подмена `Downloader`
   (`LocalDownloader`, `ReleaseDownloader`) — образец для новых тестов.
7. `crates/vibe-cli/src/commands/vvm/error.rs`, `model.rs` (`Selector`).
8. `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/model/versions.xml` — что руководство
   обещает сегодня (центральная сессия перепишет прозу после посадки; тебе — только знать).

## Что есть сегодня (измерено центральной сессией на релизном бинарнике 1.0.0, изолированный корень)

`self update` → перекачал манифест и бандл своей версии, `reused tag:1.0.0#1`; `self update --force` →
`installed tag:1.0.0#2`; `self install 1.0.0 --force` → `#3`. Перейти на новый релиз бинарная установка
может только зная номер (`self install 1.1.0`); `self install latest` отказывает
(`BinaryFetchUnavailable`); `self install stable` на бинарном исполнении уходит в сборку из исходников
(`choose_mirror` → клон → `cargo`), хотя `##SEL-STABLE` обещает «новейший релиз». Узнать о новом релизе
нельзя ни одной командой.

## Сделать

1. **`vibe self update` на бинарном исполнении двигает версию вперёд.** Манифест новейшего релиза —
   `https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json` (cache-busted, как в
   `update_binary`); из него — `version`. Новее текущей (сравнение `semver`) → установка этой версии
   существующим проверенным путём (`select_platform` → `install_selected`, тот же `--force`), активация,
   строка вида `updating 1.0.0 → 1.1.0` в стиле соседних сообщений. Равна текущей → `reused`, строка вида
   `newest release is 1.0.0 — already installed`; с `--force` — свежий экземпляр той же версии (прежнее
   поведение `update --force`, сохраняется как есть). Новейшая **старше** текущей (откат релиза на
   стороне GitHub) — не переходить, сказать об этом одной строкой. Источниковое исполнение (`Origin::External`)
   — без изменений: rebuild `latest`.
2. **Новая подкоманда `vibe self reinstall`** — перекачать и переустановить текущую версию: бинарное
   исполнение → манифест **своей** версии (`releases/download/v<current>/DISTRIBUTIONS.json`) и
   `install_selected(force = true)` — свежий экземпляр всегда, без флага `--force` (принять `--force` как
   допустимый и бессмысленный? нет: не объявлять, чтобы не плодить флаги); источниковое исполнение →
   ровно то, что сегодня делает `update --force` для источника (rebuild с `force`), с теми же `--profile` /
   `--release`. Флаги `--json`, `--quiet`, `--invoked-by`, `--agent-mode`, `--unattended`, `--offline` — как у
   `update`. JSON-конверт — той же формы, что у `update` (новых полей не заводить, схемы не трогать).
3. **`vibe self install stable` на бинарном исполнении** — тот же путь «новейший релиз», что у `update`
   (одна функция, два входа). `self install latest` на бинарном исполнении — по-прежнему отказ, но
   текст `BinaryFetchUnavailable` ведёт к делу: «use `vibe self update` for the newest release,
   `vibe self install X.Y.Z` for a specific one, `vibe self reinstall` to refresh the current one, or
   `--mirror` for a source build».
4. **`--offline`**: `update`/`stable` на бинарном исполнении — честная ошибка «the newest release cannot be
   learned offline»; `reinstall` бинарной версии — честная ошибка (кэша бандлов нет). Посмотри, как
   `--offline` доезжает до `vvm`; если не доезжает — скажи в отчёте и не изобретай канал.
5. Тексты справки: `update` («Move a binary installation to the newest release; a source execution rebuilds
   `latest`» — в этом духе, коротко, английский) и `reinstall` — новые; остальные не трогать. Изменение
   справки неизбежно: назови его в отчёте отдельной строкой — центральная сессия перезапишет `derived`
   в руководстве.
6. Норма: в PROP-019 переписать факт `##CMD-UPDATE` по новому смыслу и добавить факт `##CMD-REINSTALL`
   рядом (статусы `impl/done`, стиль соседей, `action="continue" actionstage="doc" audience="user"` как у
   `CMD-INSTALL`); строку `##SEL-STABLE` не менять. `@scope` у нового кода —
   `spec://org.vibevm.core/vibevm/common/PROP-019#surface` (как у соседей).
7. Тесты через подмену `Downloader` (без сети): `update` на бинарном исполнении при более новом релизе →
   скачан манифест `latest`, установлена и активирована новая версия; при равной → `reused`; при равной с
   `--force` → новый экземпляр; при более старой → отказ перейти; ошибка загрузки манифеста → ошибка
   команды с понятным текстом; `reinstall` бинарной версии → свежий экземпляр той же версии; `stable`
   → тот же путь, что `update`; текст `BinaryFetchUnavailable` содержит `update` и `reinstall`;
   существующие тесты `vvm` зелёные.
8. Живая проба в изолированном корне (см. шапку): бутстрап 1.0.0 из релиза (команда — как в
   `distribution/install/install.ps1`: `vibe-bootstrap … self bootstrap --manifest … --version 1.0.0
   --release-base https://github.com/vibevm/vibevm/releases/download/v1.0.0`; бутстрап и манифест — с
   `https://github.com/vibevm/vibevm/releases/download/v1.0.0/`), затем **собранным тобой** `vibe.exe`
   (в релизе твоего кода нет): `VIBEVM_INSTALL_ROOT=… <твой vibe.exe> self update` (ожидание: новейший
   релиз 1.0.0 = текущий → `reused` и строка «already installed»), `self reinstall` (ожидание: свежий
   экземпляр `#2`), `self install stable` — вывод дословно в отчёт. Если собранный бинарник вне
   управляемого корня не считается «бинарным исполнением» (`running_record`/`provenance`), скажи, как
   проверил ветку иначе и что осталось непроверенным вживую.

## Периметр файлов

`crates/vibe-cli/src/cli/vvm.rs`, `crates/vibe-cli/src/commands/vvm/**`,
`vibevm/vibespecs/common/PROP-019-version-manager.xml`. `specmap.json`, `schemas/**`, руководство — не
трогать. Ничего вне периметра.

## Самопроверка (обязательно, вывод в отчёт)

```
cargo fmt --all -- --check
cargo clippy -p vibe-cli --all-targets -- -D warnings
cargo test -p vibe-cli vvm
cargo test -p vibe-cli cli_help   # если такой фильтр что-то цепляет: справка `self` пинится тестами
cargo xtask specmap --check
cargo build -p vibe-cli   # затем живая проба п. 8
```

Сетевые тесты `vibe-cli` (`redbook_polygon_*` и подобные) в фильтр `vvm` не входят; если фильтр цепляет
что-то сетевое и долгое — `--skip <имя>` с причиной в отчёте.

## Приёмка боссом

Диф читается как PR; на бинарном исполнении `update` ставит новейший релиз тем же проверенным путём, что
и явный номер, `reinstall` перекачивает текущую, `stable` = `update`; источниковые пути не изменились;
приватный `target` и изолированный корень удалены, PATH пользователя чист.

## Коммит

Один коммит: `feat(self): update moves a binary install forward, reinstall refreshes the current one` с
телом «почему» (владелец: перекачка текущей версии и переход на новый релиз — две команды с двумя
именами; `stable` обещан нормой как новейший релиз, а уходил в сборку из исходников; норма `CMD-UPDATE`
переписана по его слову). Авторство человеческое, без трейлеров.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-POST-O2.md` (не коммитить): хэш, что сделано, вывод гейтов
и живой пробы дословно, аномалии, что не сделано и почему, изменённые тексты справки (для `derived`).
