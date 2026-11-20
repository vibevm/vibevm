# Dormant original choice across guard changes — bounded qualification candidate

Accepted product6136d6df4733ddd8d0104e5d0d3e20e20178c7fc; qualification brief9cbdbf7447591b079ea68e1682d3e180456005cf integrated5e280248e. Test/report only. No runtime, solver, schema, default policy, interactive UI or maintenance guard is modified.

## Separate public journeys

One real project consumer requires an exact ordinary local flow. Its public switch group selects on/off. A second exactly-one detail group is guarded by the on member; the consumer explicitly selects detail=selected. That selected feature activates an optional dependency with an independent boot marker and pre-install hook marker. The driver source declaration remains available while dormant; inactive bytes do not count as an active dependency.

Two independent worlds start with a successful actual install and selected extra dependency. In one, authored detail choice remains while the switch changes off; in the other, the authored detail answer is actually removed. Normalization preserves this distinction, and pure choice controls record inactive detail with a dormant projection only when the answer was retained. Those pure observations alone are not publication proof.

Actual `update --all --offline --assume-yes` is checked separately in both worlds. It refuses `configured whole-workspace update requires explicit graph membership reconciliation before effects`; the complete semantic project snapshot remains unchanged. The precise owner is `vibe-cli/src/commands/update/scoped.rs`, comparing old and selected package membership before effects. This refusal does not prove that the independent ordinary install reconfiguration journey fails.

Actual `install --offline --assume-yes` on the existing project is the separately qualified public reconfiguration route. With guard off, it preserves original authored maps in the existing configuration record, removes selected feature/extra dependency from the effective graph, removes the extra slot/boot marker and runs no new extra hook. Inert source declarations remain. A separate fresh-process repeated install leaves semantic state and hook counts unchanged.

On guard return, the retained-choice world successfully restores the extra graph row/slot/boot marker and runs its actual hook again; repeat is stable. In the independent removed-choice world, guard return refuses with ChoiceQuestion detail/ExactlyOne and DEFAULT-CONSENT. No old choice is remembered from graph/history to fill the missing authored answer; lock/slots/boot/hooks remain at the dormant state. The test compares the whole semantic snapshot after the owner-authored input change, not before that intentional edit.

## Evidence and limits

`default_review::ordinary_dormant_guard_switch_retains_authored_choice_or_names_public_boundary` passed as one actual two-world isolated test on CLI7a70adb1c67a9e4dbbc59f9f6b16f2cc87f48f7c73a6ab402e2e36726df68ad6, fixture4facf38a75798d88328cfac710cde8754ee23785f582b287dd99644ff6597354 (dormant-qualification-public-r2.log). Initial r1 qualified only the update refusal and did not reach off/back publication; it is retained and not generalized into an all-operation unsupported claim. Architect's independent install observation prompted explicit testing of both real operations rather than altering product code to bypass update.

Strict fixture lint and format/diff passed. Exact source/artifact receipt and independent acceptance are pending. The accepted boundary is not yet promoted. This qualifies ordinary Linux local-copy install guard off/on only, not update graph renewal, nine-kind runtime delivery, forwarded/parameter/profile guards or NST parent completion. No network/provider, GUI, REMAINDER/IDE or UP-adoption change.
