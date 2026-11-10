//! The redirect-block check — the `<vibevm>` marker pair in each agent
//! instruction file at the project root is well-formed (PROP-012 §2.2).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#markers");

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
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#markers",
        r = 1
    )]
    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for name in INSTRUCTION_FILES {
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
                continue;
            }
            report_codex_budget(name, &content, report);
        }
        report_discoverable_blocks(project_root, report);
    }
}

/// The agent instruction files vibevm owns a block inside (PROP-012).
const INSTRUCTION_FILES: [&str; 3] = ["CLAUDE.md", "AGENTS.md", "GEMINI.md"];

/// Which instruction files *below* the root a harness would also read, and
/// what is wrong with each — out of line along its own seam, because "what
/// would a harness discover" is a different question from "is this file's
/// block well-formed" and is the only half that needs an opinion about
/// ignore rules.
#[path = "redirect_block/discoverable.rs"]
mod discoverable;
use discoverable::report_discoverable_blocks;

/// How close the block's end may come to a project instruction file's
/// truncation limit before it is worth saying so. The limit itself and its
/// margin are `vibe-workspace`'s, so there is one measurement rather than a
/// second copy of a ceiling (owner decision D11,
/// `PROP-076#SYS-HARNESS-DELIVERY`).
fn report_codex_budget(name: &str, content: &str, report: &mut CheckReport) {
    let Some(end) = block_end(content) else {
        return;
    };
    let budget = vibe_workspace::boot_artifacts::CodexBudget::of(end);
    // The past-the-limit case is a refusal in the writer, so a project can
    // only reach it by hand-editing; reported here it is still a warning,
    // because `vibe check` reports and never writes.
    if let Some(message) = budget.message(name) {
        report.warn(
            CheckId::RedirectBlock,
            Some(PathBuf::from(name)),
            None,
            message,
        );
    }
}

/// The byte offset just past a well-formed managed block's last byte, or
/// `None` when the file carries no block.
pub(super) fn block_end(content: &str) -> Option<usize> {
    let mut offset = 0usize;
    let mut open = false;
    for line in content.split_inclusive('\n') {
        offset += line.len();
        match line.strip_suffix('\n').unwrap_or(line).trim() {
            "<vibevm>" => open = true,
            "</vibevm>" if open => return Some(offset),
            _ => {}
        }
    }
    None
}

/// Classify the `<vibevm>` markers in `content` (PROP-012 §2.2): a line
/// whose trimmed text is exactly `<vibevm>` / `</vibevm>` is a marker.
/// Returns `Some(reason)` when the file is malformed — anything other
/// than zero markers or exactly one ordered pair. The scan is
/// duplicated from `vibe-workspace::boot_artifacts` rather than
/// re-exported, to keep `vibe-check` independent of that crate's surface.
fn malformed_vibevm_block(content: &str) -> Option<String> {
    let mut open_lines: Vec<usize> = Vec::new();
    let mut close_lines: Vec<usize> = Vec::new();
    for (i, line) in content.lines().enumerate() {
        match line.trim() {
            "<vibevm>" => open_lines.push(i + 1),
            "</vibevm>" => close_lines.push(i + 1),
            _ => {}
        }
    }
    // The report names each marker's line — the drill's precision: the
    // operator repairs the file by hand and must not have to search for
    // the markers.
    match (open_lines.len(), close_lines.len()) {
        (0, 0) => None,
        (1, 1) if open_lines[0] < close_lines[0] => None,
        (1, 1) => Some(format!(
            "the `</vibevm>` marker (line {}) precedes its `<vibevm>` opener (line {})",
            close_lines[0], open_lines[0]
        )),
        (o, c) => Some(format!(
            "expected zero markers or exactly one `<vibevm>` … `</vibevm>` pair, found \
             {o} `<vibevm>` marker line(s){} and {c} `</vibevm>` marker line(s){}",
            at_lines(&open_lines),
            at_lines(&close_lines)
        )),
    }
}

