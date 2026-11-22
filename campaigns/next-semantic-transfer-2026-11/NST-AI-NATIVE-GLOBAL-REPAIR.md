# NST-AI-NATIVE-GLOBAL-REPAIR — final global coherence and quality pass

Status: planned. Owner request2026-11-18. Place this task at the very end of NST,
after its existing stages. This planning request does not start execution or change
the currently selected PRIOR-APP-STEPS implementation. NST-ZAP-CLEANUP is removed
from the current adaptive plan; it is not a dependency or deferred subtask here.

## Owner description — verbatim

> сейчас написанное в ходе реализации кампании NST и ранее начиная с релиза 1.0.0 очень плохо покрыто принципами AI-Native Rust. Многие методы не покрыты тестированием. Это  хорошо для прототипирования, но плохо если хочется сохранить качество и когерентность со спецификациями в большой перспективе. Необходимо составить полный список измененных файлов, и сделать по ним большой проход, который будет идти по коду и проставлять для функций аннотации на соответствующие пункты спецификаций. В ходе этого может оказаться, что часть кода не связана ни с какими спецификациями, и это просто ошибочный код, либо ошибка в спецификации. Либо наоборот - описанной задачи нет ни в спеке, ни в выполненном плане, ни в полном плане (который на 222 задачи) - это значит что реализация требования пропущена. Часть кода или спек может оказаться просто устаревшим мусором. Нужно провести глобальную починку, вычистку и вычистку всего сделанного.

## Ordered child tasks — owner amendment2026-11-18

> у нас есть отдельная задача про NST-AI-NATIVE-GLOBAL-REPAIR. Разбей ее на подзадачи. Первой там будет идеальная синхронизация, второй - я прошу пройти по механизмам vibe install -g и удалить всё легаси в задачах касающихся NST (пока это все еще возможно, потому что у нас нет у этих задач пользователей). И вот только третьей задачей и дальше - синхронизация спецификаций и так далее. И под конец финальная идеальная синхронизация.

Create these real child nodes under `NST-AI-NATIVE-GLOBAL-REPAIR` in the authoritative multi-user-planning plan, not merely a prose checklist. All are initially planned. Dependencies form the explicit sequence below; existing NST prerequisites of the parent remain. These derived children do not change the original222-task denominator or erase the original mandate.

| Order | Stable child ID | Outcome | Depends on |
|---|---|---|---|
| 1 | `NST-AI-NATIVE-GLOBAL-REPAIR-SYNC-START` | Initial ideal synchronization of architect and builder under WORKING-RULES, with reviewed shared functional/tree baseline | Existing prerequisites of parent |
| 2 | `NST-AI-NATIVE-GLOBAL-REPAIR-INSTALL-G-LEGACY` | Inspect NST-related `vibe install -g` mechanisms and remove obsolete parallel systems, compatibility surfaces and maintained legacy declarations in favor of current systems | `…-SYNC-START` |
| 3 | `NST-AI-NATIVE-GLOBAL-REPAIR-RECONCILE` | Reproducible post1.0.0 inventory and bidirectional reconciliation of owner requirements, original222 tasks, plans, specifications and code | `…-INSTALL-G-LEGACY` |
| 4 | `NST-AI-NATIVE-GLOBAL-REPAIR-TRACE-TEST` | Detailed important-function specmark relations and behavior-oriented tests for actual specified user journeys | `…-RECONCILE` |
| 5 | `NST-AI-NATIVE-GLOBAL-REPAIR-REPAIR` | Repair demonstrated missing/incorrect behavior and obsolete specifications/code at their actual owners | `…-TRACE-TEST` |
| 6 | `NST-AI-NATIVE-GLOBAL-REPAIR-FINAL-GATES` | Integrated architectural review, designated comprehensive checks and truthful acceptance report/physical-proof dispositions | `…-REPAIR` |
| 7 | `NST-AI-NATIVE-GLOBAL-REPAIR-SYNC-END` | Final ideal synchronization of the accepted result, plans and functional delivery between both repositories | `…-FINAL-GATES` |

The parent closes only after all children reach their justified terminal dispositions and final synchronization is verified. The product-first planning law applies: these are outcomes, not an invitation to recursively perfect local nodes or invent extra campaigns. Refresh the task tree only after updating the authoritative source nodes/dependencies/acceptance.

## Release1.0.12 product lens — external graphical package-manager client

Owner clarification2026-11-19 — verbatim:

