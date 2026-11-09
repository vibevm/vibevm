//! What `vibe self doctor` checks about one installed instance's layout:
//! which files of the distribution are where they should be
//! (PROP-019 `#layout`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);

use crate::commands::vvm::model::{InstallRecord, Origin};
use crate::commands::vvm::store::VersionStore;
use crate::commands::vvm::{bundle, placer, source as source_tree};
use vibe_publish::release_manifest::DISTRIBUTION_SOURCE_ARCHIVE_FILENAME;

pub(super) struct InstanceLayout {
    pub(super) kind: &'static str,
    pub(super) vibe: bool,
    pub(super) vibe_index: bool,
    pub(super) source: bool,
    pub(super) source_archive: bool,
    pub(super) manifest: bool,
    pub(super) manifest_root: bool,
    pub(super) placement_manifest: bool,
    pub(super) integrity: bool,
}

pub(super) fn instance_layout(store: &VersionStore, record: &InstallRecord) -> InstanceLayout {
    let home = store.instance_dir(&record.version_id(), record.instance);
    let vibe = store
        .binary_path(&record.version_id(), record.instance)
        .is_file();
    let vibe_index = store
        .index_binary_path(&record.version_id(), record.instance)
        .is_file();
    let source = match record.origin {
        Origin::Binary => store
            .instance_source_dir(&record.version_id(), record.instance)
            .is_dir(),
        Origin::Managed => {
            let mirror = store.mirror_dir();
            mirror.join(".git").is_dir() && source_tree::find_source_root(&mirror).is_some()
        }
        Origin::External => record
            .source_path
            .as_deref()
            .and_then(|path| source_tree::find_source_root(std::path::Path::new(path)))
            .is_some(),
    };
    let source_archive = home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME).is_file();
    let manifest = home.join("DISTRIBUTION.json").is_file();
    let manifest_root = record.distribution_manifest_sha256.is_some();
    let placement_manifest = home.join(".vvm-manifest.toml").is_file();
    let bundled_index = vibe_index && !placement_manifest; // an import places companions too
    let modern = record.origin == Origin::Binary
        && (record.source_path.is_some() || bundled_index || source || source_archive || manifest);
    let (kind, integrity) = if record.origin == Origin::Binary && modern {
        (
            if manifest_root {
                "bundle"
            } else {
                "bundle-unrooted"
            },
            bundle::installed_bundle_intact(store, record),
        )
    } else if record.origin == Origin::Binary {
        (
            "legacy-single-binary",
            placer::installed_files_match(store, record),
        )
    } else {
        ("source-build", placer::installed_files_match(store, record))
    };
    InstanceLayout {
        kind,
        vibe,
        vibe_index,
        source,
        source_archive,
        manifest,
        manifest_root,
        placement_manifest,
        integrity,
    }
}

pub(super) fn same_record(left: &InstallRecord, right: &InstallRecord) -> bool {
    left.version_id() == right.version_id() && left.instance == right.instance
}

pub(super) fn diagnostic_records(
    active: Option<&InstallRecord>,
    running: Option<&InstallRecord>,
) -> Vec<(&'static str, InstallRecord)> {
    let mut records = Vec::new();
    if let Some(active) = active {
        records.push(("active", active.clone()));
    }
    if let Some(running) = running
        && active.is_none_or(|active| !same_record(active, running))
    {
        records.push(("running", running.clone()));
    }
    records
}
