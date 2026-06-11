//! The redirect-block check — the `<vibevm>` marker pair in each agent
//! instruction file at the project root is well-formed (PROP-012 §2.2).

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-012#markers");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::{cell, spec};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::RedirectBlock`] cell.
#[cell(seam = "Check", variant = "redirect-block")]
pub struct RedirectBlockCheck;

impl Check for RedirectBlockCheck {
    fn id(&self) -> CheckId {
        CheckId::RedirectBlock
    }

    /// PROP-012 §2.2 — each agent instruction file at the project root
    /// carries at most one well-formed `<vibevm>` block. A malformed file
    /// is an error: a mutating `vibe` command would refuse to proceed
    /// against it.
    #[spec(
        implements = "spec://vibevm/modules/vibe-workspace/PROP-012#markers",
        r = 1
    )]
    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for name in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
            let path = project_root.join(name);
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue, // an absent file is fine — `vibe` creates it
            };
            if let Some(reason) = malformed_vibevm_block(&content) {
                report.err(
                    CheckId::RedirectBlock,
                    Some(PathBuf::from(name)),
                    None,
                    format!("`{name}` has a malformed <vibevm> block: {reason}"),
                );
            }
        }
    }
}

/// Classify the `<vibevm>` markers in `content` (PROP-012 §2.2): a line
/// whose trimmed text is exactly `<vibevm>` / `</vibevm>` is a marker.
/// Returns `Some(reason)` when the file is malformed — anything other
/// than zero markers or exactly one ordered pair. The scan is
/// duplicated from `vibe-workspace::boot_artifacts` rather than
/// re-exported, to keep `vibe-check` independent of that crate's surface.
fn malformed_vibevm_block(content: &str) -> Option<String> {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut first_open: Option<usize> = None;
    let mut first_close: Option<usize> = None;
    for (i, line) in content.lines().enumerate() {
        match line.trim() {
            "<vibevm>" => {
                opens += 1;
                first_open.get_or_insert(i);
            }
            "</vibevm>" => {
                closes += 1;
                first_close.get_or_insert(i);
            }
            _ => {}
        }
    }
    match (opens, closes) {
        (0, 0) => None,
        (1, 1) if first_open < first_close => None,
        (1, 1) => Some("the `</vibevm>` marker precedes its `<vibevm>` opener".to_string()),
        (o, c) => Some(format!(
            "expected exactly one `<vibevm>` … `</vibevm>` pair, found {o} `<vibevm>` \
             and {c} `</vibevm>` marker line(s)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::test_support::{opts, write_minimal_project};
    use crate::{CheckId, Severity, check_project};

    #[test]
    fn redirect_block_malformed_is_an_error() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        // Two <vibevm> openers — malformed.
        fs::write(
            project.path().join("CLAUDE.md"),
            "<vibevm>\na\n</vibevm>\n<vibevm>\nb\n</vibevm>\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.check == CheckId::RedirectBlock && f.severity == Severity::Error),
            "got: {:?}",
            report.findings
        );
    }

    #[test]
    fn redirect_block_well_formed_is_clean() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        fs::write(
            project.path().join("CLAUDE.md"),
            "# my own instructions\n\n<vibevm>\nredirect\n</vibevm>\n",
        )
        .unwrap();
        let report = check_project(project.path(), &opts());
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.check == CheckId::RedirectBlock),
            "a well-formed block must not be flagged; got: {:?}",
            report.findings
        );
    }
}
