//! Public metadata publication adapters use the existing FileGuard owner.
specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");
use super::*;

/// Commit an admitted prepared source delta without overwriting a changed
/// selected manifest. The caller owns candidate admission before this write.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn publish_admitted_manifest(root: &Path, before: &Manifest, after: &Manifest) -> Result<()> {
    file_guard::publish_admitted_manifest(root, before, after)
}
/// Publish a known legacy empty lock through the same owned metadata boundary
/// used by apply, before a regenerated-world callback rereads it.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
pub fn publish_empty_legacy_lock(
    root: &Path,
    expected_schema: u32,
    replacement: &vibe_core::manifest::Lockfile,
) -> Result<()> {
    file_guard::publish_empty_legacy_lock(root, expected_schema, replacement)
}
