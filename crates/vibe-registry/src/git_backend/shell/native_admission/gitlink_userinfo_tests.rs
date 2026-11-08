//! Raw child userinfo must not become retained configuration through a rewrite.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn raw_child_credentials_rewritten_to_clean_endpoint_refuse_before_contact() {
    for scheme in ["https", "ssh", "git+https"] {
        assert_raw_rewrite_refused(scheme, false);
    }
    for scheme in ["git+https", "custom"] {
        assert_raw_rewrite_refused(scheme, true);
    }
}
#[cfg(test)]
fn assert_raw_rewrite_refused(scheme: &str, malformed: bool) {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let clean = format!("{}/repo.git", f.get("b"));
    let raw = if malformed {
        format!("{scheme}://fixture:RAW_CHILD_SECRET@[bad/child.git")
    } else {
        format!(
            "{scheme}://fixture:RAW_CHILD_SECRET@{}/repo.git",
            f.get("b").strip_prefix("https://").unwrap()
        )
    };
    let expected_reason = if malformed {
        "malformed raw child URL"
    } else {
        "raw child URL contains disallowed credentials"
    };
    let parent = Path::new(f.get("work"));
    std::fs::write(
        parent.join(".gitmodules"),
        format!("[submodule \"child\"]\npath=child\nurl={raw}\n"),
    )
    .unwrap();
    for args in [
        vec!["add".to_owned(), ".gitmodules".into()],
        vec![
            "update-index".into(),
            "--add".into(),
            "--cacheinfo".into(),
            format!("160000,{},child", f.get("head")),
        ],
        vec![
            "-c".into(),
            "user.name=Fixture".into(),
            "-c".into(),
            "user.email=f@invalid".into(),
            "commit".into(),
            "-m".into(),
            "raw child metadata".into(),
        ],
        vec!["tag".into(), "v3.0.0".into()],
    ] {
        let args: Vec<_> = args.iter().map(String::as_str).collect();
        assert!(fixture_git(&f.info, &args, parent).status.success());
    }
    let bare = Path::new(f.get("root")).join("web/repo.git");
    assert!(
        fixture_git(
            &f.info,
            &[
                "fetch",
                parent.to_str().unwrap(),
                "refs/tags/v3.0.0:refs/tags/v3.0.0"
            ],
            &bare
        )
        .status
        .success()
    );
    assert!(
        fixture_git(&f.info, &["update-server-info"], &bare)
            .status
            .success()
    );
    let selection =
        NativeConfigSelection::new(temp.path().into(), Path::new(f.get("root")).join("home"))
            .unwrap()
            .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
            .unwrap()
            .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
            .unwrap()
            .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
            .unwrap()
            .with_variable("GIT_CONFIG_COUNT", "1".into())
            .unwrap()
            .with_variable("GIT_CONFIG_KEY_0", format!("url.{clean}.insteadOf").into())
            .unwrap()
            .with_variable("GIT_CONFIG_VALUE_0", raw.clone().into())
            .unwrap();
    let profile = Arc::new(
        NativeGitAdmission::direct_https(
            Arc::new(crate::HttpAdmission::new(Vec::<String>::new()).unwrap()),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::Pem(std::fs::read(f.get("ca")).unwrap()),
        )
        .with_config_selection(selection),
    );
    let captured = profile.capture_context(None).unwrap();
    assert_eq!(gitlinks::rewritten_url(&raw, &captured.entries), clean);
    let git = ShellGit::new().with_native_admission(profile.clone());
    let slot = temp.path().join("slot");
    std::fs::create_dir(&slot).unwrap();
    std::fs::write(slot.join("KEEP"), "caller bytes").unwrap();
    let before = materialisation::tree(&slot).unwrap();
    let contacts = f.connections("b");
    let objects = temp.path().join("owned.git");
    let error = git
        .prepare_ref(
            &format!("{}/repo.git", f.get("a")),
            "v3.0.0",
            None,
            &objects,
            &slot,
        )
        .unwrap_err();
    assert!(
        matches!(&error, GitError::PreparedValidation { source } if matches!(source.as_ref(), GitError::NativeContext { reason } if *reason == expected_reason))
    );
    assert!(!error.to_string().contains("RAW_CHILD_SECRET"));
    assert_eq!(f.connections("b"), contacts);
    assert_eq!(materialisation::tree(&slot).unwrap(), before);
    assert!(!objects.join("native-children").exists());
    // Immutable parent Git objects contain the authored .gitmodules input;
    // generated credential-bearing repository configuration must not be added.
    for entry in walkdir::WalkDir::new(&objects)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "config")
    {
        assert!(
            !std::fs::read_to_string(entry.path())
                .unwrap()
                .contains("RAW_CHILD_SECRET")
        );
    }
    let supplied_objects = temp.path().join("supplied.git");
    let state = NativeState::new(profile.clone());
    let remote = profile.capture_for_request(&clean, false, None).unwrap();
    let result = state.prepare_tree(
        Preparation {
            source_address: SourceAddress::WarmOrigin,
            reporter: &git,
            address: &raw,
            reference: f.get("head"),
            reuse: None,
            objects: &supplied_objects,
            destination: &slot,
            anonymous: false,
        },
        &mut Vec::new(),
        Some(remote),
        false,
        None,
    );
    assert!(matches!(
        result,
        Err(GitError::NativeContext { reason }) if reason == expected_reason
    ));
    assert!(!supplied_objects.exists());
    assert_eq!(f.connections("b"), contacts);
    assert_eq!(materialisation::tree(&slot).unwrap(), before);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn raw_http_userinfo_guard_preserves_ssh_identity_and_relative_urls() {
    for value in [
        "ssh://git@example.invalid/repo.git",
        "git@example.invalid:repo.git",
        "../child.git",
        "../custom://literal-child.git",
        "git@example.invalid:custom://literal-child.git",
        "https://example.invalid/repo.git",
    ] {
        gitlinks::reject_raw_url_credentials(value).unwrap();
    }
    for value in [
        "http://user:secret@example.invalid/repo.git",
        "HTTPS://user@example.invalid/repo.git",
        "https://:secret@example.invalid/repo.git",
        "ssh://git:secret@example.invalid/repo.git",
        "git+https://user:secret@example.invalid/repo.git",
        "custom://user@example.invalid/repo.git",
        "git+https://user:secret@[bad/child.git",
        "custom://user:secret@[bad/child.git",
    ] {
        assert!(gitlinks::reject_raw_url_credentials(value).is_err());
    }
}
