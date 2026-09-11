# WORKER-REPORT-P0-O7

Пакет: P0-O7 — store и источники пакетов (спайки A0.8, A0.17, A0.20)
Дата: 2026-09-11
Дерево: `C:\Users\olegc\git\v\vibevm-docs`, ветка `research-preview-1-docs`, HEAD `b1291b06`
Scratch: `<scratch>/vibe-docs-phase0\P0-O7\`

## Деливераблы

- `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.8-store-and-sources.md`
- `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.17-intree-to-store.md`
- `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.20-host-from-source.md`

## Решения

1. **Изоляция домашней папки — обёрткой, а не переменными в сессии.** Все
   запуски `vibe.exe` шли через `<tmp>/vibe-iso.sh`, которая на каждый вызов
   ставит `VIBE_SETTINGS`, `VIBE_REGISTRY_CACHE`, `VIBEVM_SEARCH_CACHE_DIR` в
   scratch (`exec env … vibe.exe "$@"`). Три переменные, а не две: именно их
   ставит загрузочный конструктор `vibe-test-support`
   (`crates/vibe-test-support/src/isolate.rs:104–127`, имена — `:46`, `:52`,
   `:56`). Обёртка выбрана потому, что `export` в оболочке отвергается
   песочницей worktree-сессии.
2. **Store лежит не там, где ждал пакет.** Корень store — это
   `$VIBE_SETTINGS/cache`, а не `$VIBE_REGISTRY_CACHE`
   (`crates/vibe-registry/src/store.rs:79–83`; собственного override у store нет
   по решению владельца, `:30–37`). `VIBE_REGISTRY_CACHE` уводит git-клон-кэш
   (`~/.vibe/registries`). Поэтому дерево store в находках —
   `<tmp>/home/settings/cache/…`, а не `<tmp>/cache/…`.
3. **Контрольные пробы вместо утверждения по коду.** Чтобы доказать, что
   `cache add` из корня хоста берёт пакет из project-local `vibevm/vibepacks`, а
   не из vibe-embedded и не из сети, сделаны две отрицательные пробы: проект без
   `vibevm/vibepacks` и запуск без проекта вовсе. Обе падают — источник
   идентифицирован однозначно.
4. **Суррогатный пакет в scratch для проб мутабельности.** Фаза 0 запрещает
   менять файлы репозитория, а вопрос «обновится ли одна и та же версия»
   требует правки содержимого. Поэтому сделаны два одноразовых пакета в scratch:
   git-репозиторий `pkgrepo` (для git-источника по ветке) и in-tree пакет в
   `localproj/vibevm/vibepacks/…` (для project-local). Ни одного байта в
   `vibevm-docs` не записано.
5. **Bare-клон хоста сделан один раз**, как предписано пакетом, и использован
   для пробы git-источника по ветке `main`. Проба дала отрицательный, но
   определённый ответ (корень хоста не пакет), поэтому полный клон содержимого
   не разворачивался.
6. **`--offline` по умолчанию в пробах.** Корневой `vibe.toml` объявляет два
   сетевых реестра; без `--offline` резолвер опрашивает их ради объединения
   версий (PROP-030 `##ENUM-UNION`). Сеть в фазе 0 не нужна, поэтому пробы, где
   это возможно, шли офлайн; отдельно проверено, что офлайн не мешает и
   локальному git-источнику.

## Найденное, что стоит внимания оркестратора

- **`vibe cache add` печатает «already present — bytes untouched (write-once)»
  и тогда, когда содержимое записи в store было заменено.** `was_present`
  считается до выборки (`crates/vibe-cli/src/commands/cache/add.rs:70`), а
  выборка идёт через `insert_current_at`, который сносит устаревшую запись
  (`crates/vibe-registry/src/store/refresh.rs:24–56`). Воспроизведено четырьмя
  пробами. Для сайта это значит: свежесть определять по sidecar-хэшу
  `<store>/<group>/<name>/v<version>.sha256`, не по строке отчёта. Помечено
  `REVIEW:` в A0.8 и A0.20.
