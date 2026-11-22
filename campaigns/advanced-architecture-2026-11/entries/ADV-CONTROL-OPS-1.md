# ADV-CONTROL-OPS-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The operation record and the operations listing are registered formats (cli-operation-record, cli-operations-listing) with generated readers in vibe-wire, and a corpus test round-trips both.

**Write.** `schemas/operation_record.jtd.json`, `schemas/operations_listing.jtd.json`, `formats/REGISTRY.toml`, `crates/vibe-wire/src/generated/**`, `crates/vibe-wire/tests/operation_record_wire.rs`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-wire --test operation_record_wire 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-wire/tests/operation_record_wire.rs, fn a_record_and_a_listing_survive_the_generated_readers (new file; today `--test operation_record_wire` names no target, so the check fails).

**Read first.**
- schemas/transfers_listing.jtd.json (the pattern: metadata.description/spec/rustOptions/x-rust-type/x-wire-order, enums with x-vocabulary, arrays with x-empty)
- formats/REGISTRY.toml blocks [format.cli-transfers-listing] and [format.cli-transfer-event]
- crates/vibe-wire/tests/format_registry_completeness.rs fn format_id_completeness (registry and FormatId must agree; `cargo xtask codegen` regenerates FormatId)
- tools/jtd-codegen/README.md (the binary is gitignored; install recipe in vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md)
- vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml section OPERATION-EFFECTS (unit markup to copy)

