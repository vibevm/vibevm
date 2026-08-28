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

use vibe_orchestrator::ports::RunObserver;

use super::{CliRunObserver, render_lifecycle};
use super::{StepStatus, execute, step_report};

/// The pre-wipe epoch's ONE snapshot: the plan the clean point runs, and the
/// backend its agent rows would be served by.
///
/// Both come from a SINGLE manifest read and the SINGLE tree built from it.
/// The plan itself carries no manifest — a complete `Manifest` holds `[llm]`
/// provider configuration, and the shared plan may not become a way to smuggle
/// it — so this surface derives its own backend here, beside the read, and
/// hands the plan down clean.
pub(crate) struct CleanEpoch {
    pub(crate) plan: vibe_orchestrator::RitualPlan,
    pub(crate) agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend>,
    /// The proven bundle this epoch was prepared over — the ONE canonical
    /// selected root (the same root the caller leased) together with the exact
    /// tree the plan was collected from. Carried as one closed value so the
    /// wipe plans from THEM rather than resolving the raw `--path` a second
    /// time or rediscovering the tree.
    pub(crate) selection: vibe_orchestrator::ProvenSelection,
}

/// Prepare that epoch. One `resolve_project_root`, one `Manifest::read`, one
/// `Workspace` built from that read — no rediscovery, and nothing re-read.
///
/// A composed clean calls this BEFORE the wipe and never reuses the command's
/// own post-wipe snapshot for it: the two describe different trees, and the
/// clean point belongs to the first.
///
/// ## Why the snapshot is CONSUMED rather than borrowed for the tree
///
/// Before the extraction this path was `Workspace::discover`, which read the
/// selected manifest itself and, on a malformed one, returned the underlying
/// `vibe_core::Error` — the TOML line and column an operator needs — under the
/// collection context below. Borrowing the snapshot and asking it to rediscover
/// cannot reproduce that: with nothing parsed there is no manifest to lend, and
/// the only thing such a helper can say is that the manifest did not parse,
/// which throws the cause away.
///
/// So the ONE read is consumed here: [`SelectedManifest::into_manifest`] hands
/// back the stored `Result` with its original error object, and the tree is
/// built from the parsed value. Both failure arms wear the same outer context
/// the old discovery did, so the external chain is unchanged and the cause
/// survives. The backend is derived from the borrow first, before the consume —
/// still one read, and still nothing re-read.
pub(crate) fn prepare_epoch(
    selected: &Path,
    lease: &std::sync::Arc<vibe_lifecycle::LifecycleLease>,
) -> Result<CleanEpoch> {
    // The root arrives ALREADY canonical and ALREADY leased. This epoch does
    // not resolve a path: a second resolution of the raw `--path` is a second
    // answer to "which tree is this", and the whole point of carrying the root
    // is that the tree this command wipes is the tree it leased.
    let selection = crate::commands::install::SelectedManifest::read(selected).prepare();
    // Built from the bundle by BORROW, lazily: no credential, endpoint or
    // provider is touched until an actual agent execution runs. The root is the
    // one the lease already pinned — no locator call, here or anywhere below.
    let agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend> = std::sync::Arc::new(
        super::install_agent_backend_from(lease.root(), selection.parsed_ref()),
    );
    // …and then PROVEN in the workspace-load shape, so a malformed manifest
    // still renders the exact chain `Workspace::discover` used to produce: the
    // collection context, the `manifest at …is invalid` line, and under it the
    // operator's own TOML line and column.
    let proven = selection
        .prove_as_workspace_load()
        .context("discovering the workspace for lifecycle contribution collection")?;
    let plan = vibe_orchestrator::plan_clean_prepared(proven.root(), proven.workspace())?;
    // The epoch's own root gate: the plan's workspace root is what every
    // untracked contribution and the wipe below will act on, and it must be the
    // root this command leased. The untracked dispatcher rechecks this
    // independently — two gates, because the plan and the dispatch are two
    // moments and a tree can be retargeted between them.
    lease.ensure_root(plan.workspace_root(), "at the clean epoch")?;
    Ok(CleanEpoch {
        plan,
        agent,
        selection: proven,
    })
}

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
    // The ONE canonical resolution of this command's selected root, and the
    // lease over it. Everything below — the epoch, its tree, its plan and the
    // wipe — is carried from these two values; nothing re-reads `args.path`.
    let selected = crate::commands::install::resolve_project_root(&args.path)?;
    let lease = crate::commands::install::acquire_lease(&selected)?;
    let CleanEpoch {
        plan,
        agent,
        selection,
    } = prepare_epoch(&selected, &lease)?;
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
        run_id: new_run_id(Path::new(&plan.project().root))?,
        started: crate::commands::init::current_timestamp_utc(),
        // The clean epoch is untracked: it persists no lifecycle state, so
        // it persists no node ownership either — `"."` is the honest
        // placeholder, and the field never reaches this handler's envelope.
        selected: ".".to_string(),
    };
    let notices = plan.notices().to_vec();
    let observer = CliRunObserver::new(ctx);
    observer.observe_plan(&plan, &metadata, true)?;
    let wipe_plan =
        crate::commands::clean::plan_wipe_prepared(selection.root(), selection.workspace())?;
    crate::commands::clean::confirm_wipe(ctx, &wipe_plan, metadata.assume_yes)?;
    let contributions =
        vibe_orchestrator::dispatch_plan_untracked(&observer, &plan, &lease, &agent, metadata)?;
    let wipe_ctx = if ctx.is_json() || ctx.is_quiet() {
        ctx.quiet_child()
    } else {
        ctx.clone()
    };
    crate::commands::clean::apply_wipe(&wipe_ctx, wipe_plan)?;
    // Clean-only never creates a trace session — it compiles nothing, and its
    // wipe would destroy the very directory a session lives in — so it renders
    // its draft directly, with no member and no suffix.
    let values = vibe_orchestrator::values::LifecycleValues::completed(
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
    render_lifecycle(values, ctx, None, "")
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
    plan: &vibe_orchestrator::RitualPlan,
) -> Result<()> {
    if ctx.agent_mode() != RunAgentMode::Agent {
        return Ok(());
    }
    let hosted: Vec<String> = plan
        .executions()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lease_for(root: &Path) -> std::sync::Arc<vibe_lifecycle::LifecycleLease> {
        std::sync::Arc::new(
            vibe_lifecycle::LifecycleLease::acquire(root).expect("the fixture root is leasable"),
        )
    }

    fn project(body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temp project");
        std::fs::write(dir.path().join("vibe.toml"), body).expect("the fixture is written");
        dir
    }

    const MINIMAL: &str = "[project]
name = \"demo\"
version = \"0.1.0\"
";

    /// The epoch is prepared over the root the caller ALREADY leased, and it
    /// proves the plan agrees with that lease.
    ///
    /// The mutation this kills is dropping the epoch's `ensure_root`: a clean
    /// would then plan — and wipe — a tree it never leased. Note that
    /// `prepare_epoch` cannot even be asked to resolve a raw path any more; the
    /// root is an argument, which is the construction half of the same law.
    #[test]
    fn a_foreign_lease_refuses_the_clean_epoch() {
        let here = project(MINIMAL);
        let selected = crate::commands::install::resolve_project_root(here.path()).unwrap();

        let elsewhere = project(MINIMAL);
        let foreign_root =
            crate::commands::install::resolve_project_root(elsewhere.path()).unwrap();
        let foreign = lease_for(&foreign_root);

        let Err(error) = prepare_epoch(&selected, &foreign) else {
            panic!("a foreign lease can never prepare this tree's clean point");
        };
        assert!(
            error
                .downcast_ref::<vibe_lifecycle::LifecycleLeaseError>()
                .is_some(),
            "the refusal is the lease's typed error: {error:#}",
        );
        assert!(
            format!("{error:#}").contains("at the clean epoch"),
            "and it names the boundary it fired at: {error:#}",
        );
    }

    /// The epoch carries its canonical root and its exact tree, and the wipe is
    /// planned from THOSE — never from a raw path a second time.
    ///
    /// `commands::clean::plan_wipe_prepared` is the only wipe planner that
    /// exists: the raw-path `plan_wipe` was deleted, so a second discovery is
    /// impossible by construction rather than by review. This pins the carried
    /// values agreeing with the lease.
    #[test]
    fn the_epoch_carries_the_leased_root_and_tree_to_the_wipe() {
        let here = project(MINIMAL);
        let selected = crate::commands::install::resolve_project_root(here.path()).unwrap();
        let lease = lease_for(&selected);

        let epoch = prepare_epoch(&selected, &lease).expect("its own lease prepares it");
        assert_eq!(
            epoch.selection.root(),
            selected,
            "the carried root is the leased one"
        );
        assert_eq!(
            epoch.selection.workspace().root,
            *lease.root(),
            "and the carried tree is rooted there too",
        );
        assert_eq!(
            epoch.plan.workspace_root(),
            lease.root(),
            "as is the plan the untracked dispatcher will recheck",
        );

        // The wipe plans from the carried pair, with no path input at all —
        // `plan_wipe_prepared` is the only planner that exists.
        crate::commands::clean::plan_wipe_prepared(
            epoch.selection.root(),
            epoch.selection.workspace(),
        )
        .expect("the wipe plans from the epoch's own values");
        assert_eq!(
            epoch.selection.root(),
            selected,
            "and it planned over the leased root, not a re-resolved path",
        );
    }

    /// A malformed pre-clean `vibe.toml` still fails with the ORIGINAL parse
    /// error under the collection context — not a summary of it.
    ///
    /// Before the extraction this path was `Workspace::discover`, which read
    /// the manifest itself and surfaced the underlying TOML complaint. The v2
    /// adapter borrowed the snapshot and asked it to rediscover, which can only
    /// report "the selected manifest did not parse" — the operator loses the
    /// line, the key and the reason. This is the mutation that kills: swap the
    /// consume back for a borrow-and-rediscover and the cause assertion fails
    /// while the context assertion still passes.
    #[test]
    fn a_malformed_pre_clean_manifest_keeps_its_parse_cause() {
        let project = tempfile::tempdir().expect("a temp project");
        std::fs::write(
            project.path().join("vibe.toml"),
            "[project]\nname = \"demo\"\nversion = \n",
        )
        .expect("the fixture is written");

        // `CleanEpoch` holds a trait object, so the error arm is taken by
        // pattern rather than through `expect_err`.
        let selected =
            crate::commands::install::resolve_project_root(project.path()).expect("canonical");
        let lease = lease_for(&selected);
        let Err(error) = prepare_epoch(&selected, &lease) else {
            panic!("a malformed manifest refuses");
        };
        let rendered = format!("{error:#}");

        assert!(
            rendered.contains("discovering the workspace for lifecycle contribution collection"),
            "the historical outer context survives: {rendered}",
        );
        // Recorded from the live run, so a future reader sees what the
        // operator actually gets:
        //   discovering the workspace for lifecycle contribution collection:
        //   failed to parse `…/vibe.toml`: string values must be quoted,
        //   expected literal string at line 3, column 11 (violates
        //   spec://…#manifest-schema; fix: repair the TOML at the location
        //   reported above)

        assert!(
            !rendered.contains("no workspace can be built from it"),
            "and it is NOT the generic did-not-parse summary: {rendered}",
        );
        // The cause the operator acts on: the real parse complaint, naming the
        // file it came from.
        assert!(
            rendered.contains("vibe.toml"),
            "the failing manifest is named: {rendered}",
        );
        assert!(
            rendered.to_ascii_lowercase().contains("parse")
                || rendered.to_ascii_lowercase().contains("expected")
                || rendered.to_ascii_lowercase().contains("toml"),
            "and the underlying parse detail is retained: {rendered}",
        );
    }

    /// The malformed-manifest chain is BYTE-IDENTICAL to the discovery it
    /// replaced.
    ///
    /// The two assertions above pin the outer context and the surviving cause
    /// separately, and a chain can satisfy both while still having lost the
    /// `WorkspaceError::Manifest` link in the middle — which is what carries the
    /// FILE PATH. So this compares the whole rendering against the baseline the
    /// old code produced: `Workspace::discover(root)` under the same context.
    ///
    /// The mutation this kills is proving through the plain `prove()` instead of
    /// `prove_as_workspace_load()`: the raw `vibe_core::Error` renders without
    /// the `manifest at ... is invalid` link, and this comparison fails while
    /// both looser assertions still pass.
    #[test]
    fn the_pre_clean_chain_matches_the_discovery_it_replaced() {
        let project = tempfile::tempdir().expect("a temp project");
        std::fs::write(
            project.path().join("vibe.toml"),
            "[project]
name = \"demo\"
version =
",
        )
        .expect("the fixture is written");
        let selected =
            crate::commands::install::resolve_project_root(project.path()).expect("canonical");

        // The BASELINE: exactly what this path did before the extraction.
        let baseline = format!(
            "{:#}",
            vibe_workspace::Workspace::discover(&selected)
                .err()
                .map(anyhow::Error::new)
                .expect("the malformed manifest refuses discovery too")
                .context("discovering the workspace for lifecycle contribution collection"),
        );

        let lease = lease_for(&selected);
        let Err(error) = prepare_epoch(&selected, &lease) else {
            panic!("a malformed manifest refuses");
        };
        assert_eq!(
            format!("{error:#}"),
            baseline,
            "the operator's whole chain is unchanged — context, workspace link and cause",
        );
    }

    /// The epoch performs exactly ONE manifest read: the snapshot is taken, the
    /// file is then DESTROYED, and the answer does not change.
    ///
    /// The earlier form of this red could only observe that the epoch did not
    /// itself rewrite the file — `resolve_project_root` proves `vibe.toml`
    /// exists before the read, so a deletion could not be staged in front of it.
    /// This drives the helper that ACCEPTS the already-taken snapshot instead,
    /// which is the same code path the epoch runs, and corrupts the disk between
    /// the read and every later observation.
    #[test]
    fn the_prepared_bundle_never_rereads_the_manifest_it_was_built_from() {
        let project = tempfile::tempdir().expect("a temp project");
        let manifest = project.path().join("vibe.toml");
        std::fs::write(
            &manifest,
            "[project]
name = \"demo\"
version = \"0.1.0\"
",
        )
        .expect("the fixture is written");
        let selected =
            crate::commands::install::resolve_project_root(project.path()).expect("canonical");

        // The ONE read, and the tree built from it.
        let selection = crate::commands::install::SelectedManifest::read(&selected).prepare();

        // Now destroy the file entirely. A second read would fail outright.
        std::fs::remove_file(&manifest).expect("the snapshot's file is removed");

        assert_eq!(
            selection
                .parsed_ref()
                .and_then(|m| m.project.as_ref())
                .map(|p| p.name.as_str()),
            Some("demo"),
            "the bundle still answers from its own snapshot",
        );
        let proven = selection
            .prove_as_workspace_load()
            .expect("and it proves without touching the disk again");
        assert_eq!(proven.root(), selected);
        assert_eq!(proven.workspace().root, selected);

        // And the plan the epoch builds from that pair needs no file either.
        let plan = vibe_orchestrator::plan_clean_prepared(proven.root(), proven.workspace())
            .expect("the clean point plans from the carried tree");
        assert_eq!(plan.workspace_root(), selected);
        assert!(
            !manifest.exists(),
            "nothing along the way recreated or re-read the manifest",
        );
    }
}
