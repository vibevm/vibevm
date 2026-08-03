# CONTINUE — cold-resume snapshot (2026-08-04, checkpoint: батч 2 волны Б ПОСТРОЕН)

**Не цитируй числа из этого файла — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот.

**TL;DR.** Один длинный автономный прогон **построил батч 2 волны Б целиком**
(15 зелёных коммитов). Движок получил `go-seam-error-cites-req` (обе половины),
`ts-seam-error-cites-req`, `go-conformance-assertion` (gated), два новых
`Fact`-варианта + kind `seam_error_message_no_req` + `[rust] floor_disable`;
экстракторы их эмитят (Go читает тела `Error()` и `var _ Seam = (*Impl)(nil)`;
TS ловит дискриминированные union-ошибки); **все три правила СМОНТИРОВАНЫ** и
показаны на фикстурах; **B-049** — Rust-floor чтит `floor_disable`; **принцип
паритета ПОДНЯТ** в манифест (`##PARITY-ACROSS-PROJECTIONS`). Панель зелёная на
каждой посадке. **НЕ закрыто (до конца батча 2):** S4-доки (гайды всё ещё
обещают построенное как «not built»), луп B-035 №2, пересуд семьи F-185.
**Зеркала на 15 коммитов позади** (роллаут — на явном wind-down).

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — закрыть батч 2 (мандат Б/В/Г стоит, паузы нет)

Порядок (S4 → луп → пересуд, затем батчи 3/4):

1. **S4 — гайды говорят про построенное (делегированный воркер вернул ПУСТО —
   переделать боссом или пере-коммишн).** Точки:
   - `GUIDE-AI-NATIVE-GO.md:192` (`##CONFORMANCE-IS-MADE-LOUD`, тег
     `@impl/plan`) → правило `go-conformance-assertion` теперь полицирует
     **gated**-ячейки; тег → `@impl/done`.
   - `conform-frontend-go.md:41` (`##RULE-SEAM-ERROR-CONTRACT`) → построено как
     `go-seam-error-cites-req`, обе половины; маркер message-половины =
     `spec://` ИЛИ `violates REQ`.
   - TS-гайд (клауза error-union `E`, ~:152/:159) → правило
     `ts-seam-error-cites-req` + ЧЕСТНЫЕ пределы (Form-1 union, error-позиция
     по имени, замкнутый `{kind,tag,_tag}`).
   - **Цитаты паритета:** три гайда (go/ts/rust) цитируют
     `spec://org.vibevm.ai-native/core-ai-native/00-MANIFESTO#parity-across-projections`
     (развилка №9 «стеки цитируют»).
   Пакет-черновик лежал: `scratchpad/packet-E12-S4-DOCS.md` (воркер по нему не
   справился — доки-правки по точным якорям; проще боссом). После посадки:
   sync-engines ×6 (гайды — канон пакетов) + vibe install + панель.
2. **Луп B-035 №2** — перекроить паритет-таблицу по факту дерева
   (`harvest/e10-b035-parity-pass.md` — проход №1; напиши №2). Закрываются:
   строка 1 (seam-error REQ-цитирование ×3), строка 7 (conformance: Go построен
   gated / Rust причина-компилятор / TS маршрут type-level-tests), строка 13
   (floor-disable ×3, B-049 закрыл инверсию). Открыто: строка 6 (Go flag-rule —
   батч 3), строки 8/12 (Go floor residual — B-048), record-reason строки
   9/10/11 (нарратив в гайды — часть S4).
