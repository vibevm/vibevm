//! This invocation's durable run identity, chosen once, before anything is
//! allocated.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use anyhow::{Context, Result};
use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use crate::install::PreparedSelection;

/// This invocation's durable run identity, plus the ONE root its trace may be
/// stored under.
///
/// The two answers come from the same discovery and must not be confused:
///
/// * **identity/state** may fall back to the selected project root. That
///   fallback is old, load-bearing and compatible — a project outside any
///   discoverable workspace still gets a run id and a `.vibe/lifecycle.toml`.
/// * **trace storage** may NOT. A trace's lock and index belong to the
///   canonical workspace root, because one install regenerates shared package
///   units plus every node — so an invocation entered through a member that
///   silently traced into the member's own directory would let two members
///   hold independent locks over the same work. When discovery genuinely
///   fails there is no canonical root to name, so no trace opens at all and
///   the command's own validation error stays authoritative.
///
/// Hence the epoch carries the typed [`PreparedSelection`] rather than an
/// `Option<Workspace>`: a loaded tree alone names a trace home, and the
/// unavailable states say WHICH way the one attempt failed so that nothing
/// downstream retries it. The whole bundle is retained, not just its root — the
/// command is about to validate against it, install through it and lock a trace
/// to it, and re-reading for each of those would be three snapshots of a tree
/// the command is itself changing.
///
/// The canonical selected root rides INSIDE the bundle rather than beside it:
/// it is selected once per prelude epoch, nothing downstream re-resolves it,
/// and a root passed separately could name a node the carried tree was never
/// built against.
///
/// ```no_run
/// use vibe_orchestrator::RunPrelude;
/// fn trace_home(prelude: &RunPrelude) -> Option<&std::path::Path> {
///     prelude.selection.loaded_root()
/// }
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct RunPrelude {
    /// The durable identity this invocation adopted or allocated.
    pub identity: vibe_lifecycle::RunIdentity,
    /// The ONE selected-world provenance bundle: the canonical root, the
    /// manifest snapshot taken at it, and the tree built from that snapshot.
    pub selection: PreparedSelection,
    /// The workspace mutation lease this command holds — see [`run_prelude`],
    /// which both receives and returns it.
    pub lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    /// The canonical workspace-relative identity of the selected node this
    /// command runs from — derived ONCE here from the prepared workspace
    /// (never re-derived, not even across the post-clean reload, which instead
    /// PROVES it still holds). It is selector input (which park this
    /// invocation may adopt) and the value the metadata carries into the state
    /// header, deliberately NOT a member of `RunIdentity`: the selector
    /// decides identity, it does not echo its inputs.
    pub selected: String,
}

/// Choose this invocation's durable run identity through the one selector,
/// before anything is allocated.
///
/// It RESOLVES and DISCOVERS nothing: the caller has already canonicalised the
/// project root once and already built (or failed to build) the workspace from
/// its own manifest snapshot. A second resolution here would be a second
/// answer to "which node is this", and a second discovery a second answer to
/// "what does its tree look like".
///
/// ```no_run
/// use vibe_orchestrator::{RunPrelude, run_prelude};
/// # fn call(
/// #     selection: vibe_orchestrator::PreparedSelection,
/// #     lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
/// #     mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode,
/// # ) -> anyhow::Result<()> {
/// let prelude: RunPrelude = run_prelude(selection, lease, "build", &[], mode, false, false)?;
/// assert!(!prelude.selected.is_empty());
/// # Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub fn run_prelude(
    selection: PreparedSelection,
    lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    requested: &str,
    chain: &[String],
    agent_mode: RunAgentMode,
    force: bool,
    compile_trace: bool,
) -> Result<RunPrelude> {
    // The post-acquisition root law: a workspace loaded under a DIFFERENT
    // root than the one this command leased would read and write state
    // beside another process's lock, on a pre-lease snapshot this lease
    // never authorised. The one gate — and the one refusal spelling — is
    // the lease's own `ensure_root`; the locator's discovery-failed fallback
    // is the one exception (the lease already pins the selected root under
    // the same fallback law identity selection has always applied, and the
    // execution boundary then surfaces the stored discovery error itself).
    if let Some(loaded) = selection.loaded_root() {
        lease.ensure_root(loaded, "at the run prelude")?;
    }
    // The selected-node identity, derived from the ONE prepared snapshot: a
    // `Loaded` tree maps the canonical selected root through the workspace's
    // own authored rels — and a Loaded tree that cannot map it is an
    // internal refusal, never a fallback guess. Every unavailable arm falls
    // back to `"."` under the same fallback law the state root itself
    // applies there: when discovery failed, the selected node IS the root.
    let selected = match selection.loaded_workspace() {
        Some(workspace) => workspace
            .node_rel_of(selection.root())
            .with_context(|| {
                format!(
                    "internal: the canonical selected root `{}` is not a node of the \
                     workspace loaded for this run",
                    selection.root().display()
                )
            })?
            .as_str()
            .to_string(),
        None => ".".to_string(),
    };
    // The identity borrows the lease BEFORE its state read, so the prior
    // state it decides adoption against is a post-acquisition snapshot.
    let identity = vibe_lifecycle::select_run_identity(
        &lease,
        selection.root(),
        requested,
        chain,
        &selected,
        agent_mode,
        force,
        compile_trace,
        vibe_core::timestamp::now_utc(),
    )?;
    Ok(RunPrelude {
        identity,
        selection,
        lease,
        selected,
    })
}
