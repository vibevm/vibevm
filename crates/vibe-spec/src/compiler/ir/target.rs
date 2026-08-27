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
        let bytes = value.as_bytes();
        let valid = (1..=64).contains(&bytes.len())
            && valid_id_byte(bytes[0])
            && bytes
                .iter()
                .skip(1)
                .all(|byte| valid_id_byte(*byte) || b"._-".contains(byte));
        if valid {
            Ok(Self(value))
        } else {
            Err(BackendIdError { value })
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
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
