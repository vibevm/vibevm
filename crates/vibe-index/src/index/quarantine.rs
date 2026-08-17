//! The reader's side of `must_understand` (PROP-044 §4.5): what this
//! build can honour, and the quarantine record it keeps of catalog
//! entries it refused to act on.
//!
//! Since the loader stopped dropping quarantined versions, this module
//! is also THE single home of the answer path's judgement: every
//! surface that answers asks [`is_usable`] / the `usable_*` accessors
//! here, and never reads `pkg.versions` / `pkg.latest_stable` raw. The
//! writer's path is the deliberate exception — see [`usable_versions`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#entry");

use semver::Version;
use serde::Serialize;
use vibe_core::Group;

use crate::index::Index;
use crate::types::{PackageEntry, VersionEntry};

/// The reader capabilities this build understands.
///
/// Empty today: no capability has been built yet, so any non-empty
/// `must_understand` names something this reader cannot honour. The list
/// grows as capabilities land.
///
/// NOT the same vocabulary as a package's `provides.capabilities` — these are
/// capabilities of the READER (PROP-044 §4.5), not of the package.
pub const UNDERSTOOD: &[&str] = &[];

/// A catalog record the reader refused to act on, and why.
///
/// Lives in memory only — never written to any catalog file.
#[derive(Debug, Clone)]
pub struct Quarantined {
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub missing: Vec<String>,
}

/// The capabilities of `must_understand` this build does not understand.
/// Empty result = the record may be acted on.
pub fn missing_capabilities(must_understand: &[String]) -> Vec<String> {
    must_understand
        .iter()
        .filter(|cap| !UNDERSTOOD.contains(&cap.as_str()))
        .cloned()
        .collect()
}

/// The named predicate the answer path asks: may this build act on `entry`?
///
/// Quarantine is the READER's judgement about a (record × build) pair, never
/// a property of the record — so it is derived from `must_understand` at the
/// point of use and never stored on the wire (PROP-044 §4.5).
pub fn is_usable(entry: &VersionEntry) -> bool {
    missing_capabilities(&entry.must_understand).is_empty()
}

/// Every version of `pkg` this build can act on — THE DEFAULT for any
/// surface that answers. The writer's path must NOT use it: the catalog is
/// the projection of the journal, and a reader's capabilities never shrink
/// what is written.
pub fn usable_versions(pkg: &PackageEntry) -> impl DoubleEndedIterator<Item = &VersionEntry> {
    pkg.versions.iter().filter(|v| is_usable(v))
}

/// The newest non-prerelease version this build can act on — the same rule
/// `PackageEntry::finalise` applies, narrowed to the usable set. The stored
/// `latest_stable` field is capability-blind by construction (it is written
/// into the catalog for every reader), so an answering surface asks THIS.
pub fn usable_latest_stable(pkg: &PackageEntry) -> Option<&Version> {
    usable_versions(pkg)
        .filter(|v| v.version.pre.is_empty())
        .map(|v| &v.version)
        .next_back()
}

/// Every entry of `index` this build can act on, in the same deterministic
/// order as `Index::iter_versions`.
pub fn usable_entries(index: &Index) -> impl Iterator<Item = &VersionEntry> {
    index.iter_versions().filter(|v| is_usable(v))
}

/// How many entries of `index` this build can act on.
pub fn usable_version_count(index: &Index) -> u32 {
    usable_entries(index).count() as u32
}

/// One version an answering surface refused to serve, and why.
///
/// The shape EVERY surface uses. It carries the full coordinate even
/// where the envelope around it already names the package: a row that
/// identifies itself survives being copied out of its envelope by a
/// script, and a context-dependent one does not.
#[derive(Debug, Clone, Serialize)]
pub struct Unavailable {
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub missing: Vec<String>,
    pub recipe: String,
}

