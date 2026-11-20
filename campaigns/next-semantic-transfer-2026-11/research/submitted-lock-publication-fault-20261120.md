# Submitted lock publication fault — actual integration qualification

Contract `NST-SUBMITTED-LOCK-PUBLICATION-FAULT-01`, architect `783c881600fd7482919253c55794857efd4abc16`, integrated `41138495d`. Test/report only; no production switch, transaction change, schema change or permission expansion. Exact filename/contract title follows the integrated contract; this report qualifies only the named fault boundary.

## Actual boundary, not an early refusal

The integration fixture uses a real local retained-metadata solve and acquisition, `plan_prepared_with_submission` returning Ready, then explicitly invokes canonical lifecycle apply with consenting test metadata. This direct library operation is explicit test consent; it is NOT a CLI/PTY confirmation test. Existing public file/PTY consent tests remain separate evidence.

The existing safefs `inject-failures` dev feature is already enabled by vibe-install's dev dependency. Thread-local `fail_after_publish("vibe.lock")` is armed only for the fault world. This is a deterministic **injected post-publication error**, not an actual disk-full/OS-error demonstration or an entry-only mock. Existing `arm_before_publish_verify` observes the actual destination after rename, before the normal verifier/durability/fault return. It re-arms for unrelated metadata names and observes only the real lock destination.

At that point the real lock parser sees epoch16, submitted input attributed to `./vibe.toml`, original on selection and explicit false parameter; authored consumers are absent. Actual payload slot, generated boot marker and executed pre-hook exist. Provenance publication has not been reached. This discriminates `record_lock -> private proof.encode -> files.lock.publish` from solve/consent/source validation or the later provenance write. The matched control uses the same actual source/plan/apply and observer without the fault.

## Initial outcomes and rollback limits

Legacy direct-apply first runtime passed the control/fault pair, but that API returns only Error on failure; it was not credited as canonical lifecycle-report evidence. The fixture was then switched to existing canonical lifecycle apply. Runtime r2 passed: success recorded epoch16, payload/boot and pre/post hooks; fault returned the targeted metadata publication primary inside `Error::SlotFailed`, with successful pre-install lifecycle report and incomplete/nonfresh progress. No successful post-install report appeared.

After fault: lock and provenance remain absent as before, package slot is removed, boot file contents return to their prior state, root manifest and authored launcher bytes remain unchanged. No `RollbackFailed` was reported. The pre-hook's synthetic external logfile remains with one `pre` line; post-hook is absent. This is not rollback of external effects and not a promise to remove package-authored side effects. Framework-created empty directories or lifecycle coordination evidence are not represented as zero filesystem writes. The test asserts the actual framework-owned files/slots and records the truthful failure carrier, not a fabricated success rollback report.

Build r1 retained RED on test callback capture ownership; fixed by cloning the captured fixture state. Build r3 retained RED on an incorrect RunAgentMode import; fixed to its generated lifecycle owner. These are fixture compile failures, not publication observations. Final strengthened exact originals/provenance-order/progress assertions passed in canonical runtime r3 (one test containing matched control and fault, zero filtered). Exact executable SHA-256 `03f1e13964d13f9edf67e4182b6f9ab75d22c62cd34885b90e05ca345f793fee`. Strict affected test lint and separate workspace fmt/scoped diff checks passed. Independent final exact-artifact acceptance remains pending. No ordinary metadata regression is substituted for this submitted-specific receipt.

No network, provider, USER-PROJECT runner, host credential access or real external operation. No parent/NST promotion. A passing finite injection qualifies this publication/undo boundary, not every possible I/O fault or crash recovery scenario.