> когда будешь выполнять NST-AI-NATIVE-GLOBAL-REPAIR держи в уме, что одна из ключевых задач релиза 1.0.12 это то, чтобы все описанные нами фичи работали, будучи запущенными из внешнего красивого графического приложения (которое мы делаем параллельно в совершенно другом проекте и куда у тебя пока нет доступа). Это приложение очень похоже на Steam и использует наш vibe как продвинутый пакетный менеджер. Наш приоритет - на хорошем продуманном пользовательском пути из приложений подобного рода. Просто подумай, что было бы, если бы Steam использовал нас как пакетный менеджер. Расширять скоуп уже решенных задач не нужно - но это полезный фильтр, сквозь который стоит смотреть, когда ты будешь чинить спецификации.

Treat VibeVM as the advanced package-management backend of a separate graphical application, not merely a CLI someone types into a terminal. The external project is inaccessible here; do not claim its implementation, contracts or actual integration have been inspected. The Steam analogy is a product-use lens, not a request to recreate Steam or to build a GUI in this repository.

Apply this lens across the existing reconciliation, traceability, repair and final-review children: can a graphical client lead a user through the already specified discover/select/plan/install, meaningful progress and complete logs, consent or required input, cancel/resume/recovery, launch/stop, update/reinstall/uninstall and library/unavailable-storage journeys without relying on a hidden interactive terminal? Distinguish operation outcome, partial effects, actionable failure and current state; inspect whether existing machine-facing contracts give a client enough stable identity and usable evidence to present that experience. These are review questions about already owed behavior, not automatic new endpoint, event, queue, dashboard or schema requirements.

Keep capabilities in their existing shared library owners and declared adapters, as the omnichannel contract requires. Repair concrete discrepancies at those owners; do not implement a second GUI-specific resolver, lifecycle or package manager. A good CLI experience alone does not prove external graphical usability, but mock/client-contract evidence must not be presented as an actual run of the inaccessible application. Record any genuinely missing external contract as a precise bounded question rather than guessing its private design.

Do not expand or reopen completed tasks solely to perfect a hypothetical graphical interaction. Preserve accepted unchanged evidence and original scope; when existing promised behavior has an actual client-facing gap, identify that obligation and its user impact before proposing a repair. At final review report how the retained contracts support the complete graphical-client journey and disclose any unverified real-client integration. The existing seven-child ordering and owner acceptance exceptions remain unchanged.

## Architect additions — scope and method

### 1. Initial ideal synchronization — `…-SYNC-START`

Owner instruction2026-11-18 — verbatim:

> в задачу NST-AI-NATIVE-GLOBAL-REPAIR напиши, что перед тем как делать починку, нужно сделать "идеальную синхронизацию" в смысле WORKING-RULES, чтобы у нас не было расхождения репозиториев между vibevm-architect и vibevm-builder

Before beginning the repair pass, complete **ideal synchronization** between
`vibevm-architect` and `vibevm-builder` under `WORKING-RULES.md` §Ideal synchronization,
the applicable project policy and local bindings. This is a mandatory entry gate,
not a claim that matching HEADs alone establishes synchronization.

Coordinate a stable snapshot with both sessions and active workers; verify actual
repository roots/common directories, branches and full commit hashes. Inventory
staged, modified and untracked product work on both sides, preserve exclusions and
account for unfinished candidates. Through each session's own checkout and the
configured integration owner, reach the same agreed integration commit/tracked tree
and completed functional delivery, with checks and remaining limitations reconciled.
No peer-checkout writes, dirty product copying, discarded work or automatic metadata
repair are permitted. An unavailable peer, conflicting writer or unfinished work
preventing that target means partial/blocked synchronization, not an accepted gate.

Record both final hashes, actual tree states, preserved exclusions, integrated scope
and verification evidence in the existing task/campaign record; only then freeze
the coherent inventory and start repair. New divergence affecting that accepted
baseline requires reconciliation before the affected repair proceeds. External
publication is separate and is not authorized by this prerequisite. Recording this
future gate does not invoke synchronization now or interrupt PRIOR-APP-STEPS.

### 2. Remove legacy from NST-related `vibe install -g` mechanisms — `…-INSTALL-G-LEGACY`

