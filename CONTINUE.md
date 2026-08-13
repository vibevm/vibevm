# CONTINUE — cold-resume snapshot (2026-08-14, wind-down №20)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md).

## TL;DR

Сессия 2026-08-14 **исполнила фазу Ф0 целиком и посадила Ф1.1** главной стройки
change-native форматов. Вся тяжёлая работа шла делегированно на GLM-воркерах
(`claudez`/`claudez2`): **четыре пакета, 4/4 приняты с первого прохода, ни одного
цикла доработки**. 8 коммитов, панель зелёная на каждой границе, оба зеркала
синхронны на `d480f6c7`.

**Главное для решений:** вердикт Ф0.1 — переделка read-modify-write **влезает** в
бюджет двух фаз, поэтому СТОП-ВЛАДЕЛЕЦ не сработал и запасной путь D3 не
активирован. Владелец не заблокирован ничем.

## Где стоит работа

- Ветка `main`, дерево чистое, HEAD `d480f6c7`; **раскатано** (`cargo xtask
  mirror --check` — gitverse sync, github sync).
- Полная панель `bash tools/self-check.sh` — **`self-check: all green`**
  (последний полный прогон — на слайсе Ф1.1; после него только docs-коммиты,
  каждый проверен целевыми гейтами `specmap --check` + `progress check`).
- Судейство: **0 неосуждённых, 0 осиротевших**; 33 файла stale — стоячий долг,
  адресован кампанией S7. Корпус 281 файл, 0 неразмеченных.
- **`campaigns/**` и `spec/WAL.md` в судимый корпус НЕ входят** (проверено) —
  правки планов, находок и WAL судейского долга не создают.
- Воркеров нет, worktree'ов нет, логи и отчёты — в `cache/agents/sorted/F0-*`,
  `F1-1-REGISTRY` (каждый с `meta.md`, несущим вердикт ревью).

## Блокер и действие человека

**Блокера нет.** За владельцем остаётся только **S2** — переименование живых
`_`-репозиториев в org `vibespecs` (нужны org-права: `gh repo rename`, шаги в
identity-ТЗ §S2). Это не блокирует главную полосу.

## Рецепт следующего шага (дословно)

Вставить содержимое [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md) первым
сообщением свежей сессии. Вход — **Ф1.2** (поле `epoch: Option<u32>` в
`[package]`, `crates/vibe-core/src/manifest/package.rs`; отсутствие = состояние
«до эпох», НЕ «эпоха 1»; `vibe init` и шаблоны пишут `epoch = 1`; `vibe check`
сообщает отсутствие как info). Дальше Ф1.3 → Ф1.4 → Ф1.5, порядок жёсткий,
каждый шаг отдельным коммитом.

## Неочевидные находки этой сессии (сверх документов)

**Форма `vibe-wire-gen` задана заранее.** Преобразования А.5 №3 (`x-empty`) и №4
(строгость по роли) требуют входов, которых в сгенерированном Rust НЕТ —
`metadata."x-empty"` из JTD-схемы и `foreign_parsers` из реестра. Значит Ф4.2 —
конвейер `(схема + реестр + сгенерированный Rust) → Rust`, а не текстовый фильтр,
как читается Приложение А.5. Тегированные объединения (`RepomdFileEntry`, Ф1.5)
этим путём НЕ покрываются — нужен отдельный путь кодогенерации.

**Заголовок выхода jtd-codegen врёт версию** (`v0.2.1` при бинаре 0.4.1) — пинить
надо бинарь, а не верить строке. Сам бинарь **gitignored**
(`tools/.gitignore:16`): свежий worktree его не несёт, провизия обязана копировать.

**Две ловушки Ф1.3 найдены до старта** — полностью в WAL
`##WAL-KI-HASH-RECIPE-TRAPS`: третий участник `ContentHash` в `vibe-core`
(проверка на границе десериализации отвергнет `sha256-tree/1:`) и golden-фикстура,
которая не упражняет расхождение порядка, ради которого существует.

**Два дефекта плана исправлены находками:** команды `vibe load` не существует
(схема принадлежит `vibe tree --json`, и это единственный CLI-формат не на JTD);
`by-cap`/`by-purl` публикуются без ридера файла — открытое нарушение G11, которого
Ф4.1 не видела (схемы добавлены).

