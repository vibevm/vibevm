//! Bounded prior-state handle and engine-owned rollback file.

use crate::mechanism::DeployTargetRequest;
use crate::mechanism::contain::relative_to;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::vibebin::store;

use super::{Destination, VibeOptLauncherProvider, resource_digest};

const HANDLE_PREFIX: &str = "vibe-opt-launcher-prior/1";
const PRIOR_FILE: &str = "vibe-opt-launcher-prior";
const HANDLE_CAP: usize = 256;

pub(super) struct PriorHandle {
    pub(super) path: String,
    pub(super) sha256: String,
    pub(super) bytes: u64,
    pub(super) unix_mode: Option<u32>,
}

impl VibeOptLauncherProvider {
    pub(super) fn save_prior(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
        expected_resource_digest: &str,
    ) -> Result<String, DeployProviderError> {
        let backup = backup_relative(request)?;
        let source = store::resource_state(
            &request.target.id,
            request.settings_root,
            &destination.relative,
        )?
        .ok_or_else(|| DeployProviderError::Observe {
            target: request.target.id.clone(),
            resource: destination.resource.clone(),
            reason: "the receipt-owned launcher vanished before its rollback state was retained"
                .to_owned(),
        })?;
        if resource_digest(&source) != expected_resource_digest {
            return Err(DeployProviderError::OccupantDrifted {
                target: request.target.id.clone(),
                resource: destination.resource.clone(),
                recorded: expected_resource_digest.to_owned(),
                observed: resource_digest(&source),
            });
        }
        let copied = store::copy_resource_expected(
            &request.target.id,
            request.settings_root,
            &destination.relative,
            request.settings_root,
            &backup,
            source.unix_mode,
            &source.sha256,
            source.bytes,
        )?;
        if copied != source || resource_digest(&copied) != expected_resource_digest {
            return Err(DeployProviderError::Write {
                target: request.target.id.clone(),
                path: backup,
                reason: "the retained rollback state differs from the held prior launcher"
                    .to_owned(),
            });
        }
        Ok(render_handle(&PriorHandle {
            path: backup,
            sha256: source.sha256,
            bytes: source.bytes,
            unix_mode: source.unix_mode,
        }))
    }

    pub(super) fn load_prior(
        &self,
        request: &DeployTargetRequest<'_>,
        encoded: &str,
    ) -> Result<(PriorHandle, String), DeployProviderError> {
        let handle = parse_handle(&request.target.id, encoded)?;
        if request.staging.is_some() && handle.path != backup_relative(request)? {
            return Err(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: handle.path,
                reason: "the rollback handle is not bound to this deployment's staging directory"
                    .to_owned(),
            });
        }
        let observed =
            store::resource_state(&request.target.id, request.settings_root, &handle.path)?
                .ok_or_else(|| DeployProviderError::Observe {
                    target: request.target.id.clone(),
                    resource: handle.path.clone(),
                    reason: "the rollback handle's engine-owned file is absent".to_owned(),
                })?;
        if observed.sha256 != handle.sha256
            || observed.bytes != handle.bytes
            || observed.unix_mode != handle.unix_mode
        {
            return Err(DeployProviderError::Observe {
                target: request.target.id.clone(),
                resource: handle.path.clone(),
                reason: "the rollback handle does not describe the retained bytes and mode"
                    .to_owned(),
            });
        }
        let path = handle.path.clone();
        Ok((handle, path))
    }

    pub(super) fn restore_prior(
        &self,
        request: &DeployTargetRequest<'_>,
        destination: &Destination,
        handle: &PriorHandle,
        backup: &str,
    ) -> Result<vibe_safefs::StableFileState, DeployProviderError> {
        store::copy_resource_expected(
            &request.target.id,
            request.settings_root,
            backup,
            request.settings_root,
            &destination.relative,
            handle.unix_mode,
            &handle.sha256,
            handle.bytes,
        )
    }
}

pub(super) fn backup_relative(
    request: &DeployTargetRequest<'_>,
) -> Result<String, DeployProviderError> {
    let staging = request
        .staging
        .ok_or_else(|| DeployProviderError::Staging {
            target: request.target.id.clone(),
            path: PRIOR_FILE.to_owned(),
        })?;
    let base = relative_to(staging, request.settings_root).ok_or_else(|| {
        DeployProviderError::Staging {
            target: request.target.id.clone(),
            path: PRIOR_FILE.to_owned(),
        }
    })?;
    Ok(format!("{base}/{PRIOR_FILE}"))
}

pub(super) fn render_handle(handle: &PriorHandle) -> String {
    format!(
        "{HANDLE_PREFIX}|path={}|sha256={}|bytes={}|mode={}",
        handle.path,
        handle.sha256,
        handle.bytes,
        handle
            .unix_mode
            .map_or_else(|| "none".to_owned(), |mode| format!("{mode:04o}")),
    )
}

fn parse_handle(target: &str, encoded: &str) -> Result<PriorHandle, DeployProviderError> {
    let refuse = |reason: String| DeployProviderError::Write {
        target: target.to_owned(),
        path: "prior_state_handle".to_owned(),
        reason,
    };
    if encoded.len() > HANDLE_CAP {
        return Err(refuse(format!(
            "the rollback handle exceeds its {HANDLE_CAP}-byte bound"
        )));
    }
    let mut parts = encoded.split('|');
    if parts.next() != Some(HANDLE_PREFIX) {
        return Err(refuse(
            "the rollback handle has the wrong kind or epoch".to_owned(),
        ));
    }
    let path = parts
        .next()
        .and_then(|part| part.strip_prefix("path="))
        .filter(|path| valid_backup_path(path))
        .ok_or_else(|| refuse("the rollback handle has no valid staging path".to_owned()))?
        .to_owned();
    let sha256 = parts
        .next()
        .and_then(|part| part.strip_prefix("sha256="))
        .filter(|value| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
        .ok_or_else(|| refuse("the rollback handle has no lowercase SHA-256".to_owned()))?
        .to_owned();
    let bytes = parts
        .next()
        .and_then(|part| part.strip_prefix("bytes="))
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| refuse("the rollback handle has no canonical byte length".to_owned()))?;
    let unix_mode_text = parts
        .next()
        .and_then(|part| part.strip_prefix("mode="))
        .ok_or_else(|| refuse("the rollback handle has no mode".to_owned()))?;
    if parts.next().is_some() {
        return Err(refuse(
            "the rollback handle has trailing members".to_owned(),
        ));
    }
    let unix_mode = if unix_mode_text == "none" {
        None
    } else if unix_mode_text.len() == 4
        && unix_mode_text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'7'))
    {
        u32::from_str_radix(unix_mode_text, 8).ok()
    } else {
        return Err(refuse(
            "the rollback handle mode is not `none` or four octal digits".to_owned(),
        ));
    };
    Ok(PriorHandle {
        path,
        sha256,
        bytes,
        unix_mode,
    })
}

fn valid_backup_path(path: &str) -> bool {
    let Ok((parents, name)) = vibe_safefs::split_relative(path) else {
        return false;
    };
    parents.len() == 4
        && parents[0] == "state"
        && parents[1] == "deployments"
        && parents[2].len() == 64
        && parents[2].bytes().all(|byte| byte.is_ascii_hexdigit())
        && parents[3] == "staging"
        && name == PRIOR_FILE
}
