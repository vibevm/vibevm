//! The refusals a prune makes before it looks at anything, and the two
//! paths it refuses to compute: a version another source claims, and a
//! snapshot the selection no longer names.
use std::collections::BTreeSet;

use super::super::config;
use super::*;

fn config_selection(body: &str) -> Selection {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mirror.toml");
    std::fs::write(&path, body).unwrap();
    let validated = config::read(&path).unwrap();
    Selection::from_config(&validated, &[dir.path().join("drop")], None).unwrap()
}

const ONE_GROUP: &str = "schema = 1\n[select]\ngroups = ['org.corp']\n";

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn every_flag_combination_that_cannot_mean_what_it_says_is_refused() {
    let declared = config_selection(ONE_GROUP);
    let flags = Selection::from_flags(&["org.corp/demo".into()], &[]).unwrap();
    let request = |prune, payloads, yes| Request {
        prune,
        payloads,
        yes,
    };

    // Nothing is ever removed without `--prune`.
    let error = check(request(false, true, false), &declared, None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("`--prune` was not given"), "{error}");

    // A flag override is not a declaration, so it never prunes.
    let error = check(request(true, false, true), &flags, None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("no declared selection"), "{error}");
    let error = check(
        request(true, false, true),
        &flags,
        Some(std::path::Path::new("mirror.toml")),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("overrode `--config` wholesale"), "{error}");

    // `--prune-payloads` needs the drop the payloads went into.
    let error = check(request(true, true, false), &declared, None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("asks for payloads"), "{error}");

    // The combinations that do mean something are silent.
    check(request(false, false, false), &flags, None).unwrap();
    check(request(true, false, false), &declared, None).unwrap();
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_version_another_source_records_as_copied_is_never_pruned() {
    let target = tempfile::tempdir().unwrap();
    let mirrors = target.path().join("_index/mirrors");
    std::fs::create_dir_all(&mirrors).unwrap();
    let row = |package: &str, version: &str| {
        let who = "{ name = 'A', email = 'a@example', date = '2026-01-01' }";
        format!(
            "[[version]]\npackage = '{package}'\nversion = '{version}'\n\
             repository = 'ssh://host/r.git'\nrepo = 'r'\ntree_verified = true\n\
             tag = 'v{version}'\ntag_kind = 'lightweight'\ncommit = 'abc'\ntree = 'def'\n\
             author = {who}\ncommitter = {who}\nmessage = 'a commit'\n"
        )
    };
    let record = |name: &str, versions: &str| {
        format!(
            "schema = 1\n[source]\nname = '{name}'\nurl = 'ssh://host/{name}'\n\
             type = 'github'\nnaming = 'fqdn'\nref = 'main'\n\
             [sync]\nvibe = 'vibe 1.0.0'\nfirst = 'now'\nlast = 'now'\n{versions}"
        )
    };
    std::fs::write(
        mirrors.join("src.toml"),
        record("src", &row("org.other/tool", "1.0.0")),
    )
    .unwrap();
    std::fs::write(
        mirrors.join("second.toml"),
        record("second", &row("org.other/tool", "2.0.0")),
    )
    .unwrap();

    // This source's own rows protect nothing — it is the one pruning.
    let protected = copied_elsewhere(target.path(), "src").unwrap();
    assert_eq!(
        protected,
        BTreeSet::from([("org.other/tool".to_owned(), "2.0.0".to_owned())])
    );
    let other = copied_elsewhere(target.path(), "second").unwrap();
    assert_eq!(
        other,
        BTreeSet::from([("org.other/tool".to_owned(), "1.0.0".to_owned())])
    );
    // A target with no mirror records at all protects nothing.
    let empty = tempfile::tempdir().unwrap();
    assert!(copied_elsewhere(empty.path(), "src").unwrap().is_empty());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_snapshot_the_selection_names_no_row_for_is_an_orphan() {
    let target = tempfile::tempdir().unwrap();
    for package in ["org.corp/kept", "org.corp/gone"] {
        std::fs::create_dir_all(
            target
                .path()
                .join("_index/templates")
                .join(package)
                .join("digests"),
        )
        .unwrap();
    }
    let selection = config_selection(
        "schema = 1\n[[template]]\npackage = 'org.corp/kept'\nversions = ['>=1']\n",
    );
    let orphans = orphan_snapshots(target.path(), &selection).unwrap();
    assert_eq!(
        orphans
            .iter()
            .map(|(package, _, _)| package.as_str())
            .collect::<Vec<_>>(),
        ["org.corp/gone"]
    );
    // A target with no snapshots at all has no orphan.
    let empty = tempfile::tempdir().unwrap();
    assert!(
        orphan_snapshots(empty.path(), &selection)
            .unwrap()
            .is_empty()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PRUNE")]
fn a_planned_removal_path_spells_the_coordinate_of_its_fact() {
    let target = tempfile::tempdir().unwrap();
    let group = vibe_core::Group::parse("org.corp").unwrap();
    let stale = vec![StaleSnapshot {
        package: "org.corp/tool".to_owned(),
        group,
        name: "tool".to_owned(),
        versions: BTreeSet::from(["1.9.0".to_owned()]),
    }];
    let selection = config_selection(
        "schema = 1\n[[template]]\npackage = 'org.corp/tool'\nversions = ['>=2']\n",
    );
    let index = vibe_index::index::Index::new(
        "dst",
        "file:///dst",
        vibe_index::types::NamingConvention::default(),
        chrono::Utc::now(),
    );
    let planned = plan(
        Request {
            prune: true,
            payloads: false,
            yes: true,
        },
        &selection,
        "src",
        target.path(),
        &index,
        &stale,
    )
    .unwrap();
    assert_eq!(planned.rows.len(), 1);
    assert_eq!(
        planned.rows[0].path,
        "_index/templates/org.corp/tool/digests/1.9.0"
    );
    assert_eq!(planned.rows[0].what, What::Snapshot);
    assert!(
        planned.rows[0]
            .reason
            .contains("no longer selected by `[[template]]` org.corp/tool"),
        "{:?}",
        planned.rows[0]
    );
    // A snapshot digest is a path inside `_index/` and names no
    // coordinate the catalog describes.
    assert_eq!(planned.removed[0].version, None);

    // And a run that did not ask plans nothing at all.
    let unasked = plan(
        Request::default(),
        &selection,
        "src",
        target.path(),
        &index,
        &stale,
    )
    .unwrap();
    assert!(unasked.is_empty());
}
