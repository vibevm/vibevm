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

## FMT-008 — Уникальность app environment names (Unreleased; 8f84aefb9)

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

## FMT-009 — Common app runtime and continuation (design only, Unreleased)

Not implemented or released at this entry. No additional vibe.toml keys are proposed: the already accepted configuration declarations become actual selected-installer inputs. Package and installer scopes stay separate; merely acquiring a package for install -g does not execute package configuration effects. Existing ordinary workspace guards remain until that consumer is connected.

Planned public selectors: app describe/plan --configuration; complete generated input via --configuration-input FILE (stdin - read once) for plan/install -g/update -g/app modify/force repair. Protocols: vibe-configuration-description/1, vibe-configuration-input/1, vibe-configuration-evaluation/1, vibe-app-configuration-plan/1. Existing successful e1 describe/plan remain unchanged; no rich meaning hidden in empty legacy arrays. Submitted input is strict, preserving omitted versus explicit empty and exact versus additive; typed i64 transport uses canonical decimal strings.

Planned persistence: application index epoch3/protocol4 for rich records, strict v3 read as effective-only and v3 write when all retained data is losslessly representable. No read-time migration. Pending steps epoch2/protocol2 when continuation metadata is needed, including new legacy runs with secret-presence/agent requirements; old1 retains explicitly weaker provenance. Rich plan.json stays in its existing run owner with a new generated envelope. No parallel choice database or secret answer hashes.

Planned canonical command: vibe app resume GROUP/NAME@VERSION --operation ID, using the actual -g/--path home and ephemeral secret resupply. It reuses saved public input and cannot renew default consent by replaying a file or flags. Old task commands refuse a replaced generation. Whole-import continuation requires its finite cursor; it is not claimed merely because one row resumes. Exact compatibility tests/results and accepted commits will be appended after implementation.

Generator correction uses existing unknown_fields=deny independently of truthful foreign_parsers=many for public input; local/shared/projection decisions must agree. No new policy field, owner epoch flag change, release number or package version is authorized by this design entry. Codegen/corpus/reader/runtime gates are still pending. Detailed before/after decisions: research/common-app-vertical-decisions.md.

FMT-009 design clarification: typed signed i64 uses canonical decimal string with a leading minus only for negative values. Plus, leading zeros, negative zero and out-of-range values refuse. Existing unsigned fields keep the no-sign rule; no existing counter wire format changes. The former general no-sign wording was specific to the first unsigned use and could not represent declared negative parameter values.

FMT-009 compatibility correction: explicit permissive fields in existing public documents remain allowed even when the outer format has unknown_fields=deny (scrape-report.health). This does not weaken its outer admission or new configuration input; the latter declares no field projections. Local/shared default policy and strict canonical-owner checks use the one effective registry policy, while the existing public-consumer projection boundary remains explicit. The initially proposed blanket deny-consumer prohibition was withdrawn before acceptance.

FMT-009 new-wire shape decision (implementation pending): presence-sensitive members/defaults/fallback/control-selection arrays are required nullable refs. null represents omission/unavailable selection as defined by the field; [] represents an explicitly empty set. Omitting a required field refuses. Entire group entries may still be absent from an input selections map. Typed source/TOML omission semantics are unchanged by these explicit adapters. New tagged-union parameter default/source/value and optional problem reason likewise use supported required-nullable representation. Existing e1 optional preserve readers are not globally tightened; known null-admission difference is B-230. These choices are not new vibe.toml keys or an accepted release.

## FMT-010 — Truthful source point/pattern/anchor selection (Unreleased; df040becb)

No vibe.toml syntax/schema or lock/index epoch changes. Runtime probes at0e98486da show actual wrong first-anchor content, hidden matched-document failures, wrong selected-world membership and discarded group/version selectors. Planned corrections: ambiguous requested Markdown path refuses, but unique/qualified/unrelated-duplicate controls stay valid; XML existing whole-document duplicate-ID refusal remains. Pattern group is fixed, explicit @version is honored; selected mode uses only its selected roots and rejects mismatching pins, ordinary mode uses installed versions with missing requested versions as nonmembers. Genuine missing/unmatched/empty cases remain valid; matched malformed/unsafe source failures are errors.

Migration/behavior: patterns which formerly borrowed membership from another group or newer/unselected version will no longer do so. Use the actual intended group and version; no automatic rewrite of authored source directives. Same-named packages in other groups are separate coordinates, not lexical winners. Underlying held/shared-inode/no-follow reader, literal percent spelling and active point-format context remain. Pattern without version keeps existing newest-installed/versionless behavior; no whole-closure snapshot or new revision-proof guarantee. Actual implementation/API changes and integration gates remain pending.

FMT-010 implementation detail: additive public DocTree::resolve_unique_path returns Result<Option<NodeId>, AnchorAmbiguity>; the error carries segment, zero-based depth and match count. Existing first-match resolve_path/find_by_anchor remain for their low-level callers. ResolveError variants and SectionSource/prompt string surface are unchanged. Actual source/embedding/prompt/compiler address consumers now refuse ambiguous selections. No explicit install -g argument or app state format changes arise from this source slice; indirect readers using these APIs inherit truthful selection errors. Main public17+prompt14 pass; remaining integrated gates pending.

FMT-010 accepted validation: candidate264distinct tests/docs plus clippy/fmt/diff; main public17/17+actual prompt14/14; integrated fmt/conform0new/specmap --check0blocking with43known warnings. All12reviewed source hashes match. No Windows/macOS runtime or complete source-query/proof acceptance is claimed.

