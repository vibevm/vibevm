# VibeVM — REFINED-IMPLEMENTATION-PLAN

<!-- review-protocol: 1 -->
<!-- pair-revision: 4 -->
<!-- baseline-fingerprint: sha256:0b6524d4128c5ece0763b14f791cbdc3e9cc4c6fd30aefa14b4f93df12892084 -->

## 1. Outcome and non-goals

Жанр: implementation plan. Revision 4 сохраняет предыдущие решения и package/environment phase, добавляя D-025 universal IDE-compatible functionality и D-026 Extreme Total Observability. D-022 остаётся открытым с рекомендацией A; остальные decisions приняты. Все feature designs получают обязательный external service/model/operation path, до любого планирования IDE/plugin/MVP. Кроме явно открытого D-022, решения о scope/policy приняты; новые commands/types/formats ещё предстоит реализовать и проверить. Primary evidence — [PROJECT-REVIEW.md](C:/Users/olegc/git/v/final-improvement/PROJECT-REVIEW.md), E01–E51.

Обязательные результаты:

- Safe source resolution через library/CLI/MCP с exact identity и честным integrity level.
- Unified features/choices, consumer-owned intent, candidate-correct solve, offline/update/reinstall.
- Repo-owned change и отдельные create/build/verify роли; explicit full pass; controlled existing-file editing и проверка новой версии code.
- Neutral complete fact/evidence analysis, clarification и durable explicit closure.
- Structured ordinary Markdown, XML-data и opaque resources; foreign bridge content по умолчанию не получает Vibe semantics. Это необходимый foundation, не optional convenience.
- Интерактивный boot debugger: structure/provenance/why-included/compare/what-if; calculator без boot quotas; preinstall static/dynamic marginal contribution; versioned machine input/output/errors/progress для IDE.
- External package-defined write/config adapters в первой волне с complete trust/grants/ownership/recovery; обязательные Claude Code, Codex, OpenCode, Qwen Code.
- Общие project-owned derivative resources с new identity/exact provenance/rebase; sealed не запрещает собственную локальную версию.
- Distribution source/build identity и default-on disableable artifact smoke; trusted producer/channels, optional signatures.
- Подтверждение concrete prepared publication batch сейчас; архитектура bounded autonomy B готова сразу, activation только будущим owner ruling.
- Оба bridges становятся flow в 1.0.0; два native product feats проверяются на двух stacks; brownfield observe/propose/promote и complete integrated scenario.
- Nix-inspired package/environment manager: exact runtime closures, isolated declared builds, immutable payloads, profiles/generations и полный install/update/remove/verify/recover/GC lifecycle.
- Ранние target/executor/root/principal/platform/realization contracts; одна отдельная обязательная фаза HOME + Linux userspace/Docker/OCI agent images.
- Автоматический machine catalog установленных tools/services, единый интерфейс и локальная Docker-сеть с разрешёнными операциями.
- Полный supply chain без обязательного GitHub: сменяемые sources/registries/mirrors/binary caches/OCI hosts, no-GitHub и prepared-offline tests.
- Общий образ будущего Arch-oriented Linux-дистрибутива и VM acceptance outline; tools/install уже target-ready.
- Extreme Total Observability: все supported features, semantic internal/debug structures, decisions/actions/refusals доступны через machine contracts с provenance и явной неполнотой.
- Сквозная IDE compatibility: capability/model discovery, shared service API, snapshots/unsaved buffers, navigation/diagnostics, prepared edits/actions, events/cancellation/reattach/recovery и semantic package rename.
- Полное per-domain и cross-domain покрытие проверяется headless protocol clients; IDE/plugin/MVP пока не планируются.

Owner policy: вся актуальная собственная линейка — 1.0.0 prototype без постоянных пользователей до личного объявления владельца. Комплектация любой версии Redbook mutable; перед будущей перезаписью агент предлагает новый номер, current variants change уже одобрен без bump. Это не меняет чужие upstream versions, historical captures или machine schema epochs.

Non-goals: workflow language/scheduler/automatic repair loop, personal plan mirror, multi-run store, imposed boot budgets, новый generic BootIR или смена canonical boot representation в этой программе, hidden provider execution, присвоение чужой identity, собственный OS privilege broker, полноценный bootable Linux-дистрибутив/ISO, live host system installation и VMware commissioning в текущей программе. Gemini/Copilot вторичны; tracker export optional. External adapters, structured ordinary content, derivatives, existing-file edits и HOME/Docker/service-catalog phase **не deferred**. Nix runtime/language compatibility не обязательна; pacman backend replacement остаётся отдельной границей. IDE/plugin/MVP, screen design and actual LSP/DAP adapters не планируются здесь. Existing D-007 standalone boot-debugger viewer остаётся обязательным; это не начало IDE/plugin programme.

## 2. Architectural principles and boundaries

1. **Owner rulings before derived state.** §12 governs this programme; M-13 переносит решения в product contracts до зависимого runtime. Stale repository notes не переоткрывают уже принятые предпочтения.
2. **Одна capability — одна library authority.** CLI, interactive viewer и MCP используют общую domain operation; IDE работает через versioned structured protocol. No text-output scraping.
3. **Facts ≠ work ≠ execution ≠ evidence ≠ acceptance.** Existing requirements/terminality не получают hidden policy; change не scheduler.
4. **Exact identity имеет domain/recipe.** Meaningful inputs, canonical order/length framing; SemVer, slug, timestamp и filepath сами не proof.
5. **Prototype policy explicit.** Current own product/package line 1.0.0. Graduation только owner declaration; major-breaking-change commitments включаются после неё. Generic frozen/channel concepts не silently удаляются; Redbook automatic roster bump superseded specifically.
6. **Format ≠ semantics ≠ loading ≠ trust.** Source descriptor, а не suffix/directory/namespace, решает interpretation. Foreign resources сохраняют bytes и default non-Vibe semantics.
7. **Requests ≠ grants.** External adapter задаёт requested new paths/keys/client capabilities; host-issued authorization разрешает exact effects. Derived content не наследует upstream identity/permissions.
8. **Pure inspect, explicit prepare/apply.** Inspect/list/explain/compare работают с captures и не hydrate/invoke/write. Preview явно рассчитывает hypothetical world; pure builtin compiler allowed in memory, effectful stages требуют separate admitted preparation.
9. **No boot quotas.** Размер/tokens не veto установки/сборки. Transport pagination, parser safety и cancellation обеспечивают работоспособность, не вводят скрытый total boot limit. Оценки/unknown states не заменяются фиктивной точностью.
10. **Failure prefix truthful.** Prepared plan/preconditions/authority before effects; journal/receipts/recovery сохраняют выполненное. Произвольные native/remote effects не объявляются reversible или sandboxed.
11. **A and B share pipeline.** Approval receipt A сейчас; bounded policy grant B может разрешить тот же exact prepared plan позже. Unknown/out-of-scope/revoked authority fails closed.
12. **Implementation choices remain engineering work.** Формальные схемы/recipe spelling freeze перед кодом; это не повод повторно спрашивать принятые owner preferences. Открытый D-022 явно отделён.
13. **Definition ≠ realization ≠ target instance.** Desired packages/lock, produced runtime closure и local root/principal/generation имеют отдельную identity; executor OS не target OS.
14. **One authority per package domain/resource.** Vibe source solver и external backend сохраняют собственную semantics; runtime profiles reference exact closures. No competing writable package DBs.
15. **Nix-inspired, host-independent supply chain.** Declared inputs/isolated builds/generations, explicit reproducibility levels and replaceable mirrors/caches. No mandatory GitHub, privileged /nix store or Nix DSL.
16. **Offline population ≠ live activation.** Build a Linux image without starting host services; future system effects require the exact target's authority.
17. **Extreme Total Observability.** Every supported feature and semantic internal model has a discoverable structured inspection/explanation/action/result/error/event path. No hidden CLI/TUI/prose-only authority; unavailable/redacted/uncaptured states explicit.
18. **IDE readiness is architectural.** Domain services own behavior; context/snapshot/overlay/operation identities and safe edit boundaries serve any client. Per-domain coverage gates precede any plugin/MVP planning.
19. **Observe without changing meaning.** Internal capture is separate from domain execution, uses versioned semantic models and does not change solver/compiler results; graph views compose existing authorities.
20. **Connection is not the operation.** Idempotent durable operation identity, responsive controls and truthful partial/recovery results survive transport loss within explicit supported policies.

## 3. Target architecture

### 3.1 Existing layers и минимальные additions

    authored manifests/facts + exact source captures
                    ↓
    selected world + immutable configuration requests
                    ↓
    existing resolver / visibility / extension registry
             ┌──────┴────────┐
    source query library   existing compiler/materializer
             ↓
    repo-owned change binding + existing fact observations
             ↓ explicit requested operation
    existing lifecycle / artifact / deployment mechanisms
             ↓
    exact evidence + transparent change analysis
             ↓ explicit owner disposition
    durable closure

Новые logical modules допустимы как небольшие cells в существующих crates. Отдельный crate создаётся только при доказанной dependency boundary. Нельзя делать vibe-spec зависимым от lifecycle или requirements от executing engine. Source world construction остаётся в vibe-workspace/верхней composition, parser — в vibe-spec/vibe-specdoc, filesystem guarantees — в vibe-safefs.

### 3.2 Exact source query: selection, representation, proof

Предлагаемый public contract:

    vibe spec resolve <spec://address> --path <selected-node> --json
        [--expect-resolution <digest>]
        [--expect-source-hash <recipe:hash>]
        [--expect-document-hash <sha256>]
        [--require-source-proof]
        [--max-bytes <bound>]

MCP spec_resolve принимает address/expectations/response bounds, но не arbitrary path; server context предоставляет selected node. M-06-A source descriptor обязателен до discovery/parsing. Paging/chunking ограничивает ответ, не допустимый общий размер бутлейна. Текстовый default печатает readable unit, JSON возвращает один typed result. Path-only authoritative mode не вводится.

**Address grammar e1:**

- Scheme exact spec://. Canonical public authority — manifest-valid qualified group/name. Legacy host form поддерживается только существующим internal compatibility adapter, не guessed public alias.
- Optional @version — exact canonical SemVer, не version range. Для dependency должно совпасть с выбранным capture; для self возвращать явное unsupported-version-on-self, сохраняя existing отказ до отдельного решения.
- Для declared Vibe specs logical doc path extensionless и relative; URI grammar и physical portable components раздельны. Whole-resource/ordinary-section references разрешаются по explicit resource/export IDs из descriptor; наличие spec:// address не делает содержимое fact. Existing PROP/FEAT abbreviated stem применяется только к declared spec candidates при полной enumeration и ровно одном match.
- E1 отвергает percent-encoded forms; будущий decode — один strict pass до validation. Запрещены dot/dotdot, empty components, backslash, colon/ADS, control/NUL, drive/UNC/device roots, Windows reserved names, trailing dots/spaces. Используется общий portable/NFC/case identity seam, не copy blacklist.
- Не вводить молчаливую normalization разных logical addresses. Physical case/NFC aliases, ambiguous exports/stems и duplicate spec anchors дают error. Same-stem md/xml pair — collision лишь если оба declared representations одного logical spec; ordinary data/resource рядом разрешены.
- Anchor reuse existing hierarchical grammar. Whole document без fragment допустим. Pattern — не point query. ~rN до реализации revision observer → revision-unsupported, не silently ignored.
- Extension suffixes и legacy spellings имеют отдельную validated compatibility conversion; upgrade не удаляет реальные references без inventory и migration.

**Authority:** trusted constructor предоставляет opaque SelectedSourceWorld: selected workspace identity, source roots/captures, effective visible closure, exact resolution digest, format metadata. Existing owner-view semantics переиспользуются. Все lock rows нельзя объявить видимыми. World builder не invokes providers. Зафиксировать availability boundary: unrelated broken member slot не должен читаться ради выбранного point, если это можно сделать по тому же authoritative closure; при невозможности вернуть world-unavailable, не «package missing».

**Observation:** удержать root/parent/file capabilities, прочитать один retained raw snapshot; проверочные passes могут быть двумя. Из этого же buffer получить raw digest, parse, projection, fragment. Pre/post checks доказывают отсутствие наблюдённого изменения в stated window, не невозможность hostile ABA. Query не создаёт .vibe или locks.

**Integrity ladder:**

| Level | Что доказано | Что требуется |
|---|---|---|
| observed | Returned bytes и их digest | Safe retained observation; self и development source допустимы |
| record-consistent | Bytes совпали с local slot footprint | Valid bounded record + same observation; не provenance |
| source-proven | Document является членом captured source, соответствующего trusted expected root hash | Независимо проверенный исходный archive/tree с known hash recipe и membership map/proof |
| derivation-proven | Materialized bytes получены из source-proven bytes указанным transform | Exact trusted converter + authorized overlay + source→output mapping + сравнение returned bytes |

Record.source_hash, recipe label или ещё один local record digest не повышают proof level. Existing flat tree hash не даёт бесплатный Merkle proof. V1 может проверить весь bounded captured source один раз и построить immutable in-memory file index. Query не fetch-ит отсутствующий original; строгий offline запрос отказывает. Для materialized transforms допустим pure builtin conversion verified capture в памяти; native conversion внутри query запрещена.

Hardlink slot не проходит существующий single-link reader. Default e1 возвращает typed unavailable для такого observation, не делает COW. Read-only shared-file support — отдельная проверяемая policy, не ослабление single-link writer ownership. In-place source может быть observed; source-proven — только если весь нужный capture действительно совпал с expected source, не по mode label.

**Representation:** declared Vibe XML использует existing XML→Markdown pivot и named recipe; это source unit, не compiled closure. Ordinary Markdown получает CommonMark section view без Vibe facts, XML-data — raw bytes и optional safe structural view, opaque — raw resource. Binary/non-UTF8 response имеет explicit encoding, не lossy replacement. Raw document, representation и fragment digests раздельны; semantic-equivalence hash не обещается. Return: logical address/resource ID, descriptor identity, capture/proof, recipe/content/encoding, fragment extent и span basis; facts только для declared Vibe source. Physical XML-data positions, если parser их доказал, относятся к raw encoding; Vibe XML projection spans не выдаются за original map. Unparseable data fixture остаётся retrievable bytes; structured view returns diagnostic. Custom executing frontend extraction требует separate capability. M-10 позднее использует эти же proof carriers для local derivative без создания второго resolver.

Errors различают invalid-request, not-found, ambiguous, unavailable, stale, invalid, unstable, internal. Closed reasons покрывают grammar, unselected source, version/expectation mismatch, unsafe/unreadable path, incomplete enumeration, size/depth, duplicate anchor, revision, unsupported format, record/source/derivation mismatch. CLI/MCP сохраняют один reason; ошибки не печатают credential-bearing paths/content.

### 3.3 Consumer configuration: features first, choices as constraints

Выбор архитектуры: **thin choice groups над одним feature/request/effect algebra**. Plain old features недостаточны, независимые choices.requires дублируют dependency semantics. Option ссылается на feature IDs собственного package; feature activation ссылается на typed declared dependency IDs/subskill exports/config values. Version/source/link/visibility metadata живут один раз на declaration, а не копируются под каждым option.

D-003/D-017 приняты. Точный TOML spelling и code-generated contracts фиксируются engineering atom M-05-B в рамках выбранной модели, без повторного ontology vote. Target semantic model:

- Dependency declaration: qualified coordinate/source constraint, stable local declaration ID, optional flag, requested target features, default policy.
- Feature: local feature references; activate dependency by declaration ID; strong/weak target-feature request; exported subskill activation. Aliases локальны package и не заменяют qualified identities в graph/lock.
- Group: stable ID, option feature IDs, cardinality, labels/help, optional explicit default, optional positive same-package parent-option guard.
- ConsumerRequest: owning node/edge, target source constraint, explicit additive features, exact group selections, default policy, provenance.
- ResolvedConfiguration: exact owner capture/definition hash, requested intent, active feature closure, active/dormant selections, induced dependency/provider edges и activated contribution exports.

Cardinalities distinct: at-most-one = 0..1; exactly-one = 1; at-least-one = 1..n; many = 0..n. Legacy exclusive остаётся at-most-one; requires_any остаётся technical OR. E1 реализует эти четыре закрытые формы, не arbitrary expressions/min/max. --all-features не выбирает constrained group member; conflicting complete expansion даёт clear error, а structural validator не вызывает «включить всё» как proof валидности definition.

**Solver integration:**

1. Сохранять все consumer edges и version constraints до unification; current first-wins root dedup исправляется. Additive requests объединяются; incompatible exact selections одного shared instance → conflict с обоими consumers.
2. Candidate metadata связано с immutable source/content identity, а не только version. Доступный каталог фиксируется на одну resolution transaction.
3. Activation implications условны на выбранный package candidate. Rejected candidate не меняет global requests. Существующие DepSolver/DepProvider расширяются typed request/result carriers или эквивалентным complete solver encoding.
4. Одна normalized view используется initial solve, metadata/capability scan, manifest_of, solve_masked, visibility, conditional resolution, graph output и lock serialization. Final graph содержит реальные selected capability/OR-provider edges.
5. Никакого blind five-iteration fixed point для нового semantics. Existing conditional layer проверяется на name-present/wrong-version и negated predicates; если совместное решение не доказано complete, комбинация с новой activation отказывает явно.
6. Candidate lacking requested option — candidate incompatibility, не global NeedsSelection из discarded version. Source corruption — отдельная ошибка. Mandatory unresolved group обсуждается только для реально requested/reachable owner.
7. Semantic default не выбирается backtracking. Default-based selection сначала bind к показанному exact owner definition и policy, затем становится hard request. Если solve требует другой semantic default, вернуть rediscovery/reconfiguration, не выбирать его.
8. Explicit stable selection может участвовать в backtracking по совместимым candidate definitions; final exact payload/definition и induced graph входят в reviewable apply plan. Persisted definition drift не применяется unattended без explicit reconfiguration authority.
9. Backend без capability для новой algebra возвращает unsupported-solver-mode до mutation. Не обещать parity naive/sat/resolvo без реализованных и проверенных adapters; нельзя silent fallback.

**Fresh/offline/update:** requested и resolved selections хранятся раздельно; request digest проверяется до Fresh path. Reinstall воспроизводит exact lock capture/selection, не re-solves. Fresh clone восстанавливает authored intent; offline missing exact capture → unavailable. Update сохраняет selection, показывает changed payload/definition; removed option/new required group → stale/needs-selection. --force не меняет intent, --assume-yes не отвечает на вопросы.

**Discovery/UI:** show choices --json — pure cached catalog query. Explicit fetch/prepare может наполнить cache, но не менять project. TTY wizard показывает default и effects; non-TTY/JSON/unattended никогда не prompt. Для первого unattended install default допустим лишь при явной consumer/CLI allow-defaults policy (D-004), иначе NeedsSelection. Explicit empty list — clear many; group omission — unspecified, не clear. Dormant branch selections сохраняются как intent, effects выключены; удаление dormant option всё равно диагностируется.

Guard DAG только positive same-package references, никаких OS/env/Git branch/network predicates. Новые transitive unresolved groups возвращаются структурным запросом input с exact definition; не строят бесконечный wizard/resolve loop. Повтор identical discovery state без нового input завершается needs-selection, не retries.

**Transaction:** complete request validation/solve/trust/collision/effect plan до project writes. Authored manifest, lock, owned materialized/boot outputs публикуются через recovery protocol с exact before-images и request/catalog preconditions. Multi-file filesystem operation не называется физически атомарной. Third-party hooks не rollbackable автоматически: их admission и intent предшествуют execution, partial failure reported. Старые dependencies, нужные другому root, сохраняются.



**Redbook policy (accepted D-004/D-005/D-019):** continuity exactly-one, default multi-user-planning, WAL alternative; addons many и positive branch-local additions. Explicit unattended selection/allow-defaults обязателен, applied choices persist. Не добавлять symmetric global WAL/MUP conflicts: независимые package dependencies допускаются, группа отвечает только за выбор Redbook. Миграция root manual excludes/root dependencies сохраняет исходный смысл и shared dependencies; неоднозначность требует ответа. Current roster/choice change делается в existing 1.0.0. Все будущие версии Redbook допускают composition changes; перед будущей перезаписью агент предлагает new-version option. Это отдельное решение от разрешения publish batch, текущий no-bump выбор уже сделан. Reader capability/schema epoch защищает old 1.0.0 builds: min_vibe_version alone недостаточен при mutable product version.

### 3.4 Minimal repo-owned change

Change не package kind, workspace node, diff или personal plan. Оно живёт в configured spec root:

    changes/<immutable-id>/
        change.toml
        intent.md или intent.xml
        acceptance.md или acceptance.xml
        work.toml                 # optional
        questions.toml            # M-04
        closure.toml              # M-04, excluded from intent snapshot

ID имеет одну выбранную UUID/ULID grammar; slug/title — metadata. Renaming slug не меняет directory/spec addresses. Нельзя создавать второй current pointer.

ChangeManifest: ID, owning selected node, explicit origin fact addresses, accepted exact source captures, local document roles, optional refinement/decision references. Package facts не копируются; локальный refinement добавляет обязательства или явно именует deviation/replacement. Смена accepted origin — explicit rebase plan; latest SemVer не заменяет старый capture. Omitted package fact — out of selected scope, не waiver. Consumer adoption меняется отдельной операцией, не вследствие scaffold.

WorkIntentGraph: stable item IDs, facts, artifact needs, explicit predecessor relations и touches hints. Нет done/owner/current/status/evidence/runtime loops. Work DAG validation — purely structural. Existing artifact/evidence может закрывать need без work item. Absence work и absence evidence — независимые observations.

Identities:

