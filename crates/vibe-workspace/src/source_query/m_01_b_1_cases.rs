//! Prepared B1 source-world cases. Snapshot reads are explicit fixture
//! preparation before construction, never work performed by the constructor.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use sha2::{Digest, Sha256};
use vibe_core::manifest::{Manifest, SourceInterpretation};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_safefs::{Project, StableFileSnapshot};
use vibe_spec::{DescriptorOutputRow, ValidatedDescriptorMap};

use super::*;

const MEMBER: &str = "[package]\ngroup='org.demo'\nname='member'\nkind='flow'\nversion='1.0.0'\n\n[requires.packages]\n'org.demo/dep'='=1.0.0'\n";
const DEP: &str = "[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n";
const DOC: &str = "# Document {#root}\n\nCAPTURED_BODY\n";

fn snapshot(root: &std::path::Path, relative: &str) -> StableFileSnapshot {
    Project::open(root)
        .unwrap()
        .read_file_snapshot_bounded(relative, 1024 * 1024)
        .unwrap()
        .unwrap()
}

fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    Manifest,
    ExtensionWorldEpoch,
    Vec<PreparedOwnerCapture>,
) {
    let root = tempfile::tempdir().unwrap();
    let member_root = root.path().join("member");
    let dep_root = crate::vibedeps::slot_abs_path(
        root.path(),
        &Group::parse("org.demo").unwrap(),
        "dep",
        &semver::Version::parse("1.0.0").unwrap(),
    );
    for path in [&member_root, &dep_root] {
        fs::create_dir_all(path.join("spec/common")).unwrap();
        fs::write(path.join("spec/common/DOC.md"), DOC).unwrap();
    }
    fs::write(member_root.join("vibe.toml"), MEMBER).unwrap();
    fs::write(dep_root.join("vibe.toml"), DEP).unwrap();
    let manifest = Manifest::parse_str(MEMBER).unwrap();
    let dep_manifest = Manifest::parse_str(DEP).unwrap();
    let source_hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    let resolved = crate::install::ResolvedDep {
        kind: PackageKind::Flow,
        group: Group::parse("org.demo").unwrap(),
        name: "dep".into(),
        version: semver::Version::parse("1.0.0").unwrap(),
        content_dir: dep_root.clone(),
        source_hash: Some(source_hash.clone()),
        embedded_sources: Vec::new(),
        manifest: dep_manifest,
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    };
    let epoch = ExtensionWorldEpoch::from_resolution(root.path(), &[resolved]).unwrap();
    let self_output = snapshot(&member_root, "spec/common/DOC.md");
    let dep_output = snapshot(&dep_root, "spec/common/DOC.md");
    let dep_manifest_snapshot = snapshot(&dep_root, "vibe.toml");
    let descriptor = ValidatedDescriptorMap::from_witnessed_rows(
        dep_root.clone(),
        dep_root.join("spec"),
        source_hash.as_str().to_owned(),
        dep_manifest_snapshot.sha256.clone(),
        vec![DescriptorOutputRow {
            output: "spec/common/DOC.md".into(),
            source: "spec/common/DOC.md".into(),
            interpretation: None,
            output_sha256: dep_output.sha256.clone(),
            document_address: None,
        }],
    )
    .unwrap();
    let captures = vec![
        PreparedOwnerCapture::self_node(
            member_root.clone(),
            snapshot(&member_root, "vibe.toml"),
            BTreeMap::from([("spec/common/DOC.md".into(), self_output)]),
            None,
        ),
        PreparedOwnerCapture::dependency(
            "org.demo",
            "dep",
            dep_root,
            dep_manifest_snapshot,
            BTreeMap::from([("spec/common/DOC.md".into(), dep_output)]),
            Some(descriptor),
        ),
    ];
    (root, member_root, manifest, epoch, captures)
}

#[test]
fn member_self_and_admitted_dependency_bind_exact_captured_roots() {
    let (root, member, manifest, epoch, captures) = fixture();
    let visible = BTreeSet::from(["org.demo/dep".into()]);
    let workspace = crate::Workspace {
        root: root.path().to_path_buf(),
        root_manifest: Manifest::parse_str("[project]\nname='root'\nversion='1.0.0'\n").unwrap(),
        members: vec![crate::WorkspaceMember {
            rel_path: "member".into(),
            manifest,
            depth: 0,
            parent: None,
        }],
    };
    assert!(matches!(
        BoundSourceInputs::from_prepared_node(
            &workspace,
            "not-a-member",
            &epoch,
            &visible,
            captures.clone(),
        ),
        Err(SourceQueryUnavailable::UnknownNode(_))
    ));
    let authority =
        BoundSourceInputs::from_prepared_node(&workspace, "member", &epoch, &visible, captures)
            .unwrap();
    assert_eq!(authority.node_root, member);
    assert_eq!(authority.host.provider.root, member);
    assert_eq!(authority.visible, visible);
    assert_eq!(authority.owner_count(), 2);
    assert_eq!(
        authority.resolution_identity,
        epoch.resolution_identity().clone()
    );
    assert!(
        authority
            .selected_root("org.demo", "dep")
            .unwrap()
            .ends_with("1.0.0")
    );
    assert_eq!(
        authority
            .captured_output(Some(("org.demo", "dep")), "spec/common/DOC.md")
            .unwrap()
            .bytes,
        DOC.as_bytes(),
    );
    assert_eq!(
        authority
            .captured_output(None, "spec/common/DOC.md")
            .unwrap()
            .bytes,
        DOC.as_bytes(),
    );
}

