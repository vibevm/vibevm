//! Pure adapter of retained bytes; no new parser or filesystem authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-CAPTURED-ANALYSIS");
use super::{ChangeIdentityAnalysis, SavedChangeError};
use std::path::Path;
use vibe_core::manifest::change::{ChangeContext, ChangeManifest};
#[derive(Debug)]
pub struct ParsedCapturedChange {
    pub manifest: ChangeManifest,
    pub identities: ChangeIdentityAnalysis,
}
pub fn analyze_change_bytes(
    text: &str,
    context: &ChangeContext,
    directory: &Path,
    inputs: &[(&Path, &[u8])],
) -> Result<ParsedCapturedChange, SavedChangeError> {
    let manifest = ChangeManifest::parse_str(text, context, directory)?;
    let identities = super::identity::identity_from_observations(&manifest, inputs)?;
    Ok(ParsedCapturedChange {
        manifest,
        identities,
    })
}
