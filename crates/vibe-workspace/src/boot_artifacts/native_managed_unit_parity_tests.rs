use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use serde_json::Value;
use vibe_core::manifest::{LinkType, Manifest, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_extension_registry::DependencyProviderId;
use vibe_spec::{
    CompilerNativeCall, CompilerNativeImplementationDigest, CompilerNativeInvoker,
    CompilerNativeInvokerError,
};
use vibe_wire::generated::shared::Timestamp;

use super::*;
use crate::extension_world::{
    CompilerNativeFactBinding, CompilerNativeFactError, OwnerNativeCompileBinding,
    OwnerNativeCompileProvider, OwnerRuntimeView, PendingBuildFact,
};

struct CatalogBinding(super::support::FakeBinding);

impl CompilerNativeInvoker for CatalogBinding {
    fn admit_backend(
        &self,
        _key: &vibe_core::manifest::ExtensionKey,
        _order: u32,
        _config: &BTreeMap<String, Option<Value>>,
        _implementation: CompilerNativeImplementationDigest,
        backend: &str,
    ) -> Result<(), CompilerNativeInvokerError> {
        assert_eq!(backend, "selected-test");
        Ok(())
    }

    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        if call.backend() == Some("selected-test") {
            let observed = format!("{:?}", call.payload());
            assert!(observed.contains("native-selected XML"), "{observed}");
            let reply =
                vibe_wire::behaviour::native_backend::ok_reply(b"selected backend ready", None)
                    .unwrap();
            return Ok(serde_json::to_vec(&reply).unwrap());
        }
        self.0.invoke(call)
    }
}

impl CompilerNativeFactBinding for CatalogBinding {
    fn invoker(&self) -> &dyn CompilerNativeInvoker {
        self
    }

    fn take_pending_build_facts(
        &self,
        pending: &vibe_spec::CompilerPendingSet,
    ) -> Result<Vec<PendingBuildFact>, CompilerNativeFactError> {
        self.0.take_pending_build_facts(pending)
    }

    fn finish_ready(&self) -> Result<(), CompilerNativeFactError> {
        self.0.finish_ready()
    }
}

struct CatalogProvider(FakeProvider);

impl OwnerNativeCompileProvider for CatalogProvider {
    type Binding<'owner> = CatalogBinding;

    fn bind<'owner>(
        &mut self,
        owner: OwnerRuntimeView<'owner>,
    ) -> Result<OwnerNativeCompileBinding<Self::Binding<'owner>>, crate::WorkspaceError> {
        let (binding, policy) = self.0.bind(owner)?.into_parts();
        Ok(OwnerNativeCompileBinding::new(
            CatalogBinding(binding),
            policy,
        ))
    }
}

fn must<T, E: std::fmt::Debug>(result: std::result::Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error:?}"),
    }
}

fn present<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("{context}"),
    }
}

