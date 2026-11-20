# Actual dependency-conflict backtracking — default/effect isolation

Architect-confirmed finite test/report gate for PROP-069#ACCEPTANCE and CANDIDATE-BOUNDARY. Product unchanged at `153b69920dd17b30ee913b631770c15a2b26c1f0`; no solver, default, source or epoch modification.

## Why prior selectors were not this gate

`unselected_source_candidate_does_not_forward_into_target` explicitly pins1.0.0 and therefore does not prove backtracking. Parameter allowed-values fallback rejects an input-incompatible candidate, not a transitive dependency conflict. Lower resolver `dependency_unsatisfiable_newest_can_backtrack_without_retaining_its_questions` proves solver behavior but is not the actual public installed default/effect journey. None is relabeled as this new receipt.

## Two actual fresh public worlds

Both have an ordinary local Flow root range `^1`, with individually valid1.0.0 and1.1.0 definitions. Both satisfy root range and take omitted/use-declared-defaults exactly-one input. Earlier default old activates oracle-extra; newer default new activates a distinct oracle-new-candidate-effect. Their boot/payload identities and source content pins differ.

- **Conflict:** newer requires shared-conflict2.0.0; an independent actual root requires shared-conflict1.0.0. Earlier requires1.0.0. Fresh public install selects earlier1.0.0 with old basis, its actual pin, extra slot/boot/hook, and no new-effect slot/hook/boot or rejected driver-version slot.
- **Matched no-conflict:** independent root requires shared-conflict2.0.0; same newer candidate now selects1.1.0 with new basis, actual new source pin and distinct new effect. This proves newer exists, is root-range compatible and can actually install; it is not skipped by an exact pin or malformed definition.

Lock features/default provenance belong only to the selected version; the real shared dependency has exactly one hook. Physical graph/slots/boot/hooks discriminate both worlds. Fresh-process repeat preserves complete project semantic state and lock bytes. No foreign effect is executed merely because solver considered the candidate; cache/metadata observation is not claimed absent.

Focused runtime r1 passed one test containing both worlds (122 filtered). Complete `backtracking-qualification-public-all-r1.log` passed **123/123**, zero filtered. Strict affected fixture lint and separate workspace fmt/diff gates passed. Exact source/artifact/evidence manifest accompanies this report; independent acceptance pending.

Private evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z/`: focused/full/lint/build/fmt logs under `backtracking-qualification-*`. No agent/provider, network service outside isolated synthetic runtime, credentials, USER-PROJECT adoption, product change, parent-state promotion or whole-NST acceptance is implied. This discharges only the named public backtracking/default-effect isolation cell after independent acceptance.
