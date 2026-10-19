//! Serialized durable user-application index.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::fs::{self, File, OpenOptions};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::is_portable_token;
use vibe_safefs::{Pinned, Project, PublishStage, ensure_no_follow_walk};

use super::model::launcher_belongs_to_command;
use super::model::{ApplicationIndex, INDEX_PROTOCOL, PackageIdentity};

/// Exact key for one installed package version, independent of its slot.
pub(super) fn installation_key(package: &PackageIdentity) -> String {
    format!("{}/{}@{}", package.group, package.name, package.version)
}

/// A provider's private installation host within the shared application slot.
pub(super) fn installation_host_root(
    settings_root: &Path,
    slot: &str,
    package: &PackageIdentity,
) -> PathBuf {
    settings_root
        .join("opt")
        .join("apps")
        .join(slot)
        .join(&package.group)
        .join(&package.name)
        .join(&package.version)
}

pub struct ApplicationStore {
    root: PathBuf,
    settings_root: PathBuf,
    index_path: PathBuf,
    settings: Project,
    applications: Pinned,
    _lock: File,
}

/// Publication is proven absent; callers may roll back their staged payloads.
#[derive(Debug)]
pub(super) struct NotCommittedIndexCommit {
    pub intended_revision: String,
    pub cause: anyhow::Error,
}

impl std::fmt::Display for NotCommittedIndexCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "application index revision `{}` was not committed: {:#}",
            self.intended_revision, self.cause
        )
    }
}

impl std::error::Error for NotCommittedIndexCommit {}

/// Publication could not be classified; callers must preserve staged state for reconciliation.
#[derive(Debug)]
pub(super) struct UncertainIndexCommit {
    pub intended_revision: String,
    pub observed_revision: Option<String>,
    pub cause: anyhow::Error,
}

impl std::fmt::Display for UncertainIndexCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "application index revision `{}` has uncertain commit state (observed revision: {:?}): {:#}",
            self.intended_revision, self.observed_revision, self.cause
        )
    }
}

impl std::error::Error for UncertainIndexCommit {}

impl ApplicationStore {
    pub fn open(settings_root: &Path) -> Result<Self> {
        fs::create_dir_all(settings_root)
            .with_context(|| format!("creating settings root `{}`", settings_root.display()))?;
        let settings_root = crate::commands::init::strip_unc_public(
            fs::canonicalize(settings_root)
                .context("resolving the user application settings root")?,
        );
        let settings = Project::open(&settings_root)
            .context("opening the no-follow application settings root")?;
        let applications = settings
            .dir(&["applications"], true)
            .context("opening the no-follow application state directory")?;
        let root = settings_root.join("applications");
        ensure_no_follow_walk(&settings_root, &root, false)?;
        let lock_path = root.join(".mutation.lock");
        ensure_no_follow_walk(&settings_root, &lock_path, true)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .context("opening the application mutation lock")?;
        lock.try_lock().map_err(|error| {
            anyhow::anyhow!("another global application mutation is active: {error}")
        })?;
        Ok(Self {
            index_path: root.join("index.json"),
            root,
            settings_root,
            settings,
            applications,
            _lock: lock,
        })
    }

    pub fn load(&self) -> Result<ApplicationIndex> {
        read_index(&self.settings_root, &self.index_path)
    }

    /// Read a stable index snapshot without taking the mutation lock or creating state.
    pub fn load_snapshot(settings_root: &Path) -> Result<ApplicationIndex> {
        if !settings_root.exists() {
            return Ok(ApplicationIndex::default());
        }
        let settings_root = crate::commands::init::strip_unc_public(
            fs::canonicalize(settings_root)
                .context("resolving the user application settings root")?,
        );
        let _settings = Project::open(&settings_root)
            .context("opening the no-follow application settings root")?;
        read_index(
            &settings_root,
            &settings_root.join("applications").join("index.json"),
        )
    }