**Steps.**
1. In vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml replace the exact text
```
  </OPERATION-EFFECTS>
</spec>
```
with
```
  </OPERATION-EFFECTS>
  <OPERATION-RECORD title="Operations that outlive the client that started them">
    <p>`req r1`</p>
    <p><OPERATION-RECORD-FORMAT fact="true" status="impl/done">The registered format cli-operation-record (protocol vibe-operation/1, schemas/operation_record.jtd.json) has the fields protocol, id, state (running, finished, failed, stopped or lost), machine, home, cwd, pid, argv, invoked_by, started, ended, exit_code, documents (a list of JSON values) and error (a JSON value or null). The registered format cli-operations-listing (protocol vibe-operations/1, schemas/operations_listing.jtd.json) has protocol, machine, home and operations, a list of such records. vibe-wire has generated readers for both.</OPERATION-RECORD-FORMAT></p>
    <p><OPERATION-CLIENT-EXIT fact="true" status="spec/plan">A run whose standard output or standard error is closed or broken, because the client that started it exited or crashed, continues to its own end as far as its writes go through vibe's shared output layer: the JSON documents, the JSON error envelope, ordinary lines, diagnostics, trailers and the cancellation report. Those writes ignore a failed write, and the exit code is the one the run would have had with a reader attached. The disappearance of the client is never a cancellation. A command that prints its JSON report with its own println outside that layer still ends on a broken pipe.</OPERATION-CLIENT-EXIT></p>
    <p><OPERATION-RECORD-WRITE fact="true" status="spec/plan">The global --operation-id ID (1 to 64 characters from A-Z, a-z, 0-9, - and _; a ULID fits) together with --json keeps a record at SETTINGS/operations/ID.json, where SETTINGS is the settings root as the process started, before user-config [env] promotion. The record is written in state running before any command work, then with the final state, exit code, documents and error when the process ends, also after a panic (exit 101). Exit 0 is finished, 130 is stopped, any other code is failed. machine is the host name of the machine the process runs on and home is SETTINGS, so a record never assumes a single host. argv is the arguments after the program name, with URL credentials redacted and every --set value replaced by ***. documents is every JSON document the run wrote through vibe's shared output layer, in order; a command that prints its JSON report directly is recorded without that report. error is the JSON error envelope, or null. Without --operation-id nothing is recorded. --operation-id without --json is refused with exit 2 before anything runs; an ID outside the alphabet is refused with code operation_id_invalid and exit 2.</OPERATION-RECORD-WRITE></p>
    <p><OPERATION-QUERY fact="true" status="spec/plan">vibe operations show ID answers one record, and vibe operations list answers cli-operations-listing: this machine, the settings root and every record under it, newest first by started; ops is an alias. Both read the settings root as the process started, before user-config [env] promotion. A record in state running whose machine is this machine and whose process is gone is answered as lost; reading never changes a record. A record from another machine is answered as written.</OPERATION-QUERY></p>
    <p><OPERATION-RETRY fact="true" status="spec/plan">Starting an operation whose ID is already recorded: a finished, failed or stopped record with the same stored arguments is replayed (the documents it holds on stdout, its error envelope on stderr, its exit code) and nothing runs again; different arguments are refused with code operation_id_conflict; a running record on another machine, or on this machine with its process alive, is refused with code operation_running naming its pid and machine; a lost record is replaced by the new run; an unreadable record is refused with code operation_record_unreadable. Refusals exit 2.</OPERATION-RETRY></p>
    <p><OPERATION-RETENTION fact="true" status="spec/plan">Before a new record is written, finished, failed and stopped records that ended more than 30 days ago are removed, and of the remaining ones only the 200 newest by started are kept. Running and lost records are never removed by retention.</OPERATION-RETENTION></p>
    <p><LEASE-HOLDER fact="true" status="spec/plan">The process holding a workspace lifecycle lease records itself in .vibe/lifecycle.holder.json (pid, machine, started) after acquisition and removes it on release. A Busy refusal names that holder when the file is readable. Process death leaves the file and the next holder overwrites it.</LEASE-HOLDER></p>
    <p><BUSY-HOLDER-ENVELOPE fact="true" status="spec/plan">Under --json a lifecycle Busy refusal carries code project_busy, root, and holder {pid, machine, started, operation}: operation is the ID of a running record under this settings root with the same pid and machine, or null; holder is null when no holder is recorded. The exit code is unchanged.</BUSY-HOLDER-ENVELOPE></p>
  </OPERATION-RECORD>
</spec>
```
2. Create schemas/operation_record.jtd.json with exactly this content:
```
{
  "metadata": {
    "description": "Version 1 record of one operation started with --operation-id (PROP-073#OPERATION-RECORD-FORMAT): what ran, on which machine and under which settings root, and its outcome once the process ended. Kept at SETTINGS/operations/ID.json and answered by vibe operations show ID --json.",
    "spec": { "implements": "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-RECORD-FORMAT" },
    "rustOptions": { "package": "vibe_wire::operation_record" },
    "x-rust-type": "OperationRecord",
    "x-wire-order": ["protocol", "id", "state", "machine", "home", "cwd", "pid", "argv", "invoked_by", "started", "ended", "exit_code", "documents", "error"]
  },
  "properties": {
    "protocol": { "type": "string" },
    "id": { "type": "string" },
    "state": { "ref": "state" },
    "machine": { "type": "string" },
    "home": { "type": "string" },
    "cwd": { "type": "string" },
    "pid": { "type": "uint32" },
    "argv": { "elements": { "type": "string" }, "metadata": { "x-empty": "emit" } },
    "invoked_by": { "type": "string", "nullable": true },
    "started": { "type": "string" },
    "ended": { "type": "string", "nullable": true },
    "exit_code": { "type": "uint32", "nullable": true },
    "documents": { "elements": {}, "metadata": { "x-empty": "emit" } },
    "error": {}
  },
  "definitions": {
    "state": {
      "enum": ["running", "finished", "failed", "stopped", "lost"],
      "metadata": { "x-vocabulary": "closed" }
    }
  }
}
```
3. Create schemas/operations_listing.jtd.json: top-level metadata description "Version 1 answer of vibe operations list --json (PROP-073#OPERATION-RECORD-FORMAT): the machine and settings root asked, and every operation record under it, newest first by started.", the same spec.implements as step 2, rustOptions.package "vibe_wire::operations_listing", x-rust-type "OperationsListing", x-wire-order ["protocol", "machine", "home", "operations"]; properties protocol, machine and home each {"type": "string"} and operations {"elements": {"ref": "record"}, "metadata": {"x-empty": "emit"}}; definitions: "state" copied byte for byte from step 2, and "record" = {"metadata": {"x-wire-order": <the 14-name list from step 2>}, "properties": <the properties object from step 2, unchanged>}.
4. In formats/REGISTRY.toml insert directly after the [format.cli-transfer-event] block (after its `sunset = "none"` line and the blank line that follows it):
```
# A record of one operation started with `--operation-id` (PROP-073
# `#OPERATION-RECORD-FORMAT`): `SETTINGS/operations/ID.json` and
# `vibe operations show ID --json`.
[format.cli-operation-record]
epoch = 1
schema = "schemas/operation_record.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

