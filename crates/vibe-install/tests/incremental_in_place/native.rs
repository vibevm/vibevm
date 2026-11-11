//! Public plan/apply controls using actual admitted numeric TLS acquisition.
use super::*;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::MetadataExt;
use std::process::Stdio;
use vibe_registry::git_backend::shell::native_admission::{
    GitInstallation, NativeConfigSelection, NativeGitAdmission, TlsTrust,
};
struct Server(std::process::Child);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.stdin.take();
        let _ = self.0.wait();
    }
}
fn fixture(copy: bool) -> (Server, serde_json::Value, Fixture) {
    let mut server = Server(
        Command::new("python3")
            .args([
                "-u",
                "-c",
                include_str!(
                    "../../../vibe-registry/src/git_backend/shell/native_admission/tls_fixture.py"
                ),
            ])
            .env("VIBE_N2_INSTALL_FIXTURE", if copy { "4" } else { "2" })
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
    let selection = NativeConfigSelection::new(
        f._temp.path().into(),
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
    f.source.registry = GitPerPackageRegistry::open_with(
        "fixture",
        &format!("{}/public", info["a"].as_str().unwrap()),
        "main",
        NamingConvention::Fqdn,
        &f._temp.path().join("native-cache"),
        Arc::new(ShellGit::new().with_native_admission(Arc::new(profile))),
        0,
    )
    .unwrap();
    (server, info, f)
}
fn contacts(info: &serde_json::Value) -> Vec<u8> {
    fs::read(Path::new(info["root"].as_str().unwrap()).join("a.log")).unwrap()
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn public_native_fresh_then_warm_noop_and_dirty_cleanup() {
    let (_server, info, f) = fixture(false);
    let plan = f.ready();
    let before = contacts(&info);
    let result = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    assert_eq!(result.outcome.materialised.len(), 1);
    assert_eq!(contacts(&info), before);
    assert_eq!(
        fs::read(f.slot.join("payload.txt")).unwrap(),
        b"retained fixture\r\n"
    );
    assert_eq!(fs::read(f.slot.join("binary.dat")).unwrap(), b"\0A\r\nB\n");
    assert_eq!(fs::read(f.slot.join("nested/lf.txt")).unwrap(), b"nested\n");
    assert_eq!(
        fs::read(f.slot.join("nested/crlf.txt")).unwrap(),
        b"nested\r\n"
    );
    assert_eq!(
        fs::read_link(f.slot.join("payload-link")).unwrap(),
        PathBuf::from("payload.txt")
    );
    assert_ne!(
        fs::metadata(f.slot.join("executable.sh")).unwrap().mode() & 0o111,
        0
    );
    write(&f.slot, ".git/sentinel", "retained metadata");
    let plan = f.ready();
    let before = contacts(&info);
    let inode = fs::metadata(&f.slot).unwrap().ino();
    let result = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    assert_eq!(result.outcome.skipped.len(), 1);
    assert_eq!(fs::metadata(&f.slot).unwrap().ino(), inode);
    assert_eq!(contacts(&info), before);
    let plan = f.ready();
    write(&f.slot, "payload.txt", "dirty after preparation");
    write(&f.slot, "untracked.tmp", "untracked after preparation");
    let result = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    assert_eq!(result.outcome.materialised.len(), 1);
    assert!(!f.slot.join("untracked.tmp").exists());
    assert_eq!(
        fs::read(f.slot.join("payload.txt")).unwrap(),
        b"retained fixture\r\n"
    );
    assert_eq!(
        fs::read_to_string(f.slot.join(".git/sentinel")).unwrap(),
        "retained metadata"
    );
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn public_native_copy_keeps_store_identity_and_uses_normal_versioned_slot() {
    let (_server, info, f) = fixture(true);
    let plan = f.ready();
    let before = contacts(&info);
    let result = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy()).unwrap();
    assert_eq!(contacts(&info), before);
    assert_eq!(result.outcome.materialised.len(), 1);
    assert!(!f.slot.join(".git").exists());
    let slot = f.slot.join("1.0.0");
    // An ordinary copy package is content-hashed and inserted into the
    // machine-global store, so its bytes are the package's own, identical on
    // every machine: the invocation's `eol=crlf` decides what an *in-place*
    // working copy holds (above), never what a store entry holds
    // (`vibe_registry::git_backend::verbatim`).
    assert_eq!(
        fs::read(slot.join("payload.txt")).unwrap(),
        b"retained fixture\n"
    );
    assert!(!slot.join(".git").exists());
    let lock = Lockfile::read(f.project.join("vibe.lock")).unwrap();
    assert!(!lock.packages[0].materialization.is_in_place());
}

#[path = "native_failures.rs"]
mod failures;

fn legacy_second(f: &mut Fixture, info: &serde_json::Value) {
    let root = f._temp.path().join("legacy-registry");
    fs::create_dir(&root).unwrap();
    std::os::unix::fs::symlink(
        info["second_work"].as_str().unwrap(),
        root.join("org.vibevm.second.git"),
    )
    .unwrap();
    f.source.secondary = Some(
        GitPerPackageRegistry::open_with(
            "legacy",
            root.to_str().unwrap(),
            "main",
            NamingConvention::Fqdn,
            &f._temp.path().join("legacy-cache"),
            Arc::new(backend(f._temp.path())),
            0,
        )
        .unwrap(),
    );
    f.source.names = vec!["giant".into(), "second".into()];
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn mixed_cached_legacy_and_native_ignore_writes_share_owned_undo() {
    let (_server, info, mut f) = fixture(false);
    vibe_install::apply(&f.source, *f.ready(), SlotIntegrity::Verify, &policy()).unwrap();
    failures::publish(&info, "second", "exit 37\n", false);
    legacy_second(&mut f, &info);
    let planned = failures::ready(&f);
    let before = failures::snapshot(&f.project);
    let error =
        vibe_install::apply(&f.source, *planned, SlotIntegrity::Verify, &policy()).unwrap_err();
    assert!(error.to_string().contains("37"), "{error}");
    assert_eq!(failures::snapshot(&f.project), before);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn mixed_deferred_legacy_and_native_refuse_before_caller_writes() {
    let (_server, info, mut f) = fixture(false);
    legacy_second(&mut f, &info);
    vibe_install::apply(
        &f.source,
        *failures::ready(&f),
        SlotIntegrity::Verify,
        &policy(),
    )
    .unwrap();
    let planned = failures::ready(&f);
    let before = failures::snapshot(&f.project);
    let error =
        vibe_install::apply(&f.source, *planned, SlotIntegrity::Verify, &policy()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("mixed deferred legacy and native"),
        "{error}"
    );
    assert_eq!(failures::snapshot(&f.project), before);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn metadata_only_commit_is_materialised_even_when_payload_is_identical() {
    let (_server, info, f) = fixture(false);
    vibe_install::apply(&f.source, *f.ready(), SlotIntegrity::Verify, &policy()).unwrap();
    let old = git(&f.slot, &["rev-parse", "HEAD"]);
    let bytes = fs::read(f.slot.join("payload.txt")).unwrap();
    let work = Path::new(info["work"].as_str().unwrap());
    git(work, &["reset", "--hard", "v1.0.0"]);
    git(work, &["commit", "--allow-empty", "-m", "metadata only"]);
    git(work, &["tag", "-f", "v1.0.0"]);
    let bare = Path::new(info["root"].as_str().unwrap()).join("web/repo.git");
    git(
        &bare,
        &[
            "fetch",
            "--force",
            work.to_str().unwrap(),
            "refs/tags/v1.0.0:refs/tags/v1.0.0",
        ],
    );
    git(&bare, &["update-server-info"]);
    let result =
        vibe_install::apply(&f.source, *f.ready(), SlotIntegrity::Verify, &policy()).unwrap();
    assert_eq!(result.outcome.materialised.len(), 1);
    assert_ne!(git(&f.slot, &["rev-parse", "HEAD"]), old);
    assert_eq!(fs::read(f.slot.join("payload.txt")).unwrap(), bytes);
}

#[path = "native_ssh.rs"]
mod ssh;

#[path = "locked_source.rs"]
mod locked_source;
