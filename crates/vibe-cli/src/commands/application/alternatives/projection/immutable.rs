//! Construction and validation of immutable alternative projection objects.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::{Value, json};
use vibe_safefs::{Project, ensure_no_follow_walk};

use super::descriptor::descriptor_for;
use super::files::{
    link_or_copy, path_present, proxy_filename, publish_immutable_directory, sidecar_path,
    write_new,
};
use super::{PROXY_PROTOCOL, digest, managed_dir, read_regular};
use crate::commands::application::model::{ApplicationIndex, ApplicationRecord};

pub(super) struct ViewPointer {
    pub(super) path: PathBuf,
    pub(super) projection: String,
}

pub(super) struct ViewContext<'a> {
    pub(super) project: &'a Project,
    pub(super) settings: &'a Path,
    pub(super) runtime: &'a Path,
    pub(super) proxy_hash: &'a str,
    pub(super) slot_dir: &'a Path,
    pub(super) slot: &'a str,
    pub(super) reserved: &'a [String],
}

pub(super) fn prepare_view(
    context: &ViewContext<'_>,
    key: &str,
    record: &ApplicationRecord,
) -> Result<ViewPointer> {
    let ViewContext {
        project,
        settings,
        runtime,
        proxy_hash,
        slot_dir,
        slot,
        reserved,
    } = *context;
    let encoded = digest(key.as_bytes());
    let installations = slot_dir.join("installations");
    managed_dir(project, &["opt", "alternatives", slot, "installations"])?;
    let view_root = installations.join(&encoded);
    let marker = view_root.join("owner.json");
    let owner =
        serde_json::to_vec(&json!({"protocol":"vibe-app-view/1","slot":slot,"installation":key}))?;
    let mut descriptors = BTreeMap::new();
    for name in reserved {
        descriptors.insert(name.clone(), descriptor_for(record, name, settings, key)?);
    }
    let projection = projection_id(slot, Some(key), &descriptors)?;

    publish_immutable_directory(&installations, &encoded, |staging| {
        let bin = staging.join("bin");
        let projections = staging.join("projections");
        let commands = projections.join(&projection).join("commands");
        fs::create_dir(&bin)?;
        fs::create_dir(&projections)?;
        fs::create_dir(projections.join(&projection))?;
        fs::create_dir(&commands)?;
        write_new(&staging.join("owner.json"), &owner)?;
        for name in reserved {
            let alias = bin.join(proxy_filename(name));
            let sidecar = sidecar_path(&alias)?;
            let bytes = view_sidecar_bytes(slot, key, name, &view_root)?;
            write_new(&sidecar, &bytes)?;
            link_or_copy(runtime, &alias)?;
        }
        for (name, descriptor) in &descriptors {
            write_new(
                &commands.join(format!("{name}.json")),
                &serde_json::to_vec(descriptor)?,
            )?;
        }
        Ok(())
    })?;

    ensure_no_follow_walk(settings, &marker, true)?;
    if read_regular(&marker)? != owner {
        bail!("application installation view is not owned by this slot");
    }
    managed_dir(
        project,
        &[
            "opt",
            "alternatives",
            slot,
            "installations",
            &encoded,
            "bin",
        ],
    )?;
    managed_dir(
        project,
        &[
            "opt",
            "alternatives",
            slot,
            "installations",
            &encoded,
            "projections",
        ],
    )?;
    for name in reserved {
        ensure_view_alias(runtime, proxy_hash, slot, key, name, &view_root)?;
    }
    publish_view_projection(settings, &view_root, &projection, &descriptors)?;
    Ok(ViewPointer {
        path: view_root.join("current.json"),
        projection,
    })
}

fn view_sidecar_bytes(slot: &str, key: &str, name: &str, view_root: &Path) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&json!({
        "protocol":PROXY_PROTOCOL, "mode":"view", "slot":slot,
        "installation":key, "command":name,
        "current":view_root.join("current.json"),
    }))?)
}

