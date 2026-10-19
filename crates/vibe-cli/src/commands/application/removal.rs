//! Durable, crash-recoverable cleanup for built-in application uninstall.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_safefs::{Project, ensure_no_follow_walk};

use super::binary::BinaryUninstall;
use super::model::{
    ApplicationIndex, ApplicationLauncherOwnership, ApplicationRecord, ApplicationStatus,
    ManagementRuntime,
};
use super::store::installation_host_root;

const INTENT_PROTOCOL: &str = "vibe-application-removal-intent/1";
const MAX_INTENT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupLauncher {
    destination: PathBuf,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemovalIntent {
    protocol: String,
    installation_key: String,
    prior_revision: String,
    desired_revision: String,
    payload_root: PathBuf,
    launchers: Vec<CleanupLauncher>,
    force: bool,
}

struct PendingEntry {
    path: PathBuf,
    bytes: Vec<u8>,
    intent: RemovalIntent,
}

/// Loaded while the application mutation lock is held. The caller retains
/// exclusive launch leases for every key until projection recovery and cleanup
/// have both completed.
pub(super) struct PendingRemovals {
    settings: PathBuf,
    entries: Vec<PendingEntry>,
}

pub(super) struct PreparedRemoval {
    settings: PathBuf,
    entry: PendingEntry,
}

impl PreparedRemoval {
    pub(super) fn rollback(self) -> Result<()> {
        remove_intent(&self.settings, &self.entry.path, &self.entry.bytes)
    }

    pub(super) fn commit(self) -> Result<()> {
        cleanup(&self.entry.intent)?;
        remove_intent(&self.settings, &self.entry.path, &self.entry.bytes)
    }
}

impl PendingRemovals {
    pub(super) fn load(settings: &Path) -> Result<Self> {
        let directory = intent_directory(settings);
        ensure_no_follow_walk(settings, &directory, true)?;
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    settings: settings.to_path_buf(),
                    entries: Vec::new(),
                });
            }
            Err(error) => {
                return Err(error).context("reading application removal intent directory");
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("application removal intent path is not a directory");
        }
        let mut paths = Vec::new();
        for entry in
            fs::read_dir(&directory).context("reading application removal intent directory")?
        {
            let entry = entry.context("reading application removal intent entry")?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("application removal intent name is not UTF-8"))?;
            if !name.ends_with(".json") {
                bail!("application removal intent directory contains an unknown entry");
            }
            paths.push(entry.path());
        }
        paths.sort();
        let entries = paths
            .into_iter()
            .map(|path| read_intent(settings, path))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            settings: settings.to_path_buf(),
            entries,
        })
    }

    pub(super) fn installation_keys(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .map(|entry| entry.intent.installation_key.as_str())
    }

    pub(super) fn recover(self, index: &ApplicationIndex) -> Result<()> {
        for entry in self.entries {
            let intent = &entry.intent;
            if index.revision == intent.prior_revision {
                let record = matching_record(index, intent, ApplicationStatus::Ready)?;
                validate_against_record(&self.settings, intent, record)?;
                remove_intent(&self.settings, &entry.path, &entry.bytes)?;
                continue;
            }
            if index.revision == intent.desired_revision {
                let record = matching_record(index, intent, ApplicationStatus::Undeployed)?;
                validate_against_record(&self.settings, intent, record)?;
                cleanup(intent)?;
                remove_intent(&self.settings, &entry.path, &entry.bytes)?;
                continue;
            }
            bail!("application removal intent revision differs from the durable index");
        }
        Ok(())
    }
}

