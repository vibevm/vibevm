//! The failure-discriminator attempt record + the per-registry walk report.
//!
//! [`RegistryWalkAttempt`] is the structured row carried through the
//! `PackageNotFoundEverywhere` error into `vibe-cli`'s install-error JSON
//! envelope; [`format_walk_attempts`] renders the human-readable "Tried:"
//! block. Split out of `walk` to hold the file-length budget.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

/// One row in the aggregated "tried these registries" report surfaced via
/// [`crate::RegistryError::PackageNotFoundEverywhere`]. Captured per-registry
/// during the walk in `MultiRegistryResolver::resolve`; carried through the
/// `DepProvider` error chain into `vibe-cli`'s install-error JSON envelope so
/// machine-readable consumers can branch on the per-registry status without
/// parsing prose.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryWalkAttempt {
    pub name: String,
    pub url: String,
    pub auth: vibe_core::manifest::AuthKind,
    pub status: WalkAttemptStatus,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WalkAttemptStatus {
    /// Registry's `resolve` returned `UnknownPackage` — the registry was
    /// reachable, manifest parsed, just no version matching the pkgref.
    NotFound,
    /// Registry returned 401 / 403 (`AuthFailed`) but was declared
    /// `auth = "none"` and `strict_auth` was off, so the resolver
    /// reclassified the failure as "no public answer here" and walked past.
    /// The line below tells the operator the host would need credentials if
    /// they want to access this registry as authenticated.
    Public401,
}

impl WalkAttemptStatus {
    pub fn as_label(&self) -> &'static str {
        match self {
            WalkAttemptStatus::NotFound => "not found",
            WalkAttemptStatus::Public401 => "access denied (401, walked past — auth=none)",
        }
    }
}

/// Render the per-registry attempt rows as the human-readable "Tried:" block.
pub(super) fn format_walk_attempts(attempts: &[RegistryWalkAttempt]) -> String {
    use std::fmt::Write as _;
    // Compute the column width for the registry-name column so the rendered
    // table stays aligned regardless of label length. URLs are too varied to
    // align — they wrap to the right of the arrow.
    let name_width = attempts.iter().map(|a| a.name.len()).max().unwrap_or(0);
    let url_width = attempts.iter().map(|a| a.url.len()).max().unwrap_or(0);
    let mut out = String::new();
    for a in attempts {
        // Indent each line with two spaces so the report nests visually
        // under the parent error's "Tried:" label.
        let _ = writeln!(
            out,
            "  - {:<name_width$}  ({:<url_width$})  → {} (auth={})",
            a.name,
            a.url,
            a.status.as_label(),
            a.auth.as_str(),
            name_width = name_width,
            url_width = url_width,
        );
    }
    // Hint at the bottom — the most common operator next step when nothing
    // was found anywhere.
    if attempts
        .iter()
        .any(|a| matches!(a.status, WalkAttemptStatus::Public401))
    {
        out.push_str(
            "\nHint: at least one registry returned 401 / 403 and was walked past as `auth=none`.\n\
             If that registry is actually private, set `auth = \"token-env\"` and provide the\n\
             token via `VIBEVM_REGISTRY_TOKEN_<HOST>`; see docs/registry-auth.md.",
        );
    }
    out
}
