# C2 generator prerequisites — worker report

Candidate implementation, not root acceptance. Writer stopped. Eight owned files changed, no dependencies/exports/commands/schema/registry/generated/source-law edits:

- xtask/src/codegen/layout.rs: core-ai-native schema/output home points to v1.1.0.
- xtask/src/codegen/typescript.rs: existing doc-manifest TS target points to web/v1.1.0.
- xtask/src/codegen/mod.rs: pinned generator diagnostic README points to jtd-codegen/v1.1.0.
- xtask/src/codegen/shared_module.rs: shared policy uses the existing registry loader's effective unknown_fields; duplicate schema owners must agree, deny→strict and allow→permissive, mixed fragment consumers refuse.
- xtask/src/codegen/shared_module/tests.rs: five affected policy fixtures use effective-policy strings; new policy test module registered.
- xtask/src/codegen/shared_module/policy_tests.rs: six real-registry cases exercise explicit overrides, legacy defaults, shared/consumer attribute equality including an empty struct, idempotency and duplicate/mixed-policy refusals.
- xtask/src/codegen/strictness.rs: consumer map/stamp uses the same effective policy, including none+allow and many/ours+deny; no second registry parser or separate loader-default logic.
- xtask/src/codegen/strictness/tests.rs: conflict case and diagnostic assertions describe effective policy; original default/skip/refusal/idempotency coverage retained.

The three new source locations existed before path edits. Effective defaults remain computed only by format_id::load_format_registry: absent legacy policy is deny for none, allow for ours/many. Both consumer and shared maps reject conflicting duplicate ownership rather than selecting the first. Existing struct stamping is reused; postproc's explicit-deny pass remains unchanged/idempotent. Historical shared-type measurement counts remain explicitly historical. No request optional-string/null or unrelated reader behavior was pursued.

## Actual selected native results

All commands used CARGO_TARGET_DIR=/home/olegchir/git/v/vibevm-next/target, CARGO_BUILD_JOBS=4 and cargo test --locked --offline -p xtask --bin xtask. Twenty distinct affected cases passed:

- codegen::strictness::tests:: family: exit0,9passed/0failed/299filtered; /tmp/vibevm-next-M13-C2-generator-prereq-consumer.log.
- codegen::shared_module::tests::policy_tests:: family: final exit0,6passed/0failed/302filtered; /tmp/vibevm-next-M13-C2-generator-prereq-shared-registry-retry.log.
- Five exact codegen::shared_module::tests selectors below each exit0,1passed/0failed/307filtered; logs /tmp/vibevm-next-M13-C2-generator-prereq-shared-exact-1.log through -5.log, in listed order. Each used -- --exact:

1. unanimous_deny_consumers_emit_one_strict_shared_struct.
2. rewrite_generated_runs_the_shared_strictness_slot.
3. all_permissive_consumers_preserve_shared_bytes.
4. mixed_consumers_refuse_with_both_sets_named.
5. unanimous_strict_enum_and_scalar_keep_valid_generated_shape.

The first real-registry family failed exit101,0passed/6failed before policy evaluation: I gave fixture IDs owner-0/owner-1, whose numeric segments the existing loader rejects. I corrected only IDs/expectations to owner-row0/owner-row1 and repeated the same six cases. Failed attempt retained in /tmp/vibevm-next-M13-C2-generator-prereq-shared-registry.log. No product/library diagnostic drove a behavior weakening. All logs are actual redirected stdout+stderr including compiler output and existing manifest/safefs warnings, not reconstructed completion excerpts.

Targeted rustfmt used skip_children=true on only the eight owned files and exited0. Final line counts: layout222,typescript229,mod491,shared_module591,shared tests566,strictness261,strictness tests322,new policy tests151; all≤600. Static selected-file diff reviewed: seven tracked edits plus the new151-line policy_tests.rs (untracked source is included in this report). No fmt-all, whole-xtask tests, clippy, codegen/check-codegen/wire-diff, index generation, provider/public action or Git mutation ran.

Root owns final review, commit, schema emission and semantic acceptance. These cases prove the selected policy/emit-text seams, not newly generated migration reader behavior or completed transport/API/runtime closure.
