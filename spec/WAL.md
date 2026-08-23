# WAL — Project Continuation State {#root}

_Updated: 2026-08-23 (чекпойнт перед компактификацией: **стройка PROP-050
«видимость зависимостей» ЗАВЕРШЕНА за один день** — W1–W8 посажены, PROP-050
в BUILT, панель зелёная; релиз 1.0.0 по-прежнему ждёт владельческой инспекции
и PAT для C6 — то состояние не менялось)._

@fact:WAL-HOW-TO-TALK-TO-THE-OWNER **Форма доклада владельцу (правило
2026-08-20).** Сперва простыми словами: суть проблемы, решения, рекомендации.
Технические подробности — после. Номера разделов/якорей спек не приводить;
точность не терять (не «есть настройка», а «файл такой-то, делает то-то»).
Режим `AGENT-MODE.toml` = `auto`: телеграфный трекинг; блокеры, развилки и
прямые вопросы владельца — всегда полным ответом. @status:impl/done

## Current phase {#current-phase}

@fact:WAL-PHASE-VISIBILITY **Стройка PROP-050 завершена (2026-08-23,
`342b3aae…aaeecc2e`).** Система видимости зависимостей живёт целиком:
per-edge `access` (public-дефолт / private / friends-only), `friend`
(false-дефолт; friends-only имплицирует свой грант — F10), `[visibility]`
(friends / unfriend / allow-friends / ignore-concept-warnings), `[override]`
(any-node, path-stack, ближний к корню побеждает, ТИХИЙ), per-edge `exclude`.
Движок C(R)/E(R) — `vibe_core::visibility` (analyze, joint fixpoint,
экзистенциальность по цепям); резолюция и lock (schema v6, `admitted_by` /
`via_override`) — на E(R) через проекцию + итерацию строгости
(`FilteringDepProvider`, солвер-ячейки нетронуты, differential-oracle зелёный);
ленты питаются суженным `ResolvedDep`-списком по построению (доказано 11
сквозными голденами: `cli_visibility_lanes.rs`, `cli_visibility_power.rs`);
гейт концептов смягчён по F7 (warnings, омонимия, дедуп, mute,
seeping-exemption); наблюдаемость: `vibe why`, `vibe friends`, closure-diff
на install/update, `[provenance]`-суффиксы в tree, панельная ячейка
`visibility_hygiene`; переразметка W7: fractality-группа private-метками,
хост-ленты byte-identical, расчётный эффект стороннего потребителя
delegation-first ≈32→2 пакета. Авторитеты: `spec/common/PROP-050-…md` (BUILT),
`spec/terraforms/VISIBILITY-BUILD-PLAN-v0.1.md` (приёмка построчно),
`spec/research/dependency-visibility-2026-08/` (prior-art + замеры). @status:impl/done

@fact:WAL-PHASE-RELEASE **Релиз 1.0.0 — ждёт владельца (состояние 2026-08-20
не менялось).** Все слайсы C0–C10 закрыты, кроме C6 (публикация): блокер —
новый PAT (ТЗ §10). Чеклист инспекции — `RELEASE-INSPECTION-CHECKLIST.md`;
дистрибутив на диске, два smoke PASS; `EPOCHS public=false`. Носитель —
`campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md` (строка `_STATUS:`). @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-OWNER **СТОП-ВЛАДЕЛЕЦ (оба пункта его):** (а) инспекция
   релиза + PAT → C6-волна → тег v1.0.0; (б) рулинги по остаткам стройки —
   BACKLOG B-103 (per-grant lane-cost + budget-cap, PROP-048-направление) и
   B-105 (`when = "concept:X"`), опция «wal через friends-only-у-redbook
   вместо прямого хост-ребра». @status:spec/plan
2. @fact:WAL-NEXT-DRAIN **Кандидаты следующей волны без владельца:** B-102
   (стоимость per-edge proof маски), B-104 (омоним ключа `override` с легаси
   `[[override]]`-пинами — ретирование). @status:spec/plan

## Constraints — do not violate {#constraints}

- @fact:WAL-C-EPOCHS **`formats/EPOCHS.toml` не трогать никогда** — `public`
  флипает только владелец лично; тег НЕ есть публикация. @status:impl/done
- @fact:WAL-C-ANCHOR-WITH-A-VERDICT-IS-NEVER-REPLACED **Якорь с вердиктом не
  заменяют — надгробие с наследником.** Дифф множеств якорей против HEAD
  перед посадкой правки спеки. @status:impl/done
