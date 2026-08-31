//! Stable source, configuration, and record identities for native builds.

use std::path::Path;

use sha2::{Digest, Sha256};
use vibe_core::manifest::{ExtensionConfig, ExtensionHandler};
use vibe_registry::RecipeId;

use crate::ExtensionRegistryRow;

use super::provider::{ProviderFacts, ProviderHome};
use super::{NativeArtifactError, NativePlatform};

const RECORD_DOMAIN: &[u8] = b"vibe-native-build-record-id\0epoch=1\0";
const CONFIG_DOMAIN: &[u8] = b"vibe-native-build-config\0epoch=1\0";
const SOURCE_DOMAIN: &[u8] = b"vibe-native-source-witness\0epoch=1\0";

pub(super) fn record_id(provider: &str, crate_dir: &str, platform: NativePlatform) -> String {
    let mut hash = Frame::new(RECORD_DOMAIN);
    hash.field("provider", provider.as_bytes());
    hash.field("crate_dir", crate_dir.as_bytes());
    hash.field("platform", platform.key().as_bytes());
    hash.finish().hex()
}

/// Bind every declaration sharing one provider/crate build, in candidate
/// order. A later process receives the same retained slice and can therefore
/// reject a changed handler or effective config before trusting the record.
pub(super) fn config_witness(rows: &[&ExtensionRegistryRow]) -> String {
    config_witness_digest(rows).hex()
}

pub(super) fn config_witness_digest(rows: &[&ExtensionRegistryRow]) -> WitnessDigest {
    let mut hash = Frame::new(CONFIG_DOMAIN);
    hash.field("row_count", rows.len().to_string().as_bytes());
    for row in rows {
        hash.field("key", row.key().to_string().as_bytes());
        match &row.declaration().handler {
            ExtensionHandler::Native {
                crate_dir,
                prebuilt,
            } => {
                hash.field(
                    "crate_present",
                    if crate_dir.is_some() { b"1" } else { b"0" },
                );
                if let Some(path) = crate_dir {
                    hash.field("crate", slash(path).as_bytes());
                }
                hash.field(
                    "prebuilt_present",
                    if prebuilt.is_some() { b"1" } else { b"0" },
                );
                if let Some(prebuilt) = prebuilt {
                    hash.field("prebuilt_count", prebuilt.len().to_string().as_bytes());
                    for (platform, path) in prebuilt {
                        hash.field("prebuilt_platform", platform.as_bytes());
                        hash.field("prebuilt_path", slash(path).as_bytes());
                    }
                }
            }
            other => hash.field("unexpected_handler", other.kind().as_bytes()),
        }
        hash_config(&mut hash, row.effective_config());
    }
    hash.finish()
}

pub(super) fn source_witness(provider: &ProviderFacts) -> Result<String, NativeArtifactError> {
    source_witness_digest(provider).map(|digest| digest.hex())
}

pub(super) fn source_witness_digest(
    provider: &ProviderFacts,
) -> Result<WitnessDigest, NativeArtifactError> {
    let content_hash = match provider.home {
        ProviderHome::Dependency => {
            provider
                .content_hash
                .clone()
                .ok_or_else(|| NativeArtifactError::SourceWitness {
                    provider: provider.identity.clone(),
                    reason: "installed dependency has no locked content_hash".to_owned(),
                })
        }
        ProviderHome::Host => {
            vibe_registry::compute_content_hash_with(RecipeId::Tree1, provider.root()).map_err(
                |error| NativeArtifactError::SourceWitness {
                    provider: provider.identity.clone(),
                    reason: error.to_string(),
                },
            )
        }
    }?;
    validate_content_hash(provider, &content_hash)?;
    let mut hash = Frame::new(SOURCE_DOMAIN);
    hash.field("content_hash", content_hash.as_bytes());
    Ok(hash.finish())
}

fn validate_content_hash(
    provider: &ProviderFacts,
    content_hash: &str,
) -> Result<(), NativeArtifactError> {
    let Some((label, digest)) = content_hash.rsplit_once(':') else {
        return Err(NativeArtifactError::SourceWitness {
            provider: provider.identity.clone(),
            reason: "content_hash has no algorithm label".to_owned(),
        });
    };
    if !matches!(label, "sha256" | "sha256-tree/1") {
        return Err(NativeArtifactError::SourceWitness {
            provider: provider.identity.clone(),
            reason: format!(
                "content_hash label `{label}:` is not one of `sha256:` or `sha256-tree/1:`"
            ),
        });
    }
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(NativeArtifactError::SourceWitness {
            provider: provider.identity.clone(),
            reason: "content_hash digest is not exactly 64 lowercase hex characters".to_owned(),
        });
    }
    Ok(())
}

fn hash_config(hash: &mut Frame, config: Option<&ExtensionConfig>) {
    hash.field("config_present", if config.is_some() { b"1" } else { b"0" });
    if let Some(config) = config {
        hash_table(hash, config.as_table());
    }
}

fn hash_table(hash: &mut Frame, table: &toml::Table) {
    let mut keys = table.keys().collect::<Vec<_>>();
    keys.sort_unstable();
    hash.field("table_count", keys.len().to_string().as_bytes());
    for key in keys {
        hash.field("table_key", key.as_bytes());
        hash_value(hash, &table[key]);
    }
}

fn hash_value(hash: &mut Frame, value: &toml::Value) {
    match value {
        toml::Value::String(value) => hash.field("string", value.as_bytes()),
        toml::Value::Integer(value) => hash.field("integer", value.to_string().as_bytes()),
        toml::Value::Float(value) => hash.field("float_bits", &value.to_bits().to_be_bytes()),
        toml::Value::Boolean(value) => {
            hash.field("boolean", if *value { b"true" } else { b"false" });
        }
        toml::Value::Datetime(value) => hash.field("datetime", value.to_string().as_bytes()),
        toml::Value::Array(values) => {
            hash.field("array_count", values.len().to_string().as_bytes());
            for value in values {
                hash_value(hash, value);
            }
        }
        toml::Value::Table(table) => hash_table(hash, table),
    }
}

fn slash(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WitnessDigest([u8; 32]);

impl WitnessDigest {
    pub(super) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(super) fn hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut text = String::with_capacity(64);
        for byte in self.0 {
            text.push(HEX[(byte >> 4) as usize] as char);
            text.push(HEX[(byte & 0x0f) as usize] as char);
        }
        text
    }
}

pub(super) struct Frame(Sha256);

impl Frame {
    pub(super) fn new(domain: &[u8]) -> Self {
        let mut hash = Sha256::new();
        hash.update(domain);
        Self(hash)
    }

    pub(super) fn field(&mut self, label: &str, value: &[u8]) {
        self.0.update((label.len() as u64).to_be_bytes());
        self.0.update(label.as_bytes());
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }

    pub(super) fn finish(self) -> WitnessDigest {
        WitnessDigest(self.0.finalize().into())
    }
}
