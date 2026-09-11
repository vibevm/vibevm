# WORKER-REPORT-PP-C2 — снятие `expect` примеров (P.3)

Пакет: `campaigns/docs-2026-09/findings/PACKET-PP-C2.md`. Результат:
`campaigns/docs-2026-09/findings/PP-C2-expects/` (эта директория). Построение
фикстур — `PP-C2-expects/fixtures.md`.

**Итог одной строкой: снято 15 примеров из 56 положенных к снятию (59 в очереди
минус 3 «не сейчас»); 41 не снят, потому что фикстура `hello-vibe` не строится
ни офлайн, ни онлайн в этой песочнице (см. §3, §5a, §5b) — а от неё транзитивно
зависит почти вся очередь. Трипвайр `%USERPROFILE%\.vibe` пуст (before/after
побайтово идентичны, 41751 файл).**

## 1. Бинарник

- `vibe --version` (в изолированном окружении, фикстура `empty`): `vibe 1.0.0`
- mtime `target/debug/vibe.exe`: 2026-09-12 00:50 (локальное время хоста)
- `git rev-parse HEAD` (корень хоста): `5cb6dc1035caad4ccc2533f1c47213441da9b786`
- Бинарник не пересобирался.

## 2. Таблица примеров

Байты — после нормализации (§ пакета «Нормализация»). «—» — код/байты не
применимы (пример не запускался). Полные пути к `.out`/`.err`/`.exit` —
`PP-C2-expects/<страница>--<id>.*`.

| Страница | id | Фикстура | Код выхода | stdout, B | stderr, B | Примечание |
|---|---|---|---|---|---|---|
| agent/ask-your-agent | agentic-explain | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| agent/ask-your-agent | command | hello-vibe-relay | — | — | — | не снято: фикстура = skipped (родитель failed) |
| agent/give-your-agent-the-skill | mcp-status | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| agent/give-your-agent-the-skill | mcp-install | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| agent/give-your-agent-the-skill | skill-list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| architecture/traceability | explain | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| architecture/traceability | select | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| authoring/ship-tools-and-mcp-servers | bin-list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| authoring/specs-agents-can-cite | explain | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| authoring/specs-agents-can-cite | convert | package-spec | 0 | 878 | 0 | снято |
| authoring/write-a-flow | init-package | workspace-root | 0 | 1375 | 0 | снято |
| authoring/write-a-flow | manifest | workspace | 0 | 85 | 0 | снято; см. §5d (`[project]`, не `[package]`) |
| authoring/write-a-flow | check | workspace | 0 | 84 | 0 | снято |
| authoring/write-a-lang-package | init-lang | workspace-root | 0 | 1345 | 0 | снято |
| howto/install-a-package | install | hello-vibe-empty | 1 | 28 | 833 | снято; сеть, упало — см. §5b |
| howto/install-a-package | list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/install-a-package | tree | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/publish-a-package | publish-dry-run | package-notes | 1 | 224 | 319 | снято; упало — см. §5c |
| howto/publish-a-package | publish | package-notes | 1 | 224 | 319 | снято; упало — см. §5c |
| howto/read-documentation-locally | cache-add-docs | empty | — | — | — | не сейчас (A2.1, по EXAMPLES-TODO.md) |
| howto/read-documentation-locally | doc-serve | docs-store | — | — | — | не сейчас (A2.20; docs-store не строится) |
| howto/remove-a-package | uninstall | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/remove-a-package | tree | hello-vibe-removed | — | — | — | не снято: фикстура = skipped (родитель failed) |
| howto/set-up-a-workspace | workspace-table | none | 1 | 0 | 42 | снято; пусто (stdout), упало — см. §4 |
| howto/set-up-a-workspace | init-package | workspace-root | 0 | 1355 | 0 | снято |
| howto/set-up-a-workspace | install | workspace | 0 | 150 | 0 | снято |
| howto/update-packages | outdated | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/update-packages | update | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/use-a-private-registry | registry-add | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/use-a-private-registry | registry-test | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/work-offline | cache-add | empty | 1 | 0 | 867 | снято; сеть, упало — см. §5b |
| howto/work-offline | cache-list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/work-offline | cache-check | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| howto/work-offline | install-offline | hello-vibe-empty | 1 | 28 | 541 | снято; упало — см. §5a |
| lifecycle/build-package-deploy | package | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| lifecycle/build-package-deploy | deploy-plan | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| lifecycle/build-package-deploy | deployments | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| lifecycle/extensions-and-providers | extensions | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| lifecycle/extensions-and-providers | tools | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| lifecycle/phases | deploy-plan | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| lifecycle/scrape | scrape-plan | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| lifecycle/scrape | scrape-output | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| model/boot-lane | tree | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| model/lock-and-store | cache-path | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| model/packages-and-kinds | list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| model/registries | registry-list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| model/two-trees | list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| model/versions | outdated | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| reference/machine-formats | list-json | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| reference/settings-and-environment | vars | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| start/first-project | init | empty | 0 | 610 | 0 | снято |
| start/first-project | install | hello-vibe-empty | 1 | 28 | 833 | снято; сеть, упало — см. §5b |
| start/first-project | list | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| start/first-project | tree | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| start/first-project | check | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| start/install-vibe | windows-install | none | — | — | — | не сейчас (фаза 5, по EXAMPLES-TODO.md) |
| start/install-vibe | version | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| start/what-a-project-contains | tree | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |
| start/what-vibevm-is | version | hello-vibe | — | — | — | не снято: фикстура `hello-vibe` = failed |

