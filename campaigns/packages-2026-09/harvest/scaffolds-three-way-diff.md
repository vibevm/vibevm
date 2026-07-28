# the nine scaffolds â€” three-way diff across rust / typescript / go

_Captured 2026-07-28 against the three `-lang` stacks' `spec/cards/`._

C4's designed instrument. The three stacks ship the same nine scaffolds as a
**projection of one language-neutral pattern**, so where they agree one reading covers
three, and where they diverge without a language reason the divergence is the finding.

```console
$ python campaigns/packages-2026-09/tasks/scaffold-three-way.py
three-way scaffold diff — 9 card(s) × 3 languages, view --anchors

scaffold-a: rust=13, ts=13, go=13; shared 13, divergent 0

scaffold-b: rust=13, ts=13, go=13; shared 13, divergent 0

scaffold-c: rust=13, ts=13, go=13; shared 13, divergent 0

scaffold-d: rust=51, ts=52, go=52; shared 45, divergent 15
    only in ts               missing from go+rust      — CONSEQUENCE-ARBITRARY-AND-COMPARATOR-COST-EFFORT
    only in rust+ts          missing from go           — CONSEQUENCE-CHARACTERIZATION-ENSHRINES-CURRENT-BEHAVIOR
    only in go               missing from rust+ts      — CONSEQUENCE-ENCODING-AND-COMPARATOR-COST-EFFORT
    only in go               missing from rust+ts      — CONSEQUENCE-GOLDENS-ENSHRINE-CURRENT-BEHAVIOR
    only in rust             missing from go+ts        — CONSEQUENCE-STRATEGY-AND-COMPARATOR-COST-EFFORT
    only in go               missing from rust+ts      — RISK-FUZZ-ENCODING-REACHES-REPRESENTATIVE-STATES
    only in rust+ts          missing from go           — RISK-INPUTS-ARE-GENERATABLE-WITH-COVERAGE
    only in rust+ts          missing from go           — ROUTINE-CITE-THE-ORACLE
    only in rust+ts          missing from go           — ROUTINE-RUN-IN-THE-LOOP
    only in go               missing from rust+ts      — ROUTINE-RUN-SEEDS-IN-THE-LOOP
    only in go               missing from rust+ts      — ROUTINE-TAG-THE-ORACLE
    only in ts               missing from go+rust      — ROUTINE-WRITE-THE-ARBITRARY
    only in go               missing from rust+ts      — ROUTINE-WRITE-THE-DIFFERENTIAL-TARGET
    only in rust             missing from go+ts        — ROUTINE-WRITE-THE-STRATEGY
    only in go+ts            missing from rust         — status-line

scaffold-e: rust=13, ts=13, go=13; shared 13, divergent 0

scaffold-f: rust=13, ts=13, go=13; shared 13, divergent 0

scaffold-g: rust=13, ts=13, go=13; shared 13, divergent 0

scaffold-h: rust=13, ts=13, go=13; shared 13, divergent 0

scaffold-i: rust=13, ts=13, go=13; shared 13, divergent 0

anchors present in some languages and not others: 15
EXIT=0
```

**Read the anchor view, not the word view.** A companion `--words` run reports 120
shared anchors whose content words differ, and that is the projection working as
designed: each card's MOTIVATION, INTENT, STRUCTURE and RISKS sections describe that
language's own tooling â€” `ts-morph`/`jscodeshift` for TypeScript, `gofmt -r` for Go â€”
so content divergence there is the point rather than a defect. **Anchor divergence is
different: an anchor is an address, and an address that resolves in two projections and
not the third breaks the citation.**

**Scope:** every fact under the twenty-seven `spec/cards/scaffold-*.md` files of the three `-lang` stacks. The anchor list is not maintained here â€” a verdict cites this file in its `ev[]`, and the reverse index is derived at the phase close (PHASE-C-BATCH-PLAN.md Â§5).
