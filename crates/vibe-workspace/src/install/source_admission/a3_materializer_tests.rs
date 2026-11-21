#[cfg(test)]
mod cases {
    use crate::install::{
        MaterialiseOptions, SlotLifecycleMode, materialise_resolution_with_spec_format,
        test_helpers::*,
    };
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };
    use vibe_core::{manifest::SpecFormat, user_config::SlotIntegrity};

    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn walk(root: &Path, directory: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(root).unwrap().to_owned();
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
    fn row(path: &str, media: &str, interpretation: &str) -> String {
        let role = if interpretation == "vibe-spec" {
            "spec"
        } else {
            "resource"
        };
        format!(
            "[[package.sources]]\nfile='{path}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='scan'\n"
        )
    }
    #[test]
    fn m_06_a_3_later_conversion_and_collisions_refuse_before_earlier_slot_or_hook() {
        for scenario in [
            "valid",
            "invalid-native",
            "native-address",
            "output",
            "ancestor-output",
        ] {
            let root = tempfile::tempdir().unwrap();
            let (earlier, _earlier) =
                dep_with_boot("earlier", "1.0.0", "", "scripts/prepare.sh", "exit 0\n");
            let (rows, primary, second, target) = match scenario {
                "valid" => (
                    row("spec/a.md", "text/markdown", "vibe-spec"),
                    "# Native\n",
                    None,
                    SpecFormat::Xml,
                ),
                "invalid-native" => (
                    row("spec/a.md", "application/xml", "vibe-spec"),
                    "<invalid/>",
                    None,
                    SpecFormat::Xml,
                ),
                "native-address" => (
                    row("spec/a.md", "text/markdown", "vibe-spec")
                        + &row("spec/a.xml", "application/xml", "vibe-spec"),
                    "# Native\n",
                    Some((
                        "spec/a.xml",
                        "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Native</title></spec>",
                    )),
                    SpecFormat::Mixed,
                ),
                "output" => (
                    row("spec/a.md", "text/markdown", "vibe-spec")
                        + &row("spec/a.xml", "application/xml", "xml-data"),
                    "# Native\n",
                    Some(("spec/a.xml", "<raw/>")),
                    SpecFormat::Xml,
                ),
                _ => (
                    row("spec/a.md", "text/markdown", "vibe-spec"),
                    "# Native\n",
                    Some(("spec/a.xml/child", "raw child")),
                    SpecFormat::Xml,
                ),
            };
            let (bad, source) = dep_with_boot("declared", "1.0.0", &rows, "spec/a.md", primary);
            if let Some((path, body)) = second {
                write(source.path(), path, body);
            }
            let before = snapshot(root.path());
            let spy = PreInstallCount::of("earlier");
            let result = materialise_resolution_with_spec_format(
                root.path(),
                &[earlier, bad],
                MaterialiseOptions {
                    slot_integrity: SlotIntegrity::TrustPresence,
                    spec_format: target,
                    slot_verifier: None,
                    lifecycle: SlotLifecycleMode::Callback(&spy),
                    progress: Default::default(),
                    rollback: None,
                },
            );
            if scenario == "valid" {
                assert!(result.is_ok());
                assert_eq!(
                    spy.get(),
                    1,
                    "positive control must raise the earlier slot event"
                );
                continue;
            }
            assert!(result.is_err(), "{scenario}");
            assert_eq!(
                spy.get(),
                0,
                "earlier slot event raised before {scenario} preflight"
            );
            assert_eq!(
                snapshot(root.path()),
                before,
                "earlier slot/metadata changed before {scenario} preflight"
            );
        }
    }
}
