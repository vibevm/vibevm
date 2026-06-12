//! `vibe show features` — active feature lines per locked package.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use anyhow::Result;
use serde::Serialize;

use crate::cli::ShowFeaturesArgs;
use crate::output;

use super::resolve_project_root;

// ===================== show features =====================

#[derive(Debug, Serialize)]
struct FeaturesEntry {
    package: String,
    features: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FeaturesReport {
    ok: bool,
    command: &'static str,
    project: String,
    packages: Vec<FeaturesEntry>,
    /// Total active feature lines, project-wide.
    total: usize,
}

pub(super) fn run_features(ctx: &output::Context, args: ShowFeaturesArgs) -> Result<()> {
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

    let mut entries: Vec<FeaturesEntry> = Vec::new();
    let mut total = 0usize;
    for p in &lockfile.packages {
        if p.features.is_empty() {
            continue;
        }
        total += p.features.len();
        entries.push(FeaturesEntry {
            package: format!("{}/{}", p.group, p.name),
            features: p.features.clone(),
        });
    }

    if ctx.is_json() {
        ctx.emit_json(&FeaturesReport {
            ok: true,
            command: "show:features",
            project: project_root.display().to_string(),
            packages: entries,
            total,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show features: {} active feature{} across {} package{}",
            total,
            if total == 1 { "" } else { "s" },
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    if entries.is_empty() {
        ctx.summary("(no features active in this project)");
        return Ok(());
    }
    for e in &entries {
        ctx.heading(&e.package);
        for f in &e.features {
            ctx.step(f);
        }
    }
    ctx.summary(&format!(
        "\n{} active feature{} across {} package{}",
        total,
        if total == 1 { "" } else { "s" },
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}
