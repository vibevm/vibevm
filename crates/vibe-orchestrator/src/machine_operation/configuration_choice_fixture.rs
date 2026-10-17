//! Shared real-source fixture for CLI, MCP and service regressions.
#[path = "../install/m_05_e_2_fixture.rs"]
mod source_fixture;
use super::vibe_orchestrator::document_analysis::{
    ConfigurationAdmission, PreparedConfigurationDiscovery, prepare_configuration_discovery,
};
use super::vibe_orchestrator::machine_operation::configuration_choice::ConfigurationChoiceHost;

pub fn fixture() -> (tempfile::TempDir, ConfigurationChoiceHost) {
    fixture_with_source(None)
}

/// An online-posture host that counts and refuses every source operation.
pub fn online_refusing_fixture(
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) -> (tempfile::TempDir, ConfigurationChoiceHost) {
    fixture_with_source(Some(calls))
}

fn fixture_with_source(
    refused: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
) -> (tempfile::TempDir, ConfigurationChoiceHost) {
    let mut f = source_fixture::Fixture::new();
    // Match the declared wildcard root, including its authored constraint shape.
    let root: vibe_core::PackageRef = format!("{}@*", f.request.roots[0]).parse().unwrap();
    f.request.roots = vec![root.clone()];
    f.request.root_origins[0].constraint = root;
    let mut install = f.install_request();
    install.offline = refused.is_none();
    let c = f.request.configuration.as_mut().unwrap();
    c.request.edges[0].selections.remove("choice");
    use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
    let feature = |s| LocalFeatureRef::new(s).unwrap();
    c.definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&feature("choice"))
        .unwrap()
        .defaults = Some([feature("b")].into());
    c.definitions[0]
        .definition
        .program
        .features
        .insert(feature("optional"), Default::default());
    c.definitions[0]
        .definition
        .constraints
        .member_guards
        .insert(feature("optional"), vec![Guard::Member(feature("a"))]);
    c.definitions[0].definition.constraints.groups.insert(
        feature("extras"),
        ConstrainedGroup {
            cardinality: Cardinality::ZeroOrMore,
            members: [feature("optional")].into(),
            guards: vec![],
            defaults: None,
        },
    );
    c.definitions[0]
        .definition
        .program
        .features
        .get_mut(&feature("a"))
        .unwrap()
        .insert(FeatureEffect::ConfigValue(feature("picked")));
    c.definitions[0]
        .definition
        .declarations
        .config_values
        .insert(feature("picked"), ConfigValue::Bool(true));
    let candidate = c.definitions[0].clone();
    let package = f
        .temp
        .path()
        .join("registry")
        .join(candidate.package.group.as_ref().unwrap().as_str())
        .join("app/v1.0.0");
    let metadata = vibe_registry::CandidateMetadata::read_local_registry(&package).unwrap();
    std::fs::create_dir(f.root.join("cache")).unwrap();
    let bytes = std::fs::read(package.join("vibe.toml")).unwrap();
    std::fs::write(f.root.join("cache/vibe.toml"), &bytes).unwrap();
    let capture = prepare_configuration_discovery(&f.root, &["cache/vibe.toml".into()]).unwrap();
    let resource = capture
        .resources()
        .find(|(_, r)| r.path() == "cache/vibe.toml")
        .unwrap()
        .0;
    let basis = PreparedConfigurationDiscovery::admit(
        capture,
        vec![ConfigurationAdmission {
            resource,
            manifest_bytes: bytes,
            metadata,
            candidate,
        }],
    );
    let host = ConfigurationChoiceHost::new(
        basis,
        f.request,
        match refused {
            Some(calls) => Box::new(RefusingSource(calls)),
            None => Box::new(f.source),
        },
        install,
        vibe_core::manifest::SpecFormat::Mixed,
    )
    .unwrap()
    .with_application(
        vibe_core::user_config::SlotIntegrity::TrustPresence,
        vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    );
    (f.temp, host)
}

struct RefusingSource(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl RefusingSource {
    fn refuse<T>(&self) -> T {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        panic!("offline mismatch reached source activity")
    }
}
impl vibe_install::InstallSource for RefusingSource {
    fn begin_normalized_transaction(
        &self,
        _: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
    ) -> Result<Option<Box<dyn vibe_install::InstallSource + '_>>, vibe_resolver::SolveError> {
        self.refuse()
    }
    fn resolve_and_fetch(
        &self,
        _: &vibe_core::PackageRef,
        _: &std::path::Path,
        _: Option<&str>,
    ) -> Result<vibe_registry::CachedPackage, vibe_registry::RegistryError> {
        self.refuse()
    }
    fn solve(
        &self,
        _: &[vibe_core::PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        self.refuse()
    }
    fn manifest_of(
        &self,
        _: &vibe_core::PackageRef,
    ) -> Result<vibe_core::manifest::Manifest, vibe_resolver::SolveError> {
        self.refuse()
    }
    fn solve_masked(
        &self,
        _: &[vibe_core::PackageRef],
        _: &std::collections::BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        self.refuse()
    }
    fn materialise_in_place(
        &self,
        _: &vibe_core::PackageRef,
        _: &std::path::Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, vibe_registry::RegistryError> {
        self.refuse()
    }
}
