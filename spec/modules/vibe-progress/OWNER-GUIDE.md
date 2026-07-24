# Progress Control — руководство владельца {#root}

<status stage="doc" state="done" action="drift" audience="dev" comment="владельческий гайд; жанр — guide, не контракт; fact grain 2026-07-24; S1 предшествует fact-поправке PROP-043 S3.8 items 4-6 — нет элементов списков, ячеек, ##-якорей (F-020)"/>

- ##guide-purpose Этот документ — для человека. Контракт системы — [PROP-043](PROP-043-progress-markup.md);
  план кампании — [SPEC-ACTUALIZATION-CAMPAIGN-v0.1](../../terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md). @doc/done
- ##guide-scope Здесь — как этим пользоваться, что смотреть и какие решения ждут лично вас. @doc/done
- ##guide-language Язык — русский, потому что аудитория этого файла — владелец проекта. @doc/done

---

## 1. Как читать маркеры в спеках

##marker-reading Маркер — XML-тег в тексте. Читается как «стадия/состояние [+ что делать]»: @doc/done

```
<status stage="impl" state="work"/>
```

##shorthand-examples — «реализация в процессе». То же самое сокращённо: `@impl` (state=work
подразумевается). `@test/plan` — «тестирование запланировано». @doc/done

##vocab-lead Полный словарик: @doc/done

| stage | значит |
|---|---|
| ##ST-IDEA `idea` @doc/done | идея, ещё не специфицирована @doc/done |
| ##ST-SPEC `spec` @doc/done | пишем/написали спеку @doc/done |
| ##ST-IMPL `impl` @doc/done | реализуем/реализовано @doc/done |
| ##ST-TEST `test` @doc/done | тестируем/протестировано @doc/done |
| ##ST-DOC `doc` @doc/done | документируем/задокументировано @doc/done |
| ##ST-FREEZE `freeze` @doc/done | замораживаем (plan → work → done = заморожено; разморозка = смена маркера назад) @doc/done |
| ##ST-UNKNOWN `unknown` @doc/done | «смотрел и не понял» — явный запрос на триаж @doc/done |

| state | значит |
|---|---|
| ##SS-PLAN `plan` @doc/done | собираемся @doc/done |
| ##SS-WORK `work` @doc/done | делаем @doc/done |
| ##SS-DONE `done` @doc/done | сделали (для этой стадии) @doc/done |
| ##SS-HOLD `hold` @doc/done | сознательно отложено @doc/done |

##optional-fields-lead Необязательные поля: @doc/done

- ##FIELD-ACTION `action` — вердикт «что делать»
  (`continue` — доделать; `drift` — разъехалось с реальностью, свести;
  `rework` — переделать; `remove` — убрать); @doc/done
- ##FIELD-ACTIONSTAGE `actionstage` — на какую стадию
  действует action (`remove`+`actionstage="doc"` = «удалить документацию»); @doc/done
- ##FIELD-AUDIENCE `audience` — для кого это документировать (`user` — пользователь vibevm,
  `author` — автор пакетов, `dev` — мы сами); @doc/done
- ##FIELD-COMMENT-REF `comment`, `ref` (ссылка на
  задачу DRIFT-NNN или spec://-анкер). @doc/done

##placement-lead Куда можно ставить маркер: @doc/done

- ##PLACE-DOCUMENT до первого заголовка (весь документ); @doc/done
- ##PLACE-SECTION отдельной строкой сразу после заголовка (секция); @doc/done
- ##PLACE-PARAGRAPH первым или последним токеном внутри абзаца (абзац); @doc/done
- ##PLACE-FRAGMENT парным тегом вокруг текста (фрагмент). @doc/done
- ##STANDALONE-ERROR Одинокий маркер между
  абзацами — ошибка, инструмент его отвергнет. @doc/done

## 2. Ежедневные команды

```bash
vibe progress check            # валидация разметки (это же — в гейт-панели)
vibe progress report --md      # статус дерева таблицей
vibe progress report --md --view todo        # что доделать
vibe progress report --md --view qa          # что тестировать
vibe progress report --md --view remove      # что удалить
vibe progress report --md --view doc --audience user   # оглавление user-доки
vibe progress weave --digest   # карта всего корпуса, влезает в контекст LLM
```

- ##HYGIENE-RULE Правило гигиены между кампаниями (одно): **правите юнит спеки — обновите его
  маркер в том же коммите.** @doc/done
- ##tool-guards Всё остальное караулит инструмент. @doc/done

## 3. Кампания: запуск, наблюдение, ваша роль

- ##campaign-home Кампания живёт в `campaigns/<id>/` (например `campaigns/progress-2026-08/`). @doc/done
- ##guide-side-pointer Все стадии, гейты и правила — в плане кампании; здесь — ваша сторона. @doc/done

### 3.1 Дашборд

```bash
node tools/progress-dashboard/serve.mjs    # затем открыть http://localhost:<port>
```

