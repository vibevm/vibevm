//! Thin CLI projection over the `vibe-scrape` planning kernel (PROP-056).

use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::cli::{
    ScrapeArgs, ScrapeCommand, ScrapeContractCheckArgs, ScrapeContractCommand,
    ScrapeContractInitArgs,
};
use crate::output::Context;
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
        if args.path.is_none() {
            bail!("`vibe scrape --recover` requires an explicit `--path <project>`");
        }
        bail!(
            "`vibe scrape --recover` is not implemented yet; recovery lands with the external transaction in implementation E"
        );
    }

    if !args.plan {
        if args.in_place && !args.assume_yes && !std::io::stdin().is_terminal() {
            bail!(
                "in-place scrape requires the explicit scrape-local `--assume-yes`; `--unattended` does not authorize destructive confirmation"
            );
        }
        if args.output.is_some() {
            bail!(
                "`vibe scrape --output` execution is not implemented yet; use `vibe scrape --plan --output <dir>` until implementations C-D land"
            );
        }
        if args.in_place {
            bail!(
                "`vibe scrape --in-place` execution is not implemented yet; use `vibe scrape --plan --in-place` until implementations C-E land"
            );
        }
        bail!(
            "choose one scrape operation: `--plan`, `--output`, `--in-place`, `--recover`, or `contract init|check`"
        );
    }

    let root = absolute_existing_root(args.path.as_deref().unwrap_or_else(|| Path::new(".")))?;
    let mode = match args.output {
        Some(output) => ScrapeMode::Export {
            output: absolute_output(output)?,
        },
        None => ScrapeMode::InPlace,
    };
    let prepared = vibe_scrape::prepare(ScrapeRequest {
        root,
        contract: args.contract,
        mode,
    })?;
    render_plan(ctx, prepared.plan.to_wire()?)
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

    let mode = serde_json::to_value(&plan.mode)?
        .as_str()
        .context("generated scrape plan mode is not a JSON string")?
        .to_owned();
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
    println!("  plan      {}", plan.plan_id);
    println!("  project   {}", plan.project.display_root);
    println!("  contract  {}", plan.contract.display_path);
    println!("  mode      {mode}");
    println!("  {counts}");
    for blocker in &plan.blockers {
        let path = blocker
            .path
            .as_deref()
            .map(|path| format!(" [{path}]"))
            .unwrap_or_default();
        println!("  blocker   {}{path}: {}", blocker.code, blocker.message);
    }
    finish_render(blocker_count)
}

fn finish_render(blocker_count: usize) -> Result<()> {
    if blocker_count == 0 {
        Ok(())
    } else {
        bail!("scrape plan is blocked by {blocker_count} finding(s)")
    }
}
