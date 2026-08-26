use std::fs;

use super::*;

fn project_with_skill(agents: &[&str], include: &[&str]) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let agents = agents
        .iter()
        .map(|agent| format!("\"{agent}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let include = include
        .iter()
        .map(|pattern| format!("\"{pattern}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        project.path().join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.example\"\nname = \"demo\"\nkind = \"tool\"\n\
             version = \"0.1.0\"\nauthors = [\"Fixture\"]\nlicense = \"EULA\"\n\
             description = \"fixture\"\nkeywords = [\"fixture\"]\n\n\
             [[skill]]\nname = \"demo\"\npath = \"skills/demo\"\n\
             agents = [{agents}]\ninclude = [{include}]\n"
        ),
    )
    .unwrap();
    let source = project.path().join("skills/demo");
    fs::create_dir_all(source.join("references")).unwrap();
    fs::write(source.join("SKILL.md"), "v1").unwrap();
    fs::write(source.join("references/keep.md"), "keep").unwrap();
    fs::write(source.join("drop.md"), "drop").unwrap();
    project
}

fn report(agent: Agent, scope: Scope, name: &str, dry_run: bool) -> PackageSkillReport {
    PackageSkillReport {
        skill: name.to_string(),
        agent: agent.as_str().to_string(),
        scope: scope.as_str(),
        path: None,
        status: if dry_run { "would-create" } else { "created" },
        note: None,
    }
}

#[test]
fn project_only_helper_never_passes_user_scope_in_preview_or_apply() {
    let project = project_with_skill(&[], &[]);
    for dry_run in [true, false] {
        let mut calls = Vec::new();
        let reports = project_declared_skills_project_scope_with(
            project.path(),
            &DeclaredSkillFilter::all(),
            dry_run,
            |agent, scope, root, name, _source, _include, received_dry_run| {
                assert_eq!(scope, Scope::Project, "user roots must never be resolved");
                assert_eq!(root, Some(project.path()));
                assert_eq!(received_dry_run, dry_run);
                calls.push((agent, scope, root.unwrap().to_path_buf()));
                Ok(report(agent, scope, name, received_dry_run))
            },
        )
        .unwrap();
        assert_eq!(reports.len(), Agent::ALL.len());
        assert_eq!(calls.len(), Agent::ALL.len());
        assert!(calls.iter().all(|(_, scope, root)| {
            *scope == Scope::Project && root.as_path() == project.path()
        }));
    }
    assert!(!project.path().join(".claude").exists());
    assert!(!project.path().join(".opencode").exists());
    assert!(!project.path().join(".agents").exists());
}

#[test]
fn package_binding_defaults_to_every_project_skill_loader_and_hashes_selected_source() {
    let project = project_with_skill(&[], &["SKILL.md", "references/**"]);
    let first = collect_project_skill_bindings(project.path()).unwrap();
    assert_eq!(first.len(), 1);
    let binding = &first[0];
    assert_eq!(
        binding
            .targets
            .iter()
            .map(|target| target.agent.as_str())
            .collect::<Vec<_>>(),
        ["claude", "opencode", "codex"]
    );
    assert_eq!(
        binding
            .targets
            .iter()
            .map(|target| target.path.clone())
            .collect::<Vec<_>>(),
        [
            project.path().join(".claude/skills/demo"),
            project.path().join(".opencode/skills/demo"),
            project.path().join(".agents/skills/demo"),
        ]
    );
    let initial = binding.source_snapshot.clone();

    fs::write(project.path().join("skills/demo/drop.md"), "ignored change").unwrap();
    assert_eq!(
        collect_project_skill_bindings(project.path()).unwrap()[0].source_snapshot,
        initial
    );
    fs::write(
        project.path().join("skills/demo/SKILL.md"),
        "selected change",
    )
    .unwrap();
    assert_ne!(
        collect_project_skill_bindings(project.path()).unwrap()[0].source_snapshot,
        initial
    );
}

#[test]
fn explicit_both_scope_remains_project_then_user() {
    let project = project_with_skill(&["claude"], &[]);
    let plan =
        prepare_declared_skill_projection(project.path(), &DeclaredSkillFilter::all(), Scope::Both)
            .unwrap();
    let mut scopes = Vec::new();
    plan.install_with(
        true,
        |agent, scope, _root, name, _source, _include, dry_run| {
            scopes.push(scope);
            Ok(report(agent, scope, name, dry_run))
        },
    )
    .unwrap();
    assert_eq!(scopes, [Scope::Project, Scope::User]);
}

#[test]
fn declaration_and_cli_agent_filters_intersect() {
    let project = project_with_skill(&["claude", "opencode"], &[]);
    let opencode = prepare_declared_skill_projection(
        project.path(),
        &DeclaredSkillFilter::new(&[], Some("opencode")),
        Scope::Project,
    )
    .unwrap()
    .install(true)
    .unwrap();
    assert_eq!(opencode.len(), 1);
    assert_eq!(opencode[0].agent, "opencode");

    let codex = project_declared_skills_project_scope(
        project.path(),
        &DeclaredSkillFilter::new(&[], Some("codex")),
        true,
    )
    .unwrap();
    assert!(codex.is_empty());
}

#[test]
fn project_orchestrator_is_idempotent_preserves_neighbor_and_cleans_owned_stale_files() {
    let project = project_with_skill(&["claude"], &[]);
    let neighbor = project.path().join(".claude/skills/foreign/KEEP.md");
    fs::create_dir_all(neighbor.parent().unwrap()).unwrap();
    fs::write(&neighbor, "foreign").unwrap();

    let first =
        project_declared_skills_project_scope(project.path(), &DeclaredSkillFilter::all(), false)
            .unwrap();
    assert_eq!(first[0].status, "created");
    let second =
        project_declared_skills_project_scope(project.path(), &DeclaredSkillFilter::all(), false)
            .unwrap();
    assert_eq!(second[0].status, "unchanged");

    let source = project.path().join("skills/demo");
    fs::remove_file(source.join("drop.md")).unwrap();
    fs::write(source.join("SKILL.md"), "v2").unwrap();
    let updated =
        project_declared_skills_project_scope(project.path(), &DeclaredSkillFilter::all(), false)
            .unwrap();
    assert_eq!(updated[0].status, "updated");
    let target = project.path().join(".claude/skills/demo");
    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "v2");
    assert!(!target.join("drop.md").exists());
    assert_eq!(fs::read_to_string(neighbor).unwrap(), "foreign");
}