- @fact:WAL-C-EMPTY-OUTPUT-IS-A-CLAIM **Пустой ИЛИ ОБРЕЗАННЫЙ вывод есть
  утверждение** — скорми инструменту случай, который он обязан различить. @status:impl/done
- @fact:WAL-C-EVERY-LANDING-FALSIFIES-THE-PREVIOUS-STATUS **Каждая посадка
  делает ложными утверждения предыдущей о сегодняшнем состоянии.** @status:impl/done
- @fact:WAL-C-A-COUNT-CAN-BE-A-PROPERTY-OF-THE-TOOL **Число, на котором стоит
  решение, меряют двумя способами.** @status:impl/done
- @fact:WAL-C-THE-PANEL-MUST-COVER-WHAT-LANDS **Панель, запущенная до правки,
  не про то дерево; вердикт — только по хвосту (`all green`).** @status:impl/done
- @fact:WAL-C-CODEGEN-DIES-ON-A-PIPE **`cargo xtask codegen` не пропускать
  через `head`** (SIGPIPE стирает generated; лечение `git restore`); **свап
  generated-дерева бьётся о Windows-локи** — `generated.new-*`-дебрис
  снести и перегнать (дважды за стройку). @status:impl/done
- @fact:WAL-C-MIRROR-CHECK-SAYS-IN-SYNC-OVER-A-BEHIND-HOST **`mirror --check`
  печатает `BEHIND` и следом «all in sync», код 0** (B-090) — читать строки. @status:spec/plan
- @fact:WAL-C-GITVERSE-IS-FLAKY **GitVerse отвечает через раз** — повторить;
  отказ одного хоста НЕ расхождение. @status:impl/done
- @fact:WAL-C-A-PROMISE-IS-RE-MARKED-NOT-DELETED **Спека, обещающая
  непостроенное, чинится статусом, не удалением требования.** @status:impl/done
- @fact:WAL-C-JUDGING-DEBT-READS-THE-MIRROR **Суд: scan → mirror → батч →
  merge → scan → долг 0 — целиком.** Юрисдикция кампании packages-2026-09
  факты `spec/common/PROP-050` НЕ покрывает (долг 0/0 при живых фактах —
  проверено скриптом); статусы стройки ведены вручную по посадкам. @status:impl/done
- @fact:WAL-C-PANEL-STOPS-AT-FIRST-RED **Панель обрывается на первом красном;
  зелёный хвост — единственное доказательство всех шагов.** @status:impl/done
- @fact:WAL-C-CARGO-TEST-FAIL-FAST **`cargo test` стопается на первом упавшем
  таргете; `fmt` без `--check` выходит нулём и на грязном; красный
  существующий тест при аддитивной правке — дефект правки.** @status:impl/done
- @fact:WAL-C-RED-BEFORE-GREEN **Страж доказывается красным прогоном.** @status:impl/done
- @fact:WAL-C-LANDING-ORDER **Порядок посадки:** дифф → `cargo fmt --all` →
  `cargo xtask specmap` → стейдж → панель → суд → коммит →
  `cargo xtask mirror`. @status:impl/done
- @fact:WAL-C-DELEGATION **Транспорт воркеров:** `SUBAGENT-LAUNCHERS.md`
  босс читает целиком до первого фан-аута. **Действующий приоритет лейнов
  (2026-08-20): `codexrunner` (gpt-5.6-sol/xhigh, YOLO — заборы только
  текстом пакета и боссовым ревью диффа) → `claudez`; `claudez2` ЗАПРЕЩЁН**;
  до 5 воркеров на лейн, cargo-тяжёлых — 2-3; конфликтоопасное — одним
  потоком; спека/план коммитятся ДО спавна читающего их воркера. @status:impl/done
- @fact:WAL-C-CODEX-STDIN **`codexrunner exec` читает stdin** — спавнить
  только с `< /dev/null`, иначе воркер виснет на «Reading additional input»
  (стоило одного фальстарта W1). @status:impl/done
- @fact:WAL-C-CROSS-WAVE-FIXTURE-DRIFT **Межволновой дрейф фикстур:** воркер,
  взятый с worktree до схемного бампа, пишет фикстуры старой схемы — lock-
  заголовки в тестах только через `CURRENT_SCHEMA_VERSION`, никогда числом
  (стоило трёх красных панелей за стройку). @status:impl/done
