# M-05-B.1 review: changes required before full-task acceptance

Reviewed only the isolated worktree /home/olegchir/.codex/worktrees/vibevm-m05b1-parallel/vibevm-next. All 20 unique baseline/final subjects matched candidate expectations before review and remained unchanged afterward (17 baseline paths, 3 new paths; 8 final candidate hashes). No tests/builds or product edits were performed.

## B1-F1 [P2] Update the two active defect-characterization tests affected by B1

[crates/vibe-resolver/src/m_05_a_2_cases.rs:23](/home/olegchir/.codex/worktrees/vibevm-m05b1-parallel/vibevm-next/crates/vibe-resolver/src/m_05_a_2_cases.rs:23)

qualified_forwarding_first_slash_defect expects dependency org.example and target feature dep/fast for org.example/dep/fast; the new shared parser correctly returns org.example/dep and fast. exclusive_at_most_one_and_all_features_validation_defect expects one diagnostic for a valid exclusive table with no defaults; the new validator correctly returns none. Both remain ordinary compiled #[test] cases in the library.

The new behavior is intended, but the candidate leaves two active tests guaranteed to fail and permanent M05A2 characterization wording unreconciled with the new implementation. Selected V4 omits these directly affected cases.

Correction: Retain historical evidence, convert the two live cases to corrected regression expectations, and record supersession of the old behavioral assertions without deleting historical receipts. Run only those exact affected cases plus necessary changed B1 checks.

Evidence classification: Source-derived deterministic assertion failures; reviewer did not execute tests.

## B1-F2 [P2] Use available declarations at the current manifest expansion boundary

[crates/vibe-resolver/src/features.rs:169](/home/olegchir/.codex/worktrees/vibevm-m05b1-parallel/vibevm-next/crates/vibe-resolver/src/features.rs:169)

A full manifest with empty requires and default=["dep:org.example/missing"] succeeds through table-only expand_features and returns the undeclared dependency as active. A dormant _private=["org.example/missing?/fast"] also passes. The named install fetch and preview paths already have the full Manifest but discard its requires context when calling this compatibility API. Likewise a declared required dependency plus a weak feature request is never seeded active by this path.

The core declared API correctly refuses unknown IDs and seeds required dependencies, but the required behavior is bypassed by current consumers where the necessary dependency declarations are available. This is a concrete B1 integration gap, not a request for B2 domains, B3 migration, or broad installer redesign.

Correction: Compose the existing declaration normalization and shared evaluator at the named full-manifest boundaries, preserving package-local metadata/aliases and actual active-dependency context; keep the table-only API for callers that genuinely lack declarations. Add a bounded caller-level undeclared/dormant refusal and required weak-request case. Ask Root for the narrow additional write perimeter if needed.

Evidence classification: Source-derived counterexample; no installed integration or runtime reproduction claimed.

## Confirmed implementation and bounded evidence

- One FeatureEffect/FeatureProgram algebra and one expand_program evaluator serve typed and legacy entrypoints; no second choices.requires engine.
- Stable declaration keys preserve metadata once and normalize package-local aliases. Typed/declaration-aware validation covers dormant/private local, dependency, config and exported-subskill references.
- Strong activation, delayed weak forwarding, required/already-active seeds and atomic config-conflict merge implemented in the declared API.
- New wire grammar remains unpublished; no schema/generated/dependency changes. Additive four-anchor permanent spec update preserves prior text.

Producer logs show 27 nonignored successful executions: 9 core + 10 resolver new cases, 3 core exact + 5 resolver exact compatibility cases. Scoped formatting, diff whitespace and permanent XML/prior-text preservation checks report exit0. This is not full-suite or installed integration evidence. Missing case-bindings.json is covered by explicit names/denominators in REPORT.json; no additional gate is requested for that filename.

No FeatureExpansion::merge call exists in the four additionally authorized consumer files. The changed existing resolver merge test and new tests handle Result with unwrap; merge itself checks all config conflicts before mutation. No ignored-result production defect established within the named roster. Root confirmed the named consumer search is sufficient; no broader search or ignored-result finding.

## Runtime evidence note

Root verified actual strict producer/job/work/contract/basis/observation binding matches. The stale C3 candidate label is cosmetic in a fresh store; generic requirement anchors carry original criterion statements in the contract. No product defect or header correction requested.

Root owns scope amendments, managed correction, integration and acceptance. No full suite, new grammar, B2/B3 implementation or optional exhaustive matrix is needed to resolve these findings.
