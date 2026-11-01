use super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct RemovalEffects {
    pub any: bool,
    pub bin: bool,
    pub binary_source: bool,
    pub managed_source: bool,
    pub failed: bool,
}

impl RemovalEffects {
    pub(super) fn merge(&mut self, other: RemovalEffects) {
        self.any |= other.any;
        self.bin |= other.bin;
        self.binary_source |= other.binary_source;
        self.managed_source |= other.managed_source;
        self.failed |= other.failed;
    }
}

/// A remark a scoped removal leaves behind: a warning when the removal
/// left an instance that will not activate, otherwise a dim remark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RemovalNote {
    Warning(&'static str),
    Remark(&'static str),
}

impl std::ops::Deref for RemovalNote {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            RemovalNote::Warning(text) | RemovalNote::Remark(text) => text,
        }
    }
}

impl PartialEq<&str> for RemovalNote {
    fn eq(&self, other: &&str) -> bool {
        **self == **other
    }
}

pub(super) fn scoped_removal_notes(
    scope: RemoveScope,
    effects: RemovalEffects,
    selected_external: bool,
    selected_managed: bool,
    selected_binary: bool,
    mirror_candidate: bool,
) -> Vec<RemovalNote> {
    use RemovalNote::{Remark, Warning};
    if scope == RemoveScope::Both {
        return Vec::new();
    }
    if scope == RemoveScope::Bin {
        return vec![if effects.bin {
            Warning(
                "scoped removal intentionally retained a partial instance; it is non-activatable until reinstalled.",
            )
        } else {
            Remark("no instance binaries were present to remove.")
        }];
    }

    let mut notes = Vec::new();
    if selected_binary {
        notes.push(if effects.binary_source {
            Warning(
                "scoped removal intentionally retained a partial binary bundle; it is non-activatable until reinstalled.",
            )
        } else {
            Remark("no instance-owned binary source was present to remove.")
        });
    }
    if selected_managed {
        notes.push(if effects.managed_source {
            Warning(
                "shared managed source was removed; retained binaries no longer have managed source provenance.",
            )
        } else if mirror_candidate {
            Remark("no managed source was present to remove.")
        } else {
            Remark(
                "shared managed mirror retained because another installed managed record still uses it.",
            )
        });
    }
    if selected_external {
        notes.push(Remark(
            "external checkout retained: VVM never deletes contributor-owned source trees.",
        ));
    }
    if notes.is_empty() {
        notes.push(Remark("no instance-owned source was present to remove."));
    }
    notes
}

pub(super) fn guard_remove_targets(
    store: &VersionStore,
    state: &model::State,
    targets: &[RemoveTarget],
    scope: RemoveScope,
) -> Result<()> {
    for record in state
        .installs
        .iter()
        .filter(|record| targets.iter().any(|target| target.matches(record)))
    {
        let id = record.version_id();
        let home = store.instance_dir(&id, record.instance);
        match scope {
            RemoveScope::Both => store.guard_mutation_tree(&home)?,
            RemoveScope::Bin => {
                store.guard_mutation_tree(&store.instance_bin_dir(&id, record.instance))?;
                store.guard_mutation_path(&home.join(crate::commands::vvm::store::BINARY_NAME))?;
            }
            RemoveScope::Src if record.origin == model::Origin::Binary => {
                store.guard_mutation_tree(&store.instance_source_dir(&id, record.instance))?;
                store.guard_mutation_path(&home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME))?;
            }
            RemoveScope::Src => {}
        }
    }
    Ok(())
}

pub(super) fn removes_last_managed(
    state: &model::State,
    targets: &[RemoveTarget],
    scope: RemoveScope,
) -> bool {
    matches!(scope, RemoveScope::Src | RemoveScope::Both)
        && state.installs.iter().any(|record| {
            record.origin == model::Origin::Managed && targets.iter().any(|t| t.matches(record))
        })
        && !state.installs.iter().any(|record| {
            record.origin == model::Origin::Managed
                && !targets.iter().any(|target| target.matches(record))
        })
}

pub(super) fn remove_mirror_after(
    candidate: bool,
    scope: RemoveScope,
    state: &model::State,
) -> bool {
    candidate
        && (scope == RemoveScope::Src
            || !state
                .installs
                .iter()
                .any(|record| record.origin == model::Origin::Managed))
}