#[test]
fn include_is_lowered_from_manifest_by_the_orchestrator() {
    let project = project_with_skill(&["claude"], &["SKILL.md", "references/**"]);
    project_declared_skills_project_scope(project.path(), &DeclaredSkillFilter::all(), false)
        .unwrap();
    let target = project.path().join(".claude/skills/demo");
    assert!(target.join("SKILL.md").is_file());
    assert!(target.join("references/keep.md").is_file());
    assert!(!target.join("drop.md").exists());
}

#[test]
fn inventory_preserves_root_member_and_lock_order_while_skipping_malformed_slots() {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        r#"[package]
group = "org.example"
name = "root"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "EULA"
description = "root fixture"
keywords = ["fixture"]

[workspace]
members = ["zeta", "alpha"]

[[skill]]
name = "root-skill"
path = "skills/root"
"#,
    )
    .unwrap();
    fs::create_dir_all(project.path().join("skills/root")).unwrap();
    fs::write(project.path().join("skills/root/SKILL.md"), "root").unwrap();

    for (member, skill) in [("alpha", "alpha-skill"), ("zeta", "zeta-skill")] {
        let member_root = project.path().join(member);
        fs::create_dir_all(member_root.join(format!("skills/{member}"))).unwrap();
        fs::write(
            member_root.join(format!("skills/{member}/SKILL.md")),
            member,
        )
        .unwrap();
        fs::write(
            member_root.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.example\"\nname = \"{member}\"\n\
                 kind = \"tool\"\nversion = \"0.1.0\"\nauthors = [\"Fixture\"]\n\
                 license = \"EULA\"\ndescription = \"member fixture\"\n\
                 keywords = [\"fixture\"]\n\n[[skill]]\nname = \"{skill}\"\n\
                 path = \"skills/{member}\"\n"
            ),
        )
        .unwrap();
    }

    fs::write(
        project.path().join("vibe.lock"),
        r#"[meta]
generated_by = "test"
generated_at = "2026-08-26T00:00:00Z"
schema_version = 6

[[package]]
kind = "tool"
group = "org.example"
name = "locked-a"
version = "0.1.0"
source_url = "file:///locked-a"
content_hash = "sha256:aaaa"

[[package]]
kind = "tool"
group = "org.example"
name = "broken"
version = "0.1.0"
source_url = "file:///broken"
content_hash = "sha256:bbbb"

[[package]]
kind = "flow"
group = "org.example"
name = "locked-z"
version = "0.2.0"
source_url = "file:///locked-z"
content_hash = "sha256:cccc"
"#,
    )
    .unwrap();

    let deps = project
        .path()
        .join(vibe_core::layout::current_vibedeps_root());
    for (slot_name, version, kind, skill) in [
        ("org.example.locked-a", "0.1.0", "tool", "locked-a-skill"),
        ("org.example.locked-z", "0.2.0", "flow", "locked-z-skill"),
    ] {
        let package_name = slot_name.strip_prefix("org.example.").unwrap();
        let slot = deps.join(slot_name).join(version);
        fs::create_dir_all(slot.join(format!("skills/{package_name}"))).unwrap();
        fs::write(
            slot.join(format!("skills/{package_name}/SKILL.md")),
            package_name,
        )
        .unwrap();
        fs::write(
            slot.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.example\"\nname = \"{package_name}\"\n\
                 kind = \"{kind}\"\nversion = \"{version}\"\n\
                 authors = [\"Fixture\"]\nlicense = \"EULA\"\n\
                 description = \"locked fixture\"\nkeywords = [\"fixture\"]\n\n\
                 [[skill]]\nname = \"{skill}\"\npath = \"skills/{package_name}\"\n"
            ),
        )
        .unwrap();
    }
    let broken = deps.join("org.example.broken/0.1.0");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("vibe.toml"), "[package\nname = nope\n").unwrap();

    let skills = collect_declared_skills(project.path()).unwrap();
    let actual: Vec<(String, std::path::PathBuf)> = skills
        .into_iter()
        .map(|skill| (skill.origin, skill.source))
        .collect();
    assert_eq!(
        actual,
        vec![
            ("project".to_string(), project.path().join("skills/root"),),
            (
                "alpha".to_string(),
                project.path().join("alpha/skills/alpha"),
            ),
            ("zeta".to_string(), project.path().join("zeta/skills/zeta"),),
            (
                "tool:locked-a".to_string(),
                deps.join("org.example.locked-a/0.1.0/skills/locked-a"),
            ),
            (
                "flow:locked-z".to_string(),
                deps.join("org.example.locked-z/0.2.0/skills/locked-z"),
            ),
        ]
    );
}

#[test]
fn standalone_filter_diagnostics_keep_the_original_precedence_and_bytes() {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        r#"[project]
name = "empty"
version = "0.1.0"
"#,
    )
    .unwrap();

    let unknown_agent = prepare_declared_skill_projection(
        project.path(),
        &DeclaredSkillFilter::new(&[], Some("nope")),
        Scope::Project,
    )
    .unwrap_err();
    assert_eq!(
        unknown_agent.to_string(),
        "unknown --agent value `nope` (expected one of `all`, `claude`, \
         `claude-desktop`, `cursor`, `opencode`, `codex`)"
    );

    let no_match = prepare_declared_skill_projection(
        project.path(),
        &DeclaredSkillFilter::all(),
        Scope::Project,
    )
    .unwrap_err();
    assert_eq!(
        no_match.to_string(),
        "no matching skills (run `vibe skill list` to see what is declared)"
    );
}
