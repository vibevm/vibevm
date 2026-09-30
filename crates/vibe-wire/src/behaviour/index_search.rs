//! Text search over catalog records — the one home of the rule that
//! turns a query and a package's versions into a ranked hit.
//!
//! Two readers ask it. The `vibe-index` server answers `/v1/packages`
//! from its loaded index, and the registry client, when the index it
//! found is a static mirror with no route to ask — the shape of every
//! raw GitHub index, the default `vibespecs` one included — reads
//! `primary.jsonl` and scores it here. Both must return the same hits
//! for the same catalog, and two copies of a tokeniser agree by
//! accident until the day they do not, so the tokeniser, the token set
//! of a record and the ranking live beside the record's own type and
//! nowhere else (PROP-005 §2.12 `##TEXT-INDEX`, §2.14 `##INT-SEARCH`).
//!
//! A token is a lowercased ASCII alphanumeric run of at least two
//! characters, minus a ~30-word stopword list (the same list
//! `vibe-check::activation_conflict` uses, deliberately reused).
//! Scoring is term overlap — one point per distinct query token the
//! scored version carries — and ties are broken by the `(group, name)`
//! identity in lexicographic order. Good enough for the indexed scale
//! PROP-005 targets (≤ 10k packages); a stored postings index is the
//! lever if that ever moves.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#types");

use std::collections::BTreeSet;

use semver::Version;

use crate::generated::shared::{Group, PackageKind, VersionEntry};

/// The words a query and a record both drop before they are compared.
pub const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is", "it",
    "its", "of", "on", "or", "she", "that", "the", "this", "to", "was", "were", "with", "you",
    "your",
];

/// One package matched by a query: its identity, the version that was
/// scored, and what matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageHit {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    /// The newest non-prerelease version among the ones offered, or
    /// `None` when every one is a prerelease.
    pub latest_stable: Option<Version>,
    /// How many distinct query tokens the scored version carries.
    pub score: usize,
    /// Those tokens, in lexicographic order.
    pub matched_tokens: Vec<String>,
    pub description: Option<String>,
}

/// Split `text` into the tokens a query and a record are compared by.
///
/// ```
/// use vibe_wire::behaviour::index_search::tokenise;
///
/// assert_eq!(tokenise("The Write-Ahead LOG"), ["write", "ahead", "log"]);
/// assert!(tokenise("a of the").is_empty());
/// ```
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

/// The tokens a record is found by: its name, description, keywords,
/// the capabilities it advertises and the PURL it describes.
pub fn entry_tokens(entry: &VersionEntry) -> Vec<String> {
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

/// Score every package against `query` and rank the hits.
///
/// `packages` yields, per package, the versions the caller can act on,
/// in ascending order — whether a version may be acted on is the
/// reader's judgement (`must_understand`, PROP-044 §4.5) and never this
/// function's. The version scored is the newest non-prerelease one, or
/// the newest of all when every one is a prerelease; a package that
/// offers no version is skipped, as is one whose scored version is not
/// of `kind_filter`. The result is ranked by score descending, then by
/// `(group, name)` ascending. A query with no token left after the
/// stopwords matches nothing.
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use vibe_wire::behaviour::index_search::search_packages;
/// use vibe_wire::generated::shared::{Group, PackageKind, VersionEntry};
///
/// let at = Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0).unwrap();
/// let group = Group::parse("org.example").unwrap();
/// let mut wal = VersionEntry::minimal(
///     PackageKind::Flow, group.clone(), "wal", "0.1.0".parse().unwrap(), at,
/// );
/// wal.description = Some("Write-ahead log.".to_string());
/// let redbook = VersionEntry::minimal(
///     PackageKind::Flow, group, "redbook", "1.0.0".parse().unwrap(), at,
/// );
///
/// let hits = search_packages([vec![&wal], vec![&redbook]], "wal log", None);
/// assert_eq!(hits.len(), 1);
/// assert_eq!(hits[0].name, "wal");
/// assert_eq!(hits[0].score, 2);
/// assert_eq!(hits[0].matched_tokens, ["log", "wal"]);
/// ```
pub fn search_packages<'a, P, V>(
    packages: P,
    query: &str,
    kind_filter: Option<&PackageKind>,
) -> Vec<PackageHit>
where
    P: IntoIterator<Item = V>,
    V: IntoIterator<Item = &'a VersionEntry>,
{
    let query_tokens: BTreeSet<String> = tokenise(query).into_iter().collect();
    if query_tokens.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for versions in packages {
        let ascending: Vec<&VersionEntry> = versions.into_iter().collect();
        if let Some(hit) = score_package(&query_tokens, kind_filter, &ascending) {
            hits.push(hit);
        }
    }
    rank(&mut hits);
    hits
}

/// Score one package — its versions ascending — or `None` when nothing
/// of it matches.
fn score_package(
    query_tokens: &BTreeSet<String>,
    kind_filter: Option<&PackageKind>,
    ascending: &[&VersionEntry],
) -> Option<PackageHit> {
    let latest_stable = ascending.iter().rev().find(|v| v.version.pre.is_empty());
    let scored = latest_stable.or_else(|| ascending.last())?;
    // `kind` is per-version metadata (PROP-008 §2.3) — filter on the
    // version actually scored.
    if kind_filter.is_some_and(|k| scored.kind != *k) {
        return None;
    }
    let tokens: BTreeSet<String> = entry_tokens(scored).into_iter().collect();
    let matched_tokens: Vec<String> = query_tokens.intersection(&tokens).cloned().collect();
    if matched_tokens.is_empty() {
        return None;
    }
    Some(PackageHit {
        kind: scored.kind.clone(),
        group: scored.group.clone(),
        name: scored.name.clone(),
        latest_stable: latest_stable.map(|v| v.version.clone()),
        score: matched_tokens.len(),
        matched_tokens,
        description: scored.description.clone(),
    })
}