FMT-009 continuation compatibility refinements (candidate, Unreleased): new index4 may carry value-free secretOrigins even for legacy installer syntax; these rows participate in lossless-v3 eligibility. Old3 read/list remains unchanged and has unknown historical secret provision, retaining its existing legacy fallback semantics. New actual explicit provision requires resupply. No secret values/digests or read-time migrations are introduced.

Existing manual --resume is retained as a current-work selector with weaker stale-generation guarantees; it does not become an implicit fresh configuration submission. New app resume/generated commands bind exact generation. Reopened kept work gets a new generation. Skipped-versus-first-unfinished inconsistency is being corrected. Public app work may gain optional derived operation target/phase and retainedFor fields for new2 state; legacy1 row shapes remain, existing fields keep truthful step evidence, and these additions require generated reader/compatibility checks. Explicit same-home cleanup-all still abandons selected pending work; surviving-parent protection is checked before deleting an individually selected child. There is NO home-cache layout migration: work was already home-local.

Rich portable export/import is not implemented by the first candidate. Legacy portable routes must refuse unrepresentable rich state before effects rather than flatten it. A separate required portable-format atom remains before common configuration completion; existing legacy portable public/effective data and weaker secret provenance are preserved. No portable epoch number is assigned by this note.

FMT-009 secret-default clarification: authored defaults/choices/labels remain public definition data under the existing contract. A definition snapshot/hash may retain that literal; evaluated secret values and supplied overrides remain excluded/redacted. This does not change legacy describe and does not conceal credentials authored into a public manifest default. New origin metadata describes the current binding; it never invents older secret-provision history.

FMT-009 pending completion refinement (candidate, Unreleased): `application_pending_operation.properties.publicationRecord` in formats/vocabularies.json is a required nullable string referenced by application/e2/steps. It holds the expected public per-installation record digest before index publication. The digest includes the pending marker as published and excludes only derived requiredBy edges; it never includes supplied secret answers. Completion requires the final record with pending=null and matching identity; first and later parent acknowledgements use that anchored result instead of adopting the then-current same-key record. This remains in steps2, with no second index receipt/store, no additional epoch or released-format migration. Actual owner RED→GREEN and two public child scenarios passed in the candidate; full acceptance remains pending.

FMT-009 import-owner correction (candidate, Unreleased): an actual first-row failure followed by a later dependency retry disproved the first-package cohort owner. Whole import now has one independent `vibe-app-import-work/1` record, `schemas/application/e1/import_work.jtd.json`, under existing app-work `imports/<operation>/import.json`. Its fields are protocol/target/phase/context/cohort. Context is public executor/lookup metadata, never credentials, raw argv/environment or supplied secret answers. Package steps2 removes the unshipped draft Import kind/cohort field; application index.pending still points only to an application operation. There is no migration from that rejected draft layout because it never shipped in main.

New exact target `ApplicationImportTarget {home, operation}` and one tagged `ApplicationContinuationTarget` distinguish application and import ownership. Its import arm also carries the finite owning row index. Private steps2 parent is required-nullable. New optional public `retainedFor` has `{owner: <canonical union>}` through supported output projection; existing ordinary rows omit it. Work output gains optional `imports` with real cohort targets, and `app work --import ID` selects an import for inspection/cleanup. Actual execution-row outcomes carry required-nullable rowIndex/target; non-executing preflight/arrangement notes are distinguished. No duplicate semantic target enum or optional-union generator extension is introduced.

`vibe app import FILE` versus `vibe app import --resume ID` are mutually exclusive; import remains user-home only. Cohort resume does not accept unscoped --set. Generated continuation follows the verified owner chain, while each child's envelope and answer retain its own generation. Scoped child resupply remains `app resume KEY --operation ID`; a same-named parent answer is never forwarded. Whole-cohort reporting keeps `vibe-app-import/1` and exit1 for refused rows; completing the cohort does not fabricate an installation for a failed row. Explicit cohort cleanup must preserve unrelated work/newer generations and leave matching selected pending references usable for fresh repair.

Secret resupply refinement: explicit Default/Absent→Explicit provision to the exact application operation is allowed, while original prefix provenance stays unchanged. Supplied-ID membership is not immutable public execution identity. A value-free strengthened requirement is stored before suffix effects, making a later park require resupply rather than fallback. No answer value/equality digest is stored. Portable apps/1 parameters remain public: values classified secret by the selected installer refuse before cohort persistence; ordinary ephemeral application prompts/explicit application input remain the route. These candidate semantics require full compatibility/old-reader gates before acceptance.

Rust API compatibility note (candidate): public Index::to_wire/from_wire use generated application/e3 index types rather than the former e2 types, even when the serialized document remains lossless protocol3. Explicitly typed Rust callers must adapt; wire compatibility does not imply an unchanged Rust signature. Index epoch3/protocol4 and steps epoch2/protocol2 remain distinct from retained index-v3 epoch2 and steps-v1 epoch1. Public Installation literals also need the new configuration/pending/secret-origin fields. No new vibe.toml syntax follows from these storage/API changes.

Update cleanup refinement (approved design, candidate not accepted): --purge-previous and automatic older-kept removal are deferred until replacement setup completes under the existing required/optional-step success rule. A parked or failed replacement keeps previous installations available; terminal setup failure returns its original error, not purge success. The exact requested victims/identities and queued/removing/removed progress will be retained once in existing steps2 before their removal effects. A published new index does not certify cleanup completion. Possibly executed external removal stays explicitly uncertain, with no automatic replay or silent success. This deliberately changes legacy failure/parking behavior; exact new wire members and final tests will be recorded at implementation acceptance.

