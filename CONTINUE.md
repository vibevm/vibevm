# CONTINUE — cold-resume snapshot (2026-08-02, session end №2)

**Не цитируй числа из этого файла — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот везде,
где они разойдутся.

**TL;DR.** PROP-043 wave 2, Phase D — **очередь одобрений осушена целиком**,
на столе один исполняемый рулинг и за ним выходной гейт. Сессия 2026-08-02
(вторая половина дня, «ситтинг предъявлений», ~десять обменов с владельцем)
осушила ВСЮ sync-очередь: F-185/F-217/F-218/F-132 (стройки вместо смягчения),
F-285 (снят), F-154/F-355 (build-first pivot), F-161/F-167/F-181/F-284/F-215
(канон 75.3/70.2, семьи замеров и зомби), F-210 (история OracleRegistry),
F-178/F-199 (B-045, исключение boot-поверхностей), и финальную девятку
(F-114/F-157/F-216/F-270/F-273/F-275/F-280/F-309 + стройка пина). Последний
замер: корпус **11 187 / 191 / 44 — 97.9 %** из 11 422 (`ai-native` 98.3 %);
реестр **91 обязательство / 191 дрейфов — 32 отложено рулингами владельца, 59
открыто; owed 18 = 17 на владельце-руленных стройках + 1 открытый F-279,
рулинг по которому УЖЕ ДАН** (см. следующий раздел). Resolved **138**.
Boss-owed — ноль. Бэклог стоит на **B-047**; свип B-027 исполнен (done);
карта развития живёт в корне (`TOOLING-MAP.md`). Всё закоммичено, оба зеркала
на HEAD (этот файл и WAL едут следом).

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — исполнить уже данный рулинг F-279

Владелец ответил на разбор «где должна жить specmap.jtd.json» в самом конце
сессии, исполнить не успели. Рулинг дословно: **«Вариант a + По-моему там был
еще jtd генератор в tools/jtd-codegen, его стоит положить в отдельный пакет
org.vibevm.ai-native/jtd-codegen»**.

Что это значит (все факты сверены в сессии):

1. **Схема переезжает в пакет движка:** `schemas/specmap.jtd.json` (корень
   хоста, единственная копия в дереве) →
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/specmap.jtd.json`.
   Остальные 7 файлов в `schemas/` — vibe-wire-репорты хоста, их НЕ трогать.
   Метаданные схемы внутри файла до-переездные (`crates/specmap-core/...`,
   `specmap_core::specmap`) — обновить на `core-ai-native-specmap` (B-013).
2. **Канонический пример рядом** («может нужен не он, а пример» — включено в
   (а)): маленький образцовый `specmap.json` рядом со схемой; кандидат-источник
   — фикстуры экстракторов (например, у go-extract/ts-extract) или минимальный
   рукописный.
3. **`xtask/src/codegen.rs` перенацелить:** `generated_dir_for` (:50-52) и
   drift-check-близнец (:215) целят в МЁРТВЫЙ слот
   `rust-ai-native-lang/v0.5.0/crates/specmap-core` — перенаправить на
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated`
   и на пакетный путь схемы. Заголовок движкового крейта
   (`core-ai-native-specmap/src/lib.rs:24-27`: «generated from the package's
   schemas/specmap.jtd.json») после переезда станет ПРАВДОЙ — не трогать.
   Проверка: `cargo xtask codegen --check` (byte-compare) должен пройти.
   **Это закрывает B-013 целиком** (все четыре координаты из его fix shape).
4. **README растового стека** (`rust-ai-native-lang/v0.7.0/README.md`
   `##SHIPS-SPECMAP-WIRE-SCHEMA`) — честный текст: пакет шипит генерированные
   типы (`crates/vendor/core-ai-native-specmap/src/generated/`), схема и пример
   живут в `core-ai-native`. Затем merge (батч D42) → seal → re-judge
   confirmed → **F-279 резолвится, owed = 17 (все на рулингах) → CONVERGENCE
   гейта ВЫПОЛНЕНА**.
