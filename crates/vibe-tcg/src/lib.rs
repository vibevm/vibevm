//! vibe-tcg — the portable tcg tool family (PROP-026): the four
//! `tcg_*` tool descriptors/schemas and their run logic over a narrow
//! host seam, plus the per-language oracle registry.
//!
//! PORTABILITY IS THE DESIGN CONSTRAINT (PROP-026 §3, the owner
//! amendment): this crate never imports vibe-mcp. vibe-mcp mounts the
//! family through one thin adapter; a future standalone tcg MCP server
//! is a new binary mounting the same crate, zero changes here.
//!
//! The family is ALGORITHMIC (PROP-018 §2.3 — the query_package path):
//! no affinity, no relay, no Intent. Requests go lockfile → slot →
//! `tcg-typescript serve` (the PROP-025 dispatch model through the
//! shared `vibe_workspace::bins` cell) and come back discipline-
//! enriched by that relay.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-026#root");

use std::path::Path;

mod registry;
pub use registry::{OracleLink, OracleRegistry, Spawner};

/// The narrow host abstraction (PROP-026 §3): everything the family
/// needs from whoever mounts it. Deliberately tiny — a project root is
/// the whole context; consent policy is fixed by §5 (no prompts).
pub trait TcgHost {
    fn project_root(&self) -> &Path;
}

/// The family's failure surface. Every variant is a recipe, not a dead
/// end (PROP-026 §4).
#[derive(Debug, thiserror::Error)]
pub enum TcgError {
    #[error(
        "language `{given}` is not supported by the tcg tools yet \
         (supported: {supported:?}); the Rust twin arrives as a new \
         language value (PROP-026 s2)"
    )]
    LanguageUnsupported {
        given: String,
        supported: Vec<&'static str>,
    },

    #[error(
        "no installed package declares `{binary}` for language `{language}` — \
         add `\"stack:org.vibevm/typescript-ai-native\" = \"^0.4\"` to \
         [requires.packages] and run `vibe install` (PROP-026 s4)"
    )]
    StackNotInstalled { language: String, binary: String },

    #[error(
        "`{binary}` ({package}) is declared by a non-allow-listed group and \
         is not built; an MCP server never prompts — build it once in a \
         terminal: `vibe bin build {binary} --assume-yes` (PROP-026 s5)"
    )]
    NotBuiltThirdParty { binary: String, package: String },

    #[error("building `{binary}` in its slot failed: {detail}")]
    BuildFailed { binary: String, detail: String },

    #[error(
        "the oracle relay for `{language}` is gone: {detail} — it was \
         respawned once already; run the op one-shot \
         (`vibe bin exec tcg-typescript -- validate ...`) to see stderr \
         (PROP-026 s4)"
    )]
    OracleGone { language: String, detail: String },

    #[error("tcg protocol violation: {detail}")]
    Protocol { detail: String },

    #[error("tool argument error: {detail}")]
    BadArguments { detail: String },
}

/// One tool's static face: name, human description, JSON-schema for the
/// arguments. The mounting server serialises this into its own
/// descriptor shape.
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: serde_json::Value,
}

const LANGUAGES: [&str; 1] = ["typescript"];

fn language_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "enum": LANGUAGES,
        "description": "The language whose oracle answers (the stack must be installed)",
    })
}

fn position_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "description": "1-based line, 0-based character",
        "properties": {
            "line": { "type": "integer", "minimum": 1 },
            "character": { "type": "integer", "minimum": 0 },
        },
        "required": ["line", "character"],
    })
}