| Carrier | Содержимое |
|---|---|
| intent_snapshot_id | Meaning-bearing change manifest fields, local normative docs, accepted origin captures, relevant explicit dispositions; canonical frame |
| work_plan_id | Work graph, отдельный digest; не auto-invalidates unrelated execution |
| resolution_id | Selected effective world, exact package captures, active configuration/stack/provider identities |
| lifecycle_declaration_id | Existing execution declaration semantics |
| execution_request_id | Intent snapshot, selected work/operation, consumed artifacts/input contract, applicable policy, resolution и declaration |
| execution/run ID | Конкретная попытка; не substitute для перечисленных digests |
| closure ID | Explicit disposition и exact accepted snapshot/evidence/policy; excluded from its own subject |

Selected work влияет на request; cosmetic editing или unrelated work reordering не invalidates intent. Если contribution реально читает весь work graph, его digest становится explicit input. Questions влияют на readiness profile; promoted answers становятся normative facts. Closure/observations/run logs не попадают в snapshot входа самих себя.

Read-only check/show/coverage полезны без executing change. New --from scaffold — отдельный явный write с no-overwrite/CAS. V1: one selected node, one origin feat, existing stack; несколько authored changes разрешены, multi-origin execution позже не prerequisite.

### 3.5 Lifecycle compatibility and post-state causality

D-002/D-018 приняты: create производит/меняет code; build алгоритмически собирает; verify проверяет результат; full pass имеет отдельный явный вход. Existing internal nine-phase declarations/order remain reusable machinery; прежнее правило безусловного producer dispatch из inclusive verify изменяется по D-002. Обычный verify и verify --change могут remeasure/rebuild/retest и reuse/adopt completed exact transition, но не dispatch нового producer. Если creation ещё нужна, возвращает creation-required и не выдаёт это за успешную проверку. Прежнее public inclusive behavior фиксируется как historical source contract и мигрируется в M-13/M-03, а не остаётся незаметным вторым поведением обычного verify. Build/verify не dispatch declared production даже из prerequisite slot; явные trusted verification/judge operations могут выпускать evidence, но не редактировать product source. Unknown effect classification отказывает до execution.

Предлагаемый convenience entry — change apply: fixed create→verify application composition, без нового workflow language, next-task selection или repair loop. Он показывает полный план, выполняет не более одной production operation для exact request, может park для host, затем проверяет новую source state. Verification failure завершает invocation; новое исправление выбирает внешний процесс. Спецификация command grammar и legacy build feat explanation обновляется M-13/M-03-A; build feat generation promise переносится в explicit create/full-pass journey, не объявляется уже реализованной старой формой.

Machine state workspace-rooted, один active/parked scoped continuation на workspace. Другой scope/node не displaces его автоматически. Cancel/displace explicit; foreign outputs сохраняются. Отдельная worktree имеет собственный state root.

Carriage: CLI/MCP, lower RunMetadata/Context, state header/records, plan/report/tasks wires, outbox/resume descriptor, slot continuation, artifact records и verification frames. **Production request identity отдельно от observing invocation:** переход create→verify не должен сам менять identity уже выполненного producer из-за requested-phase/chain observers. New recipe задаёт именно реально consumed inputs/artifacts и selected work/intent/world/declaration. Legacy fingerprint recipe остаётся legacy.

M-03-D может начать proof с disjoint outputs, но такой proof **не закрывает** milestone. Transition atom M-11-D сохраняет свой ID и выполняется раньше, после M-03-C, до complete M-03-D/E; он не ждёт завершения всего brownfield M-11.

Required transition contract:

- Immutable baseline capture фиксирует pre-read set, write-before images, directory membership, intent/production request/declaration/consumed provider identities.
- Completion связывает declared write set и post witnesses; executed producer и explicitly adopted host output различаются по authority. Non-empty file — shape acceptance, не semantic proof.
- Outside write set все required inputs сохраняют baseline witnesses. Overlap хранит оба pre/post; новые/deleted files и parent directory changes включены.
- Legacy pre-dispatch measurements не переписываются. Scoped evidence явно различает historical consumed pre-state, admitted transition и current post-output observation; старые input bytes не relabelled matched current bytes.
- Producer reuse: same production request/declaration/genuinely consumed captured inputs плюс current owned outputs==post. Unrelated rebuilt predecessor output не invalidates producer просто из-за нахождения в общем artifact registry.
- Build/test старого source остаются историческими/неприменимыми. Scoped verify выполняет нужный deterministic prefix на post-source и требует новые current-source measurements, затем checks transition/output. Gate не «прощает» произвольный stale: только exact admitted transition переносит applicability; остальные drift/missing/unstable отказывают.
- Если genuinely consumed input изменился, verify возвращает typed stale/creation-required; не запускает repair. Full pass не превращает этот отказ в retry loop.
- Concurrent outside-write edits, lost ownership, unsafe aliases или отсутствующий comparable witness запрещают acceptance. Captured proof заявляет ровно измеренную stability, не невозможность hostile ABA.
- При crash recovery доступны exact before/after/third-state distinctions; не присваивать unknown state и не считать external native code sandboxed.

Exit complete scenario: агент меняет существующий исходник, проверяются разрешённые effects, новая версия реально собирается/тестируется, повторный verify fresh не создаёт код снова. Это обязательный результат первой полноценной change delivery.

### 3.6 Analysis, clarification and closure

Change analysis — shared library composition existing fact observations/terminal resolver, work edges, exact execution evidence и explicit policy/dispositions. Новый generated report допустим как envelope этого join; новый независимый terminal evaluator — нет.

Каждая строка содержит fact subject/capture, artifact kind, authored state, work coverage, evidence source/strength, current comparison, observer availability, applicable waiver/accepted risk. В report есть completeness и omitted/unavailable scopes. Truncated requirements_query нельзя использовать для closure: нужен same-snapshot exact-address set/batch API либо complete internal observation. Несколько страниц из меняющегося дерева не считаются одним snapshot без identity check.

Legacy implements/verifies relation count свидетельствует о связи, не successful execution. Matched lifecycle evidence свидетельствует о matching identity, не semantic correctness; successful handler без evidence остаётся отдельно. Unclassified requirements не vacuously terminal. Documentation/decision/research/plan/disposition/external учитываются existing artifact vocabulary, не только implementation/test.

Waiver требует exact fact+artifact need, rationale, approving authority, scope/policy и revisit condition; не превращает missing в satisfied. Accepted risk отдельна от waiver. Observer unavailable не равен missing. Profile может разрешить acceptance with risk; нейтральный query не скрывает missing/unavailable.

Questions привязаны к exact target fact digest. Explicit answer recording transactionally создаёт local refinement/decision, не правит package. Stale question нельзя silently apply; unknown/hold — local status. Sensitive answers не сохраняют secret value. Deterministic checks — grammar/refs/duplicates/DAG/staleness/conflicting dispositions. Ambiguity/testability/Impact×Uncertainty — explicit advisory provider, bounded and optional; no hardcoded five questions or English regex quality verdict.

Closure — accepted/abandoned/superseded; exact subject snapshot, selected world/policy, retained evidence payloads либо проверяемые immutable references, explicit author authority и unresolved dispositions. Lifecycle cache IDs недостаточны. Closure record excluded from intent snapshot. Source/definition/policy/output change делает применимость closure stale; историческое решение остаётся фактом, current readiness пересчитывается. Personal plan import/export optional one-way, не central authority.

### 3.7 Source classification: ordinary Markdown, XML-data and foreign bridges

D-014 обязательна до public lookup/scan/conversion. Один SourceDescriptor несёт logical resource/export identity, source path/capture, media/syntax, interpretation, role/loading и provenance classification rule. Разрешённые interpretations: vibe-spec, commonmark, xml-data, opaque. Это interpretations существующих resources, не новые package kinds.

Package/project author задаёт explicit file declaration или default для группы путей. Exact declaration может уточнять directory default; две conflicting equally applicable declarations → diagnostic, не last-wins. Suffix/namespace/content sniffing могут проверить заявленный syntax, но не включить Vibe semantics. Legacy source layout переносится через deterministic migration в M-06-A одновременно с новым reader; M-13 поставляет runnable migration engine/recovery, а не требует ещё не существующий SourceDescriptor implementation. Unknown/ambiguous intent не угадывается.

**Bridge default:** imported foreign payload по умолчанию не знает VibeVM. Markdown получает ordinary structural interpretation либо explicit opaque; XML — data либо opaque. Только явно заявленная bridge-authored Vibe wrapper/spec или explicit admitted source declaration участвует в facts/directives. Resource closure целиком сохраняется, включая вспомогательные файлы, scripts/templates/data. Inert for Vibe parser не означает безопасный prompt для LLM или разрешённое выполнение script; activation trust отдельно.

- CommonMark: полноценный structural parse headings/sections/fences/links, literal Vibe annotations. Stable durable exports задаются manifest mapping к section selector и exact source witness; snapshot-local node IDs могут измениться при изменении структуры, но не притворяются persistent requirement IDs. Duplicate heading без однозначного export → ambiguity.
- XML-data: raw encoding/comments/whitespace/bytes сохраняются при install/derive. Optional safe tree view не загружает внешние entities/resources и не transforms в Markdown. Opaque mode хранит даже intentionally invalid XML fixture; parse failure structured view не invalidates unrelated fact source.
- Vibe XML/Markdown: existing fact/directive semantics и pure pivot применяются только после classification. Source/projection spans и hashes раздельны.
- Same physical stem, например manual.md и manual.xml, не означает same identity. Pair collision применяется к competing representations одного declared logical spec; ordinary data и documentation могут сосуществовать.
- Resource conversion, facts sync, requirements enumeration, source resolution, compiler closure, skills/bridge hydration и debugger потребляют один descriptor view. Нельзя оставить suffix-only fallback в одном из consumers.
- Обычный ресурс не включается в boot из-за нахождения под spec root. Explicit loading определяет участие; embedding literal data не должен превращать его содержимое в compiler directives/managed markers.
- Profile spec_format преобразует только declared Vibe representations. Foreign CommonMark/XML-data/opaque sources остаются source-exact; consumer-side projection — отдельный derived artifact с recipe/provenance.

### 3.8 Boot debugger and preinstall calculator

D-006/D-007 требуют полноценную capability без imposed boot-size/token limits. First output — interactive visual inspector на существующем canonical representation плюс тот же machine API. Не закрывать эту цель одной таблицей bytes или CLI --json.

**Captured views:** package/dependency/contribution/document/fragment tree; ordered static/dynamic/conditional/on-demand lanes; source positions и inclusion chains; direct/indirect/shared costs; diagnostics; recorded compilation stages; comparison двух snapshots. Selecting a fragment показывает source и почему он оказался в boot. Selecting package показывает собственный вклад и зависимости. Unsupported exact mapping после whole-lane transform маркируется unavailable/aggregate-only, не распределяется пропорционально без proof.

**Measures:** exact raw/emitted bytes, Unicode scalar count, tokens с tokenizer identity/version и exact|estimated|unavailable, unique payload vs load occurrences, known/unknown session tail. Provider billing/cache hit не выводится из локальных bytes. Никаких high/low budget verdicts или automatic refusal по итоговому размеру; million-token boot допустим. Transport chunks/page size, streaming/tokenization и cancellation не ограничивают total допустимый boot.

**Preinstall/what-if:** baseline selected world и candidate world с exact source captures/configuration сравниваются тем же resolver/compiler semantics. Отдельно static and dynamic hypothetical placement, только где legal loading contract; нелегальный сценарий explicit incompatible, не secretly меняет пакет. Candidate includes transitive/shared deps, choice selections, condition context и derivative bindings. Direct package bytes, inclusive dependency contribution и marginal delta могут различаться; нельзя складывать marginal deltas от независимых experiments. Dynamic result сообщает selected OS/context сценарий, condition list и conditional potential; exact total неизвестного future context не выдумывается.

**Purity/effects:** inspect/explain/compare/get-section работают с retained snapshots. Explicit preview допускает pure builtin compilation в памяти на prepared captures и ничего не публикует. Missing source или stage requiring external/native execution → structured incomplete/needs-preparation. Fetch/hydrate/native stage допустимы только separate explicit admitted preparation operation, заранее показывающей effects; она создаёт immutable preview evidence, которое viewer потом читает. Инсталляция ещё не применена, project manifest/lock/slots/boot unchanged. Ни один обычный query не запускает provider исподтишка.

**Machine contract:** versioned request and response unions для capture/read, tree/list, explain, costs, preview, compare, cancel/progress. Request содержит operation, selected context, snapshot/expected-world ID, filters/selection scenario; response — nodes/edges/source descriptors/mappings/measurements/completeness/diagnostics. Errors and progress events typed; request_id/sequence позволяют связать asynchronous progress с terminal result. Snapshot IDs content-derived; element IDs reproducible within same snapshot, surviving logical entities correlate across snapshots explicitly. Pagination/chunk handles bound to snapshot/filter; mixing versions returns stale, not plausible merged tree. Абсолютный filesystem root приходит из trusted surface, не из arbitrary MCP body.

Одна library выдаёт значения CLI JSON input/output, MCP и локальному interactive tree/details/diff UI. IDE может использовать протокол независимо от UI. Exact visual frontend framework — implementation choice; протокол не зависит от него. Existing compiler observer/IR/owner-world APIs обеспечивают actual trace/provenance, не второй сборщик. Canonical compaction/status stripping/externalization — вне этого прохода: debugger first.

### 3.9 External adapters: new clients without compiled catalogue lock-in

D-008/D-012: first-wave supported clients — Claude Code, Codex, OpenCode, Qwen Code. Current five legacy profiles — characterization input, не новая граница extensibility. Gemini/Copilot secondary, не acceptance dependency.

**Package-authored request:** qualified client/adapter identity, captured descriptor, supported version predicate, surface×scope×resource-shape matrix, relative destinations, structured config member paths, rendering data, requested observation/installation operations. Новый client, path или key не требует нового enum case/core release, если выражается existing operation algebra.

**Host authority:** closed generic operation semantics — contained file/tree projection, typed JSON/TOML member reconciliation, approved renderers, safe observation, receipt-scoped removal. Host binds symbolic roots и issues grants по exact normalized resources/operations. Package supplies requested path/key; оно не может само выдать grant. No broad implicit home-write. Package operations cannot edit their own grant issuer/policy store, trusted-producer configuration or publication-mode switch; эти owner-only control-plane resources исключены из generic adapter write grants. Plan exposes resulting active content/MCP launch references и effects до consent. Presence/detection/manifest/hash не authority.

ClientExecutables/deploy native wire сегодня имеет три named fields (E27); migrate to open validated capability map. Compatibility readers/adapters preserve current values, but Qwen не добавляется merely fourth hardcoded slot. Real external package must introduce a client with unchanged engine binary.

Pure discovery/plan использует prepared captures and host-read observations; no executable version probes, PATH search, hydrate, native provider or network. Explicit probe/prepare creates bound evidence. Actual client CLI operations используют admitted executable identity и operation/typed-parameter recipe; generic renderer не превращается в shell. Novel effectful native provider проходит separate explicit code trust; typed replies/grants не OS sandbox.

Ownership: existing standalone/project-binding/deploy receipt families retain authorities. Cross-family adoption explicit, equal bytes не ownership. Resource collision judged physically; config members owned logically, whole config lock excludes concurrent updates. Upgrade added/retained/removed resources, preimage/after-image и descriptor/grant/semantics version bound; expanded grant requires new admission. Current OpenCode update not automatically reversible (E27): retain real before-state либо report non-reversible prepared effect, не обещать inverse без evidence.

Revocation blocks future apply under revoked grant; receipts/inspection remain. Cleanup uses separately authorized receipt-bounded host primitive and retained operation data even if package missing. Never execute revoked native implementation just to uninstall it. Client unsupported feature must refuse explicitly; mandatory installation/update/removal scenarios for chosen four cannot close on unsupported placeholders.

### 3.10 General project-owned derivative resources

D-009 принят: механизм обязательный. Пользователь создаёт собственный resource на основе captured package asset, даёт отдельную logical identity и явно выбирает, где подключить его. Upstream source/slot и author attribution не меняются.

Authored DerivationBinding фиксирует owner node/resource ID, exact base package/resource/capture, source interpretation, local source/tree, operation/recipe и optional export compatibility target. Generated result records bind base/local/result digests и complete provenance chain. Local source хранится в repo-owned месте; content-addressed cache — disposable output, не единственная authority. Same result bytes не сливают разные provenance/ownership identities.

Full replacement/copy-local-source — базовая универсальная операция для ordinary Markdown, XML-data, Vibe specs и opaque file/tree resources, включая code-bearing bytes как данные. Structured named-slot composition дополнительна; она не условие права иметь local fork. Derive/inspect/diff не executes scripts/native/config artifacts. При downstream execution/install/deploy новую identity нужно отдельно admit; upstream grants/signatures не переходят на derivative.

Sealed/export policy может описывать supported interface/compatibility, но не запрещает создание/хранение собственной версии. Нарушение конкретного target interface диагностируется при подключении к нему; local owner может выбрать другой host-owned target/explicit override, не выдавая это за поддержанный upstream вариант. Source license/notices/provenance сохраняются; new identity не сама по себе новое разрешение на распространение.

Update/rebase: changed base capture/descriptor → rebase-required. Show old base/new base/local diff; no automatic merge even for text. User-approved rebase binds a new base after explicit compatibility/result validation, preserving old accepted provenance. Missing old capture/ambiguous export/conflicting patch cannot be repaired guessing. Offline uses retained exact capture; missing source unavailable. Concurrent local/base edits invalidate prepared apply.

M-01 proof carriers reused; M-06 debugger shows base→local→result and actual contribution. First proof narrow text file is an intermediate stop, not completion of general resource support. Install/build/deploy use existing registry/receipt engines; no generic priority/workflow language added.

### 3.11 Bridges and brownfield

Both Spec Kit and external-skills bridges become flow in current 1.0.0, same package/upstream identities. Imported resources preserve exact closure and declared non-Vibe semantics; wrapper instructions are separate authored resources. Actual Spec Kit pin governs resource/script/template/platform audit; research snapshot is not substituted.

Required bridge procedure steps using scripts go through explicit typed admitted action and external-adapter/grant machinery; no implicit script execution merely on read. Native bug/assess are flow-supplied change profiles, not runtime state machine. Native product feat reuse is tested on two stacks with actual controlled existing-file edits after M-03.

Brownfield observe/propose/promote retains original intent/evidence boundary. Core inventory safe and explicit scope/exclusions; stack analyzers separate operations. Immutable capture or complete affected-set revalidation/exclusion before promote. Prototype does not justify silent overwrite. M-11-D general transition is pulled forward into M-03; later M-11-E proves real brownfield composition.

### 3.12 Release evidence, prototype policy and A/B authorization

Extend existing distribution manifests, not parallel receipt engines. Subject binds exact source commit/tree/lock/recipe/features/toolchain, native target/environment, bundle digest and measured checks. Source tests/checks remain explicitly requested; source identity always observed, missing full-test result never pretends passed.

**D-010:** artifact smoke default-on at publication preparation; explicit parameter disables it for named scope/targets. Report enum/status separates passed, failed, skipped and unavailable; skip flag/policy/provenance bound to prepared plan. Failed running smoke blocks default publication; a later explicit skip creates a new plan, not a rewritten pass. Ready bundles required before destructive prepare; smoke requirement is satisfied by actual passed evidence or the explicit accepted skipped policy. Skipping smoke does not skip archive/identity validation or authorize extra destinations.

**D-011:** trust derives from configured approved producer/channel plus exact subjects/recipes, or optional approved signatures. Local arbitrary JSON observational. Protected service API, authenticated cluster channel or local trusted execution may serve; no single vendor/crypto system mandatory. Authorized issuer/subject/policy checked when signing enabled. Untrusted PR jobs have no publish credentials or release-authoritative signer; execution/build children credential-free.

**D-015 shared pipeline from outset:**

    prepare exact plan → validate effects/evidence/preconditions
      → authorize(plan_digest, policy_generation)
      → apply with immediate pre-write revalidation → verify/receipt/recovery

Mode A is active: one user approval binds complete reviewed batch, targets, replaced captures/assets, effective smoke policy и inverse/non-reversible facts. No repeated approvals inside unchanged admitted batch. Any material plan/world/destination change requires fresh plan/authorization.

Mode B is implemented and tested now, **not activated now**: owner-approved policy enables autonomous batches within explicit repository/org/target/operation/version/capture/effect boundaries. Policy grants may be revoked/expired; active generation and exact scope rechecked before effects. Outside policy → approval-required/refused before write. No silent fallback to global permission; no inference of B from prototype status or previous successful batches. Same plan/receipt/recovery implementation serves both modes. Parameterized integration tests prove equivalent effects for the same authorized plan under A/B.

Redbook version offer applies when an agent changes the Redbook package definition/roster, not when a consumer selects an existing option or reinstalls the same capture. It remains independent: future composition overwrite requires offering keep-version/new-version choice before overwrite; B cannot silently suppress it. Current variants change has explicit keep-1.0.0 decision. Completing that choice does not authorize actual remote publication; current A batch approval still applies.

Publish complete local bundle set before remote delete/tag mutation. Preflight binds expected remote generation/IDs; retain old assets/metadata for recovery. Missing server CAS cannot become an atomicity claim; detect interference and stop/report exact partial truth. No live mode-B enablement, CI change, package publish or release occurs while updating this review pair.

D-013: current line is prototype until owner personally declares graduation. Neither open repository, uploaded archive, current 1.0.0 nor usage count flips stability flags. Algorithmic migrations delivered per D-003; no perpetual support promise for every experimental epoch. On future graduation define major-breaking-change notification/migration/support policy. Product version, schema epoch, exact capture and compatibility mode remain independent.

### 3.13 Environment management: one package system, explicit targets

**Owner mandate D-020/D-021:** VibeVM развивается в менеджер пакетов и окружений: личная система пользователя в $HOME и сборка Docker/OCI-образов обязательны в одной отдельной фазе. Будущий Linux-дистрибутив — направление дизайна; tools/install interfaces должны подходить и для него. Это расширяет прежний non-goal OS management. Собственный privilege broker, ядро, загрузчик и замена всех сторонних package engines сейчас не требуются. Архитектурные seams M-14 входят раньше consumers; практическая фаза M-15 не прячется в defer table.

