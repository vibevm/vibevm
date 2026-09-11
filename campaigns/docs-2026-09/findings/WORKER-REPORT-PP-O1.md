# WORKER-REPORT-PP-O1 — прогон промптов страниц сценариев агентом (P.7b)

Дата: 2026-09-12. Дерево: `C:\Users\olegc\git\v\vibevm-docs`.

Прогнано 19 промптов: 15 в песочницах, 1 (`install-vibe`) — только ассерты на
этой машине, 3 ждут фазы 2. Прогоны выполнялись как агент пользователя со
скиллом `vibevm`: сначала `<subcommand> --help`, `--assume-yes` в каждой
неинтерактивной команде, ошибка сообщается как есть, конфигурация реестров не
«улучшается» по своей инициативе.

Каталоги прогонов:
`C:\Users\olegc\git\v\vibevm-docs\campaigns\docs-2026-09\findings\PP-O1-runs\<id>\{prompt.txt,agent-log.md,asserts.md}`.
Песочницы (с `home\` и `work\`):
`<scratch>\PP-O1\<id>\`. Полный diff трипвайра —
`PP-O1-runs\_tripwire-diff.txt`.

## 1. Бинарник, HEAD, трипвайр

**Бинарник.** `C:\Users\olegc\git\v\vibevm-docs\target\debug\vibe.exe`,
`vibe 1.0.0`. Не пересобирался мной, но **пересобран сторонней сессией в
середине прогона**: на входе `104 577 536` байт, mtime `2026-09-12 00:50`; на
выходе `104 571 904` байта, mtime `2026-09-12 01:37`. Версия в обоих случаях
`vibe 1.0.0`. Прогоны 1–16 и все ассерты, кроме четырёх перезаписанных (§2,
сноска о дрейфе), сняты со сборки 00:50; повторные прогоны
`write-a-feat-or-stack` и `write-a-lang-package` и их ассерты — со сборки 01:37.
Поведение, о котором идёт речь в находках ниже, на обеих сборках одинаковое —
но строгого byte-to-byte сравнения одной сборки у этого отчёта нет.

**HEAD.** Дерево живое: `5cb6dc10` на входе → `9d7c233b` в середине →
`9f397db9` на выходе. Сторонняя сессия коммитила и правила страницы во время
прогона; следствие — дрейф четырёх промптов, §2.

**Трипвайр `%USERPROFILE%\.vibe`** — рекурсивный листинг (тип, размер, mtime,
путь) до и после всех прогонов. **Diff не пустой: 84 строки, 42 добавления,
8 удалений.** Все изменённые пути — внутри двух поддеревьев:

| поддерево | строк diff |
|---|---|
| `~/.vibe/cache/org.vibevm.world/multi-user-planning/**` | 44 |
| `~/.vibe/steward/contexts/d2ab5a82-5b2b-4cd9-ab41-3a4e48f8bfe1/**` | 6 |

Вне этих двух — ни одной строки (проверено: `grep -v multi-user-planning |
grep -v .vibe/steward` пусто). Все добавленные записи имеют ровно два штампа
времени: `01:35:20` и `01:36:40` — две односекундные вспышки, а не след 19
прогонов, растянутых на час.

Атрибуция: это не мои команды. Каждый `vibe` в песочнице шёл с
`VIBE_SETTINGS` внутрь песочницы, и вывод это подтверждает построчно
(`Pre-warming the machine store (…\PP-O1\<id>\home\cache)`). Настоящий дом
использовали ровно две команды — ассерты `install-vibe` (`vibe --version`,
`vibe self doctor` без `--fix`); вокруг них снят отдельный трипвайр
непосредственно до и после, и он **побайтово одинаков** (46 514 записей в обе
стороны). Содержимое изменений — пакет `multi-user-planning` и контекст
`steward` — и время (одна минута до пересборки бинарника в 01:37) указывают на
ту же стороннюю центральную сессию.

## 2. Таблица прогонов

Ассерты выполнены дословно из блоков `<assert>`, из cwd прогона; `vibe` —
шим на PATH песочницы, указывающий на отладочный бинарник хоста; `test` /
`grep` — Git Bash.

| id промпта | выполнен | ассерты | вердикт | причина красного |
|---|---|---|---|---|
| `first-project` | да | 0,0,0,0 | зелёный | — |
| `install-a-package` | да | 0,0 | зелёный | — |
| `update-packages` | частично | 0,0 | красный | баг продукта |
| `remove-a-package` | да | 1,0 | красный | баг продукта |
| `work-offline` | частично | 0,0 | красный | ассерт неверен + баг продукта |
| `set-up-a-workspace` | частично | 0,0,1,0 | красный | баг продукта + ждёт фазы 2 |
| `private-registry` | да | 0,0 | красный | ассерт неверен + промпт неясен |
| `publish-a-package` | частично | 1 | красный | баг продукта |
| `scrape` | нет | 0,0 | красный | баг продукта + ассерт неверен |
| `build-package-deploy` | частично | 1,0 | красный | промпт неясен |
| `write-a-flow` | да | 0,0 | зелёный | — |
| `write-a-feat-or-stack` | да | 0,0 | зелёный | — |
| `write-a-lang-package` | да | 0,0 | зелёный | — |
| `ship-tools` | частично | 2,1 | красный | ассерт неверен + баг продукта |
| `give-your-agent-the-skill` | частично (только `--dry-run`) | 0,0 | зелёный, условный | — |
| `install-vibe` | не прогоняется в песочнице | 0,0 | зелёный, только ассерты | — |
| `read-docs-locally` | не выполнялся | 0 | красный | ждёт фазы 2 |
| `write-documentation` | не выполнялся | 2,2 | красный | ждёт фазы 2 |
| `translate-documentation` | не выполнялся | 2,2 | красный | ждёт фазы 2 |

**Итог: 5 зелёных + 1 условно зелёный (`give-your-agent-the-skill`) + 1 вне
песочницы (`install-vibe`, оба ассерта 0); 12 красных — 9 по существу и 3 в
ожидании фазы 2.**

**Дрейф промптов во время прогона.** Четыре промпта были отредактированы
сторонней сессией пока пакет исполнялся: `write-a-feat-or-stack`,
`write-a-lang-package`, `write-documentation`, `translate-documentation` — все
четыре переведены с раскладки `packages/<name>/` на in-tree
`vibevm/vibepacks/org.acme/<name>/v0.1.0/`. Два первых **прогнаны повторно** по
новому тексту (оба зелёные; `vibe init package <group>/<name>` без пути кладёт
пакет ровно туда, куда теперь указывает ассерт); у двух последних обновлены
только ассерты. `prompt.txt` и `asserts.md` в `PP-O1-runs/` соответствуют
тексту страниц на выходе (проверено скриптом сверки: дрейфа нет).
`write-a-flow` и `set-up-a-workspace` не дрейфовали и остались на раскладке
`packages/`.

## 3. Красные прогоны: хвост вывода и чего не хватило

### `update-packages` — `vibe outdated` отказывает на свежем проекте

```
$ vibe outdated
error: no registry configured. Add a `[[registry]]` entry to `vibe.toml` or run
`vibe outdated` against a project that has one.
[exit 1]
$ vibe update --all
vibe.lock is fresh — skipping resolution
[exit 0]
```

`vibe init` **не** пишет `[[registry]]` в `vibe.toml` (манифест после
`vibe init hello-vibe` — четыре строки `[project]`). `vibe install` в том же
проекте резолвит нормально: он читает пользовательский
`~/.vibe/registry.toml`, который засеивается при первом запуске
(`vibespecs`, `vibespecs-gitverse`). `vibe outdated` и `vibe registry list`
этот уровень не читают.

Не хватило: промпт (и `needs` страницы) не сообщает, что «показать, у кого есть
версии новее» требует **проектного** `[[registry]]`, которого `vibe init` не
создаёт.

### `remove-a-package` — после удаления остаётся пустой каталог пакета

```
$ vibe uninstall org.vibevm.world/wal --assume-yes
Uninstall org.vibevm.world/wal@1.0.0 — remove `vibevm/vibedeps/org.vibevm.world.wal/1.0.0` and regenerate boot.
  - removed  vibevm/vibedeps/org.vibevm.world.wal/1.0.0
Uninstalled org.vibevm.world/wal@1.0.0 — removed its vibedeps/ slot, regenerated boot.
[exit 0]
$ ls -a vibevm/vibedeps
.  ..  org.vibevm.world.wal
===== ASSERT: test ! -e vibevm/vibedeps/org.vibevm.world.wal
----- exit: 1
```

Снимается версионный подкаталог `1.0.0`, родительский каталог пакета остаётся
пустым. Требование `outcome` («the package's folder is gone from
`vibevm/vibedeps/`») и ассерт совпадают между собой — расходится продукт.

Не хватило: ничего; промпт ясен, продукт не делает обещанного.

### `work-offline` — прогрев падает на длине пути, последний ассерт вакуумен

```
$ vibe cache add org.vibevm.world/wal org.vibevm.world/multi-user-planning
error: resolving the dependency closure: git operation failed …
git clone … org.vibevm.world.multi-user-planning.git …\home\registries\4f0def76f4089d61\packages\org.vibevm.world.multi-user-planning\clone exited with status 128
fatal: '$GIT_DIR' too big
fatal: remote helper 'https' aborted session
[exit 1]
```

`org.vibevm.world/wal` в то же дерево клонируется без проблем — лишние 17
символов имени пакета переводят путь клона за предел Windows. Прогрев выполнен
разрешённым пакетом отклонением (`--offline --path <корень хоста>`), после чего
`cache list` = 2 пакета, `cache check` = `2 ok`.

Последний ассерт ничего не доказывает: проект фикстуры уже установлен, и
`vibe install --offline --assume-yes` отвечает `vibe.lock is fresh — skipping
resolution` (exit 0). Настоящее доказательство офлайн-установки требует проекта
с пустым `vibedeps/` — а этот путь сломан, см. аномалию A1.

Не хватило: команды для «забрать всё, что нужно проекту» — `vibe cache add`
принимает только ссылки на пакеты, так что агент сам читает
`[requires.packages]` и перечисляет их.

### `set-up-a-workspace` — `vibe init package` создаёт вложенный проект, а не члена

```
$ vibe init package org.acme/notes-flow packages/notes-flow
Initializing project `notes-flow` in `packages/notes-flow`
  ✓ created  vibe.toml
  ✓ created  vibe.lock
  ✓ created  .vibe/.gitignore
  ✓ created  CLAUDE.md … AGENTS.md … GEMINI.md
Creating package `org.acme/notes-flow` in `…\packages\notes-flow`
  ✓ created  vibevm/vibepacks/org.acme/notes-flow/v0.1.0/vibe.toml
[exit 0]
$ cat packages/notes-flow/vibe.toml
[project]
name = "notes-flow"
…
===== ASSERT: test ! -e packages/notes-flow/vibe.lock
----- exit: 1
```

Против `outcome` страницы: манифест члена несёт `[project]`, а не `[package]`
(`[package]` лежит уровнем ниже, в `vibevm/vibepacks/org.acme/notes-flow/v0.1.0/`);
корневой `vibe.lock` содержит только `[meta]`, зато у каждого члена появляется
свой `vibe.lock`, что третий ассерт прямо запрещает. Сам workspace продукт
видит: корневой `vibe install --assume-yes` → `3 nodes up to date`.

Не хватило: вторая половина промпта («where notes-docs documents notes-flow»)
невыразима — `kind = "doc"` отвергается (`unknown variant 'doc', expected one
of flow, feat, stack, tool, mcp, lang`), а `vibe init package` даёт каждому
члену `kind = "tool"`.

### `private-registry` — `registry test` называет вымышленный адрес достижимым

```
$ vibe registry test
Registry test
  acme       git@github.com:acme-specs     → reachable  (auth=none)
  vibespecs  https://github.com/vibespecs  → reachable  (auth=none)
vibe registry test: 2/2 reachable
[exit 0]
```

`vibe registry test --help` обещает: `reachable` = «org URL responded», и
предлагает `unreachable` / `auth-required` как другие исходы. Для SSH-адреса
несуществующей организации ответ — `reachable`, `auth=none`. Ассерт не может
упасть, и «test that both are reachable» фактически не проверяется. (Пакет
ожидал падения на этом адресе; падения нет.)

Не хватило: «keep the public vibespecs registry as a fallback» — сохранять было
нечего: `vibe init` не пишет `[[registry]]`, у проекта их ноль. Единственное
исполнимое чтение — добавить `vibespecs` явно (`--position append`), что и
сделано и записано как интерпретация. Плюс «authenticated over SSH» не имеет
выражения на этой поверхности: `vibe registry add` знает `--ref`, `--naming`,
`--position` и ничего про аутентификацию.

### `publish-a-package` — даже `--dry-run` требует токен

```
$ vibe registry publish packages/notes --dry-run
Publishing …\work\packages\notes → registry `local` (`…\work\registry`) [dry-run]
error: loading publish token: publish refused: no token available for host `C`.
Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibe/git.publish.token`.
[exit 1]
```

Две причины, и обе стоят внимания страницы. Первая — реестр фикстуры задан
каталогом; `vibe registry add` его принял и разобрал хост как букву диска:
`host: C (adapter: none — vibe registry publish won't dispatch here)`. Значит
«publish to the first registry of this project» на этой фикстуре недостижимо
в принципе. Вторая — `--dry-run` требует учётные данные прежде, чем вообще
что-то спланировать.

Не хватило: имени переменной. «Using the publish token already in my
environment» — промпт не называет её; `VIBEVM_PUBLISH_TOKEN` виден только в
тексте отказа.

### `scrape` — три стены подряд, экспорт не состоялся

```
$ vibe scrape --plan
error: not a Vibe project: default scrape contract `vibevm/scrape/contract.toml` is absent
[exit 1]
$ vibe scrape --output ../product-clean
error: pinning absent scrape export output `…\work\hello-vibe\../product-clean`:
absolute path contains a dot component: `…\work\hello-vibe\../product-clean`
[exit 1]
$ vibe scrape contract init      → exit 0
$ vibe scrape --plan
  blocker   modified-policy-refusal [vibe.toml]: Unknown content is refused by its modification policy
  blocker   modified-policy-refusal [vibe.lock]: Unknown content is refused by its modification policy
  … 23 blocker'а такого вида …
error: scrape plan is blocked by 23 finding(s)
[exit 1]
```

1. Дефолтного контракта нет, хотя `needs` предлагает «a scrape contract, **or
   the default one**»; создаётся `vibe scrape contract init` (не `create` —
   `create` даёт exit 2).
2. `../product-clean` — путь, который диктует сам промпт — отвергается: `..`
   не нормализуется.
3. С контрактом план заблокирован 23 находками на **чистом** `vibe init` +
   `vibe install` проекте: каждый файл, которым владеет vibe, для контракта
   «unknown content». Проект песочницы не является git-репозиторием, и контракт
   не может классифицировать ни один файл как неизменённый; ни промпт, ни
   `needs` не говорят, что scrape нужен контроль версий.

Оба ассерта проходят **на отсутствии**: `../product-clean` так и не появился,
поэтому `test ! -e ../product-clean/vibe.toml` = 0.

Не хватило: «prove that the copy builds without vibe» не имеет смысла для
`hello-vibe` — в фикстуре нет ни исходников, ни нативного сборщика.

### `build-package-deploy` — профиль `local` недостижим из `--help`

```
$ vibe deploy --plan --profile local
error: `--profile local` was requested, but this project declares no deploy profiles
(violates spec://…/PROP-054#OPEN-DEPLOY-TARGETS; fix: declare `[deploy.profiles.local]`
with its `targets`, or drop the flag)
[exit 1]
```

Шесть попыток объявить профиль, каждая по подсказке предыдущей ошибки:

| попытка | ошибка |
|---|---|
| `targets = []` | `field targets is empty; a profile is a nonempty ordered selection` |
| `[deploy.targets.hello]` | `unknown field targets, expected one of default_profile, target, profiles` |
| пустая `[[deploy.target]]` | `missing field id` |
| `name = "hello"` | `unknown field name, expected one of id, artifact, mechanism, provider, when, depends_on, config` |
| `id = "hello"` | `missing field artifact` |
| `id` + `mechanism` | `missing field artifact` |

`artifact` ведёт в граф build/package, которого у `vibe init`-проекта нет, а
`vibe deploy --help` и `vibe package --help` не говорят ни про artifacts, ни про
mechanisms, ни про providers. Манифест возвращён в исходное состояние, ассерты
прогнаны по честному достижимому состоянию. «Stop and ask me before the deploy»
соблюдено: ни deploy, ни undeploy не запускались.

Не хватило: схемы профиля. Это ровно тот случай, который пакет называл
«находкой про промпт»: без чтения спеки объявить рабочий `local` нельзя.
Отдельно: `--profile` принимается `vibe deploy`, но в его `--help` отсутствует.

### `ship-tools` — `vibe bin list` не знает `--path`, а `vibe bin` не видит свой пакет

```
===== ASSERT: vibe bin list --path packages/notes-tools
error: unexpected argument '--path' found
Usage: vibe.exe bin list [OPTIONS]
----- exit: 2
===== ASSERT: vibe bin exec notes-check --path packages/notes-tools -- --help
error: violates spec://…/PROP-025#dispatch: no installed package declares a binary
`notes-check` (declared: []); fix surface: `vibe bin list` shows the full table
----- exit: 1
```

Три независимых препятствия. Первое: у `vibe bin list` вообще нет `--path`
(только `--json`, `--quiet`, `--invoked-by`, `--agent-mode`), поэтому первый
ассерт умирает на разборе аргументов. `vibe bin exec` тот же `--path` молча
проглатывает вместо отказа. Второе: `vibe bin list | build | exec` читают
**локфайл** — «every `[[binary]]` declared by the project's *installed
packages*»; `[[binary]]`, объявленный в пакете, внутри которого работаешь, для
них невидим. С cwd внутри пакета: `bin list` → `no installed package declares a
[[binary]]` (exit 0), `bin build --assume-yes` → тот же текст (exit 1). Третье:
`vibe check --path packages/notes-tools` при этом «clean» — линтер пакет и его
`[[binary]]` видит, а диспетчер нет.

Не хватило: (а) относительно чего задан `crates/notes-check` — корня проекта
или корня пакета; (б) как `[[binary]]` пакета вообще должен становиться видимым
для `vibe bin`. Промпт подразумевает, что объявления достаточно; этого
недостаточно.

### `read-docs-locally`, `write-documentation`, `translate-documentation` — фаза 2

```
$ vibe doc --help
error: unrecognized subcommand 'doc'
[exit 2]
$ vibe check --path <probe with kind = "doc">
unknown variant `doc`, expected one of `flow`, `feat`, `stack`, `tool`, `mcp`, `lang`
in `package.kind`
[exit 1]
```

Ни `vibe doc`, ни вида `doc` в этом бинарнике нет; `[translates]` также не
входит в набор допустимых таблиц верхнего уровня. Промпты не выполнялись;
ассерты прогнаны в пустой песочнице для протокола (2,2 и 2,2 —
`unrecognized subcommand`), кроме `vibe cache list --quiet` (exit 0), который
проходит на любой машине, включая пустой store, и потому ничего из `outcome`
страницы не проверяет.

## 4. Аномалии продукта

Только наблюдения; никаких правок продукта не сделано.

**A1. `vibe cache add` + `vibe install --offline` не работают вместе — и совет
из самого сообщения об ошибке не помогает.** Store прогрет
(`vibe cache list` показывает `org.vibevm.world wal 1.0.0`), но

```
$ vibe install org.vibevm.world/wal --path hello-vibe --assume-yes --offline
error: `org.vibevm.world/wal@latest` is not resolvable offline — it is in no local
(`file://`) registry and not in the machine store (…; fix: run the install once online
to warm the store, pre-warm it with `vibe cache add org.vibevm.world/wal`, …)
[exit 1]
```

Сообщение советует ровно то, что уже сделано. Более того, с явным пином
`org.vibevm.world/wal@1.0.0` текст ошибки всё равно говорит `@latest` —
затребованная версия теряется на этом пути. Прямое следствие для кампании:
рецепт фикстуры `hello-vibe` в `EXAMPLES-TODO.md`
(`vibe install … --assume-yes --offline`) **не работает**; все мои прогоны
строили `hello-vibe` установкой без `--offline` (отклонение записано в каждом
`agent-log.md`).

**A2. `vibe init` не пишет `[[registry]]`, а команды расходятся в том, читать ли
пользовательский уровень.** У свежего проекта ноль проектных реестров;
`vibe install` работает (через `~/.vibe/registry.toml`), `vibe outdated`
отказывает, `vibe registry list` печатает «No `[[registry]]` entries in
`vibe.toml`». Установленный скилл `vibevm` при этом утверждает обратное: «Two
registries are already configured by `vibe init`: `vibespecs` … and
`vibespecs-gitverse`». Расходятся три источника: скилл, `install` и
`outdated`/`registry list`.

**A3. `vibe uninstall` оставляет пустой `vibevm/vibedeps/<pkg>/`.** См. §3.

**A4. `vibe init package <pkgref> <path>` разворачивает полноценный вложенный
проект.** Внутри члена workspace появляются `vibe.lock`, `.vibe/`, `.gitignore`
и три файла инструкций агента, а `[package]` уезжает в
`<member>/vibevm/vibepacks/<group>/<name>/v<ver>/`. Без пути (`vibe init package
org.acme/sql-style`) поведение другое и, по-видимому, задуманное: пакет
создаётся in-tree в текущем проекте, ровно как ждут обновлённые страницы.

**A5. `vibe registry test` не зондирует.** SSH-адрес несуществующей организации
— `reachable (auth=none)`, при том что `--help` определяет `reachable` как «org
URL responded» и держит наготове `unreachable`. Противоречие `--help` и
поведения.

**A6. `vibe registry add` принимает каталог как URL и разбирает хост как букву
диска.** `C:\…\work\registry` → `host: C`, `org: \Users\olegc\…\registry`,
`adapter: none`. Команда сообщает `adapter: none — vibe registry publish won't
dispatch here` и всё равно регистрирует реестр как primary.

**A7. `vibe registry publish --dry-run` требует publish-токен.** Сухой прогон
падает на загрузке токена прежде, чем напечатать план.

**A8. `vibe scrape` на чистом проекте: нет дефолтного контракта; `--output` не
нормализует `..`; план блокируется 23 находками `modified-policy-refusal`.**
Вероятная причина третьего — проект не под git, поэтому у контракта нет
провенанса и всё содержимое «unknown». Ни `--help`, ни страница об этом не
говорят.

**A9. `vibe deploy --profile` не документирован в `vibe deploy --help`**, хотя
принимается и порождает осмысленную доменную ошибку.

**A10. `vibe bin list` не имеет `--path`; `vibe bin exec` тот же флаг молча
проглатывает.** Первое даёт exit 2 на ассерте страницы, второе маскирует
опечатку пользователя. Плюс расхождение внутри продукта: `vibe check` видит
`[[binary]]` пакета, `vibe bin *` — нет (они читают только локфайл).

**A11. `--quiet` не сокращает вывод у части команд.** Флаг документирован как
«Reduce output to a single summary line», но `vibe registry list --quiet`
печатает полный листинг реестров, `vibe mcp status` (в ассерте без `--quiet`) —
14 строк. Для `vibe check --quiet` и `vibe list --quiet` флаг работает.

**A12. Windows MAX_PATH: длинные имена пакетов и глубокие пути ломают и клон, и
материализацию.** `git clone … org.vibevm.world.multi-user-planning …` →
`fatal: '$GIT_DIR' too big`; материализация того же `wal` в песочницу с длинным
именем → `error: I/O error on …\vibedeps\org.vibevm.world.wal\1.0.0\vibevm\
vibespecs\skills\wal-status\SKILL.md: The system cannot find the path specified.
(os error 3)`. `core.longpaths` продукт не выставляет. Для кампании это ещё и
эксплуатационное ограничение: имя песочницы входит в длину пути.

**A13. `boot_snippet.category` не знает ни `feat`, ни `lang`, ни `mcp`.**
Допустимо `foundation, flow, stack, tool, app, user-override`, при том что
`package.kind` — `flow, feat, stack, tool, mcp, lang`. Два словаря
рассинхронизированы, и автор пакета `feat` обязан выбрать чужую категорию
(в прогонах выбран `app`, для `lang` — `foundation`).

**A14. Схема манифеста открывается только через ошибки разбора.** Ключи
`[requires] capabilities` / `[provides] capabilities`, поля
`[[binary]] name|crate|description`, поля `[[deploy.target]]
id|artifact|mechanism|provider|when|depends_on|config` не названы ни в одном
`--help`; единственный способ их узнать — скормить `vibe check` заведомо
неверный манифест и прочитать список «expected one of …». Это работает и даже
удобно, но страницы `authoring/*` на это полагаться не могут.

**A15. `vibe list` и `vibe tree` расходятся о загрузочном сниппете.**
`vibe list` печатает `BOOT SNIPPET  —` для `org.vibevm.world/wal`, `vibe tree
--plain` — `dynamic`. `outcome` страницы `howto/install-a-package` ссылается на
второе.

**Следа на диске вне песочниц не найдено.** Паник продукта не наблюдалось: все
отказы — нормальные `error:` с кодом выхода 1 или 2. `git status --short`
хоста после прогонов содержит только мои два новых пути
(`findings/WORKER-REPORT-PP-O1.md`, `findings/PP-O1-runs/`) и правки сторонней
сессии, которые были до меня или появились параллельно (§5).

## 5. Что не сделано и почему

1. **`install-vibe` не прогонялся** — по пакету: установка меняет машину.
   Выполнены только его ассерты, на машинном `vibe` из PATH
   (`C:\Users\olegc\.vibe\opt\bin\vibe`): `vibe --version` → `vibe 1.0.0`
   (0), `vibe self doctor` → `all good.` (0). `vibe self doctor` запущен
   **без** `--fix` (его собственный `--help` говорит, что изменения применяет
   именно `--fix`), и отдельный трипвайр непосредственно вокруг этих двух
   команд побайтово одинаков. Это единственное место, где пришлось выбирать
   между запретом пакета на `vibe self …` и его же указанием «ассерты выполнить
   на этой машине как есть»; выбран read-only вариант с доказательством.
2. **`read-docs-locally`, `write-documentation`, `translate-documentation` не
   выполнялись** — ждут фазы 2 (`vibe doc`, вид `doc`, `[translates]`).
   Зафиксированы прямые доказательства отсутствия, §3.
3. **`vibe mcp install` выполнен только с `--dry-run`.** Настоящие агентские
   конфиги не тронуты: рекурсивный листинг `~/.claude*`, `~/.codex*`,
   `~/.gemini*`, `~/.cursor*` снят до и после; единственные различия — файлы
   самих живых харнессов (транскрипт этой сессии, ротация `~/.claude.json`
   самим Claude Code, sqlite-WAL'ы Codex). `~/.claude/skills/vibevm/SKILL.md`
   не изменился ни по размеру, ни по mtime. Поэтому результат этого прогона
   помечен «условный»: оба его ассерта — read-only отчёты и не различают
   «установлено» и «было бы установлено».
4. **Фикстура `hello-vibe` строилась установкой без `--offline`** — рецепт
   `EXAMPLES-TODO.md` не работает (аномалия A1). Отклонение записано в каждом
   затронутом `agent-log.md`.
5. **Фикстура `hello-vibe` для `give-your-agent-the-skill` не достроилась
   вовсе** — MAX_PATH (A12) свалил и сетевой клон, и установку из in-tree
   реестра хоста. Прогон выполнен на пустом, но валидном проекте; ассерты от
   пакета не зависят, но `vibe skill list` из-за этого печатает «no skills
   declared» вместо `wal-status`.
6. **`org.vibevm.world/multi-user-planning` прогрет отклонением**, разрешённым
   пакетом: `vibe cache add --offline … --path <корень хоста>` (A12 не даёт
   взять его из сети в эту песочницу).
7. **Профиль `local` для `build-package-deploy` не объявлен** — по пакету это
   находка про промпт, а не повод читать спеку. Шесть попыток и то, чему учит
   каждая ошибка, — в §3 и в `agent-log.md`.
8. **Ни один текст страниц дальше блока `<prompt>` не читался.** Промпты
   извлечены скриптом, который вырезает только `<prompt …>…</prompt>`. Одно
   исключение по необходимости: XML-диалект спеков (`<spec xmlns=
   "https://vibevm.org/spec/1">`, `<title id="root">`, `<status>`, `<facts>`)
   и форма манифеста взяты из **установленного пакета**
   `org.vibevm.world/wal` и из того, что генерирует `vibe init package` — то
   есть из источников, доступных и агенту пользователя, а не со страниц
   документации.
9. **Дрейф дерева.** Бинарник пересобран сторонней сессией в середине прогона,
   HEAD сдвинулся дважды, четыре промпта отредактированы (§1, §2). Два из них
   перепрогнаны по новому тексту; остальные прогоны зафиксированы на том
   тексте, который лежит в `PP-O1-runs/<id>/prompt.txt`, и он сверен с текущим
   состоянием страниц на выходе.
10. **Трипвайр не пустой** — 84 строки, целиком внутри
    `~/.vibe/cache/org.vibevm.world/multi-user-planning/**` и
    `~/.vibe/steward/contexts/d2ab5a82-…/**`, двумя односекундными вспышками в
    01:35 и 01:36. Атрибуция и доказательство того, что это не мои команды, —
    в §1; полный diff — `PP-O1-runs/_tripwire-diff.txt`.
11. **Дефект пакета.** Пакет называет для чтения
    `vibevm/vibespecs/skills/vibevm/SKILL.md` «в дереве хоста» — такого файла
    в дереве нет (в репозитории есть только `…/vibevm-docs/v0.1.0/vibevm/
    vibespecs/skills/vibevm-docs/SKILL.md`). Скилл `vibevm` генерируется
    `vibe mcp install` и на этой машине лежит в
    `C:\Users\olegc\.claude\skills\vibevm\SKILL.md` (285 строк); прочитан
    именно он — это буквально «скилл, который получает агент пользователя».
    Файл только читался.
