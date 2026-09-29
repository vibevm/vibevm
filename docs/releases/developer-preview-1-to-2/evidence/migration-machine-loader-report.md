# C2 explicit policy type follow-up

Writer stopped; candidate implementation, root review/acceptance pending.

Only xtask/src/codegen/format_id.rs and xtask/src/codegen/shared_module/policy_tests.rs changed. The existing registry loader now matches key presence: None retains its existing computed legacy default; Some requires a TOML string or returns a field-named authoring refusal. Existing string allow/deny vocabulary validation remains unchanged. No new parser, default, coercion, schema or registry edit.

The governing PROP-044 migration transport policy says only actual absence inherits defaults; present non-string or unknown policy is a typed authoring refusal. Code and test now distinguish those states directly.

Bound exact native selector:

`CARGO_TARGET_DIR=/home/olegchir/git/v/vibevm-next/target CARGO_BUILD_JOBS=4 cargo test --locked --offline -p xtask --bin xtask codegen::shared_module::tests::policy_tests::explicit_non_string_policy_refuses_without_legacy_default -- --exact`

Actual exit0;1passed/0failed/308filtered. The one real-registry case covers boolean, integer, array and table overrides across none/ours/many, plus explicit allow/deny and absent fallback. Full actual stdout/stderr/compiler output is retained in /tmp/vibevm-next-M13-C2-loader-policy-tests.log. No failure/retry occurred in this follow-up; previous20 cases were not rerun because valid-string/absence semantics remain unchanged.

Targeted rustfmt only these two owners with skip_children=true exited0. Final lengths357/184, both≤600. No generator, whole test panel, public/provider operation, Git mutation or other-owner edit. Root owns commit and emission.
