mod common;

use std::fs;

use common::UserScratch;

#[test]
fn post_fail_reply_keeps_bounded_script_streams_in_generated_outcome() {
    let registry = tempfile::tempdir().unwrap();
    let package = registry
        .path()
        .join("org.reply")
        .join("failed")
        .join("v0.1.0");
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.reply'\nname='failed'\nkind='tool'\nversion='0.1.0'\n\n[hooks]\npost-install='hooks/post'\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/post.sh"),
        "printf FAIL-REPLY-OUT\npython -c \"import sys; sys.stderr.write('FAIL-REPLY-ERR' + 'x' * 1048600)\"\nprintf '%s' '{\"artifacts\":[],\"envelope\":1,\"message\":\"semantic\",\"status\":\"fail\",\"tasks\":[]}' > \"$VIBE_REPLY\"\n",
    )
    .unwrap();
    fs::write(
        package.join("hooks/post.ps1"),
        "Write-Output FAIL-REPLY-OUT\n[Console]::Error.Write('FAIL-REPLY-ERR' + ('x' * 1048600))\n'{\"artifacts\":[],\"envelope\":1,\"message\":\"semantic\",\"status\":\"fail\",\"tasks\":[]}' | Set-Content -LiteralPath $env:VIBE_REPLY -NoNewline\n",
    )
    .unwrap();
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let output = user
        .vibe()
        .args(["install", "org.reply/failed@=0.1.0", "--json", "--registry"])
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let row = docs
        .iter()
        .filter(|doc| doc["command"] == "lifecycle")
        .flat_map(|doc| doc["contributions"].as_array().into_iter().flatten())
        .find(|row| row["point"] == "slot:post-install")
        .unwrap();
    assert_eq!(row["status"], "fail");
    assert_eq!(row["flagged"], true);
    assert!(row["stdout"].as_str().unwrap().contains("FAIL-REPLY-OUT"));
    assert!(row["stderr"].as_str().unwrap().contains("FAIL-REPLY-ERR"));
    assert_eq!(row["stderr"].as_str().unwrap().len(), 1024 * 1024);
    assert_eq!(row["stderr_truncated"], true);
    assert_eq!(docs.last().unwrap()["command"], "install");
}