pub(super) fn prepare(
    settings: &Path,
    installation_key: &str,
    prior_revision: &str,
    desired_revision: &str,
    record: &ApplicationRecord,
    removal: &BinaryUninstall,
) -> Result<PreparedRemoval> {
    if prior_revision.is_empty()
        || desired_revision.is_empty()
        || prior_revision == desired_revision
        || record.status != ApplicationStatus::Ready
    {
        bail!("application removal intent has invalid commit revisions");
    }
    let intent = RemovalIntent {
        protocol: INTENT_PROTOCOL.into(),
        installation_key: installation_key.into(),
        prior_revision: prior_revision.into(),
        desired_revision: desired_revision.into(),
        payload_root: removal.payload_root.clone(),
        launchers: removal
            .launchers
            .iter()
            .map(|launcher| CleanupLauncher {
                destination: launcher.destination.clone(),
                sha256: launcher.sha256.clone(),
            })
            .collect(),
        force: removal.force,
    };
    validate_against_record(settings, &intent, record)?;
    let bytes =
        serde_json::to_vec_pretty(&intent).context("serializing application removal intent")?;
    let name = intent_filename(installation_key);
    let directory = intent_directory(settings);
    let path = directory.join(&name);
    let project = Project::open(settings).context("opening application settings for removal")?;
    let pinned = project
        .dir(&["applications", "removal-intents"], true)
        .context("opening application removal intent directory")?;
    if let Some(existing) = project
        .read_file_in(&pinned, &name)
        .context("checking existing application removal intent")?
    {
        if existing != bytes {
            bail!("a different application removal intent is already pending");
        }
    } else if let Err(error) = project.write_atomic_durable_in(&pinned, &name, &bytes) {
        let cause = error.into_report();
        if project
            .read_file_in(&pinned, &name)
            .ok()
            .flatten()
            .as_deref()
            == Some(bytes.as_slice())
        {
            let _ = remove_intent(settings, &path, &bytes);
        }
        return Err(cause).context("publishing durable application removal intent");
    }
    Ok(PreparedRemoval {
        settings: settings.to_path_buf(),
        entry: PendingEntry {
            path,
            bytes,
            intent,
        },
    })
}

fn matching_record<'a>(
    index: &'a ApplicationIndex,
    intent: &RemovalIntent,
    status: ApplicationStatus,
) -> Result<&'a ApplicationRecord> {
    let record = index
        .applications
        .get(&intent.installation_key)
        .ok_or_else(|| {
            anyhow::anyhow!("application removal intent names an absent installation")
        })?;
    if record.status != status {
        bail!("application removal intent disagrees with installation status");
    }
    Ok(record)
}

fn validate_against_record(
    settings: &Path,
    intent: &RemovalIntent,
    record: &ApplicationRecord,
) -> Result<()> {
    if intent.protocol != INTENT_PROTOCOL
        || intent.prior_revision.is_empty()
        || intent.desired_revision.is_empty()
        || intent.prior_revision == intent.desired_revision
        || record.management.runtime != ManagementRuntime::Builtin
    {
        bail!("application removal intent has invalid identity");
    }
    let expected_host = installation_host_root(
        settings,
        &record.application.id,
        &record.application.package,
    );
    if record.host_root != expected_host
        || intent.payload_root.parent() != Some(record.host_root.join("generations").as_path())
        || record.app_root.as_ref() != Some(&intent.payload_root)
    {
        bail!("application removal intent names an invalid payload");
    }
    let launchers: Vec<_> = intent
        .launchers
        .iter()
        .map(|launcher| ApplicationLauncherOwnership {
            destination: launcher.destination.clone(),
            sha256: launcher.sha256.clone(),
        })
        .collect();
    if launchers != record.launchers {
        bail!("application removal intent disagrees with receipt-owned launchers");
    }
    Ok(())
}

fn cleanup(intent: &RemovalIntent) -> Result<()> {
    BinaryUninstall {
        launchers: intent
            .launchers
            .iter()
            .map(|launcher| ApplicationLauncherOwnership {
                destination: launcher.destination.clone(),
                sha256: launcher.sha256.clone(),
            })
            .collect(),
        payload_root: intent.payload_root.clone(),
        force: intent.force,
    }
    .cleanup()
}

