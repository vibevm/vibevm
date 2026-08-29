//! The single backend/target identity cell below targets and the registry.

/// The identity of one emit backend (`BackendId`).
///
/// This is the ONE backend/target identity in the compiler: [`ArtifactTarget`]
/// borrows it for a custom target and the backend registry keys its
/// implementations by it. The charset is `[a-z0-9][a-z0-9._-]{0,63}`; the
/// constructor is the single validating authority, so a value that exists is a
/// well-formed id whoever minted it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendId(String);

#[derive(Debug, thiserror::Error)]
#[error("invalid emit backend id `{value}`: expected [a-z0-9][a-z0-9._-]{{0,63}}")]
pub struct BackendIdError {
    pub(crate) value: String,
}

impl BackendId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, BackendIdError> {
        let value = value.into();
        if valid_id_spelling(&value) {
            Ok(Self(value))
        } else {
            Err(BackendIdError { value })
        }
    }

    /// Whether one borrowed spelling satisfies the frozen grammar — the same
    /// law [`BackendId::new`] enforces, shared through the one validator so
    /// neither can drift. Revalidating callers that already hold a borrowed
    /// candidate — the R4.1 transform-plan refusal path, where a builtin
    /// name can be attacker-sized — call this and never clone the candidate
    /// into an owned error or id just to check it.
    pub(crate) fn is_valid_spelling(value: &str) -> bool {
        valid_id_spelling(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The one borrowed grammar core behind both `BackendId::new` and
/// `BackendId::is_valid_spelling`: `[a-z0-9][a-z0-9._-]{0,63}`.
fn valid_id_spelling(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=64).contains(&bytes.len())
        && valid_id_byte(bytes[0])
        && bytes
            .iter()
            .skip(1)
            .all(|byte| valid_id_byte(*byte) || b"._-".contains(byte))
}

fn valid_id_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

/// The identity of one final artifact target.
///
/// The public constants are the two shipping STATIC targets. A custom target
/// is the owned [`BackendId`] a backend names itself with: identity does not
/// imply that an implementation is installed (registration, selection and
/// invocation are R6.3). The id must therefore survive a wire round-trip
/// verbatim, which is why it is owned here rather than borrowed from a
/// registry.
#[derive(Clone, PartialEq, Eq)]
pub struct ArtifactTarget(ArtifactTargetKind);

#[derive(Clone, PartialEq, Eq)]
enum ArtifactTargetKind {
    StaticMarkdown,
    StaticXml,
    Custom(BackendId),
}

impl ArtifactTarget {
    #[allow(non_upper_case_globals)]
    pub const StaticMarkdown: Self = Self(ArtifactTargetKind::StaticMarkdown);

    #[allow(non_upper_case_globals)]
    pub const StaticXml: Self = Self(ArtifactTargetKind::StaticXml);

    /// The owned custom target a backend names itself with. Fallible: the
    /// backend id charset is the ordinary scalar law, so an invalid id is
    /// refused here and not re-validated per use.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn custom(backend: impl Into<String>) -> Result<Self, BackendIdError> {
        Ok(Self(ArtifactTargetKind::Custom(BackendId::new(backend)?)))
    }

    /// The same constructor the R6 wire conversion uses: identity without an
    /// installed implementation.
    pub(crate) fn custom_backend(backend: BackendId) -> Self {
        Self(ArtifactTargetKind::Custom(backend))
    }

    pub(crate) fn backend_id(&self) -> &str {
        match &self.0 {
            ArtifactTargetKind::StaticMarkdown => "static-md",
            ArtifactTargetKind::StaticXml => "static-xml",
            ArtifactTargetKind::Custom(backend) => backend.as_str(),
        }
    }

    pub(crate) const fn is_static_markdown(&self) -> bool {
        matches!(self.0, ArtifactTargetKind::StaticMarkdown)
    }

    pub(crate) const fn is_static_xml(&self) -> bool {
        matches!(self.0, ArtifactTargetKind::StaticXml)
    }

    pub(crate) const fn is_custom(&self) -> bool {
        matches!(self.0, ArtifactTargetKind::Custom(_))
    }
}

impl std::fmt::Debug for ArtifactTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            ArtifactTargetKind::StaticMarkdown => f.write_str("StaticMarkdown"),
            ArtifactTargetKind::StaticXml => f.write_str("StaticXml"),
            ArtifactTargetKind::Custom(backend) => {
                f.debug_tuple("Custom").field(&backend.as_str()).finish()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BackendId;

    /// The borrowed predicate and the owned constructor enforce one grammar:
    /// they agree on every corpus spelling, and the corpus covers each way
    /// the frozen `[a-z0-9][a-z0-9._-]{0,63}` law can fail.
    #[test]
    fn is_valid_spelling_agrees_with_new_on_every_corpus_spelling() {
        let corpus = [
            ("static-xml", true),
            ("a", true),
            (&"l".repeat(64), true),
            ("", false),
            ("Static", false),
            ("1st", true),
            ("-leading", false),
            // A hyphen at the tail and an underscore mid-name are legal
            // here: both edge rules belong to the group grammar, not the
            // backend-id grammar.
            ("trailing-", true),
            ("under_score", true),
            (&"l".repeat(65), false),
        ];
        for (spelling, legal) in corpus {
            assert_eq!(
                BackendId::is_valid_spelling(spelling),
                legal,
                "spelling {spelling:?} classified wrong"
            );
            assert_eq!(
                BackendId::new(spelling).is_ok(),
                legal,
                "new disagrees with is_valid_spelling on {spelling:?}"
            );
        }
        // An accepted spelling is stored verbatim — the predicate never
        // normalises what the constructor would keep.
        let id = BackendId::new("static-xml").unwrap();
        assert_eq!(id.as_str(), "static-xml");
    }
}
