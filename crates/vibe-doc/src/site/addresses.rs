//! `latest`, the languages, and what a sitemap covers — recomputed at
//! every build (PROP-057 `##SITE-CANONICAL-LATEST`,
//! `##SITE-VERSION-SHOWS-CURRENT`, `##SEO-CANONICAL-HREFLANG`,
//! `##SEO-SITEMAP`).
//!
//! `latest` is an alias and never a publication: it names the newest
//! version by semantic version, within one language, of whatever the
//! sources publish NOW. Nothing records which version held the alias
//! last time, because a version that stopped being the newest did not
//! move — something newer appeared (§14).
//!
//! ## Within one language, and why that qualifier is load-bearing
//!
//! An adaptation is a package of its own with its own version numbers
//! (`##LOC-PACKAGE-PER-LANGUAGE`), and its numbering has nothing to do
//! with the source's. So the alias is computed per coordinate AND
//! language: a Russian adaptation at `0.9.0` beside an English source at
//! `2.0.0` are two `latest` addresses, not one race.
//!
//! ## What this module refuses to decide
//!
//! It does not write the sitemap, the canonical link or the `hreflang`
//! annotations. Those are `<head>` and XML the site package emits from
//! the manifests it is handed, and a second writer of them here would be
//! a second answer with no way to tell which one a crawler read. What it
//! does is compute the FACTS those tags are made of, so the count can be
//! stated by the builder and checked against the site's own.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-CANONICAL-LATEST");

use std::collections::BTreeMap;

/// One published version, as the address map sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    pub coordinate: String,
    pub version: String,
    /// The language its pages are written in.
    pub lang: String,
    /// How many pages it carries.
    pub pages: usize,
}

/// What the site's addresses come to, this build.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Addresses {
    /// `(coordinate, language)` → the version the alias names.
    pub latest: BTreeMap<(String, String), String>,
    /// Coordinate → every language it is readable in, the source's
    /// first.
    pub languages: BTreeMap<String, Vec<String>>,
    /// How many page addresses the site will carry, counting both
    /// spellings of every version.
    pub addresses: usize,
    /// Coordinates published at more than one version.
    ///
    /// Reported rather than resolved. Every version keeps its own
    /// numbered address — that is `##SITE-VERSION-SHOWS-CURRENT` — but
    /// only one of them may answer at `latest`, and the site package
    /// writes both spellings for each edition it is handed. Until it can
    /// be told which version owns the alias, a second version of one
    /// coordinate is a collision this build names instead of picking a
    /// winner silently.
    pub contested: Vec<String>,
}

/// Compute the address map of a build.
///
/// ```
/// use vibe_doc::site::addresses::{self, Published};
///
/// let map = addresses::of(&[
///     Published { coordinate: "org.example/wal".into(), version: "1.9.0".into(),
///                 lang: "en".into(), pages: 3 },
///     Published { coordinate: "org.example/wal".into(), version: "1.10.0".into(),
///                 lang: "en".into(), pages: 3 },
/// ]);
/// // Semantic version order, not the order a string comparison gives.
/// assert_eq!(
///     map.latest.get(&("org.example/wal".to_string(), "en".to_string())),
///     Some(&"1.10.0".to_string()),
/// );
/// ```
pub fn of(published: &[Published]) -> Addresses {
    let mut out = Addresses::default();
    for one in published {
        let key = (one.coordinate.clone(), one.lang.clone());
        let standing = out.latest.get(&key);
        if standing.is_none_or(|held| newer(&one.version, held)) {
            out.latest.insert(key, one.version.clone());
        }
        let languages = out.languages.entry(one.coordinate.clone()).or_default();
        if !languages.contains(&one.lang) {
            languages.push(one.lang.clone());
        }
        // Both spellings of every version, plus the package page each of
        // them carries — the count `##SITE-CANONICAL-LATEST` makes
        // unavoidable once `latest` is a materialised address and not a
        // redirect.
        out.addresses += 2 * (one.pages + 1);
    }

    let mut versions: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for one in published {
        versions
            .entry(one.coordinate.clone())
            .or_default()
            .push(&one.version);
    }
    for (coordinate, held) in versions {
        if held.len() > 1 {
            out.contested
                .push(format!("{coordinate} at {}", held.join(", ")));
        }
    }
    out
}

/// Is `candidate` a newer version than `held`?
///
/// By semantic version, which is the only ordering that puts `1.10.0`
/// after `1.9.0`. A version neither side can parse falls back to the
/// string order rather than to a panic: the alias is an address, and an
/// address is not worth stopping a site for.
fn newer(candidate: &str, held: &str) -> bool {
    match (
        candidate.parse::<semver::Version>(),
        held.parse::<semver::Version>(),
    ) {
        (Ok(candidate), Ok(held)) => candidate > held,
        _ => candidate > held,
    }
}

impl Addresses {
    /// The human form.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for contested in &self.contested {
            out.push_str(&format!(
                "  latest {contested} — one coordinate, two versions; only one may \
                 answer at `latest` and the site is not told which\n"
            ));
        }
        let languages: usize = self
            .languages
            .values()
            .flat_map(|ls| ls.iter())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        out.push_str(&format!(
            "addresses: {} alias(es), {} package(s), {languages} language(s), \
             {} page address(es)\n",
            self.latest.len(),
            self.languages.len(),
            self.addresses,
        ));
        out
    }
}

#[cfg(test)]
mod tests;
