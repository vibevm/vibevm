//! Building the artifact's ordered inputs, and naming the TYPED provider
//! that declared each document-producing one (R4 architecture §5.3).
//!
//! Split out of the boot-artifact cell along its own seam: what an input IS
//! built from — a boot entry's kind, its path, its provenance — is a
//! different job from compiling, tracing and publishing the lane, and it is
//! the job T10B changed. Everything here is pure apart from reading a
//! `simple` contribution's text.
//!
//! **The one place a boot document's provider is named.** `entry.origin` is
//! display and may carry a `[shared by …]` suffix; identity comes from
//! [`BootProvenance`], the pair the composition authored beside that string.
//! Nothing in this file parses a coordinate out of a rendered spelling.

use std::path::Path;

use vibe_core::{Group, PackageName};
use vibe_spec::{ArtifactInput, DocumentProvider, SelfCoordinate};

use crate::boot::{BootEntry, BootProvenance};
use crate::{WorkspaceError, layout_paths};

use super::normal::{hoisted_seed, normal_seed};

/// Build one artifact's inputs from the lane's static entries, in order,
/// each with its TYPED declaring provider beside it — the attribution
/// side the R4.3 analyzer lowers into its report (`bootgen/analyze.rs`)
/// and the write path simply drops.
///
/// A document-producing entry (normal/simple) answers the provider
/// question STRICTLY — the T10B law, refusal propagating. An elided or
/// hoisted entry produces no document, so its input takes the legacy
/// constructor and its provider is typed when the provenance parses and
/// `None` when it does not: the write path never reads these (hostile
/// marker-prose names are legal lane material it must keep emitting),
/// and the analyzer refuses an unattributable seat rather than parking
/// it in a pseudo-provider.
pub(super) fn build_with_providers(
    entries: Vec<&BootEntry>,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
) -> Result<(Vec<ArtifactInput>, Vec<Option<DocumentProvider>>), WorkspaceError> {
    let mut inputs = Vec::with_capacity(entries.len());
    let mut providers = Vec::with_capacity(entries.len());
    for entry in entries {
        let provider = document_provider(entry, self_coord);
        let (input, seat) = if entry.elided {
            // An elided contribution produces NO document, so no
            // source/document transform is ever invoked for it and the
            // input takes the legacy constructor; the seat's provider is
            // typed when the provenance parses and `None` when it does
            // not (hostile marker-prose names are legal lane material).
            (
                ArtifactInput::elided(&entry.origin, &entry.path),
                provider.ok(),
            )
        } else if entry.use_ref {
            let target = hoisted_seed(&entry.origin, &entry.path).ok_or_else(|| {
                WorkspaceError::InlineCompile {
                    reason: format!(
                        "cannot derive the hoisted document target for `{}` at `{}`",
                        entry.origin, entry.path
                    ),
                }
            })?;
            (
                ArtifactInput::hoisted(&entry.origin, &entry.path, target),
                provider.ok(),
            )
        } else {
            // A document-producing entry answers the provider question
            // STRICTLY — the T10B law, refusal propagating.
            let typed = provider?;
            if entry.format.is_normal() {
                let seed = normal_seed(&entry.origin, &entry.path).ok_or_else(|| {
                    WorkspaceError::InlineCompile {
                        reason: format!(
                            "cannot derive a spec:// seed for the normal package `{}` at `{}` \
                             (PROP-035 §8): expected a `<group>/<name>` origin and a path under a \
                             package's `{}` root",
                            entry.origin,
                            entry.path,
                            layout_paths::slot_specs("<slot>", "")
                        ),
                    }
                })?;
                (
                    ArtifactInput::normal_declared_by(
                        &entry.origin,
                        &entry.path,
                        seed,
                        typed.clone(),
                    ),
                    Some(typed),
                )
            } else {
                let absolute = workspace_root.join(&entry.path);
                let (markdown, _) = vibe_specdoc::load_spec_text(&absolute).map_err(|error| {
                    WorkspaceError::Io {
                        path: absolute,
                        reason: error.to_string(),
                    }
                })?;
                (
                    ArtifactInput::simple_declared_by(
                        &entry.origin,
                        &entry.path,
                        markdown,
                        typed.clone(),
                    ),
                    Some(typed),
                )
            }
        };
        let input = input.map_err(|error| WorkspaceError::InlineCompile {
            reason: error.to_string(),
        })?;
        inputs.push(input);
        providers.push(seat);
    }
    Ok((inputs, providers))
}

/// The typed provider that DECLARED one document-producing boot entry.
///
/// The whole point of the typed carriage: `entry.origin` is display and may
/// carry a `[shared by …]` suffix, so identity comes from
/// [`BootProvenance`] instead — the pair the composition authored beside
/// that string.
///
/// The host arms mirror `extension_world::host_source`'s projection
/// component for component, because they answer the same question about the
/// same node: a coordinate-bearing node is a `HostCoordinate`, an ungrouped
/// project is a `HostUngrouped`, and a node that declares neither is the
/// virtual workspace. Reading them differently in two places would let one
/// node be two providers.
///
/// A name the install model still carries as a bare string is parsed HERE,
/// through `PackageName`'s one grammar, and refused typed on failure —
/// never panicked on, and never quietly downgraded to
/// [`DocumentProvider::Undetermined`], which would restore exactly the
/// silent mismatch the typed subject exists to remove.
fn document_provider(
    entry: &BootEntry,
    self_coord: &SelfCoordinate,
) -> Result<DocumentProvider, WorkspaceError> {
    match &entry.provenance {
        BootProvenance::Dependency { group, name } => Ok(DocumentProvider::Dependency {
            group: group.clone(),
            name: typed_name(&entry.origin, "dependency name", name)?,
        }),
        BootProvenance::Node => match (&self_coord.group, self_coord.name.as_str()) {
            (_, "") => Ok(DocumentProvider::HostVirtualWorkspace),
            (Some(group), name) => Ok(DocumentProvider::HostCoordinate {
                group: typed_group(&entry.origin, group)?,
                name: typed_name(&entry.origin, "[project].name", name)?,
            }),
            // An ungrouped project declares no coordinate: its authored name
            // is carried exactly as written, through the same arm the kernel
            // reserves for it.
            (None, name) => Ok(DocumentProvider::HostUngrouped {
                name: name.to_owned(),
            }),
        },
    }
}

/// Parse one bare-string package-name component through its one grammar.
fn typed_name(
    origin: &str,
    component: &'static str,
    spelling: &str,
) -> Result<PackageName, WorkspaceError> {
    PackageName::parse(spelling).map_err(|error| WorkspaceError::UntypedBootProvenance {
        origin: origin.to_owned(),
        component,
        spelling: spelling.to_owned(),
        reason: error.to_string(),
    })
}

/// Parse one bare-string group component through its one grammar.
fn typed_group(origin: &str, spelling: &str) -> Result<Group, WorkspaceError> {
    Group::parse(spelling).map_err(|error| WorkspaceError::UntypedBootProvenance {
        origin: origin.to_owned(),
        component: "[project].group",
        spelling: spelling.to_owned(),
        reason: error.to_string(),
    })
}

#[cfg(test)]
#[path = "inputs_tests.rs"]
mod tests;