fn ensure_view_alias(
    runtime: &Path,
    proxy_hash: &str,
    slot: &str,
    key: &str,
    name: &str,
    view_root: &Path,
) -> Result<()> {
    let alias = view_root.join("bin").join(proxy_filename(name));
    let sidecar = sidecar_path(&alias)?;
    let bytes = view_sidecar_bytes(slot, key, name, view_root)?;
    if path_present(&alias)? && digest(&read_regular(&alias)?) != proxy_hash {
        bail!("application installation view proxy drifted");
    }
    if path_present(&sidecar)? {
        if read_regular(&sidecar)? != bytes {
            bail!("application installation view sidecar drifted");
        }
    } else {
        write_new(&sidecar, &bytes)?;
    }
    if !path_present(&alias)? {
        link_or_copy(runtime, &alias)?;
    }
    Ok(())
}

fn publish_view_projection(
    settings: &Path,
    view_root: &Path,
    projection: &str,
    descriptors: &BTreeMap<String, Value>,
) -> Result<()> {
    let projections = view_root.join("projections");
    let projection_root = projections.join(projection);
    publish_immutable_directory(&projections, projection, |staging| {
        let commands = staging.join("commands");
        fs::create_dir(&commands)?;
        for (name, descriptor) in descriptors {
            write_new(
                &commands.join(format!("{name}.json")),
                &serde_json::to_vec(descriptor)?,
            )?;
        }
        Ok(())
    })?;
    for (name, descriptor) in descriptors {
        let path = projection_root
            .join("commands")
            .join(format!("{name}.json"));
        ensure_no_follow_walk(settings, &path, true)?;
        if read_regular(&path)? != serde_json::to_vec(descriptor)? {
            bail!("immutable application view projection drifted");
        }
    }
    Ok(())
}

pub(super) fn projection_descriptors(
    index: &ApplicationIndex,
    settings: &Path,
    selected: Option<&str>,
    reserved: &[String],
) -> Result<BTreeMap<String, Value>> {
    let mut output = BTreeMap::new();
    let record = selected.and_then(|key| index.applications.get(key));
    if selected.is_some() && record.is_none() {
        bail!("selected application installation is absent from the index");
    }
    for name in reserved {
        let descriptor = match (record, selected) {
            (Some(record), Some(key)) => descriptor_for(record, name, settings, key)?,
            _ => {
                json!({"protocol":PROXY_PROTOCOL,"mode":"unavailable","reason":"no selected installation"})
            }
        };
        output.insert(name.clone(), descriptor);
    }
    Ok(output)
}

pub(super) fn projection_id(
    slot: &str,
    selected: Option<&str>,
    descriptors: &BTreeMap<String, Value>,
) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&(slot, selected, descriptors))?))
}

pub(super) fn publish_projection(
    project: &Project,
    settings: &Path,
    slot_dir: &Path,
    slot: &str,
    selected: Option<&str>,
    projection: &str,
    descriptors: &BTreeMap<String, Value>,
) -> Result<()> {
    let projections = slot_dir.join("projections");
    managed_dir(project, &["opt", "alternatives", slot, "projections"])?;
    let projection_root = projections.join(projection);
    let owner_path = projection_root.join("owner.json");
    let owner = serde_json::to_vec(&json!({
        "protocol":"vibe-app-projection/1", "slot":slot, "installation":selected,
    }))?;
    publish_immutable_directory(&projections, projection, |staging| {
        let commands = staging.join("commands");
        fs::create_dir(&commands)?;
        write_new(&staging.join("owner.json"), &owner)?;
        for (name, descriptor) in descriptors {
            let bytes = serde_json::to_vec(descriptor)?;
            write_new(&commands.join(format!("{name}.json")), &bytes)?;
        }
        Ok(())
    })?;
    ensure_no_follow_walk(settings, &owner_path, true)?;
    if read_regular(&owner_path)? != owner {
        bail!("immutable application projection owner drifted");
    }
    for (name, descriptor) in descriptors {
        let path = projection_root
            .join("commands")
            .join(format!("{name}.json"));
        ensure_no_follow_walk(settings, &path, true)?;
        let bytes = serde_json::to_vec(descriptor)?;
        if read_regular(&path)? != bytes {
            bail!("immutable application command projection drifted");
        }
    }
    Ok(())
}
