//! Unit oracles for the registry store and host-spec synchronization.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#model");

use std::collections::BTreeSet;
use std::fs;

use tempfile::tempdir;
use vibe_core::layout;

use crate::store::{read_file, write_file};
use crate::sync;
use crate::{
    FactEntry, FactOrigin, FactStatus, Registry, RegistryError, adoption_counts, authored_facts,
    orphans, package_file_path, remove_package_file,
};

fn status(value: &str) -> FactStatus {
    FactStatus::parse(value).expect("test status")
}

/// A fixture path under the facts home, `/`-separated — routed through
/// the layout seam (PROP-052 L2) so the scaffold names whichever layout
/// is live.
fn facts_rel(rel: &str) -> String {
    layout::current_vibefacts_root()
        .join(rel)
        .to_string_lossy()
        .replace('\\', "/")
}

fn package_fact(address: &str, value: &str) -> FactEntry {
    FactEntry {
        address: address.to_string(),
        origin: FactOrigin::Package,
        package: Some("org.example/pkg".to_string()),
        status: Some(status(value)),
        comment: None,
    }
}

#[test]
fn store_round_trip_and_emit_order_are_deterministic() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join(facts_rel("org.example.pkg.toml"));
    let z = package_fact("spec://org.example/pkg/flows/z#Z", "impl/done");
    let a = package_fact("spec://org.example/pkg/flows/a#A", "spec/work");

    write_file(&path, [z.clone(), a.clone()]).expect("first emit");
    let first = fs::read_to_string(&path).expect("first bytes");
    let loaded = read_file(&path).expect("parse emitted file");
    assert_eq!(loaded, vec![a.clone(), z.clone()]);
    assert!(first.find(&a.address) < first.find(&z.address));

    write_file(&path, [a, z]).expect("second emit");
    assert_eq!(fs::read_to_string(&path).expect("second bytes"), first);
}

#[test]
fn store_rejects_duplicate_garbage_status_and_unknown_keys() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("facts.toml");
    let address = "spec://org.example/pkg/flows/x#X";
    fs::write(
        &path,
        format!(
            "schema = 1\n\n[[fact]]\naddress = \"{address}\"\norigin = \"package\"\npackage = \"org.example/pkg\"\n\n[[fact]]\naddress = \"{address}\"\norigin = \"package\"\npackage = \"org.example/pkg\"\n"
        ),
    )
    .expect("duplicate fixture");
    assert!(matches!(
        read_file(&path),
        Err(RegistryError::DuplicateAddress { .. })
    ));

    fs::write(
        &path,
        format!(
            "schema = 1\n\n[[fact]]\naddress = \"{address}\"\norigin = \"package\"\npackage = \"org.example/pkg\"\nstatus = \"impl/finished\"\n"
        ),
    )
    .expect("status fixture");
    assert!(matches!(
        read_file(&path),
        Err(RegistryError::TomlRead { .. })
    ));

    fs::write(
        &path,
        format!(
            "schema = 1\nextra = true\n\n[[fact]]\naddress = \"{address}\"\norigin = \"package\"\npackage = \"org.example/pkg\"\n"
        ),
    )
    .expect("unknown-key fixture");
    assert!(matches!(
        read_file(&path),
        Err(RegistryError::TomlRead { .. })
    ));
}

#[test]
fn sync_detects_and_reconciles_host_status_without_touching_spec() {
    let temp = tempdir().expect("tempdir");
    fs::write(
        temp.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    fs::create_dir_all(
        temp.path()
            .join(layout::current_specs_root())
            .join("common"),
    )
    .expect("spec dir");
    let spec = temp
        .path()
        .join(layout::current_specs_root())
        .join("common/RULE.md");
    let spec_bytes = "# Rule {#root}\n\n@fact:RULE It is implemented. @status:impl/done\n";
    fs::write(&spec, spec_bytes).expect("spec");

    let address = "spec://org.example/demo/common/RULE#RULE";
    let mut registry = Registry::default();
    registry
        .upsert(
            temp.path(),
            FactEntry::for_host(address, "org.example/demo", Some(status("spec/work")), None)
                .expect("entry"),
        )
        .expect("write registry");

    let mismatches = sync::check(temp.path(), &registry).expect("sync check");
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].spec_status, Some(status("impl/done")));
    assert_eq!(mismatches[0].registry_status, Some(status("spec/work")));
    assert_eq!(mismatches[0].line, Some(3));

    let applied = sync::reconcile(temp.path(), &mut registry).expect("reconcile");
    assert_eq!(applied, mismatches);
    assert!(
        sync::check(temp.path(), &registry)
            .expect("clean")
            .is_empty()
    );
    assert_eq!(
        fs::read_to_string(spec).expect("spec unchanged"),
        spec_bytes
    );
}

