//! Activation commands leave package inventory untouched.
use super::*;
use crate::cli::SkillActivationArgs;
use vibe_agent_projection::activation::ActivationIntent;
use vibe_agent_projection::agents::Agent;

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(super) fn run(
    ctx: &output::Context,
    args: SkillActivationArgs,
    enabled: Option<bool>,
    offline: bool,
) -> Result<()> {
    let scope = Scope::parse(&args.scope)?;
    let root = if scope == Scope::User
        && args.package == "org.vibevm.core/vibevm"
        && args.name == "vibevm"
    {
        args.path.canonicalize()?
    } else {
        super::super::resolve_project_root(&args.path)?
    };
    let intent = enabled.map(|enabled| ActivationIntent {
        enabled,
        actor: args.actor,
        provenance: if enabled {
            "vibe skill enable"
        } else {
            "vibe skill disable"
        }
        .into(),
    });
    if let Some(intent) = &intent {
        vibe_agent_projection::activation::validate(intent)?;
    }
    let rows = if args.package == "org.vibevm.core/vibevm" && args.name == "vibevm" {
        let agents = Agent::parse_filter(&args.agent)?;
        let desired = std::collections::BTreeMap::from([(
            "SKILL.md".into(),
            vibe_mcp::install::SKILL_TEMPLATE.as_bytes().to_vec(),
        )]);
        let selections = scope
            .expand()
            .into_iter()
            .flat_map(|scope| agents.iter().copied().map(move |agent| (agent, scope)))
            .collect::<Vec<_>>();
        if let Some(request) = &intent {
            for &(agent, concrete) in &selections {
                vibe_agent_projection::pkgskill::admit_managed_skill_activation(
                    agent,
                    concrete,
                    Some(&root),
                    &args.package,
                    &args.name,
                    request,
                )?;
            }
            if !args.dry_run {
                let mut retained = Vec::new();
                for &(agent, concrete) in &selections {
                    vibe_agent_projection::pkgskill::retain_managed_skill_activation(agent,concrete,Some(&root),&args.package,&args.name,request)
                        .map_err(|e|e.context(format!("retained intents [{}]; current retention unconfirmed; projection not attempted",retained.join(", "))))?;
                    retained.push(format!(
                        "{}/{}:{}:{}",
                        args.package,
                        args.name,
                        agent.as_str(),
                        concrete.as_str()
                    ));
                }
            }
        }
        let mut rows = Vec::new();
        for concrete in scope.expand() {
            for &agent in &agents {
                rows.push(if let Some(intent) = &intent {
                    vibe_agent_projection::pkgskill::change_managed_skill_activation(
                        vibe_agent_projection::pkgskill::SkillActivationTarget {
                            agent,
                            scope: concrete,
                            project: Some(&root),
                            provider: &args.package,
                            name: &args.name,
                        },
                        &desired,
                        intent,
                        args.dry_run,
                    )?
                } else {
                    vibe_agent_projection::pkgskill::managed_skill_status(
                        agent,
                        concrete,
                        Some(&root),
                        &args.package,
                        &args.name,
                    )?
                });
            }
        }
        rows
    } else {
        let names = vec![args.name];
        let filter = DeclaredSkillFilter::new(&names, Some(&args.agent))
            .with_provider(&args.package)
            .with_offline(offline);
        if intent.is_none() {
            vibe_agent_projection::pkgskill::declared_skill_activation_status(
                &root, &filter, scope,
            )?
        } else {
            let request = intent
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("activation request missing"))?;
            let admission = vibe_agent_projection::pkgskill::prepare_declared_skill_activation(
                &root, &filter, scope,
            )?;
            admission.activation_intent(request, !args.dry_run)?;
            if args.dry_run {
                vibe_agent_projection::pkgskill::declared_skill_activation_status(
                    &root, &filter, scope,
                )?
            } else {
                let plan =
                    prepare_declared_skill_projection(&root, &filter, scope).map_err(|e| {
                        e.context("activation intent retained; projection preparation failed")
                    })?;
                plan.activation(intent.as_ref(), false).map_err(|e| {
                    e.context("activation intent retained; projection reconciliation failed")
                })?
            }
        }
    };
    if ctx.is_json() {
        ctx.emit_json(
            &vibe_wire::generated::agent_activation_report::AgentActivationReport {
                protocol: "vibe-agent-activation/1".into(),
                ok: true,
                command: match enabled {
                    Some(true) => "skill:enable",
                    Some(false) => "skill:disable",
                    None => "skill:status",
                }
                .into(),
                dry_run: args.dry_run,
                requested: intent,
                results: rows
                    .into_iter()
                    .map(
                        |row| vibe_wire::generated::agent_activation_report::ActivationRow {
                            package: row.provider,
                            entry: row.entry,
                            agent: row.agent,
                            scope: row.scope.into(),
                            desired_enabled: row.desired_enabled,
                            observed: row.observed.into(),
                            activation: row.activation,
                            representation: (row.observed == "disabled")
                                .then(|| "retracted".into()),
                        },
                    )
                    .collect(),
            },
        )?;
    } else {
        for row in rows {
            ctx.step(&format!(
                "{} {} {} ({})",
                row.observed, row.entry, row.agent, row.scope
            ));
        }
    }
    Ok(())
}
