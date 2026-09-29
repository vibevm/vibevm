//! Explicit composition-root authority for a finite authored-file set.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL");
use super::ServerContext;
use std::{collections::BTreeSet, path::PathBuf};
use vibe_wire::generated::migration::e1::{
    operation_report::ApplicationErrorCode, operation_request::Authority,
};
use vibe_workspace::migration_application::{
    ApplicationValidationError, OwnedManifestMigrationFile,
};

pub(super) struct ManifestMigrationAuthority {
    project_root: PathBuf,
    cache_root: PathBuf,
    recovery_root: PathBuf,
    files: BTreeSet<String>,
}
fn refused(field: &'static str, reason: &'static str) -> ApplicationValidationError {
    ApplicationValidationError {
        code: ApplicationErrorCode::AuthorityRefused,
        field,
        reason,
        operation: None,
        operation_id: None,
    }
}
impl ServerContext {
    /// Grant only the caller-selected owned authored files. An empty list grants
    /// ONLY root vibe.toml. This builder does not initialize any filesystem state.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
        r = 1
    )]
    pub fn with_manifest_migration_authority<I, S>(
        mut self,
        recovery_root: impl Into<PathBuf>,
        files: I,
    ) -> Result<Self, ApplicationValidationError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let recovery_root = recovery_root.into();
        let mut selected = BTreeSet::new();
        for file in files {
            let file = file.into();
            vibe_safefs::split_relative(&file).map_err(|_| {
                refused(
                    "authority.target_relative",
                    "configured file is not portable",
                )
            })?;
            selected.insert(file);
        }
        if selected.is_empty() {
            selected.insert("vibe.toml".to_owned());
        }
        let representative = selected.first().expect("default supplies a file");
        OwnedManifestMigrationFile::new(&self.project_root, representative, &recovery_root)?
            .with_protected_machine_cache_root(&self.store_root)?;
        self.manifest_migration_authority = Some(ManifestMigrationAuthority {
            project_root: self.project_root.clone(),
            cache_root: self.store_root.clone(),
            recovery_root,
            files: selected,
        });
        Ok(self)
    }

    /// Bind the payload to private composed authority BEFORE opening the trusted
    /// project capability. Payload roots never become arbitrary open operands.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
        r = 1
    )]
    pub fn grant_for_manifest_migration(
        &self,
        requested: &Authority,
    ) -> Result<OwnedManifestMigrationFile, ApplicationValidationError> {
        let configured = self.manifest_migration_authority.as_ref().ok_or_else(|| {
            refused(
                "authority",
                "server has no explicit manifest migration authority",
            )
        })?;
        if self.project_root != configured.project_root || self.store_root != configured.cache_root
        {
            return Err(refused(
                "authority",
                "server roots changed after migration authority was composed",
            ));
        }
        if self.project_root.to_str() != Some(requested.project_root.as_str())
            || configured.recovery_root.to_str() != Some(requested.recovery_store_root.as_str())
        {
            return Err(refused(
                "authority",
                "request roots differ from composed migration authority",
            ));
        }
        if !configured.files.contains(&requested.target_relative) {
            return Err(refused(
                "authority.target_relative",
                "requested file is outside configured allowlist",
            ));
        }
        OwnedManifestMigrationFile::new(
            &self.project_root,
            &requested.target_relative,
            &configured.recovery_root,
        )?
        .with_protected_machine_cache_root(&self.store_root)
    }
}
