//! Existing-only recovery capabilities; absence never authorizes creation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-EXISTING-OBSERVATION");
use super::*;

impl ExternalStore {
    /// Open an existing disjoint store without creating any path component.
    /// Links, files, overlap and observation errors refuse; only absence is None.
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// use vibe_safefs::{ExternalStore, Project};
    /// let temp = tempfile::tempdir()?;
    /// let project_path = temp.path().join("project");
    /// std::fs::create_dir(&project_path)?;
    /// let project = Project::open(&project_path)?;
    /// let absent = temp.path().join("state/nested");
    /// assert!(ExternalStore::open_existing_disjoint(&absent, &project)?.is_none());
    /// assert!(!temp.path().join("state").exists());
    /// # Ok(()) }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-EXISTING-OBSERVATION"
    )]
    pub fn open_existing_disjoint(path: &Path, project: &Project) -> Result<Option<Self>> {
        let (anchor, components) = absolute_parts(path)?;
        if components.is_empty() {
            bail!("external store must not be a filesystem anchor");
        }
        let project_identity = project.root_identity()?;
        let mut current = open_anchor(&anchor)?;
        let mut ancestors = vec![current.identity()?];
        for component in components {
            match current.open_child_checked(&component)? {
                Some(child) => {
                    current = child;
                    ancestors.push(current.identity()?);
                }
                None => {
                    if ancestors.contains(&project_identity) {
                        bail!("missing external store is within the pinned project");
                    }
                    return Ok(None);
                }
            }
        }
        let store = Self {
            root: current,
            ancestor_identities: ancestors,
            bootstrap_syncs: Vec::new(),
        };
        store.prove_disjoint_from(project)?;
        Ok(Some(store))
    }

    /// Opaque physical store identity; not interchangeable with project identity.
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// let temp = tempfile::tempdir()?;
    /// let store = vibe_safefs::ExternalStore::open_or_create(&temp.path().join("state"))?;
    /// let token = store.identity_token()?;
    /// assert!(token.starts_with("sha256:"));
    /// assert_eq!(token.len(), 71);
    /// assert_ne!(token, vibe_safefs::Project::open(store.path())?.identity_token()?);
    /// # Ok(()) }
    /// ```
    pub fn identity_token(&self) -> Result<String> {
        Ok(identity_token(
            b"vibe-safefs-external-store-identity-e1\0",
            self.root.identity()?,
        ))
    }

    /// Acquire only an existing named barrier, using the creating path's same
    /// identity-rechecked OS lock. Creates neither locks/ nor a lock file.
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// let temp = tempfile::tempdir()?;
    /// let store = vibe_safefs::ExternalStore::open_or_create(&temp.path().join("state"))?;
    /// assert!(store.lock_existing_project("project-key")?.is_none());
    /// drop(store.open_and_lock_project("project-key")?);
    /// store.lock_existing_project("project-key")?.unwrap().require_still_named()?;
    /// # Ok(()) }
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-EXISTING-OBSERVATION"
    )]
    pub fn lock_existing_project(&self, project_key: &str) -> Result<Option<ExternalProjectLock>> {
        let name = external_lock_name(project_key)?;
        let Some(locks) = self.root.open_child_checked("locks")? else {
            return Ok(None);
        };
        acquire_external_lock(&locks, &name, false)
    }
}
