# A1–A4: первый положительный package runtime

Статус: предложение для root review, исходники не изменены, Cargo/Git/network не запускались. Основание: принятый F15, текущий исходный код и решения root в PROP069 OPTIONAL-DECLARATIONS / OWNER-LOCAL-ACTIVATION / DORMANT-SOURCE-BOUNDARY. Первый результат — ordinary authored incoming input действительно меняет dependency graph, сохраняется в существующем lock и воспроизводится другим процессом. Это не завершение всего common package capability.

## 1. Предлагаемая граница первого связного изменения

Поддержать несколько **реальных фиксированных workspace consumers**, потребляющих один package instance, на обычном selected resolvo + local registry/copy пути. Его common selection активирует явно optional обычную зависимость, а та проходит существующие resolution/fetch/materialize/boot owners. Поддержать сохранение исходных exact/additive/omitted/dormant inputs и проверяемый cold replay, включая точечный reinstall из pinned store. Выключенный optional source не должен читаться даже capability prescan.

Первый положительный oracle использует две конфигурации одного flow: backend выбран / backend не выбран. Это реальная установка нужной зависимости, а не новый inspection-only endpoint. До принятия этой границы не удалять F15 guards глобально: unsupported solver cells, source modes, candidate-conditioned transitive consumers, активный forwarding и неподключённые effect consumers должны давать typed pre-effect refusal. Legacy пути сохраняются. Direct CLI root без реального declaration input не получает синтетического require-selection/default consent. Workspace собственный [configuration] остаётся публикуемым интерфейсом, не ответом для его requires.

Первый production source adapter предлагаю local registry/copy, где можно однозначно связать metadata candidate с фактически полученным manifest. Registry-backed Git, path/override/embedded и in-place остаются отдельными обязательными source-route gates, а не молчаливым fallback на этот adapter. Обычный legacy install для них не запрещается. Существующий PreparedInPlace F15 сохраняется: включить rich route там можно только после сравнения **его actual prospective manifest** с решённым candidate, без re-selection.

## 2. Точные изменения владельцев

| Владелец / файлы | Конкретная ответственность |
|---|---|
| core `manifest/package/wire.rs`, `wire/requires.rs`, `capabilities.rs`, metadata validation/finalization tests | `optional: Option<bool>` в inline wire; `Requires.optional: BTreeMap<String,bool>` по существующему normalized declaration key. Omission/false mandatory; explicit false сериализуется. Не расширять PackageRef. Registry/git/path/version.var retain metadata. |
| core новый `manifest/configuration/evaluate_inputs.rs` (название рабочее), существующие `intent.rs`, `lower.rs`, `configuration/choices` | Узкий pure multi-consumer adapter: normalize каждого оригинала, объединить ordinary feature seeds, вызвать **один** существующий ChoiceModel::evaluate, вернуть original inputs, core evaluation, automatic reliance и typed public parameter agreement. Не импортировать vibe-app и generated app record в core. |
| resolver новые `configuration.rs` + `configuration/{snapshot,evaluate}.rs`; `lib.rs`, `resolvo_engine/{mod,provider,capabilities}.rs` | Operation-local snapshot, candidate-local evaluation/projection и selected-result record. Существующий resolvo остаётся solver. Missing/invalid source — operational error; несовместимый explicit answer — candidate incompatibility; questions/stale basis не становятся solver permission выбрать другой ответ. |
| package-source `cells.rs`, `source.rs`, `qualified.rs` | Один construction point нового configured route; selected non-supporting cell refuses, без скрытой замены sat/naive на resolvo. Тот же source resource, что обычный install. |
| install `lib.rs`, `plan.rs`, `plan/roots.rs`, `plan/fetch.rs`, `fetched.rs`, `visibility_projection.rs`, `record.rs`, `apply/lock.rs`, `configuration_admission.rs` | Реальные consumer identities; один snapshot через held retry/visibility/recommend/conditional seams; actual fetched manifest comparison; rich result до apply; lock writer. F15 unsupported guards заменяются проверкой admitted result **только** на поддержанном пути. |
| core `manifest/lockfile.rs` + новый child module для records/validation; workspace `freshness.rs` | Strict lock8 meaning и faithful read7; Fresh сравнивает actual consumers/inputs и проверяет retained configured world. Не ограничиться version constraints. |
| workspace `install/bootgen/materialised_read.rs`, `install` ResolvedDep owner; orchestrator `install/{mod,provisional_world,resume,ready}` (точный child путь проверить при patch) | Один durable result вместе с actual locked world до regeneration. Не перечитывать unconditional requires как selected graph. Cold reader не придумывает input по features. |
| CLI `commands/reinstall.rs`, `reinstall/{force,inputs}.rs`; update `scoped.rs`, `scoped/{stage,apply}.rs`, `update/inputs.rs` | Point replay сохраняет rich record и проверяет pinned candidate. Пока scoped update не использует configured solver, early typed refusal **до stage/checkout/prune/write**, а не копирование old effective features. Whole update идёт через тот же install. |
| CLI новый `cli_common_package_configuration.rs`, installer/resolver/core focused tests | Первый реальный graph+cold/reinstall oracle; regression guards и declaration/read7/read8 tests. Никаких тестовых PreparedInPlace замен. |

