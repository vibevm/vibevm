use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{
    ExtensionHandler, ExtensionsControl, Manifest, MechanismDecl, MechanismFreshness,
    MechanismRole, MechanismRoutes, ProviderPin,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_lifecycle::native::{
    NativeBuildExecution, NativePlatform, preflight_native_mechanisms, project_native_mechanisms,
};
use vibe_lifecycle::{
    ClientExecutable, ClientExecutables, DependencyExtensionSource, DependencyProvider,
    DependencyProviderId, DeployExecution, DeploySelection, ExtensionWorld, HostExtensionSource,
    HostIdentity, HostProvider, PackageExecution, collect_mechanisms, deploy_state_home,
    execute_deploy_targets, execute_package_targets, list_deployments,
};

const PIN: &str = "org.example/native-deploy#fixture-vibe-bin";
const NOW: &str = "2026-09-08T12:00:00Z";

struct Fixture {
    _root: TempDir,
    project: PathBuf,
    settings: PathBuf,
    state_home: PathBuf,
    user_home: PathBuf,
    manifest: Manifest,
    registry: vibe_lifecycle::MechanismRegistry,
    routes: MechanismRoutes,
    platform: NativePlatform,
    prepared: vibe_lifecycle::native::PreparedNativeMechanisms,
    clients: ClientExecutables,
}

impl Fixture {
    fn new() -> Self {
        assert_eq!(
            vibe_native_loader_mechanism_fixture::fixture_marker(),
            "vibe-native-loader-mechanism-fixture"
        );
        let root = TempDir::new().unwrap();
        let project = root.path().join("project");
        let settings = root.path().join("settings");
        let user_home = root.path().join("home");
        let provider_root = project.join("vibevm/vibedeps/org.example.native-deploy/1.0.0");
        std::fs::create_dir_all(provider_root.join("native")).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&user_home).unwrap();
        std::fs::write(project.join("payload.txt"), "a".repeat(64)).unwrap();

        let platform = NativePlatform::current().unwrap();
        let library = fixture_library();
        let relative = PathBuf::from(format!(
            "native/{}",
            library.file_name().unwrap().to_string_lossy()
        ));
        std::fs::copy(&library, provider_root.join(&relative)).unwrap();
        std::fs::write(provider_root.join("schema.jtd.json"), "{}").unwrap();

        let mechanism = MechanismDecl {
            id: "fixture-vibe-bin".to_owned(),
            role: MechanismRole::Deploy,
            name: "vibe-bin".to_owned(),
            handler: ExtensionHandler::Native {
                crate_dir: None,
                prebuilt: Some(BTreeMap::from([(platform.key().to_owned(), relative)])),
            },
            protocol: 1,
            config_schema: PathBuf::from("schema.jtd.json"),
            freshness: MechanismFreshness::Provider,
        };
        let provider = DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(
                    Group::parse("org.example").unwrap(),
                    PackageName::parse("native-deploy").unwrap(),
                ),
                root: provider_root,
                version: "1.0.0".to_owned(),
                kind: PackageKind::Tool,
                content_hash: ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: vec![mechanism],
        };
        let world = ExtensionWorld {
            installed: vec![provider],
            host: HostExtensionSource {
                provider: HostProvider {
                    identity: HostIdentity::ungrouped_project("demo"),
                    root: project.clone(),
                    version: "1.0.0".to_owned(),
                    kind: None,
                    content_hash: None,
                },
                declarations: Vec::new(),
                controls: ExtensionsControl::default(),
                mechanisms: Vec::new(),
            },
            effective_stack: None,
        };
        let registry = collect_mechanisms(&world).unwrap();
        let mut routes = MechanismRoutes::default();
        routes.insert(
            "deploy:vibe-bin".parse().unwrap(),
            ProviderPin::parse(PIN).unwrap(),
        );
        let manifest = Manifest::parse_str(MANIFEST).unwrap();
        let artifacts = manifest.artifacts.as_ref().unwrap();
        execute_package_targets(&PackageExecution {
            project_root: &project,
            targets: &artifacts.package,
            registry: &registry,
            routes: &routes,
            package_root: PackageExecution::default_package_root(),
            created_at: NOW,
        })
        .unwrap();
        let plan = project_native_mechanisms(
            &artifacts.package,
            &manifest.deploy.as_ref().unwrap().targets,
            &registry,
            &routes,
        )
        .unwrap();
        let native = NativeBuildExecution {
            candidates: &[],
            selected_project_root: &project,
            registry: &registry,
            routes: &routes,
            platform,
            offline: true,
            created_at: NOW,
        };
        let prepared = preflight_native_mechanisms(plan, &native)
            .unwrap()
            .prepare(&native)
            .unwrap();
        let state_home = deploy_state_home(&settings);
        Self {
            _root: root,
            project,
            settings,
            state_home,
            user_home,
            manifest,
            registry,
            routes,
            platform,
            prepared,
            clients: ClientExecutables {
                claude: ClientExecutable::Missing {
                    command: "claude".to_owned(),
                },
                codex: ClientExecutable::Missing {
                    command: "codex".to_owned(),
                },
                opencode: ClientExecutable::Missing {
                    command: "opencode".to_owned(),
                },
            },
        }
    }

    fn selection(&self, profile: &str, targets: &[&str]) -> DeploySelection {
        DeploySelection {
            profile: profile.to_owned(),
            targets: targets.iter().map(|target| (*target).to_owned()).collect(),
        }
    }

    fn execute<'a>(&'a self, selection: &'a DeploySelection) -> DeployExecution<'a> {
        DeployExecution {
            project_root: &self.project,
            targets: &self.manifest.deploy.as_ref().unwrap().targets,
            selection,
            registry: &self.registry,
            routes: &self.routes,
            state_home: &self.state_home,
            settings_root: &self.settings,
            user_home: &self.user_home,
            clients: &self.clients,
            project: "demo",
            package: None,
            created_at: NOW,
        }
    }

    fn rehydrate(
        &self,
        selection: &DeploySelection,
    ) -> Result<vibe_lifecycle::native::PreparedNativeMechanisms, String> {
        let artifacts = self.manifest.artifacts.as_ref().unwrap();
        let plan = project_native_mechanisms(
            &artifacts.package,
            &self.manifest.deploy.as_ref().unwrap().targets,
            &self.registry,
            &self.routes,
        )
        .map_err(|error| error.to_string())?;
        let native = NativeBuildExecution {
            candidates: &[],
            selected_project_root: &self.project,
            registry: &self.registry,
            routes: &self.routes,
            platform: self.platform,
            offline: true,
            created_at: NOW,
        };
        let prepared = preflight_native_mechanisms(plan, &native)
            .and_then(|preflight| preflight.rehydrate(&native))
            .map_err(|error| error.to_string())?;
        prepared
            .validate_restart(&self.execute(selection))
            .map_err(|error| error.to_string())?;
        Ok(prepared)
    }
}

