//! Lock-selected descriptor package facts from exact Mixed slot bytes.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};
use vibe_facts::SourceFileWitness;
use vibe_wire::generated::requirements_report::{
    FactStatusState, RequirementSourceKind, SourceResultState,
};

use crate::digest::source_result_digest;
use crate::tests_query::{ctx, project, write_lock};
use crate::{RequirementsQuery, query};

fn write(root: &Path, rel: &Path, bytes: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn fixture(format: SpecFormat) -> (TempDir, Vec<(String, Vec<u8>)>, PathBuf) {
    let root = project("# Host\n\n@fact:HOST Host. @status:spec/done\n");
    write_lock(root.path(), false);
    let source = TempDir::new().unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let rule = specs.join("RULE.md");
    let ordinary = specs.join("guide.md");
    let xml_data = specs.join("guide.xml");
    let forward = |path: &Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\ngroup='org.example'\nname='pkg'\nkind='feat'\nversion='1.0.0'\nformat='normal'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n",
        forward(&ordinary),
        forward(&rule),
        forward(&xml_data),
    );
    write(source.path(), Path::new("vibe.toml"), manifest.as_bytes());
    let bodies = vec![
        (forward(&rule), b"# Rule {#root}\n\n@fact:ACTIVE Active. @status:impl/done\n\n@fact:VOID Old. @status:spec/void\n".to_vec()),
        (forward(&ordinary), b"# Guide\n\n@fact:FAKE ordinary prose. #use is literal.\n".to_vec()),
        (forward(&xml_data), b"<spec xmlns='https://vibevm.org/spec/1'><p><FAKE fact='true'>not Vibe data</FAKE></p></spec>\n".to_vec()),
    ];
    for (path, bytes) in &bodies {
        write(source.path(), Path::new(path), bytes);
    }
    let group = Group::parse("org.example").unwrap();
    let version = vibe_core::manifest::Manifest::parse_str(&manifest)
        .unwrap()
        .package
        .unwrap()
        .version;
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root.path(),
        &group,
        "pkg",
        &version,
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        format,
        &hash,
    )
    .unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "pkg", &version);
    (root, bodies, slot)
}

fn package<'a>(
    report: &'a vibe_wire::generated::requirements_report::RequirementsReport,
) -> &'a vibe_wire::generated::requirements_report::SourceResult {
    report
        .sources
        .iter()
        .find(|row| row.source.package == "org.example/pkg")
        .unwrap()
}

#[test]
fn declared_mixed_query_uses_only_selected_vibe_facts_and_exact_source_digest() {
    let (root, bodies, _) = fixture(SpecFormat::Mixed);
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let source = package(&report);
    assert_eq!(source.state, SourceResultState::Available);
    let mut witnesses = bodies
        .iter()
        .map(|(path, bytes)| SourceFileWitness::of(path, bytes))
        .collect::<Vec<_>>();
    witnesses.sort_by(|left, right| left.path.cmp(&right.path));
    assert_eq!(
        source.digest.as_deref(),
        Some(
            source_result_digest(
                &RequirementSourceKind::Package,
                "org.example/pkg",
                &witnesses
            )
            .as_str()
        )
    );
    let package_rows = report
        .rows
        .iter()
        .filter(|row| row.source.package == "org.example/pkg")
        .collect::<Vec<_>>();
    assert_eq!(package_rows.len(), 2, "{package_rows:?}");
    assert!(
        package_rows
            .iter()
            .any(|row| row.address.ends_with("#ACTIVE"))
    );
    assert!(package_rows.iter().any(|row| {
        row.address.ends_with("#VOID")
            && row
                .authoring
                .status
                .as_ref()
                .is_some_and(|status| status.state == FactStatusState::Void)
    }));
    assert!(
        report
            .rows
            .iter()
            .all(|row| !row.address.ends_with("#FAKE"))
    );
}

#[test]
fn declared_query_refuses_foreign_hash_and_changed_output_without_suffix_fallback() {
    let (root, _, slot) = fixture(SpecFormat::Mixed);
    let lock = root.path().join("vibe.lock");
    let text = fs::read_to_string(&lock).unwrap();
    fs::write(&lock, text.replace(&"a".repeat(64), &"b".repeat(64))).unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Unavailable);
    assert_eq!(
        package(&report).reason_code.as_deref(),
        Some("declared-slot-unverified")
    );
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );

    fs::write(&lock, text).unwrap();
    fs::write(
        slot.join(vibe_core::layout::current_specs_root())
            .join("guide.xml"),
        b"changed raw data",
    )
    .unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Unavailable);
    assert!(package(&report).digest.is_none());
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );
}

