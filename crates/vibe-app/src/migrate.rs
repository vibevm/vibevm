//! `vibe app migrate SLOT FROM TO` — carrying an application's user state
//! between two installed versions (PROP-075 `#MIGRATION`,
//! `#MIGRATION-DECLARATION`, `#MIGRATION-FAILURE`).
//!
//! The strategy is the package's, never vibe's: each
//! `[[application.migration]]` entry has an `export` step the **old**
//! version runs and an `import` step the **new** one runs, both ordinary
//! step declarations executed by the existing step engine over a shared
//! work directory that is **always kept**, success or failure, because it
//! is the evidence of what was and was not carried.
//!
//! What this module does *not* do is decide. Consent belongs to the caller:
//! the library runs exactly the entries it is handed, and an attended offer
//! or an unattended refusal is the CLI's half of "offered, never run
//! without consent".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE");

mod record;
mod run;

use specmark::spec;
use vibe_core::manifest::MigrationDecl;
use vibe_wire::generated::application_migration as wire;

use crate::error::{AppError, StepError};
use crate::index::{Index, Installation, MigrationEntryRecord, MigrationOutcomeRecord};
use crate::operations::OperationContext;

pub(crate) use record::summary as migration_summary;
pub use record::{
    MIGRATION_PROTOCOL, kept_runs as kept_migration_work, work_runs as slot_migration_work,
};
pub(crate) use run::{installation_paths, observed};

/// What one migration run was asked to do. The selection is already
/// decided: `only` holds the ids of the entries to run, and an empty
/// selection runs nothing and reports every entry `skipped`.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
pub struct MigrationRequest<'a> {
    pub slot: &'a str,
    /// The old version, as a version or a whole installation key.
    pub from: &'a str,
    /// The new version, as a version or a whole installation key; equal to
    /// `from` for a maintenance entry.
    pub to: &'a str,
    /// The entry ids to run, in any order.
    pub only: &'a [String],
    /// Reuse the kept work directory and re-run only what has not
    /// succeeded.
    pub resume: bool,
}

/// The two installations a migration would run between and the entries the
/// **new** version declares — everything a caller needs to offer the run,
/// read from the inventory alone with no package tree and no network.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
pub struct MigrationPlan {
    pub slot: String,
    pub from: String,
    pub to: String,
    pub from_version: String,
    pub to_version: String,
    /// The declared entries of the new installation, in declaration order.
    pub entries: Vec<MigrationEntryRecord>,
}

impl MigrationPlan {
    /// The ids an attended offer preselects.
    pub fn preselected(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|entry| entry.default)
            .map(|entry| entry.id.clone())
            .collect()
    }

    /// Every declared id, which is what `--all` means.
    pub fn all(&self) -> Vec<String> {
        self.entries.iter().map(|entry| entry.id.clone()).collect()
    }

    /// The exact command an offer and the unattended refusal print: the
    /// entries an attended offer would preselect, or `--all` when the
    /// package marks none of them `default` (naming no entry at all would
    /// print a command that runs nothing).
    pub fn offer_command(&self, resume: bool) -> String {
        let preselected = self.preselected();
        if preselected.is_empty() {
            let mut text = format!(
                "vibe app migrate {} {} {} --all",
                self.slot, self.from_version, self.to_version
            );
            if resume {
                text.push_str(" --resume");
            }
            text.push_str(" --yes");
            return text;
        }
        self.command(&preselected, resume)
    }

    /// The exact command that runs `only` over this pair.
    pub fn command(&self, only: &[String], resume: bool) -> String {
        let mut text = format!(
            "vibe app migrate {} {} {}",
            self.slot, self.from_version, self.to_version
        );
        for id in only {
            text.push_str(&format!(" --only {id}"));
        }
        if resume {
            text.push_str(" --resume");
        }
        text.push_str(" --yes");
        text
    }
}

/// The two installations and the declared entries of `slot`'s `from` → `to`
/// pair, resolved from the inventory alone (PROP-075
/// `#MIGRATION-DECLARATION`). `from == to` is admitted: that is the
/// maintenance shape, where one version carries state into itself.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let index = vibe_app::Index::load(&settings).unwrap();
/// assert_eq!(
///     vibe_app::migration_plan(&index, "node", "22.0.0", "24.0.0")
///         .unwrap_err()
///         .code(),
///     "unknown_slot"
/// );
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
pub fn migration_plan(
    index: &Index,
    slot: &str,
    from: &str,
    to: &str,
) -> Result<MigrationPlan, AppError> {
    if !index.installations.values().any(|row| row.slot == slot) {
        return Err(AppError::UnknownSlot {
            slot: slot.to_owned(),
        });
    }
    let (from_key, from_record) = installation_in_slot(index, slot, from)?;
    let (to_key, to_record) = installation_in_slot(index, slot, to)?;
    Ok(MigrationPlan {
        slot: slot.to_owned(),
        from: from_key,
        to: to_key,
        from_version: from_record.package.version.clone(),
        to_version: to_record.package.version.clone(),
        entries: to_record.migrations.clone(),
    })
}

