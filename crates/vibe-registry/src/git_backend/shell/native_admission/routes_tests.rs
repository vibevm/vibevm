//! Actual admitted route composition on numeric loopback TLS fixtures.
#![cfg(all(test, unix))]
use super::*;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn denied_canonical_route_does_not_block_an_independently_admitted_mirror() {
    let f = Fixture::new();
    let temporary = tempfile::tempdir().unwrap();
    let selection = NativeConfigSelection::new(
        temporary.path().into(),
        Path::new(f.get("root")).join("home"),
    )
    .unwrap()
    .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
    .unwrap()
    .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
    .unwrap()
    .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
    .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(std::fs::read(f.get("ca")).unwrap()),
    )
    .with_config_selection(selection);
    let primary = format!("{}/public", f.get("b"));
    let mirror = format!("{}/public", f.get("a"));
    let registry: vibe_core::manifest::RegistrySection =
        toml::from_str(&format!("name='fixture'\nurl='{primary}'\nauth='none'\n")).unwrap();
    let alternative: vibe_core::manifest::MirrorSection =
        toml::from_str(&format!("of='fixture'\nurl='{mirror}'\npriority=0\n")).unwrap();
    let resolver = crate::MultiRegistryResolver::from_manifest_with_native_admission(
        &[registry],
        &[alternative],
        &[],
        temporary.path().join("registry"),
        0,
        Arc::new(profile),
    )
    .unwrap();
    let resolved = resolver
        .resolve(&vibe_core::PackageRef::parse("org.fixture/demo@=1.0.0").unwrap())
        .unwrap();
    let cached = resolver
        .fetch(&resolved, &temporary.path().join("store"))
        .unwrap();
    assert_eq!(cached.resolved_commit.as_deref(), Some(f.get("head")));
    assert_eq!(
        std::fs::read(cached.cache_dir.join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert_eq!(cached.source_uri, format!("{primary}/org.fixture.demo.git"));
    assert_eq!(
        cached.fetched_from.as_deref(),
        Some(format!("{mirror}/org.fixture.demo.git").as_str())
    );
    assert_eq!(f.contacts("b"), 0);
    assert_eq!(f.connections("b"), 0);
    assert!(f.contacts("a") > 0);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn package_redirect_uses_its_bound_target_and_exact_pinned_ref() {
    for denied in [false, true] {
        let f = Fixture::new();
        let temporary = tempfile::tempdir().unwrap();
        let selection = NativeConfigSelection::new(
            temporary.path().into(),
            Path::new(f.get("root")).join("home"),
        )
        .unwrap()
        .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
        .unwrap()
        .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
        .unwrap()
        .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
        .unwrap();
        let profile = NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::Pem(std::fs::read(f.get("ca")).unwrap()),
        )
        .with_config_selection(selection);
        let work = temporary.path().join("stub");
        std::fs::create_dir(&work).unwrap();
        assert!(fixture_git(&f.info, &["init"], &work).status.success());
        let target = format!("{}/repo.git", f.get(if denied { "b" } else { "a" }));
        std::fs::write(work.join("vibe-redirect.toml"),format!("[redirect]\ntarget_url='{target}'\nref_policy='pinned'\npinned_ref='{}'\nauth='none'\n",f.get("head"))).unwrap();
        assert!(fixture_git(&f.info, &["add", "."], &work).status.success());
        assert!(
            fixture_git(
                &f.info,
                &[
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=f@example.invalid",
                    "commit",
                    "-m",
                    "stub"
                ],
                &work
            )
            .status
            .success()
        );
        assert!(
            fixture_git(&f.info, &["tag", "v1.0.0"], &work)
                .status
                .success()
        );
        let bare = Path::new(f.get("root")).join("web/stubs/org.fixture.demo.git");
        std::fs::create_dir_all(bare.parent().unwrap()).unwrap();
        assert!(
            fixture_git(
                &f.info,
                &[
                    "clone",
                    "--bare",
                    work.to_str().unwrap(),
                    bare.to_str().unwrap()
                ],
                temporary.path()
            )
            .status
            .success()
        );
        assert!(
            fixture_git(&f.info, &["update-server-info"], &bare)
                .status
                .success()
        );
        let registry = toml::from_str(&format!(
            "name='stub'\nurl='{}/stubs'\nindex_url='none'\nauth='none'\n",
            f.get("a")
        ))
        .unwrap();
        let resolver = crate::MultiRegistryResolver::from_manifest_with_native_admission(
            &[registry],
            &[],
            &[],
            temporary.path().join("cache"),
            0,
            Arc::new(profile),
        )
        .unwrap();
        let result =
            resolver.resolve(&vibe_core::PackageRef::parse("org.fixture/demo@=1.0.0").unwrap());
        if denied {
            assert!(matches!(
                result,
                Err(crate::RegistryError::ContactRefused(
                    crate::ContactRefusal::ExcludedHost { .. }
                ))
            ));
            assert_eq!(f.contacts("b"), 0);
            assert_eq!(f.connections("b"), 0);
            continue;
        }
        let resolved = result.unwrap();
        assert_eq!(resolved.source_ref.as_deref(), Some(f.get("head")));
        let cached = resolver
            .fetch(&resolved, &temporary.path().join("store"))
            .unwrap();
        assert_eq!(cached.resolved_commit.as_deref(), Some(f.get("head")));
        assert_eq!(cached.source_ref.as_deref(), Some(f.get("head")));
        assert_eq!(cached.source_uri, target);
        assert_eq!(
            cached.via_redirect.as_deref(),
            Some(format!("{}/stubs/org.fixture.demo.git", f.get("a")).as_str())
        );
        assert_eq!(
            std::fs::read(cached.cache_dir.join("payload.txt")).unwrap(),
            b"retained fixture\n"
        );
        assert_eq!(f.contacts("b"), 0);
        assert_eq!(f.connections("b"), 0);
        assert!(cached.resolved.source_dir.join(".git").is_dir());
        assert!(
            fixture_git(
                &f.info,
                &[
                    "remote",
                    "set-url",
                    "origin",
                    &format!("{}/other.git", f.get("b"))
                ],
                &cached.resolved.source_dir
            )
            .status
            .success()
        );
        let repeated = resolver
            .fetch(&resolved, &temporary.path().join("store"))
            .unwrap();
        assert_eq!(repeated.resolved_commit.as_deref(), Some(f.get("head")));
        assert_eq!(f.contacts("b"), 0);
        assert_eq!(f.connections("b"), 0);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn public_stub_selects_configured_private_grant_but_cannot_mint_one() {
    let f = Fixture::new();
    let temporary = tempfile::tempdir().unwrap();
    let work = temporary.path().join("stub");
    std::fs::create_dir(&work).unwrap();
    assert!(fixture_git(&f.info, &["init"], &work).status.success());
    std::fs::write(work.join("vibe-redirect.toml"),format!("[redirect]\ntarget_url='{}/auth/org.fixture.demo.git'\nref_policy='pinned'\npinned_ref='{}'\nauth='token-env'\ntoken_env='VIBE_N4_MARKER_TOKEN'\n",f.get("b"),f.get("head"))).unwrap();
    assert!(fixture_git(&f.info, &["add", "."], &work).status.success());
    assert!(
        fixture_git(
            &f.info,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=f@example.invalid",
                "commit",
                "-m",
                "stub"
            ],
            &work
        )
        .status
        .success()
    );
    assert!(
        fixture_git(&f.info, &["tag", "v1.0.0"], &work)
            .status
            .success()
    );
    let bare = Path::new(f.get("root")).join("web/private-stubs/org.fixture.demo.git");
    std::fs::create_dir_all(bare.parent().unwrap()).unwrap();
    assert!(
        fixture_git(
            &f.info,
            &[
                "clone",
                "--bare",
                work.to_str().unwrap(),
                bare.to_str().unwrap()
            ],
            temporary.path()
        )
        .status
        .success()
    );
    assert!(
        fixture_git(&f.info, &["update-server-info"], &bare)
            .status
            .success()
    );
    for configured in [true, false] {
        std::fs::write(Path::new(f.get("root")).join("b.log"), "").unwrap();
        std::fs::write(Path::new(f.get("root")).join("b.tcp.log"), "").unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", Path::new(f.get("root")).join("home"))
            .env("VIBE_N4_INFO", serde_json::to_string(&f.info).unwrap())
            .env("VIBE_N4_CONFIGURED", if configured { "yes" } else { "no" })
            .env("VIBE_N4_TARGET_TOKEN", "SYNTHETIC_N1_AUTH")
            .env(
                "VIBE_N4_MARKER_TOKEN",
                if configured {
                    "WRONG_SYNTHETIC_TOKEN"
                } else {
                    "SYNTHETIC_N1_AUTH"
                },
            )
            .args([
                "--ignored",
                "--exact",
                "git_backend::shell::native_admission::tests::routes_tests::private_redirect_child",
                "--nocapture",
            ])
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{} {}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
        assert_eq!(f.contacts("b") > 0, configured);
        assert_eq!(f.connections("b") > 0, configured);
    }
}

#[test]
#[ignore = "isolated executable fixture launched by the public redirect test"]
fn private_redirect_child() {
    let info: serde_json::Value =
        serde_json::from_str(&std::env::var("VIBE_N4_INFO").unwrap()).unwrap();
    let configured = std::env::var("VIBE_N4_CONFIGURED").unwrap() == "yes";
    let temporary = tempfile::tempdir().unwrap();
    let selection = NativeConfigSelection::new(
        temporary.path().into(),
        Path::new(info["root"].as_str().unwrap()).join("home"),
    )
    .unwrap()
    .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
    .unwrap()
    .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
    .unwrap()
    .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
    .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(crate::HttpAdmission::new(Vec::<String>::new()).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(std::fs::read(info["ca"].as_str().unwrap()).unwrap()),
    )
    .with_config_selection(selection);
    let stub = toml::from_str(&format!(
        "name='public-stub'\nurl='{}/private-stubs'\nauth='none'\nindex_url='none'\n",
        info["a"].as_str().unwrap()
    ))
    .unwrap();
    let mut registries = vec![stub];
    if configured {
        registries.push(toml::from_str(&format!("name='private-target'\nurl='{}/auth'\nauth='token-env'\ntoken_env='VIBE_N4_TARGET_TOKEN'\nindex_url='none'\n",info["b"].as_str().unwrap())).unwrap());
    }
    let resolver = crate::MultiRegistryResolver::from_manifest_with_native_admission(
        &registries,
        &[],
        &[],
        temporary.path().join("cache"),
        0,
        Arc::new(profile),
    )
    .unwrap();
    let result =
        resolver.resolve(&vibe_core::PackageRef::parse("org.fixture/demo@=1.0.0").unwrap());
    if !configured {
        assert!(
            matches!(result,Err(crate::RegistryError::Git(GitError::NativeContext{reason})) if reason.contains("independently configured"))
        );
        return;
    }
    let resolved = result.unwrap();
    assert!(format!("{:?}", resolved.selected_route).contains("private-target"));
    let cached = resolver
        .fetch(&resolved, &temporary.path().join("store"))
        .unwrap();
    assert_eq!(cached.registry_name.as_deref(), Some("public-stub"));
    assert_eq!(cached.resolved_commit.as_deref(), info["head"].as_str());
    assert_eq!(
        std::fs::read(cached.cache_dir.join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert!(
        !serde_json::to_string(&cached.source_evidence())
            .unwrap()
            .contains("SYNTHETIC")
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn retained_mirror_is_not_replaced_by_a_warm_excluded_origin() {
    let f = Fixture::new();
    let temporary = tempfile::tempdir().unwrap();
    let ssh = super::super::ssh_tests::Fixture::new();
    let ssh_profile = SshProfile::explicit_direct(
        "/usr/bin/ssh".into(),
        "127.0.0.1".into(),
        ssh.get("user").into(),
        ssh.port(),
        std::fs::read(ssh.get("known_hosts")).unwrap(),
        SshAuthentication::Key(std::fs::read(ssh.get("key")).unwrap()),
    )
    .unwrap();
    let selection = NativeConfigSelection::new(
        temporary.path().into(),
        Path::new(f.get("root")).join("home"),
    )
    .unwrap()
    .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
    .unwrap()
    .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
    .unwrap()
    .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
    .unwrap();
    let profile = Arc::new(
        NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(["127.0.0.2"]).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::Pem(std::fs::read(f.get("ca")).unwrap()),
        )
        .with_config_selection(selection)
        .with_ssh_profile(ssh_profile),
    );
    let direct = ShellGit::new().with_native_admission(Arc::clone(&profile));
    let old_ssh = temporary.path().join("old-ssh");
    let old_objects = temporary.path().join("old-ssh.git");
    let old_revision = direct
        .prepare_ref(ssh.get("url"), "v1.0.0", None, &old_objects, &old_ssh)
        .unwrap();
    let old_stage = direct
        .stage_native_materialisation(
            &old_objects,
            &old_revision,
            crate::git_backend::PayloadBytes::AsConfigured,
        )
        .unwrap()
        .unwrap();
    std::fs::rename(old_stage.staged_path(), &old_ssh).unwrap();
    assert!(
        fixture_git(
            &f.info,
            &[
                "remote",
                "set-url",
                "origin",
                &ssh.get("url").replace("127.0.0.1", "127.0.0.2")
            ],
            &old_ssh
        )
        .status
        .success()
    );
    let ssh_contacts = (ssh.contacts("a"), ssh.contacts("b"));
    let primary = format!("{}/public", f.get("b"));
    let mirror = format!("{}/public", f.get("a"));
    let registry =
        toml::from_str(&format!("name='fixture'\nurl='{primary}'\nauth='none'\n")).unwrap();
    let alternative =
        toml::from_str(&format!("of='fixture'\nurl='{mirror}'\npriority=0\n")).unwrap();
    let resolver = crate::MultiRegistryResolver::from_manifest_with_native_admission(
        &[registry],
        &[alternative],
        &[],
        temporary.path().join("cache"),
        0,
        profile,
    )
    .unwrap();
    let client = crate::http_admission::bound_builder(reqwest::blocking::Client::builder())
        .tls_built_in_root_certs(false)
        .add_root_certificate(
            reqwest::Certificate::from_pem(&std::fs::read(f.get("ca")).unwrap()).unwrap(),
        )
        .build()
        .unwrap();
    client
        .get(format!(
            "{mirror}/org.fixture.demo.git/info/refs?service=git-upload-pack"
        ))
        .send()
        .expect("isolated discovery TLS control");
    let reference = vibe_core::PackageRef::parse("org.fixture/demo@=1.0.0").unwrap();
    let resolved = resolver.resolve(&reference).unwrap();
    let cached = resolver
        .fetch(&resolved, &temporary.path().join("store"))
        .unwrap();
    let cache = &cached.resolved.source_dir;
    assert!(
        fixture_git(
            &f.info,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("{primary}/org.fixture.demo.git")
            ],
            cache
        )
        .status
        .success()
    );
    let refreshed = resolver
        .fetch(&resolved, &temporary.path().join("store"))
        .unwrap();
    assert_eq!(
        refreshed.fetched_from.as_deref(),
        Some(format!("{mirror}/org.fixture.demo.git").as_str())
    );
    let warm = Path::new(f.get("work"));
    assert!(
        fixture_git(
            &f.info,
            &[
                "remote",
                "add",
                "origin",
                &format!("{primary}/org.fixture.demo.git")
            ],
            warm
        )
        .status
        .success()
    );
    let before = std::fs::read(warm.join(".git/config")).unwrap();
    let prepared = resolver.prepare_in_place(&resolved, warm).unwrap();
    assert_eq!(prepared.resolved_commit(), f.get("head"));
    assert_eq!(
        prepared.source_evidence().fetched_from.as_deref(),
        Some(format!("{mirror}/org.fixture.demo.git").as_str())
    );
    prepared
        .stage_native(crate::git_backend::PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap()
        .validate_before_publication()
        .unwrap();
    assert_eq!(std::fs::read(warm.join(".git/config")).unwrap(), before);
    assert_eq!(f.contacts("b"), 0);
    assert_eq!(f.connections("b"), 0);
    let contacts = f.contacts("a");
    let mut stale = resolved.clone();
    stale.resolved.version = semver::Version::new(9, 0, 0);
    assert!(
        resolver
            .fetch(&stale, &temporary.path().join("store"))
            .is_err()
    );
    assert_eq!(f.contacts("a"), contacts);
    let prepared = resolver.prepare_in_place(&resolved, &old_ssh).unwrap();
    let switched = prepared
        .stage_native(crate::git_backend::PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(switched.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert_eq!((ssh.contacts("a"), ssh.contacts("b")), ssh_contacts);
    assert_eq!(f.contacts("b"), 0);
    assert_eq!(f.connections("b"), 0);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn allowed_mirror_tls_failure_is_not_reported_as_all_excluded() {
    let f = Fixture::new();
    let other_trust = Fixture::new();
    let temporary = tempfile::tempdir().unwrap();
    for all_excluded in [false, true] {
        let before = (f.connections("a"), f.connections("b"));
        let binding = if all_excluded {
            vec!["127.0.0.1", "127.0.0.2"]
        } else {
            vec!["127.0.0.2"]
        };
        let profile = NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(binding).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::Pem(std::fs::read(other_trust.get("ca")).unwrap()),
        );
        let registry = toml::from_str(&format!(
            "name='fixture'\nurl='{}/public'\nindex_url='none'\nauth='none'\n",
            f.get("b")
        ))
        .unwrap();
        let alternative = toml::from_str(&format!(
            "of='fixture'\nurl='{}/public'\npriority=0\n",
            f.get("a")
        ))
        .unwrap();
        let resolver = crate::MultiRegistryResolver::from_manifest_with_native_admission(
            &[registry],
            &[alternative],
            &[],
            temporary
                .path()
                .join(if all_excluded { "excluded" } else { "tls" }),
            0,
            Arc::new(profile),
        )
        .unwrap();
        let error = resolver
            .resolve(&vibe_core::PackageRef::parse("org.fixture/demo@=1.0.0").unwrap())
            .unwrap_err();
        if all_excluded {
            assert_eq!((f.connections("a"), f.connections("b")), before);
            assert!(matches!(
                error,
                crate::RegistryError::ContactRefused(crate::ContactRefusal::ExcludedHost { .. })
            ));
        } else {
            assert!(f.connections("a") > before.0);
            assert_eq!(f.connections("b"), before.1);
            assert!(
                matches!(error,crate::RegistryError::Git(GitError::NetworkUnreachable{ref url}) if url.starts_with(f.get("a"))),
                "{error}"
            );
        }
    }
    assert_eq!(f.contacts("a"), 0);
    assert_eq!(f.contacts("b"), 0);
    assert_eq!(f.connections("b"), 0);
}
