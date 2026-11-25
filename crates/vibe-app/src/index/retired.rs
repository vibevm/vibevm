//! Recognition only, never conversion, of the retired dispatcher inventory.
//!
//! The closed /1 model and its writer/validation are verified at 53f1521a5,
//! `crates/vibe-cli/src/commands/application/{model,store}.rs`. The later
//! pre-replacement writer at f2c2e7a22^ used a different /2 schema; it is
//! deliberately NOT admitted by this module.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use super::{Index, SettingsRoot, StateError, io};

pub(super) const PROTOCOL: &str = "vibe-user-applications/1";
const BACKUP: &str = "index.retired-v1.json";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    protocol: String,
    #[serde(deserialize_with = "unique_applications")]
    applications: BTreeMap<String, Record>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    application: Identity,
    host_root: PathBuf,
    management: Management,
    status: Status,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Identity {
    id: String,
    package: Package,
    installer_package: Package,
    commands: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Package {
    group: String,
    name: String,
    version: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Management {
    runtime: Runtime,
    entry: PathBuf,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Runtime {
    Node,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Ready,
    Undeployed,
    Failed,
}

// A map deserializer normally keeps only the last duplicate key. Recognition
// must validate every supplied record instead of hiding corrupt earlier data.
fn unique_applications<'de, D>(deserializer: D) -> Result<BTreeMap<String, Record>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, Record>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a map of unique retired application ids")
        }
        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut entries = BTreeMap::new();
            while let Some((key, record)) = map.next_entry::<String, Record>()? {
                if entries.insert(key, record).is_some() {
                    return Err(serde::de::Error::custom("duplicate retired application id"));
                }
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_map(Unique)
}

/// Validate the entire historical closed document, not merely its tag.
/// With a selected home, also enforce the historical writer's exact host.
/// No payload or management entry is opened, and no executable is run.
pub(super) fn validate(bytes: &[u8], home: Option<&Path>) -> Result<(), String> {
    let document: Document = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if document.protocol != PROTOCOL {
        return Err("unsupported retired application index protocol".into());
    }
    let mut coordinates = BTreeSet::new();
    for (key, record) in document.applications {
        let application = record.application;
        if key != application.id || !vibe_core::manifest::is_portable_token(&key) {
            return Err("retired application key differs from a portable application id".into());
        }
        let mut commands = BTreeSet::new();
        if application.commands.is_empty()
            || application.commands.iter().any(|command| {
                !vibe_core::manifest::is_portable_token(command) || !commands.insert(command)
            })
        {
            return Err("retired application commands are empty, invalid or duplicated".into());
        }
        for package in [&application.package, &application.installer_package] {
            vibe_core::Group::parse(&package.group).map_err(|e| e.to_string())?;
            vibe_core::PackageName::parse(&package.name).map_err(|e| e.to_string())?;
            semver::Version::parse(&package.version).map_err(|e| e.to_string())?;
        }
        if !coordinates.insert((application.package.group, application.package.name)) {
            return Err("retired index binds a package coordinate more than once".into());
        }
        let suffix = Path::new("opt").join("apps").join(&key);
        let expected = home.map(|home| home.join(&suffix));
        if !record.host_root.is_absolute()
            || !record.host_root.ends_with(&suffix)
            || expected
                .as_ref()
                .is_some_and(|expected| *expected != record.host_root)
            || !record.management.entry.is_absolute()
            || !record.management.entry.starts_with(&record.host_root)
            || record.management.entry == record.host_root
            || record
                .host_root
                .components()
                .chain(record.management.entry.components())
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err("retired index carries a path outside its owned application host".into());
        }
        // Deserialization has checked both closed enum vocabularies.
        let _ = (record.status, record.management.runtime);
    }
    Ok(())
}

/// Called only after publish_locked has checked the actual home's lock.
/// Validate even modern source documents: a raw /3 save must not overwrite
/// corrupt/unknown input just because proxy admission needed no upgrade.
///
/// A backup is an immutable, no-clobber publication. ANY existing entry,
/// including identical bytes, refuses: it is not proof of a prior completed
/// preservation. A crash before index replacement leaves the source intact;
/// recovery must explicitly resolve that conflict, never guess or overwrite.
pub(super) fn before_replacement(settings: &SettingsRoot) -> Result<(), StateError> {
    let path = settings.index_path();
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io("reading", &path, e)),
    };
    let invalid = |reason| StateError::Index {
        path: path.display().to_string(),
        reason,
    };
    let (_, protocol) = Index::parse_with_protocol(&bytes).map_err(invalid)?;
    if protocol != 1 {
        return Ok(());
    }
    let home =
        fs::canonicalize(settings.path()).map_err(|e| io("resolving", settings.path(), e))?;
    validate(&bytes, Some(&home)).map_err(invalid)?;
    let directory = settings.applications_dir();
    // Refuse links/special entries in the two source components rather than
    // preserving bytes read through a redirected applications directory.
    for source in [&directory, &path] {
        let metadata = fs::symlink_metadata(source).map_err(|e| io("inspecting", source, e))?;
        if metadata.file_type().is_symlink()
            || (source == &directory && !metadata.is_dir())
            || (source == &path && !metadata.is_file())
        {
            return Err(invalid(
                "retired index source is not an ordinary file in its home".into(),
            ));
        }
    }
    let backup = directory.join(BACKUP);
    match fs::symlink_metadata(&backup) {
        Ok(_) => {
            return Err(invalid(format!(
                "retired index backup `{}` already exists; refusing to replace either file",
                backup.display()
            )));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io("inspecting retired index backup", &backup, e)),
    }
    let mut staged = tempfile::NamedTempFile::new_in(&directory)
        .map_err(|e| io("staging retired index backup", &backup, e))?;
    staged
        .write_all(&bytes)
        .and_then(|()| staged.as_file().sync_all())
        .map_err(|e| io("preserving retired index", &backup, e))?;
    let retained = staged
        .persist_noclobber(&backup)
        .map_err(|e| io("publishing retired index backup", &backup, e.error))?;
    retained
        .sync_all()
        .map_err(|e| io("syncing retired index backup", &backup, e))?;
    // Make the backup directory entry durable BEFORE the modern index rename.
    // Windows persist_noclobber publishes the already-flushed file by move;
    // Unix additionally needs the containing directory flushed.
    #[cfg(unix)]
    fs::File::open(&directory)
        .and_then(|file| file.sync_all())
        .map_err(|e| io("syncing retired index directory", &directory, e))?;
    Ok(())
}
