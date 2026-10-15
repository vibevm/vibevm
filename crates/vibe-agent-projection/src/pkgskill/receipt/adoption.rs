//! Explicit host arena composition. The arena grants exclusion only; donor
//! ownership remains with the existing projection receipt reader and writer.
use crate::agents::{Agent, Scope};
use crate::pkgskill::{PackageSkillReport, ProjectSkillBinding};
use anyhow::Result;
use std::path::Path;
use vibe_safefs::Project;

pub struct CoordinatedSkills {
    arena: Project,
    identity: String,
    standalone_root: Option<(Project, String)>,
}
impl CoordinatedSkills {
    pub fn new(arena: Project) -> Result<Self> {
        let identity = arena.root_correlation_token()?;
        Ok(Self {
            arena,
            identity,
            standalone_root: None,
        })
    }
    /// Explicit user-root capability for the participating standalone path.
    /// Legacy ambient root selection cannot silently select the hold store.
    pub fn with_standalone_root(mut self, root: Project) -> Result<Self> {
        let identity = root.root_correlation_token()?;
        self.standalone_root = Some((root, identity));
        Ok(self)
    }
    fn standalone_lease(
        &self,
        agent: Agent,
        scope: Scope,
        project_root: Option<&Path>,
        dry_run: bool,
    ) -> Result<Option<vibe_safefs::LockGuard>> {
        if scope == Scope::Project {
            return Ok(None);
        }
        anyhow::ensure!(scope == Scope::User, "one exact standalone scope required");
        let (root, identity) = self.standalone_root.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "participating user writer requires explicit standalone root capability"
            )
        })?;
        anyhow::ensure!(
            root.root_correlation_token()? == *identity,
            "standalone root changed"
        );
        let target = agent
            .skills_root(scope, project_root)?
            .ok_or_else(|| anyhow::anyhow!("missing standalone target"))?;
        let relative = target
            .strip_prefix(root.root_path())?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("standalone path encoding"))?;
        root.preflight(relative)?;
        let guard = if dry_run {
            None
        } else {
            Some(root.lock(super::nofollow::LOCK_FILE)?)
        };
        refuse_hold(root)?;
        Ok(guard)
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    fn current_arena(&self) -> Result<()> {
        anyhow::ensure!(
            Project::open(self.arena.root_path())?.root_correlation_token()? == self.identity,
            "coordination arena changed"
        );
        Ok(())
    }
    fn lock(&self) -> Result<vibe_safefs::LockGuard> {
        self.current_arena()?;
        let guard = self.arena.lock("deploy-physical-coordination.lock")?;
        self.current_arena()?;
        Ok(guard)
    }
    pub fn reconcile_vanished(
        &self,
        project_root: &Path,
        desired: &std::collections::BTreeSet<String>,
    ) -> Result<Vec<PackageSkillReport>> {
        let _arena = self.lock()?;
        super::reconcile::reconcile_vanished(project_root, desired)
    }
    pub fn recover(&self, project_root: &Path) -> Result<Vec<PackageSkillReport>> {
        let _arena = self.lock()?;
        super::recover::recover_pending(project_root)
    }
    pub fn reconcile(
        &self,
        project_root: &Path,
        binding: &ProjectSkillBinding,
    ) -> Result<Vec<PackageSkillReport>> {
        let _arena = self.lock()?;
        super::reconcile::reconcile_binding(project_root, binding)
    }
    pub fn install(
        &self,
        agent: Agent,
        scope: Scope,
        project_root: Option<&Path>,
        skill: &str,
        source: &Path,
        include: &[String],
        dry_run: bool,
    ) -> Result<PackageSkillReport> {
        let _arena = if dry_run { None } else { Some(self.lock()?) };
        let _standalone = self.standalone_lease(agent, scope, project_root, dry_run)?;
        Ok(crate::pkgskill::install_package_skill_selecting(
            agent,
            scope,
            project_root,
            skill,
            source,
            include,
            dry_run,
        )?)
    }
    pub fn uninstall(
        &self,
        agent: Agent,
        scope: Scope,
        project_root: Option<&Path>,
        skill: &str,
        dry_run: bool,
    ) -> Result<PackageSkillReport> {
        let _arena = if dry_run { None } else { Some(self.lock()?) };
        let _standalone = self.standalone_lease(agent, scope, project_root, dry_run)?;
        Ok(crate::pkgskill::uninstall_package_skill(
            agent,
            scope,
            project_root,
            skill,
            dry_run,
        )?)
    }
}

