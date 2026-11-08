//! Explicit activation of receipt-owned entries; no package mutation.
use super::*;
use crate::cli::McpActivationArgs;
use vibe_agent_projection::activation::ActivationIntent;

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(super) fn run(ctx: &output::Context, args: McpActivationArgs, enabled: bool) -> Result<()> {
    let scope = Scope::parse(&args.scope)?;
    let agents = Agent::parse_filter(&args.agent)?;
    let project = if scope != Scope::User {
        Some(resolve_project_root_required(&args.path)?)
    } else {
        None
    };
    let user = if scope != Scope::Project {
        Some(crate::commands::install::user_project_root()?)
    } else {
        None
    };
    let intent = ActivationIntent {
        enabled,
        actor: args.actor,
        provenance: if enabled {
            "vibe mcp enable"
        } else {
            "vibe mcp disable"
        }
        .into(),
    };
    vibe_agent_projection::activation::validate(&intent)?;
    let mut selections = Vec::new();
    for concrete in scope.expand() {
        let root = if concrete == Scope::Project {
            project.as_deref()
        } else {
            user.as_deref()
        }
        .ok_or_else(|| anyhow::anyhow!("activation scope has no package inventory"))?;
        let server = if let Some(package) = args.package.as_deref() {
            let exact =
                package_registration::exact_server_filter(std::slice::from_ref(&args.name))?;
            package_registration::selected_servers(root, package, Some(&exact))?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("managed MCP server not found"))?
        } else if args.name == SERVER_NAME {
            package_registration::builtin_server()
        } else {
            bail!("--package is required for a package MCP server");
        };
        for &agent in &agents {
            let config = agent
                .config_path_with_dirs(concrete, project.as_deref(), &ctx.agent_user_dirs)?
                .ok_or_else(|| anyhow::anyhow!("agent has no selected scope"))?;
            let payload = if args.package.is_none() {
                agent.build_mcp_entry()
            } else {
                package_registration::server_payload(agent, root, &server, true)?
            };
            selections.push((agent, concrete, config, server.clone(), payload));
        }
    }
    // Admit every exact owner before retaining any request; projection is a later phase.
    for (_, _, config, server, _) in &selections {
        package_registration::admit_activation(config, server, &intent)?;
    }
    if !args.dry_run {
        let mut retained = Vec::new();
        for (agent, scope, config, server, _) in &selections {
            package_registration::retain_known_intent(config,server,&intent).map_err(|e|e.context(format!("retained intents [{}]; current retention unconfirmed; projection not attempted",retained.join(", "))))?;
            retained.push(format!(
                "{}/{}:{}:{}",
                server.package,
                server.decl.name,
                agent.as_str(),
                scope.as_str()
            ));
        }
    }
    let mut rows = Vec::new();
    for (agent, concrete, config, server, payload) in &selections {
        let state = package_registration::set_activation(
            *agent,
            *concrete,
            config,
            server,
            payload,
            &intent,
            args.dry_run,
        )?;
        rows.push(
            vibe_wire::generated::agent_activation_report::ActivationRow {
                package: server.package.clone(),
                entry: server.decl.name.clone(),
                agent: state.agent.into(),
                scope: state.scope.into(),
                desired_enabled: state.desired_enabled,
                observed: state.state.into(),
                activation: state.activation,
                representation: (state.state == "disabled").then(|| {
                    if matches!(agent, Agent::OpenCode | Agent::Codex) {
                        "native-disabled"
                    } else {
                        "retracted"
                    }
                    .into()
                }),
            },
        );
    }
    if ctx.is_json() {
        ctx.emit_json(
            &vibe_wire::generated::agent_activation_report::AgentActivationReport {
                protocol: "vibe-agent-activation/1".into(),
                ok: true,
                command: if enabled { "mcp:enable" } else { "mcp:disable" }.into(),
                dry_run: args.dry_run,
                requested: Some(intent),
                results: rows,
            },
        )?;
    } else {
        ctx.summary(&format!(
            "{} managed MCP activation(s) {}",
            rows.len(),
            if args.dry_run {
                "previewed"
            } else {
                "reconciled"
            }
        ));
    }
    Ok(())
}