- **Корень хоста не является пакетом** — в `vibe.toml` таблица `[project]`, не
  `[package]`. B-031 дал корню квалифицированное имя, но не публикуемость.
  Значит, «сайт рендерит документацию хоста» сегодня исполняется чтением файлов
  выкладки, а не выборкой пакета; doc-пакет ядра (фаза 3) это меняет.
- **Рекомендованный путь для сервера** — `vibe cache add --offline` из корня
  выкладки: project-local реестр читает живой каталог на диске, клон не нужен,
  сеть не нужна, публикация не нужна. Проверено живьём, включая обновление той
  же версии после правки файла в месте.

## Отклонения от пакета

1. Пакет предписывал дерево store в `<tmp>/cache/…`; фактически оно в
   `<tmp>/home/settings/cache/…` (причина — решение 2 выше). Смысл проверки
   сохранён.
2. Пакет называл ожидаемым источником для A0.17 «встроенный реестр (PROP-030,
   `vibevm/vibepacks` как реестр)». Фактический источник — **project-local**
   реестр PROP-030 §3.3, а не vibe-embedded PROP-030 §2. Это два разных
   механизма в одном PROP: vibe-embedded выводится из записи VVM-установки и для
   `target/debug/vibe.exe` недоступен в принципе, плюс `cache add` передаёт
   `embedded_root = None`. Записано как факт, решение не переписывалось.
3. Флаг `--registry` у `cache add` отсутствует, поэтому вариант «явный путь /
   флаг `--registry`» из формулировки A0.17 проверить было нечего; зафиксировано
   по `--help` и по коду.
4. Живая проба git-источника по ветке сделана дважды: на bare-клоне хоста (как
   предписано) и дополнительно на суррогатном однопакетном репозитории.
   Дополнительная проба понадобилась потому, что предписанная упала до выборки
   содержимого и сама по себе не отвечала на вопрос «даёт ли `branch` текущее
   содержимое, а не тег».

## Дефекты пакета

- Пакет считает, что `VIBE_REGISTRY_CACHE` относится к store. Он относится к
  git-клон-кэшу; store движется только с `VIBE_SETTINGS`.
- Пакет спрашивает, «есть ли `vibe.toml` с `[package]` в корне» — в модели vibevm
  проект и пакет различаются таблицами `[project]` / `[package]`, и корень хоста
  всегда был проектом. Вопрос корректен, но его формулировка предполагает, что
  `[package]` — ожидаемая форма для корня.
- Правила из `PACKET-COMMON.md` покрыли всё, что понадобилось; недостающих
  правил не встретилось. Секретов не открывал; ни один файл `~/.vibe/*.token` не
  читался.

## Самопроверка

### `ls ~/.vibe` — до и после

До (снято перед первым запуском):

```text
aiui
cache
config.toml
github.publish.token
opt
progress-cache
registries
registry.toml
settings.toml
state
steward
zai.api.token
zai.api.token.2
```

После (exit 0):

```text
aiui
cache
config.toml
github.publish.token
opt
progress-cache
registries
registry.toml
settings.toml
state
steward
zai.api.token
zai.api.token.2
```

`diff` до/после — пусто, exit 0.

### `ls ~/.vibe/cache | head` — до и после

До:

```text
com.mattpocock
embedded
org.speckit
org.vibevm.ai-native
org.vibevm.bridges
org.vibevm.fractality
org.vibevm.world
skill-compositions
```

После (exit 0):

```text
com.mattpocock
embedded
org.speckit
org.vibevm.ai-native
org.vibevm.bridges
org.vibevm.fractality
org.vibevm.world
skill-compositions
```

`diff` до/после — пусто, exit 0. Настоящий store не тронут.

### Точные команды проб и коды выхода

Все `vibe.exe` — через `<tmp>/vibe-iso.sh` (изолированное окружение), рабочий
каталог `C:\Users\olegc\git\v\vibevm-docs`, если не сказано иное.

