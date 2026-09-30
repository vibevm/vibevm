//! Semantic binding of an exact-lock wire projection to typed, supplied core
//! values. This checks consistency; the source owner must authenticate the
//! completeness and origin of captured bytes before calling it.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY");

use std::collections::BTreeMap;
use std::fmt;

use sha2::{Digest, Sha256};
use vibe_core::environment::{
    DependencyRole as CoreRole, EnvironmentDefinition, EnvironmentLock, ExactLockDigest,
    ExternalCapture, Fact, LockTargetRole, PackageCapture, SelectedLockTarget, exact_lock_digest,
    exact_shippable_tree_digest,
};
use vibe_core::manifest::TargetOs;

use crate::generated::environment::e1::exact_lock::{DependencyRole, ExactLock, TargetRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentWireError {
    pub field: &'static str,
    pub reason: &'static str,
}
impl fmt::Display for EnvironmentWireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "environment wire field {}: {}", self.field, self.reason)
    }
}
impl std::error::Error for EnvironmentWireError {}
fn invalid(field: &'static str, reason: &'static str) -> EnvironmentWireError {
    EnvironmentWireError { field, reason }
}
fn digest64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn role(value: CoreRole) -> DependencyRole {
    match value {
        CoreRole::Build => DependencyRole::Build,
        CoreRole::Runtime => DependencyRole::Runtime,
        CoreRole::BuildAndRuntime => DependencyRole::BuildAndRuntime,
    }
}
fn target_os(value: TargetOs) -> &'static str {
    match value {
        TargetOs::Windows => "windows",
        TargetOs::Linux => "linux",
        TargetOs::Macos => "macos",
    }
}
fn fact_text(value: &Fact<vibe_core::environment::Reference>) -> Option<Option<&str>> {
    match value {
        Fact::Known(value) => Some(Some(value.as_str())),
        Fact::NotApplicable => Some(None),
        Fact::Unknown => None,
    }
}
impl ExactLock {
    /// Recompute the core recipe from the typed definition/lock and supplied
    /// complete captures, then verify every representable wire coordinate.
    /// Returned digest is an equality result, not source authentication.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY",
        r = 2
    )]
    pub fn validate_environment_exact_lock(
        &self,
        definition: &EnvironmentDefinition,
        lock: &EnvironmentLock,
        selected_target: &SelectedLockTarget<'_>,
        packages: &[PackageCapture<'_>],
        external: &[ExternalCapture<'_>],
    ) -> Result<ExactLockDigest, EnvironmentWireError> {
        if self.schema != 1 || self.recipe_epoch != 1 {
            return Err(invalid(
                "schema",
                "unsupported exact-lock schema or recipe epoch",
            ));
        }
        if !digest64(&self.lock_digest_sha256) {
            return Err(invalid(
                "lock_digest_sha256",
                "expected lowercase SHA-256 hex",
            ));
        }
        if self.lock_owner != lock.id().producer().as_str()
            || self.definition_ref != definition.id().reference().as_str()
        {
            return Err(invalid(
                "lock_owner",
                "wire owner or definition differs from typed lock",
            ));
        }
        let target_role = match selected_target.role() {
            LockTargetRole::Executor => TargetRole::Executor,
            LockTargetRole::ProducedToolRuntime => TargetRole::ProducedToolRuntime,
            LockTargetRole::EmittedCodeTarget => TargetRole::EmittedCodeTarget,
        };
        if self.target.role != target_role {
            return Err(invalid(
                "target.role",
                "wire role differs from resolver-selected target role",
            ));
        }
        let platform = selected_target.platform();
        if !matches!(platform.os(), Fact::Known(os) if self.target.os == target_os(*os))
            || !matches!(platform.architecture(), Fact::Known(arch) if self.target.architecture == arch.as_str())
            || fact_text(platform.abi()) != Some(self.target.abi.as_deref())
            || fact_text(platform.libc()) != Some(self.target.libc.as_deref())
        {
            return Err(invalid(
                "target",
                "wire target differs from selected typed role facts",
            ));
        }
        let capture_map: BTreeMap<_, _> =
            packages.iter().map(|p| (p.capture.as_str(), p)).collect();
        if capture_map.len() != packages.len()
            || self.selected_captures.len() != lock.closure().packages.len()
        {
            return Err(invalid(
                "selected_captures",
                "captured selection cardinality differs",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for selected in &self.selected_captures {
            if !seen.insert(selected.source_capture_ref.as_str()) {
                return Err(invalid("selected_captures", "duplicate selected capture"));
            }
            let facts = lock
                .closure()
                .packages
                .iter()
                .find(|p| p.facts().capture.as_str() == selected.source_capture_ref)
                .ok_or_else(|| invalid("selected_captures", "wire package is not in typed lock"))?
                .facts();
            let capture = capture_map
                .get(selected.source_capture_ref.as_str())
                .ok_or_else(|| invalid("selected_captures", "selected capture has no bytes"))?;
            let observed_content = exact_shippable_tree_digest(&capture.files)
                .map_err(|_| invalid("selected_captures", "captured tree recipe refused"))?;
            let observed_tree = observed_content
                .strip_prefix("sha256-tree/1:")
                .ok_or_else(|| invalid("selected_captures", "unexpected tree recipe identity"))?;
            let configuration_bytes = capture.configuration_bytes.ok_or_else(|| {
                invalid(
                    "selected_captures",
                    "selected configuration bytes are unavailable",
                )
            })?;
            let configuration_digest = format!("{:x}", Sha256::digest(configuration_bytes));
            if facts.content.as_str() != observed_content
                || selected.content_hash_recipe != "sha256-tree/1"
                || selected.content_hash_value != observed_tree
                || selected.capture_digest_sha256 != observed_tree
                || selected.capture_owner != capture.owner.as_str()
                || selected.selected_root.as_deref() != facts.root.as_ref().map(|r| r.as_str())
                || selected.package_kind != facts.package.kind.map(|v| v.as_str()).unwrap_or("")
                || selected.package_group
                    != facts
                        .package
                        .group
                        .as_ref()
                        .map(|v| v.as_str())
                        .unwrap_or("")
                || selected.package_name != facts.package.name.as_str()
                || selected.exact_version != facts.version.to_string()
                || selected.configuration_ref != facts.configuration.as_str()
                || selected.configuration_owner != capture.configuration_owner.as_str()
                || selected.configuration_digest_sha256 != configuration_digest
                || selected.role != role(facts.role)
            {
                return Err(invalid(
                    "selected_captures",
                    "wire package coordinates or captured digest differ",
                ));
            }
        }
        let external_map: BTreeMap<_, _> = external
            .iter()
            .map(|e| ((e.backend.as_str(), e.closure.as_str()), e))
            .collect();
        if external_map.len() != external.len()
            || self.dependency_closure.len() != lock.closure().external_backends.len()
        {
            return Err(invalid(
                "dependency_closure",
                "external closure cardinality differs",
            ));
        }
        let mut seen_external = std::collections::BTreeSet::new();
        for dep in &self.dependency_closure {
            let key = (dep.name.as_str(), dep.closure_ref.as_str());
            if !seen_external.insert(key) {
                return Err(invalid("dependency_closure", "duplicate external closure"));
            }
            let facts = lock
                .closure()
                .external_backends
                .iter()
                .find(|x| x.backend.as_str() == key.0 && x.closure.as_str() == key.1)
                .ok_or_else(|| {
                    invalid(
                        "dependency_closure",
                        "wire external closure is not selected",
                    )
                })?;
            let evidence = external_map.get(&key).ok_or_else(|| {
                invalid(
                    "dependency_closure",
                    "external closure lacks captured bytes",
                )
            })?;
            let digest = format!("{:x}", Sha256::digest(evidence.bytes));
            let configuration_bytes = evidence.configuration_bytes.ok_or_else(|| {
                invalid(
                    "dependency_closure",
                    "external configuration bytes are unavailable",
                )
            })?;
            let configuration_digest = format!("{:x}", Sha256::digest(configuration_bytes));
            if dep.role != role(facts.role)
                || dep.configuration_ref != facts.configuration.as_str()
                || dep.configuration_owner != evidence.configuration_owner.as_str()
                || dep.configuration_digest_sha256 != configuration_digest
                || dep.closure_owner != evidence.owner.as_str()
                || dep.closure_digest_sha256 != digest
            {
                return Err(invalid(
                    "dependency_closure",
                    "external closure facts or digest differ",
                ));
            }
        }
        let digest = exact_lock_digest(definition, lock, selected_target, packages, external)
            .map_err(|_| {
                invalid(
                    "lock_digest_sha256",
                    "typed exact-lock recipe refused supplied closure",
                )
            })?;
        if self.lock_digest_sha256 != digest.as_str() {
            return Err(invalid(
                "lock_digest_sha256",
                "stale or mismatched exact-lock digest",
            ));
        }
        Ok(digest)
    }
}
