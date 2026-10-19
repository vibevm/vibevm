//! Recover a slot intent after an interrupted index/proxy publication.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;
use vibe_core::manifest::is_portable_token;
use vibe_safefs::ensure_no_follow_walk;

use super::files::{path_present, prior_pointer, remove_intent};
use super::{ApplicationIndex, INTENT_PROTOCOL, normalized_settings, prepare_slot_internal};

#[cfg(test)]
thread_local! {
    static TEST_PROXY: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn source_proxy() -> Result<PathBuf> {
    #[cfg(test)]
    if let Some(path) = TEST_PROXY.with(|proxy| proxy.borrow().clone()) {
        return Ok(path);
    }
    let exe = std::env::current_exe().context("finding running Vibe executable")?;
    let name = if cfg!(windows) {
        "vibe-app-proxy.exe"
    } else {
        "vibe-app-proxy"
    };
    let path = exe
        .parent()
        .context("Vibe executable has no parent")?
        .join(name);
    let metadata =
        fs::symlink_metadata(&path).context("finding adjacent vibe-app-proxy companion")?;
    if !metadata.file_type().is_file() {
        bail!("vibe-app-proxy companion is not a regular file");
    }
    Ok(path)
}

/// Reconcile a durable intent while the application mutation lock is held.
/// Unknown or conflicting state remains on disk for operator inspection.
pub(in crate::commands::application) fn recover_pending(
    settings: &Path,
    index: &ApplicationIndex,
) -> Result<()> {
    let settings = normalized_settings(settings)?;
    let intents = settings.join("applications/alternative-intents");
    ensure_no_follow_walk(&settings, &intents, true)?;
    let metadata = match fs::symlink_metadata(&intents) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).context("reading application alternative intent directory");
        }
    };
    if !metadata.file_type().is_dir() {
        bail!("application alternative intent path is not a directory");
    }

    let mut slots = BTreeSet::new();
    for entry in
        fs::read_dir(&intents).context("reading application alternative intent directory")?
    {
        let entry = entry.context("reading application alternative intent entry")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("application alternative intent name is not UTF-8"))?;
        let slot = name
            .strip_suffix(".json")
            .filter(|slot| is_portable_token(slot))
            .ok_or_else(|| anyhow::anyhow!("application alternative intent name is invalid"))?;
        let path = entry.path();
        ensure_no_follow_walk(&settings, &path, false)?;
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_file() || metadata.len() > 64 * 1024 {
            bail!("application alternative intent is not a bounded regular file");
        }
        slots.insert(slot.to_owned());
    }
    for slot in slots {
        recover_slot(&settings, &slot, index)?;
    }
    Ok(())
}

fn recover_slot(settings: &Path, slot: &str, index: &ApplicationIndex) -> Result<()> {
    if !is_portable_token(slot) {
        bail!("invalid alternative slot in recovery");
    }
    let settings = normalized_settings(settings)?;
    let path = settings
        .join("applications/alternative-intents")
        .join(format!("{slot}.json"));
    ensure_no_follow_walk(&settings, &path, true)?;
    if !path_present(&path)? {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() || metadata.len() > 64 * 1024 {
        bail!("application alternative intent is not a bounded regular file");
    }
    let bytes = fs::read(&path)?;
    let intent: Value =
        serde_json::from_slice(&bytes).context("parsing application alternative intent")?;
    if intent.get("protocol").and_then(Value::as_str) != Some(INTENT_PROTOCOL)
        || intent.get("slot").and_then(Value::as_str) != Some(slot)
    {
        bail!("application alternative intent has an invalid identity");
    }
    let prior = field(&intent, "prior_revision")?;
    let desired = field(&intent, "desired_revision")?;
    if prior == desired {
        bail!("application alternative intent does not distinguish old and new revisions");
    }
    let projection = field(&intent, "projection")?;
    valid_projection(projection)?;
    let views = intent
        .get("views")
        .and_then(Value::as_array)
        .context("application alternative intent has no view list")?;

    if index.revision == desired {
        let mut intended_index = index.clone();
        let mut prepared =
            prepare_slot_internal(&mut intended_index, &settings, slot, Some(bytes))?;
        if prepared.projection != projection || prepared.views.len() != views.len() {
            bail!("recomputed application projection differs from committed intent");
        }
        for (actual, saved) in prepared.views.iter().zip(views) {
            let path = view_path(saved, &settings, slot)?;
            let expected = field(saved, "projection")?;
            valid_projection(expected)?;
            if actual.path != path || actual.projection != expected {
                bail!("recomputed application view differs from committed intent");
            }
        }
        prepared.commit_current()?;
        prepared.commit()?;
        return Ok(());
    }
    if index.revision == prior {
        let current = settings
            .join("opt/alternatives")
            .join(slot)
            .join("current.json");
        ensure_no_follow_walk(&settings, &current, true)?;
        if serde_json::to_value(prior_pointer(&current)?)? != intent["prior_current"] {
            bail!("published application current pointer differs from prior intent state");
        }
        for saved in views {
            let path = view_path(saved, &settings, slot)?;
            ensure_no_follow_walk(&settings, &path, true)?;
            if serde_json::to_value(prior_pointer(&path)?)? != saved["prior"] {
                bail!("published application view pointer differs from prior intent state");
            }
        }
        remove_intent(&settings, &path, &bytes)?;
        return Ok(());
    }
    bail!("application alternative intent revision differs from durable index")
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("application alternative intent has no `{name}`"))
}

