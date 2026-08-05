# CONTINUE — cold-resume snapshot (2026-08-05, wind-down №12: волна Г закрыта, план осушается)

**Не цитируй числа отсюда — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py` ·
`python campaigns/packages-2026-09/tasks/text-stability.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот.

## TL;DR

**Все четыре волны карты закрыты.** Волна Г — последняя — закрылась 2026-08-05:
B-005 и B-010 были построены раньше, F-132 закрыт правкой ложного утверждения,
B-040 построен четырьмя посадками при одной отклонённой измерением.

Дальше владелец повернул сессию к **бэклогу**: сперва измерить, что в нём уже
построено, потом реализовывать. Замер дал **10 строк из 39, просивших построить
построенное**. Три строки построены в этот же заход. **Живых осталось 25 из 62.**

Рулинг владельца 2026-08-05, дословно: «бэклог — это ПЛАН. План не должен быть
источником истины для разработки… Когда всё в плане реализовано, план больше не
нужен». Отсюда: **118 статус-маркеров сняты** с плановых документов, и строка
теперь умирает вместе с коммитом, который делает её неправдой.

## Где стоит работа

- Ветка `main` @ `010d7104`, 38 коммитов за сессию. Дерево чистое кроме `.wt/`
  (два handle-locked остатка прошлых сессий).
- Панель зелёная — «self-check: all green», bare-форма, хвост прочитан.
  **В панели новый шаг** — `cargo xtask specmap --check` (B-014).
- `gitverse` синхронен; **`github` ОТСТАЁТ** — ssh заворачивается на
  `127.92.0.49`. Это **не расхождение**, форсить нельзя.
- Реестр: 11 575 подтверждённых, 182 drift, 0 stale, 0 к перепросуждению.
  Корпус 273 файла, `progress check` clean.
- Активного блокера НЕТ.

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — бэклог, и он в ТРЁХ домах

Владелец спросил «их два — общий и про инструменты?». Двух мало:

| дом | что это | сколько несделанного |
|---|---|---|
| `BACKLOG.md` | реестр находок, 62 строки | **25 живых** |
| `TOOLING-MAP.md` | порядок волн + 11 развилок владельца | все 4 волны закрыты; остаток — `##WAVE-PARKED` |
| `AUDIT.md` | находки аудита здоровья, 25 штук | **10 открытых**, последний прогон 2026-08-03 |
| `campaigns/packages-2026-09/deferrals.md` | хвосты кампании | 5 |

Итого **40 пунктов**. `AUDIT.md` — durable home находок аудита, его открытые
десять в бэклоге не дублируются.

### Группы живых строк бэклога (25)

1. **Дисциплина не наведена на себя** — B-002, B-004, B-060, B-061. Из них
   B-060 и B-061 уже честно записаны как «Specified, not built» с маршрутом;
   B-002 и B-004 не начаты. **Самая дешёвая группа и самая ядовитая:** каждая
   строка — причина, по которой гейт молчит там, где должен кричать.
2. **Паритет языков** — B-037 (Rust-половина dylint), B-044, B-050, B-051,
   B-052, B-053. Формулировка владельца: «мы не можем писать на Typescript и Go
   пока не поправим вот это».
3. **Карта и её потребители** — B-001, B-016, B-017, B-018, B-019, B-021,
   B-024.
4. **Направление, а не дыры** — B-007 (рулинг о жанре ADR), B-032, B-045,
   B-046, B-047.
5. **Ждёт владельца или внешнего** — B-008, B-015, B-020.

### Ось владельца: инференс против всего остального

Разрез 2026-08-05: **инференс требуется ровно двум строкам** — B-020 (объяснения
через внешние LLM, стоит на кредах) и B-042 (корпус, генерируемый LLM;
запаркована). Остальные 23 живут на статической стороне. Причина: **дерево
сегодня не запускает инференс вообще** — `InferenceBackend` имеет две
реализации, и обе перекладывают рассуждение на вызывающего агента
(`RelayBackend` паркует в `.vibe/agentic/`, `InlineBackend` возвращает
результатом MCP-вызова), а `vibe-llm` — девятистрочная заглушка до вехи v1.5.
**Безопасность (B-015) — в последнюю очередь**, решение владельца; она и не
может идти раньше канала, который охраняет.

## Что построено этой сессией

