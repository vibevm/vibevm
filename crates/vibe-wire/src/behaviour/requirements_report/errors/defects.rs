//! The defect vocabularies the requirements refusals are built from.
//!
//! Split out of `errors.rs` when that file crossed the 600-line
//! budget, along its own responsibility seam: this half names WHAT can
//! be wrong with one value (an address, a coordinate, a state's
//! members, a reason code), while the parent names WHICH LAW a
//! violation witnesses and how the whole refusal renders. Each enum
//! carries its own sentence fragment, so a refusal reads as one
//! sentence without the parent knowing any of their internals.

/// Which observation axis a status-presence refusal is about. The two
/// axes answer different questions and must never be collapsed, which
/// is exactly why the refusal names which one broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusAxis {
    /// What the source document itself claims.
    Authoring,
    /// What this project recorded about the fact.
    Adoption,
}

impl StatusAxis {
    /// The wire member name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            StatusAxis::Authoring => "authoring",
            StatusAxis::Adoption => "adoption",
        }
    }
}

/// A bounded reference to one relation edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeRef {
    pub row: usize,
    pub edge: usize,
}

impl std::fmt::Display for EdgeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rows[{}].relations[{}]", self.row, self.edge)
    }
}

/// What is wrong with a full fact address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressDefect {
    /// No `spec://` scheme — a bare id names nothing resolvable.
    NoScheme,
    /// A Windows separator inside an address.
    Backslash,
    /// No `#` anchor at all.
    NoAnchor,
    /// More than one `#`.
    ExtraAnchor,
    /// The anchor is empty or whitespace-only.
    BlankAnchor,
    /// Fewer than `<group>/<package>/<path>` segments.
    TooFewSegments,
    /// An empty or whitespace-only path segment.
    BlankSegment,
    /// A `..` segment — an address that leaves its own tree.
    ParentSegment,
    /// A `.` segment — one path spelled two ways.
    DotSegment,
    /// The group does not parse as a `vibe_core::Group`.
    UnparseableGroup,
    /// The package does not parse as a `vibe_core::PackageName`.
    UnparseablePackage,
}

impl AddressDefect {
    pub(super) fn phrase(self) -> &'static str {
        match self {
            AddressDefect::NoScheme => "does not start with `spec://`",
            AddressDefect::Backslash => "contains a backslash",
            AddressDefect::NoAnchor => "carries no `#<fact>` anchor",
            AddressDefect::ExtraAnchor => "carries more than one `#`",
            AddressDefect::BlankAnchor => "carries a blank `#<fact>` anchor",
            AddressDefect::TooFewSegments => {
                "is not a full `spec://<group>/<package>/<path>` address"
            }
            AddressDefect::BlankSegment => "carries a blank path segment",
            AddressDefect::ParentSegment => "carries a `..` segment",
            AddressDefect::DotSegment => "carries a `.` segment",
            AddressDefect::UnparseableGroup => "carries a group that is not a `vibe_core::Group`",
            AddressDefect::UnparseablePackage => {
                "carries a package that is not a `vibe_core::PackageName`"
            }
        }
    }
}

/// What is wrong with a package coordinate or a source list's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceDefect {
    /// Not a `group/name` coordinate under the project's own grammars.
    NotACoordinate,
    /// Two results for one package coordinate. The identity key is the
    /// package alone: a second result under the other KIND is just as
    /// ambiguous, because `relation_sources` names a package and has
    /// no kind to disambiguate with.
    DuplicatePackage,
    /// The list is not sorted.
    OutOfOrder,
    /// A relation source names a package the base source layer never
    /// enumerated, so its kind-dependent provenance law has nothing to
    /// apply and would have to be skipped.
    NoBaseSource,
}

impl SourceDefect {
    pub(super) fn phrase(self) -> &'static str {
        match self {
            SourceDefect::NotACoordinate => {
                "is not a `group/name` coordinate that `vibe_core` can parse"
            }
            SourceDefect::DuplicatePackage => {
                "already had a source result; one package coordinate gets one, whatever its kind"
            }
            SourceDefect::OutOfOrder => "breaks the sorted source order",
            SourceDefect::NoBaseSource => {
                "has no base source result, so its provenance could only be judged by a law that \
                 skipped itself"
            }
        }
    }
}

/// What is wrong with a base source result's state-dependent members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStateDefect {
    /// A state that read no bytes carries a digest of them.
    UnexpectedDigest,
    /// A state that read the bytes carries no digest of them.
    AbsentDigest,
    /// A present digest is not `sha256:` + 64 lowercase hex.
    DigestShape,
    /// A state that is not `orphaned` carries an adoption count.
    UnexpectedAdoptionEntries,
    /// `orphaned` carries no adoption count — the count is what makes
    /// the orphan worth reporting.
    AbsentAdoptionEntries,
    /// `orphaned` carries a count of zero, which is not an orphan.
    ZeroAdoptionEntries,
}

impl SourceStateDefect {
    pub(super) fn phrase(self) -> &'static str {
        match self {
            SourceStateDefect::UnexpectedDigest => "carries a digest of bytes it never read",
            SourceStateDefect::AbsentDigest => "read the bytes but carries no digest of them",
            SourceStateDefect::DigestShape => {
                "carries a digest that is not `sha256:` followed by 64 lowercase hex characters"
            }
            SourceStateDefect::UnexpectedAdoptionEntries => {
                "carries an adoption-entry count, which only an orphan has"
            }
            SourceStateDefect::AbsentAdoptionEntries => {
                "carries no adoption-entry count; the count is what makes an orphan worth reporting"
            }
            SourceStateDefect::ZeroAdoptionEntries => {
                "carries zero adoption entries, which is not an orphan at all"
            }
        }
    }
}

/// Whether a reason code is owed, forbidden, or merely blank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonDefect {
    /// The state lost something and explains nothing.
    Absent,
    /// The state lost nothing and explains anyway.
    Unexpected,
    /// A present reason is empty or whitespace-only.
    Blank,
}

impl ReasonDefect {
    pub(super) fn phrase(self) -> &'static str {
        match self {
            ReasonDefect::Absent => "owes a reason code and carries none",
            ReasonDefect::Unexpected => "lost nothing, so it carries no reason code",
            ReasonDefect::Blank => "carries a blank reason code",
        }
    }
}
