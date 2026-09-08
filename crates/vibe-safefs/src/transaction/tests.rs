use std::fs;
use std::path::Path;

use crate::{
    CleanupPreparation, DirectoryDurability, ExternalStore, OwnedTreeCleanupError,
    OwnedTreeCleanupProgress, OwnedTreeObservation, Project, RenameError,
    arm_after_owned_tree_publish_move, arm_after_rename_source_check, arm_before_owned_tree_check,
    arm_before_owned_tree_publish, arm_before_rename_noreplace, arm_between_manifest_passes,
    arm_during_manifest_lease, arm_during_native_mutation, arm_same_filesystem_check,
};

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