Один authored environment manifest использует существующие package identities, requires и feature/choice algebra. Он может жить в репозитории образа или в пользовательском каталоге без Git и без coding project. Environment — deployable consumer context, не новый package kind, change или personal stewardship plan. tool уже существует как kind; installable payload, service и OCI image — capabilities/artifacts. Не добавлять kind=os/docker для каждой формы доставки и не заменять npm/Cargo resolution их внутренностей.

Разделить пять объектов:

| Object | Meaning / identity |
|---|---|
| EnvironmentDefinition | Переносимый desired composition: roots, choices, target constraints, tool/service exports, runtime bindings по именам; без machine paths/credentials |
| EnvironmentLock | Exact selected Vibe captures/configuration + external backend closures + build/runtime dependency roles + target ABI; derived once per environment domain |
| ExecutionContext | Где исполняется конкретная операция: Windows host, Linux BuildKit worker, local Linux process; admitted executor/tool/provider identity, network/mount/principal capabilities |
| TargetBinding | Stable environment instance ID, generation, target OS/arch/ABI/libc constraints, offline/live mode, filesystem roots/prefix, target principal/home и storage backend; local physical binding отдельно от portable definition |
| EnvironmentGeneration | Realized payload inventory/metadata, dependency closure, generated boot/adapters/service catalog и activation links, связанные с lock/recipes; mutable data и secrets не часть immutable generation |

Для compiler toolchain отдельны build machine, machine where produced tool runs и, если сам tool является compiler, emitted-code target. Native Vibe extension ABI всегда относится к executor, а не устанавливаемому Linux binary. OS-only when guard не описывает архитектуру/libc. Existing host-based guard migrates to an explicit compatibility interpretation; target predicates не переключаются молча. Feature selected world и boot conditions получают target context. Windows host не должен выбирать Windows launcher для Linux image.

Target kinds: workspace projection, user profile, offline Linux rootfs/image и future live Linux system. Поддержка описывается capabilities; M-14 implements existing local binding and explicit unsupported responses for future operations, M-15 delivers home/rootfs/OCI execution. Не требуется умение всех providers работать со всеми target kinds. Unsupported target/ABI/metadata отказывает при plan, не после partial installation.

Root paths передаёт trusted surface; provider не читает ambient HOME/PATH, чтобы заново выбрать destination. Root-relative resource identity включает environment instance/root/principal и case/path policy. Physical locks additionally recognize aliased roots across instances. Clone/image instantiation получает новую instance binding; generation content identity переиспользуется. Move/relocate/adopt — explicit validated operation, прежняя receipt не даёт права на случайный новый каталог по тому же path.

Layout remains centralized per PROP-052. Existing project vibevm/ tree не переезжает. Новая environment layout table владеет отдельными configuration/store/profile/state/cache roots; proposed home defaults under existing Vibe settings/data configuration фиксируются M-14-A, а не размазываются literals по providers. Пользователь может выбрать весь prefix под своим HOME. Stewardship state не становится environment package DB.

### 3.14 Installable tools, payloads and dependency closure

**Recommendation:** единый lifecycle source→build/package artifact→realize payload→activate profile. Существующий bare install сохраняет project materialization/boot contract. Новый explicit env apply или tool install --environment удобный вход составляет уже существующие операции по полному плану; не вызывает agent create или scripts по скрытому hook. Необходимость production возвращается отдельно. Legacy bin list/path/exec migrates to the same recorded artifact validation; existence/debug-first fallback не источник installed-runtime truth (E31).

Package may supply captured prebuilt archives/files or an explicit deterministic build recipe. Source tree, build record and installed payload — разные identities. A realization binds exact input captures, dependency roles, provider/recipe/config, executor/target/platform ABI, output digest and complete install metadata. Reuse existing A2/artifact storage where it proves these facts; extend its schema/observers for missing facts, no parallel pretend evidence engine. Native providers remain explicit trusted executable code.

**Payload manifest:** files/directories/links, raw content digest, relative target path, executable/mode semantics, ownership mapping, allowed metadata, exported commands/libs/data, configuration templates, runtime dependency closure and optional service descriptors. Linux UID/GID mappings are target principals, never Windows host IDs. Unsupported xattrs/capabilities/device nodes fail explicitly. M-15 supports the ordinary tools subset (files, directories, safe symlinks, modes); full OS metadata vocabulary and admission seams are fixed now, privileged node realization is future work.

Source-address portable grammar stays scoped to spec/resource addressing. Linux package paths have their own target filesystem grammar; a global Windows filename blacklist must not reject valid Linux payloads. Validate archives before writes: traversal, duplicate/overlapping entries, hardlink/symlink escape, case collision under the selected target policy and metadata escalation. Legitimate rootfs absolute links describe target-namespace paths; extraction never follows them into the host. A Linux executor or metadata-preserving image writer handles POSIX semantics; copying through NTFS and assuming chmod/chown survived is invalid.

**Dependencies:** reuse M-05 Vibe algebra within a resolution domain; add typed roles build, tool/executor and runtime, explicit target constraints and backend-qualified package references. Each environment may choose a different Vibe version/configuration; inside one shared instance incompatible requests still conflict. Backend-native versions (including Arch epoch/pkgrel) are opaque to Vibe SemVer ordering. Requested roots and exact resolved transitive closure remain separate. ABI/runtime libraries must be either captured payload dependencies or named verified platform requirements; ambient PATH/libc does not silently satisfy a hermetic promise.

Home first-wave packages must actually run from their chosen prefix: static/relocatable payload, supported runtime closure or a declared compatible host ABI. Prefix-bound binaries carry that constraint; no promise that arbitrary /usr-oriented Arch packages work under HOME. A wrapper setting PATH cannot repair every ELF loader/RPATH/shebang issue. relocate is a tested capability or requires rebuild; unsupported relocation explicit.

Environment generation may share verified immutable payloads via an existing/adapted content store. Do not mutate source slots into runtime payloads. Profile command conflicts require explicit selection/alias, never last-writer wins. env exec resolves a selected generation with a controlled environment; shell activation emits an explicit script/diff and only changes shell startup files when requested. Each invocation pins its generation; switching active profile does not retarget an already running process.

Package-manager operations form one library/CLI/MCP lifecycle: discover/search, inspect/explain dependencies/files/owners, resolve/lock, fetch, plan, install/apply, update, remove, verify/repair, activate, list generations, rollback and GC. E1 implementation may group verbs, but cannot claim package-manager completion from install alone. Remove recomputes the desired closure and reverse dependencies, preserves shared/explicit roots and mutable user data; cascade must be explicitly included in the plan. GC separately considers all retained generations, live uses, rollback pins, build inputs and other environments; no implicit quota or unproven reference-count-only deletion.

### 3.15 Transaction domains and Arch interoperability

**Recommendation pending D-022:** first coexist with pacman; Vibe owns environment intent and Vibe payloads, pacman/libalpm owns Arch package selection, database and files. Later replacing the pacman UI through libalpm and replacing the entire Arch backend are different projects. The generic backend port allows either without changing environment manifests into shell scripts.

Backend interface: capabilities, prepared catalog/installed snapshot, resolve with native version semantics, fetch exact artifacts, proposed transaction/footprint, apply against checked DB generation, observe/verify/recover. Package-defined implementation uses existing registry/provider model and versioned wires. A native planning call is trusted execution in explicit preparation; pure plan/inspect consumes its captured result. No parsing of human progress prose as authority; use library API or bounded machine outputs with independent post-state validation. CLI facade choice does not define core schema.

Each backend retains sole authority over its domain. Vibe records external roots, snapshot/config/verification policy and resolved closure evidence, not a second writable copy of pacman's installed DB. Foreign solver runs on a frozen catalog/current-state capture. Changed resolver/DB result requires a new prepared plan; apply never silently re-resolves latest after approval. Vibe-owned packages may require external capabilities; cross-domain dependencies must be explicit acyclic backend requests or a proven joint solver boundary. Do not invent a blind retry loop or translate foreign provides/conflicts into lossy Vibe strings.

Whole Arch update is a coherent repository epoch operation. Record repository order/snapshot, architecture, package bytes/digests and backend verification policy; partial upgrades are not a supported Arch strategy. Existing external trust requirements remain in force: D-011 optional Vibe receipt signatures does not mean disabling Arch package signatures/key trust. Installing a wrapper cannot self-enroll repository trust. See E35–E37.

Installation into a directory is not arbitrary relocation. Arch guest-root operations must bind root, DB, config, cache/keyring and hook execution to the intended Linux environment; pacman --root is not a general HOME installer, and mounted guest operation has distinct --sysroot semantics (E35). M-15 executes the backend in a disposable Linux build target; it does not operate on the developer's live host system.

There is no atomic transaction across a foreign package engine, Vibe activation and external services. Plan partitions effects into domains with ordered prerequisites, retained before-state where possible, journaled completion and typed recovery status. Vibe-owned profile generation can switch atomically only on a supporting filesystem and within that activation pointer; config edits/services/hooks remain separate recorded effects. Arbitrary post-install hooks are not reversible by deleting installed files. Interrupted foreign apply may require backend repair or rebuilding an offline image; it must not be labelled rolled-back/successful.

One file/resource has one physical authority. Vibe avoids pacman-owned paths unless an explicit integration contract delegates a configuration resource or adoption transfers ownership. Equal bytes never transfer ownership. Writing Vibe-owned snippets in an allowed directory can coexist with backend-owned base config; overwriting backend files with a blanket overwrite flag cannot. Preserve modified user config on update/removal, offer explicit conflict disposition, retain mutable data independently of package state. Database/service data migration is a separate operation with its own rollback truth.

Operation authority is target/effect-bound and revalidated, including current principal, mounts/root identity and environment generation. Installation can grant its complete reviewed declared operation without repeated confirmations. It does not authorize arbitrary home/root/network effects; root privilege inside a Linux builder does not grant root on the host. Existing system privilege enum is metadata, not an elevation broker. Live system support later uses explicit OS-supported elevation for the exact admitted operation; no blanket sudo, credential collection or assumption that native code is confined by a Rust type.

### 3.16 Docker/OCI agent-image build capability

**D-021 required phase result:** the same environment definition drives both user profile delivery and an agent image. ImageDefinition selects an exact base image/platform, environment lock/profile, runtime user/home/workdir, explicit packages/config, ports/volumes/service bindings and entrypoint. Source context is a bounded declared capture, not a recursive upload of HOME/repository secrets. Build executor, target image and registry destination have separate identities.

Reuse existing build/package/deploy provider and artifact DAG for a BuildKit backend; no independent general build language/second scheduler. Lower the normalized image plan to a reviewable deterministic Dockerfile/build context or equivalent supported BuildKit graph. Generated representation must stay inspectable; machine API reports exact stage/input/output identities. A hand-authored opaque Dockerfile may be an explicit trusted escape hatch but cannot claim complete Vibe package inventory/effect/reproducibility guarantees without evidence.

First executable target: linux/amd64 under local Docker Desktop Linux engine. Architecture supports other targets through explicit capability negotiation; arm64 execution/QEMU/remote builders are not silently assumed. Runtime native extensions load only on their own executor platform. Linux binaries are built/installed/metadata-verified in Linux stages, not executed by the Windows control process.

Stages separate fetch/preparation, dependency/build tools, realization, verification and minimal runtime output. Build dependencies need not ship. Prepared exact sources/base layers/package closure permit a network-disabled replay once locally available. Cache is optional; mutable tags/repository URLs are resolved and captured before replay. Missing captures give unavailable, never hidden fetch in offline mode. BuildKit platform selection, OCI exporter and cache behavior are external capabilities to probe and record, not shipped Vibe functionality (E38–E40).

Runtime image includes installed tools/libraries, chosen four-client-compatible configuration, classified Vibe package/spec resources needed for boot, active profile, machine inventory and service catalog. Actual mandatory client support remains D-012; image tests must cover all four client configuration/launch contracts as separate selectable profiles, rather than baking every client into every image. No model subscription is needed to verify configuration and local command launch; live model inference stays optional and its absence explicit. Runtime starts as a non-root target user; build-only root use is separate.

Credentials are runtime bindings or explicit BuildKit secret/SSH mounts. Do not pass secrets as build arguments, copy user configuration/token stores, or persist secret values in generated context, layers, history, logs, provenance or cache exports. Runtime needs no Docker socket, broad host HOME mount or privileged container flag. Builder daemon is trusted execution infrastructure; this is not a claim that ordinary containers sandbox hostile native packages. Minimal runtime filesystem and network permissions are explicit deployment choices.

Image evidence distinguishes definition/lock, package inventory, normalized rootfs digest, OCI image config/layers/index and execution proof. Package closure equality is weaker than byte-identical image output. Reproducibility level is declared and measured: reproducible resolution; reproducible installed files/metadata; byte-reproducible OCI only after two independent cache-disabled builds yield equal exact declared OCI subjects: config, manifest/index and layer digests/bytes (and archive bytes if that archive is the claimed subject). Normalized rootfs equality proves only installed-content/metadata reproducibility. Declared normalization never excludes unexplained runtime payload changes; volatile execution logs/attestations are identified separately. Attestations/SBOM retained in exported OCI evidence when supported; unavailable retention is visible, not silently assumed. Local artifact export, loading for tests and registry push are separate effects; push uses M-07 current-A policy.

Rootfs population does not start host services or run target binaries on the wrong architecture. Service definitions may be emitted during build; startup/health run in the resulting isolated environment. Container cloning binds image-generation evidence to a new runtime instance; build-host absolute paths and live receipt IDs are not copied as authority over the new host.

Docker update/remove/rollback uses rebuilt immutable image generations: prepare a replacement from changed locked intent, start/verify the new target, then switch the explicitly managed local deployment; retain the previous image/config/runtime binding for rollback. Removing a package rebuilds the image from the remaining dependency closure; no claim that deleting it in one writable container layer removes it from the original image. Runtime volumes/databases survive replacement and removal unless separately included in an admitted data operation; image rollback does not roll back data schema/content. GC protects live containers, retained image generations, exported artifacts and recovery references. Cleanup is restricted to exact owned resource IDs, never global Docker prune. M-15-I proves replacement, failed startup rollback, package removal/rebuild and live-reference-safe image GC.

### 3.17 AI-aware tools and network services

**Accepted D-023:** first prove the shared contract on a local isolated Docker network, retaining a future remote executor/service-binding port. AI awareness means observable, machine-described infrastructure under the owner's control; it is not autonomous discovery/administration of the surrounding network.

Package exports describe installed tool commands and services: stable identity/version/capture, purpose, input/output/error schema, operation effects, protocol, endpoint binding, auth requirements, dependencies, lifecycle/health observer and documentation/resource references. Bind endpoints at deployment time, including service identity/origin; do not bake localhost or developer addresses into package definitions. Reuse standard OpenAPI/MCP references where available rather than synthesizing a second API grammar. Unwrapped external tools get an honest basic inventory/usage record; capabilities not declared or observed stay unknown.

Installed generation automatically contributes its admitted descriptors to the machine catalog and boot/debugger graph. Consumers discover on demand through shared library/CLI/MCP; no boot quota and no requirement to inline every API schema into every session. The debugger correlates executable/image/package/service with why-included, dependencies and boot/storage/image contribution. A service has desired, installed, configured, running and healthy observations separately; container existence is not readiness.

Catalog read is pure. Explicit health/probe calls declare network effects and snapshot/TTL; a stale response is not current health. Credential references are resolved only by authorized runtime bindings for the intended service/operation. Discovery data cannot grant itself trust, request host credentials, rewrite authority policy or authorize a call. Redirect/origin and private-endpoint policy apply to network access; managed private Docker endpoints remain legitimate explicitly bound targets. Descriptive instructions and remote schemas are data, not injected control instructions.

First phase scenario: agent container plus at least two managed test services on an isolated network; generated catalog enables structured discovery, parameter validation, one authorized read and one controlled write, plus explicit denial for an ungranted effect. Include unavailable service, changed endpoint identity/schema, credential rotation and rebuild without leaked credentials. Service-specific action code remains in adapters/packages, not special branches per product in core. A daemon that autonomously chooses tasks, fleet scheduler, service mesh and Internet-wide controller are outside this phase.

### 3.18 Future Linux distribution shape and readiness boundary

Target result: an Arch-derived system definition with captured repository epoch, base userspace, kernel/initramfs/bootloader provider contracts, users/groups, filesystem/mount layout, networking, services and agent environments. Vibe is the uniform human/machine entry for composition, inspection, prepared changes and evidence. Arch backend may initially retain package ownership; Vibe supplies extra tools, boot knowledge, client adapters and service interfaces. Full backend replacement requires separate package format/DB/signature/hook/version/upgrade/recovery compatibility acceptance, not renaming a command.

M-14 reserves typed account/service/config/mount/boot resources and executor capabilities without claiming implemented OS support. M-15 concretely installs tools, populates Linux userspace, composes profiles and builds images; it must not require a perpetual running Vibe daemon. Bootstrap accepts a captured Vibe binary/distribution plus backend/base prerequisites; a minimal recovery path reads manifest/lock/inventory even if ordinary profile activation fails. Self-update protects the manager currently needed to recover, rather than overwriting its running recovery chain.

Future VMware gate covers firmware/bootloader/kernel/init, devices/network startup, login/users, reboot, interrupted full-system upgrades, rescue/rollback and writable state migrations. Docker proves userspace and image behavior only; it shares the Linux executor kernel, so no kernel/bootability claim follows (E41). No bootable ISO, disk installer, live root replacement, VMware setup or new distro release is implemented in this design pass or required to close M-15.

### 3.19 Nix as design reference; no GitHub dependency

**Accepted D-024:** взять Nix как architectural reference for exact dependency closures, isolated builds, immutable installed objects, concurrent versions, declarative profiles/generations and rollback. Это не обещание совместимости с Nix language/nixpkgs/Nix store и не обязательная зависимость от Nix runtime. Build realization recipe здесь отличается от project-local DerivationBinding D-009: первый описывает производство payload, второй — пользовательскую производную ресурса. Их provenance связывается без слияния identities.

**Recommendation:** freeze a canonical build recipe containing explicit source/dependency inputs, executor/toolchain, target ABI, environment variables, declared outputs and network policy. Its input digest is a derivation identity, not proof that outputs reproduced. Output manifest/digest and independently verified references establish realized closure; hidden ambient compiler/library/network dependencies invalidate the hermetic claim. Build providers expose execution isolation level and observed inputs. First Linux fixture builds with declared mounts/toolchain/env and no network after fetch; a provider incapable of enforcing that boundary reports non-hermetic/unavailable. HOME user mode does not pretend to provide an OS sandbox merely because files stay under HOME.

Read-only store semantics prevent routine in-place updates; corruption is diagnosed and repaired from captured bytes. A user controlling their own HOME can alter it, so content verification remains necessary. Input-addressed/cache lookup identities, output content digests and physical store paths remain separate. Prefix-bound binaries may contain their store location: destination/store-prefix compatibility is part of substitution; different prefix requires a relocatable payload or rebuild, never blind rewriting of binary strings. Do not require a privileged /nix/store to deliver the HOME scenario. See E44 for the reference model and its path constraints.

**No mandatory GitHub path anywhere in the delivered cycle.** Source resolution, transitive inputs, registry/catalog, build recipes/toolchains/bootstrap, binary substitution/cache, OCI base images, client packages, update checks and publishing all have explicit replaceable locators. Local directories/captured archives, ordinary Git over supported transports, self-hosted HTTP artifact mirrors and OCI registries provide the first alternative routes. Existing generic registry/source/provider seams should be extended, not bypassed by hardcoded github.com/API/release URLs. GitHub adapters may remain optional convenience; provenance may honestly name a GitHub origin without requiring a live request to it.

Logical package/capture identity does not equal download URL. Mirror substitution preserves exact expected content and origin provenance; different bytes require new capture/review, not fallback acceptance. Lock records exact recursive inputs, expected digests, source type and mirror policy; credentials remain host/runtime references. Native repository trust checks survive mirroring. A self-hosted binary cache may supply an exact authorized realization only after content/closure/target/recipe and producer policy validation; arbitrary cached bytes with a matching name are not trusted execution evidence. Offline missing realization may rebuild only from complete prepared inputs and declared build authority.

Required acceptance uses a local non-GitHub registry + source/artifact mirror + OCI registry/cache: bootstrap/fetch/resolve, source build and substitute, HOME install/update/remove/rollback, image build/inspect, and a prepared publication test against that local destination. The fixture independently mediates/observes egress for every exercised control process, native provider/child tool and builder/daemon workload, allowing only named local fixture endpoints and recording zero GitHub/API/raw/release attempts. Application transport counters alone are insufficient; an unobserved bypass makes the proof unavailable and cannot close acceptance. Prepared offline replay independently denies all network across the same execution boundary. Origin packages may still retain original attribution. No editing host firewall/DNS or contacting real publishers is required for this test.

Architecture principles come from documented Nix mechanisms, not the assertion that Nix inherently requires GitHub: the Nix manual also documents generic Git, tarball and local references. D-024 governs VibeVM's end-to-end independence and default experience. Nix expression evaluation/import support is separate optional future interoperability; no new general-purpose DSL is required by this programme.


### 3.20 Extreme Total Observability and IDE compatibility as a product law

**Accepted D-025/D-026:** every VibeVM capability, semantically meaningful internal structure, processing decision, state transition, action and refusal has an externally discoverable, machine-readable contract. This is Extreme Total Observability (ETO), a whole-product invariant. IDE compatibility follows from that invariant and a complete action interface; it is not satisfied by adding JSON to selected CLI commands or only to the boot debugger.

The intended external experience spans package/source/dependency trees, configuration/choice explanations, authored specifications and references, compiler stages/boot debugger, change/lifecycle/evidence, tools/providers/adapters, home generations, Linux roots, Docker images/build stages and network services. A client can navigate from an installed executable or image layer to the contributing package, chosen configuration, source capture, producer and receipt, and inspect why an action was taken or refused. This describes required capabilities; it does not choose an IDE, screens, plugin packaging, an MVP or a product roadmap for a new IDE.

