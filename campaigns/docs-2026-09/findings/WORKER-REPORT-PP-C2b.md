# WORKER-REPORT-PP-C2b — повторное снятие `expect` примеров, вторая редакция фикстур (P.3)

Пакет: `campaigns/docs-2026-09/findings/PACKET-PP-C2b.md` (переопределяет часть
`PACKET-PP-C2.md`, который тоже действует в непереопределённой части).
Результат: `campaigns/docs-2026-09/findings/PP-C2-expects/` (эта директория —
очищена целиком и переснята заново единым прогоном). Построение фикстур —
`PP-C2-expects/fixtures.md`.

**Итог одной строкой: снято 54 примера из 54 положенных к снятию по второй
редакции (59 строк очереди минус 5 «не сейчас»); все 12 строящихся фикстур
(кроме намеренно не строившейся `docs-store`) построились с первой попытки
под локальным реестром — фикстура `hello-vibe`, блокировавшая 41 из 59 строк
в первом прогоне, больше не падает. Трипвайр `%USERPROFILE%\.vibe` пуст
(41775 файлов, before/after побайтово идентичны).**

## 1. Бинарник

- `vibe --version` (в изолированном окружении, фикстура `hello-vibe`): `vibe 1.0.0`
- mtime `target/debug/vibe.exe`: `2026-09-12 01:59:23 +0300`, 104571904 байт
- `git rev-parse HEAD` (корень хоста) на протяжении всего снятия (все четыре
  прогона, §4): `cae9381e6b640dcbec4ebe16c43e304214276fe6`. К моменту записи
  этого отчёта, уже после того как снятие полностью завершилось, `HEAD`
  сдвинулся ещё раз, на `47234b8e5191aa45d0be67c68325b75cc91b5b32`, — ещё
  одним чужим коммитом в этом же общем дереве, не моим (я не выполнял
  `add`/`commit`). Само снятие (54+12 команд) на это уже не влияет — оно
  целиком прошло на `cae9381e…` — но следующая сессия на этом хосте должна
  сверить `HEAD` заново, а не полагаться на любое из двух значений здесь.
- Бинарник не пересобирался мной, `cargo` я не запускал.

  Хост — общий git-worktree, в котором параллельно работали другие сессии: за
  время этой задачи `target/debug/vibe.exe` трижды менял mtime/размер
  (01:37 → 01:52 → 01:59; 104571904 ↔ 104577536 байт) из-за чужих правок в
  `crates/vibe-cli/src/commands/show/source_path.rs`,
  `crates/vibe-install/src/plan/fetch.rs`, `xtask/src/bridge.rs` — не моих, я
  эти файлы не трогал и не запускал ничего, что могло бы их пересобрать.
  Итоговый прогон, чьи данные лежат в `PP-C2-expects/`, я проверил на
  стабильность mtime+размера непосредственно до запуска и сразу после — оба
  раза `104571904` байт / `01:59:23` — то есть сами эти 54+12 команд
  выполнялись на одной сборке. Но дерево хоста не было в покое всю сессию, и
  следующий прогон на этом хосте должен заново проверить mtime, а не
  полагаться на значение из этого отчёта.

## 2. Таблица примеров

Байты — после нормализации (см. §4 ниже и правило «Нормализация» пакета).
Полные пути к `.out`/`.err`/`.exit` — `PP-C2-expects/<страница>--<id>.*`.

