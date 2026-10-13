//! Fixed explicit invocation composition through the incumbent command owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ADMITTED-INVOCATION-ORDER");
use super::*;
use vibe_lifecycle::InvocationIntent;

/// Prepare an explicit causal production/build/test/verify pass. This grants
/// invocation intent, not provider or filesystem authority, and stops on the
/// existing failure/park boundaries. Ordinary preparation is unchanged.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#INVOCATION-ROLE-LAW")]
pub fn prepare_full_pass_lifecycle(
    leased: LeasedDefaultLifecycle,
    request: DefaultLifecycleRequest,
) -> Result<PreparedDefaultLifecycle> {
    ensure!(
        request.requested == Phase::Verify,
        "explicit full-pass must end at verify"
    );
    prepare(leased, request, Some(InvocationIntent::FullPass))
}

pub(super) fn prepare(
    leased: LeasedDefaultLifecycle,
    request: DefaultLifecycleRequest,
    explicit: Option<InvocationIntent>,
) -> Result<PreparedDefaultLifecycle> {
    let intent = explicit.unwrap_or_else(|| InvocationIntent::for_phase(request.requested));
    let requested_name = if intent == InvocationIntent::FullPass {
        "full-pass".into()
    } else {
        request.requested.to_string()
    };
    let LeasedDefaultLifecycle {
        selected_root,
        lease,
    } = leased;
    // The chain is DERIVED, never supplied: the canonical inclusive prefix
    // through `requested`, both as phases (for dispatch) and as the string
    // chain (for identity/state/reporting). A caller cannot forge either.
    let phases = vibe_lifecycle::invocation_chain(request.requested, intent);
    let chain: Vec<String> = phases.iter().map(Phase::to_string).collect();
    // The ONE selected-manifest snapshot, at the root the lease already
    // pinned — never at a raw `--path` again.
    let selection = SelectedManifest::read(&selected_root).prepare();
    // The effective trace request: the raw flag OR the snapshot's own
    // `[compile] trace` vote — combined once, here.
    let trace_request = selection.request(request.trace_flag);
    // The ONE identity selection, against the same lease.
    let prelude = run_prelude(
        selection,
        lease,
        &requested_name,
        &chain,
        request.agent_mode.clone(),
        request.force,
        trace_request,
    )?;
    let mut prepared = assemble_prepared(request, phases, chain, prelude);
    prepared.metadata.requested = requested_name;
    Ok(prepared)
}
