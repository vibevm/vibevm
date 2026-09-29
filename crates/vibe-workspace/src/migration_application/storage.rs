//! Capability-relative recovery storage; no unknown occupant is removed.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use vibe_safefs::{ExternalProjectLock, ExternalStore, FileIdentity, Pinned, StableFileSnapshot};
use vibe_wire::generated::migration::e1::{
    application_journal::ApplicationJournal, operation_report::*,
};

use super::super::{OwnedManifestMigrationFile, ReportBuilder, identity, validation};
use super::{
    Checked, boundary, fault, observe_durability, publication_failure, residue, set_effect, warning,
};

pub(super) struct Target {
    pub parent: Pinned,
    pub name: String,
    parents: Vec<String>,
    parent_identity: FileIdentity,
}

impl Target {
    pub fn open(grant: &OwnedManifestMigrationFile) -> Checked<Self> {
        let (parents, name) =
            vibe_safefs::split_relative(grant.target_relative()).map_err(|_| {
                fault(
                    ApplicationErrorCode::AuthorityRefused,
                    "target",
                    "unsafe target operand",
                )
            })?;
        let refs: Vec<_> = parents.iter().map(String::as_str).collect();
        let parent = grant.project().dir(&refs, false).map_err(|_| {
            fault(
                ApplicationErrorCode::AuthorityRefused,
                "target",
                "existing target parent could not be pinned",
            )
        })?;
        let parent_identity = parent.identity().map_err(|_| {
            fault(
                ApplicationErrorCode::AuthorityRefused,
                "target",
                "target parent identity unavailable",
            )
        })?;
        Ok(Self {
            parent,
            name,
            parents,
            parent_identity,
        })
    }

    pub fn observe(
        &self,
        grant: &OwnedManifestMigrationFile,
    ) -> Checked<Option<StableFileSnapshot>> {
        let refs: Vec<_> = self.parents.iter().map(String::as_str).collect();
        let named = grant.project().dir(&refs, false).map_err(|_| {
            fault(
                ApplicationErrorCode::SourceChanged,
                "target",
                "target parent no longer occupies the admitted namespace",
            )
        })?;
        if named.identity().ok() != Some(self.parent_identity) {
            return Err(fault(
                ApplicationErrorCode::SourceChanged,
                "target",
                "target parent identity changed",
            ));
        }
        grant
            .project()
            .read_file_snapshot_bounded_in(&self.parent, &self.name, grant.journal_read_limit())
            .map_err(|_| {
                fault(
                    ApplicationErrorCode::SourceChanged,
                    "target",
                    "stable regular single-link target observation failed",
                )
            })
    }

    pub fn stage_relative(&self, stage: &str) -> String {
        if self.parents.is_empty() {
            stage.to_owned()
        } else {
            format!("{}/{stage}", self.parents.join("/"))
        }
    }
}

pub(super) struct Store {
    pub store: ExternalStore,
    pub lock: ExternalProjectLock,
    pub project_binding: String,
    pub store_binding: String,
    pub key: String,
}

