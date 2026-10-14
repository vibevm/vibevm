use super::super::{ProjectSkillProviderInput, lower_project_skill_bindings};
use super::*;
use vibe_core::manifest::{
    SkillDecl,
    derivation::{DerivationBinding, DerivationRecipe},
};
use vibe_workspace::{
    derivatives::{SourceInput, prepare_binding},
    source_query::CapturedSourceFile,
};

fn prepare(root: &std::path::Path) -> (BoundDerivative, ProjectSkillBinding) {
    std::fs::create_dir_all(root.join("skill")).unwrap();
    std::fs::write(
        root.join("skill/SKILL.md"),
        "---\nname: local-skill\n---\nlocal body",
    )
    .unwrap();
    let snapshot = vibe_safefs::Project::open(root)
        .unwrap()
        .read_file_snapshot_bounded("skill/SKILL.md", 4096)
        .unwrap()
        .unwrap();
    let source = |name: &str| {
        SourceInput::new(
        vibe_core::PackageRef::parse(&format!("tool:org.example/{name}@=1.0.0")).unwrap(),
        toml::from_str("directory = 'skill'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'").unwrap(),
        vec![CapturedSourceFile::new("skill/SKILL.md".into(), snapshot.clone()).unwrap()],
    ).unwrap()
    };
    let base = source("upstream");
    let local = source("local");
    let retained = base.retained_source("skill");
    let binding = DerivationBinding::new(
        retained.clone(),
        local.retained_source("skill"),
        retained.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let result = prepare_binding(&toml::to_string(&binding).unwrap(), Some(&base))
        .unwrap()
        .value;
    let mut bindings = lower_project_skill_bindings(
        root,
        vec![ProjectSkillProviderInput {
            provider: DeclaredSkillProvider::Authored {
                group: vibe_core::Group::parse("org.example").unwrap(),
                name: vibe_core::PackageName::parse("local").unwrap(),
                version: "1.0.0".into(),
                kind: vibe_core::PackageKind::Tool,
                root: root.to_path_buf(),
                embedded_sources: vec![],
            },
            declarations: vec![SkillDecl {
                name: "local-skill".into(),
                path: "skill".into(),
                source: None,
                description: None,
                agents: vec!["claude".into()],
                include: vec![],
                resources: vec![],
            }],
        }],
    )
    .unwrap();
    (result, bindings.remove(0))
}

#[test]
fn actual_projection_interface_and_owner_are_independent_from_creation() {
    let root = tempfile::tempdir().unwrap();
    let (result, binding) = prepare(root.path());
    assert_eq!(
        inspect_binding(&result, &binding),
        Compatibility::Compatible
    );
    let mut upstream = binding.clone();
    if let DeclaredSkillProvider::Authored { name, .. } = &mut upstream.skill.provider {
        *name = vibe_core::PackageName::parse("upstream").unwrap();
    }
    assert!(matches!(
        inspect_binding(&result, &upstream),
        Compatibility::Incompatible(_)
    ));
    upstream.skill.provider = DeclaredSkillProvider::Installed {
        group: vibe_core::Group::parse("org.example").unwrap(),
        name: vibe_core::PackageName::parse("upstream").unwrap(),
        version: "1.0.0".into(),
        kind: vibe_core::PackageKind::Tool,
        root: root.path().to_path_buf(),
        content_hash: vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        embedded_sources: vec![],
        declared_sources: vec![],
    };
    assert!(matches!(
        inspect_binding(&result, &upstream),
        Compatibility::Incompatible(_)
    ));
    let mut changed = binding.clone();
    changed
        .selected_files
        .as_mut()
        .unwrap()
        .insert("SKILL.md".into(), b"different result".to_vec());
    assert!(matches!(
        inspect_binding(&result, &changed),
        Compatibility::Incompatible(_)
    ));
    let mut missing = binding.clone();
    missing.selected_files = None;
    assert!(matches!(
        inspect_binding(&result, &missing),
        Compatibility::Unknown(_)
    ));
    assert!(!root.path().join(".claude").exists());
    assert_eq!(
        std::fs::read(root.path().join("skill/SKILL.md")).unwrap(),
        result.binding().local().files[0].bytes
    );
}

#[test]
fn compatible_inspection_does_not_override_receipt_ownership() {
    let root = tempfile::tempdir().unwrap();
    let (result, binding) = prepare(root.path());
    let destination = &binding.targets[0].path;
    std::fs::create_dir_all(destination).unwrap();
    std::fs::write(destination.join("FOREIGN.txt"), b"unowned").unwrap();
    assert_eq!(
        inspect_binding(&result, &binding),
        Compatibility::Compatible
    );
    assert!(super::super::reconcile_project_skill_binding(root.path(), &binding).is_err());
    assert_eq!(
        std::fs::read(destination.join("FOREIGN.txt")).unwrap(),
        b"unowned"
    );
    assert!(!destination.join("SKILL.md").exists());
    assert_eq!(
        std::fs::read(root.path().join("skill/SKILL.md")).unwrap(),
        result.binding().local().files[0].bytes
    );
}
