use super::super::test_support::*;
use super::*;
use crate::MultiRegistryResolver;
use crate::git_backend::{GitBackend, GitError};
use crate::source_policy_provider::SourcePolicyProvider;
use std::path::Path;
use std::sync::{Arc, Mutex};
use vibe_core::manifest::SourcePolicyDecl;

fn row(name: &str, url: &str) -> RegistrySection {
    let mut row: RegistrySection =
        toml::from_str(&format!("name='{name}'\nurl='{url}'")).expect("registry row");
    row.enabled = true;
    row
}

fn effective(exclusions: &str) -> EffectiveSourcePolicy {
    let site: SourcePolicyDecl = toml::from_str(exclusions).expect("source policy");
    let empty = SourcePolicyDecl::default();
    EffectiveSourcePolicy::merge(&site, &empty, &empty).expect("effective policy")
}

#[test]
fn a_registry_on_excluded_hosting_is_named_with_its_host_and_its_selector() {
    let policy = effective("exclude_hosting=['github','gitverse']");
    let disabled =
        disabled_registry(&policy, &row("vibespecs", "https://github.com/vibespecs")).unwrap();
    assert_eq!(disabled.name, "vibespecs");
    assert_eq!(disabled.url, "https://github.com/vibespecs");
    assert_eq!(disabled.host.as_deref(), Some("github.com"));
    assert_eq!(disabled.selector, "github");
    assert_eq!(
        disabled.skip_note(),
        "registry vibespecs (github.com) skipped: excluded by site policy"
    );
    assert_eq!(
        disabled.listing_mark(),
        "disabled by site policy (exclude_hosting) — not used [github]"
    );

    let scp = disabled_registry(&policy, &row("gv", "git@gitverse.ru:vibespecs")).unwrap();
    assert_eq!(scp.host.as_deref(), Some("gitverse.ru"));
    assert_eq!(scp.selector, "gitverse");
}

#[test]
fn the_site_monorepo_is_not_disabled_and_a_pathlike_url_has_no_host() {
    let policy = effective("exclude_hosting=['github','gitverse']");
    assert!(disabled_registry(&policy, &row("corp", "ssh://git@mono.invalid/corp.git")).is_none());
    assert!(disabled_registry(&policy, &row("drop", "file:///srv/corp/mono.git")).is_none());

    let local = effective("exclude_hosting=['host:MONO.invalid']");
    let disabled = disabled_registry(&local, &row("corp", "ssh://git@mono.invalid/corp.git"))
        .expect("an exact host selector disables the row it names");
    assert_eq!(disabled.host.as_deref(), Some("mono.invalid"));
    assert_eq!(disabled.selector, "host:mono.invalid");
}

/// 4b — declared precedence is untouched, but with the excluded rows absent
/// from the walk the first ALLOWED registry is the one that actually answers
/// first (PROP-002 `#registry-layers`, PROP-019 `#site-profile`).
#[test]
fn the_effective_primary_is_the_first_allowed_registry_in_declared_order() {
    let rows = vec![
        row("vibespecs", "https://github.com/vibespecs"),
        row("vibespecs-gitverse", "https://gitverse.ru/vibespecs"),
        row("corp", "ssh://git@mono.invalid/corp.git"),
    ];

    let neutral = effective("");
    assert_eq!(
        effective_primary(&neutral, &rows).map(|row| row.name.as_str()),
        Some("vibespecs"),
        "with no exclusions the declared primary is unchanged"
    );

    let site = effective("exclude_hosting=['github','gitverse']");
    assert_eq!(
        effective_primary(&site, &rows).map(|row| row.name.as_str()),
        Some("corp")
    );

    let mut off = rows.clone();
    off[2].enabled = false;
    assert!(
        effective_primary(&site, &off).is_none(),
        "every remaining row is disabled, by policy or by hand"
    );
}