**`check-codegen` сравнивает с ИНДЕКСОМ** (`git diff --exit-code` по
generated-каталогам) и не видит untracked. Слайс, трогающий `generated/**`,
садится в порядке **стейдж → панель → коммит**; `git add` достаточно, чтобы гейт
позеленел.

**Переименование тестовой функции с `#[verifies(…)]` двигает specmap** — карта
пересобирается тем же заходом, иначе панель красная на `specmap --check`.

**Две shell-ловушки, оплаченные ошибками этой сессии** (обобщены в
`SUBAGENT-LAUNCHERS.md` §8): голый `cd` уводит НЕ только `-c`-поправку, а все
последующие команды (cwd персистентен), и проверка обязана адресоваться так же,
как правка; `cmd | tail; echo $?` печатает код пайпа, а не команды.

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов; `spec/WAL.md` —
  канонное живое состояние; `spec/modules/vibe-registry/PROP-002`, `PROP-008`,
  `spec/modules/vibe-index/PROP-005`, `spec/modules/vibe-workspace/PROP-022`.
- `campaigns/packages-2026-09/` — три ТЗ, `harvest/f0-*.md` (три находки Ф0),
  `SUBAGENT-LAUNCHERS.md` (+ `SUBAGENT-MODE.toml` = `claudez`), `tasks/*.py` (суд),
  `run/` (состояние сканера).
- `formats/REGISTRY.toml` — **новое (Ф1.1)**: реестр 20 форматов, из него
  генерируется `FormatId`.
- `crates/` — 19 крейтов + `xtask`. Предмет ближайших шагов: `vibe-core/manifest`
  (Ф1.2 эпоха, Ф1.4 слоты), `vibe-index` + `vibe-registry/shippable.rs` (Ф1.3
  рецепт хэша), `vibe-index/types` (Ф1.4/Ф1.5), `vibe-wire` + `xtask/codegen.rs`
  (Ф4).
- Корень: `BACKLOG.md`, `AUDIT.md`, `NEXT-SESSION-PROMPT.md`, `specmap.json`,
  `TASKS.md`.

## Открытые находки аудита

Активное подмножество — в [`AUDIT.md`](AUDIT.md) (это durable-дом, здесь не
зеркалится). На последнем прогоне: 13 находок, 9 open, 10 переходят дальше;
`2026-08-06-01` (P1) — «ruled — re-judgement campaign pending», её исполняет
кампания S7.

## Решения в силе (опорные; длинно — в спеках)

- **PROP-044 ратифицирован** — стоячий закон каждого формата; терминология §2b
  обязательна (snapshot ↔ frozen — антонимы; «канал» — авторский указатель;
  capture — провайдерское; режим материализации — **`copy`**, и это слово
  вычищено из дерева в смысле режима).
- **Незарегистрированный формат невыразим в системе типов** — `FormatId`
  генерируется из `formats/REGISTRY.toml`, тест полноты сверяет их как множества
  в обе стороны. `FormatId` намеренно без serde (иначе попал бы под запрет Ф4.3).
- Идентичность: LDH-группы; композит `<group>.<name>`; слот и кэш — от
  идентичности; координата = полная строка версии.
- Допубликационный режим (D13): миграции не применяются, пока владелец не
  объявит первый показ публике; факт публикации НЕ выводится из технических
  событий.
- Делегирование по умолчанию; ревью, вердикты, спеки, планы и коммиты —
  никогда не делегируются. Усилия не экономятся; объём — не довод.
- Раскатка только `cargo xtask mirror` (fast-forward, никогда `--force`).
  Никогда `git add -A`. Печать суда — только за проверенное.

## Последние коммиты (свежие сверху)

```
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
498e8c8b chore(campaign): the copy rename is judged, and a blind vouch is taken back
d84cf5b6 docs(spec): dead slot pointers follow the re-materialised tree
d4d6c475 feat(core): the default materialisation mode says copy
6f4b5750 fix(check): the lockfile-files check composes the identity slot
6e90854e chore(campaign): the identity rulings are judged in the same pass
189152af docs(specmap): the map follows the identity rulings
30049e81 docs(backlog): three owner rulings land in their rows
1cd487b3 docs(audit): the proof bar splits by claim kind
1e2c9ebe docs(campaign): the executor's plan absorbs the week's rulings
9fc59c01 docs(spec): channels become author pointers with computed built-ins
3106bf0d docs(spec): snapshot and frozen become antonyms across the contract
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
