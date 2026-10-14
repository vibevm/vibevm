use super::*;
use crate::derivatives::{SourceInput, prepare_binding};
use crate::source_query::CapturedSourceFile;
use vibe_core::manifest::derivation::{DerivationBinding, DerivationRecipe};

#[test]
fn inventory_contract_preserves_all_five_families_without_claiming_semantic_compatibility() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/next/derivatives");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixtures.join("manifest.json")).unwrap()).unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 5);
    for case in cases {
        let capture = |side: &str, directory: &str, owner: &str| {
            let root = tempfile::tempdir().unwrap();
            let mut files = Vec::new();
            for row in case[side]["files"].as_array().unwrap() {
                let path = row["path"].as_str().unwrap();
                let bytes = std::fs::read(
                    fixtures
                        .join(case[side][directory].as_str().unwrap())
                        .join(path),
                )
                .unwrap();
                let relative = format!("data/{path}");
                std::fs::create_dir_all(root.path().join(&relative).parent().unwrap()).unwrap();
                std::fs::write(root.path().join(&relative), bytes).unwrap();
                let snapshot = vibe_safefs::Project::open(root.path())
                    .unwrap()
                    .read_file_snapshot_bounded(&relative, 4096)
                    .unwrap()
                    .unwrap();
                files.push(CapturedSourceFile::new(relative, snapshot).unwrap());
            }
            let role = if case["source_interpretation"] == "vibe-spec" {
                "spec"
            } else {
                "resource"
            };
            SourceInput::new(
                vibe_core::PackageRef::parse(&format!("flow:org.example/{owner}@=1.0.0")).unwrap(),
                serde_json::from_value(serde_json::json!({"directory":"data", "media": case["media"], "interpretation":case["source_interpretation"], "role":role, "loading":"explicit"})).unwrap(),
                files,
            ).unwrap()
        };
        let base = capture("base", "capture_root", "upstream");
        let local = capture("local", "authored_root", "local");
        let original = base.retained_source("data");
        let binding = DerivationBinding::new(
            original.clone(),
            local.retained_source("data"),
            original.observation_id().unwrap(),
            DerivationRecipe::ReplaceSelectedInventory,
        )
        .unwrap();
        let result = prepare_binding(&toml::to_string(&binding).unwrap(), Some(&base))
            .unwrap()
            .value;
        let paths = result
            .binding()
            .local()
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect();
        assert_eq!(
            result
                .compatibility(Some(&ExportTarget {
                    name: "byte-inventory".into(),
                    interface: ExportInterface::Files(paths)
                }))
                .compatibility,
            Compatibility::Compatible
        );
        assert!(matches!(
            result
                .compatibility(Some(&ExportTarget {
                    name: "missing".into(),
                    interface: ExportInterface::Files(vec!["data/absent".into()])
                }))
                .compatibility,
            Compatibility::Incompatible(_)
        ));
        for file in &result.binding().local().files {
            let expected = fixtures
                .join(case["expected"]["root"].as_str().unwrap())
                .join(file.path.strip_prefix("data/").unwrap());
            assert_eq!(file.bytes, std::fs::read(expected).unwrap());
        }
        assert_eq!(result.binding().base(), &original);
        assert_eq!(result.binding().local(), &local.retained_source("data"));
    }
}

fn source(root: &std::path::Path, owner: &str) -> SourceInput {
    SourceInput::new(
        vibe_core::PackageRef::parse(&format!("flow:org.example/{owner}@=1.0.0")).unwrap(),
        serde_json::from_value(serde_json::json!({"file":"guide.md", "media":"text/markdown", "interpretation":"commonmark", "role":"documentation", "loading":"explicit"})).unwrap(),
        vec![CapturedSourceFile::new("guide.md".into(), vibe_safefs::Project::open(root).unwrap().read_file_snapshot_bounded("guide.md", 4096).unwrap().unwrap()).unwrap()],
    ).unwrap()
}

#[test]
fn sealed_fork_checks_actual_result_and_keeps_both_sources() {
    for sealed in [false, true] {
        let base_dir = tempfile::tempdir().unwrap();
        let local_dir = tempfile::tempdir().unwrap();
        std::fs::write(base_dir.path().join("guide.md"), "# Original\nbase\n").unwrap();
        std::fs::write(local_dir.path().join("guide.md"), "# Alternative\nlocal\n").unwrap();
        let mut permissions = std::fs::metadata(base_dir.path().join("guide.md"))
            .unwrap()
            .permissions();
        permissions.set_readonly(sealed);
        std::fs::set_permissions(base_dir.path().join("guide.md"), permissions).unwrap();
        // No demand or sealed permission enters the passive creation API.
        let base = source(base_dir.path(), "upstream");
        let local = source(local_dir.path(), "local");
        let retained = base.retained_source("guide");
        let binding = DerivationBinding::new(
            retained.clone(),
            local.retained_source("guide"),
            retained.observation_id().unwrap(),
            DerivationRecipe::ReplaceSelectedInventory,
        )
        .unwrap();
        let result = prepare_binding(&toml::to_string(&binding).unwrap(), Some(&base))
            .unwrap()
            .value;
        let target = |heading: &str| ExportTarget {
            name: heading.into(),
            interface: ExportInterface::CommonmarkHeading {
                path: "guide.md".into(),
                heading: heading.into(),
            },
        };
        let rejected = result.compatibility(Some(&target("Original")));
        assert_eq!(rejected.creation, CreationState::Created);
        assert!(matches!(
            rejected.compatibility,
            Compatibility::Incompatible(_)
        ));
        assert_eq!(
            result
                .compatibility(Some(&target("Alternative")))
                .compatibility,
            Compatibility::Compatible
        );
        assert!(matches!(
            result.compatibility(None).compatibility,
            Compatibility::Unknown(_)
        ));
        assert!(matches!(
            result
                .compatibility(Some(&ExportTarget {
                    name: "future".into(),
                    interface: ExportInterface::Unsupported("future ABI".into())
                }))
                .compatibility,
            Compatibility::Unknown(_)
        ));
        assert_eq!(
            std::fs::read(base_dir.path().join("guide.md")).unwrap(),
            b"# Original\nbase\n"
        );
        assert_eq!(
            std::fs::read(local_dir.path().join("guide.md")).unwrap(),
            b"# Alternative\nlocal\n"
        );
        assert_eq!(
            result.binding().local().files,
            local.retained_source("guide").files
        );
    }
}

#[test]
fn ambiguous_heading_is_never_supported() {
    let base_dir = tempfile::tempdir().unwrap();
    let local_dir = tempfile::tempdir().unwrap();
    std::fs::write(base_dir.path().join("guide.md"), "# Usage\n").unwrap();
    std::fs::write(
        local_dir.path().join("guide.md"),
        "# Usage\na\n# Usage\nb\n",
    )
    .unwrap();
    let base = source(base_dir.path(), "base");
    let local = source(local_dir.path(), "local");
    let retained = base.retained_source("guide");
    let binding = DerivationBinding::new(
        retained.clone(),
        local.retained_source("guide"),
        retained.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let result = prepare_binding(&toml::to_string(&binding).unwrap(), Some(&base))
        .unwrap()
        .value;
    assert!(matches!(
        result
            .compatibility(Some(&ExportTarget {
                name: "usage".into(),
                interface: ExportInterface::CommonmarkHeading {
                    path: "guide.md".into(),
                    heading: "Usage".into()
                }
            }))
            .compatibility,
        Compatibility::Incompatible(_)
    ));
}
