//! What a bare spelling finds in a local-directory registry
//! (PROP-008 `#short-name`): the candidate set its index answers with,
//! and the alias its owner offers for that spelling.
//!
//! Split from the registry's own file so the reader of either half has
//! one subject in front of them.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use std::fs;

use vibe_core::Group;

use super::LocalRegistry;
use crate::RegistryError;
use crate::short_name::{AliasTarget, NameLookup};

impl LocalRegistry {
    /// [`Self::indexed_candidate_groups`] with the short spelling this
    /// registry OFFERS for the name, if any (PROP-008 §2.6).
    ///
    /// Current index: the alias rides on the persisted candidate-set
    /// file, so it is one read of the file that was already being read.
    /// Stale or absent index: the authored table
    /// `_index/aliases.toml` answers instead, exactly as the directory
    /// listing answers for the candidates — the tree and the table are
    /// the two things a hand-built registry really has. The table's row
    /// is still held to the two rules that could make it wrong here: a
    /// target the tree does not carry, and a name the tree publishes
    /// itself, are both dropped rather than offered.
    pub fn indexed_name_lookup(&self, name: &str) -> Result<NameLookup, RegistryError> {
        use vibe_core::layout_registry::{IndexState, index_dir, index_state};
        #[derive(serde::Deserialize)]
        struct NameFile {
            #[serde(default)]
            packages: Vec<NamePackage>,
            #[serde(default)]
            alias: Option<NameAlias>,
        }
        #[derive(serde::Deserialize)]
        struct NamePackage {
            group: Group,
        }
        #[derive(serde::Deserialize)]
        struct NameAlias {
            group: Group,
            name: String,
        }
        let state = index_state(&self.root).map_err(|source| RegistryError::Io {
            path: self.root.clone(),
            source,
        })?;
        if state != IndexState::Current {
            return Ok(NameLookup {
                groups: self.candidate_groups(name)?,
                alias: self.authored_alias(name)?,
            });
        }
        let path = index_dir(&self.root)
            .join("by-name")
            .join(format!("{name}.json"));
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(NameLookup::default());
            }
            Err(source) => return Err(RegistryError::Io { path, source }),
        };
        let parsed: NameFile =
            serde_json::from_slice(&bytes).map_err(|e| RegistryError::MalformedMeta {
                path: path.clone(),
                reason: e.to_string(),
            })?;
        let mut groups: Vec<Group> = parsed.packages.into_iter().map(|p| p.group).collect();
        groups.sort();
        groups.dedup();
        Ok(NameLookup {
            groups,
            alias: parsed.alias.map(|a| AliasTarget {
                group: a.group,
                name: a.name,
            }),
        })
    }

    /// The alias the registry's own table offers for `name`, read
    /// straight from `_index/aliases.toml` because the projection that
    /// would have carried it is not current.
    ///
    /// The table is parsed here rather than through `vibe_index`, the
    /// way the narrow `by-name` reader above is: this crate reads a
    /// registry's files, it does not build them, and a dependency on
    /// the builder for two fields would point the arrow the wrong way.
    fn authored_alias(&self, name: &str) -> Result<Option<AliasTarget>, RegistryError> {
        let path = vibe_core::layout_registry::index_dir(&self.root)
            .join(vibe_core::layout_registry::ALIAS_TABLE_FILE);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(RegistryError::Io { path, source }),
        };
        let rows = crate::index_client::by_name::parse_alias_table(&text).map_err(|reason| {
            RegistryError::MalformedMeta {
                path: path.clone(),
                reason,
            }
        })?;
        let Some((_, target)) = rows.into_iter().find(|(alias, _)| alias == name) else {
            return Ok(None);
        };
        let (group, target_name) = (target.group, target.name);
        let target_name = target_name.as_str();
        // The two rules that could make an authored row wrong against
        // this tree, applied here because no projection did it for us:
        // a name the tree itself publishes outranks the row, and a
        // target the tree does not carry cannot be offered at all.
        if !self.candidate_groups(name)?.is_empty() {
            return Ok(None);
        }
        if !self.root.join(group.as_str()).join(target_name).is_dir() {
            return Ok(None);
        }
        Ok(Some(AliasTarget {
            group,
            name: target_name.to_string(),
        }))
    }
}
