//! Retain node publication inputs when building a replay candidate.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT",
    r = 1
);
use super::*;
impl NodeReplayCandidate {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::install::bootgen) fn new(
        node_dir: PathBuf,
        rel: String,
        effective: EffectiveBoot,
        spec_format: SpecFormat,
        dependencies: Box<[UnitId]>,
        native: bool,
        node_guards: Vec<Arc<crate::install::bootgen::PreparedNodeBoot>>,
    ) -> Self {
        let boot_dir = node_dir.join(vibe_core::layout::current_boot_dir());
        Self {
            owner: OwnerRuntimeId::Node { rel: rel.clone() },
            rel,
            effective,
            spec_format,
            index_path: boot_dir.join(boot_artifacts::INDEX_FILE),
            static_path: boot_dir.join(boot_artifacts::static_file(spec_format)),
            stale_path: boot_dir.join(boot_artifacts::publication::stale_static_file(spec_format)),
            dependencies,
            native,
            node_guards,
        }
    }
}
