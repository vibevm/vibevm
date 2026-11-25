//! `vibe self doctor` — the environment health check (PROP-019 §2.8), split out
//! of the `vvm` hub along the command-handler seam to keep the hub under the
//! 600-line file budget.
//!
//! The terminal apps (vibeterm, vibeframe) and the GUI launchers used to be
//! packaged into the instance alongside `vibe`; they have moved to a separate
//! products repo and now publish themselves to `PATH`. The doctor no longer
//! probes for them — `vibe term` / `vibe frame` resolve through `PATH` (with
//! an in-place fallback for `vibe tree`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use crate::cli::VvmDoctorArgs;
use crate::output;
use vibe_publish::release_manifest::DISTRIBUTION_SOURCE_ARCHIVE_FILENAME;

use super::embedded;
use super::env;
use super::error::VvmError;
use super::model::Origin;
use super::provenance::{self, RunningIdentity};
use super::tools;
#[path = "doctor_network.rs"]
mod network;
use super::{VvmEnv, make_persister, path_has_dir};
#[path = "doctor_environment.rs"]
mod environment;
#[path = "doctor_layout.rs"]
mod layout;
#[path = "doctor_paths.rs"]
mod paths;
#[path = "doctor_view.rs"]
mod view;
use crate::output::language::{rule_width, viewport};
use layout::{diagnostic_records, instance_layout, same_record};
use view::{DoctorRow, Mark};

fn needs_build_tools(origin: Option<Origin>) -> bool {
    origin != Some(Origin::Binary)
}

