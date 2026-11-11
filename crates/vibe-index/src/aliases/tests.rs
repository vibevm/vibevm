use super::*;

use chrono::{DateTime, Utc};

use crate::types::{NamingConvention, PackageKind, Tombstone, VersionEntry};

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-11T12:00:00Z")
        .expect("fixture timestamp parses")
        .with_timezone(&Utc)
}

fn group(s: &str) -> Group {
    Group::parse(s).expect("fixture group parses")
}

/// An index carrying `org.vibevm.qwen/qwen-code` and nothing else.
fn registry() -> Index {
    let mut index = Index::new(
        "vibespecs",
        "https://github.com/vibespecs",
        NamingConvention::Fqdn,
        now(),
    );
    index.upsert(VersionEntry::minimal(
        PackageKind::Flow,
        group("org.vibevm.qwen"),
        "qwen-code",
        "0.27.2".parse().expect("fixture version parses"),
        now(),
    ));
    index
}

fn target(g: &str, name: &str) -> AliasTarget {
    AliasTarget {
        group: group(g),
        name: name.into(),
    }
}

fn table(rows: &[(&str, &str, &str)]) -> AliasTable {
    let mut t = AliasTable::default();
    for (alias, g, name) in rows {
        t.insert(*alias, target(g, name));
    }
    t
}

fn only_fault(index: &Index, t: &AliasTable) -> AliasFault {
    let mut faults = check(index, t);
    assert_eq!(faults.len(), 1, "expected exactly one fault: {faults:?}");
    faults.remove(0)
}

#[test]
fn a_table_round_trips_through_its_own_text() {
    let t = table(&[("qwen", "org.vibevm.qwen", "qwen-code")]);
    let text = t.render();
    assert!(
        text.contains("qwen = \"org.vibevm.qwen/qwen-code\""),
        "the owner's own row shape is what is written:\n{text}"
    );
    assert_eq!(AliasTable::parse(&text).expect("renders parseable"), t);
}

#[test]
fn an_absent_file_is_an_empty_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let t = AliasTable::read(dir.path()).expect("an absent table reads");
    assert!(t.is_empty());
}

#[test]
fn an_emptied_table_leaves_no_file_behind() {
    // The listing hashes the file's bytes, so a registry that drops its
    // last alias must look exactly like one that never had any —
    // otherwise an empty table would keep every old client reprojecting
    // for ever over a feature nobody uses any more.
    let dir = tempfile::tempdir().expect("tempdir");
    table(&[("qwen", "org.vibevm.qwen", "qwen-code")])
        .write(dir.path())
        .expect("write");
    assert!(path(dir.path()).is_file());
    AliasTable::default()
        .write(dir.path())
        .expect("write empty");
    assert!(!path(dir.path()).exists());
}

#[test]
fn a_future_schema_is_refused_by_name() {
    let err = AliasTable::parse("schema = 2\n[aliases]\n").expect_err("must refuse");
    assert!(err.contains("schema = 2"), "{err}");
    assert!(err.contains("upgrade vibe"), "{err}");
}

#[test]
fn an_unknown_key_is_refused_rather_than_ignored() {
    let err = AliasTable::parse("schema = 1\nalias = { qwen = \"a.b/c\" }\n")
        .expect_err("a misspelt table is refused");
    assert!(err.contains("alias"), "the refusal names the key: {err}");
}

#[test]
fn a_target_without_a_group_is_refused() {
    let err = AliasTable::parse("schema = 1\n[aliases]\nqwen = \"qwen-code\"\n")
        .expect_err("must refuse");
    assert!(err.contains("carries no group"), "{err}");
}

#[test]
fn a_target_pinning_a_version_is_refused() {
    let err = AliasTable::parse("schema = 1\n[aliases]\nqwen = \"a.b/qwen-code@1.0.0\"\n")
        .expect_err("must refuse");
    assert!(err.contains("pins a version"), "{err}");
}

#[test]
fn a_duplicate_alias_is_refused_by_the_file_format_itself() {
    // "one target per alias" needs no rule of ours: a TOML table cannot
    // hold the same key twice.
    let err = AliasTable::parse("schema = 1\n[aliases]\nqwen = \"a.b/c\"\nqwen = \"a.b/d\"\n")
        .expect_err("must refuse");
    assert!(err.to_lowercase().contains("duplicate"), "{err}");
}

#[test]
fn a_well_formed_row_has_no_fault() {
    assert!(
        check(
            &registry(),
            &table(&[("qwen", "org.vibevm.qwen", "qwen-code")])
        )
        .is_empty()
    );
}

#[test]
fn an_alias_that_is_not_a_package_name_is_refused() {
    let t = table(&[("Qwen!", "org.vibevm.qwen", "qwen-code")]);
    let fault = only_fault(&registry(), &t);
    assert!(matches!(fault, AliasFault::Grammar { .. }), "{fault:?}");
    assert!(fault.to_string().contains("same grammar"), "{fault}");
}

#[test]
fn an_alias_naming_a_package_this_registry_lacks_is_refused() {
    let t = table(&[("qwen", "org.vibevm.qwen", "absent")]);
    let fault = only_fault(&registry(), &t);
    assert!(
        matches!(fault, AliasFault::TargetAbsent { .. }),
        "{fault:?}"
    );
    assert!(
        fault.to_string().contains("points INTO the registry"),
        "{fault}"
    );
}

