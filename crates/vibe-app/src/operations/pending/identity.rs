//! Typed public execution continuity; recipe bytes and secret equality are not authenticated.
use super::failure;
use crate::{AppError, Plan, ResolvedPackage, SettingsRoot};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use vibe_core::manifest::{InstallerOptionDecl, ResultDecl, StepDecl};

#[derive(Serialize)]
struct Step<'a> {
    declaration: &'a StepDecl,
    run: bool,
    prompt: Option<&'a str>,
}
#[derive(Serialize)]
struct Dependency<'a> {
    key: &'a str,
    scope: &'a str,
    step: Option<&'a str>,
    commands: &'a [String],
}
#[derive(Serialize)]
struct Context<'a> {
    home: String,
    source: String,
    package: String,
    application: &'a vibe_core::manifest::ApplicationDecl,
    option: &'a InstallerOptionDecl,
    result: &'a ResultDecl,
    features: &'a [String],
    parameters: BTreeMap<&'a str, &'a str>,
    modes: BTreeMap<&'a str, &'static str>,
    force: bool,
    kind: &'a str,
    steps: Vec<Step<'a>>,
    dependencies: Vec<Dependency<'a>>,
}
pub(super) fn execution(
    settings: &SettingsRoot,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    plan: &Plan,
    force: bool,
    kind: &str,
) -> Result<String, AppError> {
    let home = super::actual_home(settings)?;
    let source = super::lossless(
        &resolved
            .root
            .canonicalize()
            .map_err(|e| failure(settings, &resolved.package, e.to_string()))?,
    )?;
    let value = Context {
        home,
        source,
        package: resolved.package.key(),
        application: &resolved.application,
        option,
        result: &plan.result,
        features: &plan.features,
        parameters: plan
            .parameters
            .iter()
            .filter(|p| !p.secret)
            .map(|p| (p.id.as_str(), p.value.as_str()))
            .collect(),
        modes: plan
            .steps
            .iter()
            .filter_map(|s| s.agent_mode.map(|m| (s.step.id.as_str(), m.as_str())))
            .collect(),
        force,
        kind,
        steps: plan
            .steps
            .iter()
            .map(|p| Step {
                declaration: &p.step,
                run: p.run,
                prompt: p.prepared_prompt.as_deref(),
            })
            .collect(),
        dependencies: plan
            .dependencies
            .iter()
            .map(|d| Dependency {
                key: &d.key,
                scope: d.scope.as_str(),
                step: d.step.as_deref(),
                commands: &d.commands,
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&value)
        .map_err(|e| failure(settings, &resolved.package, e.to_string()))?;
    let mut hash = Sha256::new();
    hash.update(b"vibevm.app.execution-context/v1\0");
    hash.update(bytes);
    Ok(format!(
        "app-execution-context/1:sha256:{:x}",
        hash.finalize()
    ))
}

pub(super) fn predecessor(index: &crate::Index, key: &str) -> Result<Option<String>, String> {
    index
        .identity_record(key)
        .map(|record| record_digest(&record))
        .transpose()
}
pub(in crate::operations) fn published(record: &crate::Installation) -> Result<String, String> {
    record_digest(&record.identity_record())
}
fn record_digest(
    record: &vibe_wire::generated::application::e3::index::Installation,
) -> Result<String, String> {
    let bytes = serde_json::to_vec(record).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    hash.update(b"vibevm.app.predecessor/v1\0");
    hash.update(bytes);
    Ok(format!("app-predecessor/1:sha256:{:x}", hash.finalize()))
}