- @fact:WAL-C-SHELL **Shell-ловушки:** никогда голый `cd`; `cmd | tail;
  echo $?` печатает код пайпа; правки только editor-инструментами; python —
  `PYTHONIOENCODING=utf-8`; массовые замены в rust-строках с эскейпами —
  НЕ sed/replace вслепую, а Edit по прочитанному тексту (sed сломал
  format!-строку — красная панель). @status:impl/done
- @fact:WAL-C-GIT **Git:** никогда `git add -A` без явных путей; раскатка
  только `cargo xtask mirror`; Rules 1–4 связывают каждый коммит. @status:impl/done
- @fact:WAL-C-VENDORED **Вендоренные копии и движки дисциплины не
  редактируются** (кроме санкционированных волн + `sync-engines`). @status:impl/done
- @fact:WAL-C-B101-CACHE **B-101 жив:** правка авторского пакета без бампа
  версии не доезжает до слотов — `rm -rf ~/.vibe/cache/<группа>` перед
  переустановкой (применено на W7). @status:impl/done

@fact:WAL-EXPECTED-DIAGNOSTICS **Ожидаемые числа:** `vibe check` → 0 errors,
0 warnings, 0 info (оба WAL-предупреждения закрыты этим чекпойнтом); judging-debt: «no debt»; specmap: 0 suspects,
240 warnings (фон). Любое другое движение — находка. @status:impl/done

## State {#state}

@fact:WAL-STATE **Состояние на чекпойнте** (команды главнее): `main` чист,
HEAD `aaeecc2e`, зеркала синхронны (`mirror: all push targets synced`);
финальная панель `all green`; lock schema v6, 37 пакетов
(7 root-edge + 30 public-chain); ленты: `STATIC.xml` 248 429 B (byte-identical
через обе переустановки), `INDEX.md` 1 512 B; воркеров живых нет; `.wt/`
несёт СТАРЫЕ каталоги прошлых сессий (P2-*, R1-*, R2-*, R3-*) — стройка свои
шесть worktree снесла; архив воркеров стройки —
`C:\Users\olegc\git\v\cache\agents\sorted\W{1,2,3,4A,4B,5A,5B,6}-*/`
(логи+отчёты+meta). @status:impl/done

## done {#done}

- @fact:WAL-DONE-VISIBILITY **2026-08-23 — стройка PROP-050 целиком** (W1
  движок+словарь · W2 резолюция/lock на E(R) · W3 лейн-голдены · W4
  смягчение гейта концептов · W5 наблюдаемость · W6 инструменты силы ·
  W7 осознанное сужение · W8 закрытие с приёмкой построчно). @status:impl/done
- @fact:WAL-DONE-RELEASE **до 2026-08-20 — релизный марафон C0–C10 кроме C6**
  (история — в git log и ТЗ; здесь не накапливается). @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-BUILD-LEFTOVERS **Остатки стройки PROP-050 —
  `BACKLOG.md` B-102…B-105** (стоимость proof-маски; per-grant lane-cost и
  budget-cap; омоним ключа `override`; `concept:`-гейт). @status:spec/plan
- @fact:WAL-KI-B090-B093 **B-090** (`mirror --check` врёт «in sync») и
  **B-093** (`codegen` стирает дерево при обрыве; + Windows-локи свапа) —
  живы, P2. @status:spec/plan
- @fact:WAL-KI-B101 **B-101** (version-keyed кэш слеп к правке источника) —
  жив, P2; обход в констрейнтах. @status:spec/plan
- @fact:WAL-KI-OLDER **Прежние строки:** B-074/075/076/082/088/092/094,
  B-054, B-068, три ребра к PROP-005, 37 stale, аудит `2026-08-06-01` (P1),
  шесть файлов на `work` ждут рулинга. Пост-1.0 — `BACKLOG.md#post-1-0`. @status:spec/plan

## Session context {#session-context}

@fact:WAL-CTX-BOOT **Холодная сессия читает `CONTINUE.md` → этот WAL →
`spec/common/PROP-050-dependency-visibility.md` (если работа касается
видимости) ИЛИ `campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md`
(`_STATUS:`, если релиз)** — и берёт каждое число из команд. WAL главнее
`CONTINUE.md`; ратифицированные PROP главнее обоих. Пост-компакт-хук
вбрасывает порядок сам. @status:impl/done
