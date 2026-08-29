//! Effective role of the selected manifest — report metadata above the kernel.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

use vibe_core::PackageKind;

/// Effective role of the selected manifest for reporting and future preset
/// metadata. This is derived from the existing role tables, never authored as
/// another manifest field.
///
/// ```
/// use vibe_lifecycle::EffectiveManifestKind;
///
/// assert_ne!(
///     EffectiveManifestKind::Project,
///     EffectiveManifestKind::VirtualWorkspace,
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveManifestKind {
    Project,
    Package(PackageKind),
    VirtualWorkspace,
}
