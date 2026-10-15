//! Bounded JSON-lines surface over the host-owned adapter route.
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::{
    io::{BufRead, Read, Write},
    path::PathBuf,
};
use vibe_orchestrator::machine_operation::adapter_plan::{
    self, AdapterPlanHost, AdapterPlanRoute, AdapterScope, AdapterTarget,
};
use vibe_wire::generated::adapter_plan::e1::request::{Command, CommandDescribe, Request};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    Clients,
    Skills,
    Deploy,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Scope {
    Project,
    User,
    Both,
}
#[derive(Debug, Args)]
pub struct AdapterPlanArgs {
    #[arg(long)]
    pub root: PathBuf,
    #[arg(long, value_enum)]
    pub kind: Kind,
    #[arg(long, required_if_eq("kind", "deploy"), conflicts_with = "scope")]
    pub profile: Option<String>,
    #[arg(long, value_enum)]
    pub scope: Option<Scope>,
    #[arg(long)]
    pub prepare: bool,
}
impl AdapterPlanArgs {
    fn target(&self) -> Result<AdapterTarget> {
        match self.kind {
            Kind::Clients => anyhow::bail!("clients uses its own wire family"),
            Kind::Skills => {
                anyhow::ensure!(self.profile.is_none(), "--profile applies only to deploy");
                let scope = match self.scope.unwrap_or(Scope::Project) {
                    Scope::Project => AdapterScope::Project,
                    Scope::User => AdapterScope::User,
                    Scope::Both => AdapterScope::Both,
                };
                let user_home = if scope == AdapterScope::Project {
                    None
                } else {
                    Some(dirs::home_dir().context("host user home unavailable")?)
                };
                Ok(AdapterTarget::Skills { scope, user_home })
            }
            Kind::Deploy => {
                anyhow::ensure!(self.scope.is_none(), "--scope applies only to skills");
                Ok(AdapterTarget::Deploy {
                    profile: self.profile.clone().context("deploy requires --profile")?,
                })
            }
        }
    }
}
pub fn run(args: AdapterPlanArgs, offline: bool) -> Result<()> {
    if args.kind == Kind::Clients {
        return client_capabilities::run(args);
    }
    let target = args.target()?;
    let root = super::resolve_project_root(&args.root)?;
    let host = AdapterPlanHost::new(&root, target)?;
    let prepared = if args.prepare {
        Some(match args.kind {
            Kind::Clients => unreachable!("branched before legacy route"),
            Kind::Skills => host.prepare(offline)?,
            Kind::Deploy => super::deploy::prepare_adapter_plan(
                &host,
                &root,
                args.profile.as_deref().context("deploy profile")?,
                offline,
            )?,
        })
    } else {
        None
    };
    let route =
        AdapterPlanRoute::new(host, prepared).map_err(|e| anyhow::anyhow!("{}", e.message))?;
    serve(
        &route,
        std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    )
}
pub fn serve(
    route: &AdapterPlanRoute,
    mut input: impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    let initial = adapter_plan::input(&Request {
        schema: 1,
        context_id: route.context_id().into(),
        target_id: route.target_id().into(),
        command: Command::Describe(Box::new(CommandDescribe {})),
    });
    let first = route.exchange(&serde_json::to_vec(&initial)?);
    serde_json::to_writer(&mut *output, &first)?;
    output.write_all(b"\n")?;
    output.flush()?;
    loop {
        let mut bytes = Vec::new();
        if input
            .by_ref()
            .take((adapter_plan::MAX_INPUT + 1) as u64)
            .read_until(b'\n', &mut bytes)?
            == 0
        {
            break;
        }
        anyhow::ensure!(
            bytes.len() <= adapter_plan::MAX_INPUT,
            "adapter input line exceeds bound"
        );
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        let reply = route.exchange(&bytes);
        serde_json::to_writer(&mut *output, &reply)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;

mod client_capabilities;