# `vibe operations list --json` (PROP-073 `#OPERATION-RECORD-FORMAT`).
[format.cli-operations-listing]
epoch = 1
schema = "schemas/operations_listing.jtd.json"
recoverable = true
foreign_parsers = "many"
corpus = "none"
sunset = "none"

```
5. Generate: the gitignored binary tools/jtd-codegen/jtd-codegen (jtd-codegen.exe on Windows) must exist in the worktree; if it is missing, copy it from the main checkout's tools/jtd-codegen/ or install it per vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md. Run `cargo xtask codegen`. Keep every resulting change under crates/vibe-wire/src/generated/ (new operation_record/ and operations_listing/ modules, the generated mod.rs, format_id/). Expected shapes (observed by the skeptic in a scratch run): operation_record::{OperationRecord, State} and operations_listing::{OperationsListing, Record, State}; documents is Vec<Option<serde_json::Value>>, error is Option<serde_json::Value>, nullable fields are Option. If codegen refuses a schema, fix only the schema metadata it names, keeping fields, names and types as written above.
6. Create crates/vibe-wire/tests/operation_record_wire.rs with one test fn a_record_and_a_listing_survive_the_generated_readers. It builds with serde_json::json! one finished record {"protocol":"vibe-operation/1","id":"01JTESTOPERATION0000000001","state":"finished","machine":"build-host","home":"/home/user/.vibe","cwd":"/work/demo","pid":4242,"argv":["--json","--operation-id","01JTESTOPERATION0000000001","install"],"invoked_by":"vibevm-app","started":"2026-11-22T10:00:00Z","ended":"2026-11-22T10:00:09Z","exit_code":0,"documents":[{"command":"install","ok":true}],"error":null} and one running record (same fields, state "running", invoked_by null, ended null, exit_code null, documents []). It deserialises each into vibe_wire::generated::operation_record::OperationRecord and asserts that serde_json::to_value of the typed value equals the input. It builds a listing {"protocol":"vibe-operations/1","machine":"build-host","home":"/home/user/.vibe","operations":[finished, running]}, round-trips it through vibe_wire::generated::operations_listing::OperationsListing the same way, and asserts that FormatId::ALL mapped through FormatId::id contains "cli-operation-record" and "cli-operations-listing".
7. Run `cargo test -p vibe-wire --test format_registry_completeness`, `cargo fmt -p vibe-wire` and `cargo clippy -p vibe-wire --all-targets -- -D warnings`. Commit schemas, registry, generated code, test and spec together.

**Decisions already made.**
- Format ids are cli-operation-record and cli-operations-listing; protocols are vibe-operation/1 and vibe-operations/1; Rust types are vibe_wire::generated::operation_record::OperationRecord and vibe_wire::generated::operations_listing::OperationsListing (listing element type Record, enum State in both).
- States: running, finished, failed, stopped, lost (closed vocabulary). pid and exit_code are uint32 (Windows exit codes can exceed 255).
- documents is a JTD array of empty-schema elements and error is the empty schema: arbitrary JSON reports and envelopes are stored as written.
- corpus = "none" (as the transfers formats): the inline JSON in the test is the corpus.
- The listing duplicates the record definition inside its own schema (precedent: transfers_listing and transfer_event each define record). No shared vocabulary in formats/vocabularies.json.
- OPERATION-RECORD-FORMAT states only the shape and the generated readers, which this entry delivers entirely, so it is impl/done here. The meaning of the fields (machine, home, argv, documents) lives in OPERATION-RECORD-WRITE and the listing command in OPERATION-QUERY; those stay spec/plan.
- The flag is --operation-id, not --operation: `vibe app resume <package> --operation <ID>` already uses the id `operation`, and clap copies a same-id subcommand value up to a global arg.
- This entry creates the whole OPERATION-RECORD spec section so later entries only flip their own unit status. Every later entry edits this PROP-073 file, so the entries run one after another.

**Not in this entry.** No CLI code that writes or reads records, and no corpus directory under formats/corpora. Do not touch formats/vocabularies.json or any other schema.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml: insert the OPERATION-RECORD section (step 1 literal) after </OPERATION-EFFECTS>. OPERATION-RECORD-FORMAT is impl/done and the other seven units are spec/plan.

Entries that write the same file run one after another, never in parallel.
