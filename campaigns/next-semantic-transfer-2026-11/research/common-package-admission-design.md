# Общий admission перед package maintenance — read-only design input

Это source review, не runtime RED и не реализация. Сборок, Git-команд, source edits и новых workers не было. Root сообщил accepted MAIN34866ac87; фактические прочитанные исходники привязаны SHA256 в `/tmp/nst-common-admission-design-sources.json`. Новые CLI RED выполняет отдельный владелец `cli_common_package_maintenance.rs`.

Рекомендация: один небольшой `vibe_install::configuration_admission` с двумя чистыми проверками; применять их на действительных границах workspace-input и потребляемого package manifest. Для in-place требуется отдельное решение о точном prospective source: старый slot и metadata query по версии сами по себе не доказывают, что именно эти bytes окажутся в checkout. Этот guard закрывает silent-ignore, но не закрывает configured solver/effects/lock/UI journey.

## Минимальная общая поверхность

```rust
pub fn workspace_inputs(
    workspace: &Workspace,
    selected_root: &Path,
    raw_selected: &Manifest,
) -> vibe_install::Result<()>;

pub fn package_manifest(
    package: &PackageRef,
    manifest: &Manifest,
) -> vibe_install::Result<()>;
```

Экспортировать модуль из `crates/vibe-install/src/lib.rs`. Имена routine; существенны разделение смыслов и одна реализация проверки.

`workspace_inputs` переносит существующее правило `plan/roots.rs::reject_configuration_inputs`: все `workspace.iter_nodes()` плюс raw selected snapshot, чтобы не потерять unexpanded/source intent. Реальный selected node определяется `workspace.node_rel_of(selected_root)`; отсутствие возвращает существующий `SelectedConsumerMissing`. Для каждого реального requires declaration, содержащего `requires.configurations`, возвращается существующий `ConfigurationInputUnsupported { consumer: "<rel>/vibe.toml", dependency }`. Не сериализовать значения параметров. Сохранить детерминированный порядок выбора первой ошибки и нынешнюю raw/finalized boundary; не перечитывать selected file.

`package_manifest` заменяет два одинаковых блока в `plan/fetch.rs`: сначала `manifest.configuration.is_some()` → `ConfigurationUnsupported { package: "group/name" }`, затем первая `manifest.requires.configurations` → `ConfigurationInputUnsupported { consumer: "package group/name", dependency }`. Сохранить эту precedence и typed error chain. Этот слой не нужен для синтаксического объявления невалидности: manifest может быть совершенно корректным, операция пока не умеет его оценивать.

Важное различие: `[configuration]` у самого workspace package — объявление его интерфейса, а не автоматически вход install его зависимостей. Поэтому workspace guard проверяет requires-edge inputs, не запрещает всякий workspace с собственным definition. Definition проверяется, когда этот package действительно выбран как потребляемый candidate/slot. Не сканировать `installer_options[*].configuration`: выбранный app-installer — отдельный уже работающий owner. Не менять read/cache/consent-only boundaries doc/remap и не ставить глобальный guard внутрь Manifest::read.

Новый evaluator, solver, store, label instance/domain или manifest grammar здесь не нужны.

## Наблюдаемый поток и точные места

