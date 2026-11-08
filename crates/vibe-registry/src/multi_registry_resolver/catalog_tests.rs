//! A selected row owns its successful source independently of alternatives.
use super::*;
fn local(name: &str, root: &Path) -> RegistrySection {
    RegistrySection {
        name: name.into(),
        url: format!("file://{}", root.display()),
        r#ref: "main".into(),
        naming: Default::default(),
        auth: None,
        token_env: None,
        enabled: true,
        index_url: Some("none".into()),
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
fn retained_row_survives_unrelated_missing_local_and_legacy_all_source_error() {
    let root = tempfile::tempdir().unwrap();
    let good = local("good", root.path());
    let bad = local("missing", &root.path().join("absent"));
    let resolver = MultiRegistryResolver::from_manifest(
        &[bad, good],
        &[],
        &[],
        root.path().join("cache"),
        Arc::new(ShellGit::new()),
        3600,
    )
    .unwrap();
    let first = resolver.selected_registry_source("good").unwrap();
    assert!(resolver.selected_registry_source("missing").is_err());
    assert!(resolver.sources().is_err());
    assert!(Arc::ptr_eq(
        &first,
        &resolver.selected_registry_source("good").unwrap()
    ));
    assert!(!root.path().join("cache").exists());
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
fn unused_invalid_auth_source_does_not_open_or_invalidate_selected_local() {
    let root = tempfile::tempdir().unwrap();
    let good = local("good", root.path());
    let mut bad = local("bad-auth", root.path());
    bad.url = "https://unused.invalid/registry".into();
    bad.auth = Some(vibe_core::manifest::AuthKind::None);
    bad.token_env = Some("MUST_NOT_READ_TOKEN".into());
    let resolver = MultiRegistryResolver::from_manifest(
        &[bad, good],
        &[],
        &[],
        root.path().join("cache"),
        Arc::new(ShellGit::new()),
        3600,
    )
    .unwrap();
    let selected = resolver.selected_registry_source("good").unwrap();
    assert!(resolver.selected_registry_source("bad-auth").is_err());
    assert!(Arc::ptr_eq(
        &selected,
        &resolver.selected_registry_source("good").unwrap()
    ));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
fn source_setters_invalidate_authority_but_progress_does_not() {
    let root = tempfile::tempdir().unwrap();
    let resolver = MultiRegistryResolver::new(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Arc::new(ShellGit::new()),
        root.path().join("cache"),
    );
    let owner = resolver.candidate_owner.clone();
    let resolver = resolver.with_progress(Default::default());
    assert!(Arc::ptr_eq(&owner, &resolver.candidate_owner));
    let resolver = resolver.with_store_root(root.path().join("store"));
    assert!(!Arc::ptr_eq(&owner, &resolver.candidate_owner));
    let owner = resolver.candidate_owner.clone();
    let resolver = resolver.with_metadata_store_root(root.path().join("metadata"));
    assert!(!Arc::ptr_eq(&owner, &resolver.candidate_owner));
    let owner = resolver.candidate_owner.clone();
    let resolver = resolver.with_strict_auth(true);
    assert!(!Arc::ptr_eq(&owner, &resolver.candidate_owner));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
fn changed_project_override_refuses_a_previously_retained_token_before_acquisition() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("org.test/item/v1.0.0");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.test'\nname='item'\nversion='1.0.0'\nkind='tool'\n",
    )
    .unwrap();
    let owner = MultiRegistryResolver::from_manifest(
        &[local("local", root.path())],
        &[],
        &[],
        root.path().join("cache"),
        Arc::new(ShellGit::new()),
        0,
    )
    .unwrap();
    let retained = owner
        .retain_default_candidate(
            &Group::parse("org.test").unwrap(),
            "item",
            &"1.0.0".parse().unwrap(),
        )
        .unwrap();
    assert!(
        retained
            .candidate
            .resolution_for(&owner.candidate_owner)
            .is_ok()
    );
    let changed = owner.with_project_overrides(vec![OverrideSection {
        pkgref: "org.test/item".into(),
        source_url: "ssh://unused.invalid/replacement".into(),
        r#ref: Some("refs/heads/other".into()),
        reason: None,
    }]);
    let store = root.path().join("must-not-write-store");
    let slot = root.path().join("must-not-write-slot");
    let error = changed
        .fetch_candidate_for_install(
            &retained.candidate,
            &store,
            None,
            &slot,
            &Default::default(),
        )
        .expect_err("stale source owner must refuse");
    assert!(
        matches!(error,RegistryError::MalformedMeta {reason,..} if reason.contains("another operation source owner"))
    );
    assert!(!store.exists());
    assert!(!slot.exists());
}

#[cfg(unix)]
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
fn deferred_override_conflicts_with_registry_pin_and_cannot_be_relabelled() {
    use crate::source_candidates::{
        CandidateInventoryScope, RetainedSourceCandidate, SourceInventoryLimits,
    };
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    use vibe_core::manifest::{CanonicalSourceBinding as Binding, RegistryPin, SourcePolicyError};
    struct Deferred {
        row: RegistrySection,
        replacement: OverrideSection,
    }
    impl RegistryCatalogProvider for Deferred {
        fn declarations(&self) -> Result<RegistryDeclarations, RegistryError> {
            Ok(RegistryDeclarations {
                registries: vec![self.row.clone()],
                overrides: vec![self.replacement.clone()],
                mirrors: Vec::new(),
                admission: None,
            })
        }
    }
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let repo = root.path().join("override");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        let result = Command::new("/usr/bin/git")
            .current_dir(&repo)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .args(args)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
    };
    git(&["init", "-b", "main"]);
    fs::write(
        repo.join("vibe.toml"),
        "[package]\ngroup='org.test'\nname='item'\nversion='1.0.0'\nkind='tool'\n",
    )
    .unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "fixture"]);
    let wrapper = root.path().join("git-fixture");
    let trace = root.path().join("git-trace");
    fs::write(&wrapper,format!("#!/bin/sh\nexport HOME='{}'\nexport GIT_CONFIG_NOSYSTEM=1\nexport GIT_CONFIG_GLOBAL=/dev/null\nexport GIT_TRACE='{}'\nexec /usr/bin/git -c protocol.allow=never -c protocol.file.allow=always \"$@\"\n",home.display(),trace.display())).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let row = local("named", &root.path().join("unused-registry"));
    let row_url = row.url.clone();
    let owner = MultiRegistryResolver::from_deferred_catalog(
        Arc::new(Deferred {
            row,
            replacement: OverrideSection {
                pkgref: "org.test/item".into(),
                source_url: format!("file://{}", repo.display()),
                r#ref: Some("refs/heads/main".into()),
                reason: None,
            },
        }),
        Arc::new(ShellGit::new().with_binary(wrapper)),
        root.path().join("cache"),
        0,
    );
    let group = Group::parse("org.test").unwrap();
    let pin = RegistryPin::new("named".into()).unwrap();
    assert!(matches!(
        owner.validate_registry_pin(&group, "item", &pin),
        Err(RegistryError::SourcePolicy(
            SourcePolicyError::DirectSourceConflict { .. }
        ))
    ));
    assert!(
        !trace.exists(),
        "pin conflict must precede Git or registry opening"
    );
    let retained = owner
        .retain_default_candidate(&group, "item", &"1.0.0".parse().unwrap())
        .unwrap()
        .candidate;
    assert!(matches!(retained.binding(), Binding::Override { .. }));
    let inventory = owner
        .candidate_inventory(
            &group,
            "item",
            &SourceInventoryLimits::default(),
            CandidateInventoryScope::AllAllowed,
        )
        .unwrap();
    assert!(
        inventory
            .candidates
            .iter()
            .all(|row| matches!(row.candidate.binding(), Binding::Override { .. }))
    );
    let resolution = retained
        .resolution_for(&owner.candidate_owner)
        .unwrap()
        .clone();
    let wrong = RetainedSourceCandidate::captured(
        &owner.candidate_owner,
        Binding::Registry {
            registry: pin,
            source_url: row_url.into(),
        },
        resolution,
        retained.manifest().clone(),
        retained.source_evidence().clone(),
    );
    assert!(
        matches!(wrong,Err(RegistryError::MalformedMeta {reason,..}) if reason.contains("resolved source kind"))
    );
    assert!(
        !fs::read_to_string(trace)
            .unwrap()
            .contains("unused-registry")
    );
}
