//! Human status rows and successful write trailers for explicit overrides.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#OVERRIDE-CONTROLS");

use crate::output::language::{Block, Glyph, Meta, short_path};
use vibe_wire::generated::user_override_report::UserOverrideReport;

/// The output selected by the final report, never a success for a refused write.
pub(super) enum Human<'a> {
    Status(String),
    Trailer { what: String, root: &'a str },
}

/// Decide human output from the final report, independently of early validation.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#OVERRIDE-CONTROLS")]
pub(super) fn human<'a>(
    report: &'a UserOverrideReport,
    action: &str,
    acted: bool,
) -> Option<Human<'a>> {
    if !report.ok {
        return None;
    }
    if action == "status" {
        return (report.state != "unavailable").then(|| {
            Human::Status(status_line(
                report.alias.as_deref(),
                report.root.as_deref(),
                report.enabled,
            ))
        });
    }
    Some(Human::Trailer {
        what: trailer_what(action, report.alias.as_deref().unwrap_or_default(), acted),
        root: report.root.as_deref().unwrap_or_default(),
    })
}

/// One status row, including the disabled mark and shortened root when bound.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#OVERRIDE-CONTROLS")]
pub(super) fn status_line(alias: Option<&str>, root: Option<&str>, enabled: bool) -> String {
    let Some(alias) = alias else {
        return Block::new(Glyph::Skipped, "override  unbound").render();
    };
    let mut meta = Vec::new();
    if let Some(root) = root {
        meta.push(Meta::note(short_path(root)));
    }
    if !enabled {
        meta.push(Meta::State("disabled".into()));
    }
    Block::new(Glyph::Done, format!("override {alias}  initialized"))
        .with_meta(meta)
        .render()
}

/// Name the action's actual change, not a schema-only mapping upgrade.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#OVERRIDE-CONTROLS")]
pub(super) fn trailer_what(action: &str, alias: &str, acted: bool) -> String {
    match (action, acted) {
        ("init", true) => format!("initialized override {alias}"),
        ("init", false) => format!("override {alias} already initialized"),
        ("bind", true) => format!("bound this project to override {alias}"),
        ("bind", false) => format!("this project is already bound to override {alias}"),
        ("enable", true) => format!("override {alias} enabled"),
        ("enable", false) => format!("override {alias} already enabled"),
        ("disable", true) => format!("override {alias} disabled"),
        _ => format!("override {alias} already disabled"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsuccessful_mutation_final_report_has_no_success_rendering() {
        for action in ["init", "bind", "enable", "disable"] {
            let report = UserOverrideReport {
                command: format!("user:override:{action}"),
                ok: false,
                state: "unavailable".into(),
                alias: Some("myov".into()),
                root: Some("/synthetic/project-overrides/id".into()),
                enabled: false,
            };
            for acted in [false, true] {
                assert!(human(&report, action, acted).is_none());
            }
        }
    }
}