The candidate now implements required `retirements: [{key, expectedRecord, state}]` in application pending metadata; state is queued/removing/removed and operation phase adds retiring. Missing list is not silently treated as no obligation in the new closed format. Fresh/from-step cannot erase Removing uncertainty. Completion-only continuation includes the final Published/all-Removed cut and must not require installer source, named executor or secret answers that no further step consumes. Exact home/generation/current result and victim-state checks still apply. These are unreleased steps2 changes, not migrations of previously shipped steps2 records; final reader/corpus and runtime acceptance remain pending.

Explicit work-abandonment compatibility (candidate): same-home `app work --clean --all`, under the home mutation lock, may remove unreadable selected package work after preflighting every readable ownership relation and validated local index reference across the complete selected cohort. It clears only pending references mapping to the actual selected local work slots, preserving installations/payloads and unrelated homes. Corruption is not treated as proof of no owner; incomplete selections with unverifiable surviving ownership refuse before deletion. Exact single unknown-package cleanup may refuse with whole-cohort guidance. This preserves deliberate all-work abandonment without pretending corrupt step bytes can prove their own generation; tests and final acceptance remain required.

## FMT-011 — Apply metadata ownership and rollback (Unreleased; 50a4630aa)

No vibe.toml keys or lock/provenance schema change. The correction replaces ambient truncate/remove rollback with prepared bytes and observed owned-publication checks for selected manifest, workspace lock and provenance. Initial symlink/reparse/special/shared-link mutation targets refuse before effects under existing mutation policy; ordinary read-only shared source remains supported. Standard regular-file rollback and permissions remain valid. Later byte/object/link substitutions are preserved with a conflict, rather than silently overwritten. A guard that never wrote does not undo.

This is a bounded cooperative observation/publication guarantee, not portable hostile-race CAS, full crash recovery or multi-file atomicity. The final compare→mutation window remains explicit. No storage journal or new user format is introduced.

Accepted additive Rust APIs: Manifest and Lockfile gain render_with_comments(Option<&str>) -> Result<String>; LockProvenance gains the corresponding Result<Option<String>> seam, where None means removal. SafeFS adds OwnedFileReceipt, OwnedPublication, OwnedPublishError, write_atomic_owned_in_with_mode and publish_new_in_with_mode. Existing public writer signatures and required struct fields remain compatible; ordinary core writers retain their existing ambient behavior. The only dependency change is the existing workspace vibe-safefs edge for vibe-install; no versions/sources/checksums change.

Apply now replaces metadata atomically, retaining existing Unix mode and ordinary new-file creation mode. It validates recognized selected-manifest semantics against planning and rejects invalid UTF-8 instead of silently losing decoration. Actual mutation targets that are symlinks/reparse points, special files or shared hardlinks refuse before effects; an unchanged shared/read-only manifest stays supported. Migration guidance: use an ordinary owned metadata file for a write, replan after semantic input changes, and inspect the named conflict before retrying. No automatic file conversion or read-time migration occurs.

Rollback conflicts retain both the original failure and conflict details; an observed hosted park cannot hide them as clean delegation. Diagnostics name retained run/tasks and state categories without file contents/digests. No install -g flag, app index or app continuation format changes are introduced by this slice; package acquisition paths using guarded install apply inherit its admission and failure behavior. Direct global-app continuation acceptance remains separate under FMT-009. Linux candidate tests passed; native Windows/macOS runtime and the final uncooperative-writer window remain unverified/unsolved respectively.

Acceptance evidence:662distinct candidate tests/docs; main public apply19+direct guard3+actual CLI rollback1+owned docs4 all passed. Fmt, scoped conform0new and final map check0blocking passed. No full-platform or general recovery acceptance is inferred.

## FMT-012 — Nested committed-boot attribution (Unreleased; 7ed7e7256)

No vibe.toml keys, install -g flags, lock/index epochs or tree JSON v1 shape change. The `vibe tree` correction retains all nested/repeated embed occurrences with exact existing inclusive line coordinates, in opening order. Flat valid generated arrays retain their order. Fenced marker examples remain literal, using the existing Vibe fence grammar for both static contribution and embed discovery. Valid inline/prose static samples no longer split contributions; actual provenance requires whitespace-only opening-line prefix and closing-line suffix, preserving multiline c1 frames.

Malformed reserved embed pairs (unexpected/mismatched/crossed/empty/unclosed or crossing a contribution) fail with line context through the existing tree error envelope. Previously some returned success while dropping spans or substituting the closer's address; empty-close shorthand was accepted. Adjacent/overlapping static provenance now refuses rather than panicking; canonical static controls with trailing prose also refuse. Corrective action: fix or regenerate malformed artifacts; quote documentation examples appropriately. Compiler output already emits nonempty matching embed addresses. No automatic artifact rewrite occurs.

Additive Rust API: vibe_spec::fenced_code_lines(&str) -> Vec<bool>, sharing the existing FenceTracker and including delimiter lines. Internal CLI decoder adds a line/reason MalformedEmbed error variant; the existing error envelope remains. No new dependency. Candidate279distinct tests/docs and main public14+fixture1+fence2+doc1 pass; fmt/conform0new/map0blocking pass. This does not deliver the full boot debugger, ordinary source inspector or exact byte measurements. Design/evidence: research/boot-tree-attribution-design.md and features/F08-boot-attribution.md.

## FMT-013 — Quoted fact anchors remain literal (Unreleased; d6147a94d)

No vibe.toml key, install -g flag/app-index, authored anchor grammar, tracked campaign-record or public Rust signature change. The correction follows the current first-token/code-opacity contract: a definition-looking @fact token inside an inline code span no longer creates a second definition or DuplicateId. Real parsed heading/fact/typed-fact duplicates still fail; marked/missing and swallowed-anchor checks remain. The canonical backtick/fence parser is unchanged, including unmatched backticks being literal rather than hiding following markup. Other existing consumers using this parser inherit the corrected diagnostic; a separate global-app installation journey is not claimed by this slice.

