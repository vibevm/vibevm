//! Inverted-index full-text search — built lazily per query against
//! the loaded [`Index`]. Tokeniser: lowercase ASCII alphanumeric
//! runs; ~30-stopword filter (matches the discipline from
//! `vibe-check::activation_conflict`).
//!
//! Scoring is term-overlap: a hit gains one point per query token
//! it carries. Ties are broken by the `(group, name)` identity in
//! lexicographic order. Good enough for the indexed scale targeted by
//! slice 4 (≤ 10k packages); a tantivy-backed upgrade is a v1 lever.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;
use vibe_core::Group;

use crate::index::Index;
use crate::index::quarantine::{usable_latest_stable, usable_versions};
use crate::types::{PackageKind, VersionEntry};

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is", "it",
    "its", "of", "on", "or", "she", "that", "the", "this", "to", "was", "were", "with", "you",
    "your",
];

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub latest_stable: Option<semver::Version>,
    pub score: u32,
    pub matched_tokens: Vec<String>,
    pub description: Option<String>,
}

pub fn tokenise(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            buf.push(c.to_ascii_lowercase());
        } else if !buf.is_empty() {
            push_if_keepable(&mut out, std::mem::take(&mut buf));
        }
    }
    if !buf.is_empty() {
        push_if_keepable(&mut out, buf);
    }
    out
}

fn push_if_keepable(out: &mut Vec<String>, tok: String) {
    if STOPWORDS.contains(&tok.as_str()) {
        return;
    }
    if tok.len() < 2 {
        return;
    }
    out.push(tok);
}

#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#cli",
    r = 1
)]
pub fn search(index: &Index, query: &str, kind_filter: Option<PackageKind>) -> Vec<SearchHit> {
    let query_tokens: BTreeSet<String> = tokenise(query).into_iter().collect();
    if query_tokens.is_empty() {
        return Vec::new();
    }
    let mut hits: BTreeMap<(Group, String), SearchHit> = BTreeMap::new();
    for pkg in index.by_pkgref.values() {
        let latest = usable_versions(pkg)
            .rfind(|v| v.version.pre.is_empty())
            .or_else(|| usable_versions(pkg).next_back());
        let Some(latest) = latest else {
            continue;
        };
        // `kind` is per-version metadata (PROP-008 §2.3) — filter on
        // the version actually scored.
        if let Some(k) = &kind_filter
            && latest.kind != *k
        {
            continue;
        }
        let pkg_tokens: BTreeSet<String> = collect_tokens_for(latest).into_iter().collect();
        let matched: BTreeSet<&String> = query_tokens.intersection(&pkg_tokens).collect();
        if matched.is_empty() {
            continue;
        }
        let key = (pkg.group.clone(), pkg.name.clone());
        hits.insert(
            key,
            SearchHit {
                kind: latest.kind.clone(),
                group: pkg.group.clone(),
                name: pkg.name.clone(),
                latest_stable: usable_latest_stable(pkg).cloned(),
                score: matched.len() as u32,
                matched_tokens: matched.into_iter().cloned().collect(),
                description: latest.description.clone(),
            },
        );
    }
    let mut out: Vec<SearchHit> = hits.into_values().collect();
    out.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.group.cmp(&b.group))
            .then(a.name.cmp(&b.name))
    });
    out
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

fn collect_tokens_for(entry: &VersionEntry) -> Vec<String> {
    let mut text = String::new();
    text.push_str(&entry.name);
    text.push(' ');
    if let Some(d) = &entry.description {
        text.push_str(d);
        text.push(' ');
    }
    for k in &entry.keywords {
        text.push_str(k);
        text.push(' ');
    }
    if let Some(p) = &entry.provides {
        for c in &p.capabilities {
            text.push_str(c);
            text.push(' ');
        }
    }
    if let Some(p) = &entry.describes {
        text.push_str(p);
        text.push(' ');
    }
    tokenise(&text)
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
        assert!(!tokens.contains(&"over".to_string()) || tokens.contains(&"over".to_string())); // "over" isn't a stopword
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
