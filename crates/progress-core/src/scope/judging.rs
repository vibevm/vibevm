//! `[judging] exempt` — of an observed file, that it is never judged
//! (PROP-057 `##OBS-NOT-JUDGED`).
//!
//! A different axis from the enumeration in [`super::enumerate`], and it
//! never touches it: an exemption changes what is JUDGED, never what is
//! OBSERVED. The type is what makes that split visible at the call site
//! rather than leaving it to a comment.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#config");

use super::rel_str;
use anyhow::{Context, Result};
use std::path::Path;

/// The compiled `[judging] exempt` globs — "observed, never judged"
/// (PROP-057 `##OBS-NOT-JUDGED`).
///
/// Deliberately separate from the enumeration in [`observed_files`]: an
/// exemption must never be able to shrink the corpus. A consumer that
/// counts debt asks this; a consumer that lists, parses, checks or maps
/// the corpus never does, and the type makes that split visible at the
/// call site rather than leaving it to a comment.
#[derive(Debug, Clone, Default)]
pub struct JudgingExemption {
    globs: Vec<glob::Pattern>,
}

impl JudgingExemption {
    /// Compile the patterns, naming any that is not a valid glob.
    ///
    /// An invalid pattern is an error and never a silent skip: a skipped
    /// exemption would put a documentation package back into the debt,
    /// which is the failure this key exists to prevent.
    pub fn compile(patterns: &[String]) -> Result<Self> {
        let globs = patterns
            .iter()
            .map(|p| {
                glob::Pattern::new(p).with_context(|| format!("bad judging exempt glob `{p}`"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(JudgingExemption { globs })
    }

    /// No exemptions declared — every observed file is judged.
    pub fn is_empty(&self) -> bool {
        self.globs.is_empty()
    }

    /// Is this repo-relative path observed but never judged?
    pub fn covers(&self, rel: &Path) -> bool {
        let path = rel_str(rel);
        self.globs.iter().any(|g| g.matches(&path))
    }
}
