//! Package-owned user-application declaration.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::PackageRef;
use crate::manifest::{declarant_path, is_portable_token};

/// The first user-application runtime vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationRuntime {
    Node,
}

/// Strict package-only `[application]` metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDecl {
    pub id: String,
    pub installer_package: PackageRef,
    pub runtime: ApplicationRuntime,
    pub entry: PathBuf,
    pub commands: Vec<String>,
}

impl ApplicationDecl {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err("[application].id must be a portable lowercase token".into());
        }
        if self.installer_package.group.is_none() || !self.installer_package.version.is_exact_pin()
        {
            return Err(
                "[application].installer_package must be fully qualified and exactly pinned".into(),
            );
        }
        declarant_path(&self.entry).map_err(|fault| {
            format!(
                "[application].entry must be a portable provider-relative file path: {}",
                fault.reason()
            )
        })?;
        if self.commands.is_empty() {
            return Err("[application].commands must be nonempty".into());
        }
        let mut seen = BTreeSet::new();
        for command in &self.commands {
            if !is_portable_token(command) {
                return Err(format!(
                    "[application].commands value `{command}` is not a portable lowercase token"
                ));
            }
            if !seen.insert(command) {
                return Err(format!(
                    "[application].commands contains duplicate `{command}`"
                ));
            }
        }
        Ok(())
    }
}
