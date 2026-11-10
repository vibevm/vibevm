//! Closed version admission followed by an in-memory routing view for the proxy.
//! The full generated index is decoded first, including configuration metadata.
//! This view is never deserialized independently, persisted, or used to migrate state.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#proxy");
use serde::Deserialize;
use std::collections::BTreeMap;
use vibe_wire::generated::application::{
    e2::index as legacy, e3::index as current, e4::index as evidence, e5::index as paths,
    e6::index as migrations,
};

pub(super) struct ApplicationIndex {
    pub installations: BTreeMap<String, Installation>,
    pub slots: BTreeMap<String, Slot>,
}
pub(super) struct Slot {
    pub default: Option<String>,
    /// Switched off (`PROP-059#slot-off`): the slot resolves to nothing and
    /// a proxy of its commands falls through to the next same-named command
    /// on PATH. Only a protocol6 index can carry it.
    pub off: bool,
}
pub(super) struct Installation {
    pub slot: String,
    pub library: Option<String>,
    pub directory: Option<String>,
    pub app_root: Option<String>,
    pub launchers: Vec<Launcher>,
    pub public_commands: Vec<PublicCommand>,
}
pub(super) struct Launcher {
    pub id: String,
    pub executable: String,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
}
pub(super) struct PublicCommand {
    pub name: String,
    pub launcher: String,
}

pub(super) fn read(bytes: &[u8]) -> Result<ApplicationIndex, String> {
    #[derive(Deserialize)]
    struct Header {
        protocol: String,
    }
    let header: Header = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    match header.protocol.as_str() {
        "vibe-user-applications/3" => serde_json::from_slice::<legacy::ApplicationIndex>(bytes)
            .map(Into::into)
            .map_err(|error| error.to_string()),
        "vibe-user-applications/4" => serde_json::from_slice::<current::ApplicationIndex>(bytes)
            .map(Into::into)
            .map_err(|error| error.to_string()),
        "vibe-user-applications/5" => serde_json::from_slice::<evidence::ApplicationIndex>(bytes)
            .map(Into::into)
            .map_err(|error| error.to_string()),
        "vibe-user-applications/6" => serde_json::from_slice::<paths::ApplicationIndex>(bytes)
            .map(Into::into)
            .map_err(|error| error.to_string()),
        "vibe-user-applications/7" => serde_json::from_slice::<migrations::ApplicationIndex>(bytes)
            .map(Into::into)
            .map_err(|error| error.to_string()),
        _ => Err(format!(
            "application index protocol `{}` is unsupported; expected `vibe-user-applications/3` … `vibe-user-applications/7`",
            header.protocol
        )),
    }
}

// Both admitted versions use exactly the same routing fields. Moving them out
// avoids a duplicate JSON decoder or a dependency on the application runtime.
macro_rules! routing_view {
    ($wire:ident) => {
        impl From<$wire::ApplicationIndex> for ApplicationIndex {
            fn from(index: $wire::ApplicationIndex) -> Self {
                Self {
                    slots: index
                        .slots
                        .into_iter()
                        .map(|(key, slot)| {
                            let off = slot_off(&slot);
                            (
                                key,
                                Slot {
                                    default: slot.default,
                                    off,
                                },
                            )
                        })
                        .collect(),
                    installations: index
                        .installations
                        .into_iter()
                        .map(|(key, record)| {
                            (
                                key,
                                Installation {
                                    slot: record.slot,
                                    library: record.library,
                                    directory: record.directory,
                                    app_root: record.app_root,
                                    public_commands: record
                                        .public_commands
                                        .into_iter()
                                        .map(|command| PublicCommand {
                                            name: command.name,
                                            launcher: command.launcher,
                                        })
                                        .collect(),
                                    launchers: record
                                        .launchers
                                        .into_iter()
                                        .map(|launcher| Launcher {
                                            id: launcher.id,
                                            executable: launcher.executable,
                                            args: launcher.args,
                                            working_directory: launcher.working_directory,
                                        })
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                }
            }
        }
    };
}
routing_view!(legacy);
routing_view!(current);
routing_view!(evidence);
routing_view!(paths);
routing_view!(migrations);

/// Whether a wire slot is switched off; the protocol6 shape is the first
/// that carries the switch, and the frozen readers below it answer `false`
/// by construction.
trait SlotOff {
    fn is_off(&self) -> bool {
        false
    }
}
impl SlotOff for legacy::Slot {}
impl SlotOff for current::Slot {}
impl SlotOff for evidence::Slot {}
impl SlotOff for paths::Slot {
    fn is_off(&self) -> bool {
        self.off
    }
}
impl SlotOff for migrations::Slot {
    fn is_off(&self) -> bool {
        self.off
    }
}
fn slot_off<S: SlotOff>(slot: &S) -> bool {
    slot.is_off()
}
