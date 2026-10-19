//! Filesystem projection for application alternatives.
//!
//! The durable index owns public aliases. The proxy only reads sidecars and
//! one per-slot pointer, so a selection changes with one atomic pointer write.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use vibe_core::manifest::is_portable_token;
use vibe_safefs::{Project, ensure_no_follow_walk};

use super::super::model::{
    ApplicationIndex, ApplicationLauncherOwnership, ApplicationRecord, ApplicationStatus,
};
use super::super::store::ApplicationStore;

mod descriptor;
mod files;
mod immutable;
mod lookup;
mod recovery;
pub(super) use files::proxy_filename;
use files::{
    link_or_copy, path_present, prior_pointer, publish_immutable_directory, remove_intent,
    set_executable, sidecar_path, write_current, write_new, write_sidecar,
};
use immutable::{
    ViewContext, ViewPointer, prepare_view, projection_descriptors, projection_id,
    publish_projection,
};
pub(super) use lookup::published_global;
pub(in crate::commands::application) use recovery::recover_pending;
use recovery::source_proxy;

const PROXY_PROTOCOL: &str = "vibe-app-proxy/1";
const CURRENT_PROTOCOL: &str = "vibe-app-current/1";
const INTENT_PROTOCOL: &str = "vibe-app-alternative-intent/1";

pub(in crate::commands::application) struct PreparedSlot {
    settings: PathBuf,
    slot: String,
    projection: String,
    runtime: PathBuf,
    views: Vec<ViewPointer>,
    aliases: Vec<Alias>,
    intent: PathBuf,
    intent_bytes: Vec<u8>,
    current_committed: bool,
}

struct Alias {
    destination: PathBuf,
    sidecar: PathBuf,
    sidecar_bytes: Vec<u8>,
    create_binary: bool,
}

/// Stable PATH entry for a session that selects one exact installation.
pub(in crate::commands::application) fn view_bin(
    settings: &Path,
    slot: &str,
    key: &str,
) -> PathBuf {
    settings
        .join("opt/alternatives")
        .join(slot)
        .join("installations")
        .join(digest(key.as_bytes()))
        .join("bin")
}

/// Prepare immutable views and the selected command projection. The caller
/// holds the application's mutation lock and saves `index` before publishing
/// the current pointer.
pub(in crate::commands::application) fn prepare_slot(
    index: &mut ApplicationIndex,
    settings: &Path,
    slot: &str,
) -> Result<PreparedSlot> {
    prepare_slot_internal(index, settings, slot, None)
}

