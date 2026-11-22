# ADV-CONTROL-OPS-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe operations show <ID>` and `vibe operations list` (alias `ops`) answer the recorded operations; a running record on this machine whose process is gone is answered as lost, and a record from another machine is answered as written.

**Write.** `crates/vibe-cli/src/cli/operations.rs`, `crates/vibe-cli/src/cli.rs`, `crates/vibe-cli/src/cli/command.rs`, `crates/vibe-cli/src/command_progress.rs`, `crates/vibe-cli/src/main.rs`, `crates/vibe-cli/src/commands/mod.rs`, `crates/vibe-cli/src/commands/operations.rs`, `crates/vibe-cli/src/composition/operation.rs`, `crates/vibe-cli/tests/cli_operations_query.rs`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-cli --test cli_operations_query 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-cli/tests/cli_operations_query.rs, fns show_answers_a_record_and_reports_a_dead_running_process_as_lost and list_answers_every_record_newest_first.

**Read first.**
- crates/vibe-cli/src/commands/transfers.rs: imports (`use vibe_app::{ProcessState, ProcessTable};`, `use super::app::processes::HostProcesses;`), fn alive (`!matches!(HostProcesses.probe(pid), ProcessState::Gone)`), fn wire (serde round trip into a generated shape)
- crates/vibe-cli/src/cli/transfers.rs and the cli.rs lines `mod transfers;` / `pub use transfers::*;`
- crates/vibe-cli/src/cli/command.rs variant `Transfers(TransfersArgs),`
- crates/vibe-cli/src/command_progress.rs: const TOP_LEVEL_COMMANDS (sorted; checked by command_progress/tests.rs fn top_level_policy_inventory_matches_clap) and the `Command::Transfers(args) => match args.command {` arm of the exhaustive policy match
- crates/vibe-cli/src/composition/operation.rs (record_home, record_path, valid_id from OPS-4)
- crates/vibe-wire/src/generated/operations_listing/mod.rs (OperationsListing, Record)

**Steps.**
1. composition/operation.rs: add `pub(crate) fn read(path: &Path) -> std::io::Result<Option<OperationRecord>>`, giving Ok(None) on NotFound and Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)) when serde_json fails. Add `pub(crate) fn all(home: &Path) -> Vec<OperationRecord>`, which reads every `*.json` file in home/operations that read() parses, skips the rest, and returns empty when the directory is absent. Add `pub(crate) fn observed(mut record: OperationRecord, machine: &str) -> OperationRecord`, which sets state Lost when state is Running, record.machine == machine and `matches!(crate::commands::app::processes::HostProcesses.probe(record.pid), vibe_app::ProcessState::Gone)` (import vibe_app::ProcessTable for the method). It never writes.
2. Create crates/vibe-cli/src/cli/operations.rs with `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-QUERY");` and
```
#[derive(Debug, clap::Args)]
pub struct OperationsArgs {
    #[command(subcommand)]
    pub command: OperationsCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum OperationsCommand {
    /// Every operation record under the settings root, newest first.
    List,
    /// One operation record by the ID given to `--operation-id`.
    Show { id: String },
}
```
In cli.rs add `mod operations;` directly after `mod mcp;` and `pub use operations::*;` directly after `pub use mcp::*;`. In cli/command.rs add after `Transfers(TransfersArgs),`:
```
    /// The operations started with `--operation-id` under this settings
    /// root: `list` and `show <ID>` (PROP-073 `#OPERATION-QUERY`).
    #[command(visible_alias = "ops")]
    Operations(OperationsArgs),
