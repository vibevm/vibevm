//! Public plan/apply SSH acquisition, explicit warm authority and owned recovery.
use super::*;
use vibe_core::manifest::AuthKind;
use vibe_registry::git_backend::shell::native_admission::{SshAuthentication, SshProfile};
fn ssh_fixture(agent: bool) -> (Server, serde_json::Value, Fixture) {
    let mut server = Server(
        Command::new("python3")
            .args([
                "-u",
                "-c",
                include_str!(
                    "../../../vibe-registry/src/git_backend/shell/native_admission/ssh_fixture.py"
                ),
            ])
            .env("VIBE_N3_INSTALL_FIXTURE", "1")
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
    let mut f = Fixture::new();
    let authentication = if agent {
        SshAuthentication::AgentSocket(info["agent_socket"].as_str().unwrap().into())
    } else {
        SshAuthentication::Key(fs::read(info["key"].as_str().unwrap()).unwrap())
    };
    let ssh = SshProfile::explicit_direct(
        "/usr/bin/ssh".into(),
        "127.0.0.1".into(),
        info["user"].as_str().unwrap().into(),
        info["port"].as_u64().unwrap() as u16,
        fs::read(info["known_hosts"].as_str().unwrap()).unwrap(),
        authentication,
    )
    .unwrap();
    let selection =
        NativeConfigSelection::new(f._temp.path().into(), info["home"].as_str().unwrap().into())
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
        TlsTrust::System,
    )
    .with_ssh_profile(ssh)
    .with_config_selection(selection);
    f.source.registry = GitPerPackageRegistry::open_single_package(
        "ssh-fixture",
        info["url"].as_str().unwrap(),
        "main",
        &f._temp.path().join("ssh-cache"),
        Arc::new(ShellGit::new().with_native_admission(Arc::new(profile))),
        0,
        AuthKind::Ssh,
        None,
    )
    .unwrap();
    (server, info, f)
}
fn ssh_contacts(info: &serde_json::Value) -> (usize, usize) {
    let read = |label| {
        fs::read_to_string(
            Path::new(info["root"].as_str().unwrap()).join(format!("sshd-{label}.log")),
        )
        .unwrap()
        .matches("Connection from ")
        .count()
    };
    (read("a"), read("b"))
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn public_ssh_key_and_agent_fresh_warm_update_and_noop_use_retained_source() {
    for agent in [false, true] {
        let (_server, info, f) = ssh_fixture(agent);
        let plan = f.ready();
        let before = ssh_contacts(&info);
        let result =
            vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
        assert_eq!(result.outcome.materialised.len(), 1);
        assert_eq!(ssh_contacts(&info), before);
        assert_eq!(
            git(&f.slot, &["rev-parse", "HEAD"]),
            info["head"].as_str().unwrap()
        );
        assert_eq!(
            fs::read(f.slot.join("payload")).unwrap(),
            b"SSH exact source fixture\n"
        );
        write(&f.slot, ".git/sentinel", "kept metadata");
        git(
            &f.slot,
            &[
                "config",
                "core.sshCommand",
                info["proxy_wrapper"].as_str().unwrap(),
            ],
        );
        let config = fs::read(f.slot.join(".git/config")).unwrap();
        *f.source.version.borrow_mut() = semver::Version::new(1, 1, 0);
        let plan = f.ready();
        let before = ssh_contacts(&info);
        let result =
            vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
        assert_eq!(result.outcome.materialised.len(), 1);
        assert_eq!(ssh_contacts(&info), before);
        assert_eq!(
            git(&f.slot, &["rev-parse", "HEAD"]),
            info["updated"].as_str().unwrap()
        );
        assert_eq!(
            fs::read(f.slot.join("payload")).unwrap(),
            b"SSH updated fixture\n"
        );
        assert_eq!(fs::read(f.slot.join(".git/config")).unwrap(), config);
        assert_eq!(
            fs::read_to_string(f.slot.join(".git/sentinel")).unwrap(),
            "kept metadata"
        );
        let plan = f.ready();
        let before = ssh_contacts(&info);
        let inode = fs::metadata(&f.slot).unwrap().ino();
        let result =
            vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
        assert_eq!(result.outcome.skipped.len(), 1);
        assert_eq!(fs::metadata(&f.slot).unwrap().ino(), inode);
        assert_eq!(ssh_contacts(&info), before);
        assert_eq!(before.1, 1);
        assert!(!Path::new(info["proxy_marker"].as_str().unwrap()).exists());
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-SSH-LAW")]
fn public_ssh_hook_failure_restores_original_slot_and_metadata() {
    let (_server, info, f) = ssh_fixture(false);
    vibe_install::apply(&f.source, *f.ready(), SlotIntegrity::Verify, &policy()).unwrap();
    write(&f.slot, ".git/sentinel", "rollback metadata");
    write(&f.slot, "untracked.tmp", "old actor bytes");
    let work = Path::new(info["root"].as_str().unwrap()).join("work");
    git(&work, &["reset", "--hard", "v1.1.0"]);
    let mut manifest = fs::read_to_string(work.join("vibe.toml")).unwrap();
    manifest.push_str("\n[hooks]\npre-install='hooks/ssh-failure'\n");
    write(&work, "vibe.toml", &manifest);
    let marker = f._temp.path().join("hook-executed");
    write(
        &work,
        "hooks/ssh-failure.sh",
        &format!("printf observed > '{}'\nexit 37\n", marker.display()),
    );
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-m", "SSH recovery fixture"]);
    git(&work, &["tag", "-f", "v1.1.0"]);
    let bare = Path::new(info["home"].as_str().unwrap()).join("repo.git");
    git(
        &bare,
        &[
            "fetch",
            "--force",
            work.to_str().unwrap(),
            "refs/tags/v1.1.0:refs/tags/v1.1.0",
        ],
    );
    *f.source.version.borrow_mut() = semver::Version::new(1, 1, 0);
    let plan = f.ready();
    let before = failures::snapshot(&f.project);
    let contacts = ssh_contacts(&info);
    let error =
        vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap_err();
    assert!(error.to_string().contains("37"), "{error}");
    assert_eq!(fs::read_to_string(&marker).unwrap(), "observed");
    assert_eq!(failures::snapshot(&f.project), before);
    assert_eq!(ssh_contacts(&info), contacts);
}