/// What an `install -g`, an `update -g` or an `app use` that gave `slot` a
/// new version may offer (PROP-075 `#MIGRATION-FAILURE`): the newest other
/// installed version of the same slot as `from`, and the entries `to`
/// declares. `None` when there is nothing to carry or nothing to carry it
/// from — those commands never migrate on their own.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
pub fn migration_offer(index: &Index, slot: &str, to: &str) -> Option<MigrationPlan> {
    let to_record = index.installations.get(to)?;
    if to_record.slot != slot || to_record.migrations.is_empty() {
        return None;
    }
    let from = previous_in_slot(index, slot, to, to_record)?;
    migration_plan(index, slot, &from, to).ok()
}

/// The installation a migration would carry *from*: the newest other
/// version of the same package in the slot that has not already had every
/// entry carried out of it.
fn previous_in_slot(
    index: &Index,
    slot: &str,
    to: &str,
    to_record: &Installation,
) -> Option<String> {
    let mut candidates: Vec<(semver::Version, &String)> = index
        .installations
        .iter()
        .filter(|(key, row)| {
            key.as_str() != to
                && row.slot == slot
                && row.package.same_package(&to_record.package)
                && !row.lifecycle.hidden
        })
        .filter_map(|(key, row)| {
            semver::Version::parse(&row.package.version)
                .ok()
                .map(|version| (version, key))
        })
        .collect();
    candidates.sort();
    let (_, key) = candidates.pop()?;
    // Already carried: the new installation records what moved into it, so
    // a second `install -g` of the same version does not offer again.
    if to_record
        .migrated
        .iter()
        .any(|outcome| &outcome.from == key && outcome.result.carried())
    {
        return None;
    }
    Some(key.clone())
}

