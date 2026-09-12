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

/// One of the eight installable package kinds.
///
/// Spec: `VIBEVM-SPEC.md` §4.1. This enum is closed; admitting another kind
/// is a spec change the owner makes in the register, not a code change —
/// and once admitted, every exhaustive `match` on this type grows a branch
/// that says what the kind DOES. A wildcard arm hiding a kind is forbidden
/// (PROP-057 `##KIND-CODE-LAW`).
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
    /// Documentation as a package: it documents one or more other
    /// packages — its *subjects*, named in `[[documents]]` — and is
    /// **read, never executed**.
    ///
    /// It carries the card the site and the shelf show without
    /// downloading it (`title`, `abstract`), and it may not declare
    /// `[boot_snippet]`, `[[mcp_server]]` or `[[binary]]`: documentation
    /// never enters a boot lane. `vibe install` refuses it — the reading
    /// path is `vibe cache add`, which warms the package into the
    /// machine store for the local reader, the site and the agent
    /// skill — and a translation of it is another `doc` package, one per
    /// language.
    ///
    /// Admitted by owner amendment 2026-09-12 (VIBEVM-SPEC §4.1);
    /// semantics: PROP-057 `##KIND-DOC-LEAD`.
    Doc,
    /// A standalone product with its own deployment profile in the
    /// build, package and deploy planes (PROP-054).
    ///
    /// The boundary with `tool` is mechanical, not a matter of taste: a
    /// `tool` lives in a consumer project and runs through `vibe bin
    /// exec` by the lock file, while an `app` runs in no consumer
    /// project at all and is built and deployed on its own. Everywhere
    /// the spec draws no line between them — a package is publishable,
    /// resolvable, indexable, may carry skills — an `app` behaves
    /// exactly as a `tool` does.
    ///
    /// Example: `org.vibevm.doc/web`, the documentation site. Admitted
    /// by owner amendment 2026-09-12 (VIBEVM-SPEC §4.1); semantics:
    /// PROP-057 `##KIND-APP-VS-TOOL`.
    App,
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
            PackageKind::Doc => "doc",
            PackageKind::App => "app",
        }
    }

    pub const ALL: [PackageKind; 8] = [
        PackageKind::Flow,
        PackageKind::Feat,
        PackageKind::Stack,
        PackageKind::Tool,
        PackageKind::Mcp,
        PackageKind::Lang,
        PackageKind::Doc,
        PackageKind::App,
    ];

    /// `true` for the one kind that is read instead of installed.
    ///
    /// `vibe install` refuses such a package and names `vibe cache add`
    /// instead (PROP-057 `##KIND-DOC-NOT-INSTALLED`); every other kind
    /// materialises into a consumer's dependency root as always.
    pub const fn is_read_only(self) -> bool {
        matches!(self, PackageKind::Doc)
    }
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
            "doc" => Ok(PackageKind::Doc),
            "app" => Ok(PackageKind::App),
            other => Err(Error::BadPackageKind(other.to_owned())),
        }
    }
}
