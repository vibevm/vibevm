//! The requiring rows of a locked application package
//! (`PROP-065#app-dependencies`): which locked application packages require
//! it through an installer option or a step, and at which scope, so a
//! project home materialises it from the lockfile on its requirer's behalf.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies");

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::{LockedPackage, Lockfile};
use crate::manifest::{Manifest, RequirementScope, application_requirements};
use crate::package_ref::PackageRef;

/// One locked package that requires the row it is listed on.
///
/// ```
/// use vibe_core::manifest::{LockedRequirer, RequirementScope};
///
/// let requirer: LockedRequirer = toml::from_str(r#"
///     package = "org.example/app@=1.0.0"
///     scope = "build"
/// "#).unwrap();
/// assert_eq!(requirer.package.qualified_name(), "org.example/app");
/// assert_eq!(requirer.scope, RequirementScope::Build);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedRequirer {
    /// The requiring row, pinned to its exact locked version.
    pub package: PackageRef,
    /// `build` or `runtime`, as the requirement declares it.
    pub scope: RequirementScope,
}

impl Lockfile {
    /// Note on every row the locked application packages that require it
    /// (`LockedPackage::required_by`). `manifest_of` answers a row's
    /// manifest; each row it answers is re-read as a requirer (its earlier
    /// entries are dropped everywhere and the current ones added), a row it
    /// does not answer keeps the entries it already names. A requirement no
    /// locked row satisfies is left to the planner, which names it.
    ///
    /// ```
    /// use vibe_core::manifest::{Lockfile, LockedPackage, Manifest};
    ///
    /// fn row(name: &str) -> LockedPackage {
    ///     toml::from_str(&format!(r#"
    ///         kind = "app"
    ///         name = "{name}"
    ///         group = "org.example"
    ///         version = "1.0.0"
    ///         source_url = "file:///registry"
    ///         content_hash = "sha256:abc"
    ///     "#)).unwrap()
    /// }
    /// let app = Manifest::parse_str(r#"
    /// [package]
    /// name = "app"
    /// group = "org.example"
    /// kind = "app"
    /// version = "1.0.0"
    ///
    /// [application]
    /// id = "app"
    ///
    /// [[installer_options]]
    /// id = "main"
    /// name = "Main"
    /// platform = "linux"
    /// platform_name = "Linux"
    /// arch = "x86_64"
    ///
    /// [[installer_options.steps]]
    /// id = "build"
    /// title = "Build"
    /// kind = "copy"
    /// from = "payload"
    /// requires = ["org.example/toolchain@1"]
    /// "#).unwrap();
    /// let mut lock = Lockfile::empty("vibe", "2026-10-31T00:00:00Z");
    /// lock.packages = vec![row("app"), row("toolchain")];
    /// lock.record_app_requirements(|row| (row.name == "app").then_some(&app));
    /// let noted = &lock.packages[1].required_by;
    /// assert_eq!(noted.len(), 1);
    /// assert_eq!(noted[0].package.to_string(), "app:org.example/app@=1.0.0");
    /// assert_eq!(noted[0].scope.as_str(), "build");
    /// assert!(lock.packages[0].required_by.is_empty());
    /// ```
    pub fn record_app_requirements<'m>(
        &mut self,
        manifest_of: impl Fn(&LockedPackage) -> Option<&'m Manifest>,
    ) {
        let mut noted: Vec<(usize, LockedRequirer)> = Vec::new();
        let mut reread: Vec<PackageRef> = Vec::new();
        for requirer in &self.packages {
            let Some(manifest) = manifest_of(requirer) else {
                continue;
            };
            let Ok(pinned) = requirer.as_package_ref() else {
                continue;
            };
            for (requirement, scope) in application_requirements(&manifest.installer_options) {
                let Ok(wanted) = PackageRef::parse(requirement.package()) else {
                    continue;
                };
                let Some(index) = self.packages.iter().position(|row| {
                    wanted.group.as_ref() == Some(&row.group)
                        && wanted.name == row.name
                        && wanted.version.matches(&row.version)
                }) else {
                    continue;
                };
                let entry = LockedRequirer {
                    package: pinned.clone(),
                    scope,
                };
                if !noted.contains(&(index, entry.clone())) {
                    noted.push((index, entry));
                }
            }
            reread.push(pinned);
        }
        for row in &mut self.packages {
            row.required_by.retain(|entry| {
                !reread.iter().any(|requirer| {
                    requirer.group == entry.package.group && requirer.name == entry.package.name
                })
            });
        }
        for (index, entry) in noted {
            let row = &mut self.packages[index];
            if !row.required_by.contains(&entry) {
                row.required_by.push(entry);
            }
        }
    }
}
