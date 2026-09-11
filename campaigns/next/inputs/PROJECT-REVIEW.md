# VibeVM — PROJECT-REVIEW

<!-- review-protocol: 1 -->
<!-- pair-revision: 4 -->
<!-- baseline-fingerprint: sha256:0b6524d4128c5ece0763b14f791cbdc3e9cc4c6fd30aefa14b4f93df12892084 -->

## 1. Purpose and immutable baseline

Жанр: архитектурный review. Раздел 8 фиксирует принятые в диалоге owner decisions; они выше прежних рекомендаций и противоречащих им repository notes. Остальные архитектурные предложения требуют implementation и проверки. Обновление этой пары не меняет product code/specs и не выдаёт planned behavior за shipped.

Immutable inputs: [IMPLEMENTATION-PLAN.md](C:/Users/olegc/git/v/final-improvement/IMPLEMENTATION-PLAN.md) и [REVIEW-REQUEST.md](C:/Users/olegc/git/v/final-improvement/REVIEW-REQUEST.md). Fingerprint вычислен как SHA-256 точных bytes первого файла, одного нулевого byte и точных bytes второго. Плановый baseline этого прохода — согласованная revision 3 этой пары. Прежние 19 принятых owner decisions сохранены. Новый mandate добавляет Nix-inspired package/environment management, HOME + Docker и AI-aware локальную сеть; D-020/D-021/D-023/D-024 приняты, D-022 о начальной роли pacman остаётся открытым с рекомендацией A. Новый D-025 требует universal IDE compatibility без планирования plugin/MVP; D-026 фиксирует Extreme Total Observability как общий принцип всех фичей. Immutable inputs сохраняются для scope backscan.

Проверенное дерево VibeVM: commit b1291b06d5704ee32f7486c86e1fae617fcbbddf. Исходный tracked tree чистый; существующий untracked cache/ не использовался как authority и не изменялся. Основания ниже — исходники и нормативные решения, не legacy WAL и не выводы сравнительного исследования. Соседние bridge packages проверены отдельно от исследовательского snapshot.

**Verified fact** — подтверждение по прочитанному primary source либо явно описанному измерению. Это не утверждение о запуске соответствующих tests. **Inference** — вывод из фактов. **Recommendation** — предлагаемое решение. **Unverified** — требуемое, но не полученное доказательство. В этом pass не запускались builds, product tests, installers, providers или публикации. Предыдущая read-only Docker availability probe (E43) установила: CLI/Buildx есть, Linux daemon недоступен. В текущем IDE/ETO проходе Docker повторно не проверялся; изучены source paths и public protocol references, runtime tests не запускались.

## 2. Executive verdict

**Inference.** VibeVM имеет сильный инфраструктурный фундамент: общий extension registry, typed compiler IR, отдельные lifecycle/evidence identities, hosted continuation, generated wires и реальные механизмы package/deploy ownership. Основная проблема программы — незавершённые связи между существующими механизмами и слишком широкие обещания на их границах.

Исходный план правильно защищает package intent от локального редактирования, оставляет выбор следующего действия внешнему процессу и различает observations и policy. Но его нельзя отдавать на последовательную реализацию без исправлений. Он переоценивает slot-record integrity, lifecycle isolation по node и feature integration; недооценивает solver backtracking, сохранность closure evidence и первую разрушительную границу release. Некоторые новые слои повторяют работающие механизмы.

**Owner decision / Recommendation.** Обязательная программа теперь включает structural ordinary Markdown, XML-data и inert bridge resources; полный boot debugger с машинным протоколом; внешние write-capable adapters; общие project-owned derivatives; controlled existing-file create. Change остаётся небольшим intent binding. Early source classification предшествует safe resolver, а candidate-correct configuration algebra питает preinstall debugger. Existing compiler/registry/deploy engines переиспользуются. Новый BootIR и смена canonical boot representation сейчас не нужны.

**Inference.** Ограниченные вертикальные срезы технически реалистичны. Программа шире прежнего минимального предложения: её успех зависит от доказательства post-state semantics, source classification, реальных external-adapter grants и end-to-end связности. Прежние owner choices закрыты; новый D-022 явно открыт, остальные additions приняты. Необходимость реализации и доказательств сохраняется. Числовой прогноз успеха не обоснован. Главная illusion of completeness — принять наличие поля, helper, положительного счётчика edges или green handler за доказательство всей пользовательской возможности.

**Owner mandate / Recommendation.** VibeVM теперь проектируется и как пакетный менеджер пользовательских окружений и Linux userspace/image builder. Ориентир — Nix: exact closure, isolated builds, immutable payloads, profiles/generations и rollback, без обязательного GitHub или Nix runtime. Existing source packages/build/deploy seats переиспользуются. Недостающий объект — realized environment с явной target identity; новый общий executor/DSL не нужен. M-14 вводит ранние seams, M-15 отдельно доставляет HOME, Docker и локальную AI-aware сеть. Full Linux distribution и VMware boot tests остаются будущим этапом.

**Owner mandate / Recommendation.** Extreme Total Observability теперь является общим product law: все supported operations и semantic internal/debug structures имеют machine-readable discovery, inspection, explanation, action/result/error/event и provenance path. Current tree/MCP/trace — foundations, но они не дают такой полноты: данные теряются в human-only полях, serial MCP ограничивает controls, часть failures скрывает уже выполненные effects. M-16 вводит общие service/context/overlay/operation contracts рано; M-17 завершает покрытие всех features и semantic package rename. Плагин/IDE/MVP не планируются; headless clients доказывают совместимость до будущего UI.

## 3. Verified current-state capability matrix

| Область | Verified fact | Чего это не доказывает | Evidence |
|---|---|---|---|
| Lifecycle | Девять фаз, inclusive prefix, build/test до create, отдельный verify evidence gate | Новый код после create не обязательно был build/test input | E01–E03 |
| Hosted work | Resume сверяет fingerprint, outputs, run identity; e1 acceptance — non-empty file | Semantic correctness и безопасный overlapping rewrite | E02, E04 |
| Coordination | Lease/state в workspace root, один run header | Независимые parked runs в members | E04 |
| Requirements | Shared CLI/MCP library query, authored requires, независимые adoption/relations, bounded result | Полный change closure или execution verdict | E05 |
| Terminality | Specification-only facts могут завершаться; absent requires — unclassified | Legacy verifies count не равен successful test exact bytes | E06 |
| Features | Expansion и exclusive at-most-one существуют | Optional deps, forwarded features и subskills не интегрированы end-to-end | E07–E09 |
| Solver/world | Resolver, visibility retry и lock construction существуют | Wrapper initial solve не покрывает другие пути/candidate rollback | E08–E10 |
| Spec lookup | SpecAddress, FileResolver, DocTree, provider-scoped prompt resolver | Безопасный public content API с полной integrity | E11–E14 |
| Materialization | Source/derived/overlay hashes и converted/copied files различаются | Record не authenticated file membership proof | E13 |
| Compiler | Typed source/document/closure/lane/emitted carriers, analyzer, frontend/backend seats | Новый BootIR не доказан необходимым | E15 |
| Agent surfaces | Пять legacy MCP profiles; skills у трёх; три lifecycle skill/plugin clients | Не пять одинаковых универсальных adapters | E16–E18 |
| Bridges | Два внешних manifests отделяют bridge/upstream identity; Spec Kit выбирает пять resources | Их foreign Markdown/XML не обязаны знать Vibe grammar; suffix-based scanners пока нарушают нужную границу | E19, E26 |
| Distribution | Четыре native targets; clean committed source; commit/tree/bundle binding | Workflow definition не свидетельство выполненных native smokes | E20–E21 |
| Versions | В коде есть snapshot/frozen; прежний Redbook roster rule ещё записан | Новый owner ruling разрешает любой Redbook composition update и нынешний no-bump 1.0.0; код/спеки ещё предстоит согласовать | E22, §8 D-019 |
| Boot | STATIC и явный INDEX footprint измеримы в bytes | Не provider bill/cache hit/полный ambient context | E23 |
| Tool/runtime payload | Cargo declarations, artifact records, single-binary digest store and launcher exist | General runtime closure, relocatability, whole-environment generation and legacy-bin parity | E30–E33 |
| Environment targets | Roots injected; user and system effect vocabulary exists | Target environment/principal identity, Linux metadata from Windows, elevation or confinement | E32/E34 |
| Remove/recovery | Source slot removal and provider-specific deploy journals exist | Reverse dependency protection and atomic whole-system rollback | E34 |
| Docker | Local CLI/Buildx present; Linux daemon connection unavailable | No image build/runtime/OCI/reproducibility proof | E38–E43 |
| Nix reference / supply chain | Nix documents profiles, store and multiple source transports | Vibe has not proved complete hermetic/no-GitHub supply chain | E30/E44 |
| IDE substrate | PROP-031/032 define a model/refactoring direction; current CLI tree/conversions exist | Universal graph/refactor service is not implemented by a proposal | E45/E46 |
| Machine operations | MCP tools and shared lifecycle/requirements libraries exist | Complete operations, responsive controls, snapshot/overlay/reconnect parity | E47/E48 |
| Machine errors | Some executed lifecycle failures have structured reports | Every Err is pre-effect; universal schema validation or recoverable partial failure | E49 |
| Internal trace | Accepted compiler IR snapshots and durable index exist | Complete cross-domain debug access, pure reader and configurable full-detail retention | E50 |

