//! The oracle op surface over the LSP client (TCG-PROTOCOL-RUST §2):
//! overlay documents under LSP version law, single-document pull
//! diagnostics, completion, hover, and the shutdown dance. Positions
//! cross the boundary through the position cell against the document's
//! OWN text.

specmark::scope!("spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#overlays");

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use specmark::spec;

use crate::client::{Capabilities, ChildTransport, LspClient, Transport};
use crate::position::{OuterPosition, from_lsp, to_lsp};
use crate::{TcgBridgeError, resolve_rust_analyzer, verbatim_free};

const HANDSHAKE_BUDGET: Duration = Duration::from_secs(30);
const OP_BUDGET: Duration = Duration::from_secs(30);

/// One diagnostic in the OUTER convention (TCG-PROTOCOL-RUST §2):
/// 1-based line, 0-based character, code as the server names it.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub category: String,
    pub message: String,
    pub line: u32,
    pub character: u32,
}

/// A validate answer before enrichment: diagnostics + the degraded
/// flag (ORACLE-RUST §6 — pre-quiescent answers are legal but
/// labelled).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidateOutcome {
    pub diagnostics: Vec<Diagnostic>,
    pub degraded: bool,
}

/// A raw completion entry (the relay finalises `unsafe` policy-side).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Completion {
    pub name: String,
    pub kind: Option<u64>,
    pub type_text: Option<String>,
}

struct DocState {
    version: u64,
    text: String,
}

/// The oracle: one rust-analyzer session for one project root.
#[spec(implements = "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-ORACLE-RUST-v0.1#overlays")]
pub struct RustOracle<T: Transport> {
    client: LspClient<T>,
    caps: Capabilities,
    root: PathBuf,
    docs: HashMap<String, DocState>,
    quiescent: bool,
}

impl RustOracle<ChildTransport> {
    /// Resolve, spawn, handshake, and wait for quiescence (bounded —
    /// a slow warm-up degrades, it does not fail). ORACLE-RUST §1–§3.
    pub fn spawn(root: &Path, quiescence_budget: Duration) -> Result<Self, TcgBridgeError> {
        let root = verbatim_free(&root.canonicalize().unwrap_or_else(|_| root.to_path_buf()));
        if !root.join("Cargo.toml").is_file() {
            return Err(TcgBridgeError::WorkspaceUnloadable {
                detail: format!("{} carries no Cargo.toml", root.display()),
            });
        }
        let program = resolve_rust_analyzer(&root)?;
        let transport = ChildTransport::spawn(&program, &root)?;
        let mut client = LspClient::new(transport);
        let caps = client.initialize(&uri_from_path(&root), HANDSHAKE_BUDGET)?;
        let quiescent = client.wait_quiescent(quiescence_budget);
        Ok(Self {
            client,
            caps,
            root,
            docs: HashMap::new(),
            quiescent,
        })
    }
}

impl<T: Transport> RustOracle<T> {
    /// Assemble from parts — the replay-test seam.
    pub fn from_parts(
        client: LspClient<T>,
        caps: Capabilities,
        root: PathBuf,
        quiescent: bool,
    ) -> Self {
        Self {
            client,
            caps,
            root,
            docs: HashMap::new(),
            quiescent,
        }
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.caps
    }

    pub fn quiescent(&self) -> bool {
        self.quiescent
    }

    fn uri_for(&self, rel: &str) -> String {
        uri_from_path(&self.root.join(rel))
    }

    /// The effective text of a file: the inline overlay, the open
    /// document, or the disk state — in that order (TCG-PROTOCOL-RUST
    /// §2 validate semantics).
    fn effective_text(&self, rel: &str, content: Option<String>) -> Result<String, TcgBridgeError> {
        if let Some(text) = content {
            return Ok(text);
        }
        if let Some(doc) = self.docs.get(rel) {
            return Ok(doc.text.clone());
        }
        std::fs::read_to_string(self.root.join(rel)).map_err(|e| TcgBridgeError::Protocol {
            detail: format!("`{rel}` has no overlay and no readable disk state: {e}"),
        })
    }

    /// didOpen v1 / didChange v+1 with a MONOTONIC per-document
    /// version — the LSP-native form of the session-monotonic lesson
    /// (ORACLE-RUST §4).
    fn open_or_update(&mut self, rel: &str, text: String) -> Result<u64, TcgBridgeError> {
        let uri = self.uri_for(rel);
        match self.docs.get_mut(rel) {
            Some(doc) => {
                doc.version += 1;
                doc.text = text.clone();
                let version = doc.version;
                self.client.notify(
                    "textDocument/didChange",
                    serde_json::json!({
                        "textDocument": { "uri": uri, "version": version },
                        "contentChanges": [{ "text": text }],
                    }),
                )?;
                Ok(version)
            }
            None => {
                self.docs.insert(
                    rel.to_string(),
                    DocState {
                        version: 1,
                        text: text.clone(),
                    },
                );
                self.client.notify(
                    "textDocument/didOpen",
                    serde_json::json!({
                        "textDocument": {
                            "uri": uri, "languageId": "rust",
                            "version": 1, "text": text,
                        },
                    }),
                )?;
                Ok(1)
            }
        }
    }

    /// `update {file, content|null}` → set/clear an overlay.
    pub fn update(&mut self, rel: &str, content: Option<String>) -> Result<u64, TcgBridgeError> {
        match content {
            Some(text) => self.open_or_update(rel, text),
            None => {
                if self.docs.remove(rel).is_some() {
                    let uri = self.uri_for(rel);
                    self.client.notify(
                        "textDocument/didClose",
                        serde_json::json!({ "textDocument": { "uri": uri } }),
                    )?;
                }
                Ok(0)
            }
        }
    }

