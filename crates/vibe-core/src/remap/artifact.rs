//! The artifact identity, the candidate list and the explanation
//! (`PROP-066#locator`).
//!
//! The vocabulary of the resolution point, kept apart from the resolution
//! itself: what a caller says about the thing it is about to fetch, what
//! comes back, and what `remap explain` prints. None of it depends on the
//! rows, which is why the fetch paths can speak this language without
//! taking a dependency on how a winner is chosen.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use serde::Serialize;

use super::row::{Provenance, RemapKey, RemapScope, RemapTarget, UnmatchedPolicy};

/// Which fetch path is asking (`PROP-066#locator`). The kind never
/// changes which rows match — a `sha256` row matches a digest wherever it
/// was declared — but it is what every display, record and refusal names,
/// and it is how a caller's own defaults are kept apart.
///
/// ```
/// use vibe_core::remap::ArtifactKind;
///
/// assert_eq!(ArtifactKind::AppInput.label(), "app-input");
/// assert_eq!(ArtifactKind::RegistryClone.label(), "registry-clone");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    /// A registry's git clone, and the per-package repositories under it.
    RegistryClone,
    /// A registry index file (PROP-005).
    IndexFile,
    /// A single file read over HTTPS from a repository host (PROP-002's
    /// fast path). The default, because it is the narrowest kind: it
    /// names one file and carries no package of its own.
    #[default]
    RawFile,
    /// An embedded source tree pinned by commit and content hash
    /// (PROP-021).
    EmbeddedSource,
    /// A git-source dependency declared with a url and a rev.
    GitSourceDep,
    /// A `sources`, `archive` or `native` input of an application step
    /// (PROP-059, PROP-065).
    AppInput,
    /// A `vibe self` distribution and the manifest that lists them
    /// (PROP-019).
    Distribution,
    /// The documentation shell's release artifacts (PROP-057).
    DocShell,
}

impl ArtifactKind {
    /// The kebab-case name every human view and `--json` document prints.
    pub fn label(self) -> &'static str {
        match self {
            ArtifactKind::RegistryClone => "registry-clone",
            ArtifactKind::IndexFile => "index-file",
            ArtifactKind::RawFile => "raw-file",
            ArtifactKind::EmbeddedSource => "embedded-source",
            ArtifactKind::GitSourceDep => "git-source-dep",
            ArtifactKind::AppInput => "app-input",
            ArtifactKind::Distribution => "distribution",
            ArtifactKind::DocShell => "doc-shell",
        }
    }
}

/// The identity of one artifact a fetch path is about to go after.
///
/// ```
/// use vibe_core::remap::{Artifact, ArtifactKind};
///
/// let input = Artifact::new(ArtifactKind::AppInput)
///     .with_url("https://dist.example/tool-1.0.tgz")
///     .with_digest("AB".repeat(32))
///     .with_package("org.example/tool")
///     .with_step("fetch");
/// // The digest is normalised, because a row's `sha256` is matched on it.
/// assert!(input.digest.unwrap().starts_with("ab"));
/// assert_eq!(input.step.as_deref(), Some("fetch"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Artifact {
    /// Which fetch path is asking.
    pub kind: ArtifactKind,
    /// The declared origin, when the artifact has one.
    pub url: Option<String>,
    /// The digest the bytes will be checked against, lowercased.
    pub digest: Option<String>,
    /// The package the artifact belongs to, as `group/name[@version]`.
    pub package: Option<String>,
    /// The step of that package, for an application input.
    pub step: Option<String>,
}

impl Artifact {
    /// An artifact of this kind with nothing else declared yet.
    pub fn new(kind: ArtifactKind) -> Self {
        Artifact {
            kind,
            ..Artifact::default()
        }
    }

    /// Declare the origin URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Declare the digest the bytes will be verified against. Stored
    /// lowercased, so a manifest's spelling cannot decide whether a
    /// `sha256` row matches.
    pub fn with_digest(mut self, digest: impl AsRef<str>) -> Self {
        self.digest = Some(digest.as_ref().to_ascii_lowercase());
        self
    }

    /// Declare the owning package as `group/name[@version]`.
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    /// Declare the step, for an application input.
    pub fn with_step(mut self, step: impl Into<String>) -> Self {
        self.step = Some(step.into());
        self
    }
}

