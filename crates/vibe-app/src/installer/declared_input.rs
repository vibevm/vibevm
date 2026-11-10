//! One declared input of a manifest, fetched to a path the caller names
//! (PROP-075 `#MIRROR-PAYLOAD-PROJECTION`).
//!
//! An install fetches an input into its own work directory and keeps it in
//! the cross-run cache. A **mirror** fetches the same input into a package
//! drop, so a client inside a closed network finds it there instead of
//! downloading it. Those are the same act — the Locator's candidates in
//! winning order, the digest verified on whichever of them served the
//! bytes, and every download through the one transfer wrapper
//! (`PROP-068#coverage`) — so this is the installer's own fetch with a
//! destination the caller chooses, never a second reader.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use std::path::Path;

use vibe_core::progress::ProgressTask;
use vibe_core::progress::transfer::Purpose;
use vibe_core::remap::{Artifact, ArtifactKind, Locator};

use super::fetch::TransferFacts;
use super::{FetchWatch, Located, ServedBy, fetch_located};
use crate::error::InstallerError;

/// One input to fetch: where the manifest declares it, what it must hash
/// to, and the coordinate the drop rungs are named by.
///
/// ```
/// use vibe_app::DeclaredInput;
/// use vibe_core::progress::transfer::Purpose;
///
/// let sha256 = "ab".repeat(32);
/// let input = DeclaredInput {
///     remap: vibe_core::remap::active(),
///     package: "org.example/tool@1.2.0",
///     input: "tool",
///     declared: "https://downloads.example/tool-1.2.0.tar.gz",
///     sha256: &sha256,
///     size: Some(4096),
///     file_name: "tool-1.2.0.tar.gz",
///     http_client: None,
///     task: None,
///     purpose: Purpose::Archive,
/// };
/// // The declared URL stays the input's identity; a remap row can only
/// // change which candidate serves those exact bytes.
/// assert!(input.declared.starts_with("https://downloads.example/"));
/// assert_eq!(input.file_name, "tool-1.2.0.tar.gz");
/// ```
pub struct DeclaredInput<'a> {
    /// This invocation's remap Locator.
    pub remap: &'a Locator,
    /// `<group>/<name>@<version>` of the package the input belongs to —
    /// what names the first rungs of the drop lookup (`PROP-066#drops`).
    pub package: &'a str,
    /// The input's name: a `sources` entry's `name`, else the step id.
    pub input: &'a str,
    /// The URL the manifest declares, **before** remap. It stays the
    /// identity of the input: a remap can only change where the bytes come
    /// from, never what they must be.
    pub declared: &'a str,
    pub sha256: &'a str,
    /// The declared size, when the manifest states one; bytes beyond it
    /// are refused (`PROP-065#sizes`).
    pub size: Option<u64>,
    /// The file name the bytes are written under.
    pub file_name: &'a str,
    pub http_client: Option<&'a vibe_package_source::OperationHttpClient>,
    pub task: Option<&'a ProgressTask>,
    /// What the transfer journal says the bytes are for.
    pub purpose: Purpose,
}

/// Fetch `input` into `destination`, verified against its SHA-256.
///
/// The destination's parent stands in for the package root, because a
/// caller outside an install has no package directory to read a
/// package-relative `source` out of: such an input is not projectable and
/// is the caller's to skip before it gets here.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
pub fn fetch_declared_input(
    input: &DeclaredInput<'_>,
    destination: &Path,
    offline: bool,
) -> Result<ServedBy, InstallerError> {
    let artifact = Artifact::new(ArtifactKind::AppInput)
        .with_package(input.package)
        .with_step(input.input)
        .with_url(input.declared)
        .with_digest(input.sha256);
    let facts = TransferFacts {
        package: input.package.to_owned(),
        step: input.input.to_owned(),
        batch: None,
        resume: None,
    };
    let watch = FetchWatch {
        http_client: input.http_client,
        size: input.size,
        task: input.task,
        facts: Some(&facts),
        purpose: Some(input.purpose),
        ..FetchWatch::default()
    };
    fetch_located(
        &Located {
            locator: input.remap,
            artifact: &artifact,
            declared: input.declared,
            sha256: input.sha256,
            filename: Some(input.file_name),
        },
        destination.parent().unwrap_or(destination),
        destination,
        offline,
        &watch,
    )
}
