//! Versioned bytes for a lifecycle command's pre-install plan.
//!
//! Only the selected `vibe.toml` and optional `vibe.lock` are witnessed here.
//! Other workspace members, installed sources, handler executables, and the
//! post-install RitualPlan are outside this identity. The builder is pure and
//! does not provide a content CAS against writers outside the lease protocol.

use std::path::Path;

use anyhow::{Result, ensure};
use serde_json::{Value, json};
use vibe_core::lifecycle::Phase;
use vibe_core::user_config::SlotIntegrity;
use vibe_lifecycle::inclusive_chain;
use vibe_lifecycle::process::{SelectedFileWitness, SelectedManifestLockWitness};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use crate::install::{InstallInputs, InstallPolicy};

const FORMAT: &str = "vibevm.lifecycle.pre-effect-plan/1";
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
    pub witness: &'a SelectedManifestLockWitness,
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
    let value = json!({
        "format": FORMAT,
        "lease_root": lease_root,
        "selected_root": selected_root,
        "selected_node": input.selected_node,
        "manifest": manifest,
        "lock": lock,
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
    let mut bytes = Vec::new();
    write_canonical(&value, &mut bytes)?;
    bounded("canonical_plan", bytes.len(), PLAN_CAP)?;
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
    ensure!(witness.size <= FILE_CAP, "{name} exceeds selected-file cap");
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
mod tests {
    use super::*;
    use vibe_core::manifest::SpecFormat;

    fn witness(byte: u8) -> SelectedFileWitness {
        SelectedFileWitness::from_bytes(vec![byte])
    }

    fn model<'a>(
        root: &'a Path,
        sources: &'a SelectedManifestLockWitness,
        policy: &'a InstallPolicy,
        install: &'a InstallInputs,
        mode: &'a RunAgentMode,
    ) -> LifecyclePreEffectPlanInput<'a> {
        LifecyclePreEffectPlanInput {
            lease_root: root,
            selected_root: root,
            selected_node: ".",
            witness: sources,
            requested: Phase::Create,
            agent_mode: mode,
            trace_flag: false,
            effective_trace: false,
            force: false,
            assume_yes: false,
            policy,
            install_inputs: install,
        }
    }

    #[test]
    fn same_inputs_have_same_bytes_and_no_identity_or_clock_field() {
        let dir = tempfile::tempdir().unwrap();
        let sources = SelectedManifestLockWitness {
            manifest: witness(b'a'),
            lock: None,
        };
        let policy = InstallPolicy::default();
        let install = InstallInputs::default();
        let mode = RunAgentMode::Cli;
        let input = model(dir.path(), &sources, &policy, &install, &mode);
        // Exact-field destructuring fails to compile if a future edit adds a
        // random run ID or start clock to this input type.
        let LifecyclePreEffectPlanInput {
            lease_root,
            selected_root,
            selected_node,
            witness,
            requested,
            agent_mode,
            trace_flag,
            effective_trace,
            force,
            assume_yes,
            policy,
            install_inputs,
        } = &input;
        let _ = (
            lease_root,
            selected_root,
            selected_node,
            witness,
            requested,
            agent_mode,
            trace_flag,
            effective_trace,
            force,
            assume_yes,
            policy,
            install_inputs,
        );
        let before = std::fs::read_dir(dir.path()).unwrap().count();
        let first = canonical_lifecycle_pre_effect_plan(&input).unwrap();
        let second = canonical_lifecycle_pre_effect_plan(&input).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with(b"{\"agent_mode\":\"cli\","));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
        let parsed: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(parsed["chain"][0], "validate");
        assert_eq!(parsed["requested"], "create");
        assert!(parsed.get("run_id").is_none() && parsed.get("started").is_none());
    }

    fn assert_changed(input: LifecyclePreEffectPlanInput<'_>, baseline: &[u8]) {
        assert_ne!(
            canonical_lifecycle_pre_effect_plan(&input).unwrap(),
            baseline
        );
    }

    #[test]
    fn critical_posture_witness_and_path_changes_change_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let sources = SelectedManifestLockWitness {
            manifest: witness(b'a'),
            lock: None,
        };
        let mut policy = InstallPolicy::default();
        let mut install = InstallInputs::default();
        let mode = RunAgentMode::Cli;
        let baseline = canonical_lifecycle_pre_effect_plan(&model(
            dir.path(),
            &sources,
            &policy,
            &install,
            &mode,
        ))
        .unwrap();

        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.effective_trace = true;
        assert_changed(changed, &baseline);
        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.trace_flag = true;
        assert_changed(changed, &baseline);
        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.force = true;
        assert_changed(changed, &baseline);
        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.assume_yes = true;
        assert_changed(changed, &baseline);
        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.requested = Phase::Build;
        assert_changed(changed, &baseline);
        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.selected_node = "member";
        assert_changed(changed, &baseline);
        let other_root = dir.path().join("other");
        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.selected_root = &other_root;
        assert_changed(changed, &baseline);
        let mut changed = model(dir.path(), &sources, &policy, &install, &mode);
        changed.lease_root = &other_root;
        assert_changed(changed, &baseline);
        let agent_mode = RunAgentMode::Agent;
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &agent_mode),
            &baseline,
        );

        let with_lock = SelectedManifestLockWitness {
            manifest: witness(b'a'),
            lock: Some(witness(b'b')),
        };
        assert_changed(
            model(dir.path(), &with_lock, &policy, &install, &mode),
            &baseline,
        );
        let changed_sources = SelectedManifestLockWitness {
            manifest: witness(b'b'),
            lock: None,
        };
        assert_changed(
            model(dir.path(), &changed_sources, &policy, &install, &mode),
            &baseline,
        );

        policy.offline = true;
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        policy.offline = false;
        policy.spec_format_default = Some(SpecFormat::Xml);
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        policy.spec_format_default = None;
        policy.slot_integrity = SlotIntegrity::Verify;
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        policy.slot_integrity = SlotIntegrity::TrustPresence;

        install.packages.push("org.example/demo".into());
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        install.packages.clear();
        install.features.push("extra".into());
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        install.features.clear();
        install.language = Some("rust".into());
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        install.language = None;
        install.no_default_features = true;
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        install.no_default_features = false;
        install.all_features = true;
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
        install.all_features = false;
        install.exact = true;
        assert_changed(
            model(dir.path(), &sources, &policy, &install, &mode),
            &baseline,
        );
    }

    #[test]
    fn oversized_fields_refuse_before_any_write() {
        let dir = tempfile::tempdir().unwrap();
        let sources = SelectedManifestLockWitness {
            manifest: witness(b'a'),
            lock: None,
        };
        let policy = InstallPolicy::default();
        let mut install = InstallInputs::default();
        install.packages.push("x".repeat(INPUT_STRING_CAP + 1));
        let mode = RunAgentMode::Cli;
        let before = std::fs::read_dir(dir.path()).unwrap().count();
        let error = canonical_lifecycle_pre_effect_plan(&model(
            dir.path(),
            &sources,
            &policy,
            &install,
            &mode,
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains("packages exceeds"), "{error}");
        install.packages = vec!["x".repeat(3000); INPUT_COUNT_CAP];
        let error = canonical_lifecycle_pre_effect_plan(&model(
            dir.path(),
            &sources,
            &policy,
            &install,
            &mode,
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains("canonical_plan exceeds"), "{error}");
        install.packages.clear();
        let oversized_root = dir.path().join("x".repeat(PATH_CAP + 1));
        let mut input = model(dir.path(), &sources, &policy, &install, &mode);
        input.selected_root = &oversized_root;
        let error = canonical_lifecycle_pre_effect_plan(&input)
            .unwrap_err()
            .to_string();
        assert!(error.contains("selected_root exceeds"), "{error}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    }

    #[test]
    fn fabricated_same_format_digest_refuses_before_serialization() {
        let dir = tempfile::tempdir().unwrap();
        let mut sources = SelectedManifestLockWitness {
            manifest: witness(b'a'),
            lock: None,
        };
        sources.manifest.sha256 = format!("sha256:{}", "b".repeat(64));
        let policy = InstallPolicy::default();
        let install = InstallInputs::default();
        let mode = RunAgentMode::Cli;
        let before = std::fs::read_dir(dir.path()).unwrap().count();
        let error = canonical_lifecycle_pre_effect_plan(&model(
            dir.path(),
            &sources,
            &policy,
            &install,
            &mode,
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains("digest does not match bytes"), "{error}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    }

    #[cfg(unix)]
    #[test]
    fn unix_path_encoding_preserves_non_utf8_bytes_without_writing() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let dir = tempfile::tempdir().unwrap();
        let selected = dir.path().join(OsStr::from_bytes(b"member-\xff"));
        let sources = SelectedManifestLockWitness {
            manifest: witness(b'a'),
            lock: None,
        };
        let policy = InstallPolicy::default();
        let install = InstallInputs::default();
        let mode = RunAgentMode::Cli;
        let mut input = model(dir.path(), &sources, &policy, &install, &mode);
        input.selected_root = &selected;
        let before = std::fs::read_dir(dir.path()).unwrap().count();
        let bytes = canonical_lifecycle_pre_effect_plan(&input).unwrap();
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["selected_root"]["encoding"], "unix-bytes-hex");
        assert!(
            parsed["selected_root"]["value"]
                .as_str()
                .unwrap()
                .ends_with("ff")
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_encoding_preserves_wide_units_without_writing() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let dir = tempfile::tempdir().unwrap();
        let selected = dir.path().join(OsString::from_wide(&[0xd800]));
        let sources = SelectedManifestLockWitness {
            manifest: witness(b'a'),
            lock: None,
        };
        let policy = InstallPolicy::default();
        let install = InstallInputs::default();
        let mode = RunAgentMode::Cli;
        let mut input = model(dir.path(), &sources, &policy, &install, &mode);
        input.selected_root = &selected;
        let before = std::fs::read_dir(dir.path()).unwrap().count();
        let bytes = canonical_lifecycle_pre_effect_plan(&input).unwrap();
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            parsed["selected_root"]["encoding"],
            "windows-utf16-units-hex"
        );
        assert!(
            parsed["selected_root"]["value"]
                .as_str()
                .unwrap()
                .ends_with("d800")
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    }
}
