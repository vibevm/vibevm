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
use super::model::{InstallRecord, Origin};
use super::provenance::{self, RunningIdentity};
use super::source as source_tree;
use super::tools;
use super::{VvmEnv, bundle, confirm, make_persister, path_has_dir};

fn needs_build_tools(origin: Option<Origin>) -> bool {
    origin != Some(Origin::Binary)
}

struct InstanceLayout {
    kind: &'static str,
    vibe: bool,
    vibe_index: bool,
    source: bool,
    source_archive: bool,
    manifest: bool,
    manifest_root: bool,
    placement_manifest: bool,
    integrity: bool,
}

fn instance_layout(
    store: &super::store::VersionStore,
    record: &super::model::InstallRecord,
) -> InstanceLayout {
    let home = store.instance_dir(&record.version_id(), record.instance);
    let vibe = store
        .binary_path(&record.version_id(), record.instance)
        .is_file();
    let vibe_index = store
        .index_binary_path(&record.version_id(), record.instance)
        .is_file();
    let source = match record.origin {
        Origin::Binary => store
            .instance_source_dir(&record.version_id(), record.instance)
            .is_dir(),
        Origin::Managed => {
            let mirror = store.mirror_dir();
            mirror.join(".git").is_dir() && source_tree::find_source_root(&mirror).is_some()
        }
        Origin::External => record
            .source_path
            .as_deref()
            .and_then(|path| source_tree::find_source_root(std::path::Path::new(path)))
            .is_some(),
    };
    let source_archive = home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME).is_file();
    let manifest = home.join("DISTRIBUTION.json").is_file();
    let manifest_root = record.distribution_manifest_sha256.is_some();
    let placement_manifest = home.join(".vvm-manifest.toml").is_file();
    let modern = record.origin == Origin::Binary
        && (record.source_path.is_some() || vibe_index || source || source_archive || manifest);
    let (kind, integrity) = if record.origin == Origin::Binary && modern {
        (
            if manifest_root {
                "bundle"
            } else {
                "bundle-unrooted"
            },
            bundle::installed_bundle_intact(store, record),
        )
    } else if record.origin == Origin::Binary {
        (
            "legacy-single-binary",
            super::placer::installed_files_match(store, record),
        )
    } else {
        (
            "source-build",
            super::placer::installed_files_match(store, record),
        )
    };
    InstanceLayout {
        kind,
        vibe,
        vibe_index,
        source,
        source_archive,
        manifest,
        manifest_root,
        placement_manifest,
        integrity,
    }
}

fn same_record(left: &InstallRecord, right: &InstallRecord) -> bool {
    left.version_id() == right.version_id() && left.instance == right.instance
}

fn diagnostic_records(
    active: Option<&InstallRecord>,
    running: Option<&InstallRecord>,
) -> Vec<(&'static str, InstallRecord)> {
    let mut records = Vec::new();
    if let Some(active) = active {
        records.push(("active", active.clone()));
    }
    if let Some(running) = running
        && active.is_none_or(|active| !same_record(active, running))
    {
        records.push(("running", running.clone()));
    }
    records
}

/// `vibe self doctor`: probe the required toolchain, the shim dir on PATH,
/// the active version's binary, and the embedded registry (PROP-030).
/// Problems block.
pub(super) fn run_doctor_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmDoctorArgs,
) -> anyhow::Result<()> {
    let store = env.store()?;
    let tools = tools::check_all();
    let shim_dir = store.shim_dir();
    let on_path = path_has_dir(env.path_var.as_deref(), &shim_dir);
    let active = store.active()?;
    let running_identity = provenance::running_identity(&store)?;
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
    let instances = diagnosed
        .iter()
        .map(|(role, record)| (*role, record.clone(), instance_layout(&store, record)))
        .collect::<Vec<_>>();
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
    let mut remaining_problems = problems;
    let mut fixed = false;

    if args.fix && confirm(ctx, args.yes, "Write shims and put the shim dir on PATH?")? {
        let _lock = super::install::InstallLock::acquire(&store)?;
        env::write_shims(&store)?;
        let shell = env::Shell::detect(env.shell.as_deref());
        make_persister(env, shell)?.ensure_on_path(&shim_dir)?;
        remaining_problems =
            remaining_problems.saturating_sub(usize::from(!on_path) + shim_problems);
        fixed = true;
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

    if fixed {
        ctx.summary("fixed: shims written, shim dir ensured on PATH (open a new shell).");
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