**ETO closure rule:** no supported functionality may exist only in a CLI branch, TUI key script, prose log or private debug dump. No unexplained omission of internal model families. A mutation is available through its owning typed application service and authority policy; an inspection view exposes the same semantic data the engine uses. Access restrictions, unsupported targets/providers, unavailable observers, redaction, uncaptured values and incomplete scopes are explicit typed states with reasons. A known feature missing its service implementation is a delivery gap, not a contextual unsupported result that closes the gate.

ETO does not mean exposing credentials, raw process memory, filesystem handles or arbitrary setters for internal state. Publish complete semantic inspection schemas with root/principal-scoped access and explicit redaction metadata. Low-level incidental implementation layout is not the stable product contract. Debug-only views may use separately versioned provider/diagnostic epochs; they remain discoverable and inspectable and must not be hidden merely because their schema changes faster. Production state still changes only through validated domain operations.

**CapabilityCoverage register:** enumerate every existing and planned domain operation, query, diagnostic, state carrier and trace/model family. Each row binds stable namespaced capability/model ID, domain owner, input/result/error/event schemas, source authority, supported contexts/targets, expected effects, authorization, completeness semantics, observation/retention modes and executable external-coverage tests. Availability is distinct from implementation status and authorization. The registry includes legacy commands, source conversions, install/reinstall/update/uninstall/clean, binary dispatch, settings, registry/cache/version-manager operations, tree/provenance, lifecycle controls, publication and every M-01…M-15 deliverable.

Generate operation discovery and schema references from owning registries; compare public command/action registrations and model-family inventories with the coverage register in CI. An exhaustive list of names or matching counts does not prove functionality: each supported row needs a real structured read or action/result scenario. Pure presentation preferences (viewport, cursor, local panel arrangement) stay client state, while every domain value/action behind them follows ETO. This is an interface-completeness record, not a scheduler or a second requirements truth.

### 3.21 One application service boundary, many transports

Existing domain libraries remain authoritative. Extract CLI-local analysis/orchestration into typed application services where necessary; reuse source resolver, compiler/trace, solver, artifact/deploy journals and extension registry. CLI, MCP and an on-demand machine session all call those services. No second resolver/compiler/refactoring engine and no universal writable graph database.

The external path is explicit:

    trusted context binding + protocol/capability negotiation
      → discover operations/models and applicability
      → capture disk/overlay/world/target snapshots
      → query/navigate/explain/diagnose/compare
      → prepare concrete action with exact effects/preconditions
      → resolve typed input/authorization requirements
      → submit once with idempotency key
      → observe/control/reattach to the operation
      → inspect terminal/partial result, evidence and recovery
      → invalidate affected views and capture the new state

Pure library calls and structured one-shot CLI remain useful without a server. An optional on-demand API host supplies responsive multiplexed requests/events over a versioned local framed transport; initial implementation can use stdio JSON-RPC with a published framing/handshake contract. Standard transport envelope and domain schema epochs are separate. The existing MCP adapter negotiates its actual supported version/capabilities and maps to the same domain requests/results; limited legacy tools remain explicit compatibility adapters. MCP tool inventory is not the complete operation/debug catalog. Do not introduce a permanently resident daemon as a prerequisite for installing/running an environment.

Protocol initialization exchanges supported API/schema versions, model/operation capabilities, position encoding, streaming/replay/control support and trusted context bindings. A workspace manifest or caller-supplied path cannot self-authorize a new root. Context registration comes from the owning local session/server policy, then requests refer to context/target IDs. Multi-root workspaces, a user environment and a remote/container target remain separate bindings; a local Windows path is not automatically valid in the Linux target or on a future remote IDE client.

Strictly validate every input against its owned or admitted extension schema before action; wrong types and unknown fields cannot silently select defaults. Outputs, structured errors and events are validated too. Schema discovery does not execute a native provider or fetch a remote schema. Unknown extension schema version is explicitly unsupported or shown as declared generic structured data; it never enables arbitrary commands.

Suggested public API families are capabilities/models/schema, contexts/snapshots/documents, query/explain/diagnostics, action.prepare/submit, operation.get/events/control, and domain-specific operations below them. These are typed registry dispatch, not shell/eval endpoints. LSP concepts map to document synchronization, navigation, diagnostics and prepared edits; DAP may later adapt actual debugger capabilities. Neither protocol is the whole package/environment/image-management API. No LSP/DAP adapter, IDE plugin or client MVP is scheduled in this programme; their future adapters must consume the same contracts (E51).

### 3.22 Addressable models, live editing and coherent observations

Publish versioned semantic projections of resolver candidates/constraints/selected edges, compiler input/intermediate/output carriers, boot occurrences/inclusion mappings, source facts/references, artifact/provider records, operation state, effect plans/grants/receipts, cache/store/generations, image stages/layers and service/backend observations. Links compose a navigable typed graph view over domain-owned records. Querying the graph never becomes a new source of truth for authored intent or installed state.

Identity has four distinct levels: a logical entity reference, exact capture/content identity, observation/snapshot identity, and a presentation occurrence ID. Repeated inclusion of the same package/fragment has separate occurrences; display coordinate/order/line alone is not a persistent ID. Use existing stable domain identities where real; otherwise identify the object within its exact capture and state its stability. Package coordinate rename is an explicit old→new identity transition, not a claim that an FQID never changes. A code analyzer can contribute existing code-symbol identities/references without forcing minted code IDs onto every function; unsupported language semantics remain scoped and explicit.

Every view exposes schema, context/target, authority/source captures, source-location basis, completeness and diagnostics. A CompositeSnapshot is an explicit vector of compatible source/world/target/operation observations; independently changing domains are not falsely called one atomic global snapshot. If coherent composition cannot be established, return inconsistent/stale/partial rather than a plausible merged current tree. Filters, pages, children and large-value chunks bind the same snapshot/query projection; paging must eventually expose the full authorized structure, not enforce a hidden total object/boot limit.

Resource navigation uses opaque resource IDs plus captured raw/projection locations and a host-provided URI mapping. Range offsets refer to a declared encoding and exact document revision; conversion to negotiated editor positions is mechanical. Preserve non-UTF8/XML-data, generated projections and explicit source maps. Do not treat a projected Markdown line as a raw XML position or reveal unrestricted host paths to a target-bound client.

**Document overlays:** a client owns versioned open/change/close buffers with resource identity, buffer version/content digest, underlying disk capture, encoding, created/deleted/moved intent and context. A captured overlay set is immutable and client-scoped. Pure validation, references, diagnostics, configuration preview and builtin boot what-if run on one disk-plus-overlay snapshot. Invalid/partial editor syntax yields diagnostics and partial semantic availability; raw text remains visible. Overlays neither save files nor change lock/installed/profile state and do not contaminate another client's view. Native/effectful analysis is an explicit prepared action even when the input originated in an editor.

**Two explicit edit modes:** (1) analysis/code-action returns a versioned proposed edit for the owning client buffer; applying it changes only that client's working state and is not a committed package mutation; (2) service-managed multi-file/refactoring/domain apply uses saved/captured disk state. If relevant buffers are dirty or disk versions changed, return save-required/dirty-conflict/stale with the affected resources; never auto-save or overwrite. The client can explicitly save and request a fresh plan. A plan prepared against dirty buffer bytes is not admitted merely because the buffer later saved: disk preconditions are recaptured and checked. Actual source/artifact mutations use one writer/transaction owner; the editor and service must not both apply the same edit.

Service-managed apply acquires a cooperative EditBarrier for the complete affected resource set and registered participating client/buffer versions. Participants acknowledge the barrier and suspend conflicting buffer/save operations or refuse it; hold it through disk revalidation and commit. A changed version, failed acknowledgement, expired lease or disconnected participant returns conflict/unknown and prevents starting further writes; after any landed write, preserve partial/recovery truth. No assumption that a once-clean buffer remained clean merely because the disk digest stayed equal. Cleanliness is proven only for registered participating buffers, not every unrelated editor; disk CAS and post-state validation still govern other writers. Cross-client conflict metadata reveals affected authorized resources/versions, never another client's buffer contents. Barrier identity/lease and participant versions bind the operation; release/failed-owner recovery cannot silently endorse an unfinished edit.

Disk changes, package/lock updates, policy/target changes and overlay edits invalidate the relevant observations. Watchers are hints; events carry affected IDs/revisions and recovery/resnapshot requirements, not magical proof of a lossless filesystem stream. A missed watch/replay window produces gap/expired and a fresh capture. Do not automatically rerun install, native probes or publication in response to a watch event.

### 3.23 Responsive operations, errors and reconnection

Request correlation, idempotency key, durable operation ID, existing domain run/intent ID and snapshot ID are separate. Register the admitted operation and its identity before the first effect. Persist bindings to existing lifecycle/deploy/build/publication journals and retained results rather than copying those systems into a new scheduler. The current one-writer/one-parked-continuation limits remain real; concurrent client requests receive conflict where the domain cannot execute both, while read queries/control remain responsive.

Operation states include prepared, awaiting-input/authorization, accepted/running, parked, cancellation-requested, completed, failed, cancelled and recovery-required/unknown-effect. Optional domain-specific phases reference the same operation. All outcomes include typed reason, phase, subject/preconditions, completed/pending/uncertain effects, retained evidence and recovery applicability. Structured failures exist even if no final domain report was produced. A Rust/tool Err is never assumed to mean that nothing was written; current materialise_subskill is a concrete counterexample (E49).

Inputs/approvals arrive as typed responses bound to question/action plan, schema, scope and policy generation. No TTY prompts in machine mode, no parsing free prose as a confirmation. Discoverable startup defaults cannot silently remove per-operation functionality or broaden grants. The IDE client's connection, an installed descriptor, an AI suggestion and assume-yes are not authority to apply an arbitrary plan. Use D-008/M-07/M-14 target/effect authority; current publication A and future B switch remain unchanged.

Events bind operation/context/target IDs, domain phase, monotonic per-stream sequence and snapshot/evidence references; request IDs only correlate individual calls. Separate structured state/progress/diagnostics from bounded raw stdout/stderr. Interleaving multiple streams has explicit ordering domains, not a fictitious global clock. Use flow control, streaming and backpressure; event gaps are explicit. Slow consumers cannot silently corrupt operation results or make unlimited memory queues. Cancellation and query dispatch must not wait behind a long synchronous tools/call.

Response loss/reconnect/restart: repeated submit with the same key and same request/plan attaches to the existing durable outcome, never reruns effects; same key with different payload refuses. Reserve the context/target/principal-scoped key and exact request/plan binding atomically under the shared operation admission boundary before effects, so simultaneous CLI/MCP/API submissions cannot both win. Use a host-issued submission namespace with an admission epoch/expiry; the namespace is recorded by the host and cannot be invented or renewed by a caller. Keep compact key→request/operation/outcome tombstones while the namespace is live. Retired/expired namespaces are rejected even after their individual keys/results were collected; a fresh namespace requires a new explicit action, never automatic resubmission of an uncertain old one. Active, parked and recovery-required operation identities remain pinned through namespace expiry: they are inspectable/recoverable under current authority but cannot be resubmitted as new work. Large result/event payloads may expire independently of deduplication/recovery identity; inspection then reports retained status plus payload-expired. This is admission bookkeeping over existing domain journals, not a second effect/evidence authority. Reconnection reauthorizes context and can obtain operation status, retained events/cursor and terminal/partial evidence. A replay gap means resnapshot plus current status. Durable outcome survives API process exit.

Define connection-loss policy before execution and advertise actual support. Transport EOF is neither evidence of successful cancellation nor permission to orphan children/restart the action. An on-demand operation owner may continue to completion or stop at an explicit safe boundary, durably recording the result before exit; attach/control for a still-running owner uses a protected local operation channel. If the owner crashed, recovery uses the domain journal and does not invent successful completion. This short-lived ownership mechanism does not require a permanent service or add autonomous task scheduling.

Cancellation is cooperative: acknowledgement of the request is distinct from reached safe stop. Before effects it can be cancelled cleanly; after committed/native/backend effects, return exact partial outcome and recovery obligations. Provider capability advertises supported boundaries; unsupported immediate cancellation or pause is visible. No false rollback, and no unsafe forced kill as the default package-management cancellation. Diagnostic capture/view cancellation can stop observation without claiming the underlying build was cancelled.

### 3.24 Complete debug capture and semantic refactoring

ETO extends D-007 beyond the boot viewer. Every relevant domain stage provides addressable inputs, applied configuration/constraints, decisions and rejection reasons, transformation identity, outputs, provenance, timings where measured, diagnostics and effect/evidence links. For the solver this includes rejected candidates and why selected edges survive; for compiler/boot it includes actual intermediate carriers and occurrence mappings; for install/deploy it includes ownership/preconditions and partial journal; for environments/images/services it includes backend closure, runtime/image identities, health and permission decisions.

Use existing compiler_ir/trace-index snapshots and observation seams. Live record capture, retained trace inspection and a new diagnostic rerun are different operations. Opening a recorded trace must not acquire a writer role, sweep retention, recreate it or invoke providers. Absent/evicted/not-captured/redacted/snapshot-failed states are explicit; a missing snapshot does not silently become a rerun. Keep inspection after failed runs and API exit possible through retained captures/exported bundles with provenance.

Capture is configurable by declared detail/stage/filter/sink/retention policy with progress and cancellation; full-detail on-demand capture is supported where the engine has the data, including large streamed structures. Existing hardcoded 128 MiB/nine-run diagnostic retention (E50) is not a boot quota, but it cannot silently define “all internal structures exposed.” Reconcile capture policy so required detail can be retained/exported or marked unavailable with exact missing portions; no hidden completeness claim. Observer failures do not change compiler semantics or turn successful domain execution into a failure; a requested observability proof can separately fail its own completeness requirement. Exposing an internal field must not change solver choices or compilation bytes.

External package providers register their namespaced model/operation/diagnostic schemas and observation capabilities through the existing extension registry. Retained data remains structurally inspectable after removal/revocation, using captured schema/host-owned readers, without executing the revoked provider. Package descriptions do not inject arbitrary renderers/scripts into a client. A generic client can display structured nodes/properties/relations/actions; richer future clients may specialize by schema without moving domain decisions into UI code.

**Package rename full path (required backend capability, no plugin):**

1. Distinguish display-name edit, owned package-coordinate change, file/root move, specification-anchor rename and local alias. Select one explicitly; do not reuse one string-replace operation for all.
2. Capture the editable owned source package, selected consumer roots and complete affected reference universe from authoritative declarations/parsers. Include manifests/choice/dependency/provider references, declared spec links/code markers/resource exports, configured paths and environment definitions where applicable. Existing tree in-place-marker inventory is incomplete and cannot certify this universe (E46).
3. Return a typed plan of exact edits/moves, old→new identities, conflicts/case aliases, reference resolutions, generated outputs to regenerate, excluded external/history/foreign data and unknown/dynamic reference zones. Full-source scanning is declared and bounded by the selected authorized scope; ordinary Markdown/XML/opaque payload is not parsed as Vibe instructions or bulk rewritten.
4. Unknown references require explicit resolution, expanded capture or a bounded incomplete disposition. A rename claiming complete compatibility cannot close with unresolved mandatory in-scope references. Outside consumers are reported as outside scope, not silently assumed absent. Rename of an external installed package requires its own owned derivative/new identity or editing its authorized source; never mutate a foreign slot in place.
5. Validate new coordinates, target paths, dependency/address graph and generated projections in memory against exact captured versions. The caller receives reviewable changes and effects. Service apply requires the saved-disk synchronization boundary; optional buffer edits remain proposals until saved/replanned.
6. Revalidate all affected inputs/ownership/lock and target generations; apply with retained before-images and a journal, regenerate derived lock/boot/index outputs from authored intent, then re-resolve and inspect postconditions. Partial multi-file failure and recovery remain truthful; no physical atomicity promise across roots/backends. Expose action status/events and versioned diagnostics throughout.
7. Historical captures/receipts/signatures/closure evidence retain their original identities. Published registry identity changes are a separate prepared publication/migration action under D-015. Existing deployed environments are not silently retargeted: update/adopt through their own plan. An alias/redirect requires an explicit supported contract, never invented resolver fallback. D-019 future Redbook roster version offer remains applicable when the actual change touches that definition.

Other existing refactor/conversion/clean/settings operations use the same preview/apply/result contract, preserving their domain rules. General programming-language rename/move engines and wholesale PROP-032 code:// expansion are not prerequisites for Vibe package rename; their current supported providers expose capabilities and missing semantic coverage honestly.

### 3.25 Domain-by-domain IDE/ETO obligations and full-path gates

These rows revise the delivery contract of existing features; they are not a plugin feature list or an IDE MVP. Each domain retains its implementation milestone and gains a required external-access proof before its programme exit.

| Domain / owner | Externally inspectable models | Required external actions and closure proof |
|---|---|---|
| Sources/facts/requirements M-01/M-02/M-04 | Classified raw/projection resources, exact locations/proofs, relations/refinements/coverage/closure and diagnostics | Same-snapshot navigation/search/references and buffer-aware analysis; structured scaffold/answer/promote/close and exact stale/partial errors |
| Selection/install M-05 | All root requests, candidates/rejections, options/effects, selected edges/lock, visible-world and freshness reasons | Discover/select/prepare/install/update/reinstall/remove, typed NeedsSelection, reverse-dependency effects, no hidden TTY/default |
| Tree/boot M-06 | Complete package/occurrence graph, visibility provenance, stage carriers, source mapping, costs and preview scenarios | Tree/children/explain/compare/capture/export by snapshot; all text-only provenance becomes structured; no absent≡unreadable/unresolved≡unchecked |
| Lifecycle/change M-03/M-04/M-11 | Request/run/input/output/pre/post transition, parked work, evidence/closure and failure journal | Prepare/run/adopt/observe/control/reconnect/recover under actual workspace limits; repeated request never duplicates production |
| Adapters/extensions/derivatives M-08/M-09/M-10 | Schemas/capabilities/selection, grants, resource ownership, base/local/result and active content provenance | Full existing prepare/apply/update/remove/revoke/rebase and retained inspection after provider loss; unsupported target not missing interface |
| Build/distribution/publication M-07/M-12 | Artifact DAG/recipes/subjects, checks/smoke policy, prepared effects/authority and partial receipts | Full prepare/validate/authorize/apply/status/recovery; source tests/smoke choices represented; no public write from merely opening an IDE |
| Environment/tools/store M-14/M-15 | Target/executor/principal, runtime closure/payload/file ownership, generations/refs/GC candidates and compatibility | HOME apply/exec/update/remove/activate/rollback/verify/GC and full structured partial recovery; separate logical and physical roots |
| Linux/image/service M-15 | Backend closure/DB generation, rootfs metadata, image stages/layers, catalog/endpoints/auth and health | Image build/export/load/replacement/rollback; service probe/read/write/refusal; responsive operations, runtime-data boundaries and no-GitHub evidence |
| Existing administrative/refactor tools M-17-A/B | Actual command inventory, configuration/source conversion plans, cache/registry/version/tool state and complete typed reference scope | All supported domain operations exposed, including package rename; no shell/TUI-keyscript required for semantic functionality |
| Cross-cutting M-16/M-17 | Capability/schema coverage, context/overlay snapshots, typed errors/events, trace/capture availability and retention | Two independent headless clients prove all paths, strict inputs, reconnect/cancel/partial failures and equivalent authority without any IDE/plugin |

Full path is demonstrated by protocol conformance clients using the same services as CLI/MCP. Required journeys: inspect→unsaved edit→diagnose/preview without disk effects; package rename→review→saved-state apply→regeneration→navigation; admitted long action→disconnect/reconnect/cancel→truthful evidence; extension discovery/revocation→retained inspection; HOME update→Docker rebuild/replacement→authorized/denied service operation, all linked by shared provenance.

M-16 lays common contracts early. M-17-A completes initial-operation/model exposure before M-12; M-17-B delivers semantic package rename; M-17-C/D closes the full M-15-inclusive external contract. A domain implementation may finish its focused work before final cross-domain tests, but it cannot be called IDE/ETO complete by substituting a stub/unavailable result for a supported capability. A future plugin can be designed after these gates using the actual schemas; no plugin work is scheduled here.


## 4. Dependency-ordered milestone route

Stable M-01…M-15 identities remain. M-16 adds common ETO/IDE contracts early; M-17 closes exposure/refactoring/full-path proof. No IDE/plugin/MVP milestone is added. Atom dependencies prevent late API wrappers and false parent cycles.

    M-13-A → M-16-A capability/model inventory → M-13-B/C/D source/migrations
    M-13-B → M-14-A environment/target contract
    M-16-A + M-13-D + M-14-A → M-16-B shared schemas/facade
    M-16-B + M-14-A + M-13-D → M-14-B/C target-aware local interfaces
    M-16-B + M-14-C → M-16-C responsive operation/API host

    M-13-D → M-06-A source classification → M-01-A/B/C
    M-01-C + M-16-C → M-01-D/E exact external source query
    M-01-E + M-06-A + M-16-C → M-16-D overlays/snapshots/edit boundaries
    M-01-E → M-02-A; M-16-D → M-02-B…D
    M-13 + M-06-A → M-05-A; M-14-C → M-05-B…D; M-16-C → M-05-E…G
    M-13 + M-06-A + M-01 → M-08-A/B; M-16-C + M-14-C → M-08-C…E
    M-06-A + M-01 → M-06-B; M-16-D → M-06-C
    M-13 + M-14-A + M-16-B → M-07 publication evidence/A-B authority

    M-02 → M-03-A; M-16-C + M-14-C → M-03-B/C → M-11-D → M-03-D/E
    M-03 → M-04 analysis / clarification / closure
    M-01 + M-06-A → M-10-A/B/C
    M-10-C + M-08-D + M-06-C → M-10-D
    M-05-E + M-06-C → M-06-D/E
    M-05-G + M-10-D + M-06-E → M-06-F debugger
    M-01 → M-11-A/B; M-02 + M-11-B → M-11-C
    M-11-C + M-03-E + M-04 → M-11-E
    M-03 + M-04 + M-08 + M-10 + M-06-A → M-09
    initial domain exits + M-14-C + M-16-D → M-17-A complete initial API/model exposure
    initial domain exits + M-17-A → M-12 integration/docs/publication batch

    M-14-C + M-07-A + M-16-C → M-15-K Linux executor/bootstrap
    M-15-K + M-01-E → M-15-A payloads
    M-15-A + M-05-E → M-15-B environments/locks
    M-15-B + M-08-D → M-15-C/D HOME lifecycle
    M-15-K + M-15-B + M-07-A + D-022 → M-15-E Arch backend
    M-15-K/A/E → M-15-F rootfs
    M-15-F/D + M-07-B/C → M-15-G images
    M-15-G + M-08-E + M-06-C → M-15-H local services
    M-15-H + M-06-F + M-10-D → M-15-I runtime/provenance/no-GitHub proof
    M-15-I + M-12-A/B → M-15-J complete HOME/Docker phase
    M-17-A + M-15-B → M-17-B semantic package rename
    M-17-B + M-15-J → M-17-C full cross-domain conformance → M-17-D ETO/IDE closure

