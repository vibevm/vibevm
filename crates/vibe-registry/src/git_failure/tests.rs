//! What the operator actually reads when a registry git operation
//! fails: the errand, the registry and its real url, the package, the
//! translated cause and fix, the allowed registries — and git's own
//! stderr below all of that, never in place of it.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
);

use std::sync::Arc;

use specmark::verifies;
use vibe_core::Group;
use vibe_core::manifest::RegistrySection;

use super::*;

/// The declared set a corporate project has: a site mirror served as
/// one repository, and the public registry it was supposed to replace.
fn corporate_rows() -> Vec<RegistrySection> {
    let declared = "registry = [\
        { name = 'corp', url = 'https://git.example.invalid/vibe', type = 'monorepo' },\
        { name = 'vibespecs', url = 'https://github.com/vibespecs' }]";
    toml::from_str::<toml::Value>(declared).unwrap()["registry"]
        .clone()
        .try_into()
        .unwrap()
}

fn roster() -> Arc<RegistryRoster> {
    Arc::new(RegistryRoster::from_rows(&corporate_rows()))
}

fn group() -> Group {
    Group::parse("org.example").unwrap()
}

/// The body the owner's corporate network produced.
const INTERCEPTED_302: &str = "error: RPC failed; HTTP 302 curl 22 The requested URL returned \
                               error: 302\nfatal: expected flush after ref listing";

fn redirect() -> GitError {
    GitError::HttpRedirected {
        // `git fetch origin` records no url of its own: the endpoint
        // comes from the funnel, which is the whole point.
        url: String::new(),
        status: 302,
        stderr: INTERCEPTED_302.to_string(),
    }
}

/// The failure as the CLI renders it — variant trailer and all.
fn rendered(operation: GitOperation, url: &str, error: GitError) -> String {
    rendered_for("vibespecs", operation, url, error)
}

fn rendered_for(registry: &str, operation: GitOperation, url: &str, error: GitError) -> String {
    explain(operation, registry, url, &roster(), error).to_string()
}

