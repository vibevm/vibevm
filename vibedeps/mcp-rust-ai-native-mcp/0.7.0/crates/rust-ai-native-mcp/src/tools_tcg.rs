//! The oracle half of the tool set: the four tcg ops + the bench
//! harness, over ONE persistent rust-analyzer session shared by all
//! five tools (lazy spawn on first use, respawn-once on a crashed
//! session — the posture the serve relay and the old host registry
//! carried, now server-local). Enrichment goes through the SAME lib
//! fns the `rust-ai-native-tcg` one-shots call (`rust_ai_native_tcg::enrich_validate`
//! over the gate's own rules) — one engine, one truth, and the policy
//! reloads per call so a mid-session freeze is honoured immediately.

specmark::scope!("spec://org.vibevm.ai-native/rust-ai-native-mcp/tools/discipline-mcp-rust#tcg-tools");

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mcp_core::{Tool, ToolDescriptor, ToolOutput};
use rust_ai_native_tcg::{
    Policy, ScopeAnswer, derive_crate_module, detect_newtypes, enrich_validate,
    finalise_completions, parse_position, seam_file_for, spawn_oracle, validate_exit_code,
};
use rust_ai_native_tcg_bridge::client::ChildTransport;
use rust_ai_native_tcg_bridge::position::OuterPosition;
use rust_ai_native_tcg_bridge::{RustOracle, TcgBridgeError};
use serde_json::{Value, json};

use crate::tools_discipline::language_mismatch;

type Oracle = RustOracle<ChildTransport>;

/// The shared session: spawned on the first tcg call, kept warm across
/// calls, respawned ONCE per op when the child died under us.
pub struct TcgSession {
    root: PathBuf,
    oracle: Option<Oracle>,
}

impl TcgSession {
    pub fn new(root: &Path) -> Self {
        TcgSession {
            root: root.to_path_buf(),
            oracle: None,
        }
    }

    fn ensure(&mut self) -> Result<&mut Oracle, TcgBridgeError> {
        if self.oracle.is_none() {
            let fresh = spawn_oracle(&self.root).map_err(|e| TcgBridgeError::Protocol {
                detail: format!("{e:#}"),
            })?;
            self.oracle = Some(fresh);
        }
        // The line above guarantees presence; expressed without unwrap
        // per the §6 ban.
        match self.oracle.as_mut() {
            Some(o) => Ok(o),
            None => Err(TcgBridgeError::Protocol {
                detail: "oracle slot empty after ensure — unreachable by construction".into(),
            }),
        }
    }

    /// Run one op with the respawn-once law: a crashed session is
    /// dropped, respawned, and the op retried exactly once; a second
    /// crash surfaces.
    fn with_oracle<T>(
        &mut self,
        op: impl Fn(&mut Oracle) -> Result<T, TcgBridgeError>,
    ) -> Result<T, TcgBridgeError> {
        let first = {
            let oracle = self.ensure()?;
            op(oracle)
        };
        match first {
            Err(TcgBridgeError::OracleCrashed { .. }) => {
                self.oracle = None;
                let oracle = self.ensure()?;
                op(oracle)
            }
            other => other,
        }
    }
}

fn str_param(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn position_param(args: &Value) -> Option<OuterPosition> {
    let p = args.get("position")?;
    if let Some(s) = p.as_str() {
        return parse_position(s).ok();
    }
    Some(OuterPosition {
        line: p.get("line")?.as_u64()? as u32,
        character: p.get("character")?.as_u64()? as u32,
    })
}

fn pretty(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("(unrenderable result: {e})"))
}

/// One tcg tool: descriptor data + a handler over the shared session.
struct TcgTool {
    name: &'static str,
    description: &'static str,
    properties: Value,
    session: Rc<RefCell<TcgSession>>,
    handler: fn(&mut TcgSession, &Value) -> ToolOutput,
}

impl Tool for TcgTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: self.name.to_string(),
            description: self.description.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": self.properties,
                "additionalProperties": false,
            }),
        }
    }

    fn run(&mut self, args: &Value) -> ToolOutput {
        if let Some(refusal) = language_mismatch(args) {
            return refusal;
        }
        let mut session = self.session.borrow_mut();
        (self.handler)(&mut session, args)
    }
}

