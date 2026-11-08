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

FMT-026 accepted in `a4b322150c494e5d7245252ace109b7be9e62c1a`,79committed paths verified. Final compatibility/authority/limits: formats/breaks/012.md. Three registered schemas30corpora, required-nullable evidence, explicit grants, typed preflight and structured absence pass actual domain/surface checks. No source declaration/feature/Redbook or install-g grammar activation. Native qualification remains separately labeled. Earlier candidate paragraphs preserve chronology.


## FMT-027 — Explicit package source declarations and source-media migration

A1 introduces optional package.sources with typed presence: absent is None/legacy, explicit[] is Some(empty)/no semantic sources, nonempty rows use a shared selector. Presence survives parse/serialize; explicit JSON null refuses. Each row requires exactly one normalized file/directory selector and explicit media, interpretation, role and loading; no suffix/content/trust heuristic grants Vibe meaning. Exact file outranks directory defaults; longest component-delimited directory wins; equal-specificity conflicts become typed input requirements. Portable reserved generated-output aliases refuse declaration admission; exact G3 payload ownership does not broaden.

No project.sources or source_exports placeholder is added. Main currently rejects project.sources, even[]; the documented future project absent/empty legacy rule applies when that field actually lands. Named exports remain A2. No new vibe install -g CLI switches are introduced; the package manifest grammar itself changes and older strict readers may reject the new key. Actual supported runtime paths remain subject to final acceptance.

Approved C2 extension: optional non-null source_decisions exact-file array with five actual descriptor fields; one actual source-media question plus resource_path for that field. Neutral requests/replies omit the new optional data and preserve existing bytes. Current prerelease epoch1 gains this real capability; older strict readers do not automatically accept it. D1 operation/journal shapes remain unchanged because they retain/replay approved planning JSON; target-reader integration must recognize the real source recipe before application is advertised.

Admission closure uses a root-aware held package-policy observation outside the pure specdoc pivot. MissingManifest is content-tree compatibility, not verified legacy package authority. Explicit-empty contributes no semantic sources; unsupported declared consumers refuse before interpretation/mutation. Facts adoption must pre-admit rederive before registry upserts. Direct doc translation helpers borrowed/borrowed_by now return Result and actual build/agent/server callers propagate typed SourcePolicy failure instead of pretending examples are empty. Success document wire shape remains unchanged; Rust callers must handle the new error return.

FMT-027 compatibility refinement: semantic consumers now refuse a present invalid selected-root manifest instead of treating its mere existence as a package marker. Existing conversion perimeter fixtures were corrected to valid absent-sources packages with assertions unchanged, and the former fixture=true marker is retained as an explicit refusal/no-write regression across direct/package/spec conversion. Guard-only source-policy checkpoints are separate from held source-read capabilities; no new serialized format is introduced for these internal observations.

FMT-027 runtime boundary: the first supported declared consumer is exact-file facts adoption from original Mixed slots, plus executable source-media migration through the existing planner/apply protocol. Classified ordinary boot/transform consumers and directory census remain named unsupported operations until their later source tasks; this is not completion of all source features.

Direct library materializers now also reject a present malformed/unreadable vibe.toml; an absent manifest retains legacy content-tree compatibility. Copy preserves captured source bytes and permissions when placing files; requested Hardlink retains its checked link/copy-fallback behavior. Recorded observed files reconcile mode-only changes through replacement, preserving old shared inodes. Slot-record wire remains byte-hash based and does not acquire a persisted metadata/authentication claim; incumbent matching unrecorded occupants stay unchanged.

Known declared resolutions preflight supported input/census/conflict conditions before package metadata, slots, facts or handlers are published. Final materialization still captures/rechecks its inputs; this finite preflight is not a whole-install transaction and later source changes can produce ordinary partial failure. Existing outer lease acquisition, empty run-directory allocation and optional diagnostic compile tracing are preserved. Unsupported admission does not promise zero filesystem activity globally; the exact CLI bootstrap test covers tracing disabled.

## FMT-028 — Named ordinary Markdown source exports

`[[package.source_exports]]` adds strict `name`, exact authored `file`, and nonempty normalized `headings` to vibe.toml. Names are package-unique ASCII `[A-Za-z][A-Za-z0-9._-]*`; unknown members/null refuse. Both source metadata fields preserve absence versus explicit empty arrays. Presence of either `sources` or `source_exports` selects explicit package policy; export-only input grants no implicit semantic sources and cannot revive suffix scanning. Ordinary exports require an actually selected CommonMark descriptor and captured heading resolution; they never mint Vibe facts.

The same manifest reader governs `vibe install -g` packages: this adds no separate global format or global-root/mount behavior. Old strict readers reject the new field; absent-field serialization remains compatible. Package/global manifests are not automatically rewritten, project source fields remain unsupported, and package Simple/Normal stays independent. No slot record, migration request/report, cache or saved apply wire epoch changes are intended. CommonMark source ranges refer to original bytes and selected spans remain snapshot-bound; no URI/CLI/MCP transport is introduced in this atom. The library reader is the consumer in A2; this does not advertise a new command or transport.

