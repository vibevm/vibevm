//! The user environment of application env steps (PROP-065 `#env-step`):
//! on Windows `HKCU\Environment` edited raw with each value's kind kept and
//! the change broadcast, through the registry plumbing of `vibe self`
//! (PROP-019); elsewhere `vibe-app`'s managed `env.sh` / `env.fish` and the
//! startup-file block under the user's home.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#env-step");

use std::path::Path;

use anyhow::Result;
use vibe_app::{EnvEntry, EnvOp, ProfileEnvironment, UserEnvironment, ValueKind};

use crate::commands::vvm::env::{
    Hkcu, RawUserVar, RegistryValueKind, UserRegistry, path_with_prefix, path_without,
};

/// The environment of this host: the registry on Windows, otherwise the
/// managed files under `settings` and `home`. Without a resolvable home a
/// POSIX host has no startup files to write; env steps then fail clearly.
pub(crate) fn for_host(settings: &Path, home: Option<&Path>) -> Option<Box<dyn UserEnvironment>> {
    if cfg!(windows) {
        return Some(Box::new(RegistryEnvironment::new(Hkcu)));
    }
    home.map(|home| Box::new(ProfileEnvironment::new(settings, home)) as Box<dyn UserEnvironment>)
}

/// `HKCU\Environment`: values written raw with their kind preserved (a new
/// value is `REG_EXPAND_SZ` when it names a `%VAR%`, else `REG_SZ`), PATH
/// edited only by prepending, every replaced value recorded so removal
/// restores it.
///
/// The registry is reached through the [`UserRegistry`] seam of
/// `vibe self` (PROP-019 `#path`), so the whole apply/remove round trip is
/// covered against an in-memory environment and no test touches the real
/// `HKCU\Environment`.
pub(crate) struct RegistryEnvironment<R> {
    registry: R,
}

impl<R: UserRegistry> RegistryEnvironment<R> {
    pub(crate) fn new(registry: R) -> Self {
        Self { registry }
    }
}

/// The planned registry write of one entry: the variable, its new raw
/// value (`None` deletes it) and the kind to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryWrite {
    pub name: String,
    pub value: Option<String>,
    pub kind: RegistryValueKind,
}

fn to_kind(kind: RegistryValueKind) -> ValueKind {
    match kind {
        RegistryValueKind::String => ValueKind::String,
        RegistryValueKind::ExpandString => ValueKind::ExpandString,
    }
}

fn from_kind(kind: ValueKind) -> RegistryValueKind {
    match kind {
        ValueKind::String => RegistryValueKind::String,
        ValueKind::ExpandString => RegistryValueKind::ExpandString,
    }
}

/// The write that applies `entry` over `current`, and the entry with the
/// replaced value recorded; `None` when nothing changes.
pub(crate) fn apply_write(
    entry: &EnvEntry,
    current: Option<&RawUserVar>,
) -> (Option<RegistryWrite>, EnvEntry) {
    let mut recorded = entry.clone();
    recorded.previous = current.map(|raw| raw.value.clone());
    recorded.previous_kind = current.map(|raw| to_kind(raw.kind));
    let value = entry.value.clone().unwrap_or_default();
    let write = match entry.op {
        EnvOp::Set => Some(RegistryWrite {
            name: entry.name.clone(),
            kind: current.map_or_else(
                || {
                    if value.contains('%') {
                        RegistryValueKind::ExpandString
                    } else {
                        RegistryValueKind::String
                    }
                },
                |raw| raw.kind,
            ),
            value: Some(value),
        }),
        EnvOp::Unset => current.map(|raw| RegistryWrite {
            name: entry.name.clone(),
            value: None,
            kind: raw.kind,
        }),
        EnvOp::PathPrepend => {
            // A prepend replaces nothing, so no PATH text is kept: removal
            // edits the live value instead of restoring a snapshot, which
            // would resurrect entries added since. The `previous_kind`
            // recorded above stays, because it is `Some` exactly when the
            // variable existed — which is how a removal that empties the
            // value tells one vibe created from one that was already there
            // (PROP-059 `#path-directories`).
            recorded.previous = None;
            let raw = current.map(|raw| raw.value.as_str()).unwrap_or_default();
            path_with_prefix(raw, &value).map(|next| RegistryWrite {
                name: "Path".into(),
                value: Some(next),
                kind: current.map_or(RegistryValueKind::ExpandString, |raw| raw.kind),
            })
        }
    };
    (write, recorded)
}