#[test]
fn real_package_native_provider_runs_all_six_operations_without_builtin_fallback() {
    let fixture = Fixture::new();

    let normal = fixture.selection("normal", &["normal"]);
    let missing = execute_deploy_targets(&fixture.execute(&normal)).unwrap_err();
    assert!(matches!(
        missing,
        vibe_lifecycle::DeployError::Provider(vibe_lifecycle::MechanismError::Deploy(
            vibe_lifecycle::DeployProviderError::NativeTransport { .. }
        ))
    ));
    assert!(!fixture.settings.join("bin").exists());
    assert!(!fixture.state_home.exists());

    let pure = fixture.selection("pure", &["pure-a", "pure-b"]);
    let error = fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&pure))
        .unwrap_err();
    assert!(error.to_string().contains("both own"), "{error}");
    assert!(!fixture.settings.join("native-provider/pure").exists());
    assert!(!deploy_state_home(&fixture.settings).exists());

    let bad_lock = fixture.selection("bad-lock", &["bad-lock"]);
    let error = fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&bad_lock))
        .unwrap_err();
    assert!(error.to_string().contains("canonical forward-slashed"));
    assert!(!fixture.settings.join("native-provider/bad-lock").exists());
    assert!(!fixture.state_home.exists());

    for target in ["logical", "backslash"] {
        let selection = fixture.selection(target, &[target]);
        let outcomes = fixture
            .prepared
            .execute_deploy_targets(&fixture.execute(&selection))
            .unwrap();
        let physical = fixture
            .settings
            .join(format!("native-provider/{target}"))
            .to_string_lossy()
            .replace('\\', "/");
        let expected = if target == "backslash" {
            format!("home:{}", physical.replace('/', "\\"))
        } else {
            format!("home:{physical}")
        };
        assert_eq!(outcomes[0].resources[0].resource, expected);
        assert!(Path::new(&physical).exists());
        fixture
            .prepared
            .undeploy_targets(&fixture.execute(&selection))
            .unwrap();
        assert!(!Path::new(&physical).exists());
    }

    let outcomes = fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&normal))
        .unwrap();
    assert_eq!(outcomes[0].provider, PIN);
    assert_eq!(
        outcomes[0].displaced_default.as_deref(),
        Some("org.vibevm/vibe#vibe-bin")
    );
    assert!(fixture.settings.join("native-provider/normal").exists());
    assert!(
        !fixture.settings.join("bin").exists(),
        "builtin destination stayed untouched"
    );
    let receipts = list_deployments(&deploy_state_home(&fixture.settings)).unwrap();
    assert_eq!(
        receipts
            .iter()
            .find(|row| row.target == "normal")
            .unwrap()
            .provider,
        PIN
    );

    fixture
        .prepared
        .undeploy_targets(&fixture.execute(&normal))
        .unwrap();
    assert!(!fixture.settings.join("native-provider/normal").exists());

    let interrupted = fixture.selection("interrupted", &["interrupt"]);
    let first = fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&interrupted))
        .unwrap_err();
    assert!(
        first
            .to_string()
            .contains("interruption after destination write")
    );
    assert!(fixture.settings.join("native-provider/interrupt").exists());
    let recovered = fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&interrupted))
        .unwrap();
    assert_eq!(recovered[0].settlement, "recovered");
    fixture
        .prepared
        .undeploy_targets(&fixture.execute(&interrupted))
        .unwrap();
    assert!(!fixture.settings.join("native-provider/interrupt").exists());
    assert_no_transient_state(&deploy_state_home(&fixture.settings));
}

