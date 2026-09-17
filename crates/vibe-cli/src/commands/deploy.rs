//! The three deploy command surfaces — §7's own list:
//!
//! ```text
//! vibe deploy [--profile X] [--plan]
//! vibe undeploy --profile X
//! vibe deployments [--json]
//! ```
//!
//! What this cell owns is exactly what a command layer owns: the flags,
//! the ONE profile resolution ([`profile`]), the ONE resolution of the
//! injected home-and-client authority ([`client_authority`]), and the
//! rendering. It owns no transaction, no state layout and no provider —
//! those are the engine's, and this surface reaches them through the same
//! public functions any other surface would.
//!
//! Two of the three verbs are READ-ONLY and say so by construction: they
//! take no mutation lease, run no chain, and call only functions that
//! write nothing. `vibe deploy` without `--plan` is the exception, and it
//! is not a fourth path — it is the ordinary ninth phase verb, carrying
//! the resolved selection down as data.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::manifest::{Manifest, TargetOs};
use vibe_lifecycle::native::{
    NativeBuildExecution, NativePlatform, PreparedNativeMechanisms, preflight_native_mechanisms,
    project_native_mechanisms,
};
use vibe_lifecycle::{DeployExecution, deploy_state_home};

use crate::cli::{DeployArgs, UndeployArgs};
use crate::output;

pub(crate) mod clients;
pub(crate) mod profile;
mod report;

#[cfg(test)]
use report::plan_json;

#[cfg(test)]
#[path = "deploy/tests.rs"]
mod tests;

pub(crate) use profile::resolve_profile;

/// The deploy half of one chain run, resolved ONCE at this surface.
///
/// The two halves are resolved together because §7.0.5 and §6.3.0.6 are one
/// surface act: the profile comes off the flags this cell parsed and the
/// manifest snapshot the caller owns, and the home/client authority comes
/// off [`client_authority`]. A run that carried one without the other would
/// be half-resolved below a boundary that cannot re-derive either.
///
/// `None` back means the project declares no deploy profiles — the
/// historical no-op — and the deploy fence then arms nothing.
pub(crate) fn resolve_authority(
    deploy: Option<&vibe_core::manifest::DeploySection>,
    profile: Option<&str>,
    os: TargetOs,
) -> Result<Option<vibe_orchestrator::DeployAuthority>> {
    let Some(resolution) = resolve_profile(deploy, profile, profile::ProfileMode::Forward(os))?
    else {
        return Ok(None);
    };
    let (user_home, clients) = client_authority()?;
    Ok(Some(vibe_orchestrator::DeployAuthority {
        selection: resolution.into_selection(),
        user_home,
        clients,
    }))
}

/// `vibe deploy [--profile X] [--plan]`.
///
/// Without `--plan` this is the ninth phase verb: it runs the inclusive
/// chain through `deploy`, carrying the resolved selection as data. With
/// `--plan` it is a read-only planner and NOT a chain run (§7.0.6).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn run(
    ctx: &output::Context,
    args: DeployArgs,
    prepare_install: impl FnOnce() -> Option<std::path::PathBuf>,
    root_offline: bool,
) -> Result<()> {
    if args.plan {
        return plan(ctx, &args);
    }
    let profile = args.profile.clone();
    super::lifecycle::run(
        ctx,
        vibe_lifecycle::Phase::Deploy,
        args.lifecycle,
        prepare_install,
        root_offline,
        Some(super::lifecycle::DeployRequest { profile }),
    )
}

