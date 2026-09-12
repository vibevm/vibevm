//! The card's images — that they are there, that they are what they
//! claim to be, and that they fit the shape they will be shown in
//! ([PROP-057](../../../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)
//! §7, design decision D-20, campaign rule R-19).
//!
//! **The rules themselves live in the pipeline, not here.** `vibe-doc`
//! is where everything with content in it lives (`##PIPE-LIBRARY`), and
//! the card's limits are content: the byte ceilings, the proportions,
//! the three permitted formats and the refusal SVG earns are declared
//! once in [`vibe_doc::media`] and rendered twice — by `vibe doc check
//! --media` for a documentation build, and by this cell for a project
//! linter. A second copy of a byte ceiling is the rot this tree has
//! already paid for once, when a scanner hand-duplicated a manifest
//! parser and the copy went quietly stale.
//!
//! What this cell adds is the linter's own judgement: WHERE the images
//! of the project being checked are (its root), and how a finding
//! becomes a line of `vibe check` output. Both are this crate's
//! business and neither is the pipeline's.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#card");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_doc::media::{self, Severity};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::DocMedia`] cell.
#[cell(seam = "Check", variant = "doc-media")]
pub struct DocMediaCheck;

impl Check for DocMediaCheck {
    fn id(&self) -> CheckId {
        CheckId::DocMedia
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        // A root whose manifest cannot be read declares no card, and a
        // manifest that is not TOML is somebody else's finding: the
        // manifest-validity cell has already said so, and saying it
        // twice would put one defect on two lines.
        let Ok(checked) = media::check(project_root) else {
            return;
        };
        for finding in checked.findings {
            let path = Some(PathBuf::from(&finding.path));
            match finding.severity {
                Severity::Error => report.err(CheckId::DocMedia, path, None, finding.message),
                Severity::Warning => report.warn(CheckId::DocMedia, path, None, finding.message),
            }
        }
    }
}

#[cfg(test)]
#[path = "doc_media/tests.rs"]
mod tests;
