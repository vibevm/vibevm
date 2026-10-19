//! User-facing alternative selection. Ordinary PATH proxies never call this module.

pub(super) mod projection;
mod session;

use anyhow::{Result, bail};
use serde_json::{Value, json};
use vibe_core::manifest::is_portable_token;

use super::model::{ApplicationIndex, ApplicationStatus};
use super::store::{ApplicationStore, UncertainIndexCommit};
use super::{fresh_revision, launch, settings_root};

fn ready_installation<'a>(
    index: &'a ApplicationIndex,
    slot: &str,
    key: &str,
) -> Result<&'a super::model::ApplicationRecord> {
    let record = index
        .applications
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("unknown installation `{key}`"))?;
    if record.application.id != slot || record.status != ApplicationStatus::Ready {
        bail!("installation `{key}` is not ready in slot `{slot}`");
    }
    Ok(record)
}

/// Complete or roll back any interrupted filesystem projection under the
/// caller's application mutation lock, before beginning another mutation.
pub(super) fn recover_pending(index: &ApplicationIndex, settings: &std::path::Path) -> Result<()> {
    let removals = super::removal::PendingRemovals::load(settings)?;
    let mut _leases = Vec::new();
    for key in removals.installation_keys() {
        _leases.push(
            super::lease::try_acquire_exclusive(settings, key)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "application_in_use: `{key}` has an active launcher during removal recovery"
                )
            })?,
        );
    }
    projection::recover_pending(settings, index)?;
    removals.recover(index)
}

pub fn listing(slot: &str) -> Result<Value> {
    if !is_portable_token(slot) {
        bail!("invalid application slot `{slot}`");
    }
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    let alternative = index
        .alternatives
        .get(slot)
        .ok_or_else(|| anyhow::anyhow!("unknown application slot `{slot}`"))?;
    let published = projection::published_global(&settings, slot)?;
    let session = session::effective_from_path(slot, &settings, &index)?;
    let effective = session.as_ref().or(published.as_ref());
    let mut installations = Vec::new();
    for (key, record) in &index.applications {
        if record.application.id != slot {
            continue;
        }
        installations.push(json!({
            "key": key,
            "package": {
                "group": record.application.package.group,
                "name": record.application.package.name,
                "version": record.application.package.version,
            },
            "status": if record.status == ApplicationStatus::Ready { "ready" } else { "undeployed" },
            "platform": record.selected_platform,
            "installer": record.selected_option_id,
            "public_commands": record.public_commands.iter().map(|entry| json!({"name":entry.name,"launcher":entry.launcher})).collect::<Vec<_>>(),
            "launchers": record.user_launchers.iter().map(|entry| json!({"id":entry.id,"name":entry.name,"default":entry.default})).collect::<Vec<_>>(),
            "selected_global": published.as_deref() == Some(key.as_str()),
            "selected_effective": effective.map(String::as_str) == Some(key.as_str()),
        }));
    }
    let commands: Vec<_> = alternative
        .reserved_commands
        .iter()
        .map(|name| {
            let provided_by: Vec<_> = index
                .applications
                .iter()
                .filter(|(_, record)| {
                    record.application.id == slot && record.status == ApplicationStatus::Ready
                })
                .filter(|(_, record)| {
                    record
                        .public_commands
                        .iter()
                        .any(|entry| entry.name == *name)
                })
                .map(|(key, _)| key.clone())
                .collect();
            let binding = effective
                .and_then(|key| index.applications.get(key))
                .and_then(|record| {
                    record
                        .public_commands
                        .iter()
                        .find(|entry| entry.name == *name)
                        .map(|entry| (record, entry))
                });
            let effective_launcher = binding.map(|(_, entry)| entry.launcher.clone());
            let availability = binding.map(|(record, entry)| {
                launch::plan_public_launcher(record, &entry.launcher, &settings)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            });
            let available = availability.as_ref().is_some_and(Result::is_ok);
            let reason = match availability {
                Some(Err(reason)) => Some(reason),
                None => Some("command is not provided by the selected installation".to_owned()),
                Some(Ok(())) => None,
            };
            json!({
                "name": name,
                "path": settings.join("opt/bin").join(projection::proxy_filename(name)),
                "provided_by": provided_by,
                "effective_launcher": effective_launcher,
                "available": available,
                "reason": reason,
            })
        })
        .collect();
    Ok(json!({
        "protocol": "vibe-app-alternatives/1",
        "slot": slot,
        "desired_global": alternative.global_default,
        "global_default": published,
        "effective": effective,
        "scope": if session.is_some() { "session" } else { "global" },
        "repair_needed": published != alternative.global_default
            || settings.join("applications/alternative-intents").join(format!("{slot}.json")).exists(),
        "installations": installations,
        "commands": commands,
    }))
}