/// The write that removes `entry` from `current`: a set value is restored
/// or deleted, an unset one restored, a PATH entry taken out again — and
/// a `Path` that vibe itself created is deleted rather than left empty
/// once its last entry is gone.
pub(crate) fn remove_write(
    entry: &EnvEntry,
    current: Option<&RawUserVar>,
) -> Option<RegistryWrite> {
    match entry.op {
        EnvOp::Set | EnvOp::Unset => Some(RegistryWrite {
            name: entry.name.clone(),
            value: entry.previous.clone(),
            kind: entry
                .previous_kind
                .map(from_kind)
                .or(current.map(|raw| raw.kind))
                .unwrap_or(RegistryValueKind::String),
        })
        .filter(|write| write.value.is_some() || current.is_some() || entry.op == EnvOp::Set),
        EnvOp::PathPrepend => {
            let raw = current?;
            let value = entry.value.as_deref().unwrap_or_default();
            let next = path_without(&raw.value, value)?;
            // Nothing of the value is left. A `Path` vibe created where the
            // user had none goes away with vibe's last entry; one that was
            // there before — even empty — is restored exactly as it was,
            // because deleting it would be a change vibe never made.
            let delete = next.is_empty() && entry.previous_kind.is_none();
            Some(RegistryWrite {
                name: "Path".into(),
                value: (!delete).then_some(next),
                kind: raw.kind,
            })
        }
    }
}

fn registry_name(entry: &EnvEntry) -> &str {
    match entry.op {
        EnvOp::PathPrepend => "Path",
        EnvOp::Set | EnvOp::Unset => &entry.name,
    }
}

impl<R: UserRegistry> RegistryEnvironment<R> {
    fn perform(&self, write: &RegistryWrite) -> Result<()> {
        match &write.value {
            Some(value) => self.registry.set_raw(&write.name, value, write.kind),
            None => self.registry.delete_raw(&write.name),
        }
    }
}

impl<R: UserRegistry> UserEnvironment for RegistryEnvironment<R> {
    fn apply(&self, _key: &str, entries: &[EnvEntry]) -> Result<Vec<EnvEntry>, String> {
        let mut applied = Vec::new();
        for entry in entries {
            let current = self
                .registry
                .get_raw(registry_name(entry))
                .map_err(|e| format!("{e:#}"))?;
            let (write, recorded) = apply_write(entry, current.as_ref());
            if let Some(write) = write {
                self.perform(&write).map_err(|error| format!("{error:#}"))?;
            }
            applied.push(recorded);
        }
        Ok(applied)
    }

