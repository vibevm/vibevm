# WAL — Project Continuation State {#root}

_Updated: 2026-08-20 (вторая сессия дня: **спланирован релиз 1.0.0** — все
рулинги владельца записаны, ТЗ и промт марафона написаны, обвязка живучести
компактификации установлена; марафон НЕ запущен, кода не трогали)._

@fact:WAL-READ-THE-PROMPT-FIRST **Порядок работы — `NEXT-SESSION-PROMPT.md`**:
это промт МАРАФОНА релиза 1.0.0 (слово владельца, авторизует исполнение
целиком); носитель работы —
`campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md`. Прежний промт (склад)
не отменён — склад стал слайсом С1 релизного ТЗ. @status:impl/done

@fact:WAL-HOW-TO-TALK-TO-THE-OWNER **Форма доклада владельцу (правило
2026-08-20).** Сперва **простыми словами**: в чём суть проблемы, решения,
рекомендации. Технические подробности — отдельным блоком после. **Номера
разделов и якорей спек не приводить** — владелец их не откроет. Точность
при этом не теряется: не «есть настройка в файле», а «файл такой-то, делает
то-то». Есть внутренняя структура — показать деревом. @status:impl/done

## Current phase {#current-phase}

@fact:WAL-PHASE **Релиз 1.0.0 спланирован, не запущен.** Владелец 2026-08-20
принял полный пакет рулингов (дословно — ТЗ §0): версия 1.0.0 у продукта И
у всех пакетов (CI/CD-довод, проверен практикой); `public = false` не
трогается (закрытая альфа ≠ публикация); фаза T ОТМЕНЕНА в кампании; G
расщеплена (вариант А едет релизом, полная — пост-1.0); дистрибутив только
Windows; шесть форматных развилок решены (B-091б, B-073-2, B-083-build,
B-084-narrow, B-085-retract, B-086-полная-лестница); B-079 «починить»;
подписи не нужны; транспорт claudez2→claudez до 5 воркеров на лаунчер.
Организация `vibespecs` на GitHub жива и ПУСТА (все репо удалены владельцем)
— публикация с чистого листа. @status:impl/done

@fact:WAL-MARATHON **Исполнение — одна автономная сессия («марафон»):**
`/goal` (текст в ТЗ §8), компактификация на 90% (настроена в
`~/.claude/settings.json`: env `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=90`), хуки
живучести в `.claude/settings.json` + `.claude/hooks/*.ps1` (SessionStart
compact|resume вбрасывает порядок перечитывания и живой git status;
PreCompact пишет `.claude/compact-log.txt`, гитигнорен). Сводка
компактификации = авторитет WAL, ниже файлов. @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-LAUNCH **Владелец читает ТЗ и запускает марафон** по
   ТЗ §8 (свежая сессия → `/goal`). Слайсы С0…С10; первый содержательный —
   С1, склад (нарезка ниже без изменений). @status:spec/plan
2. @fact:WAL-NEXT-STORE **Склад (С1, нарезка 2026-08-19/20 в силе):**
   `~/.vibe/cache/` (проектная `cache/` удаляется тем же коммитом); хит
   склада важнее молчащего реестра; семья `vibe cache …`; глобальный
   `--offline`; `check`/`--repair` лестницей; развод «обновление против
   смены источника». База: `harvest/prop010-current-state.md`. @status:spec/work
3. @fact:WAL-NEXT-OWNER **СТОП-ВЛАДЕЛЕЦ живых: один** — README-умолчание
   пути установки РЕШЕНО (`~/.vibe/opt`, правится слайсом С8); S2
   растворился (репо удалены). Остаётся: финальная инспекция дистрибутива
   и слово на тег `v1.0.0` (после С10). @status:impl/done

## Constraints — do not violate {#constraints}

- @fact:WAL-C-EPOCHS **`formats/EPOCHS.toml` не трогать никогда** —
  `public` флипает только владелец лично; тег НЕ есть публикация (по
  букве PROP-044). @status:impl/done