FMT-028 verification: strict core grammar/presence/duplicate-name tests pass; captured ordinary export selection returns the existing source witness plus a document-borrowed section. Invalid UTF-8 only makes the optional ordinary structure unavailable; raw remains available and unrelated native facts remain complete. Actual Mixed CLI install → independently lock-selected held slot/record/manifest/document capture → named library resolver → facts adopt produced zero normative facts from ordinary content. Empty and nonempty export-only metadata also adopted zero facts, and actual source-policy guard matrices preserve no suffix fallback. The existing declared transformed-install refusal still applies to export-only packages before product metadata effects (coordination bootstrap remains the FMT027 exception). Frozen migration profiles are unchanged and do not acquire a new export-migration recipe. No new serialized slot/facts/cache/transport shape was introduced beyond the package manifest field. Root final joined CommonMark12/facts31/CLI14 checks pass; the final specification map retains all prior units with no new warnings.

## FMT-029 — Declared materialization and installed boot reads (A3 accepted)

Accepted in the A3 product commit recorded below. Existing package sources/source_exports declarations now require descriptor-selected conversion/read routing instead of filename fallback. Original logical source names own native addresses; converted outputs retain the incumbent .md/.xml output naming rule, so real output collisions refuse rather than receiving invented filenames. Ordinary CommonMark/XML-data/opaque bytes remain original; optional XML inspection never caps raw retrieval. Directory defaults are selected over the materializer's finite payload inventory, not claimed as a global source census.

No new durable lock-format field, slot record schema, compiler IR epoch or source-export URI is planned. Independently selected materialized input format is carried from install/command selection and is distinct from the boot output rendering target; an unbound declared reader refuses rather than infer input format from STATIC suffix or its own slot record. This uses the same manifest/selection reader for packages installed through vibe install -g; it introduces no alternate global manifest grammar or mount layout. Generated STATIC/substitution/replay inputs retain existing artifact ownership and Normal/Simple semantics, not authored descriptors. Concrete command/API compatibility, recipe decision and actual joined tests will be recorded before acceptance.

FMT-029 carrier candidate evidence: the actual installer/unit emitter now maps authored logical paths to recorded physical outputs and selects native media independently of suffix in Simple and Normal. New additive bound analysis/native/backend library overloads receive independently selected materialized-input SpecFormat; extensions analyze/compile call the existing configuration selection owner and these overloads. STATIC suffix remains only the output lane target. An unbound declared boot-graph verifier refuses instead of deriving a false fingerprint; the CLI check command's existing best-effort handling of that error remains a separately recorded A4 consumer gap.

The generated XML stream reader uses the incumbent stream/annotation/literal/framing owners, including real multiple roots, nested c1 and hoisted forms. Only actual internal generated initial seeds enter that channel; ordinary references are admitted before cache reuse. Projection preserves observable prior framing as data, not new current source/EmbedTape authority. Canonical positions are projection-relative (XmlProjected), while raw bytes/digest remain separate. No compiler IR/native reply ABI or slot-record epoch changed. The existing specdoc/4 converter recipe is retained for unchanged incumbent output semantics; this is newly admitted declared routing, not a silent legacy emitter rewrite.

Actual carrier gates now cover Mixed/Markdown/Xml raw roundtrip through materialization AND held bound reading, absent selected hash at the real emitter despite fresh artifacts, independent input-format mismatch, no-descriptor byte parity, actual generated Normal substitution with inner embed/literal/current-attribution checks, and real binary install→extensions analyze under Markdown/XML with no project writes. Managed native, explicit backend and sealed replay are exercised through existing provider seams using test providers, not claimed as fresh DLL/platform qualification. Replay prepared bytes/owner/target are checked; changed record and injected loss of the retained source-hash context refuse without publication. Full acceptance and final combined map/format gates remain root-owned.

FMT-029 final-candidate reuse refinement: declared bound analysis now requires the selected generated unit's existing INDEX freshness evidence and the expected STATIC representation before consuming a substituted input. Missing/malformed/stale INDEX, missing STATIC or an opposite-format stale sibling refuses. A validated observation remains stable through consumption; prepared replay publications can supply their newly generated overlay instead of obsolete disk bytes. This adds no persisted receipt, content-authentication claim, lock field or manifest key. An edited STATIC with otherwise valid incumbent freshness metadata is not newly claimed to be cryptographically authenticated. Legacy unbound behavior remains its existing contract.

FMT-029 acceptance: original M06A3 accepted in `3fcf514a5cf9a60262fe233f6480ea63f14a58f5`; root verified 93 committed paths, final runtime14/CLI2, static gates and specification-map preservation. Earlier candidate paragraphs retain chronology; the accepted scope and limits are research/m06-a3-transfer-acceptance.md. No additional authored manifest key or persisted schema was introduced by A3.

## FMT-030 — Selected installed package requirements queries (accepted A4 slice)

Accepted as the bounded package-query slice recorded below. The trusted Rust QueryContext gains materialized_format: Option<SpecFormat>; callers constructing struct literals must provide the field. This is an internal/library API compatibility change, not a JSON/MCP query member or report-schema change. Actual CLI and MCP surfaces select the format with the existing project/operator configuration policy; they do not infer it from a slot record or STATIC suffix. None retains legacy package support but cannot authorize a declared installed read. Package/global install manifests gain no field in this slice; project.sources remains a separate required A4 slice and is not activated here.

Selected package queries will use the A3 original-to-output map and independent lock identity/hash. Witness paths remain original logical document names while their digest describes the actual materialized bytes read, not an unobserved original-file hash. Native syntax/duplicate failures retain the existing Invalid state with captured witnesses; inability to establish binding or bytes uses Unavailable with no fabricated digest. Malformed lock/registry still aborts the query. Query-specific preparation may defer syntax interpretation but cannot relax record/media/address/hash validation; install preparation keeps eager native validation. Final compatibility and actual CLI/MCP test results remain to be recorded.

