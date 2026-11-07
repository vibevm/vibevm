//! Finite bindings chosen by the existing planner, before child execution starts.
use super::*;
use crate::PlannedDependency;
use vibe_core::manifest::RequirementScope;

pub(crate) fn resolve_binding(
    ctx: &super::super::OperationContext<'_>,
    index: &Index,
    parent: &w::ApplicationPendingOperation,
    b: &w::ApplicationPendingChild,
    scope: RequirementScope,
    step: Option<&str>,
) -> Result<PlannedDependency, AppError> {
    let package = coordinate(&b.key)?;
    let expected_scope = match b.scope {
        w::ApplicationPendingChildScope::Build => RequirementScope::Build,
        w::ApplicationPendingChildScope::Runtime => RequirementScope::Runtime,
    };
    if expected_scope != scope || b.step.as_deref() != step {
        return Err(failure(
            ctx.settings,
            &package,
            "retained dependency scope differs from the current plan".into(),
        ));
    }
    let (installed, resolved, commands) = if let Some(target) = &b.target {
        let state = load(ctx.settings, &package, Some(&target.operation))?;
        let op = state.operation.as_ref().ok_or_else(|| {
            failure(
                ctx.settings,
                &package,
                "dependency has no retained operation".into(),
            )
        })?;
        let complete = op.phase == w::ApplicationPendingOperationPhase::Complete;
        if complete {
            let record = index.installations.get(&b.key).ok_or_else(|| {
                failure(
                    ctx.settings,
                    &package,
                    "completed dependency is no longer installed".into(),
                )
            })?;
            if record.option != b.option {
                return Err(failure(
                    ctx.settings,
                    &package,
                    "completed dependency now uses another installer".into(),
                ));
            }
            if op.publication_record.is_none()
                || identity::predecessor(index, &b.key)
                    .map_err(|e| failure(ctx.settings, &package, e))?
                    != op.publication_record
            {
                return Err(failure(
                    ctx.settings,
                    &package,
                    "the completed dependency record changed".into(),
                ));
            }
            (
                true,
                None,
                record
                    .public_commands
                    .iter()
                    .map(|c| c.name.clone())
                    .collect(),
            )
        } else {
            let r = source(ctx, op)?;
            let commands = declared_commands(&r, &b.option, ctx.settings)?;
            (false, Some(Box::new(r)), commands)
        }
    } else if b.installed {
        let record = index.installations.get(&b.key).ok_or_else(|| {
            failure(
                ctx.settings,
                &package,
                "the retained installed dependency disappeared".into(),
            )
        })?;
        if record.pending.is_some()
            || identity::predecessor(index, &b.key)
                .map_err(|e| failure(ctx.settings, &package, e))?
                != b.record_identity
        {
            return Err(failure(
                ctx.settings,
                &package,
                "the retained installed dependency changed or has another pending operation".into(),
            ));
        }
        (
            true,
            None,
            record
                .public_commands
                .iter()
                .map(|c| c.name.clone())
                .collect(),
        )
    } else {
        if index.installations.contains_key(&b.key) {
            return Err(failure(ctx.settings,&package,"another operation installed the previously missing dependency; start fresh parent reconfiguration".into()));
        }
        let route = b.source.as_ref().ok_or_else(|| {
            failure(
                ctx.settings,
                &package,
                "retained dependency has no source route".into(),
            )
        })?;
        let mut r = read_source(ctx, route, &b.key)?;
        r.source_evidence = load(
            ctx.settings,
            &coordinate(&parent.target.key)?,
            Some(&parent.target.operation),
        )?
        .source_evidence
        .get(&b.key)
        .cloned();
        let commands = declared_commands(&r, &b.option, ctx.settings)?;
        (false, Some(Box::new(r)), commands)
    };
    Ok(PlannedDependency {
        package: b.requirement.clone(),
        scope,
        step: step.map(str::to_owned),
        key: b.key.clone(),
        installed,
        hidden: b.hidden,
        commands,
        option: Some(b.option.clone()),
        resolved,
    })
}
fn declared_commands(
    r: &ResolvedPackage,
    option: &str,
    settings: &SettingsRoot,
) -> Result<Vec<String>, AppError> {
    r.options
        .iter()
        .find(|o| o.id == option)
        .map(|o| o.public_commands.iter().map(|c| c.name.clone()).collect())
        .ok_or_else(|| {
            failure(
                settings,
                &r.package,
                "the retained dependency installer disappeared".into(),
            )
        })
}

pub(crate) fn link_child(
    settings: &SettingsRoot,
    parent: &w::ApplicationPendingTarget,
    child: &w::ApplicationPendingTarget,
    requirement: &str,
) -> Result<(), AppError> {
    let package = coordinate(&parent.key)?;
    let mut state = load(settings, &package, Some(&parent.operation))?;
    let op = state
        .operation
        .as_mut()
        .ok_or_else(|| failure(settings, &package, "missing parent operation".into()))?;
    if !op
        .children
        .iter()
        .any(|b| b.key == child.key && b.requirement == requirement)
    {
        return Err(failure(
            settings,
            &package,
            "child is not one of the retained dependency bindings".into(),
        ));
    }
    for b in op.children.iter_mut().filter(|b| b.key == child.key) {
        if b.target.as_ref().is_some_and(|old| old != child) {
            return Err(failure(
                settings,
                &package,
                "another child generation is already bound".into(),
            ));
        }
        b.target = Some(child.clone());
    }
    state.save(&work(settings, &package))
}

