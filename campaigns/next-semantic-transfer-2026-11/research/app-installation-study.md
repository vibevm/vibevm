# MAIN: structured application installation and option selection

Срез main: `1448ed5cc33a4fadbce9d8331bf5f2d96052aec3`. Сравниваемый next: `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Читал контракты `PROP-059-user-applications.xml`, `PROP-064-agent-hooks.xml`, `PROP-065-structured-installation.xml` и исходники/тесты по Git objects этих ревизий. Статусы adoption и старые acceptance-записи не принимались за доказательство. Сборок, запусков installer/checker, network calls и product/spec/plan edits не было.

## Что установлено

В main существует сквозной пользовательский installer/wizard, который можно вызвать из консоли или внешнего GUI через JSON CLI. Он действительно проходит декларацию выбора → проверку/план → installer effects → persisted installation choices → update/modify. Для этого не нужен bootstrap отдельного machine grant host. Это существенно более полный источник пользовательской механики выбора, чем отдельный `ConfigurationChoiceHost` next.

Семантика main сознательно ограничена обычной установкой: последовательные steps, один lock приложения на home, atomic replacement отдельных JSON-файлов, kept work/resume. Она не обещает dedup effect по request id, сохранение exact plan после смены source или транзакцию произвольных shell/agent effects. Поэтому наличие законченного wizard не означает, что его state model покрывает все требования next.

## 1. Точный путь выбора: CLI и GUI сходятся

### Декларация и представление

`vibe-app::describe(&ResolvedPackage)` (`src/describe.rs`) строит generated `ApplicationDescribe`, protocol `vibe-app-describe/1`. В документе есть package/application identity, все installer options и platform/arch, result/portability, launchers/public commands, features с parent/required/requires/exclusive group, defaultFeatures, profiles, typed parameters, license, steps, conditions, needs и requirements. Это данные для wizard, не набор вопросов, который GUI обязан извлекать из terminal output.

CLI `commands/app/describe.rs::show` сначала разрешает package обычным resolver, затем вызывает library describe. Следовательно, утверждение «describe ничего не разрешает» верно для `describe(ResolvedPackage)` как library projection; публичный CLI до этого разрешает package source. Оно не является абсолютной гарантией zero network/cache IO.

`vibe app installers ... --json` дополнительно отдаёт сгруппированную platform → architecture → installer картину с detected отметками (`select.rs::options_listing`, `vibe-application-install-options/1`). `vibe app steps` показывает author-declared steps; короткие installation methods уже нормализованы через `vibe_core::manifest::steps_of`.

### Консольный adapter

`select.rs::Choice` несёт `name`, stable `id`, `description`, `detected`, `depth`; library не кодирует цвета и terminal columns. `Chooser`, `FeatureChooser`, `ParameterPrompt`, `LicensePrompt` — injected callbacks.

`commands/app/global.rs::attended` проверяет не-JSON, не-unattended и наличие terminal. `operation` подключает callbacks только в таком режиме. `commands/app/wizard.rs` использует общие `output::prompts::choose_opt/choose_many`, общий prompt theme и `display::asking`, который освобождает step panel на время вопроса. Bool — Confirm, choice — список, secret — Password, остальные типы — Input. Отрисовка отделена от правил разрешения выбора.

`select_option` выбирает platform, затем arch, затем installer. Detected значения выводятся первыми. Явные значения минуют вопрос. Без chooser неизвестная platform/arch и неоднозначный installer дают typed SelectionRequired; произвольный первый installer не выбирается. Несколько installers на detected target требуют `--installer` в unattended режиме.

### Внешний GUI / script

GUI получает describe и передаёт стабильные значения теми же flags, которые используются unattended CLI:

- `--platform`, `--arch`, `--installer`;
- `--profile`, повторяемые `--feature`/`--no-feature`;
- повторяемый `--set ID=VALUE`, `--accept-license`;
- для исполнения `--resume` или `--from-step ID`, `--keep-work`.

`AppChoiceArgs` и `AppStepArgs` (`cli/app.rs`) общие для plan/install/update. `global::plan_request` превращает их в `InstallChoice`, `FeatureRequest`, `InstallRequest`; `global::request` добавляет Start/force/library. `vibe app plan` и `install --plan-only` вызывают тот же public `vibe_app::plan`, которым пользуется mutating operation. GUI не должен писать index или синтезировать внутренний host grant.

Здесь нет обнаруженного отдельного MCP app installer adapter: поиск `vibe_app` в current `crates/vibe-mcp` не дал callers. Подтверждённые внешние surfaces — Rust library и process CLI с generated JSON/progress. «GUI-ready» означает конкретный поддержанный protocol для внешнего клиента, а не наличие готового GUI в VibeVM.

### Один planner и сохранение результата

Оба пути приходят в `operations::plan_option` → `steps::plan`. Planner получает одну `PlanInput`: resolved package/option, request, recorded Previous, host facts callbacks, offline/attended, prompts, agent selection и requirement resolver. Он разрешает license, features, parameters, conditions, needs и выдаёт `Plan`.

`Plan::to_wire` (`steps/plan_view.rs`) формирует `vibe-app-plan/1`: selected features, конкретную implied rule/by для автоматического выбора, profile, redacted parameters, accepted license, download sizes, required applications, run/skip и причины. Текстовый tree — отдельный CLI renderer; JSON остаётся стабильной flat list, зависимости имеют structured rows. Plan используется как review view; отдельный serialized plan не является executable grant.

`operations/apply.rs::apply` строит `Installation`: option/platform/arch, **resolved feature set**, несекретные parameter values, license name/time, step outcomes/uninstall data и physical location. `operations::commit` через `Index::save` сохраняет record. Profile id остаётся в plan/run record, но отдельного поля profile в installed Installation нет: persisted meaning — resolved features. Нет сохранения arbitrary process-local question/approval state.

`operations/update.rs` берёт Previous features/parameters/license и recorded option, если новый выбор явно не задан. `operations/modify.rs` планирует новые значения с recorded state; без flags предлагает recorded features заново, с частичными feature flags сохраняет остальные recorded choices, а profile заменяет набор. Parameter flags имеют приоритет; оставшиеся значения берутся из record. Secret values не сохраняются, поэтому их нужно дать снова. Это реальная persistence seam для пользовательских выборов, которой не является один in-memory pending plan next.

## 2. Семантика features и условий

`steps/features.rs::resolve_features` — public deterministic resolver над InstallerOptionDecl. База: checkbox selection, затем named profile, затем recorded set при пустом request, иначе defaults. Explicit selection включает ancestors/requires; deselection удаляет descendants. Required root/child нельзя отменить. Для manifest-valid exclusive group core `manifest/package/application/wizard_validation.rs` (138–151) требует ровно одного `default=true`. `Tree::close` считает группу active, когда хотя бы один member без parent либо его parent выбран; active empty group получает этот default, postcheck отказывает при >1. Поэтому ровно один гарантируется для active группы валидной декларации; inactive группа может быть пустой (это проверяет rule-table case deselecting gui). Public `resolve_features` сам не проверяет unique default, поэтому нельзя приписывать ему самостоятельную exactly-one гарантию на вручную сконструированной невалидной InstallerOptionDecl. Explicit choice заменяет остальных, конфликт двух explicit members отказывает. Ответ сохраняет declaration order и объяснение ImpliedRule (`Parent`, `Requires`, `Required`, `ExclusiveDefault`). Profile default определяет порядок предложений attended wizard; unattended без явного profile использует feature defaults, не автоматически первый profile.

`params::resolve_parameters` валидирует string/bool/integer/choice/path; integer bounds, required, allowed choices и path existence решаются в plan. Same `ParameterDecl::canonical` применяется к flags и prompt results. Неизвестный id даёт ParameterInvalid. Secret value отсутствует в plan wire и installed record; children получают его в env. Это не redaction произвольных child logs/files: fixture сам пишет окружение в output, поэтому обещание «секрет нигде никогда не попадёт на диск» шире кода.

`steps/plan.rs::holds` использует закрытые Condition variants: feature, param, param_equals, command_present, file_present, env_present, agent_configured, offline. Нет eval language. `when` требует все, `unless` отвергает при любом совпадении. Conditions вычисляются при построении текущего плана; step outputs не меняют уже вычисленные решения поздних steps. Host command/env/path facts injected; file existence читается planner’ом.

`needs`: running step проверяет command/network/agent/terminal; optional unmet step становится skip, обязательный отказывает до своего execution. Missing optional agent — одно предупреждение и skip, required agent — refusal. Plan owned result без running prepare producer отказывается. Это явный конечный словарь проверок, не общее доказательство выполнимости shell/agent code.

## 3. Владельцы и effect call chain

Основной library путь:

`install(ctx,resolved,request)` → `SettingsRoot::lock` → `install_entered` → option/admit/previous availability → `plan_option` → `deps::install_required` → `apply::apply` → `Run::new`/`admit_agents` → prepare phase → owned placement или external root check → finish phase → построение Installation → `commit` → report.

- `admit` перед installer проверяет command collisions и proxy image. `Run::new` проверяет resume и forced/update variants. Agent consent проходит в `Run::admit_agents` перед steps этого package.
- Dependencies уже устанавливаются **до** parent `Run::new/admit_agents`. Общий plan не является атомарным preflight всего дерева effects: dependency installs могут остаться, если затем parent resume/force/agent admission откажет. Source order установлен; такую комбинацию здесь не запускал.
- Closed StepKind dispatcher `steps/kinds.rs::run` вызывает существующие installer modules: sources, archive, recipe, shell, native, copy; env и agent обрабатываются отдельными ветвями runner. Methods sugar не создаёт второй engine.
- Все prepare steps проходят до finish steps. Внутри каждой phase сохраняется порядок декларации. Owned `$out` должен быть непустым, затем переносится через staging в library. Shell/native external root проверяется как real directory; его Vibe не считает recursively owned.
- Обычная prepare failure сохраняет work и не commit’ит parent Installation; уже состоявшиеся external side effects не откатываются.
- Authored finish failure возвращается как `finish_failure`: payload остаётся, операция всё равно commit’ит Installation и затем возвращает ошибку; uninstall доступен. Special legacy pre/post hook semantics сохранены: post_install error может вернуть прежний owned payload вместо commit. Важна ветка StepOrigin, нельзя обобщать все finish failures одинаково.
- `commit` пишет index, затем user-layer seeding, удаляет retired directories и синхронизирует proxies. Это не multi-file transaction; ошибка после index publication не означает отсутствие установки.

## 4. Ownership, state и interruption

Index (`applications/index.json`, protocol `vibe-user-applications/3`) — inventory owner. Owned location = library id + relative directory; external = absolute app_root. Storage marker проверяется при использовании; missing library означает unavailable. Owned payload trusted после установки, без постоянного повторного hashing. Только owned directory удаляется рекурсивно; external removal выполняет записанные authored uninstall commands. Рядом существуют catalog arrangement/usage/revision/running records, но они не заменяют inventory.

`SettingsRoot::lock` — OS file lock `applications/.mutation.lock`, held на mutating command, с ограниченным 50ms retry. `write_atomic` пишет sibling, sync_all файла и rename. Нет namespace expiry, idempotency-key reservation, exact-plan receipt и automatic rollback engine. Work/run records — состояние последовательного installer и diagnostics. Project home имеет собственные index/work/runs/proxies; source download cache делится с user home (`app_work_dir` использует `root`, не общий `cache`).

`steps.json` хранит package coordinate, option, chosen features и достигнутые steps: done/skipped/failed/delegated/interrupted, времена, причины, uninstall/env/agent/input provenance. `settle` записывает его **после** step effect. Crash между effect и записью не обеспечивает exactly-once; resume способен повторить этот step.

`resume::start_point` сверяет package, option, features. Он не сравнивает source/plan hashes, parameters, полный definition body или toolchain. Текущий plan вычисляется заново. Resume выбирает первый step не Done; полностью settled Done/Skipped даёт nothing_to_resume. `--from-step` начинает заново с указанного id. Steps начиная с restart point очищаются из текущего state и выполняются заново по новому plan. Поэтому resume — operator retry, а не replay старого идентичного авторизованного запроса. Kept verified archive/source bytes используются без повторного fetch, если checks подтверждают cache hit.

Hosting agent park сохраняет outbox/task и ждёт `result.json` на следующем explicit resume; nested agent не запускается. Finish delegation может commit’ить placed payload и сохранить work; finish-resume keeps prepare phase и использует уже placed owned root. Обычная finish failure по умолчанию очищает work после commit, поэтому её нельзя описывать как тот же prepare-resume сценарий.

`installer/child.rs` и CLI `Signals` ловят SIGINT/SIGTERM; captured non-terminal Unix child получает собственную process group и forward идёт группе, чтобы остановить shell вместе с compiler/sleep. При observed interrupt runner не помечает Done даже при exit 0. Windows adapter ловит Ctrl-C/Ctrl-Break; tests/claims native platform должны проверяться отдельно. В install child runner нет обнаруженного таймера принудительного kill/escalation для child, игнорирующего сигнал; он ждёт завершения. Не следует переносить 5-second stop semantics каталога running apps на installer child. Также source здесь не доказывает cleanup при SIGKILL/power loss или каждом закрытии controlling process.

Runs (`cache/app-runs/<key>/<timestamp>`) содержат plan.json, run.json, steps.json и per-step output.log. Mirrored run state/close — best effort, тогда как save work steps.json возвращает ошибку. Наличие run record не является durable effect witness next. Prune/work clean отделены от download cache.

## 5. Реальное внешнее наблюдение и tests

GUI может читать final structured stdout и live `--progress-format json` stderr. `vibe-progress-event/1` содержит task/parent, kind, seq, measurements/detail/finish/fail/skip. Dependency subtree получает structured `requires` (`key`, `scope`, `step`); клиенту не нужно разбирать label. `--json` отключает attended prompts. Plan/describe/errors/runs/work/list — разные generated documents. Важные schemas: `application_describe.jtd.json`, `application_plan.jtd.json`, `application_report.jtd.json`, `application/e1/steps.jtd.json`, `application/e1/run.jtd.json`, `application/e2/index.jtd.json`, `application_errors.jtd.json`.

Самый показательный source test: **`crates/vibe-cli/tests/cli_app_wizard.rs::a_tool_drives_describe_plan_install_runs_cancel_and_errors`**. Он через публичный CLI:

1. Читает tree/relations/profiles/parameters/license/icon из describe.
2. Передаёт `--profile typical --feature qt --set port=9090 --set lang=ru --set token=...`, проверяет implied extras и redaction.
3. Проверяет unknown_feature, parameter_invalid, license_required, feature_conflict с exit2 и нулём обращений к fixture payload server.
4. Устанавливает с JSON progress, проверяет structured measurements/seq и record features/nonsecret parameters/license.
5. Читает runs/logs; посылает SIGTERM реальному installer process, проверяет child death, interrupted state и отсутствие parent inventory commit.
6. Запускает --resume и проверяет установленный payload и cleanup; сверяет stable error codes.

Другие конкретные проверки в source (в этом аудите не запускались):

- `select.rs` tests: detected-first console order, explicit selection for nonstandard platform, unattended installer ambiguity.
- `steps/features/tests.rs::the_rule_table`, `refusals_name_their_reason`; `steps/tests.rs::features_come_from_flags_then_the_question_then_defaults_and_update_reuses_them`; conditions и needs tests.
- `cli_app_steps.rs::a_step_list_plans_installs_fails_resumes_and_restarts`, `an_unmet_command_need_is_refused_before_any_download`, `external_steps_record_uninstalls_that_run_in_reverse`, kept-work/from-step/clean journey.
- `cli_app_modify.rs::modify_changes_features_and_installer_in_place`, `update_keeps_the_previous_version_hidden_and_rollback_swaps_them`.
- `cli_app_agents.rs`: no-agent skip, required refusal, consent before steps, external registration/selection precedence, envelope/results, hosting-agent park/resume, parked finish resume.
- `cli_app_interrupt.rs::an_interrupt_stops_the_recipe_and_what_it_waits_on` — process-group regression.
- `cli_app_dependencies.rs`: hidden build dependency and autoremove, exposed runtime dependency and cascade protection, license, agent receives required PATH.
- `application_report_wire.rs` exact camelCase roundtrip; `application_wire_corpus.rs` generated index roundtrip/unknown-member refusal; `steps/state.rs` domain/wire/disk roundtrip.

Public command tests use isolated registries/settings, local HTTP and actual subprocesses. Their existence is materially stronger evidence than README adoption labels. It is still not a green runtime report for this revision/platform; no tests were run here. Console drawing tests and callback tests are not proof of every native terminal configuration.

## 6. Сравнение с next и общий выбор для всех package kinds

### Что уже доступно reuse без новой архитектуры

Для **app consumer** существующие public `describe`, `select_option`, `resolve_features`, `plan`, `install`, `update`, `modify`, Plan::to_wire и generated records можно вызывать как есть из другого Rust UI или process client. Shared console prompts и structured Choice представление уже работают; пользовательские choices реально сохраняются, а update/modify замыкают lifecycle. Не нужно создавать второй app wizard, синхронно дублирующий эти правила.

Для **других domains** это проверенный источник presentation и interaction mechanics, но `FeatureRequest`/`resolve_features` сейчас типизированы через InstallerOptionDecl и имеют app-specific defaults/exclusive-group semantics. Буквальное переиспользование функции без mapping не доказано. Report не проектирует extraction; он фиксирует существующие seams и требования сохранения behavior.

### Что есть в next сверх app checkbox rules

Next `ConfigurationChoiceHost` требует уже готовые `PreparedConfigurationDiscovery`, NormalizedRequest, InstallSource, install request и spec format; сохраняет `pending: Mutex<Option<...PlannedInstall>>`. Он умеет questions/evaluate/prepare/apply над этой основой, связывает defaults с shown capture и исходным request. Это не законченный bootstrap/options persistence UI, хотя lower domain apply действительно пишет lock. Main app path начинается от реального package resolution и заканчивается installation record; удобный существующий consumer chain нельзя подменить одним host constructor.

Next `change/full_pass.rs::ChangeFullPassPlan` прямо возвращает `execution_carriage="unavailable:m16-change-full-pass"` и unbound resolution/declaration/target. Main application runner действительно исполняет authored sequence и поддерживает observer/agent/resume; но installation step sequence не является семантикой change solving или доказательством закрытия требований. Нельзя автоматически считать её готовым full-pass engine.

Next feature contracts в `manifest/package/features/{effects,constraints}.rs`, `manifest/consumer_configuration.rs`, resolver configuration включают:

- четыре cardinalities: exactly-one, zero-or-one, one-or-more, zero-or-more;
- positive guards на groups/members и dormant selections;
- различие omitted и explicitly empty; exact и additive intent;
- consumer/dependency declaration/instance/domain/target-role ownership, несколько consumer edges и conflict joins;
- explicit use-declared-defaults permission, привязанное к показанной definition/capture;
- effects Local, ActivateDependency, strong/weak TargetFeature, ExportedSubskill, ConfigValue;
- persistent authored intent отдельно от resolved/locked result.

App features — local choice внутри конкретного installer option. Требование другого **app** на step не равно feature effect `dep:...`: step dependency появляется только у running step, build scope hidden и может удаляться после build, runtime scope exposed и защищает removal. Next graph configuration должна учитывать recursive package dependency resolution и несколько потребителей. App resolved feature list не сохраняет exact/additive/omitted/dormant authored intent. Unattended defaults app разрешены по текущему контракту; next RequireSelection запрещает такое подразумеваемое разрешение. Эти различия нельзя молча сгладить при общих console/GUI controls.

### Все девять kinds: одинаковый interaction path не означает одинаковые effects

Current main `PackageKind::ALL`: `flow`, `feat`, `stack`, `tool`, `mcp`, `lang`, `doc`, `app`, `remap`. Это девять типов, но не девять одинаковых installer journeys. `doc` имеет read/cache path, а обычный install отказывает; `remap` — пользовательски принимаемая policy, не обычная transitive executable dependency. Остальные сохраняют существующие materialization/boot/tool/MCP semantics. Общие describe/choice/plan surfaces должны оставлять эти различия наблюдаемыми. Не вводить app shell/install effects для `doc` или автоматическое принятие remap только ради унифицированного UI.

## 7. Предлагаемые acceptance scenarios для общей функции выбора

Это рекомендации для root review, не утверждение о принятом scope или выполненные тесты:

1. **Existing app parity:** текущий cli_app_wizard journey проходит без изменения flags, output schemas, defaults, license, secret omission, installed choices и resume semantics; console callbacks и explicit JSON-client answers приводят к одному Plan/Installation.
2. **Every-kind boundary:** одинаковый вход inspect/choice показывает все применимые options для каждого из девяти kinds; итоговые действия сохраняют собственный kind owner. Doc остаётся read/cache-only; remap требует принятия policy; app запускает installer только после его обычных checks.
3. **Four cardinalities:** radio/optional-radio/multi/required-multi controls дают результат текущего next solver; empty и omitted не сливаются; invalid count получает структурированную причину, без effects.
4. **Consumer conflicts:** два consumers одной instance с совместимыми additive choices сходятся; несовместимые exact selections дают conflict с обеими сторонами. GUI выбор для одного consumer не переписывает чужой intent.
5. **Dormant/guarded intent:** временно неактивный control не теряет explicit choice; смена guard показывает прежний choice или конкретный конфликт. App parent/required/exclusive semantics остаются прежними.
6. **Persistence/restart:** обычный process exit/restart и update сохраняют authored exact/additive/omitted intent в owning configuration store; installed/locked effective set проверяется отдельно. Нельзя принимать только сохранение app-style flat feature set за этот gate.
7. **Shown defaults/source drift:** явное app-default поведение сохраняется; на RequireSelection edge default используется только по его правилам. Изменение package definition между показом и apply вызывает предусмотренное переосмысление выбора, не скрытую подстановку другого default.
8. **Feature effects and dependency roles:** choice реально меняет activation, strong/weak dependency feature, exported subskill/config в domain result; app build/runtime requirement exposure/removal остаётся действующим и отдельно проверяется.
9. **External GUI complete path:** клиент начинает с реально доступного public describe, посылает стабильные IDs/values, получает plan/conflicts, выполняет apply, наблюдает progress/failure, читает persisted choice после restart. Тест не начинает с privately constructed admitted host в fixture.
10. **Failure truth by domain:** installer interruption остаётся retry from step с возможными prior effects; machine dedup domain сохраняет one-effect invariant; один общий UI не переименовывает оба режима в одинаковую «transaction rollback».

Эти сценарии позволяют оценить общую функцию выбора по пользовательскому результату и сохранённой семантике. Из найденного source не следует необходимость переносить весь next machine-authority stack или, наоборот, удалять его гарантии ради сходства с app wizard.
