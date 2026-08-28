//! Durable run-identity selection: the ONE place an invocation decides
//! whether it continues a parked run or mints a fresh identity, and —
//! since R3.4 — what compile-trace activation it carries and which
//! displaced traced park it supersedes (PROP-054 `##REF-AGENT-RESUME`,
//! `##OBS-TRACE`). Split from `store.rs` when the identity half
//! outgrew the 600-line budget; the store owns the FILE, this cell
//! owns the DECISION, and the seam is `store::read_prior`.

use std::path::Path;

use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;

use super::error::LifecycleStateError;
use super::io;
use crate::process::is_valid_run_id;

/// One invocation's durable identity, decided before anything is allocated.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub struct RunIdentity {
    /// The effective run id: adopted from the parking invocation, or freshly
    /// allocated. Everything downstream — scratch, outbox, state — sees this.
    pub run_id: String,
    /// The run's original start: preserved on adoption, the injected clock
    /// value on a fresh run.
    pub started: String,
    /// Whether the identity continues a parked run.
    pub adopted: bool,
    /// The EFFECTIVE compile-trace activation for this invocation (PROP-054
    /// `##OBS-TRACE`, R3.4): on adoption `current_request OR the parked run's
    /// persisted sticky bit` — so a resume keeps tracing even when the
    /// original one-shot flag is absent and the manifest changed meanwhile —
    /// and the bare current request on a fresh run.
    pub compile_trace: bool,
    /// The state-proven parked traced run this invocation DISPLACES (force,
    /// changed command/chain/mode): an ownership fact naming exactly which
    /// abandoned running trace the next command atom may finalise as
    /// superseded. `None` unless the prior state owned a valid delegated run
    /// whose sticky bit was true and this invocation did not adopt it.
    pub superseded_trace: Option<SupersededTrace>,
}

/// A displaced run's durable identity: WHICH parked traced run this
/// invocation superseded, and when that run began. A named record, not
/// an anonymous tuple, so the ownership fact reads at every call site.
/// Nothing in selection touches the filesystem, a trace directory or a
/// directory NAME — the fact is proven by the state alone.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub struct SupersededTrace {
    /// The displaced run's exact 32-hex run id.
    pub run_id: String,
    /// The displaced run's original injected start.
    pub started: String,
}