/// `vibe deploy --profile X --plan` — §7.0.6's read-only planner.
///
/// It takes NO mutation lease and enters no chain: a plan that leased the
/// workspace would be a plan that could block a build, and a plan that
/// entered the chain would be a plan that built.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn plan(ctx: &output::Context, args: &DeployArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.lifecycle.path)?;
    let manifest = read_manifest(&root)?;
    let platform = NativePlatform::current()?;
    let os = current_target_os(platform);
    let Some(resolution) = resolve_profile(
        manifest.deploy.as_ref(),
        args.profile.as_deref(),
        profile::ProfileMode::Forward(os),
    )?
    else {
        return report::nothing(ctx, "plan");
    };
    let loaded = vibe_orchestrator::inspect(&root)?;
    let roots = state_roots()?;
    let (user_home, clients) = client_authority()?;
    let targets = deploy_targets(&manifest);
    let created_at = now();
    let execution = DeployExecution {
        project_root: &root,
        targets: &targets,
        selection: resolution.selection(),
        registry: &loaded.mechanisms,
        routes: &manifest.mechanism_routes,
        state_home: &roots.deployments,
        settings_root: &roots.settings,
        user_home: &user_home,
        clients: &clients,
        project: &identity(&manifest),
        package: None,
        created_at: &created_at,
    };
    let preflight = ctx.progress().task("Preparing deploy providers");
    preflight.set_progress(
        0,
        Some(resolution.selection().targets.len() as u64),
        "targets",
    );
    let prepared = match rehydrate(&execution, platform) {
        Ok(prepared) => {
            for target in &resolution.selection().targets {
                preflight
                    .progress()
                    .task(format!("Prepared deploy provider for {target}"))
                    .finish();
            }
            preflight.set_progress(
                resolution.selection().targets.len() as u64,
                Some(resolution.selection().targets.len() as u64),
                "targets",
            );
            preflight.finish();
            prepared
        }
        Err(error) => {
            preflight.fail("deploy provider preparation failed");
            return Err(error);
        }
    };
    let planning = ctx.progress().task("Planning deploy targets");
    planning.set_progress(
        0,
        Some(resolution.selection().targets.len() as u64),
        "targets",
    );
    let reports = match prepared.plan_deploy_targets(&execution) {
        Ok(reports) => {
            for report in &reports {
                planning
                    .progress()
                    .task(format!("Planned deploy target {}", report.target))
                    .finish();
            }
            planning.set_progress(
                resolution.selection().targets.len() as u64,
                Some(resolution.selection().targets.len() as u64),
                "targets",
            );
            planning.finish();
            reports
        }
        Err(error) => {
            planning.fail("deploy planning failed");
            return Err(error.into());
        }
    };
    report::plan(ctx, &resolution, &reports)
}

/// `vibe undeploy --profile X`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn run_undeploy(ctx: &output::Context, args: UndeployArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.path)?;
    let manifest = read_manifest(&root)?;
    let Some(resolution) = resolve_profile(
        manifest.deploy.as_ref(),
        Some(&args.profile),
        profile::ProfileMode::Inverse,
    )?
    else {
        bail!(
            "`--profile {}` was requested, but this project declares no deploy profiles \
             (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: \
             run `vibe deployments` to see what this machine has deployed)",
            args.profile,
        );
    };
    let loaded = vibe_orchestrator::inspect(&root)?;
    let roots = state_roots()?;
    let (user_home, clients) = client_authority()?;
    let targets = deploy_targets(&manifest);
    let platform = NativePlatform::current()?;
    let created_at = now();
    let execution = DeployExecution {
        project_root: &root,
        targets: &targets,
        selection: resolution.selection(),
        registry: &loaded.mechanisms,
        routes: &manifest.mechanism_routes,
        state_home: &roots.deployments,
        settings_root: &roots.settings,
        user_home: &user_home,
        clients: &clients,
        project: &identity(&manifest),
        package: None,
        created_at: &created_at,
    };
    let target_count = resolution.selection().targets.len();
    let preflight = ctx.progress().task("Preparing undeploy providers");
    preflight.set_progress(0, Some(target_count as u64), "targets");
    let prepared = match rehydrate(&execution, platform) {
        Ok(prepared) => {
            for target in &resolution.selection().targets {
                preflight
                    .progress()
                    .task(format!("Prepared undeploy provider for {target}"))
                    .finish();
            }
            preflight.set_progress(target_count as u64, Some(target_count as u64), "targets");
            preflight.finish();
            prepared
        }
        Err(error) => {
            preflight.fail("undeploy provider preparation failed");
            return Err(error);
        }
    };
    let removal = ctx.progress().task("Removing deployed targets");
    removal.set_progress(0, Some(target_count as u64), "targets");
    let removals = match prepared.undeploy_targets(&execution) {
        Ok(removals) => {
            for outcome in &removals {
                removal
                    .progress()
                    .task(format!("Removed deploy target {}", outcome.target))
                    .finish();
            }
            removal.set_progress(target_count as u64, Some(target_count as u64), "targets");
            removal.finish();
            removals
        }
        Err(error) => {
            removal.fail("undeploy failed");
            return Err(error.into());
        }
    };
    report::removals(ctx, resolution.selection(), &removals)
}

fn rehydrate(
    execution: &DeployExecution<'_>,
    platform: NativePlatform,
) -> Result<PreparedNativeMechanisms> {
    let targets = execution
        .targets
        .iter()
        .filter(|target| execution.selection.targets.contains(&target.id))
        .cloned()
        .collect::<Vec<_>>();
    let plan = project_native_mechanisms(&[], &targets, execution.registry, execution.routes)?;
    let native = NativeBuildExecution {
        candidates: &[],
        selected_project_root: execution.project_root,
        registry: execution.registry,
        routes: execution.routes,
        platform,
        offline: true,
        created_at: execution.created_at,
    };
    let prepared = preflight_native_mechanisms(plan, &native)?.rehydrate(&native)?;
    prepared.validate_restart(execution)?;
    Ok(prepared)
}

