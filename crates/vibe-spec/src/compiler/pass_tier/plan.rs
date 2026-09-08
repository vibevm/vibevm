//! Immutable pass-plan identity and artifact projection.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

use std::collections::BTreeSet;

use vibe_core::manifest::{ExtensionConfig, ExtensionHandler, ExtensionKey, ExtensionPass};
use vibe_extension_registry::HostIdentity;

use super::TransformProvider;
use super::fault::PassTierFault;
use crate::compiler::digest::StableDigest;
use crate::compiler::transform::plan::ProviderComponents;

const CONFIG_DOMAIN: &[u8] = b"vibe-pass-config-v1\0epoch=1\0";
const PLAN_DOMAIN: &[u8] = b"vibe-pass-plan-v1\0epoch=1\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PassPlacement {
    Before(String),
    After(String),
    Replace(String),
    Intrinsic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassEntry {
    ordinal: u32,
    key: ExtensionKey,
    provider: TransformProvider,
    config: Option<ExtensionConfig>,
    handler: ExtensionHandler,
    pass: ExtensionPass,
    placement: PassPlacement,
}

impl PassEntry {
    pub(crate) fn new(
        ordinal: u32,
        key: ExtensionKey,
        provider: TransformProvider,
        config: Option<ExtensionConfig>,
        handler: ExtensionHandler,
        pass: ExtensionPass,
        placement: PassPlacement,
    ) -> Self {
        Self {
            ordinal,
            key,
            provider,
            config,
            handler,
            pass,
            placement,
        }
    }

    pub(crate) const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    pub(crate) fn key(&self) -> &ExtensionKey {
        &self.key
    }

    pub(crate) fn provider(&self) -> &TransformProvider {
        &self.provider
    }

    pub(crate) fn config(&self) -> Option<&ExtensionConfig> {
        self.config.as_ref()
    }

    pub(crate) fn handler(&self) -> &ExtensionHandler {
        &self.handler
    }

    pub(crate) fn declaration(&self) -> &ExtensionPass {
        &self.pass
    }

    pub(crate) fn placement(&self) -> &PassPlacement {
        &self.placement
    }

    fn applies_to(&self, artifact: &str) -> bool {
        self.pass
            .artifact
            .as_deref()
            .is_none_or(|value| value == artifact)
    }
}

/// One owner-scoped immutable pass plan, separate from staged transforms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassPlan {
    entries: Vec<PassEntry>,
    digest: Option<[u8; 32]>,
}

impl PassPlan {
    pub const fn empty() -> Self {
        Self {
            entries: Vec::new(),
            digest: None,
        }
    }

    pub(crate) fn build(entries: Vec<PassEntry>) -> Result<Self, PassTierFault> {
        if entries.is_empty() {
            return Ok(Self::empty());
        }
        let mut keys = BTreeSet::new();
        for entry in &entries {
            if !keys.insert(entry.key.clone()) {
                return Err(PassTierFault::DuplicateKey {
                    key: entry.key.clone(),
                });
            }
        }
        let digest = Some(plan_digest(&entries));
        Ok(Self { entries, digest })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn digest_hex(&self) -> Option<String> {
        self.digest.map(lower_hex)
    }

    pub(crate) fn entries(&self) -> &[PassEntry] {
        &self.entries
    }

    pub(crate) fn for_artifact(&self, artifact: &str) -> Self {
        let entries = self
            .entries
            .iter()
            .filter(|entry| entry.applies_to(artifact))
            .cloned()
            .collect::<Vec<_>>();
        Self::build(entries).expect("a filtered valid pass plan remains valid")
    }
}

fn plan_digest(entries: &[PassEntry]) -> [u8; 32] {
    let mut digest = StableDigest::new(PLAN_DOMAIN);
    digest.usize(entries.len());
    for entry in entries {
        digest.u32(entry.ordinal);
        digest.field(entry.key.as_str().as_bytes());
        frame_provider(&mut digest, &entry.provider);
        frame_handler(&mut digest, &entry.handler);
        frame_config(&mut digest, entry.config.as_ref());
        frame_pass(&mut digest, &entry.pass, &entry.placement);
    }
    digest.finish()
}

fn frame_provider(digest: &mut StableDigest, provider: &TransformProvider) {
    match provider.components() {
        ProviderComponents::Dependency {
            id,
            version,
            kind,
            content_hash,
        } => {
            digest.byte(0);
            digest.field(id.group().as_str().as_bytes());
            digest.field(id.name().as_str().as_bytes());
            digest.field(version.as_bytes());
            digest.field(kind.as_str().as_bytes());
            digest.field(content_hash.as_str().as_bytes());
        }
        ProviderComponents::Host {
            identity,
            version,
            kind,
            content_hash,
        } => {
            digest.byte(1);
            match identity {
                HostIdentity::UngroupedProject(name) => {
                    digest.byte(0);
                    digest.field(name.as_bytes());
                }
                HostIdentity::Coordinate(id) => {
                    digest.byte(1);
                    digest.field(id.group().as_str().as_bytes());
                    digest.field(id.name().as_str().as_bytes());
                }
                HostIdentity::VirtualWorkspace => digest.byte(2),
            }
            digest.field(version.as_bytes());
            frame_optional(digest, kind.map(|value| value.as_str().as_bytes()));
            frame_optional(digest, content_hash.map(|value| value.as_str().as_bytes()));
        }
    }
}

fn frame_handler(digest: &mut StableDigest, handler: &ExtensionHandler) {
    match handler {
        ExtensionHandler::Builtin { name } => {
            digest.byte(0);
            digest.field(name.as_bytes());
        }
        ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } => {
            digest.byte(1);
            let crate_path = crate_dir.as_ref().map(|path| slash(path));
            frame_optional(digest, crate_path.as_deref().map(str::as_bytes));
            match prebuilt {
                Some(rows) => {
                    digest.byte(1);
                    digest.usize(rows.len());
                    for (platform, path) in rows {
                        digest.field(platform.as_bytes());
                        digest.field(slash(path).as_bytes());
                    }
                }
                None => digest.byte(0),
            }
        }
        other => {
            digest.byte(255);
            digest.field(other.kind().as_bytes());
        }
    }
}

