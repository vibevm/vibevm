use specmark::verifies;
use vibe_core::manifest::ExtensionsControl;

use crate::registry::{CollectionError, SelectorSubject, collect_extensions};

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
    registry: &crate::registry::ExtensionRegistry,
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