FMT-030 compatibility refinement: recorded legacy slots now pass the existing validator's manifest/selected-hash coherence check before falling back to legacy document parsing. Incoherent recorded metadata becomes Unavailable; this is a deliberate stricter result than the old query that ignored the slot record. Unrecorded undeclared slots retain their content-tree compatibility route. This does not authenticate arbitrary edited legacy payload bytes or introduce a new record format. Facts native interpretation uses the existing direct Spec/Doc pivot parser, avoiding an XML-to-Markdown-to-AST roundtrip.

FMT-030 report detail: an installed source whose binding/read cannot be established uses the existing Unavailable source state with reason_code `installed-source-binding-unavailable` and no digest. This is a new diagnostic value within the existing report grammar, not a new field or schema epoch; consumers should not interpret it as an available empty package. The reusable facts API now derives interpretation rows from the same attached resolver binding that reads bytes, eliminating an independent metadata argument.

FMT-030 accepted in `12ee51b0ebcb185ede2ac94f08ded0de7694caa4`; final scope and compatibility are research/m06-a4-package-query-acceptance.md/json. Full A4 and project.sources remain separate unfinished work.

## FMT-031 — Explicit project source declarations and cited host sync (accepted slice)

Accepted as the bounded host policy/sync slice below. Optional [[project.sources]] reuses package descriptor fields and presence semantics: absence remains legacy, explicit empty grants no semantic source, null/unknown members refuse. Older strict manifest readers reject the new field. Public Rust ProjectSection literals gain an optional sources field; existing constructors keep None, so old scaffold bytes are not silently opted in. Existing project.group remains optional in grammar; address-minting/resolving consumers require a valid self coordinate when used.

This changes the shared vibe.toml reader, including its use around vibe install -g, but creates no alternate global manifest dialect, mount layout, slot/lock schema or machine-report epoch. Grammar and held policy/guards are activated together. Cited host sync refuses unavailable/reclassified source rather than interpret it as deletion; absent anchor in valid admitted native content retains the existing tie-break. The accepted behavior and compatibility limits are recorded below; project.source_exports is not introduced.

FMT-031 guard/continuity detail: unsupported declared progress seal inputs now refuse before native parsing or seal-cache updates, using the named file's nearest project/package policy. Existing per-file legacy seal behavior remains. Host sync retains/rechecks the initial manifest policy for legacy as well as explicit sources before registry publication; a concurrent switch to declared policy is a refusal, not permission to finish using the old legacy interpretation. No new cache/registry schema or global transaction is added.

FMT-031 accepted in `c24dfc6c64a2d900a05b5464f0bc510d4efda925`; research/m06-a4-host-acceptance.md/json records actual guards and limits. This supersedes earlier FMT027/028 statements that project.sources is unsupported; project.source_exports remains absent. Full A4 is not accepted: nonempty host boot and broad host consumers still need support.

## FMT-032 — Declared host point reads accepted; enrollment pending

Point-reader support is accepted below; automatic enrollment remains pending. Existing spec:// self-coordinate lookup consumes declared host native bytes/media rather than remain an unsupported or suffix route. No new URI grammar, host format, source export field, lock/slot record or wire epoch is introduced. Actual caller-selected identity and current versioned-self refusal remain. Installed slots still require their independent installed binding.

Loading is operation-specific: requested native lookup/citation and facts/query may use native explicit; ordinary witnesses do not create facts/directives. Automatic own-boot discovery will require native+scan in boot scope; explicit package boot_snippet selection remains separate. This automatic host enrollment rule is a clarified target behavior, not a claim of completed prototype support. Point-reader API compatibility is recorded below; later consumer activation remains separate.

FMT-032 candidate detail: slot-record filename ownership moves to the existing core layout module with the workspace export retained. No stored filename, record schema or parser changes. Its presence can reject a derived slot masquerading as an unbound host; it cannot supply expected hash/identity. Native host point lookup must preserve both existing canonical ID and full physical slug spellings, while raw neighbors remain non-native.

FMT-032 point-reader acceptance: `d2c283e6970432f563a9c44129daf1ada750725a`, research/m06-a4-host-read-acceptance.md/json. Public metadata shape and Send/Sync remain; captured original media supplies canonical frontend text. Pin transport is unchanged and does not imply a new revision-equality validator. Broad host census/query and automatic own-boot enrollment remain pending.

## FMT-033 — Declared host facts and requirements inventory (accepted slice)

Accepted as the bounded host facts/query slice below. Existing project.sources gains finite whole-declared-scope facts/query observation; no new manifest field, wire member, URI, host format or record schema is introduced. Exact declarations and declared directories use the same selector. Native explicit remains valid in requested observation; ordinary bytes may be witnesses but never facts/directives. Invalid complete native input differs from unavailable census/read, with no fabricated empty digest for failure.

Named inventory scope rule: directory traversal excludes .git control entries and existing generated ownership; exact declarations into .git refuse. This is documented scope, not a hidden global blacklist of dot/build directories. Nested vibe.toml files inside selected outer scopes do not supply policy/identity; outer descriptors still classify their bytes. This does not activate unrelated nested projects or broaden source ownership. Actual materialized-root reads retain installed binding requirements. Cited-only sync remains narrower and unaffected by unrelated malformed files. Accepted API/report compatibility and tests are recorded below.

FMT-033 accepted in `432761746296c7913297cb7daad1302d30fc3b4b`; research/m06-a4-host-query-acceptance.md/json records final scope. Host observation failures use existing Unavailable with reason_code `host-source-observation-unavailable` and no digest; syntax/duplicate Invalid remains distinct. The report grammar and digest recipes are unchanged. Own boot/fullA4 remain open.

