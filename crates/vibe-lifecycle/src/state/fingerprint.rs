//! Epoch/domain-separated deterministic execution fingerprints.

mod declaration;
pub(crate) mod inputs;
#[cfg(test)]
pub(crate) mod legacy;
pub(crate) mod stable;

use std::path::{Component, Path};

use serde_json::Value;
use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::{ExtensionHandler, ExtensionKey};
use vibe_wire::generated::lifecycle::e1::context::Context;
use walkdir::DirEntry;

use crate::HandlerExecution;
use crate::agent::PreparedAgent;
use crate::{ExtensionProvider, ExtensionRegistryRow};

pub(crate) use inputs::{
    ManifestOutcome, PreparedFingerprint, PreparedInputManifest, prepare_execution_with,
    prepare_handler_execution_with,
};
pub(crate) use stable::InputRefusal;

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub enum FingerprintError {
    #[error(
        "extension `{key}` has invalid inputs pattern `{pattern}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT; \
          fix: use a project-root-relative forward-slash glob)"
    )]
    InvalidInput {
        key: ExtensionKey,
        pattern: String,
        reason: String,
    },
    #[error(
        "extension `{key}` cannot fingerprint `{path}`: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT; \
          fix: make the declared input readable or correct its inputs glob)"
    )]
    Read {
        key: ExtensionKey,
        path: String,
        source: std::io::Error,
    },
    #[error(
        "extension `{key}` cannot encode canonical fingerprint material: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT; \
          fix: report this generated-envelope serialization failure)"
    )]
    Encode { key: ExtensionKey, reason: String },
    #[error(
        "extension `{key}` cannot witness its declared inputs: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-SCOPE-IS-DECLARED; \
          fix: narrow the declared inputs glob)"
    )]
    InputManifestOverflow { key: ExtensionKey, reason: String },
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub fn fingerprint_execution(
    row: &ExtensionRegistryRow,
    context: &Context,
) -> Result<String, FingerprintError> {
    fingerprint_execution_with(row, context, None)
}