fn valid_projection(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("application alternative intent has an invalid projection id");
    }
    Ok(())
}

fn view_path(saved: &Value, settings: &Path, slot: &str) -> Result<PathBuf> {
    let path = PathBuf::from(field(saved, "current")?);
    let parent = settings
        .join("opt/alternatives")
        .join(slot)
        .join("installations");
    let relative = path
        .strip_prefix(&parent)
        .context("application alternative intent view escapes slot")?;
    let mut components = relative.components();
    let valid = matches!(components.next(), Some(Component::Normal(hash)) if
        hash.to_str().is_some_and(|text| valid_projection(text).is_ok()));
    if !valid
        || components.next() != Some(Component::Normal("current.json".as_ref()))
        || components.next().is_some()
    {
        bail!("application alternative intent view path is invalid");
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;

    use super::super::super::super::model::{
        AlternativeRecord, ApplicationIdentity, ApplicationRecord, ApplicationStatus,
        ManagementEntry, ManagementRuntime, PackageIdentity, PublicCommand, UserLauncher,
    };
    use super::super::prepare_slot;
    use super::{ApplicationIndex, TEST_PROXY, recover_pending, recover_slot};

    struct ProxyOverride;

    impl ProxyOverride {
        fn install(path: PathBuf) -> Self {
            TEST_PROXY.with(|proxy| {
                assert!(proxy.borrow_mut().replace(path).is_none());
            });
            Self
        }
    }

    impl Drop for ProxyOverride {
        fn drop(&mut self) {
            TEST_PROXY.with(|proxy| *proxy.borrow_mut() = None);
        }
    }

    fn record(settings: &std::path::Path, version: &str, ready: bool) -> ApplicationRecord {
        let package = PackageIdentity {
            group: "org.example".into(),
            name: "editor".into(),
            version: version.into(),
        };
        let host = settings
            .join("opt/apps/editor/org.example/editor")
            .join(version);
        let payload = host.join("payload");
        if ready {
            fs::create_dir_all(payload.join("bin")).unwrap();
            let executable = payload.join("bin/editor");
            fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        ApplicationRecord {
            application: ApplicationIdentity {
                id: "editor".into(),
                package: package.clone(),
                installer_package: package,
                commands: vec!["editor".into()],
            },
            host_root: host.clone(),
            app_root: ready.then_some(payload),
            install_kind: None,
            selected_option_id: None,
            selected_platform: None,
            management: ManagementEntry {
                runtime: ManagementRuntime::Native,
                entry: host.join("manage"),
            },
            status: if ready {
                ApplicationStatus::Ready
            } else {
                ApplicationStatus::Undeployed
            },
            provenance: None,
            launchers: Vec::new(),
            user_launchers: vec![UserLauncher {
                id: "main".into(),
                name: "Editor".into(),
                description: None,
                executable: "bin/editor".into(),
                args: Vec::new(),
                working_directory: None,
                icon: None,
                terminal: false,
                default: true,
            }],
            public_commands: if ready {
                vec![PublicCommand {
                    name: "editor".into(),
                    launcher: "main".into(),
                }]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn prior_revision_removes_only_an_unpublished_intent() {
        let root = tempfile::tempdir().unwrap();
        let intents = root.path().join("applications/alternative-intents");
        fs::create_dir_all(&intents).unwrap();
        fs::create_dir_all(root.path().join("opt/alternatives/editor")).unwrap();
        let path = intents.join("editor.json");
        fs::write(
            &path,
            serde_json::to_vec(&json!({
                "protocol":"vibe-app-alternative-intent/1", "slot":"editor",
                "prior_revision":"old", "desired_revision":"new",
                "prior_current":null, "projection":"a".repeat(64), "views":[],
            }))
            .unwrap(),
        )
        .unwrap();
        let mut index = ApplicationIndex {
            revision: "unrelated".into(),
            ..Default::default()
        };
        assert!(recover_slot(root.path(), "editor", &index).is_err());
        assert!(path.exists());
        index.revision = "old".into();
        let current = root.path().join("opt/alternatives/editor/current.json");
        fs::write(
            &current,
            serde_json::to_vec(&json!({
                "protocol":"vibe-app-current/1", "projection":"b".repeat(64),
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(recover_slot(root.path(), "editor", &index).is_err());
        assert!(path.exists());
        fs::remove_file(current).unwrap();
        recover_slot(root.path(), "editor", &index).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn pending_recovery_finds_an_intent_for_a_slot_absent_from_the_index() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("opt/bin")).unwrap();
        let fake_proxy = root.path().join("fake-vibe-app-proxy");
        fs::write(&fake_proxy, b"native proxy fixture").unwrap();
        let _override = ProxyOverride::install(fake_proxy);
        let mut intended = ApplicationIndex {
            revision: "desired".into(),
            ..Default::default()
        };
        let key = "org.example/editor@1.0.0";
        intended
            .applications
            .insert(key.into(), record(root.path(), "1.0.0", true));
        intended.alternatives.insert(
            "editor".into(),
            AlternativeRecord {
                global_default: Some(key.into()),
                reserved_commands: vec!["editor".into()],
                runners: Vec::new(),
            },
        );
        let prepared = prepare_slot(&mut intended, root.path(), "editor").unwrap();
        let path = root
            .path()
            .join("applications/alternative-intents/editor.json");
        assert!(path.is_file());
        drop(prepared);

        recover_pending(root.path(), &ApplicationIndex::default()).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn desired_revision_replays_partial_publication_and_is_idempotent() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("opt/bin")).unwrap();
        let fake_proxy = root.path().join("fake-vibe-app-proxy");
        fs::write(&fake_proxy, b"native proxy fixture").unwrap();
        let _override = ProxyOverride::install(fake_proxy);
        let mut index = ApplicationIndex {
            revision: "desired".into(),
            ..Default::default()
        };
        let ready_key = "org.example/editor@1.0.0";
        let removed_key = "org.example/editor@0.9.0";
        index
            .applications
            .insert(ready_key.into(), record(root.path(), "1.0.0", true));
        index
            .applications
            .insert(removed_key.into(), record(root.path(), "0.9.0", false));
        index.alternatives.insert(
            "editor".into(),
            AlternativeRecord {
                global_default: Some(ready_key.into()),
                reserved_commands: vec!["editor".into()],
                runners: Vec::new(),
            },
        );
        let prepared = prepare_slot(&mut index, root.path(), "editor").unwrap();
        let intent_path = root
            .path()
            .join("applications/alternative-intents/editor.json");
        let intent: serde_json::Value =
            serde_json::from_slice(&fs::read(&intent_path).unwrap()).unwrap();
        let views = intent["views"].as_array().unwrap();
        assert_eq!(views.len(), 2);
        let partial = &views[0];
        fs::write(
            partial["current"].as_str().unwrap(),
            serde_json::to_vec(&json!({
                "protocol":"vibe-app-current/1",
                "projection":partial["projection"],
            }))
            .unwrap(),
        )
        .unwrap();
        drop(prepared);

        let mut unrelated = index.clone();
        unrelated.revision = "third".into();
        assert!(recover_slot(root.path(), "editor", &unrelated).is_err());
        assert!(intent_path.exists());

        recover_slot(root.path(), "editor", &index).unwrap();
        assert!(!intent_path.exists());
        let global: serde_json::Value = serde_json::from_slice(
            &fs::read(root.path().join("opt/alternatives/editor/current.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(global["projection"], intent["projection"]);
        for view in views {
            let current: serde_json::Value =
                serde_json::from_slice(&fs::read(view["current"].as_str().unwrap()).unwrap())
                    .unwrap();
            assert_eq!(current["projection"], view["projection"]);
            let projection = view["projection"].as_str().unwrap();
            let root = std::path::Path::new(view["current"].as_str().unwrap())
                .parent()
                .unwrap();
            assert!(
                root.join("projections")
                    .join(projection)
                    .join("commands/editor.json")
                    .is_file()
            );
        }
        assert!(root.path().join("opt/bin/editor").is_file());
        assert!(root.path().join("opt/bin/editor.proxy.json").is_file());
        recover_slot(root.path(), "editor", &index).unwrap();
    }
}