impl Store {
    pub fn acquire(
        grant: &OwnedManifestMigrationFile,
        id: &str,
        create: bool,
        report: &mut ReportBuilder,
    ) -> Checked<Option<Self>> {
        grant.validate_recovery_store_policy()?;
        let project_binding = grant.project().identity_token().map_err(|_| {
            fault(
                ApplicationErrorCode::AuthorityRefused,
                "project",
                "physical project binding unavailable",
            )
        })?;
        let existing =
            ExternalStore::open_existing_disjoint(grant.recovery_store_root(), grant.project())
                .map_err(|_| {
                    fault(
                        ApplicationErrorCode::StoreNotDisjoint,
                        "store",
                        "selected store is not accessible, no-follow and disjoint",
                    )
                })?;
        let store = if let Some(store) = existing {
            store
        } else if create {
            grant.validate_recovery_store_policy()?;
            boundary("before_store_bootstrap");
            set_effect(report, EffectKind::StoreBootstrap, EffectState::Pending);
            let opened = ExternalStore::open_or_create_disjoint(
                grant.recovery_store_root(),
                grant.project(),
            );
            let store = match opened {
                Ok(store) => store,
                Err(_) => {
                    set_effect(report, EffectKind::StoreBootstrap, EffectState::Uncertain);
                    warning(
                        report,
                        WarningCode::ResidueRetained,
                        WarningAction::ManualResolution,
                    );
                    return Err(fault(
                        ApplicationErrorCode::JournalIo,
                        "store",
                        "store bootstrap failed and may have created namespace entries",
                    ));
                }
            };
            set_effect(report, EffectKind::StoreBootstrap, EffectState::Completed);
            for sync in store.bootstrap_durability() {
                observe_durability(report, sync.durability)?;
            }
            boundary("after_store_bootstrap");
            store
        } else {
            return Ok(None);
        };
        let store_binding = store.identity_token().map_err(|_| {
            fault(
                ApplicationErrorCode::AuthorityRefused,
                "store",
                "physical store binding unavailable",
            )
        })?;
        let lock = if create {
            grant.validate_recovery_store_policy()?;
            boundary("before_barrier_create");
            set_effect(report, EffectKind::Barrier, EffectState::Pending);
            match store.open_and_lock_project(&project_binding) {
                Ok(lock) => lock,
                Err(_) => {
                    set_effect(report, EffectKind::Barrier, EffectState::Uncertain);
                    warning(
                        report,
                        WarningCode::ResidueRetained,
                        WarningAction::ManualResolution,
                    );
                    return Err(fault(
                        ApplicationErrorCode::BarrierUnavailable,
                        "barrier",
                        "barrier creation/acquisition failed and may leave store state",
                    ));
                }
            }
        } else {
            match store.lock_existing_project(&project_binding) {
                Ok(Some(lock)) => lock,
                Ok(None) => {
                    return Err(fault(
                        ApplicationErrorCode::BarrierUnavailable,
                        "barrier",
                        "existing barrier is absent; inspection/recovery creates nothing",
                    ));
                }
                Err(_) => {
                    return Err(fault(
                        ApplicationErrorCode::BarrierUnavailable,
                        "barrier",
                        "existing barrier could not be acquired",
                    ));
                }
            }
        };
        lock.require_still_named().map_err(|_| {
            fault(
                ApplicationErrorCode::BarrierUnavailable,
                "barrier",
                "held barrier no longer names its acquired object",
            )
        })?;
        set_effect(report, EffectKind::Barrier, EffectState::Completed);
        report.evidence_mut().guarantees.concurrency = GuaranteesConcurrency::ParticipatingClients;
        report.evidence_mut().guarantees.barrier_binding =
            Some(format!("{project_binding};{store_binding}"));
        if create {
            boundary("after_barrier_create");
        }
        let key = identity::operation_storage_key(&project_binding, id);
        Ok(Some(Self {
            store,
            lock,
            project_binding,
            store_binding,
            key,
        }))
    }

    pub fn image_path(&self) -> String {
        format!("migrations/{}/before.bin", self.key)
    }
    pub fn journal_path(&self) -> String {
        format!("migrations/{}/journal.json", self.key)
    }

    pub fn still_named(&self) -> Checked<()> {
        self.lock.require_still_named().map_err(|_| {
            fault(
                ApplicationErrorCode::BarrierUnavailable,
                "barrier",
                "held barrier no longer occupies its named object",
            )
        })
    }

    pub fn read(
        &self,
        path: &str,
        grant: &OwnedManifestMigrationFile,
    ) -> Checked<Option<StableFileSnapshot>> {
        self.store
            .read_stable_bounded(path, grant.journal_read_limit())
            .map_err(|_| {
                fault(
                    ApplicationErrorCode::JournalIo,
                    "store",
                    "bounded stable recovery-state observation failed",
                )
            })
    }

