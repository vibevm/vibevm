# ADV-CONTROL-OPS-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A lifecycle Busy refusal names the process holding the lease (pid, machine, started), read from .vibe/lifecycle.holder.json, which the holder writes on acquisition and removes on release.

**Write.** `crates/vibe-lifecycle/src/lease.rs`, `crates/vibe-lifecycle/src/lease/tests.rs`, `crates/vibe-lifecycle/src/lib.rs`, `crates/vibe-lifecycle/src/state/tests/support.rs`, `crates/vibe-lifecycle/src/state/tests/safety.rs`, `crates/vibe-lifecycle/Cargo.toml`, `Cargo.toml`, `Cargo.lock`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-lifecycle --lib a_busy_refusal_names_its_holder_and_release_removes_it 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-lifecycle/src/lease/tests.rs, new fn a_busy_refusal_names_its_holder_and_release_removes_it (runs 0 tests today).

**Read first.**
- crates/vibe-lifecycle/src/lease.rs: enum LifecycleLeaseError (variant `Busy { root: PathBuf }` and its #[error] text with `lock = LOCK_NAME`), struct LifecycleLease (fields `project: Project`, `_guard: vibe_safefs::LockGuard`), fn LifecycleLease::acquire
- crates/vibe-lifecycle/src/lease/tests.rs fn a_second_same_process_acquisition_is_a_typed_busy (matches `Busy { root }`)
- crates/vibe-lifecycle/src/state/tests/support.rs fn vibe_names (`.filter(|name| name != crate::lease::LOCK_NAME)`)
- crates/vibe-lifecycle/src/state/tests/safety.rs (the read_dir of .vibe with `.filter(|name| name != crate::lease::LOCK_NAME)`)
- crates/vibe-lifecycle/src/lib.rs line `pub use lease::{LifecycleLease, LifecycleLeaseError};`
- vibe_core::timestamp::now_utc (crates/vibe-core/src/timestamp.rs)

**Steps.**
1. Root Cargo.toml [workspace.dependencies]: add after the line `dirs = "5"`:
```
# The machine a lease holder or an operation record names
# (PROP-073 #LEASE-HOLDER). Apache-2.0.
gethostname = "1"
```
crates/vibe-lifecycle/Cargo.toml [dependencies]: add `gethostname.workspace = true` before `getrandom.workspace = true`. Version 1.1.0 is already in Cargo.lock, so building only adds the dependency edge to Cargo.lock.
2. crates/vibe-lifecycle/src/lease.rs: add `pub(crate) const HOLDER_NAME: &str = "lifecycle.holder.json";` beside LOCK_NAME. Add
```
/// The process that holds a workspace's lifecycle lease: what a Busy
/// refusal names (PROP-073 `#LEASE-HOLDER`).
///
/// ```
/// let me = vibe_lifecycle::LeaseHolder::current();
/// assert_eq!(me.pid, std::process::id());
/// assert!(!me.machine.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#LEASE-HOLDER")]
pub struct LeaseHolder {
    pub pid: u32,
    pub machine: String,
    pub started: String,
}
```
with `impl LeaseHolder { #[must_use] pub fn current() -> Self }` returning pid std::process::id(), machine `gethostname::gethostname().to_string_lossy().into_owned()` and started vibe_core::timestamp::now_utc().
3. Change the variant to `Busy { root: PathBuf, holder: Option<LeaseHolder> }`. In its #[error] text replace `(`.vibe/{lock}` is held);` with `(`.vibe/{lock}` is held{held});` and add the format arg `held = held_by(holder.as_ref())` next to `lock = LOCK_NAME` (thiserror 2.0.18 accepts a bare field name there; `.holder` also works). Add private `fn held_by(holder: Option<&LeaseHolder>) -> String` returning `format!(" by process {} on `{}` since {}", h.pid, h.machine, h.started)` for Some(h) and an empty String for None.
4. Holder I/O uses plain std::fs, not vibe_safefs: safefs fault-injection hooks are thread-local and armed by many tests, and a holder write must not consume them. Add private `fn record_holder(root: &Path)`: serialise LeaseHolder::current() with serde_json::to_vec, remove then create_new-open `.vibe/.lifecycle.holder.json.<pid>.tmp`, write the bytes, rename it over `.vibe/lifecycle.holder.json`, and on any failure remove the staged file; all errors are ignored. Add private `fn read_holder(root: &Path) -> Option<LeaseHolder>`, which opens `.vibe/lifecycle.holder.json`, reads at most 4096 bytes via Read::take, and parses with serde_json (any failure gives None). In acquire, the `Ok(Some(guard))` arm calls record_holder(project.root_path()) before building Self; the `Ok(None)` arm builds `Busy { root: root.to_path_buf(), holder: read_holder(project.root_path()) }`. Add `impl Drop for LifecycleLease { fn drop(&mut self) { let _ = std::fs::remove_file(self.project.root_path().join(".vibe").join(HOLDER_NAME)); } }`; it runs before the `_guard` field is dropped, so the file is gone before the lock is released.
5. crates/vibe-lifecycle/src/lib.rs: change the re-export to `pub use lease::{LeaseHolder, LifecycleLease, LifecycleLeaseError};`. In lease/tests.rs change the pattern `Err(LifecycleLeaseError::Busy { root }) =>` to `Err(LifecycleLeaseError::Busy { root, .. }) =>` (grep confirmed it is the only `Busy { root }` pattern in the workspace). In state/tests/support.rs fn vibe_names and in the state/tests/safety.rs read_dir filter, change `name != crate::lease::LOCK_NAME` to `name != crate::lease::LOCK_NAME && name != crate::lease::HOLDER_NAME`, so those state inventories keep ignoring lease infrastructure while a lease is held.
6. Add the test to lease/tests.rs. Acquire a lease on a tempdir and assert `.vibe/lifecycle.holder.json` is a file. A second acquire must match `Err(LifecycleLeaseError::Busy { root, holder: Some(holder) })` with root == dir.path(), holder.pid == std::process::id(), holder.machine == LeaseHolder::current().machine and a non-empty holder.started. A third acquire's to_string() contains `format!("by process {} on `{}`", std::process::id(), LeaseHolder::current().machine)` and still contains "before any run id". After drop(lease) the holder file does not exist. Flip the spec unit (spec_edit).
7. Self-verify: `cargo test -p vibe-lifecycle` (whole crate), `cargo test -p vibe-cli --test cli_lifecycle_lease --test cli_hook_lifecycle --test cli_project_cancel --test cli_agent_stop --test cli_host_sources --test cli_source_admission --test cli_declared_own_boot --test cli_common_package_optional`, `cargo test -p vibe-mcp --test lifecycle_run` and `cargo test -p vibe-orchestrator`. These suites inventory `.vibe` and skip only lifecycle.lock; they stay green because every run drops its lease (and the holder file) before the after-snapshot, and vibe-mcp's busy test takes both snapshots while the same lease is held. If any of them fails because it sees lifecycle.holder.json, stop and report it rather than editing those tests (they are outside write). Then `cargo fmt -p vibe-lifecycle` and `cargo clippy -p vibe-lifecycle --all-targets -- -D warnings`.

**Decisions already made.**
- The holder file is `.vibe/lifecycle.holder.json` with JSON {pid, machine, started}. It is not the lock file, because on Windows the locked lock file cannot be read by another process.
- machine is the host name from gethostname (lossy UTF-8), and LeaseHolder::current is the one place that computes it. Later CLI entries call vibe_lifecycle::LeaseHolder::current() and do not compute their own.
- Removal happens in Drop, so a completed or refused command leaves no new file. Only process death leaves a stale holder, and the next acquisition overwrites it. vibe_test_support::retained_lifecycle_lease keeps one lease in a static for the test process; its holder file stays in that leaked tempdir, which no inventory reads.
- A Busy holder read from a stale file is reported as written. This crate does no liveness check.
- Errors writing or removing the holder never change acquire's result.

**Not in this entry.** No JSON code or envelope stamping in vibe-cli (that is OPS-8), no liveness probing of the holder pid, no edits to other crates' tests, and no change to the RootMismatch, SelectedNodeMismatch or Directory variants.

**Depends on.** ADV-CONTROL-OPS-1

**Spec edit (apply as a step of this entry).**

PROP-073: replace `<LEASE-HOLDER fact="true" status="spec/plan">` with `<LEASE-HOLDER fact="true" status="impl/done">`.

Entries that write the same file run one after another, never in parallel.