3. **Пересуд семьи F-185** (B-033 докладывает семью) — ПОСЛЕ S4 (гайды должны
   говорить правду, иначе якоря F-185 остаются drift): `vibe progress mirror
   --campaign <zone>` → `merge-verdicts.py` → seal (НЕ сцеплять; seal берёт
   явные PATH'ы). Тогда реестр двинется.
4. **Батч 3** (B-036 позиция инвариант-комментов + B-037 кастомные REQ-линты +
   B-038 pending-карты; **развилка №1 карты computed-names приходит С B-038** —
   стоп владельцу по одной) → **батч 4** (B-025 mark-don't-suppress → последний
   якорь F-146; B-026 SARIF-ингест → F-206). Выход — **M-PARITY**.
5. **Волна В** (B-013 done → один формат-чейндж B-019а+B-016.1+B-017, B-024
   рядом → B-018.1/.2 → B-018.4+B-016.2 → B-020+B-021, B-014 там; B-020
   разблокирует четыре interim'а LEDGER-INTENT) — выход M-ASK+M-DRIFT. **Волна
   Г** оппортунистически (B-040 цензус снят, B-005, F-132, B-010-check).

## Что построено (карта посадки за прогон)

- **Движок (canonical v0.8.0, вендорится ×6):** `Fact::GoConformance {seam,
  impl_type, line, in_test}` + `Fact::TsSeamError {symbol, cites_req, line,
  in_test}` + новый `GoUnsafe` kind `seam_error_message_no_req`; правила в
  `rules/go_parity.rs` (`GoSeamErrorCitesReq` обе половины, per-half отпечатки;
  `GoConformanceAssertion` — **gated**-предикат: `new(cells_dir, gated)`,
  полицирует только gated-ячейки) и `rules/typescript_parity.rs`
  (`TsSeamErrorCitesReq`); `RustConfig.floor_disable: Vec<FloorDisable>`.
- **Экстракторы/мосты:** go-extract читает тела `Error()` (эмит
  `seam_error_message_no_req` при отсутствии `spec://`/`violates REQ`;
  **якорь — строка метода `Error()`**, не типа) и `var _ Seam = (*Impl)(nil)`
  (эмит `go_conformance`, near-misses исключены); ts-extract ловит Form-1
  union-ошибки + `cites_req` (JSDoc `@implements spec://` или `spec://` в
  члене); мосты — новые `RawFact` арма; PROTOCOL не бампнут (аддитив).
  `tools/go-extract/go.mod` создан (для `go test`; materialise независим —
  `include_str!` только extract.go + свой go.mod).
- **Драйверы:** go-драйвер монтирует `GoSeamErrorCitesReq` (всегда) +
  `GoConformanceAssertion` (условно cells_dir, gated); ts-драйвер монтирует
  `TsSeamErrorCitesReq` (всегда). Rust-floor чтит `floor_disable`
  (`STEPS`-словарь, печать отключений, hard-fail неизвестного).
- **Фикстуры/эксибиты:** clean go-фикстура greet получила seam `Greeting` +
  `var _ Greeting = (*Greeter)(nil)` (комплаентная ячейка); dirty plan —
  красный по обеим seam-half + conformance (gate dirty = **12**); голден
  `specmap.json` регенерирован. ts-фикстуры без union → ts-правило молчит.
- **Дисциплина:** манифест §4 несёт `##PARITY-ACROSS-PROJECTIONS` (+3 клаузы).
- **Кампания:** дизайн `spec/design/seam-error-and-assertion-parity.md`.

## Уроки прогона (в durable: WAL #constraints)

- **Вынос вида-находки из умбреллы в своё правило** ломает КАЖДЫЙ тест,
  считающий находки по правилу (gate-count, TCG-parity) — монтаж + правка
  счётчиков той же посадкой (`##WAL-C-CHARACTERIZATION-COUPLING`).
- **Новое gate-правило, требующее комплаентных образцов**, каскадит в фикстуры
  + init-шаблоны + голдены + тесты — планируй каскад. Специмап-голден:
  `run_specmap_go(root, false)` пишет (CLI нет — одноразовый bless-тест).
- **Предикат conformance = gated-ячейки**, не «каждая» (бесшовные/exempt вне).
- **Message-маркер = `spec://` ИЛИ `violates REQ`** (Go рендерит URI из поля).
- **Кэш экстракции** протухает по (контент, версия фронтенда) — правка ЛОГИКИ
  экстрактора внутри версии не инвалидирует; чисти `fixtures/*/target`
  (`##WAL-C-EXTRACTION-CACHE`).
- **Fact-ВАРИАНТ = рябь** (Rust FE сорт + три health-цензуса + мосты); Fact-KIND
  — нет.
- **Воркеры хорошо эскалируют суждение** — go-воркер поймал `spec://`-only и
  `Vec<String>`-противоречие; B-049 — `tests`→`test`; ts — почистил
  package-lock. Флаг воркера ревьюится как код (`##WAL-C-WORKER-JUDGMENT`).

## Где стоит работа

- `main`, **15 коммитов впереди зеркал** (роллаут — ТОЛЬКО `cargo xtask
  mirror`, на явном wind-down). Дерево чистое; воркеров нет; `.wt/E12-S4-DOCS`
  — handle-locked leftover (gitignored, git-prune чист, снести позже).
- Панель зелёная — «all green» хвостом (последний прогон S3+B-049).
- Реестр: 88/179, owed 6 — **UNCHANGED** (пересуд F-185 не запускался; двинется
  после S4→mirror→merge→seal). Меряй командами.
- Архив воркеров: `C:\Users\olegc\git\v\cache\agents\sorted\{E12-W1-SEAM-ENGINE,
  E12-S2A-GO-EXTRACT, E12-S2B-TS-EXTRACT, E12-B049-RUST-FLOOR, E12-S4-DOCS}\` —
  пакет+лог+отчёт+meta с вердиктом (S4 — empty, re-do).

## Недавняя цепочка коммитов (прогон, сверху — свежие)

```
94e6db0e chore(packages): vendor and rematerialise the batch-2 rule mounts
32aba0ab feat(rust-ai-native): the floor honours [rust] floor_disable
f63c1d32 feat(typescript-ai-native): the ts gate mounts the seam-error rule
d09e2a19 feat(go-ai-native): the go gate mounts the conformance-assertion rule
bd4291d5 feat(core-ai-native): the conformance-assertion rule scopes to the gate list
0393ce91 chore(packages): sync the extractor twins and rematerialise
a5ba2b0b feat(typescript-ai-native): ts-extract detects the seam-error union and its REQ citation
8f1fc914 feat(go-ai-native): go-extract emits the seam-error message half and the conformance assertion
c0f99902 chore(packages): vendor the parity engine into the copies and rematerialise
549677c8 fix(rust-ai-native): the fact sort and health census cover the new variants
736cdcf5 feat(core-ai-native): the Rust config gains a floor_disable twin
c5f14183 feat(go-ai-native): the go gate mounts the dedicated seam-error rule
ae927800 feat(core-ai-native): the seam-error and conformance-assertion rules join the engine
8e03348a docs(core-ai-native): the parity principle joins the manifesto
3c5f51e5 docs(design): the seam-error and assertion parity sketch on the E11 census pair
```

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
bash tools/self-check.sh   # exit — настоящий, хвостом; фоном — bare-форма
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