    pub fn load(
        &self,
        grant: &OwnedManifestMigrationFile,
        id: &str,
        report: &mut ReportBuilder,
    ) -> Checked<
        Option<(
            ApplicationJournal,
            vibe_wire::generated::migration::e1::plan_report::Plan,
        )>,
    > {
        let Some(snapshot) = self.read(&self.journal_path(), grant)? else {
            if self.read(&self.image_path(), grant)?.is_some() {
                residue(
                    report,
                    ResidueLocation::Store,
                    self.image_path(),
                    ResidueOwnership::Unproven,
                );
                warning(
                    report,
                    WarningCode::ResidueRetained,
                    WarningAction::ManualResolution,
                );
            }
            return Ok(None);
        };
        let journal: ApplicationJournal =
            serde_json::from_slice(&snapshot.bytes).map_err(|_| {
                residue(
                    report,
                    ResidueLocation::Store,
                    self.journal_path(),
                    ResidueOwnership::Unproven,
                );
                warning(
                    report,
                    WarningCode::ResidueRetained,
                    WarningAction::ManualResolution,
                );
                fault(
                    ApplicationErrorCode::JournalInvalid,
                    "journal",
                    "retained checkpoint does not match its strict generated shape",
                )
            })?;
        let plan = validation::validate_journal(&journal, grant).map_err(|error| {
            residue(
                report,
                ResidueLocation::Store,
                self.journal_path(),
                ResidueOwnership::Unproven,
            );
            warning(
                report,
                WarningCode::ResidueRetained,
                WarningAction::ManualResolution,
            );
            error
        })?;
        if journal.operation_id != id
            || journal.project_binding != self.project_binding
            || journal.store_binding != self.store_binding
        {
            return Err(fault(
                ApplicationErrorCode::IntentMismatch,
                "journal",
                "retained intent belongs to another operation or physical root",
            ));
        }
        let image = self.read(&self.image_path(), grant)?.ok_or_else(|| {
            fault(
                ApplicationErrorCode::JournalInvalid,
                "before_image",
                "retained approved image is missing",
            )
        })?;
        if image.bytes != plan.before.as_bytes()
            || image.sha256 != journal.before_raw_sha256
            || image.size.to_string() != journal.before_length
        {
            return Err(fault(
                ApplicationErrorCode::JournalInvalid,
                "before_image",
                "retained approved image differs from validated intent",
            ));
        }
        set_effect(report, EffectKind::BeforeImage, EffectState::Completed);
        set_effect(report, EffectKind::Journal, EffectState::Completed);
        // Surviving exact evidence establishes process recovery, not a prior
        // persistent-media flush that this invocation did not observe.
        super::unconfirmed(report);
        Ok(Some((journal, plan)))
    }

    pub fn retain_image(
        &self,
        bytes: &[u8],
        grant: &OwnedManifestMigrationFile,
        report: &mut ReportBuilder,
    ) -> Checked<()> {
        if bytes.len() > grant.journal_read_limit() {
            return Err(fault(
                ApplicationErrorCode::PreservationUnavailable,
                "before_image",
                "approved image exceeds the admitting caller's observation ceiling",
            ));
        }
        if let Some(existing) = self.read(&self.image_path(), grant)? {
            if existing.bytes != bytes {
                residue(
                    report,
                    ResidueLocation::Store,
                    self.image_path(),
                    ResidueOwnership::Unproven,
                );
                return Err(fault(
                    ApplicationErrorCode::JournalInvalid,
                    "before_image",
                    "existing retained image is a different occupant",
                ));
            }
            // Matching bytes alone are not a synchronization receipt.
            super::unconfirmed(report);
            self.confirm_image(&existing, grant, report)?;
        } else {
            self.still_named()?;
            grant.validate_recovery_store_policy()?;
            boundary("before_before_image");
            let prior = super::effect_state(report, &EffectKind::BeforeImage);
            set_effect(report, EffectKind::BeforeImage, EffectState::Pending);
            match self.store.write_durable(&self.image_path(), bytes) {
                Ok(write) => {
                    set_effect(report, EffectKind::BeforeImage, EffectState::Completed);
                    if let Err(error) = super::observe_write(report, &write) {
                        set_effect(report, EffectKind::BeforeImage, EffectState::Uncertain);
                        return Err(error);
                    }
                }
                Err(error) => {
                    publication_failure(
                        report,
                        EffectKind::BeforeImage,
                        prior,
                        &error,
                        ResidueLocation::Store,
                        self.store.path(),
                    );
                    return Err(fault(
                        ApplicationErrorCode::JournalIo,
                        "before_image",
                        "retained image publication failed",
                    ));
                }
            }
            boundary("after_before_image");
        }
        let verified = self.read(&self.image_path(), grant)?.ok_or_else(|| {
            fault(
                ApplicationErrorCode::JournalInvalid,
                "before_image",
                "retained image is absent after preparation",
            )
        })?;
        if verified.bytes != bytes {
            return Err(fault(
                ApplicationErrorCode::JournalInvalid,
                "before_image",
                "retained image failed exact verification",
            ));
        }
        set_effect(report, EffectKind::BeforeImage, EffectState::Completed);
        Ok(())
    }

