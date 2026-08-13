# WAL — Project Continuation State {#root}

_Updated: 2026-08-13, wind-down №19 (**PROP-044 РАТИФИЦИРОВАН владельцем;
все рулинги идентичности приняты и посажены** — LDH-грамматика, точечная
композиция, слот от идентичности с перематериализацией 37 слотов,
терминология snapshot/frozen/канал/copy, доверие+жизненный цикл+двухъярусный
publish+каналы в спеках; написаны два новых ТЗ строек и матрица покрытия;
вход следующей сессии переведён в режим исполнения. 21 коммит раскатан, долг
0/0, панель зелёная.)_

@fact:WAL-READ-THE-PROMPT-FIRST **Работа следующей сессии и её порядок — в
`NEXT-SESSION-PROMPT.md`** (переписан 2026-08-13): boot → четыре документа
целиком → короткий доклад → **сразу исполнение** (промт и есть слово
владельца). Главная полоса Ф0→Ф6; независимая S1→S3→S6. @status:impl/done

@fact:WAL-THE-LAW **PROP-044 — ратифицированный стоячий закон каждого
формата** («Ратификацию на сам PROP-044 даю», 2026-08-13; статус в файле).
Терминология §2b обязательна: snapshot ↔ frozen — антонимы; «канал» —
авторский указатель версий (PROP-005 §2.18); capture/named ref —
провайдерское; режим материализации — `copy`. @status:impl/done

@fact:WAL-THE-PLANS **Три плана строек, взаимно сцеплены:**
[`TZ-CHANGE-NATIVE-FORMATS-v0.1.md`](../campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md)
(главная стройка Ф0–Ф6; фазы разрешены ратификацией; СТОПы D11/D14 сняты
рулингами),
[`TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md`](../campaigns/packages-2026-09/TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md)
(S1 publish / S2 переименование `_`-репо СТОП-ВЛАДЕЛЕЦ / S3 joiner-юнит /
S4 каналы+S5 lifecycle заперты до Ф3 / S6 движки-по-данным / S7 пересуд;
§0 — карта гейтов, §1.4 — закон честной печати),
[`TZ-CHANGE-NATIVE-WAVE2-v0.1.md`](../campaigns/packages-2026-09/TZ-CHANGE-NATIVE-WAVE2-v0.1.md)
(§0 — матрица покрытия PROP-044; W1 манифест / W2 локфайл D9 / W3
frozen-поверхности / W4 не-каталожные+G12/G13; заперто до Ф1–Ф5; волна 3
осознанно не написана — триггеры записаны). @status:impl/done

@fact:WAL-NUMBERS-COME-FROM-COMMANDS **Числа воспроизводятся командами;
сперва rescan, иначе они отвечают про кэш:** @status:impl/done

```bash
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/judging-debt.py
python campaigns/packages-2026-09/tasks/text-stability.py
```

## Current phase {#current-phase}