| Страница | id | Фикстура | Код выхода | stdout, B | stderr, B | Примечание |
|---|---|---|---|---|---|---|
| agent/ask-your-agent | agentic-explain | hello-vibe | 0 | 68 | 0 | снято |
| agent/ask-your-agent | command | hello-vibe-relay | 0 | 813 | 0 | снято |
| agent/give-your-agent-the-skill | mcp-status | hello-vibe | 0 | 1484 | 0 | снято |
| agent/give-your-agent-the-skill | mcp-install | hello-vibe | 0 | 615 | 0 | снято |
| agent/give-your-agent-the-skill | skill-list | hello-vibe | 0 | 187 | 0 | снято |
| architecture/traceability | explain | hello-vibe | 1 | 0 | 99 | снято; упало — см. §5d |
| architecture/traceability | select | hello-vibe | 0 | 110 | 0 | снято; «0 results» — см. §5d |
| authoring/ship-tools-and-mcp-servers | bin-list | hello-vibe | 2 | 0 | 111 | снято; упало — см. §5a |
| authoring/specs-agents-can-cite | explain | hello-vibe | 1 | 0 | 99 | снято; упало — см. §5d |
| authoring/specs-agents-can-cite | convert | package-spec | 0 | 513 | 0 | снято; см. §5e |
| authoring/write-a-flow | init-package | project | 0 | 728 | 0 | снято |
| authoring/write-a-flow | manifest | flow-slot | 0 | 291 | 0 | снято |
| authoring/write-a-flow | check | flow-slot | 0 | 108 | 0 | снято |
| authoring/write-a-lang-package | init-lang | project | 0 | 710 | 0 | снято |
| howto/install-a-package | install | hello-vibe-empty | 0 | 298 | 0 | снято |
| howto/install-a-package | list | hello-vibe | 0 | 98 | 0 | снято |
| howto/install-a-package | tree | hello-vibe | 0 | 169 | 0 | снято |
| howto/remove-a-package | uninstall | hello-vibe | 0 | 261 | 0 | снято |
| howto/remove-a-package | tree | hello-vibe-removed | 0 | 130 | 0 | снято |
| howto/set-up-a-workspace | workspace-table | workspace-root | 0 | 115 | 0 | снято |
| howto/set-up-a-workspace | member-manifest | workspace | 0 | 181 | 0 | снято |
| howto/set-up-a-workspace | install | workspace | 0 | 150 | 0 | снято |
| howto/update-packages | outdated | hello-vibe | 1 | 0 | 128 | снято; упало — см. §5b |
| howto/update-packages | update | hello-vibe | 1 | 0 | 121 | снято; упало — см. §5b |
| howto/use-a-private-registry | registry-add | hello-vibe | 1 | 0 | 378 | снято; упало — см. §5c |
| howto/use-a-private-registry | registry-test | hello-vibe | 1 | 0 | 98 | снято; упало (следствие §5c) |
| howto/work-offline | cache-add | empty | 0 | 164 | 0 | снято |
| howto/work-offline | cache-list | hello-vibe | 0 | 94 | 0 | снято |
| howto/work-offline | cache-check | hello-vibe | 0 | 91 | 0 | снято |
| howto/work-offline | install-offline | hello-vibe-empty | 0 | 298 | 0 | снято |
| lifecycle/build-package-deploy | package | hello-vibe | 0 | 651 | 0 | снято |
| lifecycle/build-package-deploy | deploy-plan | hello-vibe | 0 | 58 | 0 | снято |
| lifecycle/build-package-deploy | deployments | hello-vibe | 0 | 40 | 0 | снято |
| lifecycle/extensions-and-providers | extensions | hello-vibe | 0 | 217 | 0 | снято |
| lifecycle/extensions-and-providers | tools | hello-vibe | 2 | 0 | 108 | снято; упало — см. §5a |
| lifecycle/phases | deploy-plan | hello-vibe | 0 | 58 | 0 | снято |
| lifecycle/scrape | scrape-plan | hello-vibe | 1 | 0 | 91 | снято; ожидаемо (нет `contract.toml`) |
| lifecycle/scrape | scrape-output | hello-vibe | 1 | 0 | 91 | снято; ожидаемо (нет `contract.toml`) |
| model/boot-lane | tree | hello-vibe | 0 | 169 | 0 | снято |
| model/lock-and-store | cache-path | hello-vibe | 0 | 17 | 0 | снято |
| model/packages-and-kinds | list | hello-vibe | 0 | 98 | 0 | снято |
| model/registries | registry-list | hello-vibe | 0 | 145 | 0 | снято |
| model/two-trees | list | hello-vibe | 0 | 98 | 0 | снято |
| model/versions | outdated | hello-vibe | 1 | 0 | 128 | снято; упало — см. §5b |
| reference/machine-formats | list-json | hello-vibe | 0 | 502 | 0 | снято |
| reference/settings-and-environment | vars | hello-vibe | 0 | 105 | 0 | снято |
| start/first-project | init | empty | 0 | 610 | 0 | снято |
| start/first-project | install | hello-vibe-empty | 0 | 298 | 0 | снято |
| start/first-project | list | hello-vibe | 0 | 98 | 0 | снято |
| start/first-project | tree | hello-vibe | 0 | 169 | 0 | снято |
| start/first-project | check | hello-vibe | 0 | 73 | 0 | снято |
| start/install-vibe | version | hello-vibe | 0 | 11 | 0 | снято (было «вставлено вручную» — подтверждено) |
| start/what-a-project-contains | tree | hello-vibe | 0 | 169 | 0 | снято |
| start/what-vibevm-is | version | hello-vibe | 0 | 11 | 0 | снято (было «вставлено вручную» — подтверждено) |