fn unit_fixture() -> (Fixture, DependencyProviderId) {
    let root = must(tempfile::tempdir(), "unit workspace");
    must(
        fs::write(
            root.path().join("vibe.toml"),
            "[project]\ngroup='org.demo'\nname='host'\nversion='0.1.0'\n",
        ),
        "root manifest",
    );
    let group = must(Group::parse("org.demo"), "group");
    let version = must("1.0.0".parse(), "version");
    let slot = crate::vibedeps::slot_abs_path(root.path(), &group, "unit", &version);
    must(fs::create_dir_all(slot.join("boot")), "unit boot");
    must(
        fs::write(
            slot.join("vibe.toml"),
            "[package]\ngroup='org.demo'\nname='unit'\nkind='tool'\nversion='1.0.0'\n\
         [boot_snippet]\nsource='boot/input.md'\nlink='static'\n\
         [[extension]]\nid='native'\npoint='compile:emitted'\nhandler={kind='native',crate_dir='native'}\n",
        ),
        "unit manifest",
    );
    must(
        fs::write(slot.join("boot/input.md"), "# Unit {#root}\n\nbody\n"),
        "unit source",
    );
    let workspace = must(Workspace::load(root.path()), "workspace");
    let resolution = vec![crate::install::ResolvedDep {
        kind: PackageKind::Tool,
        group: group.clone(),
        name: "unit".to_owned(),
        version,
        content_dir: slot.clone(),
        source_hash: Some(must(ContentHash::parse("sha256:aa"), "source hash")),
        embedded_sources: Vec::new(),
        manifest: must(Manifest::read(slot.join("vibe.toml")), "unit manifest"),
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    }];
    let world = must(
        ExtensionWorldEpoch::from_resolution(root.path(), &resolution),
        "world",
    );
    let lowered = must(
        lower_owner_runtimes(
            &workspace,
            &world,
            OwnerRuntimeLowering::new(".", BTreeMap::new()),
        ),
        "runtimes",
    );
    let epoch = lowered.bind_run(OwnerRuntimeRunFacts {
        run_id: "4123456789abcdef0123456789abcdef".to_owned(),
        state_root: root.path().join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-08T00:00:00Z".to_owned(),
    });
    let relative = must(slot.strip_prefix(root.path()), "slot relative")
        .join("boot/input.md")
        .to_string_lossy()
        .replace('\\', "/");
    let boot = EffectiveBoot {
        entries: vec![BootEntry {
            path: relative,
            band: BootBand::Dependency,
            link: LinkType::Static,
            when: None,
            origin: "org.demo/unit".to_owned(),
            provenance: BootProvenance::Dependency {
                group: group.clone(),
                name: "unit".to_owned(),
            },
            use_ref: false,
            format: Default::default(),
            unit_substituted: false,
            elided: false,
        }],
    };
    let owner = DependencyProviderId::new(
        group,
        must(vibe_core::PackageName::parse("unit"), "package name"),
    );
    (
        Fixture {
            _root: root,
            workspace,
            epoch,
            boot,
            self_coord: SelfCoordinate::new(Some("org.demo".to_owned()), "host".to_owned()),
        },
        owner,
    )
}

fn compile_unit(
    fixture: &Fixture,
    owner: &DependencyProviderId,
    provider: &mut FakeProvider,
    mode: OwnerNativeCompileMode<'_>,
) -> crate::boot_artifacts::native_managed::OwnerManagedStaticCompile {
    present(
        must(
            compile_static_owner_managed(
                &fixture.boot,
                &fixture.workspace.root,
                &fixture.self_coord,
                SpecFormat::Mixed,
                must(fixture.epoch.unit(owner), "unit owner"),
                mode,
                Some(provider),
            ),
            "unit compile",
        ),
        "unit artifact",
    )
}

