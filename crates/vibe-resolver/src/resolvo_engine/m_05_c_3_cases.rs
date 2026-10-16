use super::*;
use crate::{DepProvider, DepSolver};
use vibe_core::manifest::Manifest;

struct Source(Vec<Manifest>);
impl DepProvider for Source {
    fn resolve_version(&self, p: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.list_versions(p.group.as_ref().unwrap(), p.name.as_str())?
            .into_iter()
            .filter(|v| p.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::Other("no match".into()))
    }
    fn fetch_manifest(
        &self,
        _: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.0
            .iter()
            .find(|m| {
                m.require_package().unwrap().name == n && m.require_package().unwrap().version == *v
            })
            .cloned()
            .ok_or_else(|| DepProviderError::Other("missing manifest".into()))
    }
}
impl VersionEnumerator for Source {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        let versions: Vec<_> = self
            .0
            .iter()
            .filter_map(|m| {
                let p = m.require_package().unwrap();
                (p.name == n).then(|| p.version.clone())
            })
            .collect();
        if versions.is_empty() {
            Err(DepProviderError::UnknownPackage {
                group: g.clone(),
                name: n.into(),
            })
        } else {
            Ok(versions)
        }
    }
}
fn p(n: &str) -> PackageRef {
    PackageRef::parse(&format!("org.example/{n}")).unwrap()
}
fn m(n: &str, v: u64, body: &str) -> Manifest {
    Manifest::parse_str(&format!(
        "[package]\ngroup='org.example'\nname='{n}'\nkind='flow'\nversion='{v}.0.0'\n{body}"
    ))
    .unwrap()
}
#[test]
fn selected_capability_and_or_edges_keep_exact_provenance() {
    let source = Source(vec![
        m(
            "app",
            1,
            "[[requires_any]]\none_of=['org.example/missing','org.example/provider']\n[requires]\ncapabilities=['capability:engine@1.0.0']\n",
        ),
        m(
            "provider",
            1,
            "[provides]\ncapabilities=['capability:engine@1.0.0']\n",
        ),
    ]);
    let result = ResolvoDepSolver::new(source)
        .solve_recorded(&[p("app")])
        .unwrap();
    let app = result
        .graph
        .find(p("app").group.as_ref().unwrap(), "app")
        .unwrap();
    assert_eq!(app.dependencies, vec![p("provider@=1.0.0")]);
    assert_eq!(result.choices.len(), 2);
    let or = result
        .choices
        .iter()
        .find(|e| e.declaration == "requires_any[0]")
        .unwrap();
    assert_eq!(or.alternative, 1);
    assert!(
        result
            .choices
            .iter()
            .all(|e| e.parent == p("app@=1.0.0") && e.provider == p("provider@=1.0.0"))
    );
    // Both alternatives may be selected independently; exactly one witness
    // belongs to the OR declaration, never an inferred all-provider edge set.
    let source = Source(vec![
        m(
            "app",
            1,
            "[[requires_any]]\none_of=['org.example/a','org.example/b']\n",
        ),
        m("a", 1, ""),
        m("b", 1, ""),
    ]);
    let result = ResolvoDepSolver::new(source)
        .solve_recorded(&[p("app"), p("a"), p("b")])
        .unwrap();
    assert_eq!(result.choices.len(), 1);
    assert_eq!(
        result.graph.packages[0].dependencies,
        vec![result.choices[0].provider.clone()]
    );
}
#[test]
fn duplicate_root_constraints_are_not_first_wins() {
    let solver = ResolvoDepSolver::new(Source(vec![m("app", 1, ""), m("app", 2, "")]));
    assert!(matches!(
        solver.solve(&[p("app@=1.0.0"), p("app@=2.0.0")]),
        Err(SolveError::Unsatisfiable { .. })
    ));
    let mut kind = p("app@=1.0.0");
    kind.kind = Some(vibe_core::PackageKind::Tool);
    assert!(matches!(
        solver.solve(&[p("app@=1.0.0"), kind]),
        Err(SolveError::KindMismatch { .. })
    ));
}
