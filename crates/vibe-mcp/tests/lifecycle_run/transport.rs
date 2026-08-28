use std::fs;

use serde_json::json;

use super::support::{append, context, dispatch, project};

#[test]
fn captured_script_stdout_stays_inside_one_parseable_jsonrpc_frame() {
    let project = project("");
    fs::create_dir_all(project.path().join("scripts")).unwrap();
    fs::write(
        project.path().join("scripts/run.sh"),
        "echo MCP-CAPTURE-MARKER\nprintf '%s' '{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' > \"$VIBE_REPLY\"\n",
    )
    .unwrap();
    fs::write(
        project.path().join("scripts/run.ps1"),
        "Write-Output 'MCP-CAPTURE-MARKER'\n'{\"artifacts\":[],\"envelope\":1,\"status\":\"ok\",\"tasks\":[]}' | Set-Content -NoNewline $env:VIBE_REPLY\n",
    )
    .unwrap();
    append(
        project.path(),
        r#"
[[extension]]
id = "phase-script"
point = "phase:build"
handler = { kind = "script", base = "scripts/run" }
inputs = ["scripts/**"]
"#,
    );

    let response = dispatch(context(project.path()), json!({ "phase": "build" }));
    assert_eq!(response["result"]["isError"], false);
    let contributions = response["result"]["structuredContent"]["contributions"]
        .as_array()
        .unwrap();
    assert!(contributions.iter().any(|row| {
        row["stdout"]
            .as_str()
            .is_some_and(|stdout| stdout.contains("MCP-CAPTURE-MARKER"))
    }));
    // `dispatch` parsed the server's complete output as exactly one JSON value.
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 7);
}
