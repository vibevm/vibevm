//! `vibe install <kind>:<name>[@version] …` — the thin CLI layer over
//! the `vibe-install` orchestrator (VIBEVM-SPEC §5.6, §9.1, §11.1).
//!
//! This module owns exactly the CLI's share of the transaction: input
//! normalisation (path canonicalisation, pkgref parsing, PROP-008 §2.6
//! short-name qualification), the `--git` declaration recording, cell
//! construction behind the [`vibe_install::InstallSource`] seam
//! (R-001 — the registry module builds the cells), the interactive
//! confirmation between plan and apply, and rendering. The pipeline
//! itself lives in `vibe-install`.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

mod report;
mod resolver;

pub(crate) use resolver::{InstallResolver, build_install_resolver};
pub(crate) use vibe_install::exact_pinned_pkgref;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::UserConfig;
use vibe_install::{InstallRequest, Plan, PlanEvent, PlanObserver};
use vibe_resolver::FeatureRequest;
use vibe_workspace::Workspace;
use vibe_workspace::hooks::{DEFAULT_ALLOWED_GROUPS, HookPolicy, HookTrust, decide_trust};

use crate::cli::InstallArgs;
use crate::commands::short_name;
use crate::exit_code::InstallError;
use crate::output;

use resolver::apply_git_source_flag;

/// Renders the orchestrator's typed plan events in the CLI's voice.
struct CtxObserver<'a>(&'a output::Context);

