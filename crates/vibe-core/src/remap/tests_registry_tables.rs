//! Registry tables composed onto an invocation Locator
//! (`PROP-066#locator`, `PROP-066#precedence`).

use specmark::verifies;

use super::*;

fn row(toml_body: &str, layer: Layer, source: &str, index: usize) -> RemapRow {
    let decl: RowDecl = toml::from_str(toml_body).expect("row parses");
    decl.into_row(Provenance::new(layer, source, index))
        .expect("row is well shaped")
}

fn first_target(locator: &Locator, artifact: &Artifact) -> String {
    match locator.locate(artifact) {
        Resolution::Candidates(candidates) => candidates
            .first()
            .map(|c| c.target.display())
            .unwrap_or_else(|| "<none>".into()),
        Resolution::Denied { .. } => "<denied>".into(),
    }
}

/// A catalog that reads its `[[mirror]]` tables on demand composes them
/// into a Locator that already carries other rows — an unrelated `[[remap]]`
/// prefix and the package drop — and the mirror serves the registry clone,
/// while an explicit `[[remap]]` row naming the same registry still wins
/// (`PROP-066#precedence`: equal specificity, the later local row).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
fn mirror_tables_compose_onto_a_locator_that_already_has_rows() {
    let registries = vec![vibespecs_registry()];
    let declared = vec![(
        "vibespecs".to_owned(),
        "https://github.com/vibespecs".to_owned(),
    )];
    let base = Locator::from_sources(
        vec![
            LoadedSource {
                layer: Layer::Local,
                source: REGISTRY_TABLES_SOURCE.into(),
                rows: Vec::new(),
                policy: UnmatchedPolicy::Allow,
            },
            LoadedSource {
                layer: Layer::Local,
                source: "settings.toml".into(),
                rows: vec![row(
                    "prefix = \"https://unrelated.example/\"\nurl = \"https://elsewhere.example/\"\n",
                    Layer::Local,
                    "settings.toml",
                    0,
                )],
                policy: UnmatchedPolicy::Allow,
            },
            LoadedSource {
                layer: Layer::Local,
                source: drop::PACKAGES_DROP_SOURCE.into(),
                rows: vec![drop::package_drop_row(
                    std::path::Path::new("/drop"),
                    Layer::Local,
                    "config.toml",
                )],
                policy: UnmatchedPolicy::Allow,
            },
        ],
        declared.clone(),
    );
    let mirrors = [crate::manifest::MirrorSection {
        publication: Default::default(),
        of: "vibespecs".into(),
        url: "https://nexus.corp.example/vibespecs".into(),
        priority: 0,
    }];
    let tables = rows_from_registry_tables(&registries, &mirrors, &[], Layer::Local);
    let clone = Artifact::new(ArtifactKind::RegistryClone).with_url("https://github.com/vibespecs");
    let composed = base.with_registry_tables(tables.clone(), declared.clone());
    assert_eq!(
        first_target(&composed, &clone),
        "https://nexus.corp.example/vibespecs"
    );
    // Composed exactly once: a second composition replaces, never stacks.
    let twice = composed.with_registry_tables(tables.clone(), declared.clone());
    assert_eq!(twice.rows().len(), composed.rows().len());

    // An explicit `[[remap]]` row for the same registry outranks the mirror.
    let with_remap = Locator::from_sources(
        vec![
            LoadedSource {
                layer: Layer::Local,
                source: REGISTRY_TABLES_SOURCE.into(),
                rows: Vec::new(),
                policy: UnmatchedPolicy::Allow,
            },
            LoadedSource {
                layer: Layer::Local,
                source: "settings.toml".into(),
                rows: vec![row(
                    "registry = \"vibespecs\"\nurl = \"https://explicit.example/vibespecs\"\n",
                    Layer::Local,
                    "settings.toml",
                    0,
                )],
                policy: UnmatchedPolicy::Allow,
            },
        ],
        declared.clone(),
    )
    .with_registry_tables(tables, declared);
    assert_eq!(
        first_target(&with_remap, &clone),
        "https://explicit.example/vibespecs"
    );
}

fn vibespecs_registry() -> crate::manifest::RegistrySection {
    crate::manifest::RegistrySection {
        name: "vibespecs".into(),
        r#type: None,
        url: "https://github.com/vibespecs".into(),
        r#ref: "main".into(),
        naming: crate::manifest::NamingConvention::Fqdn,
        auth: None,
        token_env: None,
        enabled: true,
        index_url: None,
    }
}