After the initial ideal synchronization, walk the actual complete global-install user journey and its NST-owned dependencies: application admission/selection, preparation and contributions, install/update/reinstall/uninstall, launchers/proxies, storage, execution/evidence and public CLI/API consumers. Identify old versus current mechanisms by actual responsibility and active callers, not a substring search for the word legacy. The owner explicitly permits removing compatibility for these still-unused NST surfaces rather than keeping two systems alive.

Replace maintained owned declarations/fixtures/docs with the current systems, remove obsolete parsers, synthetic adapters, independent old runners, compatibility-only branches/APIs/state carriage and active obsolete intentions. The package `[hooks]` mechanism and proposed legacy-only quiet-output shim are known concrete candidates, not the entire scope. Inspect application installer-option sugar and other NST global-install adapters on their actual merits; do not silently exempt a genuinely obsolete NST mechanism merely because it was outside the earlier narrow hook brief. Preserve reusable neutral capabilities such as interpreter selection and valid modern materialisation/lifecycle behavior. Reject retired inputs actionably before effects, with no no-op facades or silent fallback. Do not redesign already sound current systems merely to make a larger cleanup.

Prove complete ordinary install-g journeys through the retained modern path, including failure/partial effects, quiet/JSON/full logs, cancellation/resume and owned data preservation. Preserve security/privacy, third-party/reference ownership and actual data; no external users is permission to retire compatibility, not permission to destroy unrelated work or expose credentials. Record the finite mechanism→replacement→removed-consumers/test disposition inventory. Minimal co-incident contract/format amendments needed to implement retirement belong to this child; the comprehensive specification synchronization and global scan begin only at child3. Do not leave active contract prose demanding removed behavior.

### 3. Inventory and bidirectional specification reconciliation — `…-RECONCILE`

#### Freeze an exhaustive, reproducible inventory

Resolve the actual release1.0.0 source commit and the coherent end-of-NST candidate
from Git evidence; do not guess from a version string. Produce a complete changed-
file inventory covering intervening work,including NST and earlier post1.0.0 changes.
Distinguish added,modified,deleted,moved,generated,vendored and test/spec/format files.
Account for changes later reverted or superseded when reconstructing obligations;
net diff alone must not silently discard a still-live requirement. Record exclusions
with concrete ownership/reason; generated or dependency copies are not independent
normative sources. Do not modify third-party references as if they were owned code.

#### Reconcile in both directions

Build an auditable relation between original owner requirements,the complete222-task
architecture,executed/adaptive plans,canonical specification facts,implementation
functions and executable tests. Do not alter the historical222 denominator merely
to make coverage green. Extra derived review atoms are allowed separately.

Walk code→spec and requirement/spec/plan→code/tests. Classify every unmatched case:
valid infrastructure with an appropriate governing contract; incorrect/orphan code;
missing or contradictory specification; missing implementation; missing tests;
obsolete/superseded code or specification;or a genuine owner decision needed.
Lack of an annotation is a finding to investigate,not proof that code should be
removed. Equally,an implementation claim or task status is not executable evidence.

### 4. Apply meaningful AI-Native Rust traceability and tests — `…-TRACE-TEST`

Owner clarification2026-11-18 — verbatim:

> я хочу достаточно подробную расстановку specmark маркеров на всех важных функциях, которые связаны со спецификациями

Provide sufficiently detailed function-level `specmark` annotations on ALL important
in-scope functions whose behavior relates to specifications,including private/internal
functions that implement meaningful rules. File/module-level scope markers alone do
not satisfy this requirement. Link each important function to the precise applicable
facts and use the appropriate relation; where it implements several distinct obligations,
record those relations rather than one generic umbrella anchor. Review omissions
explicitly: visibility or a low annotation count is not an excuse to skip important
specified behavior. This does not require meaningless markers on trivial plumbing.

Use the installed project's AI-Native Rust conventions and canonical annotation/
reference tooling. Give reviewed functions the correct relation to exact owning
specification anchors,including meaningful implements/verifies/documents/deviates
relations as applicable. Do not spray broad anchors onto functions to satisfy a
counter,create fake facts merely to bless accidental code,or erase valid obligations
to match an incomplete implementation. Preserve protected contract authority.

Review behavior-oriented tests for each meaningful changed owner/branch/invariant:
positive paths,negative/refusal paths,partial-effect reporting,cancellation,privacy,
compatibility and the named consumers affected. Unit tests,property tests,integration
and actual platform evidence have distinct proof boundaries. Prioritize high-impact
untested behavior and missing requirements,not only easy numerical coverage.