M-16-A consumes only M-13-A, not full M-13; M-16-B does not wait M-14-C, which consumes B. M-16-C does not wait completed source/query/change work; M-01-E enables later overlays D. M-11-D remains pulled forward. M-15-K precedes Linux payload/backend proofs. M-17-A never waits M-12/M-15, while final M-17-C/D may. Thus no artificial dependency cycle or requirement to publish before testing.

M-12 closes initial product integration and exposure; it does not claim the added HOME/Docker or full IDE/ETO programme done. M-15-J closes actual environment/image/service delivery. M-17-D closes complete external compatibility across both, including semantic rename. Current-A publication M-12-D and optional tracker M-12-C remain separate from local compatibility evidence. D-022 may be settled before M-15-E while generic design proceeds.

## 5. Detailed atoms for every milestone

Общее правило атома: применить accepted policy §12 и ETO §3.20…3.25; заморозить точный technical contract/schema; выполнить implementation/generated projections и focused positive+negative proof. Каждый domain atom регистрирует свои capabilities/models, structured errors/events/provenance и external coverage; поддержанная CLI-only функция не считается готовой. Pure query и supported context явно отличаются от effectful preparation и unimplemented gap. Повторное approval 19 предпочтений не требуется. Future live publication approvals, autonomy activation и graduation остаются explicit owner actions. Test families ниже — требуемые proofs, не заявления об уже запущенных tests.

### M-13 — Foundation, accepted source rules and migration runtime

**Goal:** Перенести принятые решения в contracts и исполняемые migration/policy primitives.
**Prerequisites:** Immutable baseline + accepted §12 + exact current tree; M-13-B after M-16-A inventory (which depends only on M-13-A).
**Scope:** Truth contradictions, Redbook/prototype/version policy, command meanings, source classification contract, executable format migration.
**Non-scope:** Live public writes/autonomy activation/graduation, переписывание history/upstream versions.
**Surfaces:** PROP-003/018/031/032/036/043/044/045/048/054, VIBEVM-SPEC, active own manifests, schema registry/codegen, small migration library + CLI/MCP plan/apply; ETO product law and model/proposal truth reconciliation.
**Compatibility:** Active own products/packages current 1.0.0; format epoch/read capability отдельны. Existing intent/comments preserved where migrated; old experimental support not perpetual.
**Risks:** R-011/R-012/R-019/R-020.
**Negative cases:** Source notes override owner, blind version replacement in fixtures/upstreams, publication auto-graduates, migration guesses selection or loses comments.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-13-A | Characterize actual consumers/call paths, exact active-version census, root/data/source policy gaps | F-002/F-004/F-007/F-008/F-021 |
| M-13-B | Update governing source rules: D-002 separation, D-013 prototype, D-019 Redbook mutable roster/current 1.0.0, no global WAL/MUP ban, no boot quotas, D-025/D-026 universal service/ETO law from M-16-A | D-002/D-005/D-006/D-013/D-019 |
| M-13-C | Versioned deterministic migration engine + request/report format: parse preserved syntax tree, plan in memory, validate, diff, typed NeedsInput, no LLM | D-003; F-020 |
| M-13-D | Executable migration apply/recovery with before-state/CAS, preservation/ambiguity/idempotence fixtures and active own 1.0.0 reconciliation; concrete SourceDescriptor converter lands with its reader in M-06-A, feature converter with M-05-B | D-003/D-014/D-019 |

**Verification/exit:** Accepted policies authoritative in product contract; migrations actually executable and tested, not advice to an agent; derived state changes only after authored source commits. No silent scope/capture/history edits.
**Safe stop:** Contract and dry-run migration report before apply; no public effects.

### M-01 — Exact source resolution

**Goal:** Safe bounded source unit через одну library/CLI/MCP.
**Prerequisites:** M-13 and M-06-A source classification; M-16-C before M-01-D so structured query/context contracts exist. Accepted D-003 migration/wire primitives.
**Scope:** Grammar, selected authority, observation/proof, classified Vibe/CommonMark/XML-data/opaque representation и resource exports, typed result.
**Non-scope:** Effective compilation, native custom semantic query, pattern expansion, revision observer, cache hydration.
**Surfaces:** vibe-spec address/resolver/embed/DocTree; vibe-specdoc pure projection; vibe-safefs readers; vibe-workspace owner views; vibe-cli/vibe-mcp; proposed SpecResolutionReport registered in schemas/formats.
**Compatibility:** Shared security hardening может отвергнуть unsafe legacy references; inventory/migration до enforcement, без fallback escape.
**Risks:** R-001/R-002/R-009/R-016.
**Negative cases:** Traversal/ADS/reserved/case/NFC; duplicate/revision; ordinary same-stem MD/XML allowed; invalid XML-data fixture raw available; no false facts; unselected/newest slot; unreadable enumeration; record+payload tamper; transformed/in-place/hardlink; bounded response/cancellation.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-01-A | Shared validated URI/path и typed lookup errors; characterization всех existing compiler/prompt call sites | F-004 |
| M-01-B | Opaque selected-source authority, visible closure и same snapshot witnesses; no unrelated newest scan | F-004/005 |
| M-01-C | Retained reader + integrity ladder; original capture index/verified builtin derivation; refusal matrix | F-005/017 |
| M-01-D | Classified builtin unit/resource/section extraction + generated response/errors; raw encoding/span basis, CLI/MCP parity | F-017/F-013/F-021 |
| M-01-E | Native/portable adversarial corpus, bounded parsing, empty-cache read-only proof, expectation checks | F-004/005/013 |

**Verification/exit:** Returned content/digests from same snapshot; no false source proof; exact error parity; no changed tree/lock/cache/.vibe; unsupported OS primitive explicit.
**Safe stop:** Source query independently useful; lifecycle untouched.

### M-02 — Read-only change binding

**Goal:** Repo-owned intent scope без scheduler.
**Prerequisites:** M-01, M-13; M-16-D before M-02-B buffer/snapshot/write contract. D-016/D-003 already accepted.
**Scope:** One origin/node, local refinements, separate intent/work identities, scaffold/check/show/coverage.
**Non-scope:** Execution, personal plan ownership, automatic closure, multi-origin execution.
**Surfaces:** vibe-facts/progress-core/vibe-requirements public read seams, small change domain library, vibe-core authored config, CLI/MCP; proposed ChangeManifest/WorkIntentGraph/change report.
**Compatibility:** Additive opt-in directory under configured specs root, immutable ID addressability.
**Risks:** R-008/R-012/R-016.
**Negative cases:** Duplicate ID, slug rename, cycles, stale origin, copied package status, unclassified/truncated facts, closure self-reference.

| Atom | Deliverable / evidence | Основание / gate |
|---|---|---|
| M-02-A | Native feat fixture + local binding characterization; prove no copied package truth/personal cursor and complete source linkage | F-001; accepted D-016 |
| M-02-B | Ratified authored manifest/graph grammar, immutable paths, intent/work hash frames | D-003/D-016; F-001 |
| M-02-C | Origin exact capture/rebase and same-snapshot fact selection; complete observation seam без page truncation | F-005/006 |
| M-02-D | New --from no-overwrite scaffold; check/show/coverage through one library and MCP query | Immutable outcome; F-001/006 |

**Verification/exit:** Two changes with same origin/local difference have distinct intent IDs; cosmetic slug stable; graph not evidence; source update typed drift; no hidden adoption.
**Safe stop:** Binding и read-only analysis полезны без lifecycle migration.

### M-03 — Scoped lifecycle and complete existing-file execution

**Goal:** Production/verification roles и existing-file edits на одном lifecycle engine.
**Prerequisites:** M-02; accepted D-002/D-003/D-018. M-14-C and M-16-C before M-03-B target/request/operation carriage. M-11-D после M-03-C и до M-03-D/E.
**Scope:** Request/invocation distinction, all carriers, workspace park conflicts, fixed full-pass composition, pre/post transition reuse, current-source verification.
**Non-scope:** Automatic repair/next-task, multi-run store, uncoordinated arbitrary writer, hidden fresh producer in ordinary/scoped verify.
**Surfaces:** vibe-lifecycle state/execution/fingerprint/agent/evidence; orchestrator application/dispatch; CLI/MCP, lifecycle/state/context/tasks/outbox/artifact wires.
**Compatibility:** Historical fingerprint readers/records remain recognizable; old park not relabelled. Ordinary and scoped public verify both prohibit new production; changed invocation law is explicit prototype migration, not a hidden legacy branch.
**Risks:** R-003/R-004/R-008.
**Negative cases:** Different change/node replay; create→verify observation identity falsely invalidates producer; post-source tests stale; outside-write edits; forced replacement; verify secretly creates.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-03-A | Freeze create/build/ordinary-and-scoped-verify roles and explicit change apply full pass; historical invocation migration; exhaustive carrier/hash map | D-002/D-018; F-002 |
| M-03-B | Thread production request vs invocation/scope across all durable/machine surfaces; evidence retains pre/post meaning | F-002/F-003 |
| M-03-C | Workspace continuation conflict/cancel/displace/recovery, exact hosted no-provider acceptance | F-003 |
| M-03-D | After M-11-D: disjoint characterization plus mandatory existing-file producer/adopt/resume/current-source verify | D-018; F-002/F-006 |
| M-03-E | CLI/MCP/full-pass parity; no new producer on verify, fresh repeated verify, exact build/test evidence and failure recovery | D-002/D-018; F-002/F-003 |

**Verification/exit:** Real existing file changed and checked; ordinary/scoped verify and build never dispatch new producer, including prerequisite slots; wrong scope/input cannot accept outputs; new source actually build/tested. Disjoint-only is insufficient.
**Safe stop:** Intermediate candidate/park retained; complete milestone closes only after transition + integrated proof.

### M-04 — Complete analysis, clarification and durable closure

**Goal:** Видеть work/evidence gaps и записывать acceptance без synthetic neutral verdict.
**Prerequisites:** M-02; M-03 for execution joins; accepted D-003 migration/wire primitives.
**Scope:** Questions/refinements, existing terminality composition, explicit policy/waiver/risk, durable closure.
**Non-scope:** Semantic regex gate, hardcoded question count, scheduler, automatic close, hidden LLM.
**Surfaces:** Existing fact/requirements/progress APIs, change library, lifecycle evidence read projection, CLI/MCP; proposed question/closure/analysis contracts.
**Compatibility:** Requirements query остаётся metadata-only.
**Risks:** R-008/R-012.
**Negative cases:** Truncated universe, absent requires, missing observer, legacy verifies count, stale question, waiver≠satisfied, cache deletion, invalid evidence producer.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-04-A | Exact-address questions, answer→local fact transaction, stale/secret-reference validation | Clarification mandate; F-001 |
| M-04-B | Complete same-snapshot join и evidence-strength model; reuse terminal evaluator | F-006 |
| M-04-C | Policy fingerprint, explicit waivers/risks, structural vs semantic checks; optional advisory rubric | F-006/014 |
| M-04-D | Closure with retained witnesses/immutable references; revalidation after state deletion | F-006 |
| M-04-E | CLI/MCP parity, manual/adopted evidence; optional one-way personal plan projection | F-001/006 |

**Verification/exit:** Specification-only terminal works; unclassified/unavailable honest; relation не successful run; closure survives erased cache as historical decision and refuses stale applicability.
**Safe stop:** Complete analysis before closure writes; closure only after retention policy.

### M-05 — Unified feature and choice configuration

**Goal:** Consumer controls semantic selection; solver handles technical satisfiability.
**Prerequisites:** M-13, M-06-A classification for effective contributions; M-14-C before M-05-B environment/target/dependency-role carriers. M-16-C before M-05-E external operations; D-003/D-004/D-005/D-017/D-019 already accepted.
**Scope:** Typed activation, optional deps/forwarded features/subskills, four group cardinalities, positive guards, requested/resolved lock, source routes, UI/update/reinstall.
**Non-scope:** Expressions/workflows, solver-selected defaults, incompatible configs of one shared instance.
**Surfaces:** vibe-core manifests/lockfile; vibe-resolver; vibe-package-source source/cells/qualified; vibe-install plan/fetch/record/visibility/apply; vibe-workspace freshness/boot; CLI/MCP; generated config report/lock.
**Compatibility:** Validated legacy lowering; default/exclusive/requires_any meanings retained; full package bytes not filtered. Unsupported solver capability explicit.
**Risks:** R-005/R-009/R-011.
**Negative cases:** Root loss, rejected-candidate leakage, weak features, wrong existing version, old unselected candidate missing option, definition drift, conditional oscillation, missing cache, dormant stale option, lost capability edge.

| Atom | Deliverable / evidence | Основание / gate |
|---|---|---|
| M-05-A | Characterize roots/Fresh/all-features/parser/conditional defects и solver capability matrix | F-007/008 |
| M-05-B | Ratify typed activation/consumer/group grammar, environment resolution domains and dependency roles from M-14; comment-preserving migration | D-003/D-017 |
| M-05-C | Immutable catalog + candidate-conditioned activation; all incoming constraints; shared solve/manifest/masked routes | F-008 |
| M-05-D | Actual dependency/capability edges, feature/subskill contributions, requested/resolved lock provenance | F-007/008 |
| M-05-E | Freshness before fast path; pure discovery; transaction/update/reinstall and reverse-closure removal seam; unattended/TTY UI | F-007/008/013 |
| M-05-F | Redbook existing 1.0.0 exact-one MUP default/WAL alternative + global/branch addons; migrate manual exclusions preserving intent/shared deps | D-004/D-005/D-019; F-009 |
| M-05-G | Supported-backend parity, all-root conflicts/recovery and positive independent WAL/MUP coexistence; Redbook-only exclusivity; no automatic bump | D-005/D-019; F-008/F-009 |

**Verification/exit:** Rejected candidate leaks no effects; compatible older version succeeds; discarded metadata never prompts; all member constraints survive; changed selection defeats Fresh; branch switch preserves shared deps; offline replay exact; unsat/NeedsSelection before mutation. Subskills admitted only after loading consumers work.
**Safe stop:** Discovery-only; then exact-one; then addons. Public republish only M-12.

### M-06 — Classified content and interactive boot debugger

**Goal:** Foreign content works correctly; debugger explains actual/hypothetical boot без навязанного бюджета.
**Prerequisites:** M-06-A after M-13; B after A+M-01; C additionally after M-16-D; D/E also after M-05-E; F after E+M-05-G+M-10-D.
**Scope:** Mandatory CommonMark/XML-data/opaque/Vibe classification; actual tree/provenance, calculator, static/dynamic preinstall delta, structured protocol and interactive UI.
**Non-scope:** Total boot quotas, canonical compaction/status stripping/externalization, new compiler/BootIR, hidden effectful preview.
**Surfaces:** core source descriptors, specdoc/spec/facts/sync/derived converters, workspace compiler/observer/boot; shared debugger library; JSON CLI/MCP; thin local visual tree/details/compare viewer.
**Compatibility:** Current canonical representation/ordering stays; classification migrates explicitly. Foreign data bytes unaffected by spec_format. Old suffix heuristics removed from all consumers.
**Risks:** R-009/R-015/R-016/R-019.
**Negative cases:** Foreign markers parsed as facts, valid/invalid XML-data, same-stem separate resources, data reserialized, false package attribution, shared-dep double count, changing snapshots/pages, large boot rejected, missing native preview mislabelled exact.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-06-A | SourceDescriptor grammar/migration + CommonMark sections/XML-data/raw handling; classification shared before scan/resolve/convert/collision/boot | D-014; F-021/F-017 |
| M-06-B | Actual captured boot graph/provenance/why-included + bytes/scalars/tokenizer metrics, occurrence/shared distinctions and no total cap | D-006/D-007; F-016 |
| M-06-C | Versioned machine requests/results/errors/progress/snapshot pages + interactive tree/detail/compare UI on shared library | D-007; F-016 |
| M-06-D | Frozen candidate-world preview protocol; explicit preparation vs pure builtin calculation, legal static/dynamic scenarios and incomplete states | D-006/D-007; F-013/F-008 |
| M-06-E | Preinstall marginal cost with transitive/shared dependencies/choices/conditions; toggle/install/remove/reconfigure comparisons | D-006; F-016 |
| M-06-F | Full UI/API/CLI/MCP parity, derivative provenance, million-token-scale no-quota scenarios, stale snapshots and honest unknown attribution | D-006/D-007/D-009/D-014 |

**Verification/exit:** Interactive debugger and machine input/output both delivered; current canonical bytes unchanged; no hidden network/provider/write in query; counts/scenarios reproducible; no budget veto. Existing-tree calculator alone is not complete preinstall debugger.
**Safe stop:** A classification first; then current snapshot inspector. Final milestone waits full preview/UI/protocol.

### M-07 — Distribution evidence and switchable publication authority

**Goal:** Exact prepared publication with default smoke and shared A/B authorization.
**Prerequisites:** M-13 accepted policy/migration foundation + M-14-A target/executor contract + M-16-B shared service/error/event schemas; actual CI/remote mutations wait concrete mode-A batch approval.
**Scope:** Existing manifest evidence, default-on explicit-skip smoke, trusted channels/optional signatures, prepared-plan authority, B implementation/tests initially disabled, delayed destructive prepare and recovery.
**Non-scope:** Automatic live B activation/graduation, mandatory crypto/full tests, skipped==passed, unproved remote atomicity.
**Surfaces:** xtask dist build/snapshot/release, publish manifests/host adapters, generated plan/authorization/receipt schemas, wrappers/CI producer, shared policy evaluator.
**Compatibility:** Own product labels current 1.0.0; captures/epochs independent. Tests/checks explicit; smoke default-on at publish preparation, skip recorded in plan.
**Risks:** R-006/R-007/R-020.
**Negative cases:** Wrong subjects, forged/replayed trust, absent/failed smoke vs explicit skip, switched policy generation, revoked/expired/out-of-bound B, stale approval, remote generation changed.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-07-A | Shared prepared plan/evidence/authorization model; A approval and B bounded policy evaluator implemented with exact digest/scope/generation | D-011/D-015; F-010/F-011 |
| M-07-B | Four native bundle smokes default-on; explicit skip parameter/scoped status, no full-tests side effect; standalone preparation reports | D-010 |
| M-07-C | Approved producer/channel validation, explicit source tests/PR pipeline, optional signature path; read-only source provenance | D-010/D-011/D-015 |
| M-07-D | Ready bundles + smoke effective policy before destructive prepare; A current approval gate, B dispatch path tested but disabled; pre-write revalidate and recovery | D-015; F-010 |
| M-07-E | A/B same-plan effect parity, out-of-policy/revocation/stale-approval refusal, explicit skip successful path, native failures and remote restoration rehearsal | D-010/D-011/D-015 |

**Verification/exit:** Mandatory suite proves default smoke passes/failed blocks/explicit skip allowed without false pass. Same exact approved plan has same effects under A/B in isolated tests. Live policy remains A; no public writes without current batch approval. Full-test absence doesn't secretly block optional-test publication.
**Safe stop:** Prepared artifacts/plan before authorization. Old release remains until authorized exact apply.

### M-08 — First-wave external adapters and four-client commissioning

**Goal:** New client via external package without core release, with complete write/config lifecycle.
**Prerequisites:** M-13, M-06-A, prepared source/proof M-01; M-14-C and M-16-C before M-08-C so grants/receipts and external operations bind environment/principal/root. Generic adapters don't depend on change runtime.
**Scope:** Legacy characterization, open client/capability map, package-authored destination/key requests, host grants/generic operations, full ownership lifecycle, four selected clients.
**Non-scope:** Grant from detection/hash, ambient arbitrary argv in data interpreter, fake sandbox, read-only catalogue as completed external-write support.
**Surfaces:** agent-projection/profiles/pkgskill; MCP config; lifecycle deploy model/native authority wire/protocol/ownership; shared extension registry; CLI/MCP prepare/plan/apply/revoke.
**Compatibility:** Existing families retain ownership/readers; explicit cross-family adoption. Compiled client fields migrate to map; legacy views remain adapters.
**Risks:** R-009/R-010/R-013/R-014.
**Negative cases:** New client needs enum patch, requested key treated grant, malformed/overlapping members, no before-image rollback claim, shared physical targets, missing/changed descriptor, revoked provider used to uninstall, unsupported mandatory client scenario.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-08-A | Legacy operation/scope/shape matrix; concrete target contracts for Claude Code/Codex/OpenCode/Qwen Code | D-012; F-012 |
| M-08-B | Explicit prepare/probe vs pure planning; bridge hydrated source-root characterization/correction | F-013/F-015 |
| M-08-C | External descriptor and open capability map + generic host operations/grants bound to M-14 target/root/principal; retained receipt/recovery/upgrade/revocation semantics | D-008; F-014/E27 |
| M-08-D | Typed command/native client operation admission; real plan/apply/verify/recover/uninstall; expanded effects require new authority | D-008; F-014 |
| M-08-E | External package fixtures for all four clients, exact settings/resources, full lifecycle proof with unchanged core binary; no lost resources/foreign state | D-008/D-012; F-012/F-015 |

