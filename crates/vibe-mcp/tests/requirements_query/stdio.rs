//! The PRODUCTION stdio proof.
//!
//! `dispatch_one` exercises the dispatcher over an in-memory transport, and
//! that is exactly what cannot answer the question this cell asks: a
//! `println!` anywhere under the relation scan would never reach a
//! `MemoryTransport`, it would reach the process's real stdout — the same
//! file descriptor the JSON-RPC frames travel on. So the server here is the
//! real [`vibe_mcp::Server::stdio`] in a real child process, reading a real
//! pipe, and the assertion is on that process's actual bytes.
//!
//! The child is this same test binary, re-executed at one named helper (the
//! `lease` cells use the same technique). It prints one marker line and then
//! hands the stream to the server, so the boundary between the test
//! harness's own preamble and the SERVER's output is explicit; after
//! `run` returns it exits immediately, so the harness cannot append its
//! summary either. Everything after the marker is therefore what the server
//! and everything it called wrote — and the law is that this is exactly one
//! frame.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::support::{ADDRESS, HOST, PROSE, REQUEST_ID, project_with_map, request};

/// The child announces itself with this exact line; the parent splits on it.
const MARKER: &str = "--- vibe-mcp stdio server begins ---";
/// Set by the parent only. Absent, the helper below is an immediate no-op.
const CHILD_ROOT: &str = "VIBE_MCP_REQUIREMENTS_STDIO_ROOT";
/// The helper's path inside this test binary, for `--exact`.
const CHILD_TEST: &str = "stdio::stdio_server_child";

#[test]
fn production_stdio_emits_exactly_one_frame_while_the_relation_scan_runs() {
    let project = project_with_map();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(CHILD_TEST)
        // Without this the harness captures `println!`, which would HIDE
        // exactly the contamination this cell exists to catch.
        .arg("--nocapture")
        .env(CHILD_ROOT, project.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the test binary re-executes");

    // One request, then EOF — the server's loop ends on the closed pipe.
    let line = request(json!({ "relations": true })) + "\n";
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(line.as_bytes())
        .unwrap();
    let output = child.wait_with_output().expect("the child terminates");
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    let (_preamble, served) = stdout
        .split_once(MARKER)
        .unwrap_or_else(|| panic!("the child never reached the server: {stdout:?}"));
    let frames: Vec<&str> = served
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        frames.len(),
        1,
        "the tool call, relation scan included, wrote exactly one line to the \
         real stdout — got {frames:?}"
    );

    let response: Value = serde_json::from_str(frames[0])
        .unwrap_or_else(|error| panic!("the one line is one JSON-RPC frame ({error}): {served:?}"));
    assert!(response["error"].is_null(), "{response}");
    assert_eq!(response["id"], REQUEST_ID);
    assert_eq!(response["result"]["isError"], false);

    // …and it is the real answer, not an empty shell: the scan ran through
    // the production transport and reported a fresh project map.
    let structured = &response["result"]["structuredContent"];
    assert_eq!(structured["rows"][0]["address"], ADDRESS);
    assert_eq!(structured["relation_sources"][0]["package"], HOST);
    assert_eq!(structured["relation_sources"][0]["state"], "current");
    assert_eq!(
        structured["relation_sources"][0]["provenance"],
        "fresh-project-map"
    );
    assert!(!structured.to_string().contains(PROSE));
}

/// The child half: the production stdio server over this process's real
/// stdin/stdout. A no-op unless the parent set the project root.
#[test]
fn stdio_server_child() {
    let Ok(root) = std::env::var(CHILD_ROOT) else {
        return;
    };
    {
        // Written to the REAL handle, not through `println!`: the marker
        // must sit on the same stream the server is about to use.
        let mut out = std::io::stdout().lock();
        out.write_all(format!("{MARKER}\n").as_bytes()).unwrap();
        out.flush().unwrap();
    }
    let mut server = vibe_mcp::Server::stdio(vibe_mcp::ServerContext::new(root));
    server.run().expect("the stdio server runs to end-of-input");
    // Exit before the harness can append its own summary to this stream:
    // everything after the marker must be the server's bytes alone.
    std::process::exit(0);
}