/// A reference back to one row of the [`Locator`]: its provenance (layer,
/// source, position in that source) and its position in the locator's own
/// row list, so a display can fetch the whole row with
/// [`Locator::row`].
///
/// ```
/// use vibe_core::remap::{Layer, Provenance, RowRef};
///
/// let r = RowRef {
///     provenance: Provenance::new(Layer::Invocation, "--remap closed.toml", 0),
///     position: 3,
/// };
/// assert_eq!(r.provenance.layer, Layer::Invocation);
/// assert_eq!(r.position, 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RowRef {
    /// Which layer, source and position declared the row.
    pub provenance: Provenance,
    /// Index into [`Locator::rows`].
    pub position: usize,
}

/// One place to try, and the row that chose it. `row` is `None` for the
/// declared origin, which is always last.
///
/// ```
/// use vibe_core::remap::{Candidate, RemapTarget};
///
/// let origin = Candidate {
///     target: RemapTarget::Url("https://github.com/vibespecs".into()),
///     row: None,
/// };
/// assert!(origin.row.is_none());
/// assert_eq!(origin.target.display(), "https://github.com/vibespecs");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// Where to look — a URL or a local path, never `deny`.
    pub target: RemapTarget,
    /// The row that produced it.
    pub row: Option<RowRef>,
}

/// What the Locator answers: an ordered candidate list, or a refusal
/// before any connection (`PROP-066#locator`, `PROP-066#policy`).
///
/// ```
/// use vibe_core::remap::{Artifact, ArtifactKind, Locator, Resolution};
///
/// // With no rows the answer is one candidate: the declared origin.
/// let locator = Locator::default();
/// let artifact = Artifact::new(ArtifactKind::RawFile).with_url("https://host/a");
/// let Resolution::Candidates(candidates) = locator.locate(&artifact) else {
///     panic!("an empty locator denies nothing")
/// };
/// assert_eq!(candidates.len(), 1);
/// assert!(candidates[0].row.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// Try these, in order; the declared origin is last when it survives.
    Candidates(Vec<Candidate>),
    /// Refuse before opening a connection. `row` is the `deny` row that
    /// refused; `None` means no row matched and the strictest active
    /// policy is `unmatched = "deny"`.
    Denied {
        row: Option<RowRef>,
        reason: Option<String>,
    },
}

/// One matching row, as `explain` reports it.
///
/// ```
/// use vibe_core::remap::{Layer, Provenance, RemapKey, RemapTarget, RowMatch, RowRef};
///
/// let m = RowMatch {
///     row: RowRef {
///         provenance: Provenance::new(Layer::Local, "~/.vibe/settings.toml", 0),
///         position: 0,
///     },
///     key: RemapKey::Prefix("https://github.com/".into()),
///     target: RemapTarget::Url("https://mirror/".into()),
///     scope: None,
///     reason: Some("the only host the proxy allows".into()),
/// };
/// assert_eq!(m.key.kind(), "prefix");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RowMatch {
    /// Where the row came from.
    pub row: RowRef,
    /// The key that matched.
    pub key: RemapKey,
    /// The target this row resolves the artifact to.
    pub target: RemapTarget,
    /// The scope that narrowed it, when it had one.
    pub scope: Option<RemapScope>,
    /// The row's `reason`.
    pub reason: Option<String>,
}

/// The whole chain for one fetch: the declared origin, every matching row
/// in winning order, the chosen target and the digest that will be checked
/// (`PROP-066#precedence`).
///
/// ```
/// use vibe_core::remap::{Artifact, ArtifactKind, Locator};
///
/// let chain = Locator::default()
///     .explain(&Artifact::new(ArtifactKind::DocShell).with_url("https://host/shell.tgz"));
/// assert!(chain.matches.is_empty());
/// assert!(!chain.denied);
/// assert_eq!(chain.chosen.unwrap().display(), "https://host/shell.tgz");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Explanation {
    /// Which fetch path asked.
    pub kind: ArtifactKind,
    /// The declared origin, before any row.
    pub origin: Option<String>,
    /// The digest the bytes will be checked against — unchanged by any
    /// remap, which is the point.
    pub digest: Option<String>,
    /// The strictest `unmatched` policy of the active layers.
    pub policy: UnmatchedPolicy,
    /// Every matching row, most-winning first.
    pub matches: Vec<RowMatch>,
    /// The target the fetch will try first; `None` when it is refused.
    pub chosen: Option<RemapTarget>,
    /// Whether the fetch is refused before any connection.
    pub denied: bool,
}
