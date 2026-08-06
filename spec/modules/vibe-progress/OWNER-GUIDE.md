# Progress Control — руководство владельца {#root}

<status stage="doc" state="done" action="drift" audience="dev" comment="владельческий гайд; жанр — guide, не контракт; fact grain 2026-07-24; S1 предшествует fact-поправке PROP-043 S3.8 items 4-6 — нет элементов списков, ячеек, ##-якорей (F-020)"/>

- @fact:guide-purpose Этот документ — для человека. Контракт системы — [PROP-043](PROP-043-progress-markup.md);
  план кампании — [SPEC-ACTUALIZATION-CAMPAIGN-v0.1](../../terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md). @status:doc/done
- @fact:guide-scope Здесь — как этим пользоваться, что смотреть и какие решения ждут лично вас. @status:doc/done
- @fact:guide-language Язык — русский, потому что аудитория этого файла — владелец проекта. @status:doc/done

---

## 1. Как читать маркеры в спеках

@fact:marker-reading Маркер — XML-тег в тексте. Читается как «стадия/состояние [+ что делать]»: @status:doc/done

```
<status stage="impl" state="work"/>
```

@fact:shorthand-examples — «реализация в процессе». То же самое сокращённо: `@impl` (state=work
подразумевается). `@test/plan` — «тестирование запланировано». @status:doc/done

@fact:vocab-lead Полный словарик: @status:doc/done

| stage | значит |
|---|---|
| @fact:ST-IDEA `idea` @status:doc/done | идея, ещё не специфицирована @status:doc/done |
| @fact:ST-SPEC `spec` @status:doc/done | пишем/написали спеку @status:doc/done |
| @fact:ST-IMPL `impl` @status:doc/done | реализуем/реализовано @status:doc/done |
| @fact:ST-TEST `test` @status:doc/done | тестируем/протестировано @status:doc/done |
| @fact:ST-DOC `doc` @status:doc/done | документируем/задокументировано @status:doc/done |
| @fact:ST-FREEZE `freeze` @status:doc/done | замораживаем (plan → work → done = заморожено; разморозка = смена маркера назад) @status:doc/done |
| @fact:ST-UNKNOWN `unknown` @status:doc/done | «смотрел и не понял» — явный запрос на триаж @status:doc/done |

| state | значит |
|---|---|
| @fact:SS-PLAN `plan` @status:doc/done | собираемся @status:doc/done |
| @fact:SS-WORK `work` @status:doc/done | делаем @status:doc/done |
| @fact:SS-DONE `done` @status:doc/done | сделали (для этой стадии) @status:doc/done |
| @fact:SS-HOLD `hold` @status:doc/done | сознательно отложено @status:doc/done |

@fact:optional-fields-lead Необязательные поля: @status:doc/done

- @fact:FIELD-ACTION `action` — вердикт «что делать»
  (`continue` — доделать; `drift` — разъехалось с реальностью, свести;
  `rework` — переделать; `remove` — убрать); @status:doc/done
- @fact:FIELD-ACTIONSTAGE `actionstage` — на какую стадию
  действует action (`remove`+`actionstage="doc"` = «удалить документацию»); @status:doc/done
- @fact:FIELD-AUDIENCE `audience` — для кого это документировать (`user` — пользователь vibevm,
  `author` — автор пакетов, `dev` — мы сами); @status:doc/done
