//! Capability-relative, no-follow project filesystem mutation.
//!
//! Static pathname checks are not mutation safety: a path re-checked and then
//! used through ordinary `std::fs` calls can be redirected by a symlink or
//! junction swapped into an ancestor between the check and the use. This crate
//! pins mutation to capabilities instead — the trusted absolute project root
//! is opened once with ambient authority, every descendant directory is
//! reached one authored component at a time with `open_dir_nofollow`, final
//! file opens disable symlink following, and publication is a **unique owned**
//! staged file plus a capability-relative rename. Directory capabilities stay
//! pinned for the whole mutation, so a namespace swap after the walk cannot
//! redirect a write that goes through them.
//!
//! It is a crate rather than a module because two subsystems already need the
//! identical guarantee — package-skill receipts and create-phase agent outputs
//! — and a third (deploy) will. Two copies of a containment law is exactly how
//! the weaker copy becomes the one an attacker reaches.
//!
//! Everything here is safe Rust over `cap-std`; there is no handwritten
//! `unsafe` and no hand-declared OS ABI.

// The crate remains safe Rust except for the native no-replace rename calls in
// `transaction::platform`. `deny` lets that tiny cfg module opt in locally;
// every other module still rejects unsafe code.
#![deny(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE");

mod component;
mod file;
mod identity_hook;
mod project;
mod proof;
mod publish;
mod race_hook;
mod transaction;

pub use component::{
    STAGE_PREFIX, SelectionFault, UnsafeComponent, classify_component, ensure_lexically_contained,
    ensure_no_follow_walk, ensure_safe_component, identity_key, judge_selection, path_identity_key,
    paths_overlap, split_relative,
};
pub use file::{ContentDigest, FileIdentity, Presence, StableFileSnapshot, StableFileState};
#[cfg(any(test, feature = "inject-failures"))]
pub use file::{fail_after_publish, fail_before_publish, fail_before_stage_cleanup};
#[cfg(any(test, feature = "inject-failures"))]
pub use identity_hook::arm_identity_alias;
pub use project::{
    ExclusiveChildError, LockGuard, Pinned, PinnedAbsentPath, PinnedAbsoluteFile, Project,
};
pub use proof::{EntryProof, ProofRefusal};
pub use publish::{PublishError, PublishStage, Published};
#[cfg(any(test, feature = "inject-failures"))]
pub use race_hook::{
    arm_after_create_dir, arm_before_bounded_read, arm_before_create_dir, arm_before_link,
    arm_before_lock, arm_before_proved_removal, arm_before_publish_verify,
    arm_between_stream_passes, arm_bounded_read_identity_check, arm_lock_identity_check,
};
pub use transaction::{
    CleanupCompletion, CleanupIntent, CleanupPreparation, DirectoryDurability, DirectorySync,
    DurableWrite, EntryIdentity, EntryState, EntryStateKind, ExistingTreeEntryLease,
    ExternalProjectLock, ExternalStore, OwnedDirectory, OwnedDirectoryCreateError,
    OwnedDirectoryIdentity, OwnedTreeCleanupError, OwnedTreeCleanupProgress, OwnedTreeObservation,
    OwnedTreePublishError, PublishedPendingVerification, RenameError, TreeEntry, TreeManifest,
};
#[cfg(any(test, feature = "inject-failures"))]
pub use transaction::{
    arm_after_owned_tree_publish_move, arm_after_rename_source_check, arm_before_owned_tree_check,
    arm_before_owned_tree_publish, arm_before_rename_noreplace, arm_between_manifest_passes,
    arm_during_manifest_lease, arm_during_native_mutation, arm_same_filesystem_check,
};
