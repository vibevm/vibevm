//! Serialized durable user-application index.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::is_portable_token;
use vibe_safefs::{Pinned, Project, ensure_no_follow_walk};

use super::model::{ApplicationIndex, INDEX_PROTOCOL};

pub struct ApplicationStore {
    root: PathBuf,
    settings_root: PathBuf,
    index_path: PathBuf,
    _settings: Project,
    applications: Pinned,
    _lock: File,
}

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
            _settings: settings,
            applications,
            _lock: lock,
        })
    }

    pub fn load(&self) -> Result<ApplicationIndex> {
        if !self.index_path.exists() {
            return Ok(ApplicationIndex::default());
        }
        ensure_no_follow_walk(&self.settings_root, &self.index_path, false)?;
        let bytes = fs::read(&self.index_path).context("reading the application index")?;
        let index: ApplicationIndex =
            serde_json::from_slice(&bytes).context("parsing the application index")?;
        if index.protocol != INDEX_PROTOCOL {
            bail!("application index protocol is unsupported");
        }
        validate_index(&index, &self.settings_root)?;
        Ok(index)
    }

    pub fn save(&self, index: &ApplicationIndex) -> Result<()> {
        if index.protocol != INDEX_PROTOCOL {
            bail!("refusing an application index with an unsupported protocol");
        }
        validate_index(index, &self.settings_root)?;
        ensure_no_follow_walk(&self.settings_root, &self.root, false)?;
        let bytes =
            serde_json::to_vec_pretty(index).context("serializing the application index")?;
        let temporary = self
            .root
            .join(format!(".index-{}.json", std::process::id()));
        let backup = self
            .root
            .join(format!(".index-{}.previous", std::process::id()));
        for path in [&self.index_path, &temporary, &backup] {
            ensure_no_follow_walk(&self.settings_root, path, true)?;
        }
        remove_file_if_present(&temporary)?;
        remove_file_if_present(&backup)?;
        {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .context("creating the staged application index")?;
            file.write_all(&bytes)
                .context("writing the staged application index")?;
            file.write_all(b"\n")
                .context("terminating the staged application index")?;
            file.sync_all()
                .context("syncing the staged application index")?;
        }
        let had_prior = self.index_path.exists();
        if had_prior {
            fs::rename(&self.index_path, &backup)
                .context("retaining the prior application index")?;
        }
        if let Err(error) = fs::rename(&temporary, &self.index_path) {
            if had_prior {
                let _ = fs::rename(&backup, &self.index_path);
            }
            return Err(error).context("publishing the application index");
        }
        remove_file_if_present(&backup)?;
        Ok(())
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

fn validate_index(index: &ApplicationIndex, settings_root: &Path) -> Result<()> {
    let mut coordinates = std::collections::BTreeSet::new();
    for (key, record) in &index.applications {
        let application = &record.application;
        if key != &application.id || !is_portable_token(key) {
            bail!("application index key differs from a portable application id");
        }
        if application.commands.is_empty() {
            bail!("application index carries no advertised commands");
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
        let coordinate = format!("{}/{}", application.package.group, application.package.name);
        if !coordinates.insert(coordinate) {
            bail!("application index binds one package coordinate more than once");
        }
        let expected_host = settings_root.join("opt").join("apps").join(key);
        if record.host_root != expected_host
            || !record.management.entry.is_absolute()
            || !record.management.entry.starts_with(&expected_host)
            || record.management.entry == expected_host
        {
            bail!("application index carries a path outside its owned application host");
        }
    }
    Ok(())
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("removing `{}`", path.display())),
    }
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
