use super::*;
use std::collections::BTreeMap;
use vibe_orchestrator::{
    document_analysis::{AnalysisLimits, DocumentAnalysisHost, ResourceDeclaration, ResourceRole},
    machine_operation::boot_debugger::{
        BootDebuggerRoute, CapturedKind, debugger_request as q, debugger_result as r,
        hypothetical::{HostCapture, PreparedScenario},
    },
};
use vibe_resolver::resolvo_engine::request::NormalizedRequest;

fn request(host: &HostCapture) -> NormalizedRequest {
    NormalizedRequest::legacy(&host.manifest.requires.packages, &Default::default())
}
fn manifest(added: bool) -> String {
    format!(
        "[project]\nname='fixture'\nversion='1.0.0'\n[boot]\ndefault_link='static'\n[requires.packages]\n'org.example/app'='=1.0.0'\n{}",
        if added {
            "'org.example/added'='=1.0.0'\n"
        } else {
            ""
        }
    )
}
fn inventory(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = vec![];
    fn visit(root: &Path, path: &Path, files: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    std::fs::read(path).unwrap(),
                ));
            }
        }
    }
    visit(root, root, &mut files);
    files.sort();
    files
}
#[test]
fn standard_bound_route_compares_real_shared_closures_without_project_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("vibe.toml"), manifest(false)).unwrap();
    std::fs::write(root.join("INDEX.md"), "canonical index").unwrap();
    let registry = temp.path().join("registry");
    for name in ["app", "added", "shared"] {
        let dir = registry.join(format!("org.example/{name}/v1.0.0"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vibe.toml"),format!("[package]\ngroup='org.example'\nname='{name}'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n[[boot_snippet.fragment]]\nsource='dynamic.md'\nwhen='os:linux'\n{}", if name == "shared" { "" } else { "[requires.packages]\n'org.example/shared'='=1.0.0'\n" })).unwrap();
        std::fs::write(dir.join("boot.md"), format!("# {name}\n\nEXACT {name}\n")).unwrap();
        std::fs::write(dir.join("dynamic.md"), format!("potential {name}\n")).unwrap();
    }
    let initial = vibe_core::manifest::Manifest::read(root.join("vibe.toml")).unwrap();
    let source = crate::build_install_resolver(
        &crate::PackageSourceOptions {
            registry: Some(registry.clone()),
            ..Default::default()
        },
        &initial,
        None,
        &root,
        &Default::default(),
        true,
        &[],
    )
    .unwrap();
    let host = DocumentAnalysisHost::new(
        &root,
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "INDEX.md".into(),
                role: ResourceRole::Opaque,
            },
        ],
        AnalysisLimits {
            resource_bytes: 100_000,
            capture_bytes: 200_000,
            open_buffers: 2,
        },
    )
    .unwrap();
    let client = host.client().unwrap();
    let artifact = host.resource("INDEX.md").unwrap();
    let baseline = client.freeze().unwrap();
    std::fs::write(root.join("vibe.toml"), manifest(true)).unwrap();
    let candidate = client.freeze().unwrap();
    let route = BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap();
    let b = route.retain_capture(baseline).unwrap();
    let c = route.retain_capture(candidate).unwrap();
    let bh = route.hypothetical_host(&b).unwrap();
    let ch = route.hypothetical_host(&c).unwrap();
    let before = inventory(&root);
    let input = source
        .bind_boot_preview(
            &route,
            &b,
            &c,
            request(&bh),
            request(&ch),
            "os:linux".into(),
        )
        .unwrap();
    let first = route.request(input.clone()).unwrap();
    let r::Body::Preview(body) = &first.body else {
        panic!()
    };
    assert_eq!(body.status, r::BodyPreviewStatus::Complete, "{body:?}");
    let prepared = PreparedScenario::new(
        source.capture_boot_world(request(&bh), bh).unwrap(),
        source.capture_boot_world(request(&ch), ch).unwrap(),
        "os:linux".into(),
    )
    .unwrap();
    let c = prepared
        .evaluate(
            prepared.identity(),
            prepared.baseline().identity(),
            prepared.candidate().identity(),
            prepared.candidate().catalog(),
            &BTreeMap::new(),
        )
        .unwrap()
        .comparison
        .unwrap();
    assert_eq!(c.baseline.packages.len(), 2);
    assert_eq!(c.candidate.packages.len(), 3);
    let added = c
        .candidate
        .package_costs
        .iter()
        .find(|p| p.coordinate == "org.example/added")
        .unwrap();
    let vibe_orchestrator::machine_operation::boot_debugger::hypothetical::comparison::SignedCount::Exact { value: marginal } = c.marginal[0].value else { panic!() };
    let inclusive = added.static_raw_inclusive.exact_value().unwrap();
    assert!(marginal > 0 && (marginal as u64) < inclusive);
    assert!(c.baseline.compiler.is_some() && c.candidate.compiler.is_some());
    let encoded = body.comparison_json.as_ref().unwrap();
    assert!(encoded.contains(&c.baseline.world) && encoded.contains(&c.candidate.world));
    assert!(encoded.contains("dynamic_potential"));
    assert!(encoded.contains(&format!("\"value\":{marginal}")));
    for delta in &c.marginal {
        assert_eq!(delta.baseline.world, c.baseline.world);
        assert_eq!(delta.candidate.world, c.candidate.world);
        assert!(!delta.candidate.declarations.is_empty());
    }
    assert_eq!(before, inventory(&root));
    std::fs::remove_dir_all(registry).unwrap();
    assert_eq!(first, route.request(input.clone()).unwrap());
    for catalog in [true, false] {
        let mut stale = input.clone();
        let q::Command::Preview(p) = &mut stale.command else {
            panic!()
        };
        if catalog {
            p.expected_catalog.push_str("stale");
        } else {
            p.expected_baseline.push_str("stale");
        }
        let r::Body::Preview(stale) = route.request(stale).unwrap().body else {
            panic!()
        };
        assert_eq!(stale.status, r::BodyPreviewStatus::Stale);
        assert!(stale.comparison_json.is_none());
    }
    assert_eq!(before, inventory(&root));
}

