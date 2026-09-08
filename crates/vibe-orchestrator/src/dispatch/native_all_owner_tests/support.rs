use std::path::Path;
use std::sync::Arc;

use vibe_core::manifest::{LockedPackage, Lockfile, Materialization, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, PackageRef};
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{Phase, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use crate::install::{InstallInputs, InstallPolicy, SelectedManifest, resolve_project_root};
use crate::ports::*;
use crate::{PhaseOutcome, PhaseRun, run_phases};

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) spec_format: SpecFormat,
}

pub(super) type NativeOutputs = Vec<(String, Vec<(String, Vec<u8>)>)>;

impl Fixture {
    pub(super) fn new(conflict: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let builtin = "org.vibevm/vibe#cargo";
        write(
            &root.path().join("vibe.toml"),
            &node_manifest("root", Some(builtin), false),
        );
        write(
            &root.path().join("member/vibe.toml"),
            &node_manifest(
                "member",
                conflict.then_some("org.demo/member#foreign"),
                conflict,
            ),
        );
        write(&root.path().join("observer/src/main.rs"), "fn main() {}\n");
        write(
            &root.path().join("observer/Cargo.toml"),
            "[package]\nname='observer'\nversion='0.1.0'\nedition='2024'\nbuild='build.rs'\n",
        );
        write(
            &root.path().join("observer/build.rs"),
            "fn main(){let root=std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();assert!(!std::fs::read_to_string(root.join(\"vibevm/vibespecs/boot/STATIC.md\")).unwrap_or_default().contains(\"vibe:transforms-pending\"));std::fs::write(root.join(\"observer-ran\"),\"ok\").unwrap();}\n",
        );
        seed_compiler(root.path());
        write_lock(root.path());
        Self {
            root,
            spec_format: SpecFormat::Mixed,
        }
    }

    pub(super) fn run(&self) -> PhaseOutcome {
        let root = resolve_project_root(self.root.path()).unwrap();
        let lease = Arc::new(vibe_lifecycle::LifecycleLease::acquire(&root).unwrap());
        let phases = vec![
            Phase::Validate,
            Phase::Install,
            Phase::Generate,
            Phase::Build,
        ];
        let chain = phases.iter().map(ToString::to_string).collect::<Vec<_>>();
        let metadata = RunMetadata {
            requested: "build".into(),
            chain: chain.clone(),
            offline: true,
            assume_yes: true,
            agent_mode: RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id: vibe_lifecycle::process::allocate_run_id(&root).unwrap(),
            started: "2026-09-08T00:00:00Z".into(),
            selected: ".".into(),
        };
        let harness = Harness;
        run_phases(PhaseRun {
            requested: Phase::Build,
            phases,
            chain,
            metadata,
            install_args: InstallInputs::default(),
            policy: InstallPolicy {
                offline: true,
                ..InstallPolicy::default()
            },
            lease,
            selection: SelectedManifest::read(&root).prepare(),
            steps: Vec::new(),
            contributions: Vec::new(),
            notices: Vec::new(),
            observer: &harness,
            install_observer: &harness,
            confirm_gate: &harness,
            sources: &harness,
            environment: &harness,
            manifest_mutation: &NoManifestMutation,
            agent: Arc::new(vibe_lifecycle::NoAgentBackend),
            trace: None,
            target_os: vibe_core::manifest::TargetOs::Linux,
            deploy: None,
            observed_at: "2026-09-08T00:00:00Z".parse().unwrap(),
        })
    }

    pub(super) fn break_native_source(&self) {
        write(
            &self
                .root
                .path()
                .join("vibevm/vibedeps/org.demo.compiler/1.0.0/native/src/lib.rs"),
            "this is not valid Rust\n",
        );
    }

    pub(super) fn make_replay_fail(&self) {
        write_compiler_source(self.root.path(), true);
    }

    pub(super) fn break_authored_target(&self) {
        write(
            &self.root.path().join("observer/src/main.rs"),
            "this is not valid Rust\n",
        );
    }

