//! Pending replay reads one selected descriptor XML from an exact rebuilt world.

use std::fs;

use tempfile::TempDir;
use vibe_core::ContentHash;
use vibe_core::manifest::{Manifest, SpecFormat};

use super::{BootReplaySet, prepare_boot_replay, tests};
use crate::Workspace;
use crate::boot_artifacts::native_managed_tests::{FakeProvider, FakeReplayFactory, Reply};
use crate::extension_world::{
    ExtensionWorldEpoch, OwnerRuntimeLowering, OwnerRuntimeRunFacts, lower_owner_runtimes,
};
use crate::install::tests_epoch_world::native_freshness::{
    COMPILE_NATIVE, native_graph_keeping_top, regenerate,
};
use crate::install::tests_epoch_world::{group, locked, version, write_lock};
use crate::vibedeps;

fn descriptor_graph() -> (
    crate::install::tests_epoch_world::native_freshness::NativeGraph,
    Vec<u8>,
    Vec<u8>,
) {
    let mut graph = native_graph_keeping_top("", "", COMPILE_NATIVE);
    let root = graph.workspace.root.clone();
    let leaf = vibedeps::slot_abs_path(&root, &group(), "leaf", &version());
    // Only the disposable fixture leaf is replaced. The resulting slot is
    // created by the real descriptor materializer and has a real SlotRecord.
    fs::remove_dir_all(&leaf).unwrap();
    let source = TempDir::new().unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let selected = specs.join("boot/leaf.xml");
    let ordinary = specs.join("boot/guide.md");
    let data = specs.join("boot/guide.xml");
    fs::create_dir_all(source.path().join(&selected).parent().unwrap()).unwrap();
    let fwd = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
    let manifest = format!(
        "[package]\ngroup='org.lock'\nname='leaf'\nkind='tool'\nversion='1.0.0'\nformat='normal'\n\
         [boot_snippet]\nsource='vibevm/vibespecs/boot/leaf.xml'\nlink='static'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
        fwd(&ordinary),
        fwd(&data),
        fwd(&selected),
    );
    fs::write(source.path().join("vibe.toml"), &manifest).unwrap();
    let markdown = b"# Ordinary\n\n#use remains literal.\n".to_vec();
    let xml_data = b"<data><!-- keep --> raw </data>\n".to_vec();
    fs::write(source.path().join(&ordinary), &markdown).unwrap();
    fs::write(source.path().join(&data), &xml_data).unwrap();
    fs::write(source.path().join(&selected),
        b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><LEAF fact=\"true\" status=\"impl/done\">replay-selected XML</LEAF></p></spec>\n")
        .unwrap();
    let selected_hash = ContentHash::parse(&format!("sha256:{}", "b".repeat(64))).unwrap();
    vibedeps::materialise_with_spec_format(
        &root,
        &group(),
        "leaf",
        &version(),
        source.path(),
        vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &selected_hash,
    )
    .unwrap();
    let row = graph
        .resolution
        .iter_mut()
        .find(|dep| dep.name == "leaf")
        .unwrap();
    row.content_dir = leaf;
    row.source_hash = Some(selected_hash);
    row.manifest = Manifest::parse_str(&manifest).unwrap();
    write_lock(
        &root,
        vec![
            locked("top", "sha256:11", &["org.lock/middle@=1.0.0"]),
            locked("middle", "sha256:22", &["org.lock/leaf@=1.0.0"]),
            locked("leaf", &format!("sha256:{}", "b".repeat(64)), &[]),
            locked(
                "dynamic",
                "sha256:44",
                &["org.lock/middle@=1.0.0", "org.lock/support@=1.0.0"],
            ),
            locked("support", "sha256:55", &[]),
        ],
    );
    graph.workspace = Workspace::load(&root).unwrap();
    let world = ExtensionWorldEpoch::from_resolution(&root, &graph.resolution).unwrap();
    graph.epoch = lower_owner_runtimes(
        &graph.workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .unwrap()
    .bind_run(OwnerRuntimeRunFacts {
        run_id: "a123456789abcdef0123456789abcdef".into(),
        state_root: root.join(".vibe"),
        platform: "linux-x86_64".into(),
        offline: true,
        created_at: "2026-09-01T00:00:00Z".into(),
    });
    (graph, markdown, xml_data)
}

#[test]
fn pending_replay_reads_selected_xml_and_preserves_raw_same_stem_resources() {
    let (graph, markdown, xml_data) = descriptor_graph();
    let mut clean = FakeProvider::new(Reply::Skip);
    regenerate(&graph, &mut clean);
    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = tests::collect_replay(&graph, &mut pending);
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let prepared = prepare_boot_replay(replay, &graph.epoch, &mut factory).unwrap();
    let middle = tests::owner(&graph, "middle");
    let top = tests::owner(&graph, "top");
    for owner in [&middle, &top] {
        let lane = prepared
            .publications()
            .iter()
            .find(|lane| lane.owner() == owner)
            .unwrap();
        let bytes = lane.static_lane().unwrap();
        assert!(String::from_utf8_lossy(bytes).contains("replay-selected XML"));
    }
    let leaf = vibedeps::slot_abs_path(&graph.workspace.root, &group(), "leaf", &version());
    let specs = vibe_core::layout::current_specs_root();
    assert_eq!(
        fs::read(leaf.join(&specs).join("boot/guide.md")).unwrap(),
        markdown
    );
    assert_eq!(
        fs::read(leaf.join(specs).join("boot/guide.xml")).unwrap(),
        xml_data
    );
    assert_eq!(factory.finishes, 1);
}

#[test]
fn pending_replay_refuses_foreign_hash_and_output_drift_before_factory() {
    let (graph, _, _) = descriptor_graph();
    regenerate(&graph, &mut FakeProvider::new(Reply::Skip));
    let mut replay = tests::collect_replay(&graph, &mut FakeProvider::new(Reply::Missing));
    let BootReplaySet::Sealed(ref mut sealed) = replay else {
        panic!("expected pending replay")
    };
    let leaf = sealed
        .selected_slots
        .iter_mut()
        .find(|row| row.name == "leaf")
        .unwrap();
    leaf.source_hash = Some(ContentHash::parse(&format!("sha256:{}", "c".repeat(64))).unwrap());
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let foreign = match prepare_boot_replay(replay, &graph.epoch, &mut factory) {
        Ok(_) => panic!("foreign selected hash reached replay"),
        Err(error) => error.to_string(),
    };
    assert!(
        foreign.contains("source hash or format differs"),
        "{foreign}"
    );
    assert_eq!(factory.creates, 0);

    let (graph, _, _) = descriptor_graph();
    regenerate(&graph, &mut FakeProvider::new(Reply::Skip));
    let replay = tests::collect_replay(&graph, &mut FakeProvider::new(Reply::Missing));
    let leaf = vibedeps::slot_abs_path(&graph.workspace.root, &group(), "leaf", &version());
    fs::write(
        leaf.join(vibe_core::layout::current_specs_root())
            .join("boot/leaf.xml"),
        b"changed XML output\n",
    )
    .unwrap();
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let changed = match prepare_boot_replay(replay, &graph.epoch, &mut factory) {
        Ok(_) => panic!("changed selected output reached replay"),
        Err(error) => error.to_string(),
    };
    assert!(changed.contains("changed bytes"), "{changed}");
    assert_eq!(factory.creates, 0);
}
