//! Whole payload directories on the user's PATH
//! (`PROP-059#path-directories`).
//!
//! A per-command proxy in `opt/bin` exposes exactly the commands a package
//! declared. That is the wrong shape for a runtime whose own tooling writes
//! new commands into its own directory after installation — `npm install
//! -g` puts them next to `node`, so a proxy set fixed at install time can
//! never reach them. An option may therefore opt in with `path_dirs` and
//! ask for its directories to be exposed *whole*.
//!
//! Exposure is one link per slot and per declared directory index:
//! `<paths>/<slot>/<i>` points at the slot default provider's directory
//! `i`. The link, not the payload, is what goes on the user's durable PATH,
//! so switching the default with `vibe app use` repoints a link instead of
//! rewriting PATH, and two providers of one slot can never collide —
//! each slot has its own link directory.
//!
//! Everything here is derived from the index: this module computes the
//! links the index implies, compares them with what is on disk and with
//! what vibe recorded writing, and reconciles the difference. Every
//! operation that writes the index therefore repoints by construction —
//! first install, `vibe app use`, uninstall, hide and expose, update and
//! rollback, modify, a storage move, an import, a project materialisation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#path-directories");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#slot-off");

mod link;
mod ownership;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use specmark::spec;

use crate::error::AppError;
use crate::index::{EnvEntry, EnvOp, Index};
use crate::settings::{MutationLock, SettingsRoot, io};
use crate::storage::Storage;

#[cfg(test)]
use ownership::inventory_path;

/// The step name every link-directory PATH entry carries, so a durable
/// environment line is recognisable as this feature's
/// (`PROP-065#env-step`).
pub const PATH_LINK_STEP: &str = "paths";

/// What one slot needs on the user's durable PATH, and what its links point
/// at right now.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
pub struct SlotPaths {
    pub slot: String,
    /// The link directories, lowest index first: what belongs on PATH for
    /// as long as any provider of the slot declares a payload directory.
    pub link_dirs: Vec<PathBuf>,
    /// The payload directory each link points at, when the slot is on and
    /// its default provider's payload is there.
    pub targets: Vec<Option<PathBuf>>,
}

/// The reconciliation of one home's slot links: which slots now need their
/// link directories on the durable PATH, and which no longer do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
pub struct PathSync {
    /// Slots whose link directories belong on the durable PATH, with the
    /// directories themselves.
    pub needed: Vec<SlotPaths>,
    /// Slots that have no payload directory left to expose; their PATH
    /// entries are removed and their link directory is gone.
    pub released: Vec<String>,
}

/// The number of link directories a slot has: the widest `path_dirs` any
/// exposed provider of the slot declares. It is the widest and not the
/// default's own count so that the set of directories on the user's durable
/// PATH does not change when `vibe app use` switches to a provider that
/// declares fewer — PATH churns only when a slot gains or loses the feature.
fn width(index: &Index, slot: &str) -> usize {
    index
        .installations
        .values()
        .filter(|record| record.slot == slot && !record.lifecycle.hidden)
        .map(|record| record.path_dirs.len())
        .max()
        .unwrap_or(0)
}

