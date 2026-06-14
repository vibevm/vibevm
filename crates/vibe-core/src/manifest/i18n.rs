//! Internationalisation primitives — language tags, fallback chains, and
//! sidecar filename resolution.
//!
//! Spec: PROP-003 §2.7. BCP-47 language tags ([RFC 5646](https://datatracker.ietf.org/doc/html/rfc5646)).
//!
//! ## Sidecar pattern
//!
//! Localised content lives next to the canonical file with a `.<lang>`
//! segment inserted before the extension:
//!
//! ```text
//! README.md            ← canonical (default: en)
//! README.ru.md         ← Russian translation
//! README.zh-Hans.md    ← Simplified Chinese
//! ```
//!
//! Resolution walks the fallback chain: exact tag → region-stripped tag →
//! canonical (no language suffix) → hard error if even canonical missing.

specmark::scope!("spec://vibevm/modules/vibe-resolver/PROP-003#i18n");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default canonical language for the registry. Hard-coded so a malformed
/// `[i18n]` block at any layer still parses to a usable resolution.
pub const DEFAULT_CANONICAL_LANGUAGE: &str = "en";

/// `[i18n]` block in `vibe.toml`.
///
/// At the package level: declares which languages this package ships
/// translations for. At the project level: declares the consumer's
/// preferred language and fallback chain.
///
/// ```
/// use vibe_core::manifest::i18n::I18nDecl;
///
/// let d: I18nDecl = toml::from_str(r#"
///     canonical = "en"
///     available = ["en", "ru", "ja"]
///     preferred = "ru"
/// "#).unwrap();
/// // The project's resolution chain puts the preference first, canonical last.
/// assert_eq!(d.project_preference_chain(), vec!["ru", "en"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct I18nDecl {
    /// Default language for this manifest. Files without a `.<lang>`
    /// suffix are interpreted as being in this language.
    #[serde(default = "default_canonical")]
    pub canonical: String,

    /// Languages this package/project supports (package level) or prefers
    /// (project level). At project level, the FIRST entry is the primary
    /// preference; the remainder is the fallback chain.
    #[serde(default)]
    pub available: Vec<String>,

    /// Project-level only: explicit fallback chain. If absent, falls back
    /// to canonical → DEFAULT_CANONICAL_LANGUAGE. Package-level manifests
    /// should leave this empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback: Vec<String>,

    /// Project-level: the resolved primary language preference.
    /// Convenience field — when set, overrides `available[0]` for
    /// preference resolution. Carries through from `vibe install
    /// --language ru`. Package-level manifests should leave this None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred: Option<String>,
}

fn default_canonical() -> String {
    DEFAULT_CANONICAL_LANGUAGE.to_string()
}

impl Default for I18nDecl {
    fn default() -> Self {
        I18nDecl {
            canonical: DEFAULT_CANONICAL_LANGUAGE.to_string(),
            available: Vec::new(),
            fallback: Vec::new(),
            preferred: None,
        }
    }
}

impl I18nDecl {
    /// Empty / default — no translations declared. Equivalent to
    /// English-only canonical content.
    pub fn is_default(&self) -> bool {
        self.canonical == DEFAULT_CANONICAL_LANGUAGE
            && self.available.is_empty()
            && self.fallback.is_empty()
            && self.preferred.is_none()
    }

    /// Build the resolution chain for a project-level preference.
    ///
    /// Precedence: `preferred` > `available[0]` > `canonical`. The chain
    /// always ends with `canonical` then the registry-wide
    /// [`DEFAULT_CANONICAL_LANGUAGE`] so step 3 of the fallback ladder
    /// (PROP-003 §2.7.2) is always reachable.
    pub fn project_preference_chain(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(p) = &self.preferred {
            out.push(p.clone());
        } else if let Some(first) = self.available.first() {
            out.push(first.clone());
        }
        for f in &self.fallback {
            if !out.contains(f) {
                out.push(f.clone());
            }
        }
        if !out.contains(&self.canonical) {
            out.push(self.canonical.clone());
        }
        let default = DEFAULT_CANONICAL_LANGUAGE.to_string();
        if !out.contains(&default) {
            out.push(default);
        }
        out
    }
}

/// Insert `.<lang>` before the extension of `path`. Filenames with no
/// extension take the lang segment as a suffix
/// (`README` → `README.ru`).
pub fn localised_path(path: &Path, lang: &str) -> PathBuf {
    if lang.is_empty() {
        return path.to_path_buf();
    }
    let parent = path.parent();
    let file_name = path.file_name().and_then(|n| n.to_str());
    let Some(name) = file_name else {
        return path.to_path_buf();
    };
    let new_name = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{lang}.{ext}"),
        _ => format!("{name}.{lang}"),
    };
    match parent {
        Some(p) => p.join(new_name),
        None => PathBuf::from(new_name),
    }
}

