# ADV-CONTROL-MCP-PROGRESS-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** lifecycle_run, called with a per-call CallControl, reports its phases and contributions as progress tasks and stops at the next safe point once the control's token is cancelled. It then returns its report with isError true and text starting "cancelled: ".

**Write.** `crates/vibe-mcp/src/tools.rs`, `crates/vibe-mcp/src/tools/lifecycle_run.rs`, `crates/vibe-mcp/src/tools/lifecycle_run/ports.rs`, `crates/vibe-mcp/src/tools/lifecycle_run/tests.rs`, `crates/vibe-mcp/tests/lifecycle_run/outcomes.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test lifecycle_run controlled_`

**Test.** crates/vibe-mcp/tests/lifecycle_run/outcomes.rs. Shared fixture: project("") plus appended rows `[[extension]] id="first"|"second"|"third"`, point = "phase:build", handler = { kind = "builtin", name = "log" }, config = { message = "<ID>-RAN" }. Test observer `Recorder` implements vibe_core::progress::ProgressObserver + Send + Sync and is built with `Recorder::new(cancel_on: Option<(String, CancellationToken)>)`. It maps task id to label on Started, pushes ("start"|"finish"|"fail"|"skip", label) into a Mutex<Vec<(String,String)>>, and when cancel_on is Some((label, token)) calls token.cancel() when that label's Finished event arrives. Construction order in every test: `let token = CancellationToken::new(); let recorder = Arc::new(Recorder::new(<cancel_on using token.clone()>)); let control = CallControl::new(Progress::new(recorder.clone()), token);`. (1) fn controlled_run_reports_phase_and_contribution_tasks: cancel_on None; runs `LifecycleRunMcpTool.run_controlled(&json!({"phase":"build"}), &context(root), &control)` and unwraps Ok. Asserts !output.is_error() and the recorded list contains ("start","Running build lifecycle phase"), ("start","Running contribution org.demo/demo#first") and ("finish","Running contribution org.demo/demo#first"). (2) fn controlled_cancel_after_first_contribution_returns_a_cancelled_report: cancel_on Some(("Running contribution org.demo/demo#first", token.clone())). Asserts output.is_error(), output.text().starts_with("cancelled: "), report(&output).ok == false, exactly one contribution row has status "ok" and its key ends with "#first", and no row has a key ending with "#third".

**Read first.**
- crates/vibe-mcp/src/tools.rs:40-70
- crates/vibe-mcp/src/tools/lifecycle_run.rs:95-230
- crates/vibe-mcp/src/tools/lifecycle_run/ports.rs:40-115
- crates/vibe-cli/src/commands/lifecycle/observer.rs:47-146
- crates/vibe-core/src/cancellation.rs:131-190
- crates/vibe-mcp/tests/lifecycle_run/outcomes.rs:1-80

**Steps.**
1. In crates/vibe-mcp/src/tools.rs add `#[derive(Clone, Default)] pub struct CallControl { progress: vibe_core::progress::Progress, cancellation: vibe_core::cancellation::CancellationToken }` with `pub fn inert() -> Self` (= Default), `pub fn new(progress: Progress, cancellation: CancellationToken) -> Self` (stores `progress.with_cancellation(cancellation.clone())` and the token), `pub fn progress(&self) -> &Progress` and `pub fn cancellation(&self) -> &CancellationToken`. Add a provided method to McpTool: `fn run_controlled(&self, args: &Value, ctx: &ServerContext, control: &CallControl) -> Result<ToolOutput, ToolError> { let _ = control; self.run(args, ctx) }`.
2. In ports.rs change HostedRunObserver into `pub(crate) struct HostedRunObserver { progress: Progress, cancellation: CancellationToken, observed: Mutex<ObservedTasks> }`, where ObservedTasks holds phases: BTreeMap<String, ProgressTask>, contributions: BTreeMap<String, ProgressTask> and completed_phases: BTreeSet<String>. Add `pub(crate) fn new(control: &CallControl) -> Self`. Implement cancellation() (returns the token clone), observe_phase_started, observe_phase_finished, observe_contribution_started and observe_contribution_terminal with the labels and status mapping in `decisions`, copied from CliRunObserver (crates/vibe-cli/src/commands/lifecycle/observer.rs:51-146) minus every StreamMode::Inherit branch, because this surface always captures.
3. In ports.rs change HostedInstallObserver into `pub(crate) struct HostedInstallObserver { progress: Progress }` with `pub(crate) fn new(control: &CallControl) -> Self`, and override `fn progress(&self) -> Progress { self.progress.clone() }`.
4. In lifecycle_run.rs move the body of `run` into an override of `run_controlled`, building the observers with `HostedRunObserver::new(control)` and `HostedInstallObserver::new(control)`. Make `run` call `self.run_controlled(args, ctx, &CallControl::inert())`. In the `Some(original)` arm, if `control.cancellation().acknowledged()`, the text becomes `format!("cancelled: lifecycle_run stopped at a cooperative checkpoint; publication: {}; {original:#}", control.cancellation().publication_state())`; otherwise the text stays as today.
5. Update crates/vibe-mcp/src/tools/lifecycle_run/tests.rs so hosted_ports_capture_and_never_claim_an_immediate_machine_failure builds the observers with `::new(&CallControl::inert())`.
6. Add the two tests to outcomes.rs, run the check, then `cargo test -p vibe-mcp --lib lifecycle_run` and `cargo test -p vibe-mcp --test lifecycle_run`.

**Decisions already made.**
- CallControl is public at vibe_mcp::tools::CallControl and is the only per-call channel. ServerContext stays unchanged.
- Phase label: "Running {phase} lifecycle phase". Contribution label: "Running contribution {key}".
- Phase status mapping: ok→finish; fresh→skip("phase inputs are fresh"); no-op→skip("phase selected no work"); fail→fail("lifecycle phase failed") always, since the phase observer receives no message; any other→skip("phase ended with {other}").
- Contribution status mapping: ok→finish; fresh→skip("contribution inputs are fresh"); skip→skip("contribution skipped"); delegated→skip("waiting for the hosting agent"); cancelled→skip("parked contribution was cancelled"); fail→fail(report.message, or "lifecycle contribution failed" when it has none); any other→skip("contribution ended with {other}"). Where CliRunObserver's text differs from this list, copy CliRunObserver's text.
- The report shape does not change: a cancelled run returns the existing failed lifecycle report under isError:true. Only the text channel says `cancelled: `.
- The inert control (CancellationToken::default()) keeps today's uncontrolled process path, so calls through `run` behave exactly as before. Tests use CancellationToken::new(), because only a controlled token makes per-row checkpoints run.
- Cancellation in the test is triggered from inside the progress observer on a specific event, so the test needs no timing.

**Not in this entry.** Any change to lib.rs or the dispatcher (no notifications/progress or notifications/cancelled handling yet), and extracting a shared phase-progress helper out of vibe-cli.

Entries that write the same file run one after another, never in parallel.
