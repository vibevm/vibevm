//! `specmap-core` — the traceability engine (PROP-014): the library behind
//! the shipped `rust-ai-native-specmap` / `rust-ai-native` binaries and any
//! project-local wrapper a consumer chooses to keep.
//!
//! - [`mdspec`] parses `spec/**/*.md` into anchored units with kind /
//!   revision / status lines and content hashes (PROP-014 §2.1–2.2,
//!   GUIDE-SPEC-AUTHORING §1–4).
//! - [`rscan`] walks the workspace sources with `syn` and extracts
//!   `#[spec]` / `#[verifies]` / `specmark::scope!` tags as edges
//!   (PROP-014 §2.3) — attributes are read as AST, never expanded.
//! - [`index`] composes both into the canonical, deterministic,
//!   committed `specmap.json` (PROP-014 §2.5; wire shape generated from
//!   `schemas/specmap.jtd.json` into this crate's own [`generated`] module).
//! - [`testgate`] implements the xfail-strict diff against the project's
//!   tests-baseline registry (BROWNFIELD §4).
//! - [`tripwire`] matches a change set against the debt registry's
//!   `touch:` tripwires (BROWNFIELD §3).
//!
//! Design rule inherited from B5 (monotone utility): scanners degrade —
//! an unparseable file becomes a warning entry, never a hard error.

specmark::scope!("spec://org.vibevm.ai-native.core-ai-native/mechanisms/PROP-014#index");

/// JTD-generated wire types for the canonical `specmap.json` index,
/// generated from the package's `schemas/specmap.jtd.json` (regeneration is
/// a maintainer dev-op in the package's dev repo). specmap-core owns its own
/// data model — no `vibe-wire` edge (Traceability Relocation Plan, Phase 1).
/// `non_snake_case` is allowed because jtd-codegen emits `pub camelCase`
/// fields carrying `#[serde(rename)]`.
#[allow(non_snake_case)]
pub mod generated;

pub mod config;
pub mod explain;
pub mod index;
pub mod ledger;
pub mod mdspec;
pub mod ratchet;
pub mod rscan;
pub mod scanner;
pub mod testgate;
pub mod tripwire;

/// `sha256:<hex>` over the given text with line endings normalised to
/// LF — the same content-hash format the lockfile uses, so hashes read
/// uniformly across the project.
///
/// ```
/// let lf = core_ai_native_specmap::content_hash("alpha\nbeta\n");
/// let crlf = core_ai_native_specmap::content_hash("alpha\r\nbeta\r\n");
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
/// assert_eq!(core_ai_native_specmap::fwd(&p), "spec/WAL.md");
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