| Путь | Наблюдаемое сейчас | Необходимое место общего guard |
| --- | --- | --- |
| Обычный prepared install | `plan.rs::plan_prepared_with_spec_format` вызывает private roots guard перед migration, Fresh и solve | Заменить вызов shared workspace helper, сохранив ранний порядок. Public prepared-library callers тоже защищены. |
| Install surface source mutation | `vibe-orchestrator/src/install/mod.rs:254` вызывает `manifest_mutation.apply` раньше planner. CLI `CliGitSourceMutation::apply` → `apply_git_source_flag` реально пишет vibe.toml | Workspace admission после prove/selected lease check и до mutation callback. Проверка только внутри plan не обеспечивает no-write для этого маршрута. После callback planner всё равно проверяет новый in-memory world. |
| Scoped update | `update/scoped.rs::run` получает один proven raw manifest/workspace, затем читает lock/config, строит resolver и `solve` | Shared workspace guard сразу после `selection.prove()`/разбора snapshots, до resolver construction/solve. Нельзя проверять только named roots: sibling consumers остаются потребителями одного workspace/lock. |
| Обычный fetched root/transitive | `plan/fetch.rs::fetch_node` уже проверяет actual cached manifest до legacy feature expansion/embedded-source locking | Заменить дублирование `package_manifest`; сохранить guard до `expand_features`. |
| Deferred install in-place | `try_in_place_incremental` проверяет только существующий slot, хотя node.version уже новый | Сохранить current-slot validation, но добавить admission точного prospective candidate до возврата deferred plan. Новый выбранный manifest нельзя подменить старым slot manifest. |
| Scoped copy/hardlink candidate | `scoped.rs` fetches every graph node; common guards отсутствуют | Проверять каждый actual `cached.manifest` сразу после fetch и до `lock_embedded_sources`, `updated.push`, confirmation/stage. Весь selected closure должен пройти admission прежде первой project mutation. |
| Scoped in-place candidate | В `scoped.rs` pending row получает только coordinate/version/dependencies; `stage.rs:109` сразу вызывает materialise_in_place | Prospective admission всех pending rows в регионе1, не per-row после первой успешной mutation. Stage потребляет ту же admitted prospective identity. |
| Scoped full world | `stage.rs` лишь после in-place mutations вызывает `provisional_world`; он читает unchanged locked manifests, которые потом участвуют в whole boot | Подготовить и проверить реально потребляемую complete projection до stage, подставляя prospective replacements. Иначе unsupported unchanged provider обнаружится уже после mutation. Не проверять старый replaced slot вместо выбранного replacement. |
| Scoped apply | `scoped/apply.rs` начинает с prune, затем materialization/hooks, boot, lock | Admission должно быть завершено до входа. Защитная проверка перед prune у apply полезна для programmatic callers, но не чинит уже прошедший in-place stage. |
| Обычный apply | `apply_metadata_checked` публикует selected manifest, затем rediscover siblings, затем `apply/deferred.rs` меняет checkout | Проверять retained planned world до публикации; authoritative pre-effect sibling discovery/admission должен происходить до первой manifest write. Повторная проверка после публикации — defense/rollback, не доказательство untouched bytes. |

Lease acquisition, source/cache metadata acquisition, run identity/trace bookkeeping отличаются от semantic project effects. Не обещать «zero I/O»: oracle должен отдельно сравнивать selected/workspace manifests, lock, checkout HEAD/tree, materialized slots, boot files и hook sentinel.

## Prospective in-place: доступный seam и его предел

Уже есть `InstallSource::manifest_of(&PackageRef) -> Result<Manifest, SolveError>` (`lib.rs:165`). Он вызывает `metadata_manifest_cell` → существующий provider, а `vibe_install::metadata_manifest` резолвит версию и читает manifest. Его можно использовать для раннего selected-version preflight с exact `PackageRef`; новый solver/metadata service не нужен. Lookup failure обязан оставаться отказом до stage, без fallback на старый slot.

Но текущий контракт возвращает только Manifest, без принятого source/ref/commit. Это **не** конечное доказательство actual checkout:

- `InstallResolver::materialise_in_place` заново делает Multi `resolve_for(..., CheckoutSource)`.
- `MultiRegistryResolver::materialise_in_place` выбирает registry/backend; special sources имеют другую поддержку.
- Metadata provider walk имеет свои store/path/git/redirect branches; в комментарии override/registry ветви прямо сказано, что install override обрабатывается другой веткой.
- `GitPerPackageRegistry::materialise_in_place` вызывает `bootstrap_chain_into` и `clean_worktree`, затем читает manifest. В refresh branch `bring_clone_to_ref` вызывает `backend.update` из существующего clone origin. Mirror fallback может выбрать другой источник. Commit/manifest выдаются только после working-tree mutation.

Следовательно, добавить только `source.manifest_of(exact)` перед старым `materialise_in_place` — полезный preflight для фиксированных fixtures, но недостаточно для безусловного «actual unsupported candidate never mutates checkout». Проверка `placed.manifest` после вызова принципиально поздняя.

Предлагаемый честный minimum для этой границы: дополнить **существующий source/materialization owner** ephemeral prepare/apply seam. Prepare возвращает actual candidate manifest и immutable source/ref/commit identity, которую последующий in-place update обязан потребить без нового выбора источника/ветки. Собрать и admit весь набор до confirmation/stage; после этого stage применяет именно prepared identity. Fetch/cache/object preparation допустимы вне project tree. Fresh clone, existing-origin refresh и mirror fallback должны пройти ту же source-bound preparation. Fallback после admission либо использует уже доказанные те же bytes/commit, либо возвращает ошибку без checkout; он не может молча заменить candidate.

