//! Restore retained operation inputs; repeated manual flags are checks, never new deltas.
use super::*;
use crate::{FeatureRequest, InstallChoice, Mark, RequiredBy};

pub(crate) fn restore(
    settings: &SettingsRoot,
    op: &w::ApplicationPendingOperation,
    option: &InstallerOptionDecl,
    incoming: &InstallRequest,
    canonical: bool,
) -> Result<InstallRequest, AppError> {
    let package = coordinate(&op.target.key)?;
    let refuse = |reason: String| failure(settings, &package, reason);
    if incoming
        .choice
        .installer
        .as_ref()
        .is_some_and(|v| v != &op.request.installer)
        || incoming
            .choice
            .platform
            .as_ref()
            .is_some_and(|v| v != &op.request.platform)
        || incoming
            .choice
            .arch
            .as_ref()
            .is_some_and(|v| v != &op.request.arch)
        || incoming
            .library
            .as_ref()
            .is_some_and(|v| Some(v) != op.request.library.as_ref())
        || (incoming.force && !op.request.force)
    {
        return Err(refuse(
            "the supplied operation flags differ from retained work; start fresh reconfiguration"
                .into(),
        ));
    }
    if let Some(submitted) = &incoming.configuration {
        if canonical {
            return Err(refuse(
                "app resume accepts ephemeral secrets, not configuration replacement".into(),
            ));
        }
        let saved = op
            .request
            .configuration
            .as_ref()
            .ok_or_else(|| refuse("retained legacy work has no common input".into()))?;
        if submitted.input != saved.evaluation.input
            || submitted
                .expected_definition
                .as_ref()
                .is_some_and(|v| v != &saved.evaluation.definition)
        {
            return Err(refuse(
                "the repeated common input differs from the retained operation".into(),
            ));
        }
    }
    let common = op
        .request
        .configuration
        .as_ref()
        .map(|r| &r.evaluation.input);
    let positive = common.map_or(op.request.enabled.as_slice(), |i| {
        i.intent.features.as_slice()
    });
    let negative = common.map_or(op.request.disabled.as_slice(), |i| {
        i.disabled_features.as_slice()
    });
    let profile = common
        .and_then(|i| i.intent.profile.as_ref())
        .or(op.request.profile.as_ref());
    if incoming
        .features
        .profile
        .as_ref()
        .is_some_and(|p| Some(p) != profile)
        || incoming.features.enable.iter().any(|id| {
            !positive.contains(id) && !(common.is_none() && op.request.features.contains(id))
        })
        || incoming
            .features
            .disable
            .iter()
            .any(|id| !negative.contains(id))
    {
        return Err(refuse(
            "the repeated feature/profile flags change retained input; start fresh reconfiguration"
                .into(),
        ));
    }
    let parameters = option
        .configuration
        .as_ref()
        .map_or(option.parameters.as_slice(), |d| d.parameters.as_slice());
    for id in op.request.parameters.keys() {
        if parameters.iter().any(|p| &p.id == id && p.secret) {
            return Err(refuse(format!(
                "formerly public parameter `{id}` is now secret; reconfigure it explicitly"
            )));
        }
    }
    let mut secrets = Vec::new();
    for (id, raw) in &incoming.parameters {
        let p = parameters
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| refuse(format!("unknown parameter `{id}`")))?;
        if p.secret {
            secrets.push((id.clone(), raw.clone()));
            continue;
        }
        if canonical {
            return Err(refuse(format!(
                "app resume only accepts ephemeral secret resupply; `{id}` is public"
            )));
        }
        let expected = if let Some(input) = common {
            input.intent.parameters.get(id).map(|v| match v {
                w::ConfigurationValue::Bool(v) => v.value.to_string(),
                w::ConfigurationValue::Integer(v) => v.value.clone(),
                w::ConfigurationValue::Text(v) => v.value.clone(),
            })
        } else {
            op.request
                .public_set
                .get(id)
                .or_else(|| op.request.parameters.get(id))
                .cloned()
        };
        let compatible = expected
            .as_deref()
            .and_then(|v| p.canonical(v).ok())
            .zip(p.canonical(raw).ok())
            .is_some_and(|(old, new)| old == new);
        if !compatible {
            return Err(refuse(format!(
                "public parameter `{id}` differs from retained input; start fresh reconfiguration"
            )));
        }
    }
    let start = if matches!(
        op.phase,
        w::ApplicationPendingOperationPhase::Prepared
            | w::ApplicationPendingOperationPhase::Dependencies
    ) {
        op.request
            .restart_from
            .clone()
            .map(Start::FromStep)
            .unwrap_or(Start::Resume)
    } else {
        Start::Resume
    };
    Ok(InstallRequest {
        transfer: None,
        configuration: None,
        choice: InstallChoice {
            platform: Some(op.request.platform.clone()),
            arch: Some(op.request.arch.clone()),
            installer: Some(op.request.installer.clone()),
        },
        library: op.request.library.clone(),
        force: op.request.force,
        features: FeatureRequest::default(),
        parameters: secrets,
        accept_license: op.request.accept_license,
        start,
        keep_work: op.request.keep_work || incoming.keep_work,
        hidden: op.request.hidden,
        mark: match op.request.mark {
            w::ApplicationPendingRequestMark::Manual => Mark::Manual,
            w::ApplicationPendingRequestMark::Auto => Mark::Auto,
        },
        purge_previous: op.request.purge_previous,
        required_by: op.request.required_by.as_ref().map(|r| RequiredBy {
            package: r.package.clone(),
            step: r.step.clone(),
        }),
    })
}
