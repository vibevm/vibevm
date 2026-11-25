//! Mirror aliases are checked against the packages this run will hold.

use super::*;
use vibe_index::aliases::AliasTable;
use vibe_index::index::Index;
use vibe_index::types::{AliasTarget, NamingConvention, Tombstone};
use vibe_wire::generated::registry_mirror_report::Listing;

fn index() -> Index {
    Index::new(
        "target",
        "file:///synthetic-target",
        NamingConvention::Fqdn,
        "2026-11-08T00:00:00Z".parse().unwrap(),
    )
}

fn target() -> AliasTarget {
    AliasTarget {
        group: "org.x".parse().unwrap(),
        name: "node-runtime".into(),
    }
}

fn listing() -> source::SourceListing {
    let mut aliases = AliasTable::default();
    aliases.insert("node", target());
    source::SourceListing {
        kind: Listing::Layout,
        versions: Vec::new(),
        tombstones: Vec::new(),
        aliases,
    }
}

fn planned(decision: Decision) -> plan::Planned {
    plan::Planned {
        version: source::SourceVersion::listed(
            "org.x".parse().unwrap(),
            "node-runtime",
            "1.0.0".parse().unwrap(),
            None,
            false,
        ),
        decision,
        frozen: false,
        fetched: None,
        content_hash: None,
    }
}

fn assert_dropped(merge: &crate::commands::registry::alias::MirrorMerge, alias: &str) {
    assert_eq!(merge.table.get(alias), None);
    assert!(!merge.recorded().iter().any(|(held, _)| held == alias));
    assert!(
        merge
            .dropped
            .contains(&(alias.into(), "org.x/node-runtime".into()))
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
)]
fn mirror_alias_uses_projected_target() {
    let root = tempfile::tempdir().unwrap();
    let index = index();
    let listing = listing();
    for decision in [Decision::Copy, Decision::Update] {
        let merge =
            alias_merge(&listing, &[planned(decision)], &index, root.path(), &None).unwrap();
        assert_eq!(merge.table.get("node"), Some(&target()));
        assert_eq!(
            merge.recorded(),
            vec![("node".into(), "org.x/node-runtime".into())]
        );
        assert!(!merge.dropped.iter().any(|(alias, _)| alias == "node"));
    }
}

#[test]
fn mirror_alias_uses_projected_target_missing_without_copy() {
    let root = tempfile::tempdir().unwrap();
    let index = index();
    let listing = listing();
    let merge = alias_merge(&listing, &[], &index, root.path(), &None).unwrap();
    assert_dropped(&merge, "node");
    assert_eq!(
        drop_warning("node", "org.x/node-runtime", &index, &[]),
        "alias node not carried · mirror holds no org.x/node-runtime"
    );
}

#[test]
fn mirror_alias_uses_projected_target_matches_exact_qualified_target() {
    let root = tempfile::tempdir().unwrap();
    let mut other = planned(Decision::Copy);
    other.version.group = "org.other".parse().unwrap();
    let merge = alias_merge(&listing(), &[other], &index(), root.path(), &None).unwrap();
    assert_dropped(&merge, "node");
}

#[test]
fn mirror_alias_uses_projected_target_rejects_unavailable_and_alarm() {
    let root = tempfile::tempdir().unwrap();
    let index = index();
    for decision in [
        Decision::Unavailable("missing".into()),
        Decision::Alarm("hash".into()),
    ] {
        let merge =
            alias_merge(&listing(), &[planned(decision)], &index, root.path(), &None).unwrap();
        assert_dropped(&merge, "node");
    }
}

#[test]
fn mirror_alias_uses_projected_target_preserves_buried_and_grammar_checks() {
    let root = tempfile::tempdir().unwrap();
    let mut index = index();
    index.tombstones.insert(
        "node".into(),
        Tombstone {
            reason: "retired".into(),
            superseded_by: None,
        },
    );
    let mut listing = listing();
    listing.aliases.insert("bad/name", target());
    let merge = alias_merge(
        &listing,
        &[planned(Decision::Copy)],
        &index,
        root.path(),
        &None,
    )
    .unwrap();
    assert_dropped(&merge, "node");
    assert_dropped(&merge, "bad/name");
    assert_eq!(
        drop_warning(
            "node",
            "org.x/node-runtime",
            &index,
            &[planned(Decision::Copy)]
        ),
        "alias node not carried · row does not stand in the target registry"
    );
}

#[test]
fn mirror_alias_uses_projected_target_preserves_alias_of_alias_check() {
    let root = tempfile::tempdir().unwrap();
    let mut listing = listing();
    listing.aliases.insert(
        "node-runtime",
        AliasTarget {
            group: "org.other".parse().unwrap(),
            name: "other-runtime".into(),
        },
    );
    let mut other = planned(Decision::Copy);
    other.version.group = "org.other".parse().unwrap();
    other.version.name = "other-runtime".into();
    let merge = alias_merge(
        &listing,
        &[planned(Decision::Copy), other],
        &index(),
        root.path(),
        &None,
    )
    .unwrap();
    assert_dropped(&merge, "node");
}
