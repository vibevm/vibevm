# CONTINUE — cold-resume snapshot (2026-08-14, wind-down №21)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

Сессия 2026-08-14 посадила **Ф1.2 и Ф1.3** главной стройки change-native
форматов и закрыла **S3** независимой полосы; ещё три пакета были **замерами**
(Ф1.3, S6.1, S1) и легли находками. Делегирование: **6 пакетов, 6/6 приняты,
ноль циклов доработки**. 8 коммитов, панель зелёная на каждой посадке, оба
зеркала синхронны на `b2e7973e`.

**Главное для решений — не код, а то, что замеры опровергли четыре записанных
утверждения**, и одно из них было бы построено неверно, если бы Ф1.3 нарезали
по плану как есть. Владелец не заблокирован ничем.

## Где стоит работа

- Ветка `main`, дерево чистое, HEAD `b2e7973e`; **раскатано** (`cargo xtask
  mirror --check` — gitverse sync, github sync); origin ahead/behind = 0/0.
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`**
  (последний полный прогон — на слайсе Ф1.3, 1805 строк вывода).
- `vibe check` — **0 errors, 1 warning, 44 info**. Оба ненулевых числа
  ОЖИДАЕМЫ и объяснены ниже; рост любого из них — находка, а не шум.
- Судейство: **0 неосуждённых, 0 осиротевших**; 33 файла stale — стоячий долг,
  адресован кампанией S7. Корпус 281 файл, 0 неразмеченных.
- Карта: 6051 спек-юнит / 1020 tagged items / 968 рёбер / **0 сирот**.
- Воркеров нет, worktree'ов нет; логи и отчёты — в
  `cache/agents/sorted/{F1-2-EPOCH,S3-JOINER,S6-MEASURE,F1-3-PROBE,F1-3-BUILD,S1-PROBE}/`,
  каждый с `meta.md`, несущим вердикт ревью и разбор дефектов.

## Блокер и действие человека

**Блокера нет.** За владельцем остаётся **S2** — переименование живых
`_`-репозиториев в org `vibespecs` (нужны org-права: `gh repo rename`, шаги в
identity-ТЗ §S2). Главную полосу не блокирует. Ближайший СТОП главной полосы
(смена вида `content_hash` в локфайле) **обойдён по конструкции** — реестровая
копия осталась на рецепте 0.

## Два ожидаемых числа — не чинить

1. **44 info `manifest_epoch`.** До-эпоховые манифесты под `packages/`,
   помечаемые намеренно (Ф1.2, решение D5): отсутствие `epoch` — это состояние
   «до эпох», а не «эпоха 1», и счётчик info есть сигнал, который осушит волна
   кодмода. Периметр — все локально видимые манифесты с `[package]`, не только
   корневой: диагностика, смотрящая в корень, молчала бы ровно о той популяции,
   ради видимости которой заведена.
2. **1 warning `local_source_freshness`** по `org.vibevm.world/addressable-specs`.
   Следствие правки исходника пакета в S3 при намеренно НЕ запущенном
   `vibe install` (§S3 шаг 3 запрещает). Гаснет следующим переизданием пакета.

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **замер под Ф1.4**, затем сама Ф1.4 (слоты
`must_understand` / `yanked` / `frozen` / `tombstone`; манифестный `frozen:
Option<bool>` рядом с `epoch` в `crates/vibe-core/src/manifest/package.rs`).
Дальше Ф1.5, порядок жёсткий, каждый шаг отдельным коммитом. Независимая полоса
готова к нарезке: S1 и S6 — оба измерены, у каждого по одному вопросу, который
решает БОСС при нарезке (см. ниже).

## Неочевидные находки этой сессии (сверх документов)

**Регрессия, которую не мог поймать ни один golden — и поймал потребитель.**
Реализация Ф1.3 сменила порядок рецепта 0: сортировку `Vec<PathBuf>` на
сортировку строки. `Ord` у `Path` — **покомпонентный**, поэтому это разные
порядки, и разница тихо сдвигает хэш всякого дерева, где имя каталога
префиксует имя соседа. Три заслона были на месте и ни один не сработал: старый
golden не имеет такой пары; новый golden морозил только рецепт 1 (решение
босса, верное по причине и снявшее единственного нужного сторожа); кросс-тест
реализаций остался истинным, потому что обе копии сломались одинаково. Поймал
`vibe check`: 0 → 7 предупреждений, шесть по нетронутым пакетам, и обратно 1
после починки. **Счётчики предупреждений панели — доказательство, а не шум.**

**Посылка Б.3 опровергнута.** `Path::cmp` покомпонентный, разделителя не видит,
значит рецепт 0 платформенно стабилен и записанное подозрение о расхождении
Windows/Unix неверно. Рецепты различает порядок «по компонентам» против «по
нормализованным байтам», и расходятся они ТОЛЬКО на соседе с байтом **ниже**
`/` (`spec-x.md`, 0x2D); на байте, названном планом (`specX.md`, 0x58), они
СОВПАДАЮТ — то есть фикстура такой формы не доказывала бы ничего.

**Продюсеров формы `sha256:` три, а не два.** Третий — `commit_content_hash`
(`git_package_registry/fetch.rs:484`) — выводит хэш из коммита, а не из дерева,
и именно он пишет `content_hash` в локфайл для in-place пакетов. Рецепт,
описанный как «исключения + нормализация + порядок обхода», его не описывает.

**Паритет-тест не сторожил того, чем назван** — звал одну реализацию и сверял с
константой, хотя doc-комментарий обещал гейт расхождения между двумя.

**Координата движка conform в дереве двоится.** Компилируется ВЕНДОР-копия под
`rust-ai-native-lang` (`Cargo.toml:103`), а не каталог
`core-ai-native/v0.7.0/crates/core-ai-native-conform`: у первой восемь полей
`Finding` и есть `freezeable`, у второй шесть и нет.

**Хост не различает «нет репозитория» и «приватный, невидимый твоим кредам»** —
обоим отвечает `Repository not found`. Кодом не снимается.

**Оплачено дважды: heredoc через оболочку съедает `\`.** Правки только
editor-инструментами — правило дерева, и оно про это.

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-registry/PROP-002` (§2.1
  identity — теперь несёт рецепт), `PROP-008`, `spec/modules/vibe-index/PROP-005`
  (§2.7 trust — digest joins at a stated recipe), `spec/common/PROP-029`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/` (шесть находок: три Ф0 и
  три этой сессии), `SUBAGENT-LAUNCHERS.md` (+ `SUBAGENT-MODE.toml` = `claudez`),
  `tasks/*.py` (суд), `tasks/evidence/*.json` (батчи вердиктов), `run/`.
- `formats/` — `REGISTRY.toml` (20 форматов, из него генерируется `FormatId`) и
  **новое (Ф1.3)** `hash_recipes/1.toml` (рецепт как данные).
- `crates/` — 19 крейтов + `xtask`. Предмет ближайших шагов:
  `vibe-index/types/entry/**` (Ф1.4/Ф1.5), `vibe-core/manifest/package.rs`
  (Ф1.4 `frozen`), `vibe-index/{index,journal}` (Ф2/Ф3), `vibe-wire` +
  `xtask/codegen.rs` (Ф4), `vibe-publish` (S1), `vibe-trace` (S6).
- Корень: `BACKLOG.md`, `AUDIT.md`, `TASKS.md`, `NEXT-SESSION-PROMPT.md`,
  `specmap.json`, `conform.toml`, `conform-baseline.json`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (durable-дом, здесь не
зеркалится). На последнем прогоне: 13 находок, 9 open; `2026-08-06-01` (P1) —
«ruled — re-judgement campaign pending», её исполняет кампания S7.

## Решения в силе (опорные; длинно — в спеках)

- **PROP-044 ратифицирован** — стоячий закон каждого формата; терминология §2b
  обязательна (snapshot ↔ frozen — антонимы; «канал» — авторский указатель;
  capture — провайдерское; режим материализации — `copy`).
- **Хэш называет свой рецепт** (Ф1.3): `sha256-tree/1:` — рецепт 1, параметры
  данными в `formats/hash_recipes/1.toml`; голый `sha256:` — рецепт 0,
  заморожен В КОДЕ и НЕ конфигурируем, потому что рецепт, который можно
  править, не заморожен. Индекс эмитит рецепт 1, реестр остаётся на рецепте 0 —
  так локфайл не двигается и СТОП-ВЛАДЕЛЕЦ не срабатывает. Отвергнуто:
  «считать по рецепту 1, метить старой меткой» — молчаливый перелом.
- **Отсутствие `epoch` ≠ «эпоха 1»** (Ф1.2): это отдельное состояние «до эпох»,
  навсегда читаемое замороженным ридером №0.
- **Джойнер судится свойством, а не символом** (S3): требование —
  детерминированный обратный разбор; его дают разделитель вне обоих алфавитов
  ЛИБО грамматическая гарантия на одну половину, и тогда границей служит
  крайнее вхождение со стороны гарантированной половины.
- **Незарегистрированный формат невыразим в системе типов** — `FormatId`
  генерируется из `formats/REGISTRY.toml`.
- Допубликационный режим (D13): миграции не применяются, пока владелец не
  объявит первый показ публике; факт публикации НЕ выводится из технических
  событий.
- Делегирование по умолчанию; ревью, вердикты, спеки, планы и коммиты —
  никогда не делегируются. Усилия не экономятся; объём — не довод.
- Раскатка только `cargo xtask mirror` (fast-forward, никогда `--force`).
  Никогда `git add -A`. Печать суда — только за проверенное.

## Последние коммиты (свежие сверху)

```
b2e7973e docs(wal): the checkpoint drops a premise measurement refuted
a774651d docs(launchers): six lessons the phase-1 fan-out paid for
4910aa1f feat(hash): a content hash says which recipe produced it
d2545c5f docs(harvest): the publish tiers are measured before they are built
587fc7ef docs(harvest): the conform artifact is measured, not assumed
b224aa67 docs(harvest): the hash radius is measured before the cut
20771260 docs(addressable-specs): the joiner law stops naming one character
2907a679 feat(manifest): a missing epoch stops meaning the first one
5289cdb5 docs(wal): the checkpoint stops carrying a counter that ages
c8389531 docs(continue): cold-resume checkpoint
3d3c8dda docs(handoff): the entry prompt moves its start to the next step
d480f6c7 docs(wal): the checkpoint follows two phases of progress
fd817814 feat(wire): an unregistered format becomes untypeable
5b3f5f10 docs(launchers): record what the phase-0 fan-out taught
e3fd7ee4 chore(campaign): the corpus state follows the rescan
a420c26f docs(materialisation): finish naming the mode copy
c9b75cd4 docs(campaign): the baseline follows the measurement
406e0ee4 docs(campaign): phase 0 measures what the plan assumed
53d97e78 docs(wal): session-end checkpoint
9a1e2f6e docs(continue): cold-resume checkpoint
6cd2f995 docs(handoff): the entry prompt turns from report-and-wait to build
560e8f67 chore(campaign): the ratification pass re-vouches the contract
874561fd docs(campaign): wave 2 closes the coverage gaps PROP-044 still had
7a81956e docs(spec): PROP-044 is ratified, and every gate that waited knows it
c2dec4ef docs(campaign): the rulings get their build plan while the context is hot
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста
cargo xtask specmap --check
cargo xtask mirror --check
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
