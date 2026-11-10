//! `vibe storage` library edits and moves, reexported by the error layer.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries");

use specmark::spec;
use thiserror::Error;

/// `vibe storage` edits of `storage.toml`, and moves of owned installations
/// between libraries.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#move")]
pub enum StorageError {
    #[error(
        "library path `{path}` is not absolute (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: pass an absolute directory)"
    )]
    RelativePath { path: String },
    #[error(
        "library id `{id}` is invalid: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: pass --id with a lowercase portable token other than `default` that no configured library uses)"
    )]
    InvalidId { id: String, reason: &'static str },
    #[error(
        "directory `{path}` is already configured as library `{id}` (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: check `vibe storage list`)"
    )]
    AlreadyListed { path: String, id: String },
    #[error(
        "directory `{path}` is not an empty directory (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: `vibe storage add` creates a fresh library in a new or empty directory; use `vibe storage attach` for an existing library)"
    )]
    NotEmpty { path: String },
    #[error(
        "directory `{path}` is not a library: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: attach a directory that holds vibe-library.toml, or create one with `vibe storage add`)"
    )]
    NotALibrary { path: String, reason: String },
    #[error(
        "the default library cannot be detached (violates spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries; \
         fix: detach one of the libraries listed in storage.toml)"
    )]
    DetachDefault,
    #[error(
        "installation `{key}` is not portable: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-059#portability; \
         fix: reinstall it with `vibe install -g --library ID` instead of moving it)"
    )]
    NotPortable { key: String, reason: &'static str },
    #[error(
        "installation `{key}` already lives in library `{library}` (violates spec://org.vibevm.core/vibevm/common/PROP-059#move; \
         fix: name a different destination library)"
    )]
    SameLibrary { key: String, library: String },
    #[error(
        "destination `{path}` already exists and no index record owns it (violates spec://org.vibevm.core/vibevm/common/PROP-059#move; \
         fix: inspect it, then remove it with `vibe storage gc --delete`)"
    )]
    DestinationOccupied { path: String },
    #[error(
        "{action} `{path}` failed: {source} (violates spec://org.vibevm.core/vibevm/common/PROP-059#move; \
         fix: check the permissions and free space of both libraries; `vibe storage gc` reports any leftover directory)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
}
