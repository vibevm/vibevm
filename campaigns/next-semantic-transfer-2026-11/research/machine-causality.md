# Почему появились fixed routes и grant wrappers

Срез: `vibevm-next@7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Это follow-on к сохранённому аудиту 92 task records, а не изменение его результатов. Читал закреплённые исходники, относящуюся к ним историю Git, `PROP-080` и `/tmp/nst-zap-plan-report.md`. Старые задачи не исполнял. Сборок, тестовых запусков и изменений product/spec/plan нет.

## Вывод

Fixed Cargo/package routes — **намеренно ограниченные proof slices**, которые затем довели до внешних CLI/MCP transports. Они не возникли как случайно испорченная общая сборка. Ограничения зафиксированы до или вместе с implementation, а критерии расширения прямо требуют нового доказанного host/toolchain контракта.

При этом отдельная реализация одинаковой выдачи/восстановления grant в `cli_grant.rs`, `package_grant.rs`, `cargo_grant.rs` **не следует из необходимости раздельных разрешений**. Раздельные scope/policy/domain owners нужны; три копии одного durable pointer state machine — кандидат на общий private kernel. Это допускает даже исходная спецификация. Основание — одинаковые переходы состояния и отказные инварианты, не количество строк.

Рекомендация для semantic transfer: сохранить domain-owned журнал и границу current authority; фиксированные demo routes оставить источником regression-сценариев. Спроектировать один реальный build/package consumer, затем дать ему небольшой общий host-binding механизм. Не переносить автоматически набор специальных `--fixed-*` режимов как архитектуру будущего публичного API.

## Что подтверждает причинность

| Commit | Наблюдаемое изменение | Что из этого следует |
|---|---|---|
| `6c690a5a4` (2026-10-01) | `feat(lifecycle): journal fixed static-file package intent` | Package был bounded с начала, до внешних adapters. |
| `055638147` (2026-10-01) | `record static-file package outcome` | Сначала закрывали pre-effect intent → retained outcome внутри domain owner. |
| `7b8d2a48f` (2026-10-01) | `journal fixed Cargo build outcome`; уже содержит exact Cargo.toml/main.rs, `vibe-r8-fixture`, `r8probe`, `println!("r8")` | Hard-coded fixture не поздний shortcut общего API: он исходный предмет proof. |
| `342a426db`, `7e767a3d5` (2026-10-01) | Retained cold inputs для Cargo/package | Следующий шаг — восстановление после завершения процесса без нового эффекта. |
| `38c27c4a9` (2026-10-03) | Durable CLI lifecycle grant; commit body прямо объясняет Issuing fence до namespace и reattach после process loss | Pointer решает конкретную дыру: новый процесс не должен создавать новый namespace и повторять старый key. |
| `39eb5e6f6`, `4b0edcf96` (2026-10-03) | Public fixed static-file owner API; сохраняется первое created_at при replay | Исправление реальной семантики retry: новый wall-clock не должен менять тот же запрос. |
| `c63fdf1ca` (2026-10-03 02:14 +03:00) | Spec decision отделяет package CLI authority от Validate | Разделение объявлено до соответствующего code commit. |
| `5b48ada64` (2026-10-03 02:25) | Separate package CLI grant | Реализована описанная раздельная authority; mechanics скопированы. |
| `f7f9d2926` (2026-10-03 05:24) | Fixed Cargo owner API и `MACHINE-FIXED-CARGO-BUILD-DECISION` | Source contract запрещает обобщать на arbitrary Cargo/hermetic build. |
| `d9cb608d5`, `c38da0e1d` (2026-10-03) | Cargo durable host grant, затем public framed CLI mode | Proof slice становится достижим через настоящий production entrypoint. |
| `efcfc86c8`, `aac3bbe9c` (2026-10-03) | MCP issue/reattach для package/Cargo | Оба транспорта используют один domain grant, а не каждый свой namespace. |

Это подтверждает заявленную инженерную последовательность. История не доказывает, почему автор выбрал именно такой объём промежуточных wrappers или почему следующий шаг не завершён; это остаётся неизвестным.

### Явное rationale в `PROP-080`

`MACHINE-FIXED-PACKAGE-DECISION` выбирает сначала bounded static-file copy: у него уже есть C1-ready fence, retained result и cold attach. Cargo добавляет process/toolchain scope; deploy на тот момент ещё не имел run/key correlation. Отвергнуты вызов generic executor из descriptor и второй orchestrator reservation поверх domain fence.

`MACHINE-FIXED-CARGO-BUILD-DECISION` допускает только exact fixture и builtin Cargo. Объяснение: произвольная команда/target/toolchain пересекли бы ещё не доказанную границу. Код сохраняет `ambient-rustup-unproven`, поскольку отдельный Cargo home не доказывает PATH/rustup/ancestor Cargo config. Удалить checks и назвать route общей hermetic сборкой было бы неправильно.

`MACHINE-CLI-PACKAGE-GRANT-DECISION` объясняет два независимых требования:

1. Validate grant не разрешает package effect.
2. Новый CLI process не создаёт новый namespace для того же durable scope.

Там же буквально предусмотрен revisit: `a shared private pointer/IO kernel can be extracted without merging the distinct domain policy and owner admission laws`. Следовательно, reuse механики совместим с исходной мотивацией. Нельзя выдавать прежний отказ от «generic cross-domain grant» за вечный запрет общего durable record helper.

## Реальные consumers — что есть и чего не доказано

Pinned `git grep` по production Rust callers даёт две группы внешних composition sites:

- `crates/vibe-cli/src/commands/machine_session.rs`: `CargoGrantService::from_trusted_builtin_host` (216), `PackageGrantService::from_trusted_host` (238), `CliGrantService::from_trusted_host` (261). После explicit Issue/Reattach строятся domain host → route → ServerContext → соответствующий session runner.
- `crates/vibe-cli/src/commands/mcp/mod.rs`: `fixed_package_server_context` (467), `fixed_cargo_server_context` (494); `commands/mcp/fixed_validate.rs` для Validate. Используются те же services и local principal, что в framed route.

MCP tools и framed adapters действительно достигают domain submit/inspect. Поэтому «это только мёртвый тестовый код» неверно. Но reachable consumer — **локальный on-demand CLI/MCP protocol**, а не показанный реальный arbitrary Cargo проект, многопользовательский сервер или deployment fleet. Реального внешнего приложения, которому нужны exact `r8probe` bytes как полезный результат, в этом scope не обнаружено. Конкретная ценность fixture — проверка процесса, journal и reconnect.

`validate_mode` выбирает один fixed domain mode на framed session. Это разумное ограничение совместимости протокола во время эксперимента; оно не доказывает, что будущему user API нужен отдельный session runner и CLI switch на каждую операцию.

## Что wrappers действительно повторяют

Во всех трёх services совпадает устройство durable pointer:

`schema`, `root_token`, `scope {context,target,principal}`, `target_os`, `policy_source`, `grant = Issuing | Issued {namespace, expires_at_ms}`.

Совпадает алгоритм `issue`:

1. Проверить конечный TTL и физическую root identity; взять существующий LifecycleLease.
2. Отказать при existing pointer; если pointer отсутствует, но scope уже есть в C1, отказать вместо replacement issuance.
3. Durably записать и прочитать обратно `Issuing`.
4. Инициализировать текущий `PolicyOwner` для exact scope; выдать C1 namespace; проверить expiry.
5. Durably записать и прочитать обратно `Issued`.
6. Проверить current policy; создать typed domain host; повторно проверить pointer/namespace/policy/root перед возвратом.

Совпадает алгоритм `reattach`: lookup existing pointer → exact binding/expiry/current policy → typed host construction; никаких новых namespace, approval или effect results.

Совпадают private mechanics: `hash_fields`, identity syntax validation, `validate_binding`, `validate_namespace`, `lease`, `check_root`, bounded `read`, atomic durable `publish` с readback, `require_durable` с проверкой **всех** directory checkpoints и exact parent paths. Совпадают refusal классы: MissingPointer, IssuanceIncomplete, ForeignBinding, UncertainPublication и т.п. Небольшие различия оформления ошибок не создают разных транзакционных протоколов.

Это не дублирование engine: wrappers всё ещё используют один `SubmissionReservations` и domain journal. Но это повторные реализации одного issuer/reattach protocol. Исправление root/expiry/durability инварианта приходится удерживать согласованным в нескольких местах.

## Что объединять нельзя без явного решения

| Различие | Validate | Static-file package | Cargo |
|---|---|---|---|
| Target | host-supplied; CLI привязывает к physical root | literal `launcher` | literal `fixture-tool` |
| Policy | host-supplied facts: captured script, detach/cancel и пр. | фиксированные denied script/control facts | фиксированные denied script/control facts |
| Execution owner | FixedLifecycleDecisionHost, вопросы/approval и lifecycle outcome | FixedPackageHost, один bounded copy | FixedCargoHost, subprocess и artifact outcome |
| Дополнительная admission | selected lifecycle source/handler | exact source/output package shape | verified builtin registry/routes + exact source/target/toolchain posture |
| Retained outcome | lifecycle report/park/failure | copy intent/result/partial files | build intent/capsule/result + toolchain warning |
| При policy drift | зависит от lifecycle decision/readonly protocol | живой host `inspect(old)` разрешён, `submit(new)` отказан | `inspect(old)` и submit отказаны |

Последняя строка особенно существенна. Это наблюдаемое поведение, закреплённое тестами:

- `package_grant/tests.rs::live_policy_change_refuses_new_submit_but_old_key_can_be_inspected` сохраняет inspection уже выполненного copy.
- `cargo_grant/tests.rs::current_policy_drift_refuses_live_submit_and_cold_attach` явно ожидает refusal и для `host.inspect("old")`.
- Оба grant services отказывают **cold reattach** при policy drift. Package преимущество относится к уже живому host; после restart нельзя обещать безусловную revoked inspection.
- Namespace expiry/retirement — другая ось: tests для обоих сохраняют old-key recovery и запрещают fresh effect при неизменной policy.

Не доказано, что Cargo inspection требует более строгой policy, чем package. Возможно, это намеренная confidentiality boundary; возможно, незавершённая унификация. Нельзя сгладить различие generic helper’ом по умолчанию. Для нового продукта надо отдельно определить read/recovery authority и execute authority. Требование «revocation запрещает новые effects, но не теряет допустимую историю» нуждается в явной модели доступа к retained данным.

## Более прямой вариант с теми же инвариантами

**Предлагаемая архитектура, не выполненное изменение.**

1. Один private `HostBindingStore` владеет pointer lifecycle, pinned root, bounded decode, durable publication/readback и namespace joins. Его вход — host-constructed typed `DomainBinding`, содержащий domain discriminator, scope, target OS, policy-source identity. Request JSON не может создать этот объект. Domain separation остаётся в key/hash/record identity; один store не означает один grant на всё.
2. Маленькие typed domain adapters владеют только permission facts и owner construction. Cargo сохраняет `verify_cargo_plane` и process/toolchain admission; package сохраняет exact package plan; Validate сохраняет вопросы/решения/контроль. Ни один adapter не хранит новый result journal.
3. Выдача и reattach проходят один последовательный участок под incumbent lease. Вместо нынешнего `drop lease → constructor acquires its own lease(s) → reacquire and recheck` можно дать внутреннему constructor уже проверенный borrowed lease и построить inert host без повторного открытия. Это убирает промежуточное окно, не требует ослаблять revalidation. Во время construction не запускаются providers/processes. При каждом последующем effect domain owner снова приобретает свой lease и проверяет current authority.
4. Общий transport driver отвечает за framing, context/target correlation и сохранение response при output failure. Typed domain route отвечает за request/result schema и dispatch. Не нужен universal writable graph или новый generic executor. Даже если два runners пока оставить раздельными, pointer kernel уже даёт конкретное улучшение без изменения domain semantics.
5. Для реального Cargo consumer заменить fixture *на plan-specific owner*, а не сделать старый hard-code разрешающим всё: captured input closure, выбранный target/profile, trusted process/toolchain contract, ожидаемые writable roots, offline/network posture и exact plan digest. Тот же domain journal связывает этот plan с reservation и actual outcome. `r8probe` остаётся regression fixture.

Первая стадия может сохранить существующие pointer bytes, namespace labels, error mapping и public API: вынести только общий алгоритм. Это отделяет proof of refactor от нового продукта. При semantic transfer в другой checkout совместимость этих временных режимов должна решаться отдельно, а не приниматься автоматически.

Более радикальное объединение pointer и C1 index тоже возможно теоретически, но этого аудита недостаточно для рекомендации: необходимо доказать crash states и не превратить dedup index в competing execution store. Маленький общий private kernel уже устраняет установленное дублирование, не меняя ownership.

## Гарантии, которые должны пережить упрощение

Это gates для возможной реализации, а не запущенные здесь тесты:

- две одновременные first issuance оставляют ровно один namespace;
- crash/uncertain write после Issuing и после namespace не разрешает replacement issuance;
- потерянный pointer при сохранённом C1 не приводит к новому grant;
- copied pointer/index не принимаются для другой физической root/domain/target/principal;
- domain policy не заимствуется из Validate; Cargo builtin-plane checks сохраняются;
- same key + same plan возвращает retained owner outcome, другой plan конфликтует;
- EOF, output failure и process restart не вызывают второй copy/Cargo process;
- first-owner timestamp не превращает retry в новый plan;
- expiry/retirement запрещают fresh effect, но сохраняют предусмотренную recovery;
- read/recovery после policy change проверяются отдельно от effect admission, с явной выбранной semantics;
- every directory checkpoint/readback refusal остаётся uncertain, не превращается в no-effects;
- actual effect truth хранится только в domain journal, не в host pointer.

У исходника уже есть focused tests на эти классы, включая `separate_os_process_reattaches_same_package_result`, `package_and_validate_have_distinct_scopes_and_pointers`, `durability_evidence_requires_all_synced_checkpoints_and_exact_parent_paths` и Cargo plane drift. Их наличие объясняет необходимость инвариантов; успешный runtime результат здесь не заявляется.

## Согласование с Zap и предыдущим аудитом

Zap report — источник текущего durable плана, а не доказательство breadth реализации. Он сохраняет принятые historical/canonical task contracts и открытый M-17 whole-product external coverage. В нём нет основания автоматически объявить эти bounded demos полной универсальной machine API.

Предыдущие `partial` для широких M-16-C.2/C.4 остаются source assessment: конкретные подмаршруты работают по ограниченному контракту, широкий исторический outcome не закрыт. Это не отменяет принятия bounded contributions и не переписывает Zap. Причинность уточняет оценку: «здесь сознательный ограниченный эксперимент, который нельзя переносить как finished general feature», а не «разработчик случайно написал бессмысленную сборку».

## Короткая формулировка для владельца

В `next` сначала доказали безопасный повторный attach на простых операциях: копирование одного файла и сборка заданного Cargo fixture. Потом эти же операции вывели в CLI/MCP. Это полезные испытания журнала, но они ещё не дают обычную сборку любого проекта. Отдельные разрешения на Validate, package и Cargo оправданы; одинаковая механика выдачи и восстановления разрешения сейчас реализована трижды. Её можно объединить внутри, сохранив отдельные domain policy и журналы. При переносе стоит взять эти гарантии и тесты, а публичный маршрут построить вокруг реальной операции пользователя. Отдельно нужно согласовать доступ к истории после revocation: package и Cargo сейчас ведут себя по-разному.
