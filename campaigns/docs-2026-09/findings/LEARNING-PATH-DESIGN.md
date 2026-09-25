# Учебный путь руководства — дизайн на ревью владельца {#root}

<status stage="spec" state="done" comment="M-015, 2026-09-25; дизайн узла DOCS-LEARNING-ORDER принят владельцем целиком в тот же день («со всем согласен»); норма — PROP-057 ##NAV-CHAPTERS и соседние факты"/>

## 1. Мандат {#mandate}

Владелец, 2026-09-25: порядок пунктов руководства VibeVM, в русской и в
английской версии, хорош с точки зрения таксономии и онтологии, но плох как
учебник: учебник начинается с глубокого обзора архитектуры — как если бы физику
шестого класса начинали с топологии чёрных дыр, — а глоссарий, который в
учебниках ставят в конец, стоит в начале. Предложено хранить ещё один порядок,
обучающий, и отображать руководство в нём; пилот — официальное руководство,
русское и английское издания.

## 2. Факты (проверены 2026-09-25) {#facts}

- Порядок `pages` в манифесте страниц задаёт **закон слоёв** (PROP-048
  `##THE-LAYER-LAW`, `crates/vibe-doc/src/manifest/layer.rs:95`): сначала
  страницы с меньшей долей блоков из продукта (`derived`, `example`,
  `example ref`), при равенстве — по алфавиту адреса. Это машинный порядок:
  стабильный текст перед подвижным ради кэша промптов; им же упорядочены
  `llms.txt`, `llms-full.txt` (PROP-057 `##SEO-LLMS-FILES`).
- Боковая колонка сайта берёт этот же порядок: `pinned` из `[navigation]`
  первыми, дальше разделы по первому появлению их страницы
  (`site/src/lib/contents.ts:154`). Строки `[[navigation.section]]` — только
  словарь заголовков (`titleOf()`), их порядок не читается никем.
- Итог для читателя (одинаков в EN и RU, 49 страниц в каждом издании): после
  двух закреплённых страниц — Agent, Architecture, Authoring, Diagnostics,
  Questions, Glossary, Reference, Model, How to, Start, Lifecycle; «Install
  vibe» — 32-я страница манифеста, «Create your first project» — 47-я.
- Пейджера «назад/вперёд» нет; альтернативных порядков нет нигде
  (`crates/`, сайт, спеки).
- Издания — отдельные пакеты; перевод объявляет `[translates]`, страницы
  совпадают путь в путь (`##LOC-MIRROR`); заголовки разделов перевод даёт свои
  строками `[[navigation.section]]`.
- Настройки читалки — закрытый список из четырёх (`##READER-SETTINGS`): тема,
  кегль, ширина, якоря.
- Записи глоссария не хранят страницу, вводящую термин, поэтому «лестница
  терминов» между страницами машинно не проверяема; межстраничных ссылок в
  корпусе всего 20 (остальное — ссылки в глоссарий).

## 3. Решение: два порядка — две аудитории {#two-orders}

Закон слоёв остаётся машинным порядком: `pages`, семейство `llms`, MCP. Для
людей пакет документации МОЖЕТ объявить **учебный путь** — главы по порядку,
в каждой страницы по порядку. Сайт и локальная читалка показывают путь, когда
он объявлен; пакеты без пути выглядят как сейчас.

## 4. Развилки и рекомендации {#forks}

| # | Развилка | Рекомендация | Отвергнуто и почему |
|---|---|---|---|
| A | Заменить закон слоёв или добавить второй порядок | добавить: `pages` не трогаем | замена ломает экономику кэша агентов (PROP-048); жёсткий список разделов в библиотеке PROP-048 уже отверг — он зашил бы ИА одного руководства во все пакеты мира |
| B | Как задаётся путь | объявляется автором списком глав в манифесте | вывод из пререквизитов топосортом — валиден, но не педагогичен и требует метаданных на каждой странице; веса в страницах — порядок не виден без открытия всех страниц, перенумерация |
| C | Формат | `[[navigation.chapter]]`: `id`, `title`, `pages = [...]` в манифесте исходного издания; перевод даёт только `id` и `title` (как у разделов) | копия структуры в каждом издании — два экземпляра одного факта разойдутся |
| D | Проверки | ошибки `vibe check`: страница пути существует, стоит ровно один раз, все страницы пакета покрыты, `id` глав уникальны, перевод не объявляет страниц глав; отчёт `vibe doc check`: ссылки вперёд по пути вне приложения — измеряются, не блокируют | гейт на ссылки вперёд — ложные тревоги: страницы-указатели законно смотрят вперёд (сейчас все 13 таких ссылок — с двух ориентирующих страниц) |
| E | Показ | колонка по умолчанию — путь (главы с номерами); переключатель «По порядку / По разделам» наверху колонки, выбор помнится (пятая настройка читалки); внизу страницы пейджер «← назад / вперёд →» по пути; на странице пакета — главы и «Начать отсюда» | путь без переключателя — таксономический вид исчезнет из интерфейса; пейджер по активному виду — два набора ссылок и клиентская логика ради редкого случая |
| F | Объём пилота | формат общий, объявляет путь только официальное руководство (EN + RU) | — |