Итого: 15 снято, 41 не снято из-за фикстуры, 3 не сейчас (по EXAMPLES-TODO.md) — 59 всего.

## 3. Фикстуры, которые не построились

Полная последовательность и коды выхода всех фикстур — `fixtures.md`. Здесь —
только провал:

**`hello-vibe`** (родитель `hello-vibe-empty`), шаг
`vibe install org.vibevm.world/wal --path hello-vibe --assume-yes --offline`
→ exit 1:

```
stdout: Resolving 1 root package…

stderr: error: `org.vibevm.world/wal@latest` is not resolvable offline — it is in no
local (`file://`) registry and not in the machine store (violates
spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#offline; fix: run the
install once online to warm the store, pre-warm it with `vibe cache add
org.vibevm.world/wal`, or export a `file://` mirror with `vibe registry vendor`)
(violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator;
fix: act on the underlying provider failure named in the message)
```

Ниже по правилу пакета линия остановлена без обходного пути; разбор причины —
§5a. Как следствие, `hello-vibe-relay` и `hello-vibe-removed` (оба — потомки
`hello-vibe`) тоже не построены (`skipped`, не `failed`: сами их шаги не
запускались).

Остальные 7 фикстур (`none`, `empty`, `hello-vibe-empty`, `workspace-root`,
`workspace`, `package-spec`, `package-notes`) построены штатно, `docs-store`
намеренно не строилась.

## 4. Отклонения от пакета и их причины

- **`howto/set-up-a-workspace--workspace-table`** (`cat vibe.toml`, фикстура
  `none`) выполнен буквально, как написано в очереди `EXAMPLES-TODO.md`, хотя
  фикстура `none` по своему определению — пустые `home/`+`work/` без всякого
  `vibe.toml`. Результат — `cat: vibe.toml: No such file or directory`, код 1.
  Я не заменил фикстуру на предположительно верную (текст на странице и сама
  таблица фикстур говорят, что содержимое `[workspace]`-таблицы появляется
  только в `workspace-root`), это было бы «починкой» примера, что пакет прямо
  запрещает. Похоже на опечатку в `EXAMPLES-TODO.md` (фикстура `workspace-root`
  вместо `none` для этой строки) — на усмотрение центральной сессии.
- Для объяснения провала `hello-vibe` (обязательное по структуре отчёта, §5)
  я прочитал `hello-vibe-empty/vibe.toml`, `home/registry.toml` и вывод
  `vibe registry list`/листинг `home/cache` **на отдельной одноразовой копии**
  фикстуры `hello-vibe-empty` (каталог `…/scratchpad/PP-C2-diag/`, вне
  `PP-C2/`, ничего не пишет в каталог результатов), и повторно выполнил ровно
  ту же команду `vibe cache add --offline org.vibevm.world/wal --path <хост>`
  на одноразовой копии `none` (каталог `…/scratchpad/PP-C2-diag-none/`), чтобы
  увидеть её реальный stdout (в фикстуре он не логировался построчно, раз шаг
  не упал). Обе копии — вне `PP-C2/` и не входят в результат; сделаны тем же
  изолированным окружением, тем же бинарником, без сети и без `git`. Это
  выходит за пределы буквального «выполнить очередь и фикстуры», но без этого
  §5a/§5b было бы нечем подкрепить.
- Значения `<TMP>`/`<HOME>`/`<REPO>` подставлялись в обоих написаниях слэшей
  и регистронезависимо (диск `C:` иногда приходит как `c:`); пакет требует
  ровно это, отклонения нет, отмечаю только потому, что это не буквально
  описанный алгоритм, а его реализация.
- CRLF ни разу не встретился в захваченных потоках (проверено побайтово по
  каждому примеру) — специально оговорённое в пакете отклонение не
  применилось.
- Тайминг: на каждую команду стоял таймаут 120 с (в пакете значение не
  задано). Ни одна команда его не приблизила — самая долгая, сетевая
  установка, заняла 3.4 с.

## 5. Аномалии продукта

Список, без правок с моей стороны.

**a. `--offline` install не видит пакет, который сама же `cache add --offline`
только что положила в machine store.** Фикстура `hello-vibe` строится ровно по
рецепту `EXAMPLES-TODO.md` (`empty` → `cache add --offline …
--path <корень хоста>`, затем `hello-vibe-empty` → `hello-vibe` через
`vibe install … --offline`) и стабильно падает на втором шаге. Разбор:
`vibe cache add --offline org.vibevm.world/wal --path <корень хоста>`
действительно кладёт `org.vibevm.world/wal@1.0.0` в `<VIBE_SETTINGS>/cache`
(проверено листингом каталога на одноразовой копии, §4) — используя
project-local реестр хоста (`vibevm/vibepacks`, доступный только через
`--path <корень хоста>`). Но `hello-vibe` — обычный `vibe init`-проект без
единой `[[registry]]`-записи в своём `vibe.toml`
(`vibe registry list --path hello-vibe` буквально отвечает `No [[registry]]
entries in vibe.toml`); он резолвит зависимости только через два реестра,
которые сама программа при первом запуске молча пишет в изолированный
`registry.toml` (`vibespecs` → github.com/vibespecs, `vibespecs-gitverse` →
gitverse.ru/vibespecs) — а их клон (`VIBE_REGISTRY_CACHE`) никогда не
запрашивался, потому что прогрев шёл через реестр хоста, не через эти два. Раз
`org.vibevm.world/wal` в команде `install` не закреплён версией, резолверу
сначала нужно офлайн узнать, что такое «latest» именно в *этих* двух реестрах —
а это ровно то, чего офлайн сделать нельзя, даже когда точная версия уже лежит
в сторе. Сообщение об ошибке («not resolvable offline … not in the machine
store») в этом смысле вводит в заблуждение: пакет **в сторе есть**, просто под
версией, о которой эти два реестра ничего не знают без сети.
Это воспроизводится на каждой попытке построить `hello-vibe` по нынешнему
рецепту `EXAMPLES-TODO.md` — то есть блокирует не только эту фикстуру, но и
все 41 строки очереди, которые от неё зависят (см. таблицу §2).

**b. Неофлайновая установка того же пакета падает с `git`-ошибкой `'$GIT_DIR'
too big`, а не по сети.** `vibe install org.vibevm.world/wal --path hello-vibe
--assume-yes` (без `--offline`, фикстуры `howto/install-a-package--install`,
`start/first-project--install`) и `vibe cache add org.vibevm.world/wal` (без
`--offline`/`--path`, `howto/work-offline--cache-add`) все три падают за
3.0–3.4 с с одинаковым сообщением:
  ```
  error: git operation failed (…): git `git clone --recurse-submodules --no-checkout
  --no-tags -- https://github.com/vibespecs/org.vibevm.world.wal.git
  <TMP>/home/registries/4f0def76f4089d61/packages/org.vibevm.world.wal/clone` exited
  with status 128 (…):
  Cloning into '<TMP>/home/registries/4f0def76f4089d61/packages/org.vibevm.world.wal/clone'...
  fatal: '$GIT_DIR' too big
  fatal: remote helper 'https' aborted session (…)
  ```
  `fatal: '$GIT_DIR' too big` — не сетевая ошибка (не «could not resolve host»,
  не таймаут, не отказ TLS), а внутренняя проверка длины пути у Git для
  Windows/MSYS. Рабочая гипотеза: путь назначения клона уже ~230 символов
  внутри предписанной пакетом песочницы
  (`…\scratchpad\PP-C2\runs\<страница>--<id>\home\registries\<hash>\packages\
  org.vibevm.world.wal\clone`), и добавление `.git/...` поверх него пробивает
  внутренний буфер Git. Я не подтвердил это прямым экспериментом (клонировать
  тем же `git` в короткий путь я не могу — пакет разрешает мне только `git
  status --short` и `git rev-parse HEAD`), поэтому это гипотеза, а не факт;
  но сообщение специфично именно для пути/буфера, а не для сети или прав, что
  и заставляет отнести падение к «сеть» лишь предположительно (в таблице §2
  помечено «сеть, упало» ровно в этом смысле — попытка сети была, кончилась
  локальной git-ошибкой).

**c. `vibe registry publish … --dry-run` на локальный каталог-реестр требует
publish-токен для хоста `` `C` ``.** Оба `howto/publish-a-package` примера
(`--dry-run` без реестра и `--registry local --dry-run`, фикстура
`package-notes`, реестр — обычный пустой каталог Windows-пути) дают
побайтово одинаковую (кроме самой команды) ошибку:
  ```
  error: loading publish token: publish refused: no token available for host `C`. Set
  `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibe/git.publish.token`. (…)
  ```
  Хост `` `C` `` — это буква диска из `C:/Users/.../work/registry`, а не
  осмысленное имя хоста: похоже, что код, извлекающий «host» для поиска
  токена, наивно режет реестровый URL/путь и на пути Windows-диска берёт
  `C:` за `scheme://host`. Отдельно от идентичности бага: `--dry-run` всё
  равно требует токен ещё до какой-либо попытки записи — то есть сухой прогон
  публикации в локальный каталог-реестр сегодня в принципе не проходит дальше
  этой проверки.