This supersedes the intentional diagnostic convention recorded in B-092; that historical record is preserved. Quoted documents previously refused by progress/conversion/fact consumers can pass without source rewriting. No authored migration or unrelated documentation-corpus/version change. Design and actual evidence: research/inline-anchor-opacity-design.md and features/F07-inline-anchor-opacity.md.

Cache compatibility amendment: the erasable ParsedDoc sidecar PAYLOAD_SCHEMA advances3→4 so unchanged documents cannot inherit the old false DuplicateId result on normal cached commands. Tracked cache.json CACHE_SCHEMA2 and its verdict/evidence/campaign maps remain unchanged; source identities are not rekeyed. Old derived payloads are treated as misses and reparsed. Read-only checks do not rewrite/delete them; later ordinary cache-writing commands regenerate4. No manual user cleanup or new cache owner is required. This is an internal derived-format change, not an app index/steps or manifest-format migration. Actual old-producer warm-cache RED→GREEN,233candidate tests and main7library+5CLI+1sidecar cases passed; fmt/conform0new/map0blocking passed.

## FMT-014 — Core meaning fingerprint and direct-input delta (accepted foundation, Unreleased)

Additive Rust APIs: ConfigurationDecl::semantic_fingerprint() returns the versioned `configuration-semantic/1:sha256:…` meaning key; ChoiceModel::prepare_feature_delta(&FeatureInput) returns direct selected IDs before closure using the incumbent feature preparation rule. The exact byte recipe is formats/hash_recipes/configuration-semantic-1.toml: typed values, length framing, signed integer bytes and stable semantic ordering. Names/descriptions/presentation order/UI profile-default marker are excluded. Authored definition defaults remain public data; no operation-answer values or answer hashes are accepted as inputs to this definition recipe.

No vibe.toml keys, index/steps protocols, global install flags or current app runtime behavior change in this atom. The recipe and methods are foundations for later default-review/persistence consumers, not delivery of those consumers by themselves. No package/dependency version changes. Main65core cases+1doctest and scoped static/map gates passed; full configuration remains unfinished.

## FMT-015 — Effective generated-reader policy (accepted foundation, Unreleased)

No current schema/registry/vocabulary/epoch, vibe.toml key, install -g flag or app state change. Generic generation now consistently uses the registry's effective unknown_fields value; explicit allow/deny overrides the historical parser-population default. A present non-string value is a named error rather than a silent default. Same-schema owners must agree. Default none→deny and ours/many→allow remain when no override is specified.

Explicit public named-field projections remain permitted inside strict public outer records; private none consumers cannot request them. Optional nonnullable projected objects preserve absent/present distinction and reject explicit null, and matching respects generated serde wire rename. This does not globally change old optional-preserve null handling or add optional-union/inline-nullable collection support. Current194generated outputs remain byte-identical; no data migration is needed for current readers. The foundation enables truthful later formats, which require their own separate acceptance.

Generator190+existing wire29passed; current app/CLI compilation, clippy/fmt/conform and ordinary check-codegen all passed. Future53→104vocabulary-site count is deferred with its actual new vocabulary. Details: features/F10-generator-policy-foundation.md.

## FMT-016 — Canonical configuration contracts and forward proxy reader (accepted foundation, Unreleased)

This atom prepares four generated protocol1 public documents: vibe-configuration-input, vibe-configuration-description, vibe-configuration-evaluation and vibe-app-configuration-plan. Submitted intent is strict through nested owners; output views use explicit projections of the same canonical vocabulary. Required-nullable selection/default fields preserve null versus explicit empty arrays and reject missing keys. Signed configuration integers are represented as strings with a canonical i64 decimal domain contract. Generated JTD readers enforce the string shape; the canonical decimal/range check belongs to the later app vocabulary adapter, not this reader-only layer. These declarations require actual strict private owners, supplied by application index e3/protocol4, steps e2/protocol2 and independent import_work e1/protocol1.

The current registry IDs move to the new available index/steps schemas; archived index3/steps1 retain their old schema/corpus/reader contracts. Preserve the historical steps1 recoverable=true classification rather than silently changing the archived contract. For the new steps2 and import_work contracts, recoverable=false: exact generation, outcome/retirement uncertainty and cohort history cannot be deleted and reprojected automatically from other authoritative data. Submitted configuration-input is likewise authoritative intent (false); description/evaluation/app-plan are derived views (true). Resume support is not the same as automatic reconstruction. These flags describe metadata policy, not a new recovery implementation. New Rust generated types/FormatId variants are additive explicit modules; existing app runtime still addresses its old modules. The standalone proxy reader is selected to accept strict3/4 documents before projecting the unchanged routing fields, without read-time migration or a full app-domain dependency.

No vibe.toml key, install -g flag, new app runtime writer, public app_work shape, new resume/import operation, or portable format is activated by this layer. FMT-009 remains the later runtime candidate contract. Standalone generator190, wire37 distinct, proxy8 and incumbent CLI7 passed, with compile/clippy/fmt/conform and exact204-output reproducibility. Root staged the reviewed68paths; ordinary check-codegen and staged wire-diff returned0. Map/check passed with0blocking findings and43known warnings. See features/F11-canonical-configuration-wire.md. The full configuration/runtime milestone and old113 completion lower bound are unchanged.

## FMT-017 — Existing proxy image upgrade before index4 publication (accepted runtime amendment, Unreleased)