**Реестровый долг B-056** — 19 вердиктов, два файла запечатаны. Запись долга
была неверна дважды: считала 13 новых фактов, тогда как войти в реестр могли 8
(`campaigns/` — структурное исключение движка, `BACKLOG.md` не совпадает ни с
одним include-глобом); и говорила, что все десять сдвинулись флипом статуса —
девять да, а у десятого удалена фраза, и его прежним доказательством стоял
механизм, который работы не делает.

**F-132** — закрыт правкой ложной клаузулы PROP-014 §2.3, не постройкой.
Назначенная единица разметки не размечена и прочитана быть не может: 0 из 7
схем несут адрес, все сканеры сравнивают расширение буквально с `rs`/`md`, а
модель рёбер вешает адрес на СИМВОЛ кода, которого у JSON нет. Маршрут —
[B-060](BACKLOG.md#b-060).

**B-040** — дизайн [`spec/design/typed-seams.md`](spec/design/typed-seams.md) и
четыре посадки: `ValidatedOrg` (забытый вызов проверки области видимости больше
не компилируется), проверка идентификаторов на границе провода (пять значений в
дереве оказались не хешами), три обязательства строителя в сигнатуре
(`action.rs` 600 → 565 строк), шов `Watcher` назвал себя непостроенным.
**Пятая отклонена измерением**, и чтение ради отказа нашло пятистрочный дефект.
**Sealed traits — сознательный отказ** с архитектурной причиной.

**Три строки бэклога** — B-048 (TS-пол перестал линтовать чужие фикстуры),
B-059 (ключ исключения конформа работает так, как читается, и мёртвое
исключение объявляет себя), B-014 (у хостового `specmap.json` появился гейт).

## Не-очевидные находки

- **Опровергнутая фраза живёт во всех домах, где её пересказали.** У B-056 их
  было четыре; посадка починила два, эта сессия — ещё два. Внутри корпуса
  только два из четырёх.
- **Составной факт судится по одной из двух своих клауз.** PROP-014's правило
  было `confirmed` по двум ссылкам, и обе про первую половину предложения.
- **Два гейта, два списка изъятий, один никогда не гонялся.** `vibe-spec`
  изъят в `conform.toml` с причиной; `specmap.toml` её не отзеркалил, хотя сам
  говорит, что списки держатся в шаге. Нашлось при заведении гейта B-014.
- **Из трёх сканеров конформа разошёлся один.** TS и Go сравнивают с тем же
  путём, который хранят; Rust — нет. Это и был B-059.
- **Крейты стеков вендорятся** — mcp-пакеты зеркалят раскладку своего стека.
  Панель поймала это на `sync-engines --check` после того, как я утверждал
  обратное.
- **Усечённый пайп читается зелёным.** `cargo test --workspace | grep | head -40`
  спрятал строку `FAILED` за границей `head`; панель через десять минут была
  красной на 12 тестах.

## Закон транспорта поймал сам себя четырежды

Всё в `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`: греп маркера
совпадает с текстом самого пакета; форма запуска с `&` отбирает нотификацию,
ради которой написана; правило «mtime старше 5 минут = зависание» не может
сработать, потому что телеметрия размышления держит mtime свежим; терминальный
сигнал — событие `result`, а не `TASK-DONE`, которого воркер может не сказать
вовсе. Плюс: **хвост посадки приземляется в тех крейтах, которые пакет не
называл** — их проверяет босс рабочим-пространственным прогоном.

## Карта репозитория (верхний уровень)

- `spec/` — PROP/FEAT-контракты (`common/`, `modules/`), `boot/` (PROP-009),
  `design/` (рационали; `typed-seams.md` — дизайн B-040 с четырьмя
  записанными опровержениями), `terraforms/`, `WAL.md`.
- `campaigns/packages-2026-09/` — активная кампания: `harvest/` (в т.ч.
  `g5-backlog-truth.md` — замер 39 строк по дереву), `tasks/`
  (`summary.py`, `drift-registry.py`, `text-stability.py`,
  `merge-verdicts.py`, `evidence/`), `run/` (генерится; `run/mirror/`
  **gitignored**), `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml`.
- `packages/org.vibevm.ai-native/` — дисциплина: `core-ai-native/v0.8.0/`
  (движок, вендорится ×6 = 51 пара), `{rust,go,typescript}-ai-native-lang/`,
  `*-mcp/`. **Крейты стеков тоже вендорятся** — в mcp-пакеты.
- `crates/` — хост, 18 крейтов.
- Корень: `BACKLOG.md`, `TOOLING-MAP.md`, `AUDIT.md`, `TASKS.md`,
  `ROADMAP.md`, `specmap.json` (**теперь под гейтом**), `specmap.toml`,
  `conform.toml`, `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (байт-идентичны).

## Действующие решения (в силе)

- **План — не источник истины.** Статусы с плановых документов сняты; строка
  умирает вместе с коммитом, делающим её неправдой (рулинг владельца).
- **Инференс — за одной вехой.** Дерево не запускает моделей; две строки из
  25 по ту сторону; безопасность канала — последней.
- **Один закон — одна реализация**, и расхождение двух молчит по природе.
- **Ловить надо там, где жив провенанс.**
- **Помечать, а не гасить**; **сигнал, а не стена**; **лечи молчание**.
- **Бюджет 600 строк — нейтральный ключ**, меряет босс после `cargo fmt`.
- **BUILD-FIRST**; **T/F/G вне добра**; версии не бампать до публикации.
- **Делегация:** claudez-воркеры; вердикты, ревью и коммиты — босс.
- **Роллаут — ТОЛЬКО `cargo xtask mirror`**, fast-forward, никогда `--force`.
- **Движковую правку всегда сопровождает `cargo xtask sync-engines`** —
  отдельным шагом, не дожидаясь панели.

## Цепочка последних коммитов

```
010d7104 docs(backlog): three rows closed by the builds, with what each build found
60625d47 chore(packages): vendor the conform scanner fix across the six copies
48018a64 fix(conform): the exclusion key matches the path the finding prints, and dead ones speak
973fb56c chore(packages): vendor the floor perimeter fix into the mcp copy
c909dd78 fix(specmap): the host's own traceability index gets a gate, and it found two things
6e047882 fix(typescript-ai-native): the floor's two unscoped steps stop walking installed fixtures
c9cdf39d docs(backlog): the plan stops claiming what is, by the owner's ruling (B-062)
6d899e5d docs(backlog): a quarter of the plan was asking for work already built
cb8c34f2 chore(campaign): the B-040 design judged whole against what it built
89bd479c docs(design): ten of this design's own facts still said planned about built work
f9469422 docs(backlog): two dispositions offered the owner work that was already done
b790cc91 docs(map): волна Г closes whole, and it closed two of its four by correcting a claim
fe1fd532 docs(design): the digest newtype is declined, and the reading that declined it paid
a8fd3d86 fix(progress-core): a record with no processed_hash was projected as fresh
abc6d4c1 docs(tasks): волна Г closed whole, and with it all four waves of the map
df171094 docs(tasks): the Phase E exit gate measured, and the one ruling it needs
24784ca3 refactor(vibe-actions): the builder's three obligations move into the signature
0ed7fdaf refactor(vibe-core): four identity newtypes start checking at the wire boundary
805eec95 refactor(vibe-publish): the scope rule stops being a request to implementors
6b1d686b docs(campaign): head can hide a red run without touching the exit code
d9251b6d docs(campaign): the stall rule cannot fire, because mtime never goes stale
5e5ffc92 chore(campaign): a compound fact was confirmed on evidence for one of its clauses
cd562d55 docs(specmap): the designated taggable unit is designated and untagged (F-132)
940ca262 chore(campaign): the B-056 registry debt closed, and one re-judgement was not a re-stamp
c5db2eb8 docs(design): the B-040 seam refactor, shaped by what can be called wrongly
```

## Ждёт владельца

1. **Гейт выхода фазы E** — измерен: 273 файла, 267 `done`, 6 `work`. Шесть это
   три дизайна закрытых волн, два ручных теста и черновик PROP-010. Рулинг
   нужен потому, что **волны Б и В несут на карте `@doc/work`, а WAL называет
   их закрытыми**, и «волна закрыта» — не очевидно то же, что «её дизайн done».
2. **B-007** — жанр ADR: это ваш рулинг, а не стройка.
3. **B-024** — судьба `disputed`. **B-017** — содержание тира приватности.
   **B-020** — креды. **B-015** — безопасность канала (последней).

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/text-stability.py
bash tools/self-check.sh              # exit настоящий, хвостом; фоном — bare-форма
cargo xtask specmap --check           # НОВЫЙ шаг панели (B-014)
cargo xtask sync-engines              # после ЛЮБОЙ правки движка или крейта стека
cargo xtask conform check
cargo xtask mirror                    # раскатка, fast-forward-only
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
