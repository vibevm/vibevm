# v3 — separate causal witnesses close the finite unseeded activation counterexample

Scope: SAME standalone `/fast/tmp/nst-transitive-proof`, PRIVATE target, cached resolvo0.11.0 and current main vibe-core path dependency. No production/main source, Git, network, dependency, toolchain, agent or store change. Root archived v1/v2; their raw logs remain unchanged. This is a hand-grounded finite expressiveness probe, **not general production lowering/discovery**.

## Decisive observed result

First v3 run exits0:3PASS. With no external/required seed, normal C-off remains feasible, and forced C-on is actual Unsolvable. External and required seeds both retain C.on→A→Q.blue→forwarded C.on, with all original inputs intact and each activation fact grounded independently.

Final command explicitly runs ALL old negative encodings and current controls: exit101, **11PASS/4FAIL/0ignored**. The only four failures are the preserved v1 naive/unowned/reverse-parent encodings and v2 physical-only unseeded cycle. They remain negative controls; they do not run the v3 causal construction. All3v3 tests pass. Legitimate rooted hard A↔B and prior physical/phantom controls still pass. No selected result is trimmed or repaired.

## Exact finite construction

`src/v3.rs:25` takes the SAME v2 physical-state fixture and adds a separate causal proof layer. Existing physical rank/witness remains unchanged. Five fact names each have five immutable proof candidates (ranks0..4), sharing one NameId per fact:

| Fact | Allowed justification |
| --- | --- |
| C.on | Actual external/required seed at rank0, or justified Q-forward at a smaller rank |
| C→A active declaration | Justified C.on at a smaller rank |
| A's actual Q/blue input | Justified C→A activation at a smaller rank, plus exact A-state presence |
| Q.blue | Actual authorized A input at a smaller rank, plus exact Q-state presence and core-valid merged choice |
| Q→C.on forward | Justified Q.blue at a smaller rank, plus exact Q-state presence |

`seed_exists` at21 uses actual shared core seed-only evaluation of C. No ordinary helper or mere package presence creates a rank0 seed. At rank0 without a genuine seed, the required smaller-rank predecessor set is empty and therefore unsatisfiable. Parent/state presence checks remain independent of activation justification.

The conditional C→A dependency now demands the C.on/edge causal proofs; rooted C presence alone is insufficient. A/Q selected-state requirements demand their input, choice and forward proofs. All clauses are immutable before the single resolvo solve; there is no mutable-map re-solve loop or second solver. The tiny universe has25causal witness rows in addition to the existing12physical/helper rows. No powerset generation or arbitrary source universe was introduced.

For a legitimate seeded cycle, C.on chooses the seed justification(rank0). The remaining facts use ranks1,2,3,4. The redundant Q→C.on contribution still appears in C's exact original input. The fact's chosen justification need not use every redundant contributor, but every needed contribution itself remains justified. `assert_causal_witnesses` at77 checks the strictly decreasing nonseed dependency and validates any seed against actual external/required input. Existing exact-input and original-edge reachability assertions run as well.

## Executed discriminators

- `v3_unseeded_cycle_keeps_normal_off_feasible_but_cannot_force_on` (line115): both candidate-list orders select C-off with no A/Q or causal proofs. Independently forcing C-on returns Unsolvable, not Cancelled. No rejection of the whole ordinary off configuration.
- `v3_real_seeds_preserve_redundant_cyclic_input_and_prove_each_activation` (line139): external and required seed × both list orders pass. C.on proof rank0, declaration1, actual input2, choice3, forward4. Exact C input still contains `Q@1:blue->controller.on`; the cyclic contribution was not deleted to obtain success.
- `v3_choice_member_still_requires_actual_choice_authorization` (line176): actual core ordinary feature blue plus an omitted mandatory choice remains Unresolved/questions. Adding a REAL exact A/blue choice consumer resolves it. The causal construction's Q.blue justification requires that actual choice input, not an ordinary feature seed.
- Final all-tests run retains v2's rooted hard dependency cycle, rootless-cycle rejection, and unowned/reverse support refusal, as well as v1 candidate backtracking/weak-owner/conflicting-single-instance controls.

Questions remain data without executable effects. The real core merges consumers before cardinality: an omitted consumer may be satisfied by another actual authorized consumer; no new per-consumer mandatory-answer rule is invented.

## What is and is not established

Observed: SAME resolvo can express both a rooted physical witness and a separate grounded activation witness for this finite cyclic configuration. The new causal condition removes the v2 unseeded self-support model while retaining the specifically tested external/required seeded cycle and ordinary hard cycles.

Not established: production enumeration of input states or causes; generalized lowering of guards, profiles, defaults, merge alternatives or effects; all valid activation justifications; lazy discovery/scalability; recommendation/OR/capability support; persistence/CLI/native-platform delivery. Here A has exactly the declared C→A ingress. Another legitimate root/parent ingress would need its own real cause; this fixture does not claim to derive it automatically. The rank bound and cause relation are hand-grounded for five facts only. Candidate-list reversal is followed by deterministic priority sorting; it is not a proof across every discovery schedule.

As inherited from v2, seeded controls restrict the hand-enumerated compatible C state to on after actual core seed-only validation. They establish that the causal layer permits the legitimate seeded cyclic contribution; they do not claim a general algorithm to enumerate/filter compatible states. The unseeded test offers both off/on, deliberately prefers on, and v3 rejects only the unjustified on alternative.

Production source still explicitly refuses unsupported transitive/forwarding routes. These probes do not change that boundary or establish a production defect there. No general causal framework was built after the decisive run; this bounded refinement is complete for root judgment.

## Frozen evidence

- Source addition src/v3.rs; provider gains causal-rank diagnostic metadata; v2 helpers are exposed only to sibling test module; lib registers v3. No v2 evaluation/physical-witness body changes.
- commands-v3.json records exact commands/private target/exits.
- logs/v3-first.log:3PASS. logs/v3-final-all.log:11PASS +4preserved old-encoding failures,0ignored.
- logs/v3-fmt.log: scoped formatting check0.
- source-hashes-v3.json binds current probe sources/manifest/lock and v3 evidence.
- main-inputs-v3-after.json and prior-log-preservation-v3.json record main checked-input/log drift.

No build remains running. Root owns archival and cleanup; source, target and evidence remain in place.

Main checked-input drift: `{}`; prior log drift: `[]`.