#[test]
fn process_restart_rehydrates_plan_undeploy_and_interrupted_recovery_read_only() {
    let fixture = Fixture::new();
    let normal = fixture.selection("normal", &["normal"]);
    let before = snapshot(&fixture.project);
    let restarted = fixture.rehydrate(&normal).unwrap();
    let reports = restarted
        .plan_deploy_targets(&fixture.execute(&normal))
        .unwrap();
    assert_eq!(reports.len(), 1);
    assert!(!fixture.state_home.exists(), "plan creates no state home");
    assert_eq!(snapshot(&fixture.project), before);

    fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&normal))
        .unwrap();
    fixture
        .rehydrate(&normal)
        .unwrap()
        .undeploy_targets(&fixture.execute(&normal))
        .unwrap();
    assert!(!fixture.settings.join("native-provider/normal").exists());

    let interrupted = fixture.selection("interrupted", &["interrupt"]);
    fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&interrupted))
        .unwrap_err();
    let recovered = fixture
        .rehydrate(&interrupted)
        .unwrap()
        .execute_deploy_targets(&fixture.execute(&interrupted))
        .unwrap();
    assert_eq!(recovered[0].settlement, "recovered");
}

#[test]
fn restart_refuses_binding_route_provider_and_image_drift_without_repair() {
    let fixture = Fixture::new();
    let normal = fixture.selection("normal", &["normal"]);
    fixture
        .prepared
        .execute_deploy_targets(&fixture.execute(&normal))
        .unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(
        &std::fs::read(find_named(&fixture.state_home, "receipt.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["provider"]["version"], "1.0.0");
    assert_eq!(
        receipt["provider"]["content_hash"],
        format!("sha256:{}", "a".repeat(64))
    );
    let sidecar = find_named(&fixture.state_home, "lock-resources.json");
    let original = std::fs::read(&sidecar).unwrap();
    for (field, value) in [
        ("target", serde_json::json!("other")),
        ("mechanism", serde_json::json!("deploy:other")),
        ("pin", serde_json::json!("org.example/other#fixture")),
        ("descriptor_id", serde_json::json!("other")),
        ("protocol", serde_json::json!(2)),
        ("provider_version", serde_json::json!("2.0.0")),
        (
            "provider_hash",
            serde_json::json!(format!("sha256:{}", "b".repeat(64))),
        ),
        ("platform", serde_json::json!("other-x86_64")),
        ("record_root", serde_json::json!("project")),
        ("origin", serde_json::json!("source-record")),
        ("image", serde_json::json!(".vibe/native-load/e1/other.dll")),
        ("image_digest", serde_json::json!("b".repeat(64))),
    ] {
        let mut document: serde_json::Value = serde_json::from_slice(&original).unwrap();
        document["committed"]["native"][field] = value;
        std::fs::write(&sidecar, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        let before = snapshot(&fixture.state_home);
        assert!(
            fixture
                .prepared
                .validate_restart(&fixture.execute(&normal))
                .is_err()
        );
        assert_eq!(snapshot(&fixture.state_home), before, "{field}");
    }
    std::fs::write(&sidecar, &original).unwrap();

    let mut changed_routes = fixture.routes.clone();
    changed_routes.insert(
        "deploy:vibe-bin".parse().unwrap(),
        ProviderPin::parse("org.vibevm/vibe#vibe-bin").unwrap(),
    );
    let mut changed = fixture.execute(&normal);
    changed.routes = &changed_routes;
    assert!(fixture.prepared.validate_restart(&changed).is_err());
    let empty = collect_mechanisms(&ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: fixture.project.clone(),
                version: "1.0.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap();
    changed.registry = &empty;
    assert!(fixture.prepared.validate_restart(&changed).is_err());

    for stale in [false, true] {
        let fixture = Fixture::new();
        let image = fixture.prepared.entries[0].image.clone();
        if stale {
            std::fs::write(&image, b"stale image").unwrap();
        } else {
            std::fs::remove_file(&image).unwrap();
        }
        let before = snapshot(&fixture.project);
        assert!(
            fixture
                .rehydrate(&fixture.selection("normal", &["normal"]))
                .is_err()
        );
        assert_eq!(snapshot(&fixture.project), before);
    }
}

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = Vec::new();
    collect_files(root, root, &mut rows);
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows
}

fn find_named(root: &Path, name: &str) -> PathBuf {
    let mut files = Vec::new();
    collect_files(root, root, &mut files);
    root.join(
        files
            .into_iter()
            .find(|(path, _)| {
                Path::new(path)
                    .file_name()
                    .is_some_and(|value| value == name)
            })
            .unwrap()
            .0,
    )
}

fn collect_files(root: &Path, directory: &Path, rows: &mut Vec<(String, Vec<u8>)>) {
    for entry in std::fs::read_dir(directory).into_iter().flatten().flatten() {
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            collect_files(root, &path, rows);
        } else {
            rows.push((
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                std::fs::read(path).unwrap(),
            ));
        }
    }
}

fn assert_no_transient_state(path: &Path) {
    if !path.exists() {
        return;
    }
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            assert_ne!(entry.file_name(), std::ffi::OsStr::new("staging"));
            assert_no_transient_state(&entry.path());
        } else {
            let name = entry.file_name();
            assert!(
                !matches!(
                    name.to_str(),
                    Some("intent.json" | "checkpoints.json" | "inverse.json")
                ),
                "transient deployment state survived at {}",
                entry.path().display()
            );
        }
    }
}

fn fixture_library() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let executable_dir = executable.parent().unwrap();
    let profile = if executable_dir
        .file_name()
        .is_some_and(|name| name == "deps")
    {
        executable_dir.parent().unwrap()
    } else {
        executable_dir
    };
    let exact = format!(
        "{}vibe_native_loader_mechanism_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    [profile.to_path_buf(), profile.join("deps")]
        .into_iter()
        .flat_map(|directory| std::fs::read_dir(directory).into_iter().flatten().flatten())
        .map(|entry| entry.path())
        .find(|path| path.file_name().is_some_and(|name| name == exact.as_str()))
        .expect("one real native mechanism fixture")
}

const MANIFEST: &str = r#"
[project]
name = "demo"
version = "1.0.0"

[[artifacts.package]]
id = "payload-package"
mechanism = "package:static-file"
inputs = [{ path = "payload.txt" }]
outputs = [{ id = "payload.txt", kind = "file" }]
[[deploy.target]]
id = "normal"
artifact = "payload.txt"
mechanism = "deploy:vibe-bin"
config = { foreign = true }
[[deploy.target]]
id = "interrupt"
artifact = "payload.txt"
mechanism = "deploy:vibe-bin"
config = { foreign = true }
[[deploy.target]]
id = "pure-a"
artifact = "payload.txt"
mechanism = "deploy:vibe-bin"
config = { foreign = true }
[[deploy.target]]
id = "pure-b"
artifact = "payload.txt"
mechanism = "deploy:vibe-bin"
config = { foreign = true }
[[deploy.target]]
id = "logical"
artifact = "payload.txt"
mechanism = "deploy:vibe-bin"
config = { foreign = true }
[[deploy.target]]
id = "backslash"
artifact = "payload.txt"
mechanism = "deploy:vibe-bin"
config = { foreign = true }
[[deploy.target]]
id = "bad-lock"
artifact = "payload.txt"
mechanism = "deploy:vibe-bin"
config = { foreign = true }
"#;
