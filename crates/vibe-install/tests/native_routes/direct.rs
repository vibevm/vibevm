//! Public plan/apply and retained-ref controls for direct controlled sources.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;
use vibe_core::manifest::SourceKind;

fn git(info: &serde_json::Value, cwd: &Path, args: &[&str]) -> String {
    let result = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env(
            "HOME",
            Path::new(info["root"].as_str().unwrap()).join("home"),
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().into()
}
struct Fixture {
    _server: Server,
    info: serde_json::Value,
    _temp: tempfile::TempDir,
    project: PathBuf,
    manifest: Manifest,
    commit: String,
}
impl Fixture {
    fn new(kind: SourceKind, inplace: bool, branch: bool, denied: bool) -> Self {
        let mut server=Server(Command::new("python3").args(["-u","-c",include_str!("../../../vibe-registry/src/git_backend/shell/native_admission/tls_fixture.py")])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap());
        let mut line = String::new();
        BufReader::new(server.0.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let info: serde_json::Value = serde_json::from_str(&line).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();
        let work = Path::new(info["work"].as_str().unwrap());
        if inplace {
            let text = fs::read_to_string(work.join("vibe.toml")).unwrap();
            fs::write(
                work.join("vibe.toml"),
                format!("{text}materialization='in-place'\n"),
            )
            .unwrap();
        }
        fs::write(work.join("payload.txt"), "direct payload\n").unwrap();
        git(&info, work, &["add", "."]);
        git(&info, work, &["commit", "-m", "direct fixture"]);
        let commit = git(&info, work, &["rev-parse", "HEAD"]);
        git(&info, work, &["tag", "release-pin"]);
        let bare = Path::new(info["root"].as_str().unwrap()).join("web/repo.git");
        git(
            &info,
            &bare,
            &[
                "fetch",
                "--force",
                work.to_str().unwrap(),
                "refs/heads/main:refs/heads/main",
                "refs/tags/release-pin:refs/tags/release-pin",
            ],
        );
        // Same-named tag deliberately points elsewhere: branch intent must survive.
        git(
            &info,
            &bare,
            &[
                "update-ref",
                "refs/tags/main",
                info["head"].as_str().unwrap(),
            ],
        );
        git(&info, &bare, &["update-server-info"]);
        let url = format!(
            "{}/repo.git",
            info[if denied { "b" } else { "a" }].as_str().unwrap()
        );
        let declaration = if kind == SourceKind::Git {
            format!(
                "[requires.packages]\n'org.fixture/demo'={{git='{url}',{}='{}',auth='none'}}\n",
                if branch { "branch" } else { "tag" },
                if branch { "main" } else { "release-pin" }
            )
        } else {
            format!(
                "[[override]]\npkgref='org.fixture/demo'\nsource_url='{url}'\nref='refs/tags/release-pin'\n"
            )
        };
        let text = format!("[project]\nname='direct'\nversion='0.1.0'\n{declaration}");
        fs::write(project.join("vibe.toml"), &text).unwrap();
        let manifest = Manifest::parse_str(&text).unwrap();
        Self {
            _server: server,
            info,
            _temp: temp,
            project,
            manifest,
            commit,
        }
    }
    fn resolver(&self) -> MultiRegistryResolver {
        let selection = NativeConfigSelection::new(
            self._temp.path().into(),
            Path::new(self.info["root"].as_str().unwrap()).join("home"),
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
            TlsTrust::Pem(fs::read(self.info["ca"].as_str().unwrap()).unwrap()),
        )
        .with_config_selection(selection);
        MultiRegistryResolver::from_manifest_with_native_admission(
            &[],
            &[],
            &self.manifest.overrides,
            self._temp.path().join("cache"),
            0,
            Arc::new(profile),
        )
        .unwrap()
        .with_git_packages(self.manifest.requires.git_packages.clone())
        .with_project_overrides(self.manifest.overrides.clone())
    }
    fn contacts(&self, label: &str) -> Vec<u8> {
        fs::read(Path::new(self.info["root"].as_str().unwrap()).join(format!("{label}.tcp.log")))
            .unwrap_or_default()
    }
    fn request() -> PackageRef {
        PackageRef::parse("org.fixture/demo@=1.0.0").unwrap()
    }
    fn move_tag(&self) {
        let work = Path::new(self.info["work"].as_str().unwrap());
        fs::write(work.join("payload.txt"), "changed after metadata\n").unwrap();
        git(&self.info, work, &["add", "payload.txt"]);
        git(&self.info, work, &["commit", "-m", "moved remote ref"]);
        git(&self.info, work, &["tag", "-f", "release-pin"]);
        let bare = Path::new(self.info["root"].as_str().unwrap()).join("web/repo.git");
        git(
            &self.info,
            &bare,
            &[
                "fetch",
                "--force",
                work.to_str().unwrap(),
                "refs/tags/release-pin:refs/tags/release-pin",
            ],
        );
        git(&self.info, &bare, &["update-server-info"]);
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn direct_git_and_override_complete_copy_and_inplace_public_plan_apply() {
    for (kind, inplace) in [
        (SourceKind::Git, false),
        (SourceKind::Git, true),
        (SourceKind::Override, false),
        (SourceKind::Override, true),
    ] {
        let f = Fixture::new(kind, inplace, false, false);
        let source = Source {
            resolver: f.resolver(),
            store: f._temp.path().join("store"),
        };
        let request = InstallRequest {
            roots: vec![Fixture::request()],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "direct fixture".into(),
            offline: false,
        };
        let Plan::Ready(plan) =
            vibe_install::plan(&source, &f.project, request, &NullObserver).unwrap()
        else {
            panic!("fresh plan required")
        };
        let contacts = f.contacts("a");
        f.move_tag();
        let result = vibe_install::apply(
            &source,
            *plan,
            SlotIntegrity::Verify,
            &vibe_workspace::hooks::HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap();
        assert_eq!(result.outcome.materialised.len(), 1);
        assert_eq!(
            f.contacts("a"),
            contacts,
            "apply must not reacquire moved ref"
        );
        assert!(f.contacts("b").is_empty());
        let slot = f
            .project
            .join(vibe_core::layout::current_vibedeps_root())
            .join(if inplace {
                "org.fixture.demo"
            } else {
                "org.fixture.demo/1.0.0"
            });
        assert_eq!(
            fs::read(slot.join("payload.txt")).unwrap(),
            b"direct payload\n"
        );
        assert_eq!(slot.join(".git").exists(), inplace);
        let lock = Lockfile::read(f.project.join("vibe.lock")).unwrap();
        let row = &lock.packages[0];
        assert_eq!(row.source_kind, Some(kind));
        assert_eq!(row.overridden, kind == SourceKind::Override);
        assert_eq!(row.registry, None);
        assert_eq!(row.source_ref.as_deref(), Some("refs/tags/release-pin"));
        assert_eq!(row.resolved_commit.as_deref(), Some(f.commit.as_str()));
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn exact_retained_metadata_ref_is_consumed_without_another_source_resolution() {
    let f = Fixture::new(SourceKind::Git, false, false, false);
    let resolver = f.resolver();
    let selected = resolver.resolve(&Fixture::request()).unwrap();
    assert_eq!(
        selected.source_ref.as_deref(),
        Some("refs/tags/release-pin")
    );
    f.move_tag();
    let contacts = f.contacts("a");
    let cached = resolver
        .fetch(&selected, &f._temp.path().join("store"))
        .unwrap();
    assert_eq!(cached.resolved_commit.as_deref(), Some(f.commit.as_str()));
    assert_eq!(
        fs::read(cached.cache_dir.join("payload.txt")).unwrap(),
        b"direct payload\n"
    );
    assert_eq!(f.contacts("a"), contacts);
    assert!(
        resolver
            .fetch(&selected, &f._temp.path().join("other-store"))
            .is_err()
    );
    assert_eq!(
        f.contacts("a"),
        contacts,
        "consumed lease cannot re-resolve or fall back"
    );
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn denied_direct_git_and_override_do_not_contact_any_endpoint_or_write_project() {
    for kind in [SourceKind::Git, SourceKind::Override] {
        let f = Fixture::new(kind, true, false, true);
        let before = fs::read(f.project.join("vibe.toml")).unwrap();
        let result = f.resolver().resolve(&Fixture::request());
        assert!(matches!(
            result,
            Err(RegistryError::ContactRefused(_))
                | Err(RegistryError::Git(vibe_registry::GitError::ContactRefused(
                    _
                )))
        ));
        assert!(f.contacts("a").is_empty());
        assert!(f.contacts("b").is_empty());
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), before);
        assert!(!f.project.join("vibe.lock").exists());
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn qualified_branch_survives_same_named_tag_and_legacy_update_needs_project_clarification() {
    let f = Fixture::new(SourceKind::Git, false, true, false);
    let resolver = f.resolver();
    let selected = resolver.resolve(&Fixture::request()).unwrap();
    assert_eq!(selected.source_ref.as_deref(), Some("refs/heads/main"));
    let cached = resolver
        .fetch(&selected, &f._temp.path().join("store"))
        .unwrap();
    assert_eq!(cached.resolved_commit.as_deref(), Some(f.commit.as_str()));
    let mut row: vibe_core::manifest::LockedPackage=serde_json::from_value(serde_json::json!({"kind":"feat","group":"org.fixture","name":"demo","version":"1.0.0","source_url":cached.source_uri,"source_ref":"main","resolved_commit":f.commit,"content_hash":cached.content_hash,"source_kind":"git"})).unwrap();
    let before = f.contacts("a");
    let uncleared = f
        .resolver()
        .with_git_packages(vec![])
        .with_locked_packages(vec![row.clone()])
        .with_locked_updates();
    let error = uncleared
        .resolve(&Fixture::request())
        .unwrap_err()
        .to_string();
    assert!(error.contains("project [requires.packages]"));
    assert_eq!(f.contacts("a"), before);
    // Matching authored branch input clarifies current update intent; it is not
    // reconstructed historical information and changes no held URL.
    let clarified = f
        .resolver()
        .with_locked_packages(vec![row.clone()])
        .with_locked_updates();
    assert!(clarified.resolve(&Fixture::request()).is_ok());
    row.source_ref = Some("refs/heads/main".into());
    let qualified = f
        .resolver()
        .with_git_packages(vec![])
        .with_locked_packages(vec![row])
        .with_locked_updates();
    assert!(qualified.resolve(&Fixture::request()).is_ok());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW")]
fn denied_direct_endpoint_precedes_declared_token_capture() {
    let mut f = Fixture::new(SourceKind::Git, false, false, true);
    let dep = f.manifest.requires.git_packages.first_mut().unwrap();
    dep.auth = vibe_core::manifest::AuthKind::TokenEnv;
    dep.token_env = Some("VIBE_DIRECT_DENIED_TOKEN_FIXTURE".into());
    let result = f.resolver().resolve(&Fixture::request());
    assert!(matches!(
        result,
        Err(RegistryError::ContactRefused(_))
            | Err(RegistryError::Git(vibe_registry::GitError::ContactRefused(
                _
            )))
    ));
    assert!(f.contacts("a").is_empty());
    assert!(f.contacts("b").is_empty());
}
