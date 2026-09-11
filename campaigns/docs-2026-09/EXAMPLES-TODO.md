# Примеры к снятию — очередь для воркера (P.3 → фикстуры) {#root}

<status stage="spec" state="work" comment="ведётся с 2026-09-12; каждая строка — example на странице, чей expect снимается с target/debug/vibe.exe в песочнице фикстуры; после снятия строка получает «снято» и хэш коммита, в котором expect вставлен; вторая редакция фикстур 2026-09-12 после PP-C2 (локальный реестр вместо сети, слоты вместо packages/)"/>

Правило (PLAN P.3, R-07, PROP-057 `INV-EXAMPLES-RUN`): `expect` — только
реально снятый вывод отладочного бинарника. Дешёвая модель снимает по этому
списку (пакет `findings/PACKET-PP-C2.md`, повтор — `PACKET-PP-C2b.md`) и
кладёт вывод в `findings/PP-C2-expects/`; центральная сессия читает и
вставляет. Нормализация при снятии — правила A0.12 §3 в этом порядке:
`paths` (`<TMP>`, `<HOME>`, `<REPO>`), `slashes`, `trailing_space`, имя
исполняемого файла (`vibe.exe` → `vibe`). Версия продукта **не** подменяется
при снятии: подмену `vibe 1.0.0` → `vibe <VERSION>` решает раннер фазы 2
(A2.9), а на странице стоит настоящая строка бинарника той сборки, с
которой снято.

## Три правила песочницы {#sandbox-rules}

1. **Окружение изоляции не меняет поведение** (A0.12, решение 2). Снятие
   выставляет только `VIBE_SETTINGS`, `VIBE_REGISTRY_CACHE`,
   `VIBEVM_SEARCH_CACHE_DIR` и `NO_COLOR`; поведенческие переменные
   (`VIBE_OFFLINE`, `VIBE_UNATTENDED`, `VIBE_INVOKED_BY`,
   `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME`) удаляются из
   унаследованного окружения. `--offline` и `--assume-yes` стоят в самой
   команде примера или не стоят нигде.
