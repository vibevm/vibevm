use super::*;

use chrono::Utc;
use std::path::Path;

use crate::index::memory::default_generator;
use crate::journal::{Event, JournalRecord, append};
use crate::types::{NamingConvention, PackageKind, VersionEntry};

fn initialised(data: &Path) {
    append(
        &default_dir(data),
        &JournalRecord {
            at: Utc::now(),
            actor: default_generator(),
            event: Event::Initialised {
                registry: "vibespecs".into(),
                registry_url: "https://example.invalid/vibespecs".into(),
                naming: NamingConvention::Fqdn,
            },
        },
    )
    .expect("journal append");
}

fn publish(data: &Path, group: &str, name: &str, version: &str) {
    append(
        &default_dir(data),
        &JournalRecord {
            at: Utc::now(),
            actor: default_generator(),
            event: Event::Published {
                entry: Box::new(VersionEntry::minimal(
                    PackageKind::Flow,
                    group.parse().expect("fixture group"),
                    name,
                    version.parse().expect("fixture version"),
                    Utc::now(),
                )),
            },
        },
    )
    .expect("journal append");
}

/// A data directory with `org.vibevm.qwen/qwen-code` published and its
/// catalog written, as any real registry would have.
fn registry(tmp: &Path) -> PathBuf {
    let data = tmp.join("data");
    initialised(&data);
    publish(&data, "org.vibevm.qwen", "qwen-code", "0.27.2");
    let index = project(replay(&default_dir(&data)).expect("replay")).expect("fold");
    // Stamped from the fold, not from a fresh clock — the same rule the
    // real write paths keep, and what makes the catalog provable.
    let at = index.generated_at;
    index
        .write_to(&data, &WriteCtx { at })
        .expect("catalog writes");
    data
}

fn add(data: &Path, alias: &str, target: &str, replace: bool) -> Result<()> {
    run(Args {
        data_dir: data.to_path_buf(),
        action: Action::Add {
            alias: alias.into(),
            target: target.into(),
            replace,
        },
    })
}

#[test]
fn add_writes_the_table_and_the_catalog_together() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());

    add(&data, "qwen", "org.vibevm.qwen/qwen-code", false).expect("the alias stands");

    let text = std::fs::read_to_string(aliases::path(&data)).expect("the table was written");
    assert!(
        text.contains("qwen = \"org.vibevm.qwen/qwen-code\""),
        "{text}"
    );
    let entry = crate::index::by_name::read(&data, "qwen").expect("the catalog carries the alias");
    assert_eq!(
        aliases::spell(&entry.alias.expect("the pointer")),
        "org.vibevm.qwen/qwen-code"
    );
    assert!(
        entry.packages.is_empty(),
        "an older client must still read `qwen` as a name nobody publishes"
    );
}

#[test]
fn remove_takes_the_row_and_the_catalog_file_with_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    add(&data, "qwen", "org.vibevm.qwen/qwen-code", false).expect("the alias stands");

    run(Args {
        data_dir: data.clone(),
        action: Action::Remove {
            alias: "qwen".into(),
        },
    })
    .expect("withdrawal");

    assert!(
        !aliases::path(&data).exists(),
        "the last row gone must leave the tree of a registry that never aliased anything"
    );
    assert!(
        crate::index::by_name::read(&data, "qwen").is_err(),
        "the alias's candidate-set file must go with it"
    );
}

#[test]
fn removing_an_alias_that_is_not_there_is_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    let error = run(Args {
        data_dir: data.clone(),
        action: Action::Remove {
            alias: "ghost".into(),
        },
    })
    .expect_err("must refuse");
    assert!(error.to_string().contains("no alias `ghost`"), "{error}");
}

#[test]
fn re_pointing_a_spelling_needs_saying_so() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    publish(&data, "com.acme", "qwen-fork", "1.0.0");
    add(&data, "qwen", "org.vibevm.qwen/qwen-code", false).expect("the alias stands");

    let error = add(&data, "qwen", "com.acme/qwen-fork", false).expect_err("must refuse");
    assert!(error.to_string().contains("already means"), "{error}");
    assert!(error.to_string().contains("--replace"), "{error}");

    add(&data, "qwen", "com.acme/qwen-fork", true).expect("--replace re-points it");
    let table = AliasTable::read(&data).expect("table reads");
    assert_eq!(
        aliases::spell(table.get("qwen").expect("row")),
        "com.acme/qwen-fork"
    );
}

