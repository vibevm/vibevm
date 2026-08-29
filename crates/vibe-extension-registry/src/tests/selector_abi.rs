//! The public selector/subject ABI surface: value type, read-only pattern
//! accessors and constructor routing — the evidence a canonical plan digest
//! and a typed widening compose without exposing compiled glob internals.

use specmark::verifies;
use vibe_core::manifest::ExtensionsControl;

use crate::{
    CompiledSelector, ExtensionRegistryRow, HostIdentity, SelectorProvider, SelectorSubject,
    collect_extensions,
};

use super::support::{host, provider_id, selected_declaration, world};

fn row<'registry>(
    registry: &'registry crate::ExtensionRegistry,
    suffix: &str,
) -> &'registry ExtensionRegistryRow {
    registry
        .rows()
        .iter()
        .find(|row| row.key().as_str().ends_with(suffix))
        .unwrap_or_else(|| panic!("selector test row with suffix `{suffix}` exists"))
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn pattern_accessors_retain_authored_members_while_projection_is_canonical() {
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![
                selected_declaration(
                    "raw",
                    Some(vec!["org.beta/tool", "org.alpha/*", "org.alpha/*"]),
                    Some(vec!["docs/**", "src/*.md"]),
                ),
                selected_declaration(
                    "reordered",
                    Some(vec!["org.alpha/*", "org.beta/tool"]),
                    Some(vec!["docs/**", "src/*.md"]),
                ),
                selected_declaration(
                    "changed",
                    Some(vec!["org.alpha/*", "org.gamma/kit"]),
                    Some(vec!["docs/**", "src/*.md"]),
                ),
                selected_declaration("empty-packages", Some(Vec::new()), None),
                selected_declaration("absent", None, None),
            ],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let selector = row(&registry, "#raw").compiled_selector();
    // Raw authored members: exact spellings, authored order, duplicates.
    assert_eq!(
        selector.package_patterns(),
        Some(
            [
                "org.beta/tool".to_owned(),
                "org.alpha/*".to_owned(),
                "org.alpha/*".to_owned(),
            ]
            .as_slice()
        )
    );
    assert_eq!(
        selector.path_patterns(),
        Some(["docs/**".to_owned(), "src/*.md".to_owned()].as_slice())
    );
    // Dimension absence stays distinct from an authored empty dimension.
    assert_eq!(
        row(&registry, "#absent")
            .compiled_selector()
            .package_patterns(),
        None
    );
    assert_eq!(
        row(&registry, "#absent")
            .compiled_selector()
            .path_patterns(),
        None
    );
    let authored_empty: Vec<String> = Vec::new();
    assert_eq!(
        row(&registry, "#empty-packages")
            .compiled_selector()
            .package_patterns(),
        Some(authored_empty.as_slice())
    );

    // The canonical projection a future plan digest takes: byte-sorted,
    // deduplicated — computed over the borrowed members, never stored, so
    // authored order stays non-semantic for identity consumers.
    let mut canonical = selector.package_patterns().unwrap().to_vec();
    canonical.sort();
    canonical.dedup();
    assert_eq!(
        canonical,
        ["org.alpha/*".to_owned(), "org.beta/tool".to_owned()]
    );

    // Matching never depends on authored order: the reordered spelling of
    // the same OR-set matches the same subjects…
    let alpha = provider_id("org.alpha", "one");
    let beta = provider_id("org.beta", "tool");
    let raw = row(&registry, "#raw").compiled_selector();
    let reordered = row(&registry, "#reordered").compiled_selector();
    for subject in [
        SelectorSubject::package(&alpha),
        SelectorSubject::package(&beta),
        SelectorSubject::unscoped(),
    ] {
        assert_eq!(raw.matches(subject), reordered.matches(subject));
    }

    // Canonical OR-set equality: the reordered + deduplicated spelling of
    // the same member set is the same selector value — one semantic
    // identity, so T2 digests both spellings to one plan…
    assert_eq!(raw, reordered);
    // …a changed member stays a different selector…
    assert_ne!(raw, row(&registry, "#changed").compiled_selector());
    // …and an absent dimension never equals a present empty one.
    assert_ne!(
        row(&registry, "#absent").compiled_selector(),
        row(&registry, "#empty-packages").compiled_selector()
    );
    // …and the OR-set matches its union, not any one member alone: with the
    // path dimension satisfied, every package member carries the row.
    assert!(raw.matches(SelectorSubject::new(Some(&alpha), Some("docs/x.md"))));
    assert!(raw.matches(SelectorSubject::new(Some(&beta), Some("docs/x.md"))));
    let outsider = provider_id("org.delta", "misc");
    assert!(!raw.matches(SelectorSubject::new(Some(&outsider), Some("docs/x.md"))));
    assert!(!raw.matches(SelectorSubject::unscoped()));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn compiled_selector_is_a_public_value_type_with_private_compilation() {
    // Compile-time proof of the public trait surface.
    fn assert_value_type<T: Clone + std::fmt::Debug + PartialEq + Eq>(_: &T) {}
    let registry = collect_extensions(world(
        Vec::new(),
        host(
            vec![selected_declaration(
                "value",
                Some(vec!["org.demo/*"]),
                None,
            )],
            ExtensionsControl::default(),
        ),
        None,
    ))
    .unwrap();
    let selector: &CompiledSelector = row(&registry, "#value").compiled_selector();
    assert_value_type(selector);
    let clone = selector.clone();
    assert_eq!(&clone, selector);
    assert!(format!("{clone:?}").contains("CompiledSelector"));
    // The public evaluation path accepts the typed widened subject.
    let demo = provider_id("org.demo", "tools");
    assert!(clone.matches(SelectorSubject::dependency(&demo)));
    assert!(!clone.matches(SelectorSubject::unscoped()));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn compatibility_constructors_and_provider_accessor_route_typed_providers() {
    let dependency = provider_id("org.demo", "tools");
    let identity = HostIdentity::ungrouped_project("demo");
    assert_eq!(SelectorSubject::unscoped().provider(), None);
    assert_eq!(SelectorSubject::path("docs/x.md").provider(), None);
    assert_eq!(
        SelectorSubject::package(&dependency).provider(),
        Some(SelectorProvider::Dependency(&dependency))
    );
    assert_eq!(
        SelectorSubject::dependency(&dependency).provider(),
        Some(SelectorProvider::Dependency(&dependency))
    );
    assert_eq!(
        SelectorSubject::new(Some(&dependency), Some("docs/x.md")).provider(),
        Some(SelectorProvider::Dependency(&dependency))
    );
    assert_eq!(
        SelectorSubject::host(&identity).provider(),
        Some(SelectorProvider::Host(&identity))
    );
    // The general typed constructor routes every spelling, composed or empty.
    assert_eq!(
        SelectorSubject::scoped(Some(SelectorProvider::Host(&identity)), Some("docs/x.md"))
            .provider(),
        Some(SelectorProvider::Host(&identity))
    );
    assert_eq!(
        SelectorSubject::scoped(None, Some("docs/x.md")).provider(),
        None
    );
    assert_eq!(SelectorSubject::scoped(None, None).provider(), None);
}
