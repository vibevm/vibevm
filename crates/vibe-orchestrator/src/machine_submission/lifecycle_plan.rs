//! Versioned bytes for a lifecycle command's pre-install plan.
//!
//! The selected node and workspace root each contribute their `vibe.toml`
//! and optional `vibe.lock` witnesses. Other workspace members, installed
//! sources, handler executables, and the
//! post-install RitualPlan are outside this identity. The builder is pure and
//! does not provide a content CAS against writers outside the lease protocol.

use std::path::Path;

use anyhow::{Result, ensure};
use serde_json::{Value, json};
use vibe_core::lifecycle::Phase;
use vibe_core::user_config::SlotIntegrity;
use vibe_lifecycle::inclusive_chain;
use vibe_lifecycle::process::{
    PackageTreeWitness, SelectedFileWitness, SelectedManifestLockWitness,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use crate::install::{InstallInputs, InstallPolicy};

const FORMAT_V2: &str = "vibevm.lifecycle.pre-effect-plan/2";
const FORMAT_SOURCE_V3: &str = "vibevm.lifecycle.pre-effect-plan/3";
const FILE_CAP: u64 = 8 * 1024 * 1024;
const PATH_CAP: usize = 128 * 1024;
const SELECTED_CAP: usize = 128 * 1024;
const INPUT_COUNT_CAP: usize = 1024;
const INPUT_STRING_CAP: usize = 4096;
const LANGUAGE_CAP: usize = 256;
// C.1 reserves at most 1 MiB of canonical plan bytes.
const PLAN_CAP: usize = 1024 * 1024;

/// Exact values the owner has already selected under its workspace lease.
/// There is deliberately no run ID or start clock: retries of the same key
/// must compare the same plan even after a new pure ID proposal.
pub(crate) struct LifecyclePreEffectPlanInput<'a> {
    pub lease_root: &'a Path,
    pub selected_root: &'a Path,
    pub selected_node: &'a str,
    /// Exact selected-node files; may differ from the workspace-root files.
    pub witness: &'a SelectedManifestLockWitness,
    /// Exact workspace-root files, including the effective-world lock.
    pub workspace_witness: &'a SelectedManifestLockWitness,
    /// Present only for a separately captured and later rechecked local
    /// package fixture. Opaque filesystem identity stays outside wire bytes.
    pub package_source: Option<&'a PackageTreeWitness>,
    pub requested: Phase,
    pub agent_mode: &'a RunAgentMode,
    pub trace_flag: bool,
    pub effective_trace: bool,
    pub force: bool,
    pub assume_yes: bool,
    pub policy: &'a InstallPolicy,
    pub install_inputs: &'a InstallInputs,
}

/// Serialize a fixed-field JSON model after checking all individual and total
/// bounds. This authenticates the supplied witness against its own bytes; the
/// caller still owns the later filesystem recheck before any effect.
pub(crate) fn canonical_lifecycle_pre_effect_plan(
    input: &LifecyclePreEffectPlanInput<'_>,
) -> Result<Vec<u8>> {
    input.witness.validate()?;
    input.workspace_witness.validate()?;
    ensure!(
        input.lease_root != input.selected_root || input.witness == input.workspace_witness,
        "one root cannot have two different manifest/lock witnesses"
    );
    let lease_root = path_value(input.lease_root, "lease_root")?;
    let selected_root = path_value(input.selected_root, "selected_root")?;
    bounded("selected_node", input.selected_node.len(), SELECTED_CAP)?;
    let manifest = witness_value(&input.witness.manifest, "vibe.toml")?;
    let lock = input
        .witness
        .lock
        .as_ref()
        .map(|witness| witness_value(witness, "vibe.lock"))
        .transpose()?;
    let workspace_manifest =
        witness_value(&input.workspace_witness.manifest, "workspace vibe.toml")?;
    let workspace_lock = input
        .workspace_witness
        .lock
        .as_ref()
        .map(|witness| witness_value(witness, "workspace vibe.lock"))
        .transpose()?;
    let packages = bounded_strings("packages", &input.install_inputs.packages)?;
    let features = bounded_strings("features", &input.install_inputs.features)?;
    if let Some(language) = input.install_inputs.language.as_ref() {
        bounded("language", language.len(), LANGUAGE_CAP)?;
    }
    let chain: Vec<&str> = inclusive_chain(input.requested)
        .iter()
        .map(|phase| phase.as_str())
        .collect();
    let agent_mode = match input.agent_mode {
        RunAgentMode::Agent => "agent",
        RunAgentMode::Cli => "cli",
    };
    let slot_integrity = match input.policy.slot_integrity {
        SlotIntegrity::TrustPresence => "trust-presence",
        SlotIntegrity::Verify => "verify",
    };
    let mut value = json!({
        "format": if input.package_source.is_some() { FORMAT_SOURCE_V3 } else { FORMAT_V2 },
        "lease_root": lease_root,
        "selected_root": selected_root,
        "selected_node": input.selected_node,
        "manifest": manifest,
        "lock": lock,
        "workspace_manifest": workspace_manifest,
        "workspace_lock": workspace_lock,
        "requested": input.requested.as_str(),
        "chain": chain,
        "agent_mode": agent_mode,
        "trace_flag": input.trace_flag,
        "effective_trace": input.effective_trace,
        "force": input.force,
        "assume_yes": input.assume_yes,
        "policy": {
            "offline": input.policy.offline,
            "slot_integrity": slot_integrity,
            "spec_format_default": input.policy.spec_format_default.map(|format| format.as_str()),
        },
        "install": {
            "packages": packages,
            "features": features,
            "no_default_features": input.install_inputs.no_default_features,
            "all_features": input.install_inputs.all_features,
            "language": input.install_inputs.language,
            "exact": input.install_inputs.exact,
        },
    });
    if let Some(source) = input.package_source {
        ensure!(
            input.requested == Phase::Install,
            "package source is only valid for Install"
        );
        source.validate()?;
        value
            .as_object_mut()
            .expect("fixed pre-effect plan is a JSON object")
            .insert("source".into(), package_source_value(source)?);
    }
    let mut bytes = Vec::new();
    write_canonical(&value, &mut bytes)?;
    bounded("canonical_plan", bytes.len(), PLAN_CAP)?;
    Ok(bytes)
}

