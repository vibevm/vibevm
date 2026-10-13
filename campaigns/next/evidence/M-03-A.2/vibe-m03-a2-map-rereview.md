# M-03-A.2 carrier-map rereview — candidate

Documentation revision2 closes the substantive pending-owner/recipe omissions. Verification evidence-ID and output file/tree witness schedules match actual defining code. Obsolete pending/TO DEFINE table entries are removed, and missing runtime Work/request carriage remains explicitly unavailable.

One small factual correction remains in the Prepared input measurement row: `PreparedDeclaredInputs::manifest` does not exist; the owner is `PreparedInputs::manifest` (`state/fingerprint/inputs.rs:241,273`). Also, `StateInputMeasurement` does not carry typed refusal. `PreparedFingerprint::state_measurement` first obtains its input manifest and calls `manifest.measured()?`, so a `ManifestOutcome::Refused(InputRefusal)` yields no durable input measurement. State the refusal in the prepared outcome, with only successful measured fields in the durable row.

All four new seal links match. Source13 and producer65 hashes match before/after bounded inspection. No source or behavioral change is needed; no tests/builds/API calls were run. Root acceptance remains owed.