pub(crate) fn ack_child(
    settings: &SettingsRoot,
    parent: &w::ApplicationPendingTarget,
    key: &str,
) -> Result<(), AppError> {
    let package = coordinate(&parent.key)?;
    let child = coordinate(key)?;
    let mut state = load(settings, &package, Some(&parent.operation))?;
    let op = state
        .operation
        .as_mut()
        .ok_or_else(|| failure(settings, &package, "missing parent operation".into()))?;
    let target = op
        .children
        .iter()
        .find(|b| b.key == key)
        .and_then(|b| b.target.as_ref())
        .ok_or_else(|| {
            failure(
                settings,
                &package,
                "completed child lacks its generation binding".into(),
            )
        })?;
    let child_state = load(settings, &child, Some(&target.operation))?;
    if child_state
        .operation
        .as_ref()
        .is_none_or(|o| o.phase != w::ApplicationPendingOperationPhase::Complete)
    {
        return Err(failure(
            settings,
            &child,
            "child has not completed its operation".into(),
        ));
    }
    let index = Index::load(settings)?;
    let identity = identity::predecessor(&index, key)
        .map_err(|e| failure(settings, &child, e))?
        .ok_or_else(|| {
            failure(
                settings,
                &child,
                "completed child has no installed record".into(),
            )
        })?;
    if child_state
        .operation
        .as_ref()
        .and_then(|o| o.publication_record.as_ref())
        != Some(&identity)
    {
        return Err(failure(
            settings,
            &child,
            "the current record differs from the child's pre-publication completion identity"
                .into(),
        ));
    }
    for binding in op.children.iter_mut().filter(|b| b.key == key) {
        binding.completed = true;
        binding.record_identity = Some(identity.clone());
    }
    // Keep the child's completed work until this parent finishes; a later same-key
    // generation must not masquerade as the child acknowledged here.
    state.save(&work(settings, &package))
}

pub(crate) fn bound_child(
    ctx: &super::super::OperationContext<'_>,
    parent: &w::ApplicationPendingTarget,
    key: &str,
) -> Result<Option<w::ApplicationPendingTarget>, AppError> {
    let parent_package = coordinate(&parent.key)?;
    let owner = load(ctx.settings, &parent_package, Some(&parent.operation))?;
    let binding = owner
        .operation
        .as_ref()
        .and_then(|op| op.children.iter().find(|b| b.key == key))
        .ok_or_else(|| {
            failure(
                ctx.settings,
                &parent_package,
                "dependency is absent from the retained parent plan".into(),
            )
        })?;
    if let Some(target) = &binding.target {
        return Ok(Some(target.clone()));
    }
    if binding.installed {
        return Ok(None);
    }
    let child_package = coordinate(key)?;
    let Some(child) = StepsFile::load(&work(ctx.settings, &child_package))? else {
        return Ok(None);
    };
    let Some(operation) = child.operation.as_ref() else {
        return Err(failure(
            ctx.settings,
            &child_package,
            "unbound legacy child work cannot be adopted as this parent's generation".into(),
        ));
    };
    // An unowned terminal attempt is historical kept work, not a child of
    // this parent. Fresh dependency installation creates its own generation.
    if operation.phase == w::ApplicationPendingOperationPhase::Complete
        && operation.parent.is_none()
    {
        return Ok(None);
    }
    let owned = operation.parent.as_ref() == Some(&application_reference(parent))
        && operation.kind == w::ApplicationPendingOperationKind::Install
        && operation.request.installer == binding.option
        && binding.source.as_ref() == Some(&operation.source);
    if !owned {
        return Err(failure(
            ctx.settings,
            &child_package,
            format!(
                "dependency work belongs to another operation; continue `{}` separately",
                command(&operation.target)
            ),
        ));
    }
    load(
        ctx.settings,
        &child_package,
        Some(&operation.target.operation),
    )?;
    let admitted = source(ctx, operation)?;
    if admitted
        .options
        .iter()
        .find(|o| o.id == binding.option)
        .is_none_or(|o| {
            o.platform != operation.request.platform || o.arch != operation.request.arch
        })
    {
        return Err(failure(
            ctx.settings,
            &child_package,
            "saved child selection differs from its retained source".into(),
        ));
    }
    // The child saved its exact backward parent identity before the forward link.
    // Reconnecting that known generation repairs the interrupted link, not a guess.
    link_child(
        ctx.settings,
        parent,
        &operation.target,
        &binding.requirement,
    )?;
    Ok(Some(operation.target.clone()))
}