fn read_intent(settings: &Path, path: PathBuf) -> Result<PendingEntry> {
    ensure_no_follow_walk(settings, &path, false)?;
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_INTENT_BYTES
    {
        bail!("application removal intent is not a bounded regular file");
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(&path)?
        .take(MAX_INTENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INTENT_BYTES {
        bail!("application removal intent exceeds its size limit");
    }
    let intent: RemovalIntent =
        serde_json::from_slice(&bytes).context("parsing application removal intent")?;
    let expected = intent_filename(&intent.installation_key);
    if path.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
        bail!("application removal intent name does not match its installation");
    }
    Ok(PendingEntry {
        path,
        bytes,
        intent,
    })
}

fn remove_intent(settings: &Path, path: &Path, expected: &[u8]) -> Result<()> {
    ensure_no_follow_walk(settings, path, true)?;
    match fs::read(path) {
        Ok(current) if current == expected => {}
        Ok(_) => bail!("application removal intent changed during reconciliation"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("reading application removal intent for deletion"),
    }
    fs::remove_file(path).context("removing completed application removal intent")?;
    sync_directory(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("application removal intent has no parent"))?,
    )
}

fn intent_directory(settings: &Path) -> PathBuf {
    settings.join("applications").join("removal-intents")
}

fn intent_filename(installation_key: &str) -> String {
    format!("{:x}.json", Sha256::digest(installation_key.as_bytes()))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .with_context(|| format!("syncing application removal directory `{}`", path.display()))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::application::model::{
        AlternativeRecord, ApplicationIdentity, PackageIdentity,
    };

    const KEY: &str = "org.example/demo@1.0.0";

    fn record(settings: &Path, status: ApplicationStatus) -> ApplicationRecord {
        let package = PackageIdentity {
            group: "org.example".into(),
            name: "demo".into(),
            version: "1.0.0".into(),
        };
        let host_root = installation_host_root(settings, "demo", &package);
        let payload = host_root.join("generations").join("a".repeat(64));
        let launcher = settings.join("opt/bin/demo");
        ApplicationRecord {
            application: ApplicationIdentity {
                id: "demo".into(),
                package: package.clone(),
                installer_package: package,
                commands: vec!["demo".into()],
            },
            host_root: host_root.clone(),
            app_root: Some(payload),
            install_kind: None,
            selected_option_id: None,
            selected_platform: None,
            management: super::super::model::ManagementEntry {
                runtime: ManagementRuntime::Builtin,
                entry: host_root.join("management/binary.json"),
            },
            status,
            provenance: None,
            launchers: vec![ApplicationLauncherOwnership {
                destination: launcher,
                sha256: format!("{:x}", Sha256::digest(b"runner")),
            }],
            user_launchers: Vec::new(),
            public_commands: Vec::new(),
        }
    }

    fn index(settings: &Path, revision: &str, status: ApplicationStatus) -> ApplicationIndex {
        let ready = status == ApplicationStatus::Ready;
        let mut index = ApplicationIndex {
            revision: revision.into(),
            ..ApplicationIndex::default()
        };
        index
            .applications
            .insert(KEY.into(), record(settings, status));
        index.alternatives.insert(
            "demo".into(),
            AlternativeRecord {
                global_default: ready.then(|| KEY.into()),
                reserved_commands: vec!["demo".into()],
                runners: Vec::new(),
            },
        );
        index
    }

    fn removal(record: &ApplicationRecord) -> BinaryUninstall {
        BinaryUninstall {
            launchers: record.launchers.clone(),
            payload_root: record.app_root.clone().unwrap(),
            force: false,
        }
    }

    fn create_targets(record: &ApplicationRecord) {
        fs::create_dir_all(record.app_root.as_ref().unwrap().join("nested")).unwrap();
        fs::write(
            record.app_root.as_ref().unwrap().join("nested/file"),
            b"payload",
        )
        .unwrap();
        fs::create_dir_all(record.launchers[0].destination.parent().unwrap()).unwrap();
        fs::write(&record.launchers[0].destination, b"runner").unwrap();
    }

    #[test]
    fn prior_revision_discards_intent_without_touching_payload() {
        let settings = tempfile::tempdir().unwrap();
        let before = index(settings.path(), "before", ApplicationStatus::Ready);
        let record = before.applications.get(KEY).unwrap();
        create_targets(record);
        let prepared = prepare(
            settings.path(),
            KEY,
            "before",
            "after",
            record,
            &removal(record),
        )
        .unwrap();
        assert!(prepared.entry.path.is_file());
        PendingRemovals::load(settings.path())
            .unwrap()
            .recover(&before)
            .unwrap();
        assert!(record.app_root.as_ref().unwrap().is_dir());
        assert!(record.launchers[0].destination.is_file());
        assert!(!prepared.entry.path.exists());
    }

    #[test]
    fn desired_revision_finishes_partial_cleanup_idempotently() {
        let settings = tempfile::tempdir().unwrap();
        let before = index(settings.path(), "before", ApplicationStatus::Ready);
        let record = before.applications.get(KEY).unwrap();
        create_targets(record);
        let prepared = prepare(
            settings.path(),
            KEY,
            "before",
            "after",
            record,
            &removal(record),
        )
        .unwrap();
        fs::remove_file(&record.launchers[0].destination).unwrap();
        fs::remove_file(record.app_root.as_ref().unwrap().join("nested/file")).unwrap();
        let after = index(settings.path(), "after", ApplicationStatus::Undeployed);
        PendingRemovals::load(settings.path())
            .unwrap()
            .recover(&after)
            .unwrap();
        assert!(!record.app_root.as_ref().unwrap().exists());
        assert!(!prepared.entry.path.exists());
    }

    #[test]
    fn drift_after_validation_keeps_cleanup_intent_for_retry() {
        let settings = tempfile::tempdir().unwrap();
        let before = index(settings.path(), "before", ApplicationStatus::Ready);
        let record = before.applications.get(KEY).unwrap();
        create_targets(record);
        let prepared = prepare(
            settings.path(),
            KEY,
            "before",
            "after",
            record,
            &removal(record),
        )
        .unwrap();
        fs::write(&record.launchers[0].destination, b"foreign").unwrap();
        let after = index(settings.path(), "after", ApplicationStatus::Undeployed);
        let error = PendingRemovals::load(settings.path())
            .unwrap()
            .recover(&after)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("drifted after uninstall validation"),
            "{error}"
        );
        assert!(prepared.entry.path.is_file());
        assert!(record.app_root.as_ref().unwrap().is_dir());
    }

    #[test]
    fn uncertain_index_commit_keeps_payload_and_intent_for_recovery() {
        use crate::commands::application::store::{ApplicationStore, UncertainIndexCommit};

        let settings = tempfile::tempdir().unwrap();
        let store = ApplicationStore::open(settings.path()).unwrap();
        let before = index(settings.path(), "before", ApplicationStatus::Ready);
        let record = before.applications.get(KEY).unwrap();
        create_targets(record);
        store.save(&before).unwrap();
        let prepared = prepare(
            settings.path(),
            KEY,
            "before",
            "after",
            record,
            &removal(record),
        )
        .unwrap();
        let after = index(settings.path(), "after", ApplicationStatus::Undeployed);

        vibe_safefs::fail_after_publish(Some("index.json"));
        let result = store.save(&after);
        vibe_safefs::fail_after_publish(None);
        assert!(
            result
                .unwrap_err()
                .downcast_ref::<UncertainIndexCommit>()
                .is_some()
        );
        assert!(record.app_root.as_ref().unwrap().is_dir());
        assert!(record.launchers[0].destination.is_file());
        assert!(prepared.entry.path.is_file());

        let durable = store.load().unwrap();
        PendingRemovals::load(settings.path())
            .unwrap()
            .recover(&durable)
            .unwrap();
        assert!(!record.app_root.as_ref().unwrap().exists());
        assert!(!record.launchers[0].destination.exists());
        assert!(!prepared.entry.path.exists());
    }

    #[test]
    fn rejected_index_save_rolls_back_intent_without_touching_payload() {
        use crate::commands::application::store::ApplicationStore;

        let settings = tempfile::tempdir().unwrap();
        let store = ApplicationStore::open(settings.path()).unwrap();
        let before = index(settings.path(), "before", ApplicationStatus::Ready);
        let record = before.applications.get(KEY).unwrap();
        create_targets(record);
        store.save(&before).unwrap();
        let prepared = prepare(
            settings.path(),
            KEY,
            "before",
            "after",
            record,
            &removal(record),
        )
        .unwrap();
        let intent_path = prepared.entry.path.clone();
        let mut rejected = index(settings.path(), "after", ApplicationStatus::Undeployed);
        rejected.protocol = "unsupported-index-protocol".into();

        let error = store.save(&rejected).unwrap_err();
        assert!(error.to_string().contains("unsupported protocol"));
        prepared.rollback().unwrap();
        assert_eq!(store.load().unwrap().revision, "before");
        assert!(record.app_root.as_ref().unwrap().is_dir());
        assert!(record.launchers[0].destination.is_file());
        assert!(!intent_path.exists());
    }
}