/// Decide this invocation's durable run identity in ONE place, before any
/// scratch directory is created, so adopting a parked run can never leak an
/// abandoned candidate directory (PROP-054 `##REF-AGENT-RESUME`).
///
/// The original parking invocation and its resume are the SAME run exactly
/// when: the resolved mode is agent, `--force` is absent, the prior run
/// requested the same phase, the prior COMPLETE persisted chain equals this
/// invocation's complete chain exactly, at least one prior execution is
/// delegated, and the persisted run id is valid. Every other case — different
/// command, different chain, CLI mode, no delegated row, `--force`, or a
/// missing/invalid identity — gets one fresh allocated id. Corrupt delegated
/// state is refused upstream of here (`read_prior`); it never silently mints a
/// new identity.
///
/// OWNERSHIP precedes all of that: `selected` is the canonical
/// workspace-relative identity of the node this invocation runs from (the
/// caller's ONE derivation, from the prepared workspace), and a live park
/// whose persisted `selected` differs belongs to a sibling node. That is the
/// typed [`LifecycleStateError::ForeignPark`] — fired immediately after
/// `parked` is computed, BEFORE adoption, displacement and allocation, and
/// regardless of mode, `--force`, chain or trace posture: force and
/// supersession are SAME-node rulings, so they are simply unreachable for a
/// foreign park. The comparison is raw-string equality of the two spellings;
/// the stored value never passes through `RelPath::new`, which would repair
/// an attacker-editable cache (`""` becoming `"."`) and could forge
/// ownership. A foreign park is also deliberately NOT superseded: this node
/// has no right to terminalise a sibling's running trace, so the refusal
/// returns no [`SupersededTrace`] and leaves the owner's trace index exactly
/// as found.
///
/// The comparison is exact — no phase is stripped from either side. A
/// clean-composed invocation therefore never adopts: `vibe clean create`
/// carries a leading `clean` its predecessor's persisted chain does not, so it
/// wipes and reparks honestly. Its own resume is `vibe create` (the handoff's
/// `resume` line), whose chain is the persisted one, and that adopts.
///
/// Trace activation is sticky beside the identity (PROP-054 `##OBS-TRACE`,
/// R3.4): an adopted run traces when the current request OR its own persisted
/// bit says so, a fresh run when the current request alone does. And when the
/// exact adoption conditions fail for a prior state that owns a valid parked
/// run with the sticky bit set, that run is DISPLACED: the returned
/// [`SupersededTrace`] names it (id + original start) so the command owner can
/// reopen and terminalise its running trace. Ownership is proven by the state
/// alone — never inferred from a 32-hex directory name — and a prior without
/// a delegated row, without a valid identity, or with the bit false claims
/// nothing. The prior state is therefore read on every path, not only the
/// adoption-eligible ones; a corrupt state still refuses upstream rather than
/// being minted around.
///
/// `lease` is the workspace's outermost mutation lease: the proof that this
/// invocation owns `.vibe/lifecycle.lock`, taken BEFORE this read, so the
/// prior state the selector decides against is a POST-acquisition snapshot —
/// never a pre-lease fact another process has since replaced. The read goes
/// through the lease's pinned capability at `lease.root()`, so there is no
/// second capability and no second root answer. `allocation_root` is where a
/// fresh scratch run directory is created (the selected project root) — it
/// differs from the lease root only in a multi-node workspace, and must be
/// an existing ABSOLUTE path: an unallocatable selected root refuses as
/// [`LifecycleStateError::Allocation`] naming that root.
/// Selection completes BEFORE allocation, so an adopted run never mints and
/// abandons a candidate scratch directory.
#[allow(clippy::too_many_arguments)]
// the selector's fact set is one flat signature — a struct here would hide which facts are selection inputs
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub fn select_run_identity(
    lease: &crate::lease::LifecycleLease,
    allocation_root: &Path,
    requested: &str,
    chain: &[String],
    selected: &str,
    agent_mode: RunAgentMode,
    force: bool,
    current_request: bool,
    fresh_started: String,
) -> Result<RunIdentity, LifecycleStateError> {
    let prior =
        io::read_prior(lease.project(), &io::state_path(lease.root()))?.map(|prior| prior.state);
    let parked = prior.as_ref().is_some_and(|state| {
        state
            .execution
            .values()
            .any(|record| record.status == ExecutionRecordStatus::Delegated)
    });
    // The ownership gate — see the doc above. It fires before the adoption
    // branch, before displacement and before `allocate_run_id`, so a foreign
    // refusal performs no state, scratch or outbox mutation at all. The
    // refusal carries EXACT strings: `read_prior` already validated this
    // state, and validation pins both identities for a parked run, so an
    // absent one is an internal invariant breach — reported as
    // [`LifecycleStateError::Invariant`], never guessed around and never
    // dressed as an ownership ruling.
    if parked
        && let Some(prior) = prior.as_ref()
        && prior.run.selected.as_deref() != Some(selected)
    {
        let (Some(stored), Some(parked_run_id)) = (&prior.run.selected, &prior.run.run_id) else {
            return Err(LifecycleStateError::Invariant {
                path: io::state_path(lease.root()),
                reason: format!(
                    "a parked run reached identity selection without its validated exact \
                     identities (selected = {:?}, run_id = {:?})",
                    prior.run.selected, prior.run.run_id,
                ),
            });
        };
        return Err(LifecycleStateError::ForeignPark {
            path: io::state_path(lease.root()),
            stored: stored.clone(),
            selected: selected.to_string(),
            run_id: parked_run_id.clone(),
        });
    }
    if agent_mode == RunAgentMode::Agent
        && !force
        && let Some(prior) = prior.as_ref()
        && parked
        && prior.run.requested == requested
        && prior.run.chain == chain
        && let Some(run_id) = prior.run.run_id.as_deref()
        && is_valid_run_id(run_id)
    {
        return Ok(RunIdentity {
            run_id: run_id.to_string(),
            started: prior.run.started.clone(),
            adopted: true,
            // Sticky: a parked traced run keeps tracing through its
            // resume even when this invocation asked once, long ago, or
            // never (the bit itself is how it became sticky).
            compile_trace: current_request || prior.run.compile_trace,
            superseded_trace: None,
        });
    }
    // Displacement: the prior state OWNS a parked traced run (a valid
    // identity, a delegated row, the sticky bit set) and this invocation
    // does not adopt it. The fact is exact — which run, when it started —
    // and is all this atom does with it; the command atom terminalises.
    let superseded_trace = prior.as_ref().and_then(|prior| {
        (parked && prior.run.compile_trace)
            .then(|| {
                prior
                    .run
                    .run_id
                    .as_deref()
                    .filter(|id| is_valid_run_id(id))
                    .map(|run_id| SupersededTrace {
                        run_id: run_id.to_string(),
                        started: prior.run.started.clone(),
                    })
            })
            .flatten()
    });
    Ok(RunIdentity {
        run_id: crate::process::allocate_run_id(allocation_root)
            .map_err(|source| allocate_failed(allocation_root, source))?,
        started: fresh_started,
        adopted: false,
        compile_trace: current_request,
        superseded_trace,
    })
}

/// A failed fresh-id allocation is an allocation problem at the SELECTED
/// root, named as such — never a state-publication problem and never a
/// reason to touch the state cache.
fn allocate_failed(root: &Path, source: crate::process::ScratchError) -> LifecycleStateError {
    LifecycleStateError::Allocation {
        path: root.to_path_buf(),
        reason: source.to_string(),
    }
}
