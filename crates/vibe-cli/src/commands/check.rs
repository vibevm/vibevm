//! `vibe check` — run the spec-consistency linter against the project.
//!
//! Spec: `VIBEVM-SPEC.md` §12 (the linter), ROADMAP §M1.3.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_check::{CheckId, CheckOptions, CheckReport, Finding, Severity};
use vibe_core::manifest::Manifest;
use vibe_workspace::Workspace;
use vibe_workspace::install::verify_boot_graph;

use crate::cli::CheckArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: CheckArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let opts = CheckOptions {
        wal_max_age_hours: args.wal_max_age_hours,
        review_max_age_days: args.review_max_age_days,
        now_unix_utc: None,
    };
    let mut report = vibe_check::check_project(&project_root, &opts);
    // PROP-038 §3 — the boot-graph integrity check runs on the installed
    // workspace, which vibe-check (project-file only) does not load; append
    // its findings here where the vibe-workspace stack is available.
    append_boot_graph_findings(&project_root, &mut report);
    let errors = report.count(Severity::Error);
    let warnings = report.count(Severity::Warning);
    let infos = report.count(Severity::Info);

    emit_report(ctx, &project_root, &report, errors, warnings, infos)?;

    // Exit code per VIBEVM-SPEC.md §12: 0 if no errors, 1 if errors,
    // 0 with warnings displayed if only warnings.
    if errors > 0 {
        bail!(
            "vibe check: {errors} error{} ({warnings} warning{})",
            if errors == 1 { "" } else { "s" },
            if warnings == 1 { "" } else { "s" }
        );
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct CheckJsonReport<'a> {
    ok: bool,
    command: &'static str,
    project: String,
    summary: CheckJsonSummary,
    findings: Vec<CheckJsonFinding<'a>>,
}

#[derive(Debug, Serialize)]
struct CheckJsonSummary {
    error: usize,
    warning: usize,
    info: usize,
}

#[derive(Debug, Serialize)]
struct CheckJsonFinding<'a> {
    check: &'static str,
    severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    message: &'a str,
}

fn emit_report(
    ctx: &output::Context,
    project_root: &Path,
    report: &CheckReport,
    errors: usize,
    warnings: usize,
    infos: usize,
) -> Result<()> {
    if ctx.is_json() {
        let findings: Vec<CheckJsonFinding<'_>> = report
            .findings
            .iter()
            .map(|f| CheckJsonFinding {
                check: f.check.as_str(),
                severity: f.severity.as_str(),
                path: f
                    .path
                    .as_ref()
                    .map(|p| p.to_string_lossy().replace('\\', "/")),
                line: f.line,
                message: f.message.as_str(),
            })
            .collect();
        let payload = CheckJsonReport {
            ok: errors == 0,
            command: "check",
            project: project_root.display().to_string(),
            summary: CheckJsonSummary {
                error: errors,
                warning: warnings,
                info: infos,
            },
            findings,
        };
        ctx.emit_json(&payload)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe check: {errors} error{}, {warnings} warning{}, {infos} info",
            if errors == 1 { "" } else { "s" },
            if warnings == 1 { "" } else { "s" },
        ));
        return Ok(());
    }

    if report.findings.is_empty() {
        ctx.summary(&format!(
            "vibe check: clean — every check passed against `{}`",
            project_root.display()
        ));
        return Ok(());
    }

    ctx.heading(&format!(
        "vibe check: {} finding{} in `{}`",
        report.findings.len(),
        if report.findings.len() == 1 { "" } else { "s" },
        project_root.display()
    ));
    for finding in &report.findings {
        render_finding(finding);
    }
    ctx.summary(&format!(
        "\n{errors} error{}, {warnings} warning{}, {infos} info",
        if errors == 1 { "" } else { "s" },
        if warnings == 1 { "" } else { "s" },
    ));
    Ok(())
}

fn render_finding(f: &Finding) {
    let sigil = match f.severity {
        Severity::Error => "[E]",
        Severity::Warning => "[W]",
        Severity::Info => "[i]",
    };
    let path_part = match (&f.path, f.line) {
        (Some(p), Some(line)) => {
            format!("{}:{line}", p.to_string_lossy().replace('\\', "/"))
        }
        (Some(p), None) => p.to_string_lossy().replace('\\', "/"),
        (None, _) => "-".to_string(),
    };
    println!(
        "  {sigil}  [{check}] {path_part} — {msg}",
        check = f.check.as_str(),
        msg = f.message
    );
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first or pass `--path <dir>` pointing at a project root",
            stripped.display()
        );
    }
    Ok(stripped)
}

/// Append PROP-038 §3 boot-graph integrity findings: any per-unit boot
/// artifact whose recorded fingerprint is stale (the hybrid linker's
/// dirty-subgraph should have refreshed it). Best-effort — a project that is
/// not an installed workspace, or whose materialisation cannot be read,
/// contributes no findings.
fn append_boot_graph_findings(project_root: &Path, report: &mut CheckReport) {
    let Ok(ws) = Workspace::load(project_root) else {
        return;
    };
    let Ok(stale) = verify_boot_graph(&ws) else {
        return;
    };
    for (group, name) in stale {
        report.findings.push(Finding {
            check: CheckId::BootGraphIntegrity,
            severity: Severity::Warning,
            path: None,
            line: None,
            message: format!(
                "stale boot artifact for {group}/{name} — its fingerprint is out of date; run `vibe reinstall`"
            ),
        });
    }
}