Итого: **54 снято, 0 не снято, 5 не сейчас** (по EXAMPLES-TODO.md) — 59 всего.

## 3. Фикстуры, которые не построились

Ни одна. Все 12 строящихся фикстур — `none`, `empty`, `hello-vibe-empty`,
`hello-vibe`, `hello-vibe-relay`, `hello-vibe-removed`, `project`,
`flow-slot`, `package-spec`, `workspace-root`, `workspace`, `package-notes` —
построены с первой попытки под локальным реестром; `docs-store` не строилась
по прямому указанию пакета. Полная последовательность команд и кодов выхода
для каждой фикстуры — `fixtures.md`.

В частности, локальный `file://`-реестр (`home/registry.toml` → `[[registry]]
name = "local" url = "file:///<корень хоста>/vibevm/vibepacks"`, заведённый
до первого `vibe init` каждой линии) снял обе причины провала `hello-vibe` из
первого прогона (WORKER-REPORT-PP-C2 §5a/§5b): `vibe install
org.vibevm.world/wal --path hello-vibe --assume-yes` резолвит `latest` и
материализует пакет без единого обращения к сети. Ни в одном из 54+12
запусков вывод не содержал признаков сетевой активности (`github.com`,
`Cloning into`, `git`-ошибок) и ни один не приблизился к секундным таймингам
сетевого случая из первого отчёта (все — от долей миллисекунды до долей
секунды); правило пакета «если команда всё же пошла в сеть — записать как
аномалию» не сработало ни разу.

## 4. Отклонения от пакета и их причины

- **Тайминг:** таймаут 120 с на команду (в пакете не задан); ни одна команда
  его не приблизила.
- **Список чтения:** «Читать сначала» PACKET-PP-C2b не включает
  `A0.12-runner-mock.py`, §2/§4/§5 `A0.12-example-runner.md` и страницы
  `.xml` (в отличие от PACKET-PP-C2 — но эти пункты этот, второй, пакет не
  называет). Нормализацию и изоляцию я реализовал заново по текстовым
  правилам PACKET-PP-C2 («Нормализация») и A0.12-example-runner.md §1/§3, не
  читая и не переиспользуя код макета.
