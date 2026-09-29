# D1 foundation independent review

Read-only review; no Cargo/test/Git/source/runtime action. Retained logs show the foundation check completed and the selected validation family8passed/0failed/546filtered. Runtime apply/inspect/recover is not implemented in the reviewed cells; no platform/effect proof is claimed.

## One blocking retained-evidence seam

/home/olegchir/git/v/vibevm-next/crates/vibe-workspace/src/migration_application/report.rs, ReportBuilder::into_report consumes self, calls validate_operation_report with ?, and returns only ApplicationValidationError on failure. The builder's OperationReport—including already observed completed/uncertain effects, warnings and residue—is dropped. ReportBuilder::preflight constructs a fresh empty-effects report and cannot recover those observations from that error.

Concrete admitted construction path: start a recognized apply builder; record BeforeImage=Completed and Journal=Uncertain; call error(JournalIo, Refused). error intentionally preserves arrays, but validate_operation_report then refuses because an uncertain effect requires Uncertain outcome. into_report's Err contains no prior report/evidence. This is a source-level ownership/error-carriage defect, not a claim that the unimplemented runtime has lost a real file. The existing report_errors_retain_observed_effects_without_fabricated_guarantees case checks the same uncertainty rule by borrowing an altered report, and therefore does not close the consuming-builder retention boundary.

Before runtime uses this completion seam after any write, retain prior observations on invariant failure: for example return the fault together with the owned report/measured-evidence fragments, or validate without consuming the builder so the operation owner can produce a truthful failure report. Do not emit an unchecked incoherent report or fabricate weaker/stronger guarantees merely to make validation pass. A new bounded case should exercise consuming finalization after known completed plus uncertain effects and prove that recovery/report construction can still access them. Root chooses the actual response/error API; no patch is proposed outside that ownership seam.

## Checked limits around the finding

Independent standard-library SHA256 computation over source-defined u64 big-endian UTF-8 byte-length frames reproduces the test's intent, storage-key and both stage golden constants exactly, including op-é. Domains, field order, mode string and phase exclusion match reqr3; planner hashing is not duplicated.

Semantic corpus setup patches authority spellings to the actual fixture grant. Request schema refusal precedes grant checks; identity mismatch follows exact replay. Journal tests patch physical-token syntax and assert schema/protocol or length fields, so the later stale intent hash cannot mask those intended failures. Report schema/external-writer-exclusion cases hit their specific early checks. Generated required-nullable decode and unknown-field refusals remain distinct from semantic admission. Frozen effectful plans and supplied planner-ID mismatch are rejected; actual target-byte comparison, physical binding equality, retained before.bin proof and before-write ordering remain runtime obligations, not foundation completion claims.

Report initialization is unestablished/unconfirmed, compiled platform only, no effects and no external-writer exclusion; error() itself does not wipe arrays. No additional concrete blocking issue was found in the requested hash or logical admission review. This does not accept runtime, platform or D1 completion. Writer stopped.

Root accepted the retention finding and reports a candidate ReportValidationFailure carrying the original generated report plus fault is being implemented, with a consuming-finalization regression. That candidate has not been independently re-read or verified here; the finding is not closed by this status update. No expanded review was performed.
