//! Contract-to-health-plan preparation. No child process is started here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use crate::contract::{
    BaselineMode, CustomProtocol, Healthcheck, MavenRunner, NetworkPolicy, NodeManager, TestsMode,
};
use crate::glob::Glob;
use crate::model::{EntryKind, Inventory};

use super::model::*;
use super::preset::{cargo_commands, maven_commands, npm_commands, python_commands};
use super::{preset, protocol};

mod checks;
mod modes;

use checks::prepare_check;
use modes::{blocker_for, health_identity};

const CUSTOM_FILE_CAP: usize = 16 * 1024 * 1024;
const CUSTOM_BUNDLE_CAP: u64 = 64 * 1024 * 1024;

/// Injected executable/test discovery. Implementations may observe and seal
/// identities, but must not execute a shell command or mutate the project.
pub trait HealthResolver {
    fn resolve_asset(&mut self, request: ResolveAssetRequest)
    -> Result<AssetIdentity, HealthError>;

    fn resolve_custom_launch(
        &mut self,
        check_id: &str,
        interpreter: &str,
        source: &str,
    ) -> Result<ResolvedCustomLaunch, HealthError>;

    fn discover_tests(
        &mut self,
        project: &Project,
        inventory: &Inventory,
        request: &TestDiscoveryRequest,
    ) -> Result<TestPresence, HealthError>;
}

pub fn prepare<R: HealthResolver>(
    project: &Project,
    contract: &crate::contract::Contract,
    inventory: &Inventory,
    resolver: &mut R,
) -> Result<PreparedHealth, HealthError> {
    usize::try_from(contract.health.max_stdout_bytes).map_err(|_| {
        HealthError::Preparation("health.max_stdout_bytes exceeds this platform's usize".to_owned())
    })?;
    usize::try_from(contract.health.max_stderr_bytes).map_err(|_| {
        HealthError::Preparation("health.max_stderr_bytes exceeds this platform's usize".to_owned())
    })?;
    usize::try_from(contract.health.max_result_bytes).map_err(|_| {
        HealthError::Preparation("health.max_result_bytes exceeds this platform's usize".to_owned())
    })?;
    let mut rows = contract.healthcheck.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.id().as_bytes().cmp(right.id().as_bytes()));
    let mut checks = Vec::with_capacity(rows.len());
    let mut blockers = Vec::new();
    for row in rows {
        match prepare_check(project, contract, inventory, resolver, row) {
            Ok(check) => checks.push(check),
            Err(error) => blockers.push(blocker_for(row.id(), error)),
        }
    }
    if !checks
        .iter()
        .any(|check| check.applicability == Applicability::Applicable)
    {
        blockers.push(HealthBlocker {
            code: "health-no-applicable-required-check".to_owned(),
            check_id: None,
            message: "the health panel has no applicable required check".to_owned(),
        });
    }
    blockers.sort_by(|left, right| {
        (&left.code, &left.check_id, &left.message).cmp(&(
            &right.code,
            &right.check_id,
            &right.message,
        ))
    });
    let baseline = match contract.health.baseline {
        BaselineMode::Strict => BaselinePolicy::Strict,
        BaselineMode::NoRegression => BaselinePolicy::NoRegression,
    };
    let mut prepared = PreparedHealth {
        plan_id: String::new(),
        baseline,
        max_stdout_bytes: contract.health.max_stdout_bytes,
        max_stderr_bytes: contract.health.max_stderr_bytes,
        max_result_bytes: contract.health.max_result_bytes,
        termination_grace_seconds: contract.health.termination_grace_seconds,
        checks,
        blockers,
    };
    if let Some(blocker) = persistence_capacity_blocker(&prepared, inventory)? {
        prepared.blockers.push(blocker);
        prepared.blockers.sort_by(|left, right| {
            (&left.code, &left.check_id, &left.message).cmp(&(
                &right.code,
                &right.check_id,
                &right.message,
            ))
        });
    }
    prepared.plan_id = health_identity(&prepared)?;
    Ok(prepared)
}

pub(crate) fn persistence_capacity_blocker(
    prepared: &PreparedHealth,
    inventory: &Inventory,
) -> Result<Option<HealthBlocker>, HealthError> {
    const JSON_EXPANSION: u128 = 6;
    const FIXED_ENVELOPE: u128 = 4 * 1024 * 1024;

    let commands = prepared
        .checks
        .iter()
        .map(|check| check.commands.len() as u128)
        .sum::<u128>();
    let structured = prepared
        .checks
        .iter()
        .filter(|check| check.protocol == ResultProtocol::VibeHealthJsonV1)
        .count() as u128;
    let streams_per_command = u128::from(prepared.max_stdout_bytes)
        .checked_add(u128::from(prepared.max_stderr_bytes))
        .ok_or_else(|| HealthError::Preparation("health evidence size overflow".to_owned()))?;
    let retained = commands
        .checked_mul(2)
        .and_then(|value| value.checked_mul(streams_per_command))
        .and_then(|value| {
            structured
                .checked_mul(2)
                .and_then(|count| count.checked_mul(u128::from(prepared.max_result_bytes)))
                .and_then(|results| value.checked_add(results))
        })
        .ok_or_else(|| HealthError::Preparation("health evidence size overflow".to_owned()))?;
    let health_plan_bytes = super::snapshot_bytes(prepared)?.len() as u128;
    let tree_overhead = inventory.entries.iter().try_fold(0u128, |total, entry| {
        total
            .checked_add(2048)
            .and_then(|value| value.checked_add((entry.path.len() as u128).saturating_mul(12)))
            .ok_or_else(|| HealthError::Preparation("project evidence size overflow".to_owned()))
    })?;
    let worst_case = retained
        .checked_mul(JSON_EXPANSION)
        .and_then(|value| value.checked_add(health_plan_bytes.saturating_mul(4)))
        .and_then(|value| value.checked_add(tree_overhead))
        .and_then(|value| value.checked_add(FIXED_ENVELOPE))
        .ok_or_else(|| HealthError::Preparation("transaction evidence size overflow".to_owned()))?;
    let capacity = u128::from(
        crate::transaction::MAX_CANONICAL_REPORT_BYTES
            .min(crate::transaction::MAX_TRANSACTION_JOURNAL_BYTES) as u64,
    );
    Ok((worst_case > capacity).then(|| HealthBlocker {
        code: "health-evidence-store-capacity".to_owned(),
        check_id: None,
        message: format!(
            "declared health panel can require {worst_case} encoded bytes, exceeding the {capacity}-byte transaction/report capacity"
        ),
    }))
}

pub fn add_blockers(
    prepared: &mut PreparedHealth,
    blockers: impl IntoIterator<Item = HealthBlocker>,
) -> Result<(), HealthError> {
    prepared.blockers.extend(blockers);
    prepared.blockers.sort_by(|left, right| {
        (&left.code, &left.check_id, &left.message).cmp(&(
            &right.code,
            &right.check_id,
            &right.message,
        ))
    });
    prepared.blockers.dedup();
    prepared.plan_id = health_identity(prepared)?;
    Ok(())
}