## FMT-034 — Declared own/inherited boot enrollment (accepted slice)

Accepted as the own-boot slice below. Automatic own boot uses VibeSpec+scan in the existing immediate boot directory; ordinary/documentation and explicit-only native files are not enrolled by location. This clarifies operation selection for the already introduced source rows, not a new manifest field or host format. Existing own/inherited Dynamic+Simple linkage, category/order and INDEX shape remain. INDEX kind=static means unconditional direct reading and is not compiler static linkage.

The temporary blanket refusal for nonempty host source policy is replaced by supported scoped preparation before effects and retained checks before actual publication, including zero-static/native/replay paths. Ungrouped hosts keep the existing provider and no fabricated group. No new record/lock/IR/wire epoch or automatic rewrite of authored source bytes is planned. Accepted API/behavior evidence is recorded below.

FMT-034 accepted in `33a680809c903a0fa0e40fbdb64640126b19296d`; research/m06-a4-own-boot-acceptance.md/json. The operation-enrollment contract moves r1→r2 for the explicit VibeSpec instruction rule. Nonempty host install no longer fails merely for opt-in policy; actual selected invalid input still refuses before effects. Public AuthoredBoot/INDEX/storage/wire shapes remain unchanged. Publish staging and remaining A4 consumers stay separate unfinished work.

## FMT-035 — Bound boot graph check and visible inability to verify (accepted adapter)

Accepted as the adapter below. An additive selected-format library verifier reuses existing binding/table/fingerprint semantics. Existing unbound verifier remains. CLI check uses the actual project/operator format rather than suffix/record guesses. No new manifest key, CheckId, wire schema or command option.

Behavior change: no durable lock remains non-applicable; stale/missing unit INDEX remains Warning and warning-only exit0. Applicable load/configuration/binding/verification failures now become BootGraphIntegrity Error, JSON ok=false and error exit after normal report emission, instead of being silently discarded. Verification remains read-only with no lifecycle lease or regeneration. This does not broaden fingerprint checking into STATIC-content authentication. Final receipts and compatibility are recorded below.

FMT-035 accepted in `04e7ccad8ead2002fd49d936e6917b8631e7006a`; research/m06-a4-boot-check-acceptance.md/json. The existing warning-only behavior is retained; inability to verify an applicable graph now fails visibly. No wire/manifest schema, CheckId or storage format changes.

## FMT-036 — Declared package facts report/adopt (accepted slice)

Accepted as the adapter below. Supported transformed and record-backed directory sources use the existing installed map/observer with independent lock hash and selected format. Report counts/status fields remain truthful on invalid/unavailable source; diagnostic/API details are recorded below. No new vibe.toml key, registry entry schema, source-hash recipe or global install dialect is planned.

Transformed adopt still executes real post-registry rederive so overlay statuses reach derived payload/record. Before upserts, exact offline source/config/destination and prospective overlay conversion are checked through existing owners without publishing a temporary registry or materialization. Package source_hash, output row hashes, exact registry overlay_file_hash and derived_hash remain distinct. Existing entries/prefix/void/Mixed no-op and legacy behavior remain; declared in-place still needs a genuine binding owner and is not fabricated. This is prevalidation, not whole-command atomicity.

FMT-036 workspace compatibility: shared slots continue deriving from workspace-root registry. Member adoption changes its local registry, while preflight and retained rederive consume the same captured workspace overlay; prospective member additions must not be mistaken for that input. This does not add per-member slot views or move registry data. Report remains text-only; aggregate authored total is '?' when any source total is unknown, and invalid materialized input is distinguished from absent slot.

FMT-036 accepted in `728d41319b04b2e527058ed29e77e4d88209c566`; research/m06-a4-facts-command-acceptance.md/json. Public Rust additions are package_overlay_with_additions, preflight_declared_derivation and its retained recheck guard; no registry/hash/wire schema changed. Present invalid workspace lock is now rejected from member commands too; genuine absence and valid cleanup remain compatible. No per-member slot ownership change.


## FMT-037 — Captured documentation point reads (accepted slice)

Accepted adapter: existing citations and agent page reads retain declared media and original logical paths rather than rereading by suffix. Selected malformed lock/binding must fail instead of falling back to a store instance. Installed declared inputs require trusted materialized format from actual CLI/server composition. Additive Rust context/capture API; no new vibe.toml, install -g, URI, Doc IR or report schema. Full page census/borrowed joins remain separate. Record final actual APIs and compatibility evidence at acceptance.

FMT-037 implementation perimeter approved: expose existing PreparedInstalledSource captured manifest by borrowed accessor; expose existing PackagePolicyObservation materialized_record_present presence probe for authored consumers. These are Rust API additions only; the probe neither decodes nor authenticates records. Existing internal vibe-safefs becomes a direct vibe-doc dependency. Final names/behavior remain subject to review and tests.

FMT-037 additional Rust error distinction under implementation: SourcePolicyErrorKind gains MissingNativeAddress for zero admitted native candidates after ordinary validation. It does not mean a selected/promised file was safely read: ambiguity, missing promised file, invalid policy and binding/read errors remain failures. The doc adapter may use only this typed absence for legitimate adaptation fallback. This is a public Rust enum addition (exhaustive downstream matches may need updating), not a manifest/wire change. Existing host cited synchronization must retain refusal rather than interpreting this as anchor deletion.