## 4. Findings

### F-004 · P1 · Point resolver требует общей безопасной path boundary

**Verified fact:** SpecAddress отвергает пустые doc segments, но не portable escape components; lookup использует path join; section lookup не проверяет pinned revision и duplicate anchors перед выбором узла (E11–E12).
**Inference:** Подтверждена опасная static call chain parser→join→read. Exploit на файловом дереве не воспроизводился.
**Impact:** Public CLI/MCP расширит доступность небезопасного seam, используемого также compiler/embed.
**Disposition:** Общая validated address/path grammar, capability-relative reads, typed ambiguity/revision refusals, сохранение I/O errors вместо guessed not-found.
**Mapping:** M-13, M-01; R-001.

### F-005 · P1 · Slot-record consistency не равна locked-source integrity

**Verified fact:** record.source_hash сравнивается с expected hash; payload проверяется по hashes из того же record; mixed record path не пересчитывает aggregate source hash. Transformed path использует record-local derived hash (E13).
**Inference:** Согласованная подмена record+payload не опровергается этими сравнениями. Это static trust finding, не воспроизведённая атака.
**Impact:** Baseline require-locked-integrity обещает больше существующего verifier. Абсолютный закон «любой slot byte-exact source» также неверен: conversion и overlay существуют.
**Disposition:** Разделить observed, record-consistent, source-proven, derivation-proven. Строгий запрос требует доказательства соответствующего уровня либо отказывает.
**Mapping:** M-01; R-002.

### F-002 · P1 · Create→verify требует causal evidence, не нового имени fingerprint

**Verified fact:** Measurement берётся до dispatch. Verify повторно наблюдает completed prefix до своих handlers и останавливается на stale/missing/unstable. Fingerprint включает весь context.artifacts. Build/test предшествуют create (E01–E03).
**Inference:** Disjoint outputs самого create не гарантируют disjointness со всеми прежними producer inputs. Проверка в verify handler не обходит pre-handler gate. Обновление predecessor artifacts может снова инвалидировать create.
**Impact:** Возможны повторное создание и ложное принятие «новый код проверен». Foundational build feat/--stack обещание не представлено текущими LifecycleArgs (E24).
**Disposition:** D-018 требует existing-file edits в complete scenario. Transition atom M-11-D выполняется раньше, внутри первого executable route M-03. Disjoint artifact — только промежуточный proof. Pre-input measurement сохраняется; обычный/scoped verify проверяет/reuses completed production, но не запускает новое создание; full pass — отдельная явная composition по D-002.
**Mapping:** M-03, M-11; D-002, D-018; R-003.

### F-003 · P1 · Изоляция сейчас workspace, не selected node

**Verified fact:** Lease и state path workspace-rooted; fresh run меняет единственный header и убирает прежние delegated rows (E04).
**Impact:** Baseline допускает параллельные independent parked changes в разных members, которых store не поддерживает.
**Disposition:** E1 — один scoped continuation на workspace; другой scope получает conflict до displacement. Другая worktree допустима со своим state root. Multi-run store отложен.
**Mapping:** M-03; R-004.

### F-008 · P1 · Selection должна переживать backtracking, visibility и все consumer constraints

**Verified fact:** solve, manifest_of, solve_masked — разные routes. Visibility реконструирует declarations. Resolvo предварительно читает candidate metadata; root union first-wins de-duplicates по package identity (E08–E10).
**Inference:** Mutable ChoiceOverlayProvider на одном route теряет edges или переносит requests отвергнутого candidate в selected world. First-wins теряет member constraints. Capability-valued effects требуют actual selected-provider edges в graph/lock.
**Disposition:** Один immutable normalized request/catalog на всех routes; candidate-conditioned implications; сохранение каждого consumer edge. Не глобальный mutable feature union.
**Mapping:** M-05; D-017; R-005.

### F-007 · P1 · Feature integration шире известного optional-dep gap

**Verified fact:** Solve предшествует expansion; active_deps/forwarded features не управляют production solve; record пишет пустой subskills_active. Parser делит dep/feature по первому slash при qualified dependency names. Bare Fresh path предшествует feature processing (E07–E10).
**Inference:** Optional=true само по себе проблему не решает; свежая установка может проигнорировать новый feature request. Structural validation через all=true ошибочно отвергает selectable exclusive groups.
**Disposition:** Один typed feature/request/effect algebra; choice groups — thin authoring/UI constraints над теми же feature IDs. Отдельная полная choices.*.requires grammar отклонена. Full package bytes сохраняются; activation управляет effective contributions.
**Mapping:** M-13, M-05; D-003, D-017; R-005.

### F-010 · P1 · Release изменяет remote state до native builds

**Verified fact:** Workflow prepare предшествует build; prepare_with удаляет existing release, перемещает/создаёт tag, создаёт draft (E20–E21).
**Impact:** Gate перед finalize не защищает старый release при неудачной сборке.
**Disposition:** Все exact bundles готовы до destructive prepare. По D-010 smoke включён по умолчанию; explicit parameter позволяет skipped, что фиксируется отдельно от passed. Preflight проверяет effective smoke policy, subjects и approval перед первым write. По D-015 один prepared-plan pipeline поддерживает approval A сейчас и bounded policy B в будущем, без включения B этим pass.
**Mapping:** M-07; D-010, D-011, D-015; R-006.

### F-011 · P1 · Receipt hashes не удостоверяют producer, reproducibility не следует из identity

**Verified fact:** Actions уже pin по SHA; Rust channel stable; runner labels/apt packages не immutable environment. Distribution manifests уже связывают source и bundle (E20–E21).
**Inference:** Три параллельных receipt системы дублируют existing manifest. Unsigned external JSON с верными hashes не доказывает прохождение checks; signature без authorized producer/policy тоже недостаточна.
**Disposition:** Расширить existing evidence chain. D-011 выбирает approved producer/channel и exact subject validation; криптографическая подпись опциональна. Imported arbitrary JSON остаётся observational. Full source tests explicit, smoke separately default-on/skip-capable. Identity, execution provenance и воспроизводимость — разные claims.
**Mapping:** M-07; D-010, D-011; R-007.

### F-006 · P1 · Counts, matched identity и closure — разные утверждения

**Verified fact:** Terminality допускает self-carried specification и legacy edge counts; requirements не отдаёт terminal verdict, имеет limit 256/truncated; lifecycle state erasable (E04–E06).
**Inference:** Verifies>0, matched, command ok и semantic acceptance нельзя склеивать. Closure с одними IDs из erasable state теряет основания. Truncated result не полная universe.
**Disposition:** Прозрачный complete join с evidence strength/freshness; existing terminal resolver используется один раз. Closure содержит retained evidence values/immutable references и policy digest, исключается из собственного intent digest.
**Mapping:** M-02, M-04; R-008.

### F-013 · P1 · Prepare helper может hydrate/cache

**Verified fact:** prepare_declared_skill_projection вызывает hydration; composition создаёт cache files/directories; binding передаёт offline=false. Analyzer recompiles через compiler seats (E15, E17).
**Inference:** Rename helper в plan/query не убирает эффекты.
**Disposition:** Prepare/fetch отделить от pure query. Последняя получает admitted captures или unavailable; пустые caches не дают hidden fetch/write/provider execution.
**Mapping:** M-01, M-06, M-08; R-009.

### F-014 · P1 · Closed descriptor не обезвреживает arbitrary argv и active content

**Inference:** Typed argv может запустить interpreter, effectful client subcommand либо записать future-executable MCP config. Prompt способен требовать script execution; hash не safety approval.
**Verified fact:** Current deploy использует закрытые client-specific argv builders; presence markers слабы (E16, E18).
**Disposition:** D-008 включает external write adapters сразу. Package может описать новый client ID, относительные paths, keys и requested operations; host владеет semantics, symbolic-root bindings и issued grants. Запрос нового destination не даёт права писать туда. Closed client executable fields тоже заменяются open validated capability map (E27), иначе новый client всё ещё требует core release. Произвольный native provider остаётся explicit trusted execution, не sandbox.
**Mapping:** M-08; D-008; R-010.

