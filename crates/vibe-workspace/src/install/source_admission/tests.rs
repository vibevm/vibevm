use super::*;
#[cfg(test)]
mod cases {
    use super::*;
    use crate::install::{apply_resolution_with_spec_format, test_helpers::*};
    use std::{collections::BTreeMap, fs, path::PathBuf};
    use tempfile::TempDir;
    fn workspace() -> (TempDir, Workspace) {
        let root = TempDir::new().unwrap();
        write(
            root.path(),
            "vibe.toml",
            "[project]\nname='host'\ngroup='org.host'\nversion='1.0.0'\n",
        );
        let workspace = Workspace::load(root.path()).unwrap();
        (root, workspace)
    }
    fn declared(name: &str, extra: &str) -> (ResolvedDep, TempDir) {
        dep_with_boot(
            name,
            "1.0.0",
            &format!("sources=[]\n{extra}"),
            "payload.txt",
            "AUTHORED_RAW",
        )
    }
    fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                if path.is_dir() {
                    out.insert(relative, None);
                    walk(root, &path, out);
                } else {
                    out.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }
    #[test]
    fn source_admission_rejects_entire_resolution_before_first_slot_effect() {
        let (root, ws) = workspace();
        let (legacy, _legacy) = dep_with_boot("legacy", "1.0.0", "", "payload.txt", "LEGACY");
        let (bad, _bad) = declared("declared", "[boot_snippet]\nsource='boot.md'\n");
        let before = inventory(root.path());
        let error = apply_resolution_with_spec_format(
            &ws,
            &[legacy, bad],
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            SpecFormat::Mixed,
            None,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("classified boot"));
        assert_eq!(inventory(root.path()), before);
    }
    #[test]
    fn source_admission_known_declaration_survives_missing_content_manifest() {
        let (root, ws) = workspace();
        let (dep, source) = declared("declared", "");
        fs::remove_file(source.path().join("vibe.toml")).unwrap();
        let before = inventory(root.path());
        assert!(
            apply_resolution_with_spec_format(
                &ws,
                &[dep],
                vibe_core::user_config::SlotIntegrity::TrustPresence,
                SpecFormat::Mixed,
                None,
                None
            )
            .unwrap_err()
            .to_string()
            .contains("manifest")
        );
        assert_eq!(inventory(root.path()), before);
    }
    #[test]
    fn source_admission_binds_boot_and_requires_beyond_package_metadata() {
        for change in [
            "[boot_snippet]\nsource='later.md'\n",
            "[requires.packages]\n'org.vibevm/later'='=1.0.0'\n",
        ] {
            let (root, ws) = workspace();
            let (dep, source) = declared("declared", "");
            let path = source.path().join("vibe.toml");
            let old = fs::read_to_string(&path).unwrap();
            fs::write(&path, format!("{old}\n{change}")).unwrap();
            let changed = Manifest::read(&path).unwrap();
            assert_eq!(dep.manifest.package, changed.package);
            let before = inventory(root.path());
            assert!(
                apply_resolution_with_spec_format(
                    &ws,
                    &[dep],
                    vibe_core::user_config::SlotIntegrity::TrustPresence,
                    SpecFormat::Mixed,
                    None,
                    None
                )
                .unwrap_err()
                .to_string()
                .contains("full source manifest")
            );
            assert_eq!(inventory(root.path()), before);
        }
    }
    #[test]
    fn source_admission_declared_package_can_use_legacy_dependency_boot() {
        let (root, ws) = workspace();
        let boot = boot_rel("legacy.md");
        let (legacy, _legacy) = dep_with_boot(
            "legacy",
            "1.0.0",
            &format!("[boot_snippet]\nsource='{boot}'\n"),
            &boot,
            "# Legacy\nLEGACY_CHILD\n",
        );
        let (mut declared, _declared) = declared(
            "declared",
            "[requires.packages]\n'org.vibevm/legacy'={version='=1.0.0',link='static'}\n",
        );
        declared.requires = vec![(legacy.group.clone(), legacy.name.clone())];
        let outcome = apply_resolution_with_spec_format(
            &ws,
            &[legacy, declared],
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            SpecFormat::Mixed,
            None,
            None,
        )
        .unwrap();
        assert_eq!(outcome.materialised.len(), 2);
        let slot = crate::vibedeps::slot_abs_path(
            root.path(),
            &vibe_core::Group::parse("org.vibevm").unwrap(),
            "declared",
            &ver("1.0.0"),
        );
        assert_eq!(
            fs::read_to_string(slot.join("payload.txt")).unwrap(),
            "AUTHORED_RAW"
        );
        assert!(
            fs::read_to_string(slot.join(vibe_core::layout::current_boot_static_md()))
                .unwrap()
                .contains("LEGACY_CHILD")
        );
    }

