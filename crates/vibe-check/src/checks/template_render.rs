//! The template check (PROP-075 §2): a template whose **skeleton** render
//! does not validate as an ordinary manifest is a broken template, and the
//! author should hear it from `vibe check` rather than from a user's
//! `vibe install`.
//!
//! The check is offline by construction, which is what makes it a linter
//! rule: a `kind = "json"` version source's document lives on the network,
//! so the check renders a **probe** version — the newest declared item of
//! a `kind = "list"` source, the declared `min` of a `json` source, or
//! `1.0.0` when neither says otherwise. That exercises the whole variable
//! grammar, the digest-field scoping and the validity of the result. The
//! one thing a probe cannot answer is a `{meta.…}` variable, whose value
//! only the real version record carries; such a template is reported as
//! *unchecked* (a warning), never as broken.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-TWO-STAGE");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{Manifest, VersionSource};
use vibe_template::{RenderErrorKind, VersionRecord, skeleton, template_section};

use crate::{Check, CheckId, CheckOptions, CheckReport, Finding, Severity};

/// The version a probe render uses when the declaration names no bound.
const PROBE_VERSION: &str = "1.0.0";

/// The [`CheckId::TemplateRender`] cell.
#[cell(seam = "Check", variant = "template-render")]
pub struct TemplateRenderCheck;

impl Check for TemplateRenderCheck {
    fn id(&self) -> CheckId {
        CheckId::TemplateRender
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for (root, label) in super::scan_local_packages(project_root) {
            let path = root.join(Manifest::FILENAME);
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            if !declares_a_template(&text) {
                continue;
            }
            let relative = path
                .strip_prefix(project_root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| PathBuf::from(Manifest::FILENAME));
            check_one(&text, &relative, &label, report);
        }
    }
}

/// Whether the document carries a `[template]` table at all. Read from the
/// raw TOML rather than from a typed [`Manifest`], because a template whose
/// numeric field varies holds a string where the manifest wants a number
/// (PROP-075 `#TEMPLATE-DOCUMENT-READING`).
fn declares_a_template(text: &str) -> bool {
    toml::from_str::<toml::Value>(text)
        .ok()
        .and_then(|value| value.get("template").cloned())
        .is_some()
}

fn check_one(text: &str, path: &Path, label: &str, report: &mut CheckReport) {
    let section = match template_section(text) {
        Ok(section) => section,
        Err(error) => {
            report.err(
                CheckId::TemplateRender,
                Some(path.to_path_buf()),
                None,
                format!("{label}: {error}"),
            );
            return;
        }
    };
    let probe = match &section.versions {
        VersionSource::List { items } => items
            .iter()
            .filter_map(|item| item.version.parse::<semver::Version>().ok())
            .max()
            .map(|version| version.to_string())
            .unwrap_or_else(|| PROBE_VERSION.to_string()),
        VersionSource::Json { min, .. } => min
            .as_ref()
            .map(semver::Version::to_string)
            .unwrap_or_else(|| PROBE_VERSION.to_string()),
    };
    let record = match VersionRecord::plain(&probe) {
        Ok(record) => record,
        Err(error) => {
            report.err(
                CheckId::TemplateRender,
                Some(path.to_path_buf()),
                None,
                format!("{label}: the probe version `{probe}` is unusable: {error}"),
            );
            return;
        }
    };
    let rendered = match skeleton(text, &record) {
        Ok(rendered) => rendered,
        Err(error) if needs_a_version_record(&error) => {
            report.warn(
                CheckId::TemplateRender,
                Some(path.to_path_buf()),
                None,
                format!(
                    "{label}: the skeleton render was not checked — `{}` needs a value only the \
                     version document carries ({}). Run `vibe app versions` against the real \
                     source to exercise it.",
                    error.at, error.message
                ),
            );
            return;
        }
        Err(error) => {
            refuse(report, path, label, &probe, error.at, error.message);
            return;
        }
    };
    if let Err(error) = Manifest::parse_str(&rendered.toml) {
        let field = error
            .manifest_field()
            .map(|field| field.to_string())
            .unwrap_or_default();
        refuse(report, path, label, &probe, field, error.to_string());
    }
}

/// A grammar refusal a probe render cannot avoid: `{meta.…}` is answered by
/// the version record, and the check has none.
fn needs_a_version_record(error: &vibe_template::RenderError) -> bool {
    error.kind == RenderErrorKind::Grammar && error.message.contains("{meta.")
}

fn refuse(
    report: &mut CheckReport,
    path: &Path,
    label: &str,
    probe: &str,
    field: String,
    message: String,
) {
    report.push(Finding {
        check: CheckId::TemplateRender,
        severity: Severity::Error,
        path: Some(path.to_path_buf()),
        line: None,
        column: None,
        field: (!field.is_empty()).then_some(field),
        package: None,
        message: format!(
            "{label}: the skeleton render of `{probe}` is not a valid manifest — {message}"
        ),
    });
}

#[cfg(test)]
#[path = "template_render/tests.rs"]
mod tests;
