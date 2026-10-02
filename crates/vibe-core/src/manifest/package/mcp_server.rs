//! `[[mcp_server]]` — the MCP servers an `mcp`-kind package declares
//! (PROP-027; VIBEVM-SPEC §4.1). Declaration only at this layer:
//! registration into agent configs lives in `vibe-mcp`'s install
//! family; `vibe-core` gives the manifest shape and the structural
//! validation `vibe check` runs. Unlike `[[skill]]` and `[[binary]]`
//! this table is NOT any-kind: it is legal only in `mcp`-kind
//! packages — the kind IS the taxonomy.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest");

use serde::{Deserialize, Serialize};

/// The closed set of substitution variables a server's `args` may
/// carry, resolved by `vibe mcp install` at registration time
/// (PROP-027): `{project_root}` — the absolute, verbatim-free root of
/// the consuming project.
pub const MCP_ARG_VARS: &[&str] = &["{project_root}"];

/// `[[mcp_server]]` — one agent-facing MCP server (PROP-027).
///
/// `name` is the agent-visible server name (what an MCP host shows as
/// the tool namespace). A local stdio server names a `[[binary]]` in this
/// manifest; a remote Streamable HTTP server declares its HTTPS `url` and
/// `transport = "streamable-http"` instead.
///
/// ```
/// use vibe_core::manifest::McpServerDecl;
///
/// let s: McpServerDecl = toml::from_str(r#"
///     name = "rust-ai-native"
///     binary = "rust-ai-native-mcp"
///     description = "AI-Native Rust discipline + type oracle over MCP"
///     args = ["--path", "{project_root}"]
/// "#).unwrap();
/// assert_eq!(s.name, "rust-ai-native");
/// assert_eq!(s.binary.as_deref(), Some("rust-ai-native-mcp"));
/// assert!(s.unknown_arg_vars().is_empty());
///
/// // `description` and `args` are optional.
/// let bare: McpServerDecl = toml::from_str(
///     "name = \"x\"\nbinary = \"x-mcp\"",
/// ).unwrap();
/// assert!(bare.args.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpServerDecl {
    /// The agent-visible server name; unique within the package.
    pub name: String,
    /// The `[[binary]]` (by `name`) that serves this entry over stdio.
    /// Mutually exclusive with `url`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    /// Remote Streamable HTTP endpoint. Mutually exclusive with `binary`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Explicit transport for remote servers. Local binary declarations
    /// remain stdio by default for backwards compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    /// Optional human description, surfaced by `vibe mcp status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Launch arguments; `{…}` tokens must come from [`MCP_ARG_VARS`]
    /// and are substituted at registration time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

impl McpServerDecl {
    /// Validate a published remote endpoint without normalizing its original
    /// declaration. An agent receives the URL string exactly as authored.
    pub(crate) fn valid_remote_url(&self) -> bool {
        let Some(raw) = self.url.as_deref() else {
            return false;
        };
        if raw.chars().any(char::is_whitespace)
            || raw.chars().any(char::is_control)
            || raw.contains('\\')
        {
            return false;
        }
        let Ok(parsed) = url::Url::parse(raw) else {
            return false;
        };
        parsed.scheme() == "https"
            && parsed.host().is_some()
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.fragment().is_none()
    }

    /// Substitution tokens (`{…}`) in `args` that are NOT in the closed
    /// set — the offenders `Manifest::validate` refuses. A lone `{`
    /// without a closing brace is treated as literal text, not a token.
    ///
    /// ```
    /// use vibe_core::manifest::McpServerDecl;
    ///
    /// let s: McpServerDecl = toml::from_str(
    ///     "name = \"x\"\nbinary = \"b\"\nargs = [\"{project_root}\", \"{secret}\"]",
    /// ).unwrap();
    /// assert_eq!(s.unknown_arg_vars(), vec!["{secret}".to_string()]);
    /// ```
    pub fn unknown_arg_vars(&self) -> Vec<String> {
        let mut out = Vec::new();
        for arg in &self.args {
            let mut rest = arg.as_str();
            while let Some(open) = rest.find('{') {
                let Some(close_rel) = rest[open..].find('}') else {
                    break;
                };
                let token = &rest[open..=open + close_rel];
                if !MCP_ARG_VARS.contains(&token) && !out.iter().any(|t| t == token) {
                    out.push(token.to_string());
                }
                rest = &rest[open + close_rel + 1..];
            }
        }
        out
    }
}