**Verification/exit:** New external descriptor configures a real additional client without core enum change; four supported matrices have actual evidence. No hidden probes/hydration in query; grants exact, changes visible, revocation/cleanup safe. Qwen official docs are reference, not runtime pass.
**Safe stop:** Internal parity/plan-only are intermediate; milestone cannot close until external writes and all four client scenarios pass.

### M-09 — Exact bridges and native reuse

**Goal:** Useful journeys, honest upstream compatibility and reusable feat intent.
**Prerequisites:** Audit after M-13; bridge runtime after M-06-A/M-01/M-08-E; derivative journey after M-10-D; native journey after M-03-E/M-04.
**Scope:** Pinned Spec Kit dependency matrix; both bridge taxonomies; change/bug/assess profiles; two product feats across two stacks.
**Non-scope:** Copy upstream runtime into VibeVM, claim latest upstream, automatic scripts, workflow state machine, monorepo foreign products.
**Surfaces:** Separate bridge roots, embedded resources/composition, external native examples, change/profile integration, docs.
**Compatibility:** D-001 accepted: both bridges flow, own wrapper version 1.0.0; coordinate/upstream captured identity retained, lock kind/capability migration explicit.
**Risks:** R-003/R-008/R-010/R-014.
**Negative cases:** Research pin mismatch, missing scripts/templates/layout, executable prompt instruction, directory resource loss, lost notices, duplicated feat per stack.

| Atom | Deliverable / evidence | Основание / gate |
|---|---|---|
| M-09-A | Audit current five and clarify/checklist/analyze/converge exact resources; effects/layout/platform table | F-015 |
| M-09-B | Both bridges flow at 1.0.0; complete foreign non-Vibe resource closure, source classification/provenance/notices and consumer migration | D-001/D-014; F-015/F-021 |
| M-09-C | Native change/bug/assess profiles on existing primitives; scripts only explicit trusted action | F-001/014/015 |
| M-09-D | Two product feats × two stacks; unchanged origins, independent acceptance, drift/rebase | Reusable-feat mandate; F-015 |
| M-09-E | Required bridge/native journeys with external adapter effects, local derivatives and complete resource closure; no unsupported stub as acceptance | D-008/D-009/D-014; F-015 |

**Verification/exit:** Resources offline and byte-exact after preparation; ordinary Markdown/XML never accidentally interpreted; required script steps explicit admitted actions. Two-stack reuse uses M-03 proven existing-file transitions with current-source tests. Mandatory journey cannot close on diagnostic-only unsupported or disjoint-only demo; no upstream authorship conflation.
**Safe stop:** Audited capability matrix/local fixtures without republish.

### M-10 — General local derivatives and consumer sovereignty

**Goal:** Project owner creates/adapts resources with exact provenance and explicit update handling.
**Prerequisites:** M-13, M-06-A and M-01 initial source/proof carriers; D accepted. M-10-D additionally M-08-C/D and M-06-C.
**Scope:** Authored derivation binding/local source, full replacement/file-tree resources, resource roles, show/diff/rebase/apply, downstream references/activation separation.
**Non-scope:** Automatic merge/execute, priority/workflow language, inheritance of upstream grants/identity, sealed blocking local fork.
**Surfaces:** Shared source/derivation proof library, owning project manifests/spec resources, existing artifact cache/registry/deploy, source query and boot debugger projections.
**Compatibility:** Original capture/source slot stays exact; own derivative has new qualified logical identity. Different role doesn't silently reinterpret upstream.
**Risks:** R-002/R-010/R-018/R-019.
**Negative cases:** Sealed local fork refused, same identity reused, missing/changed base, concurrent local edits, data XML normalized, source notices lost, executable auto-runs, inherited grants, unknown third-state recovery.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-10-A | Classify required resource families/use cases and common derive lifecycle; integrate existing mechanisms without a demand-permission gate | D-009; F-019 |
| M-10-B | DerivationBinding/base/local/result/recipe/provenance contract; owner sovereignty and compatibility/activation distinction | D-009; F-005/F-019 |
| M-10-C | Generic derive/show/diff/rebase/apply for CommonMark, XML-data, Vibe specs, opaque files/trees; local source retained, no auto merge/execute | D-009/D-014 |
| M-10-D | Source query/boot debugger/adapter/artifact integration, explicit activation of code-bearing derivative under new grants, drift/recovery proof | D-008/D-009; F-005/F-019 |

**Verification/exit:** All resource families have real scenarios; sealed original can yield separately owned derivative; new identity/provenance visible; same-version base drift requires explicit rebase; passive derivation no execution; active use reauthorized.
**Safe stop:** One text/resource slice intermediate; parent remains open until generic family and downstream proofs.

### M-11 — Brownfield intake and reusable pre/post transition

**Goal:** Observe/propose/promote with honest intent boundary and integrated existing-project proof.
**Prerequisites:** A after M-01; B after A; C after B+M-02. **D is pulled forward after M-02 and M-03-C**, not after C or complete M-03. E after C+D+M-03-E+M-04.
**Scope:** Safe inventory/captures, algorithmic stack proposal, explicit promote, shared transition contract used by M-03, real brownfield composition.
**Non-scope:** Every-language inference, hidden tools, uncoordinated arbitrary writer, automatic promotion or repair.
**Surfaces:** safefs/inventory, fact/source query, stack analyzer, change/lifecycle, generated inventory/proposal/transition reports.
**Compatibility:** Pure observe no cache/state writes; old intent preserved; legacy lifecycle recipe remains distinct.
**Risks:** R-003/R-008/R-009/R-017.
**Negative cases:** Mixed-time tree, unsafe links, exclusion ignored, stale proposal, changed directory membership, lost pre-state, outside-write edit, wrong current-source tests.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-11-A | Pure inventory with witnesses/membership/capture and skipped/unstable scope, exclusions before reads | F-018 |
| M-11-B | Stack-owned algorithmic proposal and explicit optional assist, assumptions and source evidence | Brownfield mandate |
| M-11-C | Explicit promote with complete affected-set revalidation, preserve existing desired intent | D-016; F-018 |
| M-11-D | Pulled-forward general pre/post transition and source applicability: after M-03-C, before M-03-D/E; exact consumed inputs, outside-write equality, explicit adopted authority | D-018; F-002 |
| M-11-E | Real existing-project intake→promote→create→verify through completed M-03, current-source build/tests, concurrent-edit refusal | D-018; F-002/F-018 |

**Verification/exit:** D is accepted independently to unblock M-03; full parent closes only after A/B/C/E. Observations don't become intent without explicit promote; no false current-state evidence.
**Safe stop:** Inventory/proposal and pulled-forward transition are valid intermediate stops, not completion of entire brownfield scope.

### M-12 — Initial prototype integration and approved publication

**Goal:** Original accepted programme plus M-14 target readiness and M-17-A initial ETO exposure integrated, documented and ready for explicit batch publication; new environment phase closes M-15-J, universal IDE/ETO closure M-17-D.
**Prerequisites:** Required exits M-01…M-11, M-13, M-14-C and M-17-A initial external/model coverage, including M-06-F debugger, M-08-E four external adapters, M-10-D derivatives, M-11-E brownfield. Optional tracker not a dependency.
**Scope:** Local package graph, bridge kinds, own 1.0.0 census, end-user docs, registry/index checks, current mode-A publication plan; B readiness and prototype state visible.
**Non-scope:** Automatic graduation/B activation, Gemini/Copilot as blockers, changing foreign upstream versions, public write based solely on this review.
**Surfaces:** External own packages/bridges, registry/index, distribution chain, docs, optional one-way tracker provider; common authorization pipeline.
**Compatibility:** Prototype own line 1.0.0; exact captures and machine formats separate; future support commitments only after owner graduation.
**Risks:** R-006/R-007/R-011/R-014/R-020.
**Negative cases:** Required feature deferred in final report, wrong target/version, mixed capture graph, missing inert resource, stale approval, mode B enabled by default, auto public/stable declaration.

| Atom | Deliverable / evidence | Основание |
|---|---|---|
| M-12-A | Full local graph Redbook/bridges/adapters/derivatives/native examples, active own 1.0.0 and offline exact reproduction | D-001/D-008/D-009/D-012/D-019 |
| M-12-B | Docs match actual operations/machine debugger/prototype policy; no stable-release or full-test claim not supported by evidence | D-013; F-020 |
| M-12-C | If demanded: one-way tracker export using same approved-plan/idempotent/concurrency protocol | D-015; optional baseline scope |
| M-12-D | Initial-route acceptance and exact publication plan; one current-A approval, then approved external writes/receipts and declared smoke policy. Demonstrate B-ready tests without activating live B | D-010/D-011/D-015/D-019 |

**Verification/exit:** Every mandatory initial programme result implemented and proven; M-15 HOME/Docker/service scope remains separately tracked and cannot be reported delivered here; external batch either executed under exact approval or remains concrete ready-to-approve state. Prototype status doesn't permit false completeness. Smoke skip visible; no actual publisher action performed by this document update.
**Safe stop:** Initial local programme and approved-candidate publication plan before live consent; M-15 proceeds without waiting for public writes.

### M-14 — Early package-manager architecture seams

**Goal:** Existing feature/lifecycle/adapter work remains compatible with home environments, Linux roots and image execution.
**Prerequisites:** A after M-13-B; B after A+M-13-D+M-16-B shared schemas; C after B. M-13 does not depend on full M-14. M-05-B, M-03-B and M-08-C consume C; M-07-A consumes A. No dependency on M-15 delivery.
**Scope:** Concrete versioned target/environment/realization/payload/backend contracts, explicit local compatibility binding, target-root/principal/effect carriage, layout centralization, provider capabilities and future system-resource vocabulary; Nix-inspired exact builds/profiles and transport-independent supply paths.
**Non-scope:** Docker builds, rootfs population, actual pacman transactions, privileged service/account effects, live distro installation.
**Surfaces:** VIBEVM-SPEC; PROP-024/025/052/054 and source rules; core manifest/target_when/artifact/deploy/layout, lock/hash frames, lifecycle/native wires/receipts/locks, registry capabilities; CLI/MCP bindings.
**Compatibility:** Existing project layout and bare install meanings preserved. Old host-only records retain old semantics; no assumed environment ownership or target-ABI proof. New tool behavior routes to existing validated artifact engine.
**Risks:** R-021/R-022/R-025; overdesign or second generic executor.
**Negative cases:** Same project/package deployed into two roots shares receipt ID; Windows host chooses Linux payload's launcher; stale root path reused; runtime ABI mistaken for native provider ABI; old user-home receipt adopted silently.

| Atom | Deliverable / evidence | Exact prerequisites |
|---|---|---|
| M-14-A | Ratify EnvironmentDefinition/TargetBinding/ExecutionContext and platform roles; centralized layout/source-law amendment; freeze existing bare-install compatibility and future distro boundary | M-13-B |
| M-14-B | Generated schemas/hash frames for exact lock/realization/payload inventory and backend-native closure; dependency roles and target filesystem semantics; legacy readers/migration fixtures, input/output/prefix identities and mirror/substitution policy | M-14-A, M-13-D, M-16-B |
| M-14-C | Implement explicit local target binding through provider requests/plans/grants/receipts/locks, same-root alias fencing and cross-target refusal; executable environment/schema seams for M-03/M-05/M-08 | M-14-B |

**Verification/exit:** Real local compatibility flow preserves behavior; two isolated injected homes have independent authority; unsupported Linux metadata/target explicit; request/receipt replay against another root refuses. Future capabilities are typed and tested as unsupported, not fake delivered providers. M-15 can extend implementations without replacing accepted core contracts.
**Safe stop:** Target-aware local operations and reviewed future interfaces; no system or Docker mutation.

### M-15 — Package-manager delivery: HOME, Linux userspace and AI-agent images

**Goal:** One declarative package/environment model delivers a user-owned environment and reproducible Docker/OCI userspace with machine-described agent tools/services.
**Prerequisites:** K is pulled forward after M-14-C+M-07-A+M-16-C; A after K+M-01-E; B after A+M-05-E; remaining edges below. Delivery may proceed before original-publication M-12-D; final local integration consumes M-12-A/B, never waits for live publication approval.
**Scope:** Real payload installation and profile generations, exact runtime dependencies, complete install/update/remove/verify/recover/GC; Linux rootfs through isolated backend; BuildKit/OCI images; four selectable client profiles; local multi-service AI catalog and honest reproducibility/health evidence.
**Non-scope:** Full Linux distro/ISO/kernel/bootloader/VMware tests, live host root mutation, autonomous fleet orchestration, forced pacman replacement, automatic network administration, model-inference subscription requirement.
**Surfaces:** Shared source/solver/artifact/store/deploy engines, new environment consumer/library, versioned environment/image/service wires and CLI/MCP, package-defined backend/BuildKit adapters, isolated Linux fixtures and docs, M-06 debugger integration.
**Compatibility:** Workspace install stays available; environments have separate resolution/ownership domains. Vibe/pacman ownership not merged. Runtime source resources retain D-014 semantics. No rewrite of upstream versions or native signature policies.
**Risks:** R-021…R-026; root confusion, runtime dependency leakage, unsafe extraction, false atomic rollback/reproducibility, backend drift, leaked image secrets, catalog-as-authority.
**Negative cases:** HOME not writable, different ABI/libc, command collision, shared dependency removal, config drift, GC of pinned/running generation, malicious archive paths/links, backend DB drift, hook failure, host service started by build, unavailable daemon/platform, stale endpoint, forged descriptors and injected secrets.

| Atom | Deliverable / evidence | Exact prerequisites |
|---|---|---|
| M-15-K | Early Docker/Linux executor commissioning: pure availability/capability preflight, explicit scoped worker preparation, captured base/toolchain/bootstrap Vibe binary or independent bootstrap recipe, target-bound process/mount/network/principal execution and egress test boundary. No dependence on Vibe payload realization or pacman; unavailable engine blocks actual Linux proof | M-14-C, M-07-A, M-16-C |
| M-15-A | Implement payload realization over captured source/prebuilt artifacts and existing build/package providers; files/directories/safe links/modes, complete runtime closure and independent digest/metadata verification; explicit isolated-build recipe, prefix-compatible trusted cache substitution; migrate legacy bin routing | M-15-K, M-01-E |
| M-15-B | Environment manifest/lock + per-target dependency roles/backend refs; install/explain/plan/fetch through shared solver; source/prebuilt selection and ABI/prefix compatibility, recursive source/mirror lock; no coding-project/Git/GitHub requirement | M-15-A, M-05-E |
| M-15-C | User-owned store/profile generations, explicit env exec/shell activation, install/update/remove/verify; reverse-dependency/command ownership checks; two independent HOME environments with shared immutable payloads | M-15-B, M-08-D |
| M-15-D | Crash/rollback/repair/GC protocol across retained generations, mutable config/state and live references; concurrent/alias-root fencing; full home lifecycle fixture | M-15-C |
| M-15-E | External backend port; recommended A delivers Arch/pacman coexistence in disposable Linux, while B requires expanded replacement compatibility gates before closure: coherent repo snapshot, trusted packages, exact plan/DB generation, native dependency/version semantics, separate file authority and partial-failure records | M-15-K, M-15-B, M-07-A; D-022 confirmation before committing backend-specific policy |
| M-15-F | Linux rootfs population through already commissioned M-15-K executor from Windows control plane; target principal/metadata, offline-vs-live effects, root-bound hooks and independent runtime verification | M-15-K, M-15-A, M-15-E |
| M-15-G | BuildKit provider + deterministic image-plan lowering, pinned base/closure, separate build/runtime stages, OCI export + inspect + non-root smoke, credential-free image/runtime bindings and offline prepared rebuild | M-15-F, M-15-D, M-07-B/C |
| M-15-H | Package tool/service descriptor registry with structured requests/results/errors/probes, endpoint/auth bindings and automatic catalog projection; two local services + agent on isolated Docker network, authorized read/write and denied effect | M-15-G, M-08-E, M-06-C; D-023 accepted |
| M-15-I | Integrate boot debugger with environment/package/runtime/service/image provenance and independent boot/disk/download/layer measurements; four selectable client image profiles; two clean builds, offline/missing-input cases and recovery/secret tests; Docker replacement/removal/failed-start rollback and runtime-reference GC; full supply-chain fixture with independently enforced/observed egress over control/children/builders, GitHub denied and separate all-network-off replay | M-15-H, M-06-F, M-10-D |
| M-15-J | Complete HOME + Docker user journeys, machine API parity and future Linux/VM acceptance outline; reconcile docs/capability matrix and retained recovery/bootstrap route and self-hosted registry/source/cache/image walkthrough; explicit local evidence report | M-15-I, M-12-A/B |

**Verification/exit:** Both home and Docker required. One home fixture installs at least two tools sharing a runtime dependency, switches versions, preserves user config, refuses breaking removal, recovers interrupted activation and safely collects only unreferenced generations. An Arch-based linux/amd64 image contains captured packages and Vibe-managed tools, runs as the chosen ordinary user and exposes correct boot/service catalog. All four chosen client profiles have actual config/launch verification. Local network has two services and bounded operations; no model subscription or remote publish required. Two independent builds establish declared resolution/content reproducibility; byte-identical OCI claims only if actually measured. Docker additionally proves image replacement, failed-start rollback, package removal/rebuild and safe image GC while retaining runtime data. No-GitHub fixture covers bootstrap, source build, trusted substitution, home lifecycle, image creation and local registry publication test with independently observed zero GitHub attempts across control/native children/builders; unobserved paths fail the proof. Rootfs/image proof does not imply Linux bootability. Required proof failures remain blockers, not skipped success.
**Safe stop:** Home completion, prepared image, and local runtime proof are intermediate accepted slices; parent remains open until both modes and catalog pass. Daemon unavailability is an environment blocker for affected runtime atoms only. Public image push is a separate prepared batch under current A; it is not needed to prove local image creation.


### M-16 — Early universal service and observation contracts

**Goal:** Enforce ETO and external-client compatibility while domain features are designed, before an IDE/plugin is considered.
**Prerequisites:** A after M-13-A; M-13-B consumes A. B after A+M-13-D+M-14-A; M-14-B consumes B. C after B+M-14-C. D after C+M-06-A+M-01-E. These are atom-level edges, not a dependency on complete M-13/M-14/M-01.
**Scope:** Complete operation/model inventory, strict generated schemas, typed application facade, capability/context discovery, responsive operation/event/result controls, overlay/snapshot/location contract and conformance clients.
**Non-scope:** IDE/plugin/MVP, screen design, actual LSP/DAP adapter, generic workflow scheduler, multi-park engine, universal writable graph DB.
**Surfaces:** PROP-018/031/032/036/037/054 source reconciliation; CLI/MCP adapters, operation/schema registries, domain request/result/error/event frames, context/resource/overlay library and on-demand API host; existing journals/trace/read seams.
**Compatibility:** Existing one-shot CLI and MCP legacy calls retain explicit adapters; negotiated new capabilities never inherit hidden old restrictions as full coverage. Current local/workspace execution limits and publication authority stay. No permanent daemon required for runtime environments.
**Risks:** R-028…R-032; hidden CLI authority, scope escalation, false snapshot, duplicate/unknown effects, observational backpressure.
**Negative cases:** Supported operation omitted from catalog; invalid input defaulted; response loss re-executes action; root replay; slow consumer blocks cancellation; stale/foreign buffer mixed into analysis; API EOF labelled rollback; unavailable stub passes coverage.

| Atom | Deliverable / evidence | Exact prerequisites |
|---|---|---|
| M-16-A | Whole-product capability/model coverage census, ETO source-law decisions and ownership map; reconcile PROP-032 proposal with real domain authorities and current public commands | M-13-A |
| M-16-B | Versioned registry/facade/context/entity/snapshot/location/error/event contracts and generated validation; strict schema/effect/applicability discovery, local framed API protocol negotiation | M-16-A, M-13-D, M-14-A |
| M-16-C | Responsive on-demand host/CLI/MCP service adapters; atomic scoped submission reservation, host-issued namespace epochs/expiry and compact dedup tombstones, pinned active/recovery identities; domain-journal linkage, progress/replay/control/attach, typed input/admission and partial outcomes; existing execution limits preserved | M-16-B, M-14-C |
| M-16-D | Client-scoped versioned overlays and same-capture pure analysis/navigation/preview, source-position conversion, watched invalidation/resnapshot, two edit modes, acknowledged cooperative edit barrier through commit and saved-state mutation boundary | M-16-C, M-06-A, M-01-E |

**Verification/exit:** Actual local domain operation through CLI/canonical protocol/MCP has equal effects and structured result. Malformed inputs fail before effects; an injected post-effect failure reports its partial truth. Long operation permits queries/cancellation, reconnect does not duplicate it, old epoch incompatibility is explicit. Two headless clients have independent dirty buffers and compare same-snapshot diagnostics without file changes; stale/save-required applies refuse. A second client editing between check/commit or disconnecting during its barrier cannot pass cleanliness; concurrent same-key submits and retries after result eviction/namespace expiry cannot duplicate effects. Test service is a protocol client, not an IDE implementation.
**Safe stop:** A inventory/source law, B contracts and C basic host are usable intermediate boundaries. D closes common editing semantics; future domain capabilities still need their own real coverage.

### M-17 — Complete external model/action coverage and semantic package rename

