//! The consumer view of a root manifest — one code path for both roles.
//!
//! `[project]` and `[package]` are equipotent consumer nodes (the owner's
//! role-equipotence law, PROP-024 ##MANIFEST-ROLES-ARE-EQUIPOTENT): a
//! checkout whose root manifest is a `[package]` installs, generates boot
//! lanes and runs every consumer operation exactly as a `[project]` does.
//! The role is a cosmetic marker for humans and UI. Consumer-side code
//! therefore reads THIS view and never matches on the role.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

use crate::Group;
use crate::manifest::SpecFormat;

use super::document::Manifest;

/// Which section a [`ConsumerNode`] was read from — display only, never a
/// capability switch.
///
/// ```
/// use vibe_core::manifest::NodeRole;
///
/// assert_ne!(NodeRole::Project, NodeRole::Package); // two spellings, one power
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Project,
    Package,
}

/// The role-blind consumer identity and settings of a root manifest.
///
/// ```
/// use vibe_core::manifest::{ConsumerNode, NodeRole};
///
/// let node = ConsumerNode {
///     name: "demo".into(),
///     group: None,
///     spec_format: None,
///     role: NodeRole::Project,
/// };
/// assert_eq!(node.coordinate(), "demo"); // no group — the bare name
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerNode {
    pub name: String,
    pub group: Option<Group>,
    pub spec_format: Option<SpecFormat>,
    /// Cosmetic provenance of this view (`[project]` or `[package]`).
    pub role: NodeRole,
}

impl ConsumerNode {
    /// `group/name` when a group is declared, bare `name` otherwise — the
    /// self coordinate a `spec://` address names to reach the authored tree.
    pub fn coordinate(&self) -> String {
        match &self.group {
            Some(group) => format!("{group}/{}", self.name),
            None => self.name.clone(),
        }
    }
}

impl Manifest {
    /// The consumer view of this manifest's root section, whichever role it
    /// carries. `None` only for a virtual `[workspace]`-only coordinator.
    ///
    /// ```
    /// use vibe_core::manifest::{Manifest, NodeRole};
    ///
    /// let project: Manifest =
    ///     toml::from_str("[project]\nname = \"a\"\nversion = \"0.1.0\"\n").unwrap();
    /// let package: Manifest = toml::from_str(
    ///     "[package]\nname = \"b\"\ngroup = \"org.x\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    /// )
    /// .unwrap();
    /// assert_eq!(project.consumer_node().unwrap().role, NodeRole::Project);
    /// let node = package.consumer_node().unwrap();
    /// assert_eq!(node.role, NodeRole::Package);
    /// assert_eq!(node.coordinate(), "org.x/b");
    /// ```
    pub fn consumer_node(&self) -> Option<ConsumerNode> {
        if let Some(project) = &self.project {
            return Some(ConsumerNode {
                name: project.name.clone(),
                group: project.group.clone(),
                spec_format: project.spec_format,
                role: NodeRole::Project,
            });
        }
        self.package.as_ref().map(|package| ConsumerNode {
            name: package.name.clone(),
            group: Some(package.group.clone()),
            spec_format: package.spec_format,
            role: NodeRole::Package,
        })
    }
}