### F-009 · P1 · Owner version policy расходится с записанным Redbook edition rule

**Verified fact:** Код и PROP-044 имеют snapshot/frozen; source Redbook требует bump при roster change, root называет redbook@1 immutable (E22).
**Owner decision:** D-019 явно заменяет автоматический Redbook edition-bump rule: состав любой версии Redbook менять разрешено; перед будущей перезаписью агент предлагает новый номер, нынешние variants — existing 1.0.0 без bump. Все актуальные собственные продукты/пакеты — 1.0.0. D-013 считает их прототипом до личного объявления владельца.
**Impact:** Нельзя оставить старое правило скрытой преградой реализации либо переписать исторические hashes. Номер 1.0.0 не exact capture и не обещание стабильности.
**Disposition:** Сначала согласовать действующие source rules и актуальные manifests/references с ruling. Не менять чужие upstream versions, исторические receipts/fixtures и machine epoch numbers под предлогом выравнивания product version. Generic frozen semantics не отменена для прочих явно frozen packages; обнаруженный такой conflict классифицируется точно, не снимается bulk rewrite.
**Mapping:** M-13, M-05, M-12; D-013, D-019; R-011.

### F-001 · P2 · Change — intent binding; work DAG не самостоятельный verdict

**Recommendation:** Repo-owned scope оправдан переносимой локальной выборкой intent, отличной от dependency package, personal plan и run. Но graph без completion evidence не вычисляет честную eligible work. Slug в directory ломает stable addresses, closure внутри snapshot даёт self-reference.
**Disposition:** Immutable ID directory; slug metadata; intent, work, resolution, request digests отдельно. Graph сообщает predecessors; blocked/ready — только явная policy над complete evidence.
**Mapping:** M-02, M-04; D-016; R-012.

### F-012 · P2 · Adapters развиваются из трёх existing ownership lanes

**Verified fact:** Standalone skills, automatic bindings и lifecycle deploy имеют разные receipts. Codex coarse scope bool и project skill path различаются. OpenCode владеет logical config members под physical lock (E16–E18).
**Impact:** Новый generic receipt может ослабить recovery или смешать owners.
**Disposition:** Characterization по surface/scope/shape/operation; reuse engines и explicit adoption между receipt families. First-wave write adapters обслуживают Claude Code, Codex, OpenCode, Qwen Code. Existing hardcoded client slots — конкретная migration surface (E27). New code/executable probes никогда не скрываются в pure plan.
**Mapping:** M-08; R-013.

### F-015 · P2 · Bridge pin, taxonomy и resource closure — самостоятельные inputs

**Verified fact:** Оба bridges kind=feat. Spec Kit bridge pin 96c9bd657bfd5de0d651a6165084932b7304ac99 отличается от research ce593cdc; выбраны пять resources; authorship metadata разделена. File-only skill deploy отвергает directory artifacts (E18–E19).
**Inference / Owner decision:** Research не доказывает реально pinned templates. D-001 принят: оба bridges переводятся в flow в текущей 1.0.0, без смены имени/upstream identity.
**Unverified:** Exact script/resource closure и возможное hydrated source_root/provider.root mismatch automatic binding не воспроизведены.
**Disposition:** Проверить current five до расширения, сохранить весь resource closure, завершить mandatory bridge journeys через admitted actions. Foreign resources по D-014 default non-Vibe, plain Markdown структурный, XML-data byte-preserving. Diagnostic unsupported не заменяет required journey. Две native feats доказываются на двух stacks с реальными existing-file edits.
**Mapping:** M-09, M-12; D-001; R-014.

### F-016 · P2 · Boot debugger должен объяснять точный состав без навязанного бюджета

**Verified fact:** STATIC 246936 bytes; RENAMED ANCHORS comment 81113; STATIC+INDEX+восемь entry files 305959; с AGENTS.md 332080. Это UTF-8 bytes, не tokens, без personal/task/ambient tail. Existing compiler IR/analyzer есть (E15, E23).
**Inference:** Marginal package contribution зависит от уже установленных/shared dependencies, choices, условий и compilation; сумма package sizes не даёт точный delta. Byte identity файлов не гарантирует provider cache hit.
**Owner decision:** D-006 запрещает boot size/token quotas вообще. Нормален и маленький local model, и миллион tokens; программа сообщает факты, не диктует бюджет. D-007 требует interactive structure/provenance/why-included/compare/what-if debugger с machine-readable input/output, errors/progress и snapshot IDs для IDE, на существующем представлении.
**Disposition:** Streaming/paged inspection без total boot cap; direct/inclusive/marginal metrics отдельно. Existing snapshot inspection pure; exact builtin preview использует тот же compiler in memory. Missing captures или требуемый effectful native stage дают incomplete/unknown, не fabricated zero; explicit preparation отделена от query.
**Mapping:** M-06; D-006, D-007; R-015.

### F-017 · P2 · XML→Markdown — existing pivot, не raw map или semantic identity

**Verified fact:** Pure project_spec_text уже проецирует XML; comments опускаются, DocTree normalizes lines; custom frontend — native execution (E12, E14–E15).
**Disposition:** Сначала explicit source classification, затем соответствующий parser. Только declared Vibe XML проходит existing XML→Markdown pivot. Structured CommonMark имеет sections без Vibe facts; XML-data сохраняется byte-exact и может иметь безопасный структурный view. Raw hash/representation digest/span basis раздельны. Full compiler IR не становится source-query wire.
**Mapping:** M-01, M-06; R-016.

### F-018 · P2 · Individually stable files не доказывают atomic tree snapshot

**Verified fact:** Scrape inventory использует safe file identity и повторную directory listing, пропуская .git; это не ready-made budgeted brownfield policy (E25).
**Inference:** Разные файлы могут представлять разные моменты состояния; tree digest не исправляет это.
**Disposition:** Observation window/completeness явны. Promote требует immutable capture либо полной affected-set revalidation и cooperative exclusion boundary; uncooperative writers → refuse. Generic core не запускает stack tools.
**Mapping:** M-11; D-018; R-017.

### F-019 · P2 · Обязательные local derivatives требуют собственной identity и отдельной activation authority

**Verified fact:** Existing skill composition/host controls/boot composition покрывают отдельные семейства ресурсов (E15, E17–E18), но сами по себе не являются общим lifecycle local derivative.
**Owner decision:** D-009 делает общий механизм обязательным; проект может создать собственную версию любого ресурса. Package sealed не запрещает такую отдельную identity; source provenance сохраняется, исходная compatibility не наследуется.
**Impact:** Копия не должна притворяться upstream, получать его execution grants или автоматически сливаться с новой mutable версией base.
**Disposition:** Общий authored binding/base capture/local source/result identity, derive/show/diff/rebase/apply с exact preconditions. Markdown, XML-data, Vibe specs и opaque resources; derive не executes. Activation/deploy executable derivative — существующий effect/trust protocol. Named slots могут расширять full replacement, но не ограничивают право local fork.
**Mapping:** M-10 mandatory, M-01, M-08, M-06; D-009; R-018.

### F-020 · P2 · Portfolio должен проверять seams до умножения систем

**Inference:** Baseline связывает descriptors с change, inventory с overlapping execution и прячет большие transaction/schema forks внутри длинных milestones.
**Disposition:** Foundation M-13, затем ранний M-06-A classification; source resolver и selected-graph work питают debugger. M-11-D transition pulled forward в executable M-03 route. M-10 и M-08 mandatory, M-09 требует их реального integration. Atom-level dependencies явно разрывают кажущиеся parent cycles; M-12 зависит от initial programme exits; added HOME/Docker phase отдельно закрывается M-15-J.
**Mapping:** весь route; R-012.

### F-021 · P1 · Foreign XML/Markdown ошибочно получают Vibe semantics по suffix

**Verified fact:** load.rs классифицирует md/xml по расширению и направляет XML в closed spec dialect; pair detection считает X.md/X.xml одним документом. Facts scanner применяет это ко всем найденным sources; resolver также набирает candidates по suffix. Materialization converter может преобразовать spec-shaped XML без знания назначения (E26).
**Inference:** Обычные XML-data способны вызвать false pair collision/invalid authored observation, а внешние Markdown — интерпретироваться как Vibe markup. Это static evidence; runtime exploit/failure fixtures ещё предстоит выполнить.
**Owner decision:** D-014 называет non-interpretation жизненно необходимой bridge machinery. Foreign payload обычно вообще не знает VibeVM.
**Disposition:** Один source descriptor перед enumeration, collision, canonical identity, facts/sync, conversion, source query и boot. Media, semantics, role/loading и trust раздельны. Spec semantics только explicit; namespace/path/suffix сами её не включают. XML-data and opaque fixtures сохраняют bytes; одинаковый filename stem разных logical resources не collision.
**Mapping:** M-06-A, M-01, M-09, M-10; D-014; R-019.

