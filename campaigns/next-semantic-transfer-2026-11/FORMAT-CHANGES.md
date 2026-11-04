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

## FMT-006 — Original input на requires edge (Unreleased)

Статус: **принято как source/input этап**, commit `237f7bf2b` (`feat(configuration): preserve original dependency input`). Runtime configured-solving/persistence/UI ещё не завершены. Cargo version остаётся1.0.11; release number не назначен.

В inline `[requires.packages."<qualified coordinate>"]` добавлено optional `configuration`, одинаково для registry/git/path/version.var. Version/source/auth/link/access/friend/exclude сохраняются. Bare version string без metadata остаётся bare. Existing duplicate normalized coordinate check сохраняется; checked serialization теперь отказывает при dangling configuration или ambiguous manually constructed Requires вместо потери данных.

| Поле configuration | Точное значение |
|---|---|
| policy | Обязательное `require-selection` / `use-declared-defaults`; отсутствующий policy не становится assume-yes. |
| profile | Optional literal profile ID; default marker не выбирает его автоматически. |
| features | Public direct additions; default []; duplicate source additions rejected. |
| default_features | Boolean default true; false убирает automatic feature-default seeding, но не explicit profile seeds. |
| selections.<group> | `{intent="additive"|"exact", members=[...]}`; omitted members и [] различаются; absent entry сохраняется. |
| parameters.<id> | Typed bool/i64/string supplied public inputs. Target-aware validation проверяет name/type/bounds/secret classification. Host paths не разрешаются этим этапом. |

Исходный input сохраняется отдельно. Normalization выбирает явно названный profile, затем explicit selection/parameter entries заменяют его entries; explicit omitted и [] тоже заменяют profile answer. Возвращаются существующие FeatureInput/ConsumerChoices и supplied parameters, без заполнения missing defaults, effects, receipts или физической instance authority. Consumer identity задаёт реальный node/declaration, а не поле из TOML.

До:

```toml
[requires.packages]
"flow:org.example/workflow" = "^1.0"
```

После явного выбора нового input format:

```toml
[requires.packages."flow:org.example/workflow"]
version = "^1.0"
link = "dynamic"
[requires.packages."flow:org.example/workflow".configuration]
policy = "require-selection"
default_features = false
[requires.packages."flow:org.example/workflow".configuration.selections]
planning = { intent = "exact", members = ["mup"] }
extras = { intent = "additive", members = ["audit"] }
```

Inactive audit answer остаётся в source, возвращается при разрешающем guard. Это пример исходных данных, не готовая migration installed state; временные runtime guards ниже ещё действуют. Старые binaries отвергают new fields; новый reader сохраняет legacy absence/fields.

Common feature rows также получили `visibility="public"|"internal"` (default public), в package и installer definitions. Direct additions могут называть public; target-owned profiles/implications/declared choice interfaces могут активировать internal backing nodes. При явной миграции legacy `_helper` его private interface обозначается `visibility="internal"`; старый `[features]` сохраняет underscore policy. Internal не ограничивает raw source read и не делает bytes секретными. Foreign forwarding enforcement ещё pending.

Commands: prepared project install отказывает на неподключённый input любого workspace consumer/raw selected manifest до case-c migration/solve/Fresh/apply. Такой же guard действует на requires input fetched transitive package и deferred in-place. App source projection отказывает с dependency scope/configuration_unsupported; это касается acquisition для **vibe install -g**, app describe и source-based update/modify. Здесь проверена library guard; новая public CLI серия для каждого из этих verbs не заявляется. Предыдущий атом проверил используемую typed error propagation. Doc/remap границы не расширены.

Lock7/index3 и public wire formats не менялись. Rust Requires и ConfigurationFeatureDecl получили поля; exhaustive struct literals требуют обновления. Serializer больше не схлопывает неоднозначный вручную построенный Requires.

Evidence:529distinct successful candidate tests/docs, включая actual Workspace finalization двух nodes и2diagnostic RED→GREEN. Main source6/6, normalization6/6, prepared9/9+F02constraints22/22, fmt и4scoped conform0new; specmap0blocking,43known warnings. Source/serialize errors называют реальную edge и input field, не values. Configured solve, default binding, selected-node answer writing, scalar merge, lock/replay и common UI остаются следующей работой.

## FMT-007 — Secret references в step templates (Unreleased)

Статус: **принято в main**, commit `eefe840b8` (`fix(app): prepare prompts without secret interpolation`). Новых TOML-ключей, Vibe lock/index epochs или app machine-schema fields нет. Cargo.lock меняется только добавлением уже существующего workspace dependency app→safefs, без смены package versions.

Ужесточается existing secret=true: framework-interpreted `$param:ID`/`${param:ID}` запрещены для secret parameter в copy destination, env set/path_prepend, agent prompt и file_present condition. Inline declaration validation проверяет также inactive branches. Active prompt_file проверяется при planning до dependency/app execution, work/env/envelope publication. Skipped/agentless external prompt file не читается.

До:

```toml
[[installer_options.parameters]]
id = "token"
name = "Token"
type = "string"
secret = true
[[installer_options.steps]]
id = "agent"
title = "Agent"
kind = "agent"
prompt = "Use $param:token"
```

