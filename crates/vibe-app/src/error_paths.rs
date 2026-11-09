//! Errors of the second PATH surface: the links that expose whole payload
//! directories (`PROP-059#path-directories`) and the per-slot off switch
//! (`PROP-059#slot-off`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#path-directories");

use specmark::spec;
use thiserror::Error;

/// What can refuse a slot's link reconciliation. Both refuse before any
/// link is written or removed.
///
/// ```
/// let error = vibe_app::PathsError::Target {
///     slot: "node".into(),
///     target: r"\\server\share\node\bin".into(),
///     reason: "a Windows directory junction needs a local absolute path".into(),
/// };
/// assert_eq!(error.code(), "path_link_target_unsupported");
/// assert!(error.to_string().contains("slot `node` cannot expose"));
/// ```
#[derive(Debug, Error)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
pub enum PathsError {
    #[error(
        "`{path}` is not a link vibe wrote, and it is where slot `{slot}` exposes its payload \
         directory {index} on PATH (violates spec://org.vibevm.core/vibevm/common/PROP-059#path-directories; \
         fix: remove or rename `{path}` yourself, then re-run)"
    )]
    ForeignLink {
        slot: String,
        index: u32,
        path: String,
    },
    #[error(
        "slot `{slot}` cannot expose `{target}` on PATH: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-059#path-directories)"
    )]
    Target {
        slot: String,
        target: String,
        reason: String,
    },
}
