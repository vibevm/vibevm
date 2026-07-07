//! Shared harness for the integration suites: drive the BUILT server
//! binary over real stdio. A subprocess is the honest transport here
//! twice over — main.rs is under test too, and the stderr-capture
//! guard only sees a full report outside libtest's own output
//! diversion (the same lesson mcp-core's capture suite records).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// PATH without any dir that carries a vibe binary — serving must not
/// need the product (PROP-027 §2.6); BOTH suites inherit the law.
pub fn vibe_free_path() -> String {
    let path = std::env::var("PATH").unwrap_or_default();
    let sep = if cfg!(windows) { ';' } else { ':' };
    let exe = if cfg!(windows) { "vibe.exe" } else { "vibe" };
    path.split(sep)
        .filter(|dir| !PathBuf::from(dir).join(exe).exists())
        .collect::<Vec<_>>()
        .join(&sep.to_string())
}

pub struct Session {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl Session {
    pub fn spawn(root: &Path) -> Session {
        let mut child = Command::new(env!("CARGO_BIN_EXE_discipline-mcp-typescript"))
            .arg("--path")
            .arg(root)
            .env("PATH", vibe_free_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the server binary");
        let stdout = child.stdout.take().expect("stdout piped");
        Session {
            reader: BufReader::new(stdout),
            child,
        }
    }

    pub fn call(&mut self, frame: serde_json::Value) -> serde_json::Value {
        let stdin = self.child.stdin.as_mut().expect("stdin piped");
        writeln!(stdin, "{frame}").expect("write frame");
        stdin.flush().expect("flush");
        let mut line = String::new();
        self.reader.read_line(&mut line).expect("read answer");
        serde_json::from_str(&line).expect("answer is JSON")
    }

    pub fn tool(&mut self, id: u64, name: &str, args: serde_json::Value) -> serde_json::Value {
        self.call(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": { "name": name, "arguments": args },
        }))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
