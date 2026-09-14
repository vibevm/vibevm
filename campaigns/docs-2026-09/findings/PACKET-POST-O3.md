# PACKET-POST-O3 — `--offline` доходит до `vibe self` (B-160; Rust)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией и web-воркерами (они в `vibevm/vibepacks/org.vibevm.doc/web/**`; ты в `crates/vibe-cli/**`
и одной спеке). Коммитишь **только** формой `git commit -m … -- <пути>`, новые файлы — `git add -- <файл>`;
никогда `git add -A`, никогда голый `git commit`, никаких трейлеров `Co-Authored-By` и никаких упоминаний
модели или агента в сообщениях коммитов — авторство репозитория человеческое (PROP-000 `#commits`).
`git push` не делаешь. `specmap.json` не регенерируешь. Никакие процессы, которых ты не запускал, не
останавливать. Общий `target/` свободен; `CARGO_TARGET_DIR` не переопределяй (два теста `tests/vvm.rs`
красны вне дерева — B-157).

## Зачем

B-160 (`BACKLOG.md`): корневой `--offline` разбирается и виден в `vibe self … --help`, но `main.rs` не передаёт
его в `VvmEnv`, и `vibe --offline self update` идёт в сеть и успешно обновляется (измерено POST-O2 на
изолированном корне). Флаг, который парсится, но не доходит до домена, — ложь пользователю, который сеть
запретил.

## Читать сначала, ровно эти файлы

1. `crates/vibe-cli/src/main.rs` — ветка `Command::Vvm`, где собирается `VvmEnv`; и то место, где остальные
   команды разрешают офлайн-позицию (флаг > `VIBE_OFFLINE` > `[net] offline` в `~/.vibe/config.toml`,
   PROP-010 `OFFLINE-LAYERING`) — найди функцию и переиспользуй её, не пиши вторую.
2. `crates/vibe-cli/src/commands/vvm/mod.rs` (`VvmEnv`, правило «домен vvm не читает окружение процесса»),
   `commands/vvm/bundle.rs` (`RemoteContext`, `fetch_aggregate`, `install_selected`, `move_to_newest_release`,
   `install_release_version`, `run_reinstall_cmd`, `rebuild_latest`), `commands/vvm/error.rs`,
   `commands/vvm/bundle/network_tests.rs`, `bundle/release_tests.rs`, `commands/vvm/tests.rs`,
   `crates/vibe-cli/src/output.rs` (что несёт `Context`).
3. `vibevm/vibespecs/common/PROP-019-version-manager.xml` §2.2 (`req r4`, `CMD-UPDATE`, `CMD-REINSTALL`,
   `CMD-INSTALL`) и `vibevm/vibespecs/modules/vibe-registry/PROP-010-*.xml` (факты `OFFLINE-*`: как сформулирован
   офлайн-отказ у реестра — тем же тоном).
4. `findings/WORKER-REPORT-POST-O2.md`, раздел «`--offline`» — измеренное поведение и три места вызова.

## Сделать — два атомарных коммита

**Q. Позиция едет в домен.** `VvmEnv` получает поле `offline: bool`, которое корень композиции заполняет
разрешённой позицией процесса (тем же кодом, что у реестровых команд); домен по-прежнему не читает
окружение. Под `offline = true` ни один глагол `self` не делает ни одного сетевого запроса: путь релиза
(`update`, `install stable`, `install X.Y.Z`, `reinstall`, `bootstrap`) отказывает **до** первого запроса
ошибкой `VvmError`, которая называет глагол и адрес, который ему понадобился бы, и цитирует правило;
путь исходников (`update`/`reinstall` на source-экземпляре) не делает `git fetch`/`clone` и не тянет
крейты — если пересборка возможна из локального состояния (чекаут на месте, `cargo build --offline`),
она идёт, иначе — тот же честный отказ. Переиспользуемый экземпляр (`installed_from_published_bundle`)
под офлайном не спасает: манифест новейшего выпуска — сетевой, отказ до него. Тесты: фальшивый
`Downloader`, который паникует при вызове, под `offline = true` для каждого глагола пути релиза; тест
корня композиции, что флаг доходит (по образцу существующих тестов `main`/`cli`, если есть); `--json`
отказа — обычный конверт ошибки.
Коммит: `fix(self): honour the offline posture on every self verb`.

**R. Норма.** Факт в PROP-019 рядом с `CMD-UPDATE`, id `CMD-OFFLINE`:

> Every `self` verb honours the offline posture the process resolved — `--offline`, `VIBE_OFFLINE` or
> `[net] offline`, layered as PROP-010 `##OFFLINE-LAYERING` says. Under it a release-lane verb is refused
> before its first request, naming the verb and the address it would have needed; a source-lane rebuild
> runs without fetching when the checkout and the crates are already local, and is refused the same way
> otherwise. The domain never reads the environment for this: the composition root hands the resolved
> posture in.

Если факт ложится внутрь секции `#surface`, её содержимое меняется не редакционно — подними `req r4` →
`r5` и в том же коммите переведи единственное ребро `#[verifies("…PROP-019#surface", r = 4)]`
(`commands/vvm/tests.rs`) на `r = 5`, перечитав тест против нового текста. Статус факта — как у соседей
(`impl/done`, `action="continue" actionstage="doc" audience="user"`; если `vibe doc check --coverage` над
руководством потребует цитаты — сними `audience` и скажи в отчёте: прозу пишет центральная сессия).
Коммит: `docs(spec): say that self obeys the offline posture`.

## Периметр файлов

`crates/vibe-cli/src/main.rs`, `crates/vibe-cli/src/commands/vvm/**`, `crates/vibe-cli/src/output.rs` (только
если позиция логично живёт в `Context`), `vibevm/vibespecs/common/PROP-019-version-manager.xml`. Ничего в
`crates/vibe-registry/**` (офлайн реестра уже есть — переиспользуй), в web-пакете, в руководстве, в
`specmap.json`.

## Самопроверка (обязательно, вывод в отчёт)

```
cargo fmt --all -- --check
cargo clippy -p vibe-cli --all-targets -- -D warnings
cargo test -p vibe-cli vvm
cargo test -p vibe-cli offline
cargo build -p vibe-cli
target/debug/vibe.exe --offline self update          # на этой машине: честный отказ без сети, exit != 0, без изменения инвентаря
target/debug/vibe.exe --offline self ls              # читает локально, работает
cargo xtask specmap --check                          # красный по новому факту / бампу — в отчёт
```

Живой прогон `--offline self update` — на изолированном корне (`VIBEVM_INSTALL_ROOT=<scratch>`), как делал
POST-O2 (импорт собранного бинарника `self import --tag 1.0.0 --use`), чтобы не трогать инвентарь машины;
удалить корень после; ничего в user `Path` не оставлять (POST-O2 описал, как восстанавливал).

## Приёмка боссом

Два коммита; `vibe --offline self update` на бинарном экземпляре отказывает до сети с адресом и правилом;
`self ls`/`current`/`which` под офлайном работают; тесты с паникующим загрузчиком зелёные; факт и, если
нужно, бамп ревизии с переведённым ребром.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-POST-O3.md` (не коммитить): хэши и subject'ы, где живёт
разрешение офлайн-позиции и как оно переиспользовано, вывод гейтов дословно, живой прогон дословно,
аномалии, что не сделано и почему.
