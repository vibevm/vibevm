//! Forward weak dependencies — the `[recommends]` and `[suggests]`
//! package sections (the two forward levels of the libsolv / RPM
//! weak-dependency model; PROP-003 §2.3.3). The reverse levels
//! (`[supplements]` / `[enhances]`) need a reverse index and are held in
//! the far backlog (PROP-017 §8); they will join this module when they
//! land.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability");

use serde::{Deserialize, Serialize};

use crate::package_ref::PackageRef;

/// `[recommends]` — soft forward dependencies: packages this one prefers
/// to install alongside itself, **best-effort**. A recommended package
/// that cannot be satisfied is silently skipped, never a solve failure
/// (the libsolv / RPM `Recommends` level — see PROP-003 §2.3.3).
///
/// ```
/// use vibe_core::manifest::Recommends;
///
/// let r: Recommends = toml::from_str(r#"packages = ["flow:atomic-commits"]"#).unwrap();
/// assert_eq!(r.packages.len(), 1);
/// assert!(!r.is_empty());
/// assert!(Recommends::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recommends {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
}

impl Recommends {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// `[suggests]` — forward hints: packages that might be useful but are
/// **never auto-installed**; surfaced to the user only (the libsolv / RPM
/// `Suggests` level). The solver ignores them entirely.
///
/// ```
/// use vibe_core::manifest::Suggests;
///
/// let s: Suggests = toml::from_str(r#"packages = ["flow:sync-from-code"]"#).unwrap();
/// assert!(!s.is_empty());
/// assert!(Suggests::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suggests {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageRef>,
}

impl Suggests {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}
