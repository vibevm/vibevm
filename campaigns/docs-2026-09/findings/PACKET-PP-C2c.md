# PACKET-PP-C2c — снятие `expect` примеров третьей редакции фикстур (P.3)

> **Не запускался.** Слово владельца 2026-09-12: снятия минимизировать до
> абсолютно необходимого. Пятнадцать примеров ниже получают `expect` от
> раннера фазы 2 (A2.9), который снимает весь корпус механически; пакет
> сохранён как точный список того, что раннеру предстоит снять первым.

##subagent-quiet-clause

Ты — воркер кампании документации. Boot-лейн репозитория не читать. Твоя
инструкция — этот файл и ровно те файлы, которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `campaigns/docs-2026-09/findings/PACKET-PP-C2.md` и
   `PACKET-PP-C2b.md` — окружение, нормализация, форма результата и отчёта;
   всё действует, кроме переопределённого ниже.
3. `campaigns/docs-2026-09/EXAMPLES-TODO.md` **третьей редакции** — таблица
   фикстур с `hello-vibe-cwd`, `hello-cargo`, `hello-deploy`,
   `hello-vibe-scrape`.

## Что снимать

Только примеры, которых не было или которые изменились после прогона
PP-C2b (страницы правились коммитом `50439d94`):

| Страница | id | Команда | Фикстура |
|---|---|---|---|
| `authoring/ship-tools-and-mcp-servers` | `bin-list` | `vibe bin list` | `hello-vibe-cwd` |
| `lifecycle/extensions-and-providers` | `tools` | `vibe tools` | `hello-vibe-cwd` |
| `lifecycle/build-package-deploy` | `package` | `vibe package --path hello-deploy --assume-yes` | `hello-deploy` |
| `lifecycle/build-package-deploy` | `deploy-plan` | `vibe deploy --plan --profile local --path hello-deploy` | `hello-deploy` |
| `lifecycle/phases` | `deploy-plan` | `vibe deploy --plan --profile local --path hello-deploy` | `hello-deploy` |
| `lifecycle/scrape` | `scrape-contract` | `vibe scrape contract init --path hello-vibe` | `hello-cargo` |
| `lifecycle/scrape` | `scrape-plan` | `vibe scrape --plan --path hello-vibe` | `hello-vibe-scrape` |
| `lifecycle/scrape` | `scrape-output` | `vibe scrape --output hello-clean --path hello-vibe` | `hello-vibe-scrape` |
| `architecture/traceability` | `explain` | `vibe explain "spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET"` | `host` |
| `architecture/traceability` | `select` | `vibe select --where "uri:spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET depth:1"` | `host` |
| `authoring/specs-agents-can-cite` | `explain` | `vibe explain "spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET"` | `host` |
| `howto/use-a-private-registry` | `registry-add` | `vibe registry add acme git@github.com:acme-specs --path hello-vibe --position primary` | `hello-vibe` |
| `howto/update-packages` | `outdated` | `vibe outdated --path hello-vibe` | `hello-vibe-registry` |
| `howto/update-packages` | `update` | `vibe update org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe-registry` |
| `model/versions` | `outdated` | `vibe outdated --path hello-vibe` | `hello-vibe-registry` |

Фикстура `host` — единственная, чей cwd лежит в дереве хоста: команды на ней
только читают (`explain`, `select`); после них `git status --short` хоста
обязан совпасть со снимком «до». Всего к снятию — **пятнадцать** примеров.

Корень песочниц — `%TEMP%\vdocs-c\`; реестр локальный, как в правиле 3
`EXAMPLES-TODO.md`. `cargo` есть на машине; сборка `hello` — крошечная.
Каталог результатов тот же, `campaigns/docs-2026-09/findings/PP-C2-expects/`
— **дописать** восемь троек файлов, ничего не удаляя. Если фикстура не
строится (например, `vibe scrape contract check` не зелёный после правки
контракта) — не изобретать обход: записать вывод и остановить только эту
линию.

Отчёт: `campaigns/docs-2026-09/findings/WORKER-REPORT-PP-C2c.md` той же
формы (бинарник; таблица; фикстуры; отклонения; новые аномалии; что не
сделано).

Готово, когда восемь примеров сняты или их падение описано, отчёт лежит на
месте, а `git status --short` хоста показывает только результаты и то, что
было до тебя.
