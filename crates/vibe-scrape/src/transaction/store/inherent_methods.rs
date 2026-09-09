macro_rules! system_transaction_store_inherent_methods {
    () => {
    fn external(&self) -> Result<&ExternalStore, TransactionError> {
        self.external.as_ref().ok_or_else(|| {
            store_error("external state root has not been proven against the project")
        })
    }

    fn proven_display_root(&self) -> Result<&str, TransactionError> {
        self.proven_display_root
            .as_deref()
            .ok_or_else(|| store_error("pinned project display root is absent"))
    }

    fn locked_project(&self) -> Result<&ProjectKey, TransactionError> {
        if self.external_lock.is_none() {
            return Err(store_error("external project lock is not held"));
        }
        self.locked_project
            .as_ref()
            .ok_or_else(|| store_error("locked project identity is absent"))
    }

    fn require_locked(&self, project: &ProjectKey) -> Result<(), TransactionError> {
        self.external_lock
            .as_ref()
            .ok_or_else(|| store_error("external project lock is not held"))?
            .require_still_named()
            .map_err(|error| store_error(format!("rechecking held project lock: {error:#}")))?;
        let locked = self.locked_project()?;
        if locked != project {
            return Err(store_error(
                "operation names a project other than the locked project",
            ));
        }
        Ok(())
    }

    fn ensure_project_home(
        &self,
        project: &ProjectKey,
    ) -> Result<ExternalDirectory, TransactionError> {
        let root = self
            .external()?
            .root_directory()
            .map_err(|error| store_error(format!("opening external root capability: {error:#}")))?;
        let (transactions, _, durability) = root
            .ensure_child(TRANSACTIONS_DIRECTORY)
            .map_err(|error| store_error(format!("ensuring transaction directory: {error:#}")))?;
        require_optional_sync(durability, "transaction-home parent")?;
        let (project_home, _, durability) = transactions
            .ensure_child(project_component(project)?)
            .map_err(|error| {
                store_error(format!("ensuring project transaction home: {error:#}"))
            })?;
        require_optional_sync(durability, "project transaction-home parent")?;
        Ok(project_home)
    }

    fn open_project_home(
        &self,
        project: &ProjectKey,
    ) -> Result<Option<ExternalDirectory>, TransactionError> {
        self.external()?
            .open_directory(&format!(
                "{}/{}",
                TRANSACTIONS_DIRECTORY,
                project_component(project)?
            ))
            .map_err(|error| store_error(format!("opening project transaction home: {error:#}")))
    }

    fn transaction_directory(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<Option<ExternalDirectory>, TransactionError> {
        let Some(project_home) = self.open_project_home(project)? else {
            return Ok(None);
        };
        project_home
            .open_child(transaction_component(transaction)?)
            .map_err(|error| store_error(format!("opening transaction directory: {error:#}")))
    }

    fn load_owner(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<(OwnerWire, ExternalDirectory), TransactionError> {
        let project_home = self
            .open_project_home(project)?
            .ok_or_else(|| store_error("project transaction home is absent"))?;
        let name = transaction_component(transaction)?;
        let root_state = project_home
            .inspect_child_state(name)
            .map_err(|error| store_error(format!("inspecting transaction root: {error:#}")))?
            .ok_or_else(|| store_error("transaction root is absent"))?;
        if root_state.kind != EntryStateKind::Directory {
            return Err(store_error("transaction root is not a directory"));
        }
        let directory = project_home
            .open_child(name)
            .map_err(|error| store_error(format!("opening transaction root: {error:#}")))?
            .ok_or_else(|| store_error("transaction root vanished while opening"))?;
        let owner = directory
            .read_stable_bounded(OWNER_FILE, MAX_OWNER_BYTES)
            .map_err(|error| store_error(format!("reading transaction owner seal: {error:#}")))?
            .ok_or_else(|| store_error("transaction owner seal is absent"))?;
        let owner: OwnerWire = strict_json_parse(&owner.bytes, "owner seal")?;
        let expected_token = transaction_ownership_token(project, transaction);
        if owner.schema != 3
            || owner.project_key != project.0
            || owner.transaction_id != transaction.0
            || !owner
                .journal_intent_sha256
                .strip_prefix("sha256:")
                .is_some_and(valid_lower_hex_digest)
            || owner.ownership_token != expected_token
            || owner.entry_identity != root_state.identity.as_str()
            || !owner
                .workspace_project_identity
                .strip_prefix("sha256:")
                .is_some_and(valid_lower_hex_digest)
        {
            return Err(store_error(
                "transaction owner seal differs from the selected project/root identity",
            ));
        }
        OwnedDirectoryIdentity::from_token(&owner.directory_identity).map_err(|error| {
            store_error(format!("invalid owned-directory identity seal: {error:#}"))
        })?;
        OwnedDirectoryIdentity::from_token(&owner.workspace_directory_identity).map_err(
            |error| store_error(format!("invalid workspace directory identity: {error:#}")),
        )?;
        EntryIdentity::from_token(&owner.workspace_entry_identity)
            .map_err(|error| store_error(format!("invalid workspace entry identity: {error:#}")))?;
        Ok((owner, directory))
    }

    fn load_journal(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<Journal, TransactionError> {
        let (owner, directory) = self.load_owner(project, transaction)?;
        let bytes = directory
            .read_stable_bounded(JOURNAL_FILE, MAX_TRANSACTION_JOURNAL_BYTES)
            .map_err(|error| store_error(format!("reading transaction journal: {error:#}")))?
            .ok_or_else(|| store_error("transaction journal is absent"))?
            .bytes;
        let journal: Journal = strict_json_parse(&bytes, "transaction journal")?;
        if journal.transaction_id != *transaction {
            return Err(store_error(
                "transaction directory and journal identity differ",
            ));
        }
        if journal_intent_sha256(&journal)? != owner.journal_intent_sha256 {
            return Err(store_error(
                "transaction journal immutable intent differs from its owner seal",
            ));
        }
        validate::journal(&journal, project, self.proven_display_root()?)?;
        validate_workspace_owner(&directory, &owner, &journal)?;
        validate_snapshot_bounds(&journal.snapshots)?;
        Ok(journal)
    }

    fn write_journal(&self, journal: &Journal, initial: bool) -> Result<(), TransactionError> {
        let bytes = strict_json_bytes(
            journal,
            MAX_TRANSACTION_JOURNAL_BYTES,
            "transaction journal",
        )?;
        self.write_journal_bytes(journal, initial, &bytes)
    }

    fn write_journal_bytes(
        &self,
        journal: &Journal,
        initial: bool,
        bytes: &[u8],
    ) -> Result<(), TransactionError> {
        let (_, directory) = self.load_owner(&journal.project_key, &journal.transaction_id)?;
        let existing = directory
            .read_stable_bounded(JOURNAL_FILE, MAX_TRANSACTION_JOURNAL_BYTES)
            .map_err(|error| store_error(format!("checking transaction journal: {error:#}")))?;
        if initial && existing.is_some() {
            return Err(store_error("new transaction journal already exists"));
        }
        if !initial && existing.is_none() {
            return Err(store_error(
                "transaction journal vanished before persistence",
            ));
        }
        self.write_durable(
            &journal_relative(&journal.project_key, &journal.transaction_id)?,
            bytes,
            "transaction journal",
        )?;
        let observed = directory
            .read_stable_bounded(JOURNAL_FILE, MAX_TRANSACTION_JOURNAL_BYTES)
            .map_err(|error| store_error(format!("re-reading transaction journal: {error:#}")))?
            .ok_or_else(|| store_error("transaction journal vanished after persistence"))?;
        if observed.bytes != bytes {
            return Err(store_error("transaction journal changed after persistence"));
        }
        Ok(())
    }

    fn write_durable(
        &self,
        relative: &str,
        bytes: &[u8],
        label: &str,
    ) -> Result<(), TransactionError> {
        let write = self
            .external()?
            .write_durable(relative, bytes)
            .map_err(|error| store_error(format!("publishing {label}: {error}")))?;
        if !write.file_synced {
            return Err(store_error(format!("{label} data was not durably flushed")));
        }
        require_namespace_checkpoint(write.parent, &format!("{label} parent"))?;
        for sync in write.directory_syncs {
            require_namespace_checkpoint(
                sync.durability,
                &format!("{label} directory `{}`", sync.directory.display()),
            )?;
        }
        Ok(())
    }

    fn read_snapshot_record(
        &self,
        journal: &Journal,
        record: &SnapshotRecord,
    ) -> Result<Option<Vec<u8>>, TransactionError> {
        validate_snapshot_record_bound(record)?;
        let relative =
            snapshot_relative(&journal.project_key, &journal.transaction_id, &record.name)?;
        let Some(snapshot) = self
            .external()?
            .read_stable_bounded(
                &relative,
                usize::try_from(record.bytes).map_err(|_| {
                    store_error(format!(
                        "snapshot `{}` size is not addressable",
                        record.name
                    ))
                })?,
            )
            .map_err(|error| {
                store_error(format!("reading snapshot `{}`: {error:#}", record.name))
            })?
        else {
            return Ok(None);
        };
        verify_snapshot_bytes(record, &snapshot.bytes)?;
        let (parent, name) = split_parent(&record.name)?;
        let transaction = self
            .transaction_directory(&journal.project_key, &journal.transaction_id)?
            .ok_or_else(|| store_error("transaction root vanished while reading snapshot"))?;
        let snapshots = transaction
            .open_child(SNAPSHOTS_DIRECTORY)
            .map_err(|error| store_error(format!("opening snapshot root: {error:#}")))?
            .ok_or_else(|| store_error("snapshot root vanished while reading snapshot"))?;
        let state = if parent.is_empty() {
            snapshots.inspect_child_state(name)
        } else {
            let directory = open_descendant(&snapshots, parent)?
                .ok_or_else(|| store_error(format!("snapshot parent `{parent}` vanished")))?;
            directory.inspect_child_state(name)
        }
        .map_err(|error| store_error(format!("inspecting snapshot `{}`: {error:#}", record.name)))?
        .ok_or_else(|| store_error(format!("snapshot `{}` vanished", record.name)))?;
        let expected_sha256 = record
            .sha256
            .0
            .strip_prefix("sha256:")
            .ok_or_else(|| store_error("snapshot journal digest has no sha256 prefix"))?;
        if state.kind != EntryStateKind::File
            || state.sha256.as_deref() != Some(expected_sha256)
            || state.bytes != Some(record.bytes)
            || state.unix_mode != record.mode
        {
            return Err(store_error(format!(
                "snapshot `{}` metadata differs from its journal record",
                record.name
            )));
        }
        Ok(Some(snapshot.bytes))
    }

    fn snapshot_entries(
        &self,
        journal: &Journal,
    ) -> Result<Vec<(String, EntryStateKind)>, TransactionError> {
        let Some(transaction) =
            self.transaction_directory(&journal.project_key, &journal.transaction_id)?
        else {
            return Err(store_error("transaction root is absent"));
        };
        let Some(root) = transaction
            .open_child(SNAPSHOTS_DIRECTORY)
            .map_err(|error| store_error(format!("opening snapshot root: {error:#}")))?
        else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();
        collect_external_entries(&root, "", &mut entries, 0)?;
        Ok(entries)
    }

    fn resume_retirement(
        &mut self,
        project: &ProjectKey,
        transaction: &TransactionId,
    ) -> Result<(), TransactionError> {
        let project_home = match self.open_project_home(project)? {
            Some(home) => home,
            None => return Ok(()),
        };
        let sidecar_name = retirement_name(transaction)?;
        let mut wire = match project_home
            .read_stable_bounded(&sidecar_name, MAX_RETIREMENT_BYTES)
            .map_err(|error| store_error(format!("reading retirement checkpoint: {error:#}")))?
        {
            Some(bytes) => {
                let wire: RetirementWire =
                    strict_json_parse(&bytes.bytes, "retirement checkpoint")?;
                validate_retirement_identity(&wire, project, transaction)?;
                self.validate_stable_retirement_report(&wire)?;
                wire
            }
            None => {
                let durable = self.load_journal(project, transaction)?;
                if durable.state != TransactionState::Complete {
                    return Err(store_error(
                        "retirement cannot begin before the durable journal is complete",
                    ));
                }
                self.verify_snapshot_progress(&durable)?;
                let stable_report = self.require_stable_complete_report(&durable)?;
                let (owner, transaction_directory) = match self.load_owner(project, transaction) {
                    Ok(value) => value,
                    Err(error)
                        if project_home
                            .open_child(transaction_component(transaction)?)
                            .map_err(|source| {
                                store_error(format!(
                                    "checking retired transaction root: {source:#}"
                                ))
                            })?
                            .is_none() =>
                    {
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                };
                validate_transaction_home_shape(&transaction_directory, &durable, &owner)?;
                let manifest = observe_safefs_manifest(&transaction_directory)?;
                // Close the pre-sidecar adoption window: the manifest may
                // contain only the exact authority re-proved after observation.
                // Anything inserted during the observation is therefore
                // rejected before its identity can become cleanup authority.
                validate_transaction_home_shape(&transaction_directory, &durable, &owner)?;
                self.verify_snapshot_progress(&durable)?;
                if self.load_journal(project, transaction)? != durable {
                    return Err(store_error(
                        "transaction journal changed while sealing retirement manifest",
                    ));
                }
                let wire = RetirementWire {
                    schema: 2,
                    project_key: project.0.clone(),
                    transaction_id: transaction.0.clone(),
                    stable_report_sha256: sha256_bytes(&stable_report),
                    ownership_token: owner.ownership_token,
                    directory_identity: owner.directory_identity,
                    manifest: safefs_manifest_to_wire(&manifest),
                    completed: Vec::new(),
                    active: None,
                    tree_removed: false,
                };
                self.write_retirement(project, transaction, &wire)?;
                wire
            }
        };

        let identity = OwnedDirectoryIdentity::from_token(&wire.directory_identity)
            .map_err(|error| store_error(format!("invalid retirement root identity: {error:#}")))?;
        let manifest = safefs_manifest_from_wire(wire.manifest.clone())?;
        let mut progress = OwnedTreeCleanupProgress::from_completed(wire.completed.clone())
            .map_err(|error| store_error(format!("invalid retirement progress: {error:#}")))?;

        loop {
            if wire.tree_removed {
                if project_home
                    .open_child(transaction_component(transaction)?)
                    .map_err(|error| {
                        store_error(format!("checking retired transaction: {error:#}"))
                    })?
                    .is_some()
                {
                    return Err(store_error(
                        "retirement checkpoint says removed but transaction root is present",
                    ));
                }
                return self.remove_retirement_sidecar(&project_home, &sidecar_name);
            }

            if let Some(active) = wire.active.take() {
                let intent = cleanup_intent_from_wire(active)?;
                let completion = project_home
                    .execute_owned_child_retirement(
                        transaction_component(transaction)?,
                        &wire.ownership_token,
                        &identity,
                        &manifest,
                        &progress,
                        &intent,
                    )
                    .map_err(map_cleanup_error)?;
                require_namespace_checkpoint(
                    completion.durability(),
                    &format!("retirement step `{}`", completion.progress_key()),
                )?;
                progress.record(&completion).map_err(|error| {
                    store_error(format!("recording retirement progress: {error:#}"))
                })?;
                wire.completed = progress.completed().to_vec();
                wire.tree_removed = intent.root;
                self.write_retirement(project, transaction, &wire)?;
                continue;
            }

            match project_home
                .prepare_owned_child_retirement(
                    transaction_component(transaction)?,
                    &wire.ownership_token,
                    &identity,
                    &manifest,
                    &progress,
                )
                .map_err(map_cleanup_error)?
            {
                CleanupPreparation::Complete => {
                    wire.tree_removed = true;
                    self.write_retirement(project, transaction, &wire)?;
                }
                CleanupPreparation::Intent(intent) => {
                    wire.active = Some(cleanup_intent_to_wire(&intent));
                    self.write_retirement(project, transaction, &wire)?;
                }
            }
        }
    }

    fn write_retirement(
        &self,
        project: &ProjectKey,
        transaction: &TransactionId,
        wire: &RetirementWire,
    ) -> Result<(), TransactionError> {
        let bytes = strict_json_bytes(wire, MAX_RETIREMENT_BYTES, "retirement checkpoint")?;
        self.write_durable(
            &retirement_relative(project, transaction)?,
            &bytes,
            "retirement checkpoint",
        )
    }

    fn validate_stable_retirement_report(
        &self,
        wire: &RetirementWire,
    ) -> Result<(), TransactionError> {
        let transaction = TransactionId(wire.transaction_id.clone());
        let observed = self
            .external()?
            .read_stable_bounded(&report_relative(&transaction)?, MAX_CANONICAL_REPORT_BYTES)
            .map_err(|error| store_error(format!("reading retirement stable report: {error:#}")))?
            .ok_or_else(|| store_error("retirement stable report is absent"))?;
        if sha256_bytes(&observed.bytes) != wire.stable_report_sha256 {
            return Err(store_error(
                "retirement stable report differs from the sealed canonical report",
            ));
        }
        let report: report_wire::Report =
            strict_json_parse(&observed.bytes, "retirement stable report")?;
        if report.schema != 1
            || report.transaction_id != wire.transaction_id
            || report.cleanup != report_wire::ReportCleanup::Complete
        {
            return Err(store_error(
                "retirement stable report is not the selected cleanup-complete transaction",
            ));
        }
        Ok(())
    }

    fn remove_retirement_sidecar(
        &self,
        project_home: &ExternalDirectory,
        sidecar_name: &str,
    ) -> Result<(), TransactionError> {
        let Some(state) = project_home
            .inspect_child_state(sidecar_name)
            .map_err(|error| store_error(format!("inspecting retirement checkpoint: {error:#}")))?
        else {
            return Ok(());
        };
        let durability = project_home
            .remove_file_expected(sidecar_name, &state)
            .map_err(map_cleanup_error)?;
        require_namespace_checkpoint(durability, "retirement-checkpoint removal")
    }
    };
}
