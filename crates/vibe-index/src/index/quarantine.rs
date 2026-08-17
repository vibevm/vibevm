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
}