Такое объявление раньше принималось и могло записать значение в envelope. Теперь оно отказывает при validation; тот же текст в активном prompt_file отказывает при planning. Для предназначенного выполнения secret остаётся в существующем `VIBE_PARAM_TOKEN` окружении recipe/shell. Agent/native не получают нового secret transport. Не заменять отказ сохранением plaintext в prompt/config. Public templates и secret recipe/shell input не требуют переписывания.

`$$param:token` — literal, допустим. Returned public parameter text не разбирается повторно как template. Direct Param/ParamEquals conditions остаются допустимыми. Inline и file prompt используют одинаковое правило prose controls (newline/CR/tab допустимы). Missing public value в исполняемом prompt теперь является ошибкой, а не поводом скрыть её raw fallback.

Активный prompt_file читается existing SafeFS shared-inode observer: explicit root допускается отдельно, descendants не canonicalize/follow; hardlinks сохранены. План хранит exact checked literal, preview/run больше не перечитывают файл. Лишь unavailable finish app_root/app_bin могут остаться placeholders в preview. Эта continuity относится к одному плану; новая persisted resume identity этим patch не вводится.

Commands: **vibe install -g** проверен реальными CLI scenarios. Общий planner используется app plan/install/update/modify/import и project installation; отдельные public прогоны каждого maintenance/import verb не заявляются. Исчерпывающие Rust literals PlannedStep должны добавить prepared_prompt:None либо пользоваться planner; поле не сериализуется в authored StepDecl/e1 plan.

Evidence: actual RED core1/runtime1/public2; в старом -g earlier step и outbox успевали появиться, а изменённый recipe prompt_file подменял показанный prompt. После исправления actual dependency/earlier-step sentinels/work/outbox/index отсутствуют при refusal exit2, canary не попадает в output; replacement recipe действительно меняет файл, но envelope содержит ADMITTED_PROMPT. Candidate75distinct tests/docs, включая10existing agent cases и1wizard. Main joint CLI4common-format+2secret-template, fmt,3scoped conform0new и specmap --check0blocking/43known warnings прошли.

Это не исправление hosted raw-argv replay, optional-secret resume, batch/modify/import/dependency continuation, unclassified malformed CLI input или arbitrary child output. SECRET-BOUNDARY остаётся незавершённой общей обязанностью.

## FMT-008 — Уникальность app environment names (Unreleased)

Принято после root review и main gates: publicCLI2/2, app2/2, core16/16, fmt/conform0new/specmap --check0blocking;43прежних warnings сохранены. Новых TOML keys, wire epochs, lock/index schemas или package versions нет. Версия остаётся1.0.11; номер следующего релиза не назначен.

У installer options с recipe/shell (включая expanded hooks и conditional declarations) проверяется существующая проекция ID в VIBE_FEATURE_/VIBE_PARAM_: ASCII uppercase, dash/dot→underscore. Проверяются все feature и отдельно все parameter declarations, включая inactive/optional. Feature/parameter prefixes и разные options независимы. Package/generic literal IDs и options без этого consumer нового ограничения не получают.

Раньше два допустимых ID `a-b` и `a.b` давали один environment key: позднее объявление затирало первое, в том числе выключенная feature могла затереть выбранную значением0. Теперь такое объявление отказывает до выполнения. Для common IDs также отвергаются case/underscore aliases и имена, которые нельзя представить portable ASCII environment identifier, только в consuming option. Обычные noncolliding spellings сохраняются.

```toml
[[installer_options.features]]
id = "a-b"
name = "First"
[[installer_options.features]]
id = "a.b"
name = "Second"
```

При recipe/shell эта пара теперь возвращает ошибку с обоими IDs, `VIBE_FEATURE_A_B` и `installer_options[0].features[1].id`. Parameter/common scopes дают соответствующий точный field path. Answer values в эту диагностику не включаются.

Migration: явно переименовать один неоднозначный/unrepresentable installer-local ID, например `a.b` → `a-dot-b`, и обновить references, profiles, conditions, CLI flags и retained consumer intent. Автоматического encoding/renaming нет; существующие records не переписываются и миграция record не заявляется. Agent/native не получают нового transport.

**vibe install -g:** actual CLI проверяет обе очередности colliding feature/parameter definitions; refusal остаётся incumbent exit1/registry_failed, без app-work/index publication. Это не valid-but-unsupported refusal2. Positive real recipe видит оба feature значения1/0 и оба parameter values. Plan/install/update/modify/import/project installation используют общие validation/planning/runtime owners; отдельные новые public maintenance/import прогоны этим patch не заявляются.

Rust API: additive core `step_env_name`, `StepEnvironment`, `StepEnvironmentError`, `StepKind::receives_step_environment`, `InstallerOptionDecl::validate_step_environment`. Прежний `vibe_app::step_env_name` сохраняет signature/output и делегирует core helper. Public struct fields не меняются.

Candidate89distinct tests/docs, clippy/fmt/diff прошли. Это защита projection identity, не завершение common configuration, secret argv replay или cross-process resume.