#[test]
fn selected_native_managed_builtin_reads_vibe_xml_beside_raw_pair_and_refuses_drift() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.demo'\nname='host'\nversion='1.1.0'\n",
    )
    .unwrap();
    let source = tempfile::tempdir().unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let rule = specs.join("common/rule.xml");
    let ordinary = specs.join("common/guide.md");
    let data = specs.join("common/guide.xml");
    fs::create_dir_all(source.path().join("vibevm/vibespecs/common")).unwrap();
    let fwd = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\ngroup='org.demo'\nname='selected'\nkind='tool'\nversion='1.1.0'\nformat='normal'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[extension]]\nid='selected-test'\npoint='compile:pass'\nhandler={{kind='native',crate_dir='native'}}\ncompiler_internals=true\npass={{kind='backend',artifact='selected-test'}}\n",
        fwd(&ordinary),
        fwd(&data),
        fwd(&rule),
    );
    fs::write(source.path().join("vibe.toml"), &manifest).unwrap();
    fs::write(
        source.path().join(&ordinary),
        b"# Ordinary\n\n#use is literal.\n",
    )
    .unwrap();
    fs::write(
        source.path().join(&data),
        b"<data><!-- keep --> raw </data>\n",
    )
    .unwrap();
    fs::write(source.path().join(&rule),
        b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"impl/done\">native-selected XML</RULE></p></spec>\n")
        .unwrap();
    let group = Group::parse("org.demo").unwrap();
    let version: semver::Version = "1.1.0".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    crate::vibedeps::materialise_with_spec_format(
        root.path(),
        &group,
        "selected",
        &version,
        source.path(),
        crate::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let slot = crate::vibedeps::slot_abs_path(root.path(), &group, "selected", &version);
    let resolution = vec![crate::install::ResolvedDep {
        kind: PackageKind::Tool,
        group: group.clone(),
        name: "selected".into(),
        version: version.clone(),
        content_dir: slot.clone(),
        source_hash: Some(hash.clone()),
        embedded_sources: Vec::new(),
        manifest: Manifest::parse_str(&manifest).unwrap(),
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    }];
    let workspace = Workspace::load(root.path()).unwrap();
    let world = ExtensionWorldEpoch::from_resolution(root.path(), &resolution).unwrap();
    let lowered = lower_owner_runtimes(
        &workspace,
        &world,
        OwnerRuntimeLowering::new(".", BTreeMap::new()),
    )
    .unwrap();
    let epoch = lowered.bind_run(OwnerRuntimeRunFacts {
        run_id: "7123456789abcdef0123456789abcdef".into(),
        state_root: root.path().join(".vibe"),
        platform: "linux-x86_64".into(),
        offline: true,
        created_at: "2026-09-08T00:00:00Z".into(),
    });
    let owner = DependencyProviderId::new(
        group.clone(),
        vibe_core::PackageName::parse("selected").unwrap(),
    );
    let boot = EffectiveBoot {
        entries: vec![BootEntry {
            path: slot
                .join(&rule)
                .strip_prefix(root.path())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            band: BootBand::Dependency,
            link: LinkType::Static,
            when: None,
            origin: "org.demo/selected".into(),
            provenance: BootProvenance::Dependency {
                group: group.clone(),
                name: "selected".into(),
            },
            use_ref: false,
            format: vibe_core::manifest::PackageFormat::Normal,
            unit_substituted: false,
            elided: false,
        }],
    };
    let witness = crate::boot_artifacts::SelectedSlotWitness {
        group,
        name: "selected".into(),
        version,
        root: slot.clone(),
        source_hash: Some(hash),
        spec_format: SpecFormat::Mixed,
    };
    let mut static_provider = CatalogProvider(FakeProvider::new(Reply::Skip));
    let selected = crate::boot_artifacts::native_managed::compile_static_owner_managed_selected(
        &boot,
        root.path(),
        &SelfCoordinate::new(Some("org.demo".into()), "host".into()),
        SpecFormat::Mixed,
        epoch.unit(&owner).unwrap(),
        OwnerNativeCompileMode::Plain,
        Some(&mut static_provider),
        &[witness.clone()],
    )
    .unwrap()
    .unwrap();
    assert!(String::from_utf8_lossy(selected.artifact().bytes()).contains("native-selected XML"));
    assert_eq!(
        fs::read(slot.join(&ordinary)).unwrap(),
        b"# Ordinary\n\n#use is literal.\n"
    );
    assert_eq!(
        fs::read(slot.join(&data)).unwrap(),
        b"<data><!-- keep --> raw </data>\n"
    );
    let mut selected_provider = CatalogProvider(FakeProvider::new(Reply::Skip));
    let explicit_ready =
        crate::boot_artifacts::native_managed::compile_backend_owner_managed_selected(
            &boot,
            root.path(),
            &SelfCoordinate::new(Some("org.demo".into()), "host".into()),
            vibe_spec::BackendId::new("selected-test").unwrap(),
            epoch.unit(&owner).unwrap(),
            &mut selected_provider,
            &[witness.clone()],
        )
        .unwrap()
        .unwrap();
    assert_eq!(explicit_ready.bytes(), b"selected backend ready");
    let mut foreign = witness.clone();
    foreign.source_hash = Some(ContentHash::parse(&format!("sha256:{}", "2".repeat(64))).unwrap());
    let error = crate::boot_artifacts::native_managed::compile_static_owner_managed_selected(
        &boot,
        root.path(),
        &SelfCoordinate::new(Some("org.demo".into()), "host".into()),
        SpecFormat::Mixed,
        epoch.unit(&owner).unwrap(),
        OwnerNativeCompileMode::Plain,
        None::<&mut FakeProvider>,
        &[foreign.clone()],
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("source hash or format differs"), "{error}");
    let mut explicit_provider = FakeProvider::new(Reply::Missing);
    let explicit = crate::boot_artifacts::native_managed::compile_backend_owner_managed_selected(
        &boot,
        root.path(),
        &SelfCoordinate::new(Some("org.demo".into()), "host".into()),
        vibe_spec::BackendId::new("selected-test").unwrap(),
        epoch.unit(&owner).unwrap(),
        &mut explicit_provider,
        &[foreign],
    )
    .unwrap_err()
    .to_string();
    assert!(
        explicit.contains("source hash or format differs"),
        "{explicit}"
    );
    assert!(explicit_provider.owners.is_empty());
    fs::write(slot.join(rule), b"changed XML\n").unwrap();
    let error = crate::boot_artifacts::native_managed::compile_static_owner_managed_selected(
        &boot,
        root.path(),
        &SelfCoordinate::new(Some("org.demo".into()), "host".into()),
        SpecFormat::Mixed,
        epoch.unit(&owner).unwrap(),
        OwnerNativeCompileMode::Plain,
        None::<&mut FakeProvider>,
        &[witness],
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("changed bytes"), "{error}");
}