    fn remove(&self, _key: &str, entries: &[EnvEntry]) -> Result<(), String> {
        for entry in entries.iter().rev() {
            let current = self
                .registry
                .get_raw(registry_name(entry))
                .map_err(|e| format!("{e:#}"))?;
            if let Some(write) = remove_write(entry, current.as_ref()) {
                self.perform(&write).map_err(|error| format!("{error:#}"))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(op: EnvOp, name: &str, value: Option<&str>) -> EnvEntry {
        EnvEntry {
            step: "env".into(),
            op,
            name: name.into(),
            value: value.map(str::to_owned),
            previous: None,
            previous_kind: None,
        }
    }

    fn raw(value: &str, kind: RegistryValueKind) -> RawUserVar {
        RawUserVar {
            value: value.into(),
            kind,
        }
    }

    #[test]
    fn a_set_keeps_the_kind_records_the_previous_value_and_restores_it() {
        let set = entry(EnvOp::Set, "TOOL_HOME", Some(r"%LOCALAPPDATA%\tool"));
        let (write, recorded) = apply_write(&set, None);
        let write = write.unwrap();
        assert_eq!(write.kind, RegistryValueKind::ExpandString);
        assert_eq!(recorded.previous, None);
        let old = raw("C:\\old", RegistryValueKind::String);
        let (write_over, recorded) = apply_write(&set, Some(&old));
        assert_eq!(write_over.unwrap().kind, RegistryValueKind::String);
        assert_eq!(recorded.previous.as_deref(), Some("C:\\old"));
        let restore = remove_write(&recorded, Some(&raw("x", RegistryValueKind::String))).unwrap();
        assert_eq!(restore.value.as_deref(), Some("C:\\old"));
        assert_eq!(restore.kind, RegistryValueKind::String);
        let fresh = apply_write(&set, None).1;
        assert_eq!(
            remove_write(&fresh, Some(&old)).unwrap().value,
            None,
            "deleted"
        );
    }

    #[test]
    fn path_entries_are_prepended_and_taken_out_again() {
        let path = entry(EnvOp::PathPrepend, "PATH", Some("/opt/tool/bin"));
        let current = raw("/usr/bin;/bin", RegistryValueKind::ExpandString);
        let (write, _) = apply_write(&path, Some(&current));
        let write = write.unwrap();
        assert_eq!(write.name, "Path");
        assert_eq!(write.value.as_deref(), Some("/opt/tool/bin;/usr/bin;/bin"));
        assert_eq!(write.kind, RegistryValueKind::ExpandString);
        let after = raw(
            "/opt/tool/bin;/usr/bin;/bin",
            RegistryValueKind::ExpandString,
        );
        let undo = remove_write(&path, Some(&after)).unwrap();
        assert_eq!(undo.value.as_deref(), Some("/usr/bin;/bin"));
    }

    /// The environment of this test, over an in-memory `HKCU\Environment`.
    fn registry(
        values: &[(&str, &str, RegistryValueKind)],
    ) -> RegistryEnvironment<crate::commands::vvm::env::MemoryRegistry> {
        RegistryEnvironment::new(crate::commands::vvm::env::MemoryRegistry::with(values))
    }

    /// The last link directory off the durable PATH takes the value with
    /// it when the user had no `Path` of their own: an empty string is a
    /// value vibe left behind, not the state it found
    /// (`PROP-059#path-directories`).
    #[test]
    fn the_last_path_entry_deletes_a_value_vibe_created() {
        let env = registry(&[]);
        let entry = entry(EnvOp::PathPrepend, "PATH", Some("/opt/slot/0"));
        let applied = env.apply("org.example/runtime@1.0.0", &[entry]).unwrap();
        assert_eq!(
            env.registry.value("Path").unwrap().value,
            "/opt/slot/0",
            "vibe created the value"
        );
        assert_eq!(applied[0].previous_kind, None, "there was nothing before");
        env.remove("org.example/runtime@1.0.0", &applied).unwrap();
        assert_eq!(env.registry.value("Path"), None, "the value is gone");
    }

    /// A `Path` that was already there is restored exactly, even when it
    /// was empty: removing it would be a change vibe never made.
    #[test]
    fn the_last_path_entry_restores_a_value_that_was_already_there() {
        for before in ["", "/usr/bin;/bin"] {
            let env = registry(&[("Path", before, RegistryValueKind::ExpandString)]);
            let entry = entry(EnvOp::PathPrepend, "PATH", Some("/opt/slot/0"));
            let applied = env.apply("org.example/runtime@1.0.0", &[entry]).unwrap();
            assert_eq!(
                applied[0].previous_kind,
                Some(ValueKind::ExpandString),
                "the value existed, so its kind is recorded"
            );
            env.remove("org.example/runtime@1.0.0", &applied).unwrap();
            let after = env.registry.value("Path").expect("still there");
            assert_eq!(after.value, before, "restored byte for byte");
            assert_eq!(after.kind, RegistryValueKind::ExpandString);
        }
    }

    /// Both link directories of one slot: the second comes off without
    /// touching the value, the first takes it with it.
    #[test]
    fn two_path_entries_of_one_slot_come_off_in_reverse() {
        let env = registry(&[]);
        let entries = [
            entry(EnvOp::PathPrepend, "PATH", Some("/opt/slot/0")),
            entry(EnvOp::PathPrepend, "PATH", Some("/opt/slot/1")),
        ];
        let applied = env.apply("org.example/runtime@1.0.0", &entries).unwrap();
        assert_eq!(
            env.registry.value("Path").unwrap().value,
            "/opt/slot/1;/opt/slot/0"
        );
        assert_eq!(applied[0].previous_kind, None);
        assert_eq!(
            applied[1].previous_kind,
            Some(ValueKind::ExpandString),
            "the first entry had just created the value"
        );
        env.remove("org.example/runtime@1.0.0", &applied).unwrap();
        assert_eq!(env.registry.value("Path"), None);
    }

    #[test]
    fn an_unset_of_an_absent_value_writes_nothing() {
        let unset = entry(EnvOp::Unset, "TOOL_DEBUG", None);
        let (write, recorded) = apply_write(&unset, None);
        assert!(write.is_none());
        assert!(remove_write(&recorded, None).is_none());
        let (write, recorded) = apply_write(&unset, Some(&raw("1", RegistryValueKind::String)));
        assert_eq!(write.unwrap().value, None);
        assert_eq!(
            remove_write(&recorded, None).unwrap().value.as_deref(),
            Some("1")
        );
    }
}
