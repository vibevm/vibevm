# v2 — rooted physical witness succeeds; activation grounding still fails

Private probe `/fast/tmp/nst-transitive-proof`, same resolvo0.11/main-core path dependency, same PRIVATE target and unchanged baseline Cargo.lock. Root archived v1 and verified all24hashes before this refinement. Old logs were not overwritten; v1-log-preservation.json checks their hashes. No production source/Git/network/toolchain/dependency change or additional solver was introduced. The root-supplied grounding-invariants memo was read: main imposes no package DAG invariant, and ordinary output dependencies omit some recommendation/OR/capability support. This finite fixture does not claim to cover those routes.

## Actual outcome

First v2 command:4PASS/1intendedFAIL, exit101. Final current-source command runs v1 and v2 including every ignored negative test:8PASS/4intendedFAIL/0ignored, exit101. Three retained v1 failures demonstrate incomplete earlier models; the new failure is the requested unseeded activation cycle. It remains unfixed and bounds the hypothesis. No failure was converted into a green assertion or repaired selected result.

### Physical presence is separated from rooted witness

`src/v2.rs:11` expands only the hand-enumerated real fixture states into tiny rank variants: actual root names have rank0; other real states have ranks1..N−1; solver support helpers have no rank. All states of one real package still share one NameId, preserving one actual instance. Every nonroot variant has ONE OR-requirement for a lower-rank selected parent-state whose ORIGINAL active authored/effect declaration targets it. No witness means that variant is excluded. Original dependency requirements remain independently encoded and need not decrease in rank.

Active support values now require their exact real parent-state presence. That requirement cannot itself establish rootedness: the parent's separate witness must descend through authored edges to an actual root. `assert_rank_witnesses` at68 checks the selected witness invariant and the independent original-edge reachability oracle. Solver helper needs never enter authored_dependencies, and no solver result is trimmed.

This is a finite expressiveness construction, not source discovery: the state universe and active declaration relation are manually supplied, and ranks multiply those already-known states. It does not solve how to discover every legitimate ingress cause lazily or how to record soft/OR/capability support in production. It deliberately does not require every physical dependency edge to be acyclic.

### Executed controls

| Control | Result |
| --- | --- |
| Controller absent from actual roots, Q remains root; support requires A presence | PASS: A/controller cannot obtain a rooted witness; selected Q retains only the real omitted workspace input and NeedsSelection. No invented blue answer. Both candidate-list orders pass. |
| Rooted hard dependency A→B→A | PASS: selected A rank0, B rank1. B witnesses A, while the authored back-edge B→A remains valid. No DAG restriction was imposed on all dependencies. |
| Rootless A↔B support cycle, Q actual root | PASS: ordinary Q branch stays unresolved without A/B; forcing the circular active support produces actual Unsolvable. A helper root requirement is not promoted to an actual package root. |
| External controller.on seed | PASS: actual core evaluation with only the explicit input selects on; the finite admitted on-state completes C→A→Q→C with exact inputs and ranks. |
| Required controller.on seed | PASS: actual core evaluation with only the required declaration selects on; same physical cycle and exact-input checks pass. |
| No external or required seed | **RED**: actual core with seed-only controller input selects no features, but resolvo selects C.on(rank0), A(rank1), Q.blue(rank2). Per-package evaluations, exact input equality and physical rooted witnesses all pass; the no-invented-activation assertion fails. |

The two seeded controls explicitly restrict the hand-enumerated compatible controller state to on after independently checking that the core seed evaluation selects it. They prove that the physical witness mechanism permits those legitimate seeded cycles, not a general algorithm for discovering/filtering configuration states. The unseeded case admits both off/on states, preferring on to expose the unwanted model.

## Decisive activation-cycle counterexample

The REAL root is controller C. Its optional A declaration is active only in the C.on state. A supplies Q's exact blue choice. Q.blue forwards on to C. There is no external answer or required feature that starts on.

The finite SAT model finds a self-consistent assignment:

`C.on(root,rank0) → A(rank1) → Q.blue(rank2) → forwarded C.on`.

Every package is physically reachable from the root along the selected state's declared edges; every input has a real selected producer; every local core configuration resolves. Nevertheless the first activating premise C.on is supported only by the cycle it enabled. Root membership of the package is not a seed for all of its features. `src/v2.rs:281` independently evaluates C using only actual external/required seeds, observes the empty feature set, then asserts that the circular on-state must not be selected. The assertion fails in logs/v2-first.log and logs/v2-final-input-control.log.

The package/effect relation in this test is a hand-authored finite table. Actual shared core evaluates each package's choice/feature inputs; production forwarding/dependency lowering is not invoked. Thus the result is an executable counterexample to physical-reachability-only authorization, not a newly claimed production bug in the still-refusing forwarding path.

## Boundary and next decision

Physical rootedness fixes the earlier unowned/reverse-parent counterexamples in this finite scope and preserves compatible hard cycles. It is **insufficient** to ground the activation facts that make an optional authored edge active. A future design needs an additional causal/seed condition for feature/declaration activation, distinct from physical selection and compatible with legitimate seeded cycles. This experiment does not supply that design, force a DAG on every dependency, or claim impossibility of correct encoding with resolvo. I stopped the bounded experiment after the requested tests; no general discovery framework, powerset enumeration or second solver was added.

Required questions remain unresolved data, not SAT incompatibility used to choose a different answer. The current shared kernel can legitimately resolve an omitted consumer when another REAL consumer supplies the shared choice; that earlier v1 control remains passing. No new default-consent semantics, persisted record, CLI completion or native-platform coverage is implied.

## Frozen evidence

- New source: src/v2.rs; shared probe-only provider/fixtures/lib receive witness metadata and per-definition evaluation support. Original tests remain, including their expected REDs.
- commands-v2.json: exact commands, private target, exits and logs.
- logs/v2-first.log:4PASS/1FAIL; logs/v2-final-input-control.log:8PASS/4FAIL/0ignored.
- logs/v2-fmt.log: scoped rustfmt check0.
- source-hashes-v2.json: current source/manifest/lock/report and evidence hashes, excluding target.
- main-inputs-v2-after.json: named main source/manifest hashes and drift; v1-log-preservation.json: old evidence log verification.

Root retains archive/cleanup. This worker leaves source, private target and raw evidence in place; no build remains running.

Main checked-input drift: `{}`; prior log drift: `[]`.

Before final handback I removed an unnecessary omitted workspace:q input from the activation-cycle fixture: Q is not an actual root there, and its only real consumer is A. This tightened fixture provenance without changing the mechanism. The current-source full rerun still has8PASS/4intendedFAIL/0ignored. Previous v2 source/report/manifests are preserved as before-input-control artifacts; earlier logs remain unchanged.
