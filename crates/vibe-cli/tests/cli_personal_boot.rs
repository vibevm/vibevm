//! Actual capture transport reads personal boot late, without changing project prefix.
mod common;
use common::UserScratch;
use std::{fs, path::Path};

fn capture(user: &UserScratch, root: &Path) -> serde_json::Value {
    let output = user
        .vibe()
        .args(["prompt", "capture", "--path"])
        .arg(root)
        .args([
            "--session-target",
            "os=linux",
            "--with-text",
            "--json",
            "--task",
            "task-marker",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn personal_boot_is_late_current_and_never_enters_shared_prefix() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    fs::write(user.settings.join("registry.toml"), "").unwrap();
    user.init_project(project.path());
    let boot = project.path().join(vibe_core::layout::current_boot_dir());
    fs::write(boot.join("SYSTEM-TEMPLATE.md"), "project-template-marker\n").unwrap();
    fs::write(boot.join("project.md"), "project-continuation-marker\n").unwrap();
    let static_package = registry.path().join("org.personal/static/v1.0.0");
    fs::create_dir_all(static_package.join("boot")).unwrap();
    fs::write(static_package.join("vibe.toml"), "[package]\ngroup='org.personal'\nname='static'\nkind='flow'\nversion='1.0.0'\nformat='simple'\n[boot_snippet]\nsource='boot/static.md'\n").unwrap();
    fs::write(
        static_package.join("boot/static.md"),
        "# Static\n\nproject-static-marker\n",
    )
    .unwrap();
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(&manifest_path, format!("{manifest}\n[requires.packages]\n'org.personal/static'={{version='=1.0.0',link='static'}}\n")).unwrap();
    user.vibe()
        .args(["install", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let original = capture(&user, project.path());
    let prefix = original["instruction_blocks"][0].clone();
    user.vibe()
        .args(["user", "init", "--json"])
        .assert()
        .success();
    let package = registry.path().join("org.personal/boot/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.personal'\nname='boot'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    user.vibe()
        .args([
            "install",
            "-g",
            "--user-project",
            "org.personal/boot",
            "--registry",
        ])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let personal_root = user.settings.join("user-project");
    let personal_boot = personal_root.join(vibe_core::layout::current_boot_dir());
    fs::write(
        personal_boot.join("preferences.md"),
        "personal-first-marker\nMemory opt-out: use my chosen notebook instead of personal records.\n",
    )
    .unwrap();
    // Ordinary install regenerates its INDEX; no second compiler or boot-time install.
    user.vibe()
        .args(["install", "-g", "--user-project", "--registry"])
        .arg(registry.path())
        .args(["--offline", "--json"])
        .assert()
        .success();
    let authored = fs::read(personal_boot.join("preferences.md")).unwrap();
    let first = capture(&user, project.path());
    assert_eq!(
        fs::read(personal_boot.join("preferences.md")).unwrap(),
        authored
    );
    let delivered = first["instruction_blocks"].to_string();
    assert!(delivered.contains("Personal memory: when authorized"));
    assert!(delivered.contains("explicit durable opt-out"));
    assert!(delivered.contains("Memory opt-out: use my chosen notebook"));
    let blocks = first["instruction_blocks"].as_array().unwrap();
    assert_eq!(blocks[0], prefix);
    let personal = blocks
        .iter()
        .position(|block| block["band"] == "personal")
        .expect("personal lane delivered");
    assert!(blocks[..personal].iter().any(|block| {
        block["text"]
            .as_str()
            .unwrap()
            .contains("project-continuation-marker")
    }));
    assert!(blocks[personal..].iter().any(|block| {
        block["text"]
            .as_str()
            .unwrap()
            .contains("personal-first-marker")
    }));
    assert!(blocks[..personal].iter().all(|block| {
        !block["text"]
            .as_str()
            .unwrap()
            .contains("personal-first-marker")
    }));
    fs::write(
        personal_boot.join("preferences.md"),
        "personal-second-marker\n",
    )
    .unwrap();
    let second = capture(&user, project.path());
    assert_eq!(second["instruction_blocks"][0], prefix);
    assert!(
        second["instruction_blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|block| block["text"]
                .as_str()
                .unwrap()
                .contains("personal-second-marker"))
    );
    #[cfg(unix)]
    verify_launch(&user, project.path(), &boot, &personal_boot);
    let index = personal_boot.join("INDEX.md");
    let original_index = fs::read_to_string(&index).unwrap();
    fs::write(
        personal_root.join("linux-only.md"),
        "personal-linux-only-marker\n",
    )
    .unwrap();
    fs::write(
        personal_root.join("windows-only.md"),
        "personal-windows-only-marker\n",
    )
    .unwrap();
    fs::write(&index, format!("{original_index}\n[[entry]]\npath='linux-only.md'\nkind='dynamic'\nwhen='os:linux'\n[[entry]]\npath='windows-only.md'\nkind='dynamic'\nwhen='os:windows'\n")).unwrap();
    let gated = capture(&user, project.path());
    let text = gated["instruction_blocks"].to_string();
    assert!(text.contains("personal-linux-only-marker"));
    assert!(!text.contains("personal-windows-only-marker"));
    fs::write(
        &index,
        format!("{original_index}\n[[entry]]\npath='../outside-private-marker'\nkind='static'\n"),
    )
    .unwrap();
    let output = user
        .vibe()
        .args(["prompt", "capture", "--path"])
        .arg(project.path())
        .args(["--session-target", "os=linux", "--with-text", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let skipped: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        skipped["instruction_blocks"],
        original["instruction_blocks"]
    );
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("personal boot skipped"));
    assert!(!diagnostic.contains("outside-private-marker"));
    fs::write(&index, original_index).unwrap();
    fs::write(personal_root.join("vibe.toml"), "invalid [manifest").unwrap();
    let broken = capture(&user, project.path());
    assert_eq!(broken["instruction_blocks"], original["instruction_blocks"]);
}

#[cfg(unix)]
fn verify_launch(user: &UserScratch, project: &Path, boot: &Path, personal: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let shim = tempfile::tempdir().unwrap();
    let executable = shim.path().join("record-agent");
    let argv = shim.path().join("argv");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$PERSONAL_TEST_ARGV\"\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        user.settings.join("agents.toml"),
        format!(
            "[launchers.personal-test]\nkind='claude'\ncommand=['{}']\n",
            executable.display()
        ),
    )
    .unwrap();
    let prefix = fs::read(boot.join("SYSTEM.md")).unwrap();
    let launch = || {
        user.vibe()
            .current_dir(project)
            .env("PERSONAL_TEST_ARGV", &argv)
            .args(["agent", "personal-test"])
            .assert()
            .success();
        let args = fs::read_to_string(&argv).unwrap();
        let args: Vec<_> = args.lines().collect();
        let path = args
            .windows(2)
            .find(|pair| pair[0] == "--append-system-prompt-file")
            .map(|pair| std::path::PathBuf::from(pair[1]))
            .expect("actual launch slot");
        let text = fs::read_to_string(&path).unwrap();
        (path, text)
    };
    let (path, text) = launch();
    assert_eq!(path.parent().unwrap(), boot.join("local"));
    assert_eq!(fs::read(boot.join("local/.gitignore")).unwrap(), b"*\n");
    assert!(
        text.find("project-continuation-marker").unwrap()
            < text.find("personal-second-marker").unwrap()
    );
    assert_eq!(text.matches("project-static-marker").count(), 1);
    assert!(
        text.find("project-static-marker").unwrap()
            < text.find("project-continuation-marker").unwrap()
    );
    assert!(!text.contains("project-template-marker"));
    assert_eq!(fs::read(boot.join("SYSTEM.md")).unwrap(), prefix);
    fs::write(
        personal.join("preferences.md"),
        "personal-launch-edited-marker\n",
    )
    .unwrap();
    let (next, text) = launch();
    assert_ne!(next, path);
    assert!(text.contains("personal-launch-edited-marker"));
    assert!(!text.contains("personal-second-marker"));
    assert_eq!(fs::read(boot.join("SYSTEM.md")).unwrap(), prefix);
    let ignore = boot.join("local/.gitignore");
    fs::write(&ignore, "user-owned-ignore\n").unwrap();
    let output = user
        .vibe()
        .current_dir(project)
        .env("PERSONAL_TEST_ARGV", &argv)
        .args(["agent", "personal-test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("personal launch boot skipped"));
    let args = fs::read_to_string(&argv).unwrap();
    assert!(args.contains(boot.join("AGENT-LANE.md").to_str().unwrap()));
    assert!(!args.contains(next.to_str().unwrap()));
    assert_eq!(fs::read_to_string(&ignore).unwrap(), "user-owned-ignore\n");
}

#[test]
fn classic_init_without_system_template_has_late_personal_discovery_contract() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let boot = project.path().join(vibe_core::layout::current_boot_dir());
    assert!(!boot.join("SYSTEM-TEMPLATE.md").exists());
    assert!(!boot.join("SYSTEM.md").exists());
    assert!(!user.settings.join("user-project").exists());
    for name in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
        let text = fs::read_to_string(project.path().join(name)).unwrap();
        let personal = text.find("# Optional personal boot — read last").unwrap();
        assert!(text[..personal].contains("INDEX"));
        assert!(text.contains("After all project INDEX entries/live continuation"));
        assert!(text.contains("nonempty VIBE_SETTINGS, else HOME"));
        assert!(text.contains("not mcp/user-project"));
        assert!(text.contains("initialize/install/compile/repair"));
        assert!(text.contains("settings/override-bindings.toml (schema=1)"));
        assert!(text.contains("Disabled means do not read that project"));
        assert!(text.contains("subagent\nquiet clause"));
        assert!(text.contains("materialised dependency copy, ignore this block"));
        assert!(!text.contains(user.settings.to_str().unwrap()));
        assert!(!text.contains("~/.vibe/user-project"));
    }
    assert!(!boot.join("local").exists());
}

#[test]
fn authored_personal_memory_record_is_indexed_and_read_by_next_request_without_prefix_change() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let boot = project.path().join(vibe_core::layout::current_boot_dir());
    fs::write(boot.join("SYSTEM-TEMPLATE.md"), "memory-project-frame\n").unwrap();
    user.vibe()
        .args(["prompt", "build", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let prefix = fs::read(boot.join("SYSTEM.md")).unwrap();
    user.vibe().args(["user", "init"]).assert().success();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    let personal = user.settings.join("user-project");
    let personal_boot = personal.join(vibe_core::layout::current_boot_dir());
    // This fixture is the explicit authorized authored-memory write, not a model call.
    let record = personal_boot.join("preferences-memory.md");
    fs::write(
        &record,
        "# User memory\n\nUSER_PREFERS_CONCISE_RUSSIAN_CANARY\n",
    )
    .unwrap();
    let common = personal
        .join(vibe_core::layout::current_specs_root())
        .join("common");
    fs::create_dir_all(&common).unwrap();
    fs::write(
        common.join("unlinked-memory.md"),
        "UNLINKED_MEMORY_CANARY\n",
    )
    .unwrap();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    assert!(
        fs::read_to_string(personal_boot.join("INDEX.md"))
            .unwrap()
            .contains("preferences-memory.md")
    );
    let first = capture(&user, project.path());
    let text = first["instruction_blocks"].to_string();
    assert_eq!(
        text.matches("USER_PREFERS_CONCISE_RUSSIAN_CANARY").count(),
        1
    );
    assert!(!text.contains("UNLINKED_MEMORY_CANARY"));
    assert_eq!(fs::read(boot.join("SYSTEM.md")).unwrap(), prefix);
    let changed = "# User memory\n\nUSER_PREFERS_DETAILED_RUSSIAN_CANARY\nMemory opt-out: use my chosen notebook for new facts.\n";
    fs::write(&record, changed).unwrap();
    let second = capture(&user, project.path());
    let text = second["instruction_blocks"].to_string();
    assert!(text.contains("USER_PREFERS_DETAILED_RUSSIAN_CANARY"));
    assert!(!text.contains("USER_PREFERS_CONCISE_RUSSIAN_CANARY"));
    assert!(text.contains("Memory opt-out: use my chosen notebook"));
    assert_eq!(fs::read_to_string(&record).unwrap(), changed);
    assert_eq!(fs::read(boot.join("SYSTEM.md")).unwrap(), prefix);
}