Это perimeter для coherent activation, не поручение всем файлам немедленно меняться. Root сохраняет schema/spec/API review. После одобрения один writer сначала связывает существующий первый путь; независимые тестовые файлы можно отделить явно. Сейчас все source paths свободны, я их не менял.

## 3. Enduring carriers, без второго solver/host/store

Предлагаемые типы (точные Rust имена можно уточнить при реализации):

- `PackageConsumerId`: tagged `Workspace { node_relative, declaration }` или `Package { owner_coordinate, owner_candidate_fingerprint, declaration }`. Второй вариант уже нужен durable provenance и следующим transitive increment; это не authored instance label. Сам physical instance определяется текущим workspace lock owner + existing package coordinate, как сегодня один row на group/name.
- `PackageConsumerInput { id, requirement, original: Option<ConfigurationIntentDecl> }`. Option не означает автоматически согласиться на defaults. Старый потребитель без input остаётся видимым; required common answer возвращает needs-input.
- `PackageSolveRequest { roots, consumers, retained_configuration }`: roots сохраняют все реальные version constraints, включая pin optimization отдельно от исходных требований. Explicit request не стирает inline configuration того же edge; если невозможно однозначно связать запрос — typed refusal.
- `PackageCandidateSnapshot`: operation-local version enumeration/manifest observations, keyed by actual selected provider transaction + coordinate/version. Не новый durable catalog и не prefetch всей потенциальной closure. Мемоизация одинаковая при основной попытке, held-pin retry и masks. Исходный manifest не изменяется; derived dependency view отдельна.
- `PackageResolution { graph, selected_configurations, selected_declarations, candidate_observations }`. Ни rejected candidate, ни unresolved evaluation не появляется как executable result. Graph dependencies берутся из той же active-declaration projection, что решала constraints.

Минимальный additive `InstallSource` seam: `solve_configured(request, snapshot)` и `solve_configured_masked(request, snapshot, blocked)` возвращают этот result. Legacy `solve/solve_masked` остаются. Default implementation честно сообщает unsupported configured route. Source composition owns provider construction; resolver owns evaluation/solve. Metadata projection для configured result читает уже наблюдённый candidate/active view из result/snapshot, а не новый `source.manifest_of`. Сигнатуры могут быть объединены одним masks argument; важно отсутствие трёх расходящихся capture paths. Не вводить процесс/сокет/bootstrapped host.

При fetch/materialisation compare actual candidate manifest meaning (включая requires source declarations, optionality, visibility и child original input), не только `ConfigurationDecl::semantic_fingerprint()`. Последний не включает requires и недостаточен. Фактические content_hash/source/ref/commit поступают из existing fetch/PreparedInPlace, сохраняются существующим record owner. Metadata hash — drift check, не authentication и не замена content pin. Несовпадение до project effects → refusal/replan, не повторный solve после частичного apply.

## 4. Candidate projection и почему здесь не нужен второй solver

Для фиксированных real incoming consumers вычисление candidate A независимо от выбранных версий потомков: normalize inputs against A, core merge, затем `mandatory ∪ dependency effects ∪ strong-forward activations`. На первом шаге active strong/weak forwarding ещё требует explicit unsupported result, пока следующий consumer-encoding не подключён; dependency activation работает полностью. Обычный mandatory edge не становится conditional только потому, что effect его называет.

Candidate projection применяется **во всех** местах, читающих dependency semantics:

