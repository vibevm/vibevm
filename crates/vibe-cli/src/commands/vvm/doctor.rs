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
use layout::{diagnostic_records, instance_layout, same_record};

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

    ctx.heading("vibe self doctor");
    ctx.step(&format!("durable environment target: {} ({:?}); observation {:?}, managed block {:?}, PATH entry {:?}; login effectiveness unknown (not probed)",
        durable_environment.target.as_deref().unwrap_or("unavailable"), durable_environment.shell,
        durable_environment.observation, durable_environment.managed_block, durable_environment.path_entry));
    if let Some(error) = &durable_environment.error {
        ctx.diagnostic(error);
    }
    for t in &tools {
        match (&t.version, build_tools_required) {
            (Some(v), false) => ctx.step(&format!(
                "info {} {} (optional for binary instance)",
                t.name, v
            )),
            (None, false) => ctx.step(&format!(
                "skip {} not found (not required by binary instance)",
                t.name
            )),
            (Some(v), true) if t.ok => ctx.step(&format!("ok   {} {}", t.name, v)),
            (Some(v), true) => ctx.step(&format!(
                "MISS {} {} (need >= {}) — {}",
                t.name, v, t.min_version, t.help_url
            )),
            (None, true) => ctx.step(&format!("MISS {} not found — {}", t.name, t.help_url)),
        }
    }
    let (linker, lurl) = tools::linker_hint();
    ctx.step(&format!("also {linker} — {lurl}"));
    ctx.step(&format!(
        "{} shim dir {} {}",
        if on_path { "ok  " } else { "MISS" },
        shim_dir.display(),
        if on_path {
            "(on PATH)"
        } else {
            "(NOT on PATH)"
        }
    ));
    for (name, ok) in &final_shim_statuses {
        ctx.step(&format!(
            "{} stable {name} shim",
            if *ok { "ok  " } else { "MISS" }
        ));
    }
    for line in paths::lines(&link_dirs) {
        ctx.step(&line);
    }
    match &active {
        Some(r) if !active_missing => {
            ctx.step(&format!("ok   active {} #{}", r.version_id(), r.instance))
        }
        Some(r) => ctx.step(&format!(
            "MISS active {} — its binary is gone",
            r.version_id()
        )),
        None => ctx.step("-    no active version (set one with `vibe self use <selector>`)"),
    }
    if let Some(record) = running
        && active
            .as_ref()
            .is_none_or(|active| !same_record(active, record))
    {
        let ok = store
            .binary_path(&record.version_id(), record.instance)
            .is_file();
        ctx.step(&format!(
            "{} running {}",
            if ok { "ok  " } else { "MISS" },
            record.selector()
        ));
    }
    for (role, record, layout) in &instances {
        if layout.kind.starts_with("bundle") {
            for (name, ok) in [
                ("vibe", layout.vibe),
                ("vibe-index", layout.vibe_index),
                ("source", layout.source),
                (DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, layout.source_archive),
                ("DISTRIBUTION.json", layout.manifest),
                ("authenticated manifest root", layout.manifest_root),
                ("full integrity", layout.integrity),
            ] {
                ctx.step(&format!(
                    "{} {role} {} bundle {name}",
                    if ok { "ok  " } else { "MISS" },
                    record.selector()
                ));
            }
        } else if layout.kind == "source-build" {
            for (name, ok) in [
                ("vibe", layout.vibe),
                ("vibe-index", layout.vibe_index),
                ("source", layout.source),
                ("placement manifest", layout.placement_manifest),
                ("full integrity", layout.integrity),
            ] {
                ctx.step(&format!(
                    "{} {role} {} source-build {name}",
                    if ok { "ok  " } else { "MISS" },
                    record.selector()
                ));
            }
        } else {
            ctx.step(&format!(
                "{} {role} {} legacy single-binary import (partial); upgrade: `vibe self update --force`",
                if layout.integrity { "info" } else { "MISS" }, record.selector()
            ));
        }
    }
    // PROP-030: the embedded registry the active source install exposes for
    // every project (its in-tree `packages/`).
    match &embedded_registry {
        Some(root) => ctx.step(&format!(
            "ok   embedded registry {} (source install; precedence embedded-first)",
            root.display()
        )),
        None => ctx.step("-    no embedded registry (the active version is not a source install)"),
    }

    for check in network.iter().flatten() {
        ctx.step(&network::line(check));
    }

    if fixed {
        ctx.summary("repair completed: shims written and durable PATH requested; current process PATH is unchanged, login effectiveness was not probed.");
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
mod tests {
    use super::*;
    use crate::commands::vvm::model::{InstallRecord, Kind, Profile};
    use crate::commands::vvm::store::{BINARY_NAME, VersionStore};

    #[test]
    fn binary_instances_do_not_require_a_source_build_toolchain() {
        assert!(!needs_build_tools(Some(Origin::Binary)));
        assert!(needs_build_tools(Some(Origin::External)));
        assert!(needs_build_tools(Some(Origin::Managed)));
        assert!(needs_build_tools(None));
    }

    fn binary_record(source_path: Option<String>) -> InstallRecord {
        InstallRecord {
            kind: Kind::Tag,
            id: "1.0.0".into(),
            instance: 1,
            commit: "a".repeat(40),
            toolchain: "prebuilt".into(),
            profile: Profile::Release,
            installed_at: "now".into(),
            origin: Origin::Binary,
            source_path,
            payload_sha256: Some("b".repeat(64)),
            distribution_manifest_sha256: None,
        }
    }

    #[test]
    fn doctor_distinguishes_legacy_imports_from_complete_and_broken_bundles() {
        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path());
        let legacy = binary_record(None);
        let home = store.instance_dir(&legacy.version_id(), legacy.instance);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(BINARY_NAME), b"legacy").unwrap();
        let legacy_layout = instance_layout(&store, &legacy);
        assert_eq!(legacy_layout.kind, "legacy-single-binary");
        assert!(
            !legacy_layout.integrity,
            "missing placement manifest is corrupt"
        );

        let source = store.instance_source_dir(&legacy.version_id(), legacy.instance);
        let modern = binary_record(Some(source.display().to_string()));
        std::fs::create_dir_all(store.instance_bin_dir(&modern.version_id(), modern.instance))
            .unwrap();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            store.index_binary_path(&modern.version_id(), modern.instance),
            b"index",
        )
        .unwrap();
        std::fs::write(home.join("DISTRIBUTION.json"), b"manifest").unwrap();
        std::fs::write(
            home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME),
            b"source zip",
        )
        .unwrap();
        let complete = instance_layout(&store, &modern);
        assert!(
            complete.kind == "bundle-unrooted"
                && complete.vibe_index
                && complete.source
                && complete.source_archive
                && complete.manifest
        );
        assert!(
            !complete.integrity,
            "dummy manifest is not a verified bundle"
        );
        assert!(!complete.manifest_root);
        std::fs::remove_dir_all(&source).unwrap();
        let broken = instance_layout(&store, &modern);
        assert_eq!(broken.kind, "bundle-unrooted");
        assert!(!broken.source);

        let mut source_record = legacy.clone();
        source_record.origin = Origin::External;
        source_record.instance = 2;
        source_record.source_path = Some(temp.path().display().to_string());
        let source_home = store.instance_dir(&source_record.version_id(), 2);
        std::fs::create_dir_all(source_home.join("bin")).unwrap();
        std::fs::write(source_home.join("bin").join(BINARY_NAME), b"vibe").unwrap();
        let source_layout = instance_layout(&store, &source_record);
        assert_eq!(source_layout.kind, "source-build");
        assert!(
            !source_layout.source,
            "an arbitrary directory is not a source root"
        );
        assert!(!source_layout.vibe_index);
        assert!(!source_layout.placement_manifest);
        assert!(!source_layout.integrity);
    }

    #[test]
    fn active_and_distinct_running_instances_are_diagnosed_separately() {
        let mut running = binary_record(None);
        let mut active = running.clone();
        running.instance = 1;
        active.instance = 2;
        let rows = diagnostic_records(Some(&active), Some(&running));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "active");
        assert_eq!(rows[0].1.instance, 2);
        assert_eq!(rows[1].0, "running");
        assert_eq!(rows[1].1.instance, 1);
    }
}
