//! Thin CLI projection over the `vibe-scrape` planning kernel (PROP-056).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#root");

use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::cli::{
    ScrapeArgs, ScrapeCommand, ScrapeContractCheckArgs, ScrapeContractCommand,
    ScrapeContractInitArgs,
};
use crate::output::Context;
use vibe_scrape::transaction::{
    Engine, NoFaults, PreparedHealthVerifier, RecoveryHealthVerifier, SafefsTransactionFilesystem,
    SystemTransactionStore, TransactionState, TransactionStore, prepared_transaction,
    project_identity_token, project_key, report_to_wire_plan,
};
use vibe_scrape::{ScrapeMode, ScrapeRequest};

pub fn run(ctx: &Context, args: ScrapeArgs) -> Result<()> {
    if let Some(command) = args.command {
        return match command {
            ScrapeCommand::Contract(contract) => match contract.command {
                ScrapeContractCommand::Init(args) => init(ctx, args),
                ScrapeContractCommand::Check(args) => check(ctx, args),
            },
        };
    }

    if args.recover {
        let root = absolute_existing_root(args.path.as_deref().ok_or_else(|| {
            anyhow::anyhow!("`vibe scrape --recover` requires an explicit `--path <project>`")
        })?)?;
        return recover(ctx, &root);
    }

    if !args.plan {
        if args.in_place
            && !args.assume_yes
            && (ctx.is_unattended() || ctx.is_json() || !std::io::stdin().is_terminal())
        {
            bail!(
                "scrape execution requires the explicit scrape-local `--assume-yes` when unattended/noninteractive"
            );
        }
        if args.output.is_none() && !args.in_place {
            bail!("choose `--output` or `--in-place` for scrape execution");
        }
    }

    let root = absolute_existing_root(args.path.as_deref().unwrap_or_else(|| Path::new(".")))?;
    let mode = match args.output {
        Some(output) => ScrapeMode::Export {
            output: absolute_output(output)?,
        },
        None => ScrapeMode::InPlace,
    };
    let request = ScrapeRequest {
        root,
        contract: args.contract,
        mode,
    };
    if args.plan {
        let prepared = vibe_scrape::prepare(request)?;
        return render_plan(ctx, prepared.plan.to_wire()?);
    }
    execute(ctx, request, args.assume_yes)
}

fn execute(ctx: &Context, request: ScrapeRequest, assume_yes: bool) -> Result<()> {
    ensure_execution_platform()?;
    let identity = project_identity_token(&request.root)?;
    let key = project_key(&identity);
    let root_display = request.root.display().to_string();
    let mut store = SystemTransactionStore::new(scrape_state_root()?)?;
    store.prove_outside_project(&root_display)?;
    let _lock = store.lock_project(&key)?;
    if store.pending(&key)?.is_some() {
        bail!("a pending scrape transaction must be recovered before preparing a new contract");
    }
    let prepared = vibe_scrape::prepare(request)?;
    let plan = prepared.plan.to_wire()?;
    if !plan.blockers.is_empty() {
        return render_plan(ctx, plan);
    }
    if !assume_yes && !ctx.is_json() && matches!(prepared.mode, ScrapeMode::InPlace) {
        print_full_plan(&plan)?;
        confirm(&plan)?;
    }
    let transaction = prepared_transaction(prepared.clone())?;
    let mut filesystem = SafefsTransactionFilesystem::for_prepared(&transaction)?;
    let mut verifier = PreparedHealthVerifier::new(prepared.health.clone());
    match Engine::new(&mut store, &mut filesystem, &mut verifier, &mut NoFaults)
        .execute_under_held_gate(key.clone(), &identity, &root_display, transaction)
    {
        Ok(report) => render_report(ctx, report_to_wire_plan(&report, &plan)?),
        Err(error) => render_durable_rollback_failure(ctx, &mut store, &key, &plan, error),
    }
}

