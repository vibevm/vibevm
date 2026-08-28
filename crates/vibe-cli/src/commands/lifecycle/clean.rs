//! The clean epochs of the lifecycle adapter: composed clean (`vibe clean
//! <phase>`, which wipes then continues through the ordinary step list) and
//! the independent clean (`vibe clean`, which wipes and stops). Split from
//! the adapter's main cell along that responsibility seam when the file
//! outgrew the 600-line budget; the tracked `execute` path stays above and
//! reaches back here only for the shared pre-wipe agent refusal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::path::Path;

use anyhow::{Context, Result};
use specmark::spec;
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{LifecycleRequest, Phase, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use crate::cli::{CleanArgs, CleanChain, InstallArgs};
use crate::output;

use super::LifecycleDraft;
use super::dispatch;
use super::plan::surface_plan;
use super::world;
use super::{StepStatus, execute, step_report};

/// Compose clean with any default-lifecycle phase through the same step list.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
pub fn run_clean(
    ctx: &output::Context,
    args: CleanArgs,
    prepare_install: impl FnOnce() -> Option<std::path::PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let CleanArgs {
        path,
        assume_yes,
        chain,
    } = args;
    let chain = chain.context("internal: chained clean lost its continuation")?;
    let (requested, mut install_args) = clean_continuation(chain);
    if path != Path::new(".") {
        install_args.path = path;
    }
    execute(
        ctx,
        LifecycleRequest::Clean {
            then: Some(requested),
        },
        requested,
        install_args,
        assume_yes,
        prepare_install,
        root_offline,
    )
}

/// Run the independent clean lifecycle: dispatch once, then terminal wipe.
pub(crate) fn run_clean_only(
    ctx: &output::Context,
    args: CleanArgs,
    root_offline: bool,
) -> Result<()> {
    let chain = vec!["clean".to_string()];
    // The OUTERMOST lock first: resolve the selected root, locate the lease
    // root read-only, acquire — and only then plan, refuse, mint and wipe. A
    // contended workspace refuses here, typed, before a run id exists and
    // before anything destructive is even planned. The owner is this local:
    // the clean epoch is untracked, so no store carries it; it releases at
    // the end of the command, after the draft renders.
    let lease = crate::commands::install::acquire_lease(
        &crate::commands::install::resolve_project_root(&args.path)?,
    )?;
    let plan = world::plan_clean(&args.path)?;
    // The refusal comes FIRST — before a run id is minted, before the plan is
    // narrated, before the wipe is confirmed. Allocating `.vibe/lifecycle/<id>`
    // is itself a mutation, and an invocation this build cannot host must
    // leave the tree byte-identical.
    refuse_untracked_agent_rows(ctx, &plan)?;
    let metadata = RunMetadata {
        requested: "clean".to_string(),
        chain: chain.clone(),
        offline: effective_clean_offline(root_offline)?,
        assume_yes: metadata_assume_yes(ctx, args.assume_yes),
        agent_mode: ctx.agent_mode(),
        force: false,
        trace_compile: false,
        run_id: new_run_id(Path::new(&plan.project.root))?,
        started: crate::commands::init::current_timestamp_utc(),
        // The clean epoch is untracked: it persists no lifecycle state, so
        // it persists no node ownership either — `"."` is the honest
        // placeholder, and the field never reaches this handler's envelope.
        selected: ".".to_string(),
    };
    let notices = plan.notices.clone();
    surface_plan(ctx, &plan, &metadata, true)?;
    let wipe_plan = crate::commands::clean::plan_wipe(&args.path)?;
    crate::commands::clean::confirm_wipe(ctx, &wipe_plan, metadata.assume_yes)?;
    let contributions = dispatch::dispatch_plan_untracked(ctx, &plan, &lease, metadata)?;
    let wipe_ctx = if ctx.is_json() || ctx.is_quiet() {
        ctx.quiet_child()
    } else {
        ctx.clone()
    };
    crate::commands::clean::apply_wipe(&wipe_ctx, wipe_plan)?;
    // Clean-only never creates a trace session — it compiles nothing, and its
    // wipe would destroy the very directory a session lives in — so it renders
    // its draft directly, with no member and no suffix.
    let draft = LifecycleDraft::completed(
        "clean",
        chain,
        vec![step_report("clean", StepStatus::Ok)],
        contributions,
        notices,
        // The clean epoch is untracked, so it can never park: an agent row
        // here is refused above, before anything is wiped.
        None,
    );
    ctx.flush_json_plans()?;
    draft.render(ctx, None, "")
}

/// The clean epoch runs UNTRACKED: it keeps no `.vibe/lifecycle.toml` record,
/// and its wipe destroys the very tree a parked task would have to live in.
/// There is therefore no honest place in R7.3 to park a pre-wipe `agent` row —
/// and paying the provider for one in resolved agent mode is exactly the
/// accident this refusal exists to prevent. So: refuse explicitly, before the
/// wipe confirmation, spending nothing and destroying nothing.
///
/// Remaining R7 debt, named rather than hidden: a safe pre-wipe park/resume
/// for `phase:clean` agent rows through a tracked seam (a clean-epoch state
/// home that survives its own wipe), tracked with R7.4's bounded outbox GC.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub(super) fn refuse_untracked_agent_rows(
    ctx: &output::Context,
    plan: &world::RitualPlan,
) -> Result<()> {
    if ctx.agent_mode() != RunAgentMode::Agent {
        return Ok(());
    }
    let hosted: Vec<String> = plan
        .executions
        .iter()
        .filter(|execution| execution.row.declaration().handler.kind() == "agent")
        .map(|execution| execution.row.key().to_string())
        .collect();
    if hosted.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "the clean lifecycle cannot host agent contribution(s) {hosted:?} under \
         `--agent-mode agent`: the clean epoch is untracked and its wipe would destroy the \
         outbox a parked task lives in, so this invocation neither paid a provider nor removed \
         anything (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
         fix: run the clean lifecycle with `--agent-mode cli`, or disable the named \
         `phase:clean` agent contribution(s) for hosted runs)"
    )
}

fn clean_continuation(chain: CleanChain) -> (Phase, InstallArgs) {
    match chain {
        CleanChain::Validate(args) => (Phase::Validate, args.install_args()),
        CleanChain::Install(args) => (Phase::Install, args),
        CleanChain::Generate(args) => (Phase::Generate, args.install_args()),
        CleanChain::Build(args) => (Phase::Build, args.install_args()),
        CleanChain::Test(args) => (Phase::Test, args.install_args()),
        CleanChain::Create(args) => (Phase::Create, args.install_args()),
        CleanChain::Verify(args) => (Phase::Verify, args.install_args()),
        CleanChain::Package(args) => (Phase::Package, args.install_args()),
        CleanChain::Deploy(args) => (Phase::Deploy, args.install_args()),
    }
}

fn new_run_id(project_root: &Path) -> Result<String> {
    vibe_lifecycle::process::allocate_run_id(project_root).map_err(Into::into)
}

fn effective_clean_offline(root_offline: bool) -> Result<bool> {
    let user = UserConfig::load().context("loading user config for clean lifecycle envelope")?;
    Ok(output::resolve_offline(root_offline, user.net.offline))
}

fn metadata_assume_yes(ctx: &output::Context, explicit: bool) -> bool {
    explicit || ctx.is_unattended() || ctx.is_json()
}
