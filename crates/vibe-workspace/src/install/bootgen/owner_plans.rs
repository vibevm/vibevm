//! Boot fingerprint adapters over retained owner runtimes.

use std::collections::HashMap;

use crate::WorkspaceError;
use crate::boot::hybrid::UnitId;
use crate::extension_world::{ExtensionWorldError, LoweredOwnerRuntimes};
use vibe_core::manifest::SpecFormat;

/// Unit fingerprint frames from the retained canonical runtime plans.
pub(super) fn plan_digest_frames(runtimes: &LoweredOwnerRuntimes) -> HashMap<UnitId, String> {
    plan_digest_frames_for(runtimes, SpecFormat::Mixed)
}

pub(super) fn plan_digest_frames_for(
    runtimes: &LoweredOwnerRuntimes,
    spec_format: SpecFormat,
) -> HashMap<UnitId, String> {
    let artifact = if matches!(spec_format, SpecFormat::Xml) {
        "static-xml"
    } else {
        "static-md"
    };
    runtimes
        .units()
        .iter()
        .filter_map(|(owner, runtime)| {
            runtime
                .compile_plans()
                .digest_hex_for_artifact(artifact)
                .map(|digest| ((owner.group().clone(), owner.name().to_string()), digest))
        })
        .collect()
}

pub(super) fn world_error(source: ExtensionWorldError) -> WorkspaceError {
    WorkspaceError::ExtensionWorld {
        source: Box::new(source),
    }
}

#[cfg(test)]
#[path = "owner_plans_tests.rs"]
mod tests;