#[test]
fn transformed_descriptor_slot_refuses_original_facts_until_recipe_exists() {
    let (root, _, _) = fixture(SpecFormat::Xml);
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Unavailable);
    assert_eq!(
        package(&report).reason_code.as_deref(),
        Some("declared-original-unavailable")
    );
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );
}

#[test]
fn missing_exact_source_is_invalid_with_only_captured_bytes_in_digest() {
    let (root, bodies, slot) = fixture(SpecFormat::Mixed);
    let missing = &bodies[0].0;
    fs::remove_file(slot.join(missing)).unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let result = package(&report);
    assert_eq!(result.state, SourceResultState::Invalid);
    assert_eq!(
        result.reason_code.as_deref(),
        Some("declared-source-incomplete")
    );
    let mut captured = bodies[1..]
        .iter()
        .map(|(path, bytes)| SourceFileWitness::of(path, bytes))
        .collect::<Vec<_>>();
    captured.sort_by(|left, right| left.path.cmp(&right.path));
    let expected = source_result_digest(
        &RequirementSourceKind::Package,
        "org.example/pkg",
        &captured,
    );
    assert_eq!(result.digest.as_deref(), Some(expected.as_str()));
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );
}

#[test]
fn directory_default_refuses_complete_facts_claim() {
    let root = project("# Host\n\n@fact:HOST Host.\n");
    write_lock(root.path(), false);
    let source = TempDir::new().unwrap();
    let manifest = "[package]\ngroup='org.example'\nname='pkg'\nkind='feat'\nversion='1.0.0'\nformat='normal'\n\
        [[package.sources]]\ndirectory='vibevm/vibespecs'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n";
    write(source.path(), Path::new("vibe.toml"), manifest.as_bytes());
    write(
        source.path(),
        &vibe_core::layout::current_specs_root().join("RULE.md"),
        b"# Rule\n\n@fact:A A. @status:spec/done\n",
    );
    let group = Group::parse("org.example").unwrap();
    let version = vibe_core::manifest::Manifest::parse_str(manifest)
        .unwrap()
        .package
        .unwrap()
        .version;
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root.path(),
        &group,
        "pkg",
        &version,
        source.path(),
        vibe_workspace::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Unavailable);
    assert_eq!(
        package(&report).reason_code.as_deref(),
        Some("declared-source-incomplete")
    );
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );
}

#[test]
fn missing_manifest_cannot_downgrade_descriptor_slot_to_suffix_scan() {
    let (root, _, slot) = fixture(SpecFormat::Mixed);
    fs::remove_file(slot.join("vibe.toml")).unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Unavailable);
    assert_eq!(
        package(&report).reason_code.as_deref(),
        Some("declared-slot-unverified")
    );
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );
}

#[cfg(unix)]
#[test]
fn symlinked_manifest_cannot_downgrade_descriptor_slot_to_suffix_scan() {
    use std::os::unix::fs::symlink;
    let (root, _, slot) = fixture(SpecFormat::Mixed);
    let manifest = fs::read(slot.join("vibe.toml")).unwrap();
    fs::remove_file(slot.join("vibe.toml")).unwrap();
    fs::write(slot.join("replacement.toml"), manifest).unwrap();
    symlink("replacement.toml", slot.join("vibe.toml")).unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Unavailable);
    assert_eq!(
        package(&report).reason_code.as_deref(),
        Some("declared-slot-unverified")
    );
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );
}

#[test]
fn descriptor_rows_removed_from_installed_manifest_cannot_enable_suffix_scan() {
    let (root, _, slot) = fixture(SpecFormat::Mixed);
    fs::write(slot.join("vibe.toml"),
        "[package]\ngroup='org.example'\nname='pkg'\nkind='feat'\nversion='1.0.0'\nformat='normal'\n")
        .unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Unavailable);
    assert_eq!(
        package(&report).reason_code.as_deref(),
        Some("declared-slot-unverified")
    );
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package != "org.example/pkg")
    );
}

#[test]
fn validated_in_place_legacy_without_record_keeps_suffix_route() {
    let root = project("# Host\n\n@fact:HOST Host.\n");
    write_lock(root.path(), true);
    let group = Group::parse("org.example").unwrap();
    let slot = vibe_workspace::vibedeps::in_place_slot_abs_path(root.path(), &group, "pkg");
    write(
        &slot,
        Path::new("vibe.toml"),
        b"[package]\ngroup='org.example'\nname='pkg'\nkind='feat'\nversion='1.0.0'\n",
    );
    write(
        &slot,
        &vibe_core::layout::current_specs_root().join("RULE.md"),
        b"# Rule\n\n@fact:LEGACY Legacy. @status:spec/done\n",
    );
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(package(&report).state, SourceResultState::Available);
    assert!(
        report
            .rows
            .iter()
            .any(|row| row.address.ends_with("#LEGACY"))
    );
}
