    use super::*;
    use crate::commands::vvm::model::{InstallRecord, Kind, Profile};
    use crate::commands::vvm::store::{BINARY_NAME, VersionStore};

    #[test]
    fn binary_instances_do_not_require_a_source_build_toolchain() {
        assert!(!needs_build_tools(Some(Origin::Binary)));
        assert!(needs_build_tools(Some(Origin::External)));
        assert!(needs_build_tools(Some(Origin::Managed)));
        assert!(needs_build_tools(None));
    }

    fn binary_record(source_path: Option<String>) -> InstallRecord {
        InstallRecord {
            kind: Kind::Tag,
            id: "1.0.0".into(),
            instance: 1,
            commit: "a".repeat(40),
            toolchain: "prebuilt".into(),
            profile: Profile::Release,
            installed_at: "now".into(),
            origin: Origin::Binary,
            source_path,
            payload_sha256: Some("b".repeat(64)),
            distribution_manifest_sha256: None,
        }
    }

    #[test]
    fn doctor_distinguishes_legacy_imports_from_complete_and_broken_bundles() {
        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path());
        let legacy = binary_record(None);
        let home = store.instance_dir(&legacy.version_id(), legacy.instance);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(BINARY_NAME), b"legacy").unwrap();
        let legacy_layout = instance_layout(&store, &legacy);
        assert_eq!(legacy_layout.kind, "legacy-single-binary");
        assert!(
            !legacy_layout.integrity,
            "missing placement manifest is corrupt"
        );

        let source = store.instance_source_dir(&legacy.version_id(), legacy.instance);
        let modern = binary_record(Some(source.display().to_string()));
        std::fs::create_dir_all(store.instance_bin_dir(&modern.version_id(), modern.instance))
            .unwrap();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            store.index_binary_path(&modern.version_id(), modern.instance),
            b"index",
        )
        .unwrap();
        std::fs::write(home.join("DISTRIBUTION.json"), b"manifest").unwrap();
        std::fs::write(
            home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME),
            b"source zip",
        )
        .unwrap();
        let complete = instance_layout(&store, &modern);
        assert!(
            complete.kind == "bundle-unrooted"
                && complete.vibe_index
                && complete.source
                && complete.source_archive
                && complete.manifest
        );
        assert!(
            !complete.integrity,
            "dummy manifest is not a verified bundle"
        );
        assert!(!complete.manifest_root);
        std::fs::remove_dir_all(&source).unwrap();
        let broken = instance_layout(&store, &modern);
        assert_eq!(broken.kind, "bundle-unrooted");
        assert!(!broken.source);

        let mut source_record = legacy.clone();
        source_record.origin = Origin::External;
        source_record.instance = 2;
        source_record.source_path = Some(temp.path().display().to_string());
        let source_home = store.instance_dir(&source_record.version_id(), 2);
        std::fs::create_dir_all(source_home.join("bin")).unwrap();
        std::fs::write(source_home.join("bin").join(BINARY_NAME), b"vibe").unwrap();
        let source_layout = instance_layout(&store, &source_record);
        assert_eq!(source_layout.kind, "source-build");
        assert!(
            !source_layout.source,
            "an arbitrary directory is not a source root"
        );
        assert!(!source_layout.vibe_index);
        assert!(!source_layout.placement_manifest);
        assert!(!source_layout.integrity);
    }

    #[test]
    fn active_and_distinct_running_instances_are_diagnosed_separately() {
        let mut running = binary_record(None);
        let mut active = running.clone();
        running.instance = 1;
        active.instance = 2;
        let rows = diagnostic_records(Some(&active), Some(&running));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "active");
        assert_eq!(rows[0].1.instance, 2);
        assert_eq!(rows[1].0, "running");
        assert_eq!(rows[1].1.instance, 1);
    }