/// The absolute payload directory `template` names for `record`, or the
/// reason it cannot be exposed. `None` means the payload is simply not
/// there — an unavailable library or a directory the option does not
/// create on this host — which is not an error: the link is left out and
/// the lookup falls through.
fn target(
    settings: &SettingsRoot,
    storage: &Storage,
    record: &crate::index::Installation,
    template: &str,
) -> Result<Option<PathBuf>, String> {
    let app_root = match crate::launch::app_root(record, storage) {
        Ok(root) => root,
        Err(_) => return Ok(None),
    };
    let vars = crate::launch::TemplateVars {
        app_root: app_root.clone(),
        vibe_home: settings.path().to_path_buf(),
        project_root: None,
    };
    let expanded = crate::launch::expand_template(template, &vars)?;
    let candidate = if Path::new(&expanded).is_absolute() {
        PathBuf::from(&expanded)
    } else {
        app_root.join(&expanded)
    };
    if !candidate.is_dir() {
        return Ok(None);
    }
    // The declaration is contained by the manifest's own validation; the
    // resolved path is checked again here, because a payload may hold a
    // link of its own.
    let resolved = std::fs::canonicalize(&candidate)
        .map(link::normalise)
        .map_err(|error| format!("`{}` cannot be resolved: {error}", candidate.display()))?;
    let root = std::fs::canonicalize(&app_root)
        .map(link::normalise)
        .map_err(|error| format!("`{}` cannot be resolved: {error}", app_root.display()))?;
    if !resolved.starts_with(&root) {
        return Err(format!(
            "`{template}` resolves to `{}`, which is outside the application root `{}`",
            resolved.display(),
            root.display()
        ));
    }
    link::supported_target(&resolved)?;
    Ok(Some(resolved))
}

/// What the index implies for every slot that has, or recorded, links.
fn wanted(
    settings: &SettingsRoot,
    index: &Index,
    storage: &Storage,
    recorded: &[String],
) -> Result<BTreeMap<String, SlotPaths>, AppError> {
    let slots: BTreeSet<String> = index
        .slots
        .keys()
        .cloned()
        .chain(index.installations.values().map(|r| r.slot.clone()))
        .chain(recorded.iter().cloned())
        .collect();
    let mut wanted = BTreeMap::new();
    for slot in slots {
        let width = width(index, &slot);
        if width == 0 {
            wanted.insert(
                slot.clone(),
                SlotPaths {
                    slot,
                    link_dirs: Vec::new(),
                    targets: Vec::new(),
                },
            );
            continue;
        }
        let default = index
            .effective_default_of(&slot)
            .and_then(|key| index.installations.get(key));
        // Index `i` of the slot has a target only when the default declares
        // that many directories; a provider with fewer leaves the rest
        // unlinked, so the lookup falls through for them.
        let mut targets = vec![None; width];
        if let Some(record) = default {
            for (position, template) in record.path_dirs.iter().enumerate().take(width) {
                targets[position] =
                    target(settings, storage, record, template).map_err(|reason| {
                        crate::error_paths::PathsError::Target {
                            slot: slot.clone(),
                            target: template.clone(),
                            reason,
                        }
                    })?;
            }
        }
        wanted.insert(
            slot.clone(),
            SlotPaths {
                link_dirs: (0..width as u32)
                    .map(|position| settings.slot_link_path(&slot, position))
                    .collect(),
                targets,
                slot,
            },
        );
    }
    Ok(wanted)
}

