use std::fs;
use std::path::Path;

#[cfg(windows)]
use crate::{
    CleanupPreparation, OwnedTreeCleanupError, OwnedTreeCleanupProgress, OwnedTreeObservation,
    arm_after_owned_tree_publish_move, arm_after_rename_source_check, arm_before_owned_tree_check,
    arm_before_owned_tree_publish, arm_before_rename_noreplace, arm_between_manifest_passes,
    arm_during_manifest_lease, arm_during_native_mutation,
};
use crate::{DirectoryDurability, ExternalStore, Project, RenameError, arm_same_filesystem_check};

#[path = "tests/external_read.rs"]
mod external_read;
#[path = "tests/external_sync.rs"]
mod external_sync;

fn project() -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    let project = Project::open(root.path()).unwrap();
    (root, project)
}

include!("tests/external.rs");
external_tests!();
include!("tests/native.rs");
native_tests!();
include!("tests/publication.rs");
publication_tests!();
include!("tests/recovery.rs");
recovery_tests!();
include!("tests/links.rs");
links_tests!();