    pub(super) fn native_context(&self) -> crate::install::NativeInstallContext {
        let workspace = vibe_workspace::Workspace::load(self.root.path()).unwrap();
        let slot = self
            .root
            .path()
            .join("vibevm/vibedeps/org.demo.compiler/1.0.0");
        let resolution = vec![
            vibe_workspace::install::ResolvedDep {
                kind: PackageKind::Tool,
                group: Group::parse("org.demo").unwrap(),
                name: "compiler".into(),
                version: "1.0.0".parse().unwrap(),
                content_dir: slot,
                source_hash: Some(
                    ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
                ),
                manifest: vibe_core::manifest::Manifest::read(
                    self.root
                        .path()
                        .join("vibevm/vibedeps/org.demo.compiler/1.0.0/vibe.toml"),
                )
                .unwrap(),
                requires: vec![(Group::parse("org.demo").unwrap(), "base".into())],
                admitted_by: None,
                via_override: None,
                source_mutable: false,
                in_place_changed: None,
            },
            vibe_workspace::install::ResolvedDep {
                kind: PackageKind::Tool,
                group: Group::parse("org.demo").unwrap(),
                name: "base".into(),
                version: "1.0.0".parse().unwrap(),
                content_dir: self.root.path().join("vibevm/vibedeps/org.demo.base/1.0.0"),
                source_hash: Some(
                    ContentHash::parse(&format!("sha256:{}", "b".repeat(64))).unwrap(),
                ),
                manifest: vibe_core::manifest::Manifest::read(
                    self.root
                        .path()
                        .join("vibevm/vibedeps/org.demo.base/1.0.0/vibe.toml"),
                )
                .unwrap(),
                requires: Vec::new(),
                admitted_by: None,
                via_override: None,
                source_mutable: false,
                in_place_changed: None,
            },
        ];
        let (world, lowering, sidecar) =
            crate::world::prepare_owner_runtime_inputs(self.root.path(), &workspace, &resolution)
                .unwrap();
        let platform = vibe_lifecycle::native::NativePlatform::current().unwrap();
        let facts = vibe_workspace::extension_world::OwnerRuntimeRunFacts {
            run_id: "0123456789abcdef0123456789abcdef".into(),
            state_root: self.root.path().join(".vibe"),
            platform: platform.key().into(),
            offline: true,
            created_at: "2026-09-08T00:00:00Z".into(),
        };
        let mut provider = |policies| {
            Ok(vibe_lifecycle::native::ArtifactCompilerNativeProvider::new(
                platform, policies,
            ))
        };
        let (_, carriage) = vibe_workspace::install::regenerate_boot_from_traced_native(
            &workspace,
            &resolution,
            world,
            self.spec_format,
            None,
            lowering,
            facts,
            &mut provider,
        )
        .unwrap();
        crate::install::NativeInstallContext::new(carriage, sidecar)
    }

    pub(super) fn build_and_replay(
        &self,
        context: crate::install::NativeInstallContext,
    ) -> anyhow::Result<()> {
        let platform = vibe_lifecycle::native::NativePlatform::from_key(context.platform_key())?;
        let (epoch, _) = context.parts();
        let selected = epoch.selected()?;
        super::super::native_mechanism::prepare(
            Some(&context),
            &[],
            Default::default(),
            self.root.path(),
            selected.runtime().mechanisms(),
            selected.runtime().routes(),
            platform,
            true,
            "2026-09-08T00:00:00Z",
        )?;
        if !context.replay_is_empty() {
            let mut factory = platform.replay_factory();
            context.into_carriage().replay(&mut factory)?;
        }
        Ok(())
    }