    #[test]
    fn source_admission_existing_trusted_slot_does_not_skip_new_unsupported_policy() {
        let (root, ws) = workspace();
        let (mut dep, source) = declared("declared", "");
        apply_resolution_with_spec_format(
            &ws,
            &[dep.clone()],
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            SpecFormat::Mixed,
            None,
            None,
        )
        .unwrap();
        let before = inventory(root.path());
        let manifest = source.path().join("vibe.toml");
        let old = fs::read_to_string(&manifest).unwrap();
        fs::write(
            &manifest,
            format!("{old}\n[boot_snippet]\nsource='later.md'\n"),
        )
        .unwrap();
        dep.manifest = Manifest::read(&manifest).unwrap();
        assert!(
            apply_resolution_with_spec_format(
                &ws,
                &[dep],
                vibe_core::user_config::SlotIntegrity::TrustPresence,
                SpecFormat::Mixed,
                None,
                None
            )
            .unwrap_err()
            .to_string()
            .contains("classified boot")
        );
        assert_eq!(inventory(root.path()), before);
    }
    struct Probe;
    impl crate::hooks::InterpreterProbe for Probe {
        fn has(&self, _: &str) -> bool {
            true
        }
    }
    #[derive(Default)]
    struct HookSpy(std::cell::Cell<usize>);
    impl crate::hooks::HookRunner for HookSpy {
        fn run(
            &self,
            _: &crate::hooks::HookInvocation,
            _: &Path,
            _: &[(String, String)],
        ) -> Result<i32, String> {
            self.0.set(self.0.get() + 1);
            Ok(0)
        }
    }
    fn input_before_earlier_hook(rows: &str, expected: Option<&str>) {
        let (root, _ws) = workspace();
        let (legacy, _legacy) = dep_with_boot(
            "legacy-hook",
            "1.0.0",
            "[hooks]\npre-install='hooks/prepare'\n",
            "hooks/prepare.sh",
            "#!/bin/sh\nexit 0\n",
        );
        let (bad, _bad) = dep_with_boot("declared", "1.0.0", rows, "payload.txt", "PAYLOAD");
        let before = inventory(root.path());
        let spy = HookSpy::default();
        let policy = crate::hooks::HookPolicy {
            allowed_groups: vec!["org.vibevm".into()],
            allow_hooks: false,
        };
        let outcome = crate::install::materialise_resolution(
            root.path(),
            &[legacy, bad],
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            None,
            Some(&policy),
            &Probe,
            &spy,
        );
        let Some(expected) = expected else {
            assert_eq!(outcome.unwrap().materialised.len(), 2);
            assert_eq!(
                spy.0.get(),
                1,
                "supported finite inventory must allow normal hook execution"
            );
            assert_eq!(
                fs::read(
                    root.path()
                        .join(crate::vibedeps::slot_rel_path(
                            &vibe_core::Group::parse("org.vibevm").unwrap(),
                            "declared",
                            &ver("1.0.0")
                        ))
                        .join("payload.txt")
                )
                .unwrap(),
                b"PAYLOAD"
            );
            return;
        };
        let error = outcome.unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert_eq!(
            spy.0.get(),
            0,
            "earlier hook executed before later input admission"
        );
        assert_eq!(
            inventory(root.path()),
            before,
            "earlier slot published before later input admission"
        );
    }
    #[test]
    fn source_admission_missing_input_precedes_earlier_hook() {
        input_before_earlier_hook(
            "[[package.sources]]\nfile='missing.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
            Some("missing"),
        );
    }
    #[test]
    fn source_admission_zero_match_directory_default_allows_normal_hook_order() {
        input_before_earlier_hook(
            "[[package.sources]]\ndirectory='docs'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
            None,
        );
    }
    #[test]
    fn source_admission_conflicting_input_precedes_earlier_hook() {
        input_before_earlier_hook(
            "[[package.sources]]\nfile='payload.txt'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n[[package.sources]]\nfile='payload.txt'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n",
            Some("conflict"),
        );
    }
}