fn package_source_value(source: &PackageTreeWitness) -> Result<Value> {
    let root = path_value(&source.root, "source.root")?;
    let files: Vec<Value> = source
        .files
        .iter()
        .map(|file| {
            json!({
                "path": file.relative_path,
                "size": file.bytes,
                "sha256": file.sha256,
            })
        })
        .collect();
    Ok(json!({
        "package_pin": source.package_pin.to_string(),
        "root": root,
        "tree_recipe": "vibevm.lifecycle.package-tree/1",
        "tree_sha256": source.tree_sha256,
        "files": files,
    }))
}

/// Fixture-only request identity independent of post-effect manifest/lock
/// changes. Its source object is identical to the v3 plan's source object.
pub(crate) fn canonical_one_package_fixture_request(
    source: &PackageTreeWitness,
    target_os: vibe_core::manifest::TargetOs,
) -> Result<Vec<u8>> {
    source.validate()?;
    let value = json!({
        "operation": "lifecycle.install-one-package-fixture",
        "schema": 1,
        "target_os": target_os.as_str(),
        "source": package_source_value(source)?,
    });
    let mut bytes = Vec::new();
    write_canonical(&value, &mut bytes)?;
    bounded("canonical_request", bytes.len(), PLAN_CAP)?;
    Ok(bytes)
}

// Sort every object key independently of serde_json's optional map backend.
// Arrays retain their authored order; this model contains no floating values.
fn write_canonical(value: &Value, out: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(number) => out.extend_from_slice(number.to_string().as_bytes()),
        Value::String(string) => serde_json::to_writer(out, string)?,
        Value::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                write_canonical(item, out)?;
            }
            out.push(b']');
        }
        Value::Object(fields) => {
            out.push(b'{');
            let mut keys: Vec<&String> = fields.keys().collect();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key)?;
                out.push(b':');
                write_canonical(&fields[key], out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

fn bounded(name: &str, length: usize, cap: usize) -> Result<()> {
    ensure!(length <= cap, "{name} exceeds {cap} bytes/items");
    Ok(())
}

fn bounded_strings<'a>(name: &str, values: &'a [String]) -> Result<Vec<&'a str>> {
    bounded(name, values.len(), INPUT_COUNT_CAP)?;
    for value in values {
        bounded(name, value.len(), INPUT_STRING_CAP)?;
    }
    Ok(values.iter().map(String::as_str).collect())
}

fn witness_value(witness: &SelectedFileWitness, name: &str) -> Result<Value> {
    ensure!(witness.size <= FILE_CAP, "{name} exceeds source-file cap");
    ensure!(
        witness.size == witness.bytes.len() as u64,
        "{name} witness byte count differs from its size"
    );
    let digest = witness
        .sha256
        .strip_prefix("sha256:")
        .ok_or_else(|| anyhow::anyhow!("{name} witness digest is not sha256"))?;
    ensure!(
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "{name} witness digest is not 64 lowercase hex characters"
    );
    Ok(json!({"sha256": witness.sha256, "size": witness.size}))
}

fn path_value(path: &Path, name: &str) -> Result<Value> {
    ensure!(path.is_absolute(), "{name} must be absolute");
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let raw = path.as_os_str().as_bytes();
        bounded(name, raw.len(), PATH_CAP)?;
        return Ok(json!({"encoding": "unix-bytes-hex", "value": hex(raw)}));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<u16> = path.as_os_str().encode_wide().collect();
        bounded(name, units.len().saturating_mul(2), PATH_CAP)?;
        let mut value = String::with_capacity(units.len() * 4);
        use std::fmt::Write;
        for unit in units {
            write!(&mut value, "{unit:04x}").expect("writing hex into String cannot fail");
        }
        return Ok(json!({"encoding": "windows-utf16-units-hex", "value": value}));
    }
    #[cfg(not(any(unix, windows)))]
    anyhow::bail!("{name} has no supported lossless path encoding on this platform")
}

#[cfg(unix)]
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 15)] as char);
    }
    out
}

#[cfg(test)]
#[path = "lifecycle_plan/tests.rs"]
mod tests;
