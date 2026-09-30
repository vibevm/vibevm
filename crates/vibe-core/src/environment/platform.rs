//! Role-labelled platform facts; absence never silently becomes the executor.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ABI-ROLE-SEPARATION");
use super::Reference;
use crate::manifest::TargetOs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact<T> {
    Known(T),
    Unknown,
    NotApplicable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requirement<T> {
    Any,
    Exact(T),
    NotApplicable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformField {
    Os,
    Architecture,
    Abi,
    Libc,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformFacts {
    os: Fact<TargetOs>,
    architecture: Fact<Reference>,
    abi: Fact<Reference>,
    libc: Fact<Reference>,
}
impl PlatformFacts {
    pub fn new(
        os: Fact<TargetOs>,
        architecture: Fact<Reference>,
        abi: Fact<Reference>,
        libc: Fact<Reference>,
    ) -> Self {
        Self {
            os,
            architecture,
            abi,
            libc,
        }
    }
    pub fn os(&self) -> &Fact<TargetOs> {
        &self.os
    }
    pub fn architecture(&self) -> &Fact<Reference> {
        &self.architecture
    }
    pub fn abi(&self) -> &Fact<Reference> {
        &self.abi
    }
    pub fn libc(&self) -> &Fact<Reference> {
        &self.libc
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformConstraints {
    os: Requirement<TargetOs>,
    architecture: Requirement<Reference>,
    abi: Requirement<Reference>,
    libc: Requirement<Reference>,
}
impl PlatformConstraints {
    pub fn new(
        os: Requirement<TargetOs>,
        architecture: Requirement<Reference>,
        abi: Requirement<Reference>,
        libc: Requirement<Reference>,
    ) -> Self {
        Self {
            os,
            architecture,
            abi,
            libc,
        }
    }
    pub fn os_requirement(&self) -> &Requirement<TargetOs> {
        &self.os
    }
    pub fn architecture_requirement(&self) -> &Requirement<Reference> {
        &self.architecture
    }
    pub fn abi_requirement(&self) -> &Requirement<Reference> {
        &self.abi
    }
    pub fn libc_requirement(&self) -> &Requirement<Reference> {
        &self.libc
    }

    /// Satisfied means THESE constraints matched, not ABI/execution authority.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#COMPATIBILITY-FACTS",
        r = 1
    )]
    pub fn compare(&self, facts: &PlatformFacts) -> PlatformCompatibility {
        let mut missing = Vec::new();
        let mut unsatisfied = Vec::new();
        compare(
            &self.os,
            &facts.os,
            PlatformField::Os,
            &mut missing,
            &mut unsatisfied,
        );
        compare(
            &self.architecture,
            &facts.architecture,
            PlatformField::Architecture,
            &mut missing,
            &mut unsatisfied,
        );
        compare(
            &self.abi,
            &facts.abi,
            PlatformField::Abi,
            &mut missing,
            &mut unsatisfied,
        );
        compare(
            &self.libc,
            &facts.libc,
            PlatformField::Libc,
            &mut missing,
            &mut unsatisfied,
        );
        PlatformCompatibility {
            missing,
            unsatisfied,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformCompatibility {
    pub missing: Vec<PlatformField>,
    pub unsatisfied: Vec<PlatformField>,
}
impl PlatformCompatibility {
    pub fn is_satisfied(&self) -> bool {
        self.missing.is_empty() && self.unsatisfied.is_empty()
    }
}
fn compare<T: PartialEq>(
    need: &Requirement<T>,
    fact: &Fact<T>,
    field: PlatformField,
    missing: &mut Vec<PlatformField>,
    unsatisfied: &mut Vec<PlatformField>,
) {
    match (need, fact) {
        (Requirement::Any, _) => {}
        (_, Fact::Unknown) => missing.push(field),
        (Requirement::Exact(a), Fact::Known(b)) if a == b => {}
        (Requirement::NotApplicable, Fact::NotApplicable) => {}
        _ => unsatisfied.push(field),
    }
}

/// All three roles are supplied independently. None means that compiler-output
/// role is not applicable; Some with Unknown facts records an unresolved role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainPlatforms {
    build_executor: PlatformFacts,
    produced_tool_runtime: PlatformFacts,
    compiler_output: Option<PlatformFacts>,
}
impl ToolchainPlatforms {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ABI-ROLE-SEPARATION",
        r = 1
    )]
    pub fn new(
        build_executor: PlatformFacts,
        produced_tool_runtime: PlatformFacts,
        compiler_output: Option<PlatformFacts>,
    ) -> Self {
        Self {
            build_executor,
            produced_tool_runtime,
            compiler_output,
        }
    }
    pub fn build_executor(&self) -> &PlatformFacts {
        &self.build_executor
    }
    pub fn produced_tool_runtime(&self) -> &PlatformFacts {
        &self.produced_tool_runtime
    }
    pub fn compiler_output(&self) -> Option<&PlatformFacts> {
        self.compiler_output.as_ref()
    }
}
