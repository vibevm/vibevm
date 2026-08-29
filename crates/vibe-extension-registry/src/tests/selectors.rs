use specmark::verifies;
use vibe_core::manifest::ExtensionsControl;

use crate::{CollectionError, HostIdentity, SelectorProvider, SelectorSubject, collect_extensions};

use super::support::{host, provider_id, selected_declaration, world};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn selector_dimensions_are_or_within_and_between() {
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![
                selected_declaration("absent", None, None),
                selected_declaration(
                    "package-or",
                    Some(vec!["org.alpha/*", "org.beta/tool"]),
                    None,
                ),
                selected_declaration("path-or", None, Some(vec!["src/*.md", "docs/**"])),
                selected_declaration("both", Some(vec!["org.alpha/*"]), Some(vec!["docs/**"])),
                selected_declaration("empty-package", Some(Vec::new()), None),
                selected_declaration("empty-path", None, Some(Vec::new())),
            ],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let alpha = provider_id("org.alpha", "one");
    let beta = provider_id("org.beta", "tool");
    let upper = provider_id("org.alpha", "one");

    assert!(matches(&registry, "#absent", SelectorSubject::unscoped()));
    assert!(matches(
        &registry,
        "#package-or",
        SelectorSubject::package(&alpha)
    ));
    assert!(matches(
        &registry,
        "#package-or",
        SelectorSubject::package(&beta)
    ));
    assert!(!matches(
        &registry,
        "#package-or",
        SelectorSubject::unscoped()
    ));
    assert!(matches(
        &registry,
        "#path-or",
        SelectorSubject::path("src/root.md")
    ));
    assert!(matches(
        &registry,
        "#path-or",
        SelectorSubject::path("docs/guides/nested.md")
    ));
    assert!(!matches(
        &registry,
        "#path-or",
        SelectorSubject::path("Docs/guides/nested.md")
    ));
    assert!(matches(
        &registry,
        "#both",
        SelectorSubject::new(Some(&alpha), Some("docs/guide.md"))
    ));
    assert!(!matches(
        &registry,
        "#both",
        SelectorSubject::new(Some(&beta), Some("docs/guide.md"))
    ));
    assert!(!matches(
        &registry,
        "#both",
        SelectorSubject::new(Some(&upper), Some("src/guide.md"))
    ));
    assert!(!matches(
        &registry,
        "#empty-package",
        SelectorSubject::package(&alpha)
    ));
    assert!(!matches(
        &registry,
        "#empty-path",
        SelectorSubject::path("anything")
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn selector_match_is_case_sensitive_and_package_text_is_versionless() {
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![selected_declaration(
                "selected",
                Some(vec!["org.demo/tools"]),
                Some(vec!["spec/Guide.md"]),
            )],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let provider = provider_id("org.demo", "tools");
    assert!(matches(
        &registry,
        "#selected",
        SelectorSubject::new(Some(&provider), Some("spec/Guide.md"))
    ));
    assert!(!matches(
        &registry,
        "#selected",
        SelectorSubject::new(Some(&provider), Some("spec/guide.md"))
    ));
    assert_eq!(provider.to_string(), "org.demo/tools");
    assert!(!provider.to_string().contains('@'));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
fn selector_mismatch_stays_in_all_view_but_not_in_plan() {
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![selected_declaration(
                "selected",
                Some(vec!["org.allowed/*"]),
                None,
            )],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let denied = provider_id("org.denied", "tool");
    let subject = SelectorSubject::package(&denied);
    let all = registry.all(subject);
    assert_eq!(registry.rows().len(), 1);
    assert_eq!(all.len(), 1);
    assert!(!all[0].selector_matches);
    assert!(!all[0].is_effective());
    assert!(
        registry
            .plan("compile:source".parse().unwrap(), subject)
            .is_empty()
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn malformed_glob_is_a_collection_error_naming_key_and_field() {
    for (packages, paths, field, pattern) in [
        (
            Some(vec!["org.demo/["]),
            None,
            "applies_to.packages",
            "org.demo/[",
        ),
        (None, Some(vec!["docs/["]), "applies_to.paths", "docs/["),
    ] {
        let error = collect_extensions(world(
            Vec::new(),
            host(
                vec![selected_declaration("bad", packages, paths)],
                ExtensionsControl::default(),
            ),
            None,
        ))
        .unwrap_err();
        let CollectionError::MalformedSelector {
            key,
            field: actual_field,
            pattern: actual_pattern,
            reason,
        } = error
        else {
            panic!("expected malformed selector");
        };
        assert_eq!(key.as_str(), "__host__/demo#bad");
        assert_eq!(actual_field, field);
        assert_eq!(actual_pattern, pattern);
        assert!(!reason.is_empty());
    }
}

fn matches(
    registry: &crate::ExtensionRegistry,
    suffix: &str,
    subject: SelectorSubject<'_>,
) -> bool {
    registry
        .all(subject)
        .into_iter()
        .find(|view| view.row.key().as_str().ends_with(suffix))
        .unwrap_or_else(|| panic!("selector test row with suffix `{suffix}` exists"))
        .selector_matches
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn host_coordinate_and_ungrouped_percent_coded_identity_match_distinctly() {
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![
                selected_declaration("alpha", Some(vec!["org.alpha/*"]), None),
                selected_declaration("awkward", Some(vec!["__host__/my%20app"]), None),
                selected_declaration("raw-space", Some(vec!["__host__/my app"]), None),
            ],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let alpha = provider_id("org.alpha", "one");
    let beta = provider_id("org.beta", "tool");
    let coordinate = HostIdentity::coordinate(provider_id("org.alpha", "one"));
    let ungrouped = HostIdentity::ungrouped_project("my app");

    // Dependency behavior stays exact through every constructor spelling.
    assert!(matches(
        &registry,
        "#alpha",
        SelectorSubject::dependency(&alpha)
    ));
    assert!(matches(
        &registry,
        "#alpha",
        SelectorSubject::package(&alpha)
    ));
    assert!(matches(
        &registry,
        "#alpha",
        SelectorSubject::new(Some(&alpha), None)
    ));
    assert!(!matches(
        &registry,
        "#alpha",
        SelectorSubject::dependency(&beta)
    ));
    assert!(!matches(&registry, "#alpha", SelectorSubject::unscoped()));

    // A coordinate host renders through the same coordinate codec…
    assert!(matches(
        &registry,
        "#alpha",
        SelectorSubject::host(&coordinate)
    ));
    // …while the ungrouped host renders its percent-coded owner spelling,
    // so the two host identities never satisfy each other's patterns.
    assert!(!matches(
        &registry,
        "#alpha",
        SelectorSubject::host(&ungrouped)
    ));
    assert!(matches(
        &registry,
        "#awkward",
        SelectorSubject::host(&ungrouped)
    ));
    assert!(!matches(
        &registry,
        "#awkward",
        SelectorSubject::host(&coordinate)
    ));
    assert!(!matches(
        &registry,
        "#awkward",
        SelectorSubject::dependency(&alpha)
    ));
    assert!(!matches(&registry, "#awkward", SelectorSubject::unscoped()));
    // The authored pattern must use the canonical encoding: the raw space
    // never matches the rendered percent-coded identity.
    assert!(!matches(
        &registry,
        "#raw-space",
        SelectorSubject::host(&ungrouped)
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn provider_and_path_dimensions_stay_or_within_and_between() {
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![
                selected_declaration("absent", None, None),
                selected_declaration("combo", Some(vec!["org.alpha/*"]), Some(vec!["docs/**"])),
                selected_declaration(
                    "either",
                    Some(vec!["org.beta/*", "org.gamma/*"]),
                    Some(vec!["docs/**", "src/*.md"]),
                ),
            ],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let alpha = provider_id("org.alpha", "one");
    let beta = provider_id("org.beta", "tool");
    let gamma = provider_id("org.gamma", "kit");
    let coordinate = HostIdentity::coordinate(provider_id("org.alpha", "one"));

    // Unscoped behaviour is unchanged: an absent selector matches everything.
    assert!(matches(&registry, "#absent", SelectorSubject::unscoped()));
    // AND between dimensions: satisfying one dimension alone never suffices.
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::dependency(&alpha)
    ));
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::host(&coordinate)
    ));
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::path("docs/guide.md")
    ));
    assert!(matches(
        &registry,
        "#combo",
        SelectorSubject::new(Some(&alpha), Some("docs/guide.md"))
    ));
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::new(Some(&alpha), Some("src/guide.md"))
    ));
    // OR within each dimension: each member alone can carry its dimension.
    assert!(matches(
        &registry,
        "#either",
        SelectorSubject::new(Some(&beta), Some("src/guide.md"))
    ));
    assert!(matches(
        &registry,
        "#either",
        SelectorSubject::new(Some(&gamma), Some("docs/guide.md"))
    ));
    // The other dimension still gates: a failed member fails the row.
    assert!(!matches(
        &registry,
        "#either",
        SelectorSubject::new(Some(&alpha), Some("src/guide.md"))
    ));
    assert!(!matches(
        &registry,
        "#either",
        SelectorSubject::new(Some(&beta), Some("root/guide.md"))
    ));
    assert!(!matches(
        &registry,
        "#either",
        SelectorSubject::path("docs/guide.md")
    ));
    assert!(!matches(
        &registry,
        "#either",
        SelectorSubject::dependency(&beta)
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn typed_provider_subjects_compose_with_a_path_dimension() {
    // Rows authoring BOTH `packages` and `paths` are the source/document
    // grain: their provider and path dimensions must compose for every
    // typed provider, including the host spellings no legacy
    // dependency-shaped constructor can carry a path beside.
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![
                selected_declaration("combo", Some(vec!["org.alpha/*"]), Some(vec!["docs/**"])),
                selected_declaration(
                    "host-combo",
                    Some(vec!["__host__/my%20app"]),
                    Some(vec!["docs/**"]),
                ),
            ],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let alpha = provider_id("org.alpha", "one");
    let beta = provider_id("org.beta", "tool");
    let coordinate = HostIdentity::coordinate(provider_id("org.alpha", "one"));
    let ungrouped = HostIdentity::ungrouped_project("my app");

    // Dependency + path composes through the general typed constructor…
    assert!(matches(
        &registry,
        "#combo",
        SelectorSubject::scoped(
            Some(SelectorProvider::Dependency(&alpha)),
            Some("docs/x.md")
        )
    ));
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::scoped(Some(SelectorProvider::Dependency(&beta)), Some("docs/x.md"))
    ));
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::scoped(Some(SelectorProvider::Dependency(&alpha)), Some("src/x.md"))
    ));
    // …and a coordinate host + path satisfies the same row, failing exactly
    // when either dimension misses (AND between dimensions).
    assert!(matches(
        &registry,
        "#combo",
        SelectorSubject::scoped(Some(SelectorProvider::Host(&coordinate)), Some("docs/x.md"))
    ));
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::scoped(Some(SelectorProvider::Host(&coordinate)), Some("src/x.md"))
    ));
    assert!(!matches(
        &registry,
        "#combo",
        SelectorSubject::scoped(Some(SelectorProvider::Host(&ungrouped)), Some("docs/x.md"))
    ));
    // An ungrouped percent-coded host + path reaches its own both-dimension
    // row, which no dependency or coordinate subject can satisfy.
    assert!(matches(
        &registry,
        "#host-combo",
        SelectorSubject::scoped(Some(SelectorProvider::Host(&ungrouped)), Some("docs/x.md"))
    ));
    assert!(!matches(
        &registry,
        "#host-combo",
        SelectorSubject::scoped(Some(SelectorProvider::Host(&ungrouped)), Some("src/x.md"))
    ));
    assert!(!matches(
        &registry,
        "#host-combo",
        SelectorSubject::scoped(
            Some(SelectorProvider::Dependency(&alpha)),
            Some("docs/x.md")
        )
    ));
    assert!(!matches(
        &registry,
        "#host-combo",
        SelectorSubject::scoped(Some(SelectorProvider::Host(&coordinate)), Some("docs/x.md"))
    ));
}
