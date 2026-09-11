# Примеры к снятию — очередь для воркера (P.3 → фикстуры) {#root}

<status stage="spec" state="work" comment="ведётся с 2026-09-12; каждая строка — example на странице, чей expect снимается с target/debug/vibe.exe в песочнице фикстуры; после снятия строка получает «снято» и хэш коммита, в котором expect вставлен"/>

Правило (PLAN P.3, R-07, PROP-057 `INV-EXAMPLES-RUN`): `expect` — только
реально снятый вывод отладочного бинарника. Дешёвая модель снимает по этому
списку (пакет `findings/PACKET-PP-C2.md`) и кладёт вывод в
`findings/PP-C2-expects/`; центральная сессия читает и вставляет.
Нормализация при снятии — правила A0.12 §3 в этом порядке: `paths`
(`<TMP>`, `<HOME>`, `<REPO>`), `slashes`, `trailing_space`, имя исполняемого
файла (`vibe.exe` → `vibe`). Версия продукта **не** подменяется при снятии:
подмену `vibe 1.0.0` → `vibe <VERSION>` решает раннер фазы 2 (A2.9), а на
странице стоит настоящая строка бинарника той сборки, с которой снято.

## Три правила песочницы {#sandbox-rules}

1. **Окружение изоляции не меняет поведение** (A0.12, решение 2). Снятие
   выставляет только `VIBE_SETTINGS`, `VIBE_REGISTRY_CACHE`,
   `VIBEVM_SEARCH_CACHE_DIR` и `NO_COLOR`; поведенческие переменные
   (`VIBE_OFFLINE`, `VIBE_UNATTENDED`, `VIBE_INVOKED_BY`,
   `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME`) удаляются из
   унаследованного окружения. `--offline` и `--assume-yes` стоят в самой
   команде примера или не стоят нигде. Команда установки без `--offline`
   идёт в сеть к реестру, который `vibe init` записал в проект, — и это
   честно: читатель видит то же.
2. **cwd — каталог `work/` песочницы**, никогда корень хоста (P0-O4,
   отклонение 1: запуск из корня оставляет `vibevm/vibedeps/.gitignore`).
   Значения переменных — в нативном написании (`C:\…`), см. A0.12 §1.
3. **Автор фикстур нейтрален.** Перед первым `vibe init` в `home/config.toml`
   песочницы записывается `[init]\nlast_author = "vibevm docs fixtures"`,
   иначе `vibe init` берёт имя оператора из `git config` (P0-O4).

## Фикстуры {#fixtures}

Каждая фикстура — состояние песочницы (`home/` + `work/`) перед командой;
строится из предыдущей ровно названными командами, cwd = `work/`.

| Фикстура | Из чего | Как готовится (cwd = `work/`) |
|---|---|---|
| `none` | — | пустые `home/` и `work/` |
| `empty` | `none` | store прогрет пакетом `org.vibevm.world/wal` и его замыканием из in-tree реестра хоста: `vibe cache add --offline org.vibevm.world/wal --path <корень хоста>` (A0.17: project-local реестр `vibevm/vibepacks` первым; флага `--registry` у `cache add` нет). Ничего не материализуется; после команды `git status --short` хоста обязан быть пустым по этому пути |
| `hello-vibe-empty` | `empty` | `vibe init hello-vibe` (реестр по умолчанию — тот, что пишет `vibe init`) |
| `hello-vibe` | `hello-vibe-empty` | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes --offline` |
| `hello-vibe-relay` | `hello-vibe` | `vibe agentic explain --path hello-vibe` — в `hello-vibe/.vibe/agentic/command.md` припаркована инструкция |
| `hello-vibe-removed` | `hello-vibe` | `vibe uninstall org.vibevm.world/wal --path hello-vibe --assume-yes` |
| `workspace-root` | `empty` | `vibe init` в `work/` (проект в cwd), затем в `vibe.toml` добавлена таблица `[workspace]` с `members = ["packages/*"]` — текст таблицы взять со страницы `howto/set-up-a-workspace`, шаг 1 |
| `workspace` | `workspace-root` | `vibe init package org.acme/notes-flow packages/notes-flow`, затем `vibe init package org.acme/review-notes packages/review-notes`; установка не выполнялась |
| `package-spec` | `workspace` | в `packages/notes-flow/vibevm/vibespecs/NOTES-FLOW.md` положен Markdown-спек из одного заголовка `# Notes flow {#root}` и одного абзаца `@fact:ONE-NOTE One note per review. @status:spec/done`; **cwd команды — `work/packages/notes-flow`** |
| `package-notes` | `workspace` | `vibe init package org.acme/notes packages/notes`; затем `vibe registry add local <абсолютный путь к каталогу work/registry> --path . --position primary`, где `work/registry/` — пустой каталог (реестр-каталог без пакетов; публикация идёт с `--dry-run`, ничего не пишется) |
| `docs-store` | `empty` | store прогрет пакетом `org.vibevm.core/vibevm-docs` — **невозможно до фазы 2** (вид `doc` неизвестен бинарнику); команды на этой фикстуре ждут A2.1 |

