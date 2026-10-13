//! Local change preparation; explicit --apply composes one finite creation grant.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD-PREPARATION");
use clap::{Args, Subcommand};
use std::{path::PathBuf, process::ExitCode};
use vibe_core::manifest::change::{ChangeContext, ChangeId};
use vibe_orchestrator::{change::scaffold::*, document_analysis::*};
pub mod query;
mod report;
#[derive(Debug, Args)]
pub struct ChangeArgs {
    #[command(subcommand)]
    pub command: ChangeCommand,
}
#[derive(Debug, Subcommand)]
pub enum ChangeCommand {
    New(NewArgs),
    Check(query::ReadArgs),
    Show(query::ReadArgs),
    Coverage(query::ReadArgs),
}
#[derive(Debug, Args)]
pub struct NewArgs {
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub id: String,
    #[arg(long, default_value = "local")]
    pub slug: String,
    #[arg(long, default_value = "Local change")]
    pub title: String,
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    #[arg(long)]
    pub spec_root: Option<PathBuf>,
    #[arg(long)]
    pub fact: Vec<String>,
    #[arg(long)]
    pub apply: bool,
}
pub fn run(args: &ChangeArgs, json_output: bool) -> ExitCode {
    let args = match &args.command {
        ChangeCommand::New(args) => args,
        ChangeCommand::Check(args) => return query::run(args, "check", json_output),
        ChangeCommand::Show(args) => return query::run(args, "show", json_output),
        ChangeCommand::Coverage(args) => return query::run(args, "coverage", json_output),
    };
    let output = match execute(args) {
        Ok(report) => report,
        Err(error) => report::refused(&error, args.apply),
    };
    let failed = matches!(
        output,
        report::Report::Partial(_) | report::Report::Refused(_)
    );
    match report::bytes(&output) {
        Ok(bytes) => {
            if json_output {
                println!("{}", std::str::from_utf8(&bytes).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        }
        Err(error) => {
            eprintln!(
                "vibe: invalid scaffold report after possible effects: {error}; retained actual report: {}",
                serde_json::to_string(&output).unwrap()
            );
            return ExitCode::from(1);
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
fn execute(args: &NewArgs) -> anyhow::Result<report::Report> {
    let root = super::resolve_project_root(&args.path)?;
    let project = vibe_safefs::Project::open(&root)?;
    let saved = project
        .read_file_snapshot_bounded("vibe.toml", 4 * 1024 * 1024)?
        .ok_or_else(|| anyhow::anyhow!("missing saved node manifest"))?;
    let manifest = vibe_core::manifest::Manifest::parse_str(std::str::from_utf8(&saved.bytes)?)?;
    let context = ChangeContext::for_node(&manifest, ".", args.spec_root.as_deref())?;
    let directory = context.directory(&ChangeId::parse(&args.id)?);
    let mut declarations = vec![
        ResourceDeclaration {
            path: "vibe.toml".into(),
            role: ResourceRole::Manifest,
        },
        ResourceDeclaration {
            path: "vibe.lock".into(),
            role: ResourceRole::Lockfile,
        },
    ];
    for name in ["change.toml", "intent.md", "refinement.md", "acceptance.md"] {
        declarations.push(ResourceDeclaration {
            path: format!("{}/{name}", directory.display()),
            role: ResourceRole::Opaque,
        });
    }
    let host = DocumentAnalysisHost::new(
        &root,
        "vibe.toml",
        declarations,
        AnalysisLimits {
            resource_bytes: 4 * 1024 * 1024,
            capture_bytes: 16 * 1024 * 1024,
            open_buffers: 8,
        },
    )
    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let client = host.client().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let capture = client.freeze().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let node = host.resource("vibe.toml").unwrap();
    // --apply is the host's explicit local operation authorization. No
    // registered external editor transport is claimed by this CLI process.
    if args.apply {
        let authority = ScaffoldAuthority::new(host.clone(), node, directory)?;
        let plan = prepare_new_change(
            &authority,
            &capture,
            &args.from,
            &args.id,
            &args.slug,
            &args.title,
            args.spec_root.as_deref(),
            &args.fact,
        )?;
        let prepared = report::describe(plan.output())?;
        client.acknowledge_absent_edit(plan.barrier(), true)?;
        let outcome = plan.apply()?;
        Ok(report::applied(prepared, outcome))
    } else {
        let preview = preview_new_change(
            &host,
            &capture,
            node,
            &args.from,
            &args.id,
            &args.slug,
            &args.title,
            args.spec_root.as_deref(),
            &args.fact,
        )?;
        Ok(report::Report::Prepared(Box::new(report::ReportPrepared {
            schema: 1,
            prepared: report::describe(&preview)?,
        })))
    }
}
#[cfg(test)]
mod m_02_d_1_cases;
