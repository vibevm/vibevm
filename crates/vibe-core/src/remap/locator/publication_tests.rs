#![cfg(test)]
use super::*;
use crate::{
    RoutePublication,
    manifest::{Manifest, MirrorSection},
    remap::{Provenance, RowDecl, rows_from_mirrors},
};
#[test]
fn project_origin_is_retained_before_ordering_even_when_user_url_is_identical() {
    let project:Manifest=toml::from_str("[project]\nname='test'\nversion='0.1.0'\n[[registry]]\nname='r'\nurl='https://origin.invalid'\n[[mirror]]\nof='r'\nurl='https://same.invalid'\n").unwrap();
    let global: crate::GlobalRegistryConfig =
        toml::from_str("[[mirror]]\nof='r'\nurl='https://same.invalid'\n").unwrap();
    let merged = crate::merge_effective(&project, &global);
    let rows = rows_from_mirrors(&merged.mirrors, &merged.registries, Layer::Local, "catalog");
    assert_eq!(rows[0].from.publication, RoutePublication::Project);
    assert_eq!(rows[1].from.publication, RoutePublication::Local);
    let locator = Locator::from_sources(
        vec![LoadedSource {
            layer: Layer::Local,
            source: "catalog".into(),
            policy: UnmatchedPolicy::Allow,
            rows,
        }],
        vec![("r".into(), "https://origin.invalid".into())],
    );
    let artifact =
        Artifact::new(super::super::ArtifactKind::RegistryClone).with_url("https://origin.invalid");
    assert_eq!(
        locator
            .choose_candidate(&artifact)
            .unwrap()
            .publication_scope(),
        RoutePublication::Local,
        "same URL cannot promote the actual user winner to project scope"
    );
    let mirror: MirrorSection = toml::from_str("of='r'\nurl='https://same.invalid'").unwrap();
    assert_eq!(mirror.publication, RoutePublication::Local);
    assert!(
        !toml::to_string(&merged.mirrors[0])
            .unwrap()
            .contains("publication")
    );
}
#[test]
fn held_operator_selection_excludes_catalog_and_preserves_operator_precedence() {
    let mut catalog: crate::remap::RemapRow =
        toml::from_str::<RowDecl>("package='org.example/p'\nurl='https://catalog.invalid/p'")
            .unwrap()
            .into_row(
                Provenance::new(Layer::Local, "catalog", 0).catalog(RoutePublication::Project),
            )
            .unwrap();
    catalog.target_ref = Some("different-ref".into());
    let user = toml::from_str::<RowDecl>("registry='held-alias'\nurl='https://user.invalid/p'")
        .unwrap()
        .into_row(Provenance::new(Layer::Invocation, "explicit remap", 0))
        .unwrap();
    let locator = Locator::from_sources(
        vec![
            LoadedSource {
                layer: Layer::Local,
                source: "catalog".into(),
                policy: UnmatchedPolicy::Allow,
                rows: vec![catalog],
            },
            LoadedSource {
                layer: Layer::Invocation,
                source: "explicit remap".into(),
                policy: UnmatchedPolicy::Allow,
                rows: vec![user],
            },
        ],
        vec![("held-alias".into(), "https://edited-catalog.invalid".into())],
    );
    let artifact = Artifact::new(super::super::ArtifactKind::GitSourceDep)
        .with_url("https://canonical.invalid/p")
        .with_package("org.example/p@1.0.0");
    let chosen = locator
        .choose_operator_candidate(&artifact, Some("held-alias"))
        .unwrap();
    assert_eq!(chosen.target.display(), "https://user.invalid/p");
    assert_eq!(chosen.publication_scope(), RoutePublication::Local);
    assert_eq!(chosen.row.unwrap().provenance.source, "explicit remap");
}