fn bridge_failure(e: &TcgBridgeError) -> ToolOutput {
    ToolOutput::failed(format!("{} — {e}", e.wire_kind()))
}

fn tcg_validate(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let Some(file) = str_param(args, "file") else {
        return ToolOutput::failed("`tcg_validate` needs `file`");
    };
    let policy = match Policy::load(&session.root) {
        Ok(p) => p,
        Err(e) => return ToolOutput::failed(format!("{e:#}")),
    };
    let text = match str_param(args, "content") {
        Some(t) => t,
        None => match std::fs::read_to_string(session.root.join(&file)) {
            Ok(t) => t,
            Err(e) => {
                return ToolOutput::failed(format!("`{file}`: no content and no disk state: {e}"));
            }
        },
    };
    match session.with_oracle(|o| o.validate(&file, Some(text.clone()))) {
        Ok(raw) => {
            let enriched = enrich_validate(&policy, &file, &text, raw);
            let output = pretty(&enriched);
            if validate_exit_code(&enriched) == 0 {
                ToolOutput::ok(output)
            } else {
                ToolOutput::failed(output)
            }
        }
        Err(e) => bridge_failure(&e),
    }
}

fn tcg_scope(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let Some(file) = str_param(args, "file") else {
        return ToolOutput::failed("`tcg_scope` needs `file`");
    };
    let policy = match Policy::load(&session.root) {
        Ok(p) => p,
        Err(e) => return ToolOutput::failed(format!("{e:#}")),
    };
    let content = str_param(args, "content");
    let pos = position_param(args).unwrap_or(OuterPosition {
        line: 1,
        character: 0,
    });
    match session.with_oracle(|o| o.complete(&file, pos, content.clone())) {
        Ok(entries) => {
            let symbols = finalise_completions(entries, &file, None, 200);
            let (_crate_name, module) = derive_crate_module(&policy.config.roots, &file);
            let root = session.root.clone();
            let text = content
                .clone()
                .or_else(|| std::fs::read_to_string(root.join(&file)).ok());
            let seam = seam_file_for(&root, &file);
            let mut branded = text
                .as_deref()
                .map(|t| detect_newtypes(t, &file))
                .unwrap_or_default();
            if seam != file
                && let Ok(seam_text) = std::fs::read_to_string(root.join(&seam))
            {
                branded.extend(detect_newtypes(&seam_text, &seam));
            }
            ToolOutput::ok(pretty(&ScopeAnswer {
                symbols,
                cell: module,
                seam_file: seam,
                branded,
            }))
        }
        Err(e) => bridge_failure(&e),
    }
}

fn tcg_complete(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let (Some(file), Some(pos)) = (str_param(args, "file"), position_param(args)) else {
        return ToolOutput::failed("`tcg_complete` needs `file` and `position`");
    };
    let content = str_param(args, "content");
    let prefix = str_param(args, "prefix");
    let max = args.get("max").and_then(Value::as_u64).unwrap_or(50).max(1) as usize;
    match session.with_oracle(|o| o.complete(&file, pos, content.clone())) {
        Ok(entries) => ToolOutput::ok(pretty(&json!({
            "entries": finalise_completions(entries, &file, prefix.as_deref(), max),
        }))),
        Err(e) => bridge_failure(&e),
    }
}

fn tcg_type(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let (Some(file), Some(pos)) = (str_param(args, "file"), position_param(args)) else {
        return ToolOutput::failed("`tcg_type` needs `file` and `position`");
    };
    let content = str_param(args, "content");
    match session.with_oracle(|o| o.hover(&file, pos, content.clone())) {
        Ok((display, documentation)) => ToolOutput::ok(pretty(&json!({
            "display": display, "documentation": documentation,
        }))),
        Err(e) => bridge_failure(&e),
    }
}

