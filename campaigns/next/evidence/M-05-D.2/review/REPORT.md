# Независимый semantic/source review M-05-D.2

**Вердикт: оба исходных критерия поддержаны кодом и фактическими scoped checks. Конкретных P1/P2 blockers не найдено.** Финальное принятие, Git и изменение campaign count остаются у Root. Candidate667c8cd…/revision814 — self-report, не основание вердикта. Work пока Active/unreviewed;93/222 не меняется этим review.

## Проверенная цепочка

- `vibe-install/src/record.rs::configuration_record` проверяет выбранные SubskillManifest/content в captured provider до публикации configuration. `apply.rs` передаёт **текущий preflight_configuration** в `apply_resolution_with_configuration` и native counterpart. `hook_output.rs` проводит его в bootgen явно; готовящийся install не читает старый durable lock как новую selection. Нет TLS и скрытого изменения Manifest/ResolvedDep.
- `vibe-workspace/src/install/bootgen/effective_subskills.rs` связывает exact package coordinate/content с существующим D1 record. Выбранный путь должен иметь настоящий SubskillManifest с совпадающим canonical path, валидной delivery/description и объявленными content files. Наличие directory само по себе не создаёт export. Чтение идёт через safefs; captured declaration digest и bytes удерживаются вместе. Никакой связи по имени с SkillDecl не добавлено.
- Обычный и native bootgen действительно вызывают append_loading. Eager files попадают в static loading; lazy-push получает dynamic entry с typed SubskillOffer. `boot_artifacts.rs::prepare_index_body` реально выводит description и конкретный read_subskill request в INDEX. Lazy-pull boot entry не получает. Это реальная существующая поверхность context offer, а не fixture-only publisher.
- Native selected-source path удерживает отдельный SubskillManifest witness. `boot_artifacts/inputs.rs` перед компиляцией перепроверяет declaration digest/content bytes и overlay. Incumbent package.sources descriptor path остаётся для прочих источников; новый witness не объявляет произвольный одноимённый файл Vibe source. Subskill files находятся под subskills/, вне legacy materializer specs-root/README conversion; существующая `project_spec_text` проекция используется в compile owner.
- `vibe-mcp/src/tools.rs` actual QueryPackage/ReadSubskill/MaterialiseSubskill handlers используют новый derived view при наличии configuration record. Выключенный export недоступен через read; выбранный читается из lock-selected slot, а не случайного global cache. Новая materialise ветка не копирует файлы в project tree. Legacy reader остаётся отдельной совместимой веткой.
- Independent Agent Skills не фильтруются по subskill activation. Их реальные projection/world paths остаются прежними. Изменения selection меняют effective loading/visibility, не исходный package payload или его content identity.

## Критерии и доказательства

**1. End-to-end fixture наблюдает contribution activation:** install fixture проходит solver/bound RouteResolution/actual apply и проверяет реальный STATIC content, отсутствие lazy content в STATIC, INDEX offer, durable regeneration/analyzer. Native case вызывает обычный bound-native writer с явной текущей selection при отсутствующем old lock; деактивация удаляет contribution из результата. MCP case вызывает зарегистрированные tool implementations: query/read/no-copy, inactive refusal; одновременно выполняет настоящую Agent Skill projection с независимым именем. Это не только заполнение поля.

**2. Изменение activation сохраняет captured bytes:** install fixture сравнивает captured full-tree hash между active/inactive вариантами и каждый исходный файл с materialized counterpart, включая unselected. Отдельные workspace/native/MCP assertions сохраняют исходные content bytes после выбора/деактивации. Неравенство общего slot hash из-за generated boot files не скрыто: первый ошибочный assertion и его red run сохранены; финальная проверка сравнивает именно captured source payload.

Dormant-selection cases проверяют отсутствие feature/subskill/config effects при inactive guard, сохранение authored request/provenance/default absence и ошибку при удалённом dormant member. Это incumbent B2 evaluator, без нового solver.

Прочитаны реальные финальные log summaries: install2 + workspace2 + resolver2 + projection1 + MCP1 + native-group4 + orchestrator1 + descriptor1 =14 distinct passed cases. Native одиночный запуск повторно не суммировался. Четыре прежних nonzero checks сохранены. Reviewer ничего не запускал. Изменения только `subskills: Vec::new()` в прежних fixture constructors механические; существующие descriptor refusal cases включены в доказательства.

## Спецификация и границы

PROP-003 additions явно описывают full-slot derived view и supersession старого project/cache-copy описания только для этой ветки; B1/B2/D1 identity/default/request laws не удалены. Lazy-push здесь означает реальный offer в INDEX + получение content через MCP по решению читающего агента. Автоматический task-matching engine и live-agent демонстрация не реализованы и не заявлены; это соответствует уточнённому packet context-offer contract. Нельзя представлять эти14 cases как проверку поведения произвольной модели.

Exact configured selection использует существующий D1 формат; неизвестные/несогласованные records отказывают через incumbent decode. Неподтверждённый hostile concurrent slot mutation/cross-platform matrix не добавлялся как новая обязанность. Полная materialization remains independent of activation; generated boot outputs могут закономерно отличаться.

## Стабильность и scope review

Все30 файлов ROOT-REVIEW-SUBJECTS совпали с исходными hashes при проверке и в конце; changing view не обнаружен. REPORT.json/FINDINGS, current packet/два semantic answers, original task, governing D2 additions и immediate owner callees сопоставлены с реальным кодом. В REPORT.json рядом сохранён полный конечный hash set. Никаких source edits, builds/tests/models, runtime controls или Git mutations. Review не превращает прежние WIP в доказательство их исторической правильности: оценён итоговый код и наблюдаемые проверки.