Это небольшой ephemeral source value, не durable receipt database и не новый configuration runtime. Точные Rust type/method signatures принадлежат source owner и требуют root ruling: текущий `InstallSource` не может выразить связку accepted manifest→actual checkout. Neutral registry layer не должен зависеть от `vibe-install::Error`; общий configuration guard остаётся в install layer, а source preparation/commit errors остаются Registry/Solve errors. Нельзя stringify typed configuration errors ради callback или объявлять неизвестную metadata заведомо configured. Если такую точную prospective identity невозможно получить данным source, отказать до mutation с честной source/admission причиной. Не использовать blind blanket refusal всех legacy in-place updates как финальную совместимость.

При этом проверка actual returned placed manifest всё ещё полезна как invariant: она обнаруживает нарушение source contract, но уже не заменяет pre-effect gate. Старые транзакционные progress/rollback владельцы сохраняются; не обещать rollback рабочего checkout там, где его нет.

## Другие реальные maintenance обходы — root должен явно решить perimeter

Это не просьба прямо сейчас переписывать всё, но нельзя называть guard «all maintenance covered», оставив эти пути без решения:

1. `reinstall/force.rs::run` напрямую fetches every lock row и строит resolution; не вызывает normal plan. Shared workspace check нужен до force branch/confirmation; package check — по каждому fetched manifest до lifecycle/materialization. Pin validation остаётся отдельным invariant. 
2. `reinstall.rs` point re-derivation directly resolves/fetches exact locked source, then `vibedeps::materialise_with_spec_format`. Candidate admission требуется перед этой записью, если операция реально потребляет unsupported package definitions/inputs.
3. Ordinary Fresh and non-force reinstall regenerate boot from installed slots, не обязаны fetch новые registry manifests. Не нужно запрещать невыбранную новую версию в registry. Но реально читаемый installed provider manifest, изменённый на common declaration/input, не должен пройти ложную legacy projection. Проверка должна читать тот же admitted full-world snapshot, который boot consumer затем использует.
4. `PlannedInstall` is a public/programmatic boundary. Guard only at CLI cannot protect injected plans. Before lifecycle construction/prepare_manifest/publication, validate supplied workspace and consumed manifests. Late rediscovery must not introduce unadmitted sibling input; preferably prepare the post-manifest tree in memory before publication, using the existing `discover_with_selected_manifest` owner.

## Дискриминаторы / control matrix

| Case | Expected before-effect outcome | Control |
| --- | --- | --- |
| Rich input only in sibling requires; scoped request names another node | ConfigurationInputUnsupported names actual sibling/edge; resolver callback not called; project hashes unchanged | Same sibling with ordinary legacy requires still updates |
| Selected raw source intent survives version.var/finalization | Same typed refusal before migration/--git rewrite/Fresh/solve | Ordinary var/source declarations retain old normalization |
| New selected cached candidate has definition | ConfigurationUnsupported before confirmation/stage/prune/hooks/lock/boot | Legacy v2 updates and hook runs normally |
| New candidate has no definition but has rich child requires intent | ConfigurationInputUnsupported names package consumer and child; no partial install | Legacy transitives remain supported |
| Rich transitive later in fetched closure | Entire closure refused before first project mutation | No root-only admission shortcut |
| Current in-place slot legacy, target ref common | Refuse based on target before HEAD/tree/clean/gitignore write | Legacy→legacy incremental update works |
| Two in-place candidates; first legacy, later common | No first-checkout advance; all prospective admissions precede stage | Legacy multi-update preserves measured progress on actual later I/O failure |
| Actual refresh origin/mirror differs from metadata provider | Source-bound prepared identity or honest pre-mutation refusal | No success based only on matching coordinate/version |
| Actual source metadata unavailable | Source/Solve failure before mutation, no old-slot substitute | Metadata/cache reads may occur and are reported separately |
| Programmatic plan or changed sibling before apply | Refusal before authored/lock/slot/boot effects | Normal pre-existing metadata transaction/undo tests still pass |
| Unselected registry version common | Does not poison a selected legacy/locked world | Safety guard is selected-consumer/candidate admission, not a global ban |
| Workspace-own package definition, no rich requires input | Does not get mistaken for its dependencies' request | If that package is selected as a dependency candidate, package guard applies |
| App installer-only common declaration | Package guard does not inspect selected-installer configuration | Accepted app wizard/portable journeys unchanged |

Required test evidence remains the actual CLI RED/control panel and borrowed/programmatic owner tests; source reads alone do not establish no-effect coverage. The new module removes duplicate policy, but completion of A0 additionally requires the call sites above (or an explicitly narrowed claim) and the precise in-place decision. No M05/common configuration feature is closed by this refusal atom.
