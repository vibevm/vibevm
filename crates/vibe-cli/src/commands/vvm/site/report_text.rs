//! Human steps, next actions and trailer text of the site profile report.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use vibe_wire::generated::self_apply_report::{
    ApplyAction, FileChangeKind, PathFixOutcome, PathPlan, PathReport, PxOutcome, PxStep,
    SelfApplyReport, WhereToPut,
};

use crate::commands::vvm::bundle::release_source::kind_label;
use crate::output::language::{Block, Cell, Glyph, List, Meta, Role, Row, hint, paint, rule_at};

/// Every human report section, leaving the command's trailer to its context.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
pub(super) fn render(report: &SelfApplyReport, width: usize, rule: usize) -> String {
    let mut head = Vec::new();
    let headline = headline_text(report);
    if trailer_what(report).is_none() && !headline.is_empty() {
        let glyph = if report.action == ApplyAction::Show && report.profile.is_some() {
            Glyph::Done
        } else {
            Glyph::Skipped
        };
        let meta = report.profile.as_ref().map_or_else(Vec::new, |profile| {
            vec![
                Meta::note(format!(
                    "sha256 {}",
                    profile.sha256.get(..12).unwrap_or(&profile.sha256)
                )),
                Meta::note(&profile.source),
            ]
        });
        head.push(Block::new(glyph, headline).with_meta(meta).render_at(width));
    }
    let mut rows = Vec::new();
    for file in &report.files {
        let word = match file.change {
            FileChangeKind::Create => "create",
            FileChangeKind::Update => "update",
            FileChangeKind::Unchanged => "unchanged",
            FileChangeKind::Delete => "delete",
        };
        let skipped =
            report.action == ApplyAction::DryRun || file.change == FileChangeKind::Unchanged;
        rows.push(step_row(word, file.path.clone(), skipped, vec![]));
    }
    if let Some(imported) = &report.imported {
        rows.push(step_row(
            "import",
            format!("the running vibe as {}", imported.selector),
            imported.planned,
            vec![Meta::note(&imported.executable)],
        ));
    }
    if let Some(px) = &report.px
        && let Some(row) = px_row(px)
    {
        rows.push(row);
    }
    if let Some(path) = &report.path
        && let Some(row) = path_row(path)
    {
        rows.push(row);
    }
    let list = List::new(rows).render_at(width);
    for (index, line) in list.lines().enumerate() {
        head.push(line.to_owned());
        if let Some(file) = report.files.get(index)
            && let Some(diff) = &file.diff
        {
            head.extend(diff.lines().map(|line| format!("    {line}")));
        }
    }
    if let Some(path) = &report.path {
        for text in [&path.activation_hint, &path.fix_hint]
            .into_iter()
            .flatten()
        {
            head.push(format!("  {}", hint(text)));
        }
    }
    let mut parts = vec![head.join("\n")];
    if let Some(effective) = &report.effective {
        parts.push(super::report::effective::sections(effective, width, rule));
    }
    if let Some(where_to_put) = &report.where_
        && (!where_to_put.releases.is_empty() || where_to_put.packages.is_some())
    {
        parts.push(format!(
            "{}\n\n{}",
            rule_at("where releases and payloads go", None, rule),
            where_text(where_to_put),
        ));
    }
    let mut next = Vec::new();
    for step in &report.next_steps {
        if let Some((text, command)) = step.split_once('`') {
            next.push(format!("  {}", hint(text.trim_end_matches([':', ' ']))));
            next.push(format!(
                "  {}",
                paint(Role::Accent, command.split('`').next().unwrap_or(command))
            ));
        } else {
            next.push(format!("  {}", hint(step)));
        }
    }
    parts.push(next.join("\n"));
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The work summary whose elapsed time and retained path the context supplies.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
pub(super) fn trailer_what(report: &SelfApplyReport) -> Option<String> {
    let profile = report.profile.as_ref()?;
    let name = profile
        .name
        .as_ref()
        .map(|name| format!(" {name}"))
        .unwrap_or_default();
    match report.action {
        ApplyAction::Apply if report.changed => Some(format!("applied site profile{name}")),
        ApplyAction::Apply => Some(format!(
            "site profile{name} already applied · nothing changed"
        )),
        ApplyAction::Remove => Some(format!("removed site profile{name}")),
        _ => None,
    }
}

/// The plain report headline, also the single quiet line when no work ran.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
pub(super) fn headline_text(report: &SelfApplyReport) -> String {
    if let Some(what) = trailer_what(report) {
        return what;
    }
    match (&report.action, &report.profile) {
        (ApplyAction::DryRun | ApplyAction::Show, Some(profile)) => {
            let name = profile
                .name
                .as_ref()
                .map(|name| format!(" {name}"))
                .unwrap_or_default();
            let verb = if report.action == ApplyAction::DryRun {
                "would be applied"
            } else {
                "in effect"
            };
            format!("site profile{name} {verb}")
        }
        (ApplyAction::Show | ApplyAction::Remove, None) if !report.changed => {
            "no site profile is applied".to_owned()
        }
        _ => String::new(),
    }
}

/// A measured successful or skipped step, with inline metadata.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn step_row(word: &str, text: String, skipped: bool, meta: Vec<Meta>) -> Row {
    let (glyph, role) = if skipped {
        (Glyph::Skipped, Role::Dim)
    } else {
        (Glyph::Done, Role::Ok)
    };
    Row::new(vec![Cell::text(word), Cell::text(text)])
        .glyph(glyph, role)
        .dim(skipped)
        .meta(meta)
}