5. **Новый пакет `org.vibevm.ai-native/jtd-codegen`** (kind `tool`):
   содержимое `tools/jtd-codegen/` — это `README.md` + **`jtd-codegen.exe`**
   (прекомпилированный бинарь, уже в дереве). ВНИМАНИЕ: класть .exe в ПАКЕТ =
   вопрос large-blob/бинарей в пакетной поставке — прочитай
   `tools/jtd-codegen/README.md` первым (там способ установки; вероятно,
   честнее пакет с README-рецептом установки + [[binary]]-подход или
   source-vendoring, чем .exe в payload). Если форма пакета неочевидна —
   один короткий вопрос владельцу с вариантами (это new-package + бинарь,
   пограничье Rule 4). `find_jtd_codegen` в xtask (codegen.rs:13-34) ищет
   `tools/jtd-codegen/<exe>` с fallback на PATH — после упаковки перенацелить
   (или оставить tools/-путь как локальный fallback, решить при исполнении).
   Схема mirror-layout — PROP-000 §13; naming — PROP-008
   (`org.vibevm.ai-native/jtd-codegen`).
6. Пере-вендор слотов НЕ нужен немедленно (§3.5: judged-vs-installed различие
   штатно; слоты догоняют release-событием). Транспорт правок — обычные
   коммиты + `cargo xtask mirror`.

## ВТОРОЕ ДЕЛО — выходной гейт фазы D (после F-279)