Compatibility note — tutorial correction `839fdd0c3` changes no format or default: omitted package manifest format remains Simple, init package scaffold remains Normal, omitted --link remains Dynamic and explicit Static/Dynamic remain honored. EN/RU prose and root layout comment were corrected only.

FMT-037 accepted `7859d3e2dc0d94991cbc9908c6fc0b1c23b0d31f`: SpecSources::capture_document/with_materialized_format, CapturedDocument relative/parse/recheck, PreparedInstalledSource::manifest and materialized_record_present; MissingNativeAddress distinguishes native absence. Actual installed Mixed/Xml, original Doc page identity, missing-versus-invalid adaptation and workspace-member CLI selection verified. No declared page census acceptance.


## FMT-038 — Generated documentation input boundary (accepted slice)

Repair level0/failure-page rendering through an explicit producer input to the existing build/card pipeline. Preserve actual subject identity and optional original kind; do not add pack to PackageKind, invent kind=doc or weaken authored manifest validation. Internal Rust composition change; no new vibe.toml/install-g/URI/DocIR/wire schema. Correct existing fixtures and lock-field expectation to current valid formats, without changing those formats. Final actual API/compatibility details pending acceptance.

FMT-038 candidate detail: unknown-kind generated work metadata now omits kind rather than writing the non-PackageKind word pack; known original kinds remain. This changes internal generated diagnostic card spelling, not authored package grammar. Prepared generated page AST and Card fields are retained in memory; existing ancillary translation/glossary/media/review owners still read their normal inputs. No claim of a globally immutable render snapshot or zero filesystem rereads.

FMT-038 accepted `8e61e9c754f8d9d9467c75e394c7833f0f68e096`. Internal Generated/PageSet/Card/shared finish and Card parsing module restore failure/retry and unknown-kind rendering; no public package kind, authored key, install-g default or wire schema added. Full doc library634PASS closes all62previous failures; four new boundary/differential tests pass. Ancillary filesystem inputs remain outside the captured PageSet/Card scope.


## FMT-039 — Selected Doc inventory and actual consumers (accepted slice)

Extend declared Doc support from point reads to selected page enumeration, build/server, self citations and borrowed examples. Preserve original page identity across physical conversion, raw exclusion and strict bound installed reads. Intended Rust preparation/context changes; no new authored manifest/install-g/URI/DocIR/wire schema. Existing public bare-path compatibility or refusal must be explicit. Final APIs, route compatibility and source/output behavior will be recorded at acceptance.

FMT-039 design decision: preserve the public plain Built struct shape. Actual selected publication will use an additive prepared-build/write wrapper retaining source observations; plain Built remains output data and has no retained source-guard claim. SpecSources capture_pages(explicit_root) and a lower native-only inventory operation are planned additions over existing owners; exact names remain pending implementation. Recheck must retain the same selection scope. No new wire/manifest fields.

FMT-039 compatibility review: preserve legacy fullslug XML routes and the public document_of borrowed-string contract; canonical resolver aliases must not silently rename existing served pages. Support declared XML under misleading.md separately through captured identity, with concrete legacy route controls. Draft blanket canonicalization was rejected before acceptance.

FMT-039 real-surface correction: Doc packages remain read/warmed with cache add and refused by vibe install under PROP057. No Doc install support is introduced. Cached/original Doc builds are actual CLI coverage; constructed materializer+lock cases qualify the lower bound reader only. Existing installed-flow citation coverage remains.

FMT-039 explicit API tightening: bare recorded legacy page roots previously reached read_package's XML walk; they now refuse without independently selected context. Ordinary unrecorded legacy remains supported, and valid independently bound legacy is covered. This prevents a materialized root with stripped source declarations from entering the authored route solely by path; no provenance authentication is claimed. Actual Doc CLI remains cache/store based. Point absence errors preserve their existing prefix and add truthful absent/unclassified-native context; no diagnostic schema changed.

FMT-039 LLMS compatibility: generated page links and copied spec addresses now reuse the shared document spelling helper, so declared native XML stored with .md links to the same logical page as build/server. Legacy fullslug/xml behavior and wire schema remain; no shell change. This fixes a duplicated suffix rule, not a new source interpretation rule.

FMT-039 accepted `3cafec8b7c795811daa08ce17ccf116805817bc5`; research/m06-a4-doc-census-acceptance.md/json. Additive capture_pages/native inventory/prepared build and borrowed wrappers retain selected observations. Public Built/Page shapes and document_of return type preserved; .md presentation support and recorded-legacy tightening are explicit. Actual Doc cache CLI verified; no Doc install feature or wire/manifest epoch change.


## FMT-040 — Declared native level0 projection (accepted slice)

Complete native source projection for non-Doc packages and host projects without routing Spec inputs through a Doc-only census. Honor declarations outside conventional SPEC_ROOT and retain bytes/observations through composition. Preserve incomplete card-only subjects and existing legacy presentation. Intended internal Rust preparation changes; no new package kind, manifest/install-g/URI/wire schema. Final exact compatibility and tests pending acceptance.

FMT-040 planned Rust seam: native membership presence query reuses existing selector/inventory and grants no parser vocabulary. Declaration-presence hints reuse already decoded Card metadata; no new TOML/source descriptor decoder. Known layout prefix comes from the existing owner. Public wire/manifest formats remain unchanged.

