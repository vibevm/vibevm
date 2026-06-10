# Phase 1 — pilot dossier: PROP-003 §2.6.1 × `vibe-resolver/src/conditional.rs`

_2026-06-10, branch `new`. Per the owner's in-session direction the
pilot landed directly on `new` (no side-branch PR); this dossier is the
review surface the playbook's "PR description" would have been. The
drill commits stay in history as living documentation:
`4395d3b` (pilot) → `b3a947c` (drill a) → `73b6e81` (drill b) →
`4afe716` (revert)._

## 1. The units (PROP-003 §2.6.1, additions-only)

Five units; **not one existing prose line was changed** — only anchored
headings and kind lines inserted, plus the two-sentence body of the
planned unit (the only genuinely new text):

| anchor | kind line | covers |
|---|---|---|
| `#req-conditional-grammar` | `req r1` | the probe set `context(...)` accepts (existing paragraph) |
| `#design-conditional-when-to-use` | `design r1` | the guidance paragraph — anchored so it does not dilute a req span |
| `#req-conditional-fixpoint` | `req r1` | monotone fixed-point convergence (the Solver-impact paragraph) |
| `#req-conditional-host-invariance` | `req r1` | evaluation against resolved project state, never host state |
| `#req-conditional-composition` | `req r1 planned` | boolean composition — unbuilt; parser MUST stay loud (new 2-line body) |

Judgment calls (flagged for the in-chat review):

1. The `design` unit is one beyond the playbook's three-req list —
   added so the grammar unit's span would not swallow unrelated
   guidance. Strike it if unwanted.
2. `#req-conditional-fixpoint` and the re-solve loop: the loop lives
   outside `vibe-resolver` (workspace install orchestration), so the
   unit deliberately carries **no** `implements` edge yet — honest
   zero, filled by Phase 2 backfill, not faked from this module.
3. The drill ran over the **grammar** unit, not fixpoint, because only
   grammar had pinned edges to flip suspect.

## 2. The tags (`conditional.rs`)

- `implements → #req-conditional-grammar` on `ConditionalPredicate`
  (enum) and `parse` (r = 1);
- `deviates → #req-conditional-composition` on `parse`, with the
  mandatory reason (composition surfaces as
  `PredicateError::Unsupported`);
- `implements → #req-conditional-host-invariance` on `evaluate` (r = 1);
- `#[verifies]` on all six tests — four against grammar, two against
  host-invariance.

`vibe-resolver` gained the `specmark` dependency; the attributes are
inert (48 tests green, untouched behaviour). Index after the pilot:
**413 spec units, 17 tagged code items, 19 edges, 0 suspects**.

## 3. Drift drill (a) — semantic bump → suspects → re-affirm

Edit: one semantic sentence appended to the grammar paragraph + kind
line `req r1` → `req r2`. Captured `cargo xtask specmap --check`
output, verbatim:

```
Error: `…\specmap.json` is out of date relative to the tree.
  drift: revision bump: `spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar` r1 → r2
  drift:   now SUSPECT: `vibe_resolver::conditional::ConditionalPredicate` (pinned r1) at crates/vibe-resolver/src/conditional.rs:23 — re-affirm after review
  drift:   now SUSPECT: `vibe_resolver::conditional::ConditionalPredicate::parse` (pinned r1) at crates/vibe-resolver/src/conditional.rs:37 — re-affirm after review
  drift:   now SUSPECT: `vibe_resolver::conditional::tests::flags_unsupported_richer_forms` (pinned r1) at crates/vibe-resolver/src/conditional.rs:135 — re-affirm after review
  drift:   now SUSPECT: `vibe_resolver::conditional::tests::parses_simple_present_predicate` (pinned r1) at crates/vibe-resolver/src/conditional.rs:99 — re-affirm after review
  drift:   now SUSPECT: `vibe_resolver::conditional::tests::parses_with_whitespace` (pinned r1) at crates/vibe-resolver/src/conditional.rs:109 — re-affirm after review
  drift:   now SUSPECT: `vibe_resolver::conditional::tests::rejects_malformed` (pinned r1) at crates/vibe-resolver/src/conditional.rs:119 — re-affirm after review
Run `cargo xtask specmap`, review the drift, and commit the result.
```