/// Make the slot links on disk agree with `index`, inside an already-held
/// mutation. Nothing vibe did not write is ever replaced or removed: a
/// foreign file, directory or link at a link's name refuses the whole
/// reconciliation before the first change.
///
/// ```
/// let temp = tempfile::tempdir().unwrap();
/// let home = vibe_app::SettingsRoot::new(temp.path());
/// let lock = home.lock().unwrap();
/// let index = vibe_app::Index::default();
/// let storage = vibe_app::Storage::load(&home).unwrap();
/// let synced = vibe_app::sync_slot_paths(&home, &index, &storage, &lock).unwrap();
/// assert!(synced.needed.is_empty() && synced.released.is_empty());
/// ```
pub fn sync_locked(
    settings: &SettingsRoot,
    index: &Index,
    storage: &Storage,
    lock: &MutationLock,
) -> Result<PathSync, AppError> {
    lock.require_home(settings)?;
    let mut inventory = ownership::Inventory::load(settings)?;
    let recorded = inventory.slots();
    let wanted = wanted(settings, index, storage, &recorded)?;

    // Refuse before the first write: every name this reconciliation would
    // touch must be vibe's own or free.
    for state in wanted.values() {
        let keep = state.link_dirs.len() as u32;
        let touched: BTreeSet<u32> = (0..keep)
            .chain(inventory.recorded_for(&state.slot))
            .collect();
        for position in touched {
            let path = settings.slot_link_path(&state.slot, position);
            if !inventory.classify(&state.slot, position, &path)?.writable() {
                return Err(crate::error_paths::PathsError::ForeignLink {
                    slot: state.slot.clone(),
                    index: position,
                    path: path.display().to_string(),
                }
                .into());
            }
        }
    }

    let mut synced = PathSync::default();
    for state in wanted.values() {
        // Links the slot no longer has at all, and links it has but whose
        // target is gone or whose slot is off.
        let keep = state.link_dirs.len() as u32;
        let obsolete: Vec<u32> = inventory
            .recorded_for(&state.slot)
            .into_iter()
            .filter(|position| {
                *position >= keep
                    || state
                        .targets
                        .get(*position as usize)
                        .is_none_or(Option::is_none)
            })
            .collect();
        for position in obsolete {
            let path = settings.slot_link_path(&state.slot, position);
            link::remove(&path).map_err(|source| io("removing the slot link", &path, source))?;
            inventory.forget(&state.slot, position);
        }
        for (position, wanted_target) in state.targets.iter().enumerate() {
            let Some(wanted_target) = wanted_target else {
                continue;
            };
            let position = position as u32;
            let path = settings.slot_link_path(&state.slot, position);
            let current = inventory.classify(&state.slot, position, &path)?;
            if current != ownership::Ownership::Recorded(wanted_target.clone()) {
                link::point(&path, wanted_target)
                    .map_err(|source| io("pointing the slot link", &path, source))?;
            }
            inventory.record(&state.slot, position, wanted_target);
        }
        if state.link_dirs.is_empty() {
            // The directory itself goes once nothing is in it; a directory
            // the operator put something else into is left alone.
            let dir = settings.slot_links_dir(&state.slot);
            let _ = std::fs::remove_dir(&dir);
            if recorded.contains(&state.slot) || !index.slot_path_env(&state.slot).is_empty() {
                synced.released.push(state.slot.clone());
            }
        } else {
            synced.needed.push(state.clone());
        }
    }
    inventory.save()?;
    Ok(synced)
}

/// Reconcile the links and then the user's durable PATH: a slot that now
/// exposes payload directories gains a PATH entry per link directory, and a
/// slot whose last such provider went loses exactly the entries vibe
/// recorded applying for it (`PROP-065#env-step`).
///
/// The entries are applied from the last declared directory to the first,
/// because every durable-environment implementation *prepends*: the first
/// declared directory therefore ends up first on PATH, ahead of `opt/bin`
/// and ahead of whatever the system put there — which is the point, since a
/// payload command must win over the system program it replaces.
///
/// `index` is updated in place and saved again only when a slot's recorded
/// entries actually changed, so the common operation writes the index once.
/// A failure after the environment was changed leaves the apply to be
/// repeated: both implementations are idempotent per entry.
pub(crate) fn apply_locked(
    settings: &SettingsRoot,
    index: &mut Index,
    storage: &Storage,
    environment: Option<&dyn crate::environment::UserEnvironment>,
    lock: &MutationLock,
) -> Result<PathSync, AppError> {
    let synced = sync_locked(settings, index, storage, lock)?;
    let mut changes: Vec<(String, Vec<EnvEntry>, Vec<EnvEntry>)> = Vec::new();
    for state in &synced.needed {
        let desired: Vec<EnvEntry> = state
            .link_dirs
            .iter()
            .rev()
            .map(|dir| path_entry(dir))
            .collect();
        let current = index.slot_path_env(&state.slot);
        if !same_entries(current, &desired) {
            changes.push((state.slot.clone(), current.to_vec(), desired));
        }
    }
    for slot in &synced.released {
        let current = index.slot_path_env(slot);
        if !current.is_empty() {
            changes.push((slot.clone(), current.to_vec(), Vec::new()));
        }
    }
    if changes.is_empty() {
        return Ok(synced);
    }
    let environment = environment.ok_or_else(|| crate::error_paths::PathsError::Target {
        slot: changes[0].0.clone(),
        target: settings.path_links_dir().display().to_string(),
        reason: "this host has no durable user environment vibe can write, so the slot's link \
                 directories cannot be put on PATH; set HOME (or USERPROFILE) and re-run"
            .into(),
    })?;
    for (slot, previous, desired) in changes {
        let tag = env_tag(&slot);
        if !previous.is_empty() {
            environment.remove(&tag, &previous).map_err(|reason| {
                crate::error_paths::PathsError::Target {
                    slot: slot.clone(),
                    target: "the durable PATH".into(),
                    reason,
                }
            })?;
        }
        let applied = if desired.is_empty() {
            Vec::new()
        } else {
            environment.apply(&tag, &desired).map_err(|reason| {
                crate::error_paths::PathsError::Target {
                    slot: slot.clone(),
                    target: "the durable PATH".into(),
                    reason,
                }
            })?
        };
        index.slot_mut(&slot).path_env = applied;
    }
    // A slot kept only for that bookkeeping leaves once it owes nothing.
    let populated: BTreeSet<String> = index
        .installations
        .values()
        .map(|record| record.slot.clone())
        .collect();
    index.slots.retain(|slot, state| {
        populated.contains(slot) || !state.path_env.is_empty() || state.default.is_some()
    });
    index.save_locked(
        settings,
        &crate::catalog::HomeEvent::new(crate::catalog::EventKind::Updated),
        lock,
    )?;
    Ok(synced)
}