Add the original-image capability query `vibe-app-proxy --supports-index vibe-user-applications/4` (and3): success prints exactly the requested supported protocol plus newline, unsupported/old images return nonzero. Existing --probe remains vibe-app-proxy/2; no new JSON/storage schema, vibe.toml key or install-g argument is introduced. The image actually copied is the one admitted. Runtime publication and session-exposure APIs must enforce compatible managed copies before index4 appears; any necessary Rust API change will be logged after implementation. The precise decisions and required real old-binary tests are in research/common-app-proxy-upgrade-decisions.md. This closes an executable-upgrade gap that same-new-binary protocol tests cannot prove.

FMT-017 API direction (implementation in progress): the public Index::save and sync_proxies entrypoints acquire the existing mutation lock themselves; already-locked writers use checked counterparts with a home-bound MutationLock. A documented public locked-save path preserves atomic external read-modify-write. Public session exposure takes the lock, while dependency exposure reuses its outer lock. No lock token enters serialized request, pending state or manifest data. This distinction is required to prevent both nested-lock failures and direct library bypass of the compatible-image prerequisite. Final signatures and executed checks remain to be recorded.

FMT-009 selected-installer runtime is now accepted at the F12 boundary: existing installer configuration declarations are consumed by real install/maintenance/continuation; public application_work e1 gains optional operation/retainedFor/import fields with old-reader compatibility. No new vibe.toml key was added during this activation. Full rich portable apps export/import and configured package solving remain required; their honest legacy refusal is not completion.

FMT-017 actual public APIs are Index::save (self-locking) and Index::save_locked (existing lock), sync_proxies (self-locking) and sync_proxies_locked, and locked current session/dependency exposure. Actual stored3→next4 triggers the existing-copy barrier; same4 metadata does not rescan session caches. Both3/4 capabilities are checked on exactly captured bytes. Compatibility/ownership refusal is proxy_upgrade_failed; ordinary filesystem errors remain state_io. Real old-binary global/session, postrevision failure, pending-only and old/missing-image scenarios pass; ordinary failures remain reported as failures. Final map/check has9068units/4902items/4058edges,0blocking and43known warnings. No native Windows/Mac execution claim.

Generator accounting clarification for FMT-015/016: earlier194/204numbers describe preservation supersets. The L1 owner set was170; current actual codegen owner set is180, with24 additional release-managed mirrors checked separately for preservation. Root previously called the superset generator outputs too loosely; actual ordinary gates were run and the owner outputs were included, so the correction narrows the wording rather than removing evidence.

## FMT-018 — Init honors the existing host spec_format (Unreleased)

No new vibe.toml field, grammar, epoch or install-g option. Ordinary init package now respects the existing root consumer spec_format when regenerating boot, instead of always choosing Mixed. Explicit xml keeps STATIC.xml and matching INDEX/launcher references; explicit markdown and absent-setting Mixed compatibility remain. New package --format/--link and scaffold Normal/Dynamic defaults remain separate from the host dialect. Actual nonempty-dependency preservation and both converter paths are exercised through real CLI fixtures; see F13.

## FMT-019 — Full portable configuration (candidate implementation, not yet accepted)

Planned vibe-apps/2 retains typed original configuration/history and required-Auto bindings in JSON/TOML; apps/1 remains compatible. None uses omission in the new representation, preserving empty values and selection-object presence; canonical common JSON stays unchanged. Planned export --configuration forces the full form; existing --json controls JSON versus default TOML. Planned import --target-input and --secret-input have separately versioned typed per-application inputs; secrets remain ephemeral. Private import-work versioning must be reviewed with its actual continuation delta. No new vibe.toml key or new HOME/rootfs/Docker support is implied. Full decisions and acceptance cases: research/common-portable-implementation-decisions.md.

FMT-019 generator follow-through (candidate): the new optional-null alias grammar requires explicit schema metadata x-null-alias="absent" on eligible optional nullable fields. This is generator/schema metadata, not a new vibe.toml key. Existing unopted/nullability contracts stay unchanged. Actual generation initially failed before this support. The implemented support now has 31 focused passing checks including compiled JSON/TOML witnesses, and generation succeeds; full portable runtime integration remains pending.

FMT-019 selection clarification (candidate): apps/2 roots explicitly identify selected Manual jobs, separately from carried rows/ownership. Automatic old-format compatibility is retained for legacy-only exports; common configuration/new secret provenance in the selected closure triggers full, and --configuration explicitly requests full closure for legacy data too. No installer-default guess from an option name is allowed.


FMT-019 implementation checkpoint (2026-11-05, not release acceptance): apps/2 export and the typed target-input/secret-input readers now exist. New CLI `app export --configuration` forces full export; common configuration in the required closure selects it automatically. Full JSON stdout is the strict portable document without command-context decoration; legacy JSON decoration remains unchanged. Four new CLI export scenarios plus eight existing portable scenarios passed. This proves export only, not full import or roundtrip.

The explicit optional-null alias generator support is implemented; 31 focused checks including compiled JSON/TOML witnesses passed and generation succeeded. Private `vibe-app-import-work/2` is now a candidate strict owner alongside the unchanged old reader; it retains roots, exact bindings, public target decisions, cursor/results/current work, waiting IDs and value-free per-recipient secret resupply obligations. Integration into execution is still in progress. Mutable resupply obligations must not change immutable source history or enter the execution-context digest; secret answers must never be persisted. Index/steps protocols are not renumbered for this addition. No change to vibe.toml or vibe install -g syntax is introduced by this checkpoint.