pub fn select(slot: &str, installation: &str) -> Result<Value> {
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    recover_pending(&index, &settings)?;
    ready_installation(&index, slot, installation)?;
    let alternative = index
        .alternatives
        .get_mut(slot)
        .ok_or_else(|| anyhow::anyhow!("unknown application slot `{slot}`"))?;
    alternative.global_default = Some(installation.to_owned());
    index.revision = fresh_revision();
    let mut prepared = projection::prepare_slot(&mut index, &settings, slot)?;
    if let Err(error) = store.save(&index) {
        if error.downcast_ref::<UncertainIndexCommit>().is_some() {
            return Err(error.context("alternative selection has an uncertain index commit; staged files retained for recovery"));
        }
        prepared.rollback()?;
        return Err(error);
    }
    prepared.commit_current()?;
    prepared.commit()?;
    drop(store);
    listing(slot)
}

pub fn use_session(slot: &str, installation: &str, shell: &str) -> Result<Value> {
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    ready_installation(&index, slot, installation)?;
    session::use_session(slot, installation, shell, &settings, &index)
}

pub fn clear_session(slot: Option<&str>, all: bool, shell: &str) -> Result<Value> {
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    session::clear_session(slot, all, shell, &settings, &index)
}

pub fn shell(slot: &str, installation: &str) -> Result<i32> {
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    ready_installation(&index, slot, installation)?;
    session::shell(slot, installation, &settings, &index)
}

pub fn run(slot: &str, command: &str, args: &[String]) -> Result<i32> {
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    let published = projection::published_global(&settings, slot)?;
    let selected = session::effective_from_path(slot, &settings, &index)?
        .or(published)
        .ok_or_else(|| anyhow::anyhow!("application slot `{slot}` has no selected installation"))?;
    let record = ready_installation(&index, slot, &selected)?;
    let binding = record
        .public_commands
        .iter()
        .find(|entry| entry.name == command)
        .ok_or_else(|| {
            anyhow::anyhow!("command_unavailable: `{command}` is not provided by `{selected}`")
        })?;
    let reference = format!(
        "{}/{}@={}",
        record.application.package.group,
        record.application.package.name,
        record.application.package.version
    );
    launch::run(&reference, &binding.launcher, args, None)
}

/// Launch the selected installation's authored default point, including GUI
/// applications that expose no public PATH command.
pub fn start(slot: &str, launcher_id: Option<&str>, args: &[String]) -> Result<i32> {
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    let published = projection::published_global(&settings, slot)?;
    let selected = session::effective_from_path(slot, &settings, &index)?
        .or(published)
        .ok_or_else(|| anyhow::anyhow!("application slot `{slot}` has no selected installation"))?;
    let record = ready_installation(&index, slot, &selected)?;
    let launcher = match launcher_id {
        Some(id) => id,
        None => record
            .user_launchers
            .iter()
            .find(|entry| entry.default)
            .map(|entry| entry.id.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "selection_required: selected installation `{selected}` has no default launcher"
                )
            })?,
    };
    let reference = format!(
        "{}/{}@={}",
        record.application.package.group,
        record.application.package.name,
        record.application.package.version
    );
    launch::run(&reference, launcher, args, None)
}
