//! Read-only full import decisions use the same exact binding and ordinary planner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT");
use super::PreparedFullImport;
use crate::{AppError, ImportScope, ImportSecrets, Index, InstallRequest, OperationContext};

/// A dry-run row is a proposed install, a deliberate skip, or a concrete refusal.
#[derive(Debug)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
pub enum FullImportPreview {
    Install { key: String, installer: String },
    Skip { key: String, reason: String },
    Refused { key: String, error: AppError },
}
/// Preview prepared roots and all consumed dependency plans without allocating an operation.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
pub fn preview_full_import(
    ctx: &OperationContext<'_>,
    prepared: &PreparedFullImport,
    secrets: &ImportSecrets,
) -> Result<Vec<FullImportPreview>, AppError> {
    let scope = ImportScope::preview(
        prepared.state.roots.clone(),
        prepared.state.bindings.clone(),
        prepared.state.secret_requirements.clone(),
        secrets,
    )?;
    let scoped = OperationContext {
        import_scope: Some(&scope),
        parameter_prompt: None,
        license_prompt: None,
        chooser: None,
        ..*ctx
    };
    let index = Index::load(ctx.settings)?;
    let mut rows = Vec::new();
    for key in &prepared.state.roots {
        let row = (|| {
            let binding = scope
                .binding(key)
                .ok_or_else(|| super::document::invalid("preview", "missing root binding"))?;
            let (resolved, option) = scope.admit(&scoped, binding)?;
            if let Some(installed) = index.installations.get(key) {
                match scope.compatible(&scoped, binding, &option, installed) {
                    Ok(()) => {
                        return Ok(FullImportPreview::Skip {
                            key: key.clone(),
                            reason: "existing installation already matches full configuration"
                                .into(),
                        });
                    }
                    Err(_) if prepared.state.skip_installed => {
                        return Ok(FullImportPreview::Skip {
                            key: key.clone(),
                            reason: "kept installed choice; imported configuration was not applied"
                                .into(),
                        });
                    }
                    Err(error) => return Err(error),
                }
            }
            let request =
                scope.request(&scoped, binding, &option, prepared.state.accept_license)?;
            plan_tree(&scoped, &resolved, &request, &mut Vec::new())?;
            Ok(FullImportPreview::Install {
                key: key.clone(),
                installer: option.id,
            })
        })();
        rows.push(row.unwrap_or_else(|error| FullImportPreview::Refused {
            key: key.clone(),
            error,
        }));
    }
    Ok(rows)
}
fn plan_tree(
    ctx: &OperationContext<'_>,
    resolved: &crate::ResolvedPackage,
    request: &InstallRequest,
    stack: &mut Vec<String>,
) -> Result<(), AppError> {
    let key = resolved.package.key();
    if stack.contains(&key) {
        return Err(super::document::invalid(
            "preview",
            "application requirement cycle",
        ));
    }
    stack.push(key);
    let plan = crate::plan(ctx, resolved, request)?;
    for dependency in &plan.dependencies {
        if dependency.installed {
            continue;
        }
        let source = dependency.resolved.as_deref().ok_or_else(|| {
            super::document::invalid("preview", "uninstalled dependency has no resolved source")
        })?;
        let request = if let Some(scope) = ctx.import_scope
            && let Some(binding) = scope.binding(&dependency.key)
        {
            let (_, option) = scope.admit(ctx, binding)?;
            scope.request(ctx, binding, &option, request.accept_license)?
        } else {
            InstallRequest {
                choice: crate::InstallChoice {
                    installer: dependency.option.clone(),
                    ..Default::default()
                },
                accept_license: request.accept_license,
                hidden: dependency.hidden,
                mark: crate::Mark::Auto,
                ..Default::default()
            }
        };
        plan_tree(ctx, source, &request, stack)?;
    }
    stack.pop();
    Ok(())
}