/// The four tool faces (PROP-026 §2).
pub fn tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "tcg_validate",
            description: "Type-check a file (optionally with hypothetical content, never \
                          touching disk) through the project's own compiler; returns \
                          compiler diagnostics PLUS the discipline gate's findings \
                          (flagged against the frozen baseline) and advice.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": language_schema(),
                    "file": { "type": "string", "description": "Project-root-relative path" },
                    "content": { "type": "string", "description": "Hypothetical file content (an in-memory overlay); omit to validate the disk state" },
                },
                "required": ["language", "file"],
            }),
        },
        ToolSpec {
            name: "tcg_scope",
            description: "What is in scope at a file/position: symbols with kinds, the \
                          file's cell and seam, and the branded types exported at \
                          reachable seams (heuristic-labelled).",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": language_schema(),
                    "file": { "type": "string" },
                    "position": position_schema(),
                },
                "required": ["language", "file"],
            }),
        },
        ToolSpec {
            name: "tcg_complete",
            description: "Type-valid completions at a position (prefix-filtered; entries \
                          carry type text and an `unsafe` flag for any-typed candidates).",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": language_schema(),
                    "file": { "type": "string" },
                    "position": position_schema(),
                    "content": { "type": "string" },
                    "prefix": { "type": "string" },
                    "max": { "type": "integer", "minimum": 1, "default": 50 },
                },
                "required": ["language", "file", "position"],
            }),
        },
        ToolSpec {
            name: "tcg_type",
            description: "Quick info (type display + documentation) at a position.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": language_schema(),
                    "file": { "type": "string" },
                    "position": position_schema(),
                    "content": { "type": "string" },
                },
                "required": ["language", "file", "position"],
            }),
        },
    ]
}

fn oracle_op(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "tcg_validate" => Some("validate"),
        "tcg_scope" => Some("scope"),
        "tcg_complete" => Some("complete"),
        "tcg_type" => Some("type"),
        _ => None,
    }
}

/// Run one family tool: validate the language, strip it from the
/// params, relay the op to the (lazily spawned) enriching relay, and
/// return its result verbatim — the relay already enriched it
/// (TCG-PROTOCOL §3).
pub fn run_tool(
    name: &str,
    args: &serde_json::Value,
    host: &dyn TcgHost,
    registry: &OracleRegistry,
) -> Result<serde_json::Value, TcgError> {
    let op = oracle_op(name).ok_or_else(|| TcgError::BadArguments {
        detail: format!("`{name}` is not a tcg tool"),
    })?;
    let language = args
        .get("language")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TcgError::BadArguments {
            detail: "`language` is required".to_string(),
        })?;
    if !LANGUAGES.contains(&language) {
        return Err(TcgError::LanguageUnsupported {
            given: language.to_string(),
            supported: LANGUAGES.to_vec(),
        });
    }
    if args.get("file").and_then(|v| v.as_str()).is_none() {
        return Err(TcgError::BadArguments {
            detail: "`file` is required".to_string(),
        });
    }
    let mut params = args.clone();
    if let Some(map) = params.as_object_mut() {
        map.shift_remove("language");
    }
    registry.request(language, host, op, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_faces_carry_language_and_schemas() {
        let specs = tool_specs();
        let names: Vec<_> = specs.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            ["tcg_validate", "tcg_scope", "tcg_complete", "tcg_type"]
        );
        for spec in &specs {
            let required = spec.input_schema["required"]
                .as_array()
                .expect("required");
            assert!(required.iter().any(|r| r == "language"), "{}", spec.name);
            assert!(required.iter().any(|r| r == "file"), "{}", spec.name);
        }
    }

    #[test]
    fn unsupported_language_and_missing_args_are_typed_refusals() {
        struct H(std::path::PathBuf);
        impl TcgHost for H {
            fn project_root(&self) -> &Path {
                &self.0
            }
        }
        let host = H(std::path::PathBuf::from("."));
        let registry = OracleRegistry::default();
        let err = run_tool(
            "tcg_validate",
            &serde_json::json!({"language": "rust", "file": "src/a.rs"}),
            &host,
            &registry,
        )
        .expect_err("unsupported");
        assert!(matches!(err, TcgError::LanguageUnsupported { .. }));
        assert!(err.to_string().contains("typescript"));

        let err = run_tool(
            "tcg_validate",
            &serde_json::json!({"language": "typescript"}),
            &host,
            &registry,
        )
        .expect_err("no file");
        assert!(matches!(err, TcgError::BadArguments { .. }));
    }
}