/// The same fingerprint, plus an agent row's credential-free preparation.
///
/// PROP-054 `##PHASE-FINGERPRINT` names the prompt **documents** — not the
/// prompt address — as create's material, and an address is a constant while
/// its document is authored text. Folding the resolved bytes (instructions and
/// their spec closure) in here is what makes an edited prompt rerun; hashing
/// the address alone would fresh-skip it and silently serve stale outputs.
/// Non-agent rows pass `None` and keep a byte-identical fingerprint.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub fn fingerprint_execution_with(
    row: &ExtensionRegistryRow,
    context: &Context,
    prepared: Option<&PreparedAgent>,
) -> Result<String, FingerprintError> {
    prepare_execution_with(row, context, prepared).map(|prepared| prepared.fingerprint)
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub fn fingerprint_handler_execution(
    execution: &HandlerExecution,
    context: &Context,
) -> Result<String, FingerprintError> {
    fingerprint_handler_execution_with(execution, context, None)
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub fn fingerprint_handler_execution_with(
    execution: &HandlerExecution,
    context: &Context,
    prepared: Option<&PreparedAgent>,
) -> Result<String, FingerprintError> {
    prepare_handler_execution_with(execution, context, prepared)
        .map(|prepared| prepared.fingerprint)
}

/// Stable non-reusable fingerprint for failures before the real fingerprint
/// exists. Error text and paths are deliberately excluded.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub fn preparation_error_fingerprint(key: &ExtensionKey, phase: &str) -> String {
    preparation_error_fingerprint_for_identity(&key.to_string(), phase)
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub fn preparation_error_fingerprint_for_identity(key: &str, phase: &str) -> String {
    let mut hash = FramedHash::new();
    hash.field("key", key.as_bytes());
    hash.field("phase", phase.as_bytes());
    hash.field("transition", b"preparation-error");
    hash.finish()
}

fn handler_coordinates(hash: &mut FramedHash, handler: &ExtensionHandler) {
    match handler {
        ExtensionHandler::Builtin { name } | ExtensionHandler::Binary { name } => {
            hash.field("handler-name", name.as_bytes());
        }
        ExtensionHandler::Script { base } => {
            hash.field("handler-base", machine_path(base).as_bytes());
        }
        ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } => {
            hash.field(
                "handler-crate",
                crate_dir
                    .as_deref()
                    .map(machine_path)
                    .unwrap_or_default()
                    .as_bytes(),
            );
            if let Some(prebuilt) = prebuilt {
                for (platform, path) in prebuilt {
                    hash.field("handler-platform", platform.as_bytes());
                    hash.field("handler-prebuilt", machine_path(path).as_bytes());
                }
            }
        }
        ExtensionHandler::Agent { prompt } => hash.field("handler-prompt", prompt.as_bytes()),
    }
}

fn provider_material(hash: &mut FramedHash, provider: &ExtensionProvider) {
    hash.field("provider-id", provider.to_string().as_bytes());
    match provider {
        ExtensionProvider::Dependency(provider) => {
            hash.field("provider-version", provider.version.as_bytes());
            hash.field(
                "provider-content",
                provider.content_hash.to_string().as_bytes(),
            );
        }
        ExtensionProvider::Host(provider) => {
            hash.field("provider-version", provider.version.as_bytes());
            hash.field(
                "provider-content",
                provider
                    .content_hash
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default()
                    .as_bytes(),
            );
        }
    }
}

/// The lockfile as an INPUT is the locked world it describes — not the moment
/// the file happened to be written.
///
/// `[meta].generated_at` is a fresh RFC3339 stamp on every write, so hashing
/// the raw bytes made a row's fingerprint depend on when its lock was last
/// rewritten. An install that writes the lock and then parks a
/// `slot:post-install` row could therefore NEVER satisfy that park: the resume
/// re-reads a lock whose stamp has moved, the fingerprint differs, and the row
/// reparks forever even though declaration, manifest, resolution and prompt
/// are all unchanged.
///
/// The fix canonicalises at this one authority rather than dropping the input:
/// the lock is parsed and re-serialised with the provenance stamp neutralised,
/// so every resolution-bearing field — packages, roots, schema, solver,
/// features, and `generated_by` (a different vibe can resolve differently) —
/// still contributes exactly as before. A lock that is unparseable falls back
/// to its raw bytes, so nothing silently stops being fingerprinted.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
fn lockfile_identity(
    hash: &mut FramedHash,
    path: &Path,
    key: &ExtensionKey,
) -> Result<(), FingerprintError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            hash.field("lockfile-presence", b"missing");
            return Ok(());
        }
        Err(source) => {
            return Err(FingerprintError::Read {
                key: key.clone(),
                path: machine_path(path),
                source,
            });
        }
    };
    hash.field("lockfile-presence", b"present");
    let canonical = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| toml::from_str::<vibe_core::manifest::Lockfile>(text).ok())
        .map(|mut lockfile| {
            lockfile.meta.generated_at = String::new();
            lockfile
        });
    match canonical {
        Some(lockfile) => hash.json("lockfile", &lockfile, key),
        None => {
            hash.field("lockfile", &bytes);
            Ok(())
        }
    }
}

fn file_or_missing(
    hash: &mut FramedHash,
    label: &str,
    path: &Path,
    key: &ExtensionKey,
) -> Result<(), FingerprintError> {
    match std::fs::read(path) {
        Ok(bytes) => {
            hash.field(&format!("{label}-presence"), b"present");
            hash.field(label, &bytes);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            hash.field(&format!("{label}-presence"), b"missing");
        }
        Err(source) => {
            return Err(FingerprintError::Read {
                key: key.clone(),
                path: machine_path(path),
                source,
            });
        }
    }
    Ok(())
}