/// The snapshot mints addresses in the router's canonical form: a
/// `PROP-NNN-descriptive-slug.md` filename addresses as `PROP-NNN`
/// (the CLAUDE.md citation style), so registry keys written from
/// canonical citations match the scanned spec.
#[test]
fn sync_addresses_use_the_canonical_slug_truncated_doc_path() {
    let temp = tempdir().expect("tempdir");
    fs::write(
        temp.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    fs::create_dir_all(
        temp.path()
            .join(layout::current_specs_root())
            .join("common"),
    )
    .expect("spec dir");
    fs::write(
        temp.path()
            .join(layout::current_specs_root())
            .join("common/PROP-099-descriptive-slug.md"),
        "# Slugged {#root}\n\n@fact:LAW The law. @status:impl/done\n",
    )
    .expect("spec");

    let mut registry = Registry::default();
    registry
        .upsert(
            temp.path(),
            FactEntry::for_host(
                "spec://org.example/demo/common/PROP-099#LAW",
                "org.example/demo",
                Some(status("impl/done")),
                None,
            )
            .expect("entry"),
        )
        .expect("write registry");

    assert!(
        sync::check(temp.path(), &registry)
            .expect("canonical address matches")
            .is_empty()
    );
}

#[test]
fn package_overlay_filters_statuses_and_hashes_exact_file_bytes() {
    let temp = tempdir().expect("tempdir");
    let mut registry = Registry::default();
    let done = package_fact("spec://org.example/pkg/spec/RULE#DONE", "impl/done");
    let mut indeterminate = package_fact(
        "spec://org.example/pkg/spec/RULE#INDETERMINATE",
        "spec/work",
    );
    indeterminate.status = None;
    registry
        .upsert(temp.path(), done.clone())
        .expect("done entry");
    registry
        .upsert(temp.path(), indeterminate.clone())
        .expect("indeterminate entry");

    let overlay = registry.package_overlay("org.example/pkg");
    assert_eq!(overlay.status_for(&done.address), done.status);
    assert_eq!(overlay.status_for(&indeterminate.address), None);
    assert!(overlay.contains_address(&indeterminate.address));
    assert!(overlay.contains_document("spec://org.example/pkg/spec/RULE#"));
    assert!(!overlay.is_empty());
    assert!(registry.package_overlay("org.example/other").is_empty());

    let first = crate::overlay_file_hash(temp.path(), "org.example/pkg").expect("hash");
    fs::write(
        temp.path().join(facts_rel("org.example.pkg.toml")),
        "schema = 1\n",
    )
    .expect("replace bytes");
    let second = crate::overlay_file_hash(temp.path(), "org.example/pkg").expect("new hash");
    assert_ne!(first, second);
    assert_eq!(
        crate::overlay_file_hash(temp.path(), "org.example/missing"),
        None
    );
}

#[test]
fn lifecycle_reports_only_uninstalled_package_files_and_prunes_empty_home() {
    let temp = tempdir().expect("tempdir");
    let mut registry = Registry::default();
    let package = package_fact("spec://org.example/pkg/spec/RULE#ONE", "impl/done");
    registry
        .upsert(temp.path(), package)
        .expect("package overlay");
    registry
        .upsert(
            temp.path(),
            FactEntry {
                address: "spec://org.consumer/demo/common/RULE#HOST".to_string(),
                origin: FactOrigin::Spec,
                package: None,
                status: Some(status("impl/done")),
                comment: None,
            },
        )
        .expect("host overlay");

    let installed = BTreeSet::from(["org.example/pkg".to_string()]);
    assert!(
        orphans(temp.path(), &installed)
            .expect("installed")
            .is_empty()
    );

    let orphaned = orphans(temp.path(), &BTreeSet::new()).expect("orphans");
    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].package, "org.example/pkg");
    assert_eq!(orphaned[0].entries, 1);
    assert_eq!(
        orphaned[0].file,
        package_file_path(temp.path(), "org.example/pkg")
    );
    assert!(remove_package_file(temp.path(), "org.example/pkg").expect("remove"));
    assert!(!remove_package_file(temp.path(), "org.example/pkg").expect("absent"));
    assert!(temp.path().join(facts_rel("spec.toml")).is_file());

    let package_only = tempdir().expect("package-only tempdir");
    let mut registry = Registry::default();
    registry
        .upsert(
            package_only.path(),
            package_fact("spec://org.example/pkg/spec/RULE#ONE", "impl/done"),
        )
        .expect("package-only overlay");
    assert!(
        remove_package_file(package_only.path(), "org.example/pkg").expect("remove package-only")
    );
    assert!(
        !package_only
            .path()
            .join(layout::current_vibefacts_root())
            .exists()
    );
}

#[test]
fn shared_pivot_counts_authored_adopted_and_indeterminate_facts() {
    let doc = vibe_specdoc::from_markdown(
        "# Rules\n\n@fact:FIRST First. @status:impl/done\n\n## Child\n\n- @fact:SECOND Second.\n",
    )
    .expect("pivot parse");
    let authored = authored_facts(&doc, "spec://org.example/pkg/RULE#");
    assert_eq!(authored.len(), 2);
    assert_eq!(authored[0].address, "spec://org.example/pkg/RULE#FIRST");
    assert_eq!(authored[0].status, Some(status("impl/done")));
    assert_eq!(authored[1].address, "spec://org.example/pkg/RULE#SECOND");
    assert_eq!(authored[1].status, None);

    let temp = tempdir().expect("tempdir");
    let mut registry = Registry::default();
    registry
        .upsert(
            temp.path(),
            package_fact("spec://org.example/pkg/RULE#FIRST", "impl/done"),
        )
        .expect("adopted entry");
    registry
        .upsert(
            temp.path(),
            FactEntry {
                address: "spec://org.example/pkg/RULE#SECOND".to_string(),
                origin: FactOrigin::Package,
                package: Some("org.example/pkg".to_string()),
                status: None,
                comment: None,
            },
        )
        .expect("indeterminate entry");
    let counts = adoption_counts(&registry, "org.example/pkg", &authored);
    assert_eq!(counts.adopted, 1);
    assert_eq!(counts.indeterminate, 1);
    assert_eq!(counts.total_authored, 2);
}
