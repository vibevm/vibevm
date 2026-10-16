use super::*;
use vibe_core::manifest::{LockedPackage, Lockfile};

#[test]
fn captured_delivery_is_visible_readable_and_inactive_is_hidden() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vibe.toml"), "[project]\ngroup='org.example'\nname='consumer'\nversion='1.0.0'\n[requires]\npackages={'org.example/owner'='*'}\n").unwrap();
    let ctx = ServerContext::with_store_root(dir.path(), dir.path().join("wrong-store"));
    let hash = format!("sha256:{}", "1".repeat(64));
    let package: LockedPackage = toml::from_str(&format!("kind='flow'\ngroup='org.example'\nname='owner'\nversion='1.0.0'\nsource_url='file:///fixture'\ncontent_hash='{hash}'\n")).unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(
        dir.path(),
        &package.group,
        &package.name,
        &package.version,
    );
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(slot.join("vibe.toml"), "[package]\nkind='flow'\ngroup='org.example'\nname='owner'\nversion='1.0.0'\n[[skill]]\nname='independent'\npath='skills/other'\nagents=['codex']\n").unwrap();
    std::fs::create_dir_all(slot.join("skills/other")).unwrap();
    std::fs::write(
        slot.join("skills/other/SKILL.md"),
        "Independent Agent Skill bytes\n",
    )
    .unwrap();
    for (name, mode) in [
        ("eager", "eager"),
        ("push", "lazy-push"),
        ("pull", "lazy-pull"),
        ("inactive", "eager"),
    ] {
        let sub = slot.join("subskills").join(name);
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("vibe-subskill.toml"), format!("[subskill]\npath='{name}'\ndelivery='{mode}'\ndescription='Use {name} for task'\n[content]\nfiles_written=['guide.md']\n")).unwrap();
        std::fs::write(
            sub.join("guide.md"),
            format!("{name}: original captured bytes\n"),
        )
        .unwrap();
    }
    let request: vibe_wire::generated::lock_configuration::e1::requested::Requested = serde_json::from_value(json!({"epoch":"1","roots":["org.example/owner"],"origins":[],"features":["selected"],"no_defaults":true,"all_features":false,"definitions":[],"defaults":[]})).unwrap();
    let record = |names: Vec<&str>| {
        vibe_wire::behaviour::lock_configuration::encode(request.clone(),
        serde_json::from_value(json!({"epoch":"1","request_digest":"pending","packages":[{"package":"org.example/owner@1.0.0","content":hash,"feature_closure":["selected"],"effects":json!({"subskills":names}).to_string()}],"edges":[],"configurations":[]})).unwrap()).unwrap()
    };
    let mut lock = Lockfile::empty("fixture", "0");
    lock.packages.push(package);
    lock.meta.configuration = Some(record(vec!["eager", "push", "pull"]));
    lock.write(dir.path().join(Lockfile::FILENAME)).unwrap();
    use vibe_agent_projection::pkgskill::{
        DeclaredSkillFilter, project_declared_skills_project_scope,
    };
    project_declared_skills_project_scope(dir.path(), &DeclaredSkillFilter::all(), false).unwrap();
    let projected = dir.path().join(".agents/skills/independent/SKILL.md");
    assert_eq!(
        std::fs::read(&projected).unwrap(),
        b"Independent Agent Skill bytes\n"
    );
    assert!(!dir.path().join(".agents/skills/push").exists());
    let query = QueryPackageMcpTool
        .run(&json!({"name":"org.example/owner"}), &ctx)
        .unwrap();
    assert_eq!(query["subskills_active"].as_array().unwrap().len(), 3);
    assert!(
        query["subskills_active"]
            .as_array()
            .unwrap()
            .iter()
            .any(|sub| sub["description"] == "Use push for task"
                && sub["read_tool"] == "read_subskill")
    );
    for name in ["eager", "push", "pull"] {
        let args = json!({"package":"org.example/owner", "subskill_path":name});
        let read = ReadSubskillMcpTool.run(&args, &ctx).unwrap();
        assert!(
            read["content"]
                .as_str()
                .unwrap()
                .contains(&format!("{name}: original captured bytes"))
        );
        assert!(read["definition_digest"].as_str().unwrap().len() >= 64);
        assert_eq!(
            MaterialiseSubskillMcpTool.run(&args, &ctx).unwrap()["written"],
            json!([])
        );
    }
    assert!(
        ReadSubskillMcpTool
            .run(
                &json!({"package":"org.example/owner","subskill_path":"inactive"}),
                &ctx
            )
            .is_err()
    );
    lock.meta.configuration = Some(record(vec![]));
    lock.write(dir.path().join(Lockfile::FILENAME)).unwrap();
    project_declared_skills_project_scope(dir.path(), &DeclaredSkillFilter::all(), false).unwrap();
    assert_eq!(
        std::fs::read(&projected).unwrap(),
        b"Independent Agent Skill bytes\n"
    );
    assert_eq!(
        QueryPackageMcpTool
            .run(&json!({"name":"org.example/owner"}), &ctx)
            .unwrap()["subskills_active"],
        json!([])
    );
    assert!(
        ReadSubskillMcpTool
            .run(
                &json!({"package":"org.example/owner","subskill_path":"pull"}),
                &ctx
            )
            .is_err()
    );
    assert_eq!(
        std::fs::read(slot.join("subskills/pull/guide.md")).unwrap(),
        b"pull: original captured bytes\n"
    );
    assert!(!dir.path().join("guide.md").exists());
}