/// `vibe self doctor`: probe the required toolchain, the shim dir on PATH,
/// the active version's binary, and the embedded registry (PROP-030).
/// Problems block.
pub(super) fn run_doctor_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmDoctorArgs,
) -> anyhow::Result<()> {
    let progress = ctx.progress();
    let checks = progress.task("Checking self installation");
    let store = match env.store() {
        Ok(store) => store,
        Err(error) => {
            checks.fail(error.to_string());
            return Err(error.into());
        }
    };
    checks.detail(format!(
        "store: {}",
        env.root.as_deref().map_or_else(
            || "unavailable".to_string(),
            |path| path.display().to_string()
        )
    ));
    let tool_checks = checks.progress().task("Checking build tools");
    let tools = tools::check_all();
    tool_checks.set_progress(tools.len() as u64, Some(tools.len() as u64), "tools");
    tool_checks.finish();
    let shim_dir = store.shim_dir();
    let on_path = path_has_dir(env.path_var.as_deref(), &shim_dir);
    let mut durable_environment = environment::observe(env, &shim_dir);
    // The whole payload directories the user's applications expose on PATH
    // are a second PATH surface, reported exactly like the shim directory
    // (PROP-059 `#path-directories`).
    let link_dirs = paths::observe(env.path_var.as_deref());
    let probe_login = environment::login_probe_consent(ctx, args.probe_login_shell);
    if probe_login {
        environment::probe_login_shell(env, &shim_dir, &mut durable_environment);
    }
    let active = match store.active() {
        Ok(active) => active,
        Err(error) => {
            checks.fail(error.to_string());
            return Err(error.into());
        }
    };
    let running_identity = match provenance::running_identity(&store) {
        Ok(running) => running,
        Err(error) => {
            checks.fail(error.to_string());
            return Err(error);
        }
    };
    let running = match &running_identity {
        Some(RunningIdentity::Installed(record)) => Some(record),
        _ => None,
    };
    let diagnosed = diagnostic_records(active.as_ref(), running);
    let current = match &running_identity {
        Some(RunningIdentity::Installed(record)) => Some(record),
        Some(RunningIdentity::Source(_)) => None,
        None => active.as_ref(),
    };
    let build_tools_required = needs_build_tools(current.map(|record| record.origin));
    let embedded_registry = current.and_then(embedded::embedded_root_for);
    let active_missing = active
        .as_ref()
        .map(|r| !store.binary_path(&r.version_id(), r.instance).is_file())
        .unwrap_or(false);
    let instance_checks = checks
        .progress()
        .task("Checking installed instance integrity");
    instance_checks.set_progress(0, Some(diagnosed.len() as u64), "instances");
    let instances = diagnosed
        .iter()
        .enumerate()
        .map(|(index, (role, record))| {
            let component = instance_checks
                .progress()
                .task(format!("Checking {role} {}", record.selector()));
            let layout = instance_layout(&store, record);
            component.finish();
            instance_checks.set_progress(
                (index + 1) as u64,
                Some(diagnosed.len() as u64),
                "instances",
            );
            (*role, record.clone(), layout)
        })
        .collect::<Vec<_>>();
    if diagnosed.is_empty() {
        instance_checks.skip("no active or installed running instance");
    } else {
        instance_checks.finish();
    }
    let instance_problems = instances
        .iter()
        .map(|(_, record, layout)| {
            usize::from(!layout.integrity)
                + usize::from(record.origin != Origin::Binary && !layout.source)
        })
        .sum::<usize>();
    let shim_statuses = env::shim_statuses(&store);
    let shim_problems = shim_statuses.iter().filter(|(_, ok)| !ok).count();
    let running_missing = running
        .filter(|record| {
            active
                .as_ref()
                .is_none_or(|active| !same_record(active, record))
        })
        .is_some_and(|record| {
            !store
                .binary_path(&record.version_id(), record.instance)
                .is_file()
        });
    let problems = usize::from(build_tools_required) * tools.iter().filter(|t| !t.ok).count()
        + usize::from(!on_path)
        + usize::from(active_missing)
        + usize::from(running_missing)
        + instance_problems
        + shim_problems;
    let network = network::run(&checks.progress(), args.network.as_deref())?;
    let problems = problems + network.as_deref().map_or(0, network::failures);
    let mut remaining_problems = problems;
    let settings_profile = vibe_core::settings::site_profile_path()
        .map(|path| vibe_core::site_profile::installed_at(&path))
        .transpose()?
        .flatten();
    let mut settings_notes = Vec::new();
    if let Some(site) = &settings_profile {
        super::site::settings::step(
            &site.profile,
            env,
            super::site::settings::Action::Inspect,
            true,
            &mut settings_notes,
        )?;
    }
    let mut fixed = false;
    checks.detail(format!("problems found: {problems}"));
    checks.finish();

    if args.fix {
        let confirmed = ctx.suspend_progress(|| {
            environment::confirm_fix(ctx, args.yes, durable_environment.target.as_deref())
        })?;
        let repair = progress.task("Repairing self installation");
        if confirmed {
            let result = (|| -> anyhow::Result<()> {
                let _lock = super::install::InstallLock::acquire(&store)?;
                env::write_shims(&store)?;
                let shell = env::Shell::detect(env.shell.as_deref());
                let persister = make_persister(env, shell)?;
                // Consent came from `confirm_fix`; the shared core writes
                // only when the durable target lacks the shim directory.
                env::ensure_shim_dir_on_durable_path(
                    &*persister,
                    &shim_dir,
                    &env::PathFixConsent::granted(),
                    &mut |_, _| false,
                )?;
                if let Some(site) = &settings_profile {
                    super::site::settings::step(
                        &site.profile,
                        env,
                        super::site::settings::Action::Apply,
                        false,
                        &mut settings_notes,
                    )?;
                }
                Ok(())
            })();
            match result {
                Ok(()) => {
                    durable_environment = environment::observe(env, &shim_dir);
                    if probe_login {
                        environment::probe_login_shell(env, &shim_dir, &mut durable_environment);
                    }
                    remaining_problems = remaining_problems.saturating_sub(shim_problems);
                    fixed = true;
                    repair.finish();
                }
                Err(error) => {
                    repair.fail(error.to_string());
                    return Err(error);
                }
            }
        } else {
            repair.skip("declined");
        }
    }
    let final_shim_statuses = if fixed {
        env::shim_statuses(&store)
    } else {
        shim_statuses.clone()
    };

    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": remaining_problems == 0,
            "command": "self:doctor",
            "problems": remaining_problems,
            "initial_problems": problems,
            "fixed": fixed,
            "tools": tools.iter().map(|t| serde_json::json!({
                "name": t.name, "version": t.version, "ok": t.ok,
                "required": build_tools_required,
                "min": t.min_version, "help": t.help_url,
            })).collect::<Vec<_>>(),
            "shim_dir": shim_dir.display().to_string(),
            "shim_dir_on_path": on_path,
            "application_path_dirs": paths::json(&link_dirs),
            "process_path_refresh_needed": !on_path,
            "durable_environment": durable_environment,
            "settings_home": settings_notes,
            "shims": final_shim_statuses.iter().map(|(name, ok)| serde_json::json!({
                "name": name, "ok": ok,
            })).collect::<Vec<_>>(),
            "active": active.as_ref().map(|r| r.version_id().to_string()),
            "current": current.as_ref().map(|r| r.selector().to_string()),
            "build_tools_required": build_tools_required,
            "active_binary_ok": !active_missing,
            "installed_instances": instances.iter().map(|(role, record, layout)| serde_json::json!({
                "role": role,
                "selector": record.selector().to_string(),
                "origin": record.origin.as_str(),
                "kind": layout.kind,
                "vibe_ok": layout.vibe,
                "vibe_index_ok": layout.vibe_index,
                "source_ok": layout.source,
                "source_archive_ok": layout.source_archive,
                "manifest_ok": layout.manifest,
                "manifest_root_ok": layout.manifest_root,
                "placement_manifest_ok": layout.placement_manifest,
                "integrity_ok": layout.integrity,
                "upgrade": (layout.kind == "legacy-single-binary" || layout.kind == "bundle-unrooted").then_some("vibe self update --force"),
            })).collect::<Vec<_>>(),
            "embedded_registry": embedded_registry.as_ref().map(|p| p.display().to_string()),
            "network": network.as_deref().map(network::json),
        }))?;
        if remaining_problems > 0 {
            return Err(VvmError::DoctorProblems {
                problems: remaining_problems,
            }
            .into());
        }
        return Ok(());
    }

    let mut rows = Vec::new();
    rows.push(view::environment_row(&durable_environment));
    rows.extend(
        tools
            .iter()
            .map(|tool| view::tool_row(tool, build_tools_required)),
    );
    if build_tools_required {
        let (linker, url) = tools::linker_hint();
        rows.push(DoctorRow {
            mark: Mark::Note,
            name: "linker".into(),
            value: linker.into(),
            notes: vec![url.into()],
            states: Vec::new(),
            children: Vec::new(),
        });
    }
    rows.push(view::shim_dir_row(&shim_dir, on_path));
    rows.extend(
        final_shim_statuses
            .iter()
            .map(|(name, ok)| view::shim_row(name, *ok)),
    );
    rows.extend(paths::rows(&link_dirs));
    let (mark, value, notes) = match &active {
        Some(record) if !active_missing => (
            Mark::Ok,
            format!("{}#{}", record.version_id(), record.instance),
            Vec::new(),
        ),
        Some(record) => (
            Mark::Miss,
            record.version_id().to_string(),
            vec!["its binary is gone".into()],
        ),
        None => (Mark::Note, "none".into(), vec!["vibe self use <selector>".into()]),
    };
    rows.push(DoctorRow {
        mark,
        name: "active".into(),
        value,
        notes,
        states: Vec::new(),
        children: Vec::new(),
    });
    if let Some(record) = running
        && active
            .as_ref()
            .is_none_or(|active| !same_record(active, record))
    {
        let ok = store
            .binary_path(&record.version_id(), record.instance)
            .is_file();
        rows.push(DoctorRow {
            mark: if ok { Mark::Ok } else { Mark::Miss },
            name: "running".into(),
            value: record.selector().to_string(),
            notes: Vec::new(),
            states: if ok { Vec::new() } else { vec!["binary missing".into()] },
            children: Vec::new(),
        });
    }
    for (role, record, layout) in &instances {
        let (kind, flags) = if layout.kind.starts_with("bundle") {
            (
                "bundle",
                vec![
                    ("vibe", layout.vibe),
                    ("vibe-index", layout.vibe_index),
                    ("source", layout.source),
                    (DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, layout.source_archive),
                    ("DISTRIBUTION.json", layout.manifest),
                    ("authenticated manifest root", layout.manifest_root),
                    ("full integrity", layout.integrity),
                ],
            )
        } else if layout.kind == "source-build" {
            (
                "source-build",
                vec![
                    ("vibe", layout.vibe),
                    ("vibe-index", layout.vibe_index),
                    ("source", layout.source),
                    ("placement manifest", layout.placement_manifest),
                    ("full integrity", layout.integrity),
                ],
            )
        } else {
            ("legacy single-binary import", vec![("full integrity", layout.integrity)])
        };
        let missing = flags
            .iter()
            .filter_map(|(name, ok)| (!ok).then_some(*name))
            .collect::<Vec<_>>();
        let legacy = kind == "legacy single-binary import";
        let mut children = Vec::new();
        if !missing.is_empty() {
            children.push(format!("missing {}", missing.join(", ")));
        }
        if legacy {
            children.push("upgrade with vibe self update --force".into());
        }
        rows.push(DoctorRow {
            mark: if !missing.is_empty() {
                Mark::Miss
            } else if legacy {
                Mark::Note
            } else {
                Mark::Ok
            },
            name: format!("{role} instance"),
            value: record.selector().to_string(),
            notes: vec![kind.into()],
            states: vec![if missing.is_empty() { "complete" } else { "incomplete" }.into()],
            children,
        });
    }
    // PROP-030: the embedded registry the active source install exposes for
    // every project (its in-tree `packages/`).
    let (mark, value, notes) = match &embedded_registry {
        Some(root) => (
            Mark::Ok,
            root.display().to_string(),
            vec!["source install".into(), "embedded first".into()],
        ),
        None => (Mark::Note, "none".into(), vec!["not a source install".into()]),
    };
    rows.push(DoctorRow {
        mark,
        name: "embedded registry".into(),
        value,
        notes,
        states: Vec::new(),
        children: Vec::new(),
    });
    rows.extend(network.iter().flatten().map(network::row));
    ctx.block(&view::render(&rows, viewport(), rule_width()));
    for note in &settings_notes {
        ctx.hint(note);
    }
    if let Some(error) = &durable_environment.error {
        ctx.diagnostic(error);
    }

    if fixed {
        ctx.trailer("repair completed · shims written and durable PATH requested");
        if !on_path {
            ctx.hint("refresh the launcher/terminal environment and recheck PATH; a correct durable entry does not require repeated profile repair");
        }
    }

    if remaining_problems == 0 {
        ctx.summary("all good.");
        Ok(())
    } else {
        Err(VvmError::DoctorProblems {
            problems: remaining_problems,
        }
        .into())
    }
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