2. **cwd — каталог `work/` песочницы**, никогда корень хоста (P0-O4,
   отклонение 1: запуск из корня оставляет `vibevm/vibedeps/.gitignore`).
   Значения переменных — в нативном написании (`C:\…`), см. A0.12 §1.
   Корень песочницы — короткий путь (`%TEMP%\vdocs\`): в глубоком каталоге
   `git clone` реестра падает с `fatal: '$GIT_DIR' too big` (PP-C2 §5b).
3. **Реестр — локальный.** `home/registry.toml` песочницы объявляет один
   реестр `local` с адресом `file:///<корень хоста>/vibevm/vibepacks`:
   in-tree реестр хоста, только чтение. Проект, созданный `vibe init`,
   резолвит через него без сети; `vibe registry list --path hello-vibe`
   честно печатает «No [[registry]] entries in vibe.toml» — реестр
   машинный, не проектный. Прогрев store через `cache add --path <хост>`
   больше не нужен: первая установка кладёт пакет в store сама. Причина
   смены (PP-C2 §5a): оффлайн-установка после прогрева не резолвит `latest`
   без клона реестра (B-131), а сетевой клон в песочнице рвётся на длине
   пути (B-132).
4. **Автор фикстур нейтрален.** До первого `vibe init` в `home/config.toml`
   записывается `[init]\nlast_author = "vibevm docs fixtures"`, иначе `vibe
   init` берёт имя оператора из `git config` (P0-O4).

## Фикстуры {#fixtures}

Каждая фикстура — состояние песочницы (`home/` + `work/`) перед командой;
строится из предыдущей ровно названными командами, cwd = `work/`.
`vibe init package <координата>` без пути добавляет слот
`vibevm/vibepacks/<группа>/<имя>/v0.1.0/` в проект cwd (PP-C2 §5d, проба
2026-09-12); флаг `--kind` сейчас не действует (B-134) — вид правится в
манифесте слота.

| Фикстура | Из чего | Как готовится (cwd = `work/`) |
|---|---|---|
| `none` | — | пустые `home/` и `work/`; `home/config.toml` с автором фикстур; `home/registry.toml` с реестром `local` (правило 3) |
| `empty` | `none` | то же (store пуст; имя оставлено ради очереди: команды `cache add` на этой фикстуре наполняют store из локального реестра) |
| `hello-vibe-empty` | `empty` | `vibe init hello-vibe` |
| `hello-vibe` | `hello-vibe-empty` | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes` |
| `hello-vibe-relay` | `hello-vibe` | `vibe agentic explain --path hello-vibe` — в `hello-vibe/.vibe/agentic/command.md` припаркована инструкция |
| `hello-vibe-removed` | `hello-vibe` | `vibe uninstall org.vibevm.world/wal --path hello-vibe --assume-yes` |
| `hello-vibe-cwd` | `hello-vibe` | то же дерево; **cwd команды — `work/hello-vibe`** (для команд без `--path`: `vibe bin`, `vibe tools`) |
| `hello-cargo` | `hello-vibe` | в `work/hello-vibe`: `cargo init --name hello --vcs none` (Cargo.toml и `src/main.rs`), затем `cargo generate-lockfile --offline` — проект с настоящим инструментом сборки для health-проверок и артефактов |
| `hello-deploy` | `hello-cargo` | копия дерева `hello-vibe` под именем `work/hello-deploy`; в её `vibe.toml` дописаны таблицы со страницы `lifecycle/build-package-deploy`, шаг 1 (артефакт `hello`, цель `local` с `deploy:vibe-bin`, профиль `local`) |
| `hello-vibe-scrape` | `hello-cargo` | `vibe scrape contract init --path hello-vibe`; в `hello-vibe/vibevm/scrape/contract.toml` оба `modified = "refuse"` заменены на `modified = "delete"`; `vibe scrape contract check --path hello-vibe` обязан быть зелёным, иначе — в отчёт |
| `project` | `empty` | `vibe init` в `work/` (проект в cwd; `[project] name = "work"`) |
| `flow-slot` | `project` | `vibe init package org.acme/review-notes` — слот `vibevm/vibepacks/org.acme/review-notes/v0.1.0/` как есть (`kind = "tool"`, страница объясняет правку вида) |
| `package-spec` | `flow-slot` | в слот положен `vibevm/vibespecs/NOTES-FLOW.md` из одного заголовка `# Notes flow {#root}` и одного абзаца `@fact:ONE-NOTE One note per review. @status:spec/done`; **cwd команды — `work/vibevm/vibepacks/org.acme/review-notes/v0.1.0`** |
| `workspace-root` | `project` | в `work/vibe.toml` дописана таблица `[workspace]` с `members = ["packages/*"]` — текст со страницы `howto/set-up-a-workspace`, шаг 1 |
| `workspace` | `workspace-root` | руками написаны `packages/notes-flow/vibe.toml` и `packages/review-notes/vibe.toml`: таблица `[package]` с теми же полями, что пишет `vibe init package` (`group`, `name`, `kind = "flow"`, `version = "0.1.0"`, `epoch = 1`, `authors`, `license = "UPL-1.0"`, `description`, `format = "normal"`), без `[boot_snippet]`; установка не выполнялась |
| `package-notes` | `project` | `vibe init package org.acme/notes`; затем `vibe registry add local <абсолютный путь к пустому каталогу work/registry> --path . --position primary` — примеры публикации на ней ждут B-133 |
| `docs-store` | `empty` | store с пакетом `org.vibevm.core/vibevm-docs` — **невозможно до фазы 2** (вид `doc` неизвестен бинарнику) |

Фикстуры фазы 2 (A2.9) наследуют эту таблицу как первый корпус раннера:
каждая строка становится `examples/<фикстура>/example.toml` пакета
документации с объявленными правилами нормализации (A0.12 §5); локальный
реестр становится копией нужных пакетов внутри фикстуры (X-025).

## Что не снимается сейчас {#not-now}

| Страница | id | Почему | Когда |
|---|---|---|---|
| `start/install-vibe` | `windows-install` | вывод установщика снимается с дистрибутива релиза, не с отладочной сборки | фаза 5, публикация |
| `howto/read-documentation-locally` | `cache-add-docs`, `doc-serve` | вид `doc` и `vibe doc` появляются в фазе 2 | A2.1, A2.20 |
| `howto/publish-a-package` | `publish-dry-run`, `publish` | `--dry-run` требует publish-токен и режет букву диска как хост (B-133) | после починки B-133 |

## Очередь {#queue}

Состояния: `ждёт` → `снято <хэш>` (вставлено на страницу) → `расходится`
(вывод снят, но страница ждёт правки текста) → `не сейчас` (таблица выше).

| Страница | id | Команда | Фикстура | Состояние |
|---|---|---|---|---|
| `agent/ask-your-agent` | `agentic-explain` | `vibe agentic explain --path hello-vibe` | `hello-vibe` | ждёт |
| `agent/ask-your-agent` | `command` | `vibe command --path hello-vibe` | `hello-vibe-relay` | ждёт |
| `agent/give-your-agent-the-skill` | `mcp-status` | `vibe mcp status --path hello-vibe` | `hello-vibe` | ждёт |
| `agent/give-your-agent-the-skill` | `mcp-install` | `vibe mcp install --auto --yes --dry-run --path hello-vibe` | `hello-vibe` | ждёт |
| `agent/give-your-agent-the-skill` | `skill-list` | `vibe skill list --path hello-vibe` | `hello-vibe` | ждёт |
| `architecture/traceability` | `explain` | `vibe explain "spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET" --path hello-vibe` | `hello-vibe` | ждёт |
| `architecture/traceability` | `select` | `vibe select --where "uri:spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET depth:1" --path hello-vibe` | `hello-vibe` | ждёт |
| `authoring/ship-tools-and-mcp-servers` | `bin-list` | `vibe bin list` | `hello-vibe-cwd` | ждёт |
| `authoring/specs-agents-can-cite` | `explain` | `vibe explain "spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET" --path hello-vibe` | `hello-vibe` | ждёт |
| `authoring/specs-agents-can-cite` | `convert` | `vibe refactor convert-source --from md --to xml --dry-run vibevm/vibespecs` | `package-spec` | ждёт |
| `authoring/write-a-flow` | `init-package` | `vibe init package org.acme/review-notes` | `project` | ждёт |
| `authoring/write-a-flow` | `manifest` | `cat vibevm/vibepacks/org.acme/review-notes/v0.1.0/vibe.toml` | `flow-slot` | ждёт |
| `authoring/write-a-flow` | `check` | `vibe check --path vibevm/vibepacks/org.acme/review-notes/v0.1.0` | `flow-slot` | ждёт |
| `authoring/write-a-lang-package` | `init-lang` | `vibe init package org.acme/sql-style` | `project` | ждёт |
| `howto/install-a-package` | `install` | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe-empty` | ждёт |
| `howto/install-a-package` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/install-a-package` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/publish-a-package` | `publish-dry-run` | `vibe registry publish vibevm/vibepacks/org.acme/notes/v0.1.0 --dry-run` | `package-notes` | не сейчас (B-133) |
| `howto/publish-a-package` | `publish` | `vibe registry publish vibevm/vibepacks/org.acme/notes/v0.1.0 --registry local --dry-run` | `package-notes` | не сейчас (B-133) |
| `howto/read-documentation-locally` | `cache-add-docs` | `vibe cache add org.vibevm.core/vibevm-docs` | `empty` | не сейчас (A2.1) |
| `howto/read-documentation-locally` | `doc-serve` | `vibe doc serve --help` | `docs-store` | не сейчас (A2.20) |
| `howto/remove-a-package` | `uninstall` | `vibe uninstall org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe` | ждёт |
| `howto/remove-a-package` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe-removed` | ждёт |
| `howto/set-up-a-workspace` | `workspace-table` | `cat vibe.toml` | `workspace-root` | ждёт |
| `howto/set-up-a-workspace` | `member-manifest` | `cat packages/notes-flow/vibe.toml` | `workspace` | ждёт |
| `howto/set-up-a-workspace` | `install` | `vibe install --assume-yes` | `workspace` | ждёт |
| `howto/update-packages` | `outdated` | `vibe outdated --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/update-packages` | `update` | `vibe update org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe` | ждёт |
| `howto/use-a-private-registry` | `registry-add` | `vibe registry add acme file:///tmp/acme-specs --path hello-vibe --position primary` | `hello-vibe` | ждёт |
| `howto/use-a-private-registry` | `registry-test` | `vibe registry test --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/work-offline` | `cache-add` | `vibe cache add org.vibevm.world/wal` | `empty` | ждёт |
| `howto/work-offline` | `cache-list` | `vibe cache list` | `hello-vibe` | ждёт |
| `howto/work-offline` | `cache-check` | `vibe cache check` | `hello-vibe` | ждёт |
| `howto/work-offline` | `install-offline` | `vibe install org.vibevm.world/wal --path hello-vibe --offline --assume-yes` | `hello-vibe-empty` | ждёт |
| `lifecycle/build-package-deploy` | `package` | `vibe package --path hello-deploy --assume-yes` | `hello-deploy` | ждёт |
| `lifecycle/build-package-deploy` | `deploy-plan` | `vibe deploy --plan --profile local --path hello-deploy` | `hello-deploy` | ждёт |
| `lifecycle/build-package-deploy` | `deployments` | `vibe deployments` | `hello-vibe` | ждёт |
| `lifecycle/extensions-and-providers` | `extensions` | `vibe extensions --path hello-vibe` | `hello-vibe` | ждёт |
| `lifecycle/extensions-and-providers` | `tools` | `vibe tools` | `hello-vibe-cwd` | ждёт |
| `lifecycle/phases` | `deploy-plan` | `vibe deploy --plan --profile local --path hello-deploy` | `hello-deploy` | ждёт |
| `lifecycle/scrape` | `scrape-contract` | `vibe scrape contract init --path hello-vibe` | `hello-cargo` | ждёт |
| `lifecycle/scrape` | `scrape-plan` | `vibe scrape --plan --path hello-vibe` | `hello-vibe-scrape` | ждёт |
| `lifecycle/scrape` | `scrape-output` | `vibe scrape --output hello-clean --path hello-vibe` | `hello-vibe-scrape` | ждёт |
| `model/boot-lane` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `model/lock-and-store` | `cache-path` | `vibe cache path` | `hello-vibe` | ждёт |
| `model/packages-and-kinds` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `model/registries` | `registry-list` | `vibe registry list --path hello-vibe` | `hello-vibe` | ждёт |
| `model/two-trees` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `model/versions` | `outdated` | `vibe outdated --path hello-vibe` | `hello-vibe` | ждёт |
| `reference/machine-formats` | `list-json` | `vibe list --json --path hello-vibe` | `hello-vibe` | ждёт |
| `reference/settings-and-environment` | `vars` | `vibe vars` | `hello-vibe` | ждёт |
| `start/first-project` | `init` | `vibe init hello-vibe` | `empty` | ждёт |
| `start/first-project` | `install` | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe-empty` | ждёт |
| `start/first-project` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `start/first-project` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `start/first-project` | `check` | `vibe check --path hello-vibe` | `hello-vibe` | ждёт |
| `start/install-vibe` | `windows-install` | `powershell -ExecutionPolicy Bypass -File .\install.ps1` | `none` | не сейчас (фаза 5) |
| `start/install-vibe` | `version` | `vibe --version` | `hello-vibe` | вставлено вручную, подтвердить снятием |
| `start/what-a-project-contains` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `start/what-vibevm-is` | `version` | `vibe --version` | `hello-vibe` | вставлено вручную, подтвердить снятием |