## 5. Предлагаемый путь {#path}

| Глава (EN / RU) | Страницы |
|---|---|
| 1. Getting started / Первые шаги | start/what-vibevm-is, start/index, start/install-vibe, start/first-project, start/what-a-project-contains |
| 2. How a project works / Как устроен проект | model/two-trees, model/boot-lane, model/packages-and-kinds, model/registries |
| 3. Everyday work with packages / Повседневная работа с пакетами | howto/install-a-package, model/lock-and-store, model/versions, howto/update-packages, howto/remove-a-package, howto/work-offline, howto/use-a-private-registry |
| 4. Working through your agent / Работа через агента | agent/give-your-agent-the-skill, agent/ask-your-agent, howto/read-documentation-locally, agent/how-agents-read-this-manual |
| 5. Build and ship a project / Сборка и выкладка проекта | lifecycle/phases, lifecycle/build-package-deploy, lifecycle/extensions-and-providers, lifecycle/scrape |
| 6. Writing packages / Свои пакеты | authoring/write-a-flow, authoring/specs-agents-can-cite, authoring/facts-and-status-markers, authoring/write-a-feat-or-stack, authoring/write-a-lang-package, authoring/ship-tools-and-mcp-servers |
| 7. Beyond one package / За пределами одного пакета | model/dependency-visibility, authoring/bridge-a-repository, howto/set-up-a-workspace, howto/publish-a-package |
| 8. Documentation as a package / Документация как пакет | authoring/write-documentation, authoring/translate-documentation, architecture/how-this-manual-is-maintained |
| 9. Under the hood / Под капотом | architecture/how-vibe-is-built, architecture/traceability, architecture/what-the-lifecycle-epic-delivered |
| 10. Appendices / Приложения | reference/commands, reference/manifest, reference/lock-file, reference/tree, reference/settings-and-environment, reference/machine-formats, diagnostics/errors, faq/index, glossary/index |

49 страниц, каждая ровно один раз. Ритм глав 2–3: понятие → действие
(установить пакет → что такое lock и store → версии → обновить). Глоссарий —
последняя страница.

## 6. Замеры {#measures}

Скрипт `scratchpad/order_metrics.py` сессии 2026-09-25 (перенесётся в
проверку `vibe doc check`):

| | Сейчас (колонка сайта) | Предлагаемый путь |
|---|---|---|
| Ссылок вперёд вне приложения | 18 из 20 | 13 из 20, все — с двух ориентирующих страниц |
| Новых терминов глоссария за первые 5 / 10 / 20 страниц | 13 / 29 / 36 | 11 / 23 / 28 |

## 7. Работы после ответа владельца {#route}

1. Норма: поправка PROP-057 (`##NAV-CHAPTERS…`, запись решения из четырёх
   полей, расширение `##READER-SETTINGS`), facts check и specmap.
2. Конвейер (пакет `opus5`, High): схема `doc_manifest.jtd.json` и codegen,
   разбор `[[navigation.chapter]]`, проверки `vibe check`, отчёт ссылок вперёд
   в `vibe doc check`, тесты.
3. Читалка (пакет `opus5`, High): колонка-путь, переключатель, пейджер,
   страница пакета, настройка, строки интерфейса EN/RU, тесты и e2e.
4. Руководство (центральная сессия): главы в двух манифестах, страница
   `authoring/write-documentation` и её адаптация рассказывают о главах.
5. Гейт: сборка сайта, floor, e2e, скриншоты; коммиты без трейлеров, mirror;
   выкладка — только словом владельца.