FMT-019 installer API integration (candidate, 2026-11-05): Rust InstallRequest gains optional typed transfer input, carrying public historical configuration, explicit target decisions and value-free required secret IDs through the ordinary planner. The transfer branch rejects simultaneous ordinary complete configuration/feature deltas, mismatched target installer and a selected legacy installer without common configuration. This is not a new vibe.toml key or install-g CLI switch. Exhaustive Rust struct literals must initialize the new field (ordinary requests use None); the first focused test compilation caught one missed pending-input constructor. Full-cohort resumption must reconstruct its verified retained context separately, not treat None as proof that imported history is unnecessary. This API integration is not yet accepted or released.

FMT-019 runtime checkpoint: three actual library tests now pass for one configured root A-install/export→B-import, durable pre-effect missing-secret waiting and same-key candidate drift refusal. Private2 files are actually written/read in that boundary. This is not full CLI/resume/dependency acceptance or release readiness; no vibe.toml/install-g syntax change.

FMT-019 full-cohort continuity amendment (approved implementation direction, not accepted): completed descendants of a FULL2 import are retained until the cohort settles, then released through the existing work-retirement owner. Ordinary and legacy1 retention behavior stays unchanged. For full-scope operations only, a new execution-context/2 identity representation will expose the imported-scope component and bind it into the execution digest, permitting exact terminal proof without reopening source/executor inputs. The exact spelling/parser still requires source review and tests. None-scope operations must preserve the old execution-context/1 bytes and hash; the genuine old unfinished CLI baseline is the required compatibility gate. No new index/steps epoch, vibe.toml field or install-g syntax is implied. This is drift detection, not authentication of editable state.

FMT-019 observed context2 source spelling: `app-execution-context/2:{scope}:sha256:{contextDigest}`, where scope is `app-import-bindings/1:sha256:{scopeDigest}`. The actual serialized execution context includes import_scope in its hashed bytes only for Some; serde omission keeps None on the existing v1 field sequence/domain prefix and output spelling. Terminal matching checks the exact expected scope and 64 lowercase hex digest suffix. Source inspected, not yet old-binary runtime-qualified; no authentication claim.

FMT-019 acceptance boundary: F14 selected-installer portability is now root-accepted after the final frozen runtime/static panels, fresh map/check and staged check-codegen exit0. Earlier pending/candidate paragraphs are historical checkpoints superseded at this boundary. Definitive Unreleased behavior and migration guidance: formats/breaks/006.md. Generic configured-package solving/persistence and full F04 remain open.


## FMT-020 — Consistent package admission and source-bound in-place preparation (candidate)

No new vibe.toml keys or direct install-g grammar. Scoped maintenance must refuse previously ignored authored common input or a selected common definition until real generic consumers exist, preserving project semantic state. Actual Git source/ref/commit and manifest are prepared together before checkout; apply consumes that identity. Cache/object acquisition is classified separately from project mutation. Existing canonical source URI and actual-origin behavior remain distinct; no durable preparation store is introduced. New Rust prepared-source capability and caller adaptation are in progress.

Legacy case-c requires migration still seeds the in-memory request before solving. Physical publication now waits for successful complete admission (including lock-respecting Fresh); a failed plan leaves original files and caller seed intact. Earlier eager persistence before a solve failure is intentionally corrected and is not claimed as unchanged behavior. Successful migration and legacy update controls remain required. This safety increment is not completed generic configuration support.

FMT-020 source identity refinement: the same origin URL can resolve different bytes under slot-local Git transport configuration. Existing-slot preparation therefore uses native original-repository transport, allowing only bounded object/FETCH_HEAD cache metadata before admission; HEAD/index/config/named refs/worktree and hooks remain unchanged. Pure prepared-source refusals have their own typed error rather than a fabricated failed Git command. Admission/manifest errors must not be hidden by a mirror fallback.

FMT-020 root acceptance boundary: F15 consistently admits actual package inputs/candidates before project semantic effects, with exact prepared in-place source and explicit Git-cache exception. No positive generic configured-package runtime is inferred. The definitive Unreleased compatibility/API account is formats/breaks/007.md.


## FMT-021 — Explicit optional package declarations (F16 accepted boundary)

Planned existing inline requires.packages key: optional:boolean, omitted=false. Explicit false remains authored metadata; registry/git/path/version.var share one declaration owner. New readers must retain old mandatory meaning; old strict manifest readers may reject the new key. No install-g grammar change has been implemented or accepted. Lock representation/version and maintenance-reader changes are still being designed and will be logged before writer activation. Canonical planned facts: PROP-069 OPTIONAL-DECLARATIONS, OWNER-LOCAL-ACTIVATION, DORMANT-SOURCE-BOUNDARY. Parser/schema alone will not be counted as delivery.

FMT-021 reader/writer design decision: rich package configuration requires explicit lock schema8; the new reader must retain strict legacy7 support without fabricated original requests or read-time rewrite. Legacy-only worlds retain lossless7 writes. Old7 binaries reject8; do not describe that as universal backward compatibility. Existing8 must not be discarded/regenerated with guessed defaults after reader failure. These are approved implementation requirements, not yet tested or delivered schema behavior; actual key shapes and maintenance evidence follow with the candidate.

FMT-021 legacy regeneration decision: retain only the existing known schema1–6/no-rich install regeneration and historical empty remap requirement-layer behavior, via exact UnsupportedLockfile classification. The core reader must reject rich data under an older epoch before that classification. Schema7/8 are faithful reads; malformed/rich-invalid/future schemas propagate. This does not claim faithful reading of old1–6 remap precedence or a new migration service; actual CLI old6 regeneration and malformed/future/no-consent-mutation controls remain pending.

FMT-021 classification correction: format.lockfile recoverable=false for the rich retained-state capability; deleting the lock cannot be assumed to reproduce historical decisions from present inputs. The registry's existing epoch1 is not the TOML meta.schema_version discriminator (7/8), and schema=none continues to identify the existing Rust-owned TOML format. PROP-002#LF-CONFIGURED-STATE records the planned compatibility/association contract without rewriting historical v2 examples. No generated wire schema is introduced by this classification change.

