//! Actual submitted plan/apply; existing safefs test-only post-publication fault.
#![cfg(unix)]
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use vibe_core::{
    PackageRef,
    manifest::{Lockfile, Manifest, SpecFormat, SubmittedInput, SubmittedSource},
    user_config::SlotIntegrity,
};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, InPlaceMaterialised, LocalRegistry, RegistryError};
use vibe_resolver::{
    DepSolver, FeatureRequest, LocalRegistryDepProvider, ResolvedGraph, ResolvoDepSolver,
    SolveError,
};
#[path = "support/retained.rs"]
mod retained;
struct Source {
    registry: LocalRegistry,
    store: PathBuf,
}
impl InstallSource for Source {
    fn source_solve_limits(&self) -> Result<vibe_resolver::SourceSolveLimits, SolveError> {
        retained::limits()
    }
    fn solve_metadata(
        &self,
        r: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        retained::solve(self.registry.root(), r)
    }
    fn acquire_candidate_for_install(
        &self,
        c: &vibe_registry::source_candidates::RetainedSourceCandidate,
        s: &Path,
        h: Option<&str>,
        slot: &Path,
        p: &vibe_core::progress::Progress,
    ) -> Result<vibe_registry::PackageAcquisition, RegistryError> {
        self.acquire_for_install(&retained::exact(c), s, h, slot, p)
    }
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let mut f = self
            .registry
            .fetch(&self.registry.resolve(p)?, &self.store)?;
        f.registry_name = Some("fixture".into());
        Ok(f)
    }
    fn solve(&self, r: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        ResolvoDepSolver::new(LocalRegistryDepProvider::new(&self.registry)).solve(r)
    }
    fn manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        vibe_install::metadata_manifest(&LocalRegistryDepProvider::new(&self.registry), p)
    }
    fn solve_masked(
        &self,
        r: &[PackageRef],
        b: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        vibe_install::solve_with_visibility_mask(
            LocalRegistryDepProvider::new(&self.registry),
            None,
            r,
            b,
            true,
        )
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        panic!("copy-only subject")
    }
}
#[derive(Default, Debug)]
struct Seen {
    lock: bool,
    submitted: bool,
    slot: bool,
    boot: bool,
    pre_hook: bool,
}
fn observe_lock(root: PathBuf, seen: Arc<Mutex<Seen>>) {
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |parent, name| {
        if name != "vibe.lock" {
            observe_lock(root.clone(), seen.clone());
            return;
        }
        let lock =
            Lockfile::read(parent.path().join(name)).expect("real lock published before fault");
        let slot = vibe_workspace::vibedeps::slot_abs_path(
            &root,
            &vibe_core::Group::parse("org.vibevm").unwrap(),
            "submitted",
            &"1.0.0".parse().unwrap(),
        );
        let mut s = seen.lock().unwrap();
        s.lock = true;
        s.submitted = lock.meta.schema_version == 16
            && lock.configurations["org.vibevm/submitted"]
                .submitted_inputs
                .contains_key("./vibe.toml");
        let record = &lock.configurations["org.vibevm/submitted"];
        assert!(record.consumers.is_empty());
        assert_eq!(
            record.submitted_inputs["./vibe.toml"].original.parameters["verbose"],
            vibe_core::manifest::ParameterValue::Bool(false)
        );
        assert_eq!(
            record.submitted_inputs["./vibe.toml"].original.selections["mode"]
                .members
                .as_deref(),
            Some(&["on".to_owned()][..])
        );
        assert!(
            !root.join("vibe.lock.provenance").exists(),
            "fault observation reached later provenance write"
        );
        s.slot = slot.join("payload.txt").is_file();
        s.boot = walk_files(&root.join(vibe_core::layout::current_boot_dir()))
            .values()
            .any(|b| String::from_utf8_lossy(b).contains("SUBMITTED_BOOT"));
        s.pre_hook = root.join("external-hook.log").is_file();
    })));
}
fn walk_files(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, p: &Path, out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        if p.is_dir() {
            for e in fs::read_dir(p).unwrap() {
                visit(base, &e.unwrap().path(), out)
            }
        } else if p.is_file() {
            out.insert(
                p.strip_prefix(base).unwrap().to_owned(),
                fs::read(p).unwrap(),
            );
        }
    }
    let mut out = Default::default();
    if root.exists() {
        visit(root, root, &mut out)
    }
    out
}
struct Disarm;
impl Drop for Disarm {
    fn drop(&mut self) {
        vibe_safefs::fail_after_publish(None);
        vibe_safefs::arm_before_publish_verify(None);
    }
}
#[test]
fn submitted_lock_post_publication_fault_reaches_real_record_and_rolls_back_owned_state() {
    for fault in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let registry = root.join("registry");
        let package = registry.join("org.vibevm/submitted/v1.0.0");
        fs::create_dir_all(package.join("boot")).unwrap();
        fs::create_dir(package.join("hooks")).unwrap();
        fs::write(package.join("vibe.toml"),"[package]\ngroup='org.vibevm'\nname='submitted'\nversion='1.0.0'\nkind='flow'\n[boot_snippet]\nsource='boot/rules.md'\n[hooks]\npre-install='hooks/pre'\npost-install='hooks/post'\n[[configuration.features]]\nid='off'\nname='Off'\n[[configuration.features]]\nid='on'\nname='On'\n[[configuration.groups]]\nmode='choice'\nid='mode'\nname='Mode'\ncardinality='exactly-one'\nmembers=['off','on']\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n").unwrap();
        fs::write(package.join("boot/rules.md"), "SUBMITTED_BOOT\n").unwrap();
        fs::write(package.join("payload.txt"), "synthetic payload").unwrap();
        for (name, mark) in [("pre", "pre"), ("post", "post")] {
            fs::write(
                package.join(format!("hooks/{name}.sh")),
                format!(
                    "set -eu\nprintf '%s\\n' '{mark}' >> '{}'\n",
                    root.join("external-hook.log").display()
                ),
            )
            .unwrap();
        }
        let authored = format!(
            "[project]\nname='submitted-fault'\nversion='0.1.0'\n[requires.packages.'org.vibevm/submitted']\nversion='=1.0.0'\nlink='static-transitive'\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
            registry.display()
        );
        fs::write(root.join("vibe.toml"), &authored).unwrap();
        fs::write(root.join("AGENTS.md"), "# Prior authored launcher\n").unwrap();
        let before = walk_files(&root);
        let before_boot = walk_files(&root.join(vibe_core::layout::current_boot_dir()));
        let source = Source {
            registry: LocalRegistry::new(&registry).unwrap(),
            store: temp.path().join("store"),
        };
        let hash = vibe_registry::compute_content_hash(&package).unwrap();
        let original=toml::from_str("policy='require-selection'\ndefault_features=false\nselections.mode={intent='exact',members=['on']}\nparameters.verbose=false\n").unwrap();
        let submitted = SubmittedInput {
            original,
            requirement: PackageRef::parse("org.vibevm/submitted@=1.0.0").unwrap(),
            project_witness: vibe_core::manifest::submitted_project_witness(
                &root,
                authored.as_bytes(),
            )
            .unwrap(),
            expected_source: SubmittedSource {
                registry: "fixture".into(),
                content_hash: vibe_core::ContentHash::parse(&hash).unwrap(),
            },
        };
        let mut manifest = Manifest::read(root.join("vibe.toml")).unwrap();
        let mut workspace = vibe_workspace::Workspace::discover(&root).unwrap();
        let request = InstallRequest {
            roots: vec![],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "submitted publication fault fixture".into(),
            offline: true,
            no_recommends: true,
        };
        let plan = vibe_install::plan_prepared_with_submission(
            &source,
            &root,
            &mut manifest,
            &mut workspace,
            request,
            SpecFormat::Mixed,
            &NullObserver,
            submitted,
        )
        .unwrap();
        let Plan::Ready(planned) = plan else {
            panic!("real submitted plan must be Ready")
        };
        // This direct integration test explicitly consents by invoking apply after
        // Ready. It is not a terminal/CLI confirmation test or an OS error injection.
        assert!(!root.join("vibe.lock").exists());
        let seen = Arc::new(Mutex::new(Seen::default()));
        observe_lock(root.clone(), seen.clone());
        let _disarm = Disarm;
        if fault {
            vibe_safefs::fail_after_publish(Some("vibe.lock"));
        }
        let lease = Arc::new(vibe_lifecycle::LifecycleLease::acquire(&root).unwrap());
        let run_id = vibe_lifecycle::process::allocate_run_id(&root).unwrap();
        let metadata = vibe_lifecycle::RunMetadata {
            requested: "install".into(),
            chain: vec!["validate".into(), "install".into()],
            offline: true,
            assume_yes: true,
            agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
            force: false,
            trace_compile: false,
            run_id,
            started: "2026-11-20T00:00:00Z".into(),
            selected: ".".into(),
        };
        let result = vibe_install::apply_with_spec_format_and_lifecycle_observed(
            &source,
            *planned,
            SlotIntegrity::Verify,
            SpecFormat::Mixed,
            metadata,
            vibe_lifecycle::process::StreamMode::Capture,
            vibe_install::SlotLifecycleSeams::refusing(),
            lease,
        );
        vibe_safefs::fail_after_publish(None);
        vibe_safefs::arm_before_publish_verify(None);
        let seen = seen.lock().unwrap();
        assert!(
            seen.lock && seen.submitted && seen.slot && seen.boot && seen.pre_hook,
            "precise publication boundary not reached: {seen:?}"
        );
        let slot = vibe_workspace::vibedeps::slot_abs_path(
            &root,
            &vibe_core::Group::parse("org.vibevm").unwrap(),
            "submitted",
            &"1.0.0".parse().unwrap(),
        );
        assert_eq!(
            fs::read_to_string(root.join("vibe.toml")).unwrap(),
            authored
        );
        if fault {
            let error = result.unwrap_err();
            let diagnostic = error.to_string();
            assert!(
                diagnostic.contains("injected post-publication failure for `vibe.lock`"),
                "wrong failure: {diagnostic}"
            );
            let vibe_install::Error::SlotFailed {
                source,
                reports,
                progress,
            } = &error
            else {
                panic!("canonical fault lacks lifecycle carrier: {diagnostic}")
            };
            assert!(
                !matches!(source.as_ref(), vibe_install::Error::RollbackFailed { .. }),
                "owned rollback failed: {diagnostic}"
            );
            assert!(
                reports
                    .iter()
                    .any(|r| r.point == "slot:pre-install" && r.status == "ok"),
                "executed pre-hook absent from failure report: {reports:?}"
            );
            assert!(
                !reports
                    .iter()
                    .any(|r| r.point == "slot:post-install" && r.status == "ok"),
                "post-hook reported after failed publication"
            );
            assert!(
                !progress.complete && !progress.fresh,
                "failed publication reported complete/fresh"
            );
            println!("canonical failure reports={reports:?} progress={progress:?}");
            assert!(!root.join("vibe.lock").exists());
            assert!(!root.join("vibe.lock.provenance").exists());
            assert!(!slot.exists());
            assert_eq!(
                walk_files(&root.join(vibe_core::layout::current_boot_dir())),
                before_boot
            );
            assert_eq!(
                fs::read(root.join("AGENTS.md")).unwrap(),
                before[Path::new("AGENTS.md")]
            );
            assert_eq!(
                fs::read_to_string(root.join("external-hook.log")).unwrap(),
                "pre\n"
            );
            println!(
                "submitted publication fault: visible epoch16 record observed; primary={diagnostic}; owned lock/slot/boot restored; external pre-hook retained; post-hook absent"
            );
        } else {
            let report = result.unwrap();
            assert!(
                report.progress.complete,
                "matched successful control incomplete"
            );
            let lock = Lockfile::read(root.join("vibe.lock")).unwrap();
            assert_eq!(lock.meta.schema_version, 16);
            assert!(
                lock.configurations["org.vibevm/submitted"]
                    .submitted_inputs
                    .contains_key("./vibe.toml")
            );
            assert!(
                report
                    .lifecycle_reports
                    .iter()
                    .any(|r| r.point == "slot:post-install" && r.status == "ok")
            );
            assert!(root.join("vibe.lock").exists());
            assert!(slot.exists());
            assert_eq!(
                fs::read_to_string(root.join("external-hook.log")).unwrap(),
                "pre\npost\n"
            );
            println!(
                "submitted publication control: epoch16 recorded, payload/boot/hooks complete"
            );
        }
    }
}
