use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use vibe_core::manifest::{LinkType, Manifest, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_extension_registry::DependencyProviderId;
use vibe_wire::generated::shared::Timestamp;

use super::*;

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