FMT-021 current implementation checkpoint: four actual CLI cases now pass for the bounded explicit local-copy route, including original-only intent change and missing-record refusal after a reproduced/fixed bug. Earlier paragraphs retain design chronology. Candidate user/API compatibility notes: formats/breaks/008.md; broad runtime/static acceptance and wider capability remain pending.

FMT-021 adjacent compatibility correction under review: remap matching must respect loaded-source order before comparing unrelated source-local row indexes. This makes existing package priorities effective across package sources without renumbering public provenance or changing JSON fields. Known old1–6 EmptyWorld regeneration must publish a supported empty7 lock through the existing owned atomic file mechanism; genuinely absent lock remains absent. Actual remap/core regression and old6 CLI gates pending.

FMT-021 root acceptance: F16's explicit fixed-workspace/local-copy runtime, lock7/8 replay/admission and adjacent compatibility corrections are accepted at the boundaries in features/F16-explicit-package-configuration.md and formats/breaks/008.md. Broader common configuration remains open. Root corrected its own omitted codegen consequence: changing recoverable=false also requires the generated FormatId::Lockfile metadata; normal codegen changed that one bool, with183other inspected generated files and146captured inputs unchanged. No new JTD schema or codegen algorithm was introduced.


## FMT-022 — Compiler IR2 and generated embed provenance (F17/F19 accepted boundary)

Root authorized a typed occurrence carrier through existing expansion/compiler/XML/tree owners after actual installed XML lost nested attribution. Active compiler-ir registry advances1→2, with ir_schema2 and e2 schema/corpus; e1 historical representations remain strict and cannot silently enter provenance-preserving execution without rebuilding from actual inputs. The implementation/generation and reader compatibility gates are pending. Generated XML c1 embed comments must preserve paragraph text; legacy raw-control laws remain. Tree v1 shape is retained. No authored vibe.toml or install-g declaration grammar change. Candidate user/API notes: formats/breaks/009.md.

FMT-022 native-consumer extension: actual compiler request/reply also require e2 schemas/envelope2 around ir_v2; historical e1 rows preserve existing many/none reader policies. This does not change FFI ABI, phase/slot lifecycle or separate backend-byte reply epoch merely by numeric substitution. Old compiler schema declarations refuse before invocation. Candidate registry rows updated; active/historical consumer gates remain pending.

FMT-022 cache upgrade: generated unit INDEX metadata gains `# vibe:compiler-recipe compiler-artifact/2` beside the existing fingerprint/native-pending frame. Graph hash bytes and TOML entry schema remain unchanged; ordinary/native reuse and verification require the current recipe, rebuilding unstamped old units through existing publication. No authored vibe.toml/install-g change or sidecar. Upgrade/reuse runtime control pending.

FMT-022 root acceptance: current implementation passes real XML install/cache-upgrade/reinstall/tree, typed provenance/working-surface, strict historical token/epoch, SDK/native declaration, source-preserving minifier and scoped static/traceability gates. Normal staged codegen is deterministic; wire-diff reports the explicit pre-public schema/corpus shift. Earlier pending paragraphs are design chronology. Linux/current controlled native fixtures are the executed platform boundary; opaque post-emitted rewrites and historical inspection keep their stated limits. Permanent receipt: user-local evidence/2026-11-06/xml-ir2-acceptance; implementation commit is recorded after final publication.


## FMT-023 — Pure authored migration proposals (M13C1 candidate)

No authored vibe.toml keys, lock/wire epochs or vibe install -g syntax change. The core library adds explicitly selected finite materialization profiles: materialization-legacy-absent-r1, materialization-current-absent-r1 and materialization-current-epoch1-r1. snapshot-to-copy-r1 proposes one exact scalar edit; identity-r1 preserves the two distinct current profiles. Unknown revisions and unsupported shapes refuse; missing compatible selection returns typed input requirements before syntax validation. The output carries exact before/after text, edits and domain-framed content/diff identities, not write authority. No source/feature/Redbook converter or public CLI/apply activation is implied. Successful fixtures are synthetic; retained historical scalar evidence demonstrates refusal, not a positive historical migration corpus. Acceptance is pending the scoped implementation gates.

FMT-023 acceptance: original neutral M13C1 accepted in `f56a4cba60e502416d194f6fc3677e51a1ad2e45`;19behaviour/10doctests plus independent root vector and scoped static/specification gates pass. Earlier candidate paragraph records chronology. This is a core proposal API, without authored grammar changes, file publication, CLI/MCP or install-g activation.


## FMT-024 — Authored change manifest and structural work declarations (candidate)

M02B1/B2 introduce a separate authored change.toml grammar, not new vibe.toml or vibe install -g keys. Approved registry row: format.change-manifest epoch1, schema=none (Rust-owned TOML), recoverable=false, foreign_parsers=ours, unknown_fields=deny, corpus=formats/corpora/change-manifest/e1, sunset=none. Ordinary parser accepts its defined schema1; explicit pure pilot0→1 and identity1→1 proposals use the accepted token replay owner and do not publish files. Unknown epochs and fields refuse. The initial reader covers original B1 ordinary role/capture declarations; later final-prototype epoch1 bound-origin/release-impact vocabulary is not claimed supported before its own integration. No existing lock epoch/recoverability policy changes.

Immutable directory identity is distinct from editable slug/title. Normative intent, structural work graph and selected-work contribution identities have different inputs and domains; no completion/scheduling authority follows from them. Origin addresses and leaf digests are declared capture metadata, not source authentication. Saved reads retain bytes observed through incumbent no-follow source reads; this must not claim whole-change atomic snapshots or deny legitimate shared source hardlinks by importing a mutation-only reader. Exact user-facing grammar, corpus and hash recipe remain under implementation/review.

