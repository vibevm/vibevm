//! Errors of the version listing — `vibe app versions`
//! (PROP-075 `#VERSIONS-LIST-OUTPUT`).
//!
//! Its own family rather than four more arms of [`crate::AppError`], for
//! the reason the sibling `error_paths` and `error_lifecycle` families
//! exist: the four refusals belong to one surface, they all refuse before
//! anything is read beyond the home's index, and `AppError` delegates to
//! them transparently so a caller's `match` is unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT");

use specmark::spec;
use thiserror::Error;

/// What can refuse a listing. Every one of them refuses before any
/// version is listed, and every message names what to write instead.
///
/// ```
/// let error = vibe_app::VersionsError::UnknownChannel {
///     package: "org.vibevm.node/node".into(),
///     channel: "lts/argon".into(),
///     channels: "`lts`, `lts/jod`".into(),
/// };
/// assert_eq!(error.code(), "unknown_channel");
/// assert!(error.to_string().contains("declares no channel `lts/argon`"));
/// ```
#[derive(Debug, Error)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
pub enum VersionsError {
    #[error(
        "`{target}` is neither an application slot of this home nor an installed package name; \
         slots: {slots} (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT; \
         fix: name the package as <group>/<name>, or a slot `vibe app list` shows)"
    )]
    UnknownTarget { target: String, slots: String },
    #[error(
        "`{target}` names several installed packages: {spellings} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT; \
         fix: name one exactly as <group>/<name>)"
    )]
    AmbiguousTarget { target: String, spellings: String },
    #[error(
        "`{package}` declares no channel `{channel}`; channels: {channels} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX; \
         fix: name one of the channels listed, or drop --channel)"
    )]
    UnknownChannel {
        package: String,
        channel: String,
        channels: String,
    },
    #[error(
        "the version domain of `{package}` could not be read: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST; \
         fix: correct the registry configuration, or run `vibe app versions --installed` for what \
         this home holds)"
    )]
    Domain { package: String, reason: String },
}