**d. `vibe init package <coord> <path>` не кладёт `[package]`-таблицу прямо в
`<path>/vibe.toml`.** Пример `authoring/write-a-flow--manifest`
(`cat packages/review-notes/vibe.toml`, фикстура `workspace`) снят и даёт:
  ```
  [project]
  name = "review-notes"
  version = "0.0.1"
  authors = ["vibevm docs fixtures"]
  ```
  Это `[project]`, не `[package]`. Страница `howto/set-up-a-workspace.xml`
  («What happens») утверждает обратное: «creates each member with `vibe init
  package org.acme/notes-flow packages/notes-flow`, which writes a manifest
  with a `[package]` table» — и сам пример `manifest` явно читает файл именно
  по этому пути, то есть ожидает увидеть там `[package]`. По факту
  `vibe init package` создаёт по `<path>` полноценный vibevm-проект
  (`vibe.toml`+`vibe.lock`+`.vibe/`+boot-лейн+`CLAUDE.md`/`AGENTS.md`/
  `GEMINI.md`), а настоящую `[package]`-таблицу кладёт тремя уровнями глубже:
  `packages/review-notes/vibevm/vibepacks/org.acme/review-notes/v0.1.0/vibe.toml`
  (проверено — она там есть, с `group`/`name`/`kind`/`version`). Не берусь
  судить, что здесь неверно — текст страницы и путь примера, или раскладка
  CLI, — фиксирую только сам факт расхождения, ровно то, что этот пример
  должен показать читателю.