- @fact:FIELD-COMMENT-REF `comment`, `ref` (ссылка на
  задачу DRIFT-NNN или spec://-анкер). @status:doc/done

@fact:placement-lead Куда можно ставить маркер — шесть гранулярностей (PROP-043 §3.8): @status:doc/done

- @fact:PLACE-DOCUMENT в преамбуле, до первого заголовка (весь документ). В файле без
  преамбулы — а это стандартная форма в этом репозитории — маркер сразу после первого
  заголовка и есть документный; @status:doc/done
- @fact:PLACE-SECTION отдельной строкой сразу после заголовка (секция) — кроме первого
  заголовка файла без преамбулы: там эта позиция занята документным маркером; @status:doc/done
- @fact:PLACE-PARAGRAPH первым или последним токеном внутри абзаца (абзац); @status:doc/done
- @fact:PLACE-LIST-ITEM последним токеном внутри элемента списка (элемент списка); @status:doc/done
- @fact:PLACE-TABLE-CELL внутри ячейки таблицы (ячейка); @status:doc/done
- @fact:PLACE-FRAGMENT парным тегом вокруг текста (фрагмент). @status:doc/done
- @fact:PLACE-FACT-ANCHOR Любая из этих единиц может нести якорь факта `@fact:<ID>` в начале —
  тогда она адресуема по `spec://…#<ID>`. Прежнее написание `##<ID>` значит ровно то
  же и по-прежнему читается, но пишется теперь первое. Закон anchored-when-marked:
  размеченный факт обязан быть заякорен; @status:doc/done
- @fact:STANDALONE-ERROR Одинокий маркер между
  абзацами — ошибка, инструмент его отвергнет. @status:doc/done

## 2. Ежедневные команды

@fact/code:DAILY-COMMANDS Команды ниже — **утверждение о том, что умеет
инструмент сегодня**, а не пример: забор входит в тело этого факта, поэтому
любая правка внутри него приводит факт к пересуду. Первая строка когда-то
утверждала обратное тому, что есть на самом деле, и прожила так долго именно
потому, что забор нельзя было осудить. @status:doc/done

```bash
vibe progress check --exhaustive   # валидация разметки; С 2026-08-06 стоит и в гейт-панели
vibe progress report --md      # статус дерева таблицей
vibe progress report --md --view todo        # что доделать
vibe progress report --md --view qa          # что тестировать
vibe progress report --md --view remove      # что удалить
vibe progress report --md --view doc --audience user   # оглавление user-доки
vibe progress weave --digest   # карта всего корпуса, влезает в контекст LLM
```

- @fact:HYGIENE-RULE Правило гигиены между кампаниями (одно): **правите юнит спеки — обновите его
  маркер в том же коммите.** @status:doc/done
- @fact:tool-guards Всё остальное караулит инструмент. @status:doc/done

## 3. Кампания: запуск, наблюдение, ваша роль

- @fact:campaign-home Кампания живёт в `campaigns/<id>/` (например `campaigns/progress-2026-08/`). @status:doc/done
- @fact:guide-side-pointer Все стадии, гейты и правила — в плане кампании; здесь — ваша сторона. @status:doc/done

### 3.1 Дашборд

```bash
node tools/progress-dashboard/serve.mjs    # затем открыть http://localhost:<port>
```

- @fact:DASH-RESUME Первый экран — Resume: что не завершено (красным), что дальше, свежесть
  состояния (жёлтая плашка = state давно не обновлялся — загляните, жива ли
  сессия). @status:doc/done
- @fact:DASH-CORPUS Дальше: Корпус (дерево файлов цветом по статусу), @status:doc/done
- @fact:DASH-STITCHING Сшивка (график
  открытых обязательств по волнам — линия обязана падать), @status:doc/done
- @fact:DASH-TASKS Задачи (чем занят
  Opus, что застряло в review). @status:doc/done
- @fact:DASH-READ-ONLY Дашборд read-only: он ничего не считает и
  ничего не может испортить. @status:doc/done
- @fact:dash-terminology (Терминология: эта страница — «дашборд», не
  «витрина»/«storefront» — те слова заняты витриной магазина vibevm.) @status:doc/done

### 3.2 Какие решения ждут лично вас (по стадиям кампании)

- @fact:OWNER-A **A (scaffold):** ратифицировать PROP-043; подтвердить имя зоны
  `campaigns/`; ничего больше. @status:doc/done
- @fact:OWNER-B **B (разметка):** выборочно читать диффы батчей — маркеры и сплиты, смысл
  текста меняться не должен. Сигнал тревоги: любой содержательный дифф. @status:doc/done
- @fact:OWNER-C **C (верификация):** ничего решать не нужно; полезно поглядывать на
  сводку X% confirmed / Y% drift — это первый измеренный уровень
  актуальности ваших спеков. @status:doc/done
- @fact:OWNER-D **D (сшивка):** к вам приходят только **эскалации** — пары документов, чей
  конфликт не сходится две волны. Это концептуальные развилки: нужен ваш
  вердикт, какая трактовка верна. Плюс все правки спек по мотивам
  sync-from-code показываются вам ДО применения — как и всегда в этом
  проекте. @status:doc/done
- @fact:OWNER-E **E (кодирование):** приёмка спорных PR после ревью Fable; вердикты по
  `remove`/`rework` спискам (удалять ли, отключать ли фичефлагом). @status:doc/done
- @fact:OWNER-F **F (планы):** три плана (release / улучшения / идеи) приходят к вам на
  утверждение приоритетов. @status:doc/done
- @fact:OWNER-G **G (документация):** вычитка глав двух гайдов — регистр и правда. Все
  примеры в доке уже реально исполнялись (это гарантия конвейера), ваша
  проверка — «то ли это, что я хотел сказать людям». @status:doc/done

### 3.3 Если сессия оборвалась (бюджет, питание, что угодно)

@fact:crash-recovery Ничего не чините руками. Новая сессия (любая — Fable или Opus) начинает с: @status:doc/done

```
прочитай campaigns/<id>/run/RESUME.md и продолжай по нему
```

- @fact:RESUME-CONTRACT RESUME.md сгенерирован журналом и говорит буквально: какой шаг не закрыт,
  какие файлы откатить (`git restore …`), что делать следующим. @status:doc/done
- @fact:MAX-LOSS-ONE-STEP Максимальная
  потеря при любом обрыве — один шаг (один файл разметки / один юнит
  верификации / одна задача). @status:doc/done

### 3.4 Перезапуск через месяц (и далее регулярно)

```bash
vibe progress rescan --baseline campaigns/<прошлая>/baseline.json
```

- @fact:RESCAN-TRIAGE Инструмент сам разложит корпус на «новое / изменившееся (перепроверить) /
  нетронутое (переносим вердикт)». Дальше — тот же цикл, но объёмом O(дельты):
  дни, не месяц. @status:doc/done
- @fact:FOUR-SURVIVORS Между кампаниями из зоны кампании хранятся только четыре вещи
  (PROP-043 §7.4): `baseline.json` (ускоритель перепроверки), `deferrals.md` (открытые
  хвосты), `harvest/` (сырьё для доки) и `tasks/` (корпус задач). Маркеры тоже
  переживают кампанию, но они живут в спеках — это корпус, а не зона. @status:doc/done
- @fact:ERASURE-SAFE Всё остальное можно
  стирать в любой момент — знание не теряется. @status:doc/done

## 4. Аварийные случаи

- @fact:EMERG-DISPUTED-MARKER **Маркер спорный / кажется неправдой** — правьте смело или ставьте
  `unknown`: маркер — это state-слой (как WAL), а не нормативный текст;
  ваша правка законна всегда. @status:doc/done
- @fact:EMERG-TOOL-FALSE-POSITIVE **Инструмент ругается на легальный, по-вашему, случай** — это баг
  инструмента или пробел PROP-043; фиксируйте как обычный баг, маркер
  временно допустимо сопроводить `comment="check false-positive: …"`. @status:doc/done
- @fact:EMERG-DASHBOARD-WEIRD **Дашборд показывает странное** — он лишь проекция; истина в
  `campaigns/<id>/run/state/*.json`, а выше неё — маркеры в спеках. Конфликт
  решается перегенерацией (`vibe progress scan`), никогда правкой JSON. @status:doc/done
- @fact:EMERG-ABANDON-SAFE **Хочется бросить кампанию посреди** — безопасно в любой момент: границы
  батчей закоммичены, RESUME.md всегда говорит, где вы. Возврат через месяц
  = п. 3.4. @status:doc/done
