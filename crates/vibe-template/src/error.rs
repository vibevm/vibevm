//! What a render refuses, and where.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-VARIABLE-GRAMMAR");

use std::fmt;

/// Why a render was refused. The kind is the stable token a caller matches
/// on; the message is the sentence a user reads.
///
/// ```
/// use vibe_template::RenderErrorKind;
///
/// assert_eq!(RenderErrorKind::MissingDigest.as_str(), "template_missing_digest");
/// assert_eq!(RenderErrorKind::Grammar.as_str(), "template_grammar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderErrorKind {
    /// The template text is not TOML, or `[template]` is missing or
    /// malformed.
    Template,
    /// The variable grammar was broken: an unknown variable, an unclosed
    /// `{`, a digest variable outside a digest-bearing field, `{platform}`
    /// outside an installer option, a `{meta.…}` path resolving to an
    /// array or a table, or a `{size:…}` with a sizeless digest source.
    Grammar,
    /// A `{sha256:FILE}` or `{size:FILE}` whose file the digest document
    /// does not name, in a row that is not `template_optional`.
    MissingDigest,
    /// The render produced a manifest with no installer option at all.
    NoOption,
}

impl RenderErrorKind {
    /// The stable token, as a refusal code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Template => "template_declaration",
            Self::Grammar => "template_grammar",
            Self::MissingDigest => "template_missing_digest",
            Self::NoOption => "template_no_option",
        }
    }
}

/// One refusal, naming the TOML path it is about.
///
/// ```
/// use vibe_template::{RenderErrorKind, VersionRecord, skeleton};
///
/// let error = skeleton(
///     "[package]\nname = \"a\"\ngroup = \"org.e\"\nkind = \"app\"\nversion = \"1.0.0\"\n\
///      template = true\ndescription = \"{nope}\"\n\
///      [template.versions]\nkind = \"list\"\n\
///      [[template.versions.item]]\nversion = \"2.0.0\"\n\
///      [template.digests]\nkind = \"sha256sums\"\nurl = \"https://e.invalid/{version}\"\n",
///     &VersionRecord::plain("2.0.0")?,
/// )
/// .expect_err("an unknown variable is refused");
/// assert_eq!(error.kind, RenderErrorKind::Grammar);
/// assert_eq!(error.at, "package.description");
/// assert!(error.to_string().contains("package.description"));
/// # Ok::<(), vibe_template::RenderError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError {
    pub kind: RenderErrorKind,
    /// The TOML path of the field the refusal is about, or an empty string
    /// for the document as a whole.
    pub at: String,
    pub message: String,
    /// For a [`RenderErrorKind::MissingDigest`], the file the digest
    /// document does not name — what `template_optional` reports as the
    /// reason a row was dropped.
    pub subject: Option<String>,
}

impl RenderError {
    pub(crate) fn new(
        kind: RenderErrorKind,
        at: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            at: at.into(),
            message: message.into(),
            subject: None,
        }
    }

    pub(crate) fn grammar(at: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(RenderErrorKind::Grammar, at, message)
    }

    pub(crate) fn template(message: impl Into<String>) -> Self {
        Self::new(RenderErrorKind::Template, String::new(), message)
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.at.is_empty() {
            write!(f, "{} ({})", self.message, self.kind.as_str())
        } else {
            write!(
                f,
                "`{}`: {} ({})",
                self.at,
                self.message,
                self.kind.as_str()
            )
        }
    }
}

impl std::error::Error for RenderError {}