| # | команда | exit |
|---|---|---|
| 1 | `target/debug/vibe.exe --version` (в изоляции) | 0 |
| 2 | `vibe.exe cache --help` | 0 |
| 3 | `vibe.exe cache add --help` | 0 |
| 4 | `vibe.exe install --help` | 0 |
| 5 | `vibe.exe --help` | 0 |
| 6 | `vibe.exe show --help` | 0 |
| 7 | `vibe.exe cache path` | 0 |
| 8 | `vibe.exe cache list` | 0 |
| 9 | `vibe.exe cache add --offline "org.vibevm.world/multi-user-planning@1.0.0"` | 0 |
| 10 | `vibe.exe cache add --offline --path <tmp>/emptyproj "org.vibevm.world/multi-user-planning@1.0.0"` | 1 |
| 11 | `vibe.exe cache add --offline --path <tmp>/noproj "org.vibevm.world/qualified-naming@1.0.0"` | 1 |
| 12 | `git clone --bare "C:/Users/olegc/git/v/vibevm-docs" "<tmp>/host.git"` | 0 |
| 13 | `vibe.exe cache add --path <tmp>/consumer-host "org.vibevm.core/vibevm"` | 1 |
| 14 | `git init -q -b main` + 4 коммита в `<tmp>/pkgrepo` | 0 |
| 15 | `vibe.exe cache add --path <tmp>/consumer-pkg "org.probe/probe-doc"` (проба B1) | 0 |
| 16 | то же, после коммита «revision two» (B2) | 0 |
| 17 | то же, при незакоммиченной правке (B3) | 0 |
| 18 | `vibe.exe cache add --path <tmp>/consumer-plainpath "org.probe/probe-doc"` (B4, голый путь) | 0 |
| 19 | `vibe.exe cache add --offline --path <tmp>/consumer-plainpath "org.probe/probe-doc"` (B5) | 0 |
| 20 | `vibe.exe cache add --offline --path <tmp>/localproj "org.probe/probe-local@0.1.0"` (C1) | 0 |
| 21 | то же, после правки файла в месте (C2) | 0 |
| 22 | `vibe.exe cache list --json` | 0 |
| 23 | `vibe.exe show config` | 0 |
| 24 | `git log -1 --format=%B 1ef63a37` | 0 |

### `ls` файлов находок

```text
C:/Users/olegc/git/v/vibe-docs-vision/findings/A0.17-intree-to-store.md
C:/Users/olegc/git/v/vibe-docs-vision/findings/A0.20-host-from-source.md
C:/Users/olegc/git/v/vibe-docs-vision/findings/A0.8-store-and-sources.md
```

exit 0.

### `git -C C:/Users/olegc/git/v/vibevm-docs status --short`

```text
 M .claude/agents/opus5.md
```

exit 0. Единственная строка — ожидаемое пакетом чужое изменение; моих правок в
репозитории нет. `git add` / `commit` / `stash` / `checkout` / `restore` не
выполнялись.

## Что не сделано и почему

- **Повторный `cache add` того же in-tree пакета хоста после правки его файлов**
  — фаза 0 запрещает менять файлы репозитория, а мутабельный слот лежит внутри
  него. Механизм доказан на суррогатных пакетах в scratch (пробы B2/B4/C2) и по
  коду `local_registry.rs:197` → `store/refresh.rs:24`.
- **Полный клон хоста через git-источник** — предписанная проба упала на чтении
  манифеста (корень не пакет), до выборки содержимого; разворачивать клон
  ≈175 МиБ ради факта, который уже известен, смысла не было.
- **`vibe update` для ветки-git-источника** — вне объёма пакета; отмечено как
  открытый вопрос в A0.20.
- **Проверка store после переноса `$VIBE_SETTINGS` между машинами** — вне
  возможностей одной машины; отмечено как открытый вопрос в A0.8.
- Файлы `~/.vibe/*.token` и `C:\Users\olegc\git\infra\**` не открывались;
  IP-адресов, портов, путей на сервере и имён VPN-компонентов в находках нет.