#[test]
fn missing_capture_is_unavailable_without_provider_or_cache_effect() {
    let (root, member, manifest, epoch, _) = fixture();
    let cache = root.path().join("must-remain-absent-cache");
    let before = fs::read(root.path().join("member/vibe.toml")).unwrap();
    let visible = BTreeSet::from(["org.demo/dep".into()]);
    assert!(matches!(
        BoundSourceInputs::from_prepared(&epoch, &member, &manifest, &visible, vec![]),
        Err(SourceQueryUnavailable::MissingPreparedCapture(_))
    ));
    assert!(!cache.exists(), "constructor created a cache path");
    assert_eq!(
        fs::read(root.path().join("member/vibe.toml")).unwrap(),
        before
    );
    // No InstallSource/registry/provider argument exists at this boundary.
}

#[test]
fn hidden_and_unselected_sources_never_gain_a_capture_grant() {
    let (root, member, manifest, epoch, captures) = fixture();
    let newer = crate::vibedeps::slot_abs_path(
        root.path(),
        &Group::parse("org.demo").unwrap(),
        "dep",
        &semver::Version::parse("2.0.0").unwrap(),
    );
    fs::create_dir_all(&newer).unwrap();
    let hidden = BoundSourceInputs::from_prepared(
        &epoch,
        &member,
        &manifest,
        &BTreeSet::new(),
        captures.clone(),
    )
    .unwrap();
    assert!(hidden.selected_root("org.demo", "dep").is_none());
    assert_eq!(hidden.owner_count(), 1);
    assert!(matches!(
        hidden.captured_output(Some(("org.demo", "dep")), "spec/common/DOC.md"),
        Err(SourceQueryUnavailable::MissingPreparedCapture(_))
    ));
    let visible = BTreeSet::from(["org.demo/dep".into()]);
    let selected =
        BoundSourceInputs::from_prepared(&epoch, &member, &manifest, &visible, captures).unwrap();
    assert_ne!(
        selected.selected_root("org.demo", "dep"),
        Some(newer.as_path())
    );
    assert!(selected.selected_root("org.demo", "unselected").is_none());
}

#[test]
fn descriptor_or_manifest_identity_mismatch_refuses_prepared_capture() {
    let (_root, member, manifest, epoch, mut captures) = fixture();
    let visible = BTreeSet::from(["org.demo/dep".into()]);
    captures[1]
        .outputs
        .get_mut("spec/common/DOC.md")
        .unwrap()
        .bytes
        .push(b'!');
    assert!(matches!(
        BoundSourceInputs::from_prepared(&epoch, &member, &manifest, &visible, captures),
        Err(SourceQueryUnavailable::InvalidCapture { .. })
    ));

    let (_root, member, manifest, epoch, mut captures) = fixture();
    let dep = &mut captures[1];
    let output = dep.outputs.get("spec/common/DOC.md").unwrap();
    dep.descriptor = Some(
        ValidatedDescriptorMap::from_witnessed_rows(
            dep.root.clone(),
            dep.root.join("spec"),
            "sha256:".to_owned() + &"a".repeat(64),
            "0".repeat(64),
            vec![DescriptorOutputRow {
                output: "spec/common/DOC.md".into(),
                source: "spec/common/DOC.md".into(),
                interpretation: Some(SourceInterpretation::VibeSpec),
                output_sha256: output.sha256.clone(),
                document_address: Some("common/DOC".into()),
            }],
        )
        .unwrap(),
    );
    assert!(matches!(
        BoundSourceInputs::from_prepared(&epoch, &member, &manifest, &visible, captures),
        Err(SourceQueryUnavailable::InvalidCapture { .. })
    ));

    let (_root, member, manifest, epoch, mut captures) = fixture();
    let dep = &mut captures[1];
    let output = dep.outputs.get("spec/common/DOC.md").unwrap();
    dep.descriptor = Some(
        ValidatedDescriptorMap::from_witnessed_rows(
            dep.root.clone(),
            dep.root.join("spec"),
            "sha256:".to_owned() + &"a".repeat(64),
            dep.manifest.sha256.clone(),
            vec![DescriptorOutputRow {
                output: "spec/common/DOC.md".into(),
                source: "spec/common/DOC.md".into(),
                interpretation: Some(SourceInterpretation::VibeSpec),
                output_sha256: output.sha256.clone(),
                document_address: Some("common/DOC".into()),
            }],
        )
        .unwrap(),
    );
    assert!(matches!(
        BoundSourceInputs::from_prepared(&epoch, &member, &manifest, &visible, captures),
        Err(SourceQueryUnavailable::InvalidCapture { .. })
    ));
}

