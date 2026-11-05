# Same-resolvo finite transitive-configuration probe — bounded result, not production readiness

Probe: `/fast/tmp/nst-transitive-proof`, PRIVATE target `<probe>/target`. Main baseline supplied by root: F16f4f965fd9 + docs6995fa631. No main source/worktree/Git mutation, network, agent spawning or toolchain change. Actual solver is cached resolvo=0.11.0; current main vibe-core is a path dependency. Choice normalization, exact/additive merging, cardinality/questions and feature closure are executed by that core, not a copied evaluator. Probe source and raw evidence remain for root to archive/remove.

## Decisive result

**The finite owner-guarded construction passes the four requested discriminator families, but does not solve general support/absence.** Removing the controller from actual roots produces a phantom answer again. Adding a reverse support→parent requirement merely makes that phantom parent appear in the solver result; it is still unreachable through the original authored/effect declaration graph. Both are actual failing assertions, not source speculation. No selected node or input was trimmed/repaired after solve.

Final current-source command includes every negative control: `cargo test --offline --locked -- --include-ignored --nocapture` with explicit standalone manifest/private target. It exited101: **4PASS,3FAIL,0ignored**. The three failures are deliberate counterexamples retained as `#[ignore]` only so the bounded green subset can run independently; they were explicitly executed individually and in this final all-tests command. A plain default-green result must not be advertised as general completion.

## What is encoded

`src/provider.rs` supplies immutable finite rows to the real resolvo Pool/Interner/DependencyProvider. Multiple Q configuration states share the SAME `pkg:q` NameId, each with its own SolvableId and input signature. Dependencies never consult a mutable selected-input map. Incompatibility from actual core evaluation yields Dependencies::Unknown for that state. Core NeedsSelection remains a selectable unresolved state; it is not converted to SAT rejection merely to find an easier answer. No second solver or solve/re-evaluate repair loop exists.

Hand-enumerated support rows carry inactive/red/blue or active+trace under one internal support NameId. A real selected parent demands its exact support value. In the bounded absent-parent case, a selected REAL controller with an inactive optional A declaration demands inactive support. Q states require the corresponding support value; they cannot manufacture the controller's opposite value. This is a fixture-specific complete support cover, not a general discovery algorithm and not a proof that arbitrary absent owners already have such a guard.

`src/fixtures.rs:107` asserts exact equality of Q's claimed original inputs/forwarded contributions and the selected real declarations plus fixed workspace input. A second independent assertion at256 traverses ONLY `authored_dependencies`, separate from solver `needs`, and demands that ALL selected real packages are reachable from actual roots. It diagnoses extra nodes without deleting them. Both contracts are used by the bounded positive cases. Reverse-support counterexample demonstrates why selected-row input equality alone is insufficient.

Forwarding activation is a small hand-authored fixture truth table (weak inactive, weak with independently active owner declaration, strong). The shared core evaluates the resulting feature/choice inputs; production forwarding lowering is NOT called because it currently refuses that effect. Forwarded feature provenance remains separate from authored consumer answers. Thus this tests SAT wiring/merge semantics, not a completed package effect adapter.

## Actual observations

| Test | Observed outcome |
| --- | --- |
| `rejected_parent_changes_consumers_without_stale_q_cache` | A2/red + fixed workspace Q/blue conflicts through real core; solver selects A1/blue. Exactly one Q; A2 input and red-effect node absent, blue-effect present. Q's newer incompatible state was actually requested. Each requested immutable dependency row was fetched once. Both candidate-list enumeration orders PASS. |
| `weak_is_owner_local_and_strong_activates_without_ambient_q` | Independent workspace Q/blue does not give A's inactive weak edge authority: trace absent. Independently active A edge contributes trace; strong introduces Q even with no ambient Q root; weak alone does not. All8 declared cases PASS. |
| `absence_guard_preserves_questions_and_real_parent_can_answer` | Controller-off leaves Q with its real omitted workspace answer: Unresolved/questions, no fabricated blue. Controller-on reaches real A and its blue answer legitimately resolves the shared choice, even though workspace input remains omitted. Both list orders PASS. No invented per-consumer mandatory-answer requirement. |
| `incompatible_real_consumers_cannot_install_two_q_states` | Pin A2/red with fixed Q/blue: actual Unsolvable, not Cancelled or duplicated Q. PASS. |
| `naive_support_red` | Without Q's support guard, solver selects Q-state claiming absent A/blue. Exact-input equality fails. Expected RED. |
| `unowned_support_red` | Keep support guard but remove controller from actual roots. Free active support is selected to manufacture A/blue although A is absent. Exact-input equality fails. Expected RED. |
| `reverse_support_parent_red` | Add support→A requirement. Selected-row input equality now passes, but authored reachability fails: selected real set={A,Q}, legitimate reachable set={Q}. Expected RED. |

Positive top-level count4 covers15 declared subcases; this is not exhaustive state-space testing. Reversal changes the provider's candidate lists, followed by the same deterministic priority ordering; it is not proof across arbitrary lazy-discovery schedules. No automatic-default receipt, persistence, CLI or native-platform acceptance is claimed.

## What this decides and what remains

Immutable configuration-state identity CAN coexist with one actual package instance and current core merging in the SAME resolvo. It fixes the stale `(package,version)` evaluation assumption for the tiny fixed support universe. Arbitrary input-subset enumeration would be exponential and was not attempted or proposed as production architecture.

The general support problem remains OPEN. A required support token is not itself evidence that its owner was selected legitimately. Reverse requirements are not a solution: they can add the supposed authority. The next architecture decision must establish complete owner-conditioned support/absence without phantom ingress and without requiring unrelated inactive foreign metadata. These counterexamples reject the demonstrated incomplete encodings; they do not prove resolvo0.11 is fundamentally incapable of a correct finite encoding. No solver fork, generalized catalog or extra store has been introduced to evade the boundary.

## Mistakes, dependencies and evidence discipline

I initially omitted required presentation names from common feature fixtures; first-red.log is a parser failure, not an intended solver RED. I corrected those names before meaningful results. The first independent offline generate-lockfile selected newer already-cached transitive versions than main. That was a probe setup mistake; no main manifest/lock/toolchain was changed. Cargo.initial-latest.lock and logs are retained. I regenerated only the probe lock offline from the main-lock baseline; `logs/dependency-version-drift.json` is [] and the final repeated comparison is also empty. All meaningful solver results use that baseline lock.

`commands.json` records commands/exits/logs; `<probe>` means the absolute path at the top, and Cargo commands used `CARGO_TARGET_DIR=<probe>/target`. `logs/metadata.json` is the offline Cargo graph. Final scoped rustfmt check exits0. `main-inputs-before.json` and `main-inputs-after.json` bind the named main sources/manifests: drift is recorded explicitly. This is a checked input set, not a claim to have hashed every main repository file.

`source-hashes.json` binds all probe sources/manifest/lock and evidence logs. No Cargo process remains from this worker. Root owns archival and eventual cargo-clean/removal; nothing is deleted by this worker.

Checked main-input drift: `{}`.
