# PACKET-PP-O1b — повторный прогон исправленных промптов (P.7b)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет.

## Читать сначала

1. Этот пакет целиком.
2. `campaigns/docs-2026-09/findings/PACKET-PP-O1.md` — первый пакет: цель,
   правила, окружение, форма результата. Всё в нём действует, кроме того,
   что переопределено ниже.
3. `campaigns/docs-2026-09/findings/WORKER-REPORT-PP-O1.md` §3 — что было
   красным и почему; не повторять диагностику продукта.
4. `campaigns/docs-2026-09/EXAMPLES-TODO.md` — правила песочницы (четыре)
   и таблица фикстур **третьей редакции** (локальный реестр, слоты,
   `hello-cargo`, `hello-deploy`, `hello-vibe-scrape`).
5. Скилл агента: `C:\Users\olegc\.claude\skills\vibevm\SKILL.md`
   (установленный; в дереве хоста файла нет — дефект первого пакета).
6. Страницы — только блок `<prompt>`, как в первом пакете.

## Что переопределено

- **Корень песочниц короткий:** `%TEMP%\vdocs-o1\<id>\` (B-132).
- **Реестр локальный**, как в `EXAMPLES-TODO.md` правило 3:
  `home\registry.toml` с `[[registry]] name = "local"
  url = "file:///C:/Users/olegc/git/v/vibevm-docs/vibevm/vibepacks"` до
  первого запуска `vibe`. Сеть не нужна.
- **Прогоняются только промпты, чьи страницы правились после первого
  прогона**, на фикстурах третьей редакции:

| id промпта | Страница | Фикстура | Что изменилось |
|---|---|---|---|
| `ship-tools` | `authoring/ship-tools-and-mcp-servers` | `project`, cwd = `work\` | слот вместо `packages/`; ассерты без `--path`; пакет ставится в проект перед `vibe bin` |
| `scrape` | `lifecycle/scrape` | `hello-cargo`, cwd = `work\hello-vibe` | контракт: `init`, `modified = "delete"`, health по `cargo`; папка рядом без `..` |
| `build-package-deploy` | `lifecycle/build-package-deploy` | `hello-cargo`, cwd = `work\hello-vibe` | таблицы профиля в `by-hand` страницы — но ты читаешь только промпт: промпт обязан довести до плана через `needs`; если не довёл — находка про промпт |
| `work-offline` | `howto/work-offline` | `hello-vibe`, cwd = `work\hello-vibe` | второй ассерт — `vibe reinstall --force --offline --assume-yes` |
| `set-up-a-workspace` | `howto/set-up-a-workspace` | `hello-vibe-empty`, cwd = `work\hello-vibe` | члены пишутся руками; `vibe check` может отказать на члене вида `doc` — записать как «ждёт фазы 2», не как баг |
| `update-packages` | `howto/update-packages` | `hello-vibe`, cwd = `work\hello-vibe` | ожидается отказ `vibe outdated` (B-136); если агент по промпту добавит реестр и пройдёт — записать, как именно |
| `write-a-flow` | `authoring/write-a-flow` | `project`, cwd = `work\` | контроль: промпт переписан на слот; должен остаться зелёным |

- Остальные промпты не прогонять: их вердикт первого прогона стоит.
- Как и раньше: `vibe registry publish` и `vibe mcp install` только с
  `--dry-run`; `vibe self …` не запускать; страницы не читать дальше
  `<prompt>`; бинарник не пересобирать.

## Результат

`campaigns/docs-2026-09/findings/WORKER-REPORT-PP-O1b.md` той же формы, что
первый отчёт (бинарник, HEAD, трипвайр; таблица прогонов; красные с хвостом
вывода; новые аномалии; что не сделано), и каталоги
`campaigns/docs-2026-09/findings/PP-O1-runs/<id>/` **перезаписать** для
семи прогнанных промптов (prompt.txt, agent-log.md, asserts.md), не трогая
остальные двенадцать.

Готово, когда отчёт и семь каталогов лежат на местах, а `git status --short`
хоста показывает только их и то, что было до тебя.