FMT-040 accepted `a3bd65d92971144b9c5cd20338c3455c1131e7ea`. Existing Card presence hint, additive native membership query and retained native projection/copy use existing owners; public Built/Page and authored/wire formats unchanged. Declared native files outside SPEC_ROOT participate, actual vocabulary/kind preserved. Materialized original-root level0 input without independent context still refuses. CLI host feed not expanded to ordinary flow packages.


## FMT-041 — Selected native tree directive reads (accepted slice)

Join tree's existing selected boot-file paths to captured source interpretation and independently configured installed bindings. Preserve tree JSON/carrier schema, committed load attribution and original path/projection-line convention. Intended additive internal bound API; old build_tree compatibility remains qualified. No new manifest/install-g/URI/CLI option. Final public errors/API compatibility pending acceptance.

FMT-041 additional decoder repair: existing emitted once-each elision annotation must be decoded by the framing owner rather than treated as source path. Preserve emitted bytes/compiler recipe and ordinary semicolon path data; no artifact epoch/schema change is intended. Existing selected-slot admission requires real legacy manifests for legacy fixture setup; exact tree JSON assertions remain.

FMT-041 accepted `954ffc3c197edc02e7ec2f0b33aed8878ba9fabb`: additive internal bound tree read and public static_marker_source_path framing reader. Existing tree schema/carriers/load attribution and emitter bytes/recipe retained. Unbound declared installed Empty/nonempty now require independent format; original host Empty remains zero. No authored/wire format changes.


## FMT-042 — Declared source display in show effective (accepted slice)

Preserve existing boot/generated/WAL/files_written display universe and report fields. Add declared-media native display and independently bound installed reads; explicitly named ordinary UTF8 stays literal, unsupported binary display remains honest. Legacy provenance-header grammar is retained as display metadata, not a new resource URI or grant of native membership. No new manifest/install-g/schema/CLI option; exact API/error compatibility pending acceptance.

FMT-042 accepted `8aaea9d7cbba510509b2804b695fb3b7be9bf153`: declared native display, literal named UTF8, independent installed binding and retained observations; no report/manifest/install-g/URI schema change. Existing generated/WAL/header/MISSING behavior retained. Binary remains an explicit display error; no new encoding.


## FMT-043 — Captured progress corpus and derived-output barriers (accepted slice)

Preserve progress.toml include/exclude/default scope and projection-based cache hash/schema. Add declared-media reading before cache reuse and retain the same projection for weave; guard each existing derived write boundary. No new vibe.toml/install-g/URI/cache/state schema is intended. Seal and human gate input remain separate; final API/error/identity details pending acceptance.

FMT-043 boundary clarification: progress include globs/defaults are unchanged; a native exact File declaration selected by scope is an availability promise; ordinary inputs and directory defaults alone are not. Qualified unrecorded legacy slot-root reading remains possible without invented lock context; recorded/declared inputs retain independent binding. Installed record integrity is not weakened by source-scope filtering.

FMT-043 accepted `751c53600834df31a79d20b8aeaba772be600b1b`: scope predicate + captured source/evidence/projection and actual write barriers. Existing include/default/exclusions/cache/state schemas and projection hash unchanged; qualified legacy/recorded context distinctions explicit. Seal remains separate.

## FMT-044 — Shared source evidence and durable application provenance (planned)

VIBE-NEEDS1/2/3 foundation. Preserve canonical origin separately from actual acquisition/read route and unknown legacy data. Existing closed application index4/steps2 must not receive silently incompatible fields: new index5/application-e4 and steps3/application-e3 readers/writers plus existing proxy transition are planned. Read old epochs with unknown provenance; minimum writer and compatibility tests remain required. Generated shared/report types only; no hand-edited generated Rust. Source-kind meanings, package identity/hash and portable import semantics must not be redefined. No project plan-only/pin/hosting-exclusion completion implied.

FMT-044 scope extension: existing import_work cohort also needs a new strict epoch/protocol3 to retain source evidence before pending steps exist. Prior cohort readers remain frozen; old missing evidence stays unknown. This is the existing cohort owner, not a new sidecar. CLI shared evidence uses the existing permissive-reader projection; durable records stay strict.

FMT-044 documentation correction (candidate): application plan schema previously claimed computation without downloading or running anything. Existing resolution may acquire packages and write cache/configuration. The description now limits the promise to no installer-step execution; strict no-write project preview remains a separate unimplemented effect-contract task. This corrects an overclaim, not a new grant to perform preview side effects. Existing report schema formatting was restored to keep the source-evidence diff reviewable.

FMT-044 candidate lock semantic correction: the shared source-kind helper will preserve a known recorded kind for store-backed acquisition and leave unknown legacy kind absent, rather than manufacture registry from route flags. Direct acquisition selection meanings stay unchanged. This uses the existing optional lock field; no new vibe.toml grammar or lock epoch is introduced. Final acceptance/test evidence pending.

## FMT-045 — SYSTEM prompt convention (design proposal only)

Proposed authored boot/SYSTEM-TEMPLATE.md literal opt-in; generated boot/SYSTEM.md and versioned SYSTEM.meta.json. All three need shared role/reservation rules to avoid own-boot self-inclusion. Existing AGENTS/static/INDEX route remains compatible; template absent is no-op. No template DSL, arbitrary include expansion or automatic home-state export. API prepare/check must be read-only and cannot execute effectful compiler providers. This proposal has not changed vibe.toml, install-g or any live wire format; exact generated manifest schema belongs to later implementation. See architecture/SYSTEM-PROMPT-SPEC.md.