/// The recipe an `unavailable` answer carries: what a person or a
/// script does about a capability this build does not have.
///
/// One home, N surfaces — the text is built here and never written as
/// a literal at a call site (PROP-044 §4.5 asks for a generated
/// recipe, and a literal per surface is N texts that drift).
///
/// DEGENERATE BY MEASUREMENT, not by omission: `UNDERSTOOD` is empty,
/// so every missing capability is one this build does not know, and
/// there is no second class of recipe to write. The per-capability
/// table this will grow into gets its first row from the first
/// capability that lands — inventing rows for capabilities that do not
/// exist would be machinery for a consumer that does not exist.
pub fn recipe_for(missing: &[String]) -> String {
    let caps = missing
        .iter()
        .map(|cap| format!("`{cap}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "this build does not understand {caps} (reader capabilities — \
         spec://org.vibevm.core/vibevm/common/PROP-044#machinery); \
         fix: update vibe-index to a build that names them, or ask for \
         a version this build can act on"
    )
}

/// Every version of `pkg` this build refuses, as answer rows.
/// The complement of `usable_versions` over the same package.
pub fn unavailable_for(pkg: &PackageEntry) -> Vec<Unavailable> {
    pkg.versions
        .iter()
        .filter(|v| !is_usable(v))
        .map(|v| {
            let missing = missing_capabilities(&v.must_understand);
            Unavailable {
                group: pkg.group.clone(),
                name: pkg.name.clone(),
                version: v.version.clone(),
                recipe: recipe_for(&missing),
                missing,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NamingConvention, PackageKind};
    use chrono::{DateTime, Utc};

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    fn entry(name: &str, version: &str) -> VersionEntry {
        VersionEntry::minimal(
            PackageKind::Flow,
            org(),
            name,
            version.parse().unwrap(),
            now(),
        )
    }

    fn quarantined(name: &str, version: &str) -> VersionEntry {
        let mut e = entry(name, version);
        e.must_understand = vec!["x".into()];
        e
    }

    /// A package finalised the way the loader leaves every package:
    /// versions sorted, `latest_stable` computed over ALL of them.
    fn package(versions: Vec<VersionEntry>) -> PackageEntry {
        let mut pkg = PackageEntry::new(org(), "wal", now());
        pkg.versions = versions;
        pkg.finalise();
        pkg
    }

    /// The primitive keeps its own two guards, and they are not the
    /// predicate's. `is_usable` only ever asks whether the list is
    /// EMPTY; the list's CONTENT is what an `unavailable` answer will
    /// carry as `missing`, so it needs an assertion of its own or the
    /// derived tests would let the content drift unnoticed.
    #[test]
    fn empty_must_understand_needs_nothing() {
        assert!(missing_capabilities(&[]).is_empty());
    }

    #[test]
    fn unknown_capability_is_reported_missing() {
        let caps: Vec<String> = vec!["x".into(), "y".into()];
        assert_eq!(
            missing_capabilities(&caps),
            vec!["x".to_string(), "y".to_string()]
        );
    }

    #[test]
    fn empty_must_understand_is_usable() {
        assert!(is_usable(&entry("wal", "0.1.0")));
    }

    #[test]
    fn unknown_capability_is_not_usable() {
        assert!(!is_usable(&quarantined("wal", "0.1.0")));
    }

    /// The accessor narrows without reshuffling: the stored order
    /// (`finalise` sorted it) survives, only the quarantined versions
    /// drop out.
    #[test]
    fn usable_versions_skips_quarantined_and_keeps_order() {
        let pkg = package(vec![
            entry("wal", "0.1.0"),
            quarantined("wal", "0.2.0"),
            entry("wal", "0.3.0"),
        ]);
        let got: Vec<String> = usable_versions(&pkg)
            .map(|v| v.version.to_string())
            .collect();
        assert_eq!(got, vec!["0.1.0".to_string(), "0.3.0".to_string()]);
    }

    /// The reason the accessor exists: when the NEWEST stable version is
    /// quarantined, the stored `latest_stable` field still names it
    /// (`finalise` is capability-blind — the field rides the catalog for
    /// every reader), while `usable_latest_stable` names the newest
    /// version THIS build can act on — the `finalise` rule (newest
    /// non-prerelease), narrowed to the usable set: a usable prerelease
    /// never wins, and an all-quarantined package answers `None`.
    #[test]
    fn usable_latest_stable_is_finalise_over_the_usable_set() {
        let pkg = package(vec![entry("wal", "0.1.0"), quarantined("wal", "0.2.0")]);
        assert_eq!(
            pkg.latest_stable.as_ref().map(|v| v.to_string()),
            Some("0.2.0".to_string())
        );
        assert_eq!(
            usable_latest_stable(&pkg).map(|v| v.to_string()),
            Some("0.1.0".to_string())
        );

        let with_pre = package(vec![entry("wal", "0.1.0"), entry("wal", "0.4.0-rc.1")]);
        assert_eq!(
            usable_latest_stable(&with_pre).map(|v| v.to_string()),
            Some("0.1.0".to_string())
        );

        let all_quarantined = package(vec![quarantined("wal", "0.2.0")]);
        assert!(usable_latest_stable(&all_quarantined).is_none());
    }

    /// The whole-index accessors walk the same deterministic order as
    /// `Index::iter_versions` and count only what this build can act on.
    #[test]
    fn usable_entries_and_count_narrow_the_whole_index() {
        let mut idx = Index::new(
            "vibespecs",
            "https://example.invalid",
            NamingConvention::Fqdn,
            now(),
        );
        idx.upsert(entry("wal", "0.1.0"));
        idx.upsert(quarantined("wal", "0.2.0"));
        idx.upsert(entry("rust", "1.0.0"));
        assert_eq!(usable_version_count(&idx), 2);
        // `iter_versions` walks `by_pkgref` in `(group, name)` order —
        // both fixtures share `org.vibevm`, so the names order it.
        let walked: Vec<String> = usable_entries(&idx)
            .map(|v| format!("{}/{}", v.name, v.version))
            .collect();
        assert_eq!(
            walked,
            vec!["rust/1.0.0".to_string(), "wal/0.1.0".to_string()]
        );
    }

    /// The recipe names EVERY missing capability, in the order the
    /// primitive listed them, and carries both parts — the explanation
    /// with the PROP-044 anchor, and the `fix:` surface.
    #[test]
    fn recipe_names_every_capability_and_carries_both_parts() {
        let recipe = recipe_for(&["a".into(), "b".into()]);
        assert!(recipe.contains("`a`"), "recipe: {recipe}");
        assert!(recipe.contains("`b`"), "recipe: {recipe}");
        let a_first = recipe.find("`a`").unwrap() < recipe.find("`b`").unwrap();
        assert!(a_first, "the primitive's order survives: {recipe}");
        assert!(
            recipe.contains(
                "(reader capabilities — spec://org.vibevm.core/vibevm/common/PROP-044#machinery)"
            ),
            "recipe: {recipe}"
        );
        assert!(
            recipe.starts_with("this build does not understand"),
            "recipe: {recipe}"
        );
        assert!(
            recipe.contains("fix: update vibe-index to a build that names them"),
            "recipe: {recipe}"
        );
        assert!(
            !recipe.contains("violates"),
            "a refusal is legal — the accusative verb belongs to gates, not answers: {recipe}"
        );
    }

    /// One capability and two produce different enumerations — the
    /// recipe is built from the list, not a fixed sentence.
    #[test]
    fn recipe_distinguishes_one_capability_from_two() {
        let one = recipe_for(&["a".into()]);
        let two = recipe_for(&["a".into(), "b".into()]);
        assert!(one.contains("`a`") && !one.contains("`b`"), "one: {one}");
        assert!(two.contains("`a`") && two.contains("`b`"), "two: {two}");
        assert_ne!(one, two);
    }

    /// `unavailable_for` is the EXACT complement of `usable_versions`
    /// over the same package: lengths sum to the stored vector's, no
    /// version appears on both sides, and each row carries the full
    /// coordinate plus its own missing list and recipe.
    #[test]
    fn unavailable_for_is_the_exact_complement_of_usable_versions() {
        let pkg = package(vec![
            quarantined("wal", "0.1.0"),
            entry("wal", "0.2.0"),
            quarantined("wal", "0.3.0"),
        ]);
        let unavailable = unavailable_for(&pkg);
        let usable: Vec<String> = usable_versions(&pkg)
            .map(|v| v.version.to_string())
            .collect();
        assert_eq!(
            unavailable.len() + usable.len(),
            pkg.versions.len(),
            "complement sizes must sum to the stored vector's"
        );
        let refused: Vec<String> = unavailable.iter().map(|u| u.version.to_string()).collect();
        assert!(
            refused.iter().all(|v| !usable.contains(v)),
            "no version may be both usable and unavailable"
        );
        assert_eq!(refused, vec!["0.1.0".to_string(), "0.3.0".to_string()]);
        let first = &unavailable[0];
        assert_eq!(first.group.as_str(), "org.vibevm");
        assert_eq!(first.name, "wal");
        assert_eq!(first.missing, vec!["x".to_string()]);
        assert_eq!(first.recipe, recipe_for(&["x".into()]));
    }

    /// A package with nothing quarantined answers with an empty vector
    /// — surfaces skip the field/line entirely on this result.
    #[test]
    fn unavailable_for_is_empty_without_quarantine() {
        let pkg = package(vec![entry("wal", "0.1.0"), entry("wal", "0.2.0")]);
        assert!(unavailable_for(&pkg).is_empty());
    }
}