```
3. command_progress.rs: insert "operations" into TOP_LEVEL_COMMANDS between "mcp" and "outdated" (byte order: "operations" < "outdated"), and add the arm `Command::Operations(_) => FastMetadata,` directly before `Command::Transfers(args) => match args.command {`. main.rs: add `Command::Operations(args) => commands::operations::run(&ctx, args),` directly before `Command::Transfers(args) => commands::transfers::run(&ctx, args),`. commands/mod.rs: add `pub mod operations;` between `pub mod mcp;` and `pub mod outdated;`.
4. Create crates/vibe-cli/src/commands/operations.rs (with `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-QUERY");`) with `#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-QUERY")] pub fn run(ctx: &output::Context, args: OperationsArgs) -> anyhow::Result<()>`. home = crate::composition::operation::record_home(), or the error "the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable"; machine = vibe_lifecycle::LeaseHolder::current().machine. Show: an ID failing valid_id fails with "operation id `{id}` must be 1 to 64 characters from A-Z, a-z, 0-9, - and _"; a read error fails with "cannot read the operation record `{path}`: {e}"; Ok(None) fails with "no operation `{id}` is recorded under `{home}`"; otherwise answer observed(record, &machine). List: all(&home) mapped through observed, sorted by started descending then id descending, converted element-wise into operations_listing::Record by a serde_json round trip (as transfers.rs fn wire does), wrapped in OperationsListing { protocol: "vibe-operations/1".into(), machine, home: home.display().to_string(), operations }. JSON mode: ctx.emit_json of the record or listing. Human mode uses ctx.block: for show, one block with `<id>  <state>`, then `machine <m>`, `home <h>`, `pid <p>`, `started <s>`, `ended <e or ->`, `exit <code or ->` and `command vibe <argv joined by spaces>`; for list, one line per record `<id>  <state>  <started>  vibe <argv joined>`, or `no operations recorded under <home>` when empty. The state word comes from serde_json::to_value(&state).
5. Tests. A helper plants records as serde_json::json! objects written to SETTINGS/operations/<id>.json with all 14 fields (protocol "vibe-operation/1", id, state as given, machine as given, home = settings display string, cwd "/work", pid as given, argv ["--json","install"], invoked_by null, started as given; for terminal states ended = started, exit_code 0 and documents [{"command":"install","ok":true}], otherwise ended null, exit_code null and documents []; error null). Get a dead pid by spawning env!("CARGO_BIN_EXE_vibe") with --version, reading child.id(), then wait(). (1) Plant OPS-LOST (running, machine = vibe_lifecycle::LeaseHolder::current().machine, dead pid), OPS-DONE (finished) and OPS-FAR (running, machine "another-machine", dead pid). Via user.vibe(): `operations show OPS-LOST --json` gives state "lost" while the file still says "running"; `operations show OPS-DONE --json` gives state "finished" with the planted documents; `operations show OPS-FAR --json` gives state "running"; `operations show NOPE --json` exits non-zero and stderr contains "no operation `NOPE` is recorded". (2) Plant three finished records started 2026-11-01T10:00:00Z, 2026-11-02T10:00:00Z and 2026-11-03T10:00:00Z. `operations list --json` gives protocol "vibe-operations/1", home == settings, machine == the current machine, and ids newest first; `ops list --json` gives the same ids. Flip the spec unit.
6. Self-verify: `cargo test -p vibe-cli --bin vibe top_level_policy_inventory_matches_clap 2>&1 | grep -F "test result: ok. 1 passed"`, `cargo fmt -p vibe-cli`, `cargo clippy -p vibe-cli --all-targets -- -D warnings`, and `wc -l crates/vibe-cli/src/command_progress.rs crates/vibe-cli/src/composition/operation.rs` (each 600 or less; command_progress.rs is 578 today and grows by 2).

**Decisions already made.**
- Top-level command `operations` with visible alias `ops`, subcommands `list` and `show <ID>`. Policy FastMetadata, like transfers.
- The settings root is record_home() (pre-promotion, pinned by begin), the same root OPS-4 writes under.
- Lost is computed at read time only for records whose machine equals this host name. Records from other machines are never probed (no single-host assumption).
- An unknown ID is a plain error (exit 1) with the text above. A typed code for it belongs to the stable-codes capability.
- list returns full records, newest first by started, ties by id descending.

**Not in this entry.** No filtering flags (--state, --since), no delete or clear verb, no tailing of event lines, and no rewriting of lost records on disk.

**Depends on.** ADV-CONTROL-OPS-4

**Spec edit (apply as a step of this entry).**

PROP-073: replace `<OPERATION-QUERY fact="true" status="spec/plan">` with `<OPERATION-QUERY fact="true" status="impl/done">`.

Entries that write the same file run one after another, never in parallel.