### F-022 · P1 · Deployment identity and platform describe the host, not a target environment

**Verified fact:** DeployExecution injects project/settings/user-home roots, but no environment instance/principal/rootfs identity. DeploymentHome hashes project/package/target; vibe-bin chooses LauncherFlavour::NATIVE, with POSIX mode operations compiled out on Windows. TargetWhen only carries OS; NativePlatform describes current-process native extension platforms (E32).
**Inference:** A different output directory alone cannot produce a faithful Linux installation from Windows or independently identify two environments in one state home. Native provider ABI and payload runtime ABI cannot share one implicit platform.
**Disposition:** Early M-14 target/executor/realization contracts; environment/root/principal/generation in requests, identity, grants, receipts and locks. Target filesystem semantics explicit; Linux operations execute in Linux or a proven metadata-preserving writer. Existing local compatibility retained; unsupported targets refuse.
**Mapping:** M-14, M-03/M-05/M-08 carrier seams, M-15; D-020/D-021; R-021.

### F-023 · P1 · Source installation and a single binary store do not form an installed runtime closure

**Verified fact:** BinaryDecl is Cargo-specific. New build/package providers really execute and record artifacts; build records use toolchain.host and omit a complete input census. vibe-bin has digest-addressed single-file payloads and per-command pointers. Public bin command routing still calls legacy existence/debug-first helpers (E30–E33).
**Inference:** Neither source slot nor A2 existence proves a relocatable tool with exact runtime libraries, metadata and a coherent environment generation. Legacy bin dispatch does not inherit the successor's promised artifact-record verification.
**Disposition:** Separate source capture, realization and installed payload manifest; build/tool/runtime dependency roles and target ABI; profile generations above reused store/receipt primitives. Migrate legacy bin consumers to exact validated artifacts. HOME compatibility is an explicit property, not arbitrary relocation of Arch /usr packages.
**Mapping:** M-14-B/C, M-15-A/B/C; R-022.

### F-024 · P1 · Whole-environment remove and recovery need closure and ownership checks

**Verified fact:** Current uninstall handler directly removes the selected slot, removes its lock row/root declaration, writes lock/manifest and then regenerates boot; this run path has no pre-removal incoming-dependency check. Current source hooks are script callbacks; source spec leaves undeclared output outside slot ownership. Deploy rollback differs by provider: retained payload pointer versus OpenCode update marked non-reversible (E18/E33/E34).
**Inference:** Reusing this removal path as an OS package manager can break remaining consumers; deleting installed bytes does not reverse arbitrary hooks or preserve mutable application state. Static call-path finding, not a reproduced failure.
**Disposition:** Environment remove re-solves desired/reverse closure and distinguishes explicit roots/shared dependencies. Profile activation, config/state migration, backend transactions and service effects have separate recorded recovery boundaries. GC roots include retained generations/live uses/recovery pins, with explicit ownership and no collateral deletion.
**Mapping:** M-05-E integration, M-15-C/D/E; R-023.

### F-025 · P1 · Arch integration requires a backend authority boundary

**Verified fact:** Pacman/libalpm owns dependency transactions and package files. Its root/sysroot behavior is not arbitrary prefix relocation; hooks can execute transaction effects. Repository order/signature policy and coherent Arch upgrades are material external contracts (E35–E37).
**Inference:** Two independent solvers/databases claiming the same Arch files would introduce inconsistent ownership and recovery. Optional Vibe evidence signatures cannot be interpreted as disabling native repository/package trust.
**Recommendation:** Vibe owns declarative composition and its own payloads; a backend preserves native version/DB semantics, exposes exact closure/transaction evidence and uses isolated Linux targets first. Pacman coexistence versus immediate full replacement is the one open D-022 preference; generic interfaces do not depend on that answer.
**Mapping:** M-14, M-15-E/F; D-022; R-024.

### F-026 · P1 · Docker support needs a real image backend and scoped evidence

**Verified fact:** Existing mechanisms have build/package/deploy extension seats; inspected code has no proven environment→OCI delivery contract. Docker/BuildKit provides target-platform builds, OCI export and secret mounts, but exporter/attestation support depends on configured driver/store (E30/E38–E40). Local read-only probe found Docker CLI 29.2.0 and Buildx v0.31.1-desktop.1; desktop-linux daemon connection failed (E43).
**Inference:** Host cross-build metadata or a generated Dockerfile is not proof of an executable image, retained attestations, isolation or reproducibility. Docker userspace tests cannot prove a bootable distribution.
**Disposition:** Package-defined BuildKit provider on existing artifact DAG, pinned inputs/runtime closure, actual Linux rootfs/metadata and non-root image tests; export/load/push distinct. Exact installed-content and bit-identical image claims separate. No secret values in layers/context/evidence. Docker runtime proof remains unverified here; future VMware covers boot/kernel/reboot.
**Mapping:** M-14, M-15-F/G/I/J; D-020/D-021; R-025.

### F-027 · P2 · AI-aware infrastructure needs structured service bindings beyond agent config

**Verified fact:** Current inspected agent projection/deploy surfaces configure clients and tools; they do not establish a managed network service catalog with operation schema, runtime endpoint identity and authorization. OpenAPI supplies reusable API description concepts (E16/E27/E42).
**Owner decision:** D-023 selects local Docker network and one machine interface first.
**Disposition:** Package-exported tool/service descriptors automatically enter the realized environment catalog and boot/debugger graph; explicit runtime endpoint/auth bindings, separate desired/installed/running/healthy states, pure catalog queries and admitted effectful probes/calls. Unwrapped resources remain honestly partial. Local agent plus two services proves discovery/read/write/refusal; no inferred network-wide administration.
**Mapping:** M-14-A/B, M-15-H/I; D-023; R-026.

### F-028 · P1 · Nix-like profiles need closed builds and host-independent supply paths

**Verified fact:** Current build records do not assert a complete input census; source/tree and binary payload identities differ (E30/E33). Nix documents store references, profiles and generic source locations, including non-GitHub references (E44).
**Owner decision:** D-024 selects Nix as a design reference and requires VibeVM independence from GitHub.
**Inference:** Content store and lockfile alone do not prove hermetic execution, relocatability, trusted binary substitution or an independently hosted complete supply chain. Existing generic Git registry is useful but does not prove every bootstrap/client/image/update path avoids GitHub.
**Disposition:** Exact build recipes and runtime closure, explicit isolation levels, prefix-compatible substitution, self-hosted source/registry/artifact/OCI routes and mirror-preserved provenance. Required end-to-end fixture independently mediates/observes control-process, native-child and builder/daemon egress; application transport counters alone are insufficient. It permits only local fixture endpoints and proves source build, cache substitution, home lifecycle and image creation with zero GitHub attempts. Unobserved bypass makes proof unavailable; offline replay separately denies all network. Nix language/runtime not a prerequisite.
**Mapping:** M-14-A/B, M-15-A/B/E/G/I/J; D-024; R-027.


### F-029 · P1 · Universal IDE/ETO coverage is not implied by selected JSON commands

**Verified fact:** PROP-032 names a model/agent-first direction but explicitly remains a proposal. MCP registers eleven tools, while public CLI exposes a broader command set; list_tools enumerates installed tools, not every product capability/debug model (E45/E47/E48).
**Owner decision:** D-025 requires all features and internal analysis/debug structures to support external IDE use before any plugin/MVP is planned. D-026 names the product principle Extreme Total Observability.
**Disposition:** Whole-product capability/model census with strict external contracts and actual per-row conformance, shared application services, early M-16 contracts and complete M-17 coverage. No universal writable graph replacing domain truth; no CLI/TUI-only semantic feature. Known unimplemented interface is a gap, not a passing unavailable result.
**Mapping:** M-16/M-17 and all domain exits; D-025/D-026; R-028.

### F-030 · P1 · Current package tree is a useful summary, not a complete source/reference snapshot

**Verified fact:** Tree model/build modules are CLI-local. Visibility provenance is serde-skipped; IDs use group/name; collected references are emitted with resolved=false without resolving targets. Some read failures become absence, repeated inputs collapse into maps, and current time substitutes for no actual snapshot identity (E46).
**Inference:** An IDE cannot infer dangling references, exact rename scope, complete inclusion occurrences or consistent live state from this summary. A mutable coordinate is not an immutable refactor-stable address.
**Disposition:** Move semantic query authority into shared services, preserve existing renderer compatibility, expose typed provenance/occurrences/unchecked-vs-missing states and snapshot/completeness/location contracts. Logical identity, exact capture and view occurrence remain distinct; rename supplies old→new transition.
**Mapping:** M-06/M-16-D/M-17-A/B; R-029.