fn prepare_slot_internal(
    index: &mut ApplicationIndex,
    settings: &Path,
    slot: &str,
    existing_intent: Option<Vec<u8>>,
) -> Result<PreparedSlot> {
    if !is_portable_token(slot) {
        bail!("invalid application slot");
    }
    let settings = normalized_settings(settings)?;
    let project = Project::open(&settings).context("opening no-follow settings root")?;
    let alternatives = index
        .alternatives
        .get(slot)
        .ok_or_else(|| anyhow::anyhow!("unknown application slot `{slot}`"))?;
    let reserved = alternatives.reserved_commands.clone();
    let selected = alternatives.global_default.clone();
    let prior_runners = alternatives.runners.clone();
    let prior_revision = ApplicationStore::load_snapshot(&settings)?.revision;
    let desired_revision = index.revision.clone();
    if desired_revision.is_empty() {
        bail!("application index has no intended revision");
    }
    let mut seen = BTreeSet::new();
    for name in &reserved {
        if !is_portable_token(name) || !seen.insert(name) {
            bail!("application slot has an invalid or duplicate reserved command name");
        }
    }
    let (runtime, proxy_hash) = if reserved.is_empty() {
        (PathBuf::new(), String::new())
    } else {
        proxy_runtime(&project, &settings, &prior_runners)?
    };
    let slot_dir = settings.join("opt/alternatives").join(slot);
    managed_dir(&project, &["opt", "alternatives", slot])?;

    let mut views = Vec::new();
    let view_context = ViewContext {
        project: &project,
        settings: &settings,
        runtime: &runtime,
        proxy_hash: &proxy_hash,
        slot_dir: &slot_dir,
        slot,
        reserved: &reserved,
    };
    for (key, record) in index
        .applications
        .iter()
        .filter(|(_, record)| record.application.id == slot)
    {
        views.push(prepare_view(&view_context, key, record)?);
    }
    let descriptors = projection_descriptors(index, &settings, selected.as_deref(), &reserved)?;
    let projection = projection_id(slot, selected.as_deref(), &descriptors)?;
    publish_projection(
        &project,
        &settings,
        &slot_dir,
        slot,
        selected.as_deref(),
        &projection,
        &descriptors,
    )?;

    let mut aliases = Vec::new();
    let mut next_runners = prior_runners.clone();
    for name in &reserved {
        let destination = settings.join("opt/bin").join(proxy_filename(name));
        let sidecar = sidecar_path(&destination)?;
        let bytes = serde_json::to_vec(&json!({
            "protocol": PROXY_PROTOCOL,
            "mode": "slot",
            "slot": slot,
            "command": name,
            "current": slot_dir.join("current.json"),
        }))?;
        let owned = prior_runners
            .iter()
            .find(|runner| runner.destination == destination);
        let create_binary = check_global_alias(
            &settings,
            &destination,
            &sidecar,
            owned,
            &proxy_hash,
            &bytes,
        )?;
        if owned.is_none() {
            next_runners.push(ApplicationLauncherOwnership {
                destination: destination.clone(),
                sha256: proxy_hash.clone(),
            });
        }
        aliases.push(Alias {
            destination,
            sidecar,
            sidecar_bytes: bytes,
            create_binary,
        });
    }
    index
        .alternatives
        .get_mut(slot)
        .ok_or_else(|| anyhow::anyhow!("application slot disappeared during preparation"))?
        .runners = next_runners;

    let intent_dir = managed_dir(&project, &["applications", "alternative-intents"])?;
    let intent = settings
        .join("applications/alternative-intents")
        .join(format!("{slot}.json"));
    let intent_bytes = if let Some(bytes) = existing_intent {
        if !path_present(&intent)? || read_regular(&intent)? != bytes {
            bail!("application alternative recovery intent changed");
        }
        bytes
    } else {
        if path_present(&intent)? {
            bail!("unreconciled application alternative intent for `{slot}`");
        }
        let prior_current = prior_pointer(&slot_dir.join("current.json"))?;
        let view_intents = views
            .iter()
            .map(|view| {
                Ok(json!({
                    "current":view.path,
                    "prior":prior_pointer(&view.path)?,
                    "projection":view.projection,
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        let bytes = serde_json::to_vec(&json!({
            "protocol": INTENT_PROTOCOL,
            "slot": slot,
            "prior_revision": prior_revision,
            "desired_revision": desired_revision,
            "prior_current": prior_current,
            "projection": projection,
            "views": view_intents,
        }))?;
        project
            .write_atomic_durable_in(&intent_dir, &format!("{slot}.json"), &bytes)
            .map_err(|error| error.into_report())
            .context("writing application alternative intent")?;
        bytes
    };
    Ok(PreparedSlot {
        settings,
        slot: slot.to_owned(),
        projection,
        runtime,
        views,
        aliases,
        intent,
        intent_bytes,
        current_committed: false,
    })
}

fn normalized_settings(settings: &Path) -> Result<PathBuf> {
    Ok(crate::commands::init::strip_unc_public(
        fs::canonicalize(settings).context("resolving application settings root")?,
    ))
}

#[cfg(all(test, windows))]
mod path_tests {
    use super::normalized_settings;

    #[test]
    fn settings_root_uses_the_public_windows_path_form() {
        let root = tempfile::tempdir().unwrap();
        let normalized = normalized_settings(root.path()).unwrap();

        assert_eq!(
            normalized,
            crate::commands::init::strip_unc_public(std::fs::canonicalize(root.path()).unwrap())
        );
        assert!(!normalized.to_string_lossy().starts_with(r"\\?\"));
    }
}

impl PreparedSlot {
    /// Publish aliases first, then atomically switch the one selected pointer.
    /// On failure, the durable intent remains for orchestration repair.
    pub(in crate::commands::application) fn commit_current(&mut self) -> Result<()> {
        let project = Project::open(&self.settings)?;
        for view in &self.views {
            write_current(&project, &self.settings, &view.path, &view.projection)?;
        }
        managed_dir(&project, &["opt", "bin"])?;
        for alias in &self.aliases {
            if alias.create_binary {
                link_or_copy(&self.runtime, &alias.destination)?;
            }
            write_sidecar(
                &project,
                &self.settings,
                &alias.sidecar,
                &alias.sidecar_bytes,
            )?;
        }
        let current = self
            .settings
            .join("opt/alternatives")
            .join(&self.slot)
            .join("current.json");
        write_current(&project, &self.settings, &current, &self.projection)?;
        self.current_committed = true;
        Ok(())
    }

    /// Only call after a proven pre-publication index failure. Immutable
    /// projections remain harmless; the intent is removed.
    pub(in crate::commands::application) fn rollback(self) -> Result<()> {
        if self.current_committed {
            bail!("cannot roll back an already published application pointer");
        }
        remove_intent(&self.settings, &self.intent, &self.intent_bytes)
    }

    /// Finish a confirmed index + pointer publication.
    pub(in crate::commands::application) fn commit(self) -> Result<()> {
        if !self.current_committed {
            bail!("application current pointer was not published");
        }
        remove_intent(&self.settings, &self.intent, &self.intent_bytes)
    }
}

fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in hash {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn read_regular(path: &Path) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        bail!(
            "refusing non-regular application projection file `{}`",
            path.display()
        );
    }
    fs::read(path).with_context(|| format!("reading `{}`", path.display()))
}

fn managed_dir(project: &Project, path: &[&str]) -> Result<vibe_safefs::Pinned> {
    project
        .dir(path, true)
        .context("opening no-follow projection directory")
}

fn install_runtime(
    project: &Project,
    settings: &Path,
    bytes: &[u8],
    hash: &str,
) -> Result<PathBuf> {
    let runtime_root = settings.join("opt/alternatives/runtime");
    managed_dir(project, &["opt", "alternatives", "runtime"])?;
    let name = proxy_filename("vibe-app-proxy");
    let path = runtime_root.join(hash).join(&name);
    publish_immutable_directory(&runtime_root, hash, |staging| {
        let staged_runtime = staging.join(&name);
        write_new(&staged_runtime, bytes)?;
        set_executable(&staged_runtime)?;
        Ok(())
    })?;
    ensure_no_follow_walk(settings, &path, true)?;
    if digest(&read_regular(&path)?) != hash {
        bail!("owned application proxy runtime drifted");
    }
    Ok(path)
}

fn proxy_runtime(
    project: &Project,
    settings: &Path,
    prior_runners: &[ApplicationLauncherOwnership],
) -> Result<(PathBuf, String)> {
    // A slot's public aliases and installation views share one proxy ABI.
    // Keep their receipt-selected runtime until an explicit refresh can rotate all of them.
    if let Some(hash) = retained_proxy_hash(prior_runners)? {
        let path = settings
            .join("opt/alternatives/runtime")
            .join(hash)
            .join(proxy_filename("vibe-app-proxy"));
        ensure_no_follow_walk(settings, &path, true)?;
        if !path_present(&path)? {
            bail!("owned application proxy runtime is absent");
        }
        if digest(&read_regular(&path)?) != hash {
            bail!("owned application proxy runtime drifted");
        }
        return Ok((path, hash.to_owned()));
    }

    let proxy = source_proxy()?;
    let proxy_bytes = read_regular(&proxy)?;
    let proxy_hash = digest(&proxy_bytes);
    let runtime = install_runtime(project, settings, &proxy_bytes, &proxy_hash)?;
    Ok((runtime, proxy_hash))
}

fn retained_proxy_hash(prior_runners: &[ApplicationLauncherOwnership]) -> Result<Option<&str>> {
    let Some(hash) = prior_runners.first().map(|runner| runner.sha256.as_str()) else {
        return Ok(None);
    };
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("owned application proxy receipt has an invalid digest");
    }
    if prior_runners
        .iter()
        .any(|runner| runner.sha256.as_str() != hash)
    {
        bail!("owned application proxies use inconsistent runtimes");
    }
    Ok(Some(hash))
}

fn check_global_alias(
    settings: &Path,
    destination: &Path,
    sidecar: &Path,
    owned: Option<&ApplicationLauncherOwnership>,
    expected_hash: &str,
    expected_sidecar: &[u8],
) -> Result<bool> {
    ensure_no_follow_walk(settings, destination, true)?;
    ensure_no_follow_walk(settings, sidecar, true)?;
    if path_present(destination)? {
        let Some(owned) = owned else {
            bail!(
                "refusing unowned public alias `{}` even if bytes match",
                destination.display()
            );
        };
        let actual = digest(&read_regular(destination)?);
        if owned.sha256 != actual || actual != expected_hash {
            bail!("owned public alias drifted");
        }
        if path_present(sidecar)? && read_regular(sidecar)? != expected_sidecar {
            bail!("public alias sidecar is not owned by this slot");
        }
        Ok(false)
    } else {
        if path_present(sidecar)? {
            bail!("public alias sidecar exists without its owned proxy");
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_runtime_survives_a_companion_upgrade() -> Result<()> {
        let root = tempfile::tempdir()?;
        let settings = fs::canonicalize(root.path())?;
        let project = Project::open(&settings)?;
        let retained_bytes = b"retained proxy runtime";
        let retained_hash = digest(retained_bytes);
        let retained = install_runtime(&project, &settings, retained_bytes, &retained_hash)?;
        managed_dir(&project, &["opt", "bin"])?;

        let existing = settings.join("opt/bin/editor");
        link_or_copy(&retained, &existing)?;
        let ownership = vec![ApplicationLauncherOwnership {
            destination: existing.clone(),
            sha256: retained_hash.clone(),
        }];

        let (runtime, selected_hash) = proxy_runtime(&project, &settings, &ownership)?;
        assert_eq!(runtime, retained);
        assert_eq!(selected_hash, retained_hash);

        let existing_sidecar = sidecar_path(&existing)?;
        assert!(!check_global_alias(
            &settings,
            &existing,
            &existing_sidecar,
            Some(&ownership[0]),
            &selected_hash,
            b"existing sidecar",
        )?);

        let added = settings.join("opt/bin/editor-helper");
        let added_sidecar = sidecar_path(&added)?;
        assert!(check_global_alias(
            &settings,
            &added,
            &added_sidecar,
            None,
            &selected_hash,
            b"added sidecar",
        )?);
        link_or_copy(&runtime, &added)?;
        assert_eq!(digest(&read_regular(&added)?), selected_hash);
        Ok(())
    }
}