/// `vibe deployments [--json]` — the machine's receipts, and nothing else.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn run_deployments(ctx: &output::Context) -> Result<()> {
    report::deployments(ctx, &state_roots()?.deployments)
}

/// Observe the process OS at the command boundary and nowhere below it.
fn current_target_os(platform: NativePlatform) -> TargetOs {
    match platform {
        NativePlatform::WindowsX86_64 => TargetOs::Windows,
        NativePlatform::LinuxX86_64 => TargetOs::Linux,
        NativePlatform::MacosAarch64 => TargetOs::Macos,
    }
}

/// The selected node's manifest — ONE read, at the resolved root.
fn read_manifest(root: &std::path::Path) -> Result<Manifest> {
    Manifest::read(root.join(Manifest::FILENAME))
        .with_context(|| format!("reading `{}`", root.join(Manifest::FILENAME).display()))
}

/// The declared deploy targets, or none.
fn deploy_targets(manifest: &Manifest) -> Vec<vibe_core::manifest::DeployTarget> {
    manifest
        .deploy
        .as_ref()
        .map(|section| section.targets.clone())
        .unwrap_or_default()
}

/// The two user-state roots one deploy surface hands down: the settings
/// directory itself and the deployment state home inside it.
///
/// §7.1.0 ruling 2 puts BOTH on the execution — a user-scope provider
/// reconciles a destination under the settings root, and the engine keeps
/// its intents and receipts under the state home — and this is the ONE
/// place either is resolved. Nothing below a command surface calls
/// `settings_dir()`, so a test that relocates `$VIBE_SETTINGS` relocates
/// the whole deployment, destination included.
struct StateRoots {
    settings: std::path::PathBuf,
    deployments: std::path::PathBuf,
}

/// Resolve them, once.
fn state_roots() -> Result<StateRoots> {
    let settings = vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the vibevm settings directory could not be resolved, so deployment receipts have \
             nowhere to live (violates \
             spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: set \
             `$VIBE_SETTINGS`, or make a home directory resolvable, then rerun)"
        )
    })?;
    Ok(StateRoots {
        deployments: deploy_state_home(&settings),
        settings,
    })
}

/// §6.3.0.6's ONE resolution: the invoking user's home, and the three
/// client executables a deploy run may invoke.
///
/// > "Home and executable authority are injected. `DeployExecution` carries
/// > the exact user home beside `settings_root`, plus explicit
/// > Claude/Codex/OpenCode executable paths. The CLI surface resolves them
/// > once; every lower cell and provider is forbidden from calling
/// > `dirs::home_dir`, reading `HOME`/`USERPROFILE`/`CODEX_HOME`/
/// > `CLAUDE_CONFIG_DIR`, searching `PATH`, or finding a real client."
///
/// The home is NOT derived from the settings root: `$VIBE_SETTINGS`
/// relocates that root anywhere, while a client destination hangs off the
/// home itself, and deriving one from the other would put a user's client
/// state inside vibevm's own directory (or, with the override unset, the
/// reverse).
///
/// The three executables are RESOLVED here, once, by [`clients`]: each
/// member comes back as an absolute path or as a typed `Missing` naming the
/// command word. Handing a bare command word down would not be a
/// resolution at all — `Command::new("claude")` searches `PATH` inside the
/// provider, which is the lookup this surface exists to have already done.
///
/// A client that is not installed does NOT fail the run: an ordinary
/// `deploy:vibe-bin` profile never looks at any of the three, and three
/// eager refusals here would make every deploy depend on three unrelated
/// CLIs. The typed absence travels down and the provider that selected that
/// client refuses with remediation.
fn client_authority() -> Result<(std::path::PathBuf, vibe_lifecycle::ClientExecutables)> {
    let home = dirs::home_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the invoking user's home directory could not be resolved, so a client deployment has \
             no destination root (violates \
             spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: make a home \
             directory resolvable, then rerun)"
        )
    })?;
    Ok((home, clients::resolve_clients()))
}

/// The selected node's identity — the same rendering the dispatch
/// assembles, so a receipt written by `vibe deploy` and one read by
/// `vibe undeploy` key under one name.
fn identity(manifest: &Manifest) -> String {
    if let Some(package) = &manifest.package {
        return format!("{}/{}", package.group, package.name);
    }
    if let Some(project) = &manifest.project {
        return match &project.group {
            Some(group) => format!("{group}/{}", project.name),
            None => project.name.clone(),
        };
    }
    "<workspace>".to_owned()
}

/// The invocation's RFC 3339 instant. The read-only surfaces stamp
/// nothing durable, but the executor's record vocabulary takes one, and a
/// surface is where a clock is read.
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