### F-031 · P1 · Serial MCP dispatch and startup policy do not provide an interactive operation/session path

**Verified fact:** Server loop is synchronous; notifications ignored, initialization returns a fixed version and tools.listChanged=false; stdio EOF ends the loop. Lifecycle MCP uses fixed hosted/no-force/no-trace behavior and no deploy profile; its observer discards plan/contribution events. Context reloads lock per call without a cross-query snapshot. Hosted tasks do retain domain continuation, not transport reconnect (E47/E48).
**Inference:** Long calls cannot process independent cancel/query; a deploy phase name does not expose all deployment functionality. A connected editor also needs dirty-buffer snapshots, invalidation and reconnect identity absent from this surface.
**Disposition:** Responsive typed service API with capability negotiation, explicit context/overlay snapshots, durable operation/event/result identity, atomic scoped submission reservation with host-issued expiry namespaces/tombstones, reattach and cooperative controls on existing journals. Registered buffer versions participate in a held edit barrier through commit; unrelated editors are not silently included in that guarantee. No permanent daemon or new task scheduler required.
**Mapping:** M-16-B/C/D, M-03/M-08/M-15/M-17; R-030.

### F-032 · P1 · Text-only tool errors can hide effects and advertised schemas are not always enforced

**Verified fact:** ToolError emits text-only failure; materialise_subskill copies in a loop and propagates mkdir/copy errors, so prior writes can exist without returned structured written list. Query optional types silently default, and list_tools ignores arguments despite restrictive advertised schemas (E49).
**Inference:** Err≠not-executed; a generic IDE retry can duplicate or overwrite partial work if error shape obscures execution phase. Descriptor schema alone does not establish runtime validation. Static call-path finding, no failure injection executed here.
**Disposition:** Strict owned/admitted request decoding and validated result/error/event schemas for every operation; structured phase/partial effects/uncertainty/recovery even without final report. Idempotency records precede effects; unsupported legacy forms migrate explicitly.
**Mapping:** M-16-B/C, M-17-A/C; R-031.

### F-033 · P1 · Package rename requires a semantic transaction, not the current conversion command

**Verified fact:** Public refactor grammar exposes three source-conversion operations; rename-package is proposed in PROP-031. PROP-031's all-or-nothing intent coexists with unresolved transaction/concurrency details; current tree reference enumeration is incomplete (E45/E46).
**Inference:** String replacement or renaming a directory cannot prove package/reference/deployment compatibility. Published identity, authored references, generated outputs and immutable evidence have different owners.
**Disposition:** Required owned-package rename library + prepare/apply/recover protocol: exact reference scope and edit plan, unknown/collision diagnostics, saved-buffer boundary and cooperative edit barrier, preconditions/journal, regeneration and re-resolution. Preserve history/foreign captures and separate publication/deployment migration.
**Mapping:** M-16-D/M-17-B/C; R-032.

### F-034 · P2 · Existing compiler trace is a foundation, not total observability

**Verified fact:** Accepted compiler carriers have compiler_ir/e1 snapshot bytes and a durable trace-index/observer seam. Production trace retention is fixed at 128 MiB per run and nine older runs; failures/skipped snapshots are explicitly observational. TraceRun reopening belongs to the writer/lock lifecycle (E50).
**Inference:** Serializing some IR does not expose all solver/lifecycle/target/image/service state. Reading through a writer-open helper or silently rerunning providers would violate pure inspection. Fixed retention cannot justify a claim that every requested internal value is present.
**Disposition:** Read-only semantic snapshot access across model families, configurable explicit capture/detail/export/retention, complete missing/redacted/evicted states, provenance and observational health separate from domain success. No memory-layout ABI, hidden total boot quota or mandatory retention of every frame forever.
**Mapping:** M-06/M-16/M-17-A/C; D-026; R-033.


## 5. Specification-versus-implementation contradictions

| Утверждение в source/prior draft | Разница с evidence или owner ruling | Разрешение |
|---|---|---|
| PROP-003 features/subskills impl/done | Expansion есть, production effects неполны | M-13 truth, M-05 unified implementation |
| Foundational build feat/--stack | Current LifecycleArgs не выражает его; D-002 выбрал create/build/verify + отдельный full pass | M-13 source law, M-03 explicit public/scoped contract |
| Redbook roster requires new edition | D-019 разрешил change любой версии и current variants в 1.0.0 | Сначала supersede current rule; hashes/captures не переписывать |
| Public flag/compatibility ожидания | D-013: prototype до личного объявления владельца | Никакого auto graduation; future breaking-change policy отдельно |
| md/xml suffix означает spec | Foreign data не обязаны знать VibeVM | M-06-A общий classification, затем все readers/converters |
| Current draft CommonMark/derivatives/external adapters optional | D-014/D-009/D-008 сделали их mandatory | Убрать defer conditions и gate omissions во всём route |
| Stable/status-free STATIC planned wave | D-007 оставляет нынешнее representation для debugger first | Никакого silent canonical rewrite |
| Any slot byte-exact source | Existing conversions/overlays | Source/representation/proof identities отдельно |
| Always-required smoke | D-010: default-on и explicit skip | Typed passed/failed/skipped; effective policy и approval связаны с plan |
| Pure plan helper | Hydration/cache/probe/native effects существуют | Explicit preparation, pure captured inspection |
| Host owns exact client names/paths | D-008 требует новые clients из external package без core release | Open validated requests, host-issued exact grants; E27 migration |
| Prior plan excludes OS privilege management / system providers are future only | D-020 requires OS-ready target/install design now and HOME+Docker delivery; own elevation broker/full distro still future | M-14 source contracts now, M-15 userspace/image phase |
| PROP-025 successor record checks apply to every bin entry | Current public bin route still uses existence/debug-first helpers | M-15-A migration/parity; no inherited guarantee |
| Host OS/platform and user_home enough for deploy | Alternate target/rootfs has own ABI, principal, metadata and instance authority | M-14 explicit carriers; M-15 Linux executor |
| Installation-is-consent supplies unlimited effects | Accepted D-008 + target selection bind actual operations; package discovery and installing one target do not grant all host/root/network writes | Reconcile source wording; one reviewed install may authorize its complete declared batch without redundant prompts |
| Store path/lock implies a Nix-like hermetic environment | Inputs, runtime closure, isolation, prefix compatibility and every supply-chain transport still need proof | D-024; M-14/M-15, explicit reproducibility levels and no-GitHub fixture |
| PROP-032 universal graph/minted addresses imply navigation/refactoring complete | It is a proposal; current package coordinate changes on rename and tree references are not fully resolved | ETO service projections over real domain authorities, explicit identity transitions; M-16/M-17 |
| Some JSON/MCP tools mean IDE compatibility | Missing models/operations/events/dirty buffers and text-only partial failures remain | Whole-product coverage and full-path gates, no plugin-first workaround |
| ToolError means nothing executed | materialise_subskill can copy earlier files before later error | Structured partial/unknown effects and recovery for every failure |
| Fixed compiler trace contains all internal data | Retention/skips and other domain structures limit actual coverage | Explicit capture completeness, pure retained readers and all-domain schema inventory |

Принятые решения являются новым mandate, не доказательством реализации. Этот pass не исправляет repository specs/code; план задаёт соответствующие product atoms.

## 6. Architectural corrections to the prior plan

1. **Принято D-014:** shared source classification и structural CommonMark/XML-data — ранний foundation для bridges, safe lookup и derivative resources.
2. **Принято D-017:** одна candidate-conditioned feature/request/effect algebra, consumer-owned selections и all-edge constraints.
3. **Принято D-016/D-002/D-018:** small change binding, separate public production/verification intent, explicit full pass, controlled existing-file transition без automatic repair loop.
4. **Recommendation:** transparent complete analysis и retained closure evidence; unchanged neutral requirements root.
5. **Принято D-006/D-007:** полноценный debugger, static/dynamic preinstall marginal estimates и machine protocol; no boot quotas; canonical format сейчас сохраняется.
6. **Принято D-008/D-009/D-012:** first-wave external write adapters, general local derivatives, mandatory Claude Code/Codex/OpenCode/Qwen Code. Existing ownership/registry/IR remain foundations.
7. **Принято D-010/D-011/D-015:** default-on disableable smoke, trusted channels с optional signatures; one prepared publication plan, manual approval A now, bounded-policy B готов архитектурно.
8. **Принято D-001/D-013/D-019:** оба bridges flow, current own line 1.0.0 prototype, Redbook current no-bump update и future explicit version offer. Ни public visibility, ни SemVer не являются exact identity/compatibility declaration.

