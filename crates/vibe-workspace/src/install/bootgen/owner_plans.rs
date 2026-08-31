//! Boot fingerprint adapters over retained owner runtimes.

use std::collections::HashMap;

use crate::WorkspaceError;
use crate::boot::hybrid::UnitId;
use crate::extension_world::{ExtensionWorldError, LoweredOwnerRuntimes};

/// Unit fingerprint frames from the retained canonical runtime plans.
pub(super) fn plan_digest_frames(runtimes: &LoweredOwnerRuntimes) -> HashMap<UnitId, String> {
    runtimes
        .units()
        .iter()
        .filter_map(|(owner, runtime)| {
            runtime
                .transform_plan()
                .digest_hex()
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