/// A git backend that refuses to be the one that broke the promise: any call
/// naming an excluded host panics the test outright. "Never contacted" is
/// then a property of the run, not of the prose in the output.
struct NeverContact {
    inner: Arc<FakeBackend>,
    forbidden: Vec<String>,
    reached: Mutex<Vec<String>>,
}

impl NeverContact {
    fn new(inner: Arc<FakeBackend>, forbidden: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            inner,
            forbidden: forbidden.iter().map(|host| (*host).to_owned()).collect(),
            reached: Mutex::new(Vec::new()),
        })
    }
    fn gate(&self, url: &str) {
        self.reached
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(url.to_owned());
        assert!(
            !self.forbidden.iter().any(|host| url.contains(host)),
            "the excluded host was contacted at `{url}`"
        );
    }
    fn reached(&self) -> Vec<String> {
        self.reached
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl GitBackend for NeverContact {
    fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
        self.gate(url);
        self.inner.bootstrap(url, refname, dest)
    }
    fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
        self.inner.update(dest, refname)
    }
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        self.gate(url);
        self.inner.list_tags(url)
    }
    fn fetch_file_at_ref(&self, url: &str, refname: &str, path: &str) -> Result<Vec<u8>, GitError> {
        self.gate(url);
        self.inner.fetch_file_at_ref(url, refname, path)
    }
    fn recorded_origin_url(&self, dest: &Path) -> Result<String, GitError> {
        self.inner.recorded_origin_url(dest)
    }
}

struct SitePolicy(Arc<EffectiveSourcePolicy>);

impl SourcePolicyProvider for SitePolicy {
    fn source_choice_mode(&self) -> crate::source_candidates::SourceChoiceMode {
        crate::source_candidates::SourceChoiceMode::LegacyKnown
    }
    fn policy(&self) -> Result<Arc<EffectiveSourcePolicy>, crate::RegistryError> {
        Ok(self.0.clone())
    }
    fn admission(&self) -> Result<Arc<crate::HttpAdmission>, crate::RegistryError> {
        Ok(Arc::new(crate::HttpAdmission::new(
            self.0.denied_host_keys(),
        )?))
    }
}