    /// Single-document pull diagnostics over the effective text
    /// (ORACLE-RUST §5: the oracle's answer is r-a's view — the floor
    /// stays the truth).
    #[spec(implements = "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#ops")]
    pub fn validate(
        &mut self,
        rel: &str,
        content: Option<String>,
    ) -> Result<ValidateOutcome, TcgBridgeError> {
        let text = self.effective_text(rel, content)?;
        self.open_or_update(rel, text.clone())?;
        let uri = self.uri_for(rel);
        let result = self.client.request(
            "textDocument/diagnostic",
            serde_json::json!({ "textDocument": { "uri": uri } }),
            OP_BUDGET,
        )?;
        let items = result
            .get("items")
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let diagnostics = items
            .iter()
            .map(|d| {
                let l0 = d
                    .pointer("/range/start/line")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let c = d
                    .pointer("/range/start/character")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let line_text = lines.get(l0 as usize).copied().unwrap_or("");
                let outer = from_lsp(l0, c, line_text, self.caps.position_encoding);
                Diagnostic {
                    code: match d.get("code") {
                        Some(serde_json::Value::String(s)) => s.clone(),
                        Some(other) => other.to_string(),
                        None => String::new(),
                    },
                    category: match d.get("severity").and_then(|s| s.as_u64()) {
                        Some(1) => "error".to_string(),
                        Some(2) => "warning".to_string(),
                        Some(3) => "information".to_string(),
                        _ => "hint".to_string(),
                    },
                    message: d
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or_default()
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                    line: outer.line,
                    character: outer.character,
                }
            })
            .collect();
        Ok(ValidateOutcome {
            diagnostics,
            degraded: !self.quiescent,
        })
    }

    /// Completions at a position over the effective text.
    #[spec(implements = "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#ops")]
    pub fn complete(
        &mut self,
        rel: &str,
        pos: OuterPosition,
        content: Option<String>,
    ) -> Result<Vec<Completion>, TcgBridgeError> {
        let text = self.effective_text(rel, content)?;
        self.open_or_update(rel, text.clone())?;
        let uri = self.uri_for(rel);
        let line_text = text
            .lines()
            .nth(pos.line.saturating_sub(1) as usize)
            .unwrap_or("");
        let (line, character) = to_lsp(pos, line_text, self.caps.position_encoding);
        let result = self.client.request(
            "textDocument/completion",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
            OP_BUDGET,
        )?;
        let items = match &result {
            serde_json::Value::Array(a) => a.clone(),
            other => other
                .get("items")
                .and_then(|i| i.as_array())
                .cloned()
                .unwrap_or_default(),
        };
        Ok(items
            .iter()
            .map(|i| Completion {
                name: i
                    .get("label")
                    .and_then(|l| l.as_str())
                    .unwrap_or_default()
                    .to_string(),
                kind: i.get("kind").and_then(|k| k.as_u64()),
                type_text: i
                    .get("detail")
                    .and_then(|d| d.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        i.pointer("/labelDetails/description")
                            .and_then(|d| d.as_str())
                            .map(str::to_string)
                    }),
            })
            .collect())
    }

    /// Quick info (hover) at a position: type display + docs.
    #[spec(implements = "spec://org.vibevm.ai-native.rust-ai-native-lang/mechanisms/TCG-PROTOCOL-RUST-v0.1#ops")]
    pub fn hover(
        &mut self,
        rel: &str,
        pos: OuterPosition,
        content: Option<String>,
    ) -> Result<(String, String), TcgBridgeError> {
        let text = self.effective_text(rel, content)?;
        self.open_or_update(rel, text.clone())?;
        let uri = self.uri_for(rel);
        let line_text = text
            .lines()
            .nth(pos.line.saturating_sub(1) as usize)
            .unwrap_or("");
        let (line, character) = to_lsp(pos, line_text, self.caps.position_encoding);
        let result = self.client.request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
            }),
            OP_BUDGET,
        )?;
        let raw = result
            .pointer("/contents/value")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        Ok(split_hover(raw))
    }

    /// The graceful LSP exit dance; kill-on-drop remains the backstop
    /// (ORACLE-RUST §7).
    pub fn shutdown(mut self) -> Result<(), TcgBridgeError> {
        let _ = self
            .client
            .request("shutdown", serde_json::Value::Null, Duration::from_secs(5));
        self.client.notify("exit", serde_json::Value::Null)
    }
}

/// Split a hover payload into (type display, documentation): the first
/// fenced code block is the display; everything outside fences is doc
/// prose.
fn split_hover(raw: &str) -> (String, String) {
    let mut display = Vec::new();
    let mut docs = Vec::new();
    let mut in_fence = false;
    let mut saw_fence = false;
    for line in raw.lines() {
        if line.trim_start().starts_with("```") {
            saw_fence = true;
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            // EVERY fence joins the display: rust-analyzer emits the
            // module path and the signature as SEPARATE code blocks
            // (live-chain finding).
            display.push(line);
        } else if !line.trim().is_empty() && line.trim() != "---" {
            docs.push(line);
        }
    }
    if !saw_fence {
        // A plaintext hover IS the type display (no markdown to split).
        return (raw.trim().to_string(), String::new());
    }
    (
        display.join("\n").trim().to_string(),
        docs.join("\n").trim().to_string(),
    )
}

/// `file:///` URI from an absolute path, verbatim-free, forward
/// slashes (the Windows lesson: URIs never see `\\?\`).
fn uri_from_path(path: &Path) -> String {
    let clean = verbatim_free(path);
    let fwd = clean.to_string_lossy().replace('\\', "/");
    if fwd.starts_with('/') {
        format!("file://{fwd}")
    } else {
        format!("file:///{fwd}")
    }
}

#[cfg(test)]
#[path = "oracle/tests.rs"]
mod tests;