/// A donor-owned snapshot. The lifecycle coordinator cannot manufacture its
/// ownership rows, receipt bytes or retirement state from a descriptor.
pub struct SkillDonor {
    project: Project,
    id: String,
    family: vibe_wire::generated::deploy_generic_state::TransferFamily,
    receipt_path: String,
    original: Vec<u8>,
    retired: Option<Vec<u8>>,
    resources: Vec<(String, String)>,
    root_identity: String,
}
const HOLD: &str = ".vibe/generic-skill-transfer.json";
impl SkillDonor {
    pub fn automatic(project: Project, key: &str) -> Result<Self> {
        let receipt = super::state::read_receipt(&project)?
            .ok_or_else(|| anyhow::anyhow!("missing donor receipt"))?;
        anyhow::ensure!(receipt.applying.is_none(), "donor has pending transaction");
        let binding = receipt
            .binding
            .iter()
            .find(|b| b.key == key)
            .ok_or_else(|| anyhow::anyhow!("missing donor binding"))?;
        let mut resources = Vec::new();
        for target in &binding.target {
            anyhow::ensure!(
                super::state::owned_target_matches(&project, project.root_path(), target)?,
                "changed donor destination"
            );
            let prefix = Path::new(&target.path)
                .strip_prefix(project.root_path())?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("non UTF8 donor"))?;
            for file in &target.file {
                resources.push((
                    format!("{prefix}/{}", file.path),
                    file.sha256.trim_start_matches("sha256:").into(),
                ));
            }
        }
        let original = project
            .read_file_bounded(".vibe/package-skills.toml", 4 * 1024 * 1024)?
            .ok_or_else(|| anyhow::anyhow!("missing receipt bytes"))?;
        let mut after = receipt.clone();
        after.binding.retain(|b| b.key != key);
        let retired = Some(toml::to_string_pretty(&after)?.into_bytes());
        let root_identity = project.root_correlation_token()?;
        Ok(Self {
            project,
            id: key.into(),
            family: vibe_wire::generated::deploy_generic_state::TransferFamily::PackageSkill,
            receipt_path: ".vibe/package-skills.toml".into(),
            original,
            retired,
            resources,
            root_identity,
        })
    }
    /// The embedding host supplies a project capability and exact standalone
    /// skill directory. No descriptor resolves a user home or grants a root.
    pub fn standalone(project: Project, directory: &str, skill: &str) -> Result<Self> {
        vibe_safefs::split_relative(directory)?;
        let receipt_path = format!("{directory}/.{skill}.vibe-skill-receipt.toml");
        let receipt = crate::pkgskill::read_standalone_receipt(
            &project.root_path().join(&receipt_path),
            skill,
        )?
        .ok_or_else(|| anyhow::anyhow!("missing standalone donor receipt"))?;
        let original = project
            .read_file_bounded(&receipt_path, 4 * 1024 * 1024)?
            .ok_or_else(|| anyhow::anyhow!("missing donor bytes"))?;
        let resources = receipt
            .file
            .iter()
            .map(|f| (format!("{directory}/{skill}/{}", f.path), f.sha256.clone()))
            .collect();
        let root_identity = project.root_correlation_token()?;
        let donor = Self {
            project,
            id: format!("{directory}/{skill}"),
            family: vibe_wire::generated::deploy_generic_state::TransferFamily::StandaloneSkill,
            receipt_path,
            original,
            retired: None,
            resources,
            root_identity,
        };
        donor.validate_destinations()?;
        Ok(donor)
    }
    pub fn resume(project: Project) -> Result<Self> {
        use sha2::{Digest, Sha256};
        use vibe_wire::generated::deploy_generic_state::*;
        let bytes = project
            .read_file_bounded(HOLD, 4 * 1024 * 1024)?
            .ok_or_else(|| anyhow::anyhow!("missing hold"))?;
        let state: DeployGenericState = serde_json::from_slice(&bytes)?;
        vibe_wire::behaviour::deploy_generic_state::validate(&state)?;
        let t = state
            .pending
            .as_ref()
            .and_then(|b| b.transfer.as_ref())
            .ok_or_else(|| anyhow::anyhow!("missing transfer"))?;
        anyhow::ensure!(
            project.root_correlation_token()? == t.donor_root_identity,
            "donor root differs from retained transfer"
        );
        let original = project
            .read_file_bounded(
                &format!(".vibe/generic-donor-blobs/{}", t.receipt_digest),
                4 * 1024 * 1024,
            )?
            .ok_or_else(|| anyhow::anyhow!("missing original donor receipt"))?;
        anyhow::ensure!(
            format!("{:x}", Sha256::digest(&original)) == t.receipt_digest,
            "corrupt original donor receipt"
        );
        let mut resources = Vec::new();
        let (receipt_path, retired) = match t.family {
            TransferFamily::PackageSkill => {
                let mut r: vibe_wire::generated::package_skill_receipt::PackageSkillReceipt =
                    toml::from_str(std::str::from_utf8(&original)?)?;
                super::state::validate_receipt(project.root_path(), &r)?;
                let b = r
                    .binding
                    .iter()
                    .find(|b| b.key == t.donor_id)
                    .ok_or_else(|| anyhow::anyhow!("missing held binding"))?;
                for target in &b.target {
                    let prefix = Path::new(&target.path)
                        .strip_prefix(project.root_path())?
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("donor path"))?;
                    for f in &target.file {
                        resources.push((
                            format!("{prefix}/{}", f.path),
                            f.sha256.trim_start_matches("sha256:").into(),
                        ));
                    }
                }
                r.binding.retain(|b| b.key != t.donor_id);
                (
                    ".vibe/package-skills.toml".into(),
                    Some(toml::to_string_pretty(&r)?.into_bytes()),
                )
            }
            TransferFamily::StandaloneSkill => {
                let (directory, skill) = t
                    .donor_id
                    .rsplit_once('/')
                    .ok_or_else(|| anyhow::anyhow!("donor coordinate"))?;
                vibe_safefs::split_relative(directory)?;
                let r: crate::pkgskill::StandaloneSkillReceipt =
                    toml::from_str(std::str::from_utf8(&original)?)?;
                anyhow::ensure!(
                    r.schema == 1 && r.skill == skill,
                    "standalone donor identity"
                );
                super::containment::judge_selection(r.file.iter().map(|f| f.path.as_str()))
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                for f in &r.file {
                    resources.push((format!("{directory}/{skill}/{}", f.path), f.sha256.clone()));
                }
                (
                    format!("{directory}/.{skill}.vibe-skill-receipt.toml"),
                    None,
                )
            }
            _ => anyhow::bail!("not a skill donor"),
        };
        let root_identity = project.root_correlation_token()?;
        Ok(Self {
            project,
            id: t.donor_id.clone(),
            family: t.family.clone(),
            receipt_path,
            original,
            retired,
            resources,
            root_identity,
        })
    }
    pub fn root(&self) -> &Path {
        self.project.root_path()
    }
    pub fn root_identity(&self) -> &str {
        &self.root_identity
    }
    pub fn matches_transfer(
        &self,
        t: &vibe_wire::generated::deploy_generic_state::Transfer,
    ) -> Result<()> {
        self.current_root()?;
        anyhow::ensure!(
            t.donor_root_identity == self.root_identity
                && t.donor_id == self.id
                && t.family == self.family,
            "wrong donor capability"
        );
        Ok(())
    }
    fn current_root(&self) -> Result<()> {
        anyhow::ensure!(
            Project::open(self.project.root_path())?.root_correlation_token()?
                == self.root_identity,
            "donor root changed"
        );
        Ok(())
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn family(&self) -> vibe_wire::generated::deploy_generic_state::TransferFamily {
        self.family.clone()
    }
    pub fn receipt_bytes(&self) -> &[u8] {
        &self.original
    }
    pub fn resources(&self) -> &[(String, String)] {
        &self.resources
    }
    pub fn lock(&self) -> Result<vibe_safefs::LockGuard> {
        self.current_root()?;
        let guard = self.project.lock(super::nofollow::LOCK_FILE)?;
        self.current_root()?;
        Ok(guard)
    }
    fn validate_destinations(&self) -> Result<()> {
        self.current_root()?;
        use sha2::{Digest, Sha256};
        anyhow::ensure!(
            self.project.root_correlation_token()? == self.root_identity,
            "donor root changed"
        );
        for (path, digest) in &self.resources {
            let bytes = self
                .project
                .read_file_bounded(path, 4 * 1024 * 1024)?
                .ok_or_else(|| anyhow::anyhow!("donor file missing"))?;
            anyhow::ensure!(
                format!("{:x}", Sha256::digest(bytes)) == *digest,
                "donor destination changed"
            );
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        self.validate_destinations()?;
        anyhow::ensure!(
            self.project
                .read_file_bounded(&self.receipt_path, 4 * 1024 * 1024)?
                .as_ref()
                == Some(&self.original),
            "donor receipt changed"
        );
        anyhow::ensure!(
            self.project
                .read_file_bounded(HOLD, 4 * 1024 * 1024)?
                .is_none(),
            "donor already held"
        );
        Ok(())
    }
    pub fn hold(
        &self,
        state: &vibe_wire::generated::deploy_generic_state::DeployGenericState,
    ) -> Result<()> {
        self.validate()?;
        vibe_wire::behaviour::deploy_generic_state::validate(state)?;
        let b = state
            .pending
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing recipient pending"))?;
        let t = b
            .transfer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing transfer"))?;
        self.matches_transfer(t)?;
        anyhow::ensure!(
            t.family == self.family && t.donor_id == self.id,
            "wrong donor join"
        );
        use sha2::{Digest, Sha256};
        anyhow::ensure!(
            t.receipt_digest == format!("{:x}", Sha256::digest(&self.original))
                && t.receipt_blob.bytes as usize == self.original.len(),
            "donor receipt reference mismatch"
        );
        anyhow::ensure!(t.resources.len()==self.resources.len() && t.resources.iter().all(|r| r.key.is_empty() && self.resources.iter().any(|(path,digest)| *path==r.destination && matches!(&r.desired,vibe_wire::generated::deploy_generic_state::Value::Present(v) if v.blob.digest==*digest))),"donor resource references mismatch");
        self.project
            .write_atomic_with_mode(
                &format!(".vibe/generic-donor-blobs/{}", t.receipt_digest),
                &self.original,
                Some(0o600),
            )
            .map_err(vibe_safefs::PublishError::into_report)?;
        self.project
            .write_atomic_with_mode(HOLD, &serde_json::to_vec(state)?, Some(0o600))
            .map_err(vibe_safefs::PublishError::into_report)?;
        Ok(())
    }
    fn check_hold(
        &self,
        hold_id: &str,
    ) -> Result<vibe_wire::generated::deploy_generic_state::DeployGenericState> {
        let bytes = self
            .project
            .read_file_bounded(HOLD, 4 * 1024 * 1024)?
            .ok_or_else(|| anyhow::anyhow!("missing donor hold"))?;
        let state: vibe_wire::generated::deploy_generic_state::DeployGenericState =
            serde_json::from_slice(&bytes)?;
        vibe_wire::behaviour::deploy_generic_state::validate(&state)?;
        let t = state
            .pending
            .as_ref()
            .and_then(|b| b.transfer.as_ref())
            .ok_or_else(|| anyhow::anyhow!("missing hold transfer"))?;
        self.matches_transfer(t)?;
        anyhow::ensure!(
            t.hold_id == hold_id && t.donor_id == self.id && t.family == self.family,
            "donor hold changed"
        );
        Ok(state)
    }
    pub fn ensure_hold(
        &self,
        record: &vibe_wire::generated::deploy_generic_state::DeployGenericState,
    ) -> Result<()> {
        let b = record
            .pending
            .as_ref()
            .or(record.committed.as_ref())
            .ok_or_else(|| anyhow::anyhow!("missing recipient binding"))?;
        let t = b
            .transfer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing transfer"))?;
        self.matches_transfer(t)?;
        if self
            .project
            .read_file_bounded(HOLD, 4 * 1024 * 1024)?
            .is_none()
        {
            anyhow::ensure!(
                b.operation
                    == vibe_wire::generated::deploy_generic_state::BindingOperation::Adoption,
                "missing retained donor hold"
            );
            return self.hold(record);
        }
        let current = self
            .project
            .read_file_bounded(&self.receipt_path, 4 * 1024 * 1024)?;
        anyhow::ensure!(
            current.as_ref() == Some(&self.original) || current == self.retired,
            "changed donor receipt under hold"
        );
        let old = self.check_hold(&t.hold_id)?;
        anyhow::ensure!(
            old.pending.as_ref().unwrap().coordination_identity == b.coordination_identity,
            "donor arena mismatch"
        );
        Ok(())
    }
    pub fn retire(&self, hold_id: &str) -> Result<()> {
        self.validate_destinations()?;
        let mut hold = self.check_hold(hold_id)?;
        let current = self
            .project
            .read_file_bounded(&self.receipt_path, 4 * 1024 * 1024)?;
        anyhow::ensure!(
            current.as_ref() == Some(&self.original) || current == self.retired,
            "donor receipt changed during retirement"
        );
        match &self.retired {
            Some(bytes) => {
                self.project
                    .write_atomic_with_mode(&self.receipt_path, bytes, Some(0o600))
                    .map_err(vibe_safefs::PublishError::into_report)?;
            }
            None if current.is_some() => {
                self.project.remove_file(&self.receipt_path)?;
            }
            None => {}
        }
        hold.pending
            .as_mut()
            .unwrap()
            .transfer
            .as_mut()
            .unwrap()
            .phase = vibe_wire::generated::deploy_generic_state::TransferPhase::Retired;
        self.project
            .write_atomic_with_mode(HOLD, &serde_json::to_vec(&hold)?, Some(0o600))
            .map_err(vibe_safefs::PublishError::into_report)?;
        Ok(())
    }
    pub fn restore(&self, hold_id: &str) -> Result<()> {
        self.validate_destinations()?;
        self.check_hold(hold_id)?;
        let current = self
            .project
            .read_file_bounded(&self.receipt_path, 4 * 1024 * 1024)?;
        anyhow::ensure!(
            current == self.retired || current.as_ref() == Some(&self.original),
            "donor receipt changed before restoration"
        );
        self.project
            .write_atomic_with_mode(&self.receipt_path, &self.original, Some(0o600))
            .map_err(vibe_safefs::PublishError::into_report)?;
        // Caller first relinquishes recipient ownership durably. Only then
        // does this last publication make the original donor writable again.
        let mut hold = self.check_hold(hold_id)?;
        hold.pending
            .as_mut()
            .unwrap()
            .transfer
            .as_mut()
            .unwrap()
            .phase = vibe_wire::generated::deploy_generic_state::TransferPhase::Restored;
        self.project
            .write_atomic_with_mode(HOLD, &serde_json::to_vec(&hold)?, Some(0o600))
            .map_err(vibe_safefs::PublishError::into_report)?;
        Ok(())
    }
}

pub(crate) fn refuse_hold(project: &Project) -> Result<()> {
    if let Some(bytes) = project.read_file_bounded(HOLD, 4 * 1024 * 1024)? {
        let state: vibe_wire::generated::deploy_generic_state::DeployGenericState =
            serde_json::from_slice(&bytes)?;
        vibe_wire::behaviour::deploy_generic_state::validate(&state)?;
        anyhow::ensure!(
            state
                .pending
                .as_ref()
                .and_then(|b| b.transfer.as_ref())
                .is_some_and(|t| t.phase
                    == vibe_wire::generated::deploy_generic_state::TransferPhase::Restored),
            "generic transfer holds skill donor; use donor-aware recovery"
        );
    }
    Ok(())
}