/// The owner's shape: two user-scope registries on excluded hosting, declared
/// first, and the site's own registry last. Resolution answers from the site
/// registry and the excluded hosts are never reached.
#[test]
fn resolution_walks_past_the_excluded_registries_and_never_opens_them() {
    let cache = tempfile::tempdir().unwrap();
    let inner = Arc::new(FakeBackend::default());
    inner.seed_tags(
        "git@mono.invalid:corp/org.vibevm.node.git",
        vec!["v1.2.0".to_string()],
    );
    inner.seed_file(
        "git@mono.invalid:corp/org.vibevm.node.git",
        "v1.2.0",
        "vibe.toml",
        manifest_text("node", "runtime", "1.2.0").into_bytes(),
    );
    let backend = NeverContact::new(inner, &["github.com", "gitverse.ru"]);

    let rows = vec![
        registry_section("vibespecs", "https://github.com/vibespecs"),
        registry_section("vibespecs-gitverse", "https://gitverse.ru/vibespecs"),
        registry_section("corp", "git@mono.invalid:corp"),
    ];
    let resolver = MultiRegistryResolver::from_manifest(
        &rows,
        &[],
        &[],
        cache.path().to_path_buf(),
        backend.clone(),
        crate::registry_cache::DEFAULT_FRESHNESS_SECS,
    )
    .unwrap()
    .with_source_policy_provider(Arc::new(SitePolicy(Arc::new(effective(
        "exclude_hosting=['github','gitverse']",
    )))));

    // The excluded rows stay declared…
    assert_eq!(resolver.declared_registry_rows().unwrap().len(), 3);
    // …and are absent from the walk list.
    let walked: Vec<&str> = resolver
        .sources()
        .unwrap()
        .iter()
        .map(|source| source.name())
        .collect();
    assert_eq!(walked, vec!["corp"]);
    let disabled: Vec<&str> = resolver
        .disabled_registries()
        .unwrap()
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    assert_eq!(disabled, vec!["vibespecs", "vibespecs-gitverse"]);

    let versions = resolver.list_versions(&org(), "node").unwrap();
    assert_eq!(versions, vec![semver::Version::new(1, 2, 0)]);
    let reached = backend.reached();
    assert!(
        reached
            .iter()
            .all(|url| !url.contains("github.com") && !url.contains("gitverse.ru")),
        "{reached:?}"
    );
    assert!(!reached.is_empty(), "the allowed registry was still asked");

    // A package no admitted registry holds: the report names both halves —
    // the registry that was asked and the two that were not.
    let missing = vibe_core::PackageRef::new(
        None,
        Some(org()),
        "absent".to_string(),
        vibe_core::VersionSpec::Latest,
    )
    .unwrap();
    let Err(error) = resolver.resolve(&missing) else {
        panic!("no registry holds this package");
    };
    let rendered = error.to_string();
    assert!(
        rendered.contains(
            "vibespecs           (https://github.com/vibespecs )  \
             → disabled by site policy (exclude_hosting) — not used"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("Allowed registries: corp\n"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "Disabled by the effective source policy (`exclude_hosting`), left configured \
             and never contacted: vibespecs, vibespecs-gitverse"
        ),
        "{rendered}"
    );

    // Naming an excluded registry explicitly is refused by name, not by a
    // contact that was never made.
    let Err(refused) = resolver.selected_registry_source("vibespecs") else {
        panic!("naming a disabled registry must be refused");
    };
    assert!(
        matches!(
            &refused,
            crate::RegistryError::RegistryDisabledByPolicy { registry, selector }
                if registry == "vibespecs" && selector == "github"
        ),
        "{refused}"
    );
}

/// When the policy disables every configured registry, the failure names both
/// halves of the split rather than dying on one excluded endpoint.
#[test]
fn every_registry_excluded_errors_and_names_allowed_and_skipped_registries() {
    let cache = tempfile::tempdir().unwrap();
    let backend = NeverContact::new(
        Arc::new(FakeBackend::default()),
        &["github.com", "gitverse.ru"],
    );
    let rows = vec![
        registry_section("vibespecs", "https://github.com/vibespecs"),
        registry_section("vibespecs-gitverse", "https://gitverse.ru/vibespecs"),
    ];
    let resolver = MultiRegistryResolver::from_manifest(
        &rows,
        &[],
        &[],
        cache.path().to_path_buf(),
        backend.clone(),
        crate::registry_cache::DEFAULT_FRESHNESS_SECS,
    )
    .unwrap()
    .with_source_policy_provider(Arc::new(SitePolicy(Arc::new(effective(
        "exclude_hosting=['github','gitverse']",
    )))));

    let pkgref = vibe_core::PackageRef::new(
        None,
        Some(org()),
        "node".to_string(),
        vibe_core::VersionSpec::Latest,
    )
    .unwrap();
    let Err(error) = resolver.resolve(&pkgref) else {
        panic!("every registry is disabled, so nothing can resolve");
    };
    // Nothing admitted was asked, so the refusal is the exclusion itself,
    // not a package report nobody gathered.
    assert!(
        matches!(
            &error,
            crate::RegistryError::SourcePolicy(
                vibe_core::manifest::SourcePolicyError::ExcludedOnly { .. }
            )
        ),
        "{error}"
    );
    let rendered = error.to_string();
    assert!(rendered.starts_with("source_host_excluded: "), "{rendered}");
    assert!(
        rendered.contains(
            "allowed registries: (none — the source policy disables every configured registry)"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("https://github.com/vibespecs"),
        "{rendered}"
    );
    assert!(
        rendered.contains("https://gitverse.ru/vibespecs"),
        "{rendered}"
    );
    assert!(rendered.contains("were not contacted"), "{rendered}");
    assert!(
        backend.reached().is_empty(),
        "nothing was contacted: {:?}",
        backend.reached()
    );
}