- **Собственные дефекты раннера, найденные и исправленные по ходу** (это про
  мой скрипт снятия, не про бинарник — раскрываю ради прозрачности результата):
  1. Первая попытка: три файла фикстур, которые пишу я сам
     (`package-spec/.../NOTES-FLOW.md`, дописанный `workspace-root/work/vibe.toml`,
     оба `workspace/work/packages/*/vibe.toml`) были записаны текстовым
     режимом Python без `newline=""`, который на Windows молча превращает
     `\n` → `\r\n` при записи. Это внесло CRLF в 3 из 54 снятий
     (`howto/set-up-a-workspace--workspace-table`, `--member-manifest`,
     `authoring/specs-agents-can-cite--convert`), не имеющий отношения к
     бинарнику. Исправлено записью этих файлов как raw bytes с явным `\n`.
     Заодно подтверждено побайтово, что сам `vibe init` пишет `vibe.toml` в
     LF, не CRLF (see `fixtures.md`, строка `workspace-root`) — то есть CRLF
     в первой попытке был целиком артефактом моего кода, не продукта.
  2. Вторая находка: подстановка `<TMP>`/`<HOME>`/`<REPO>` не учитывала, что
     путь внутри JSON-строки (`vibe list --json`) экранирует обратный слэш
     его удвоением — из-за этого в `reference/machine-formats--list-json`
     настоящий абсолютный путь песочницы протекал в вывод нетронутым и затем
     удваивался в слэши шагом «slashes»; отдельно, порядок замены (`<HOME>`
     раньше `<REPO>`) не давал `<REPO>` вообще сработать, поскольку `<REPO>`
     — вложенный путь внутри `<HOME>`, и более короткий `<HOME>` перехватывал
     совпадение первым. Оба дефекта исправлены (схлопывание удвоенных
     обратных слэшей перед подстановкой; порядок замены от самого
     специфичного пути к самому общему: `<TMP>`, затем `<REPO>`, затем
     `<HOME>`).

  После каждого из двух исправлений весь прогон (все 12 фикстур + все 54
  строки) повторён целиком. Бинарник проверен на стабильность mtime+размером
  непосредственно перед каждым повтором и сразу после — 2-й, 3-й и финальный
  (4-й) прогоны все выполнялись на одной и той же сборке (104571904 байт,
  `01:59:23`), так что итоговые данные в `PP-C2-expects/` — из одного прогона
  на одной сборке, как требует пакет, а не смесь до/после исправления кода
  раннера.
- Значения `<TMP>`/`<HOME>`/`<REPO>` подставлялись регистронезависимо и в
  обоих написаниях слэшей — пакет требует ровно это, отклонения нет.
- CRLF после исправления выше не встретился ни в одном из 54 снятий (проверено
  побайтово по каждому примеру).

## 5. Аномалии продукта

Список, без правок с моей стороны. Только новое — §5a–§5d из
WORKER-REPORT-PP-C2 (офлайн-резолв `latest` не видящий прогретый стор,
`git`-клон `'$GIT_DIR' too big`, токен публикации на хосте `` `C` ``) не
повторяю: первые два сценария в этом прогоне физически не воспроизводятся
(локальный реестр не клонирует и не требует офлайн-резолва `latest` через
сеть), а третий (публикация) в этот раз не запускался («не сейчас», B-133).

**a. `vibe bin list` и `vibe tools` отвергают `--path`, хотя очередь именно
так их вызывает.**

```
$ vibe bin list --path hello-vibe
error: unexpected argument '--path' found

Usage: vibe bin list [OPTIONS]
```

```
$ vibe tools --path hello-vibe
error: unexpected argument '--path' found

Usage: vibe tools [OPTIONS]
```

Оба документированных примера очереди (`authoring/ship-tools-and-mcp-servers--bin-list`,
`lifecycle/extensions-and-providers--tools`) не выполнимы как написаны против
этой сборки — код выхода 2 (clap отвергает флаг ещё до попытки резолва
проекта). Большинство других подкоманд (`list`, `tree`, `check`, `install`,
`explain`, `mcp status` и т.д.) принимают `--path` свободно; `bin list` и
`tools`, похоже, его не объявили.

**b. `vibe outdated`/`vibe update` требуют `[[registry]]` уровня проекта и не
видят реестр уровня машины, которого `install`/`cache add` вполне достаточно.**

```
$ vibe outdated --path hello-vibe
error: no registry configured. Add a `[[registry]]` entry to `vibe.toml` or
run `vibe outdated` against a project that has one.

$ vibe update org.vibevm.world/wal --path hello-vibe --assume-yes
error: no `[[registry]]` configured in `<TMP>/work/hello-vibe/vibe.toml` —
`vibe update` re-fetches from the registry.
```

`hello-vibe`, по своему рецепту в EXAMPLES-TODO.md, никогда не получает
`[[registry]]` в собственном `vibe.toml` — он резолвится целиком через
machine-level `home/registry.toml` (наш `local`). `vibe install`/`vibe cache
add` этим реестром пользуются свободно (вся фикстура `hello-vibe` и обе
`install`-строки очереди тому доказательство). `vibe outdated`/`vibe update`
— нет: они требуют реестр именно в `vibe.toml` проекта и отказываются, даже
когда машинный реестр прекрасно резолвит тот же пакет. Разные команды одного
семейства («что знает про registry») расходятся в том, какой уровень
конфигурации они готовы использовать.

