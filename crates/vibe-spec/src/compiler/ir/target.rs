//! Closed shipping targets plus an unnameable test-support custom target.

/// The identity of one final artifact target.
///
/// The public constants are the two shipping STATIC targets. The
/// representation stays private so crate-internal test support can exercise a
/// genuinely custom target without adding a production enum variant or an
/// external backend-registration surface ahead of R6.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ArtifactTarget(ArtifactTargetKind);

#[derive(Clone, Copy, PartialEq, Eq)]
enum ArtifactTargetKind {
    StaticMarkdown,
    StaticXml,
    Custom(&'static str),
}

impl ArtifactTarget {
    #[allow(non_upper_case_globals)]
    pub const StaticMarkdown: Self = Self(ArtifactTargetKind::StaticMarkdown);

    #[allow(non_upper_case_globals)]
    pub const StaticXml: Self = Self(ArtifactTargetKind::StaticXml);

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) const fn custom(backend: &'static str) -> Self {
        Self(ArtifactTargetKind::Custom(backend))
    }

    pub(crate) const fn backend_id(self) -> &'static str {
        match self.0 {
            ArtifactTargetKind::StaticMarkdown => "static-md",
            ArtifactTargetKind::StaticXml => "static-xml",
            ArtifactTargetKind::Custom(backend) => backend,
        }
    }

    pub(crate) const fn is_static_markdown(self) -> bool {
        matches!(self.0, ArtifactTargetKind::StaticMarkdown)
    }

    pub(crate) const fn is_static_xml(self) -> bool {
        matches!(self.0, ArtifactTargetKind::StaticXml)
    }

    pub(crate) const fn is_custom(self) -> bool {
        matches!(self.0, ArtifactTargetKind::Custom(_))
    }
}

impl std::fmt::Debug for ArtifactTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            ArtifactTargetKind::StaticMarkdown => f.write_str("StaticMarkdown"),
            ArtifactTargetKind::StaticXml => f.write_str("StaticXml"),
            ArtifactTargetKind::Custom(backend) => f.debug_tuple("Custom").field(&backend).finish(),
        }
    }
}
