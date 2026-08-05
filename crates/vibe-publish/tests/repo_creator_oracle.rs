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

/// Every adapter that *declares* a scope (`expected_org() == Some`) must
/// *enforce* it: a foreign org is refused with a `ScopeViolation` whose
/// fields name the host, the declared org, and the attempted org. This is
/// the runtime property the `ValidatedOrg` type **cannot** by itself
/// guarantee — the type only forces a `validate_scope` call before the
/// host methods run; it does not stop a future adapter from overriding
/// `validate_scope` to mint unconditionally (silently dropping the guard
/// while still type-checking, the exact latent hole this landing closes).
/// The table below is the authoritative list of scoped adapters: adding a
/// scoped adapter means adding a row, so a new host that declares a scope
/// but forgets to enforce it merges red here. Parameterised over
/// `&dyn RepoCreator` so a cell that diverges on the guard fails by name.
/// Sharper than [`api_adapters_share_the_scope_guard_contract`]: that test
/// pins the *kind* of the error, this one pins its *fields*.
#[test]
fn scoped_adapters_refuse_a_foreign_org() {
    let github = GithubRepoCreator::new(token(), "vibespecs").expect("github adapter constructs");
    let gitverse =
        GitverseRepoCreator::new(token(), "vibespecs").expect("gitverse adapter constructs");

    // Every adapter whose `expected_org()` is `Some` belongs here. The
    // direct adapter (`expected_org() == None`) is intentionally absent:
    // it declares no scope, so there is nothing to enforce.
    let cases: [(&dyn RepoCreator, &str); 2] =
        [(&github, "github.com"), (&gitverse, "gitverse.ru")];

    for (creator, host) in cases {
        assert_eq!(
            creator.expected_org(),
            Some("vibespecs"),
            "{host}: this table is for adapters that declare a scope"
        );
        let err = creator
            .validate_scope("someone-else")
            .expect_err("a scoped adapter must refuse a foreign org");
        match err {
            PublishError::ScopeViolation {
                host: h,
                expected_org,
                attempted_org,
            } => {
                assert_eq!(h, host, "{host}: the violation must name the host");
                assert_eq!(
                    expected_org, "vibespecs",
                    "{host}: the org the adapter was scoped to at construction"
                );
                assert_eq!(
                    attempted_org, "someone-else",
                    "{host}: the org that attempted the escalation"
                );
            }
            other => panic!("{host}: expected ScopeViolation, got {other:?}"),
        }
    }
}
