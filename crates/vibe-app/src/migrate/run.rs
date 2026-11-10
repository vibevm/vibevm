//! Running the two sides of one migration entry through the existing step
//! engine (PROP-075 `#MIGRATION-DECLARATION`): no new step kind, no second
//! runner — the per-step entry point of `PROP-065#kinds` with a migration
//! environment and the two installations' PATHs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION");

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{
    ECOSYSTEM_MIGRATION_PREFIX, MigrationDecl, MigrationSide, Need, StepDecl, StepPhase,
};
use vibe_wire::generated::application_migration as wire;

use crate::error::{AppError, MigrationFailure, StepError};
use crate::exposure::Exposure;
use crate::index::{Index, MigrationEntryRecord};
use crate::installer::{ChildIo, TransferFacts, io};
use crate::operations::OperationContext;
use crate::resolve::ResolvedPackage;
use crate::steps::{EcosystemEnv, MigrationEnv, StepContext, StepPaths, StepVars, run_step};

use super::{MigrationPlan, declared, record};

/// Everything one migration run needs that does not change between its
/// entries.
pub(super) struct Session<'a> {
    pub ctx: &'a OperationContext<'a>,
    pub plan: &'a MigrationPlan,
    pub work: &'a Path,
    /// The old version's package tree and the new one's; the same tree for
    /// a maintenance entry (`from == to`).
    pub from: &'a ResolvedPackage,
    pub to: &'a ResolvedPackage,
    /// The old and new installations' application roots.
    pub from_root: PathBuf,
    pub to_root: PathBuf,
    /// What each side puts first on PATH: the old installation's payload
    /// and proxy directories for the export, the new one's for the import
    /// (`PROP-065#app-dependencies`, reused verbatim).
    pub from_paths: StepPaths,
    pub to_paths: StepPaths,
    /// The platform and architecture ids the new installation recorded,
    /// which are what `VIBE_PLATFORM` and `VIBE_ARCH` name.
    pub platform: String,
    pub arch: String,
}

/// The side being run and everything that differs between the two.
struct Half<'a> {
    side: MigrationSide,
    step: &'a StepDecl,
    package_root: &'a Path,
    paths: &'a StepPaths,
    /// The adapter this side is a verb of, when the entry is a
    /// synthesised `ecosystem:<id>` one (PROP-075
    /// `#ECOSYSTEM-MIGRATION`): the export is its `list` verb and the
    /// import its `install` verb, handed exactly the items the export
    /// observed.
    adapter: Option<Adapter<'a>>,
}

/// The adapter half of a synthesised entry: its opaque id, the file the
/// `list` verb writes and the `install` verb's argv is read from, and the
/// items this side is handed.
struct Adapter<'a> {
    id: &'a str,
    list: PathBuf,
    items: Vec<String>,
}

