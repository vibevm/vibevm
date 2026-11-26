//! Human prune plans and display-only replay commands; never deletion policy.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE");

use std::path::Path;

use vibe_publish::git_publish::redact_credentials;

use super::Plan;
use crate::output;
use crate::output::language::{Cell, Glyph, List, Meta, Role, Row, heading_separator};

/// Original argument spellings, used only to explain how to replay this run.
pub(in crate::commands::registry) struct Retry<'a> {
    pub source: &'a str,
    pub into: &'a str,
    pub config: Option<&'a Path>,
    pub branch: Option<&'a str>,
    pub name: Option<&'a str>,
    pub payloads: bool,
    pub update_snapshots: bool,
    pub keep_history: bool,
    pub offline: bool,
    pub path: &'a Path,
}

impl Retry<'_> {
    /// A shell-safe command, with credentials removed before quoting.
    pub(super) fn command(&self, answer: &str, declaration: bool) -> String {
        let mut args = vec![
            "vibe".to_owned(),
            "registry".to_owned(),
            "mirror".to_owned(),
            quote(self.source),
            "--into".to_owned(),
            quote(self.into),
        ];
        if let Some(config) = self.config {
            args.extend(["--config".to_owned(), quote(&config.to_string_lossy())]);
        } else if declaration {
            args.extend(["--config".to_owned(), "mirror.toml".to_owned()]);
        }
        for (value, flag) in [(self.branch, "--branch"), (self.name, "--name")] {
            if let Some(value) = value {
                args.extend([flag.to_owned(), quote(value)]);
            }
        }
        if self.path != Path::new(".") {
            args.extend(["--path".to_owned(), quote(&self.path.to_string_lossy())]);
        }
        for (enabled, flag) in [
            (self.payloads, "--prune-payloads"),
            (self.update_snapshots, "--update-snapshots"),
            (self.keep_history, "--keep-history"),
            (self.offline, "--offline"),
        ] {
            if enabled {
                args.push(flag.to_owned());
            }
        }
        args.push("--prune".to_owned());
        if !answer.is_empty() {
            args.push(answer.to_owned());
        }
        args.join(" ")
    }
}

fn quote(value: &str) -> String {
    let value = redact_credentials(value);
    // ANSI-C quoting keeps control characters and the refusal parser's
    // backtick delimiters out of the displayed command, without evaluation.
    if value.chars().any(|ch| ch == '`' || ch.is_control()) {
        let escaped: String = value.bytes().map(|byte| format!("\\x{byte:02x}")).collect();
        return format!("$'{escaped}'");
    }
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "_./:@%+=,-".contains(ch))
    {
        value
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

/// Draw the whole measured plan, preserving its policy-defined row order.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn show(ctx: &output::Context, plan: &Plan) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    ctx.rule(&format!(
        "prune plan {} {} to remove",
        heading_separator(),
        plan.rows.len()
    ));
    let rows = plan
        .rows
        .iter()
        .map(|row| {
            let version = row
                .version
                .as_deref()
                .map(|version| format!("@{version}"))
                .unwrap_or_default();
            Row::new(vec![
                Cell::new(row.what.label(), Role::Dim),
                Cell::new(format!("{}{version}", row.package), Role::Accent),
            ])
            .glyph(Glyph::Skipped, Role::Dim)
            .meta(vec![
                Meta::note(row.reason.replace('`', "")),
                Meta::note(redact_credentials(&row.path)),
            ])
        })
        .collect();
    ctx.block(&List::new(rows).meta_column().render());
}

/// An explicit empty prune is still an answer, not a silent success.
pub(super) fn empty(ctx: &output::Context) {
    ctx.block(
        &List::new(vec![
            Row::new(vec![Cell::text(
                "recorded selection still matches the target",
            )])
            .glyph(Glyph::Skipped, Role::Dim),
        ])
        .render(),
    );
}