**c. `vibe registry add` принимает голый Windows-путь как локальный реестр,
но отказывает точно такой же по смыслу `file://`-ссылке — ровно в примере,
который это иллюстрирует.**

Пример очереди `howto/use-a-private-registry--registry-add`:

```
$ vibe registry add acme file:///tmp/acme-specs --path hello-vibe --position primary
error: registry URL `file:///tmp/acme-specs`: publish refused: cannot derive
an organization segment from `file:///tmp/acme-specs`. Configure
`[[registry]].url` to a value `git` accepts (e.g. `git@gitverse.ru:vibespecs`).
```

exit 1 — сам пример страницы про приватный реестр не проходит. Но шаг
построения фикстуры `package-notes` (тот же пакет, `EXAMPLES-TODO.md`,
таблица «Фикстуры») делает концептуально то же самое — называет локальный
пустой каталог реестром — и проходит:

```
$ vibe registry add local "<абсолютный путь>/work/registry" --path . --position primary
```
→ exit 0 (см. `fixtures.md`, `package-notes`).

Разница — только в форме: голый Windows-путь принимается, `file://`-URL с тем
же самым каталогом отвергается с требованием «организационного сегмента» и
подсказкой, ссылающейся на git-remote (`git@gitverse.ru:vibespecs`) — активно
вводящей в заблуждение того, кто как раз хочет чисто локальный, не-git
реестр. Наш собственный `home/registry.toml` (см. §3) тоже сделан в обход
`registry add`, прямой записью TOML — то есть на сегодня «добавить локальный
файловый реестр через CLI» работает только через одну из двух документируемых
форм, и это не та форма, которую показывает страница.

Как следствие: `howto/use-a-private-registry--registry-test` на свежей копии
`hello-vibe` тоже падает (`error: not-configured: no [[registry]] entries to
probe; add one with vibe registry add first`) — ожидаемо, поскольку каждая
строка очереди выполняется на независимой свежей копии своей фикстуры и не
видит эффекта соседней строки, но по факту это значит, что двухшаговый
рассказ страницы («добавь реестр, затем проверь его») не держится ни на одном
шаге как написано.

**d. `vibe explain`/`vibe select` по иллюстративному URI со страниц дают
ошибку или пустой результат на фикстуре `hello-vibe`.**

```
$ vibe explain "spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET" --path hello-vibe
error: no spec unit with URI `spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET` in the index

$ vibe select --where "uri:spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET depth:1" --path hello-vibe
map select · grammar v1 · uri=spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET AND depth=1
0 results
```

Использовано на двух страницах (`architecture/traceability--explain`,
`authoring/specs-agents-can-cite--explain`) с одинаковым результатом. URI
принадлежит спек-системе самого vibevm-docs, а не тому, что производит голый
`vibe init` + установка `wal`: у `hello-vibe` попросту нет такого узла в
индексе. `explain` в этом случае честно падает (exit 1); `select` тем же URI
формально не падает (exit 0), но возвращает «0 results» — то есть ни один из
двух примеров не показывает читателю ту трассируемость, ради которой,
похоже, задуман раздел, а только ошибку/пустоту.

**e. `vibe refactor convert-source --dry-run` считает лоссовым round-trip
даже двухстрочного md-документа и теряет комментарий boot-сниппета целиком; в
самом комментарии — опечатка `org.org.acme`.**

```
$ vibe refactor convert-source --from md --to xml --dry-run vibevm/vibespecs
dry-run ir-stable-loss vibevm/vibespecs/NOTES-FLOW.md
--- source
+++ reverse-projection
@@ -3,2 +3,3 @@
 @fact:ONE-NOTE One note per review. @status:spec/done

+

dry-run ir-stable-loss vibevm/vibespecs/boot/10-tool-review-notes.md
--- source
+++ reverse-projection
@@ -1,6 +1,5 @@
-<!-- vibe:static org.org.acme/review-notes — boot snippet -->
-
 # review-notes

 A `tool` package.

+

summary converted=0 already=0 lossy-confirmed=0 refused=0 skipped-generated=0 skipped-foreign=0 skipped-harness=0 dry-run=2
```