#[test]
fn package_unit_plain_traced_observed_paths_agree_on_bytes_status_and_evidence() {
    for (reply, run_id) in [
        (Reply::Skip, "5123456789abcdef0123456789abcdef"),
        (Reply::Missing, "6123456789abcdef0123456789abcdef"),
    ] {
        let (plain_fixture, plain_owner) = unit_fixture();
        let (observed_fixture, observed_owner) = unit_fixture();
        let (traced_fixture, traced_owner) = unit_fixture();
        let mut plain_provider = FakeProvider::new(reply);
        let mut observed_provider = FakeProvider::new(reply);
        let mut traced_provider = FakeProvider::new(reply);
        let plain = compile_unit(
            &plain_fixture,
            &plain_owner,
            &mut plain_provider,
            OwnerNativeCompileMode::Plain,
        );
        let observer = Arc::new(Observer::default());
        let observed = compile_unit(
            &observed_fixture,
            &observed_owner,
            &mut observed_provider,
            OwnerNativeCompileMode::Observed(observer.clone()),
        );
        let run = TraceRun::open_with_limits(
            &traced_fixture.workspace.root,
            run_id,
            Timestamp::from_timestamp(1_100, 0).expect("timestamp"),
            TraceLimits::for_test(u64::MAX, 9),
        )
        .expect("trace run");
        let trace_unit = (Group::parse("org.demo").expect("group"), "unit".to_owned());
        let acquisition = ScopeAcquisition::unit(&run, &trace_unit, "1.0.0", SpecFormat::Mixed);
        let traced = compile_unit(
            &traced_fixture,
            &traced_owner,
            &mut traced_provider,
            OwnerNativeCompileMode::Traced(&acquisition),
        );
        assert_eq!(plain.artifact().bytes(), observed.artifact().bytes());
        assert_eq!(plain.artifact().bytes(), traced.artifact().bytes());
        assert_eq!(
            plain.native().map(|outcome| outcome.status()),
            observed.native().map(|outcome| outcome.status())
        );
        assert_eq!(
            plain.native().map(|outcome| outcome.status()),
            traced.native().map(|outcome| outcome.status())
        );
        if matches!(reply, Reply::Missing) {
            let evidence = plain
                .native()
                .and_then(|outcome| outcome.pending())
                .unwrap()
                .0;
            assert_eq!(
                observed
                    .native()
                    .and_then(|outcome| outcome.pending())
                    .unwrap()
                    .0,
                evidence
            );
            assert_eq!(
                traced
                    .native()
                    .and_then(|outcome| outcome.pending())
                    .unwrap()
                    .0,
                evidence
            );
        }
        assert_eq!(observer.emissions.lock().expect("observer").len(), 1);
    }
}
