# PACKET-PP-O1 — прогон промптов страниц сценариев агентом (P.7b)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `campaigns/docs-2026-09/EXAMPLES-TODO.md` — §«Три правила песочницы» и
   таблица «Фикстуры» (рецепты); очередь примеров не нужна.
3. `campaigns/docs-2026-09/findings/A0.12-example-runner.md` §1 — переменные
   изоляции и нативное написание путей.
4. Скилл, который получает агент пользователя:
   `vibevm/vibespecs/skills/vibevm/SKILL.md` (в дереве хоста) — прочитать
   целиком и следовать ему при выполнении промптов, как следовал бы агент
   пользователя.
5. Страницы сценариев, по одной перед соответствующим прогоном:
   `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/<страница>.xml`
   — только блок `<prompt>` (текст, `needs`, `outcome`, `assert`). Остальной
   текст страницы читать **нельзя**: прогон проверяет, достаточно ли
   промпта самого по себе.

Стоящие правила, которые тебя связывают:

- **Секреты.** Не читать `~/.vibe/*.token`, `secrets.local.md`, `infra/`.
  Публикация в сеть запрещена: `vibe registry publish` — только с `--dry-run`.
- **Настоящий дом** `%USERPROFILE%\.vibe` и конфиги агентов
  (`~/.claude*`, `~/.codex*`, `~/.gemini*`, `~/.cursor*`) не трогать:
  `vibe mcp install` — только с `--dry-run`. Трипвайр: рекурсивный листинг
  `%USERPROFILE%\.vibe` до и после всех прогонов, diff в отчёт.
- **Git.** Только `git status --short` и `git rev-parse HEAD` в корне хоста.
  Никаких коммитов. Команды продукта никогда не запускаются с cwd в корне
  хоста.
- **Запись** — только в песочницы под scratch и в каталог результатов ниже.
- **Сеть.** Разрешена к `github.com/vibespecs` (реестр по умолчанию) для
  установок без `--offline`. Ничего не публиковать, ничего не пушить.

## Цель

Каждый `prompt` страниц сценариев прогоняется руками по макету A2.29,
которого ещё нет: чистая песочница с фикстурой → ты получаешь текст
промпта как задание пользователя и выполняешь его так, как выполнил бы
агент со скиллом `vibevm` → затем выполняются `assert` промпта, их коды
выхода записываются вместе с хвостом твоего вывода. Промпт проверяется на
самодостаточность: если тебе не хватило информации, чтобы выполнить его
без чтения страницы, — это находка про промпт, а не повод читать страницу.

## Установка

- Хост `C:\Users\olegc\git\v\vibevm-docs`; бинарник
  `C:\Users\olegc\git\v\vibevm-docs\target\debug\vibe.exe` — не
  пересобирать; версия, mtime, HEAD — в отчёт. В командах он зовётся
  `vibe` через абсолютный путь или временный каталог с копией на PATH
  песочницы.