Re-affirmation: the six grammar pins updated to `r = 2` (the `deviates`
pin into the planned unit and both host-invariance pins untouched —
they pin other units). Regenerated: **0 suspects**, `--check` clean,
tests green. Commit `b3a947c`.

## 4. Drift drill (b) — editorial edit, no bump

Edit: one comma in the host-invariance paragraph, revision deliberately
left at r1. Captured `--check` output, verbatim:

```
Error: `…\specmap.json` is out of date relative to the tree.
  drift: unbumped-hash: `spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-host-invariance` content changed while the revision stayed at r1 — editorial, or forgot to bump? (bump `r`, or mark the commit body `spec-editorial: req-conditional-host-invariance`)
Run `cargo xtask specmap`, review the drift, and commit the result.
```

Answered with the GUIDE-SPEC-AUTHORING §4 editorial marker — commit
`73b6e81` carries `spec-editorial: req-conditional-host-invariance` in
its body, the convention's first live use.

## 5. Revert

`4afe716` restores both files to the pilot commit's exact bytes
(verified `git diff 4395d3b --stat` empty over the three files); the
reverse direction produced its own symmetric drift report
(`revision bump r2 → r1` + the editorial hash moving back).

## 6. `trace explain` acceptance

`cargo xtask trace explain vibe_resolver::conditional::ConditionalPredicate::parse --text`, verbatim:

```
code item `vibe_resolver::conditional::ConditionalPredicate::parse`
  fn in vibe-resolver (crates/vibe-resolver/src/conditional.rs:34)
  --implements--> spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar (pinned r1)
      unit: req r1 — The predicate grammar (spec/modules/vibe-resolver/PROP-003-dep-evolution.md:410)
      also: implements ← `vibe_resolver::conditional::ConditionalPredicate` (crates/vibe-resolver/src/conditional.rs:23)
      also: verifies ← `vibe_resolver::conditional::tests::flags_unsupported_richer_forms` (crates/vibe-resolver/src/conditional.rs:135)
      also: verifies ← `vibe_resolver::conditional::tests::parses_simple_present_predicate` (crates/vibe-resolver/src/conditional.rs:99)
      also: verifies ← `vibe_resolver::conditional::tests::parses_with_whitespace` (crates/vibe-resolver/src/conditional.rs:109)
      also: verifies ← `vibe_resolver::conditional::tests::rejects_malformed` (crates/vibe-resolver/src/conditional.rs:119)
  --deviates--> spec://vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition (pinned r1)
      deviation: boolean composition (`and`/`or`/`not`) intentionally unimplemented; every composition form surfaces as PredicateError::Unsupported, pending the PROP-014 pilot decision
      unit: req r1 [PLANNED] — Boolean composition over predicates (spec/modules/vibe-resolver/PROP-003-dep-evolution.md:434)
```

The `planned`/`deviates` relationship renders exactly as the playbook's
acceptance line requires. `--json` emits the same subgraph as raw data.

## 7. Acceptance checklist (playbook Phase 1)

- [x] Additions-only `req` markers in PROP-003 §2.6.1 — `4395d3b`;
      review in-conversation per the owner's direction.
- [x] Tags on `conditional.rs`: implements / deviates(+reason) /
      verifies — `4395d3b`.
- [x] Drill (a): bump → suspects reported → re-affirm → green —
      `b3a947c`, §3 above.
- [x] Drill (b): typo edit without bump → hash warning — `73b6e81`,
      §4 above.
- [x] Spec change reverted — `4afe716`.
- [x] `xtask trace explain … --text` emits the correct subgraph incl.
      the planned/deviates relationship — §6.
- [x] Index deterministic (double `--check` clean at every step).
- [x] `cargo xtask test-gate` exits 0 — 1051 results, 0 failed,
      3 skipped (the quarantined live trio), xfail-strict.
- [x] Full `self-check.sh` green.

Standing observations: the six `pin-into-unmarked-unit` warnings from
specmark's own usage tests remain by design — they retire when PROP-014
is unit-ified after ratification. Tripwire note for this change set
(PLAYBOOK §7.5): DBT-0011's `touch:crates/vibe-resolver/**` fires —
addressed: the pilot tags the module without touching solver behaviour;
the SAT-solver debt itself is untouched and stays open.
