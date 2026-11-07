//! Original host capture through managed generation and sealed replay.
use super::*;
#[cfg(test)]
fn host_fixture() -> (Fixture, OwnerRuntimeEpoch) {
    let mut f = fixture(false);
    let path = f.workspace.root.join("vibe.toml");
    let original = fs::read_to_string(&path).unwrap();
    let own = boot_rel("own.payload");
    fs::write(&path,format!("{original}\n[[project.sources]]\nfile='{own}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n[[extension]]\nid='host-native'\npoint='compile:emitted'\nhandler={{kind='native',crate_dir='native'}}\n")).unwrap();
    write(
        &f.workspace.root,
        &own,
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Host original</title></spec>",
    );
    f.workspace = Workspace::load(&f.workspace.root).unwrap();
    native_fixture_for(f, SpecFormat::Mixed)
}
#[test]
fn own_capture_survives_managed_native_and_actual_sealed_replay() {
    let (f, epoch) = host_fixture();
    let pending = regenerate(&f, &epoch, &mut FakeProvider::new(Reply::Missing));
    let index = f
        .workspace
        .root
        .join(vibe_core::layout::current_boot_index());
    let before = fs::read(&index).unwrap();
    assert!(String::from_utf8_lossy(&before).contains(&boot_rel("own.payload")));
    let replay = pending.into_replay_set(&epoch).unwrap();
    let prepared = crate::install::bootgen::replay_prepare::prepare_boot_replay(
        replay,
        &epoch,
        &mut FakeReplayFactory::new(Reply::Skip),
    )
    .unwrap();
    crate::install::bootgen::replay_publish::publish_boot_replay(prepared).unwrap();
    assert_eq!(fs::read(index).unwrap(), before);
    assert!(
        fs::read_to_string(f.workspace.root.join(boot_rel("own.payload")))
            .unwrap()
            .starts_with("<spec")
    );
}
#[test]
fn actual_sealed_replay_refuses_host_drift_before_node_publication() {
    for during in ["prepare", "publish"] {
        let (f, epoch) = host_fixture();
        let pending = regenerate(&f, &epoch, &mut FakeProvider::new(Reply::Missing));
        let replay = pending.into_replay_set(&epoch).unwrap();
        let index = f
            .workspace
            .root
            .join(vibe_core::layout::current_boot_index());
        let before = fs::read(&index).unwrap();
        if during == "prepare" {
            write(&f.workspace.root, boot_rel("own.payload"), "<changed>");
        }
        let result = crate::install::bootgen::replay_prepare::prepare_boot_replay(
            replay,
            &epoch,
            &mut FakeReplayFactory::new(Reply::Skip),
        );
        if during == "prepare" {
            assert!(result.is_err());
        } else {
            let prepared = result.unwrap();
            write(&f.workspace.root, boot_rel("own.payload"), "<changed>");
            let error =
                crate::install::bootgen::replay_publish::publish_boot_replay(prepared).unwrap_err();
            assert_eq!(
                error.disposition(),
                crate::boot_artifacts::transaction::TransactionFailureDisposition::Uncommitted
            );
            assert!(
                matches!(error.failed_owner().owner(),crate::extension_world::OwnerRuntimeId::Node { rel } if rel==".")
            );
            assert!(
                !error.committed_before().is_empty(),
                "earlier unit publication keeps existing committed-prefix law"
            );
        }
        assert_eq!(fs::read(index).unwrap(), before);
    }
}

struct MutatingProvider {
    inner: FakeProvider,
    root: std::path::PathBuf,
}
impl crate::extension_world::OwnerNativeCompileProvider for MutatingProvider {
    type Binding<'a> =
        <FakeProvider as crate::extension_world::OwnerNativeCompileProvider>::Binding<'a>;
    fn bind<'a>(
        &mut self,
        owner: crate::extension_world::OwnerRuntimeView<'a>,
    ) -> Result<
        crate::extension_world::OwnerNativeCompileBinding<Self::Binding<'a>>,
        crate::WorkspaceError,
    > {
        if matches!(
            owner.runtime().id(),
            crate::extension_world::OwnerRuntimeId::Node { .. }
        ) {
            write(&self.root, boot_rel("own.payload"), "<changed>");
        }
        crate::extension_world::OwnerNativeCompileProvider::bind(&mut self.inner, owner)
    }
}
#[test]
fn managed_native_compiler_mutation_refuses_before_host_index_write() {
    let (f, epoch) = host_fixture();
    let mut provider = MutatingProvider {
        inner: FakeProvider::new(Reply::Skip),
        root: f.workspace.root.clone(),
    };
    let result = crate::install::bootgen::native_managed::regenerate_boot_from_bound_native(
        &f.workspace,
        &f.resolution,
        SpecFormat::Mixed,
        None,
        &epoch,
        Some(&mut provider),
    );
    assert!(result.is_err());
    assert!(
        !f.workspace
            .root
            .join(vibe_core::layout::current_boot_index())
            .exists()
    );
}
