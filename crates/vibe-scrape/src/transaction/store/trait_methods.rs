macro_rules! system_transaction_store_trait_methods {
    () => {
    fn prove_outside_project(
        &mut self,
        project_display_root: &str,
    ) -> Result<(), TransactionError> {
        // A previous operation's lock is released only at the explicit start
        // of the next proof/lock sequence.
        self.external_lock = None;
        self.locked_project = None;
        self.external = None;
        self.proven_project = None;
        self.proven_display_root = None;
        let project = Project::open(Path::new(project_display_root)).map_err(|error| {
            TransactionError::Filesystem(format!("pinning scrape project: {error:#}"))
        })?;
        let identity_token = project.identity_token().map_err(|error| {
            TransactionError::Filesystem(format!("identifying pinned scrape project: {error:#}"))
        })?;
        let proven_project = derive_project_key(&identity_token);
        let external = ExternalStore::open_or_create_disjoint(&self.state_root, &project).map_err(
            |error| store_error(format!("opening external transaction root: {error:#}")),
        )?;
        external.require_durable_bootstrap().map_err(|error| {
            store_error(format!(
                "external transaction root is not durable: {error:#}"
            ))
        })?;
        self.external = Some(external);
        self.proven_project = Some(proven_project);
        self.proven_display_root = Some(project_display_root.to_owned());
        Ok(())
    }

    fn lock_project(&mut self, project: &ProjectKey) -> Result<ProjectLock, TransactionError> {
        validate_project_key(project)?;
        if self.external_lock.is_some() {
            return Err(store_error("a scrape project lock is already held"));
        }
        if self.proven_project.as_ref() != Some(project) {
            return Err(store_error(
                "project key does not identify the project pinned for the external-store proof",
            ));
        }
        let lock = self
            .external()?
            .open_and_lock_project(&project.0)
            .map_err(|error| store_error(format!("locking external project state: {error:#}")))?;
        self.external_lock = Some(lock);
        self.locked_project = Some(project.clone());
        Ok(ProjectLock::acquired())
    }

    fn pending(&mut self, project: &ProjectKey) -> Result<Option<Journal>, TransactionError> {
        self.require_locked(project)?;
        let Some(project_home) = self.open_project_home(project)? else {
            return Ok(None);
        };
        let mut transaction_ids = Vec::new();
        let mut retirement_ids = Vec::new();
        for name in project_home
            .child_names_bounded(MAX_DIRECTORY_CHILDREN)
            .map_err(|error| store_error(format!("enumerating pending transactions: {error:#}")))?
        {
            let state = project_home
                .inspect_child_state(&name)
                .map_err(|error| {
                    store_error(format!("inspecting pending entry `{name}`: {error:#}"))
                })?
                .ok_or_else(|| store_error(format!("pending entry `{name}` vanished")))?;
            match state.kind {
                EntryStateKind::Directory if valid_transaction_id_text(&name) => {
                    transaction_ids.push(TransactionId(name));
                }
                EntryStateKind::File => {
                    if let Some(id) = transaction_id_from_retirement_name(&name) {
                        retirement_ids.push(id);
                    } else {
                        return Err(store_error(format!(
                            "unexpected file `{name}` in project transaction home"
                        )));
                    }
                }
                _ => {
                    return Err(store_error(format!(
                        "unexpected entry `{name}` in project transaction home"
                    )));
                }
            }
        }

        // A crash can leave the external retirement checkpoint after the
        // owned root has already disappeared. Complete that exact checkpoint
        // before deciding whether a journal remains pending.
        for id in retirement_ids {
            self.resume_retirement(project, &id)?;
        }
        let mut still_present = Vec::new();
        for id in transaction_ids {
            if project_home
                .open_child(&id.0)
                .map_err(|error| {
                    store_error(format!(
                        "rechecking retired transaction `{}`: {error:#}",
                        id.0
                    ))
                })?
                .is_some()
            {
                still_present.push(id);
            }
        }
        let transaction_ids = still_present;
        if transaction_ids.len() > 1 {
            return Err(store_error(format!(
                "project has {} pending scrape transactions; exactly one is recoverable",
                transaction_ids.len()
            )));
        }

        transaction_ids
            .first()
            .map(|id| self.load_journal(project, id))
            .transpose()
    }

    fn verify_snapshot_progress(
        &mut self,
        journal: &Journal,
    ) -> Result<SnapshotActiveObservation, TransactionError> {
        self.require_locked(&journal.project_key)?;
        validate::journal(journal, &journal.project_key, self.proven_display_root()?)?;
        validate_snapshot_bounds(&journal.snapshots)?;

        let expected_files = journal
            .snapshots
            .iter()
            .take(journal.snapshots_persisted)
            .map(|record| record.name.clone())
            .chain(
                journal
                    .snapshot_active
                    .map(|index| journal.snapshots[index].name.clone()),
            )
            .collect::<BTreeSet<_>>();
        let expected_directories = snapshot_directories(&expected_files);
        let actual = self.snapshot_entries(journal)?;
        for (path, kind) in &actual {
            let expected = match kind {
                EntryStateKind::File => expected_files.contains(path),
                EntryStateKind::Directory => expected_directories.contains(path),
            };
            if !expected {
                return Err(store_error(format!(
                    "unjournaled snapshot entry `{path}` is present"
                )));
            }
        }

        for record in journal.snapshots.iter().take(journal.snapshots_persisted) {
            if self.read_snapshot_record(journal, record)?.is_none() {
                return Err(store_error(format!(
                    "durable-prefix snapshot `{}` is absent",
                    record.name
                )));
            }
        }

        let Some(active) = journal.snapshot_active else {
            return Ok(SnapshotActiveObservation::None);
        };
        match self.read_snapshot_record(journal, &journal.snapshots[active])? {
            Some(_) => Ok(SnapshotActiveObservation::ExactPresent),
            None => Ok(SnapshotActiveObservation::Absent),
        }
    }

    fn read_snapshot(
        &mut self,
        journal: &Journal,
        name: &str,
    ) -> Result<Vec<u8>, TransactionError> {
        SystemTransactionStore::read_snapshot(self, journal, name)
    }

    fn mint_transaction_id(
        &mut self,
        project: &ProjectKey,
    ) -> Result<TransactionId, TransactionError> {
        self.require_locked(project)?;
        let counter = TRANSACTION_NONCE.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| store_error(format!("system clock precedes Unix epoch: {error}")))?;
        let mut hash = Sha256::new();
        hash.update(b"vibe-scrape-transaction-id-e1\0");
        hash.update(project.0.as_bytes());
        hash.update(std::process::id().to_be_bytes());
        hash.update(now.as_nanos().to_be_bytes());
        hash.update(counter.to_be_bytes());
        let full = format!("{:x}", hash.finalize());
        Ok(TransactionId(full[..32].to_owned()))
    }

    fn verification_workspace_intent(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspaceIntent, TransactionError> {
        self.require_locked(&journal.project_key)?;
        let display_root = self
            .external()?
            .path()
            .join(TRANSACTIONS_DIRECTORY)
            .join(project_component(&journal.project_key)?)
            .join(transaction_component(&journal.transaction_id)?)
            .join(VERIFICATION_DIRECTORY)
            .display()
            .to_string();
        Ok(VerificationWorkspaceIntent {
            name: VERIFICATION_DIRECTORY.to_owned(),
            display_root,
            ownership_token: verification_workspace_token(
                &journal.project_key,
                &journal.transaction_id,
            ),
        })
    }

    fn create_transaction(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspace, TransactionError> {
        self.require_locked(&journal.project_key)?;
        validate::journal(journal, &journal.project_key, self.proven_display_root()?)?;
        if journal.revision != 0
            || journal.state != TransactionState::Preparing
            || journal.snapshots_persisted != 0
            || journal.snapshot_active.is_some()
        {
            return Err(store_error(
                "new transaction journal is not at the initial preparation boundary",
            ));
        }
        validate_snapshot_bounds(&journal.snapshots)?;
        let expected_workspace = self.verification_workspace_intent(journal)?;
        if journal.verification_workspace.as_ref() != Some(&expected_workspace) {
            return Err(store_error(
                "revision-zero journal has a foreign verification-workspace intent",
            ));
        }
        // This exact byte vector is the revision-zero journal publication.
        // Its strict serialization and 64 MiB cap are proven before the first
        // project-home/transaction/workspace namespace creation.
        let initial_journal_bytes = journal_wire::encode(journal).map_err(store_error)?;
        if initial_journal_bytes.len() > MAX_TRANSACTION_JOURNAL_BYTES {
            return Err(store_error(format!(
                "encoded initial transaction journal exceeds {MAX_TRANSACTION_JOURNAL_BYTES} byte bound"
            )));
        }
        if self.pending(&journal.project_key)?.is_some() {
            return Err(store_error(
                "a pending scrape transaction already occupies the project home",
            ));
        }

        let project_home = self.ensure_project_home(&journal.project_key)?;
        let transaction_name = transaction_component(&journal.transaction_id)?;
        let ownership_token =
            transaction_ownership_token(&journal.project_key, &journal.transaction_id);
        let owned = project_home
            .create_owned_child_exclusive(transaction_name, &ownership_token)
            .map_err(map_owned_create_error)?;
        require_namespace_checkpoint(
            owned.parent_durability(),
            "transaction-directory publication",
        )?;
        let root_state = project_home
            .inspect_child_state(transaction_name)
            .map_err(|error| store_error(format!("reobserving transaction root: {error:#}")))?
            .ok_or_else(|| store_error("new transaction root vanished after creation"))?;
        if root_state.kind != EntryStateKind::Directory {
            return Err(store_error("new transaction root is not a directory"));
        }
        let transaction_root = owned
            .directory()
            .map_err(|error| store_error(format!("retaining transaction root: {error:#}")))?;
        let workspace_token = expected_workspace.ownership_token.clone();
        let workspace = transaction_root
            .create_owned_child_exclusive(VERIFICATION_DIRECTORY, &workspace_token)
            .map_err(map_owned_create_error)?;
        require_namespace_checkpoint(
            workspace.parent_durability(),
            "verification-workspace publication",
        )?;
        let workspace_state = transaction_root
            .inspect_child_state(VERIFICATION_DIRECTORY)
            .map_err(|error| store_error(format!("reobserving verification workspace: {error:#}")))?
            .ok_or_else(|| store_error("verification workspace vanished after creation"))?;
        let workspace_project = Project::open(workspace.path()).map_err(|error| {
            store_error(format!(
                "opening verification workspace as a project: {error:#}"
            ))
        })?;
        if workspace.path().display().to_string() != expected_workspace.display_root {
            return Err(store_error(
                "created verification workspace differs from its journaled display root",
            ));
        }
        let workspace_project_identity = workspace_project
            .identity_token()
            .map_err(|error| store_error(format!("sealing verification workspace: {error:#}")))?;
        let owner = OwnerWire {
            schema: 3,
            project_key: journal.project_key.0.clone(),
            transaction_id: journal.transaction_id.0.clone(),
            journal_intent_sha256: journal_intent_sha256(journal)?,
            ownership_token,
            directory_identity: owned.identity().as_str().to_owned(),
            entry_identity: root_state.identity.as_str().to_owned(),
            workspace_directory_identity: workspace.identity().as_str().to_owned(),
            workspace_entry_identity: workspace_state.identity.as_str().to_owned(),
            workspace_project_identity: workspace_project_identity.clone(),
        };
        let owner_bytes = strict_json_bytes(&owner, MAX_OWNER_BYTES, "owner seal")?;
        self.write_durable(
            &format!(
                "{}/{}/{}/{}",
                TRANSACTIONS_DIRECTORY,
                project_component(&journal.project_key)?,
                transaction_name,
                OWNER_FILE
            ),
            &owner_bytes,
            "owner seal",
        )?;
        self.write_journal_bytes(journal, true, &initial_journal_bytes)?;
        self.live_verification_workspace = Some(workspace);
        Ok(VerificationWorkspace {
            intent: expected_workspace,
            directory_identity: owner.workspace_directory_identity,
            entry_identity: owner.workspace_entry_identity,
            project_identity_token: workspace_project_identity,
        })
    }

    fn persist_snapshot(
        &mut self,
        transaction: &TransactionId,
        record: &SnapshotRecord,
        bytes: &[u8],
    ) -> Result<(), TransactionError> {
        let project = self.locked_project()?.clone();
        let current = self.load_journal(&project, transaction)?;
        let expected = current
            .snapshots
            .get(current.snapshots_persisted)
            .filter(|_| current.snapshot_active == Some(current.snapshots_persisted))
            .ok_or_else(|| store_error("journal has no active snapshot write intent"))?;
        if expected != record {
            return Err(store_error(
                "snapshot write differs from active journal intent",
            ));
        }
        verify_snapshot_bytes(record, bytes)?;
        let relative = snapshot_relative(&project, transaction, &record.name)?;
        if let Some(existing) = self.read_snapshot_record(&current, record)? {
            if existing == bytes {
                return Ok(());
            }
            return Err(store_error(format!(
                "snapshot `{}` already exists with different bytes",
                record.name
            )));
        }
        self.write_durable(&relative, bytes, "snapshot")?;
        let reread = self
            .read_snapshot_record(&current, record)?
            .ok_or_else(|| store_error(format!("snapshot `{}` vanished", record.name)))?;
        if reread != bytes {
            return Err(store_error(format!(
                "snapshot `{}` changed after publication",
                record.name
            )));
        }
        Ok(())
    }

    fn persist_journal(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.require_locked(&journal.project_key)?;
        validate::journal(journal, &journal.project_key, self.proven_display_root()?)?;
        let existing = self.load_journal(&journal.project_key, &journal.transaction_id)?;
        if existing == *journal {
            self.verify_snapshot_progress(&existing)?;
            return Ok(());
        }
        require_same_transaction(&existing, journal)?;
        let expected_revision = existing
            .revision
            .checked_add(1)
            .ok_or_else(|| store_error("transaction journal revision overflow"))?;
        if journal.revision != expected_revision {
            return Err(store_error(format!(
                "journal revision must advance exactly once from {} to {expected_revision}",
                existing.revision
            )));
        }
        let snapshot_observation = self.verify_snapshot_progress(&existing)?;
        validate_journal_update(&existing, journal, snapshot_observation)?;
        self.write_journal(journal, false)
    }

    fn persist_report(
        &mut self,
        report: &TransactionReport,
        canonical_wire: &[u8],
    ) -> Result<(), TransactionError> {
        self.require_locked(&report.project_key)?;
        if canonical_wire.len() > MAX_CANONICAL_REPORT_BYTES {
            return Err(store_error(format!(
                "canonical transaction report exceeds {MAX_CANONICAL_REPORT_BYTES} byte bound"
            )));
        }
        let durable = self.load_journal(&report.project_key, &report.transaction_id)?;
        if durable.report.as_ref() != Some(report) {
            return Err(store_error(
                "stable report request differs from the authoritative durable journal report",
            ));
        }
        let bytes = self.canonical_report_bytes(&durable, report)?;
        if canonical_wire != bytes {
            return Err(store_error(
                "provided stable report bytes are not the canonical durable report projection",
            ));
        }
        let relative = report_relative(&report.transaction_id)?;
        if let Some(existing) = self
            .external()?
            .read_stable_bounded(&relative, MAX_CANONICAL_REPORT_BYTES)
            .map_err(|error| store_error(format!("reading stable report: {error:#}")))?
        {
            if existing.bytes == bytes {
                return Ok(());
            }
            let old: report_wire::Report =
                strict_json_parse(&existing.bytes, "existing stable transaction report")?;
            let new: report_wire::Report =
                strict_json_parse(&bytes, "replacement stable transaction report")?;
            if !canonical_report_update_is_legal(&old, &new) {
                return Err(store_error(format!(
                    "stable report `{}` replacement is not an append-only pending cleanup transition",
                    report.transaction_id.0
                )));
            }
        }
        self.write_durable(&relative, &bytes, "stable report")?;
        let observed = self
            .external()?
            .read_stable_bounded(&relative, MAX_CANONICAL_REPORT_BYTES)
            .map_err(|error| store_error(format!("re-reading stable report: {error:#}")))?
            .ok_or_else(|| store_error("stable report vanished after publication"))?;
        if observed.bytes != bytes {
            return Err(store_error("stable report changed after publication"));
        }
        Ok(())
    }

    fn retire_transaction(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.require_locked(&journal.project_key)?;
        let durable = self.load_journal(&journal.project_key, &journal.transaction_id)?;
        if durable != *journal {
            return Err(store_error(
                "retirement request differs from the authoritative durable journal",
            ));
        }
        if durable.state != TransactionState::Complete {
            return Err(store_error(
                "only a complete transaction journal may be retired; rollback-failed remains pending",
            ));
        }
        self.require_stable_complete_report(&durable)?;
        // The retirement primitive needs DELETE access to the workspace and
        // transaction root; release only our own live namespace seal after
        // the authoritative complete journal and stable report were proved.
        self.live_verification_workspace = None;
        self.resume_retirement(&durable.project_key, &durable.transaction_id)
    }
    };
}