FMT-044 accepted in2df122323: index5/e4, steps3/e3, import-work3/e3, additive generated source evidence in legacy/common application reports, strict frozen readers/minimum writers and guarded proxy compatibility. Existing optional lock source_kind preserves known/unknown store history. No new authored vibe.toml grammar. Actual project event/source policy/read-only preview not delivered.

## FMT-046 — Real project install plan source observations (planned)

Preserve the actual live `install:plan` envelope and `packages[{package,version}]` rows, adding shared source_evidence from held preparation. The historical cli-install-plan epoch1 schema instead describes `plans` with fields not emitted by this producer. Correct this explicitly through a new registered epoch/current generated report with the old schema binding retained, not an undocumented reinterpretation of a frozen format. Keep command/envelope construction at the CLI boundary and existing deferred/parked event timing. No project read-only-preview/pin/preference/exclusion support is implied.

In-place preparation currently feeds the old lock registry alias into a prospective CachedPackage even though it holds current canonical source/ref/commit. Capture the actual selected alias in the preparation owner; do not describe old registry settings as a fresh choice. Any resulting correction of the existing lock registry field must be included in acceptance evidence. No new authored vibe.toml grammar is intended.

FMT-046 acceptedc600cb539: current cli-install-plan epoch2 matches live command/packages and adds optional source_evidence; old declared schema bytes retained as cli-install-plan-v1. No new runtime protocol field/command and no timing change. In-place current alias now reaches lock.registry; source URI/content identity/placement unchanged. No new vibe.toml fields or no-write preview.

## FMT-047 — Derived hosting views (planned)

Add optional report-only source_hosting alongside source_evidence in project install events and global app plan/list/describe, plus installed_hosting beside installed history in describe; common-configuration counterparts share the model. HostingView selected_origin and differing_acquisition_route are optional extensible label strings, derived from the corresponding held URLs only. Persisted SourceEvidence, index5/steps3/import3 schemas and contents remain unchanged. No new vibe.toml/settings/CLI flags or preference/exclusion behavior in this slice. Actual compatibility and field naming confirmed by final schema/golden review before acceptance.

FMT-047 accepted1f0b53aa0: optional source_hosting across six report schemas and installed_hosting for both describe forms; open String labels, no new epoch/REGISTRY entry. SourceEvidence, all persisted inventory/work schemas/readers and existing corpora unchanged. Exact known-domain URL observations are not contact/permission facts.

## FMT-048 — Internal HTTP-only admission binding (planned)

Introduce typed internal client/backend refusal and pre-construction binding; no authored settings, CLI flag, persisted schema or report wire field is currently proposed. Existing unbound behavior remains. Checked APIs must retain policy denial through probe/name/catalog/raw fallback instead of absence/None; any Rust signature changes are recorded with actual caller updates. Native fallback under this temporary HTTP-only capability refuses; this is not final Need3 or full source exclusion availability.

FMT-048 accepted57d30367f: new internal HTTP-only binding and typed ContactRefused; checked freshness/name APIs return Result with callers updated. No wire/persistence/config/CLI grammar change. Shared core normalized_host_key removes one terminal DNS root dot, preserves IP/IDNA and rejects malformed empty labels; equivalent dotted report hosts now classify consistently. Credential-origin comparisons and original source URI bytes remain unchanged. Public exclusion/native Git availability are not delivered.

## FMT-049 — Admitted native HTTPS preparation (planned)

Separate non-serialized native capability/profile with explicit caller-trusted installation/trust inputs; accepted HTTP-only capability remains strict. No CLI/settings/wire/persisted-source schema change intended. Captured effective warm endpoint cannot be replaced by raw recorded origin; unsupported configuration meaning refuses. Existing source/commit identity and guarded publication owner remain. Exact Rust API/error changes recorded at acceptance.

FMT-049 accepted46b6297a3: separate native direct-HTTPS library capability and auth/materialisation preflight API, explicit trusted installation/TLS inputs and Git>=2.53 floor (tested Linux2.53.0). No wire/config/CLI grammar or dependency change. Source identities remain; actual owned HEAD now differs correctly from selected revision. No N2/public-policy completion.

## FMT-050 — Native staging and checked slot publication (implemented)

Retained native materialisation/captured config inputs and neutral workspace placement/undo interfaces; no new authored manifest, CLI policy or persisted wire format intended. Early capability/transform admission must precede caller metadata writes. Fresh copy/cache and in-place semantics remain distinct. Shared rollback/park behavior changes, if needed, require explicit compatibility evidence; no generic transaction framework or registry→workspace dependency.

FMT-050 implemented: additive Rust acquisition/placement interfaces retain Cached versus InPlace ownership; no `vibe.toml`, `vibe install -g`, lock/provenance, CLI or report-wire grammar change. Checked workspace rollback failures propagate as RollbackFailed rather than a successful parked continuation. Mixed native plus legacy deferred mutations explicitly refuse before publication; ordinary cache-backed packages can share the checked gitignore undo. Unix identity support is required for this profile. Full workspace self-check passed on 2026-11-07; accepted in the explicitly qualified scope.

## FMT-051 — Retained authored workspace inputs (implemented)

Workspace retains original root/member manifests before version-variable finalization. Source admission compares their complete TOML wire semantics, respecting unordered requirement maps while retaining variable identity, source policy and other authored fields. Known selected-node changes advance only their owned input; other nodes and membership remain checked. No `vibe.toml`, `vibe install -g`, lock/provenance or JSON/wire format change. Internal Rust construction changes deliberately: the unpublished `vibe-workspace::Workspace` gains a private authored-input snapshot, so external struct literals must use existing `load`/`discover` factories. `WorkspaceMember` literals remain unchanged. This avoids a parallel family of bound APIs and is not a claim of backwards-compatible Rust struct construction.

