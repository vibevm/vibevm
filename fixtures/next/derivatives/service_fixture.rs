// Shared by the three narrowly selected service test modules. All authored
// inputs are real files copied from the manifest's exact resource membership.
use std::{fs, path::Path};
use vibe_core::manifest::derivation::{
    DerivationBinding, DerivationRecipe, RetainedFile, RetainedSource,
};
use vibe_safefs::Project;

struct Fixture {
    _home: tempfile::TempDir,
    base: Project,
    local: Project,
    output: Project,
    binding: DerivationBinding,
    text: String,
    expected: Vec<RetainedFile>,
}
impl Fixture {
    fn new(family: &str) -> Self {
        let fixtures =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/next/derivatives");
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(fixtures.join("manifest.json")).unwrap()).unwrap();
        let case = manifest["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == family)
            .unwrap();
        let home = tempfile::tempdir().unwrap();
        let relative = |path: &str| {
            if family == "nested-tree" {
                format!("payload/{path}")
            } else {
                path.to_owned()
            }
        };
        let mut descriptor = serde_json::json!({"media":case["media"], "interpretation":case["source_interpretation"], "role":if family == "vibe-spec" {"spec"} else {"resource"}, "loading":"explicit"});
        if family == "nested-tree" {
            descriptor["directory"] = "payload".into();
        } else {
            descriptor["file"] = case["base"]["files"][0]["path"].clone();
        }
        let descriptor: vibe_core::manifest::SourceDescriptor =
            serde_json::from_value(descriptor).unwrap();
        let source = |side: &str, root_key: &str, owner: &str| {
            let root = home.path().join(side);
            fs::create_dir(&root).unwrap();
            let mut files = Vec::new();
            for row in case[side]["files"].as_array().unwrap() {
                let path = row["path"].as_str().unwrap();
                let bytes = fs::read(
                    fixtures
                        .join(case[side][root_key].as_str().unwrap())
                        .join(path),
                )
                .unwrap();
                let path = relative(path);
                fs::create_dir_all(root.join(&path).parent().unwrap()).unwrap();
                fs::write(root.join(&path), &bytes).unwrap();
                files.push(RetainedFile { path, bytes });
            }
            let retained = RetainedSource {
                owner: vibe_core::PackageRef::parse(&format!("flow:org.example/{owner}@=1.0.0"))
                    .unwrap(),
                resource: "payload".into(),
                descriptor: descriptor.clone(),
                files,
            };
            (Project::open(&root).unwrap(), retained)
        };
        let (base, original) = source("base", "capture_root", "upstream");
        let (local, authored) = source("local", "authored_root", "local");
        let expected = case["expected"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let path = row["path"].as_str().unwrap();
                RetainedFile {
                    path: relative(path),
                    bytes: fs::read(
                        fixtures
                            .join(case["expected"]["root"].as_str().unwrap())
                            .join(path),
                    )
                    .unwrap(),
                }
            })
            .collect();
        let binding = DerivationBinding::new(
            original.clone(),
            authored,
            original.observation_id().unwrap(),
            DerivationRecipe::ReplaceSelectedInventory,
        )
        .unwrap();
        let text = format!(
            "# Independently retained local author\n{}",
            toml::to_string(&binding).unwrap()
        );
        fs::write(home.path().join("authored.toml"), &text).unwrap();
        fs::create_dir(home.path().join("output")).unwrap();
        let output = Project::open(&home.path().join("output")).unwrap();
        Self {
            _home: home,
            base,
            local,
            output,
            binding,
            text,
            expected,
        }
    }
}
