# M-05-C.2 final independent review

**Recommendation: request correction before accepting full C2.** Two substantive gaps occur inside the explicitly admitted fixed-consumer/registry surface. Neither needs C3 integration or a broader solver feature to reproduce. Findings below are source-derived counterexamples, not newly executed tests. No tests/builds, product edits, runtime controls or acceptance mutation were performed.

## 1. [P1] Removing an optional group's shown default silently changes authorized effects

Location: `crates/vibe-resolver/src/resolvo_engine/configuration.rs:337–349`; related B2 behavior `crates/vibe-resolver/src/features/configuration.rs:120–132`.

The binding check iterates only `outcome.defaults_used` from the **current** definition. B2 records an entry there only when the current group has a default. Consequently, a default disappearing makes the exact-binding check vacuously succeed.

Concrete counterexample using existing helpers:

1. Set the app@1 group to `ZeroOrOne`, members `{a,b}`, default `{a}`; request is omitted selection with `UseDeclaredDefaults`.
2. Create `DefaultBinding::shown(0, &request, &definitions[0])`.
3. Change that same admitted normalized definition's group default to `None` (the same kind of semantic mutation the existing test already permits), retaining the old binding.
4. Solve with root app pinned to 1 and blocker as in the test.

The current B2 result is Resolved with empty selected group and empty defaults_used. C2 enters Ready and returns success without `a`. Expected: Rediscovery, because authorization referred to the previously shown definition/default and the selected meaning changed. The full shown definition is already stored in the binding; the bug is the conditional validation trigger, not missing identity data. The same failure can occur when fallback reaches another candidate with no default.

Existing test `changed_semantic_default_requires_rediscovery` covers ExactlyOne and Some(a)→Some(b), which always records a current default; it misses Some(a)→None on an optional group. Correct the binding/current-definition comparison for default-governed omitted intent even when the new definition uses no default. Keep explicit selections independent. Add this one negative case and assert no successful changed effects are returned.

## 2. [P1] An unresolved group removes known ordinary-feature dependencies from the solve

Location: `crates/vibe-resolver/src/resolvo_engine/configuration.rs:284–295,332–336,350–378`; eventual prompt at `:488–492`.

Before B2 resolution, the candidate manifest contains only nonoptional declarations. Induced requirements are installed only in the fully Ready branch. NeedsSelection (and similarly Rediscovery) therefore omits implications of ordinary features the consumer already explicitly requested. A candidate may appear compatible solely because those known constraints disappeared; it is selected and prompts instead of being rejected by the incumbent solver.

Concrete counterexample is already almost present in `unreachable_mandatory_group_has_no_prompt`:

- Keep its original definitions: both good and bad declarations are optional; ordinary explicitly activates bad for app@2 and good for app@1; bad conflicts with root blocker.
- Add mandatory group without a default to app@2, then solve roots `[app, blocker]` with the existing request for ordinary.
- Do **not** perform the test's mutation at `m_05_c_2_cases.rs:362–368` that makes bad nonoptional.

Current path: B2 reports NeedsSelection with no full expansion; app@2's manifest receives no bad dependency; resolvo chooses newest app@2; C2 returns NeedsSelection. Its already-known ordinary activation actually makes app@2 incompatible with blocker, independent of how the unresolved group is answered. It should be discarded and app@1 selected without prompting, with only app@1 effects. The test's conversion of bad to a required declaration masks exactly the candidate-conditioned implication gap.

Retain all already-determined, authorized implications on the candidate while unresolved choices remain unresolved; do not erase known constraints to defer a prompt. This can use existing B1/B2 machinery and incumbent SAT implications, without guessing group selections or adding a retry bound. Add the above variant and compare complete graph/providers/dependencies/features/subskills/config, as the current positive fixture already does.

## What is sound in the reviewed increment

The ready-candidate path rebuilds dependencies per candidate and uses existing resolvo `get_dependencies`/candidate exclusion rather than a separate bounded-retry engine. Original request values are retained in result/error projections; conflict reasons include original edge indices, candidate coordinate and C1 identity. Definition identity mismatch and malformed declaration/constraint data are distinguished from candidate rejection. The C1 catalog and ordinary resolver/B3 reader routes remain intact. Spec additions preserve prior rules, but the two implementations above do not yet satisfy their added default/conditioning promises.

The explicit refusal boundary for native selection, multiple instances, forwarding, non-registry sources, conditional targets, disjunctions and obsoletes is documented. This review does not expand that boundary or require universal route parity; both findings use supported declarations and group kinds.

## Evidence boundaries

Final V1 reports five passing cases; V2/V3/V4 report 3/2/3, totaling 13. I independently matched referenced log hashes. V1 recorded source hashes match current source. V2–V4 captured earlier versions of `resolvo_engine/configuration.rs` and PROP003, while their directly exercised B1/B2/C1/ordinary solver source remains unchanged; do not call those receipts identical-full-source final runs. This is not an additional rerun demand: the missing semantic counterexamples above are the deciding new checks.

Final scoped formatter/diff receipts match current captured sources. V7 structural/spec preservation is not semantic acceptance. Earlier V1 failures remain historical; the current passing family does not cover these two cases. No existing green count cures either gap.

Return one bounded correction to the actual managed Zap→PodBay producer. Preserve the current candidate and original task criteria; verify the two deciding variants and directly affected existing cases only. Root retains acceptance, commit, stop and count decisions.