/// The companion observation, leaving any repair sentence to the warnings.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn px_row(px: &PxStep) -> Option<Row> {
    let (text, skipped, meta) = match (&px.outcome, px.planned) {
        (PxOutcome::NotNeeded, _) => return None,
        (PxOutcome::AlreadyInstalled, _) => (
            "already installed",
            false,
            px.program.as_ref().map(Meta::note).into_iter().collect(),
        ),
        (PxOutcome::Installed, false) => (
            "installed",
            false,
            px.program.as_ref().map(Meta::note).into_iter().collect(),
        ),
        (PxOutcome::Installed, true) => ("would be installed", true, vec![Meta::note(&px.store)]),
        (PxOutcome::Missing | PxOutcome::NoTarget, planned) => {
            let meta = if planned {
                vec![Meta::note("and would not be")]
            } else {
                vec![]
            };
            return Some(
                Row::new(vec![Cell::text("px"), Cell::text("not installed")])
                    .glyph(Glyph::Failed, Role::Alert)
                    .meta(meta),
            );
        }
    };
    Some(step_row("px", text.to_owned(), skipped, meta))
}

/// One performed or planned PATH step, without changing its consent policy.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn path_row(path: &PathReport) -> Option<Row> {
    let dir = &path.shim_dir;
    let (text, skipped, meta) = if let Some(outcome) = &path.outcome {
        match outcome {
            PathFixOutcome::AlreadyOnPath => (format!("{dir} already on it"), false, vec![]),
            PathFixOutcome::Applied => (format!("put {dir} on it"), false, vec![]),
            PathFixOutcome::Skipped => (
                "left as it is".to_owned(),
                true,
                vec![Meta::note(format!("{dir} not on it"))],
            ),
        }
    } else {
        let (text, skipped) = match path.planned.as_ref()? {
            PathPlan::AlreadyOnPath => (format!("{dir} already on it"), false),
            PathPlan::WouldApply => (format!("would put {dir} on it"), true),
            PathPlan::WouldAsk => (format!("would ask to put {dir} on it"), true),
            PathPlan::WouldSkip => (format!("would skip {dir}"), true),
        };
        (
            text,
            skipped,
            path.target.as_ref().map(Meta::note).into_iter().collect(),
        )
    };
    Some(step_row("PATH", text, skipped, meta))
}

/// The uncut release and payload destinations, preserving their full paths.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
fn where_text(where_to_put: &WhereToPut) -> String {
    let mut lines = Vec::new();
    if let Some(first) = where_to_put.releases.first() {
        lines.push(format!(
            "put vibe releases into {} ({}), one directory per release, holding exactly the release page's files:",
            first.location,
            kind_label(first.source.clone()),
        ));
        for versioned in &where_to_put.releases {
            if let Some(dir) = &versioned.example_dir {
                lines.push(format!("  {dir}/"));
                lines.extend(versioned.files.iter().map(|file| format!("    {file}")));
            }
        }
        lines.push(format!(
            "  {}",
            hint("(a bare vibevm-<version>-<target>.zip at the top is accepted too; vibe self releases lists what is there)")
        ));
    }
    if let Some(packages) = &where_to_put.packages {
        lines.push(format!(
            "put package payloads into {}; an input F (sha256 D) of package G/N@V is looked for at:",
            packages.dir,
        ));
        lines.extend(packages.layout.iter().map(|rung| format!("  {rung}")));
        if let Some(example) = packages.example.first() {
            lines.push(format!("  e.g. {example}"));
        }
        lines.push(format!(
            "  {}",
            hint("(vibe artifacts where <package> names the exact files a package needs)")
        ));
    }
    lines.join("\n").replace('`', "")
}

#[cfg(test)]
#[path = "report_text/tests.rs"]
mod tests;