## FMT-052 — Explicit SSH capability (implemented)

Additive internal Rust profile/authentication types, typed redacted SSH failures and per-registry credential binding; no new manifest, user settings, CLI flag or wire grammar intended. Direct profile selection is explicit fresh/warm transport authority, not automatic legacy SSH-config migration. Public policy availability and N4 are separate. Actual final API/qualification recorded at acceptance.

FMT-052 implemented: SshProfile/SshAuthentication and default-compatible GitBackend registry-auth binding, fixed redacted host-key/auth refusal variants, and private HTTPS/SSH command metadata. Explicit-direct fresh/warm authority supports captured key or selected agent with strict host keys. No manifest/settings/CLI/wire grammar changed. Qualified on Linux Git2.53/OpenSSH10.2; conservative direct endpoint/path grammar, no automatic legacy SSH-config migration. Per-registry credential binding cannot widen on rebinding; anonymous HTTPS remains available across declared auth modes.

## FMT-053 — Admitted native route closure (implemented)

Internal source-route carriage, initial relocation trust and recursive exact-gitlink ownership extend current registry/staging owners. No manifest/user-setting/public flag or persistent schema change is assumed. Explicit HTTP discovery trust is distinct from native Git System trust; multiple SSH endpoints require exact caller-selected profiles without widening registry credential mechanisms. Record actual API/compatibility changes and qualification at acceptance.

FMT-053 candidate API inventory (not acceptance): `MultiResolution` now carries `selected_route: Option<RegistryRoute>`; manual Rust struct construction must supply this field. `RegistryRoute` has private fields, is runtime-only and does not itself authorize network or credential use. Native profiles add `with_ssh_profiles` (replaces the grant set; duplicate endpoints refuse) and `with_http_discovery_trust` with explicit PEM or bundled WebPKI roots. The existing single-profile builder still replaces the whole SSH grant set. Registry resolve/fetch entry points carry the selected candidate without changing canonical persisted source identity. These are Rust integration changes, not new `vibe.toml`, `vibe install -g`, lockfile or report-wire fields; public hosting controls are still separate work. Final qualification and acceptance remain pending.

FMT-053 implemented in the qualified internal native capability: opaque runtime RegistryRoute carriage, exact plural SSH profiles and explicit HTTP discovery trust; no authored manifest, `vibe install -g`, lock, provenance sidecar or report-wire grammar change. In-place actual-route information now reaches the existing report-only provenance field instead of being dropped; canonical lock source stays separate. Unbound warm actual-origin behavior is retained. Core routes are qualified on Linux Git2.53/OpenSSH10.2; ordinary SSH config capture and public locked/new source integration are not included. Full self-check v2 passed8/8 before the final existing-field provenance projection; that final delta passed install units6, native routes3, incremental17, strict install Clippy, Conform0new and independent review. New auth-presence semantics and LOCKED-SOURCE-LAW are future work and were not applied here.

## FMT-054 — Registry auth declaration presence and user SSH integration (planned)

Proposed `RegistrySection.auth` preserves omitted versus explicit `none` rather than collapsing both; `AuthKind` and package/redirect auth defaults remain unchanged. Omission may select transport-appropriate defaults for the source already chosen by its owner; explicit none remains a credential restriction. Explicit none must roundtrip instead of disappearing. Present invalid/null values stay rejected. This is a deliberate Rust construction and declaration-semantics change, not an already implemented format. The patch is currently prepared outside the worktree; all production consumers must lower deliberately before acceptance. No private key material is added to `registry.toml`, `vibe.toml` or `vibe install -g` settings. Existing shared locks remain source-authoritative and unchanged for unchanged selections. Record the final realized compatibility surface and supported SSH settings at acceptance.

## FMT-055 — Captured ordinary SSH profile foundation (implemented)

Additive internal library `SshConfigSelection`/`SshConfiguration` and native `with_ssh_configuration` APIs select audited immutable user/system SSH inputs, with endpoint-only preflight and actual-request admission before credential/trust lowering. Existing explicit-direct profiles retain their semantics. No `vibe.toml`, `registry.toml`, `vibe install -g`, lock/provenance or JSON-wire grammar changes in this foundation. Existing pinned glob/libc packages become registry dependencies without version changes. Qualified Linux/OpenSSH10.2/Git2.53 subset; ordinary CLI factory and FMT-054 declaration changes remain pending. Static/compatibility/runtime evidence is recorded in the campaign log; no user keys or profiles were modified.

## FMT-056 — Self import profile consent and doctor observations (implemented subset)

`vibe self import --use` preserves durable PATH and advisory HOME by default while still activating shims/current. New explicit `--yes` grants durable activation; `--no-profile` overrides it, including Windows user-environment persistence. Unattended alone is not consent. Other activation callers retain their incumbent policy. Doctor adds inline JSON `durable_environment` (target/shell/read-state/managed-block/path-entry observations) and `process_path_refresh_needed`; process PATH, durable configuration and unobserved login effectiveness remain separate. No shell startup execution. Doctor --fix requires explicityes or affirmative interactive consent and no longer counts an unchanged process PATH as repaired merely because persistent configuration was written. This can leave a nonzero remaining-problems outcome after successful durable repair. Existing externally configured PATH does not require a managed block. No vibe.toml or install-g grammar change. Actual gates/review precede final acceptance; optional login-probe behavior awaits the owner answer.