/// Order hits the way every answering surface presents them: score
/// descending, then the `(group, name)` identity ascending.
pub fn rank(hits: &mut [PackageHit]) {
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.group.cmp(&b.group))
            .then(a.name.cmp(&b.name))
    });
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use super::*;

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0).unwrap()
    }

    fn entry(group: &str, name: &str, version: &str, kind: PackageKind) -> VersionEntry {
        VersionEntry::minimal(
            kind,
            Group::parse(group).unwrap(),
            name,
            version.parse().unwrap(),
            at(),
        )
    }

    #[test]
    fn tokenise_lowercases_and_drops_stopwords() {
        let tokens = tokenise("The quick BROWN fox-jumps_OVER the lazy dog");
        assert_eq!(
            tokens,
            ["quick", "brown", "fox", "jumps", "over", "lazy", "dog"]
        );
    }

    #[test]
    fn tokenise_drops_short_tokens() {
        assert_eq!(tokenise("a b ab abc"), ["ab", "abc"]);
    }

    #[test]
    fn a_record_is_found_by_name_description_keywords_capabilities_and_purl() {
        let mut e = entry("org.example", "wal", "1.0.0", PackageKind::Flow);
        e.description = Some("Write-ahead log.".to_string());
        e.keywords = vec!["durability".to_string()];
        e.describes = Some("pkg:github/example/wal@1.0.0".to_string());
        let tokens = entry_tokens(&e);
        for expected in [
            "wal",
            "write",
            "ahead",
            "log",
            "durability",
            "pkg",
            "github",
            "example",
        ] {
            assert!(
                tokens.contains(&expected.to_string()),
                "{expected} in {tokens:?}"
            );
        }
    }

    #[test]
    fn the_newest_stable_version_is_scored_and_named() {
        let mut old = entry("org.example", "wal", "0.1.0", PackageKind::Flow);
        old.description = Some("first edition".to_string());
        let mut stable = entry("org.example", "wal", "0.2.0", PackageKind::Flow);
        stable.description = Some("second edition".to_string());
        let mut pre = entry("org.example", "wal", "0.3.0-beta.1", PackageKind::Flow);
        pre.description = Some("third edition".to_string());

        let hits = search_packages([vec![&old, &stable, &pre]], "wal edition", None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].latest_stable.as_ref().unwrap().to_string(), "0.2.0");
        assert_eq!(hits[0].description.as_deref(), Some("second edition"));
        assert_eq!(hits[0].matched_tokens, ["edition", "wal"]);
        assert_eq!(hits[0].score, 2);
    }

    #[test]
    fn a_prerelease_only_package_is_scored_by_its_newest_prerelease() {
        let a = entry("org.example", "wal", "0.1.0-alpha.1", PackageKind::Flow);
        let b = entry("org.example", "wal", "0.1.0-beta.1", PackageKind::Flow);
        let hits = search_packages([vec![&a, &b]], "wal", None);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].latest_stable.is_none());
        assert_eq!(hits[0].name, "wal");
    }

    #[test]
    fn the_kind_filter_applies_to_the_scored_version() {
        let flow = entry("org.example", "wal", "1.0.0", PackageKind::Flow);
        let tool = entry("org.example", "wal", "2.0.0", PackageKind::Tool);
        let both = vec![&flow, &tool];
        assert!(search_packages([both.clone()], "wal", Some(&PackageKind::Flow)).is_empty());
        let hits = search_packages([both], "wal", Some(&PackageKind::Tool));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, PackageKind::Tool);
    }

    #[test]
    fn hits_rank_by_score_then_identity() {
        let mut one = entry("org.zeta", "audit-log", "1.0.0", PackageKind::Feat);
        one.description = Some("Append-only audit trail.".to_string());
        let mut two_b = entry("org.beta", "wal", "1.0.0", PackageKind::Flow);
        two_b.description = Some("Write-ahead log.".to_string());
        let mut two_a = entry("org.alpha", "wal", "1.0.0", PackageKind::Flow);
        two_a.description = Some("Another write-ahead log.".to_string());

        let hits = search_packages([vec![&one], vec![&two_b], vec![&two_a]], "wal log", None);
        let order: Vec<(String, usize)> = hits
            .iter()
            .map(|h| (format!("{}/{}", h.group, h.name), h.score))
            .collect();
        assert_eq!(
            order,
            [
                ("org.alpha/wal".to_string(), 2),
                ("org.beta/wal".to_string(), 2),
                ("org.zeta/audit-log".to_string(), 1),
            ]
        );
    }

    #[test]
    fn a_query_of_stopwords_matches_nothing_and_an_empty_package_is_skipped() {
        let e = entry("org.example", "the", "1.0.0", PackageKind::Flow);
        assert!(search_packages([vec![&e]], "the of", None).is_empty());
        let none: Vec<&VersionEntry> = Vec::new();
        assert!(search_packages([none], "wal", None).is_empty());
    }
}