impl PlanObserver for CtxObserver<'_> {
    fn on(&self, event: PlanEvent) {
        let ctx = self.0;
        match event {
            PlanEvent::MigratingRequires { entries } => ctx.step(&format!(
                "Migrating [requires] from `vibe.lock` meta.root_dependencies ({} entr{})",
                entries,
                if entries == 1 { "y" } else { "ies" },
            )),
            PlanEvent::Reresolving { reason } => ctx.step(&format!("re-resolving — {reason}")),
            PlanEvent::HeldPinsConflicted { error } => ctx.step(&format!(
                "held pins conflicted with the change ({error}); re-resolving freely"
            )),
            PlanEvent::ResolvingRoots { roots } => ctx.heading(&format!(
                "Resolving {} root package{}…",
                roots,
                if roots == 1 { "" } else { "s" },
            )),
            PlanEvent::GraphSolved { roots, total } => ctx.step(&format!(
                "{} root, {} transitive — {} package{} total",
                roots,
                total - roots,
                total,
                if total == 1 { "" } else { "s" },
            )),
            PlanEvent::ConditionalIteration { iteration, extras } => ctx.step(&format!(
                "Conditional dependencies (iter {}): {} extra root{}",
                iteration,
                extras,
                if extras == 1 { "" } else { "s" },
            )),
            PlanEvent::FeaturesUnmatched { features } => ctx.step(&format!(
                "warning: requested feature{} {} not declared on any root package — silently ignored",
                if features.len() == 1 { "" } else { "s" },
                features
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

pub fn run(ctx: &output::Context, args: InstallArgs, embedded_root: Option<PathBuf>) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    // PROP-011 §2.3 — the materialise-diff strategy, read once from the
    // user config so a malformed config fails before any resolution.
    let slot_integrity = UserConfig::load()
        .context("loading the user config")?
        .install
        .slot_integrity;

    let mut manifest = Manifest::read(project_root.join(Manifest::FILENAME))?;

    // M1.15: `vibe install <pkgref> --git <url> --tag/branch/rev <ref>`
    // adds a git-source declaration to `[requires.packages]` before
    // resolving. The added declaration is picked up by the resolver
    // built immediately below; subsequent installs of the same project
    // reproduce the install via the now-recorded git-source entry.
    if args.git.is_some() {
        apply_git_source_flag(&args, &mut manifest, &project_root)
            .context("recording --git declaration to vibe.toml")?;
    }

    let resolver = build_install_resolver(&args, &manifest, embedded_root.as_deref())?;

    // Parse the CLI pkgrefs and qualify short names at the input
    // boundary (PROP-008 §2.6) — manifests only ever store the
    // qualified form, and the orchestrator requires it.
    let cli_roots: Vec<PackageRef> = args
        .packages
        .iter()
        .map(|raw| PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`")))
        .collect::<Result<_>>()?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let lockfile_path = workspace.root.join(Lockfile::FILENAME);
    let lockfile_snapshot = if lockfile_path.exists() {
        Lockfile::read(&lockfile_path)?
    } else {
        Lockfile::empty(
            generated_by(),
            crate::commands::init::current_timestamp_utc(),
        )
    };
    let cli_roots: Vec<PackageRef> = cli_roots
        .iter()
        .map(|r| short_name::qualify(&resolver, r, &lockfile_snapshot))
        .collect::<Result<_>>()?;

    let request = InstallRequest {
        roots: cli_roots,
        features: FeatureRequest {
            explicit: args.features.clone(),
            no_defaults: args.no_default_features,
            all: args.all_features,
        },
        language: args.language.clone(),
        exact: args.exact,
        generated_by: generated_by(),
    };

    let plan = vibe_install::plan(&resolver, &project_root, request, &CtxObserver(ctx))?;
    match plan {
        Plan::Fresh => {
            // PROP-011 §2.2 — application is just a whole-tree boot
            // regeneration (cheap, self-healing — §2.4).
            ctx.heading("vibe.lock is fresh — skipping resolution");
            let ws = Workspace::discover(&project_root)
                .context("re-discovering the workspace for boot regeneration")?;
            let nodes = vibe_workspace::install::regenerate_boot(&ws)
                .context("regenerating boot artifacts from the materialised state")?;
            report::emit_fresh_report(ctx, &nodes)
        }
        Plan::Ready(planned) => {
            // Show the plan: the packages to materialise.
            report::present_resolution(ctx, &planned.resolution);

            // Confirm (unless --assume-yes or --json or not a TTY).
            let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
                true
            } else if !console::user_attended() {
                // No TTY → refuse to apply without explicit --assume-yes.
                // This matches the book's "ask a human" discipline for any
                // destructive action.
                bail!(
                    "no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively"
                );
            } else {
                Confirm::new()
                    .with_prompt(format!(
                        "Materialise {} package{} into vibedeps/ and regenerate boot artifacts?",
                        planned.resolution.len(),
                        if planned.resolution.len() == 1 {
                            ""
                        } else {
                            "s"
                        },
                    ))
                    .default(false)
                    .interact()
                    .context("reading user confirmation")?
            };
            if !approved {
                return Err(InstallError::UserDeclined.into());
            }

            // PROP-020 §2.3 — resolve install-hook trust before applying:
            // allow-listed groups (incl. `org.vibevm`) and `--allow-hooks`
            // run silently, any other hook-declaring package prompts for
            // consent interactively, and a non-interactive run aborts rather
            // than run third-party code unseen.
            let hook_policy = resolve_hook_policy(ctx, &args, &planned.resolution)?;

            let applied = vibe_install::apply(&resolver, *planned, slot_integrity, &hook_policy)?;
            report::emit_report(ctx, &applied)
        }
    }
}

/// Resolve the install-hook trust policy for a planned resolution
/// (PROP-020 §2.3). Allow-listed groups (`DEFAULT_ALLOWED_GROUPS`, including
/// `org.vibevm`) and `--allow-hooks` run with no prompt; any other
/// hook-declaring package's group is asked for consent once, interactively;
/// a non-interactive run carrying such a package aborts unless `--allow-hooks`
/// is set — a hook never runs unseen. A declined group is simply left out of
/// the policy, so the pipeline skips-and-reports its hooks rather than failing.
pub(crate) fn resolve_hook_policy(
    ctx: &output::Context,
    args: &InstallArgs,
    resolution: &[vibe_workspace::install::ResolvedDep],
) -> Result<HookPolicy> {
    let allowed: Vec<String> = DEFAULT_ALLOWED_GROUPS
        .iter()
        .map(|s| s.to_string())
        .collect();
    // Hook consent is interactive only on an attended TTY in human mode;
    // `--assume-yes`, `--json`, and `--unattended` are each non-interactive
    // for the purpose of running third-party code (PROP-020 §2.3).
    let interactive =
        console::user_attended() && !ctx.is_json() && !ctx.is_unattended() && !args.assume_yes;
    let mut consented: Vec<String> = Vec::new();
    let mut decided: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for dep in resolution {
        if dep.manifest.hooks.is_empty() {
            continue;
        }
        // One decision per group covers all its hook-declaring packages.
        let group = dep.group.as_str().to_string();
        if !decided.insert(group.clone()) {
            continue;
        }
        match decide_trust(&dep.group, &allowed, interactive, args.allow_hooks) {
            HookTrust::Allowed => {}
            HookTrust::NeedsConsent => {
                let ok = Confirm::new()
                    .with_prompt(format!(
                        "Package group `{group}` declares install hooks (PROP-020). \
                         Run them during this install?"
                    ))
                    .default(false)
                    .interact()
                    .context("reading hook consent")?;
                if ok {
                    consented.push(group);
                }
            }
            HookTrust::Refused => {
                bail!(
                    "package group `{group}` declares install hooks but is not trusted to run \
                     them non-interactively (PROP-020 §2.3). Re-run interactively to consent, \
                     allow-list `{group}`, or pass `--allow-hooks`."
                );
            }
        }
    }
    let mut allowed_groups = allowed;
    allowed_groups.extend(consented);
    Ok(HookPolicy {
        allowed_groups,
        allow_hooks: args.allow_hooks,
    })
}

/// The lockfile provenance stamp this binary writes.
fn generated_by() -> String {
    format!("vibe {}", env!("CARGO_PKG_VERSION"))
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = crate::commands::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}