fn tcg_bench(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let (Some(corpus), Some(report)) = (str_param(args, "corpus"), str_param(args, "report"))
    else {
        return ToolOutput::failed("`tcg_bench` needs `corpus` and `report`");
    };
    let root = session.root.clone();
    match mcp_core::capture(|| {
        rust_ai_native_tcg::bench::run_bench(&root.join(corpus), &root.join(report), &root)
    }) {
        Ok((Ok(0), said)) => ToolOutput::ok(said),
        Ok((Ok(_nonzero), said)) => ToolOutput::failed(said),
        Ok((Err(e), said)) => ToolOutput::failed(format!("{said}{e:#}")),
        Err(e) => ToolOutput::failed(e.to_string()),
    }
}

/// Build the five tcg tools over one shared session for `root`.
pub fn tcg_tools(root: &Path) -> Vec<Box<dyn Tool>> {
    let session = Rc::new(RefCell::new(TcgSession::new(root)));
    let lang = json!({
        "type": "string",
        "description": "must be `rust` when given — this server serves one language",
    });
    let file = json!({"type": "string", "description": "repo-relative file path (required)"});
    let content = json!({
        "type": "string",
        "description": "hypothetical file content (an overlay; disk is read when absent)",
    });
    let position = json!({
        "description": "`L:C` string (1-based line, 0-based character) or {line, character}",
    });
    let t = |name, description, properties, handler| -> Box<dyn Tool> {
        Box::new(TcgTool {
            name,
            description,
            properties,
            session: session.clone(),
            handler,
        })
    };
    vec![
        t(
            "tcg_validate",
            "Check a file (or a hypothetical overlay) BEFORE writing it: rust-analyzer \
             diagnostics + the conform gate's own findings flagged against the frozen \
             ratchet + REQ-citing advice. isError mirrors the one-shot exit contract: \
             an error diagnostic OR a non-baselined finding. = `rust-ai-native-tcg validate`.",
            json!({"file": file, "content": content, "language": lang}),
            tcg_validate,
        ),
        t(
            "tcg_scope",
            "What is in scope here: completion-sweep symbols, the module cell, the \
             enclosing seam file, and detected newtype brands (heuristic, labelled). \
             = `rust-ai-native-tcg scope`.",
            json!({"file": file, "position": position, "content": content, "language": lang}),
            tcg_scope,
        ),
        t(
            "tcg_complete",
            "Type-valid continuations at a position, with the §6-ban flag on unsafe \
             forms (unwrap/expect in domain code). = `rust-ai-native-tcg complete`.",
            json!({
                "file": file, "position": position, "content": content,
                "prefix": {"type": "string"}, "max": {"type": "integer"},
                "language": lang,
            }),
            tcg_complete,
        ),
        t(
            "tcg_type",
            "The type at a position (hover): display text + documentation. \
             = `rust-ai-native-tcg type`.",
            json!({"file": file, "position": position, "content": content, "language": lang}),
            tcg_type,
        ),
        t(
            "tcg_bench",
            "The differential corpus harness: oracle-vs-cargo agreement + latency \
             report. = `rust-ai-native-tcg bench`. Heavy: materialises a scratch workspace and \
             runs cargo per case — expect minutes.",
            json!({
                "corpus": {"type": "string", "description": "root-relative corpus dir (required)"},
                "report": {"type": "string", "description": "root-relative report path (required)"},
                "language": lang,
            }),
            tcg_bench,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_tools_with_schemas_and_shared_session() {
        let tools = tcg_tools(Path::new("."));
        assert_eq!(tools.len(), 5);
        let names: Vec<String> = tools.iter().map(|t| t.descriptor().name).collect();
        assert_eq!(
            names,
            [
                "tcg_validate",
                "tcg_scope",
                "tcg_complete",
                "tcg_type",
                "tcg_bench"
            ]
        );
    }

    #[test]
    fn missing_required_params_refuse_without_an_oracle() {
        // No rust-analyzer is spawned for a malformed call — the
        // refusal comes before the session.
        let mut tools = tcg_tools(Path::new("."));
        let out = tools[0].run(&json!({}));
        assert!(out.is_error);
        assert!(out.report.contains("needs `file`"), "{}", out.report);
        let out = tools[2].run(&json!({"file": "src/lib.rs"}));
        assert!(out.is_error);
        assert!(out.report.contains("`position`"), "{}", out.report);
    }
}
