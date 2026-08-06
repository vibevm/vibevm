//! The installable package kinds — a closed vocabulary (VIBEVM-SPEC §4.1).
//!
//! Its own cell because the vocabulary is self-contained: it references no
//! other identity type, and it changes on a different clock from the pkgref
//! grammar around it — by owner amendment to the spec, not by code design.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref");

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// One of the four installable package kinds.
///
/// Spec: `VIBEVM-SPEC.md` §4.1. This enum is closed; adding a fifth kind is a
/// spec change, not a code change.
///
/// ```
/// use vibe_core::PackageKind;
///
/// let k: PackageKind = "feat".parse().unwrap();
/// assert_eq!(k, PackageKind::Feat);
/// assert_eq!(k.as_str(), "feat");
/// assert!("widget".parse::<PackageKind>().is_err()); // closed set
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageKind {
    Flow,
    Feat,
    Stack,
    Tool,
    /// An agent-server package: its primary deliverable is one or more
    /// MCP servers (`[[mcp_server]]`, legal only in this kind),
    /// delivered as PROP-025 binaries and exact-pinned to the package
    /// whose toolchain they serve (VIBEVM-SPEC §4.1, PROP-027).
    Mcp,
    /// Guidance for *writing in* something — a language, a notation, a
    /// format: the idioms, the constraints, the shape authors follow.
    /// The AI-Native language guides are `lang`; so would be a package
    /// explaining how to write GitHub-flavoured markdown.
    ///
    /// Split out of `stack` by owner amendment 2026-08-06 (VIBEVM-SPEC
    /// §4.1): `stack` had been carrying two genres at once — language
    /// guidance and family aggregators that pin other packages — and a
    /// word naming two genres names neither. A package is recognised as
    /// an AI-Native language by its dependency on the discipline core,
    /// never by its group, so a third party can publish one in its own
    /// namespace and be recognised.
    Lang,
}

impl PackageKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            PackageKind::Flow => "flow",
            PackageKind::Feat => "feat",
            PackageKind::Stack => "stack",
            PackageKind::Tool => "tool",
            PackageKind::Mcp => "mcp",
            PackageKind::Lang => "lang",
        }
    }

    pub const ALL: [PackageKind; 6] = [
        PackageKind::Flow,
        PackageKind::Feat,
        PackageKind::Stack,
        PackageKind::Tool,
        PackageKind::Mcp,
        PackageKind::Lang,
    ];
}

impl fmt::Display for PackageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PackageKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "flow" => Ok(PackageKind::Flow),
            "feat" => Ok(PackageKind::Feat),
            "stack" => Ok(PackageKind::Stack),
            "tool" => Ok(PackageKind::Tool),
            "mcp" => Ok(PackageKind::Mcp),
            "lang" => Ok(PackageKind::Lang),
            other => Err(Error::BadPackageKind(other.to_owned())),
        }
    }
}
