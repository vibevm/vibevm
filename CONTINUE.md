# CONTINUE — cold-resume snapshot (2026-08-18, wind-down №30)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

**ТЗ change-native форматов доведено до конца главной полосы.** Ф6.2 закрыта
четырьмя шагами; приёмка §10 прогнана целиком и зелена; предсказания P1–P6
сверены **прогоном** — четыре подтверждены, два фальсифицированы и вынесены в
`BACKLOG` (B-081, B-082).

**Но осталось одно, и оно не код: спека лжёт о коде в ШЕСТИ местах.**
Приложение Б.5 ТЗ обязывает каждую фазу сдавать спек-дифф в своей же посадке;
диффы дошли до Ф3 и оборвались. Ф4 и Ф6 не оплачены. Хуже всего то, что две из
шести лжей — это фразы вида «measured 2026-08-05», на которые следующий
читатель обопрётся как на замер и не перепроверит.

Это вход следующей сессии, и это же предусловие §11 — смертности плана:
содержимое обязано переехать в спеки ПРЕЖДЕ, чем план умрёт.

## Где стоит работа

- Ветка `main`, HEAD `55f36630`, дерево чистое, зеркала синхронны
  (gitverse + github).
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`**,
  реальный код выхода 0, **53 шага**.
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых ОЖИДАЕМЫ и не
  двигались более сорока посадок подряд.
- Судейство: **0 неосуждённых, 0 осиротевших**; 34 файла stale. Корпус 281
  файл, 13788 маркеров, 0 неразмеченных.
- Карта: 6061 спек-юнит / **1042** tagged / **975** ребра / 0 подозрений /
  0 сирот; `specmap --check` зелен.
- Воркеров нет, **`.wt/` пуст** — четыре рабочих каталога сессии сняты чисто.
  Два worktree под `~/.fractality/runs/**` — чужие, не трогать.
- Логи, отчёты, патчи и `meta.md` с вердиктами — в
  `C:\Users\olegc\git\v\cache\agents\sorted\{F62A-QUARANTINE-KEEP,F62B-UNAVAILABLE,F62C-PROBE,F62C-BUILD,F62D-LOG-LEVEL,F62-PREDICTIONS}\`.
  **Архив живёт ВНЕ чекаута:** путь `cache/agents/...` в текстах — это
  `git/v/cache/...`, а не подкаталог репозитория.

## Блокер и действие человека

**Блокера нет.** За владельцем восемь вопросов, ни один не блокирует спек-диффы:

- **СТОП-ВЛАДЕЛЕЦ:** **S2** (org-права), **Ж8** (`--full` и запись, которую
  скан больше не видит).
- **Развилки `BACKLOG`:** **B-056** (у JTD нет 64-битного целого — держит
  реэкспорт `Repomd`/`RepomdFileEntry`), **B-078**, и четыре новых —
  **B-079**, **B-080**, **B-081**, **B-082**.

## Главный долг — точные координаты

Все шесть в `spec/modules/vibe-index/PROP-005-package-index.md`. Измерено
грепом с рабочим контролем (0 хитов на `hello.json` при 24 на `repomd`):

| # | якорь / строка | что спека утверждает | почему это ложь |
|---|---|---|---|
| 1 | `##ENTRY-SCHEMA` **:248** | «This section IS the schema … there is no JTD file for the index entry anywhere in the tree … types are hand-written against this text» | `schemas/index/e1/entry.jtd.json` существует (Ф4.1b); типы сгенерированы и реэкспортированы (Ф4.2c); `check-codegen` их сторожит |
| 2 | `##RUST-TYPES` **:552** | «hand-written against §2.6 rather than generated … the compiler checks nothing between them» | неверно с Ф4.2c |
| 3 | `#layout` **:118-120** | дерево корня каталога | не несёт `hello.json`, который `write_to` пишет с Ф6.1c |
| 4 | `#http` | таблица маршрутов | не знает `/v1/index/hello.json`; маршрутов **16**, не 15 |
| 5 | `#optional` **:39** | «Index is OPTIONAL» | обнаружение через хэндшейк с фолбэком не описано вовсе |
| 6 | весь `spec/**` | — | ответ `unavailable` не описан НИГДЕ (греп с контролем) |

## Что сделала эта сессия — по существу

| шаг | коммит | что построено |
|---|---|---|
| Ф6.2a | `5fabcea6` | загрузчик перестал уничтожать карантинную версию |
| Ф6.2b | `fa50b653` | семь глаголов CLI говорят `unavailable` |
| Ф6.2c | `0798614f` | пять поверхностей HTTP называют отказ |
| Ф6.2d | `ce3de248` | флаг и переменная стали одним рычагом |

Плюс решения Р52–Р55 (`5f308eb0`, `a02ccf82`, `72702642`), замер серверной и
клиентской поверхности (`d7aff0ce`), закрытие фазы (`4842ad89`), правка
скрипта приёмки (`3037db77`) и сверка предсказаний (`55f36630`).

## Девять вещей, которые эта сессия установила

**1. Согласие двух поверхностей пришло на шаг раньше — и это НАБЛЮДЕНИЕ.**
Р49 назначала закрытие расхождения работой Ф6.2c. Замер после Ф6.2a показал:
серверные ответы уже отказывают, потому что предикат читает `must_understand`
САМОЙ ЗАПИСИ, а не носитель, который у сервера вечно пуст. Место проверки в
пути ОТВЕТА оказалось достаточным условием. Ф6.2c получила вместо согласия —
речь.

**2. Молчал не тот, на кого думали.** Сырой `by-name/{name}` считался шестой
молчащей поверхностью; он оказался самой честной — отдаёт запись дословно
вместе с объявлением, которое отказ и объясняет. Обязанных заговорить стало
ПЯТЬ, и число получило довод вместо наследования.

**3. Запрет пакета обязан вырезать требование, которое лишь делит с ним
написание.** Дважды за сессию: «не трогай общий дом» породил две копии
предиката совпадения, потом четырёхкратный проход отказа; «не добавляй
`#[spec(...)]`» запретил форму `deviates`, которой conform-гейт требует для
`set_var` в `unsafe`. Оба раза воркер исполнил букву и честно назвал
последствие; оба раза буква была неверной, и чинил её босс хвостом.

**4. Пустой вывод — это утверждение.** Строка приёмки САМОГО ТЗ фильтровала
тесты по слову `hello`, не ловила ни одного и читалась зелёной; вдобавок cargo
берёт один фильтр, а второе слово молча считает частью первого. Поймано тем,
что нулевому выводу скормили контрольный случай.

**5. Панель обрывается на первом красном шаге.** Красный прогон Ф6.2d кончился
на шаге **9 из 53**. Перегон после починки — не перестраховка, а единственный
способ узнать про остальные сорок четыре.

**6. Паника, найденная чтением и подтверждённая прогоном.** `cli/get.rs`
печатал `args.version.unwrap()` в ветке, достижимой БЕЗ `--version`: пакет без
пригодных версий ронял `get`. Найдено боссом при нарезке, воспроизведено
воркером (`exit 101`), удалено по построению.

**7. Один воркер проходит несколько шагов фазы через `-c`, и это дёшево.**
Босс принимает и САЖАЕТ шаг, сбрасывает worktree на посаженный коммит, шлёт
`-c` с новым пакетом. Тёплый `target/` плюс накопленный контекст: три шага
одного воркера — 17.6 → 22.5 → 12.5 минуты.

**8. Когда одна лана держит cargo, вторая получает ПОЛНЫЙ запрет cargo,** а не
совет. Read-only замер Ф6.2c прошёл с нулём cargo, нулём git и нулём
изменённых файлов — и дал находку, которая перевернула нарезку.

**9. Два фальсифицированных предсказания — результат, а не неудача.** P4 и P5
утверждали свойства, которых у дерева нет. Оба нашлись только потому, что
предсказания ПРОГНАЛИ, а не перечитали.

## Что решено НЕ делать, и это решение

**Сырой байтовый маршрут не заставляют «говорить»** — он не молчит, а сделать
его «говорящим» можно только убрав из него сведения. **`unavailable` не
кладут внутрь `VersionEntry`** — суждение читателя на проводе не хранится.
**Статус `404` не меняют** — говорит тело через расширенный член RFC 7807.
**Операционные счётчики остаются писательскими** — они о том, что индекс
ДЕРЖИТ. **Общую серверную фикстуру не трогают** — новые сторожа строят свою.
**Конверты ответа не описывают схемой поштучно** — чеканка формата
владельческий акт (B-079). **Печать суда (`seal`) не поставлена** — восьмое
сворачивание подряд; закрывает S7.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`** — до-эпоховые манифесты под `packages/`.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.

**Рост любого из них — находка.**

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **спек-диффы Ф4 и Ф6**, шесть мест из таблицы
выше; форма — правка спеки в её собственном стиле (`@fact:` + `@status:`),
затем `cargo xtask specmap`, затем **суд тем же заходом** (правленый спек-файл
входит в судимый корпус). После них — §11, смертность плана.

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-index/PROP-005` (**предмет
  ближайшей работы**), `PROP-002`, `PROP-008`, `spec/common/PROP-029`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (**двадцать пять**
  находок; свежие — `f6-2c-server-and-client-surface.md`,
  `f6-predictions-verdicts.md`), `SUBAGENT-LAUNCHERS.md` (§8 — транспортные
  факты; `SUBAGENT-MODE.toml` = `claudez`), `tasks/*.py`, `run/`.
- `formats/` — `REGISTRY.toml` (20 записей), `EPOCHS.toml`,
  `corpora/index/e1/` (журнал + спроецированный каталог, **13 файлов**),
  `vocabularies.json`, `hash_recipes/1.toml`, `breaks/001.md`.
- `schemas/` — семь CLI-отчётов в корне плюс `index/e1/` (пять),
  `journal/e1/` (одна) и `hello/e1/hello.jtd.json`.
- `crates/` — 19 крейтов + `xtask`. Предмет прошедшей фазы:
  `crates/vibe-index/src/index/quarantine.rs` (единственный дом ответа:
  предикат, аксессоры, `Unavailable`, `recipe_for`, `refused_where`),
  `index/memory.rs`, `index/search.rs`, `cli/**` (семь глаголов),
  `server/{error.rs,routes/**}`, `crates/vibe-registry/src/index_client/**`.
- Корень: `BACKLOG.md`, `AUDIT.md`, `TASKS.md`, `NEXT-SESSION-PROMPT.md`,
  `specmap.json`, `conform.toml`, `wire-derive-baseline.json`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). `2026-08-06-01` (P1) — «ruled — re-judgement campaign pending»,
её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках и в ТЗ)

- **PROP-044 ратифицирован**; терминология §2b обязательна.
- **Карантин — суждение ЧИТАТЕЛЯ о паре «запись × сборка»**, выводится в точке
  применения и никогда не хранится на проводе.
- **Умолчание безопасно по КОНСТРУКЦИИ:** отвечающий путь спрашивает
  именованные аксессоры, писатель и мутации — нет; каталог есть проекция
  журнала, и возможности читателя не сокращают записанное.
- **Спек-дифф входит в посадку фазы**, а не следует за ней — оплачено этой
  сессией шестью лжами в PROP-005.
- **Нормативное значение не копируют** — порядок пассов, поверхность писателя,
  эпоха, текст рецепта, предикаты совпадения, проход отказа.
- **Сгенерированный файл заменяют, а не перезаписывают**; эталон
  регенерируют, а не патчат; корпус — прогоном, не рукой.
- **Каталог — проекция журнала**; пути «прочитал → записал» нет с Ф3.
- Допубликационный режим (D13): ломать бесплатно и без миграций, `wire-diff`
  отчётный. Делегирование по умолчанию; ревью, вердикты, спеки, планы и
  коммиты — никогда не делегируются. Раскатка только `cargo xtask mirror`.
  Никогда `git add -A`. Печать — только за проверенное.

## Последние коммиты (свежие сверху)

```
55f36630 docs(campaign): two predictions were false, and that is the result
3037db77 docs(campaign): the acceptance script could pass over an untested surface
4842ad89 docs(campaign): phase 6.2 closes, and two of its lessons are about packets
ce3de248 feat(vibe-index): the logging dial and the variable become one lever
0798614f feat(vibe-index): the server names the version it will not serve
72702642 docs(campaign): the raw file was never the one keeping silent
fa50b653 feat(vibe-index): seven verbs stop hiding what they cannot serve
d7aff0ce docs(harvest): the server already refuses, and the client never asks
a02ccf82 docs(campaign): a surface nobody inventoried is growing a field
5fabcea6 feat(vibe-index): the loader stops destroying what the answer must describe
5f308eb0 docs(campaign): the safe default had to be a construction, not a wish
ee198286 docs(handoff): the entry prompt starts from a phase already measured
1f79938d docs(continue): cold-resume checkpoint
9b5d166b docs(wal): session-end checkpoint
ea3bc3e0 docs(campaign): both remaining forks were asked about the wrong thing
e2a8b60c docs(campaign): quarantine belongs where the answer is formed
489c56d3 docs(campaign): phase 6.1 closes, and the forced order paid observably
5c023848 feat(vibe-registry): the client asks the handshake before the catalog
69bdad89 feat(vibe-index): the index starts publishing its handshake
0b708bd5 docs(campaign): the eternal file found a hole nothing else could
2668fa55 feat(vibe-wire): the eternal file gets the schema it is read by
e1d77d5b fix(xtask): the shapes pass learns the one form it never met
c6a984e9 refactor(vibe-index): the catalog's surface gets one home before it grows
4b021a01 docs(wal): the map count was quoted where it should have been re-run
0c48b43e docs(campaign): phase 6 gets the slots its measurement forced
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста, 53 шага
cargo xtask specmap --check
cargo xtask check-codegen         # git diff против ИНДЕКСА — untracked не видит
cargo xtask wire-diff             # корпуса + флаги эпох → вердикт
cargo xtask rebuild --check formats/corpora/index/e1   # 13 файлов
cargo test -p vibe-index --no-fail-fast
cargo test -p vibe-registry --no-fail-fast
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
