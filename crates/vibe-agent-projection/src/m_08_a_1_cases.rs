//! Bounded legacy cases; every filesystem root is a temporary fixture.
use crate::agents::{Agent, AgentUserDirectories, Scope};
use crate::pkgskill::*;
use specmark::verifies;
use std::{collections::BTreeSet, fs, path::Path};
use vibe_core::{Group, PackageKind, PackageName, manifest::SkillDecl};

const FIRST: &[u8] = include_bytes!("../../../fixtures/next/adapters/skill-v1.md");
const SECOND: &[u8] = include_bytes!("../../../fixtures/next/adapters/skill-v2.md");
const CLIENTS: [(Agent, &str, &str); 4] = [
    (Agent::ClaudeCode, ".claude/skills", ".claude/skills"),
    (Agent::Codex, ".agents/skills", ".agents/skills"),
    (
        Agent::OpenCode,
        ".opencode/skills",
        ".config/opencode/skills",
    ),
    (Agent::QwenCode, ".qwen/skills", ".qwen/skills"),
];
fn input(root: &Path, agent: Agent) -> ProjectSkillProviderInput {
    ProjectSkillProviderInput {
        provider: DeclaredSkillProvider::Authored {
            group: Group::parse("org.example").unwrap(),
            name: PackageName::parse("fixture").unwrap(),
            version: "1.0.0".into(),
            kind: PackageKind::Tool,
            root: root.into(),
            embedded_sources: vec![],
        },
        declarations: vec![SkillDecl {
            name: "demo".into(),
            source: None,
            path: "source".into(),
            description: None,
            agents: vec![agent.as_str().into()],
            include: vec![],
            resources: vec![],
        }],
    }
}
fn binding(project: &Path, source: &Path, agent: Agent) -> ProjectSkillBinding {
    lower_project_skill_bindings(project, vec![input(source, agent)])
        .unwrap()
        .remove(0)
}
fn seed(root: &Path, tree: bool) -> std::path::PathBuf {
    let source = root.join("source");
    if tree {
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), FIRST).unwrap();
        fs::write(source.join("reference.txt"), b"inert reference\r\n").unwrap();
    } else {
        fs::write(&source, FIRST).unwrap();
    }
    source
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-SURFACES")]
fn injected_paths_and_mcp_flags_are_separate_from_skill_support() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let project = tmp.path().join("project");
    let dirs = AgentUserDirectories {
        home: Some(home.clone()),
        config: Some(tmp.path().join("config")),
    };
    for (agent, local, user) in CLIENTS {
        assert_eq!(
            agent.skills_root(Scope::Project, Some(&project)).unwrap(),
            Some(project.join(local))
        );
        assert_eq!(
            agent.user_skill_entry_from_home(&home, "demo"),
            Some(home.join(user).join("demo/SKILL.md"))
        );
    }
    for (agent, local, user) in [
        (Agent::ClaudeCode, Some(".mcp.json"), ".claude.json"),
        (
            Agent::Codex,
            Some(".codex/config.toml"),
            ".codex/config.toml",
        ),
        (
            Agent::OpenCode,
            Some("opencode.json"),
            ".config/opencode/opencode.json",
        ),
        (
            Agent::QwenCode,
            Some(".qwen/settings.json"),
            ".qwen/settings.json",
        ),
        (Agent::Cursor, Some(".cursor/mcp.json"), ".cursor/mcp.json"),
        (
            Agent::ClaudeCodeDesktop,
            None,
            "Claude/claude_desktop_config.json",
        ),
    ] {
        assert_eq!(agent.supports_project_scope(), local.is_some());
        assert_eq!(
            agent
                .config_path_with_dirs(Scope::Project, Some(&project), &dirs)
                .unwrap(),
            local.map(|p| project.join(p))
        );
        let base = if agent == Agent::ClaudeCodeDesktop {
            dirs.config.as_ref().unwrap()
        } else {
            &home
        };
        assert_eq!(
            agent
                .config_path_with_dirs(Scope::User, None, &dirs)
                .unwrap(),
            Some(base.join(user))
        );
        assert!(
            agent
                .config_path_with_dirs(Scope::Both, Some(&project), &dirs)
                .is_err()
        );
    }
    for agent in [Agent::Cursor, Agent::ClaudeCodeDesktop] {
        assert!(!agent.supports_skill());
        assert!(agent.user_skills_root_from_home(&home).is_none());
    }
    assert!(!home.exists());
    assert!(!project.exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-SURFACES")]
fn standalone_project_file_and_tree_install_update_remove_exact_bytes() {
    for (agent, local, _) in CLIENTS {
        for tree in [false, true] {
            let entry = if tree { "SKILL.md" } else { "source" };
            let project = tempfile::tempdir().unwrap();
            let package = tempfile::tempdir().unwrap();
            let source = seed(package.path(), tree);
            let install = || {
                install_package_skill(
                    agent,
                    Scope::Project,
                    Some(project.path()),
                    "demo",
                    &source,
                    false,
                )
                .unwrap()
            };
            assert_eq!(install().status, "created");
            let target = project.path().join(local).join("demo");
            assert_eq!(fs::read(target.join(entry)).unwrap(), FIRST);
            assert_eq!(install().status, "unchanged");
            let receipt = target
                .parent()
                .unwrap()
                .join(".demo.vibe-skill-receipt.toml");
            let value: toml::Value =
                toml::from_str(&fs::read_to_string(&receipt).unwrap()).unwrap();
            assert_eq!(value["skill"].as_str(), Some("demo"));
            assert_eq!(value["schema"].as_integer(), Some(1));
            fs::write(target.join("foreign.txt"), b"keep\n").unwrap();
            if tree {
                fs::remove_file(source.join("reference.txt")).unwrap();
            }
            fs::write(
                if tree {
                    source.join("SKILL.md")
                } else {
                    source.clone()
                },
                SECOND,
            )
            .unwrap();
            assert_eq!(install().status, "updated");
            assert_eq!(fs::read(target.join(entry)).unwrap(), SECOND);
            assert!(!target.join("reference.txt").exists());
            assert_eq!(
                uninstall_package_skill(agent, Scope::Project, Some(project.path()), "demo", false)
                    .unwrap()
                    .status,
                "removed"
            );
            assert!(!receipt.exists());
            assert!(!target.join(entry).exists());
            assert_eq!(fs::read(target.join("foreign.txt")).unwrap(), b"keep\n");
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-SURFACES")]
fn automatic_file_and_tree_bindings_install_update_vanish_with_own_receipt() {
    for (agent, local, _) in CLIENTS {
        for tree in [false, true] {
            let entry = if tree { "SKILL.md" } else { "source" };
            let project = tempfile::tempdir().unwrap();
            let package = tempfile::tempdir().unwrap();
            let source = seed(package.path(), tree);
            let first = binding(project.path(), package.path(), agent);
            assert_eq!(
                first.identity(),
                "@vibe/package/skill/org.example/fixture/demo"
            );
            assert_eq!(
                first.targets[0].path,
                project.path().join(local).join("demo")
            );
            reconcile_project_skill_binding(project.path(), &first).unwrap();
            let target = &first.targets[0].path;
            assert_eq!(fs::read(target.join(entry)).unwrap(), FIRST);
            assert!(project.path().join(".vibe/package-skills.toml").is_file());
            assert!(
                !target
                    .parent()
                    .unwrap()
                    .join(".demo.vibe-skill-receipt.toml")
                    .exists()
            );
            fs::write(target.join("foreign.txt"), b"keep\n").unwrap();
            if tree {
                fs::remove_file(source.join("reference.txt")).unwrap();
            }
            fs::write(
                if tree {
                    source.join("SKILL.md")
                } else {
                    source.clone()
                },
                SECOND,
            )
            .unwrap();
            reconcile_project_skill_binding(
                project.path(),
                &binding(project.path(), package.path(), agent),
            )
            .unwrap();
            assert_eq!(fs::read(target.join(entry)).unwrap(), SECOND);
            assert!(!target.join("reference.txt").exists());
            reconcile_vanished_project_skill_bindings(project.path(), &BTreeSet::new()).unwrap();
            assert!(!target.join(entry).exists());
            assert_eq!(fs::read(target.join("foreign.txt")).unwrap(), b"keep\n");
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-OWNERSHIP")]
fn equal_bytes_do_not_transfer_between_standalone_and_automatic_receipts() {
    for (agent, local, _) in CLIENTS {
        for standalone_first in [true, false] {
            let project = tempfile::tempdir().unwrap();
            let package = tempfile::tempdir().unwrap();
            let source = seed(package.path(), false);
            let binding = binding(project.path(), package.path(), agent);
            if standalone_first {
                install_package_skill(
                    agent,
                    Scope::Project,
                    Some(project.path()),
                    "demo",
                    &source,
                    false,
                )
                .unwrap();
                assert!(
                    reconcile_project_skill_binding(project.path(), &binding)
                        .unwrap_err()
                        .to_string()
                        .contains("unowned")
                );
            } else {
                reconcile_project_skill_binding(project.path(), &binding).unwrap();
                assert!(
                    install_package_skill(
                        agent,
                        Scope::Project,
                        Some(project.path()),
                        "demo",
                        &source,
                        false
                    )
                    .is_err()
                );
            }
            assert_eq!(
                fs::read(project.path().join(local).join("demo/source")).unwrap(),
                FIRST
            );
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LEGACY-ADAPTER-OWNERSHIP")]
fn unsupported_clients_empty_trees_and_foreign_directory_shapes_are_explicit() {
    let project = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let source = seed(package.path(), false);
    for agent in [Agent::Cursor, Agent::ClaudeCodeDesktop] {
        assert_eq!(
            install_package_skill(
                agent,
                Scope::Project,
                Some(project.path()),
                "demo",
                &source,
                false
            )
            .unwrap()
            .status,
            "skipped"
        );
        assert!(
            lower_project_skill_bindings(project.path(), vec![input(package.path(), agent)])
                .is_err()
        );
    }
    fs::remove_file(&source).unwrap();
    fs::create_dir(&source).unwrap();
    assert!(
        lower_project_skill_bindings(project.path(), vec![input(package.path(), Agent::Codex)])
            .unwrap_err()
            .to_string()
            .contains("zero files")
    );
    fs::write(source.join("SKILL.md"), FIRST).unwrap();
    let target = project.path().join(".agents/skills/demo/SKILL.md");
    fs::create_dir_all(&target).unwrap();
    assert!(
        install_package_skill(
            Agent::Codex,
            Scope::Project,
            Some(project.path()),
            "demo",
            &source,
            false
        )
        .is_err()
    );
    assert!(target.is_dir());
}
