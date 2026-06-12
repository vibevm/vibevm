//! `vibe show purls` — PURL bindings recorded in the lockfile.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use anyhow::Result;
use serde::Serialize;

use crate::cli::ShowPurlsArgs;
use crate::output;

use super::resolve_project_root;

// ===================== show purls =====================

#[derive(Debug, Serialize)]
struct PurlEntry {
    package: String,
    purl: String,
}

#[derive(Debug, Serialize)]
struct PurlsReport {
    ok: bool,
    command: &'static str,
    project: String,
    bindings: Vec<PurlEntry>,
}

pub(super) fn run_purls(ctx: &output::Context, args: ShowPurlsArgs) -> Result<()> {
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
    let mut bindings: Vec<PurlEntry> = Vec::new();
    for p in &lockfile.packages {
        if let Some(purl) = &p.describes {
            bindings.push(PurlEntry {
                package: format!("{}/{}", p.group, p.name),
                purl: purl.clone(),
            });
        }
        for s in &p.subskills_active {
            if let Some(purl) = &s.describes {
                bindings.push(PurlEntry {
                    package: format!("{}/{}/{}", p.group, p.name, s.path),
                    purl: purl.clone(),
                });
            }
        }
    }
    if ctx.is_json() {
        ctx.emit_json(&PurlsReport {
            ok: true,
            command: "show:purls",
            project: project_root.display().to_string(),
            bindings,
        })?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe show purls: {} binding{}",
            bindings.len(),
            if bindings.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    if bindings.is_empty() {
        ctx.summary("(no PURL bindings in this project)");
        return Ok(());
    }
    for b in &bindings {
        ctx.step(&format!("{}  →  {}", b.package, b.purl));
    }
    ctx.summary(&format!(
        "\n{} PURL binding{}",
        bindings.len(),
        if bindings.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}
