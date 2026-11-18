# NST-AI-NATIVE-GLOBAL-REPAIR — final global coherence and quality pass

Status: planned. Owner request2026-11-18. Place this task at the very end of NST,
after its existing stages. This planning request does not start execution or change
the currently selected PRIOR-APP-STEPS implementation. NST-ZAP-CLEANUP is removed
from the current adaptive plan; it is not a dependency or deferred subtask here.

## Owner description — verbatim

> сейчас написанное в ходе реализации кампании NST и ранее начиная с релиза 1.0.0 очень плохо покрыто принципами AI-Native Rust. Многие методы не покрыты тестированием. Это  хорошо для прототипирования, но плохо если хочется сохранить качество и когерентность со спецификациями в большой перспективе. Необходимо составить полный список измененных файлов, и сделать по ним большой проход, который будет идти по коду и проставлять для функций аннотации на соответствующие пункты спецификаций. В ходе этого может оказаться, что часть кода не связана ни с какими спецификациями, и это просто ошибочный код, либо ошибка в спецификации. Либо наоборот - описанной задачи нет ни в спеке, ни в выполненном плане, ни в полном плане (который на 222 задачи) - это значит что реализация требования пропущена. Часть кода или спек может оказаться просто устаревшим мусором. Нужно провести глобальную починку, вычистку и вычистку всего сделанного.

## Architect additions — scope and method

### 1. Freeze an exhaustive, reproducible inventory

Resolve the actual release1.0.0 source commit and the coherent end-of-NST candidate
from Git evidence; do not guess from a version string. Produce a complete changed-
file inventory covering intervening work,including NST and earlier post1.0.0 changes.
Distinguish added,modified,deleted,moved,generated,vendored and test/spec/format files.
Account for changes later reverted or superseded when reconstructing obligations;
net diff alone must not silently discard a still-live requirement. Record exclusions
with concrete ownership/reason; generated or dependency copies are not independent
normative sources. Do not modify third-party references as if they were owned code.

### 2. Reconcile in both directions

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

### 3. Apply meaningful AI-Native Rust traceability and tests

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

### 4. Repair and remove demonstrated defects and obsolete surfaces

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

### 5. Acceptance and final evidence

- Complete file inventory tied to exact baseline/candidate commits and reconciled
  against historical task/requirement sources.
- Every reviewed in-scope implementation function has a justified traceability
  disposition; exact annotations resolve and describe actual behavior.
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

## Authorization boundaries

This is the final NST quality/coherence task,not permission for an immediate cleanup
of run directories or for NST-ZAP-CLEANUP. External publication,releases,history
rewrites,credential/configuration access and irreversible unrelated operations retain
their ordinary approval requirements. At execution time inspect actual source and
then refine the batch route; do not turn this description into a new unbounded
prerequisite campaign before the currently selected work can proceed.