- Песочницы под
  `<scratch>\PP-O1\<id промпта>\`
  с `home\` и `work\`; окружение и фикстуры — по `EXAMPLES-TODO.md`
  (три переменные изоляции + `NO_COLOR`, поведенческие удалены; нативные
  пути; `[init] last_author = "vibevm docs fixtures"`).
- cwd каждого прогона — `work\` песочницы (для промптов «в текущей папке» —
  это и есть проект).

## Прогоны

| id промпта | Страница | Фикстура (по рецептам) | Особые условия |
|---|---|---|---|
| `first-project` | `start/first-project` | `empty` | — |
| `install-a-package` | `howto/install-a-package` | `hello-vibe-empty`, cwd = `work\hello-vibe` | — |
| `update-packages` | `howto/update-packages` | `hello-vibe`, cwd = `work\hello-vibe` | «нечего обновлять» — допустимый исход, если реестр не даёт версии новее |
| `remove-a-package` | `howto/remove-a-package` | `hello-vibe`, cwd = `work\hello-vibe` | — |
| `work-offline` | `howto/work-offline` | `hello-vibe`, cwd = `work\hello-vibe` | пакет `org.vibevm.world/multi-user-planning` есть в in-tree реестре хоста; если сеть его не даёт, второй запуск с `--offline --path <корень хоста>` для прогрева — записать как отклонение |
| `set-up-a-workspace` | `howto/set-up-a-workspace` | `hello-vibe-empty`, cwd = `work\hello-vibe` | — |
| `private-registry` | `howto/use-a-private-registry` | `hello-vibe`, cwd = `work\hello-vibe` | адрес `git@github.com:acme-specs` из промпта вымышлен: `vibe registry test` обязан упасть на нём; записать факт; **не** подменять адрес — это проверка промпта |
| `publish-a-package` | `howto/publish-a-package` | `package-notes`, cwd = `work\` | промпт просит спросить перед публикацией: остановиться на вопросе, выполнить только `--dry-run`; ассерт — dry-run |
| `scrape` | `lifecycle/scrape` | `hello-vibe`, cwd = `work\hello-vibe` | — |
| `build-package-deploy` | `lifecycle/build-package-deploy` | `hello-vibe`, cwd = `work\hello-vibe` | профиль `local` в проекте не объявлен: как агент, объяви его минимально по `vibe deploy --help` / `vibe package --help`; если это невозможно без чтения спеки — находка про промпт |
| `write-a-flow` | `authoring/write-a-flow` | `workspace-root`, cwd = `work\` | — |
| `write-a-feat-or-stack` | `authoring/write-a-feat-or-stack` | `workspace-root`, cwd = `work\` | — |
| `write-a-lang-package` | `authoring/write-a-lang-package` | `workspace-root`, cwd = `work\` | — |
| `ship-tools` | `authoring/ship-tools-and-mcp-servers` | `workspace-root`, cwd = `work\` | крейт `crates/notes-check` создаётся тобой минимальным (`cargo init`), сборка через `vibe bin build`; `cargo` есть на машине |
| `give-your-agent-the-skill` | `agent/give-your-agent-the-skill` | `hello-vibe`, cwd = `work\hello-vibe` | **только** `vibe mcp install … --dry-run`; ассерты запускать, результат записать как «условный» |
| `install-vibe` | `start/install-vibe` | — | **не выполнять**: установка меняет машину; записать «не прогоняется в песочнице», ассерты выполнить на этой машине как есть |
| `read-docs-locally` | `howto/read-documentation-locally` | — | ждёт фазы 2 (`vibe doc`); не выполнять, записать |
| `write-documentation` | `authoring/write-documentation` | — | ждёт фазы 2 (`vibe doc check`); не выполнять, записать |
| `translate-documentation` | `authoring/translate-documentation` | — | ждёт фазы 2; не выполнять, записать |

Порядок внутри прогона: построить фикстуру → записать текст промпта в
`PP-O1\<id>\prompt.txt` → выполнить задание → записать, что сделал, в
`PP-O1\<id>\agent-log.md` (команды по порядку, их коды выхода, твои
решения, что осталось непонятным из промпта) → выполнить каждый `assert`
из cwd прогона, записать код выхода и вывод в `PP-O1\<id>\asserts.md`.
Ассерт с `test`/`grep` — в Git Bash; `vibe …` — бинарник хоста.

## Результат

`campaigns/docs-2026-09/findings/WORKER-REPORT-PP-O1.md`:

1. Бинарник, HEAD, трипвайр (пустой diff или diff целиком).
2. Таблица: id промпта | выполнен (да / частично / не выполнялся) | ассерты
   (коды через запятую) | вердикт (зелёный / красный) | причина красного:
   промпт неясен / ассерт неверен / баг продукта / ждёт фазы 2.
3. Для каждого красного — хвост вывода (до 30 строк) и одна фраза, чего не
   хватило.
4. Аномалии продукта (паника, противоречие `--help` и поведения, след на
   диске вне песочницы) — списком, без правок.
5. Что не сделано и почему.

Копию каталогов `PP-O1\<id>\{prompt.txt,agent-log.md,asserts.md}` положить в
`campaigns/docs-2026-09/findings/PP-O1-runs/<id>/`.

## Что не делать

Не редактировать страницы и файлы зоны, кроме отчёта и `PP-O1-runs/`. Не
читать текст страниц дальше блока `prompt`. Не пересобирать бинарник хоста.
Не «улучшать» промпт по ходу: выполняется ровно тот текст, что на
странице. Не запускать `vibe self …`, `vibe cache clean`, `vibe mcp
install` без `--dry-run`, `vibe registry publish` без `--dry-run`.

Готово, когда отчёт и `PP-O1-runs/` лежат на местах, а `git status --short`
хоста показывает только их и то, что было до тебя.
