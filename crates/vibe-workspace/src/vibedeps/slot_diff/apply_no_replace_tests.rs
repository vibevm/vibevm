#[cfg(test)]
mod cases {
    use super::super::*;
    use crate::{
        Workspace,
        install::{ResolvedDep, apply_resolution_with_spec_format},
        vibedeps,
    };
    use vibe_core::{
        Group, PackageKind,
        manifest::{Manifest, SpecFormat},
        user_config::SlotIntegrity,
    };

    fn fixture(hardlink: bool) -> (tempfile::TempDir, Workspace, tempfile::TempDir, ResolvedDep) {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("vibe.toml"),
            "[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\n",
        )
        .unwrap();
        let workspace = Workspace::load(root.path()).unwrap();
        let source = tempfile::tempdir().unwrap();
        let mode = if hardlink { "hardlink" } else { "copy" };
        fs::write(source.path().join("vibe.toml"), format!("[package]\ngroup='org.applyrace'\nname='sample'\nversion='1.0.0'\nkind='flow'\nsources=[]\nmaterialization='{mode}'\n")).unwrap();
        fs::write(source.path().join("seed"), b"seed").unwrap();
        let dep = ResolvedDep {
            kind: PackageKind::Flow,
            group: Group::parse("org.applyrace").unwrap(),
            name: "sample".to_owned(),
            version: "1.0.0".parse().unwrap(),
            content_dir: source.path().to_owned(),
            source_hash: Some(
                ContentHash::parse(
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                )
                .unwrap(),
            ),
            manifest: Manifest::read(source.path().join("vibe.toml")).unwrap(),
            embedded_sources: Vec::new(),
            requires: Vec::new(),
            admitted_by: None,
            via_override: None,
            source_mutable: true,
            in_place_changed: None,
            configuration: None,
        };
        (root, workspace, source, dep)
    }
    fn apply(
        workspace: &Workspace,
        dep: &ResolvedDep,
    ) -> Result<crate::install::InstallOutcome, WorkspaceError> {
        apply_resolution_with_spec_format(
            workspace,
            std::slice::from_ref(dep),
            SlotIntegrity::TrustPresence,
            SpecFormat::Mixed,
            None,
            None,
        )
    }
    #[test]
    fn m_06_a_3_public_apply_preserves_racing_payload_instead_of_rollback_removal() {
        for hardlink in [false, true] {
            let (root, workspace, source, mut dep) = fixture(hardlink);
            apply(&workspace, &dep).unwrap();
            fs::write(source.path().join("new.payload"), b"source").unwrap();
            dep.source_hash = Some(
                ContentHash::parse(
                    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                )
                .unwrap(),
            );
            let slot = vibedeps::slot_abs_path(root.path(), &dep.group, &dep.name, &dep.version);
            let old_record = fs::read(slot.join(SLOT_RECORD_FILENAME)).unwrap();
            placement::inject::arm(Box::new(|path| {
                assert!(path.ends_with("new.payload"));
                fs::write(path, b"entrant").unwrap();
            }));
            assert!(apply(&workspace, &dep).is_err());
            assert_eq!(fs::read(slot.join("new.payload")).unwrap(), b"entrant");
            assert_eq!(
                fs::read(slot.join(SLOT_RECORD_FILENAME)).unwrap(),
                old_record
            );
            assert_eq!(
                fs::read(source.path().join("new.payload")).unwrap(),
                b"source"
            );
        }
    }
    #[test]
    fn m_06_a_3_public_apply_preserves_record_entrant_and_truthful_partial_payload() {
        let (root, workspace, _source, dep) = fixture(false);
        placement::inject::arm(Box::new(|path| {
            fs::write(
                path.parent().unwrap().join(SLOT_RECORD_FILENAME),
                b"entrant record",
            )
            .unwrap();
        }));
        assert!(apply(&workspace, &dep).is_err());
        let slot = vibedeps::slot_abs_path(root.path(), &dep.group, &dep.name, &dep.version);
        assert_eq!(
            fs::read(slot.join(SLOT_RECORD_FILENAME)).unwrap(),
            b"entrant record"
        );
        assert_eq!(fs::read(slot.join("seed")).unwrap(), b"seed");
        assert!(slot.join("vibe.toml").is_file());
    }
}
