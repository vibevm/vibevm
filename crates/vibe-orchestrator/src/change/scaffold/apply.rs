//! Create-new publication and read-only cold reconciliation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD-RECOVERY");
use super::*;
use journal::{Journal, State};
use vibe_safefs::{DirectoryDurability, Project, PublishStage};
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScaffoldState {
    Completed,
    Partial,
}
#[derive(Debug, serde::Serialize)]
pub struct ScaffoldOutcome {
    pub state: ScaffoldState,
    pub id: String,
    pub journal: String,
    pub created_directories: Vec<String>,
    pub directory_uncertain: bool,
    pub uncertain_directories: Vec<String>,
    pub verified: Vec<String>,
    pub uncertain: Vec<String>,
    pub pending: Vec<String>,
    pub reason: Option<String>,
    pub durability_reduced: bool,
    pub infrastructure_may_exist: bool,
}
fn outcome(j: &Journal, reason: Option<String>, reduced: bool) -> ScaffoldOutcome {
    let mut dirs = j.created_parents.clone();
    if j.directory_identity.is_some() {
        dirs.push(j.directory.clone());
    }
    ScaffoldOutcome {
        state: if j.completed {
            ScaffoldState::Completed
        } else {
            ScaffoldState::Partial
        },
        id: j.id.clone(),
        journal: journal::path(&j.id).unwrap(),
        created_directories: dirs,
        directory_uncertain: j.directory_uncertain,
        uncertain_directories: j.uncertain_directories.clone(),
        verified: j
            .files
            .iter()
            .filter(|f| f.state == State::Verified)
            .map(|f| format!("{}/{}", j.directory, f.name))
            .collect(),
        uncertain: j
            .files
            .iter()
            .filter(|f| f.state == State::Uncertain)
            .map(|f| format!("{}/{}", j.directory, f.name))
            .collect(),
        pending: j
            .files
            .iter()
            .filter(|f| f.state == State::Pending)
            .map(|f| format!("{}/{}", j.directory, f.name))
            .collect(),
        reason,
        durability_reduced: reduced,
        infrastructure_may_exist: true,
    }
}
impl PreparedScaffold {
    /// Consumes one invocation; the barrier stays held through durable outcome.
    pub fn apply(self) -> Result<ScaffoldOutcome, ScaffoldRefusal> {
        self.barrier.ready()?;
        let preflight = Project::open(self.authority.host.root()).map_err(infra)?;
        self.current(&preflight)?;
        absence(&preflight, &self.authority.directory)?;
        let lease =
            vibe_lifecycle::LifecycleLease::acquire(self.authority.host.root()).map_err(infra)?;
        let project = lease.document_edit_project();
        self.current(project)?;
        absence(project, &self.authority.directory)?;
        let mut j = journal::from_plan(&self);
        // An existing journal is never overwritten by a new invocation.
        if project
            .read_file_snapshot_bounded(&journal::path(&j.id)?, 32 * 1024 * 1024)
            .map_err(infra)?
            .is_some()
        {
            return Err(ScaffoldRefusal::Journal);
        }
        journal::checkpoint(project, &j)?;
        self.barrier.consume()?;
        let mut reduced = false;
        let mut changed = false;
        let effect = (|| -> Result<(), ScaffoldRefusal> {
            let (parents, name) = vibe_safefs::split_relative(&j.directory).map_err(infra)?;
            let mut parent = project.root_dir().map_err(infra)?;
            let mut relative = PathBuf::new();
            for part in parents {
                self.current(project)?;
                relative.push(&part);
                parent = if let Some(existing) = parent.open_child_checked(&part).map_err(infra)? {
                    existing
                } else {
                    // Record attempted absent mkdir before the call. A failed
                    // reopen remains explicit, never treated as no effect.
                    j.uncertain_directories
                        .push(relative.to_str().unwrap().into());
                    journal::checkpoint(project, &j)?;
                    match parent.create_scaffold_directory(&part) {
                        Ok(created) => {
                            changed = true;
                            j.uncertain_directories.pop();
                            j.created_parents.push(relative.to_str().unwrap().into());
                            j.created_parent_identities.insert(
                                relative.to_str().unwrap().into(),
                                created.identity.clone(),
                            );
                            reduced |= matches!(
                                created.durability,
                                DirectoryDurability::Failed(_)
                                    | DirectoryDurability::Unsupported(_)
                            );
                            journal::checkpoint(project, &j)?;
                            created.directory
                        }
                        Err(error) => {
                            if !error.created() {
                                j.uncertain_directories.pop();
                            } else {
                                changed = true;
                            }
                            return Err(infra(format!("{error:?}")));
                        }
                    }
                };
            }
            self.current(project)?;
            if parent.inspect_child_state(&name).map_err(infra)?.is_some() {
                return Err(ScaffoldRefusal::Occupied);
            }
            j.directory_uncertain = true;
            journal::checkpoint(project, &j)?;
            let created = match parent.create_scaffold_directory(&name) {
                Ok(created) => created,
                Err(error) => {
                    if !error.created() {
                        j.directory_uncertain = false;
                    } else {
                        changed = true;
                    }
                    return Err(infra(format!("{error:?}")));
                }
            };
            changed = true;
            j.directory_uncertain = false;
            j.directory_identity = Some(created.identity);
            j.directory_mode = created.directory.unix_mode().map_err(infra)?;
            reduced |= matches!(
                created.durability,
                DirectoryDurability::Failed(_) | DirectoryDurability::Unsupported(_)
            );
            journal::checkpoint(project, &j)?;
            for index in 0..j.files.len() {
                self.current(project)?;
                if parent
                    .open_child(&name)
                    .map_err(infra)?
                    .identity()
                    .map_err(infra)?
                    .observation_correlation_token()
                    != j.directory_identity.as_ref().unwrap().as_str()
                {
                    return Err(ScaffoldRefusal::ThirdState);
                }
                let file = &j.files[index];
                if created
                    .directory
                    .inspect_child_state(&file.name)
                    .map_err(infra)?
                    .is_some()
                {
                    return Err(ScaffoldRefusal::ThirdState);
                }
                j.files[index].state = State::Uncertain;
                journal::checkpoint(project, &j)?;
                let file = &j.files[index];
                if let Err(error) =
                    project.publish_new_proved_in(&created.directory, &file.name, &file.desired)
                {
                    if error.stage == PublishStage::BeforePublication {
                        j.files[index].state = State::Pending;
                    }
                    return Err(infra(error));
                }
                let snapshot = project
                    .read_file_snapshot_bounded_in(
                        &created.directory,
                        &file.name,
                        file.desired.len(),
                    )
                    .map_err(infra)?
                    .ok_or(ScaffoldRefusal::ThirdState)?;
                if snapshot.bytes != file.desired {
                    return Err(ScaffoldRefusal::ThirdState);
                }
                j.files[index].identity = Some(snapshot.identity.observation_correlation_token());
                j.files[index].unix_mode = snapshot.unix_mode;
                j.files[index].state = State::Verified;
                let durability = created.directory.sync_directory();
                reduced |= !matches!(durability, DirectoryDurability::Synced);
                if matches!(durability, DirectoryDurability::Failed(_)) {
                    return Err(ScaffoldRefusal::Journal);
                }
                journal::checkpoint(project, &j)?;
            }
            self.current(project)?;
            j.completed = true;
            journal::checkpoint(project, &j)?;
            Ok(())
        })();
        let mut reason = effect.err().map(|e| e.to_string());
        if reason.is_some() {
            j.completed = false;
            if let Err(error) = journal::checkpoint(project, &j) {
                reason = Some(format!(
                    "{}; journal checkpoint failed: {error}",
                    reason.unwrap()
                ));
            }
        }
        self.barrier.release_with_outcome(changed);
        Ok(outcome(&j, reason, reduced))
    }
}
impl ScaffoldAuthority {
    /// Cold reconciliation reads retained state and current exact identities.
    /// It does not retry publication or adopt a same-byte uncertain object.
    /// Pending artifacts remain recovery-owned; a fresh invocation cannot
    /// overwrite this occupied immutable directory.
    pub fn inspect_recovery(&self, id: &str) -> Result<ScaffoldOutcome, ScaffoldRefusal> {
        let project = Project::open(self.host.root()).map_err(infra)?;
        if project.root_correlation_token().map_err(infra)? != self.root_binding {
            return Err(ScaffoldRefusal::Invalid);
        }
        let j = journal::load(&project, self, id)?;
        for (path, identity) in &j.created_parent_identities {
            let parts: Vec<_> = path.split('/').collect();
            let current = project
                .dir_if_present(&parts)
                .map_err(|_| ScaffoldRefusal::ThirdState)?
                .ok_or(ScaffoldRefusal::ThirdState)?;
            if current
                .identity()
                .map_err(infra)?
                .observation_correlation_token()
                != *identity
            {
                return Err(ScaffoldRefusal::ThirdState);
            }
        }
        let (parents, name) = vibe_safefs::split_relative(&j.directory).map_err(infra)?;
        let parents: Vec<_> = parents.iter().map(String::as_str).collect();
        let parent = project.dir_if_present(&parents).map_err(infra)?;
        if let Some(identity) = &j.directory_identity {
            let directory = parent
                .ok_or(ScaffoldRefusal::ThirdState)?
                .open_child(&name)
                .map_err(|_| ScaffoldRefusal::ThirdState)?;
            if directory
                .identity()
                .map_err(infra)?
                .observation_correlation_token()
                != *identity
            {
                return Err(ScaffoldRefusal::ThirdState);
            }
            if directory.unix_mode().map_err(infra)? != j.directory_mode {
                return Err(ScaffoldRefusal::ThirdState);
            }
            for file in &j.files {
                let snapshot = project
                    .read_file_snapshot_bounded_in(&directory, &file.name, file.desired.len())
                    .map_err(|_| ScaffoldRefusal::ThirdState)?;
                match (&file.state, snapshot) {
                    (State::Verified, Some(s))
                        if s.bytes == file.desired
                            && s.unix_mode == file.unix_mode
                            && Some(s.identity.observation_correlation_token())
                                == file.identity => {}
                    (State::Pending, None) => {}
                    (State::Uncertain, None) => {}
                    (State::Uncertain, Some(s)) if s.bytes == file.desired => {} // remains uncertain, no ownership promotion
                    _ => return Err(ScaffoldRefusal::ThirdState),
                }
            }
        } else if !j.directory_uncertain {
            if let Some(parent) = parent {
                if parent.inspect_child_state(&name).map_err(infra)?.is_some() {
                    return Err(ScaffoldRefusal::ThirdState);
                }
            }
        }
        Ok(outcome(
            &j,
            if j.completed {
                None
            } else {
                Some("recovery-owned partial; no retry or ownership promotion performed".into())
            },
            false,
        ))
    }
}
