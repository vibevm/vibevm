//! The file half of `PROP-059#menu`, run by `vibe check`: every icon file a
//! package names exists in its tree, a PNG is square and matches the size
//! it is listed under, and a menu launcher has a scalable or large icon.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::fs;
use std::path::Path;

use specmark::spec;

use crate::manifest::FieldPath;

use super::*;

/// One icon defect of a package, with the package-relative file it names
/// when there is one and the manifest field that names it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#menu")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconFinding {
    pub path: Option<String>,
    pub field: FieldPath,
    pub message: String,
}

/// The address of one file of an icon: the `icon` field itself for the
/// short form, else the slot's key in the set (`icon.png.256`).
pub(super) fn icon_slot_field(at: &FieldPath, icon: &IconDecl, slot: IconSlot) -> FieldPath {
    match (icon, slot) {
        (IconDecl::Path(_), _) => at.clone(),
        (IconDecl::Set(_), IconSlot::Png(size)) => at.table("png").field(&size.to_string()),
        (IconDecl::Set(_), slot) => at.field(slot.name()),
    }
}

/// Check the icon files of a package rooted at `root`.
///
/// A path written without a template variable is package-relative and must
/// exist in the tree. A path starting with a template variable (`$app_root`,
/// `$app_bin`, …) names the installed payload, which does not exist before
/// installation; its file is checked when a launcher is listed. A launcher
/// with `menu = true` needs an SVG or a PNG of at least 256 pixels in its
/// own icon or, failing that, the application's.
///
/// ```
/// use vibe_core::manifest::{Manifest, icon_findings};
///
/// let dir = tempfile::tempdir().unwrap();
/// let manifest = Manifest::parse_str(
///     "[package]\nname = \"demo\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\
///      [application]\nid = \"demo\"\nicon = \"share/demo.svg\"\n",
/// )
/// .unwrap();
/// let findings = icon_findings(dir.path(), manifest.application.as_ref(), &[]);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].message.contains("does not exist"));
/// ```
pub fn icon_findings(
    root: &Path,
    application: Option<&ApplicationDecl>,
    options: &[InstallerOptionDecl],
) -> Vec<IconFinding> {
    let mut findings = Vec::new();
    let application_icon = application.and_then(|application| application.icon.as_ref());
    if let Some(icon) = application_icon {
        let at = FieldPath::root().table("application").field("icon");
        check_files(root, icon, "[application] icon", &at, &mut findings);
    }
    for (option_index, option) in options.iter().enumerate() {
        for (launcher_index, launcher) in option.launchers.iter().enumerate() {
            let launcher_at = FieldPath::root()
                .table("installer_options")
                .index(option_index)
                .table("launchers")
                .index(launcher_index);
            let table = format!(
                "installer option `{}` launcher `{}`",
                option.id, launcher.id
            );
            if let Some(icon) = &launcher.icon {
                let at = launcher_at.field("icon");
                check_files(root, icon, &format!("{table} icon"), &at, &mut findings);
            }
            for (action_index, action) in launcher.actions.iter().enumerate() {
                if let Some(icon) = &action.icon {
                    let label = format!("{table} action `{}` icon", action.id);
                    let at = launcher_at
                        .table("actions")
                        .index(action_index)
                        .field("icon");
                    check_files(root, icon, &label, &at, &mut findings);
                }
            }
            // An override row of a derived launcher may get its icon from
            // the desktop entry; the engine knows only after installation.
            let derived_override = !option.launchers_from.is_empty() && !launcher.is_complete();
            if launcher.shows_in_menu() && !derived_override {
                let icon = launcher.icon.as_ref().or(application_icon);
                if !icon.is_some_and(|icon| menu_ready(root, icon)) {
                    findings.push(IconFinding {
                        path: None,
                        field: launcher_at.field("menu"),
                        message: format!(
                            "{table} has menu = true but neither it nor the application names an SVG or a PNG of at least 256 pixels"
                        ),
                    });
                }
            }
        }
    }
    findings
}

/// Whether a file is package-relative (no leading template variable).
fn in_package(value: &str) -> bool {
    !value.starts_with('$')
}

fn check_files(
    root: &Path,
    icon: &IconDecl,
    label: &str,
    at: &FieldPath,
    findings: &mut Vec<IconFinding>,
) {
    for (slot, value) in icon.files() {
        if !in_package(value) {
            continue;
        }
        let path = root.join(value);
        let finding = |message: String| IconFinding {
            path: Some(value.to_owned()),
            field: icon_slot_field(at, icon, slot),
            message,
        };
        if !path.is_file() {
            findings.push(finding(format!(
                "{label} `{value}` does not exist in the package tree"
            )));
            continue;
        }
        if !value.to_ascii_lowercase().ends_with(".png") {
            continue;
        }
        match fs::read(&path)
            .ok()
            .and_then(|bytes| png_dimensions(&bytes))
        {
            None => findings.push(finding(format!(
                "{label} `{value}` does not carry a PNG header"
            ))),
            Some((width, height)) if width != height => findings.push(finding(format!(
                "{label} `{value}` is {width}x{height}, not square"
            ))),
            Some((width, _)) => {
                if let IconSlot::Png(size) = slot
                    && size != 0
                    && size != width
                {
                    findings.push(finding(format!(
                        "{label} `{value}` is listed as {size} pixels but its header says {width}"
                    )));
                }
            }
        }
    }
}

/// An SVG, or a PNG of at least 256 pixels (a short-form PNG by its header).
fn menu_ready(root: &Path, icon: &IconDecl) -> bool {
    icon.files().into_iter().any(|(slot, value)| match slot {
        IconSlot::Svg => true,
        IconSlot::Png(0) => {
            in_package(value)
                && fs::read(root.join(value))
                    .ok()
                    .and_then(|bytes| png_dimensions(&bytes))
                    .is_some_and(|(width, height)| width == height && width >= 256)
        }
        IconSlot::Png(size) => size >= 256,
        _ => false,
    })
}