#[test]
fn hidden_manifest_capture_cannot_widen_the_owner_visibility_input() {
    let (_root, _member, _manifest, epoch, mut captures) = fixture();
    let forged = DEP.replace("version='1.0.0'", "version='2.0.0'");
    captures[1].manifest.bytes = forged.into_bytes();
    captures[1].manifest.size = captures[1].manifest.bytes.len() as u64;
    captures[1].manifest.sha256 = format!("{:x}", Sha256::digest(&captures[1].manifest.bytes));
    assert!(matches!(
        captured_manifest_closure(&epoch, &captures),
        Err(SourceQueryUnavailable::InvalidCapture { .. })
    ));
}

#[test]
fn public_raw_visible_set_cannot_promote_private_transitive_member() {
    let root = tempfile::tempdir().unwrap();
    let group = Group::parse("org.demo").unwrap();
    let version = semver::Version::parse("1.0.0").unwrap();
    let member_root = root.path().join("member");
    let dep_root = crate::vibedeps::slot_abs_path(root.path(), &group, "dep", &version);
    let private_root = crate::vibedeps::slot_abs_path(root.path(), &group, "private", &version);
    let dep_text = format!(
        "{DEP}\n[requires.packages]\n'org.demo/private'={{version='=1.0.0',access='private'}}\n"
    );
    let private_text = DEP.replace("name='dep'", "name='private'");
    for (path, manifest) in [
        (&member_root, MEMBER),
        (&dep_root, dep_text.as_str()),
        (&private_root, private_text.as_str()),
    ] {
        fs::create_dir_all(path.join("spec/common")).unwrap();
        fs::write(path.join("vibe.toml"), manifest).unwrap();
        fs::write(path.join("spec/common/DOC.md"), DOC).unwrap();
    }
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    let make_dep =
        |name: &str, text: &str, requires: Vec<(Group, String)>| crate::install::ResolvedDep {
            kind: PackageKind::Flow,
            group: group.clone(),
            name: name.into(),
            version: version.clone(),
            content_dir: crate::vibedeps::slot_abs_path(root.path(), &group, name, &version),
            source_hash: Some(hash.clone()),
            embedded_sources: vec![],
            manifest: Manifest::parse_str(text).unwrap(),
            requires,
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        };
    let epoch = ExtensionWorldEpoch::from_resolution(
        root.path(),
        &[
            make_dep("dep", &dep_text, vec![(group.clone(), "private".into())]),
            make_dep("private", &private_text, vec![]),
        ],
    )
    .unwrap();
    let capture = |path: &std::path::Path, owner: Option<&str>| {
        let manifest = snapshot(path, "vibe.toml");
        let outputs = BTreeMap::from([(
            "spec/common/DOC.md".into(),
            snapshot(path, "spec/common/DOC.md"),
        )]);
        match owner {
            Some(name) => PreparedOwnerCapture::dependency(
                "org.demo",
                name,
                path.to_path_buf(),
                manifest,
                outputs,
                None,
            ),
            None => PreparedOwnerCapture::self_node(path.to_path_buf(), manifest, outputs, None),
        }
    };
    let captures = vec![
        capture(&member_root, None),
        capture(&dep_root, Some("dep")),
        capture(&private_root, Some("private")),
    ];
    let workspace = crate::Workspace {
        root: root.path().to_path_buf(),
        root_manifest: Manifest::parse_str("[project]\nname='root'\nversion='1.0.0'\n").unwrap(),
        members: vec![crate::WorkspaceMember {
            rel_path: "member".into(),
            manifest: Manifest::parse_str(MEMBER).unwrap(),
            depth: 0,
            parent: None,
        }],
    };
    let claimed_visible = BTreeSet::from(["org.demo/dep".into(), "org.demo/private".into()]);
    assert!(matches!(
        BoundSourceInputs::from_prepared_node(
            &workspace,
            "member",
            &epoch,
            &claimed_visible,
            captures
        ),
        Err(SourceQueryUnavailable::Visibility(_))
    ));
}