#[test]
fn configured_selected_provider_worlds_attribute_shared_membership_once() {
    // Reuse the incumbent real local catalog fixture, including alternative
    // provider and capability declarations and a configured dependency toggle.
    let (temp, source, mut off) = super::m_05_c_3_cases::fixture_with_boot(None, true);
    let project = temp.path().join("host");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("vibe.toml"), manifest(false)).unwrap();
    std::fs::write(project.join("INDEX.md"), "canonical").unwrap();
    let host = DocumentAnalysisHost::new(
        &project,
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "INDEX.md".into(),
                role: ResourceRole::Opaque,
            },
        ],
        AnalysisLimits {
            resource_bytes: 100_000,
            capture_bytes: 200_000,
            open_buffers: 2,
        },
    )
    .unwrap();
    let capture = host.client().unwrap().freeze().unwrap();
    let artifact = host.resource("INDEX.md").unwrap();
    let route = BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap();
    let id = route.retain_capture(capture).unwrap();
    let captured = route.hypothetical_host(&id).unwrap();
    let mut baseline_request = request(&captured);
    baseline_request.configuration = off.configuration.take();
    let mut candidate_request = baseline_request.clone();
    candidate_request
        .configuration
        .as_mut()
        .unwrap()
        .request
        .edges[0]
        .features = vec!["on".into()];
    let before = inventory(temp.path());
    let input = source
        .bind_boot_preview(
            &route,
            &id,
            &id,
            baseline_request.clone(),
            candidate_request.clone(),
            "os:linux".into(),
        )
        .unwrap();
    let r::Body::Preview(response) = route.request(input).unwrap().body else {
        panic!()
    };
    assert_eq!(
        response.status,
        r::BodyPreviewStatus::Complete,
        "{response:?}"
    );
    let baseline = source
        .capture_boot_world(baseline_request, captured.clone())
        .unwrap();
    let candidate = source
        .capture_boot_world(candidate_request, captured)
        .unwrap();
    let scenario = PreparedScenario::new(baseline, candidate, "os:linux".into()).unwrap();
    let p = scenario
        .evaluate(
            scenario.identity(),
            scenario.baseline().identity(),
            scenario.candidate().identity(),
            scenario.candidate().catalog(),
            &BTreeMap::new(),
        )
        .unwrap();
    let c = p.comparison.unwrap();
    let encoded = response.comparison_json.as_ref().unwrap();
    assert!(encoded.contains(&c.baseline.world) && encoded.contains(&c.candidate.world));
    assert!(encoded.contains("provider_identity") && encoded.contains("configured_definition"));
    assert_eq!(c.baseline.packages.len(), 2);
    assert_eq!(c.candidate.packages.len(), 3);
    assert_eq!(
        c.candidate
            .packages
            .iter()
            .filter(|p| p.coordinate == "org.example/provider")
            .count(),
        1
    );
    assert!(
        !c.candidate
            .packages
            .iter()
            .any(|p| p.coordinate == "org.example/missing")
    );
    assert!(
        c.candidate
            .resolution_evidence
            .contains("provider_identity")
    );
    assert!(
        c.candidate
            .resolution_evidence
            .contains("configured_definition")
    );
    assert!(c.baseline.missing.is_empty() && c.candidate.missing.is_empty());
    assert!(c.marginal.iter().all(|d| matches!(d.value, vibe_orchestrator::machine_operation::boot_debugger::hypothetical::comparison::SignedCount::Exact {..})));
    assert_eq!(before, inventory(temp.path()));
}