9. **Принято D-020/D-021:** stable environment/target/realization/payload contracts now; HOME and Docker are required later phase, full OS only design outline.
10. **Recommendation D-022:** pacman/libalpm remains Arch backend initially, preserving its version/signature/DB/file authority; open owner preference does not block generic design.
11. **Принято D-023/D-024:** local structured service catalog and Nix-inspired generations/build isolation; replaceable source/cache/registry/OCI hosts, full no-GitHub and offline fixtures.

12. **Принято D-025/D-026:** universal typed service/model contracts and Extreme Total Observability across all domains; a client can discover, inspect, explain, preview, authorize, act, observe, reconnect and recover without parsing prose.
13. **Recommendation:** preserve separate logical/capture/occurrence identities, client-local unsaved overlays, read-only internal snapshots and existing domain truth. Semantic package rename is a prepared journaled operation, not string replacement.
14. **Scope boundary:** M-16 common interfaces early; M-17 real coverage and headless conformance. Plugin/IDE/MVP and actual LSP/DAP client adapters are separate future decisions.

## 7. Risks and failure modes

| ID | Failure mode | Защита / маршрут |
|---|---|---|
| R-001 | URI escape/link race/duplicate lookup | M-01 grammar/capabilities/adversarial tests |
| R-002 | Согласованная record+payload tamper | Source membership/derivation proof или weaker result |
| R-003 | Untested output принят; repark loop | M-11-D pulled forward; M-03 integrated current-source proof |
| R-004 | Member displaces parked work | Workspace conflict before mutation |
| R-005 | Candidate contamination/root loss | Immutable inputs, conditional implications, all edges |
| R-006 | Release удалён до ready bundle | Ready bundles + passed smoke или explicit skipped policy до prepare |
| R-007 | Forged/replayed receipt | Authorized producer/policy/generation fence |
| R-008 | Truncation/counts/cache IDs → acceptance | Complete typed joins, durable evidence |
| R-009 | Query hydrates/executes | Empty-cache zero-effect tests |
| R-010 | Requested descriptor scope выдаётся за grant | Package-authored requests, host-issued exact capabilities |
| R-011 | Старое edition rule блокирует ruling / hash identity потеряна | D-019 no-bump 1.0.0, version offer отдельно, exact captures |
| R-012 | Второй planner/unbounded scope | Split identities, bounded route |
| R-013 | Два owners одного target | Coexistence/adoption/physical lock |
| R-014 | Bridge теряет resources/identity | Exact pinned closure и provenance |
| R-015 | Debugger даёт ложный attribution/delta или навязывает бюджет | Exact snapshots, non-additive metrics, no quotas, unknown explicit |
| R-016 | Projection выдана за raw truth | Explicit representation/basis/recipe |
| R-017 | Mixed-time observation → intent | Capture/revalidation + promote |
| R-018 | Local derivative наследует чужую identity/grants | Mandatory new identity/provenance и separate activation |
| R-019 | Foreign data классифицированы как Vibe specs | Explicit source descriptor до всех consumers |
| R-020 | Подготовленный режим B включился без owner switch | Policy state/generation, exact plan grant, revocation/out-of-scope refusal |
| R-021 | Host/target/root/principal confusion or receipt replay | M-14 target/executor bindings, target metadata and physical alias locks |
| R-022 | Ambient runtime dependency or legacy binary accepted as exact realization | Payload/ABI closure, shared recorded artifact validation, prefix capability |
| R-023 | Shared dependency/state deleted or false whole-system rollback | Reverse closure, retained generations/live pins, partitioned journals and mutable-state boundaries |
| R-024 | Vibe and Arch both own DB/files; backend trust lost | Single backend authority, exact prepared closure, native version/signature policy |
| R-025 | OCI reproducibility/bootability/isolation overclaimed; secret baked | Actual Linux executor, separate evidence levels, runtime secret bindings, Docker/VM gate separation |
| R-026 | AI catalog metadata becomes execution/network authority | Bound endpoints, operation schemas/grants, pure queries and explicit probes |
| R-027 | Hidden GitHub dependency or untrusted binary substitution | Recursive inputs/mirror policy, prefix/closure/producer validation, no-GitHub/offline E2E |
| R-028 | Supported feature/model has only private/CLI/TUI/prose access | Complete capability/model census and executable per-row external coverage |
| R-029 | Summary/dirty buffers/unchecked refs treated as coherent truth | Domain snapshot vectors, client-scoped overlays, typed completeness/positions/occurrences |
| R-030 | Connection/request identity confused with operation authority/lifetime | Responsive dispatch, durable idempotency/events/reattach/control on domain journals |
| R-031 | Invalid input silently defaults; error hides completed effects | Strict schemas and structured phase/partial/unknown/recovery outcomes |
| R-032 | Rename corrupts references/foreign/history/unsaved files | Exact typed scope/plan, saved-state CAS, journal/regeneration and identity transition |
| R-033 | Internal debug values silently absent, rerun or mutate during inspection | Pure captured readers, explicit capture/retention/redaction/completeness and provider schemas |

## 8. Owner decisions — accepted policy and future owner actions

Прежние 19 design choices и D-020/D-021/D-023/D-024/D-025/D-026 приняты владельцем. Единственный открытый preference — D-022; его рекомендация явно не выдана за согласие. Таблица различает owner input и proposed default.

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


Помимо открытого D-022 остались **будущие owner actions**: подтвердить конкретный внешний batch в действующем A; позже явно включить B и задать его boundaries; объявить graduation из prototype; выбрать новый номер при будущем Redbook version offer. Текущий no-bump 1.0.0 уже выбран. Отдельно требуется implementation/verification, но повторно спрашивать эти 19 предпочтений не нужно.

## 9. Rejected/deferred alternatives with reasons

| Альтернатива | Current disposition |
|---|---|
| Independent choices.requires и post-solve features | Rejected: duplicate semantics и отсутствие candidate correctness |
| Mutable callback feature union | Rejected: rejected candidate contamination |
| Boot size/token quotas, future threshold gate in this programme | Rejected by D-006 |
| Canonical compaction/externalization/new BootIR сейчас | Deferred by D-007; debugger existing representation first |
| Deferring CommonMark/inert XML bridges, external write adapters или general derivatives | Rejected by D-014/D-008/D-009; обязательная работа |
| Package sealed как запрет project-owned derivative | Rejected by D-009; interface conformance и execution grants остаются отдельными |
| Pure source query silently runs compiler/native/hydration | Rejected; explicit preparation и pure snapshot inspection |
| Mandatory cryptographic signature для любого trusted result | Rejected by D-011; signatures optional |
| Unconditionally required artifact smoke без override | Rejected by D-010; default-on explicit skip, no false pass |
| Автопубликация сейчас | Not enabled by D-015; B-ready policy implementation/tests required now |
| Multi-park, arbitrary cross-package expressions, automatic next-task/repair | Deferred или non-goal; не prerequisites |
| Auto brownfield promotion / tracker-owned intent | Rejected; explicit promote / optional one-way export |

Full bootable distro/ISO, live host system installation, full pacman-engine replacement if D-022 selects A, VMware commissioning, remote fleet orchestration and Nix language/store compatibility are deferred. HOME+Docker delivery, local service catalog, target-ready tools and no-GitHub supply chain are required, not deferred. Universal IDE/ETO service/model exposure, dirty-buffer analysis, semantic package rename and headless conformance are also required. IDE/plugin/MVP planning and actual client implementation are explicitly deferred by D-025.

Детали дополнительных clients Gemini/Copilot вторичны и не блокируют принятую четвёрку.

## 10. Evidence map

V = C:/Users/olegc/git/v/vibevm. Paths ниже точные относительно V; line numbers — locators, нормативные ссылки — существующие anchors.

