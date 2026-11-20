//! `[[package.subskills_active]]` — one subskill of a locked package, with
//! the delivery mode it resolved to and the files it contributed.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#lockfile");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One subskill entry under a package's `subskills_active` list.
///
/// ```
/// use vibe_core::manifest::LockedSubskill;
///
/// let s: LockedSubskill = toml::from_str(r#"
///     path = "stack/rust"
///     delivery = "eager"
///     files_written = ["spec/boot/15-flow-wal-rust.md"]
/// "#).unwrap();
/// assert_eq!(s.path, "stack/rust");
/// assert_eq!(s.delivery, "eager");
/// assert_eq!(s.files_written.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedSubskill {
    /// Subskill canonical path within the parent package, e.g.
    /// `stack/rust`.
    pub path: String,
    /// Resolved delivery mode — `eager` / `lazy-push` / `lazy-pull`.
    pub delivery: String,
    /// PURL inherited or declared on the subskill, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub describes: Option<String>,
    /// Project-relative files this subskill *specifically* contributed
    /// — distinct from the package-level `files_written` aggregate.
    /// Empty for `delivery=lazy-pull` since those files are never
    /// materialised on disk; the per-subskill index lives in the
    /// lockfile so `vibe-mcp::read_subskill` can resolve them by
    /// loading from the package cache rather than the project tree.
    /// Empty for legacy `subskills_active` entries written before
    /// this field landed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_written: Vec<PathBuf>,

    /// For `delivery=lazy-pull`: the files the subskill carries
    /// inside the package cache, relative to the subskill's own
    /// root (`<cache>/subskills/<path>/<...>`). Used by
    /// `vibe-mcp::read_subskill` to fetch on-demand without ever
    /// touching the project tree.
    /// Absent for `eager` / `lazy-push` deliveries (they
    /// materialise into the project, recorded under
    /// `files_written` above).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_files: Vec<PathBuf>,

    /// Independently observed local-source members; never inferred from cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_witness: Option<SubskillSourceWitness>,
}

/// Exact bounded file identity observed by the admitted source owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillFileWitness {
    pub path: String,
    pub byte_len: u64,
    pub sha256: String,
}

/// Source-admitted membership, independent of the historical tree recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubskillSourceWitness {
    pub package_content_hash: crate::ContentHash,
    pub declaration: SubskillFileWitness,
    pub member: SubskillFileWitness,
}

impl LockedSubskill {
    /// Judge only the configured witness shape; filesystem admission is separate.
    pub fn validate_source_witness(
        &self,
        package_pin: &crate::ContentHash,
    ) -> std::result::Result<(), String> {
        let witness = self
            .source_witness
            .as_ref()
            .ok_or("configured subskill source witness is missing")?;
        if self.delivery != "lazy-pull"
            || !self.files_written.is_empty()
            || self.cache_files.len() != 1
            || &witness.package_content_hash != package_pin
        {
            return Err("configured subskill delivery or package pin disagrees".into());
        }
        super::super::declarant_path(std::path::Path::new(&self.path))
            .map_err(|_| "invalid canonical subskill path")?;
        let member = self.cache_files[0].to_str().ok_or("non-UTF8 member path")?;
        super::super::declarant_path(std::path::Path::new(member))
            .map_err(|_| "invalid subskill member path")?;
        if witness.declaration.path != format!("subskills/{}/vibe-subskill.toml", self.path)
            || witness.member.path != format!("subskills/{}/{member}", self.path)
            || witness.declaration.byte_len > 64 * 1024
            || witness.member.byte_len > 1024 * 1024
        {
            return Err("configured subskill source paths or bounds disagree".into());
        }
        for file in [&witness.declaration, &witness.member] {
            if file.sha256.len() != 64
                || !file
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err("configured subskill witness requires exact lowercase SHA-256".into());
            }
        }
        Ok(())
    }
}