    pub fn save(&self, index: &ApplicationIndex) -> Result<()> {
        if index.protocol != INDEX_PROTOCOL {
            bail!("refusing an application index with an unsupported protocol");
        }
        if index.revision.is_empty() {
            bail!("refusing an application index without a commit revision");
        }
        validate_index(index, &self.settings_root)?;
        ensure_no_follow_walk(&self.settings_root, &self.root, false)?;
        let bytes = serde_json::to_vec_pretty(&index.to_wire())
            .context("serializing the application index")?;
        let prior = self
            .settings
            .read_file_in(&self.applications, "index.json")
            .context("reading the prior application index before publication")?;
        if let Some(prior_bytes) = prior.as_deref() {
            let prior_wire: vibe_wire::generated::application::e1::index::ApplicationIndex =
                serde_json::from_slice(prior_bytes)
                    .context("parsing the prior application index")?;
            let prior_index = ApplicationIndex::from_wire(prior_wire)
                .context("decoding the prior application index")?;
            for (slot, previous) in &prior_index.alternatives {
                let Some(next) = index.alternatives.get(slot) else {
                    bail!("application slot reservation cannot disappear from the index");
                };
                if previous
                    .reserved_commands
                    .iter()
                    .any(|command| !next.reserved_commands.contains(command))
                {
                    bail!("application slot cannot release a reserved public command");
                }
            }
        }
        if prior.as_deref().and_then(observed_revision).as_deref() == Some(index.revision.as_str())
            && prior.as_deref() != Some(bytes.as_slice())
        {
            bail!("application index commit revision was already used for different bytes");
        }
        match self
            .settings
            .write_atomic_durable_in(&self.applications, "index.json", &bytes)
        {
            Ok(_) => Ok(()),
            Err(error) => {
                let stage = error.stage;
                let cause = error.into_report();
                if stage == PublishStage::BeforePublication {
                    return Err(NotCommittedIndexCommit {
                        intended_revision: index.revision.clone(),
                        cause,
                    }
                    .into());
                }
                match self.settings.read_file_in(&self.applications, "index.json") {
                    Ok(Some(observed)) if observed == bytes => Err(UncertainIndexCommit {
                        intended_revision: index.revision.clone(),
                        observed_revision: Some(index.revision.clone()),
                        cause: cause.context(
                            "new index bytes are visible, but publication durability was not confirmed",
                        ),
                    }
                    .into()),
                    Ok(observed) if observed == prior => Err(NotCommittedIndexCommit {
                        intended_revision: index.revision.clone(),
                        cause,
                    }
                    .into()),
                    Ok(observed) => Err(UncertainIndexCommit {
                        intended_revision: index.revision.clone(),
                        observed_revision: observed.as_deref().and_then(observed_revision),
                        cause,
                    }
                    .into()),
                    Err(read_error) => Err(UncertainIndexCommit {
                        intended_revision: index.revision.clone(),
                        observed_revision: None,
                        cause: anyhow::anyhow!(
                            "{cause:#}; unable to classify visible index: {read_error:#}"
                        ),
                    }
                    .into()),
                }
            }
        }
    }

    pub fn request_paths(&self) -> Result<(PathBuf, PathBuf)> {
        let requests = self.root.join("requests");
        self.applications
            .ensure_child("requests")
            .context("opening the no-follow application request state")?;
        ensure_no_follow_walk(&self.settings_root, &requests, false)?;
        let nonce = format!("{}-{}", std::process::id(), monotonic_nonce());
        Ok((
            requests.join(format!("context-{nonce}.json")),
            requests.join(format!("reply-{nonce}.json")),
        ))
    }
}

fn observed_revision(bytes: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()?
        .get("revision")?
        .as_str()
        .map(ToOwned::to_owned)
}

fn read_index(settings_root: &Path, index_path: &Path) -> Result<ApplicationIndex> {
    if !index_path.exists() {
        return Ok(ApplicationIndex::default());
    }
    ensure_no_follow_walk(settings_root, index_path, false)?;
    let bytes = fs::read(index_path).context("reading the application index")?;
    let wire: vibe_wire::generated::application::e1::index::ApplicationIndex =
        serde_json::from_slice(&bytes).context("parsing the application index")?;
    let index =
        ApplicationIndex::from_wire(wire).context("parsing the application index selection")?;
    if index.protocol != INDEX_PROTOCOL {
        bail!("application index protocol is unsupported");
    }
    if index.revision.is_empty() {
        bail!("application index has no commit revision");
    }
    validate_index(&index, settings_root)?;
    Ok(index)
}