По `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` §11 (acceptance)
и §7-gate: (0) `bash tools/self-check.sh` → 0; (1)
`./target/debug/vibe.exe progress check --exhaustive --campaign
campaigns/packages-2026-09` → clean; (2) `summary.py` — пер-неймспейсные
числа показаны; (3) `drift-registry.py` — CONVERGENCE: owed 0-или-ruled
(после F-279 выполнится: 17 owed, все на рулингах B-022/023/025/026/029/031/
033/034) + `test -s run/state/routing.json`; (4) **`baseline.json` написать**
(A6; как Phase C — образец её baseline в git); (5) **A–D инвентаризация
health-audit** (пункт принятия аудита 2026-08-01; категории — `AUDIT.md` +
skill `/health-audit`); (6) LOG-запись закрытия фазы + коммиты по commit map
(§7.1 #cm-planned: «the routing record closes…», «phase D closes…», «the
phase boundary's baseline»). Фазы E/T/F/G — спроектированы, НЕ стартуют без
слова владельца.

## Очередь владельца

Пусто, кроме встроенного в первое дело вопроса о форме пакета jtd-codegen
(если форма с бинарём окажется неочевидной). Все прочие решения даны и
исполнены либо записаны стройками. B-027 — done. Остальное открытое в
реестре (59 строк) — non-sync маршруты (prose-edit босса 49 строк — отдельная
боссова очередь, этой сессией не тронута; release 2; build-or-demote остаток)
и отроученные строки, ждущие гейт-сверки; сверься с `drift-registry.py`.

## Стоячие правила этой сессии (сверх прежних; каждое оплачено)

- **BUILD-FIRST (владелец, 4-й обмен):** для механизмов дисциплины
  «аннотировать отсутствие» как финал — умерло; аннотация легитимна только
  как интерим с именем записанной стройки («Specified, not built (→ B-nnn)»);
  ослаблять правило за неиспользование запрещено. «Система не заморожена».
- **Рамка кампании (5-й обмен):** волны карты (`TOOLING-MAP.md`) исполняются
  фазами кампании (E после гейта D); из карты ничего не стартует.
- **Формат предъявлений (уточнение поверх (ii)):** сначала суть
  по-человечески, затем ТОЧНЫЕ технические имена (настройки, файлы,
  поведение) — точность не теряется; спец-жаргон приложением. «Две настройки»
  без имён — недопустимо.
- **«Замеров нет и нескоро будет»** — стоячий ответ записан (B-042 + карта +
  три complete-цели); вопрос «почему нет замеров» владельцу БОЛЬШЕ НЕ
  ЗАДАЁТСЯ.
- **Норма поверхностей (B-047)** и **закон автономности композиции (B-046)**
  — направления владельца дословно в записях.
- **`deferred` в реестре = рулинг владельца**, а не routing босса: массовый
  флип 58 строк сделан и ОТКАЧЕН (9-й обмен) — не повторять.
- **Настоящий mirror — `vibe progress mirror --campaign <зона>`** (per-file
  views в `run/mirror/`); `progress check` им НЕ является. Новые якоря в
  файле требуют mirror перед merge-verdicts (прецедент — два якоря ростера
  git-practices).
- **merge-verdicts может словить WinError 5** на os.replace cache.json
  (гонка локов Windows) — просто повторить, идемпотентно.
- **Вердикты в кэше** — `files.<path>.campaign.verdicts.<ANCHOR>.v`;
  печать через `PYTHONIOENCODING=utf-8` (cp1252-консоль давится юникодом).
- **Очередь владельца строится из РЕЕСТРА, не из снимка harvest'а**
  (диагноз «девятки F-132»: партия 1a осушила её ещё 2026-08-01).
- **Пер-якорная проверка — святое:** носитель клятвы в ts-оракуле оказался
  `RUST-SIDE-OWNS-TERMINATION`, а не SHUTDOWN-якорь из таблицы harvest'а;
  четвёртый член floor-семьи нашёлся в rust-оракуле по ходу правки.

## Решения владельца этой сессии — карта исполнения

Все зафиксированы в ledger
(`campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md#rulings-2026-08-02-2`,
~десять обменов) и §7 LOG (записи 2026-08-02 с конца). Ключевое: стройки
B-033…B-047 зафайлены (паритет языков B-033/034/035/039, классы правил
B-036/037/038, рефакторинг швов B-040, карта B-041 одобрена+интегрирована,
B-042 accepted, B-043 дефект генератора, B-044 зомби-тест, B-045
qualified-naming, B-046 композиция, B-047 норма поверхностей); B-011 — Самый
Высокий Приоритет; B-027 done (19 флипов `@impl/plan`); B-013 закрывается
первым делом новой сессии; шаг 0c панели (байт-сверка тройки
CLAUDE/AGENTS/GEMINI) построен; исключение boot-поверхностей — PROP-000
`##ATTRIBUTION-BOOT-SURFACE-EXCEPTION`; канон цифры — пара ATLAS 75.3/70.2;
редбук-правило «смена состава двигает номер» — в манифесте; стройка пина —
`crates/vibe-cli/build.rs` + `RUST_PIN` (источник — workspace `rust-version`).

## Где стоит работа

- Ветка `main`, оба зеркала синхронны; роллаут — ТОЛЬКО `cargo xtask mirror`.
  Дерево чистое.
- `tools/self-check.sh` — all green, прогнан многократно, последний раз со
  стройкой пина (шаг 0c в составе). Никаких vibe-команд параллельно панели.
- `vibe progress check --campaign campaigns/packages-2026-09` — clean
  (260 files) на последнем прогоне. `--campaign` ВСЕГДА.
- cargo-audit 0.22.2 / cargo-outdated 0.19.0 установлены.

## Карта

`campaigns/packages-2026-09/`: PHASE-D-BATCH-PLAN.md (закон; §3.6/3.7/3.8 +
§6.1 читать дважды), PHASE-D-HOST-OBLIGATIONS.md (+`#rulings-2026-08-01`,
`#rulings-2026-08-02-2` — вся хроника обменов), harvest/d7a…d7f (тексты
пере-верификаций), tasks/*.py (merge → seal НЕ сцеплять; drift-registry.py
--write; summary.py), tasks/evidence/batch-D29…D41 (батчи сессии),
run/mirror/ (per-file views). `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`
§7 LOG — с конца. `BACKLOG.md` B-001…B-047 + раздел `#map`. `TOOLING-MAP.md` —
карта развития (корень; ROADMAP.md — продуктовый, другой документ).
`AUDIT.md` — активные находки. `spec/WAL.md` — канон.

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
bash tools/self-check.sh   # реальный exit-код; vibe-команды параллельно запрещены
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