- @fact:WAL-C-ANCHOR-WITH-A-VERDICT-IS-NEVER-REPLACED **Якорь, у которого
  есть вердикт, не заменяют — надгробие с наследником.** Сверка дифом
  множеств якорей против HEAD перед посадкой всякой правки спеки. @status:impl/done
- @fact:WAL-C-EMPTY-OUTPUT-IS-A-CLAIM **Пустой ИЛИ ОБРЕЗАННЫЙ вывод есть
  утверждение.** Скорми инструменту случай, который он ОБЯЗАН различить;
  `ls | grep | head -5` уже стоил четырёх уехавших фактов. @status:impl/done
- @fact:WAL-C-EVERY-LANDING-FALSIFIES-THE-PREVIOUS-STATUS **Каждая посадка
  делает ложными утверждения предыдущей о сегодняшнем состоянии.** Счёт в
  прозе подстрока с именем предмета не находит — искать отдельно. @status:impl/done
- @fact:WAL-C-RECORDING-A-DECISION-FINDS-DEFECTS **Запись решения находит
  дефекты, которых не находит поиск.** @status:impl/done
- @fact:WAL-C-A-COUNT-CAN-BE-A-PROPERTY-OF-THE-TOOL **Счёт бывает свойством
  инструмента.** Всякое число, на котором стоит решение, меряют двумя
  способами. @status:impl/done
- @fact:WAL-C-THE-PANEL-MUST-COVER-WHAT-LANDS **Панель, запущенная до
  правки, не про то дерево.** @status:impl/done
- @fact:WAL-C-CODEGEN-DIES-ON-A-PIPE **`cargo xtask codegen` нельзя
  пропускать через `head`** — SIGPIPE стирает 22 файла; лечение
  `git restore crates/vibe-wire/src/generated/` (B-093). @status:impl/done
- @fact:WAL-C-MIRROR-CHECK-SAYS-IN-SYNC-OVER-A-BEHIND-HOST **`mirror
  --check` печатает `BEHIND <хост>` и следом «all targets in sync», код 0**
  (B-090) — читать строки, не хвост. @status:spec/plan
- @fact:WAL-C-GITVERSE-IS-FLAKY **GitVerse отвечает через раз** — повторить;
  отказ push'а на один хост НЕ расхождение. @status:impl/done
- @fact:WAL-C-A-PROMISE-IS-RE-MARKED-NOT-DELETED **Спека, обещающая
  непостроенное, чинится СТАТУСОМ, а не удалением требования.** @status:impl/done
- @fact:WAL-C-JUDGING-DEBT-READS-THE-MIRROR **Суд: scan → mirror → батч →
  merge → scan ещё раз → долг 0** — целиком, иначе проекция уедет коммитом
  позже. @status:impl/done
- @fact:WAL-C-JUDGE-SAME-PASS **Новый/правленый спек-файл размечен и осуждён
  тем же заходом.** `campaigns/**`, `BACKLOG.md`, `TASKS.md`, `spec/WAL.md`
  в судимый корпус НЕ входят; **`spec/boot/90-user.md` — ВХОДИТ**. @status:impl/done
- @fact:WAL-C-PANEL-STOPS-AT-FIRST-RED **Панель обрывается на первом красном
  шаге; зелёный хвост — единственное доказательство прогона всех.** @status:impl/done
- @fact:WAL-C-CARGO-TEST-FAIL-FAST **`cargo test` останавливается на первом
  упавшем ТАРГЕТЕ** (`--no-fail-fast`); **`cargo fmt` без `--check` выходит
  нулём и на грязном**; **красный СУЩЕСТВУЮЩИЙ тест при аддитивной правке —
  дефект правки**. @status:impl/done
- @fact:WAL-C-RED-BEFORE-GREEN **Страж доказывается КРАСНЫМ прогоном.** @status:impl/done
- @fact:WAL-C-LANDING-ORDER **Порядок посадки:** дифф → `cargo fmt --all` →
  `cargo xtask specmap` → стейдж → панель → суд → коммит →
  `cargo xtask mirror`. @status:impl/done