Фикстуры фазы 2 (A2.9) наследуют эту таблицу как первый корпус раннера:
каждая строка становится `examples/<фикстура>/example.toml` пакета
документации с объявленными правилами нормализации (A0.12 §5).

## Что не снимается в фазе P {#not-now}

| Страница | id | Почему | Когда |
|---|---|---|---|
| `start/install-vibe` | `windows-install` | вывод установщика снимается с дистрибутива релиза, не с отладочной сборки | фаза 5, публикация |
| `howto/read-documentation-locally` | `cache-add-docs`, `doc-serve` | вид `doc` и `vibe doc` появляются в фазе 2 | A2.1, A2.20 |

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
| `authoring/ship-tools-and-mcp-servers` | `bin-list` | `vibe bin list --path hello-vibe` | `hello-vibe` | ждёт |
| `authoring/specs-agents-can-cite` | `explain` | `vibe explain "spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET" --path hello-vibe` | `hello-vibe` | ждёт |
| `authoring/specs-agents-can-cite` | `convert` | `vibe refactor convert-source --from md --to xml --dry-run vibevm/vibespecs` | `package-spec` | ждёт |
| `authoring/write-a-flow` | `init-package` | `vibe init package org.acme/review-notes packages/review-notes` | `workspace-root` | ждёт |
| `authoring/write-a-flow` | `manifest` | `cat packages/review-notes/vibe.toml` | `workspace` | ждёт |
| `authoring/write-a-flow` | `check` | `vibe check --path packages/review-notes` | `workspace` | ждёт |
| `authoring/write-a-lang-package` | `init-lang` | `vibe init package org.acme/sql-style packages/sql-style` | `workspace-root` | ждёт |
| `howto/install-a-package` | `install` | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe-empty` | ждёт |
| `howto/install-a-package` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/install-a-package` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/publish-a-package` | `publish-dry-run` | `vibe registry publish packages/notes --dry-run` | `package-notes` | ждёт |
| `howto/publish-a-package` | `publish` | `vibe registry publish packages/notes --registry local --dry-run` | `package-notes` | ждёт |
| `howto/read-documentation-locally` | `cache-add-docs` | `vibe cache add org.vibevm.core/vibevm-docs` | `empty` | не сейчас (A2.1) |
| `howto/read-documentation-locally` | `doc-serve` | `vibe doc serve --help` | `docs-store` | не сейчас (A2.20) |
| `howto/remove-a-package` | `uninstall` | `vibe uninstall org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe` | ждёт |
| `howto/remove-a-package` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe-removed` | ждёт |
| `howto/set-up-a-workspace` | `workspace-table` | `cat vibe.toml` | `none` | ждёт |
| `howto/set-up-a-workspace` | `init-package` | `vibe init package org.acme/notes-flow packages/notes-flow` | `workspace-root` | ждёт |
| `howto/set-up-a-workspace` | `install` | `vibe install --assume-yes` | `workspace` | ждёт |
| `howto/update-packages` | `outdated` | `vibe outdated --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/update-packages` | `update` | `vibe update org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe` | ждёт |
| `howto/use-a-private-registry` | `registry-add` | `vibe registry add acme file:///tmp/acme-specs --path hello-vibe --position primary` | `hello-vibe` | ждёт |
| `howto/use-a-private-registry` | `registry-test` | `vibe registry test --path hello-vibe` | `hello-vibe` | ждёт |
| `howto/work-offline` | `cache-add` | `vibe cache add org.vibevm.world/wal` | `empty` | ждёт |
| `howto/work-offline` | `cache-list` | `vibe cache list` | `hello-vibe` | ждёт |
| `howto/work-offline` | `cache-check` | `vibe cache check` | `hello-vibe` | ждёт |
| `howto/work-offline` | `install-offline` | `vibe install org.vibevm.world/wal --path hello-vibe --offline --assume-yes` | `hello-vibe-empty` | ждёт |
| `lifecycle/build-package-deploy` | `package` | `vibe package --path hello-vibe` | `hello-vibe` | ждёт |
| `lifecycle/build-package-deploy` | `deploy-plan` | `vibe deploy --plan --path hello-vibe` | `hello-vibe` | ждёт |
| `lifecycle/build-package-deploy` | `deployments` | `vibe deployments` | `hello-vibe` | ждёт |
| `lifecycle/extensions-and-providers` | `extensions` | `vibe extensions --path hello-vibe` | `hello-vibe` | ждёт |
| `lifecycle/extensions-and-providers` | `tools` | `vibe tools --path hello-vibe` | `hello-vibe` | ждёт |
| `lifecycle/phases` | `deploy-plan` | `vibe deploy --plan --path hello-vibe` | `hello-vibe` | ждёт |
| `lifecycle/scrape` | `scrape-plan` | `vibe scrape --plan --path hello-vibe` | `hello-vibe` | ждёт |
| `lifecycle/scrape` | `scrape-output` | `vibe scrape --output hello-clean --path hello-vibe` | `hello-vibe` | ждёт |
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
