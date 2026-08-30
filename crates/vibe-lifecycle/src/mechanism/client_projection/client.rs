//! The closed commissioning-client vocabulary — §6.3's three adapters,
//! and everything that differs between them.
//!
//! §6.3.0.4 fixes "the three projection shapes are exact", and this cell is
//! where those three exact shapes are DATA rather than three copies of a
//! provider. Which client a target gets is decided by the SELECTED
//! PROVIDER's identity (§3.1's four steps over `package:claude-plugin`,
//! `package:codex-plugin`, `package:opencode-plugin`) and never by a config
//! member: a stringly `client = "claude"` would put routing inside the very
//! table the routing law exists to sit above.
//!
//! The asymmetry between Claude/Codex and OpenCode is the architecture's,
//! not a convenience: §6.3 records OpenCode as "the documented different
//! adapter", whose plugin genre is a different npm/TypeScript API, so it
//! projects portable components into its own configuration shape instead of
//! carrying the canonical manifest into a hidden directory.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use crate::mechanism::{
    BUILTIN_CLAUDE_PLUGIN_PROJECTION_PIN, BUILTIN_CODEX_PLUGIN_PROJECTION_PIN,
    BUILTIN_OPENCODE_PLUGIN_PROJECTION_PIN,
};

/// The client-native directory Claude keeps its plugin manifest in.
pub(crate) const CLAUDE_MANIFEST_DIR: &str = ".claude-plugin";

/// The same, for Codex — §6.3.0.4's "current OpenAI Docs plugin contract".
pub(crate) const CODEX_MANIFEST_DIR: &str = ".codex-plugin";

/// The file name a canonical manifest keeps inside that directory.
pub(crate) const PLUGIN_MANIFEST: &str = "plugin.json";

/// Where a client that consumes the canonical MCP declaration expects it.
pub(crate) const DOT_MCP_MANIFEST: &str = ".mcp.json";

/// OpenCode's own configuration fragment.
pub(crate) const OPENCODE_CONFIG: &str = "opencode.json";

/// How one client takes the canonical `mcp.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpShape {
    /// Copied byte-for-byte to `.mcp.json` — §6.3.0.4 for Claude and Codex.
    CanonicalCopy,
    /// Translated into one strict `opencode.json` fragment whose root
    /// carries only `mcp` — §6.3.0.4 and §6.3.0.8.
    OpenCodeFragment,
}

/// One commissioning client, as a projection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionClient {
    Claude,
    Codex,
    OpenCode,
}

impl ProjectionClient {
    /// The client's own word, as plan summaries, evidence and the
    /// fingerprint's domain separation spell it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
        }
    }

    /// The reserved provider identity this client's projection answers
    /// under. §6.3.0.2 keeps provider id and logical name separate — the
    /// row is `#claude-plugin-projection`, servicing `package:claude-plugin`
    /// — so a record names the projector, never the destination.
    pub(crate) const fn pin(self) -> &'static str {
        match self {
            Self::Claude => BUILTIN_CLAUDE_PLUGIN_PROJECTION_PIN,
            Self::Codex => BUILTIN_CODEX_PLUGIN_PROJECTION_PIN,
            Self::OpenCode => BUILTIN_OPENCODE_PLUGIN_PROJECTION_PIN,
        }
    }

    /// The client-native directory the FULL canonical `plugin.json` bytes
    /// are placed in, when this client keeps a plugin manifest at all.
    ///
    /// `None` is OpenCode's documented answer, not an omission: it emits no
    /// manifest, and §6.3.0.4 binds the parsed name/version into its plan,
    /// fingerprint and evidence instead of inventing a metadata file.
    pub(crate) const fn manifest_dir(self) -> Option<&'static str> {
        match self {
            Self::Claude => Some(CLAUDE_MANIFEST_DIR),
            Self::Codex => Some(CODEX_MANIFEST_DIR),
            Self::OpenCode => None,
        }
    }

    /// How this client takes a selected `mcp` component.
    pub(crate) const fn mcp_shape(self) -> McpShape {
        match self {
            Self::Claude | Self::Codex => McpShape::CanonicalCopy,
            Self::OpenCode => McpShape::OpenCodeFragment,
        }
    }
}