@fact:WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`,
Phase E. Вопрос №1 владельца ЗАКРЫТ ратификацией PROP-044; стройки
расписаны тремя ТЗ и ждут свежей сессии исполнения (Ф0 — первая).** @status:impl/done

@fact:WAL-STATE **Состояние на чекпойнте** (команды главнее): корпус **281
файл, 0 неразмеченных**; долг **0 неосуждённых, 0 осиротевших**, 33 stale
(стоячий долг — кампания S7 по рулингу №4). `main` чист; зеркала синхронны
(HEAD сворачивания; `mirror --check`); панель зелёная (полный прогон —
слайс copy; после него только docs-коммиты). @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-EXECUTE **Свежая сессия исполняет по
   `NEXT-SESSION-PROMPT.md`**: Ф0 (три спайка БЕЗ коммитов → находки в
   `harvest/f0-*.md`; развилка только Ф0.1-бюджет) → Ф1 → … Независимая
   полоса при СТОПе: S1 → S3 → S6; S7 — судейская, в любой момент. @status:spec/plan
2. @fact:WAL-NEXT-OWNER **За владельцем:** S2 (переименовать `_`-репо в
   vibespecs — org-права, шаги в identity-ТЗ); вопрос №4 — кампания S7
   стартуема без него; №5 — строится S6. @status:spec/plan

## Constraints — do not violate {#constraints}

- @fact:WAL-C-PROP044-IS-THE-LAW **Вся идеология форматов — ратифицированный
  PROP-044**; здесь не пересказывается. Терминология §2b обязательна в каждом
  новом тексте и коде. @status:impl/done
- @fact:WAL-C-JUDGE-SAME-PASS **Новый/правленый спек-файл размечен и осуждён
  тем же заходом**: scan → mirror → batch JSON → `merge-verdicts.py`
  (правка существующего — `--force` с evidence «что заменяет и почему
  прежний был верен») → `seal` → долг 0. Имена `@fact:` ≠ якоря `{#…}`. @status:impl/done
- @fact:WAL-C-HONEST-SEAL **Печать — только за проверенное**: seal файла
  ручается за ВСЕ его вердикты; файл со стоячим stale не печатается без
  проверки каждого двинувшегося факта (прецедент отката — `498e8c8b`;
  правило — identity-ТЗ §1.4). `BACKLOG.md` вне судимого корпуса. @status:impl/done
- @fact:WAL-C-SPECMAP **Правка текста спеки двигает карту** — `cargo xtask
  specmap` в той же посадке; `--check` зелёный. @status:impl/done
- @fact:WAL-C-PLAN-IS-TEMPORARY **Планы временные**: посадка фазы = спек-дифф
  + могильник секции тем же коммитом; при закрытии плана ссылки на него
  уходят из спек (§11/§M планов). Содержимое не живёт в двух местах. @status:impl/done
- @fact:WAL-C-PHASE0-NO-COMMITS **Ф0 — спайки без коммитов**; находки в
  `harvest/`, дерево чисто. @status:impl/done
- @fact:WAL-C-DELEGATION **Транспорт воркеров**:
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` читает БОСС ЦЕЛИКОМ;
  пакеты собирает босс; воркеры не запускают git; диффы читает и коммитит
  босс. Native-агенты Claude — только по тесту проверяемости. @status:impl/done
- @fact:WAL-C-SHELL **Shell-ловушки**: cwd Bash-инструмента персистентен —
  абсолютные пути/`git -C`; правки только editor-инструментами; python — в
  файл, `PYTHONIOENCODING=utf-8`; реальные коды выхода; бюджет файла 600
  строк ПОСЛЕ `cargo fmt`. @status:impl/done
- @fact:WAL-C-GIT **Git**: никогда `git add -A` по всему дереву — явные
  пути; коммиты heredoc `-F -`; раскатка только `cargo xtask mirror`
  (fast-forward, никогда `--force`); Rules 1–4 связывают каждый коммит. @status:impl/done
- @fact:WAL-C-VENDORED **Вендоренные копии не редактируются**; движки
  дисциплины не трогаются; правка авторского движка ⇒ `cargo xtask
  sync-engines` отдельным шагом. @status:impl/done
- @fact:WAL-C-PRESENTATION **Подача владельцу**: сначала простой смысл,
  дерево для развилок, точные имена приложением. @status:impl/done
- @fact:WAL-C-AUTONOMY **Промт авторизует исполнение** — остановки только на
  СТОП-ВЛАДЕЛЕЦ (сводно: Ф0.1-бюджет; Ф1.3-локфайл-строка; S2-креды), по
  одной за раз; доклады статусов, не вопросы. @status:impl/done

## Done (collapsed — see `git log`) {#done}

@fact:WAL-DONE **2026-08-13, 21 коммит (`7a5d8f4b`..`6cd2f995`):** рулинги
идентичности приняты и посажены (LDH-грамматика + точечная композиция в 7
местах + слот от идентичности с перематериализацией 37 слотов + доверие/
жизненный цикл/двухъярусный publish/каналы/терминология в спеках) → суд
33+18+17+1 вердиктов с честной печатью (один слепой vouch откачен) →
`copy`-rename с отказом-рецептом → два новых ТЗ (identity-слайсы S1–S7;
волна 2 W1–W4 с матрицей покрытия) → **ратификация PROP-044** записана в 4
местах → вход сессии переведён в режим исполнения. Панель зелёная; седьмое
место композиции поймала именно она (vibe-check). @status:impl/done

## In progress {#in-progress}

@fact:WAL-INFLIGHT **Ничего в полёте.** Воркеров нет; дерево чисто; фаза Ф0
не начата (её начинает свежая сессия). @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-STALE **33 файла stale** (стоячий долг сдвигов байтов;
  крупнейший — PROP-043, 31 факт) — адресовано кампанией S7 (двухсортная
  планка, рулинг №4 2026-08-13); до неё живут видимыми. @status:impl/done
- @fact:WAL-KI-DOC-LEVEL **Document-level вердиктов ~4150 (host)** — предмет
  S7; правило теперь есть (планка по виду утверждения), работа не начата. @status:impl/done
- @fact:WAL-KI-B067 **B-067 растворён ратифицированным D14** (мутабельные
  версии легальны навсегда; бампы — события релиза) — строка бэклога ждёт
  могильника при ближайшей уборке; B-070→Ф4, B-071→Ф6.2, B-072→Ф2–Ф3 —
  растворяются посадками фаз. @status:impl/done
- @fact:WAL-KI-B074 **Второй якорь факта в абзаце глотается молча** (B-074)
  — механизм чека не построен. @status:impl/done
- @fact:WAL-KI-B075 **Панель может мигнуть на чистом дереве** (B-075) —
  читать отказ, не перезапускать вслепую. @status:impl/done
- @fact:WAL-KI-PHASE-E-SIX **Шесть файлов корпуса на `work`** — условие
  выходного гейта Phase E, ждёт рулинга. @status:impl/done
- @fact:WAL-KI-UPSTREAM-JOINER **Юнит `#modules` флоу addressable-specs всё
  ещё говорит «joiner never `.`»** — расхождение записано в PROP-029,
  чинится слайсом S3. @status:impl/done

## Session context {#session-context}

@fact:WAL-CTX-BOOT **Холодная сессия читает `CONTINUE.md` →
`NEXT-SESSION-PROMPT.md` → четыре документа предмета (PROP-044 + три ТЗ) →
транспортный закон воркеров при фан-ауте** — и берёт каждое число из команд
сверху, после rescan. Этот файл главнее `CONTINUE.md`; ратифицированный
PROP-044 главнее обоих. @status:impl/done
