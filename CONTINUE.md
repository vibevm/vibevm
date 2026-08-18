# CONTINUE — cold-resume snapshot (2026-08-18, wind-down №31)

**Не цитируй числа отсюда — меряй.** И меряй в правильном порядке: `scan` →
**`mirror`** → `judging-debt.py`, потому что долг читается из ЗЕРКАЛА и до
`mirror` показывает ноль над реальным долгом (поймано этой сессией).
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

**Спека PROP-005 перестала лгать о коде.** Мандат называл шесть мест; полный
делегированный проход с контролями вернул **тридцать**, и в трёх классах, из
которых один точечным грепом ненаходим по построению.

**Предусловие §11 закрыто.** 70 рулингов плана картированы с цитатами, семь
бездомных получили дом, три деферала Приложения Б.6 спасены в реестр, которого
они не достигали.

**Осталась одна работа — сама свёртка плана**, и её рецепт лежит в `TASKS.md`.
Не сделана сознательно: editor-инструментами файл на 3055 строк сворачивается
только переписыванием набело, а сессия прочитала около сорока процентов.

## Где стоит работа

- Ветка `main`, HEAD `9c663b09`, дерево чистое, зеркала синхронны
  (gitverse + github @ `9c663b09`).
- Панель `bash tools/self-check.sh` — **`self-check: all green`** четырежды за
  сессию, каждый раз **53 шага**, реальный код выхода 0.
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых ОЖИДАЕМЫ и не
  двигались более сорока посадок подряд.
- Судейство: **0 неосуждённых, 0 осиротевших**; 34 файла stale. Корпус 281
  файл, 13836 маркеров, 0 неразмеченных. За сессию — **46 вердиктов**.
- Карта: 6108 спек-юнитов / 1042 tagged / 975 рёбер / 0 подозрений / 0 сирот.
- Воркеров нет, **`.wt/` пуст** — все четыре рабочих каталога сняты чисто.
  Два worktree под `~/.fractality/runs/**` — чужие, не трогать.