/// Resolve a logical path against the language-fallback chain. Returns
/// the first variant that exists on disk, or `None` if even the
/// canonical-form file is missing.
///
/// `chain` is a list of language tags in priority order — typically
/// produced by [`I18nDecl::project_preference_chain`]. The canonical
/// (no-suffix) form is appended automatically as the last fallback.
pub fn resolve_localised(base_dir: &Path, logical: &Path, chain: &[String]) -> Option<PathBuf> {
    for lang in chain {
        if lang.is_empty() {
            continue;
        }
        // Region fallback: try exact tag; if it carries a `-region`, try
        // the language-only stripped form too.
        let mut tags: Vec<String> = vec![lang.clone()];
        if let Some((primary, _region)) = lang.split_once('-') {
            tags.push(primary.to_string());
        }
        for tag in tags {
            let candidate = base_dir.join(localised_path(logical, &tag));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    // Canonical / no-suffix fallback.
    let fallback = base_dir.join(logical);
    if fallback.is_file() {
        return Some(fallback);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn localised_path_inserts_segment() {
        assert_eq!(
            localised_path(Path::new("README.md"), "ru"),
            PathBuf::from("README.ru.md")
        );
        assert_eq!(
            localised_path(Path::new("spec/flows/wal/PROTOCOL.md"), "zh-Hans"),
            PathBuf::from("spec/flows/wal/PROTOCOL.zh-Hans.md")
        );
        assert_eq!(
            localised_path(Path::new("README"), "ru"),
            PathBuf::from("README.ru")
        );
    }

    #[test]
    fn project_preference_chain_orders_correctly() {
        let decl = I18nDecl {
            canonical: "en".into(),
            available: Vec::new(),
            fallback: vec!["en".into()],
            preferred: Some("ru".into()),
        };
        let chain = decl.project_preference_chain();
        assert_eq!(chain, vec!["ru", "en"]);
    }

    #[test]
    fn project_preference_falls_back_to_canonical() {
        let decl = I18nDecl::default();
        let chain = decl.project_preference_chain();
        assert_eq!(chain, vec!["en"]);
    }

    #[test]
    fn resolve_picks_localised_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("README.md"), "EN").unwrap();
        fs::write(root.join("README.ru.md"), "RU").unwrap();
        let chain = vec!["ru".to_string(), "en".to_string()];
        let resolved = resolve_localised(root, Path::new("README.md"), &chain).unwrap();
        assert_eq!(resolved, root.join("README.ru.md"));
    }

    #[test]
    fn resolve_falls_back_to_canonical_when_lang_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("README.md"), "EN").unwrap();
        let chain = vec!["ja".to_string(), "en".to_string()];
        let resolved = resolve_localised(root, Path::new("README.md"), &chain).unwrap();
        assert_eq!(resolved, root.join("README.md"));
    }

    #[test]
    fn resolve_returns_none_when_canonical_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let chain = vec!["en".to_string()];
        let resolved = resolve_localised(tmp.path(), Path::new("MISSING.md"), &chain);
        assert!(resolved.is_none());
    }

    #[test]
    fn region_stripped_fallback_works() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("README.md"), "EN").unwrap();
        fs::write(root.join("README.pt.md"), "PT").unwrap();
        let chain = vec!["pt-BR".to_string(), "en".to_string()];
        let resolved = resolve_localised(root, Path::new("README.md"), &chain).unwrap();
        assert_eq!(resolved, root.join("README.pt.md"));
    }

    #[test]
    fn i18n_decl_is_default_works() {
        assert!(I18nDecl::default().is_default());
        let mut decl = I18nDecl::default();
        decl.available.push("ru".into());
        assert!(!decl.is_default());
    }

    #[test]
    fn parses_canonical_only() {
        let raw = r#"canonical = "en""#;
        let d: I18nDecl = toml::from_str(raw).unwrap();
        assert_eq!(d.canonical, "en");
    }

    #[test]
    fn parses_full_block() {
        let raw = r#"
canonical = "en"
available = ["en", "ru", "ja"]
fallback = ["en"]
preferred = "ru"
"#;
        let d: I18nDecl = toml::from_str(raw).unwrap();
        assert_eq!(d.canonical, "en");
        assert_eq!(d.available, vec!["en", "ru", "ja"]);
        assert_eq!(d.preferred.as_deref(), Some("ru"));
    }
}
