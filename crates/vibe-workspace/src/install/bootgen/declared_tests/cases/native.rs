//! Declared source joins through actual managed-native, backend and replay owners.
use super::*;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot_artifacts::native_managed_tests::{FakeProvider, FakeReplayFactory, Reply};
    use crate::extension_world::{
        ExtensionWorldEpoch, OwnerRuntimeEpoch, OwnerRuntimeLowering, OwnerRuntimeRunFacts,
        lower_owner_runtimes,
    };

    fn run_facts(root: &std::path::Path) -> OwnerRuntimeRunFacts {
        OwnerRuntimeRunFacts {
            run_id: "0123456789abcdef0123456789abcdef".into(),
            state_root: root.join(".vibe"),
            platform: "linux-x86_64".into(),
            offline: true,
            created_at: "2026-11-06T00:00:00Z".into(),
        }
    }
    fn native_fixture() -> (Fixture, OwnerRuntimeEpoch) {
        native_fixture_for(fixture(false), SpecFormat::Mixed)
    }
    fn native_fixture_for(mut f: Fixture, format: SpecFormat) -> (Fixture, OwnerRuntimeEpoch) {
        let parent = &mut f.resolution[0];
        let path = parent.content_dir.join("vibe.toml");
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path,format!("{text}\n[[extension]]\nid='native'\npoint='compile:emitted'\nhandler={{kind='native',crate_dir='native'}}\n")).unwrap();
        parent.manifest = vibe_core::manifest::Manifest::read(path).unwrap();
        for dep in &f.resolution {
            vibedeps::materialise_with_spec_format(
                &f.workspace.root,
                &dep.group,
                &dep.name,
                &dep.version,
                &dep.content_dir,
                vibedeps::CopyMode::Copy,
                format,
                dep.source_hash.as_ref().unwrap(),
            )
            .unwrap();
        }
        let world = ExtensionWorldEpoch::from_resolution(&f.workspace.root, &f.resolution).unwrap();
        let epoch = lower_owner_runtimes(
            &f.workspace,
            &world,
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        )
        .unwrap()
        .bind_run(run_facts(&f.workspace.root));
        (f, epoch)
    }
    fn regenerate(
        f: &Fixture,
        epoch: &OwnerRuntimeEpoch,
        provider: &mut FakeProvider,
    ) -> crate::install::bootgen::native_managed::BoundBootRegeneration {
        crate::install::bootgen::native_managed::regenerate_boot_from_bound_native(
            &f.workspace,
            &f.resolution,
            SpecFormat::Mixed,
            None,
            epoch,
            Some(provider),
        )
        .unwrap()
    }
    fn files(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        fn walk(
            root: &std::path::Path,
            at: &std::path::Path,
            out: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        ) {
            for row in fs::read_dir(at).unwrap() {
                let path = row.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out)
                } else {
                    out.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut out = std::collections::BTreeMap::new();
        walk(root, root, &mut out);
        out
    }
    #[test]
    fn declared_native_unit_and_node_use_bound_media_once_per_owner() {
        let (f, epoch) = native_fixture();
        let mut provider = FakeProvider::new(Reply::Skip);
        let output = regenerate(&f, &epoch, &mut provider);
        assert!(!output.native.is_empty());
        assert!(*provider.invocations.lock().unwrap() > 0);
        let unique = provider
            .owners
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            unique.len(),
            provider.owners.len(),
            "one native binding per actual artifact owner"
        );
        let text = fs::read_to_string(
            f.workspace
                .root
                .join(vibe_core::layout::current_boot_static_md()),
        )
        .unwrap();
        assert_eq!(text.matches("declared_simple_marker").count(), 1, "{text}");
        assert!(
            !text.contains("<spec") && !text.contains("raw_neighbor"),
            "{text}"
        );
    }
    #[test]
    fn declared_sealed_replay_retains_binding_and_prepares_without_publication() {
        let (f, epoch) = native_fixture();
        let pending = regenerate(&f, &epoch, &mut FakeProvider::new(Reply::Missing));
        let replay = pending.into_replay_set(&epoch).unwrap();
        let before = files(&f.workspace.root);
        let mut factory = FakeReplayFactory::new(Reply::Skip);
        let prepared = crate::install::bootgen::replay_prepare::prepare_boot_replay(
            replay,
            &epoch,
            &mut factory,
        )
        .unwrap();
        assert!(!prepared.publications().is_empty());
        for publication in prepared.publications() {
            let text = std::str::from_utf8(
                publication
                    .static_lane()
                    .expect("declared replay carries static text"),
            )
            .unwrap();
            assert_eq!(text.matches("declared_simple_marker").count(), 1, "{text}");
            assert!(
                !text.contains("<spec") && !text.contains("raw_neighbor"),
                "{text}"
            );
            assert!(
                publication
                    .static_path()
                    .ends_with(vibe_core::layout::current_boot_static_md())
            );
            match publication.owner() {
                crate::extension_world::OwnerRuntimeId::Unit { provider } => {
                    assert_eq!(provider.name().as_str(), "parent")
                }
                crate::extension_world::OwnerRuntimeId::Node { rel } => assert_eq!(rel, "."),
            }
        }
        assert_eq!(factory.creates, 1);
        assert_eq!(factory.finishes, 1);
        assert_eq!(files(&f.workspace.root), before);
    }
    #[test]
    fn declared_generated_replay_uses_prepared_unit_instead_of_old_disk_static() {
        let (f, epoch) = native_fixture_for(generated_fixture(), SpecFormat::Xml);
        let pending = crate::install::bootgen::native_managed::regenerate_boot_from_bound_native(
            &f.workspace,
            &f.resolution,
            SpecFormat::Xml,
            None,
            &epoch,
            Some(&mut FakeProvider::new(Reply::Missing)),
        )
        .unwrap();
        let replay = pending.into_replay_set(&epoch).unwrap();
        let parent = &f.resolution[0];
        let path = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        )
        .join(vibe_core::layout::current_boot_static_xml());
        fs::write(&path, b"old disk must not be read").unwrap();
        let before = files(&f.workspace.root);
        let mut factory = FakeReplayFactory::new(Reply::Skip);
        let prepared = crate::install::bootgen::replay_prepare::prepare_boot_replay(
            replay,
            &epoch,
            &mut factory,
        )
        .unwrap();
        let node = prepared.publications().iter().find(|publication| {
            matches!(publication.owner(), crate::extension_world::OwnerRuntimeId::Node { rel } if rel == ".")
        }).expect("affected node is replayed after its generated unit");
        let text = std::str::from_utf8(node.static_lane().unwrap()).unwrap();
        assert_eq!(text.matches("embedded_once_marker").count(), 1, "{text}");
        assert_eq!(text.matches("fragment_once_marker").count(), 1, "{text}");
        assert!(!text.contains("old disk must not be read"));
        assert_eq!(files(&f.workspace.root), before);
    }
    #[test]
    fn declared_sealed_replay_refuses_changed_record_without_publication() {
        let (f, epoch) = native_fixture();
        let pending = regenerate(&f, &epoch, &mut FakeProvider::new(Reply::Missing));
        let replay = pending.into_replay_set(&epoch).unwrap();
        let parent = &f.resolution[0];
        let record = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        )
        .join(vibedeps::SLOT_RECORD_FILENAME);
        let text = fs::read_to_string(&record).unwrap();
        fs::write(&record, format!("{text}\n# changed after seal\n")).unwrap();
        let before = files(&f.workspace.root);
        let mut factory = FakeReplayFactory::new(Reply::Skip);
        let error = match crate::install::bootgen::replay_prepare::prepare_boot_replay(
            replay,
            &epoch,
            &mut factory,
        ) {
            Ok(_) => panic!("changed record accepted"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("record"), "{error}");
        assert_eq!(files(&f.workspace.root), before);
    }

    struct BackendProbe(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    struct BackendBinding(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl vibe_spec::CompilerNativeInvoker for BackendBinding {
        fn admit_backend(
            &self,
            _: &vibe_core::manifest::ExtensionKey,
            _: u32,
            _: &std::collections::BTreeMap<String, Option<serde_json::Value>>,
            _: vibe_spec::CompilerNativeImplementationDigest,
            backend: &str,
        ) -> Result<(), vibe_spec::CompilerNativeInvokerError> {
            assert_eq!(backend, "probe");
            Ok(())
        }
        fn invoke(
            &self,
            call: vibe_spec::CompilerNativeCall<'_>,
        ) -> Result<Vec<u8>, vibe_spec::CompilerNativeInvokerError> {
            assert_eq!(call.backend(), Some("probe"));
            let payload = serde_json::to_string(call.payload()).unwrap();
            assert!(
                payload.contains("declared_simple_marker"),
                "bound source meaning absent from backend input: {payload}"
            );
            assert!(
                !payload.contains("<spec") && !payload.contains("raw_neighbor"),
                "wrong interpretation reached backend: {payload}"
            );
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(serde_json::to_vec(
                &vibe_wire::behaviour::native_backend::ok_reply(b"declared-backend-ok", None)
                    .unwrap(),
            )
            .unwrap())
        }
    }
    impl crate::extension_world::CompilerNativeFactBinding for BackendBinding {
        fn invoker(&self) -> &dyn vibe_spec::CompilerNativeInvoker {
            self
        }
        fn take_pending_build_facts(
            &self,
            pending: &vibe_spec::CompilerPendingSet,
        ) -> Result<
            Vec<crate::extension_world::PendingBuildFact>,
            crate::extension_world::CompilerNativeFactError,
        > {
            if pending.iter().next().is_some() {
                return Err(crate::extension_world::CompilerNativeFactError::missing(0));
            }
            Ok(Vec::new())
        }
        fn finish_ready(&self) -> Result<(), crate::extension_world::CompilerNativeFactError> {
            Ok(())
        }
    }
    impl crate::extension_world::OwnerNativeCompileProvider for BackendProbe {
        type Binding<'a> = BackendBinding;
        fn bind<'a>(
            &mut self,
            _: crate::extension_world::OwnerRuntimeView<'a>,
        ) -> Result<
            crate::extension_world::OwnerNativeCompileBinding<Self::Binding<'a>>,
            crate::WorkspaceError,
        > {
            Ok(crate::extension_world::OwnerNativeCompileBinding::new(
                BackendBinding(self.0.clone()),
                vibe_spec::CompilerNativePolicy::fail(),
            ))
        }
    }
    #[test]
    fn declared_explicit_backend_receives_bound_source_meaning() {
        let mut f = fixture(false);
        apply(&f, SpecFormat::Mixed);
        let path = f.workspace.root.join("vibe.toml");
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path,format!("{text}\n[[extension]]\nid='probe'\npoint='compile:pass'\nhandler={{kind='native',crate_dir='native'}}\ncompiler_internals=true\npass={{kind='backend',artifact='probe'}}\n")).unwrap();
        f.workspace = Workspace::load(&f.workspace.root).unwrap();
        crate::install::tests_analyze_parity::publish_resolution_lock(
            &f.workspace.root,
            &f.resolution,
        );
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut provider = BackendProbe(calls.clone());
        let artifact = install::compile_node_backend_with_source_format(
            &f.workspace,
            ".",
            vibe_spec::BackendId::new("probe").unwrap(),
            run_facts(&f.workspace.root),
            &mut provider,
            SpecFormat::Mixed,
        )
        .unwrap()
        .unwrap();
        assert_eq!(artifact.bytes(), b"declared-backend-ok");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let before = files(&f.workspace.root);
        assert!(
            install::compile_node_backend(
                &f.workspace,
                ".",
                vibe_spec::BackendId::new("probe").unwrap(),
                run_facts(&f.workspace.root),
                &mut provider
            )
            .is_err()
        );
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "unbound refusal invoked backend"
        );
        assert_eq!(files(&f.workspace.root), before);
    }

    #[test]
    fn declared_replay_missing_hash_binding_is_not_rebuilt_from_slot_record() {
        let (f, epoch) = native_fixture();
        let pending = regenerate(&f, &epoch, &mut FakeProvider::new(Reply::Missing));
        let mut replay = pending.into_replay_set(&epoch).unwrap();
        let parent = &f.resolution[0];
        let slot = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        );
        let record = vibedeps::read_slot_record(&slot).unwrap();
        assert_eq!(Some(&record.source_hash), parent.source_hash.as_ref());
        crate::install::bootgen::replay_prepare::drop_source_binding_for_test(&mut replay);
        let before = files(&f.workspace.root);
        let mut factory = FakeReplayFactory::new(Reply::Skip);
        let error = match crate::install::bootgen::replay_prepare::prepare_boot_replay(
            replay,
            &epoch,
            &mut factory,
        ) {
            Ok(_) => panic!("missing source-hash binding reconstructed from installed record"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("binding"), "{error}");
        assert_eq!(files(&f.workspace.root), before);
    }
}