**Goal:** All supported VibeVM functionality and debug models are externally usable end-to-end, including package rename and HOME/Docker/service workflows.
**Prerequisites:** A after initial domain exits M-01-E/M-03-E/M-04-E/M-05-G/M-06-F/M-07-E/M-08-E/M-09-E/M-10-D/M-11-E and M-14-C/M-16-D; M-12-A consumes A. B after A+M-15-B. C after B+M-15-J. D after C. No dependency on a plugin or on live publication M-12-D.
**Scope:** Extract remaining CLI/TUI-only domain seams, publish full semantic tree/trace/internal-model views, migrate legacy errors/arguments, semantic package rename library/operations, extension observation contracts, whole-product conformance and portable API documentation.
**Non-scope:** GUI/IDE/plugin planning or implementation; all-language code-refactor engine; silent foreign/history/published-identity edits; public publishing for the sake of an integration test.
**Surfaces:** tree builder/model/provenance, refactor/source/manifest/reference consumers, tool/admin/registry/cache/settings/version commands, all domain services/MCP/CLI adapters, compiler trace/read model and runtime/image/service inspectors, schema/coverage registry and source governing docs.
**Compatibility:** Old tree JSON and command forms are versioned adapters; provenance/unknown-resolution state appears in new semantic models without silent old-schema changes. Rename records identity transitions; old captures/receipts unchanged. Trace capture policy revised explicitly; domain result unchanged by observer health.
**Risks:** R-028…R-033, R-021/R-025; partial census mistaken for complete model, raw-memory ABI freeze, unsafe rename, discarded evidence, effect duplication.
**Negative cases:** Tree false≡unchecked, read error≡absence, missing occurrence/provenance, unbounded event buffering, capture silently capped/evicted, unknown reference rewritten, unsaved file overwritten, alias-root/case collision, updated image erases data, revoked provider required for inspection.

| Atom | Deliverable / evidence | Exact prerequisites |
|---|---|---|
| M-17-A | Complete initial-operation/model coverage: shared tree/reference/provenance and legacy/admin services, strict requests and partial-effect errors, externally readable stage snapshots, configurable capture/retention/export and captured extension schemas; real headless coverage for every supported initial row | Initial domain exits listed above, M-14-C, M-16-D |
| M-17-B | Owned package-coordinate rename prepare/apply/status/recover: exact affected references/edits/moves/identity transitions, dirty/save boundary, collisions/unknowns, authored→generated regeneration and re-resolution; include environment definitions, preserve history/foreign sources | M-17-A, M-15-B |
| M-17-C | Two-client full-path conformance across all initial + M-15 operations: inspect/unsaved edit/preview, rename/regenerate, long jobs/reconnect/cancel/partial failure, extension revoke/read, HOME update + Docker replacement + service grants | M-17-B, M-15-J |
| M-17-D | Close coverage census with exact request/result/error/event/model examples and generated schema checks; publish local developer contract documentation and capability compatibility matrix; no implemented supported feature lacks an external path | M-17-C |

**Verification/exit:** Every supported capability/model row has an actual external scenario; no supported feature closes on shell scripting, TUI keys, a prose-only error or a stub unavailable. Full structure traversal reaches all authorized nodes; captures/overlays/positions are consistent. Rename updates known owned reference scope, preserves immutable/foreign records and reports unknown/external scope; concurrent modification and partial apply have honest recovery. Headless clients can explain package→boot→artifact→environment/image/service provenance after process exit. Current publication A, no-GitHub and no-boot-quota rules survive. ETO/IDE architectural compatibility complete; no plugin/MVP/IDE has been planned or built.
**Safe stop:** M-17-A closes initial exposure before M-12. M-17-B rename and C cross-domain proofs follow new environment contracts. Whole external-compatibility completion is only M-17-D; independent future UI work starts from these verified contracts.


## 6. Compatibility and migration strategy

- D-003 means implemented migrations, not prose for an agent: strict source version detection → syntax-preserving parse → deterministic in-memory transform chain → validation → diff → explicit apply with CAS/recovery. Preserve choices/comments/user values; ambiguity returns typed NeedsInput before writes; no LLM.
- Legacy files without an epoch use one recognized frozen old grammar, not guessed shape. Product/package 1.0.0 and machine format epochs are independent; old 1.0.0 binaries may lack new reader capabilities. New schema/capability handshake, not SemVer alone, protects them.
- Machine contracts start JTD+registry+corpus+codegen. EPOCHS flags change only through governing owner/product protocol. Current prototype has no perpetual old-epoch support obligation, but transitions preserve identified user data and refuse unknown state.
- SourceDescriptor migration covers project/bridge default modes, explicit Vibe source lists, ordinary CommonMark exports, XML-data/opaque files and collision identity. Never bulk parse foreign content as Vibe or rewrite XML-data through a spec converter.
- D-002 public operations: historical inclusive producer behavior migrates explicitly; ordinary/scoped verify cannot create new producer work. Old parks finish with compatible reader or explicit abandonment; no relabelled source/evidence.
- Consumer selections stored authored, lock derived. All incoming edges retained; default changes never silently reselect. Redbook continuity default MUP, WAL alternative; no global package conflict.
- D-019 supersedes Redbook automatic edition bump. Current variants and both bridge-kind changes remain 1.0.0. Active own-version census includes owned manifests/constraints/docs, not third-party dependencies, upstream pins, historical receipts/test data or machine epochs. Future Redbook overwrite offers keep/new version; old captures retained.
- External adapters migrate closed client fields to open capabilities without losing old receipts. Equal target bytes not adoption; expanded permissions need new grant. Missing/revoked package can be inspected/removed through retained host-owned operation data.
- Derivatives have new identity, preserved origin and explicit rebase. Sealed compatibility declarations cannot prohibit owner's separate resource. Execution/deploy reauthorized separately.
- D-013 current mode prototype; publication, version label, elapsed time or usage does not graduate it. D-015 current A; B code/policy tests available, live enablement remains owner-only.

- M-14 introduces target/executor/platform/root/principal/generation fields before changed selection/lifecycle/adapter consumers. Old local receipts are explicitly bound or remain legacy; never replayed into another HOME/rootfs by rewriting a path.
- Source-package shippable tree semantics stay; runtime payload manifest describes build/prebuilt outputs. Legacy binary CLI migrates to shared validation. Tool runtime, provider ABI and compiler output target are distinct.
- New environment consumers can exist without a Git/coding project; existing workspace manifests/layout and bare install contract remain. Central layout module owns new paths.
- Foreign backend version/DB/file/signature semantics stay native; exact mirror captures preserve origin. D-024 changes no foreign authorship or package hashes. Nix-like design does not require Nix language/runtime or /nix/store.

- D-025/D-026 makes service/model exposure part of every feature, including legacy CLI-local code. Keep old tree/MCP epochs as explicit adapters; new fields/states negotiate a new schema. Prototype version 1.0.0 does not prove client reader compatibility.
- Extract shared analysis/actions without changing domain authority; PROP-032 universal graph and rename-stable identity claims are reconciled with actual FQID/capture/occurrence semantics. A composed read model never replaces authored manifests or package/evidence stores.
- Uniform strict validation replaces silent wrong-type defaults; text-only post-effect failures migrate to phase/partial/unknown/recovery envelopes. Domain journals remain authority.
- Client overlays are disposable unsaved working state, not package/lock/installed truth. Saved-state CAS and identity transitions govern actual refactoring; old captures/receipts and foreign resources retain identity.
- Existing trace storage policy migrates separately from boot-size policy. Observer completeness/retention is visible; query never opens a writer or recreates an absent trace.

## 7. Determinism, offline, security and authority rules

| Surface | Offline/unattended behavior | Authority / effects |
|---|---|---|
| spec/resource resolve | Captured classified source only; exact proof or honest unavailable | Trusted selected context, no writes/fetch |
| choices query | Cached frozen catalog; missing unavailable | Never chooses/persists silently |
| install/reconfigure | Explicit requests or explicit allow-defaults; otherwise NeedsSelection, no prompt JSON/non-TTY | Full solve/trust/plan before project mutation |
| reinstall/update | Exact captures; preserve chosen intent; definition drift explicit | Same-version never sole identity |
| change read/analysis | Complete captured observations or explicit incomplete | No provider/adoption/write |
| migrate/scaffold/answer/promote/close | Explicit operation, preconditions and source epoch | Preserve source values/ownership, CAS/recovery |
| scoped create/verify | Offline propagated; hosted no provider; verify only reused/completed producer | Exact request/transition, no hidden repair |
| adapter inspect/plan | Prepared descriptor/capability observations | Requested paths/keys aren't grants; no probes/hydration |
| adapter prepare/apply | Explicit allowed probes/effects; unattended supplies exact grant/policy | Existing engine receipts/locks, no arbitrary data-script execution |
| derivative read/derive/rebase | Retained base + local bytes; missing base unavailable | New identity; no execution from derivation |
| brownfield observe/propose | No tools/network unless explicit analyzer/assist operation | Exclusions and capture; promotion separate |
| release prepare/preflight | Local plan offline; external source/probe explicit | Default smoke or explicit skipped policy; trusted subject |
| release apply | Current A approval; future B only enabled bounded policy | Same plan digest, pre-write policy/target recheck |
| debugger inspect/explain/compare | Existing snapshots, chunked/paged across same ID | No total boot quota, no hidden compiler/provider |
| debugger preview | Pure builtin hypothetical calculation over captures; unknown stages honest | Exact graph from same resolver; effectful preparation separate |
| environment inspect/plan | Captured catalogs/locks/installed inventory; no hidden native probe | Target binding and expected generation supplied by trusted surface |
| environment apply/update/remove | Exact runtime closure, native backend constraints; no silent latest solve | Complete target-bound effects; shared dependencies/config/data preserved |
| profile activate/rollback/GC | Retained generations, live-use/recovery roots; no network needed | Owned store/profile only; atomic pointer claim scoped to actual backend |
| image prepare/build | Explicit fetch; replay uses complete captures with network off | Linux executor, explicit mounts/principals; target hooks cannot act on host |
| image export/load/push | Distinct operations and exact image subjects | Local tests separate; public push current A |
| service catalog/probe/call | Pure descriptors; explicit bound probe/call can be offline-unavailable | Discovery not trust/grant; runtime credentials only intended target |
| mirror/cache/substitute | Exact recursive capture/closure/target/prefix/producer checks | No obligatory GitHub; missing offline input explicit, no fallback trust |
| capability/model/schema discovery | Captured registered schemas and actual applicability | No native load/probe/network/permission grant |
| overlay/query/diagnostics | Client-local versioned buffers plus compatible disk capture | Pure analysis; no autosave, installed-state change or cross-client leakage |
| action prepare/apply | Exact saved-state plan and typed input; dirty/stale explicitly refused | One transaction owner, target/principal/policy revalidated |
| operation events/attach/control | Retained domain status/cursors; gap/expired/unknown explicit | Reauthorize context; cancellation not automatic rollback; retry not duplicate submit |
| internal trace inspect/capture | Recorded snapshots pure; new capture/export explicit | No writer-open/retention sweep/rerun hidden in reads; redaction/missing portions typed |
| package rename | Complete scoped typed references and exact edits/regeneration | Owned source only; saved-state preconditions, journal, immutable history and publication kept separate |

Domain-separated hashes bind meaningful identities, recipes/interpretations and ordered semantics; counts use integer-safe wire forms. Large boot handled by streaming/paging rather than rejecting customer budget. Parser/platform failures and cancellation are explicit technical outcomes, never a product token quota or guessed empty result.

Foreign data may contain active instructions; inert parsing is not LLM trust admission. Native providers not OS-sandboxed by typed reply validation. Source credentials never copied into outputs/argv/receipts. Grants bind exact prepared effects and policy generation; future autonomy does not waive explicit Redbook version choice.

## 8. CLI/library/MCP and machine contracts

Names below are implementation proposals; accepted behavior governs them. All surfaces use one library result.

| Capability | CLI / machine input | Library/MCP and output |
|---|---|---|
| Source/resource | spec resolve, expectations and chunk request | Classified unit/section/raw resource, proof/encoding/spans/errors |
| Configuration | choices query, install/reconfigure/update | Captured catalog, requested/resolved effects and graph |
| Change read/write | change check/show/coverage; new/answer/promote/close | Snapshot/complete observations; explicit mutation preconditions |
| Production/verification | create --change; build; verify --change | Existing engine with explicit operation policy/request/transition; ordinary/scoped verify no new producer |
| Full pass | change apply explicit fixed composition | Production once + current-source verification; park/failure explicit |
| Migration | migrate plan/apply with source/target epoch | Deterministic diff, NeedsInput, safe apply/recovery |
| Boot debugger | JSON request stream/file or equivalent structured CLI input | Versioned inspect/tree/explain/cost/preview/compare/cancel/progress, snapshot IDs |
| Interactive debugger | Tree/detail/source/compare frontend | Same library/protocol, suitable for independent IDE client |
| External adapter | prepare/plan/apply/update/revoke/remove | Requested capabilities, exact grants, receipts and outcomes |
| Derivatives | derive/show/diff/rebase/apply | Owner identity/base/local/result provenance and activation status |
| Publication | prepare/authorize/apply/status/recover | Exact prepared batch; A approval or future B policy decision; smoke status |
| Brownfield | intake observe/propose/promote | Inventory/proposal/promotion witnesses |
| Environments/tools | env inspect/plan/apply/update/remove/verify/exec; tool install --environment | Definition/lock/target/realization/inventory/owned effects/recovery; exact environment ID and generation |
| Profiles/store | generation list/activate/rollback; store gc plan/apply | Retained roots/live leases, command conflicts, mutable-state distinctions |
| Image build | image plan/build/inspect/export/load/push | Same artifact DAG, BuildKit operations, platform/closure/OCI evidence and typed progress |
| Services | service list/describe/probe/call | Tool/API descriptors, bound endpoints, schemas/effects/auth scope, health completeness |
| Supply-chain portability | mirror prepare/export/import; realization substitute | Exact captured inputs, alternate locators, cache trust and no-GitHub diagnostics |
| ETO discovery | capabilities/models/schema, canonical API call and negotiated session | Whole-product operation/model coverage, schema/effects/applicability/availability/capture capabilities |
| Context/working state | context register/select; snapshot capture/get; document open/change/close | Trusted root/target binding, per-client overlays, raw/projection positions and coherent snapshot vectors |
| Generic action/control | action prepare/submit; operation get/events/control/attach | Typed plan/input/authority, durable idempotency, responsive progress/cancel/reconnect and partial recovery |
| Semantic refactoring | package rename prepare/apply/status/recover | Exact typed scope/edits/identity transition, versioned diagnostics, journal/regeneration/postconditions |
| Internal observations | model children/query/explain; trace inspect/capture/export | Versioned stage structures, provenance/completeness, chunked values and retained extension schemas |

Owned machine surfaces use strict versioned JTD plus generated types/corpora. Carrier map includes authored config, normalized domain, request/result/error/event, durable plan/receipt, outbox/resume, hash frame, artifact/evidence and CLI/MCP. UI never scrapes human sentences. Unsupported operation/epoch/field refuses before effects.

Debugger exposes machine **input and output**, not only --json output. Every event binds request_id + snapshot/operation context + sequence; terminal status explicit. Pagination bound to snapshot/filter, stale cursors refused. Limits on response payload do not truncate total boot silently. Dynamic cost scenarios expose conditions; unsupported exact native attribution remains typed unknown.

No second evidence authority or general workflow interpreter: API operation envelopes reference existing domain records/journals. Publication authorization API is not autonomous task selection.

ETO full path (§3.21) is normative for every row, not only debugger calls: strict schemas, context/snapshot/overlay identity, typed pre/post-effect errors, controls/reconnect and generated capability/model coverage. A negotiated canonical machine session and one-shot calls expose the same semantic operations; MCP maps those capabilities without inheriting its current limited tool grammar as the product boundary. LSP/DAP mapping is future adapter work, not a new domain schema or planned IDE MVP.

## 9. Verification matrix and exit gates

Это required implementation evidence, не tests, выполненные данным review pass.

| Family | Required proof |
|---|---|
| Migrations | Real old→new code path; exact choices/comments preserved; ambiguous input before write; dry-run pure; CAS/rollback/idempotence; no LLM |
| Classification | Foreign Vibe-looking Markdown inert; CommonMark sections; XML-data valid/invalid/other encodings retained; same-stem resource coexistence; scan/sync/resolve/convert share classification |
| Source | Portable URI/path corpus, duplicate exports/anchors, selected member/capture, wrong version/revision, unreadable vs absent, snapshot/projection basis |
| Integrity | Record-only/payload-only/coordinated tamper; source membership; builtin materialization vs local-derivation provenance; hardlink/in-place honest result |
| Purity | Empty-cache inspect/list/plan/explain calls no hydrate/probe/network/native/write; explicit prepare separated and effects declared |
| Configuration | Rejected candidate effects disappear; older compatible candidate works; all consumer edges preserved; weak features/cardinality/guards; request defeats Fresh |
| Configuration recovery | Unsat/NeedsSelection before mutation; exact owned recovery prefixes; hooks partial truth; shared deps retained; independent WAL/MUP coexist, Redbook one selection |
| Change | Stable ID/slug, separate intent/work/production request/invocation, exact origin drift, no personal cursor/source copies |
| Public/scoped lifecycle | Ordinary build/verify and their prerequisite slots never dispatch new production; scoped workspace park/replay guards; producer request stable across create→verify |
| Transition | Pre witnesses retained, outside-write stable, owned post exact, new/deleted paths covered; existing code changes; new source build/tested; fresh verify no recreation |
| Full pass | Explicit fixed production→verification composition, hosted park/resume, failure stops without auto repair; no workflow-language cursor |
| Analysis/closure | Complete universe, unclassified/absent requires, unavailable vs missing, counts not test result, waiver/risk visible, cache deletion survival/stale applicability |
| External adapters | New client from package with same core binary; open capability map; requested new path/key requires actual grant; exact four-client install/update/remove matrix |
| Adapter recovery | Physical collisions/logical key ownership, before-state or honest non-reversible, descriptor disappearance, revoked cleanup no old native invoke, cross-family adoption |
| Derivatives | All resource interpretations/files/trees, sealed source local fork allowed, new identity/notices/provenance, no auto execution/grant inheritance, base drift/rebase conflict |
| Debugger API/UI | Structured input and output, typed progress/errors, snapshot-bound pages/IDs; tree/source/why/compare UI agrees with library/CLI/MCP |
| Debugger cost | Actual vs unique/inclusive/marginal separation, shared/transitive deps/choices/conditions, static/dynamic legality, no total boot/token quota, million-token scenario |
| Debugger preview | Same resolver/compiler on captures; exact pure builtin result; effectful/unavailable stage explicitly partial, no guessed zero or proportionate fake attribution |
| Brownfield | Capture/moving-tree exclusions, no tools in observe, algorithmic proposal, stale promote refuse, real existing-project verification |
| Distribution | Exact source/target/bundle; default smoke pass/fail and explicit skip; skipped not passed; full tests opt-in; trusted channel optional signature |
| Publication A/B | Same plan effects under approved A and enabled bounded B in isolated tests; B disabled live; out-of-scope/revocation/generation/stale-plan before-write refusal |
| Publication recovery | Ready artifacts and smoke policy before delete/tag move; remote interference/partial failure explicit; old assets retained for real restoration |
| Version/prototype | Active own 1.0.0, no rewriting upstream/history/epochs; Redbook current no-bump; future version offer; no automatic graduation/B enablement |
| Ecosystem | Both bridges flow 1.0.0, complete foreign resources/actions, two unchanged native feats across stacks, four chosen clients; secondary clients not blockers |
| Target environments | Same project/package in two homes has distinct receipt/root authority; alias roots collide physically; Windows executor never installs Windows launcher in Linux target; target ABI/libc/principal replay refusal |
| Payload/closure | Recorded source/prebuilt realization, explicit runtime dependencies and modes/links; corrupt archive/content, path escapes, unsupported metadata/relocation fail; no debug-first unrecorded runtime fallback |
| HOME manager | No Git/repo/root privilege required; two tools share dependency; install/update/remove/verify/exec, command conflicts and modified config; independent profiles/versions coexist |
| Activation/recovery/GC | Interrupted stage/switch/config effects preserve truthful prior/partial state; running/pinned generations and shared dependencies not collected; uninstall retains mutable data |
| Arch backend | Native epoch/pkgrel, coherent repository snapshot, package verification and DB generation preserved; Vibe/pacman ownership distinct, unsat/drift before write; hook failure not fake rollback |
| Linux rootfs | POSIX mode/links/principals survive; host roots/services untouched; wrong-platform execution refused; offline population does not start services |
| Docker/OCI | Real linux/amd64 image/non-root runtime; pinned base/closure; four selectable client config/launch cases; no Docker socket/HOME-secret leakage; export/attestation retention observed; image replacement, package removal/rebuild, failed-start rollback and live-container-safe GC; volumes/data retained |
| Reproducibility | Two independent cache-disabled builds compare declared inventory/rootfs/OCI levels; all-input replay network off; missing captures explicit; container tests do not claim Linux kernel/boot correctness |
| AI-aware services | Agent + two local services; generated catalog, typed discovery/read/controlled write/refusal, offline/unhealthy/stale endpoint/schema/credential rotation |
| No-GitHub supply chain | Local independent registry/source/mirror/binary cache/OCI hosts; bootstrap/fetch/source build/substitute/home lifecycle/image/local publication test; independent egress mediation/observation covers control/native children/BuildKit/daemon and permits only local endpoints, zero GitHub attempts; unobserved bypass fails; all-network-off replay separate |
| Complete ETO coverage | Every supported command/action/internal semantic model has a registered external path and real scenario; CLI/TUI-only/prose-only/missing/stub rows fail; names/counts alone insufficient |
| Semantic model | Full structured visibility/inclusion/provenance, repeated occurrences, source/capture/observation distinctions; missing/unreadable/unchecked/unresolved distinct; complete snapshot-bound paging without total boot cap |
| Editor snapshots | Two clients with different dirty buffers, Unicode/raw XML positions, partial syntax, new/delete/move buffers and external disk drift; analysis/boot preview pure and coherent; dirty/save-required mutation refused; other client edits between check/commit or disconnects during acknowledged barrier, no buffer-content leakage |
| Operation protocol | Strict negotiation/inputs/results/errors/events; long action allows status/cancel queries; request ID distinct from operation/idempotency; lost response/reconnect/restart does not repeat admitted effects; concurrent same-key cross-adapter submissions, result eviction, namespace expiry and pinned active/uncertain states |
| Partial effects/control | Inject failure after first write and expose exact partial/unknown/recovery result; cancellation before/after commit boundary, disconnect owner crash and event replay gap all truthful |
| Semantic package rename | Owned declaration plus typed manifest/spec/provider/path/environment refs, collision/ambiguous/unknown/dirty cases, exact preview/apply/regenerate/re-resolve; foreign/history/deployed identities preserved |
| Internal capture | Actual stage/carrier snapshots across domains, selected/rejected reasons, failure inspection after exit, explicit missing/redacted/evicted data, configurable detail/export; observer failure does not change domain result |
| Extension observability | New package operation/model schema discoverable through unchanged generic service contract; no arbitrary renderer/eval; revoke/remove then inspect retained schema/evidence without running provider |
| Full IDE-compatible journey | Protocol clients navigate package→boot→artifact→HOME/image/service, perform an admitted change and reconnect/control/recover; initial gate M-17-A before M-12 and full gate M-17-D after M-15; no plugin implementation |