/// Render a marker-line list as ` at line(s) 12, 40` — empty when there
/// are no markers, so a zero count is not followed by an empty list.
fn at_lines(lines: &[usize]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        let joined = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(" at line(s) {joined}")
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
        let finding = report
            .findings
            .iter()
            .find(|f| f.check == CheckId::RedirectBlock && f.severity == Severity::Error)
            .unwrap_or_else(|| panic!("got: {:?}", report.findings));
        // The drill's precision: the report names each marker's line.
        assert!(
            finding.message.contains("at line(s) 1, 4"),
            "the malformed report must name the marker lines; got: {}",
            finding.message
        );
    }

    /// A nested instruction file is reported, because a harness's downward
    /// discovery concatenates it — and the root's own block is not, because
    /// that one is this project's boot.
    #[test]
    fn a_nested_managed_block_is_a_warning_and_the_root_block_is_not() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let block = "# Mine\n\n<vibevm>\nrouter\n</vibevm>\n";
        fs::write(project.path().join("CLAUDE.md"), block).unwrap();
        fs::create_dir_all(project.path().join("research/demo")).unwrap();
        fs::write(project.path().join("research/demo/GEMINI.md"), block).unwrap();
        // A nested file with no block at all is nothing to report.
        fs::write(project.path().join("research/demo/AGENTS.md"), "# notes\n").unwrap();

        let report = check_project(project.path(), &opts());
        let nested = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::RedirectBlock)
            .collect::<Vec<_>>();
        assert_eq!(nested.len(), 1, "got: {nested:?}");
        assert_eq!(nested[0].severity, Severity::Warning);
        assert!(
            nested[0].message.contains("research/demo/GEMINI.md")
                && nested[0]
                    .message
                    .contains("discovers instruction files downward"),
            "{}",
            nested[0].message
        );
    }

    /// A dependency copy whose block is neutralised is clean; one that still
    /// carries a live block names the command that rewrites it.
    #[test]
    fn an_un_neutralised_dependency_copy_names_the_regeneration() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let slot = project
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.x.pkg/1.0.0");
        fs::create_dir_all(&slot).unwrap();
        fs::write(
            slot.join("AGENTS.md"),
            "# package\n\n<vibevm>\nPACKAGE ROUTER\n</vibevm>\n",
        )
        .unwrap();
        fs::write(
            slot.join("CLAUDE.md"),
            format!(
                "# package\n\n<vibevm>\n{}\n</vibevm>\n",
                vibe_workspace::boot_artifacts::NEUTRALISED_BODY
            ),
        )
        .unwrap();

        let report = check_project(project.path(), &opts());
        let found = report
            .findings
            .iter()
            .filter(|f| f.check == CheckId::RedirectBlock)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "got: {found:?}");
        assert_eq!(found[0].severity, Severity::Warning);
        assert!(
            found[0].message.contains("not \\\n")
                || found[0].message.contains("is not neutralised")
                || found[0].message.contains("not \n")
                || found[0].message.contains("neutralised"),
            "{}",
            found[0].message
        );
        assert!(
            found[0].message.contains("vibe install"),
            "{}",
            found[0].message
        );
    }

    /// The instruction-file truncation budget, reported at the one offset
    /// that matters: where the block ends.
    #[test]
    fn a_block_near_the_instruction_file_limit_is_a_warning() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        let limit = vibe_workspace::boot_artifacts::PROJECT_DOC_MAX_BYTES;
        let margin = vibe_workspace::boot_artifacts::PROJECT_DOC_WARN_MARGIN;
        let block = "<vibevm>\nrouter\n</vibevm>\n";
        let padding = "x".repeat(limit - margin - block.len());
        fs::write(
            project.path().join("AGENTS.md"),
            format!("{padding}\n{block}"),
        )
        .unwrap();

        let report = check_project(project.path(), &opts());
        let found = report
            .findings
            .iter()
            .find(|f| f.check == CheckId::RedirectBlock)
            .unwrap_or_else(|| panic!("got: {:?}", report.findings));
        assert_eq!(found.severity, Severity::Warning);
        assert!(
            found.message.contains("within 4 KB") && found.message.contains("32768"),
            "{}",
            found.message
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
