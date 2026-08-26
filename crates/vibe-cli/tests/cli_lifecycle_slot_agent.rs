//! A slot-scoped agent row reaches the configured backend on every CLI path
//! that runs the slot lifecycle — install, reinstall and update.
//!
//! Each command constructs its own `InstallSlotLifecycle`, so each needs its
//! own evidence: replacing any one call site's `CliAgentBackend` with the
//! refusing seams turns that command's assertion red, because the counter
//! stops moving and the declared output stops appearing.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::agent_provider::{MockProvider, configure_provider};
use common::{UserScratch, git_available, run_git, write_project_with_per_package_registry};

const RESULT: &str = r#"{"outputs":[{"path":"docs/slot.md","content":"slot body\n"}]}"#;

/// The source repo of one package whose only contribution is a
/// `slot:post-install` agent row.
fn slot_agent_source(root: &Path) -> PathBuf {
    let source = root.join("src-slot-agent");
    fs::create_dir_all(source.join("vibevm/vibespecs/common")).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    fs::write(
        source.join(".gitattributes"),
        "* text=auto eol=lf
",
    )
    .unwrap();
    source
}

/// Commit and tag one more published version. The prompt body carries the
/// version marker, so the request bytes say which instance answered.
fn add_version(source: &Path, version: &str, payload: &str) {
    fs::write(
        source.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.demo"
name = "tools"
kind = "flow"
version = "{version}"

[[extension]]
id = "slot-produce"
point = "slot:post-install"
handler = {{ kind = "agent", prompt = "spec://org.demo/tools/common/agent-prompt#root" }}
config.outputs = [
  {{ path = "docs/slot.md", kind = "file", accept = "non-empty file" }},
]
"#
        ),
    )
    .unwrap();
    fs::write(
        source.join("vibevm/vibespecs/common/agent-prompt.md"),
        format!(
            "# Prompt {{#root}}

Write the slot document. MARKER=SLOT-{version}
"
        ),
    )
    .unwrap();
    fs::write(source.join("payload.txt"), payload).unwrap();
    run_git(source, &["add", "-A"]);
    run_git(
        source,
        &["commit", "-m", &format!("org.demo/tools@{version}")],
    );
    run_git(source, &["tag", &format!("v{version}")]);
}

/// Clone the source into the per-package bare repo the registry serves.
fn publish(root: &Path, source: &Path) -> PathBuf {
    let bare = root.join("org.demo.tools.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    root.to_path_buf()
}

fn registry_url(registry: &Path) -> String {
    format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    )
}

fn slot_dir(project: &Path, version: &str) -> PathBuf {
    project
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.demo.tools")
        .join(version)
}

#[test]
fn install_reinstall_and_update_reach_the_configured_agent_backend() {
    if !git_available() {
        eprintln!("skipping slot-agent CLI e2e: git not on PATH");
        return;
    }

    let provider = MockProvider::serving(RESULT);
    let outer = tempfile::tempdir().unwrap();
    let source = slot_agent_source(outer.path());
    add_version(&source, "0.1.0", "payload one\n");
    let registry = publish(outer.path(), &source);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    configure_provider(&user, &provider.endpoint());

    // ---- install ---------------------------------------------------------
    let installed = user
        .vibe()
        .arg("install")
        .arg("org.demo/tools")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    assert_eq!(
        provider.hits(),
        1,
        "direct install must reach the configured backend"
    );
    assert!(
        provider.bodies().join("\n").contains("SLOT-0.1.0"),
        "and must resolve the slot package's own prompt"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/slot.md")).unwrap(),
        "slot body\n",
    );

    // ---- reinstall --force ----------------------------------------------
    // A changed payload makes the force pass re-materialise the slot, which is
    // what re-fires the slot lifecycle.
    fs::write(
        slot_dir(project.path(), "0.1.0").join("payload.txt"),
        "corrupted\n",
    )
    .unwrap();
    fs::remove_file(project.path().join("docs/slot.md")).unwrap();
    let before = provider.hits();
    let reinstalled = user
        .vibe()
        .arg("reinstall")
        .arg(project.path())
        .arg("--force")
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        reinstalled.status.success(),
        "reinstall: {}",
        String::from_utf8_lossy(&reinstalled.stderr)
    );
    assert!(
        provider.hits() > before,
        "`reinstall` must reach the configured backend, not the refusing seams",
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/slot.md")).unwrap(),
        "slot body\n",
        "`reinstall` writes the declared output",
    );

    // ---- update ----------------------------------------------------------
    // The *scoped* form is the one that owns a slot-lifecycle call site of its
    // own (`--all` delegates to `install`, which would prove that seam
    // instead). A corrupted slot payload makes the verified re-materialisation
    // repair the slot, which is what re-fires the slot lifecycle — no version
    // bump needed, and no dependence on registry-cache refresh timing.
    fs::write(
        slot_dir(project.path(), "0.1.0").join("payload.txt"),
        "corrupted again
",
    )
    .unwrap();
    fs::remove_file(project.path().join("docs/slot.md")).unwrap();
    let before = provider.hits();
    let updated = user
        .vibe()
        .arg("update")
        .arg("org.demo/tools")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        updated.status.success(),
        "update: {}",
        String::from_utf8_lossy(&updated.stderr)
    );
    assert!(
        provider.hits() > before,
        "`update` must reach the configured backend, not the refusing seams; update said: {}{}",
        String::from_utf8_lossy(&updated.stdout),
        String::from_utf8_lossy(&updated.stderr),
    );
    assert!(
        provider
            .bodies()
            .join(
                "
"
            )
            .contains("SLOT-0.1.0"),
        "and must resolve the slot package's own prompt",
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/slot.md")).unwrap(),
        "slot body
",
        "`update` writes the declared output",
    );
}
