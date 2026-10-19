//! Shell-local alternative selection through PATH precedence.
//!
//! Each installation view has a stable native proxy. Vibe constructs an
//! environment delta for the caller's shell; an ordinary proxy invocation
//! never needs the application index or a session token.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use super::super::model::{ApplicationIndex, ApplicationStatus};
use super::projection::view_bin;

const PROTOCOL: &str = "vibe-app-session-env/1";
const VIEW_PROTOCOL: &str = "vibe-app-view/1";
const MAX_OWNER_BYTES: u64 = 1024;

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "Session alternatives are a CLI environment adapter: they read the caller's PATH once per operation to derive a child-shell delta and never pass ambient access into installation domain logic."
)]
fn ambient_path() -> OsString {
    std::env::var_os("PATH").unwrap_or_default()
}

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "Opening a nested interactive shell requires reading the caller's SHELL or COMSPEC at this CLI process boundary; the chosen program is passed as data to Command."
)]
fn ambient_shell_program() -> OsString {
    std::env::var_os(if cfg!(windows) { "COMSPEC" } else { "SHELL" })
        .unwrap_or_else(|| if cfg!(windows) { "cmd.exe" } else { "/bin/sh" }.into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewOwner {
    protocol: String,
    slot: String,
    installation: String,
}

pub(super) fn effective_from_path(
    slot: &str,
    settings: &Path,
    index: &ApplicationIndex,
) -> Result<Option<String>> {
    let settings = fs::canonicalize(settings).context("resolving application settings root")?;
    let current = ambient_path();
    effective_from_entries(slot, &settings, index, &current)
}

fn effective_from_entries(
    slot: &str,
    settings: &Path,
    index: &ApplicationIndex,
    current: &OsStr,
) -> Result<Option<String>> {
    for entry in std::env::split_paths(current) {
        if !managed_view(&entry, settings, Some(slot)) {
            continue;
        }
        let owner_path = entry
            .parent()
            .context("installation view has no root")?
            .join("owner.json");
        let metadata = fs::symlink_metadata(&owner_path)
            .with_context(|| format!("stale application session view `{}`", entry.display()))?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_OWNER_BYTES {
            bail!("application session view has an invalid owner marker");
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        File::open(&owner_path)?
            .take(MAX_OWNER_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_OWNER_BYTES {
            bail!("application session view owner marker exceeds its size limit");
        }
        let owner: ViewOwner = serde_json::from_slice(&bytes)
            .context("parsing application session view owner marker")?;
        if owner.protocol != VIEW_PROTOCOL
            || owner.slot != slot
            || view_bin(settings, slot, &owner.installation) != entry
            || !entry.is_dir()
        {
            bail!("application session view does not match its owner marker");
        }
        let Some(record) = index.applications.get(&owner.installation) else {
            bail!(
                "application session selected a removed installation `{}`",
                owner.installation
            );
        };
        if record.application.id != slot || record.status != ApplicationStatus::Ready {
            bail!(
                "application session selected a non-ready installation `{}`",
                owner.installation
            );
        }
        return Ok(Some(owner.installation));
    }
    Ok(None)
}

pub(super) fn use_session(
    slot: &str,
    key: &str,
    shell: &str,
    settings: &Path,
    index: &ApplicationIndex,
) -> Result<Value> {
    let view = selected_view(slot, key, settings, index)?;
    let settings = fs::canonicalize(settings).context("resolving application settings root")?;
    let current = ambient_path();
    let path = select_path(&current, &settings, slot, &view)?;
    path_delta(path, shell)
}

pub(super) fn clear_session(
    slot: Option<&str>,
    all: bool,
    shell: &str,
    settings: &Path,
    _index: &ApplicationIndex,
) -> Result<Value> {
    if !all && slot.is_none() {
        bail!("specify an application slot or --all");
    }
    let settings = fs::canonicalize(settings).context("resolving application settings root")?;
    let current = ambient_path();
    let path = clear_path(&current, &settings, if all { None } else { slot })?;
    path_delta(path, shell)
}

pub(super) fn shell(
    slot: &str,
    key: &str,
    settings: &Path,
    index: &ApplicationIndex,
) -> Result<i32> {
    let view = selected_view(slot, key, settings, index)?;
    let settings = fs::canonicalize(settings).context("resolving application settings root")?;
    let current = ambient_path();
    let path = select_path(&current, &settings, slot, &view)?;
    let program = ambient_shell_program();
    let status = Command::new(program)
        .env("PATH", path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("opening application alternative shell")?;
    Ok(status.code().unwrap_or(1))
}

fn selected_view(
    slot: &str,
    key: &str,
    settings: &Path,
    index: &ApplicationIndex,
) -> Result<PathBuf> {
    let record = index
        .applications
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("unknown application installation `{key}`"))?;
    if record.application.id != slot || record.status != ApplicationStatus::Ready {
        bail!("application installation `{key}` is not ready in slot `{slot}`");
    }
    let canonical_settings =
        fs::canonicalize(settings).context("resolving application settings root")?;
    let view = view_bin(&canonical_settings, slot, key);
    let metadata = fs::symlink_metadata(&view)
        .with_context(|| format!("installation view is unavailable: `{}`", view.display()))?;
    if !metadata.file_type().is_dir() {
        bail!("installation view is not a directory: `{}`", view.display());
    }
    let one_entry = std::env::join_paths([view.clone()])?;
    if effective_from_entries(slot, &canonical_settings, index, &one_entry)?.as_deref() != Some(key)
    {
        bail!("installation view does not select `{key}`");
    }
    Ok(view)
}

fn select_path(current: &OsStr, settings: &Path, slot: &str, selected: &Path) -> Result<OsString> {
    let mut entries = vec![selected.to_path_buf()];
    entries.extend(
        std::env::split_paths(current).filter(|entry| !managed_view(entry, settings, Some(slot))),
    );
    std::env::join_paths(entries).context("joining application session PATH")
}

fn clear_path(current: &OsStr, settings: &Path, slot: Option<&str>) -> Result<OsString> {
    let entries = std::env::split_paths(current)
        .filter(|entry| !managed_view(entry, settings, slot))
        .collect::<Vec<_>>();
    std::env::join_paths(entries).context("joining application session PATH")
}

fn managed_view(entry: &Path, settings: &Path, slot: Option<&str>) -> bool {
    let alternatives = settings.join("opt").join("alternatives");
    let Ok(relative) = entry.strip_prefix(&alternatives) else {
        return false;
    };
    let parts = relative.components().collect::<Vec<_>>();
    let [
        Component::Normal(found_slot),
        Component::Normal(installations),
        Component::Normal(hash),
        Component::Normal(bin),
    ] = parts.as_slice()
    else {
        return false;
    };
    if *installations != OsStr::new("installations") || *bin != OsStr::new("bin") {
        return false;
    }
    let Some(hash) = hash.to_str() else {
        return false;
    };
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return false;
    }
    slot.is_none_or(|slot| *found_slot == OsStr::new(slot))
}

fn path_delta(path: OsString, shell: &str) -> Result<Value> {
    let path = path.into_string().map_err(|_| {
        anyhow::anyhow!("PATH is not Unicode; cannot emit a shell or JSON environment delta")
    })?;
    let script = match shell.to_ascii_lowercase().as_str() {
        "posix" | "sh" | "bash" | "zsh" => {
            let quoted = path.replace('\'', "'\"'\"'");
            format!("export PATH='{quoted}'; hash -r 2>/dev/null || true")
        }
        "powershell" | "pwsh" => {
            let quoted = path.replace('\'', "''");
            format!("$env:PATH = '{quoted}'")
        }
        "cmd" | "cmd.exe" => {
            if path.contains(['%', '!', '^', '"', '\r', '\n']) {
                bail!("PATH contains characters that cannot be safely assigned in cmd");
            }
            format!("set \"PATH={path}\"")
        }
        _ => bail!("unsupported shell `{shell}`; expected posix, powershell, or cmd"),
    };
    Ok(json!({"protocol":PROTOCOL,"set":{"PATH":path},"unset":[],"script":script}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::application::model::{
        ApplicationIdentity, ApplicationRecord, ManagementEntry, ManagementRuntime, PackageIdentity,
    };
    use sha2::{Digest, Sha256};

    fn fake_view(settings: &Path, slot: &str, key: &str) -> PathBuf {
        let digest = format!("{:x}", Sha256::digest(key.as_bytes()));
        settings
            .join("opt/alternatives")
            .join(slot)
            .join("installations")
            .join(digest)
            .join("bin")
    }

    #[test]
    fn independent_slots_keep_each_other_and_unrelated_path_entries() {
        let root = tempfile::tempdir().unwrap();
        let unrelated = root.path().join("other-bin");
        let first = fake_view(root.path(), "one", "group/one@1.0.0");
        let second = fake_view(root.path(), "two", "group/two@1.0.0");
        let initial = std::env::join_paths([unrelated.clone()]).unwrap();
        let after_one = select_path(&initial, root.path(), "one", &first).unwrap();
        let after_two = select_path(&after_one, root.path(), "two", &second).unwrap();
        assert_eq!(
            std::env::split_paths(&after_two).collect::<Vec<_>>(),
            vec![second.clone(), first.clone(), unrelated.clone()]
        );
        let cleared = clear_path(&after_two, root.path(), Some("one")).unwrap();
        assert_eq!(
            std::env::split_paths(&cleared).collect::<Vec<_>>(),
            vec![second, unrelated]
        );
    }

    #[test]
    fn reselect_and_clear_remove_stale_views_even_when_missing_on_disk() {
        let root = tempfile::tempdir().unwrap();
        let stale = fake_view(root.path(), "demo", "group/demo@old");
        let fresh = fake_view(root.path(), "demo", "group/demo@new");
        let unrelated = root.path().join("unrelated");
        let initial = std::env::join_paths([stale, unrelated.clone()]).unwrap();
        let selected = select_path(&initial, root.path(), "demo", &fresh).unwrap();
        assert_eq!(
            std::env::split_paths(&selected).collect::<Vec<_>>(),
            vec![fresh, unrelated.clone()]
        );
        let cleared = clear_path(&selected, root.path(), None).unwrap();
        assert_eq!(
            std::env::split_paths(&cleared).collect::<Vec<_>>(),
            vec![unrelated]
        );
    }

    #[test]
    fn shell_delta_quotes_path_and_requires_supported_shell() {
        let path = OsString::from("/tmp/a'b:/usr/bin");
        let value = path_delta(path, "posix").unwrap();
        assert_eq!(value["set"]["PATH"], "/tmp/a'b:/usr/bin");
        assert!(value["script"].as_str().unwrap().contains("'\"'\"'"));
        assert!(path_delta(OsString::from("path"), "fish").is_err());
    }

    #[test]
    fn path_override_checks_owner_and_ready_index_entry() {
        let root = tempfile::tempdir().unwrap();
        let slot = "demo";
        let key = "org.example/demo@1.0.0";
        let view = view_bin(root.path(), slot, key);
        fs::create_dir_all(&view).unwrap();
        fs::write(
            view.parent().unwrap().join("owner.json"),
            serde_json::to_vec(&json!({
                "protocol": VIEW_PROTOCOL, "slot": slot, "installation": key
            }))
            .unwrap(),
        )
        .unwrap();
        let package = PackageIdentity {
            group: "org.example".into(),
            name: "demo".into(),
            version: "1.0.0".into(),
        };
        let mut index = ApplicationIndex::default();
        index.applications.insert(
            key.into(),
            ApplicationRecord {
                application: ApplicationIdentity {
                    id: slot.into(),
                    package: package.clone(),
                    installer_package: package,
                    commands: Vec::new(),
                },
                host_root: root.path().join("host"),
                app_root: None,
                install_kind: None,
                selected_option_id: None,
                selected_platform: None,
                management: ManagementEntry {
                    runtime: ManagementRuntime::Native,
                    entry: root.path().join("manage"),
                },
                status: ApplicationStatus::Ready,
                provenance: None,
                launchers: Vec::new(),
                user_launchers: Vec::new(),
                public_commands: Vec::new(),
            },
        );
        let path = std::env::join_paths([root.path().join("other"), view.clone()]).unwrap();
        assert_eq!(
            effective_from_entries(slot, root.path(), &index, &path).unwrap(),
            Some(key.into())
        );
        index.applications.clear();
        assert!(effective_from_entries(slot, root.path(), &index, &path).is_err());
        assert!(selected_view(slot, key, root.path(), &index).is_err());
    }
}
