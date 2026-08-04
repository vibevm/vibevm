//! Seam-driving oracle for the `RepoCreator` cells (card
//! scaffold-d-differential-oracle; R-040). The three host adapters are
//! constructed and driven *through `&dyn RepoCreator`*, pinning the
//! seam-level contract every cell shares — host naming, the scope-guard
//! refusal, and the direct-vs-API branch — without a network call. A
//! replacement of any cell that diverges on these merges red here.

use vibe_publish::{
    DirectRepoCreator, GithubRepoCreator, GitverseRepoCreator, PublishError, RepoCreator, Token,
};

fn token() -> Token {
    Token::from_explicit("oracle-token-please-redact")
}

/// The two API adapters share one contract through the trait object:
/// each names its host, reports the org it was scoped to, takes the
/// regular (non-direct) flow, and refuses any org but its own with a
/// `ScopeViolation` — the PROP-000 §20 scope-discipline guard.
#[test]
fn api_adapters_share_the_scope_guard_contract() {
    let github = GithubRepoCreator::new(token(), "vibespecs").expect("github adapter constructs");
    let gitverse =
        GitverseRepoCreator::new(token(), "vibespecs").expect("gitverse adapter constructs");

    let cases: [(&dyn RepoCreator, &str); 2] =
        [(&github, "github.com"), (&gitverse, "gitverse.ru")];

    for (creator, host) in cases {
        assert_eq!(creator.host_name(), host);
        assert_eq!(creator.expected_org(), Some("vibespecs"));
        assert!(
            creator.direct_repo_url().is_none(),
            "{host}: an API adapter is not a direct-push adapter"
        );
        creator
            .validate_scope("vibespecs")
            .expect("the adapter's own org passes the guard");
        assert!(
            matches!(
                creator.validate_scope("someone-else"),
                Err(PublishError::ScopeViolation { .. })
            ),
            "{host}: an out-of-scope org must be refused"
        );
    }
}

/// The direct adapter takes the *other* branch of the same seam: no
/// host API, no org scope, and a `direct_repo_url` the publisher
/// short-circuits the whole create-repo dance on.
#[test]
fn direct_adapter_short_circuits_the_api_flow() {
    let direct = DirectRepoCreator::new("file:///tmp/local-bare.git");
    let creator: &dyn RepoCreator = &direct;

    assert_eq!(
        creator.direct_repo_url(),
        Some("file:///tmp/local-bare.git"),
        "the direct adapter exposes its URL for the short-circuit"
    );
    assert_eq!(creator.expected_org(), None);
    // No org scope → the guard trusts the caller for any org.
    creator
        .validate_scope("anything")
        .expect("a scope-free adapter trusts its caller");
}
