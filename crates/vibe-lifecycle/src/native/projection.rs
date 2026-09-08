//! Read-only projection of current-platform native source groups.

use std::path::PathBuf;

use specmark::spec;

use super::provider::ProviderHome;
use super::{ExtensionRegistryRow, NativeArtifactError, NativePlatform, source_groups};

/// One source group after the lifecycle layer applies prebuilt-first selection.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
pub struct NativeSourceGroupProjection<'a> {
    pub provider: String,
    pub provider_root: PathBuf,
    pub record_root: NativeArtifactRecordRoot,
    pub crate_dir: String,
    pub candidates: Vec<&'a ExtensionRegistryRow>,
}

/// Relative-root vocabulary written into the durable artifact record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
pub enum NativeArtifactRecordRoot {
    Project,
    Slot,
}

/// Validate current-platform prebuilts and return only source fallbacks.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
pub fn project_native_source_groups<'a>(
    candidates: &[&'a ExtensionRegistryRow],
    platform: NativePlatform,
) -> Result<Vec<NativeSourceGroupProjection<'a>>, NativeArtifactError> {
    source_groups(candidates, platform, true).map(|groups| {
        groups
            .into_iter()
            .map(|group| NativeSourceGroupProjection {
                provider: group.provider.identity,
                provider_root: group.provider.root,
                record_root: match group.provider.home {
                    ProviderHome::Dependency => NativeArtifactRecordRoot::Slot,
                    ProviderHome::Host => NativeArtifactRecordRoot::Project,
                },
                crate_dir: group.crate_wire,
                candidates: group.rows,
            })
            .collect()
    })
}