- ##DASH-RESUME Первый экран — Resume: что не завершено (красным), что дальше, свежесть
  состояния (жёлтая плашка = state давно не обновлялся — загляните, жива ли
  сессия). @doc/done
- ##DASH-CORPUS Дальше: Корпус (дерево файлов цветом по статусу), @doc/done
- ##DASH-STITCHING Сшивка (график
  открытых обязательств по волнам — линия обязана падать), @doc/done
- ##DASH-TASKS Задачи (чем занят
  Opus, что застряло в review). @doc/done
- ##DASH-READ-ONLY Дашборд read-only: он ничего не считает и
  ничего не может испортить. @doc/done
- ##dash-terminology (Терминология: эта страница — «дашборд», не
  «витрина»/«storefront» — те слова заняты витриной магазина vibevm.) @doc/done

### 3.2 Какие решения ждут лично вас (по стадиям кампании)

- ##OWNER-A **A (scaffold):** ратифицировать PROP-043; подтвердить имя зоны
  `campaigns/`; ничего больше. @doc/done
- ##OWNER-B **B (разметка):** выборочно читать диффы батчей — маркеры и сплиты, смысл
  текста меняться не должен. Сигнал тревоги: любой содержательный дифф. @doc/done
- ##OWNER-C **C (верификация):** ничего решать не нужно; полезно поглядывать на
  сводку X% confirmed / Y% drift — это первый измеренный уровень
  актуальности ваших спеков. @doc/done
- ##OWNER-D **D (сшивка):** к вам приходят только **эскалации** — пары документов, чей
  конфликт не сходится две волны. Это концептуальные развилки: нужен ваш
  вердикт, какая трактовка верна. Плюс все правки спек по мотивам
  sync-from-code показываются вам ДО применения — как и всегда в этом
  проекте. @doc/done
- ##OWNER-E **E (кодирование):** приёмка спорных PR после ревью Fable; вердикты по
  `remove`/`rework` спискам (удалять ли, отключать ли фичефлагом). @doc/done
- ##OWNER-F **F (планы):** три плана (release / улучшения / идеи) приходят к вам на
  утверждение приоритетов. @doc/done
- ##OWNER-G **G (документация):** вычитка глав двух гайдов — регистр и правда. Все
  примеры в доке уже реально исполнялись (это гарантия конвейера), ваша
  проверка — «то ли это, что я хотел сказать людям». @doc/done

### 3.3 Если сессия оборвалась (бюджет, питание, что угодно)

##crash-recovery Ничего не чините руками. Новая сессия (любая — Fable или Opus) начинает с: @doc/done

```
прочитай campaigns/<id>/run/RESUME.md и продолжай по нему
```

- ##RESUME-CONTRACT RESUME.md сгенерирован журналом и говорит буквально: какой шаг не закрыт,
  какие файлы откатить (`git restore …`), что делать следующим. @doc/done
- ##MAX-LOSS-ONE-STEP Максимальная
  потеря при любом обрыве — один шаг (один файл разметки / один юнит
  верификации / одна задача). @doc/done

### 3.4 Перезапуск через месяц (и далее регулярно)

```bash
vibe progress rescan --baseline campaigns/<прошлая>/baseline.json
```

- ##RESCAN-TRIAGE Инструмент сам разложит корпус на «новое / изменившееся (перепроверить) /
  нетронутое (переносим вердикт)». Дальше — тот же цикл, но объёмом O(дельты):
  дни, не месяц. @doc/done
- ##FOUR-SURVIVORS Между кампаниями хранятся только четыре вещи:
  `baseline.json` (ускоритель перепроверки), `deferrals.md` (открытые хвосты),
  `harvest/` (сырьё для доки), сами маркеры в спеках. @doc/done
- ##ERASURE-SAFE Всё остальное можно
  стирать в любой момент — знание не теряется. @doc/done

## 4. Аварийные случаи

- ##EMERG-DISPUTED-MARKER **Маркер спорный / кажется неправдой** — правьте смело или ставьте
  `unknown`: маркер — это state-слой (как WAL), а не нормативный текст;
  ваша правка законна всегда. @doc/done
- ##EMERG-TOOL-FALSE-POSITIVE **Инструмент ругается на легальный, по-вашему, случай** — это баг
  инструмента или пробел PROP-043; фиксируйте как обычный баг, маркер
  временно допустимо сопроводить `comment="check false-positive: …"`. @doc/done
- ##EMERG-DASHBOARD-WEIRD **Дашборд показывает странное** — он лишь проекция; истина в
  `campaigns/<id>/run/state/*.json`, а выше неё — маркеры в спеках. Конфликт
  решается перегенерацией (`vibe progress scan`), никогда правкой JSON. @doc/done
- ##EMERG-ABANDON-SAFE **Хочется бросить кампанию посреди** — безопасно в любой момент: границы
  батчей закоммичены, RESUME.md всегда говорит, где вы. Возврат через месяц
  = п. 3.4. @doc/done