    pub fn confirm_image(
        &self,
        expected: &StableFileSnapshot,
        grant: &OwnedManifestMigrationFile,
        report: &mut ReportBuilder,
    ) -> Checked<()> {
        self.still_named()?;
        grant.validate_recovery_store_policy()?;
        boundary("before_retained_image_sync");
        let durability = self
            .store
            .sync_stable_file_bounded(&self.image_path(), grant.journal_read_limit(), expected)
            .map_err(|_| {
                super::unconfirmed(report);
                set_effect(report, EffectKind::BeforeImage, EffectState::Uncertain);
                fault(
                    ApplicationErrorCode::SynchronizationFailed,
                    "before_image",
                    "retained image synchronization could not be confirmed",
                )
            })?;
        // The owner has confirmed actual held-file data sync and returned its
        // parent tier. The tier is evaluated after this crash boundary.
        boundary("after_retained_image_sync");
        // This result confirms this held image's file sync and its actual
        // parent tier; it does not authenticate other past journal boundaries.
        super::observe_durability(report, durability)?;
        Ok(())
    }

    pub fn checkpoint(
        &self,
        journal: &ApplicationJournal,
        previous: Option<&ApplicationJournal>,
        grant: &OwnedManifestMigrationFile,
        report: &mut ReportBuilder,
        before_hook: &'static str,
        after_hook: &'static str,
    ) -> Checked<()> {
        validation::validate_journal(journal, grant)?;
        let current = self.read(&self.journal_path(), grant)?;
        match (previous, current) {
            (None, None) => {}
            (Some(expected), Some(actual)) => {
                let decoded: ApplicationJournal =
                    serde_json::from_slice(&actual.bytes).map_err(|_| {
                        fault(
                            ApplicationErrorCode::JournalInvalid,
                            "journal",
                            "existing checkpoint is malformed",
                        )
                    })?;
                if &decoded != expected || identity::intent_identity(expected) != journal.intent_id
                {
                    return Err(fault(
                        ApplicationErrorCode::IntentMismatch,
                        "journal",
                        "checkpoint changed before update",
                    ));
                }
            }
            _ => {
                return Err(fault(
                    ApplicationErrorCode::JournalInvalid,
                    "journal",
                    "unexpected journal occupancy before update",
                ));
            }
        }
        let bytes = serde_json::to_vec(journal).map_err(|_| {
            fault(
                ApplicationErrorCode::JournalInvalid,
                "journal",
                "generated checkpoint serialization failed",
            )
        })?;
        if bytes.len() > grant.journal_read_limit() {
            return Err(fault(
                ApplicationErrorCode::JournalInvalid,
                "journal",
                "checkpoint exceeds the admitting caller's observation ceiling",
            ));
        }
        self.still_named()?;
        grant.validate_recovery_store_policy()?;
        boundary(before_hook);
        let prior = super::effect_state(report, &EffectKind::Journal);
        set_effect(report, EffectKind::Journal, EffectState::Pending);
        match self.store.write_durable(&self.journal_path(), &bytes) {
            Ok(write) => {
                set_effect(report, EffectKind::Journal, EffectState::Completed);
                if let Err(error) = super::observe_write(report, &write) {
                    set_effect(report, EffectKind::Journal, EffectState::Uncertain);
                    return Err(error);
                }
            }
            Err(error) => {
                publication_failure(
                    report,
                    EffectKind::Journal,
                    prior,
                    &error,
                    ResidueLocation::Store,
                    self.store.path(),
                );
                return Err(fault(
                    ApplicationErrorCode::JournalIo,
                    "journal",
                    "checkpoint publication failed",
                ));
            }
        }
        let verified = self.read(&self.journal_path(), grant)?.ok_or_else(|| {
            fault(
                ApplicationErrorCode::JournalInvalid,
                "journal",
                "checkpoint absent after publication",
            )
        })?;
        if verified.bytes != bytes {
            set_effect(report, EffectKind::Journal, EffectState::Uncertain);
            return Err(fault(
                ApplicationErrorCode::JournalInvalid,
                "journal",
                "published checkpoint differs from selected generated bytes",
            ));
        }
        boundary(after_hook);
        Ok(())
    }
}