fn recover(ctx: &Context, root: &Path) -> Result<()> {
    ensure_execution_platform()?;
    let identity = project_identity_token(root)?;
    let key = project_key(&identity);
    let root_display = root.display().to_string();
    let mut store = SystemTransactionStore::new(scrape_state_root()?)?;
    store.prove_outside_project(&root_display)?;
    let _lock = store.lock_project(&key)?;
    let journal = store
        .pending(&key)?
        .ok_or_else(|| anyhow::anyhow!("no pending scrape transaction for `{root_display}`"))?;
    let plan: vibe_wire::generated::scrape::e1::plan::Plan =
        serde_json::from_slice(&journal.canonical_plan)
            .context("decoding journaled scrape plan")?;
    let mut verifier = if journal.state == TransactionState::Preparing {
        RecoveryHealthVerifier::unavailable(
            "preparation stopped before the verifier snapshot set was complete",
        )
    } else {
        let health = store.read_snapshot(&journal, "health-plan")?;
        RecoveryHealthVerifier::available(PreparedHealthVerifier::from_journal_snapshots(
            &health,
            &journal,
            |name| store.read_snapshot(&journal, name),
        )?)
    };
    let mut filesystem = SafefsTransactionFilesystem::open(root, &identity)?;
    match Engine::new(&mut store, &mut filesystem, &mut verifier, &mut NoFaults)
        .recover_under_held_gate(key.clone(), &identity, &root_display, journal)
    {
        Ok(report) => render_report(ctx, report_to_wire_plan(&report, &plan)?),
        Err(error) => render_durable_rollback_failure(ctx, &mut store, &key, &plan, error),
    }
}

fn render_durable_rollback_failure(
    ctx: &Context,
    store: &mut SystemTransactionStore,
    key: &vibe_scrape::transaction::ProjectKey,
    plan: &vibe_wire::generated::scrape::e1::plan::Plan,
    error: vibe_scrape::transaction::TransactionError,
) -> Result<()> {
    if let Some(journal) = store.pending(key)?
        && journal.state == TransactionState::RollbackFailed
        && let Some(report) = journal.report.as_ref()
    {
        return render_report(ctx, report_to_wire_plan(report, plan)?);
    }
    Err(error.into())
}

fn render_report(
    ctx: &Context,
    report: vibe_wire::generated::scrape::e1::report::Report,
) -> Result<()> {
    use vibe_wire::generated::scrape::e1::report::{ReportCleanup, ReportOutcome};

    let successful = matches!(&report.outcome, ReportOutcome::Verified)
        && matches!(&report.cleanup, ReportCleanup::Complete);
    let recovery_command = matches!(&report.cleanup, ReportCleanup::Pending).then(|| {
        format!(
            "vibe scrape --recover --path {}",
            report.project_display_root
        )
    });
    if ctx.is_json() {
        // The transaction store persists this exact generated projection in
        // compact canonical field order. Scrape JSON deliberately bypasses
        // generic context stamping/pretty-printing so stdout and the stable
        // external report are byte-identical (apart from stdout's newline).
        println!("{}", serde_json::to_string(&report)?);
    } else {
        let headline = format!(
            "scrape {:?} / {:?} / {:?} ({})",
            report.outcome, report.assurance, report.cleanup, report.transaction_id
        );
        if ctx.is_quiet() {
            if let Some(command) = &recovery_command {
                ctx.summary(&format!("{headline}; recover with: {command}"));
            } else {
                ctx.summary(&headline);
            }
        } else {
            ctx.heading("Scrape report");
            ctx.summary(&headline);
            if let Some(command) = &recovery_command {
                println!("  recover   {command}");
            }
        }
    }
    if successful {
        Ok(())
    } else {
        bail!(
            "scrape did not complete successfully: outcome={:?}, cleanup={:?}",
            report.outcome,
            report.cleanup
        )
    }
}

fn confirm(plan: &vibe_wire::generated::scrape::e1::plan::Plan) -> Result<()> {
    println!(
        "Scrape will rewrite {}, relocate {}, delete-unmodified {}, delete-modified {}, delete-unknown {}, and delete-last {}. Continue? [y/N]",
        plan.summary.rewrite,
        plan.summary.relocate,
        plan.summary.delete_unmodified,
        plan.summary.delete_modified,
        plan.summary.delete_unknown,
        plan.summary.delete_last,
    );
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    if matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
        Ok(())
    } else {
        bail!("scrape cancelled before transaction creation")
    }
}

