//! Package-owned user-application declaration.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::PackageRef;
use crate::manifest::{declarant_path, is_portable_token};

/// The first user-application runtime vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[serde(rename_all = "lowercase")]
pub enum ApplicationRuntime {
    Node,
}

/// Stable release-index locator for platform application distributions.
///
/// Artifact digests deliberately live in the release-owned index rather than
/// in source: publishing a binary must not mutate the commit it describes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#distribution")]
#[serde(deny_unknown_fields)]
pub struct ApplicationDistributionDecl {
    pub repository: String,
    pub release_tag: String,
    pub index_asset: String,
}

/// A published bridge may delegate its application declaration to a mutable,
/// explicitly named Git branch. Each operation resolves that branch to an
/// immutable commit and records the resulting provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#sources")]
#[serde(deny_unknown_fields)]
pub struct ApplicationSourceDecl {
    pub kind: ApplicationSourceKind,
    pub url: String,
    pub tracked_ref: String,
    pub registry_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#sources")]
#[serde(rename_all = "lowercase")]
pub enum ApplicationSourceKind {
    Git,
}

/// Strict package-only `[application]` metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#declaration")]
#[serde(deny_unknown_fields)]
pub struct ApplicationDecl {
    pub id: String,
    pub installer_package: PackageRef,
    pub runtime: ApplicationRuntime,
    pub entry: PathBuf,
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribution: Option<ApplicationDistributionDecl>,
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
        if let Some(distribution) = &self.distribution {
            distribution.validate()?;
        }
        Ok(())
    }
}

impl ApplicationDistributionDecl {
    fn validate(&self) -> Result<(), String> {
        if !valid_repository(&self.repository) {
            return Err("[application.distribution].repository must be owner/name".into());
        }
        if !valid_release_token(&self.release_tag) {
            return Err(
                "[application.distribution].release_tag must be a portable release token".into(),
            );
        }
        declarant_path(PathBuf::from(&self.index_asset).as_path()).map_err(|fault| {
            format!(
                "[application.distribution].index_asset must be one portable file name: {}",
                fault.reason()
            )
        })?;
        if PathBuf::from(&self.index_asset).components().count() != 1 {
            return Err(
                "[application.distribution].index_asset must be one portable file name".into(),
            );
        }
        Ok(())
    }
}

impl ApplicationSourceDecl {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !credential_free_https(&self.url) {
            return Err(
                "[application_source].url must be credential-free HTTPS without query or fragment"
                    .into(),
            );
        }
        if !self.tracked_ref.starts_with("refs/heads/") || !valid_git_ref(&self.tracked_ref) {
            return Err(
                "[application_source].tracked_ref must be an explicit safe refs/heads/... name"
                    .into(),
            );
        }
        declarant_path(&self.registry_path).map_err(|fault| {
            format!(
                "[application_source].registry_path must be portable and relative: {}",
                fault.reason()
            )
        })?;
        Ok(())
    }
}

fn valid_repository(value: &str) -> bool {
    let mut parts = value.split('/');
    matches!((parts.next(), parts.next(), parts.next()), (Some(owner), Some(repo), None)
        if is_slug(owner) && is_slug(repo))
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn valid_release_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn credential_free_https(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    value.trim() == value
        && !authority.is_empty()
        && !authority.contains('@')
        && !value.contains(['?', '#', '\\'])
        && !value.bytes().any(|b| b.is_ascii_whitespace())
}

fn valid_git_ref(value: &str) -> bool {
    !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && !value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || "~^:?*[\\".contains(ch))
}
