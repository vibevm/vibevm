//! The `read_doc` MCP tool — a documentation page, by address, as an
//! agent reads it (PROP-057 §7.3 of the vision, `##OBS-AUDIENCE-AGENT`).
//!
//! An agent already has three ways into this project's map: `query`
//! finds nodes, `explain` opens one node's subgraph, `select` walks the
//! graph — and documentation now travels in that map, so «who explains
//! this rule» is answered by tools that already exist. What none of them
//! can do is hand over the PAGE. That is this tool, and it is the only
//! one the documentation needed.
//!
//! It returns a projection, never HTML: the island is for a reader with
//! a browser, and an agent paying by the token wants Markdown (the text
//! of every cited rule substituted in) or the dialect XML (the addresses
//! kept, so it resolves them itself). Both carry the same `pNN` block
//! numbers the site and the local reader show, so a human quoting `p12`
//! from the web and an agent quoting `p12` from here quote one block
//! (`##READER-NUMBERED-BLOCKS`, R-26).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");

use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_doc::build::Format;
use vibe_doc::citations::SpecSources;

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

/// Fetch one documentation page by its `spec://` address.
///
/// ```
/// use vibe_mcp::tools::{McpTool, ReadDocMcpTool};
/// assert_eq!(ReadDocMcpTool.descriptor().name, "read_doc");
/// assert_eq!(ReadDocMcpTool.descriptor().input_schema["required"][0], "address");
/// ```
#[cell(seam = "McpTool", variant = "read_doc")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]
pub struct ReadDocMcpTool;

impl McpTool for ReadDocMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "read_doc".to_string(),
            description:
                "Read one page of a documentation package by its `spec://` address — the extended documentation of a package (kind `doc`), as opposed to the specifications themselves. Give `address` as `spec://<group>/<name>[@<version>]/<document>`; a `#anchor` is accepted and ignored, because this returns the whole page. `format` is `md` (default — the text of every cited rule substituted in, which is what you want to READ) or `xml` (the citation ADDRESSES kept, which is what you want if you will resolve them yourself). `lang` asks for a translation: one documentation package is one language, so `lang` looks for the adaptation published as `<name>-<lang>` and falls back to the source, telling you in the `lang` field which one you got. Both projections carry the `pNN` block numbers the website and the local reader show, so `spec://…/<document>#p12` means one block everywhere. The package is looked for in this checkout, its in-tree packages, the lock file's slots and the machine store, in that order; warm one with `vibe cache add <coordinate>`. Read-only. Use `explain` to find WHICH page documents a rule, then this to read it."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "`spec://<group>/<name>[@<version>]/<document>` — the page to read. A trailing `#anchor` is ignored."
                    },
                    "format": {
                        "type": "string",
                        "enum": ["md", "xml"],
                        "description": "`md` substitutes each cited rule's current text; `xml` keeps the addresses. Default: `md`."
                    },
                    "lang": {
                        "type": "string",
                        "description": "BCP-47 tag of the language you want. Falls back to the source language and says so."
                    }
                },
                "required": ["address"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let address = args
            .get("address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("`address` must be a string".into()))?;
        let format = match args.get("format").and_then(|v| v.as_str()) {
            None | Some("md") => Format::Md,
            Some("xml") => Format::Xml,
            Some(other) => {
                return Err(ToolError::InvalidArguments(format!(
                    "`format` is `md` or `xml`, not `{other}` — the island (HTML) is for a \
                     reader with a browser, and a page's projections are what an agent reads"
                )));
            }
        };
        let lang = args.get("lang").and_then(|v| v.as_str());

        let sources = world(ctx);
        let page = vibe_doc::agent::read_page(address, format, lang, &sources)
            .map_err(|e| ToolError::NotFound(e.to_string()))?;
        Ok(ToolOutput::ok(json!({
            "address": page.address,
            "format": format.as_str(),
            "lang": page.lang,
            "source": page.source.as_str(),
            "page": page.text,
        })))
    }
}

/// The four sources a documentation address resolves through
/// (`##LOCAL-WARMUP`), composed from what the server context already
/// holds. The library discovers none of them: naming them here is what
/// keeps «which copy did I read» answerable.
fn world(ctx: &ServerContext) -> SpecSources {
    let (group, name) = self_coordinate(&ctx.project_root);
    SpecSources::for_checkout(&ctx.project_root, group.as_deref(), &name)
        .with_store(ctx.store_root.clone())
}

/// The project's own `<group>/<name>`, read as TOML data: the one
/// question is «what coordinate does this directory answer to», and a
/// directory with no manifest answers to none, which is a state rather
/// than a failure.
fn self_coordinate(root: &std::path::Path) -> (Option<String>, String) {
    let Ok(text) = std::fs::read_to_string(root.join("vibe.toml")) else {
        return (None, String::new());
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return (None, String::new());
    };
    let read = |table: &str, key: &str| {
        value
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let group = read("project", "group").or_else(|| read("package", "group"));
    let name = read("project", "name")
        .or_else(|| read("package", "name"))
        .unwrap_or_default();
    (group, name)
}

#[cfg(test)]
mod tests;