#[test]
fn adding_the_same_row_twice_changes_nothing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    add(&data, "qwen", "org.vibevm.qwen/qwen-code", false).expect("the alias stands");
    let before = std::fs::read(data.join("repomd.json")).expect("read");
    add(&data, "qwen", "org.vibevm.qwen/qwen-code", false).expect("a no-op is not a failure");
    assert_eq!(
        before,
        std::fs::read(data.join("repomd.json")).expect("read"),
        "a mutation that changes nothing must change no byte"
    );
}

#[test]
fn an_alias_of_a_package_this_registry_lacks_is_refused_and_writes_nothing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    let error = add(&data, "qwen", "org.vibevm.qwen/absent", false).expect_err("must refuse");
    assert!(error.to_string().contains("would not stand"), "{error}");
    assert!(
        error.to_string().contains("points INTO the registry"),
        "{error}"
    );
    assert!(
        !aliases::path(&data).exists(),
        "a refused edit must leave no table behind"
    );
}

#[test]
fn an_alias_that_shadows_a_published_name_is_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    publish(&data, "org.vibevm.nvm", "nvm", "0.41.0");
    let error = add(&data, "nvm", "org.vibevm.qwen/qwen-code", false).expect_err("must refuse");
    assert!(error.to_string().contains("already the name"), "{error}");
}

#[test]
fn an_unqualified_or_versioned_target_is_refused_before_anything_is_read() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    let bare = add(&data, "qwen", "qwen-code", false).expect_err("must refuse");
    assert!(bare.to_string().contains("carries no group"), "{bare}");
    let pinned =
        add(&data, "qwen", "org.vibevm.qwen/qwen-code@0.27.2", false).expect_err("must refuse");
    assert!(pinned.to_string().contains("pins a version"), "{pinned}");
}

#[test]
fn a_row_the_registry_outgrew_is_named_on_the_next_edit() {
    // The whole table is re-validated on every edit, so publishing a
    // package under a name an older row already spells is caught the
    // next time the owner touches the table — by the person who can do
    // something about it.
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    add(&data, "qwen", "org.vibevm.qwen/qwen-code", false).expect("the alias stands");
    publish(&data, "com.acme", "qwen", "1.0.0");
    publish(&data, "com.acme", "other", "1.0.0");

    let error = add(&data, "oth", "com.acme/other", false).expect_err("must refuse");
    assert!(
        error.to_string().contains("`qwen` is already the name"),
        "the refusal must name the row that went bad, not the one being added: {error}"
    );
}

/// The proof that ships must keep passing on a registry that offers a
/// spelling — and it is the reason this verb runs no clock.
///
/// `rebuild --check` projects the catalog from the registry's inputs and
/// byte-compares it with what is on disk. The alias table is one of
/// those inputs and the journal is the other, so a catalog stamped with
/// `now()` by an alias edit — while the journal's last moment stayed
/// where it was — would carry a `generated_at` its inputs cannot
/// explain, and the check would fail for ever after the first alias.
#[test]
fn the_rebuild_proof_still_holds_after_an_alias_edit() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = registry(tmp.path());
    crate::rebuild::check_catalog(&data)
        .expect("the fixture catalog projects")
        .drift
        .is_clean()
        .then_some(())
        .expect("the fixture starts clean");

    add(&data, "qwen", "org.vibevm.qwen/qwen-code", false).expect("the alias stands");

    let outcome = crate::rebuild::check_catalog(&data).expect("the check runs");
    assert!(
        outcome.drift.is_clean(),
        "an alias edit must leave the catalog provable from its inputs: {:?}",
        outcome.drift.lines()
    );

    // And withdrawing it returns to a provable catalog too.
    run(Args {
        data_dir: data.clone(),
        action: Action::Remove {
            alias: "qwen".into(),
        },
    })
    .expect("withdrawal");
    let outcome = crate::rebuild::check_catalog(&data).expect("the check runs");
    assert!(
        outcome.drift.is_clean(),
        "withdrawing must leave it provable as well: {:?}",
        outcome.drift.lines()
    );
}