    pub(super) fn observer_exists(&self) -> bool {
        self.root.path().join("observer-ran").is_file()
    }
    pub(super) fn root(&self) -> &Path {
        self.root.path()
    }
    pub(super) fn native_record_count(&self) -> usize {
        self.native_records().len()
    }
    pub(super) fn native_records(&self) -> Vec<serde_json::Value> {
        std::fs::read_dir(self.root.path().join(".vibe/state/artifacts"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name() != "observer.json")
            .map(|entry| {
                serde_json::from_slice(&std::fs::read(entry.path()).expect("artifact record bytes"))
                    .expect("artifact record JSON")
            })
            .collect()
    }
    pub(super) fn native_outputs(&self) -> NativeOutputs {
        let boot = vibe_core::layout::current_boot_dir();
        [
            ("selected-root", self.root.path().join(&boot)),
            (
                "unselected-member",
                self.root.path().join("member").join(&boot),
            ),
            (
                "package-unit",
                self.root
                    .path()
                    .join("vibevm/vibedeps/org.demo.compiler/1.0.0")
                    .join(&boot),
            ),
        ]
        .into_iter()
        .map(|(owner, directory)| {
            let mut files = std::fs::read_dir(&directory)
                .unwrap_or_else(|error| panic!("{owner} output {}: {error}", directory.display()))
                .map(|entry| {
                    let entry = entry.expect("boot output entry");
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let bytes = std::fs::read(entry.path()).expect("boot output bytes");
                    (name, bytes)
                })
                .collect::<Vec<_>>();
            files.sort_by(|left, right| left.0.cmp(&right.0));
            (owner.to_owned(), files)
        })
        .collect()
    }

    pub(super) fn build_count(&self) -> u32 {
        std::fs::read_to_string(
            self.root
                .path()
                .join("vibevm/vibedeps/org.demo.compiler/1.0.0/native/build-count"),
        )
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
    }

    pub(super) fn output_mtimes(&self) -> Vec<std::time::SystemTime> {
        let boot = vibe_core::layout::current_boot_dir();
        let file = if matches!(self.spec_format, SpecFormat::Xml) {
            "STATIC.xml"
        } else {
            "STATIC.md"
        };
        [
            self.root.path().join(&boot).join(file),
            self.root.path().join("member").join(&boot).join(file),
            self.root
                .path()
                .join("vibevm/vibedeps/org.demo.compiler/1.0.0")
                .join(&boot)
                .join(file),
        ]
        .into_iter()
        .map(|path| std::fs::metadata(path).unwrap().modified().unwrap())
        .collect()
    }
}

pub(super) fn node_manifest(name: &str, route: Option<&str>, foreign: bool) -> String {
    let workspace = if name == "root" {
        "[workspace]\nmembers=['member']\n"
    } else {
        ""
    };
    let route = route
        .map(|pin| format!("[mechanisms]\n'build:cargo'={pin:?}\n"))
        .unwrap_or_default();
    let foreign = if foreign {
        "[[mechanism]]\nid='foreign'\nrole='build'\nname='cargo'\nprotocol=1\nconfig_schema='schemas/foreign.jtd.json'\nfreshness='provider'\nhandler={kind='native',crate_dir='foreign'}\n"
    } else {
        ""
    };
    format!(
        "[project]\ngroup='org.demo'\nname={name:?}\nversion='0.1.0'\n{workspace}[requires.packages]\n'org.demo/compiler'={{version='=1.0.0',link='static'}}\n[[extensions.use]]\nref='org.demo/compiler#native'\n{foreign}{route}[[artifacts.build]]\nid='observer-build'\nmechanism='build:cargo'\nworkdir='observer'\noutputs=[{{id='observer',kind='executable'}}]\n[[extension]]\nid='after'\npoint='phase:build'\nhandler={{kind='builtin',name='log'}}\nconfig={{message='after'}}\n"
    )
}

fn seed_compiler(root: &Path) {
    let slot = root.join("vibevm/vibedeps/org.demo.compiler/1.0.0");
    write(
        &slot.join("vibe.toml"),
        &compiler_manifest("handler={kind='native',crate_dir='native'}"),
    );
    write(&slot.join("boot/compiler.md"), "# Compiler {#root}\n");
    let base = root.join("vibevm/vibedeps/org.demo.base/1.0.0");
    write(
        &base.join("vibe.toml"),
        "[package]\ngroup='org.demo'\nname='base'\nkind='tool'\nversion='1.0.0'\n[boot_snippet]\nsource='boot/base.md'\nlink='static'\n",
    );
    write(&base.join("boot/base.md"), "# Base\n");
    let vibe_ext = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("vibe-ext")
        .display()
        .to_string()
        .replace('\\', "/");
    write(
        &slot.join("native/Cargo.toml"),
        &format!(
            "[package]\nname='compiler-fixture'\nversion='1.0.0'\nedition='2024'\n[lib]\ncrate-type=['cdylib']\n[dependencies]\nvibe-ext={{path={vibe_ext:?}}}\n"
        ),
    );
    write_compiler_source(root, false);
}

pub(super) fn compiler_manifest(handler: &str) -> String {
    format!(
        "[package]\ngroup='org.demo'\nname='compiler'\nkind='tool'\nversion='1.0.0'\n[boot_snippet]\nsource='boot/compiler.md'\nlink='static'\n[requires.packages]\n'org.demo/base'={{version='=1.0.0',link='static'}}\n[[extension]]\nid='native'\npoint='compile:emitted'\n{handler}\n"
    )
}

fn write_compiler_source(root: &Path, fail: bool) {
    let body = if fail {
        "use vibe_ext::{CompileReply,CompileReplyFail,CompileRequest,Manifest,ManifestExtension};fn manifest()->Manifest{Manifest{extensions:vec![ManifestExtension{id:\"native\".into(),point:\"compile:emitted\".into(),ir_schema:Some(1)}]}}fn handle(_:CompileRequest)->CompileReply{CompileReply::Fail(Box::new(CompileReplyFail{envelope:1,message:Some(\"replay refusal\".into())}))}vibe_ext::vibe_compile_extension!(manifest=manifest(),handler=handle);\n"
    } else {
        "use vibe_ext::{CompileReply,CompileReplyOk,CompileRequest,Manifest,ManifestExtension};fn manifest()->Manifest{Manifest{extensions:vec![ManifestExtension{id:\"native\".into(),point:\"compile:emitted\".into(),ir_schema:Some(1)}]}}fn handle(r:CompileRequest)->CompileReply{CompileReply::Ok(Box::new(CompileReplyOk{envelope:1,payload:r.payload,message:None}))}vibe_ext::vibe_compile_extension!(manifest=manifest(),handler=handle);\n"
    };
    write(
        &root.join("vibevm/vibedeps/org.demo.compiler/1.0.0/native/src/lib.rs"),
        body,
    );
}

fn write_lock(root: &Path) {
    let dep = PackageRef::parse("org.demo/compiler@=1.0.0").unwrap();
    let base = PackageRef::parse("org.demo/base@=1.0.0").unwrap();
    let mut lock = Lockfile::empty("fixture", "2026-09-08T00:00:00Z");
    lock.meta.root_dependencies = vec![dep];
    lock.packages.push(LockedPackage {
        kind: PackageKind::Tool,
        name: PackageName::parse("compiler").unwrap(),
        group: Group::parse("org.demo").unwrap(),
        version: "1.0.0".parse().unwrap(),
        registry: None,
        source_url: "file:///fixture".into(),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: vec![base.clone()],
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    });
    lock.packages.push(LockedPackage {
        kind: PackageKind::Tool,
        name: PackageName::parse("base").unwrap(),
        group: Group::parse("org.demo").unwrap(),
        version: "1.0.0".parse().unwrap(),
        registry: None,
        source_url: "file:///fixture".into(),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse(&format!("sha256:{}", "b".repeat(64))).unwrap(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: Vec::new(),
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    });
    lock.write(root.join("vibe.lock")).unwrap();
}

pub(super) fn write(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

struct Harness;
impl RegistryEnvironment for Harness {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: None,
            global: vibe_core::GlobalRegistryConfig::load()?,
        })
    }
}
impl PackageSourceFactory for Harness {
    fn build(&self, _: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        Ok(Box::new(DummySource))
    }
}

