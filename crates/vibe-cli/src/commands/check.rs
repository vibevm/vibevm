//! `vibe check` — run the spec-consistency linter against the project.
//!
//! Spec: `VIBEVM-SPEC.md` §12 (the linter), ROADMAP §M1.3.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_check::{CheckId, CheckOptions, CheckReport, Finding, Severity};
use vibe_core::machine_json_path;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_workspace::Workspace;
use vibe_workspace::install::verify_boot_graph_with_spec_format;

use crate::cli::CheckArgs;
use crate::output;
use crate::output::language::{
    Block, Glyph, LabelValue, Line, Meta, Role, join_meta, paint, short_path, warning,
};

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter")]
pub fn run(ctx: &output::Context, args: CheckArgs) -> Result<()> {
    let locate = ctx.progress().task("Locating project checks");
    let selected = if args.global {
        super::user::initialized_root()
    } else {
        resolve_project_root(&args.path)
    };
    let project_root = match selected {
        Ok(root) => {
            locate.finish();
            root
        }
        Err(error) => {
            locate.fail("project location failed");
            return Err(error);
        }
    };
    let opts = CheckOptions {
        wal_max_age_hours: args.wal_max_age_hours,
        review_max_age_days: args.review_max_age_days,
        now_unix_utc: None,
    };
    let scan = ctx.progress().task("Scanning project consistency");
    let mut report = vibe_check::check_project(&project_root, &opts);
    scan.finish();
    // PROP-038 §3 — the boot-graph integrity check runs on the installed
    // workspace, which vibe-check (project-file only) does not load; append
    // its findings here where the vibe-workspace stack is available.
    let boot = ctx.progress().task("Checking installed boot graph");
    append_boot_graph_findings(&project_root, &mut report);
    boot.finish();
    let errors = report.count(Severity::Error);
    let warnings = report.count(Severity::Warning);
    let infos = report.count(Severity::Info);

    emit_report(ctx, &project_root, &report, errors, warnings, infos)?;

    // Exit code per VIBEVM-SPEC.md §12: 0 if no errors, 1 if errors,
    // 0 with warnings displayed if only warnings.
    if errors > 0 {
        bail!("{}", counts_line(errors, warnings, infos));
    }
    Ok(())
}

/// `3 errors · 0 warnings · 0 info`: the counts a check run closes with.
fn counts_line(errors: usize, warnings: usize, infos: usize) -> String {
    format!(
        "{errors} error{} · {warnings} warning{} · {infos} info",
        if errors == 1 { "" } else { "s" },
        if warnings == 1 { "" } else { "s" },
    )
}

/// The closing of a run with no finding.
fn clean_line(project_root: &Path) -> String {
    Block::new(Glyph::Done, "clean")
        .with_meta(vec![
            Meta::note("every check passed"),
            Meta::note(short_path(&project_root.display().to_string())),
        ])
        .render()
}

/// Errors close with the refusal returned by `run`, not a second line.
fn closing_line(report: &CheckReport, project_root: &Path) -> Option<String> {
    let (errors, warnings, infos) = (
        report.count(Severity::Error),
        report.count(Severity::Warning),
        report.count(Severity::Info),
    );
    if errors > 0 {
        return None;
    }
    if report.findings.is_empty() {
        return Some(clean_line(project_root));
    }
    let counts = counts_line(errors, warnings, infos);
    Some(if warnings > 0 {
        warning(&counts)
    } else {
        Block::new(Glyph::Done, counts).render()
    })
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
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<usize>,
    /// The manifest field as a TOML path, so an editor can put the
    /// finding under that field without reading the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'a str>,
    /// The package the finding is about (`group/name@version`), so a
    /// reader can offer `vibe update` without decoding the slot path.
    #[serde(skip_serializing_if = "Option::is_none")]
    package: Option<&'a str>,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    why: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<&'a str>,
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
                path: f.path.as_ref().map(|p| machine_json_path(p)),
                line: f.line,
                column: f.column,
                field: f.field.as_deref(),
                package: f.package.as_deref(),
                message: f.message.as_str(),
                why: f.why.as_deref(),
                fix: f.fix.as_deref(),
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
        if let Some(line) = closing_line(report, project_root) {
            ctx.summary(&line);
        }
        return Ok(());
    }

    if report.findings.is_empty() {
        ctx.block(&clean_line(project_root));
        return Ok(());
    }

    ctx.rule(&format!(
        "{} finding{} in {}",
        report.findings.len(),
        if report.findings.len() == 1 { "" } else { "s" },
        project_root.display()
    ));
    let blocks: Vec<String> = report
        .findings
        .iter()
        .map(|finding| finding_block(finding).render())
        .collect();
    ctx.block(&blocks.join("\n"));
    if let Some(line) = closing_line(report, project_root) {
        ctx.block(&line);
    }
    Ok(())
}

