//! Strict lock admission and lossless ordinary writer dispatch.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
);
use super::*;

impl Lockfile {
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        Self::admit(read_toml(path)?)
    }

    /// Parse captured lock bytes with the same admission as a path read.
    /// This lets an owned publication classify exactly the snapshot it may replace.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
    )]
    pub fn parse_str(text: &str) -> Result<Self> {
        let lockfile = toml::from_str(text).map_err(|source| {
            crate::Error::parse_toml(PathBuf::from(Self::FILENAME), source, text)
        })?;
        Self::admit(lockfile)
    }

    fn admit(lockfile: Self) -> Result<Self> {
        if lockfile.meta.schema_version < CONFIGURATION_SCHEMA_VERSION
            && !lockfile.configurations.is_empty()
        {
            return Err(crate::Error::InvalidLockfile {
                reason: "rich package configuration requires schema8".into(),
            });
        }
        if !matches!(
            lockfile.meta.schema_version,
            LEGACY_SCHEMA_VERSION | CONFIGURATION_SCHEMA_VERSION | CURRENT_SCHEMA_VERSION
        ) {
            return Err(crate::error::Error::UnsupportedLockfile {
                found: lockfile.meta.schema_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }
        lockfile.validate_remap_sources()?;
        // `find`/`find_mut`/`remove` below treat `(group, name)` as a
        // unique key — first match wins. A lockfile carrying duplicate
        // identities would make those lookups silently
        // position-dependent; the solver never emits duplicates, so a
        // duplicate here means a hand-edited or corrupted file.
        debug_assert!(
            lockfile
                .packages
                .iter()
                .map(|p| (&p.group, &p.name))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == lockfile.packages.len(),
            "lockfile carries duplicate (group, name) identities"
        );
        lockfile.validate_configurations()?;
        Ok(lockfile)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        write_toml(path, &self.for_write()?)
    }
}
