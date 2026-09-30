//! Full-text search over the loaded [`Index`] — built per query, never
//! stored, so no mutation has anything to invalidate.
//!
//! The tokeniser, the token set of a record and the ranking are not
//! this crate's: they live in [`vibe_wire::behaviour::index_search`]
//! beside the record type, one home for this server and for the
//! registry client's scan of a static mirror's `primary.jsonl`
//! (PROP-005 §2.12 `##TEXT-INDEX`, §2.14 `##INT-SEARCH`). What stays
//! here is the answer path's own judgement — search answers only over
//! what this build can act on (`quarantine::usable_versions`, never
//! `pkg.versions` raw, §2.6) — and the capability / PURL lookups, which
//! are exact matches and not text.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use specmark::spec;

use crate::index::Index;
use crate::index::quarantine::usable_versions;
use crate::types::{PackageKind, VersionEntry};

pub use vibe_wire::behaviour::index_search::{PackageHit as SearchHit, tokenise};

#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#cli",
    r = 2
)]
pub fn search(index: &Index, query: &str, kind_filter: Option<PackageKind>) -> Vec<SearchHit> {
    vibe_wire::behaviour::index_search::search_packages(
        index.by_pkgref.values().map(usable_versions),
        query,
        kind_filter.as_ref(),
    )
}

/// Find every package whose latest version `provides` the named capability.
pub fn lookup_capability<'a>(index: &'a Index, capability: &str) -> Vec<&'a VersionEntry> {
    let cap_norm = capability.trim();
    let mut out = Vec::new();
    for pkg in index.by_pkgref.values() {
        for v in usable_versions(pkg) {
            if provides_capability(v, cap_norm) {
                out.push(v);
            }
        }
    }
    out.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    out
}

/// Does `entry` advertise the capability `query`?
///
/// One home for the rule, because two passes ask it and they must not
/// disagree: this module's lookup walks the versions this build CAN
/// act on, while the `capabilities` verb makes a second pass over the
/// ones it cannot, to name them instead of hiding them (PROP-044
/// §4.5). Two copies of a match predicate agree by accident until the
/// day they do not — the defect this tree already paid for once (B7).
pub(crate) fn provides_capability(entry: &VersionEntry, query: &str) -> bool {
    entry
        .provides
        .as_ref()
        .is_some_and(|p| p.capabilities.iter().any(|c| capability_matches(c, query)))
}

fn capability_matches(advertised: &str, query: &str) -> bool {
    if advertised == query {
        return true;
    }
    // Allow query to omit the version constraint — match by left side
    // up to the first `@`.
    let advertised_left = advertised.split('@').next().unwrap_or(advertised);
    let query_left = query.split('@').next().unwrap_or(query);
    advertised_left == query_left
}

pub fn lookup_purl<'a>(index: &'a Index, purl: &str) -> Vec<&'a VersionEntry> {
    let mut out = Vec::new();
    let q = purl.trim();
    for pkg in index.by_pkgref.values() {
        for v in usable_versions(pkg) {
            if describes_purl(v, q) {
                out.push(v);
            }
        }
    }
    out.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    out
}

/// Does `entry` bind the PURL `query` — as the package itself, or
/// through one of its subskills?
///
/// Named for the same reason as [`provides_capability`]: the `purls`
/// verb makes a second pass over the versions this build cannot act
/// on, and the two passes must ask ONE question.
pub(crate) fn describes_purl(entry: &VersionEntry, query: &str) -> bool {
    entry.describes.as_deref() == Some(query)
        || entry
            .subskills
            .iter()
            .any(|s| s.describes.as_deref() == Some(query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenise_lowercases_and_drops_stopwords() {
        let tokens = tokenise("The quick BROWN fox-jumps_OVER the lazy dog");
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
        assert!(tokens.contains(&"jumps".to_string()));
        assert!(tokens.contains(&"lazy".to_string()));
        assert!(tokens.contains(&"dog".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
    }

    #[test]
    fn tokenise_drops_short_tokens() {
        let tokens = tokenise("a b ab abc");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"b".to_string()));
        assert!(tokens.contains(&"ab".to_string()));
        assert!(tokens.contains(&"abc".to_string()));
    }

    #[test]
    fn capability_matches_exact_and_left_only() {
        assert!(capability_matches(
            "ui:landing-page@0.3.0",
            "ui:landing-page@0.3.0"
        ));
        assert!(capability_matches(
            "ui:landing-page@0.3.0",
            "ui:landing-page"
        ));
        assert!(capability_matches("ui:landing-page", "ui:landing-page"));
        assert!(!capability_matches("ui:landing-page", "ui:dashboard"));
    }
}