fn validate_input(key: &ExtensionKey, pattern: &str) -> Result<(), FingerprintError> {
    let path = Path::new(pattern);
    let drive = pattern.as_bytes().get(1) == Some(&b':');
    let invalid = pattern.contains('\\')
        || path.has_root()
        || pattern.starts_with('/')
        || drive
        || path
            .components()
            .any(|component| component == Component::ParentDir);
    if invalid {
        return Err(FingerprintError::InvalidInput {
            key: key.clone(),
            pattern: pattern.to_string(),
            reason: "absolute, drive-prefixed, backslash, and `..` paths are forbidden".to_string(),
        });
    }
    Ok(())
}

fn shippable_entry(entry: &DirEntry) -> bool {
    entry.depth() == 0
        || !matches!(
            entry.file_name().to_str(),
            Some(".git" | ".vibe" | "target" | "node_modules")
        )
}

fn machine_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The house field frame, seeded by its caller's own domain: every identity
/// this crate mints is `be64(label_len)||label||be64(value_len)||value` under
/// exactly one seed. The frame removes AMBIGUITY — two different member
/// sequences cannot produce one byte stream, and a value cannot drift between
/// identities that use different domains. It is not, and cannot be, a claim
/// that SHA-256 itself admits no collision.
pub(crate) struct FramedHash(Sha256);

impl FramedHash {
    fn new() -> Self {
        Self::seeded(b"vibe-lifecycle-fingerprint\0epoch=1\0")
    }
    /// The declaration fingerprint's own epoch domain — a second identity,
    /// never a relabelling of the execution fingerprint above.
    fn declaration() -> Self {
        Self::seeded(b"vibe-execution-declaration-v1\0epoch=1\0")
    }
    pub(crate) fn seeded(seed: &[u8]) -> Self {
        let mut hash = Sha256::new();
        hash.update(seed);
        Self(hash)
    }
    pub(crate) fn field(&mut self, label: &str, bytes: &[u8]) {
        self.0.update((label.len() as u64).to_be_bytes());
        self.0.update(label.as_bytes());
        self.0.update((bytes.len() as u64).to_be_bytes());
        self.0.update(bytes);
    }
    /// An ASCII `0|1` presence byte under `label` — the explicit
    /// optional-member frame the declaration recipe freezes, so an absent
    /// value can never collide with an authored empty one.
    pub(crate) fn presence(&mut self, label: &str, present: bool) {
        self.field(label, if present { b"1" } else { b"0" });
    }
    /// A canonical decimal UTF-8 count under `label`.
    pub(crate) fn count(&mut self, label: &str, count: usize) {
        self.field(label, count.to_string().as_bytes());
    }
    fn json<T: serde::Serialize>(
        &mut self,
        label: &str,
        value: &T,
        key: &ExtensionKey,
    ) -> Result<(), FingerprintError> {
        let value = serde_json::to_value(value).map_err(|error| FingerprintError::Encode {
            key: key.clone(),
            reason: error.to_string(),
        })?;
        let mut bytes = Vec::new();
        canonical_json(&value, &mut bytes).map_err(|error| FingerprintError::Encode {
            key: key.clone(),
            reason: error.to_string(),
        })?;
        self.field(label, &bytes);
        Ok(())
    }
    pub(crate) fn finish(self) -> String {
        format!("sha256:{:x}", self.0.finalize())
    }
}

fn canonical_json(value: &Value, out: &mut Vec<u8>) -> Result<(), serde_json::Error> {
    match value {
        Value::Object(map) => {
            out.push(b'{');
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                out.extend(serde_json::to_vec(key)?);
                out.push(b':');
                canonical_json(&map[key], out)?;
            }
            out.push(b'}');
        }
        Value::Array(values) => {
            out.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                canonical_json(value, out)?;
            }
            out.push(b']');
        }
        _ => out.extend(serde_json::to_vec(value)?),
    }
    Ok(())
}
