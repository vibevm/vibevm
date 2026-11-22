# ADV-CONTROL-OPS-8

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Under --json a project-busy refusal carries code project_busy, root and holder {pid, machine, started, operation}, where operation is the holding operation's ID; the exit code stays 1.

**Write.** `crates/vibe-cli/src/output/structured.rs`, `crates/vibe-cli/src/composition/operation.rs`, `crates/vibe-cli/tests/cli_operation_busy.rs`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-cli --test cli_operation_busy 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_operation_busy.rs, fn a_busy_refusal_names_the_holding_operation. Verified in round 1: a busy `vibe --json install` prints {"ok":false,"error":"taking the lifecycle mutation lease at ..."} with no code and exits 1.

**Read first.**
- crates/vibe-cli/src/output/structured.rs: fn stamp_structured_error (the call list ending in `stamp_user_project(payload, err);`) and fn stamp_user_project (chain downcast pattern)
- crates/vibe-orchestrator/src/install/lease.rs fn acquire_lease (wraps LifecycleLeaseError with `.with_context`, so err.chain() reaches it)
- vibe_lifecycle::LifecycleLeaseError::Busy { root, holder } and LeaseHolder (from OPS-2)
- crates/vibe-cli/src/composition/operation.rs: record_home, all (from OPS-4 and OPS-5)
- crates/vibe-cli/tests/cli_lifecycle_lease.rs: fns canonical and hold (how the test process holds the same lock the child takes)

**Steps.**
1. composition/operation.rs: add `pub(crate) fn running_id(pid: u32, machine: &str) -> Option<String>`, which uses `let home = record_home()?;` and returns the id of the first record in all(&home) with state Running, pid == pid and machine == machine.
2. output/structured.rs: add `fn stamp_project_busy(payload: &mut Value, err: &anyhow::Error)`. It finds `err.chain().find_map(|cause| cause.downcast_ref::<vibe_lifecycle::LifecycleLeaseError>())`. Only for `vibe_lifecycle::LifecycleLeaseError::Busy { root, holder }` with payload a Value::Object does it insert code = "project_busy", root = root.display().to_string(), and holder = (when Some(h)) `serde_json::json!({"pid": h.pid, "machine": h.machine, "started": h.started, "operation": crate::composition::operation::running_id(h.pid, &h.machine)})` or Value::Null (when None). The enum is non_exhaustive: use `if let` on the Busy variant, not a match. Call it in stamp_structured_error directly after `stamp_user_project(payload, err);`.
3. Test. Copy fn canonical from cli_lifecycle_lease.rs. Set up UserScratch, an init'd tempdir project, and `let lease = vibe_lifecycle::LifecycleLease::acquire(&canonical(project.path())).unwrap();` held for the run. Plant SETTINGS/operations/HOLDER-OPERATION.json (planting shape of cli_operations_query.rs) with state running, pid std::process::id() and machine vibe_lifecycle::LeaseHolder::current().machine. Run user.vibe() with `--json install --path <project> --assume-yes` via .output(). Assert exit code Some(1). Parse the last non-empty stderr line and assert code "project_busy", holder.pid == std::process::id(), holder.machine == the current machine, holder.operation == "HOLDER-OPERATION", holder.started a non-empty string, and root a non-empty string. Then drop(lease). Flip the spec unit.
4. Run `cargo test -p vibe-cli --test cli_lifecycle_lease`, `cargo fmt -p vibe-cli`, `cargo clippy -p vibe-cli --all-targets -- -D warnings` and `wc -l crates/vibe-cli/src/output/structured.rs` (376 today; must stay at 600 or less).

**Decisions already made.**
- The code is snake_case project_busy, following the codes already in structured.rs and vibe-app.
- The exit code is unchanged (1 today); codes are additive only.
- The holder comes from the lease's holder file (OPS-2), so a holder without --operation-id is still named by pid and machine, with operation null.
- running_id reads record_home(), the same pre-promotion root the records are written under.
- Runs last: shares composition/operation.rs with OPS-5 to OPS-7.

**Not in this entry.** No change to exit codes, no `refusal` or `remedy` fields, and no codes for other lease variants (Directory, RootMismatch, SelectedNodeMismatch) or other refusals: those belong to the stable-codes capability. No human-mode text change beyond what OPS-2 already rendered.

**Depends on.** ADV-CONTROL-OPS-2, ADV-CONTROL-OPS-7

**Spec edit (apply as a step of this entry).**

PROP-073: replace `<BUSY-HOLDER-ENVELOPE fact="true" status="spec/plan">` with `<BUSY-HOLDER-ENVELOPE fact="true" status="impl/done">`.

Entries that write the same file run one after another, never in parallel.
