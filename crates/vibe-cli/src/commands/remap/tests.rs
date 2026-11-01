//! The two documents and the layout they render (`PROP-066#display`).

use super::*;
use specmark::verifies;
use vibe_core::remap::{LoadedSource, Provenance, RowDecl};

fn row(body: &str, layer: Layer, source: &str, index: usize) -> RemapRow {
    toml::from_str::<RowDecl>(body)
        .expect("row parses")
        .into_row(Provenance::new(layer, source, index))
        .expect("row is well shaped")
}

/// Two sources, one of them the invocation, with a scoped row, a deny row
/// and a local path — enough for every label of the layout to appear.
fn locator() -> Locator {
    Locator::from_sources(
        vec![
            LoadedSource {
                layer: Layer::Local,
                source: "~/.vibe/settings.toml".into(),
                rows: vec![
                    row(
                        "prefix = \"https://github.com/\"\n\
                         url = \"https://10.1.0.5/gh/\"\n\
                         reason = \"the only host the proxy allows\"",
                        Layer::Local,
                        "~/.vibe/settings.toml",
                        0,
                    ),
                    row(
                        "sha256 = \"ab\"\npath = \"/srv/artifacts\"\n\
                         [scope]\npackage = \"org.example/tool\"\nstep = \"fetch\"",
                        Layer::Local,
                        "~/.vibe/settings.toml",
                        1,
                    ),
                ],
                policy: UnmatchedPolicy::Allow,
            },
            LoadedSource {
                layer: Layer::Invocation,
                source: "closed.toml".into(),
                rows: vec![row(
                    "registry = \"vibespecs\"\ndeny = true\nreason = \"policy: one store only\"",
                    Layer::Invocation,
                    "closed.toml",
                    0,
                )],
                policy: UnmatchedPolicy::Deny,
            },
        ],
        vec![("vibespecs".into(), "https://github.com/vibespecs".into())],
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#display")]
fn the_listing_carries_one_block_per_source_and_the_per_host_footer() {
    let document = listing(&locator(), None, true);
    assert_eq!(document.protocol, "vibe-remap/1");
    // The strictest policy of the layers is the document's.
    assert_eq!(document.policy, wire::UnmatchedPolicy::Deny);
    assert_eq!(document.sources.len(), 2);
    assert_eq!(document.sources[0].source, "~/.vibe/settings.toml");
    assert_eq!(document.sources[0].rows.len(), 2);
    assert_eq!(document.sources[1].layer, wire::Layer::Invocation);
    assert_eq!(document.denies, 1);

    let first = &document.sources[0].rows[0];
    assert_eq!(
        (first.number, first.kind.clone()),
        (1, wire::KeyKind::Prefix)
    );
    assert_eq!(first.scheme, "https");
    assert_eq!(first.reach, wire::Reach::Private);
    assert_eq!(first.host.as_deref(), Some("10.1.0.5"));
    assert!(first.notes.is_empty(), "a private prefix is not broad");

    let scoped = &document.sources[0].rows[1];
    assert_eq!(scoped.reach, wire::Reach::Local);
    assert_eq!(
        scoped.scope.as_ref().unwrap().step.as_deref(),
        Some("fetch")
    );

    // The footer counts the two reachable targets and reports the deny
    // separately; a `deny` row points nowhere.
    assert_eq!(document.hosts.len(), 2);
    assert_eq!(document.hosts[0].host, "10.1.0.5");
    assert_eq!(document.hosts[1].host, "local path");

    let text = render::listing(&document);
    assert!(text.contains("~/.vibe/settings.toml  [local, unmatched = allow]"));
    assert!(text.contains("1. prefix    https://github.com/"));
    assert!(text.contains("to       https://10.1.0.5/gh/  [https, private]"));
    assert!(text.contains("why      the only host the proxy allows"));
    assert!(text.contains("scope    org.example/tool, step fetch"));
    assert!(text.contains("1 row → 10.1.0.5 (https, private); 1 row → local path; 1 deny"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn explain_lists_two_competing_rows_in_winning_order() {
    let built = Locator::from_sources(
        vec![LoadedSource {
            layer: Layer::Local,
            source: "~/.vibe/settings.toml".into(),
            rows: vec![
                row(
                    "prefix = \"https://github.com/\"\n\
                     url = \"https://wide.example/\"\nreason = \"everything github\"",
                    Layer::Local,
                    "~/.vibe/settings.toml",
                    0,
                ),
                row(
                    "prefix = \"https://github.com/vibespecs/\"\n\
                     url = \"https://narrow.example/\"\nreason = \"just the registry\"",
                    Layer::Local,
                    "~/.vibe/settings.toml",
                    1,
                ),
            ],
            policy: UnmatchedPolicy::Allow,
        }],
        Vec::new(),
    );
    let artifact =
        Artifact::new(ArtifactKind::RegistryClone).with_url("https://github.com/vibespecs/index");
    let document = explanation(&built, &artifact, true);
    assert_eq!(document.matches.len(), 2);
    assert_eq!(
        document.matches[0].why.as_deref(),
        Some("just the registry")
    );
    assert_eq!(
        document.matches[1].why.as_deref(),
        Some("everything github")
    );
    assert_eq!(
        document.chosen.as_deref(),
        Some("https://narrow.example/index")
    );
    assert!(!document.denied);

    let text = render::explain(&document);
    assert!(text.contains("origin   https://github.com/vibespecs/index"));
    assert!(text.contains("1. prefix    https://github.com/vibespecs/"));
    assert!(text.contains("2. prefix    https://github.com/"));
    assert!(text.contains("chosen  https://narrow.example/index"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#row")]
fn explain_reports_a_refusal_and_names_no_chosen_target() {
    let artifact =
        Artifact::new(ArtifactKind::RegistryClone).with_url("https://github.com/vibespecs/index");
    let document = explanation(&locator(), &artifact, true);
    assert!(document.denied);
    assert!(document.chosen.is_none());
    assert!(render::explain(&document).contains("remap_denied"));
}

#[test]
fn a_digest_question_is_asked_about_the_digest_and_a_url_about_the_url() {
    let ask = |what: &str| RemapExplainArgs {
        what: what.into(),
        package: None,
        step: None,
        json: false,
    };
    let digest = "AB".repeat(32);
    let by_digest = asked_about(&ask(&format!("sha256:{digest}")));
    assert_eq!(by_digest.digest.as_deref(), Some("ab".repeat(32).as_str()));
    assert!(by_digest.url.is_none());

    // A bare 64-hex spelling is the same question.
    let bare = asked_about(&ask(&digest));
    assert_eq!(bare.digest, by_digest.digest);

    let by_url = asked_about(&ask("https://host/a"));
    assert_eq!(by_url.url.as_deref(), Some("https://host/a"));
    assert!(by_url.digest.is_none());
}

#[test]
fn an_empty_locator_lists_nothing_and_says_so() {
    let document = listing(&Locator::default(), None, true);
    assert!(document.sources.is_empty() && document.hosts.is_empty());
    assert_eq!(document.denies, 0);
    assert!(render::listing(&document).contains("no remap rows are active"));
}
