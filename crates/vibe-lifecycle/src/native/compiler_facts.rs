//! Structured compiler-native pending facts and their one-shot recorder.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use vibe_core::manifest::{ExtensionHandler, ExtensionKey, MechanismKey};
#[cfg(test)]
use vibe_extension_registry::MechanismRegistryRow;
use vibe_extension_registry::{HostIdentity, MechanismProvider};
use vibe_workspace::extension_world::{
    CompilerNativeFactError, PendingBuildFact, PendingBuildProviderDigest,
    PendingHandlerConfigWitness, PendingPlatformKey, PendingSourceWitness,
};

use crate::ExtensionRegistryRow;

use super::witness::{Frame, WitnessDigest};
use super::{NativeArtifactError, NativePlatform, SelectedBuildProvider};

const PROVIDER_DOMAIN: &[u8] = b"vibe-native-build-provider\0epoch=1\0";

#[derive(Clone, PartialEq, Eq)]
pub(super) struct PendingFactCapture {
    pub(super) order: u32,
    pub(super) key: ExtensionKey,
    pub(super) platform: PendingPlatformKey,
    pub(super) source: PendingSourceWitness,
    pub(super) config: PendingHandlerConfigWitness,
    pub(super) route: MechanismKey,
    pub(super) provider: PendingBuildProviderDigest,
}

enum RecorderState {
    Open {
        facts: BTreeMap<u32, PendingFactCapture>,
    },
    Conflict(u32),
    Taken,
}

pub(super) struct PendingFactRecorder {
    state: Mutex<RecorderState>,
}

impl PendingFactRecorder {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(RecorderState::Open {
                facts: BTreeMap::new(),
            }),
        }
    }

    /// Exact repeat observations coalesce. Semantic drift at the same manager
    /// order is a terminal conflict; duplicate fact vectors are refused by
    /// the later workspace evidence join, not represented in this recorder.
    pub(super) fn record(&self, fact: PendingFactCapture) -> Result<(), CompilerNativeFactError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CompilerNativeFactError::poisoned())?;
        let facts = match &mut *state {
            RecorderState::Open { facts } => facts,
            RecorderState::Conflict(order) => {
                return Err(CompilerNativeFactError::conflict(*order));
            }
            RecorderState::Taken => return Err(CompilerNativeFactError::already_taken()),
        };
        match facts.get(&fact.order) {
            None => {
                facts.insert(fact.order, fact);
                Ok(())
            }
            Some(prior) if prior == &fact => Ok(()),
            Some(_) => {
                let order = fact.order;
                *state = RecorderState::Conflict(order);
                Err(CompilerNativeFactError::conflict(order))
            }
        }
    }

    pub(super) fn take(
        &self,
        pending: &vibe_spec::CompilerPendingSet,
    ) -> Result<Vec<PendingBuildFact>, CompilerNativeFactError> {
        let mut recorded = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| CompilerNativeFactError::poisoned())?;
            match std::mem::replace(&mut *state, RecorderState::Taken) {
                RecorderState::Open { facts } => facts,
                RecorderState::Conflict(order) => {
                    return Err(CompilerNativeFactError::conflict(order));
                }
                RecorderState::Taken => return Err(CompilerNativeFactError::already_taken()),
            }
        };

        let mut facts = Vec::with_capacity(pending.len());
        for reference in pending.iter() {
            let order = reference.order();
            let captured = recorded
                .remove(&order)
                .ok_or_else(|| CompilerNativeFactError::missing(order))?;
            if captured.key != *reference.key() {
                return Err(CompilerNativeFactError::conflict(order));
            }
            let fact = PendingBuildFact::from_pending(
                reference,
                captured.platform,
                captured.source,
                captured.config,
                captured.route,
                captured.provider,
            )
            .map_err(|_| CompilerNativeFactError::construction(order))?;
            facts.push(fact);
        }
        if let Some(order) = recorded.keys().next().copied() {
            return Err(CompilerNativeFactError::extra(order));
        }
        Ok(facts)
    }

    pub(super) fn finish_ready(&self) -> Result<(), CompilerNativeFactError> {
        let recorded = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| CompilerNativeFactError::poisoned())?;
            match std::mem::replace(&mut *state, RecorderState::Taken) {
                RecorderState::Open { facts } => facts,
                RecorderState::Conflict(order) => {
                    return Err(CompilerNativeFactError::conflict(order));
                }
                RecorderState::Taken => return Err(CompilerNativeFactError::already_taken()),
            }
        };
        match recorded.keys().next().copied() {
            Some(order) => Err(CompilerNativeFactError::extra(order)),
            None => Ok(()),
        }
    }

    #[cfg(test)]
    pub(super) fn poison_for_test(&self, action: impl FnOnce()) {
        let _guard = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        action();
    }
}

pub(super) enum CompilerArtifactResolutionError {
    Artifact(NativeArtifactError),
    Missing {
        record: String,
        fact: Box<PendingFactCapture>,
    },
    Fact(String),
}

impl CompilerArtifactResolutionError {
    pub(super) fn into_artifact_error(self) -> NativeArtifactError {
        match self {
            Self::Artifact(error) => error,
            Self::Missing { record, .. } => NativeArtifactError::SourceRecordMissing { record },
            Self::Fact(reason) => NativeArtifactError::MechanismSelection { reason },
        }
    }
}

