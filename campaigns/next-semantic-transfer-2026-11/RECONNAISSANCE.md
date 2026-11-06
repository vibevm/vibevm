# Semantic transfer: observed starting state

This is a research record, not product acceptance. Captured 2026-11-03.

## Bound inputs

- Target: `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`, main, version line 1.0.x.
- Committed prototype: `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`, source checkout branch `1.1.0`.
- Common ancestor: `30d217bfce47bcaccadabd9468f00e315dbcfead`.
- `git rev-list --left-right --count <target>...<prototype>`: 158 target-only, 1056 prototype-only commits.
- Source task files contain 222 distinct task records. A refined parent may remain a record; this is not automatically the number of executable leaves or delivered features.
- Source has five existing modified tracked files: workspace slot path limits, slot placement/record, their M-06-G.4 tests and workspace PROP-007. Their contents are excluded from committed-source findings and remain an unaccepted separate candidate.
- Main has an existing untracked `vibevm/vibedeps/org.vibevm.doc.web/`; it is outside this campaign's write perimeter.
- The existing campaign worktree was fast-forwarded from `5e470926a` to the target above. No product patch was applied by that operation.

## Recovery limits

The original campaign XML was untracked and referenced a reconnaissance report, inventory and verification script that did not exist in its worktree. Its claims about earlier reports and remote freshness are historical claims, not independently reproduced facts. This record does not certify them retroactively.

The owner clarified that actual NEXT development ran through Zap. Its durable plan/status must be recovered from that system. Repository seed and stewardship plans are historical requirements/context, not the current execution verdict. The retained graph projection claims revision 131548 and 113 accepted contracts out of 222, but its expiry is 2026-10-19; its literal `availability=current` is not evidence of freshness today. The historical and canonical accepted sets overlap. Database recovery is in progress using read-only access; no service is restarted and no old task is dispatched.

## Findings checked directly against committed source

### Full-pass change execution is unfinished

`crates/vibe-orchestrator/src/change/full_pass.rs::ChangeFullPassPlan::execution_carriage` returns `unavailable:m16-change-full-pass`. The public CLI `crates/vibe-cli/src/commands/change/apply.rs::prepare` reports `prepared`, null production request and null machine operation IDs. Its tests explicitly assert preparation without effects. These tests are useful and honest for a preparation slice; they do not establish an executable full-pass feature.

Transfer implication: review the useful change/intent model separately from the unfinished execution dependency. Do not present the preparation slice as a completed user journey or silently implement all originally planned work.

### Public source queries do not deliver every implemented proof strength

`crates/vibe-workspace/src/source_query/capture/continuity/service.rs` serializes observed and record-consistent results; the SourceProven arm refuses with `verified source proof owner`. Independent proof-building helpers may still be useful. Their tests do not demonstrate public source-proven delivery.

Transfer implication: bind the exact public claim to a concrete consumer and test. Preserve the difference between reading bytes, confirming a stored record and proving original source correspondence.

### A protected build route is a hard-coded demonstration

Production `crates/vibe-lifecycle/src/mechanism/build/journal.rs` accepts a fixed Cargo package `vibe-r8-fixture`, binary `r8probe`, target `fixture-tool`, exact manifest and exact `println!("r8")` program. Its journal protects a bounded demonstration against duplicate execution. The ordinary build mechanism and this protected route are distinct; this observation does not say that all VibeVM builds are restricted to the fixture.

Transfer implication: the need for durable, nonduplicated effects is reasonable; the fixed demonstration is not a general protected-build API. Establish which parts are experiments, which are reusable domain logic and what remains to expose a useful supported operation.

## Causal hypotheses, not blanket verdicts

1. Internal contract completion can be mistaken for user-feature completion. The missing public paths above are concrete discriminating examples.
2. A minimal demonstration can acquire production carriers and surrounding machinery before its general consumer exists. The fixed Cargo route is evidence to investigate, not proof that journaling itself is unnecessary.
3. A generic machine/control prerequisite may delay useful vertical delivery. Evaluate actual dependency chains before copying their DAG.

Counterevidence matters: explicit refusals protect users from unsupported operations, filesystem observations address real race conditions, and targeted negative tests often encode useful invariants. Removing those checks to reduce line count would not improve the design.

## Verification boundary

No product build/test gate has run for this campaign yet. Findings above are source-observed. Inventory reports and old runtime receipts remain evidence to review, not fresh runtime proof. Next: recover Zap's durable plan, reconcile all historical task records and commits, then choose a feature only after necessity/design review.

## 2026-11-06 — additional causes observed during foundation transfer

These are bounded engineering findings, not claims about the previous model's mental state or a blanket verdict on the prototype.

- **Specification evidence can outrun the test.** M13's permanent text claimed historical rename rehearsal; the retained historical scalar case actually asserts refusal, while positive package cases are synthetic. Target M13C1 preserves the useful exact-edit planner and corrects that evidence claim. See research/m13-foundation-review.md and accepted f56a4cba6.
- **A final module can mix several architectural stages.** Prototype migration.rs combines neutral replay with source, typed-feature and Redbook recipes. Main already chose a different configuration owner. Copying the final file would import competing semantics and future prerequisites. The neutral planner was separated; later recipes remain explicit transfer obligations, not empty placeholders.
- **An implemented grammar can be omitted from compatibility inventory.** B2 has a closed work TOML schema1 parser, but the prototype registry names only change-manifest. The transfer completes work-format registration and actual corpus validation instead of treating parser presence as full format integration. B1/B2 acceptance is recorded in 940f93db4.
- **Validation from one domain can be mistaken for another.** The prototype change address constructor accepts a portable filename containing #, then interpolates it into a spec URI, where # changes meaning. A fresh target regression reproduced the wrong address; constructor-only refusal fixes it without banning valid stored paths or changing general literal-percent URI semantics. RED/GREEN evidence is retained in nst-change-address logs; final B1/B2 acceptance is recorded in 940f93db4.
- **Read ownership needs explicit selection.** The prototype saved-change adapter uses a mutation-oriented snapshot type. Main's existing source reader intentionally accepts hardlinks and offers a finite per-file observation window. The transfer uses that owner and tests real hardlink/symlink cases; it does not invent whole-change atomicity or authentication from editable hashes.

These findings support a practical review rule: verify the complete useful behavior and each actual consumer, retain good existing target owners, and separate an unimplemented guarantee from a different but sufficient implementation. Task counts, copied type names, corpus existence and green narrow tests are not substitutes for that review.