- Логи, отчёты, пакеты и `meta.md` с вердиктами — в
  `C:\Users\olegc\git\v\cache\agents\sorted\{PROP005-DRIFT-A,PROP005-DRIFT-B,PLAN-MORTALITY-C,PLAN-MORTALITY-D}\`.
  **Архив живёт ВНЕ чекаута:** `cache/agents/...` в текстах — это
  `git/v/cache/...`, а не подкаталог репозитория.

## Блокер и действие человека

**Блокера нет.** За владельцем девять вопросов, ни один не блокирует свёртку:

- **СТОП-ВЛАДЕЛЕЦ:** **S2** (org-права), **Ж8** (`--full` и запись, которую
  скан больше не видит).
- **Развилки `BACKLOG`:** **B-056**, **B-078**, **B-079**, **B-080**, **B-081**,
  **B-082** и семь новых этой сессии — **B-083** … **B-089**.

## Рецепт следующего шага (дословно)

Вход — `NEXT-SESSION-PROMPT.md`. Работа одна: **§11, свёртка плана**. Рецепт
уже написан в `TASKS.md`, пункт «§11 — смертность плана: сама свёртка», и
ключевое из него:

1. **Прочесть план ЦЕЛИКОМ** — свёртка есть переписывание набело.
2. Сохранить дословно **§0** (базовая линия), **§10** (приёмка + сверка
   предсказаний), **§11**.
3. Свернуть в могильники §1, §2, §3–§9 и приложения А и Б. Форма:
   «Closed \<дата\> by \<коммиты\>. The ruling and its reasoning now live in
   \<дома\>». Коммиты — из таблиц «Коммиты посадки» трёх карт; дома — из «Карт
   домов».
4. **Дом бывает ДВУХ видов**, и могильник говорит какого: якорь спеки либо
   файл, чей докблок несёт рассуждение. Карты размечены `spec` / `code` /
   `both`.
5. **Ссылок на план в `spec/**` ЧЕТЫРЕ, а не две**, как утверждает §11.2.
   Живые указатели `PROP-044` `##PURPOSE` и `##SOURCES` уходят;
   провенанс-упоминания (`PROP-044:3` в комментарии `<status>` и
   `spec/research/schema-evolution-2026-08/README-PROVENANCE.md:22`) остаются —
   три жанра, а не два (`BACKLOG` **B-088**). Если файл плана потом удалят,
   провенанс-ссылка обязана указывать на коммит, а не на путь.

Карты: `campaigns/packages-2026-09/harvest/plan-mortality-{c,d,section1}.md`.

## Что сделала эта сессия — по существу

| работа | результат |
|---|---|
| №1 спек-диффы Ф4/Ф6 | **30 находок вместо шести**, все закрыты или честно перемечены |
| №1 следствие | пять непостроенных обещаний → развилки владельца B-083…B-087 |
| №2 предусловие §11 | 70 рулингов картированы; 7 бездомных получили дом |
| №2 спасение | три деферала Б.6 → `deferrals.md`; иначе свёртка стёрла бы их |
| №2 §11.3 | пробел машинерии вынесен из умирающего плана в `BACKLOG` B-088 |
| попутно | `TASKS.md` догнал три закрытые фазы; два правила нарезки пакетов в §8 запускалок; B-089 (документы индекса) филирован, не починен |

## Девять вещей, которые эта сессия установила

**1. Спека лжёт ТРЕМЯ способами, и греп находит один.** «Отстала от кода»
ищется по имени новой сущности. **«Впереди кода при `impl/done`» не ищется
ничем** — искать нечего, надо читать утверждение и спрашивать дерево, есть ли
оно. «Противоречит себе» находится только сплошным чтением. Отсюда шесть → 30.

**2. Самая дорогая ложь — не устаревшая, а оптимистичная.** `[[registry]]
index_url` описан спекой, отсутствует в манифесте, а секция несёт
`deny_unknown_fields`: TOML-пример спеки — **не пример, а отказ разбора**.
Читатель вокруг такого утверждения ПЛАНИРУЕТ.

**3. Периметр, заданный смыслом, уже периметра, заданного пересчётом.** Замер
смертности плана был нарезан по понятию «секция фазы» — 2762 строки из 3055, и
**оба бездомных куска оказались ровно в непокрытых 293**. Когда работа
разрушительна, периметр считают по файлу.

**4. Счёт бывает свойством регекспа.** Три шаблона дали 68, 69 и 70 рулингов на
одном файле: один заголовок с отступом, другой с тире вместо точки. Верное
число нашлось потому, что воркер прогнал ДВА шаблона.

**5. `judging-debt.py` читает зеркало.** Прогнанный до `progress mirror`, он
показал **0 неосуждённых при одном неосуждённом**. Порядок scan → mirror →
batch → merge → debt обязателен целиком.

**6. Якорь не удаляют — ему ставят надгробие.** Правка §3.2 удалила
`##dep-prometheus`, и это увидел не глаз, а **диф множеств якорей** (`comm`
старого и нового списка). Такую сверку гонять перед каждой посадкой спеки.

**7. Дом рулинга бывает двух законных видов** — якорь спеки для политики,
докблок кода для формы этого кода. Домом НЕ являются: сам план, WAL,
`CONTINUE.md`, находка в `harvest/`, коммит-сообщение.

**8. План сам назвал половину своих домов.** D13 пишет «контракт — PROP-044
`##THE-PUBLIC-SWITCH`», D14 — «контракт: PROP-005
`##RESOLVER-DEFAULT-IS-STABLE-THEN-LATEST`». Решение, записанное вместе с
адресом своего будущего дома, переживает документ, в котором записано.

**9. Замер — идеальная форма делегирования.** Четыре read-only пакета, четыре
приёмки с первого прохода, ноль `-c`-циклов, ноль git- и cargo-вызовов
воркерами. Работающая форма: закрытый список записи из ДВУХ файлов,
`find -newer` вместо запрещённого git, и **контрольный список якорей, где
ложные вперемешку с заведомо верными** — им ловится и пасс, помечающий всё
подряд, и пасс, не нашедший ничего.

## Что решено НЕ делать, и это решение

**Свёртка плана не выполнена** — переписывать набело документ, зная меньше
половины его текста, нельзя. **`B-089` филирован, а не починен** — три файла
документации это отдельная связная работа, и приклеивать её к §11 значило бы
нарушить атомарность. **Непостроенные обещания не удалены из спеки** — код
обязан соответствовать спеке, а не наоборот; двигается СТАТУС, требование
остаётся, выбор уходит владельцу. **Печать (`seal`) не поставлена** — PROP-005
переписана в тридцати местах за день.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`** — до-эпоховые манифесты под `packages/`.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.

**Рост любого из них — находка.**

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-index/PROP-005` — **выправлена
  2026-08-18 в тридцати местах**, новая §2.19 «The `unavailable` answer».
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (**пять новых находок**:
  `prop005-drift-{a,b}.md`, `plan-mortality-{c,d,section1}.md`),
  `SUBAGENT-LAUNCHERS.md` (§8 — транспортные факты; `SUBAGENT-MODE.toml` =
  `claudez`), `deferrals.md` (**раздел `{#change-native}` — новый**),
  `tasks/*.py`, `run/`.
- `formats/` — `REGISTRY.toml`, `EPOCHS.toml`, `corpora/index/e1/`,
  `vocabularies.json`, `hash_recipes/1.toml`, `breaks/001.md`.
- `schemas/` — семь CLI-отчётов в корне плюс `index/e1/` (пять), `journal/e1/`,
  `hello/e1/hello.jtd.json`.
- `crates/` — 19 крейтов + `xtask`. Предмет: `crates/vibe-index/**` (журнал,
  карантин, сервер, CLI), `crates/vibe-registry/src/index_client/**`,
  `crates/vibe-wire/**` (сгенерированные типы + слой поведения).
- Корень: `BACKLOG.md` (**+7 записей**), `AUDIT.md`, `TASKS.md`
  (**+рецепт свёртки**), `NEXT-SESSION-PROMPT.md`, `specmap.json`,
  `conform.toml`, `wire-derive-baseline.json`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). `2026-08-06-01` (P1) — «ruled — re-judgement campaign pending»,
её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках и в WAL)

- **PROP-044 ратифицирован**; терминология §2b обязательна.
- **Карантин — суждение ЧИТАТЕЛЯ о паре «запись × сборка»**, выводится в точке
  применения и никогда не хранится на проводе; отсюда согласие CLI и сервера
  ПО ПОСТРОЕНИЮ.
- **Умолчание безопасно по КОНСТРУКЦИИ**; каталог — проекция журнала.
- **Спек-дифф входит в посадку фазы**, а не следует за ней.
- **Нормативное значение не копируют**; числа-замеры в прозе спеки не пишутся
  вовсе — они живут в датированной находке.
- **Обещание чинится СТАТУСОМ, а не удалением требования.**
- **Якоря неизменны** — устаревшее получает надгробие с наследником.
- Допубликационный режим (D13): ломать бесплатно и без миграций. Делегирование
  по умолчанию; **ревью, вердикты, спеки и планы — никогда**. Раскатка только
  `cargo xtask mirror`. Никогда `git add -A`.

## Последние коммиты (свежие сверху)

```
9c663b09 docs(tasks): the collapse gets a recipe instead of a memory
9a05f57b docs(campaign): two packet-cutting rules the collapse would have erased
8aea1247 docs(spec): three rulings the plan would have taken with it
d77fa168 docs(harvest): seventy rulings, and where each of them lives now
a1654ca8 docs(campaign): a perimeter cut by meaning under-covers the file
e7ac5a49 docs(backlog): two gaps the mortality measurement surfaced
f5f857a4 docs(campaign): what the plan's death would erase, moved out ahead of it
0088c0cf docs(spec): the logging ruling gets the home it never had
3772ca1a docs(campaign): the anchors this landing minted are judged
d444a4ec docs(tasks): the checklist catches up with the closed phases
35bf5985 docs(backlog): five unbuilt promises get their owner forks
414581d4 docs(spec): PROP-005 stops lying about the code it describes
5f6be1e4 docs(harvest): a full pass finds what a pointed grep cannot
bc7fb68d docs(handoff): the entry prompt starts where the code stopped and the spec did not
4c6b2f8e docs(continue): cold-resume checkpoint
cdd40557 docs(wal): session-end checkpoint
55f36630 docs(campaign): two predictions were false, and that is the result
3037db77 docs(campaign): the acceptance script could pass over an untested surface
4842ad89 docs(campaign): phase 6.2 closes, and two of its lessons are about packets
ce3de248 feat(vibe-index): the logging dial and the variable become one lever
0798614f feat(vibe-index): the server names the version it will not serve
72702642 docs(campaign): the raw file was never the one keeping silent
fa50b653 feat(vibe-index): seven verbs stop hiding what they cannot serve
d7aff0ce docs(harvest): the server already refuses, and the client never asks
a02ccf82 docs(campaign): a surface nobody inventoried is growing a field
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan   --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress mirror --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py    # ТОЛЬКО после mirror
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста, 53 шага
cargo xtask specmap --check
cargo xtask check-codegen         # git diff против ИНДЕКСА — untracked не видит
cargo xtask wire-diff
cargo xtask rebuild --check formats/corpora/index/e1
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
