use super::*;
#[cfg(test)]
mod cases {
    use super::*;
    use std::{collections::BTreeMap, fs};
    const MANIFEST: &str =
        "[package]\ngroup='org.source'\nname='doc'\nkind='flow'\nversion='1.0.0'\n";
    fn write(root: &Path, policy: &str) {
        fs::create_dir_all(root.join(vibe_core::layout::current_specs_root())).unwrap();
        fs::write(root.join("vibe.toml"), format!("{MANIFEST}{policy}")).unwrap();
        fs::write(
            root.join(vibe_core::layout::current_specs_root())
                .join("ONE.md"),
            "# One {#root}\nBODY\n",
        )
        .unwrap();
    }
    fn selected(root: &Path, slot: &Path) -> FileResolver {
        let mut map = BTreeMap::new();
        map.insert(
            ("org.source".into(), "doc".into()),
            SelectedPackage::new("1.0.0", slot),
        );
        FileResolver::with_selected_world(
            root,
            root,
            map,
            SelfCoordinate::new(Some("org.host".into()), "host".into()),
        )
    }
    #[test]
    fn source_policy_empty_has_no_semantic_address_and_nonempty_refuses_unsupported_lookup() {
        let root = tempfile::tempdir().unwrap();
        let slot = root
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.source.doc/1.0.0");
        write(&slot, "sources=[]\n");
        let address = SpecAddress::parse("spec://org.source/doc/ONE#root").unwrap();
        assert!(matches!(
            selected(root.path(), &slot).resolve_file(&address),
            Err(ResolveError::DocNotFound { .. })
        ));
        write(&slot, "source_exports=[]\n");
        assert!(matches!(
            selected(root.path(), &slot).resolve_file(&address),
            Err(ResolveError::DocNotFound { .. })
        ));
        let file =
            vibe_core::machine_json_path(&vibe_core::layout::current_specs_root().join("ONE.md"));
        write(
            &slot,
            &format!(
                "[[package.sources]]\nfile='{file}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n"
            ),
        );
        let error = selected(root.path(), &slot)
            .resolve_file(&address)
            .unwrap_err();
        assert!(error.to_string().contains("spec address lookup"));
    }
    #[cfg(unix)]
    #[test]
    fn source_policy_selected_installed_slot_symlink_is_not_a_new_root_grant() {
        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        write(external.path(), "");
        let slot = root
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.source.doc/1.0.0");
        fs::create_dir_all(slot.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(external.path(), &slot).unwrap();
        let address = SpecAddress::parse("spec://org.source/doc/ONE#root").unwrap();
        assert!(selected(root.path(), &slot).resolve_file(&address).is_err());
        // Genuinely caller-selected root aliases remain a separate compatibility.
        let alias = root.path().join("explicit-alias");
        std::os::unix::fs::symlink(external.path(), &alias).unwrap();
        let direct = FileResolver::new(
            &alias,
            SelfCoordinate::new(Some("org.source".into()), "doc".into()),
        );
        assert!(direct.resolve_file(&address).is_ok());
    }
}
