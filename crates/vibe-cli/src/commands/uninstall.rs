//! `vibe uninstall <group>/<name>` — remove an installed package.
//!
//! In the PROP-009 loading model, uninstalling a package removes its
//! `vibedeps/` slot, drops its lockfile entry and its `[requires]`
//! declaration, and regenerates every node's boot artifacts so the
//! package no longer appears in the computed boot sequence.
//!
//! Spec: spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009-loading-model.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};

use crate::exit_code::InstallError;
use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::UserConfig;
use vibe_core::{Group, PackageRef};
use vibe_facts::{package_file_path, remove_package_file};
use vibe_workspace::Workspace;
use vibe_workspace::materialization::{DestructiveGuard, guard_destructive};
use vibe_workspace::vibedeps;

use crate::cli::UninstallArgs;
use crate::commands::short_name;
use crate::output;

pub fn run(ctx: &output::Context, args: UninstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let manifest = load_project_manifest(&project_root)?;
    let lockfile = load_lockfile(&workspace.root)?;
    let user_config = UserConfig::load().context("loading the user config")?;
    let spec_format =
        crate::commands::install::resolve_spec_format(&manifest, user_config.install.spec_format);

    let pkgref =
        PackageRef::parse(&args.package).with_context(|| format!("parsing `{}`", args.package))?;
    // `vibe uninstall` acts on an already-installed package, so a bare
    // short name resolves against `vibe.lock` alone — no index, no network
    // (the lockfile is the authority for what is installed). A name that is
    // not locked fails here with a clear "not installed", not a lookup.
    let pkgref = short_name::qualify_locked(&pkgref, &lockfile)?;
    let Some(group) = pkgref.group.as_ref() else {
        bail!("`{pkgref}` resolved without a group — internal: `qualify_locked` should qualify");
    };

    // The materialised slot is keyed by `(group, name, version)`; the
    // resolved version and the package `kind` (metadata) are both read
    // from the lockfile entry.
    let locked = lockfile.find(group, &pkgref.name).ok_or_else(|| {
        anyhow!(
            "package `{}/{}` is not installed in `{}`",
            group,
            pkgref.name,
            workspace.root.display()
        )
    })?;
    let version = locked.version.clone();
    // The recorded materialization mode drives the PROP-022 §2.6 destructive
    // guard below — an in-place slot is a non-vendored git clone.
    let mode = locked.materialization;

    // The slot path depends on the mode: an in-place slot is the unversioned
    // `vibedeps/<group>.<name>/` git working tree (PROP-022 §2.4); every other
    // mode is the versioned slot.
    let slot = if mode.is_in_place() {
        vibedeps::in_place_slot_rel_path(&locked.group, &pkgref.name)
    } else {
        vibedeps::slot_rel_path(&locked.group, &pkgref.name, &version)
    };
    let plan = vibe_install::removal::plan_removal(&workspace, &project_root, group, &pkgref.name)?;
    let selected_removed = plan.removed().contains(&plan.selected);
    let preflight = ctx.progress().task("Preflighting package removal");
    preflight.detail(format!("selected {group}/{}@{version}", pkgref.name));
    preflight.detail(format!("materialisation mode: {mode:?}"));
    preflight.detail(plan.explanation());
    preflight.finish();
    if !ctx.is_json() && !ctx.is_quiet() {
        ctx.heading(&format!(
            "\nUninstall {}/{}@{} — {}",
            group,
            pkgref.name,
            version,
            plan.explanation()
        ));
    }

    // PROP-022 §2.6 — removing an `in-place` slot deletes a project-local git
    // clone whose only restoration is a network re-clone; it must never happen
    // silently. The guard refuses a non-interactive run with no explicit
    // opt-in, and forces a mandatory `y/n` that `--json` cannot auto-answer.
    let interactive = console::user_attended() && !ctx.is_json() && !ctx.is_unattended();
    let opted_in = args.assume_yes || ctx.is_unattended();
    let guarded_mode = if selected_removed {
        mode
    } else {
        vibe_core::manifest::Materialization::Copy
    };
    let approved = match guard_destructive(guarded_mode, interactive, opted_in) {
        DestructiveGuard::Abort => bail!(
            "`{group}/{}` is materialised in-place (PROP-022 §2.6) — a project-local git \
             clone restorable only by re-cloning over the network. Refusing to remove it \
             non-interactively; re-run interactively or pass `--assume-yes` to confirm.",
            pkgref.name,
        ),
        DestructiveGuard::ConfirmInteractively => ctx.suspend_progress(|| {
            Confirm::new()
                .with_prompt(format!(
                    "Remove the in-place slot for {}/{}@{}? This deletes the local git clone; \
                     restoring it needs a network re-clone.",
                    group, pkgref.name, version
                ))
                .default(false)
                .interact()
                .context("reading user confirmation")
        })?,
        // Non-in-place, or in-place with an explicit opt-in: the established
        // uninstall confirmation contract (`--assume-yes` / `--unattended` /
        // `--json` imply yes; a non-TTY without them is a hard error).
        DestructiveGuard::Proceed => {
            if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
                true
            } else if !console::user_attended() {
                bail!(
                    "no TTY available for confirmation; re-run with `--assume-yes` to uninstall non-interactively"
                );
            } else {
                ctx.suspend_progress(|| {
                    Confirm::new()
                        .with_prompt(format!("Uninstall {}/{}@{}?", group, pkgref.name, version))
                        .default(false)
                        .interact()
                        .context("reading user confirmation")
                })?
            }
        }
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    vibe_install::removal::apply_removal(plan, spec_format)?;

    let package = format!("{group}/{}", pkgref.name);
    let adoption_facts = handle_adoption_facts(
        ctx,
        &project_root,
        &package,
        interactive && selected_removed,
        args.assume_yes,
    )?;
    emit_report(
        ctx,
        group,
        &pkgref.name,
        &version.to_string(),
        selected_removed.then_some(slot.as_str()),
        &adoption_facts,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AdoptionFactsDisposition {
    Absent,
    Removed(String),
    Kept(String),
}

fn handle_adoption_facts(
    ctx: &output::Context,
    project_root: &Path,
    package: &str,
    interactive: bool,
    assume_yes: bool,
) -> Result<AdoptionFactsDisposition> {
    let file = package_file_path(project_root, package);
    if !file.is_file() {
        return Ok(AdoptionFactsDisposition::Absent);
    }
    let display = file
        .strip_prefix(project_root)
        .unwrap_or(&file)
        .to_string_lossy()
        .replace('\\', "/");
    let remove = if interactive && !assume_yes {
        ctx.suspend_progress(|| {
            Confirm::new()
                .with_prompt(format!("Remove its adoption facts ({display})?"))
                .default(false)
                .interact()
                .context("reading adoption-facts confirmation")
        })?
    } else {
        false
    };
    if !remove {
        return Ok(AdoptionFactsDisposition::Kept(display));
    }
    if !remove_package_file(project_root, package)? {
        bail!("adoption facts `{display}` disappeared before removal");
    }
    Ok(AdoptionFactsDisposition::Removed(display))
}

fn emit_report(
    ctx: &output::Context,
    group: &Group,
    name: &str,
    version: &str,
    slot: Option<&str>,
    adoption_facts: &AdoptionFactsDisposition,
) -> Result<()> {
    if ctx.is_json() {
        let adoption_facts = match adoption_facts {
            AdoptionFactsDisposition::Absent => serde_json::json!({"status": "absent"}),
            AdoptionFactsDisposition::Removed(path) => {
                serde_json::json!({"status": "removed", "path": path})
            }
            AdoptionFactsDisposition::Kept(path) => serde_json::json!({
                "status": "kept",
                "path": path,
                "hint": "run `vibe facts clean` to drop orphaned overlays",
            }),
        };
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "uninstall",
            "package": format!("{group}/{name}"),
            "version": version,
            "removed_slot": slot,
            "adoption_facts": adoption_facts,
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let facts = match adoption_facts {
            AdoptionFactsDisposition::Kept(path) => {
                format!("; kept {path}; run `vibe facts clean` to drop orphaned overlays")
            }
            AdoptionFactsDisposition::Removed(path) => format!("; removed {path}"),
            AdoptionFactsDisposition::Absent => String::new(),
        };
        let action = if slot.is_some() {
            "declaration and owned slot removed"
        } else {
            "declaration removed; slot retained by dependency closure"
        };
        ctx.summary(&format!(
            "vibe uninstall: {group}/{name}@{version} {action}{facts}"
        ));
        return Ok(());
    }
    if let Some(slot) = slot {
        ctx.removed(slot);
    }
    match adoption_facts {
        AdoptionFactsDisposition::Absent => {}
        AdoptionFactsDisposition::Removed(path) => ctx.removed(path),
        AdoptionFactsDisposition::Kept(path) => ctx.skipped(
            path,
            "adoption facts kept; run `vibe facts clean` to drop orphaned overlays",
        ),
    }
    ctx.summary(&format!(
        "\nRemoved declaration {group}/{name}@{version} — regenerated boot; shared dependencies retained."
    ));
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join("vibe.toml").exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    Ok(Lockfile::read(&path)?)
}

fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    Ok(Manifest::read(&path)?)
}

#[cfg(test)]
mod m_05_e_3_cases {
    mod fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vibe-orchestrator/src/install/m_05_e_2_fixture.rs"
        ));
    }
    use super::*;

    fn installed_shared_world() -> fixture::Fixture {
        installed_shared_world_mode(false)
    }

    fn installed_shared_world_mode(configured: bool) -> fixture::Fixture {
        let mut f = fixture::Fixture::new();
        if !configured {
            f.request.configuration = None;
        }
        let group = f.request.roots[0].group.clone().unwrap();
        let package = f.temp.path().join(format!("registry/{group}/other/v1.0.0"));
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("vibe.toml"), format!("[package]\ngroup='{group}'\nname='other'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'{group}/app'='*'\n[boot_snippet]\nsource='boot.md'\n")).unwrap();
        std::fs::write(f.root.join("vibe.toml"), format!("[project]\nname='remove'\nversion='1.0.0'\n[requires.packages]\n'{group}/app'='*'\n'{group}/other'='*'\n")).unwrap();
        std::fs::create_dir_all(package.join("a")).unwrap();
        std::fs::write(package.join("a/b"), b"owned nested file").unwrap();
        f.request
            .roots
            .push(format!("{group}/other").parse().unwrap());
        std::fs::write(package.join("boot.md"), "# other contribution\n").unwrap();
        f.request.roots = Manifest::read(f.root.join("vibe.toml"))
            .unwrap()
            .requires
            .packages;
        f.installed();
        f
    }

    #[cfg(unix)]
    #[test]
    fn actual_pre_and_post_hook_failures_retain_external_effect_uncertainty() {
        use vibe_install::Plan;
        use vibe_workspace::configuration_transaction::RECOVERY_DIRECTORY;
        for phase in ["pre-install", "post-install"] {
            let mut f = fixture::Fixture::new();
            f.request.configuration = None;
            let group = f.request.roots[0].group.clone().unwrap();
            let package = f.temp.path().join(format!("registry/{group}/app/v1.0.0"));
            std::fs::write(package.join("vibe.toml"), format!("[package]\ngroup='{group}'\nname='app'\nkind='flow'\nversion='1.0.0'\n[hooks]\n{phase}='hooks/fail'\n")).unwrap();
            std::fs::create_dir(package.join("hooks")).unwrap();
            std::fs::write(
                package.join("hooks/fail.sh"),
                "printf 'external effect' > ../hook-effect\nexit 19\n",
            )
            .unwrap();
            let source = f.source.bind_request(f.request.clone()).unwrap();
            let Plan::Ready(plan) = vibe_install::plan(
                &source,
                &f.root,
                f.install_request(),
                &vibe_install::NullObserver,
            )
            .unwrap() else {
                panic!("ready");
            };
            let result = vibe_install::apply(
                &source,
                *plan,
                vibe_core::user_config::SlotIntegrity::TrustPresence,
                &vibe_workspace::hooks::HookPolicy {
                    allowed_groups: vec![],
                    allow_hooks: true,
                },
            );
            if phase == "pre-install" {
                assert!(result.is_err());
            } else {
                let report = result.unwrap();
                assert!(
                    report
                        .post_install_reports
                        .iter()
                        .any(|report| report.status == "post-install-failed")
                );
                assert!(f.root.join("vibe.lock").exists());
            }
            let slot =
                vibedeps::slot_abs_path(&f.root, &group, "app", &semver::Version::new(1, 0, 0));
            assert_eq!(
                std::fs::read(slot.parent().unwrap().join("hook-effect")).unwrap(),
                b"external effect"
            );
            let operations = std::fs::read_dir(f.root.join(RECOVERY_DIRECTORY))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(operations.len(), 1);
            let journal: serde_json::Value = serde_json::from_slice(
                &std::fs::read(operations[0].path().join("journal.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(journal["external_effects_possible"], true);
            assert_eq!(journal["in_flight"], true);
        }
    }

    #[test]
    fn standard_source_unsat_and_needs_selection_refuse_before_product_writes() {
        for unsat in [false, true] {
            let mut f = fixture::Fixture::new();
            let group = f.request.roots[0].group.clone().unwrap();
            if unsat {
                f.request.configuration = None;
                f.request.roots = vec![format!("{group}/app@=9.0.0").parse().unwrap()];
                std::fs::write(f.root.join("vibe.toml"), format!("[project]\nname='unsat'\nversion='1.0.0'\n[requires.packages]\n'{group}/app'='=9.0.0'\n")).unwrap();
            } else {
                f.request.configuration.as_mut().unwrap().request.edges[0]
                    .selections
                    .remove("choice");
            }
            let before = std::fs::read(f.root.join("vibe.toml")).unwrap();
            let result = f
                .source
                .bind_request(f.request.clone())
                .map_err(|e| e.to_string())
                .and_then(|source| {
                    vibe_install::plan(
                        &source,
                        &f.root,
                        f.install_request(),
                        &vibe_install::NullObserver,
                    )
                    .map_err(|e| e.to_string())
                });
            assert!(result.is_err(), "unexpected admitted plan: {result:?}");
            assert_eq!(std::fs::read(f.root.join("vibe.toml")).unwrap(), before);
            assert!(!f.root.join("vibe.lock").exists());
            assert!(
                !f.root
                    .join(vibe_workspace::configuration_transaction::RECOVERY_DIRECTORY)
                    .exists()
            );
        }
    }

    #[test]
    fn configured_branch_switch_keeps_dependency_required_by_another_root() {
        use vibe_core::manifest::{SpecFormat, feature_effects::*};
        let mut f = fixture::Fixture::new();
        let group = f.request.roots[0].group.clone().unwrap();
        let registry = f.temp.path().join("registry");
        for (name, requires) in [
            ("shared", String::new()),
            (
                "other",
                format!("[requires.packages]\n'{group}/shared'='*'\n"),
            ),
        ] {
            let package = registry.join(format!("{group}/{name}/v1.0.0"));
            std::fs::create_dir_all(&package).unwrap();
            std::fs::write(package.join("vibe.toml"), format!("[package]\ngroup='{group}'\nname='{name}'\nkind='flow'\nversion='1.0.0'\n{requires}")).unwrap();
        }
        let app = registry.join(format!("{group}/app/v1.0.0"));
        std::fs::write(app.join("vibe.toml"), format!("[package]\ngroup='{group}'\nname='app'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'{group}/shared'='*'\n")).unwrap();
        let app_manifest = Manifest::read(app.join("vibe.toml")).unwrap();
        let dependency = DependencyDeclarationId::new(format!("{group}/shared")).unwrap();
        let configuration = f.request.configuration.as_mut().unwrap();
        configuration.definitions[0].identity =
            vibe_registry::CandidateMetadata::read_local_registry(&app)
                .unwrap()
                .identity;
        configuration.definitions[0].definition.declarations = FeatureDeclarations::from_requires(
            &app_manifest.requires,
            &[dependency.clone()].into(),
        )
        .unwrap();
        configuration.definitions[0]
            .definition
            .program
            .features
            .insert(
                fixture::feature("a"),
                [FeatureEffect::ActivateDependency(dependency)].into(),
            );
        f.request
            .roots
            .push(format!("{group}/other").parse().unwrap());
        std::fs::write(f.root.join("vibe.toml"), format!("[project]\nname='switch'\nversion='1.0.0'\n[requires.packages]\n'{group}/app'='*'\n'{group}/other'='*'\n")).unwrap();
        f.request.roots = Manifest::read(f.root.join("vibe.toml"))
            .unwrap()
            .requires
            .packages;
        let (before, _) = f.installed();
        assert!(
            before
                .find(&group, "app")
                .unwrap()
                .dependencies
                .iter()
                .any(|dep| dep.name == "shared")
        );
        let shared = before.find(&group, "shared").unwrap();
        let slot = vibedeps::slot_abs_path(&f.root, &group, "shared", &shared.version);
        let record_before = std::fs::read(slot.join(vibedeps::SLOT_RECORD_FILENAME)).unwrap();
        f.request.configuration.as_mut().unwrap().request.edges[0]
            .selections
            .get_mut("choice")
            .unwrap()
            .members = Some(vec!["b".into()]);
        let source = f.source.bind_request(f.request.clone()).unwrap();
        let plan = vibe_install::evolution::plan_reconfiguration(
            &source,
            &f.root,
            f.install_request(),
            SpecFormat::Mixed,
            &vibe_install::NullObserver,
        )
        .unwrap();
        vibe_install::apply(
            &source,
            plan,
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            &vibe_workspace::hooks::HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap();
        let after = load_lockfile(&f.root).unwrap();
        assert!(after.find(&group, "app").unwrap().dependencies.is_empty());
        assert!(
            after
                .find(&group, "other")
                .unwrap()
                .dependencies
                .iter()
                .any(|dep| dep.name == "shared")
        );
        assert!(after.find(&group, "shared").is_some());
        assert_eq!(
            std::fs::read(slot.join(vibedeps::SLOT_RECORD_FILENAME)).unwrap(),
            record_before
        );
        let (requested, _) = vibe_wire::behaviour::lock_configuration::decode(
            after.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        assert!(requested.configuration.unwrap().contains("b"));
    }

    #[test]
    fn admitted_configured_removal_retains_authored_choices_and_shared_dependency() {
        use vibe_core::manifest::SpecFormat;
        let mut f = installed_shared_world_mode(true);
        let group = f.request.roots[0].group.clone().unwrap();
        let before = load_lockfile(&f.root).unwrap();
        let (requested_before, _) = vibe_wire::behaviour::lock_configuration::decode(
            before.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        f.request.roots.retain(|root| root.name != "other");
        let source = f.source.bind_request(f.request.clone()).unwrap();
        let workspace = Workspace::discover(&f.root).unwrap();
        let plan = vibe_install::removal::plan_admitted_removal(
            &source,
            &workspace,
            &f.root,
            &group,
            "other",
            f.install_request(),
            SpecFormat::Mixed,
            &vibe_install::NullObserver,
        )
        .unwrap();
        assert_eq!(plan.removed(), [format!("{group}/other")]);
        assert!(plan.explanation().contains(&format!("keep: [{group}/app]")));
        vibe_install::removal::apply_admitted_removal(
            &source,
            plan,
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            SpecFormat::Mixed,
            &vibe_workspace::hooks::HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap();
        let after = load_lockfile(&f.root).unwrap();
        assert_eq!(after.packages.len(), 1);
        assert_eq!(after.packages[0].name, "app");
        let (requested_after, _) = vibe_wire::behaviour::lock_configuration::decode(
            after.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        assert_eq!(
            requested_before.configuration,
            requested_after.configuration
        );
        assert!(
            vibedeps::slot_abs_path(&f.root, &group, "app", &after.packages[0].version)
                .join("payload.txt")
                .exists()
        );
    }

    #[test]
    fn ordinary_uninstall_faults_retain_real_writer_prefixes_and_guarded_recovery() {
        use vibe_workspace::configuration_transaction::{
            ConfigurationTransaction, RECOVERY_DIRECTORY, with_boundary_fault,
        };
        for phase in ["manifest", "lock", "slot", "boot"] {
            let f = installed_shared_world();
            let group = f.request.roots[0].group.as_ref().unwrap();
            let workspace = Workspace::discover(&f.root).unwrap();
            let lock = load_lockfile(&f.root).unwrap();
            let other = lock.find(group, "other").unwrap();
            let relative = PathBuf::from(vibedeps::slot_rel_path(group, "other", &other.version));
            let manifest_before = std::fs::read(f.root.join("vibe.toml")).unwrap();
            let lock_before = std::fs::read(f.root.join("vibe.lock")).unwrap();
            let context = crate::output::Context::from_flags(
                true,
                false,
                None,
                false,
                crate::cli::AgentModeArg::Cli,
            );
            let result = with_boundary_fault(&workspace.root, phase, || {
                run(
                    &context,
                    UninstallArgs {
                        package: format!("{group}/other"),
                        global: false,
                        agent: None,
                        path: f.root.clone(),
                        assume_yes: true,
                    },
                )
            });
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("injected interruption"),
                "{phase}"
            );
            assert_ne!(
                std::fs::read(f.root.join("vibe.toml")).unwrap(),
                manifest_before
            );
            assert_eq!(
                f.root.join(&relative).exists(),
                matches!(phase, "manifest" | "lock")
            );
            let operations = std::fs::read_dir(f.root.join(RECOVERY_DIRECTORY))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(operations.len(), 1);
            let operation = operations[0].file_name().into_string().unwrap();
            let mut tx =
                ConfigurationTransaction::reopen(&workspace, &[relative.clone()], &operation)
                    .unwrap();
            assert_eq!(tx.report()["phase"], phase);
            if phase == "boot" {
                assert!(
                    tx.restore_local_prefix()
                        .unwrap_err()
                        .to_string()
                        .contains("boot compensation")
                );
                assert!(tx.recovery_path().join("journal.json").is_file());
            } else {
                tx.restore_local_prefix().unwrap();
                assert_eq!(
                    std::fs::read(f.root.join("vibe.toml")).unwrap(),
                    manifest_before
                );
                assert_eq!(
                    std::fs::read(f.root.join("vibe.lock")).unwrap(),
                    lock_before
                );
                assert_eq!(
                    std::fs::read(f.root.join(&relative).join("a/b")).unwrap(),
                    b"owned nested file"
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn ordinary_uninstall_refuses_native_filename_aliases_without_writes() {
        use std::os::unix::ffi::OsStringExt;
        for filename in [
            std::ffi::OsString::from("a\\b"),
            std::ffi::OsString::from_vec(vec![0xff]),
        ] {
            let f = installed_shared_world();
            let group = f.request.roots[0].group.as_ref().unwrap();
            let lock = load_lockfile(&f.root).unwrap();
            let other = lock.find(group, "other").unwrap();
            let slot = vibedeps::slot_abs_path(&f.root, group, "other", &other.version);
            std::fs::write(slot.join(&filename), b"unrecorded user state").unwrap();
            let manifest_before = std::fs::read(f.root.join("vibe.toml")).unwrap();
            let lock_before = std::fs::read(f.root.join("vibe.lock")).unwrap();
            let context = crate::output::Context::from_flags(
                true,
                false,
                None,
                false,
                crate::cli::AgentModeArg::Cli,
            );
            let result = run(
                &context,
                UninstallArgs {
                    package: format!("{group}/other"),
                    global: false,
                    agent: None,
                    path: f.root.clone(),
                    assume_yes: true,
                },
            );
            assert!(result.is_err());
            assert_eq!(
                std::fs::read(slot.join(&filename)).unwrap(),
                b"unrecorded user state"
            );
            assert_eq!(
                std::fs::read(slot.join("a/b")).unwrap(),
                b"owned nested file"
            );
            assert_eq!(
                std::fs::read(f.root.join("vibe.toml")).unwrap(),
                manifest_before
            );
            assert_eq!(
                std::fs::read(f.root.join("vibe.lock")).unwrap(),
                lock_before
            );
        }
    }

    #[test]
    fn ordinary_uninstall_retains_a_root_still_required_by_another_root() {
        let f = installed_shared_world();
        let group = f.request.roots[0].group.as_ref().unwrap();
        let context = crate::output::Context::from_flags(
            true,
            false,
            None,
            false,
            crate::cli::AgentModeArg::Cli,
        );
        let workspace = Workspace::discover(&f.root).unwrap();
        let plan =
            vibe_install::removal::plan_removal(&workspace, &workspace.root, group, "app").unwrap();
        assert!(plan.affected_dependents.contains(&format!("{group}/other")));
        assert!(plan.removed().is_empty());
        run(
            &context,
            UninstallArgs {
                package: format!("{group}/app"),
                global: false,
                agent: None,
                path: f.root.clone(),
                assume_yes: true,
            },
        )
        .unwrap();
        let lock = load_lockfile(&f.root).unwrap();
        assert_eq!(lock.packages.len(), 2);
        assert!(
            load_project_manifest(&f.root)
                .unwrap()
                .requires
                .packages
                .iter()
                .all(|p| p.name != "app")
        );
        let app = lock.find(group, "app").unwrap();
        assert!(
            vibedeps::slot_abs_path(&f.root, group, "app", &app.version)
                .join("payload.txt")
                .is_file()
        );
    }

    #[test]
    fn ordinary_uninstall_removes_unused_root_and_preserves_shared_dependency() {
        let f = installed_shared_world();
        let group = f.request.roots[0].group.as_ref().unwrap();
        let context = crate::output::Context::from_flags(
            true,
            false,
            None,
            false,
            crate::cli::AgentModeArg::Cli,
        );
        run(
            &context,
            UninstallArgs {
                package: format!("{group}/other"),
                global: false,
                agent: None,
                path: f.root.clone(),
                assume_yes: true,
            },
        )
        .unwrap();
        let lock = load_lockfile(&f.root).unwrap();
        assert_eq!(lock.packages.len(), 1);
        assert_eq!(lock.packages[0].name, "app");
        assert!(
            vibedeps::slot_abs_path(&f.root, group, "app", &lock.packages[0].version)
                .join("payload.txt")
                .is_file()
        );
    }
}
