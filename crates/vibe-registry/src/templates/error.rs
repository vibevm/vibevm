//! What the template layer refuses, and by which named code
//! (PROP-075 `#TEMPLATE-DISCOVERY`, `#TEMPLATE-CACHE`,
//! `#TEMPLATE-LOCK-RECORD`).
//!
//! Every variant names the package and the fix, because a refusal a user
//! cannot act on is a bug. The two codes PROP-075 spells out —
//! `template_index_required` and `template_cache_missing` — appear
//! verbatim in the message, so a test and a user grep for the same token.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY");

use std::path::PathBuf;

use vibe_core::Group;

/// A refusal of the template layer.
///
/// ```
/// use vibe_registry::templates::TemplateError;
///
/// let refusal = TemplateError::IndexRequired {
///     registry: "corp".to_string(),
///     package: "org.vibevm.node/node".to_string(),
/// };
/// assert!(refusal.to_string().contains("template_index_required"));
/// assert!(refusal.to_string().contains("fix:"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY")]
pub enum TemplateError {
    /// A per-package git registry with no index cannot be asked which of
    /// its tags is a template revision without one fetch per tag. A
    /// stated limitation, never a silent fallback.
    #[error(
        "template_index_required: registry `{registry}` has no index, so whether `{package}` \
         carries a template cannot be read without one fetch per tag \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY; \
          fix: publish an index for that registry, or mirror it into a layout registry)"
    )]
    IndexRequired { registry: String, package: String },

    /// Offline, and the cache has nothing to read.
    #[error(
        "template_cache_missing: `{package}` has no cached {what} at `{}` and the offline \
         posture forbids fetching it \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE; \
          fix: run `{command}` once with network access, then retry offline)",
        path.display()
    )]
    CacheMissing {
        package: String,
        /// `version list` or `digest document for <version>`.
        what: String,
        path: PathBuf,
        command: String,
    },

    /// The render itself refused — a missing digest, a variable that
    /// opens nothing, a `{meta.…}` naming an array. The inner message is
    /// `vibe-template`'s own and already names the TOML path.
    #[error(
        "`{package}@{version}` does not render: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-VARIABLE-GRAMMAR; \
          fix: correct the template at the named field, or publish that version concretely)"
    )]
    Render {
        package: String,
        version: String,
        reason: String,
    },

    /// The `[template]` declaration could not be read out of the template
    /// package, or the fetched version document did not parse.
    #[error(
        "the template of `{package}` is unreadable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-SPELLING; \
          fix: correct the template package's [template] table)"
    )]
    Declaration { package: String, reason: String },

    /// A fetch of the version list or a digest document failed.
    #[error(
        "the {what} of `{package}` could not be fetched from `{url}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE; \
          fix: resolve the reported transport cause, or run offline against a filled cache)"
    )]
    Fetch {
        package: String,
        what: String,
        url: String,
        reason: String,
    },

    /// The remap Locator refused the URL outright (`PROP-066#row`).
    #[error(
        "remap_denied: fetching the {what} of `{package}` at `{url}` is refused by the active \
         remap rows: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#row; \
          fix: add a remap row for that host, or run `vibe remap explain {url}`)"
    )]
    RemapDenied {
        package: String,
        what: String,
        url: String,
        reason: String,
    },

    /// The cache or the rendered store directory could not be written.
    #[error(
        "the template cache at `{}` is unusable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE; \
          fix: make that path writable, or point $VIBE_SETTINGS elsewhere)",
        path.display()
    )]
    Io { path: PathBuf, reason: String },

    /// A recomputed render disagrees with the lock or the installation
    /// record. The alarm of PROP-075 `#TEMPLATE-LOCK-RECORD`, raised
    /// before any effect.
    #[error(
        "{0} (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD; \
          fix: re-lock deliberately with `vibe update`, or publish that version concretely)"
    )]
    RenderChanged(Box<vibe_core::manifest::TemplateRenderChanged>),
}

impl TemplateError {
    /// `<group>/<name>`, the spelling every refusal names a package by.
    pub fn coordinate(group: &Group, name: &str) -> String {
        format!("{group}/{name}")
    }

    /// The stable code a test and a user grep for, without the prose.
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use vibe_registry::templates::TemplateError;
    ///
    /// let refusal = TemplateError::CacheMissing {
    ///     package: "org.vibevm.node/node".to_string(),
    ///     what: "version list".to_string(),
    ///     path: PathBuf::from("/tmp/versions.json"),
    ///     command: "vibe app versions org.vibevm.node/node".to_string(),
    /// };
    /// assert_eq!(refusal.code(), "template_cache_missing");
    /// ```
    pub fn code(&self) -> &'static str {
        match self {
            TemplateError::IndexRequired { .. } => "template_index_required",
            TemplateError::CacheMissing { .. } => "template_cache_missing",
            TemplateError::Render { .. } => "template_render_failed",
            TemplateError::Declaration { .. } => "template_declaration_unreadable",
            TemplateError::Fetch { .. } => "template_fetch_failed",
            TemplateError::RemapDenied { .. } => "remap_denied",
            TemplateError::Io { .. } => "template_cache_unusable",
            TemplateError::RenderChanged(_) => "template_render_changed",
        }
    }
}

impl From<TemplateError> for crate::RegistryError {
    fn from(error: TemplateError) -> Self {
        crate::RegistryError::Template(Box::new(error))
    }
}
