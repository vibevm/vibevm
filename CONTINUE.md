# CONTINUE — cold-resume snapshot (2026-08-03, phase D closed)

**Не цитируй числа из этого файла — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`.
`spec/WAL.md` переписан той же сессией и **суперсидит** этот снапшот везде,
где они разойдутся.

**TL;DR.** PROP-043 wave 2, **фаза D ЗАКРЫТА 2026-08-03 на зелёном гейте**.
Уже данный рулинг F-279 исполнен целиком, вопрос владельцу не понадобился:
схема `specmap.jtd.json` переехала в пакет движка
(`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/`, рядом
канонический `specmap.example.json`), оба маршрута `cargo xtask codegen`
перенацелены с мёртвого слота v0.5.0 (**B-013 закрыт целиком**,
`check-codegen` байт-чист), README растового стека пересужен (батч D42,
**F-279 резолвирован — resolved 139**), генератор упакован как
**`tool:org.vibevm.ai-native/jtd-codegen`** — рецепт без бинаря (бинарь и
не был в git — `tools/.gitignore`). Выходной гейт §11 шаги 0–4 измерены:
панель дважды зелёная (и теперь на шаг богаче — 6b `check-codegen`),
exhaustive clean **261 файл**, корпус **11 188 / 190 / 44 — 98.0 %**
(`ai-native` 98.3 %), CONVERGENCE **173 routed / 17 owed / 0 partial** —
все 17 на шести deferred-строках с рулингами (таблица закрытия в ledger),
`baseline.json` записан (2 221 юнит). A–D аудит отработал у гейта: 5
находок, 2 починены в-ране (шаг панели + quinn-proto), DBT-0023 зафайлен.
Тройка закрытия связана в §7.1: `755d664a` · `dcc23250` · `9c965514`.
**Фазы E/T/F/G спроектированы и НЕ стартуют без слова владельца.**

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — доложить и ждать

Фаза D закрыта; ничего дальше не авторизовано. Новая сессия:
(1) boot по контракту (CLAUDE.md → spec/boot/ → WAL) + два замера выше +
`bash tools/self-check.sh` → 0 (vibe-команды параллельно запрещены;
шаг 6b требует локальный бинарь jtd-codegen — рецепт в пакете
`tool:org.vibevm.ai-native/jtd-codegen`);
(2) доклад владельцу: гейт-панель закрытия, остаток (58 open non-sync +
32 deferred, owed 17 — все на стройках), кандидатные следующие шаги;
(3) **стоп — ждать слова**. Кандидат (не авторизация): мандат фазы E,
осушающий записанные стройки — источники: таблица закрытия
`campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md#close-2026-08-03`
(шесть строк → B-022/023/025/026/029/031/032/033/034), `BACKLOG.md`,
волны `TOOLING-MAP.md` — строго в рамке кампании.

## Формат подачи владельцу (оплачено дважды — 2026-08-02 и 2026-08-03)

Суть проблемы / решения / рекомендации — сначала простым языком, без
чтения спецификаций; жаргон спецификаций и кампании — только приложением;
пункты/строки спек не цитировать (владелец их не читает); точность
обязательна: настройка — именем, файл — путём, поведение — конкретно;
вопрос со структурой — деревом компонентов/решений; сначала ясно, потом
точные технические детали. Канон — WAL `##WAL-C-PRESENTATION-FORMAT`.

## Очередь владельца

Ничего блокирующего. На нём: слово на фазу E; открытые строки аудита
(`AUDIT.md` §2026-08-03: cargo-outdated неисполним на этой раскладке;
тень dead_code-подавлений 28 → 79 — триаж или accept следующим раном;
райдер о history-rewrite 2026-06-11 — третий ран подряд); DBT-0023
(quinn-proto в локе specspace fractality — правит его собственная
сессия); MT-02/MT-03 ручные подписи; стоячий known-issues список в WAL.

## Неочевидные находки этой сессии (сверх прежних)

