//! Existing shared mirror audit facts reach the report-only sidecar.
//! This is not a user-local transport preference or transition receipt.
#![cfg(unix)]
use super::*;
use std::os::unix::fs::MetadataExt;
use vibe_core::lock_provenance::{LockProvenance, provenance_path};

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn in_place_mirror_audit_survives_recording_noop_and_failed_apply() {
    let mut server = Server(
        Command::new("python3")
            .args([
                "-u",
                "-c",
                include_str!(
                    "../../../vibe-registry/src/git_backend/shell/native_admission/tls_fixture.py"
                ),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut line = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let info: serde_json::Value = serde_json::from_str(&line).unwrap();
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    let work = Path::new(info["work"].as_str().unwrap());
    git(&info, work, &["reset", "--hard", "v1.0.0"]);
    let package = "[package]\ngroup='org.fixture'\nname='demo'\nkind='feat'\nversion='1.0.0'\nmaterialization='in-place'\n";
    write(work, "vibe.toml", package);
    git(&info, work, &["add", "vibe.toml"]);
    publish(&info);
    let wanted = git(&info, work, &["rev-parse", "HEAD"]);
    let primary = format!("{}/public", info["b"].as_str().unwrap());
    let mirror = format!("{}/public", info["a"].as_str().unwrap());
    let canonical = format!("{primary}/org.fixture.demo.git");
    let actual = format!("{mirror}/org.fixture.demo.git");
    let authored = format!(
        "[project]\nname='mirror-audit'\nversion='0.1.0'\n[[registry]]\nname='audit'\nurl='{primary}'\nauth='none'\n[[mirror]]\nof='audit'\nurl='{mirror}'\npriority=0\n"
    );
    write(&project, "vibe.toml", &authored);
    let manifest = Manifest::parse_str(&authored).unwrap();
    let selection = NativeConfigSelection::new(
        root.path().into(),
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
        Arc::new(vibe_registry::HttpAdmission::new(["127.0.0.2"]).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(fs::read(info["ca"].as_str().unwrap()).unwrap()),
    )
    .with_config_selection(selection);
    let source = Source {
        resolver: MultiRegistryResolver::from_manifest_with_native_admission(
            &manifest.registries,
            &manifest.mirrors,
            &[],
            root.path().join("registry"),
            0,
            Arc::new(profile),
        )
        .unwrap(),
        store: root.path().join("store"),
    };
    let policy = vibe_workspace::hooks::HookPolicy {
        allowed_groups: vec!["org.fixture".into()],
        allow_hooks: false,
    };
    let plan = ready(&source, &project);
    let (_, _, _, evidence) = plan.package_sources().next().unwrap();
    assert_eq!(evidence.source_url.as_deref(), Some(canonical.as_str()));
    assert_eq!(evidence.fetched_from.as_deref(), Some(actual.as_str()));
    assert_eq!(evidence.resolved_commit.as_deref(), Some(wanted.as_str()));
    let report = vibe_install::apply(&source, *plan, SlotIntegrity::Verify, &policy).unwrap();
    assert_eq!(report.outcome.materialised.len(), 1);
    let lock_path = project.join("vibe.lock");
    let sidecar = provenance_path(&lock_path);
    let lock = Lockfile::read(&lock_path).unwrap();
    assert_eq!(lock.packages[0].source_url.as_str(), canonical);
    assert_eq!(
        lock.packages[0].resolved_commit.as_deref(),
        Some(wanted.as_str())
    );
    assert!(
        !fs::read_to_string(&lock_path)
            .unwrap()
            .contains("fetched_from")
    );
    let audit = LockProvenance::read_beside(&lock_path).unwrap();
    assert_eq!(audit.packages.len(), 1);
    let row = audit.of("org.fixture/demo").unwrap();
    assert_eq!(row.version, "1.0.0");
    assert_eq!(row.fetched_from, actual);
    let audit_bytes = fs::read(&sidecar).unwrap();
    let audit_inode = fs::metadata(&sidecar).unwrap().ino();

    let noop = ready(&source, &project);
    let (_, _, _, evidence) = noop.package_sources().next().unwrap();
    assert_eq!(evidence.fetched_from.as_deref(), Some(actual.as_str()));
    let report = vibe_install::apply(&source, *noop, SlotIntegrity::Verify, &policy).unwrap();
    assert_eq!(report.outcome.skipped.len(), 1);
    assert_eq!(fs::read(&sidecar).unwrap(), audit_bytes);
    assert_eq!(fs::metadata(&sidecar).unwrap().ino(), audit_inode);
    assert_eq!(
        Lockfile::read(&lock_path).unwrap().packages[0]
            .source_url
            .as_str(),
        canonical
    );

    let observed = root.path().join("failed-hook-ran");
    write(
        work,
        "vibe.toml",
        &format!("{package}\n[hooks]\npre-install='hooks/failure'\n"),
    );
    write(
        work,
        "hooks/failure.sh",
        &format!("printf ran > {}\nexit 37\n", quote(&observed)),
    );
    git(&info, work, &["add", "-A"]);
    publish(&info);
    let failing = ready(&source, &project);
    let before = snapshot(&project);
    let error = vibe_install::apply(&source, *failing, SlotIntegrity::Verify, &policy).unwrap_err();
    assert!(error.to_string().contains("37"), "{error}");
    assert_eq!(fs::read_to_string(observed).unwrap(), "ran");
    assert_eq!(snapshot(&project), before);
    assert_eq!(fs::read(&sidecar).unwrap(), audit_bytes);
    assert_eq!(fs::metadata(&sidecar).unwrap().ino(), audit_inode);
    assert_eq!(
        Lockfile::read(&lock_path).unwrap().packages[0]
            .source_url
            .as_str(),
        canonical
    );
    let denied = Path::new(info["root"].as_str().unwrap()).join("b.tcp.log");
    assert!(!denied.exists() || fs::read_to_string(denied).unwrap().is_empty());
}