exit 0 (dry-run отчитывается, не отказывает), но оба документа, которые он
находит, помечены `ir-stable-loss`:

- наш собственный, ровно по рецепту пакета написанный `NOTES-FLOW.md` (один
  заголовок, один абзац, чистый LF, без хвостовых пустых строк) — round-trip
  добавляет одну лишнюю пустую строку в конце.
- настоящий, сгенерированный `vibe init package` файл
  `vibevm/vibespecs/boot/10-tool-review-notes.md` — round-trip целиком
  теряет первую строку, HTML-комментарий `<!-- vibe:static
  org.org.acme/review-notes — boot snippet -->`.

Отдельно от лоссовости: сам этот комментарий, который `vibe init package`
проставляет автоматически, содержит собственную опечатку — `org.org.acme`
(группа `org.acme` задвоена лишним ведущим `org.`). Это не имеет отношения к
конвертации; видно только потому, что `convert-source --dry-run` печатает
исходную строку целиком.

## 6. Что не сделано и почему

- **5 строк очереди** пропущены по их собственному статусу «не сейчас» в
  `EXAMPLES-TODO.md`: `howto/publish-a-package--publish-dry-run` и `--publish`
  (B-133), `howto/read-documentation-locally--cache-add-docs` (A2.1),
  `--doc-serve` (A2.20), `start/install-vibe--windows-install` (фаза 5;
  единственный пример на `powershell` во всей очереди — не запускался вовсе).
- Фикстура **`docs-store`** не строилась — по прямому указанию пакета.
- Ни одна команда очереди не попала под список защищаемых команд пакета
  (`vibe self …`, `mcp install`/`registry publish`/`deploy` без защитного
  флага, `cache clean`) — нужные флаги (`--dry-run`, `--plan`) уже стояли в
  самих примерах, пропускать по этому правилу было нечего.
- Страницы `.xml`, `EXAMPLES-TODO.md` и другие файлы зоны не редактировались
  — только каталог результатов и этот отчёт.
- `A0.12-runner-mock.py` не читался и не запускался (не назван этим пакетом);
  нормализация и изоляция реализованы заново с нуля по текстовым правилам.

## Трипвайр и git

- `tripwire-before.txt` / `tripwire-after.txt`: побайтово идентичны (41775
  файлов, `diff` пуст). Отличие от 41751 файлов в первом отчёте — естественный
  дрейф настоящего `~/.vibe` между сессиями (владелец использует машину между
  задачами); внутри же этой задачи, как и в первой, before и after у самой
  задачи совпадают побайтово.
- `host-status-before.txt` / `host-status-after.txt`: идентичны; `git status
  --short` хоста и до, и после этой задачи показывает только
  `campaigns/docs-2026-09/findings/PP-C2-expects/` (untracked, как и до
  начала этой задачи) и три чужих изменения, уже бывших в дереве до начала
  этой задачи (`crates/vibe-cli/src/commands/show/source_path.rs`,
  `crates/vibe-install/src/plan/fetch.rs`, `xtask/src/bridge.rs` — не мои, не
  трогал, не связаны с этим пакетом). Этот отчёт (`WORKER-REPORT-PP-C2b.md`)
  добавится к статусу как новый untracked-файл после записи.
- `%USERPROFILE%\.vibe` не читался и не писался ничем, кроме сбора метаданных
  для трипвайра (относительный путь/размер/mtime); содержимое файлов не
  открывалось.
- Git из этого воркера: только `git status --short` и `git rev-parse HEAD` в
  корне хоста, несколько раз (для проверки стабильности бинарника и дерева до
  и после каждой попытки прогона, включая два повтора после исправлений
  раннера). Никаких `add`/`commit`/`checkout`/`stash`.
