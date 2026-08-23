//! The `[[boot_snippet.fragment]]` table — a snippet's conditional
//! side-contributions (PROP-049 §3).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-049#installed-predicate");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::when::WhenCondition;

/// One `[[boot_snippet.fragment]]` contribution: its own source file and
/// its own activation predicate, riding the parent snippet's category and
/// link (PROP-049 §3).
///
/// ```
/// use vibe_core::manifest::BootSnippetFragment;
///
/// let f: BootSnippetFragment = toml::from_str(
///     "source = \"boot/10a-wal-aside.md\"\nwhen = \"installed:org.vibevm.world/wal\"\n",
/// )
/// .unwrap();
/// assert_eq!(f.source.to_str(), Some("boot/10a-wal-aside.md"));
/// assert_eq!(f.when.unwrap().to_string(), "installed:org.vibevm.world/wal");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootSnippetFragment {
    /// Path to the fragment source inside the package directory.
    pub source: PathBuf,
    /// Optional independent activation predicate for this fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenCondition>,
}
