use super::*;

#[test]
fn child_process_cannot_observe_explicit_publish_or_github_credentials() {
    let mut child = credential_probe();
    child
        .env("VIBEVM_PUBLISH_TOKEN_GITHUB", "do-not-inherit")
        .env("VIBEVM_PUBLISH_TOKEN_GITVERSE", "do-not-inherit")
        .env("VIBEVM_PUBLISH_TOKEN", "do-not-inherit")
        .env("GITHUB_TOKEN", "do-not-inherit")
        .env("GH_TOKEN", "do-not-inherit")
        .env("GIT_CONFIG_VALUE_0", "http.extraHeader=secret")
        .env("DIST_SAFE_MARKER", "preserved");
    scrub_release_credentials(&mut child);
    let output = child.output().expect("credential probe starts");
    assert!(
        output.status.success(),
        "credential probe failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
fn credential_probe() -> Command {
    let mut command = Command::new("cmd");
    command.args([
        "/d",
        "/c",
        "if not defined VIBEVM_PUBLISH_TOKEN_GITHUB if not defined VIBEVM_PUBLISH_TOKEN_GITVERSE if not defined VIBEVM_PUBLISH_TOKEN if not defined GITHUB_TOKEN if not defined GH_TOKEN if not defined GIT_CONFIG_VALUE_0 if \"%DIST_SAFE_MARKER%\"==\"preserved\" exit /b 0 & exit /b 9",
    ]);
    command
}

#[cfg(unix)]
fn credential_probe() -> Command {
    let mut command = Command::new("sh");
    command.args([
        "-c",
        "test -z \"${VIBEVM_PUBLISH_TOKEN_GITHUB+x}\" && test -z \"${VIBEVM_PUBLISH_TOKEN_GITVERSE+x}\" && test -z \"${VIBEVM_PUBLISH_TOKEN+x}\" && test -z \"${GITHUB_TOKEN+x}\" && test -z \"${GH_TOKEN+x}\" && test -z \"${GIT_CONFIG_VALUE_0+x}\" && test \"$DIST_SAFE_MARKER\" = preserved",
    ]);
    command
}
