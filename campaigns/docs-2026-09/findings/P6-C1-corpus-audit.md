# Аудит корпуса руководства — P6-C1 (месячная петля, п. 2)

Пакет: `PACKET-P6-C1.md`. Воркер вернул отчёт текстом (харнесс запрещает
ему файлы отчёта); файл записан центральной сессией дословно по существу,
без правок чисел. Бинарник `vibe 1.0.0` (`target/debug/vibe.exe`), дата
2026-09-12, пакет `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/`
(48 страниц на момент аудита; страница «How this manual is maintained»
легла в ту же сессию позже).

## Восемь чисел

| № | Показатель | Значение |
|---|---|---|
| 1 | команд верхнего уровня без страницы | 0 из 52 (без служебной `help`); второй уровень — 45 из 93 подкоманд ни разу не упомянуты в прозе |
| 2 | полей манифеста и лока, не названных нигде | 25 (13 из `vibe.toml`, 12 из `vibe.lock`) |
| 3 | видов пакетов без страницы или раздела | 0 из 8 |
| 4 | терминов глоссария в чужом смысле | 1 (`index`: индекс реестра против файла `INDEX.md`) |
| 5 | синонимов одного понятия | 2 (`manual`/`documentation`; `catalogue`/`index` реестра) |
| 6 | пар страниц-дублей (≥ 60 % общих `rule`) | 0 (ближайшая пара — 40 %) |
| 7 | страниц без единой прямой ссылки из прозы других страниц | 35 из 48 |
| 8 | превышений бюджета `llms` | 0 — бюджет в PROP-057 не назван числом |

## По пунктам

1. **Команды.** Верхний уровень — 100 % (у каждой команды `derived
   cli-help` в `reference/commands.xml`). Второй уровень без прозы: `doc`
   4/9 (`build-site`, `shell`, `surface`, `diff`); `registry` 3/11
   (`remove`, `redirect-sync`, `redirect-update`); `self` 10/15
   (`bootstrap`, `update`, `use`, `current`, `which`, `source`, `remove`,
   `gc`, `env`, `relocate`); `prefs` 2/7 (`migrate`, `ui`); `facts` 5/9
   (`list`, `get`, `set`, `rm`, `report`); `progress` 9/10; `aiui` 11/11;
   `extensions` 1/2 (`compile`). `mcp`, `cache`, `workspace`, `bin`,
   `skill`, `scrape` — 100 %. Третий уровень `--help` — X-046.
2. **Поля.** Снимок `vibe doc surface --record probe`: 221 поле манифеста,
   55 поля лока. Нигде не встречаются — манифест (13):
   `artifacts.build[].workdir`, `boot.default_link`, `boot_snippet.concepts`,
   `deploy.target[].depends_on`, `embedded_source[].license_path`,
   `embedded_source[].license_url`, `llm.default_model`,
   `llm.default_provider`, `mechanism[].config_schema`, `package.spec_format`,
   `project.spec_format`, `recommends`, `suggests`; лок (12):
   `meta.active_features`, `meta.virtual_capabilities[]` (+ `.emitted_at`,
   `.trace_id`), `package[].admitted_by`,
   `package[].embedded_source[].license_file_sha256` / `.license_path` /
   `.license_url` / `.tree_oid`, `package[].subskills_active[]`
   (+ `.cache_files`), `package[].via_override`.
3. **Виды.** Все восемь — в таблице `model/packages-and-kinds`.
   Наблюдение: авторские страницы «как написать» есть у `flow`,
   `feat`/`stack`, `lang`, `doc`; у `tool`, `mcp`, `app` — нет
   (`ship-tools-and-mcp-servers` учит подключать бинарники и серверы
   внутри пакета, не писать пакет этих видов с нуля).
4. **Глоссарий.** `index`: глоссарная статья — «index (of a registry)»,
   а на семи и более страницах то же слово означает файл `INDEX.md`
   (`model/boot-lane`, `start/what-a-project-contains`, `authoring/write-a-flow`,
   `write-a-lang-package`, `howto/install-a-package`, `work-offline`,
   `start/first-project`). `store`, `anchor`, `receipt`, `fingerprint`,
   `contribution` — чужого смысла нет. `fact` — историческая находка,
   уже исправлена (J-112).
