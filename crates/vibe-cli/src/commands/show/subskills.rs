//! `vibe show subskills` — active subskill entries per locked package.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use anyhow::Result;
use serde::Serialize;

use crate::cli::ShowSubskillsArgs;
use crate::output;

use super::resolve_project_root;

// ===================== show subskills =====================

#[derive(Debug, Serialize)]
struct SubskillEntry {
    path: String,
    delivery: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    describes: Option<String>,
}

#[derive(Debug, Serialize)]
struct SubskillsPackageEntry {
    package: String,
    subskills: Vec<SubskillEntry>,
}

#[derive(Debug, Serialize)]
struct SubskillsReport {
    ok: bool,
    command: &'static str,
    project: String,
    packages: Vec<SubskillsPackageEntry>,
    total: usize,
}

pub(super) fn run_subskills(ctx: &output::Context, args: ShowSubskillsArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let lockfile_path = project_root.join(vibe_core::manifest::Lockfile::FILENAME);
    let lockfile = if lockfile_path.exists() {
        vibe_core::manifest::Lockfile::read(&lockfile_path)?
    } else {
        vibe_core::manifest::Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            crate::commands::init::current_timestamp_utc(),
        )
    };

    let mut entries: Vec<SubskillsPackageEntry> = Vec::new();
    let mut total = 0usize;
    for p in &lockfile.packages {
        if p.subskills_active.is_empty() {
            continue;
        }
        total += p.subskills_active.len();
        entries.push(SubskillsPackageEntry {
            package: format!("{}/{}", p.group, p.name),
            subskills: p
                .subskills_active
                .iter()
                .map(|s| SubskillEntry {
                    path: s.path.clone(),
                    delivery: s.delivery.clone(),
                    describes: s.describes.clone(),
                })
                .collect(),
        });
    }

    if ctx.is_json() {
        ctx.emit_json(&SubskillsReport {
            ok: true,
            command: "show:subskills",
            project: project_root.display().to_string(),
            packages: entries,
            total,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show subskills: {} active subskill{} across {} package{}",
            total,
            if total == 1 { "" } else { "s" },
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    if entries.is_empty() {
        ctx.summary("(no subskills active in this project)");
        return Ok(());
    }
    for e in &entries {
        ctx.heading(&e.package);
        for s in &e.subskills {
            let mut line = format!("{} ({})", s.path, s.delivery);
            if let Some(d) = &s.describes {
                line.push_str(&format!("  describes: {d}"));
            }
            ctx.step(&line);
        }
    }
    ctx.summary(&format!(
        "\n{} active subskill{} across {} package{}",
        total,
        if total == 1 { "" } else { "s" },
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}
