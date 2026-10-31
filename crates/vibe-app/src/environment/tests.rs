//! The managed `env.sh` / `env.fish` lines and startup-file blocks.

use super::*;

fn entry(step: &str, op: EnvOp, name: &str, value: Option<&str>) -> EnvEntry {
    EnvEntry {
        step: step.into(),
        op,
        name: name.into(),
        value: value.map(str::to_owned),
        previous: None,
        previous_kind: None,
    }
}

#[test]
fn entries_of_two_installations_share_the_files_and_leave_foreign_bytes_alone() {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("settings");
    let home = dir.path().join("home");
    fs::create_dir_all(home.join(".config/fish")).unwrap();
    fs::write(home.join(".bashrc"), "alias ll='ls -l'").unwrap();
    fs::write(home.join(".config/fish/config.fish"), "set -x EDITOR vi\n").unwrap();
    let env = ProfileEnvironment::new(&settings, &home);
    let tool = [
        entry("env", EnvOp::Set, "TOOL_HOME", Some("/opt/it's")),
        entry("env", EnvOp::PathPrepend, "PATH", Some("/opt/tool/bin")),
        entry("env", EnvOp::Unset, "TOOL_DEBUG", None),
    ];
    let other = [entry("vars", EnvOp::Set, "OTHER", Some("1"))];
    env.apply("org.example/tool@1.0.0", &tool).unwrap();
    env.apply("org.example/other@2.0.0", &other).unwrap();

    let script = fs::read_to_string(env.script_path()).unwrap();
    assert!(script.contains(
        r"export TOOL_HOME='/opt/it'\''s' # vibe-app org.example/tool@1.0.0 env set TOOL_HOME"
    ));
    assert!(script.contains("export PATH='/opt/tool/bin'\"${PATH:+:$PATH}\""));
    assert!(script.contains("unset TOOL_DEBUG # vibe-app"));
    assert!(script.contains("export OTHER='1' # vibe-app org.example/other@2.0.0"));
    let fish = fs::read_to_string(env.fish_path()).unwrap();
    assert!(fish.contains("set -gx PATH '/opt/tool/bin' $PATH"));

    let bashrc = fs::read_to_string(home.join(".bashrc")).unwrap();
    assert!(bashrc.starts_with("alias ll='ls -l'\n# >>> vibe"));
    assert_eq!(bashrc.matches(blocks::BEGIN).count(), 1, "{bashrc}");
    assert!(
        !home.join(".zshrc").exists(),
        "absent startup files are not created"
    );
    let config = fs::read_to_string(home.join(".config/fish/config.fish")).unwrap();
    assert!(config.contains("source '") && config.contains("env.fish'"));

    env.remove("org.example/tool@1.0.0", &tool).unwrap();
    let script = fs::read_to_string(env.script_path()).unwrap();
    assert!(!script.contains("TOOL"), "{script}");
    assert!(script.contains("OTHER"));
    env.remove("org.example/other@2.0.0", &other).unwrap();
    assert!(!env.script_path().exists());
    assert!(!env.fish_path().exists());
    assert_eq!(
        fs::read_to_string(home.join(".bashrc")).unwrap(),
        "alias ll='ls -l'\n"
    );
    assert_eq!(
        fs::read_to_string(home.join(".config/fish/config.fish")).unwrap(),
        "set -x EDITOR vi\n"
    );
    assert_eq!(fs::read_to_string(home.join(".profile")).unwrap(), "");
}

#[test]
fn a_block_is_rewritten_in_place_and_never_duplicated() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(".profile");
    fs::write(&file, "before\n").unwrap();
    blocks::ensure(&file, "first").unwrap();
    fs::write(
        &file,
        format!("{}after\n", fs::read_to_string(&file).unwrap()),
    )
    .unwrap();
    blocks::ensure(&file, "second").unwrap();
    blocks::ensure(&file, "second").unwrap();
    let text = fs::read_to_string(&file).unwrap();
    assert_eq!(
        text,
        format!(
            "before\n{}\nsecond\n{}\nafter\n",
            blocks::BEGIN,
            blocks::END
        )
    );
    blocks::remove(&file).unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "before\nafter\n");
}