### 5. Repair and remove demonstrated defects and obsolete surfaces — `…-REPAIR`

Fix implementation,specifications and tests together at their actual owners. Remove
verified obsolete code/specs and misleading active intentions instead of leaving
zombie alternatives that future agents may follow. Preserve necessary historical
decision/evidence records and real compatibility contracts; do not delete unrelated
user work,private data or retained execution evidence as a side effect.

Break the large pass into reviewable ownership-based batches with independent
verification and cross-cutting architectural review. Workers provide evidence;
central acceptance remains explicit. Avoid simultaneous incompatible writers.
No broad rewrite solely for aesthetics,no second resolver/transaction framework,
and no waiver/baseline update that hides newly introduced defects.

### 6. Acceptance and final evidence — `…-FINAL-GATES`

- Mandatory pre-repair ideal synchronization completed and evidenced for both
  architect and builder repositories under WORKING-RULES; matching HEADs without
  reconciled tracked/dirty work, functional delivery and checks is insufficient.
- Complete file inventory tied to exact baseline/candidate commits and reconciled
  against historical task/requirement sources.
- Every reviewed in-scope implementation function has a justified traceability
  disposition; exact annotations resolve and describe actual behavior. All important
  spec-related functions carry sufficiently detailed function-level `specmark`
  relations; module scopes or generic umbrella links do not substitute for them.
- Bidirectional matrix explains coverage of all222original contracts and additional
  owner mandates without fabricating completeness from one directional scan.
- All discovered missing requirements,orphan behavior,contradictions,untested
  important methods and obsolete surfaces have repaired,removed or explicit owner-
  disposition outcomes. Unresolved actual obligations remain visible,not green.
- Canonical annotation/reference validation,affected tests,conform/codegen/wire and
  designated final comprehensive gates run on the coherent committed candidate.
  Actual native/provider/manual checks are not inferred from compilation or mocks.
- Final report distinguishes repaired findings,retained justified deviations,
  historical evidence,executed/failed/unrun checks and any remaining owner decisions.
  Source/adaptive plan and published tree agree with verified evidence.

### 7. Final ideal synchronization — `…-SYNC-END`

After integrated repair acceptance and final gates, perform ideal synchronization again under WORKING-RULES. Both sessions verify the exact final integration revision/tracked tree, completed functional scope, relevant checks, remaining owner dispositions and accounted staged/modified/untracked candidates. Preserve machine-local exclusions and private state. Each session updates only its own checkout; matching HEAD alone is insufficient. Reconcile source plans and the living physical acceptance guide with the accepted result, then publish and verify the derived task tree from its authoritative source. Record both final hashes and actual tree states. An unavailable peer, unaccounted product delta or unresolved functional divergence leaves this final child open and prevents parent closure. No external push is implied.

## Mark legacy as it is encountered — do not improve it now

Owner instructions2026-11-18: do not spend current work improving legacy behavior that will soon be deleted; mark the behavior for removal as it is encountered. This current-work restriction applies before the global repair starts. It does not itself start child2 early.

Record concrete discoveries in the existing campaign ledger [`NST-LEGACY-REMOVAL.md`](NST-LEGACY-REMOVAL.md), with stable ID, behavior, exact owner paths/symbols and old tests/contracts, current replacement, user impact, target removal child and disposition. Where source is already being edited in an authorized task, add a concise nearby `LEGACY-REMOVE` comment referencing that stable ledger ID; do not sweep untouched files merely to add comments. The comment is a removal pointer, not a new normative contract or evidence of completed deletion. Retire/update markers and ledger together when the behavior is actually removed. Do not confuse a historical word or reusable neutral capability with a removal candidate.

Do not add compatibility flags, legacy-origin metadata, output relays, fallback runners, no-op facades or more tests solely to perfect marked legacy. Existing failed old-compatibility assertions must be reported as superseded by the owner's decision, not converted to PASS or treated as obligations to restore that behavior. Continue protecting actual current-system user journeys and security/data integrity. Reflect current holds/removal dispositions in authoritative planning before refreshing the task tree.

## Authorization boundaries

This is the final NST quality/coherence task,not permission for an immediate cleanup
of run directories or for NST-ZAP-CLEANUP. External publication,releases,history
rewrites,credential/configuration access and irreversible unrelated operations retain
their ordinary approval requirements. At execution time inspect actual source and
then refine the batch route; do not turn this description into a new unbounded
prerequisite campaign before the currently selected work can proceed.