1. resolvo provider `get_dependencies` (`provider.rs:257+`),
2. selected output в `resolvo_engine/mod.rs` (сейчас опять вызывает raw `resolution_requires`),
3. `capabilities::prescan` (`capabilities.rs:43–84`, также raw `resolution_requires`, все версии),
4. recommends retries / provider cache,
5. visibility `project/resolve_effective` и masked solve,
6. install conditional expansion и fetched result/lock graph.

Prescan не должен даже перечислять inactive optional target. Дать ему ту же filtered candidate view. Если неподключённое сочетание capabilities/OR/conditional не позволяет сохранить правильные selected-edge reasons, отказ до mutation; не подменять selected edges всеми возможными вариантами. Local malformed references проверяются всегда, foreign feature existence — при реально consumed edge.

Не мутировать global input после найденного graph и повторять solve «пока похоже стабильно»: это теряет candidate conditioning. Следующий transitive/forwarding increment должен добавить incoming edge activation, связанную с **выбранным parent candidate/declaration**, внутри существующего resolvo encoding (synthetic internal constraints/literals с точной projection обратно в package graph допустимы, второй solver нет). Та же `PackageConsumerId::Package`, selected declarations и lock8 records принимают эти inputs; не нужен новый persistence owner. Weak contribution condition = активный declaration этого parent instance, а не наличие target в graph. При нескольких consumers config variants одного instance должны быть связаны с одним actual selected version и совместным core evaluation; нельзя независимо удовлетворить каждый exact input отдельной фиктивной установкой.

Реализация этого encoding — оставшаяся настоящая работа, **не обещание, что fixed-consumer wrapper её уже решает**. До неё selected transitive rich input/forwarding refuses with actual owner/edge before effects. Dormant locally valid forwarding не требует foreign lookup.

## 5. Defaults, parameters, app reuse

Shared kernel уже реализует choice cardinality/guards/exact/additive/dormant semantics; повторять его в package adapter нельзя. `normalize_input` даёт per-consumer ordinary FeatureInput и choices. Ordinary seeds объединяются до одного closure call, но origins/default reliance остаются per consumer; union не превращает согласие одного consumer в согласие другого.

App evaluator сейчас действительно single-installer: hardcoded `"installer"`, app HostFacts/ParamValue, data-only effect restrictions. Не переносить его целиком в resolver и не создавать зависимость vibe-install→vibe-app. Нейтральные normalization/typed parameter/default-basis helpers можно вынести в core с двумя настоящими callers. Первая explicit no-parameter fixture не доказательство всех параметров. До host/path/ephemeral adapter public paths и secret-consuming configuration должны иметь отдельный typed unsupported/needs-input результат, а не пропадать из record. Pure scalar canonical agreement может быть подключён сразу через существующий ParameterDecl validator, с named conflicts. Не сохранять secret values/answer hashes.

Fresh authored explicit input может привязаться к actual candidate. Автоматическая reliance имеет historical semantic basis; cold reuse не обновляет его автоматически. Для первой свежей solve-попытки предлагаю bind automatic meaning к deterministically selected highest version satisfying all real root constraints **до** effect-induced backtracking; альтернативный candidate с другим automatic meaning даёт rediscovery, а не новый default. Fully explicit/no-auto answers могут backtrack между совместимыми definitions. Это конкретный policy choice для root review; первый oracle ниже намеренно explicit `default_features=false`, поэтому не скрывает эту развилку успехом fixture. Update с изменившимся default не получает согласие от `-y`.

## 6. Lock8: явная семантика, не «regenerate и потерять»

Текущий `Lockfile` и package rows strict deny_unknown_fields; read принимает только7 (`lockfile.rs:517–524`). Просто добавив rich keys в7, мы получим документ с тем же номером и иным обязательным смыслом, который старый7 reader отвергнет либо иной consumer неправильно пересоберёт. Нужен8.

Предложение: new reader явно принимает strict legacy7 и strict8. Legacy7 не получает fabricated original/default receipt. Writer сохраняет7 для losslessly legacy-only world;8 обязателен при наличии richer accepted state. Чтение не переписывает файл. Нельзя downgrade8→7, пока остаётся хотя бы один rich input/basis/edge result; actual authored removal + successful replan может убрать больше не существующий state, это не read migration. Old binary7 отвергает8: это ожидаемая несовместимость, не universal backward compatibility. Уже accepted8 никогда не восстанавливается через «discard lock and assume defaults».

