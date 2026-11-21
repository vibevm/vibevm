//! The version-store layer's failure surface, split from `store.rs` for the
//! file-length budget; `store` re-exports it, so `store::StoreError` is
//! unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#layout");

use std::io;
use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

/// The version-store layer's failure surface (PROP-019 §2.4, §2.5): reading,
/// parsing, or writing the on-disk inventory and the `current` pointer.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#layout")]
pub enum StoreError {
    #[error(
        "reading the VVM inventory `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: ensure the install root is readable)"
    )]
    ReadState {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "the VVM inventory `{path}` is malformed: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: repair or delete the corrupt state.toml)"
    )]
    ParseState { path: PathBuf, detail: String },

    #[error(
        "writing the VVM layout at `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: ensure the install root is writable)"
    )]
    WriteLayout {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "serialising the VVM inventory failed: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: report this — the in-memory state is malformed)"
    )]
    Serialise { detail: String },

    #[error(
        "refusing to rewrite immutable local instance `{selector}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: allocate a new terminal #N instance instead)"
    )]
    ImmutableInstance { selector: String },

    #[error(
        "refusing unsafe VVM identity: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: use a relative git ref that remains below the versions root)"
    )]
    UnsafeIdentity { detail: String },

    #[error(
        "refusing unsafe VVM mutation path `{path}`: {detail} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-019#layout; \
          fix: remove the symlink/reparse/special path component from the VVM store)"
    )]
    UnsafeMutationPath { path: PathBuf, detail: String },
}
