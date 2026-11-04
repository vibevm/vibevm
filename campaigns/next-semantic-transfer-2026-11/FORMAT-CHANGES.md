# Изменения форматов и поведения для будущего changelog

Журнал начат по указанию владельца 2026-11-04. Он фиксирует изменения `vibe.toml`, связанных форматов и их потребителей, включая `vibe install -g`. Это материал для release notes, а не утверждение о выпущенной версии. Канонические требования находятся в соответствующих PROP; реализация и проверки — в указанных коммитах/feature reports.

## Версии и статусы

- База main перед переносом: `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`.
- Исследуемый prototype: ветка `1.1.0`, commit `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Имя ветки не означает выпуск этих изменений в основной продукт.
- В текущем main `Cargo.toml` всё ещё указывает **1.0.11**. Версия следующего релиза этой кампании **не назначена**. Все записи ниже относятся к **Unreleased** и привязаны к коммитам. Их нельзя автоматически включать в описание уже выпущенной 1.0.11.
- Статусы: **принято в main** — код и указанные проверки завершены; **в разработке** — кандидат ещё не принят; **только дизайн** — формат ещё не реализован. При принятии/смене формы запись обновляется; изменённые решения сохраняются в хронологическом LOG.md.

## FMT-001 — App features: исправленная семантика без нового синтаксиса

Статус: **принято в main**, `9f3a271ef`, общий kernel подключён в `c20b6bc89`.

Формат: существующие `[[installer_options.features]]`, `[[installer_options.profiles]]`, поля `parent`, `requires`, `required`, `default`, `exclusive_group`. Новых TOML-ключей нет.

Раньше результат мог зависеть от порядка feature declarations и имён exclusive groups; flags поверх профиля могли ошибочно конфликтовать; замена ветки могла молча удалить прямо выбранного потомка. Теперь parent/requires/required замыкаются до defaults, profile/prompt служат базой для прямых flags, а несовместимые прямые требования дают отказ вместо потери выбора.

Команды: app planning/install/update/modify, включая **`vibe install -g`**. `--feature`, `--no-feature`, `--profile` сохраняют синтаксис. Например профиль с gtk плюс `--feature qt` выбирает qt; `--feature parent-b --feature child-a` отказывает, если child-a требует взаимоисключающий parent-a.

Переход: корректные manifests переписывать не нужно. Если пакет зависел от случайного порядка или молчаливого удаления прямого ответа, нужно устранить противоречие. Existing app describe/plan wire и installation index не менялись. Evidence: F04-common-configuration.md, исходные5RED и corrected8/8 + public wizard.

## FMT-002 — Package features: полная локальная проверка и set-семантика exclusive

Статус: **принято в main**, `5af2eadc8`.

Формат: прежние `[features]` со списками строк и `[features.exclusive]`. Нового синтаксиса нет; literal quoted IDs со spaces сохраняются без нормализации. Общий kernel используется и app, и package consumer, но package default+explicit по-прежнему объединяются; app-style replacement сюда не переносится.

До: validator включал все alternatives и ошибочно отвергал валидный exclusive definition; `g=["a","a"]` считалось двумя выбранными вариантами; dormant/private unknown local references и undeclared exclusive members могли игнорироваться. После: definition проверяется целиком отдельно от actual default request; exclusive — множество; invalid dormant references отказывают.

Пример без миграции:

```toml
[features]
default = ["a"]
a = []
b = []
[features.exclusive]
mode = ["a", "b"]
```

Эта декларация валидна; явный `--all-features` для неё может по-прежнему конфликтовать.

Команды: `vibe check` и package install feature expansion. Это не изменение installer-option flags у **`vibe install -g`**: его app feature namespace описан в FMT-001. Связь package effects с solver/boot пока не объявлена готовой.

Переход: удалить/исправить неизвестные ссылки даже в неактивных private branches; повторяющиеся exclusive members допустимы и дедуплицируются. No lock/wire schema change. Evidence: F04 report,5originalRED,120candidate tests/docs и integrated28resolver+38configuration.

## FMT-003 — Secret app parameters: формат прежний, вывод и ввод исправлены

Статус: **принято в main**, `d0c961277`.

Формат: прежние `[[installer_options.parameters]]` и `secret = true`; `--set ID=VALUE` не менялся. `type`, typed defaults, bounds, choice lists и required semantics сохранены.

До: ошибки могли раскрывать raw/canonical answer, resolved path, opaque prompt failure; Debug показывал marked value. Secret bool/choice могли попасть в видимый widget. После: classified errors и Debug скрывают ответ, сохраняют parameter/option IDs и safe reason; secret prompt выбирает hidden input для любого type. Пустой hidden input может сохранить имеющийся preset.

Команды: `vibe app plan`, **`vibe install -g`**, app update/modify и связанные prompts. Public nonsecret errors сохраняют подробности. Authored defaults/choices являются публичными данными определения и остаются в describe. Secret ответы по-прежнему не пишутся в обычную installation parameter map.

Переход: изменения manifest не требуются. App machine documents и index protocol не менялись. Это не обещание исправления secret interpolation в templates или raw argv в hosted resume; они остаются отдельной работой. Evidence:7реальных CLI human/JSON tests,23candidate tests/docs; Password branch проверен source/compile, без PTY claim.

## FMT-004 — Common configuration в обычном vibe.toml

Статус: **принято в main как admission/validation этап**, source design `d16fb59c8`; acceptance commit — `da741ee16` (`feat(manifest): retain common configuration definitions`). Версия выпуска не назначена, **Unreleased**. Поддержка чтения/проверки определения не равна готовой установке; переходить всем пакетам на этот формат пока рано.

### Точные новые формы

| Место | Поля и значение |
|---|---|
| `[configuration]` | Optional package definition. Project-only/virtual-workspace node не может владеть package definition. Пустая, но присутствующая таблица отличается от отсутствия. |
| `[installer_options.configuration]` | Та же definition type для конкретного installer option; допускается с installation shorthand и со steps. Package scope остаётся отдельным. |
| `[[configuration.features]]` | id/name; optional description/parent; required/default=false; requires/guards/effects=[] по умолчанию. Literal IDs без trim/normalization. |
| Guard | Ровно `{group="ID"}` или `{member="ID"}`; положительные зависимости проверяются на полный DAG, включая dormant branches. |
| `[[configuration.groups]]`, mode=`choice` | id/name/optional description/cardinality/members; optional defaults (отсутствие и [] различаются); guards. Не принимает fallback. |
| `[[configuration.groups]]`, mode=`closure` | id/name/optional description/cardinality/members; optional fallback. Не принимает choice defaults/guards. |
| Cardinality | `zero-or-more`, `one-or-more`, `zero-or-one`, `exactly-one`. |
| Effect `kind="dependency"` | target реальной qualified requires declaration. |
| Effect `kind="forward"` | target, feature, strength=`strong` или `weak`. Source declaration связывается сейчас; foreign feature delivery ещё не реализуется этим parser. |
| Effect `kind="subskill"` | path безопасного relative export selector; existence/capture export отдельно проверяется его owner. |
| Effect `kind="config-value"` | name declared config_values entry. Выбирает данные, не разрешает произвольную runtime запись. |
| `[configuration.config_values]` | Named bool/i64/string; float/other shapes rejected. |
| `[[configuration.parameters]]` | Existing ParameterDecl fields/types/default/bounds/choices/required/must_exist/secret. ID сохраняет portable lowercase-token grammar. Definition admission не проверяет host filesystem. |
| `[[configuration.profiles]]` | id/name/optional description, default=false (не более одного), features seeds, selections, public typed parameters presets. Secret presets rejected. |
| Profile selection | `{intent="additive"|"exact", members=[...]}`. Отсутствующий members — omitted; [] — explicit empty. Unknown/dormant refs проверяются. Неполный профиль допустим. |

Все records закрыты для unknown fields. Arrays сохраняют declaration order. Все относительные формы выше также применяются под installer_options.configuration. Default marker профиля — сохранённые данные; compile_choices не применяет профиль автоматически.

### Пример до/после

Старое объявление package features остаётся валидным и автоматически не переписывается:

```toml
[features]
default = ["fast"]
fast = []
careful = []
[features.exclusive]
mode = ["fast", "careful"]
```

Представление того же initial default и at-most-one ограничения в новой **closure**-форме:

```toml
[[configuration.features]]
id = "default"
name = "Default feature set"
default = true
requires = ["fast"]
[[configuration.features]]
id = "fast"
name = "Fast"
[[configuration.features]]
id = "careful"
name = "Careful"
[[configuration.groups]]
id = "mode"
name = "Mode"
mode = "closure"
cardinality = "zero-or-one"
members = ["fast", "careful"]
```

Замена closure на choice — изменение смысла, а не механическое переименование: choice хранит отдельный ответ/политику defaults. Это пример формы определения для будущей миграции, **не** обещание, что install нового формата уже работает.

### Совместимость и область команд

В одном scope rich configuration конфликтует с nonempty legacy package features или installer features/profiles/parameters. Absent configuration сохраняет старые документы; unrelated sections сохраняются reader/serializer. Package и installer scopes не объединяются при одинаковых IDs. Installer/doc/remap scopes допускают только config-value DATA effects; dependency/forward/subskill effects там rejected.

Старые binaries с deny_unknown_fields откажут на новых ключах. Новый reader читает прежние manifests. Минимальная released version будет названа после выбора релиза, не угадана сейчас.

| Путь | Состояние этого этапа |
|---|---|
| Ordinary Manifest read/parse/validate/serialize | Данные сохраняются; compile_choices использует существующий pure evaluator. |
| Package install, normal/deferred | Явный ConfigurationUnsupported до apply; приобретение source/metadata могло уже произойти. |
| **`vibe install -g`**, package scope | App source resolution отказывает с `configuration_unsupported`, scope=package, exit2. |
| **`vibe install -g`**, selected installer scope | Planning отказывает до prompts/license/steps/app-work/index; scope=`installer <id>`, тот же code/exit2. Legacy selected option остаётся доступным при rich unselected alternative. |
| App update/modify | Используют те же source/selected-option owners и наследуют временный отказ; отдельные новые public update/modify сценарии этим этапом не доказаны. |
| `vibe app describe` / `steps`, human/JSON | Отказывают на rich option вместо misleading empty legacy arrays. Успешный legacy e1 document сохраняется. |
| Doc cache/read; remap offer/consent | Существующие операции сохраняются над data-only declarations. Новый общий evaluated result/новые эффекты/consent не создаются. |

Guards временные: они предотвращают тихую потерю настроек и не считаются завершённой поддержкой общего пользовательского пути. Rich requires-edge intent, persistence/default receipts, solver effects и common machine/console consumers ещё подключаются. Lock7/index3 и старые app e1 documents не расширены скрытыми полями.

### Rust API и error propagation

Новые optional поля `Manifest.configuration` и `InstallerOptionDecl.configuration` требуют обновления полных Rust struct literals; callers с подходящим Default/construction path сохраняют старые значения. `vibe_app::describe` возвращает `Result<ApplicationDescribe, AppError>` вместо direct document; source caller должен обработать Result. Workspace crates имеют publish=false; версия опубликованного crate этой записью не объявляется. CLI сериализует прежний successful e1 document, **не** Rust Result wrapper.

Global source wrapper теперь сохраняет typed ResolveError через AppError: новый package-scope отказ до исправления давал exit1 без JSON code, после — exit2/configuration_unsupported. Incumbent missing package проверен отдельно: остаётся exit1/registry_failed. Error class не выбирается по строке сообщения.

Evidence: ordinary-reader/all9kind/roundtrip/local-evaluation matrix; real global install4CLI cases (включая describe/steps human+JSON и legacy error control); прежний full wizard после изменения describe. Candidate597distinct tests/docs дедуплицированы по именам; main reader11/11 и public CLI4/4, fmt,4scoped conform и specmap --check прошли. Structural amendment подтверждён15existing app cases; они не прибавляются повторно к coverage count. Traceability0blocking findings,43known warnings. Profile application/parameter answer merge и whole runtime user journey ещё не принимаются.

## FMT-005 — Workspace requirements и lock root snapshot

Статус: **принято в main**, `075d38cb2`.

Authored `[requires.packages]` и `version.var` сохраняют прежний синтаксис. Изменяется смысл обработки: учитываются требования всех workspace consumers, explicit изменение касается только выбранного node, pins/retries не удаляют sibling constraints.

Generated `vibe.lock` root snapshot остаётся unique по координате: одинаковые ranges сохраняются, разные ranges записывают реально выбранную exact version; manifests остаются источником authored constraints. **Schema version остаётся7.** Старый неоднозначный snapshot с различающимися дубликатами требует восстановить authored declarations вместо угадывания.

Команды: workspace `vibe install` и его retries/freshness/apply. **`vibe install -g`** использует отдельную app home operation; эта запись не обещает изменения её index. Evidence: F02-workspace-constraints.md.

## Ещё не изменённые форматы

На текущей принятой границе `vibe.lock` остаётся schema7; app index — `vibe-user-applications/3`; существующие app describe/plan документы сохранены. Rich original consumer intent на requires edge, durable default binding, следующий lock/index protocol и новый common machine document — будущая работа. Для каждой такой смены нужен отдельный entry с точным old/new reader behavior, migration/legacy interpretation, командами и verification evidence. Нельзя прятать новую структуру в старой строке или объявлять старые effective sets исходным пользовательским intent.

## Шаблон следующей записи

Записывать: ID/дата; статус; base/commit; released range или Unreleased; exact keys/types/значение; old/new example; затронутые команды (явно проверить install -g, update, modify, import, project install); совместимость обоих направлений; migration/recovery; record/wire/schema versions; tests и ограничения. Изменение только semantics/validation тоже фиксировать, даже если TOML-ключи не поменялись.