/// Whether two entry lists name the same PATH values in the same order;
/// `previous` carries what the host recorded, which the desired entries do
/// not, so only the operation and the value are compared.
fn same_entries(left: &[EnvEntry], right: &[EnvEntry]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.op == b.op && a.name == b.name && a.value == b.value)
}

/// The real payload directories of one installation, in declaration order:
/// what `vibe shell --use SLOT=KEY` puts in front of the child's PATH, so a
/// session reaches that provider directly instead of through the slot's
/// link — no link is repointed and the global default is untouched
/// (`PROP-059#session-selection`).
///
/// A directory the payload does not have is left out, exactly as the links
/// leave it out.
pub fn session_dirs(
    settings: &SettingsRoot,
    index: &Index,
    storage: &Storage,
    key: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let Some(record) = index.installations.get(key) else {
        return Ok(Vec::new());
    };
    let mut dirs = Vec::new();
    for template in &record.path_dirs {
        let resolved = target(settings, storage, record, template).map_err(|reason| {
            crate::error_paths::PathsError::Target {
                slot: record.slot.clone(),
                target: template.clone(),
                reason,
            }
        })?;
        if let Some(resolved) = resolved {
            dirs.push(resolved);
        }
    }
    Ok(dirs)
}

/// The link directories of `slot` as the index implies them: what
/// `vibe shell --use SLOT=none` takes out of the child's PATH, so a lookup
/// in that session falls through to the system program without changing
/// anything durable (`PROP-059#slot-off`).
pub fn slot_link_dirs(settings: &SettingsRoot, index: &Index, slot: &str) -> Vec<PathBuf> {
    (0..width(index, slot) as u32)
        .map(|position| settings.slot_link_path(slot, position))
        .collect()
}

/// The durable PATH entry one link directory needs (`PROP-065#env-step`).
pub fn path_entry(dir: &Path) -> EnvEntry {
    EnvEntry {
        step: PATH_LINK_STEP.to_owned(),
        op: EnvOp::PathPrepend,
        name: "PATH".to_owned(),
        value: Some(dir.to_string_lossy().into_owned()),
        previous: None,
        previous_kind: None,
    }
}

/// The tag a slot's durable PATH entries are written under, so one slot's
/// lines are recognisable in `env.sh` and removable on their own.
pub fn env_tag(slot: &str) -> String {
    format!("slot:{slot}")
}

#[cfg(test)]
#[path = "paths/tests.rs"]
mod tests;
