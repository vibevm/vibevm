# C2 correction — independent final review

**Recommendation: accept the corrected implementation on the retained admitted C2 surface. Both original P1 findings are resolved; no additional blocking finding identified in this bounded correction review.** Root owns canonical judgment, commit, count and process stop. No tests/builds, source edits or runtime controls were performed by this reviewer.

## Original findings

**C2-R1 — fixed.** `resolvo_engine/configuration.rs:311–348` now determines default-governed omitted intent from both the current and previously shown definitions. The comparison no longer depends solely on the current `defaults_used` list. A removed optional default therefore retains the stale authorization condition. Exact request/receipt/full-definition matching is still required; changing the consumer to an explicit selection does not adopt or depend on defaults. The new `removed_optional_default_requires_rediscovery` case first observes the bound selection, removes the ZeroOrOne default, requires Rediscovery with original request/candidate identity, and separately proves explicit selection remains valid.

**C2-R2 — fixed.** The unresolved/stale-default branch now derives only already-authorized ordinary and determined selection implications through B1. Those requirements are attached to the candidate's manifest before incumbent resolvo solving (`configuration.rs:375–455`), even though B2 still exposes no complete expansion for NeedsSelection. Missing group members are not chosen. Unbound defaults are disabled in the internal B2 request, preventing unauthorized default implications from influencing candidate selection. Final effects are still returned only for selected Ready candidates. The new optional-bad fixture retains `bad.optional=true`, adds the unresolved group and verifies the older complete graph plus all features/dependencies/providers/subskills/config. It no longer relies on changing bad to a required declaration.

The changes keep the original consumer request in results/errors, exact C1 source admission, candidate exclusion reasons and existing solver mechanism. No retry budget, separate engine, default parser change or C3 integration was added. PROP003 retains prior rules and adds the two precise semantic clarifications. Explicit unsupported combinations remain reported as before; this review does not broaden that surface.

## Verified receipts

I checked actual retained artifact hashes rather than only the summary:

- RED1 and RED2 each show exit 101, zero passed/one failed against the preserved pre-fix implementation. Both log hashes match. These are genuine red observations and remain failures in history.
- Final V1 shows exit 0, seven passed/zero failed. Log hash and recorded source hashes match current source.
- V2/V3/V4 explicitly report zero fresh tests. Their reused 3/2/3 case logs and listed unchanged consumed-owner hashes match. Reuse is qualified, not a claim that the complete source snapshot is identical or that those tests were rerun.
- V5 scoped formatter **check** and V6 diff check report exit 0 with matching receipts. V7 remains structural preservation evidence; semantic conclusions above come from reading the code and contracts. The excluded historical global formatting failure remains retained.

No additional broad retesting is recommended. The two deciding counterexamples and the seven-case green family establish the requested correction, alongside qualified unchanged-owner reuse.

## Reviewed source pins

- `crates/vibe-resolver/src/resolvo_engine/configuration.rs`: `10525778011a61cae4aa889d9223dcf1b8d0fb32a58519f2d2480b803568be0a`
- `crates/vibe-resolver/src/resolvo_engine/m_05_c_2_cases.rs`: `117284db695224fcd16bf4a409505f9d40f111a48db42e8f84360e87addeb4c7`
- `vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml`: `a5a7cc70e7a11a217e7d161c70ca829c60d24cf2c463c5b9be1a1ba82b7a84ec`

This is approval of the reviewed correction and admitted configured resolver behavior, not a claim of universal route coverage, campaign completion or physical worker retirement.