pub(super) fn pending_source_capture(
    row: &ExtensionRegistryRow,
    order: u32,
    platform: NativePlatform,
    source: WitnessDigest,
    config: WitnessDigest,
    selected: &SelectedBuildProvider<'_>,
) -> Result<PendingFactCapture, String> {
    let provider = provider_digest(selected)?;
    let platform = PendingPlatformKey::new(platform.key())
        .map_err(|_| "the closed platform projection refused".to_owned())?;
    Ok(PendingFactCapture {
        order,
        key: row.key().clone(),
        platform,
        source: PendingSourceWitness::new(*source.as_bytes()),
        config: PendingHandlerConfigWitness::new(*config.as_bytes()),
        route: selected.key.clone(),
        provider: PendingBuildProviderDigest::new(provider),
    })
}

fn provider_digest(selected: &SelectedBuildProvider<'_>) -> Result<[u8; 32], String> {
    let row = selected.row;
    let mut hash = Frame::new(PROVIDER_DOMAIN);
    hash_provider(&mut hash, row.provider());
    hash.field("pin", row.pin().to_string().as_bytes());
    hash.field("key", row.key().to_string().as_bytes());
    hash.field("declaration_id", row.declaration().id.as_bytes());
    hash_handler(&mut hash, row.handler())?;
    hash.field("protocol", &row.protocol().to_le_bytes());
    hash.field("config_schema", path_wire(row.config_schema())?.as_bytes());
    hash.field("freshness", row.declaration().freshness.as_str().as_bytes());
    hash.field("enabled", if row.is_enabled() { b"1" } else { b"0" });
    Ok(*hash.finish().as_bytes())
}

fn hash_provider(hash: &mut Frame, provider: &MechanismProvider) {
    match provider {
        MechanismProvider::Builtin => hash.field("provider_kind", b"builtin"),
        MechanismProvider::Dependency(provider) => {
            hash.field("provider_kind", b"dependency");
            hash.field("provider_group", provider.id.group().as_str().as_bytes());
            hash.field("provider_name", provider.id.name().as_str().as_bytes());
            hash.field("provider_version", provider.version.as_bytes());
            hash.field("provider_package_kind", provider.kind.as_str().as_bytes());
            hash.field(
                "provider_content_hash",
                provider.content_hash.to_string().as_bytes(),
            );
        }
        MechanismProvider::Host(provider) => {
            hash.field("provider_kind", b"host");
            match &provider.identity {
                HostIdentity::UngroupedProject(name) => {
                    hash.field("host_identity_kind", b"ungrouped");
                    hash.field("host_project", name.as_bytes());
                }
                HostIdentity::Coordinate(identity) => {
                    hash.field("host_identity_kind", b"coordinate");
                    hash.field("host_group", identity.group().as_str().as_bytes());
                    hash.field("host_name", identity.name().as_str().as_bytes());
                }
                HostIdentity::VirtualWorkspace => {
                    hash.field("host_identity_kind", b"virtual");
                }
            }
            hash.field("provider_version", provider.version.as_bytes());
            hash.field("provider_kind_present", bool_byte(provider.kind.is_some()));
            if let Some(kind) = provider.kind {
                hash.field("provider_package_kind", kind.as_str().as_bytes());
            }
            hash.field(
                "provider_hash_present",
                bool_byte(provider.content_hash.is_some()),
            );
            if let Some(content_hash) = &provider.content_hash {
                hash.field("provider_content_hash", content_hash.to_string().as_bytes());
            }
        }
    }
}

fn hash_handler(hash: &mut Frame, handler: &ExtensionHandler) -> Result<(), String> {
    hash.field("handler_kind", handler.kind().as_bytes());
    match handler {
        ExtensionHandler::Builtin { name } => hash.field("builtin_name", name.as_bytes()),
        ExtensionHandler::Script { base } => {
            hash.field("script_base", path_wire(base)?.as_bytes());
        }
        ExtensionHandler::Binary { name } => hash.field("binary_name", name.as_bytes()),
        ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } => {
            hash.field("crate_present", bool_byte(crate_dir.is_some()));
            if let Some(path) = crate_dir {
                hash.field("crate_dir", path_wire(path)?.as_bytes());
            }
            hash.field("prebuilt_present", bool_byte(prebuilt.is_some()));
            if let Some(prebuilt) = prebuilt {
                hash.field("prebuilt_count", &(prebuilt.len() as u64).to_le_bytes());
                for (platform, path) in prebuilt {
                    hash.field("prebuilt_platform", platform.as_bytes());
                    hash.field("prebuilt_path", path_wire(path)?.as_bytes());
                }
            }
        }
        ExtensionHandler::Agent { prompt } => hash.field("agent_prompt", prompt.as_bytes()),
    }
    Ok(())
}

fn path_wire(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| "provider path is not UTF-8".to_owned())
}

const fn bool_byte(value: bool) -> &'static [u8] {
    if value { b"1" } else { b"0" }
}

#[cfg(test)]
pub(super) fn provider_digest_for_test(row: &MechanismRegistryRow) -> Result<[u8; 32], String> {
    provider_digest(&SelectedBuildProvider {
        key: row.key().clone(),
        row,
    })
}
