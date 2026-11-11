//! Executable local checkout/capture controls; no public service or user profile.
#![cfg(all(test, unix))]
use super::*;
use crate::git_backend::PayloadBytes;
use std::os::unix::fs::PermissionsExt;

#[path = "materialisation_tests/fixtures.rs"]
mod fixtures;
use fixtures::{script, selected, warm_origin};

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn default_global_attributes_includes_info_override_and_external_markers() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let home = Path::new(f.get("root")).join("home");
    std::fs::create_dir_all(home.join(".config/git")).unwrap();
    let attributes = home.join(".config/git/attributes");
    std::fs::write(&attributes, "payload.txt text eol=crlf\n").unwrap();
    let marker = temp.path().join("filter-marker");
    let trace_marker = temp.path().join("trace-marker");
    let filter = temp.path().join("filter.py");
    script(&filter, &marker);
    let monitor_marker = temp.path().join("fsmonitor-marker");
    let monitor = temp.path().join("monitor.py");
    script(&monitor, &monitor_marker);
    let hook_marker = temp.path().join("hook-marker");
    let hooks = temp.path().join("hooks");
    std::fs::create_dir(&hooks).unwrap();
    script(&hooks.join("post-checkout"), &hook_marker);
    let include = temp.path().join("included.config");
    std::fs::write(&include,format!("[core]\n autocrlf=false\n eol=lf\n fsmonitor={}\n hooksPath={}\n[filter \"active\"]\n smudge={}\n required=true\n",monitor.display(),hooks.display(),filter.display())).unwrap();
    std::fs::write(
        home.join(".gitconfig"),
        format!(
            "[include]\n path={}\n[trace2]\n eventTarget={}\n",
            include.display(),
            trace_marker.display()
        ),
    )
    .unwrap();
    let git = selected(&f, temp.path(), &[], true);
    let url = format!("{}/repo.git", f.get("a"));
    let objects = temp.path().join("default.git");
    let slot = temp.path().join("slot");
    let revision = git
        .prepare_ref(&url, "v1.0.0", None, &objects, &slot)
        .unwrap();
    let stage = git
        .stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(stage.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\r\n"
    );
    assert!(
        !marker.exists()
            && !monitor_marker.exists()
            && !hook_marker.exists()
            && !trace_marker.exists()
    );
    // The same objects, staged for a tree vibe reads as data rather than for
    // a user's slot: the recorded blob bytes, whatever the configuration and
    // the attributes would have converted (`crate::git_backend::verbatim`).
    // A registry working copy's template snapshot is hashed against its own
    // record, so this is the difference between priming and warning once per
    // version.
    let objects = temp.path().join("verbatim.git");
    let revision = git
        .prepare_ref(&url, "v1.0.0", None, &objects, &slot)
        .unwrap();
    let data = git
        .stage_native_materialisation(&objects, &revision, PayloadBytes::Verbatim)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(data.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert!(
        !marker.exists()
            && !monitor_marker.exists()
            && !hook_marker.exists()
            && !trace_marker.exists()
    );
    std::fs::write(&attributes, "payload.txt text eol=lf\n").unwrap();
    assert!(stage.validate_before_publication().is_err());
    let warm = warm_origin(&f);
    std::fs::write(&attributes, "payload.txt text eol=crlf\n").unwrap();
    std::fs::write(
        warm.join(".git/info/attributes"),
        "payload.txt text eol=lf\n",
    )
    .unwrap();
    let objects = temp.path().join("info.git");
    let revision = git
        .prepare_ref(&url, "v1.0.0", Some(&warm), &objects, &warm)
        .unwrap();
    let stage = git
        .stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(stage.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert!(
        !marker.exists()
            && !monitor_marker.exists()
            && !hook_marker.exists()
            && !trace_marker.exists()
    );
    std::fs::remove_file(warm.join(".git/info/attributes")).unwrap();
    std::fs::write(&attributes, "payload.txt filter=active\n").unwrap();
    let objects = temp.path().join("active.git");
    let revision = git
        .prepare_ref(&url, "v1.0.0", Some(&warm), &objects, &warm)
        .unwrap();
    assert!(matches!(
        git.stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured),
        Err(GitError::NativeContext { .. })
    ));
    assert!(
        !marker.exists()
            && !monitor_marker.exists()
            && !hook_marker.exists()
            && !trace_marker.exists()
    );
    // The same configured real filter executes under legacy checkout.
    let legacy = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .current_dir(&warm)
        .args(["checkout", "--force", "--detach", f.get("head")])
        .output()
        .unwrap();
    assert!(
        legacy.status.success(),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    assert!(marker.exists() && hook_marker.exists() && trace_marker.exists());
    assert!(
        std::fs::read_to_string(warm.join("payload.txt"))
            .unwrap()
            .contains("FILTERED")
    );
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn reserved_filter_driver_names_are_not_mistaken_for_disabled_attributes() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let home = Path::new(f.get("root")).join("home");
    let attributes = temp.path().join("attrs");
    let marker = temp.path().join("filter-marker");
    let filter = temp.path().join("filter.py");
    script(&filter, &marker);
    std::fs::write(home.join(".gitconfig"),format!("[core]\n attributesFile={}\n[filter \"unset\"]\n smudge={}\n required=true\n[filter \"unspecified\"]\n smudge={}\n required=true\n",attributes.display(),filter.display(),filter.display())).unwrap();
    let git = selected(&f, temp.path(), &[], true);
    let url = format!("{}/repo.git", f.get("a"));
    for (i, value) in ["-filter", "!filter", "filter=unset", "filter=unspecified"]
        .iter()
        .enumerate()
    {
        std::fs::write(&attributes, format!("payload.txt {value}\n")).unwrap();
        let objects = temp.path().join(format!("case{i}.git"));
        let slot = temp.path().join(format!("slot{i}"));
        let revision = git
            .prepare_ref(&url, "v1.0.0", None, &objects, &slot)
            .unwrap();
        let result =
            git.stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured);
        assert_eq!(result.is_ok(), i < 2, "{value}: {result:?}");
        assert!(!marker.exists());
        assert!(!slot.exists());
    }
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn warm_proxy_helper_and_header_context_refuse_before_contact() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let warm = warm_origin(&f);
    let config = std::fs::read(warm.join(".git/config")).unwrap();
    let git = selected(&f, temp.path(), &[], true);
    let marker = temp.path().join("marker");
    let helper = format!("!touch {}", marker.display());
    for (i, key) in ["http.proxy", "credential.helper", "http.extraHeader"]
        .iter()
        .enumerate()
    {
        let value = if *key == "credential.helper" {
            helper.clone()
        } else if *key == "http.proxy" {
            f.get("b").to_owned()
        } else {
            "X-Synthetic: value".into()
        };
        assert!(
            fixture_git(&f.info, &["config", key, &value], &warm)
                .status
                .success()
        );
        let before = (f.contacts("a"), f.contacts("b"));
        let result = git.prepare_ref(
            &format!("{}/repo.git", f.get("a")),
            "v1.0.0",
            Some(&warm),
            &temp.path().join(format!("blocked{i}.git")),
            &warm,
        );
        assert!(
            matches!(result,Err(GitError::NativeContext{reason}) if reason.contains("warm transport"))
        );
        assert_eq!((f.contacts("a"), f.contacts("b")), before);
        assert!(!marker.exists());
        std::fs::write(warm.join(".git/config"), &config).unwrap();
    }
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn captured_command_rewrite_b_is_gated_and_config_only_override_is_not_transport() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let warm = warm_origin(&f);
    let override_file = temp.path().join("config-only");
    std::fs::write(
        &override_file,
        format!("[remote \"origin\"]\n url={}/wrong.git\n", f.get("b")),
    )
    .unwrap();
    let extras = [
        ("GIT_CONFIG", override_file.display().to_string()),
        ("GIT_CONFIG_COUNT", "1".into()),
        ("GIT_CONFIG_KEY_0", format!("url.{}/.insteadOf", f.get("b"))),
        ("GIT_CONFIG_VALUE_0", format!("{}/", f.get("a"))),
    ];
    let denied = selected(&f, temp.path(), &extras, true);
    let a = format!("{}/repo.git", f.get("a"));
    let b = format!("{}/repo.git", f.get("b"));
    assert_eq!(denied.recorded_origin_url(&warm).unwrap(), a);
    assert_eq!(denied.origin_url(&warm).unwrap(), b);
    assert!(matches!(
        denied.prepare_ref(
            &b,
            "v1.0.0",
            Some(&warm),
            &temp.path().join("denied.git"),
            &warm
        ),
        Err(GitError::ContactRefused(_))
    ));
    assert_eq!(f.contacts("b"), 0);
    let allowed = selected(&f, temp.path(), &extras, false);
    let revision = allowed
        .prepare_ref(
            &b,
            "v1.0.0",
            Some(&warm),
            &temp.path().join("allowed.git"),
            &warm,
        )
        .unwrap();
    assert_eq!(revision.commit, f.get("head"));
    assert!(f.contacts("b") > 0);
    assert_eq!(f.contacts("a"), 0);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn warm_object_compression_and_directory_modes_are_preserved_and_corruption_refuses() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let warm = warm_origin(&f);
    let git = selected(&f, temp.path(), &[], true);
    let blob = fixture_git(
        &f.info,
        &["rev-parse", &format!("{}:payload.txt", f.get("head"))],
        &warm,
    );
    assert!(blob.status.success());
    let oid = String::from_utf8(blob.stdout).unwrap().trim().to_owned();
    let object = warm.join(".git/objects").join(&oid[..2]).join(&oid[2..]);
    let original = std::fs::read(&object).unwrap();
    std::fs::remove_file(&object).unwrap();
    let plain = temp.path().join("plain");
    std::fs::write(&plain, b"retained fixture\n").unwrap();
    let rewritten = fixture_git(
        &f.info,
        &[
            "-c",
            "core.looseCompression=0",
            "hash-object",
            "-w",
            plain.to_str().unwrap(),
        ],
        &warm,
    );
    assert!(rewritten.status.success());
    assert_eq!(String::from_utf8(rewritten.stdout).unwrap().trim(), oid);
    let recompressed = std::fs::read(&object).unwrap();
    assert_ne!(recompressed, original);
    std::fs::set_permissions(
        warm.join(".git/objects"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::set_permissions(
        warm.join(".git/refs"),
        std::fs::Permissions::from_mode(0o750),
    )
    .unwrap();
    let objects = temp.path().join("objects.git");
    let url = format!("{}/repo.git", f.get("a"));
    let revision = git
        .prepare_ref(&url, "v1.0.0", Some(&warm), &objects, &warm)
        .unwrap();
    let stage = git
        .stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(
            stage
                .staged_path()
                .join(".git/objects")
                .join(&oid[..2])
                .join(&oid[2..])
        )
        .unwrap(),
        recompressed
    );
    assert_eq!(
        std::fs::metadata(stage.staged_path().join(".git/objects"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(stage.staged_path().join(".git/refs"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o750
    );
    std::fs::set_permissions(&object, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&object, b"corrupt synthetic loose object").unwrap();
    let objects = temp.path().join("corrupt.git");
    let revision = git
        .prepare_ref(&url, "v1.0.0", Some(&warm), &objects, &warm)
        .unwrap();
    assert!(
        git.stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured)
            .is_err()
    );
    assert_eq!(
        std::fs::read(&object).unwrap(),
        b"corrupt synthetic loose object"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn warm_registry_retains_explicit_token_auth_without_ambient_credentials() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let warm = Path::new(f.get("work"));
    assert!(
        fixture_git(
            &f.info,
            &[
                "remote",
                "add",
                "origin",
                &format!("{}/auth.git", f.get("a"))
            ],
            warm
        )
        .status
        .success()
    );
    let before = std::fs::read(warm.join(".git/config")).unwrap();
    let registry = crate::GitPerPackageRegistry::open_with_explicit_token(
        "fixture",
        &format!("{}/auth", f.get("a")),
        "main",
        vibe_core::manifest::NamingConvention::Fqdn,
        vec![],
        &temp.path().join("cache"),
        Arc::new(selected(&f, temp.path(), &[], true)),
        0,
        vibe_core::manifest::AuthKind::TokenEnv,
        Some("SYNTHETIC_N1_AUTH".into()),
    )
    .unwrap();
    let prepared = registry
        .prepare_in_place(
            &crate::ResolvedPackage {
                group: vibe_core::Group::parse("org.fixture").unwrap(),
                name: "demo".into(),
                version: semver::Version::new(1, 0, 0),
                source_dir: PathBuf::new(),
            },
            warm,
        )
        .unwrap();
    assert_eq!(prepared.resolved_commit(), f.get("head"));
    let stage = prepared
        .stage_native(PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(stage.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert_eq!(std::fs::read(warm.join(".git/config")).unwrap(), before);
    assert!(
        !String::from_utf8_lossy(&std::fs::read(stage.staged_path().join(".git/config")).unwrap())
            .contains("SYNTHETIC_N1_AUTH")
    );
    assert_eq!(f.contacts("b"), 0);
    let before_contacts = (f.contacts("a"), f.contacts("b"));
    assert!(
        fixture_git(
            &f.info,
            &[
                "config",
                &format!("url.{}/.insteadOf", f.get("b")),
                &format!("{}/", f.get("a"))
            ],
            warm
        )
        .status
        .success()
    );
    let unrestricted = crate::GitPerPackageRegistry::open_with_explicit_token(
        "fixture",
        &format!("{}/auth", f.get("a")),
        "main",
        vibe_core::manifest::NamingConvention::Fqdn,
        vec![],
        &temp.path().join("other-cache"),
        Arc::new(selected(&f, temp.path(), &[], false)),
        0,
        vibe_core::manifest::AuthKind::TokenEnv,
        Some("SYNTHETIC_N1_AUTH".into()),
    )
    .unwrap();
    let result = unrestricted.prepare_in_place(
        &crate::ResolvedPackage {
            group: vibe_core::Group::parse("org.fixture").unwrap(),
            name: "demo".into(),
            version: semver::Version::new(1, 0, 0),
            source_dir: PathBuf::new(),
        },
        warm,
    );
    assert!(
        matches!(result,Err(crate::RegistryError::Git(GitError::NativeContext{reason})) if reason.contains("credential origin"))
    );
    assert_eq!((f.contacts("a"), f.contacts("b")), before_contacts);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn native_stage_retains_exact_payload_and_warm_repository_metadata() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let selection =
        NativeConfigSelection::new(temp.path().into(), Path::new(f.get("root")).join("home"))
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
    let git = ShellGit::new().with_native_admission(Arc::new(profile));
    let url = format!("{}/repo.git", f.get("a"));
    let objects = temp.path().join("fresh.git");
    let destination = temp.path().join("destination");
    let revision = git
        .prepare_ref(&url, "v1.0.0", None, &objects, &destination)
        .unwrap();
    let staged = git
        .stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(staged.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert!(!destination.exists());
    staged.validate_before_publication().unwrap();

    let warm = Path::new(f.get("work"));
    assert!(
        fixture_git(&f.info, &["remote", "add", "origin", &url], warm)
            .status
            .success()
    );
    std::fs::write(warm.join("untracked.txt"), "preserve user bytes").unwrap();
    std::fs::write(warm.join(".git/sentinel"), "preserve Git metadata").unwrap();
    let config = std::fs::read(warm.join(".git/config")).unwrap();
    let objects = temp.path().join("warm.git");
    let revision = git
        .prepare_ref(&url, "v1.0.0", Some(warm), &objects, warm)
        .unwrap();
    let staged = git
        .stage_native_materialisation(&objects, &revision, PayloadBytes::AsConfigured)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read(staged.staged_path().join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert_eq!(
        std::fs::read(staged.staged_path().join(".git/config")).unwrap(),
        config
    );
    assert!(!staged.staged_path().join("untracked.txt").exists());
    assert_eq!(
        std::fs::read(warm.join("untracked.txt")).unwrap(),
        b"preserve user bytes"
    );
    assert_eq!(
        std::fs::read(staged.staged_path().join(".git/sentinel")).unwrap(),
        b"preserve Git metadata"
    );
    staged.validate_before_publication().unwrap();
    std::fs::write(warm.join("untracked.txt"), "new actor bytes").unwrap();
    assert!(staged.validate_before_publication().is_err());
}