FMT-024 work-grammar inventory completion: actual B2 has a distinct closed authored work TOML reader with schema1, but prototype registry omitted it. Approved format.change-work row uses epoch1/schema=none/recoverable=false/foreign_parsers=ours/unknown_fields=deny and its own corpus. This registers existing grammar rather than adding a new work capability; no pilot migration or publication path is inferred. Its actual parser must consume representative, unknown-field, cycle and duplicate corpus cases before acceptance. Both registry additions require normal generated FormatId/inventory verification.

FMT-024 acceptance: M02B1/B2 accepted in `940f93db4115c83484916849580f085d47c4b8ae`, with explicit compatibility note formats/breaks/010.md. Exact33committed hashes, actual17distinctcore/8saved/20doctest/2registry cases and final scoped/static/stagedcodegen/wirediff gates reviewed. No vibe.toml/install-g grammar or existing lock changes; the2Cargo lock edges are existing internal dependencies. Earlier candidate paragraphs preserve chronology.


## FMT-025 — Strict neutral migration planning transport (candidate)

Approved three new transient machine families migration-plan-request, migration-plan-report and migration-plan-error, each epoch1, foreign_parsers=many, unknown_fields=deny, recoverable=true, with schemas/migration/e1 plan_request/plan_report/plan_error roots and registered corpora. Normal generation owns domain types, shared neutral error vocabulary and FormatId additions. No authored vibe.toml/install-g syntax, lock epoch, source descriptor activation, journal or application writer changes.

Request requires schema=1 and supplied source_text; optional source_profile/target_profile/recipe are non-null strings when present. Missing compatible choices are typed NeedsInput; explicit null/wrong type/unknown fields/schema cannot silently default. Report distinguishes preflight from executed planner outcomes and exposes exact no-op/diff/preconditions and absent/epoch1 identity. Future source_decisions/feature profiles are not registered as inert fields. Source interpretation extension must update generated/runtime/advertised MCP schemas together.

Proposed text-only CLI: refactor plan-migration with request JSON literal or stdin before settings/tracing/context/provider startup; MCP plan_manifest_migration is a thin adapter to the same domain owner. Request text never grants path discovery or write authority. A new typed structured preflight error arm must retain its report without claiming planner execution; unrelated existing errors preserve meaning. Full source/runtime/static/generated parity is pending acceptance.

FMT-025 accepted in `21d7683e027d81f14b2528734dd77bb7da236821`; actual CLI/MCP/corpus/relational and static/index/generation gates pass. Unreleased details and executed example: formats/breaks/011.md. No authored manifest/install-g or application writer changes. Source/edit fields retain exact text/decimal offsets; preflight remains distinct. Root receipt verifies115committed blobs. Earlier candidate paragraphs preserve chronology.

SafeFS prerequisite acceptance (`f2ca79eee29b80fa612bb4fd17215d9d0daef398`): additive existing-store/lock/token/retained-sync Rust APIs and corrected Unix directory/FIFO read admission. No authored vibe.toml/install-g, lockfile, journal or wire epoch changes. Observed source bounds and source-versus-mutation link rules remain; this is not a new migration state format or D1 writer activation.


## FMT-026 — Guarded migration operations and durable application journal (candidate)

Approved three strict epoch1 many-parser families under schemas/migration/e1: operation_request, operation_report and application_journal. Registry IDs migration-operation-request/report are transient recoverable=true; migration-application-journal is recoverable=false because exact retained intent/before state cannot be discarded as cache. Unknown fields deny. Neutral C1/C2 recipes only; no source/feature/Redbook activation or product-version floor copied from prototype. Normal codegen owns the roots/shared closed error-effect vocabulary; required-nullable mode/evidence fields require actual missing-versus-null tests.

New operation surface is apply/inspect/recover with independently admitted project/file/store authority and exact approval/preconditions. CLI grants come from separate --project-root, --target-relative and --recovery-store flags, not merely a forwarded JSON payload. MCP uses finite configured grants/protected-store policy and a closed typed operation preflight report alongside C2's typed planning error. No generic unvalidated JSON report is substituted for either. Exact final schemas/command/runtime proof remain pending. No authored vibe.toml/install-g grammar changes are implied.

Transactional-owned publication is an approved shared-owner extension, not permission to adopt any matching filename. Fresh create-new receipt semantics remain; restart reuse requires caller-bound durable transaction intent, exact held-stage byte/mode/identity checks and actual sync before publication. Receipt comes from that pre-publication handle, never a later target reread. Existing unrelated non-owned publication behavior is outside this change.

FMT-026 output-channel refinement approved: an executed operation whose report fails validation may return explicitly text-only failure with clearly unvalidated effect diagnostics. Structured absence must be represented explicitly, independently of JSON null; Some(null) remains a present structured report. Protocol rendering must use lossless optional parts. If legacy Rust tuple/accessor signatures are retained, document their inability to distinguish new absence rather than claiming lossless compatibility. No invalid operation report may be published as ordinary structuredContent, and no executed failure may be mislabeled preflight.

FMT-026 recovery qualification: approved closed target_object_history_unproven warning on validated retained-journal load. Fresh byte/mode recognition cannot prove persistent per-target object continuity or negate a previous receipt mismatch. Current-invocation mismatch remains Uncertain/PublicationIo; historical durability remains unconfirmed. No target/stage object identity or new phase is silently added to the journal. This preserves the original refined cooperative recovery model and explicitly distinguishes it from stronger live receipt evidence.
