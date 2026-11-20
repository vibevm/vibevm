# Remaining ordinary choice cardinalities — qualification

Contract NST-REMAINING-CHOICE-CARDINALITIES-QUALIFY-01 (`e263a052030a87d2b475ddc6898745cfd29d569e`), integrated at `b53042532`, after accepted zero-or-more implementation `b31e8a499`. Test/report only, no product change or parent promotion.

## Ten independent worlds

Each cardinality uses five fresh ordinary local flow projects and one real consumer: omitted/use-declared-defaults, omitted/require-selection, exact empty, exact one, exact two. All use default_features=false, no profiles, guards, secrets or implicit feature defaults. Valid zero-or-one definition declares one default member; valid one-or-more definition declares two. Member config data and optional dependency/slot/boot/hook effects distinguish selected outcomes.

| Cardinality | Input | Kernel | Actual public install |
| --- | --- | --- | --- |
| zero-or-one | Omitted/default policy | Declared singleton selected | Unsupported automatic basis |
| zero-or-one | Omitted/require-selection | Empty | Success, no optional target |
| zero-or-one | Explicit empty | Empty | Success, original explicit [] retained |
| zero-or-one | Explicit one | One selected | Success, public value + target effects |
| zero-or-one | Explicit two | Cardinality conflict | Unsatisfiable, no package effects |
| one-or-more | Omitted/default policy | Two defaults selected | Unsupported automatic basis |
| one-or-more | Omitted/require-selection | Typed question | Requires configuration answers, no effects |
| one-or-more | Explicit empty | Typed question, original Explicit([]) | Requires configuration answers, no effects |
| one-or-more | Explicit one | One selected | Success, public value + target effects |
| one-or-more | Explicit two | Both selected | Success, both public member values + target effects |

Parser/source definition admission, normalized original input, pure outcome/default-use/question and actual public operation are separate observations. An omitted policy answer is never supplied from --assume-yes. Same question for empty and omitted one-or-more does not erase original intent: normalized Explicit([]) remains distinct from absent answer.

Success worlds retain original consumers exactly, no automatic basis, minimum epoch8; selected data, parent dependency, target physical slot, boot and hooks agree. Fresh CLI replay preserves semantic project state including hook log and lock. Refusals preserve package/lock/boot/hook state and publish no slots, lock or effects log. Full registry bytes and authored project input remain unchanged in all ten worlds. Coordination paths are printed separately; semantic snapshots intentionally exclude that namespace, not a claim of whole-project immutability.

## Owners and smallest remaining behavior

Both automatic worlds reach the existing ordinary package evaluator's automatic-reliance cardinality guard in `crates/vibe-core/src/manifest/configuration/package.rs`: `only unguarded exactly-one single-member defaults are connected`. Pure defaults are supported; ordinary automatic delivery for these two cardinalities is not yet supported. The accepted zero-or-more member-set basis and historical exactly-one scalar basis are separate scopes, not evidence that these routes already work.

For zero-or-one explicit two, the kernel Cardinality conflict is carried through package incompatibility into the existing unsatisfiable resolver result. For one-or-more empty/omitted required input, the shared evaluator returns NeedsSelection and the public error retains `ChoiceQuestion` / `OneOrMore` under DEFAULT-CONSENT. There are no effects before those answers.

The smallest proposed next extension is explicit automatic reliance admission for these two valid cardinalities through existing history owners, preserving optional empty versus mandatory minimum and actual prior-reader/writer boundaries. Do not merely relabel existing zero-or-more/exactly-one cases, waive questions or manufacture consumer answers. Architecture must choose representation/scope before implementation; this qualification grants none.

## Evidence

Selector: `default_review::remaining_choice_cardinalities_omitted_empty_one_two_public_qualification`.

Initial r1 stopped at one-or-more omitted-required because the test expected generic lowercase questions instead of actual `requires configuration answers` typed output. r2 reached explicit-empty and stopped on the same mistaken expectation of conflict. Both REDs are retained; later worlds are credited only to the completed final run. Final source/CLI/fixture/runtime/lint/format hashes are sealed in the candidate manifest; independent acceptance required.

Finite python3 -I bwrap runtime isolates filesystem/process/network with cleared environment, synthetic HOME/settings and exact read-only CLI/test/system runtime artifacts. No host home/auth stores, credentials, sockets, network or provider execution. No NST/REMAINDER/IDE/USER-PROJECT closure follows from this qualification.
