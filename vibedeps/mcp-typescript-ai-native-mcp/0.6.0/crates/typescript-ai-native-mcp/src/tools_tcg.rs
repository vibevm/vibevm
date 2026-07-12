//! The oracle half of the tool set: the four tcg ops + the bench
//! harness over ONE persistent LanguageService oracle shared by all
//! five tools (lazy spawn + init on first use, respawn-once on a
//! crashed session — the serve relay's posture, server-local).
//! Enrichment goes through `typescript_ai_native_tcg::enrich_validate` — the
//! gate's own rules — and the policy reloads per call so a mid-session
//! freeze is honoured immediately. The TS oracle IS the compiler (the
//! LanguageService is tsc's own engine), so no approximation caveat
//! rides these answers.

specmark::scope!("spec://org.vibevm.ai-native/typescript-ai-native-mcp/tools/discipline-mcp-typescript#tcg-tools");

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mcp_core::{Tool, ToolDescriptor, ToolOutput};
use serde_json::{Value, json};
use typescript_ai_native_tcg::{ORACLE_TIMEOUT, Policy, enrich_validate, parse_position};
use typescript_ai_native_tcg_bridge::{OracleTransport, Position, SystemOracle, TcgBridgeError};

use crate::tools_discipline::language_mismatch;

/// The shared session: spawned + init'ed on the first tcg call, kept
/// warm across calls, respawned ONCE per op when the child died.
pub struct TcgSession {
    root: PathBuf,
    oracle: Option<SystemOracle>,
}

impl TcgSession {
    pub fn new(root: &Path) -> Self {
        TcgSession {
            root: root.to_path_buf(),
            oracle: None,
        }
    }

    fn ensure(&mut self) -> Result<&mut SystemOracle, TcgBridgeError> {
        if self.oracle.is_none() {
            let policy = Policy::load(&self.root).map_err(|e| TcgBridgeError::Protocol {
                detail: format!("{e:#}"),
            })?;
            let mut fresh = SystemOracle::spawn(&self.root, ORACLE_TIMEOUT)?;
            fresh.init(
                &self.root,
                policy.config.typescript.cells_dir.as_deref(),
                &policy.config.typescript.seam,
            )?;
            self.oracle = Some(fresh);
        }
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
        op: impl Fn(&mut SystemOracle) -> Result<T, TcgBridgeError>,
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

fn position_param(args: &Value) -> Option<Position> {
    let p = args.get("position")?;
    if let Some(s) = p.as_str() {
        return parse_position(s).ok();
    }
    Some(Position {
        line: p.get("line")?.as_u64()?,
        character: p.get("character")?.as_u64()?,
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
    ToolOutput::failed(e.to_string())
}

fn tcg_validate(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let Some(file) = str_param(args, "file") else {
        return ToolOutput::failed("`tcg_validate` needs `file`");
    };
    let policy = match Policy::load(&session.root) {
        Ok(p) => p,
        Err(e) => return ToolOutput::failed(format!("{e:#}")),
    };
    let content = str_param(args, "content");
    match session.with_oracle(|o| o.validate(&file, content.as_deref())) {
        Ok(raw) => {
            let enriched = enrich_validate(&policy, &file, raw);
            let errors = enriched
                .raw
                .diagnostics
                .iter()
                .filter(|d| d.category == "error")
                .count();
            let new_findings = enriched
                .conform_findings
                .iter()
                .filter(|f| !f.baselined)
                .count();
            let output = pretty(&enriched);
            if errors == 0 && new_findings == 0 {
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
    let pos = position_param(args);
    match session.with_oracle(|o| o.scope(&file, pos)) {
        Ok(result) => ToolOutput::ok(pretty(&result)),
        Err(e) => bridge_failure(&e),
    }
}

fn tcg_complete(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let (Some(file), Some(pos)) = (str_param(args, "file"), position_param(args)) else {
        return ToolOutput::failed("`tcg_complete` needs `file` and `position`");
    };
    let content = str_param(args, "content");
    let prefix = str_param(args, "prefix");
    let max = args.get("max").and_then(Value::as_u64).unwrap_or(50).max(1);
    match session
        .with_oracle(|o| o.complete(&file, pos, content.as_deref(), prefix.as_deref(), max))
    {
        Ok(result) => ToolOutput::ok(pretty(&result)),
        Err(e) => bridge_failure(&e),
    }
}

fn tcg_type(session: &mut TcgSession, args: &Value) -> ToolOutput {
    let (Some(file), Some(pos)) = (str_param(args, "file"), position_param(args)) else {
        return ToolOutput::failed("`tcg_type` needs `file` and `position`");
    };
    let content = str_param(args, "content");
    match session.with_oracle(|o| o.quick_info(&file, pos, content.as_deref())) {
        Ok(result) => ToolOutput::ok(pretty(&result)),
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
        typescript_ai_native_tcg::bench::run_bench(&root, &root.join(corpus), &root.join(report))
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
        "description": "must be `typescript` when given — this server serves one language",
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
            "Check a file (or a hypothetical overlay) BEFORE writing it: the \
             LanguageService's diagnostics (tsc's own engine — agreement by \
             construction) + the conform gate's findings flagged against the frozen \
             ratchet + advice. isError mirrors the one-shot exit contract: an error \
             diagnostic OR a non-baselined finding. = `typescript-ai-native-tcg validate`.",
            json!({"file": file, "content": content, "language": lang}),
            tcg_validate,
        ),
        t(
            "tcg_scope",
            "What is in scope here: symbols, the cell, the seam, branded types. \
             = `typescript-ai-native-tcg scope`.",
            json!({"file": file, "position": position, "language": lang}),
            tcg_scope,
        ),
        t(
            "tcg_complete",
            "Type-valid continuations at a position, with the unsafe-set flag on \
             banned forms. = `typescript-ai-native-tcg complete`.",
            json!({
                "file": file, "position": position, "content": content,
                "prefix": {"type": "string"}, "max": {"type": "integer"},
                "language": lang,
            }),
            tcg_complete,
        ),
        t(
            "tcg_type",
            "The type at a position (quick info): display text + documentation. \
             = `typescript-ai-native-tcg type`.",
            json!({"file": file, "position": position, "content": content, "language": lang}),
            tcg_type,
        ),
        t(
            "tcg_bench",
            "The differential corpus harness: oracle-vs-tsc agreement + latency \
             report. = `typescript-ai-native-tcg bench`. Heavy — expect minutes.",
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
        let mut tools = tcg_tools(Path::new("."));
        let out = tools[0].run(&json!({}));
        assert!(out.is_error);
        assert!(out.report.contains("needs `file`"), "{}", out.report);
        let out = tools[3].run(&json!({"file": "src/index.ts"}));
        assert!(out.is_error);
        assert!(out.report.contains("`position`"), "{}", out.report);
    }
}