5. **Синонимы.** `manual`/`documentation` — по всему корпусу, включая
   два заголовка страниц; `catalogue`/`index` реестра — на одной странице
   `model/registries` (первый абзац и секция «The index»).
6. **Лестница.** Десять самых частых терминов (`manifest` 33, `registry`
   28, `lock file` 23, `store` 18, `coordinate` 16, `fingerprint` 15,
   `specification` 12, `skill` 11, index реестра 10, `anchor` 8) впервые
   встречаются по порядку манифеста страниц на `agent/how-agents-read-this-manual`
   (№ 1), `architecture/how-vibe-is-built` (№ 2) и
   `architecture/what-the-lifecycle-epic-delivered` (№ 3); страницы
   новичка стоят далеко: `start/what-vibevm-is` — № 33 из 48,
   `start/index` — № 12, `start/first-project` — № 46. Ни один термин не
   вводится впервые на `start/*`.
7. **Дубли и мёртвые страницы.** Дублей нет. 35 из 48 страниц не
   получают ни одной прямой ссылки `[…](….xml)` из прозы других страниц
   (все они есть в навигации по манифесту); среди них `reference/commands`,
   `reference/manifest`, `model/registries`, четыре `howto/*`, почти весь
   `authoring/*`. Причина видна в `start/index` «After the route»:
   разделы названы курсивом без ссылок.
8. **Бюджеты `llms`.** `llms.txt` 2 778 слов; `llms-small.txt` 17 735
   слов (≈ 23 000 токенов); `llms-medium.txt` 70 380 (≈ 91 000);
   `llms-full.txt` 130 043 (≈ 169 000). `##SEO-LLMS-FILES` требует
   «under a token budget» без числа — сравнивать не с чем.
9. **Промпты.** 20 страниц, у каждой ≥ 1 `assert`; все глаголы
   (`check`, `mcp`, `skill`, `bin`, `doc`, `registry`, `cache`,
   `reinstall`, `deploy`, `deployments`, `list`, `self`, `--version`) есть
   в `vibe --help`; ассертов на несуществующие команды — 0. Не
   прогонялись (нужен агент).

## Решения центральной сессии (месячная петля, 2026-09-12)

- п. 6 лестница и п. 7 мёртвые страницы — одна причина: порядок
  манифеста страниц следует закону слоёв (стабильное раньше изменчивого),
  а навигация сайта берёт порядок манифеста как есть. Для агентов
  (`llms.txt`) порядок верен, для людей — нет. Правка: навигация для
  людей упорядочивает разделы по маршруту читателя (`start`, `model`,
  `howto`, `agent`, `authoring`, `lifecycle`, `reference`, `diagnostics`,
  `faq`, `glossary`, `architecture`) — долг `docs:` P2 с атомом в
  web-пакете; плюс ссылки в «After the route» (правка сейчас).
- п. 2 поля — долг `docs:` P2 на две таблицы справочника (25 полей).
- п. 1 подкоманды второго уровня — долг `docs:` P3: решается семействами
  (`self`, `facts`, `registry`, `doc`) прозой или блоками `derived`
  второго уровня; `aiui` и `progress` — инструменты разработчика, страница
  `architecture/traceability` или отдельная.
- п. 4 `index` — файл всегда пишется кодом `INDEX.md`; голое слово в
  смысле файла не найдено (проверено grep'ом по `the index`), коллизия
  живёт только в заголовке глоссарной статьи; наблюдение без действия.
- п. 5 `manual`/`documentation` — два слова с двумя значениями: «manual»
  — этот пакет, «documentation» — жанр и вид пакета; статья `manual` в
  глоссарии закрепляет это (правка сейчас). `catalogue` в первом абзаце
  `model/registries` — обычное слово вместо термина по закону первого
  абзаца; наблюдение без действия.
- п. 8 — вопрос владельцу: назвать бюджеты `llms-small`/`llms-medium`
  числом в PROP-057 или оставить «по здравому смыслу» (X-062).
