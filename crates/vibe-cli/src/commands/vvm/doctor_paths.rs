//! What `vibe self doctor` reports about the slot link directories
//! (`PROP-059#path-directories`): the directories the user's applications
//! expose whole on PATH, and whether this process's PATH actually holds
//! each of them — the same question doctor already answers for the shim
//! directory.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#path-directories");

use std::path::PathBuf;

use super::path_has_dir;

/// One slot's link directory and its PATH state.
pub(super) struct LinkDir {
    pub(super) slot: String,
    pub(super) dir: PathBuf,
    pub(super) on_path: bool,
    /// What the link points at now; absent while the slot is off or its
    /// payload directory is not there.
    pub(super) target: Option<PathBuf>,
}

/// Every link directory the user home's applications ask for, in slot
/// order. A home without any is reported as none, and an unreadable index
/// is not doctor's problem to diagnose — `vibe app list` says so — so it
/// reports nothing here.
pub(super) fn observe(path_var: Option<&str>) -> Vec<LinkDir> {
    let Ok(settings) = crate::commands::app::global::settings_root() else {
        return Vec::new();
    };
    let Ok(index) = vibe_app::Index::load(&settings) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for slot in index.slots.keys() {
        for dir in vibe_app::slot_link_dirs(&settings, &index, slot) {
            rows.push(LinkDir {
                slot: slot.clone(),
                on_path: path_has_dir(path_var, &dir),
                target: std::fs::read_link(&dir).ok(),
                dir,
            });
        }
    }
    rows
}

/// The machine rows of the report.
pub(super) fn json(rows: &[LinkDir]) -> Vec<serde_json::Value> {
    rows.iter()
        .map(|row| {
            serde_json::json!({
                "slot": row.slot,
                "dir": row.dir.display().to_string(),
                "on_path": row.on_path,
                "target": row.target.as_ref().map(|t| t.display().to_string()),
            })
        })
        .collect()
}

/// One human line per link directory, in doctor's own shape.
pub(super) fn lines(rows: &[LinkDir]) -> Vec<String> {
    rows.iter()
        .map(|row| {
            let target = match &row.target {
                Some(target) => format!("→ {}", target.display()),
                None => "(no link: the slot is off or its payload directory is absent)".to_owned(),
            };
            format!(
                "{} slot `{}` path dir {} {} {target}",
                if row.on_path { "ok  " } else { "MISS" },
                row.slot,
                row.dir.display(),
                if row.on_path {
                    "(on PATH)"
                } else {
                    "(NOT on PATH)"
                }
            )
        })
        .collect()
}