- **`cargo xtask sync-engines --check` ловит и регенерацию генерата:**
  правка авторского движка (даже doc-комментариев в generated/) требует
  write-through по шести vendored-копиям — `cargo xtask sync-engines`,
  иначе панель красная. §3.5-интуиция «vendored догоняют релизом»
  относится к vibedeps-копиям, НЕ к in-tree crates/vendor/.
- **Шаг 6b в панели новый:** `cargo xtask check-codegen` теперь гейт;
  на машине без `tools/jtd-codegen/jtd-codegen.exe` падает действенно
  (рецепт — README пакета jtd-codegen; пин 0.4.1 живёт ТОЛЬКО там,
  `tools/jtd-codegen/README.md` — указатель).
- **`vibe progress baseline --campaign <зона>`** — штатный писатель
  baseline.json (не скрипт кампании).
- **`cargo outdated` неисполним на этом дереве** (temp-копия ломается на
  path-dep в исключённый пакетный workspace) — аудит-находка -03.
- **`json.dump` по registry-файлам:** debt.json — indent=2; чужой indent
  перекатывает весь файл (откачено в этой сессии, повторять не надо).
- **Новый README пакета входит в наблюдаемый корпус кампании** (260 →
  261): пишется сразу в house-грамматике (##ЯКОРЯ + @impl/done), иначе
  exhaustive-гейт упадёт.
- **PROP-014-якоря о схеме пережили переезд:** их confirmed стоял на
  deployment-периметре; референты резолвятся пакетно-локально.

## Где стоит работа

- Ветка `main`; роллаут — ТОЛЬКО `cargo xtask mirror` (сделан в конце
  этой сессии; при расхождении зеркал — investigate, не клоббер).
- `tools/self-check.sh` — all green (последний прогон — с шагом 6b).
- `vibe progress check --exhaustive --campaign campaigns/packages-2026-09`
  — clean, 261 файл. `--campaign` ВСЕГДА.
- cargo-audit 0.22.2 установлен (хостовый advisory закрыт:
  quinn-proto → 0.11.16); cargo-outdated 0.19.0 установлен, но
  неисполним (см. выше).

## Карта

`campaigns/packages-2026-09/`: PHASE-D-BATCH-PLAN.md (закон; §3.6/3.7/3.8
+ §6.1), PHASE-D-HOST-OBLIGATIONS.md (`#rulings-2026-08-01`,
`#rulings-2026-08-02-2`, **`#close-2026-08-03`** — таблица выживших),
harvest/, tasks/*.py (merge → seal НЕ сцеплять), tasks/evidence/batch-D29…D42,
run/mirror/. `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` —
статус в шапке (PHASE D CLOSED), §7 LOG с конца (запись 2026-08-03),
§7.1 commit map (#cm-d закрыт, D-close связан). `BACKLOG.md` B-001…B-047
(+`#map`; B-013 done). `TOOLING-MAP.md` — карта развития. `AUDIT.md` —
активные находки (секция 2026-08-03). `spec/WAL.md` — канон.

## Недавняя цепочка коммитов (закрытие)

```
9c965514 docs(campaign): phase D closes — the remainder, and who owns each row
dcc23250 chore(campaign): the phase boundary's baseline
755d664a feat(campaign): the routing record closes, and every survivor carries an owner ruling
c4e804e0 docs(audit): 2026-08-03 — A–D инвентаризация у гейта фазы D
1db359d0 chore(deps): quinn-proto 0.11.16 — закрыть RUSTSEC-2026-0185
1218c429 feat(tools): панель получает codegen-гейт (шаг 6b)
7441e7e9 chore(ai-native): sync-engines разносит правку движка по семье
7e1bdf74 docs(backlog): B-013 закрыт, карта отражает осушенную дыру
b4c48aa0 feat(ai-native): tool-пакет jtd-codegen — рецепт вместо бинаря
1ac1734e docs(campaign): D42 — F-279 резолвирован, owed падает до рулингов
ad3547f1 fix(ai-native): SHIPS-строка стека называет реальную поставку
a6bb261e fix(specmap): схема и codegen едут в пакет движка
```

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
bash tools/self-check.sh   # реальный exit-код; vibe-команды параллельно запрещены
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
