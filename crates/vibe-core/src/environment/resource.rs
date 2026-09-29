//! Logical root-relative identities; reference equality is never root admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-RESOURCE-IDENTITY");
use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceIdentity {
    instance: TargetInstanceId,
    role: RootRole,
    root_binding: Reference,
    root_producer: Reference,
    principal: Reference,
    policy: TargetPathPolicy,
    comparison: CaseComparison,
    path: TargetRelativePath,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceComparison {
    SameDeclaredIdentity,
    DifferentDeclaredIdentity,
    Unresolved,
}
impl ResourceIdentity {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-RESOURCE-IDENTITY",
        r = 1
    )]
    pub fn new(
        instance: TargetInstanceId,
        root: &DeclaredRootBinding,
        principal: Reference,
        path: TargetRelativePath,
        comparison: CaseComparison,
    ) -> Result<Self, EnvironmentError> {
        if &instance != root.instance()
            || &principal != root.principal()
            || path.policy() != root.path().policy()
        {
            return Err(invalid(
                "resource.binding",
                "instance, principal or path policy differs from declared root",
            ));
        }
        if path.components().is_empty() {
            return Err(invalid(
                "resource.path",
                "resource must name a relative component sequence",
            ));
        }
        Ok(Self {
            instance,
            role: root.role(),
            root_binding: root.references().binding.clone(),
            root_producer: root.references().producer.clone(),
            principal,
            policy: path.policy(),
            comparison,
            path,
        })
    }
    pub fn path(&self) -> &TargetRelativePath {
        &self.path
    }
    pub fn role(&self) -> RootRole {
        self.role
    }
    pub fn instance(&self) -> &TargetInstanceId {
        &self.instance
    }
    pub fn principal(&self) -> &Reference {
        &self.principal
    }
    pub fn comparison(&self) -> CaseComparison {
        self.comparison
    }
    /// The chosen lexical model can collapse ASCII spellings. The original
    /// bytes remain intact and this result cannot authorize any filesystem use.
    pub fn lexical_compare(&self, other: &Self) -> ResourceComparison {
        if self.instance != other.instance
            || self.role != other.role
            || self.root_binding != other.root_binding
            || self.root_producer != other.root_producer
            || self.principal != other.principal
            || self.policy != other.policy
            || self.comparison != other.comparison
        {
            return ResourceComparison::DifferentDeclaredIdentity;
        }
        match (
            self.path.key(self.comparison),
            other.path.key(self.comparison),
        ) {
            (Some(a), Some(b)) if a == b => ResourceComparison::SameDeclaredIdentity,
            (Some(_), Some(_)) => ResourceComparison::DifferentDeclaredIdentity,
            _ => ResourceComparison::Unresolved,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedResourceKind {
    Account,
    Service,
    Mount,
    Boot,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedOperationSupport {
    UnsupportedOperation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedEnvironmentResource {
    kind: ReservedResourceKind,
    name: Reference,
}
impl ReservedEnvironmentResource {
    pub fn new(kind: ReservedResourceKind, name: Reference) -> Self {
        Self { kind, name }
    }
    pub fn kind(&self) -> ReservedResourceKind {
        self.kind
    }
    pub fn name(&self) -> &Reference {
        &self.name
    }
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#RESERVED-ENVIRONMENT-RESOURCES",
        r = 1
    )]
    pub fn operation_support(&self) -> ReservedOperationSupport {
        ReservedOperationSupport::UnsupportedOperation
    }
}