/// One finding as a block of the findings list (PROP-067 `#block`,
/// `#list`): the severity in the gutter, the check id in the accent role,
/// the message, and where it was found as the `at` line under it; the
/// manifest field, when the finding names one, follows the path as dim
/// metadata (`#meta`).
///
/// ```text
/// x redirect-block  CLAUDE.md has an opening <vibevm> marker and no closing one
///   at   CLAUDE.md:3
/// x manifest_validity  invalid manifest: [[installer_options.steps]] `fetch`.sha256 …
///   at   vibe.toml:12 · installer_options[1].steps[0].sha256
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
fn finding_block(f: &Finding) -> Block {
    let glyph = match f.severity {
        Severity::Error => Glyph::Failed,
        Severity::Warning => Glyph::Warning,
        Severity::Info => Glyph::Done,
    };
    let message = f.message.replace('`', "");
    let message = if f.severity == Severity::Warning {
        paint(Role::Warn, &message)
    } else {
        message
    };
    let headline = format!("{}  {message}", paint(Role::Accent, f.check.as_str()));
    let mut body = Vec::new();
    if let Some(why) = &f.why {
        body.push(Line::Label(
            "why".into(),
            vec![LabelValue::Text(why.clone())],
        ));
    }
    if let Some(fix) = &f.fix {
        body.push(Line::Label(
            "fix".into(),
            vec![LabelValue::Command(fix.clone())],
        ));
    }
    let mut at = match (&f.path, f.line) {
        (Some(p), Some(line)) => Some(format!("{}:{line}", p.to_string_lossy().replace('\\', "/"))),
        (Some(p), None) => Some(p.to_string_lossy().replace('\\', "/")),
        (None, _) => None,
    };
    if let Some(field) = &f.field {
        at = Some(match at {
            Some(at) => format!("{at}{}", join_meta(&[Meta::note(field.as_str())])),
            None => paint(Role::Dim, field),
        });
    }
    if let Some(at) = at {
        body.push(Line::Label("at".into(), vec![LabelValue::Text(at)]));
    }
    Block::new(glyph, headline).with_body(body)
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

/// Report stale units and applicable verification failures without mutating state.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-BOOT-CHECK",
    r = 1
)]
fn append_boot_graph_findings(project_root: &Path, report: &mut CheckReport) {
    // Only genuine absence makes this adapter non-applicable. In particular,
    // do not load operator configuration for an uninstalled project.
    let lock = project_root.join(Lockfile::FILENAME);
    if matches!(std::fs::symlink_metadata(&lock), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    {
        return;
    }
    let result = (|| -> Result<_> {
        let ws = Workspace::load(project_root).context("loading installed workspace")?;
        let format = super::reinstall::effective_rederive_format(&ws.root_manifest)?;
        Ok(verify_boot_graph_with_spec_format(&ws, format)?)
    })();
    let stale = match result {
        Ok(stale) => stale,
        Err(error) => {
            report.findings.push(Finding {
                why: None,
                fix: None,
                check: CheckId::BootGraphIntegrity,
                severity: Severity::Error,
                path: None,
                line: None,
                column: None,
                field: None,
                package: None,
                message: format!("installed boot graph verification could not complete: {error:#}"),
            });
            return;
        }
    };
    for (group, name) in stale {
        report.findings.push(Finding {
            why: None,
            fix: None,
            check: CheckId::BootGraphIntegrity,
            severity: Severity::Warning,
            path: None,
            line: None,
            column: None,
            field: None,
            package: None,
            message: format!(
                "stale boot artifact for {group}/{name} — its fingerprint is out of date; run `vibe reinstall`"
            ),
        });
    }
}

#[cfg(test)]
#[path = "check/tests.rs"]
mod tests;