- @fact:WAL-C-DELEGATION **Транспорт воркеров:** `SUBAGENT-LAUNCHERS.md`
  читает БОСС ЦЕЛИКОМ до первого фан-аута; `claudez` = Claude Code на
  GLM-5.2; **рулинг 2026-08-20: приоритет claudez2 → claudez, до 5
  воркеров на лаунчер**; конфликтоопасные правки одним потоком; закрытый
  список записи называет файл, который правка ЛОМАЕТ; чекпойнт между
  пакетами — worktree на посаженную базу; `-c` сохраняет контекст. @status:impl/done
- @fact:WAL-C-SHELL **Shell-ловушки:** никогда голый `cd`; `cmd | tail;
  echo $?` печатает код ПАЙПА; правки только editor-инструментами; python —
  с `PYTHONIOENCODING=utf-8` и виндовым путём. @status:impl/done
- @fact:WAL-C-GIT **Git:** никогда `git add -A` — явные пути; коммиты
  heredoc `-F -`; раскатка только `cargo xtask mirror`; Rules 1–4 связывают
  каждый коммит. @status:impl/done
- @fact:WAL-C-VENDORED **Вендоренные копии не редактируются; движки
  дисциплины не трогаются** — кроме санкционированного С4.3 (B-064 +
  `sync-engines`). @status:impl/done
- @fact:WAL-C-AUTONOMY **Промт авторизует исполнение** — остановки только
  на СТОП-ВЛАДЕЛЕЦ по блокер-протоколу ТЗ §5.4; доклады статусов, не
  вопросы. @status:impl/done

@fact:WAL-EXPECTED-DIAGNOSTICS **Ожидаемые числа:** `vibe check` → 44 info
`manifest_epoch`, 1 warning `local_source_freshness`. **Слайс С5 сознательно
двигает 44 → 0** (epoch-маркеры) — единственное санкционированное движение;
любое другое движение — находка. @status:impl/done

## State {#state}

@fact:WAL-STATE **Состояние на чекпойнте** (команды главнее): `main` чист до
этой посадки; HEAD до неё — `88a847f6`; панель зелёная по прошлой сессии
(шесть прогонов), перепроверяется в С0; долг 0/0, 37 stale; воркеров нет,
`.wt/` пуст; `vibespecs` (GitHub) — org жива, репо 0; workspace version =
`0.1.0-dev`; реестр форматов — 20 записей. @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-RELEASE-TAKES **Строки, взятые релизным ТЗ:** B-091, B-073,
  B-078, B-072(замер), B-079, B-083…B-086, B-069, B-071(замер), B-047-пути,
  B-070+B-081, B-064, B-067(поглощена волной С5). Диспозиции проставлены в
  `BACKLOG.md`. @status:impl/done
- @fact:WAL-KI-B090 **`mirror --check` объявляет «in sync» над отставшим
  хостом (B-090, P2)** — не в релизе, читать строки. @status:spec/plan
- @fact:WAL-KI-B093 **`xtask codegen` уничтожает своё дерево при обрыве
  (B-093, P2).** @status:spec/plan
- @fact:WAL-KI-OLDER **Прежние строки без изменений:** B-074/075/076/082/
  088/092/094, B-054, B-068, три ребра к PROP-005, 37 stale, аудит
  `2026-08-06-01` (P1, пересуд — будущая кампания), шесть файлов на `work`
  ждут рулинга Phase E-гейта. Пост-1.0 инвентарь — `BACKLOG.md#post-1-0`. @status:spec/plan

## Session context {#session-context}

@fact:WAL-CTX-BOOT **Холодная сессия читает `CONTINUE.md` →
`NEXT-SESSION-PROMPT.md` (промт марафона) → ТЗ релиза
`campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md` (строка `_STATUS:` +
текущий слайс) → `TASKS.md`** — и берёт каждое число из команд. Этот файл
главнее `CONTINUE.md`; ратифицированные PROP-044/PROP-005/PROP-010 главнее
обоих. После компактификации хук вбрасывает тот же порядок сам. @status:impl/done