fn validate_index(index: &ApplicationIndex, settings_root: &Path) -> Result<()> {
    let mut slots = std::collections::BTreeSet::new();
    let mut public_names = std::collections::BTreeMap::<String, String>::new();
    let mut all_runner_paths = std::collections::BTreeSet::new();
    let launcher_root = settings_root.join("opt").join("bin");
    for (key, record) in &index.applications {
        let application = &record.application;
        if !is_portable_token(&application.id) {
            bail!("application index carries an invalid slot id");
        }
        if record.status == super::model::ApplicationStatus::Ready {
            slots.insert(application.id.as_str());
            if record.app_root.is_none() {
                bail!("ready application installation has no app root");
            }
        }
        if application.commands.is_empty() && record.user_launchers.is_empty() {
            bail!("application index carries no advertised commands or user launchers");
        }
        let mut commands = std::collections::BTreeSet::new();
        for command in &application.commands {
            if !is_portable_token(command) || !commands.insert(command) {
                bail!("application index carries an invalid or duplicate command");
            }
        }
        for package in [&application.package, &application.installer_package] {
            vibe_core::Group::parse(&package.group)?;
            vibe_core::PackageName::parse(&package.name)?;
            semver::Version::parse(&package.version)
                .context("application index carries an invalid package version")?;
        }
        if key != &installation_key(&application.package) {
            bail!("application index key differs from the provider package coordinate");
        }
        let expected_host =
            installation_host_root(settings_root, &application.id, &application.package);
        if record.host_root != expected_host
            || !record.management.entry.is_absolute()
            || !record.management.entry.starts_with(&expected_host)
            || record.management.entry == expected_host
        {
            bail!("application index carries a path outside its owned application host");
        }
        if let Some(app_root) = &record.app_root
            && (!app_root.is_absolute()
                || app_root
                    .components()
                    .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
                || app_root.as_os_str().is_empty())
        {
            bail!("application index carries an invalid application payload root");
        }
        match (&record.selected_option_id, &record.selected_platform) {
            (None, None) => {}
            (Some(option_id), Some(platform)) => {
                let valid_platform = match platform.split_once('/') {
                    Some((os, arch)) => {
                        is_portable_token(os)
                            && !arch.is_empty()
                            && arch.bytes().all(|byte| {
                                byte.is_ascii_lowercase()
                                    || byte.is_ascii_digit()
                                    || matches!(byte, b'_' | b'-')
                            })
                    }
                    None => is_portable_token(platform),
                };
                if !is_portable_token(option_id) || !valid_platform {
                    bail!("application index carries an invalid selected installer option");
                }
            }
            _ => bail!("application index carries an incomplete selected installer option"),
        }
        let mut user_launcher_ids = std::collections::BTreeSet::new();
        let mut has_default = false;
        for launcher in &record.user_launchers {
            if !is_portable_token(&launcher.id)
                || !user_launcher_ids.insert(&launcher.id)
                || launcher.name.trim().is_empty()
                || launcher.executable.trim().is_empty()
            {
                bail!("application index carries an invalid user launcher declaration");
            }
            if launcher.default {
                if has_default {
                    bail!("application index carries multiple default user launchers");
                }
                has_default = true;
            }
        }
        let mut provider_public_names = std::collections::BTreeSet::new();
        for command in &record.public_commands {
            if !is_portable_token(&command.name)
                || !user_launcher_ids.contains(&command.launcher)
                || !provider_public_names.insert(command.name.clone())
            {
                bail!("application index carries an invalid public command binding");
            }
        }
        let mut launcher_destinations = std::collections::BTreeSet::new();
        for launcher in &record.launchers {
            let Some(name) = launcher
                .destination
                .file_name()
                .and_then(|value| value.to_str())
            else {
                bail!("application index carries an invalid launcher destination");
            };
            if launcher.destination.parent() != Some(launcher_root.as_path())
                || !launcher_belongs_to_command(name, &application.commands)
                || !launcher_destinations.insert(launcher.destination.clone())
                || !all_runner_paths.insert(launcher.destination.clone())
                || launcher.sha256.len() != 64
                || !launcher
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                bail!("application index carries invalid launcher ownership");
            }
        }
    }
    for (slot, alternative) in &index.alternatives {
        if !is_portable_token(slot) {
            bail!("application alternatives carry an invalid slot id");
        }
        let mut reserved = std::collections::BTreeSet::new();
        for name in &alternative.reserved_commands {
            if !is_portable_token(name) || !reserved.insert(name.clone()) {
                bail!("application alternatives carry invalid or duplicate reserved commands");
            }
            if let Some(owner) = public_names.insert(name.clone(), slot.clone())
                && owner != *slot
            {
                bail!("application index reserves one public command for multiple slots");
            }
        }
        let slot_commands: Vec<String> = index
            .applications
            .values()
            .filter(|record| {
                record.status == super::model::ApplicationStatus::Ready
                    && record.application.id == *slot
            })
            .flat_map(|record| {
                record
                    .public_commands
                    .iter()
                    .map(|command| command.name.clone())
            })
            .collect();
        let has_ready = index.applications.values().any(|record| {
            record.status == super::model::ApplicationStatus::Ready
                && record.application.id == *slot
        });
        match &alternative.global_default {
            Some(key) => {
                let Some(selected) = index.applications.get(key) else {
                    bail!("application alternatives select an absent installation");
                };
                if selected.application.id != *slot
                    || selected.status != super::model::ApplicationStatus::Ready
                {
                    bail!(
                        "application alternatives select an installation that is not ready in this slot"
                    );
                }
            }
            None if has_ready => bail!("ready application slot has no global default"),
            None => {}
        }
        for command in &slot_commands {
            if !reserved.contains(command) {
                bail!("ready application command is absent from reserved slot names");
            }
        }
        let mut runner_paths = std::collections::BTreeSet::new();
        for runner in &alternative.runners {
            let Some(name) = runner
                .destination
                .file_name()
                .and_then(|value| value.to_str())
            else {
                bail!("application alternatives carry an invalid public runner destination");
            };
            if runner.destination.parent() != Some(launcher_root.as_path())
                || !launcher_belongs_to_command(name, &alternative.reserved_commands)
                || !runner_paths.insert(runner.destination.clone())
                || !all_runner_paths.insert(runner.destination.clone())
                || !valid_sha256(&runner.sha256)
            {
                bail!("application alternatives carry invalid public runner ownership");
            }
        }
        for command in &slot_commands {
            if !alternative.runners.iter().any(|runner| {
                runner
                    .destination
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| {
                        launcher_belongs_to_command(name, std::slice::from_ref(command))
                    })
            }) {
                bail!("application alternatives are missing an owned public command runner");
            }
        }
    }
    for slot in slots {
        if !index.alternatives.contains_key(slot) {
            bail!("application slot has no global default alternative");
        }
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn monotonic_nonce() -> u128 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_nanos());
    time ^ u128::from(NEXT.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests;
