//! `specmap-core` — the engine behind `cargo xtask specmap`, `test-gate`
//! and `tripwire` (PLAYBOOK-TERRAFORM-VIBEVM v0.2 Phase 0).
//!
//! - [`mdspec`] parses `spec/**/*.md` into anchored units with kind /
//!   revision / status lines and content hashes (PROP-014 §2.1–2.2,
//!   GUIDE-SPEC-AUTHORING §1–4).
//! - [`rscan`] walks the workspace sources with `syn` and extracts
//!   `#[spec]` / `#[verifies]` / `specmark::scope!` tags as edges
//!   (PROP-014 §2.3) — attributes are read as AST, never expanded.
//! - [`index`] composes both into the canonical, deterministic,
//!   committed `specmap.json` (PROP-014 §2.5; wire shape generated from
//!   `schemas/specmap.jtd.json` into `vibe-wire`).
//! - [`testgate`] implements the xfail-strict diff against
//!   `terraform/registry/tests-baseline.json` (BROWNFIELD §4).
//! - [`tripwire`] matches a change set against the debt registry's
//!   `touch:` tripwires (BROWNFIELD §3).
//!
//! Design rule inherited from B5 (monotone utility): scanners degrade —
//! an unparseable file becomes a warning entry, never a hard error.

specmark::scope!("spec://vibevm/discipline/PROP-014#index");

pub mod explain;
pub mod index;
pub mod ledger;
pub mod mdspec;
pub mod ratchet;
pub mod rscan;
pub mod testgate;
pub mod tripwire;

/// The `<package>` segment of spec-side URIs. Today this is the repo
/// name (PROP-014 §2.1); group-qualified cross-package URIs are
/// deferred (PROP-014 §7.1).
pub const SPEC_PACKAGE: &str = "vibevm";

/// `sha256:<hex>` over the given text with line endings normalised to
/// LF — the same content-hash format the lockfile uses, so hashes read
/// uniformly across the project.
///
/// ```
/// let lf = specmap_core::content_hash("alpha\nbeta\n");
/// let crlf = specmap_core::content_hash("alpha\r\nbeta\r\n");
/// assert_eq!(lf, crlf);
/// assert!(lf.starts_with("sha256:"));
/// ```
pub fn content_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let normalised: String = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut hasher = Sha256::new();
    hasher.update(normalised.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(7 + digest.len() * 2);
    hex.push_str("sha256:");
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// Forward-slash form of a path, repo-relative paths everywhere.
///
/// ```
/// let p = std::path::Path::new("spec").join("WAL.md");
/// assert_eq!(specmap_core::fwd(&p), "spec/WAL.md");
/// ```
pub fn fwd(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_crlf_invariant() {
        let lf = "alpha\nbeta\n";
        let crlf = "alpha\r\nbeta\r\n";
        assert_eq!(content_hash(lf), content_hash(crlf));
        assert!(content_hash(lf).starts_with("sha256:"));
        assert_eq!(content_hash(lf).len(), 7 + 64);
    }

    #[test]
    fn content_hash_differs_on_content() {
        assert_ne!(content_hash("a"), content_hash("b"));
    }
}