/// One item per line, trimmed, the empty ones dropped — the only reading
/// vibe ever does of an ecosystem's answer, and it parses no ecosystem's
/// listing format (PROP-075 `#ECOSYSTEM-ADAPTER`). An absent or empty file
/// is an empty list and never an error.
pub(crate) fn observed(file: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

impl Session<'_> {
    /// Run one entry: the export, then — only if it succeeded — the
    /// import. Returns the entry's record; a failure is recorded, never
    /// propagated, because one entry's failure does not stop another's.
    pub(super) fn run_entry(
        &self,
        entry: &MigrationEntryRecord,
        kept: Option<&wire::Entry>,
    ) -> Result<wire::Entry, AppError> {
        let mut record = wire::Entry {
            id: entry.id.clone(),
            name: entry.name.clone(),
            outcome: wire::Outcome::Done,
            description: entry.description.clone(),
            side: None,
            step: None,
            log: None,
            reason: None,
            retry: None,
            started: Some(now()),
            finished: None,
        };
        let Some(import) = declared(self.to, &entry.id) else {
            return Ok(record::skipped(
                entry,
                &format!(
                    "`{}` declares no migration entry `{}`",
                    self.plan.to, entry.id
                ),
            ));
        };
        let Some(export) = declared(self.from, &entry.id) else {
            return Ok(record::skipped(
                entry,
                &format!(
                    "`{}` declares no migration entry `{}`, so there is nothing to export from it",
                    self.plan.from, entry.id
                ),
            ));
        };
        let dir = self.entry_dir(&entry.id)?;
        // A synthesised entry's two sides are the adapter's `list` and
        // `install` verbs over one shared answer file.
        let adapter_id = entry.id.strip_prefix(ECOSYSTEM_MIGRATION_PREFIX);
        let list = dir.join("items");
        // `--resume` re-runs only the import when the export already
        // succeeded, the same contract `PROP-065#resume` gives an
        // installation.
        let exported = kept.is_some_and(|previous| {
            matches!(
                previous.outcome,
                wire::Outcome::Pending | wire::Outcome::ImportFailed | wire::Outcome::Done
            )
        });
        if !exported
            && let Err(failure) = self.run_side(
                &Half {
                    side: MigrationSide::Export,
                    step: &export.export,
                    package_root: &self.from.root,
                    paths: &self.from_paths,
                    adapter: adapter_id.map(|id| Adapter {
                        id,
                        list: list.clone(),
                        items: Vec::new(),
                    }),
                },
                &dir,
                &export,
            )
        {
            return Ok(self.failed(record, MigrationSide::Export, &export.export, &dir, failure));
        }
        record.outcome = wire::Outcome::Pending;
        if let Err(failure) = self.run_side(
            &Half {
                side: MigrationSide::Import,
                step: &import.import,
                package_root: &self.to.root,
                paths: &self.to_paths,
                adapter: adapter_id.map(|id| Adapter {
                    id,
                    list: list.clone(),
                    items: observed(&list),
                }),
            },
            &dir,
            &import,
        ) {
            return Ok(self.failed(record, MigrationSide::Import, &import.import, &dir, failure));
        }
        record.outcome = wire::Outcome::Done;
        record.side = Some(wire::Side::Import);
        record.step = Some(import.import.id.clone());
        record.finished = Some(now());
        Ok(record)
    }

    /// Where the entry's two logs go: `<work>/logs/<entry id>/`. They are
    /// kept one level down deliberately — the **top** of the shared work
    /// directory belongs entirely to the package's own scripts, so an
    /// entry named `state` may write a file called `state` there without
    /// colliding with its own logs.
    fn entry_dir(&self, id: &str) -> Result<PathBuf, AppError> {
        // A synthesised id carries the reserved `ecosystem:` prefix, and a
        // colon is not a portable path component, so the directory takes
        // the id with its separators replaced. An authored id is a
        // portable token and is therefore unchanged.
        let dir = self
            .work
            .join("logs")
            .join(id.replace(ECOSYSTEM_MIGRATION_PREFIX, "ecosystem-"));
        fs::create_dir_all(&dir).map_err(|source| io("creating", &dir, source))?;
        Ok(dir)
    }

    /// Record a failed side: what was not carried, where its log is, and
    /// the exact command that retries it.
    fn failed(
        &self,
        mut record: wire::Entry,
        side: MigrationSide,
        step: &StepDecl,
        dir: &Path,
        failure: AppError,
    ) -> wire::Entry {
        record.outcome = match side {
            MigrationSide::Export => wire::Outcome::ExportFailed,
            MigrationSide::Import => wire::Outcome::ImportFailed,
        };
        record.side = Some(match side {
            MigrationSide::Export => wire::Side::Export,
            MigrationSide::Import => wire::Side::Import,
        });
        record.step = Some(step.id.clone());
        record.log = Some(log_path(dir, side).display().to_string());
        record.reason = Some(failure.to_string());
        record.retry = Some(self.plan.command(std::slice::from_ref(&record.id), true));
        record.finished = Some(now());
        record
    }

    /// Whether every `needs` row of a migration step is met on this host
    /// (`PROP-065#needs`, which `#MIGRATION-DECLARATION` says a migration
    /// step honours like any other). A migration step fetches nothing, so
    /// `network` is met whenever the invocation is not offline, and `agent`
    /// is never met, because an `agent` step is refused at validation and
    /// nothing here delegates.
    fn unmet_need(&self, step: &StepDecl) -> Option<String> {
        step.needs
            .iter()
            .find(|need| match need {
                Need::Command(name) => !(self.ctx.facts.command_present)(name),
                Need::Agent => true,
                Need::Terminal => self.ctx.chooser.is_none(),
                Need::Network => self.ctx.offline,
            })
            .map(Need::to_string)
    }

    /// One side of one entry, through the ordinary step runner.
    fn run_side(&self, half: &Half<'_>, dir: &Path, entry: &MigrationDecl) -> Result<(), AppError> {
        let ctx = self.ctx;
        if let Some(need) = self.unmet_need(half.step) {
            // Refused before the child starts, so an entry whose export
            // cannot run has changed nothing (`PROP-065#needs`).
            return Err(StepError::UnmetNeed {
                key: format!("{}@{}", self.plan.slot, self.plan.to_version),
                step: half.step.id.clone(),
                need,
            }
            .into());
        }
        let log = log_path(dir, half.side);
        if let Some(told) = ctx.on_step_log {
            told.opening(&half.step.id, &log);
        }
        let task = ctx.progress.task(format!(
            "{} · {} `{}`",
            entry.name,
            half.side.as_str(),
            half.step.id
        ));
        let migration = MigrationEnv {
            id: &entry.id,
            work: self.work,
            from_app_root: &self.from_root,
            from_version: &self.plan.from_version,
            to_app_root: &self.to_root,
            to_version: &self.plan.to_version,
        };
        let served = RefCell::new(Vec::new());
        let transfer = TransferFacts {
            package: match half.side {
                MigrationSide::Export => self.plan.from.clone(),
                MigrationSide::Import => self.plan.to.clone(),
            },
            step: half.step.id.clone(),
            batch: None,
            resume: None,
        };
        let env = half.paths.child_env(&[], &half.step.id, ctx.search_path);
        let ecosystem = half.adapter.as_ref().map(|adapter| EcosystemEnv {
            id: adapter.id,
            items: &adapter.items,
            list: &adapter.list,
        });
        let package_key = match half.side {
            MigrationSide::Export => self.plan.from.clone(),
            MigrationSide::Import => self.plan.to.clone(),
        };
        let cx = StepContext {
            http_client: ctx.http_client,
            work: self.work,
            package_root: half.package_root,
            cache_root: &ctx.settings.app_sources_cache(),
            vars: StepVars {
                work: self.work.to_path_buf(),
                package: half.package_root,
                vibe_home: ctx.settings.path(),
                app_root: Some(match half.side {
                    MigrationSide::Export => &self.from_root,
                    MigrationSide::Import => &self.to_root,
                }),
                features: &[],
                params: &[],
                migration: true,
            },
            platform: &self.platform,
            arch: &self.arch,
            app_root: match half.side {
                MigrationSide::Export => &self.from_root,
                MigrationSide::Import => &self.to_root,
            },
            phase: StepPhase::Prepare,
            offline: ctx.offline,
            force: false,
            update_in_place: false,
            io: ChildIo {
                relay: ctx.relay(),
                log: Some(&log),
                stdout_log: None,
                task: Some(&task),
                interrupt: ctx.interrupt,
                stdin_null: ctx.panel.is_some(),
                panel: ctx.panel,
            },
            env: &env,
            remap: vibe_core::remap::active(),
            package_key: &package_key,
            served: &served,
            transfer: &transfer,
            migration: Some(&migration),
            ecosystem: ecosystem.as_ref(),
        };
        match run_step(&cx, half.step) {
            Ok(_) => {
                task.finish();
                Ok(())
            }
            Err(error) => {
                task.fail(error.to_string());
                Err(StepError::MigrationFailed(Box::new(MigrationFailure {
                    slot: self.plan.slot.clone(),
                    entry: entry.id.clone(),
                    side: half.side.as_str(),
                    step: half.step.id.clone(),
                    work: self.work.display().to_string(),
                    log: log.display().to_string(),
                    source: Box::new(error),
                }))
                .into())
            }
        }
    }
}

