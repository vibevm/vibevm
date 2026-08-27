//! Epoch/domain-separated deterministic execution fingerprints.

use std::path::{Component, Path};

use glob::{MatchOptions, Pattern};
use serde_json::Value;
use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::{ExtensionHandler, ExtensionKey};
use vibe_wire::generated::lifecycle::e1::context::Context;
use walkdir::{DirEntry, WalkDir};

use crate::HandlerExecution;
use crate::agent::PreparedAgent;
use crate::{ExtensionProvider, ExtensionRegistryRow};

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
    let mut hash = FramedHash::new();
    hash.field("key", row.key().to_string().as_bytes());
    hash.field("phase", context.run.phase.as_bytes());
    hash.field("point", context.point.as_bytes());
    hash.json("slot-target", &context.slot_target, row.key())?;
    hash.field("handler-kind", row.declaration().handler.kind().as_bytes());
    handler_coordinates(&mut hash, &row.declaration().handler);
    hash.json("effective-config", &context.execution.config, row.key())?;
    provider_material(&mut hash, row.provider());
    hash.field("requested", context.run.requested.as_bytes());
    hash.json("chain", &context.run.chain, row.key())?;
    hash.field("offline", &[u8::from(context.run.offline)]);
    hash.json("agent-mode", &context.run.agent_mode, row.key())?;
    hash.json("project", &context.project, row.key())?;
    hash.json("world", &context.world, row.key())?;
    hash.json("artifacts", &context.artifacts, row.key())?;
    file_or_missing(
        &mut hash,
        "manifest",
        Path::new(&context.project.manifest),
        row.key(),
    )?;
    lockfile_identity(&mut hash, Path::new(&context.world.lockfile), row.key())?;
    declared_inputs(&mut hash, row, Path::new(&context.project.root))?;
    if let Some(prepared) = prepared {
        let (address, bytes) = prepared.fingerprint_material();
        hash.field("agent-prompt-address", address.as_bytes());
        hash.field("agent-prompt-bytes", bytes);
    }
    Ok(hash.finish())
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
    let mut fingerprint = fingerprint_execution_with(execution.row(), context, prepared)?;
    if execution.slot_target().is_some() {
        let mut hash = FramedHash::new();
        hash.field("base-fingerprint", fingerprint.as_bytes());
        hash.field("execution-identity", execution.key().as_bytes());
        fingerprint = hash.finish();
    }
    Ok(fingerprint)
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

fn declared_inputs(
    hash: &mut FramedHash,
    row: &ExtensionRegistryRow,
    project_root: &Path,
) -> Result<(), FingerprintError> {
    let Some(patterns) = row.declaration().inputs.as_deref() else {
        return Ok(());
    };
    for authored in patterns {
        validate_input(row.key(), authored)?;
        let pattern = Pattern::new(authored).map_err(|error| FingerprintError::InvalidInput {
            key: row.key().clone(),
            pattern: authored.clone(),
            reason: error.to_string(),
        })?;
        hash.field("input-pattern", authored.as_bytes());
        let mut matches = Vec::new();
        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_entry(shippable_entry)
        {
            let entry = entry.map_err(|error| FingerprintError::Read {
                key: row.key().clone(),
                path: machine_path(project_root),
                source: error
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walking input tree")),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative =
                entry
                    .path()
                    .strip_prefix(project_root)
                    .map_err(|_| FingerprintError::Read {
                        key: row.key().clone(),
                        path: machine_path(entry.path()),
                        source: std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "input walk escaped its project root",
                        ),
                    })?;
            // Authored patterns are UTF-8. A non-UTF-8 filesystem name is
            // outside that namespace and cannot be selected; skipping it
            // avoids both false failures and lossy alias/order collisions.
            let Some(relative) = relative.to_str() else {
                continue;
            };
            let relative = relative.replace('\\', "/");
            let options = MatchOptions {
                case_sensitive: true,
                require_literal_separator: true,
                require_literal_leading_dot: false,
            };
            if pattern.matches_with(&relative, options) {
                matches.push((relative, entry.into_path()));
            }
        }
        matches.sort_by(|left, right| left.0.cmp(&right.0));
        for (relative, path) in matches {
            let bytes = std::fs::read(&path).map_err(|source| FingerprintError::Read {
                key: row.key().clone(),
                path: machine_path(&path),
                source,
            })?;
            hash.field("input-path", relative.as_bytes());
            hash.field("input-bytes", &bytes);
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

struct FramedHash(Sha256);

impl FramedHash {
    fn new() -> Self {
        let mut hash = Sha256::new();
        hash.update(b"vibe-lifecycle-fingerprint\0epoch=1\0");
        Self(hash)
    }
    fn field(&mut self, label: &str, bytes: &[u8]) {
        self.0.update((label.len() as u64).to_be_bytes());
        self.0.update(label.as_bytes());
        self.0.update((bytes.len() as u64).to_be_bytes());
        self.0.update(bytes);
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
    fn finish(self) -> String {
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