В существующем lock owner добавить typed `configuration` world с:
- исходными consumers (owner/declaration + exact original intent; отсутствие child answer отдельно от explicit omission),
- configured package rows: exact candidate/content association, typed historical definition + semantic/declaration basis, per-consumer automatic reliance, resolved core selection/reasons, typed public parameter origins/values где поддержаны,
- selected declaration edges: exact parent candidate, declaration key, selected target, activation causes/forward contributions, existing source/link/visibility meanings по ссылке на сохранённую декларацию,
- selected config-value data и subskill selections **только когда actual consumers подключены**; иначе selected effect refuses, не silently empty output.

Не второй app ConfigurationRecord с installer identity. Core typed record reuse исходных ConfigurationDecl/Intent и core meanings; не arbitrary JSON blob. `LockedPackage.dependencies` остаётся exact selected graph projection и проверяется на согласованность с edge records. Legacy `features` остаётся совместимой effective projection, не source intent. `subskills_active` не возрождается как инструкция копировать файлы.

Структурное read8 admission: уникальные consumers/instances/edges; endpoints есть; candidate identities согласованы; no resolved result при questions/conflicts; selected dependencies совпадают; no dangling rich row. Семантический replay сверяет retained definition/declaration meaning с pinned actual manifest и повторяет pure evaluation. Не доверять одному сохранённому resolved feature set.

### Cold/Fresh/maintenance closure — co-land с writer

- `freshness::check`: before Fresh сравнить exact current workspace consumers/configuration/optional metadata с retained baseline. Изменение only intent при том же version string обязательно Stale. Legacy7 + newly authored rich input => Stale, не «old features are original».
- `plan.rs` Fresh branch: validate configured locked world/actual slots даже без migration_pending. Orchestrator provisional_world и boot durable reader получают тот же admitted record. Внепроектный registry можно убрать: unchanged pinned world воспроизводится без source query.
- Point `reinstall`: existing exact content pin + configured replay validator **до materialise**; retain lock record без renegotiation/default renewal. Это второй фактический процесс первого oracle, а не обещание later reader.
- Force reinstall и orchestrator resume: не терять запись при clone/rebuild LockedPackage; если scope ещё не поддержан, pre-effect refusal. Scoped update до подключения configured solve retains F15 refusal, не copies old effective features into new candidate. Whole update uses same configured planner.
- Boot/native check/analyze `materialised_read` validates record against named slots and consumes selected graph; no enumeration of orphan slots. Missing/corrupt8 remains error and сохраняется; не fallback to no lock.
- Audit every `if let Ok(Lockfile::read)`/read-error fallback (например fetched.rs, orchestrator mod.rs) before writer: malformed/unknown future rich state must not be mistaken for absent legacy lock. Это required writer/read closure, не full unrelated corruption rewrite.

## 7. Первый end-to-end oracle (реальный CLI, отдельные процессы)

Local registry outside project; three ordinary flow packages: `suite`, mandatory `base`, optional `backend`; backend boot has unique marker and optional hook sentinel. Suite source bytes also contain unselected content; installation must retain them.

Workspace authored input:

```toml
[requires.packages."org.test/suite"]
version = "=1.0.0"
configuration = { policy = "require-selection", default_features = false, selections = { mode = { intent = "exact", members = ["with-backend"] } } }
```

Suite:

```toml
[requires.packages]
"org.test/base" = "=1.0.0"
"org.test/backend" = { version = "=1.0.0", optional = true }
[[configuration.features]]
id = "with-backend"
effects = [{ kind = "dependency", target = "org.test/backend" }]
[[configuration.features]]
id = "plain"
[[configuration.groups]]
id = "mode"
mode = "choice"
members = ["plain", "with-backend"]
cardinality = "exactly-one"
```

Actual gate: ordinary `vibe install --registry <local>` installs suite/base/backend; lock8 contains original exact input and suite→backend declaration reason; backend marker appears in generated boot. No runtime direct constructor/fake configuration host.

Then rename/unavailable registry; launch fresh process bare `vibe install --offline`, assert success/selected graph and original input unchanged (ignoring only established volatile provenance). Launch actual point `vibe reinstall org.test/suite` from pinned local store; bytes/selection/history retained. Do not claim cold replay from two method calls sharing one snapshot.