struct DummySource;

impl PackageSource for DummySource {
    fn qualify(&self, pkgref: &PackageRef, _locked: &Lockfile) -> anyhow::Result<PackageRef> {
        Ok(pkgref.clone())
    }
}

impl vibe_install::InstallSource for DummySource {
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<vibe_registry::CachedPackage, vibe_registry::RegistryError> {
        unreachable!("fresh fixture never fetches")
    }
    fn solve(
        &self,
        _: &[PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never solves")
    }
    fn manifest_of(
        &self,
        _: &PackageRef,
    ) -> Result<vibe_core::manifest::Manifest, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never reads source metadata")
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &std::collections::BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        unreachable!("fresh fixture never solves masked")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, vibe_registry::RegistryError> {
        unreachable!("fresh fixture never materialises in place")
    }
}
impl ConfirmGate for Harness {
    fn confirm_install(&self, _: usize) -> anyhow::Result<()> {
        Ok(())
    }
}
impl RunObserver for Harness {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Null
    }
    fn binary_quiet(&self) -> bool {
        true
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn observe_plan(&self, _: &crate::RitualPlan, _: &RunMetadata, _: bool) -> anyhow::Result<()> {
        Ok(())
    }
    fn observe_contribution(
        &self,
        _: &vibe_wire::generated::lifecycle_report::LifecycleContributionReport,
    ) {
    }
    fn observe_untracked_failure(
        &self,
        _: &RunMetadata,
        _: &str,
        _: &[vibe_wire::generated::lifecycle_report::LifecycleContributionReport],
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
impl InstallObserver for Harness {
    fn stream_mode(&self) -> StreamMode {
        StreamMode::Null
    }
    fn emit_machine_failure(&self) -> bool {
        false
    }
    fn narrate(&self, _: crate::ports::InstallNarration<'_>) {}
    fn lane_sizes(&self, _: &Path) -> Vec<(String, Option<u64>)> {
        Vec::new()
    }
    fn plan_events(&self) -> &dyn vibe_install::PlanObserver {
        self
    }
    fn slot_observer(&self, _: &RunMetadata) -> Arc<dyn vibe_install::SlotLifecycleObserver> {
        Arc::new(Harness)
    }
}
impl vibe_install::PlanObserver for Harness {
    fn on(&self, _: vibe_install::PlanEvent) {}
}
impl vibe_install::SlotLifecycleObserver for Harness {
    fn observe(&self, _: &vibe_install::SlotLifecyclePlan) -> Result<(), String> {
        Ok(())
    }
    fn outcome(&self, _: &vibe_install::SlotLifecycleReport) -> Result<(), String> {
        Ok(())
    }
}
