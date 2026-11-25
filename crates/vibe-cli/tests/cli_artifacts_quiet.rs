//! Quiet artifact inspection keeps its answer without starting progress.

use std::process::Command;

#[test]
fn quiet_artifacts_where_retains_source() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("home");
    let settings = scratch.path().join("settings");
    let drop = scratch.path().join("drop");
    for dir in [&home, &settings, &drop] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let config = settings.join("config.toml");
    let quoted_drop = serde_json::to_string(&drop.to_string_lossy()).unwrap();
    std::fs::write(
        &config,
        format!("[self]\nsource = \"localdir\"\npath = {quoted_drop}\n"),
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_vibe"))
        .args(["--quiet", "--offline", "artifacts", "where"])
        .current_dir(scratch.path())
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("VIBE_SETTINGS", &settings)
        .env_remove("VIBEVM_RELEASE_ORIGIN")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(stdout.matches("vibe releases").count(), 1);
    assert!(stdout.contains("localdir"));
    assert!(stdout.contains(config.to_string_lossy().as_ref()));
    assert!(!stdout.contains("inputs of"));
    assert!(!stdout.contains("still running"));
    assert!(result.stderr.is_empty());
}