Control edit ONLY members to `["plain"]`, restore source, run new install: backend not in selected graph/lock/boot; if another actual workspace node requires backend, it remains but suite→backend does not. Separate initial plain case with nonexistent backend source proves no dormant fetch/prescan requirement. Hook counts / unavailable target prevent a harmless-field-only pass.

Discriminators before acceptance of first bounded runtime: two actual consumers exact conflict leaves manifest/lock/boot/slots unchanged; explicit false mandatory + disabled effect retains dependency; changed intent with unchanged version cannot Fresh; read7 control survives; malformed8 cannot regenerate silently; actual fetched manifest differs from candidate => refusal before apply; selected unsupported forwarding/transitive rich/backend/source mode names scope and makes no project mutation. Candidate backtracking test: newest suite has selected optional requirement unsatisfiable, older compatible suite is selected under same explicit no-auto input, and no rejected graph/effects remain.

## 8. Что в prototype разумно и что требуется достроить

Проверен именно frozen file `/tmp/nst-common-package-next/prototype/crates/vibe-resolver/src/resolvo_engine/configuration.rs`: SHA256 b13c7516929498a3e0d314b4e3648865b89301ca757d91eb71b5b1e294ea6ab7 совпадает с `/tmp/nst-package-optional-contract.json`, pinned7fe6376. Dirty prototype не читался; Git не запускался.

Разумно: explicit optional declaration отдельно от effects; candidate-local expansion, selected-only projection, required/default receipt distinction; ordinary authored entrypoint существует, его нельзя называть исключительно host-only. Но fixed-consumer solver **прямо отказывает** в TargetFeature (`configuration.rs:395–403`: “needs explicit candidate-conditioned consumer edges”), и в nonregistry declaration (`404–413`). `required_source_manifest:66–84` убирает optional seeds, selected expansion `629–650` добавляет active deps. Это реальный частичный путь, не готовая cross-package configuration система.

Причина прежнего завышения: algebra tests/typed expansion и исторические accepted task labels были приняты за consumption closure. Main тоже содержит expansion fields без всех реальных consumers. Исправление — проверять не наличие structs, а graph+boot+lock+cold maintenance и scope-specific refusals. Не копировать универсальные catalog/capture-host/instance layers только ради внешнего сходства.

Остаются обязательными после первого runtime: transitive candidate-conditioned consumers и strong/weak forwarding, все advertised source/solver routes, effective subskill loading из intact source (eager/lazy-push/lazy-pull), inspectable config-data/diff consumer, host/path/secret package adapters, interactive/external question→answer→plan flow, rich scoped update/reconfiguration и broad failure/maintenance tests. A1–A4 начинаются одним полезным сквозным install, но не закрываются этим одним oracle.

## Root decisions requested

1. Подтвердить first bounded activation: fixed real workspace consumers + resolvo/local registry/copy + real optional graph + lock8/cold/point replay; unsupported richer routes remain typed refusals, а их obligations перечислены выше.
2. Подтвердить strict7+8 read, lossless7 writer for legacy-only,8 for rich state, no read migration / no discard-rich-lock regeneration.
3. Подтвердить fresh automatic basis selection before effect-driven backtracking либо указать иной exact rule. First explicit oracle не зависит от решения, но default support не должен угадывать его.

После root review: записать source slice + first actual CLI RED, затем coherent implementation, focused tests и этот public gate; не раздувать ещё одну design-only фазу. Команды и хэши будущих проверок относятся к actual implementation, сейчас runtime результата нет.

## Root review amendment / implementation start

Root approved first fixed-consumer runtime and7/8 compatibility. Highest-semver automatic consent proposal is NOT approved: first path must refuse unsupported automatic-basis usage until exact ruling, without effects. Do not add unused future Package consumer enum or forwarding DTO solely for a later increment; add real provenance when consumed. Independent `/tmp/nst-positive-runtime-oracle.md` supersedes simplified fixture above (required feature names and static-transitive link); worker prepares source-only CLI fixture, primary will integrate it. Existing resolvo SolvableId→manifest cache is reused for original+derived evaluation; output must never re-fetch/re-evaluate independently. Operation snapshot survives recommends trials, whose discarded candidate outcomes must not poison accepted graph. Initial source patch now adds optional wire/metadata roundtrip only; this is in-progress foundation, not runtime completion.
