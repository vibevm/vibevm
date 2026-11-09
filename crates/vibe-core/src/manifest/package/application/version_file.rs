//! `[[application.version_file]]` — the per-project version files a package
//! understands (PROP-075 `#VERSION-FILE-FORMATS`).
//!
//! The set of formats is closed, deliberately: a `.python-version` is a
//! `plain` row and a `package.json` `engines` field is not read at all. A
//! package that needs more waits for a fact amending
//! `PROP-075#VERSION-FILE-FORMATS`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

use serde::{Deserialize, Serialize};
use specmark::spec;

/// One version-file declaration of an application package.
///
/// ```
/// use vibe_core::manifest::VersionFileDecl;
///
/// let plain: VersionFileDecl =
///     toml::from_str("format = \"plain\"\nnames = [\".nvmrc\", \".node-version\"]").unwrap();
/// assert_eq!(plain.names(), [".nvmrc", ".node-version"]);
/// let asdf: VersionFileDecl =
///     toml::from_str("format = \"tool-versions\"\nkey = \"nodejs\"").unwrap();
/// assert_eq!(asdf.names(), [".tool-versions"]);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VersionFileDecl {
    /// A one-line version file named by the package (`.nvmrc`,
    /// `.node-version`, `.python-version`, `.java-version` and the like).
    /// The declaration order of `names` breaks a tie inside one directory.
    Plain { names: Vec<String> },
    /// asdf's `.tool-versions` with this package's key (`nodejs`).
    ToolVersions { key: String },
}

/// The one file name every `tool-versions` row reads.
pub const TOOL_VERSIONS_FILE: &str = ".tool-versions";

impl VersionFileDecl {
    /// The stable `format` spelling of this row.
    pub fn format(&self) -> &'static str {
        match self {
            Self::Plain { .. } => "plain",
            Self::ToolVersions { .. } => "tool-versions",
        }
    }

    /// The file names this row looks for, in search order.
    pub fn names(&self) -> Vec<&str> {
        match self {
            Self::Plain { names } => names.iter().map(String::as_str).collect(),
            Self::ToolVersions { .. } => vec![TOOL_VERSIONS_FILE],
        }
    }

    /// The package's key in a `.tool-versions` file, for a row of that
    /// format.
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Plain { .. } => None,
            Self::ToolVersions { key } => Some(key),
        }
    }
}