Per-atom focused tests/checks and source review; full panel at coherent/final integration boundaries. Counterfactual mutation only bounded diagnostic when specific evidence suspicious. Cross-compilation alone not native execution. Human manual sign-off not synthesized.

Intermediate stops remain useful but parent closure requires every mandatory descendant and complete scope. No required CommonMark/debugger/adapter/derivative/existing-file, HOME/Docker/service/no-GitHub or ETO/IDE-service/semantic-rename branch can be marked deferred to declare success. Plugin/IDE/MVP remains explicitly outside this programme.

## 10. Rollback/safe-stop points

- M-13: contract/migration plan before apply; exact before-state preserved.
- M-06-A: classified source migration independently gated before resolver/bridge consumers.
- M-01: safe query, no persistent effect on refusal.
- M-02: authored binding/scaffold; no execution required for read-only value.
- M-03/M-11-D: intermediate candidate/park retained; complete acceptance includes existing-file transition/current-source proof.
- M-04: analysis before closure; retained accepted proof survives cache cleanup.
- M-05: discovery/solve before apply; recover owned prefixes, not arbitrary hook side effects.
- M-06-B/C: current debugger before marginal preview; complete M-06 waits machine/UI/what-if integration. Current canonical bytes unchanged.
- M-07: prepared bundles/plan before authorization; default smoke or explicit skip visible. A is active; B tested without live activation.
- M-08: parity/plan are intermediate; final scope includes external writes/four clients. Receipts permit inspection after missing/revoked descriptor.
- M-09: local bridge/native fixtures before publication.
- M-10: local derivative source/provenance retained; rejected rebase never destroys prior accepted version; no silent upstream mutation.
- M-11: observe/propose before promote; complete brownfield gate after new-source verification.
- M-12: initial integration including M-17-A external coverage and concrete external batch ready for current-A approval; M-15 and full M-17 remain required.
- M-14: local target compatibility/schema gates before downstream consumers; future providers fail explicitly until implemented.
- M-15: staged payload/profile/home/rootfs/image/catalog proofs; keep previous generation and journal on failure. Both HOME and Docker required before parent closes; public push and full OS/VM deployment separate.

- M-16: inventory and schemas before consuming features; pure overlay working state can be discarded without saving; durable operation outcomes remain domain-bound across reconnect.
- M-17: initial model/action exposure before M-12; rename before cross-domain conformance; complete ETO/IDE closure only after all M-15 integrations, with no plugin work.

Multi-file/remote updates have journaled prefixes, not an unsupported atomicity promise. Unknown third state is refused. Recovery scope/authority are explicit; live B/graduate choices remain owner-only.

## 11. Risks and design forks

| Risk | Accepted direction / remaining engineering obligation |
|---|---|
| R-001/R-002 | Truthful source/derivation proof and safe paths; weaker observation explicit |
| R-003 | Pre/post transition in first complete executable scope, no producer invocation from scoped verify |
| R-004 | One workspace continuation; multi-run separate future design |
| R-005 | Unified candidate-conditioned requests/all edges; D-017 settled |
| R-006/R-007 | Ready artifacts and effective smoke policy before publication, trusted channels/signatures optional |
| R-008 | Neutral facts, complete joins, explicit retained closure |
| R-009/R-010 | Explicit preparation/effects, package requests distinct from host grants; native trust not sandbox |
| R-011 | Redbook source rule superseded, current own 1.0.0; captures/frozen elsewhere not bulk rewritten |
| R-012 | Minimal change, no scheduler; complete required atoms |
| R-013/R-014 | Four-client/resource-level parity and receipt coexistence, bridges complete |
| R-015/R-016 | Existing boot representation, exact/unknown source attribution, no quotas, typed representation |
| R-017 | Honest capture/moving-tree proof, explicit promote |
| R-018 | Mandatory general derivatives, new identity/permissions, owner sovereignty |
| R-019 | Single source classification before every consumer |
| R-020 | Same authorization pipeline A/B, explicit future switch, revoke/out-of-scope refusal |
| R-021 | Target/executor/principal/root distinction; instance/generation-bound authority and physical alias locks |
| R-022 | Realized runtime closure and prefix/ABI proof; shared legacy/new binary validation |
| R-023 | Reverse dependencies, retained generations/live roots, config/data separation, truthful backend recovery |
| R-024 | Foreign version/signature/DB/file authority preserved; exact backend transaction capture |
| R-025 | Linux execution/metadata, actual image evidence, no secrets, Docker userspace vs future VM boot |
| R-026 | Machine service catalog with bound identity/auth; discovery does not authorize effects |
| R-027 | Nix-like declared builds/substitution without mandatory GitHub; full independent-host and offline E2E |
| R-028 | Complete operation/model register and real external coverage; no CLI/TUI/prose-only semantics |
| R-029 | Snapshot/capture/occurrence identities, typed completeness and per-client overlays |
| R-030 | Responsive API, durable operation identity/idempotency/reconnect and actual control boundaries |
| R-031 | Strict inputs and all-path structured partial/unknown/recovery errors |
| R-032 | Semantic package rename with complete scoped refs, saved-state CAS and truthful journal |
| R-033 | Full versioned semantic internal views, pure trace readers, explicit detail/retention/redaction/completeness |

No unresolved design preference from the original 19-question walkthrough remains. New D-022 about pacman's initial role is open; A recommended. All generic architecture and accepted HOME/Docker/network/Nix additions proceed. Exact wire/recipe/UI framework choices remain engineering work.

## 12. Owner decisions with latest responsible milestone

**Original 19 and D-020/D-021/D-023/D-024/D-025/D-026 accepted; D-022 open.** Эта таблица фиксирует mandate, явную recommendation и responsible implementation boundary. Она совпадает с review §8. Approval конкретных external batches, future B activation, prototype graduation и future Redbook version choice — будущие owner actions, не пропущенные ответы.

| ID | Принято владельцем | Implementation / responsible boundary |
|---|---|---|
| D-001 | Оба bridge-пакета Spec Kit и external-skills skills — flow; имена/upstream provenance сохраняются, текущая версия 1.0.0 | M-09-B |
| D-002 | create создаёт/меняет код; build детерминированно собирает; verify проверяет; полный проход имеет отдельный явный вход | M-03-A; обычный/scoped verify не запускает новый producer |
| D-003 | Реальные алгоритмические версионированные миграции: сохранить выбор/комментарии, проверить, показать diff, безопасно записать; неоднозначность требует ответа, не LLM | M-13-C/D, затем каждый изменяемый authored/wire format |
| D-004 | Default Redbook — multi-user-planning, WAL — альтернатива. Unattended: explicit selection или explicit allow-defaults, иначе NeedsSelection. Выбор сохраняется и update не следует новому default | M-05-E/F |
| D-005 | Взаимоисключение только continuity group Redbook; глобальный запрет сосуществования WAL/MUP не вводится | M-05-F/G |
| D-006 | Никаких лимитов/квот/порогов допуска по размеру или tokens бутлейна. Калькулятор и диагностика, включая marginal static/dynamic вклад до установки | M-06-B/E/F |
| D-007 | Сначала интерактивный debugger существующего представления: состав, why-included, provenance, сравнение/what-if; versioned machine input/output/errors/progress/snapshot IDs для IDE | M-06-B…F; смена canonical формата не входит в текущую работу |
| D-008 | Внешние package-defined адаптеры с записью настроек входят в первую волну; permissions/trust/plan/apply/update/recovery/uninstall обязательны сразу | M-08-C/D/E |
| D-009 | Общие локальные производные ресурсы обязательны. Владелец проекта может создавать свою версию; sealed не запрещает её. New identity/exact provenance, совместимость проверяется отдельно | M-10-A…D |
| D-010 | Artifact smoke включён перед публикацией по умолчанию, отключается явным параметром; skipped никогда не passed. Полные tests/checks — отдельно явно запускаемые действия | M-07-B/C/D/E |
| D-011 | Доверенные исполнители/проверяемые каналы + exact subject validation; криптографические подписи опциональны | M-07-A/C |
| D-012 | Обязательны Claude Code, Codex, OpenCode, Qwen Code в CLI-контексте. Gemini/Copilot глубоко вторичны и не блокируют первую волну | M-08-E; existing legacy support не удаляется из-за этого списка |
| D-013 | Вся текущая собственная линейка — прототип без постоянных пользователей. Только владелец объявляет выход из prototype; после этого вводятся обязательства для крупных breaking changes. Публикация/номер версии сами режим не меняют | M-13-B/C, M-12-B; будущая graduation не объявлена |
| D-014 | Structured ordinary Markdown обязателен сейчас; XML может быть обычными данными. Foreign bridge contents по умолчанию не интерпретируются как Vibe specs. Это жизненно необходимая bridge machinery | M-06-A перед M-01/M-09: единая classification до scan/resolve/convert/collision |
| D-015 | Сейчас A: одно подтверждение complete prepared batch. Архитектура режима B — bounded autonomous publication — готова сразу, но включается только будущим явным решением владельца | M-07-A/D/E и M-12-D; live autonomy выключена |
| D-016 | Минимальный repo-owned change binding; no scheduler/personal cursor | M-02-B |
| D-017 | Одна feature/request/effects algebra и constrained choice groups; никакой второй dependency grammar | M-05-B/C/D |
| D-018 | Полноценный сценарий включает existing-file edits, pre/post witnesses и build/test новой версии source. Disjoint-only демонстрация не закрывает доставку | M-11-D pulled forward между M-03-C и M-03-D/E; M-11-E real brownfield proof |
| D-019 | Комплектацию любой версии Redbook можно менять. Перед будущей перезаписью агент предлагает новую версию; нынешнее введение вариантов — existing 1.0.0 без bump. Все актуальные собственные продукты/пакеты проекта — 1.0.0 | M-13-B/D, M-05-F, M-12-A/D; прежний автоматический edition-bump rule superseded |
| D-020 | VibeVM должен стать менеджером пакетов/окружений от пользовательского HOME до будущего Linux-дистрибутива; архитектура tools/install готовится сейчас, полный дистрибутив пока только outline | M-14 early seams; M-15 runtime phase; future VM/system boot gate |
| D-021 | HOME и Docker обязательны в одной отдельной практической фазе; image builder для ИИ-агентов использует общий environment/package design | M-15-A…J; оба результата нужны для exit |
| D-022 | Открыто: coexistence с pacman сначала или немедленная полная замена. Рекомендация A — Vibe управляет составом и собственными payloads, pacman/libalpm сохраняет Arch backend; будущая замена возможна через тот же port | M-14 нейтральный backend contract; решение до M-15-E |
| D-023 | Сначала локальная Docker-сеть сервисов и единый machine interface: автоматические descriptions/inventory, управление в пределах пользовательских разрешений | M-15-H/I; remote/fleet operations позднее |
| D-024 | Nix — образец архитектуры; VibeVM не зависит от GitHub для bootstrap/source/registry/build/cache/images/update/publication. Источники и зеркала сменяемы; полный цикл без GitHub обязателен | M-14-A/B, M-15-A/B/E/G/I/J; Nix runtime/DSL не обязательны |
| D-025 | Все возможности VibeVM и внутренние структуры для анализа/отладки должны быть доступны внешним IDE-клиентам; сначала переделать общую архитектуру фичей, сам плагин/IDE/MVP пока не планировать | M-16 early service/overlay contracts; per-domain §3.25; M-17 complete external path |
| D-026 | Extreme Total Observability — общий принцип: каждое значимое состояние, решение, преобразование, действие и отказ имеет machine-readable inspection/explanation с provenance и явной неполнотой | M-16/M-17 and every domain exit; access/trust unchanged, no secret/raw-memory dump |


Переход B пока не разрешён; code/policy/data/grant model и isolated tests для него обязательны сразу. Действует A с одним подтверждением complete prepared batch. Current variants change keep-1.0.0 уже решён, повторно спрашивать номер не нужно.

## 13. Deferred work

| Work | Disposition / activation criterion |
|---|---|
| Multi-run/multi-park store | Future keyed state/leases/sharing/GC for real concurrency; current workspace single continuation |
| Multi-origin change execution | Future conflict/refinement proof; current complete scope one origin |
| Arbitrary/cross-package choice expressions | Not needed for accepted root/group/branch additions; no workflow semantics |
| Custom executing frontend hidden in source query | Forbidden hidden effect; future explicit extraction capability may reuse native admission |
| True semantic-equivalence hash/full XML-spec source map | Requires equivalence/source-map law; current representation/raw identities honest |
| Canonical boot compaction/externalization/new BootIR | Outside current D-007 debugger-first programme; later separate owner-directed change |
| Gemini CLI/GitHub Copilot adapters | Deeply secondary; not first-wave acceptance blockers |
| Uncoordinated arbitrary overlapping writers | Not a supported exact-acceptance model; controlled edits are mandatory now |
| Every-language brownfield semantic analyzer | Stack-specific proof first; unknown analyzer honestly partial |
| Bidirectional tracker | Outside scope; optional one-way export only |
| Mandatory crypto signing | Not selected; optional admitted signature path can be enabled by user |
| Live autonomous mode B / prototype graduation | Future owner activation events only; B implementation/readiness is mandatory now |
| Bootable Linux distro/ISO/live host root replacement | Future owner-directed system phase after HOME/rootfs/OCI delivery; kernel/boot/reboot/rescue proved in VM |
| VMware Workstation commissioning | Future available VM backend; not needed for Docker userspace tests |
| Full pacman-engine replacement | Deferred if D-022 chooses A. If B is chosen, expand M-15-E with DB/format/version/signature/hook/upgrade/recovery compatibility gates before dependent delivery; no silent scope switch |
| Nix runtime/language/store compatibility | Optional interoperability later; adopted design principles and no-GitHub cycle mandatory now |
| Remote fleet/service mesh/autonomous infrastructure controller | Later scope; current local Docker network has full structured service contract |
| IDE/plugin/MVP design and implementation | Explicitly not planned by D-025; later consumes proven M-16/M-17 contracts. Existing standalone D-007 boot viewer remains in scope |
| Concrete LSP/DAP clients/adapters and every-language symbol engine | Future adapters/providers where useful; current generic external API, Vibe package rename, working-buffer analysis and ETO are mandatory |

Structured ordinary Markdown/XML-data, inert bridge machinery, external write adapters, general local derivatives, controlled existing-file transitions and HOME/Docker/local-service/no-GitHub delivery, universal IDE/ETO services, semantic package rename and headless full-path conformance are mandatory and absent from this defer table.

## 14. Traceability: finding → milestone/decision/defer

| Finding | Milestones | Accepted decision / disposition |
|---|---|---|
| F-001 | M-02/M-04/M-09 | D-016; minimal binding, no scheduler |
| F-002 | M-03/M-11-D/E/M-09 | D-002/D-018; transition pulled forward, verification no producer |
| F-003 | M-03 | Workspace single continuation; multi-run deferred |
| F-004 | M-13/M-06-A/M-01 | Shared classified safe source boundary |
| F-005 | M-01/M-10 | Source membership/derivation identity distinct |
| F-006 | M-02/M-04 | Complete retained evidence |
| F-007 | M-13/M-05 | D-003/D-017 |
| F-008 | M-05/M-06-D/E | D-017; exact candidate graph drives preview |
| F-009 | M-13/M-05/M-12 | D-013/D-019; old roster bump superseded |
| F-010 | M-07/M-12 | D-010/D-015; default smoke with skip, A/B-ready |
| F-011 | M-07 | D-011; trusted channel, optional signatures |
| F-012 | M-08 | D-008/D-012; open client map, four clients |
| F-013 | M-01/M-06/M-08 | Explicit preparation, pure snapshot query |
| F-014 | M-08/M-09/M-10 | D-008/D-009; request not grant |
| F-015 | M-09/M-12 | D-001/D-012/D-014; bridges flow/inert resources |
| F-016 | M-06 | D-006/D-007; debugger/calculator without quotas |
| F-017 | M-06-A/M-01 | D-014; explicit representation semantics |
| F-018 | M-11 | D-018 controlled transition and honest intake |
| F-019 | M-10 mandatory | D-009 owner-owned derivatives |
| F-020 | M-13/full route | Bottom-up acceptance and atom DAG |
| F-021 | M-06-A/M-01/M-09/M-10 | D-014 shared classification before consumers |
| F-022 | M-14/M-03/M-05/M-08/M-15 | D-020/D-021; target-ready carriers early |
| F-023 | M-14/M-15-A/B/C | Payload/closure/ABI and legacy-bin parity |
| F-024 | M-05-E/M-15-C/D/E | Reverse-closure removal, generations/GC and honest recovery |
| F-025 | M-14/M-15-E/F | D-022 recommendation; independent backend authority |
| F-026 | M-14/M-15-F/G/I/J | D-020/D-021; actual Docker evidence, future VM boundary |
| F-027 | M-14/M-15-H/I | D-023 local machine-readable services |
| F-028 | M-14/M-15-A/B/E/G/I/J | D-024 Nix reference, hermetic/prefix/cache claims and no-GitHub cycle |
| F-029 | M-16/M-17/all domain exits | D-025/D-026 universal service/model coverage |
| F-030 | M-06/M-16-D/M-17-A/B | Typed full tree/provenance/occurrences/snapshots |
| F-031 | M-16-B/C/D/M-03/M-08/M-15/M-17 | Responsive operations, overlays, reconnect and actual capabilities |
| F-032 | M-16-B/C/M-17-A/C | Strict schemas, partial effects and durable idempotency |
| F-033 | M-16-D/M-17-B/C | Required semantic package rename; no foreign/history corruption |
| F-034 | M-06/M-16/M-17-A/C | D-026 full internal observation with explicit capture completeness |

Immutable gap backscan: D1 bridges→M-09; D2 distribution→M-07/M-12; D3 boot→M-06 debugger; D4 agents→M-08 four external adapters; D5 feats→M-02/M-09; D6 clarify→M-04; D7 coverage→M-04; D8 CI→M-07; D9 registry→M-12; D10 human view→M-06; D11 customization→mandatory M-10; D12 workflow→external-only/non-goal; D13 tree/display→debugger and verified fixes/docs; D14 Agent Plugin→existing providers M-08/M-09; D15 docs→M-13/M-12; D16 public policy→accepted D-013; D17 URI→M-01; D18 variants→M-05. Gap labels D1…D18 are not owner decision IDs D-001…D-019.

Owner additions beyond immutable baseline: no budget quotas, full interactive machine-addressable debugger and preinstall delta; ordinary XML-data essential for bridges; mandatory external write adapters/general derivatives; Qwen first-wave and Gemini/Copilot secondary; default smoke explicit skip; Redbook future version offers/current no-bump/current own 1.0.0; prototype until owner graduation; A now with B-ready architecture. New additions: OS-ready package/environment architecture, HOME+Docker required together, local AI-aware service network, Nix design reference and no mandatory GitHub. ETO/IDE additions: every capability/internal structure externally described and inspectable, client-local working buffers, semantic rename, responsive durable actions/controls, full headless compatibility. Every addition has required atoms and tests above; D-022 has an explicit decision boundary. Plugin/IDE/MVP planning is explicitly excluded.

## 15. Changes in this revision

Revision 4 adds D-025 universal IDE-compatible functionality and D-026 Extreme Total Observability while preserving previous package/HOME/Docker/Nix decisions and the open D-022 preference. Architecture sections 3.20–3.25 define the complete external path and bind it to every existing domain.

M-16 introduces the capability/model census, shared service/schema contracts, responsive durable operations/events/reconnect/control and versioned editor overlays before consuming features. M-17 completes initial legacy/model exposure before M-12, then semantic package rename and full cross-domain conformance after M-15. Domain milestones and programme exits now require external coverage; a JSON label, shell wrapper or unavailable stub cannot close it.

Internal debug structures are versioned semantic snapshots with exact provenance/locations/occurrences and explicit completeness. Pure retained inspection is separate from new capture/execution; observer failures do not alter domain results. Uniform input/result/error/event validation includes partial effects even when a final report could not be produced. Rename uses exact authored reference scope, saved-state synchronization with a cooperative buffer edit barrier, journal/regeneration and identity transitions. Atomic scoped submission reservations and host-issued namespace epochs/tombstones preserve deduplication after result eviction without releasing active/recovery identities.

No IDE/plugin/MVP or concrete LSP/DAP adapter is planned. Two headless protocol clients prove the intended capability without choosing a graphical product. Existing D-007 boot viewer remains part of its earlier scope.

Only these paired documents change. Source/spec/manifests, immutable baseline/request, logs and stewardship are unchanged. New runtime/protocol/rename/observability tests remain implementation work; this pass performed source/contract/document checks only.

<!-- review-protocol: 1 -->
<!-- pair-revision: 4 -->
<!-- baseline-fingerprint: sha256:0b6524d4128c5ece0763b14f791cbdc3e9cc4c6fd30aefa14b4f93df12892084 -->