/// The remark requirement 6 asks for, as its own line — not the words
/// "public host" wherever they occur, which the standing redirect fix
/// also uses.
fn carries_public_host_remark(text: &str) -> bool {
    text.contains("note:     this attempt went to the public host")
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn the_failure_states_the_errand_registry_url_package_and_profile() {
    let text = rendered(
        GitOperation::fetch_package(&group(), "thing", "v26.13.0"),
        "https://github.com/vibespecs/org.example.thing.git",
        redirect(),
    );
    // 1 — the errand, in words, first.
    assert!(
        text.starts_with("fetching `org.example/thing` version `26.13.0` failed"),
        "{text}"
    );
    // 2 — the registry's name AND the endpoint actually contacted.
    assert!(text.contains("registry: vibespecs"), "{text}");
    assert!(
        text.contains("`https://github.com/vibespecs/org.example.thing.git`"),
        "{text}"
    );
    assert!(!text.contains("origin"), "{text}");
    // 3 — the package and the version.
    assert!(
        text.contains("package:  org.example/thing version 26.13.0"),
        "{text}"
    );
    // 4 — the cause, translated, with its fix.
    assert!(
        text.contains("the server redirected (HTTP 302) instead of answering the git request"),
        "{text}"
    );
    assert!(text.contains("exclude_hosting"), "{text}");
    assert!(text.contains("vibe dl --test-proxy"), "{text}");
    // 5 — the registries this profile allows.
    assert!(
        text.contains("allowed registries: corp (monorepo), vibespecs (github)"),
        "{text}"
    );
    // The variant's own spec/fix trailer is still there.
    assert!(text.contains("#failure-discriminator"), "{text}");
}

/// Requirement 7: git's own words survive, bounded and demoted — the
/// reader meets the diagnosis first and the evidence under it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#windows-ux")]
fn gits_own_stderr_tail_comes_last() {
    let text = rendered(
        GitOperation::list_versions(&group(), "thing"),
        "https://github.com/vibespecs/org.example.thing.git",
        redirect(),
    );
    let said = text.find("git said:").unwrap_or_else(|| panic!("{text}"));
    for above in [
        "cause:",
        "fix:",
        "allowed registries:",
        "#failure-discriminator",
    ] {
        assert!(text.find(above).unwrap() < said, "{above} below the tail");
    }
    assert!(
        text.contains("error: RPC failed; HTTP 302 curl 22"),
        "{text}"
    );
    assert!(
        text.contains("fatal: expected flush after ref listing"),
        "{text}"
    );
}

/// A long transfer's progress chatter does not bury the explanation.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#windows-ux")]
fn the_stderr_tail_is_bounded() {
    let noisy = (0..40)
        .map(|i| format!("remote: Counting objects: {i}%"))
        .collect::<Vec<_>>()
        .join("\n");
    let text = rendered(
        GitOperation::list_versions(&group(), "thing"),
        "https://github.com/vibespecs/org.example.thing.git",
        GitError::CommandFailed {
            cmd: "git ls-remote --tags".into(),
            status: 128,
            stderr: format!("{noisy}\nfatal: the end"),
        },
    );
    assert!(text.contains("earlier line(s) omitted"), "{text}");
    assert!(text.contains("fatal: the end"), "{text}");
    assert!(!text.contains("Counting objects: 0%"), "{text}");
    // An unclassified exit says so rather than reading as a vibe bug.
    assert!(text.contains("could not classify why"), "{text}");
}

/// Requirement 6: a public host while a site registry is configured is
/// named as such — the reader should not have to notice it themselves.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
fn a_public_host_beside_a_site_registry_is_called_out() {
    let public = rendered(
        GitOperation::list_versions(&group(), "thing"),
        "https://github.com/vibespecs/org.example.thing.git",
        redirect(),
    );
    assert!(carries_public_host_remark(&public), "{public}");
    assert!(
        public.contains("the public host `github.com`, although this project"),
        "{public}"
    );
    assert!(public.contains("site registry (`corp`)"), "{public}");

    let site = rendered_for(
        "corp",
        GitOperation::capture_mirror("corp"),
        "https://git.example.invalid/vibe",
        redirect(),
    );
    assert!(!carries_public_host_remark(&site), "{site}");
    assert!(site.contains("registry: corp"), "{site}");
    assert!(
        site.starts_with("capturing the mirror of registry `corp` failed"),
        "{site}"
    );
}

/// A credentialed url reaches the funnel on every authenticated fetch;
/// it must never reach the operator's terminal.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth")]
fn the_explanation_never_carries_a_credential() {
    let text = rendered(
        GitOperation::fetch_package(&group(), "thing", "v1.2.3"),
        "https://x-access-token:SECRET@github.com/vibespecs/org.example.thing.git",
        GitError::HttpRedirected {
            url: String::new(),
            status: 302,
            stderr: "fatal: unable to access \
                     'https://x-access-token:SECRET@github.com/vibespecs/org.example.thing.git/': \
                     The requested URL returned error: 302"
                .into(),
        },
    );
    assert!(!text.contains("SECRET"), "{text}");
    assert!(
        text.contains("`https://github.com/vibespecs/org.example.thing.git`"),
        "{text}"
    );
}

/// Every cause the classifier can produce has prose and a fix of its
/// own — a translation table with a hole in it is how `{other}` ends up
/// in front of an operator.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn every_classified_cause_translates_with_a_fix() {
    let url = "https://github.com/vibespecs/org.example.thing.git";
    let cases: Vec<(GitError, &str)> = vec![
        (redirect(), "the server redirected"),
        (
            GitError::TlsTrust {
                url: url.into(),
                detail: "SSL certificate problem: unable to get local issuer certificate".into(),
                stderr: "SSL certificate problem: unable to get local issuer certificate".into(),
            },
            "the TLS certificate chain was not trusted",
        ),
        (
            GitError::AuthFailed {
                url: url.into(),
                status: Some(403),
            },
            "refused the read (HTTP 403)",
        ),
        (
            GitError::RepoNotFound { url: url.into() },
            "no such repository",
        ),
        (
            GitError::RefNotFound {
                url: url.into(),
                refname: "v9.9.9".into(),
            },
            "no ref `v9.9.9`",
        ),
        (
            GitError::NetworkUnreachable { url: url.into() },
            "the host did not answer",
        ),
    ];
    for (error, expected) in cases {
        // `translate` is reached through the detail, so the table is
        // exercised exactly as the renderer uses it.
        let detail = GitFailureDetail {
            operation: GitOperation::list_versions(&group(), "thing"),
            registry: "vibespecs".into(),
            url: url.into(),
            roster: roster(),
            source: error,
        };
        let text = detail.explanation();
        assert!(text.contains(expected), "{text}");
        let fix = text.split("fix:      ").nth(1).unwrap_or_default();
        assert!(fix.len() > 20, "no fix clause in {text}");
    }
}

/// Only the unreadable failures are re-framed: the typed answers the
/// resolver walk routes on keep their identity, or resolution changes
/// with the prose.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn the_funnel_leaves_the_routed_classifications_alone() {
    let url = "https://github.com/vibespecs/org.example.thing.git";
    for routed in [
        GitError::AuthFailed {
            url: url.into(),
            status: Some(401),
        },
        GitError::RepoNotFound { url: url.into() },
        GitError::NetworkUnreachable { url: url.into() },
        GitError::ArchiveUnsupported { url: url.into() },
    ] {
        let explained = explain(
            GitOperation::list_versions(&group(), "thing"),
            "vibespecs",
            url,
            &roster(),
            routed,
        );
        assert!(
            matches!(explained, RegistryError::Git(_)),
            "{explained:?} was re-framed"
        );
    }
    assert!(matches!(
        explain(
            GitOperation::list_versions(&group(), "thing"),
            "vibespecs",
            url,
            &roster(),
            redirect(),
        ),
        RegistryError::GitOperation { .. }
    ));
}

/// A registry opened without a catalog has no declared set to report,
/// and says nothing rather than something made up.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn a_rosterless_registry_omits_the_profile_lines() {
    let text = explain(
        GitOperation::list_versions(&group(), "thing"),
        "direct",
        "https://github.com/vibespecs/org.example.thing.git",
        &Arc::new(RegistryRoster::default()),
        redirect(),
    )
    .to_string();
    assert!(!text.contains("allowed registries"), "{text}");
    assert!(!carries_public_host_remark(&text), "{text}");
    assert!(text.contains("the server redirected"), "{text}");
}

/// A ref that is not a version tag is reported as a ref.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
fn a_branch_is_never_reported_as_a_version() {
    let operation = GitOperation::fetch_package(&group(), "thing", "main");
    assert_eq!(
        operation.to_string(),
        "fetching `org.example/thing` at ref `main`"
    );
    assert_eq!(operation.version(), None);
}