## 6. Что не сделано и почему

- **41 строка очереди** не снята, потому что их фикстура (`hello-vibe`,
  `hello-vibe-relay` или `hello-vibe-removed`) не построилась — см. §3, §5a,
  §5b. Полный список — колонка «Примечание» в таблице §2.
- **`howto/read-documentation-locally--cache-add-docs`** и **`--doc-serve`** —
  не сняты по собственному статусу «не сейчас» в `EXAMPLES-TODO.md` (A2.1 —
  вид `doc` ещё не реализован в отладочном бинарнике; A2.20 — нужна фикстура
  `docs-store`, которую этот пакет прямо запрещает строить).
- **`start/install-vibe--windows-install`** — не снят по собственному статусу
  «не сейчас» (фаза 5, снимается с дистрибутива релиза, не с отладочной
  сборки); также это единственный пример на `powershell`, и я его не запускал
  вовсе.
- Фикстура **`docs-store`** не строилась — по прямому указанию пакета.
- Ни одна команда очереди не попала под список защищаемых команд пакета
  (`vibe self …`, `mcp install`/`registry publish`/`deploy` без защитного
  флага, `cache clean`) — все нужные флаги (`--dry-run`, `--plan`) уже стояли
  в самих примерах, пропускать по этому правилу было нечего.
- Страницы `.xml` не редактировались, `EXAMPLES-TODO.md` не редактировался —
  только каталог результатов и этот отчёт.

## Трипвайр и git

- `tripwire-before.txt` / `tripwire-after.txt`: побайтово идентичны (41751
  файлов, 6 500 074 байта каждый; `diff` пуст).
- `host-status-before.txt` / `host-status-after.txt` (вокруг шага `cache add
  --offline … --path <корень хоста>` фикстуры `empty`): побайтово идентичны —
  этот шаг не тронул дерево хоста.
- `%USERPROFILE%\.vibe` не читался и не писался ничем, кроме этого листинга
  метаданных (путь/размер/mtime); содержимое файлов не открывалось.
- Git из этого воркера: только `git status --short` и `git rev-parse HEAD` в
  корне хоста, оба — дважды (до/после шага `empty`). Никаких `add`/`commit`/
  `checkout`/`stash`.