/// One installation of `slot` by version or by whole key.
pub(crate) fn installation_in_slot<'a>(
    index: &'a Index,
    slot: &str,
    spelling: &str,
) -> Result<(String, &'a Installation), AppError> {
    let matched: Vec<(&String, &Installation)> = index
        .installations
        .iter()
        .filter(|(key, row)| {
            row.slot == slot && (key.as_str() == spelling || row.package.version == spelling)
        })
        .collect();
    match matched.as_slice() {
        [(key, record)] => Ok(((*key).clone(), *record)),
        [] => Err(AppError::NotInstalled {
            package: format!("{slot}@{spelling}"),
            home: "the selected application home".to_owned(),
            fix: format!(
                "install it with `vibe install -g <package>@{spelling}`, or name an installed \
                 version from `vibe app alternatives {slot}`"
            ),
        }),
        several => Err(AppError::AmbiguousInstallation {
            package: format!(
                "{slot}@{spelling} ({})",
                several
                    .iter()
                    .map(|(key, _)| key.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }),
    }
}

/// Run the selected entries of `slot`'s `from` → `to` pair, in declaration
/// order and independently of each other (PROP-075 `#MIGRATION-FAILURE`).
///
/// Nothing is ever rolled back: a failed `export` stops its own entry
/// before the import runs and changes nothing; a failed `import` leaves the
/// new version installed and the slot's default exactly where the user put
/// it. The work directory is kept either way, and the returned document
/// names, per entry, what was not carried and the command that retries it.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE")]
pub fn migrate(
    ctx: &OperationContext<'_>,
    request: &MigrationRequest<'_>,
) -> Result<wire::ApplicationMigration, AppError> {
    let held = ctx.settings.lock()?;
    let mut index = Index::load(ctx.settings)?;
    let plan = migration_plan(&index, request.slot, request.from, request.to)?;
    let resolver = ctx
        .resolver
        .ok_or_else(|| StepError::MigrationUnresolvable {
            slot: plan.slot.clone(),
            reason: "this command cannot resolve application packages".to_owned(),
        })?;
    let unknown: Vec<&String> = request
        .only
        .iter()
        .filter(|id| !plan.entries.iter().any(|entry| &&entry.id == id))
        .collect();
    if let Some(id) = unknown.first() {
        return Err(StepError::UnknownMigration {
            slot: plan.slot.clone(),
            id: (*id).clone(),
            declared: if plan.entries.is_empty() {
                "none".to_owned()
            } else {
                plan.all().join(", ")
            },
        }
        .into());
    }
    let (work, resumed) = record::open(ctx.settings, &plan, request.resume)?;
    let previous = record::read(&work);
    let from_package = resolve(resolver, &plan.from)?;
    let to_package = if plan.from == plan.to {
        None
    } else {
        Some(resolve(resolver, &plan.to)?)
    };
    let (from_paths, from_root) = run::installation_paths(ctx, &index, &plan.from, &held)?;
    let (to_paths, to_root) = if plan.from == plan.to {
        (from_paths.clone(), from_root.clone())
    } else {
        run::installation_paths(ctx, &index, &plan.to, &held)?
    };
    let new_record = &index.installations[&plan.to];
    let mut document = record::empty(&plan, &work, resumed);
    let session = run::Session {
        ctx,
        plan: &plan,
        work: &work,
        from: &from_package,
        to: to_package.as_ref().unwrap_or(&from_package),
        from_root,
        to_root,
        from_paths,
        to_paths,
        platform: new_record.platform.clone(),
        arch: new_record.arch.clone(),
    };
    for entry in &plan.entries {
        let kept = record::entry_of(previous.as_ref(), &entry.id);
        if !request.only.iter().any(|id| id == &entry.id) {
            // A resume keeps what the directory already knows about an
            // entry it was not asked to run, so a chain of `--only A
            // --resume` then `--only B --resume` does not lose A's export
            // (`PROP-075#MIGRATION-FAILURE`).
            document.entries.push(match kept {
                Some(previous) => previous.clone(),
                None => record::skipped(
                    entry,
                    "not selected; name it with `--only` or take every entry with `--all`",
                ),
            });
            continue;
        }
        let outcome = session.run_entry(entry, kept)?;
        document.entries.push(outcome);
        record::save(&work, &document)?;
    }
    // The verdict is over what *this* run was asked to carry: a resume of
    // one entry does not fail because another entry failed in an earlier
    // run over the same directory.
    document.ok = document
        .entries
        .iter()
        .filter(|entry| request.only.iter().any(|id| id == &entry.id))
        .all(|entry| {
            !matches!(
                entry.outcome,
                wire::Outcome::ExportFailed | wire::Outcome::ImportFailed
            )
        });
    record::save(&work, &document)?;
    // The outcomes are recorded with the **new** installation, so a later
    // `vibe app list --json` and a later migration see what already moved.
    let carried = record::outcomes(&plan, &document);
    if !carried.is_empty() {
        remember(ctx, &mut index, &plan, carried, &held)?;
    }
    Ok(document)
}

/// Record the run's outcomes on the new installation and publish the
/// inventory. Writing them is the first thing in this home that needs the
/// `vibe-user-applications/7` reader, so the publication admits and
/// refreshes the capable proxy image first (PROP-059
/// `#proxy-index-upgrade`).
fn remember(
    ctx: &OperationContext<'_>,
    index: &mut Index,
    plan: &MigrationPlan,
    carried: Vec<MigrationOutcomeRecord>,
    lock: &crate::MutationLock,
) -> Result<(), AppError> {
    let Some(record) = index.installations.get_mut(&plan.to) else {
        return Ok(());
    };
    for outcome in carried {
        record
            .migrated
            .retain(|kept| !(kept.id == outcome.id && kept.from == outcome.from));
        record.migrated.push(outcome);
    }
    let image = crate::proxies::for_publication(ctx.settings, index, ctx.proxy_image)?;
    index.publish_locked(
        ctx.settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Updated, plan.to.clone()),
        lock,
        image.as_ref(),
    )?;
    Ok(())
}

/// The package tree of one installation key, resolved through the caller's
/// ordinary registry chain: a migration step runs its **own** version's
/// script, and a script lives in a package tree the inventory does not
/// retain.
fn resolve(
    resolver: crate::operations::PackageResolver<'_>,
    key: &str,
) -> Result<crate::resolve::ResolvedPackage, AppError> {
    resolver(key).map_err(|error| {
        StepError::MigrationUnresolvable {
            slot: key.to_owned(),
            reason: error.to_string(),
        }
        .into()
    })
}

/// The declaration of one entry in a package, by id: an authored
/// `[[application.migration]]` row, or the entry vibe **synthesises** from
/// an `[[application.ecosystem]]` adapter that declares a `list` verb —
/// observe in the old version, re-install in the new, which already *is*
/// the strategy `#MIGRATION` describes (PROP-075 `#ECOSYSTEM-MIGRATION`).
/// The `ecosystem:` id namespace is reserved at validation, so a
/// synthesised entry can never be shadowed by an authored one.
fn declared(package: &crate::resolve::ResolvedPackage, id: &str) -> Option<MigrationDecl> {
    package
        .application
        .migration_entries()
        .into_iter()
        .find(|entry| entry.id == id)
        .map(std::borrow::Cow::into_owned)
}

#[cfg(test)]
#[path = "migrate/tests.rs"]
mod tests;