/// `<work>/<entry id>/export.log` or `…/import.log`.
fn log_path(dir: &Path, side: MigrationSide) -> PathBuf {
    dir.join(format!("{}.log", side.as_str()))
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// The PATH front of one installation: its own proxy directory and its
/// payload `bin`, built by the very function a step requirement uses
/// (`PROP-065#app-dependencies`).
pub(crate) fn installation_paths(
    ctx: &OperationContext<'_>,
    index: &Index,
    key: &str,
    lock: &crate::MutationLock,
) -> Result<(StepPaths, PathBuf), AppError> {
    let storage = crate::storage::Storage::load(ctx.settings)?;
    let record = index
        .installations
        .get(key)
        .ok_or_else(|| AppError::NotInstalled {
            package: key.to_owned(),
            home: ctx.settings.home_name(),
            fix: "install it before migrating".to_owned(),
        })?;
    let exposure: Exposure =
        crate::exposure::exposure_of(ctx.settings, &storage, key, record, ctx.proxy_image, lock)?;
    let root =
        crate::launch::app_root(record, &storage).map_err(|reason| AppError::Unavailable {
            key: key.to_owned(),
            reason,
        })?;
    Ok((
        StepPaths {
            option: vec![exposure],
            steps: Default::default(),
        },
        root,
    ))
}