fn frame_config(digest: &mut StableDigest, config: Option<&ExtensionConfig>) {
    match config {
        None => digest.byte(0),
        Some(config) => {
            digest.byte(1);
            let mut config_digest = StableDigest::new(CONFIG_DOMAIN);
            frame_table(&mut config_digest, config.as_table());
            digest.field(&config_digest.finish());
        }
    }
}

fn frame_pass(digest: &mut StableDigest, pass: &ExtensionPass, placement: &PassPlacement) {
    digest.byte(match pass.kind {
        vibe_core::manifest::ExtensionPassKind::Transform => 0,
        vibe_core::manifest::ExtensionPassKind::Lowering => 1,
        vibe_core::manifest::ExtensionPassKind::Frontend => 2,
        vibe_core::manifest::ExtensionPassKind::Backend => 3,
    });
    frame_level(digest, pass.level);
    frame_level(digest, pass.from);
    frame_level(digest, pass.to);
    match placement {
        PassPlacement::Before(value) => frame_tagged(digest, 0, value),
        PassPlacement::After(value) => frame_tagged(digest, 1, value),
        PassPlacement::Replace(value) => frame_tagged(digest, 2, value),
        PassPlacement::Intrinsic => digest.byte(3),
    }
    match &pass.formats {
        Some(formats) => {
            digest.byte(1);
            let mut canonical = formats.iter().map(String::as_str).collect::<Vec<_>>();
            canonical.sort_unstable();
            canonical.dedup();
            digest.usize(canonical.len());
            for format in canonical {
                digest.field(format.as_bytes());
            }
        }
        None => digest.byte(0),
    }
    frame_optional(digest, pass.artifact.as_deref().map(str::as_bytes));
}

fn frame_level(digest: &mut StableDigest, level: Option<vibe_core::manifest::ExtensionIrLevel>) {
    match level {
        None => digest.byte(0),
        Some(level) => {
            digest.byte(1);
            digest.byte(match level {
                vibe_core::manifest::ExtensionIrLevel::Source => 0,
                vibe_core::manifest::ExtensionIrLevel::Document => 1,
                vibe_core::manifest::ExtensionIrLevel::Closure => 2,
                vibe_core::manifest::ExtensionIrLevel::Lane => 3,
                vibe_core::manifest::ExtensionIrLevel::Emitted => 4,
            });
        }
    }
}

fn frame_table(digest: &mut StableDigest, table: &toml::Table) {
    digest.usize(table.len());
    let mut entries = table.iter().collect::<Vec<_>>();
    entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    for (key, value) in entries {
        digest.field(key.as_bytes());
        frame_value(digest, value);
    }
}

fn frame_value(digest: &mut StableDigest, value: &toml::Value) {
    match value {
        toml::Value::String(value) => frame_tagged(digest, 0, value),
        toml::Value::Integer(value) => {
            digest.byte(1);
            digest.u64(*value as u64);
        }
        toml::Value::Float(value) => {
            digest.byte(2);
            digest.u64(if value.is_nan() {
                f64::NAN.to_bits()
            } else {
                value.to_bits()
            });
        }
        toml::Value::Boolean(value) => {
            digest.byte(3);
            digest.byte(u8::from(*value));
        }
        toml::Value::Datetime(value) => {
            digest.byte(4);
            match value.date {
                Some(date) => {
                    digest.byte(1);
                    digest.u32(u32::from(date.year));
                    digest.byte(date.month);
                    digest.byte(date.day);
                }
                None => digest.byte(0),
            }
            match value.time {
                Some(time) => {
                    digest.byte(1);
                    digest.byte(time.hour);
                    digest.byte(time.minute);
                    digest.byte(time.second);
                    digest.u32(time.nanosecond);
                }
                None => digest.byte(0),
            }
            match value.offset {
                Some(toml::value::Offset::Z) => digest.byte(1),
                Some(toml::value::Offset::Custom { minutes }) => {
                    digest.byte(2);
                    digest.u32(i32::from(minutes) as u32);
                }
                None => digest.byte(0),
            }
        }
        toml::Value::Array(values) => {
            digest.byte(5);
            digest.usize(values.len());
            for value in values {
                frame_value(digest, value);
            }
        }
        toml::Value::Table(table) => {
            digest.byte(6);
            frame_table(digest, table);
        }
    }
}

fn frame_optional(digest: &mut StableDigest, value: Option<&[u8]>) {
    match value {
        Some(value) => {
            digest.byte(1);
            digest.field(value);
        }
        None => digest.byte(0),
    }
}

fn frame_tagged(digest: &mut StableDigest, tag: u8, value: &str) {
    digest.byte(tag);
    digest.field(value.as_bytes());
}

fn slash(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn lower_hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0x0f) as usize] as char);
    }
    text
}
