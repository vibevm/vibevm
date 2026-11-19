# Ordinary package defaults — earliest public-route gap

2026-11-20. Test-only qualification under `NST-PACKAGE-DEFAULT-REVIEW-01.md`, integrated contract `9dcee6d03`; runtime baseline includes accepted CLI `c05ebc333`. No production change or all-kind acceptance.

## Owner map at this parent

- `crates/vibe-core/src/manifest/configuration/package.rs::PackageConfiguration::evaluate_parts` owns ordinary package configuration admission and dependency activation. It explicitly refuses automatic/default/profile basis.
- `crates/vibe-resolver/src/configuration.rs::ConfigurationSnapshot` owns operation-local candidate observations and selected validation; `crates/vibe-install/src/plan.rs` constructs it for ordinary planning.
- Existing install apply/lock publication, lockfile/configuration validation and workspace freshness retain F16's explicit inputs and replay; no parallel persistence is introduced.
- Shared automatic-basis policy from F18 is a foundation, not evidence of an ordinary package shown-review acknowledgment. No package review-response owner was exercised because fresh automatic admission already refused.

## Public observation

The focused test reuses the existing `cli_common_package_optional` Fixture, its local `org.vibevm/oracle-driver@1.0.0` flow, mandatory base, optional extra and exactly-one group defaulting to disabled. The authored request changes only from explicit exact disabled selection to `policy='use-declared-defaults', default_features=false` with the choice omitted. Parser metadata confirms omission remains an empty selection map rather than an expanded explicit answer.

Actual ordinary `install --offline --assume-yes --json` refuses with `automatic/default/profile basis is not connected`. The authored manifest is byte-identical; no lock, driver/extra slot or hook-effects file is published. This is supported refusal, not successful default selection or review/retry. The optional dependency's default is intentionally off; the contract requires stopping at this fresh admission gap before attempting changed-default effects.

Explicit control restores the original authored exact disabled selection against the same package definition. Public installation succeeds; driver materializes, extra remains absent. A second fresh CLI process succeeds with byte-identical lock. This proves the fixture/public route works and F16 explicit replay survives, not that automatic defaults or their review are supported.

Changed definition/update, unchanged automatic replay, stale review response, actual acknowledgment and post-review cold replay are NOT REACHED because the first fresh automatic request cannot be admitted. No synthetic review document, default expansion into authored answers, production fix, schema change or prototype solver was added.

## Evidence

Private retained logs under `/data/git/vibevm-keyring-storage-20261120T0100Z`: `default-review-test-r3.log` has one focused PASS covering the refusal and explicit positive/replay control; `default-review-clippy-r3.log` scoped strict lint PASS, formatting and diff checks PASS. Tests execute in bwrap unshared network/process/filesystem namespaces with cleared environment, synthetic HOME/settings and exact public binary binding.

Earlier retained fixture REDs are not product failures: build r1 used the wrong singular Rust field instead of `Requires.configurations`; test r2 omitted the runtime `CARGO_BIN_EXE_vibe` needed by the existing assert_cmd fixture. Corrected build r2 and execution r3 establish the observed product gap. Exact source/binary hashes are retained in the private candidate manifest. The architecture owner must bound any implementation separately; this qualification does not close NST-3/4 or the ordinary defaults feature.
