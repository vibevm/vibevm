//! Strict application dispatcher context, reply, and durable index models.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#context");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#reply");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const CONTEXT_PROTOCOL: &str = "vibe-application-context/1";
pub const RESULT_PROTOCOL: &str = "vibe-application-result/1";
pub const INDEX_PROTOCOL: &str = "vibe-user-applications/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationOperation {
    Install,
    Update,
    Uninstall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageIdentity {
    pub group: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationIdentity {
    pub id: String,
    pub package: PackageIdentity,
    pub installer_package: PackageIdentity,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationContext {
    pub protocol: String,
    pub operation: ApplicationOperation,
    pub application: ApplicationIdentity,
    pub settings_root: PathBuf,
    pub host_root: PathBuf,
    pub registry_root: Option<PathBuf>,
    pub vibe_executable: PathBuf,
    pub offline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationStatus {
    Ready,
    Undeployed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagementEntry {
    pub runtime: ManagementRuntime,
    pub entry: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagementRuntime {
    Node,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationReply {
    pub protocol: String,
    pub operation: ApplicationOperation,
    pub application_id: String,
    pub status: ApplicationStatus,
    pub host_root: PathBuf,
    pub management: Option<ManagementEntry>,
    pub commands: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationRecord {
    pub application: ApplicationIdentity,
    pub host_root: PathBuf,
    pub management: ManagementEntry,
    pub status: ApplicationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationIndex {
    pub protocol: String,
    pub applications: BTreeMap<String, ApplicationRecord>,
}

impl Default for ApplicationIndex {
    fn default() -> Self {
        Self {
            protocol: INDEX_PROTOCOL.into(),
            applications: BTreeMap::new(),
        }
    }
}