fn scrape_state_root() -> Result<PathBuf> {
    if let Some(settings) = std::env::var_os("VIBE_SETTINGS") {
        let settings = PathBuf::from(settings);
        let base = if settings.is_file() {
            settings.parent().unwrap_or(&settings).to_path_buf()
        } else {
            settings
        };
        return Ok(base.join("scrape-state"));
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .context("resolving user home for scrape transaction state")?;
    Ok(home.join(".vibe").join("scrape"))
}

fn ensure_execution_platform() -> Result<()> {
    if cfg!(windows) {
        Ok(())
    } else {
        bail!(
            "scrape-platform-unsupported: scrape mutation and recovery require Windows in epoch 1"
        )
    }
}

fn init(ctx: &Context, args: ScrapeContractInitArgs) -> Result<()> {
    let root = absolute_existing_root(&args.path)?;
    let path = vibe_scrape::init_contract(&root)?;
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "command": "scrape-contract-init",
            "created": path.display().to_string(),
        }))?;
    } else if ctx.is_quiet() {
        ctx.summary(&format!("scrape contract created: {}", path.display()));
    } else {
        ctx.heading("Scrape contract");
        ctx.created(&path.display().to_string());
        ctx.summary("Review the contract, then run `vibe scrape contract check`.");
    }
    Ok(())
}

fn check(ctx: &Context, args: ScrapeContractCheckArgs) -> Result<()> {
    let root = absolute_existing_root(&args.path)?;
    let prepared = vibe_scrape::check_contract(ScrapeRequest {
        root,
        contract: args.contract,
        mode: ScrapeMode::InPlace,
    })?;
    render_plan(ctx, prepared.plan.to_wire()?)
}

fn absolute_existing_root(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("canonicalizing project root `{}`", path.display()))
}

fn absolute_output(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()
            .context("reading current directory for scrape output")?
            .join(path))
    }
}

fn render_plan(ctx: &Context, plan: vibe_wire::generated::scrape::e1::plan::Plan) -> Result<()> {
    let blocker_count = plan.blockers.len();
    if ctx.is_json() {
        ctx.emit_json(&plan)?;
        return finish_render(blocker_count);
    }

    let summary = &plan.summary;
    let counts = format!(
        "keep {}, rewrite {}, relocate {}, delete {} ({} modified, {} unknown), delete-last {}, blockers {}",
        summary.keep,
        summary.rewrite,
        summary.relocate,
        summary.delete_unmodified + summary.delete_modified + summary.delete_unknown,
        summary.delete_modified,
        summary.delete_unknown,
        summary.delete_last,
        plan.blockers.len(),
    );
    if ctx.is_quiet() {
        ctx.summary(&format!("scrape plan {}: {counts}", plan.plan_id));
        return finish_render(blocker_count);
    }

    ctx.heading("Scrape plan");
    print_full_plan(&plan)?;
    finish_render(blocker_count)
}

fn print_full_plan(plan: &vibe_wire::generated::scrape::e1::plan::Plan) -> Result<()> {
    let mode = serde_json::to_value(&plan.mode)?
        .as_str()
        .context("generated scrape plan mode is not a JSON string")?
        .to_owned();
    let summary = &plan.summary;
    println!("  plan      {}", plan.plan_id);
    println!("  project   {}", plan.project.display_root);
    println!("  contract  {}", plan.contract.display_path);
    println!("  mode      {mode}");
    println!(
        "  counts    keep {}, rewrite {}, relocate {}, delete-unmodified {}, delete-modified {}, delete-unknown {}, delete-last {}, blockers {}",
        summary.keep,
        summary.rewrite,
        summary.relocate,
        summary.delete_unmodified,
        summary.delete_modified,
        summary.delete_unknown,
        summary.delete_last,
        plan.blockers.len(),
    );
    println!(
        "  contract-boundary {}",
        serde_json::to_string(&plan.contract_boundary)?
    );
    for item in &plan.items {
        println!("  item      {}", serde_json::to_string(item)?);
    }
    for rewrite in &plan.rewrites {
        println!("  rewrite   {}", serde_json::to_string(rewrite)?);
    }
    for relocation in &plan.relocations {
        println!("  relocate  {}", serde_json::to_string(relocation)?);
    }
    for lock in &plan.native_lock_changes {
        println!("  lock      {}", serde_json::to_string(lock)?);
    }
    for check in &plan.healthchecks {
        println!("  health    {}", serde_json::to_string(check)?);
    }
    for assertion in &plan.assertions {
        println!("  assert    {}", serde_json::to_string(assertion)?);
    }
    for blocker in &plan.blockers {
        let path = blocker
            .path
            .as_deref()
            .map(|path| format!(" [{path}]"))
            .unwrap_or_default();
        println!("  blocker   {}{path}: {}", blocker.code, blocker.message);
    }
    Ok(())
}

fn finish_render(blocker_count: usize) -> Result<()> {
    if blocker_count == 0 {
        Ok(())
    } else {
        bail!("scrape plan is blocked by {blocker_count} finding(s)")
    }
}