#[test]
fn an_alias_that_shadows_a_real_package_name_is_refused() {
    // The rule that keeps `nvm` honest: once somebody publishes a
    // package actually called `nvm`, no registry may offer `nvm` as a
    // spelling for something else.
    let mut index = registry();
    index.upsert(VersionEntry::minimal(
        PackageKind::Flow,
        group("org.vibevm.nvm"),
        "nvm",
        "0.41.0".parse().expect("fixture version parses"),
        now(),
    ));
    let t = table(&[("nvm", "org.vibevm.qwen", "qwen-code")]);
    let fault = only_fault(&index, &t);
    assert!(
        matches!(fault, AliasFault::ShadowsPackage { .. }),
        "{fault:?}"
    );
    assert!(fault.to_string().contains("already the name"), "{fault}");
}

#[test]
fn an_alias_of_an_alias_is_refused() {
    let mut t = table(&[("qwen", "org.vibevm.qwen", "qwen-code")]);
    t.insert("q", target("org.vibevm.qwen", "qwen"));
    let fault = only_fault(&registry(), &t);
    assert!(
        matches!(fault, AliasFault::AliasOfAlias { .. }),
        "{fault:?}"
    );
    assert!(fault.to_string().contains("one step"), "{fault}");
}

#[test]
fn an_alias_of_a_buried_name_is_refused() {
    let mut index = registry();
    index.tombstones.insert(
        "qwen".into(),
        Tombstone {
            reason: "closed".into(),
            superseded_by: None,
        },
    );
    let t = table(&[("qwen", "org.vibevm.qwen", "qwen-code")]);
    let fault = only_fault(&index, &t);
    assert!(matches!(fault, AliasFault::Buried { .. }), "{fault:?}");
    assert!(fault.to_string().contains("tombstone"), "{fault}");
}

#[test]
fn a_reader_drops_the_faulty_row_and_keeps_the_sound_one() {
    // The asymmetry with `check`: somebody else's typo must not take a
    // whole registry down for everyone who reads it.
    let mut index = registry();
    let t = table(&[
        ("qwen", "org.vibevm.qwen", "qwen-code"),
        ("ghost", "org.vibevm.qwen", "absent"),
    ]);
    let faults = attach(&mut index, t);
    assert_eq!(faults.len(), 1);
    assert_eq!(faults[0].alias(), "ghost");
    assert_eq!(index.aliases.len(), 1);
    assert_eq!(
        spell(index.aliases.get("qwen").expect("the sound row stands")),
        "org.vibevm.qwen/qwen-code"
    );
}

#[test]
fn an_alias_reaches_the_catalog_as_a_pointer_beside_an_empty_candidate_set() {
    // The compatibility shape, asserted on the bytes a client fetches.
    let dir = tempfile::tempdir().expect("tempdir");
    let mut index = registry();
    attach(
        &mut index,
        table(&[("qwen", "org.vibevm.qwen", "qwen-code")]),
    );
    index
        .write_to(dir.path(), &crate::index::memory::WriteCtx { at: now() })
        .expect("catalog writes");

    let entry = crate::index::by_name::read(dir.path(), "qwen").expect("the alias file exists");
    assert!(
        entry.packages.is_empty(),
        "a reader that cannot follow the pointer must see no candidate, \
         never a package called `qwen`: {:?}",
        entry.packages
    );
    let alias = entry.alias.expect("the pointer is there");
    assert_eq!(spell(&alias), "org.vibevm.qwen/qwen-code");

    // And the target's own file is untouched by the alias.
    let target_entry =
        crate::index::by_name::read(dir.path(), "qwen-code").expect("the target file exists");
    assert!(target_entry.alias.is_none());
    assert_eq!(target_entry.packages.len(), 1);
}

#[test]
fn the_catalog_round_trips_the_alias_back_into_memory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut index = registry();
    attach(
        &mut index,
        table(&[("qwen", "org.vibevm.qwen", "qwen-code")]),
    );
    index
        .write_to(dir.path(), &crate::index::memory::WriteCtx { at: now() })
        .expect("catalog writes");
    let back = Index::load_from(dir.path()).expect("catalog loads");
    assert_eq!(back.aliases, index.aliases);
}

#[test]
fn an_aliasless_registry_writes_the_bytes_it_always_wrote() {
    // The feature must cost a registry that does not use it nothing at
    // all — not one file, not one byte.
    let plain = tempfile::tempdir().expect("tempdir");
    let aliased = tempfile::tempdir().expect("tempdir");
    let ctx = crate::index::memory::WriteCtx { at: now() };
    registry().write_to(plain.path(), &ctx).expect("writes");
    let mut index = registry();
    attach(&mut index, AliasTable::default());
    index.write_to(aliased.path(), &ctx).expect("writes");
    for file in ["repomd.json", "primary.jsonl", "by-name/qwen-code.json"] {
        assert_eq!(
            std::fs::read(plain.path().join(file)).expect("read"),
            std::fs::read(aliased.path().join(file)).expect("read"),
            "`{file}` must be byte-identical with an empty alias table"
        );
    }
}
