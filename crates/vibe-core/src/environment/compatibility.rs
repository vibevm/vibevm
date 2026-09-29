//! Explicit local compatibility; unsupported target operations never execute.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#COMPATIBILITY-FACTS");
use super::*;
use crate::manifest::{TargetApplicability, TargetOs, TargetWhen};
use std::path::{Path, PathBuf};

/// Supplied absolute LOCAL root and role references. This is a consistency
/// mapping, not an existing-directory observation or admitted capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWorkspaceMapping {
    execution: ExecutionId,
    target: TargetInstanceId,
    root: PathBuf,
}
impl LocalWorkspaceMapping {
    pub fn new(
        execution: ExecutionId,
        target: TargetInstanceId,
        root: PathBuf,
    ) -> Result<Self, EnvironmentError> {
        if !root.is_absolute() || root.to_str().is_none() {
            return Err(invalid(
                "local_mapping.root",
                "local mapping requires an injected absolute UTF8 root",
            ));
        }
        Ok(Self {
            execution,
            target,
            root,
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationCompatibility {
    SupportedLocalWorkspaceSourceProjection,
    UnsupportedTargetMode(TargetMode),
    BindingMismatch(&'static str),
    PlatformMismatch(PlatformCompatibility),
}
/// Source-materialization compatibility only. This result never admits a
/// payload launcher, native extension, filesystem write or provider operation.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#LOCAL-WORKSPACE-COMPATIBILITY",
    r = 1
)]
pub fn workspace_source_projection_compatibility(
    definition: &EnvironmentDefinition,
    execution: &ExecutionContext,
    target: &TargetBinding,
    mapping: &LocalWorkspaceMapping,
) -> OperationCompatibility {
    if target.facts().mode != TargetMode::WorkspaceProjection {
        return OperationCompatibility::UnsupportedTargetMode(target.facts().mode);
    }
    if &target.facts().definition != definition.id() {
        return OperationCompatibility::BindingMismatch("definition");
    }
    if &mapping.execution != execution.id() || &mapping.target != target.instance() {
        return OperationCompatibility::BindingMismatch("execution/instance");
    }
    if mapping.root.to_str() != Some(target.facts().roots.filesystem_root()) {
        return OperationCompatibility::BindingMismatch("local root");
    }
    let mut facts = definition
        .composition()
        .target_constraints
        .compare(&target.facts().platform);
    let e = &execution.facts().platform;
    let t = &target.facts().platform;
    same_local(e.os(), t.os(), PlatformField::Os, &mut facts);
    same_local(
        e.architecture(),
        t.architecture(),
        PlatformField::Architecture,
        &mut facts,
    );
    // Bare source projection makes no ABI claim. Only declared requirements
    // require matching executor/target ABI/libc facts; native/payload admission
    // remains a separate consumer with its own required facts.
    let constraints = &definition.composition().target_constraints;
    if !matches!(constraints.abi_requirement(), Requirement::Any) {
        same_local(e.abi(), t.abi(), PlatformField::Abi, &mut facts);
    }
    if !matches!(constraints.libc_requirement(), Requirement::Any) {
        same_local(e.libc(), t.libc(), PlatformField::Libc, &mut facts);
    }
    if matches!(e.os(), Fact::NotApplicable) && !facts.unsatisfied.contains(&PlatformField::Os) {
        facts.unsatisfied.push(PlatformField::Os);
    }
    if matches!(e.architecture(), Fact::NotApplicable)
        && !facts.unsatisfied.contains(&PlatformField::Architecture)
    {
        facts.unsatisfied.push(PlatformField::Architecture);
    }
    facts.missing.dedup();
    facts.unsatisfied.dedup();
    if facts.is_satisfied() {
        OperationCompatibility::SupportedLocalWorkspaceSourceProjection
    } else {
        OperationCompatibility::PlatformMismatch(facts)
    }
}
fn same_local<T: PartialEq>(
    executor: &Fact<T>,
    target: &Fact<T>,
    field: PlatformField,
    result: &mut PlatformCompatibility,
) {
    match (executor, target) {
        (Fact::Unknown, _) | (_, Fact::Unknown) => {
            if !result.missing.contains(&field) {
                result.missing.push(field);
            }
        }
        (Fact::Known(a), Fact::Known(b)) if a == b => {}
        (Fact::NotApplicable, Fact::NotApplicable) => {}
        _ => {
            if !result.unsatisfied.contains(&field) {
                result.unsatisfied.push(field);
            }
        }
    }
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#LEGACY-HOST-INTERPRETATION",
    r = 1
)]
pub fn legacy_host_applicability(
    name: impl Into<String>,
    guard: Option<&TargetWhen>,
    execution: &ExecutionContext,
) -> Result<TargetApplicability, EnvironmentError> {
    let Fact::Known(os) = execution.facts().platform.os() else {
        return Err(invalid(
            "execution.os",
            "legacy host applicability requires known executor OS",
        ));
    };
    Ok(TargetApplicability::decide(name, guard, *os))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherFamily {
    WindowsCommand,
    PosixShell,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherDescription {
    Family(LauncherFamily),
    MissingTargetOs,
    NotApplicable,
}
/// Descriptive target family only; no executable discovery, ABI admission or run.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ABI-ROLE-SEPARATION",
    r = 1
)]
pub fn payload_launcher_family(target: &TargetBinding) -> LauncherDescription {
    match target.facts().platform.os() {
        Fact::Known(TargetOs::Windows) => {
            LauncherDescription::Family(LauncherFamily::WindowsCommand)
        }
        Fact::Known(TargetOs::Linux | TargetOs::Macos) => {
            LauncherDescription::Family(LauncherFamily::PosixShell)
        }
        Fact::Unknown => LauncherDescription::MissingTargetOs,
        Fact::NotApplicable => LauncherDescription::NotApplicable,
    }
}
/// Describes the ABI of the executor which LOADS a native extension. It does
/// not broaden the existing lifecycle owner's closed native admission list.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ABI-ROLE-SEPARATION",
    r = 1
)]
pub fn native_extension_platform(execution: &ExecutionContext) -> &PlatformFacts {
    &execution.facts().platform
}