| ID | Primary source |
|---|---|
| E01 | crates/vibe-lifecycle/src/chain.rs:76; vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml#INVOKE-RUNS-PRIORS, #PHASE-CREATE, #PHASE-VERIFY |
| E02 | crates/vibe-lifecycle/src/runner.rs:237; crates/vibe-lifecycle/src/runner/hosted.rs:47; crates/vibe-lifecycle/src/state/fingerprint/inputs.rs:172 |
| E03 | crates/vibe-lifecycle/src/runner/verify.rs:72; crates/vibe-orchestrator/src/dispatch/verify.rs:113; PROP-054 #VERIFY-EVIDENCE-IDENTITY, #EVIDENCE-MEASUREMENT-CARRIAGE, #VERIFY-CURRENT-PREFIX |
| E04 | crates/vibe-lifecycle/src/lease.rs:1; crates/vibe-lifecycle/src/state/store.rs:91; crates/vibe-lifecycle/src/state/io.rs:27; crates/vibe-orchestrator/src/install/lease.rs:33; crates/vibe-lifecycle/src/agent/contract.rs:26 |
| E05 | crates/vibe-requirements/src/query.rs:122,188,241; crates/vibe-requirements/src/rows.rs:27; PROP-054 #REF-REQUIREMENTS-WIRE, #REF-REQUIREMENTS-SURFACES |
| E06 | crates/progress-core/src/terminal.rs:55,88,145; crates/progress-core/src/evidence.rs:101; vibevm/vibespecs/modules/vibe-facts/PROP-043-facts-markup.xml#terminal-artifacts |
| E07 | crates/vibe-resolver/src/features.rs:30,56,80,255; crates/vibe-core/src/manifest/package/features.rs:28; crates/vibe-core/src/manifest/package/capabilities.rs:71; crates/vibe-core/src/manifest/package/wire.rs:38,305 |
| E08 | crates/vibe-install/src/plan.rs:219,305,338,371,432; crates/vibe-install/src/record.rs:221; crates/vibe-install/src/plan/fetch.rs:24,161; crates/vibe-install/src/fetched.rs:92 |
| E09 | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml#features, #KEEP-OPTIONAL-DEP; crates/vibe-cli/src/commands/show/features.rs:31 |
| E10 | crates/vibe-package-source/src/source.rs:187; crates/vibe-install/src/visibility_projection.rs:110,198,205; crates/vibe-resolver/src/resolvo_engine/provider.rs:258; crates/vibe-resolver/src/resolvo_engine/capabilities.rs:56; crates/vibe-resolver/src/resolvo_engine/mod.rs:173,210; crates/vibe-resolver/src/lib.rs:216 |
| E11 | crates/vibe-spec/src/address.rs:95,120,165; crates/vibe-spec/src/resolver/lookup.rs:36,65,302; crates/vibe-spec/src/resolver.rs:84,339,407 |
| E12 | crates/vibe-spec/src/embed.rs:280,331; crates/vibe-spec/src/doctree.rs:116,205,412; crates/vibe-spec/src/doctree/invariant.rs:325 |
| E13 | crates/vibe-install/src/slot_verify.rs:55,76,93,134,139; crates/vibe-workspace/src/vibedeps/slot_record.rs:22,50,224; crates/vibe-workspace/src/vibedeps/slot_cow.rs:33 |
| E14 | crates/vibe-safefs/src/file/bounded.rs:73; crates/vibe-safefs/src/file.rs:182; crates/vibe-safefs/src/file/stream.rs:12; crates/vibe-safefs/src/component.rs:52; crates/vibe-specdoc/src/load.rs:95,125; crates/vibe-specdoc/src/xml_in.rs:96; crates/vibe-lifecycle/src/agent/resolver.rs:81 |
| E15 | crates/vibe-spec/src/compiler/ir.rs:112,422; crates/vibe-spec/src/compiler/pass_tier/frontend.rs:54,179; crates/vibe-workspace/src/install/bootgen/analyze.rs:83; crates/vibe-workspace/src/boot_artifacts/analyzed.rs:15; crates/vibe-workspace/src/extension_world.rs:94,262,331 |
| E16 | crates/vibe-agent-projection/src/agents.rs:140,224,233,245,317,398,507; crates/vibe-agent-projection/src/agents/home_paths.rs:58 |
| E17 | crates/vibe-agent-projection/src/pkgskill.rs:115,479,578; crates/vibe-agent-projection/src/pkgskill/projection.rs:333,359,388,433,487; crates/vibe-agent-projection/src/pkgskill/projection/binding.rs:107,128; crates/vibe-agent-projection/src/pkgskill/receipt/reconcile.rs:48,126 |
| E18 | crates/vibe-lifecycle/src/mechanism/deploy/model.rs:244; crates/vibe-lifecycle/src/mechanism/deploy/protocol.rs:525; crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs:44; crates/vibe-lifecycle/src/mechanism/deploy/plugin/opencode.rs:45,153,261; crates/vibe-lifecycle/src/mechanism/deploy/plugin/wire.rs:36,103; crates/vibe-lifecycle/src/mechanism/deploy/skill.rs:206 |
| E19 | C:/Users/olegc/git/v/packages/org.speckit.speckit/vibe.toml:4,16,36; C:/Users/olegc/git/v/packages/org.speckit.speckit/skills/speckit-core/SKILL.md:11; C:/Users/olegc/git/v/packages/org.speckit.speckit/README.md:3; C:/Users/olegc/git/v/packages/com.external-skills.skills/vibe.toml:4,24,29; V/docs/authoring-feat.md:3 |
| E20 | .github/workflows/release-distributions.yml:43; xtask/src/dist/build.rs:36; xtask/src/dist/release.rs:283; xtask/src/dist/release/tests.rs:365 |
| E21 | xtask/src/dist/release.rs:301; crates/vibe-publish/src/release_manifest.rs; rust-toolchain.toml:2; formats/EPOCHS.toml:4 |
| E22 | vibevm/vibespecs/common/PROP-044-change-native-formats.xml#THE-FREEZE-MODEL, #TERMS-SNAPSHOT-FROZEN-CHANNEL; crates/vibe-core/src/manifest/package.rs:106; vibevm/vibepacks/org.vibevm.world/redbook/v1.0.0/vibe.toml:62; vibe.toml:31 |
| E23 | AGENTS.md; vibevm/vibespecs/boot/STATIC.xml; vibevm/vibespecs/boot/INDEX.md + восемь entry files; vibevm/vibespecs/common/PROP-048-tokenomics.xml#STATIC-HARDENING-WAVE, #STATIC-POSITION-LAW |
| E25 | crates/vibe-scrape/src/inventory.rs:9,22,77 |
| E24 | VIBEVM-SPEC.md:56,448,1254,1582; crates/vibe-cli/src/cli/lifecycle.rs:13 |
| E26 | crates/vibe-specdoc/src/load.rs:75,125,170; crates/vibe-facts/src/scan.rs:236,268,303,418; crates/vibe-facts/src/sync.rs:143,156,241; crates/vibe-spec/src/resolver/lookup.rs:41,65,132; crates/vibe-workspace/src/vibedeps/derived.rs:407 |
| E27 | crates/vibe-lifecycle/src/mechanism/deploy/model.rs:133; crates/vibe-lifecycle/src/mechanism/deploy/protocol.rs:215; crates/vibe-lifecycle/src/mechanism/deploy/plugin/opencode.rs:45,75; crates/vibe-agent-projection/src/agents.rs:140 |
| E28 | [Qwen Code extension reference](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/); [Qwen Agent Skills](https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/). Official docs describe extension/skill/MCP/context integration; current VibeVM runtime support was not tested or inferred from docs |
| E29 | Owner rulings transcribed in §8 and the plan §12; accepted/open status explicit; these govern scope/policy, not current implementation facts |
| E30 | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml:107; vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml:55; crates/vibe-core/src/manifest/package/binary.rs:38; crates/vibe-core/src/manifest/artifact/binary_projection.rs:74; crates/vibe-lifecycle/src/mechanism/build.rs:221,342; crates/vibe-lifecycle/src/mechanism/package.rs:238; crates/vibe-lifecycle/src/mechanism/record.rs:132 |
| E31 | crates/vibe-cli/src/main.rs:291; crates/vibe-cli/src/commands/bin.rs:12,65,80; crates/vibe-workspace/src/bins.rs:197,270,294; crates/vibe-workspace/src/bins/build.rs:62,169. Verified public routing and source call chain; no CLI execution test |
| E32 | crates/vibe-lifecycle/src/mechanism/deploy/model.rs:221; crates/vibe-lifecycle/src/mechanism/deploy/state.rs:121; crates/vibe-lifecycle/src/mechanism/vibebin.rs:166,396,446; crates/vibe-lifecycle/src/mechanism/vibebin/launcher.rs:109; crates/vibe-lifecycle/src/mechanism/vibebin/store.rs:237,325; crates/vibe-core/src/manifest/target_when.rs:25,54; crates/vibe-lifecycle/src/native/platform.rs:13,49 |
| E33 | crates/vibe-lifecycle/src/mechanism/vibebin/store.rs:63,132; crates/vibe-lifecycle/src/mechanism/vibebin/launcher.rs:153; crates/vibe-lifecycle/src/mechanism/vibebin/config.rs:27; crates/vibe-install/src/record.rs:200; crates/vibe-install/src/plan.rs:39,191; crates/vibe-core/src/package_ref/kind.rs:31 |
| E34 | crates/vibe-cli/src/commands/uninstall.rs:30,130,139,164; crates/vibe-core/src/manifest/package/hooks.rs:35; vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml:54,166; crates/vibe-lifecycle/src/mechanism/deploy.rs:347; crates/vibe-lifecycle/src/mechanism/deploy/native.rs:81,94,205; vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml:406,408 |
| E35 | [pacman/libalpm, root/sysroot and ownership](https://man.archlinux.org/man/pacman.8.en). External backend semantics, not evidence of Vibe integration |
| E36 | [ALPM transaction hooks](https://man.archlinux.org/man/alpm-hooks.5.en); [repository order and signature policy](https://man.archlinux.org/man/pacman.conf.5.en) |
| E37 | [Arch full-upgrade contract](https://wiki.archlinux.org/title/System_maintenance); [Arch repository snapshots](https://wiki.archlinux.org/title/Arch_Linux_Archive). Direct page loading was denied; official indexed excerpts were available and checked; no runtime repository/pacman probe |
| E38 | [Buildx build target/platform and output](https://docs.docker.com/reference/cli/docker/buildx/build/); [OCI/Docker exporters](https://docs.docker.com/build/exporters/oci-docker/) |
| E39 | [Build secrets](https://docs.docker.com/build/building/secrets/); [Build attestations and driver/store retention](https://docs.docker.com/build/metadata/attestations/) |
| E40 | [Build cache invalidation](https://docs.docker.com/build/cache/invalidation/); [base image digest pinning](https://docs.docker.com/build/building/best-practices/). External execution/cache contracts; exact Vibe image reproducibility unverified |
| E41 | [Container model and shared kernel](https://docs.docker.com/get-started/docker-concepts/the-basics/what-is-a-container/); [Docker engine security boundary](https://docs.docker.com/engine/security/) |
| E42 | [OpenAPI operation/server/security descriptions](https://spec.openapis.org/oas/v3.1.2.html); [MCP security practices](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices). Reusable service contracts, not automatic endpoint trust |
| E43 | Local read-only commands: docker version --format JSON; docker info selecting only OSType/Architecture/ServerVersion; docker buildx version. Client 29.2.0 Windows/amd64, context desktop-linux; Linux engine named pipe absent, no server result; Buildx v0.31.1-desktop.1. No engine started, image pulled/built or container launched |
| E44 | [Nix store paths and prefix constraints](https://nix.dev/manual/nix/2.34/store/store-path.html); [Nix profiles](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-profile.html); [Nix source references](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-flake.html); [Nix sandbox/cache configuration](https://nix.dev/manual/nix/2.34/command-ref/conf-file.html). Architectural reference only; no Nix installation or mandatory GitHub claim |
| E45 | vibevm/vibespecs/common/PROP-032-project-model-ide-substrate.xml:4,34,45,120,234; vibevm/vibespecs/common/PROP-031-algorithmic-refactoring.xml:4,50,94,101,186; crates/vibe-cli/src/main.rs:213; crates/vibe-cli/src/commands/refactor/mod.rs:27,154. Proposal versus actual public grammar separated |
| E46 | crates/vibe-cli/src/commands/tree/mod.rs:15,40,134,263; crates/vibe-cli/src/commands/tree/model.rs:23,56,79,208,246; crates/vibe-cli/src/commands/tree/build.rs:41,77,195,315,400,440,515,520; crates/vibe-cli/src/commands/tree/diagnostics.rs:27. CLI-local summary/read handling/provenance, not complete snapshot/reference proof |
| E47 | crates/vibe-mcp/src/tools.rs:54; crates/vibe-mcp/src/lib.rs:65,209,232,250,294,311; crates/vibe-mcp/src/transport.rs:37,68; crates/vibe-mcp/src/context.rs:37,136 |
| E48 | crates/vibe-mcp/src/tools/lifecycle_run.rs:99; crates/vibe-mcp/src/tools/lifecycle_run/ports.rs:47; crates/vibe-mcp/src/tools/lifecycle_tasks.rs:48; crates/vibe-cli/src/commands/lifecycle/observer.rs:52; crates/vibe-cli/src/commands/tools.rs:18; vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml:286 |
| E49 | crates/vibe-mcp/src/tools/query.rs:43,73; crates/vibe-mcp/src/tools.rs:109,438; crates/vibe-mcp/src/tools/output.rs:22; crates/vibe-mcp/src/lib.rs:311; crates/vibe-mcp/src/tools/requirements_query.rs:139 |
| E50 | crates/vibe-spec/src/compiler/trace.rs:80,161,367; crates/vibe-workspace/src/compile_trace.rs:73,140,149,251,291; crates/vibe-workspace/src/compile_trace/open.rs; crates/vibe-spec/src/compiler/ir.rs:112. Current observation/writer guarantees do not imply a complete read API |
| E51 | [LSP 3.17 reference](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/), [official LSP metamodel](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/metaModel/metaModel.json); [DAP overview](https://microsoft.github.io/debug-adapter-protocol/overview.html); [MCP lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle). Reference for adapter boundaries/negotiation; no shipped Vibe LSP/DAP/IDE compatibility claim. Metamodel includes proposed entries, which are not assumed stable |

Boot measurement recipe: exact file byte lengths; INDEX paths в указанном порядке. Rename region — от literal RENAMED ANCHORS comment start до closing marker включительно. Измерение не исполняло compiler или providers.

## 11. Uncertainties and required verification

**Unverified; будущие proofs, не completed gates:**

- Native Windows/Unix path races, reparse/hardlink, source→converted membership; two-pass observation не universal atomic snapshot.
- Workspace conflicting roots, feature Fresh bypass и candidate isolation на всех solver modes.
- Create→second verify convergence при изменённых predecessor artifacts.
- Exact current-version census и source-rule reconciliation: собственные active products/packages 1.0.0, чужие upstream/history сохраняют реальные identities; generic frozen enforcement нельзя считать доказанным по одному field.
- Exact upstream template/script/resource closure реального Spec Kit pin.
- Automatic binding hydrated source_root versus provider.root: concrete characterization, не proven runtime failure.
- Четыре native artifact smoke scenarios, explicit skipped path, producer authentication, mutable-publication recovery, A/B policy parity и отказ без authority.
- Tokenizer-specific counts, exact/unknown marginal preview, source attribution после whole-lane transforms, большой бутлейн без total quota; provider cache telemetry отдельно.
- Brownfield capture, generic derivative resource matrix и real external adapters для четырёх клиентов. Native handler sandbox не доказан и не обещан.
- Source classification across scanner/sync/lookup/converter, ordinary XML/Markdown coexisting stems и byte-preserving bridge resources.

- Target-root/principal replay, alias locking, complete payload closure and legacy-bin migration remain runtime proof obligations.
- Reverse-dependency removal, generation activation, GC liveness and foreign backend partial recovery are not established by current single-file receipts.
- HOME prefix relocation, hermetic builds, binary cache substitution and the complete no-GitHub bootstrap/build/install/image supply chain remain unverified.
- Docker engine was unavailable during read-only probe; no image/container/network test executed. Buildx driver/exporter/attestation support must be probed again when the engine is running.
- Pacman backend integration, coherent Arch snapshot updates and local service catalog are planned. Full Linux boot/reboot/upgrade recovery requires future VM evidence.

- Universal capability/model coverage, strict all-path schemas and removal of CLI/TUI-only semantic actions remain implementation/verification work.
- Current tree summary requires snapshot/provenance/reference-completeness correction; current refs marked false were not all resolved.
- Long-action responsiveness, operation deduplication, reconnection/control, dirty-buffer isolation and saved-state mutation are unverified as a universal service.
- Package-coordinate rename has no proven public implementation; exact reference universe, generated regeneration, journal recovery and historical/foreign identity preservation require new proofs.
- Existing trace proves accepted compiler snapshots, not all-domain ETO. Pure retained inspection, captured extension schemas, configurable detail/retention and explicit missing data remain necessary.

## 12. Changes in this revision

Revision 4 makes Extreme Total Observability and universal IDE-compatible operation/model access explicit owner requirements D-025/D-026. Existing decisions remain; D-022 is still open.

New F-029…F-034 distinguish the existing CLI tree, MCP tools and compiler trace from a complete external interface. Verified gaps include human-only provenance, incomplete/unchecked tree references, serial MCP dispatch, hidden startup restrictions, inconsistent schema validation and text-only failures after possible partial writes. Package rename remains proposed rather than shipped.

M-16 adds early capability/schema/service/context/overlay/event/operation contracts. M-17 completes all supported feature/internal-model exposure, semantic package rename and headless full-path conformance across packages/boot/lifecycle/HOME/Docker/services. Existing domain milestones gain explicit coverage obligations; no monolithic replacement graph or new scheduler is introduced.

No IDE/plugin/MVP is planned or implemented. Protocol references define future adapter compatibility without making LSP/DAP the whole product API. Only this canonical document pair changes; no product execution tests, Docker operations, publication, source/spec/manifests or stewardship edits were performed.

<!-- review-protocol: 1 -->
<!-- pair-revision: 4 -->
<!-- baseline-fingerprint: sha256:0b6524d4128c5ece0763b14f791cbdc3e9cc4c6fd30aefa14b4f93df12892084 -->
