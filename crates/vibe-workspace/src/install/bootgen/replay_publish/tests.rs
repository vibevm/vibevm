use std::cell::RefCell;
use std::error::Error as _;
use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tempfile::TempDir;

use super::*;
use crate::boot_artifacts::{INDEX_FILE, STATIC_FILE, STATIC_XML_FILE};
use crate::install::bootgen::replay_prepare::{
    PreparedBootReplay, PreparedOwnerKind, PreparedOwnerPublication,
};
use crate::install::tests_epoch_world::native_freshness::native_graph_full;

#[derive(Clone)]
struct Targets {
    node: PathBuf,
    boot: PathBuf,
    index: PathBuf,
    selected: PathBuf,
    stale: PathBuf,
}

impl Targets {
    fn state(&self) -> (Vec<u8>, Vec<u8>, bool, SystemTime, SystemTime) {
        (
            fs::read(&self.index).expect("INDEX"),
            fs::read(&self.selected).expect("selected STATIC"),
            self.stale.exists(),
            fs::metadata(&self.index)
                .and_then(|meta| meta.modified())
                .expect("INDEX mtime"),
            fs::metadata(&self.selected)
                .and_then(|meta| meta.modified())
                .expect("STATIC mtime"),
        )
    }
}

fn lane(
    root: &Path,
    name: &str,
    owner: OwnerRuntimeId,
    kind: PreparedOwnerKind,
    xml: bool,
    before: (&[u8], &[u8]),
    after: (&[u8], &[u8]),
) -> (PreparedOwnerPublication, Targets) {
    let node = root.join(name);
    let boot = node.join("boot");
    fs::create_dir_all(&boot).expect("boot dir");
    let index = boot.join(INDEX_FILE);
    let selected = boot.join(if xml { STATIC_XML_FILE } else { STATIC_FILE });
    let stale = boot.join(if xml { STATIC_FILE } else { STATIC_XML_FILE });
    fs::write(&index, before.0).expect("old INDEX");
    fs::write(&selected, before.1).expect("old selected");
    fs::write(&stale, b"STALE").expect("old stale");
    let publication = PreparedOwnerPublication::for_test(
        owner,
        kind,
        index.clone(),
        selected.clone(),
        stale.clone(),
        after.0,
        Some(after.1.into()),
    );
    (
        publication,
        Targets {
            node,
            boot,
            index,
            selected,
            stale,
        },
    )
}

fn node_owner(rel: &str) -> OwnerRuntimeId {
    OwnerRuntimeId::Node {
        rel: rel.to_owned(),
    }
}

struct RealFaults {
    fail_parent: Option<PathBuf>,
    primary: Option<transaction::WritePoint>,
    secondary: Option<transaction::WritePoint>,
    entries: RefCell<Vec<PathBuf>>,
}

impl RealFaults {
    fn none() -> Self {
        Self {
            fail_parent: None,
            primary: None,
            secondary: None,
            entries: RefCell::new(Vec::new()),
        }
    }

    fn at(parent: &Path, point: transaction::WritePoint) -> Self {
        Self {
            fail_parent: Some(parent.to_path_buf()),
            primary: Some(point),
            secondary: None,
            entries: RefCell::new(Vec::new()),
        }
    }

    fn with_secondary(mut self, point: transaction::WritePoint) -> Self {
        self.secondary = Some(point);
        self
    }
}

impl transaction::FaultInjector for RealFaults {
    fn check(&self, point: transaction::WritePoint, path: &Path) -> Result<(), WorkspaceError> {
        let parent = if point == transaction::WritePoint::EntryRecovery {
            self.entries.borrow_mut().push(path.to_path_buf());
            path
        } else {
            path.parent().unwrap_or(path)
        };
        if self.fail_parent.as_deref() == Some(parent)
            && (self.primary == Some(point) || self.secondary == Some(point))
        {
            Err(WorkspaceError::Io {
                path: path.to_path_buf(),
                reason: format!("injected {point:?}"),
            })
        } else {
            Ok(())
        }
    }
}

#[test]
fn empty_is_zero_calls_and_success_reports_exact_unit_node_order_without_redirects() {
    assert!(
        publish_boot_replay(PreparedBootReplay::from_test(Vec::new()))
            .expect("production empty replay")
            .committed()
            .is_empty()
    );
    let empty_faults = RealFaults::none();
    let empty =
        publish_boot_replay_with_faults(PreparedBootReplay::from_test(Vec::new()), &empty_faults)
            .expect("empty replay");
    assert!(empty.committed().is_empty());
    assert!(empty_faults.entries.borrow().is_empty());

    let graph = native_graph_full("", "", "", "");
    let provider = graph
        .epoch
        .lowered()
        .units()
        .keys()
        .next()
        .expect("unit owner")
        .clone();
    let root = TempDir::new().expect("tempdir");
    let (unit, unit_targets) = lane(
        root.path(),
        "unit",
        OwnerRuntimeId::Unit { provider },
        PreparedOwnerKind::Unit,
        false,
        (b"OLD-U-I", b"OLD-U-S"),
        (b"NEW-U-I", b"NEW-U-S"),
    );
    let (node, node_targets) = lane(
        root.path(),
        "node",
        node_owner("node"),
        PreparedOwnerKind::Node,
        true,
        (b"SAME-I", b"SAME-S"),
        (b"SAME-I", b"SAME-S"),
    );
    let old_redirect_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let mut redirects = Vec::new();
    for (name, bytes) in [("AGENTS.md", b"AUTHORED-A"), ("GEMINI.md", b"AUTHORED-G")] {
        let path = node_targets.node.join(name);
        fs::write(&path, bytes).expect("redirect fixture");
        File::options()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_times(FileTimes::new().set_modified(old_redirect_time)))
            .expect("age redirect");
        redirects.push((
            path.clone(),
            fs::read(&path).expect("redirect bytes"),
            fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .expect("redirect mtime"),
        ));
    }
    let unchanged = node_targets.state();
    let faults = RealFaults::none();
    let success =
        publish_boot_replay_with_faults(PreparedBootReplay::from_test(vec![unit, node]), &faults)
            .expect("publish replay");
    assert_eq!(
        success
            .committed()
            .iter()
            .map(BootReplayOwner::kind)
            .collect::<Vec<_>>(),
        [PreparedOwnerKind::Unit, PreparedOwnerKind::Node]
    );
    assert_eq!(
        *faults.entries.borrow(),
        vec![unit_targets.boot.clone(), node_targets.boot.clone()]
    );
    assert!(!unit_targets.stale.exists() && !node_targets.stale.exists());
    assert_eq!(node_targets.state().3, unchanged.3);
    assert_eq!(node_targets.state().4, unchanged.4);
    for (path, bytes, modified) in redirects {
        assert_eq!(fs::read(&path).expect("redirect"), bytes);
        assert_eq!(
            fs::metadata(path)
                .and_then(|meta| meta.modified())
                .expect("redirect mtime"),
            modified
        );
    }
    assert!(!node_targets.node.join("CLAUDE.md").exists());
}

fn three(root: &Path) -> (PreparedBootReplay, Vec<Targets>) {
    let mut publications = Vec::new();
    let mut targets = Vec::new();
    for rel in ["a", "b", "c"] {
        let (publication, target) = lane(
            root,
            rel,
            node_owner(rel),
            PreparedOwnerKind::Node,
            false,
            (
                format!("OLD-I-{rel}").as_bytes(),
                format!("OLD-S-{rel}").as_bytes(),
            ),
            (
                format!("NEW-I-{rel}").as_bytes(),
                format!("NEW-S-{rel}").as_bytes(),
            ),
        );
        publications.push(publication);
        targets.push(target);
    }
    (PreparedBootReplay::from_test(publications), targets)
}

#[test]
fn middle_precommit_and_postcommit_faults_report_partial_truth_and_stop() {
    let root = TempDir::new().expect("tempdir");
    let (prepared, targets) = three(root.path());
    let before = targets.iter().map(Targets::state).collect::<Vec<_>>();
    let faults = RealFaults::at(&targets[1].boot, transaction::WritePoint::IndexReplace);
    let failure = publish_boot_replay_with_faults(prepared, &faults).expect_err("middle fault");
    assert_eq!(
        failure.disposition(),
        TransactionFailureDisposition::RestoredBefore
    );
    assert_eq!(failure.committed_before()[0].owner(), &node_owner("a"));
    assert_eq!(failure.failed_owner().owner(), &node_owner("b"));
    assert_eq!(failure.untouched()[0].owner(), &node_owner("c"));
    assert!(failure.source().is_some());
    assert!(failure.source_error().to_string().contains("injected"));
    assert_ne!(targets[0].state().0, before[0].0);
    let restored = targets[1].state();
    assert_eq!(
        (&restored.0, &restored.1, restored.2),
        (&before[1].0, &before[1].1, before[1].2)
    );
    assert_eq!(targets[2].state(), before[2]);
    assert_eq!(faults.entries.borrow().len(), 2);

    let root = TempDir::new().expect("tempdir");
    let (prepared, targets) = three(root.path());
    let before = targets.iter().map(Targets::state).collect::<Vec<_>>();
    let faults = RealFaults::at(
        &targets[1].boot,
        transaction::WritePoint::PostIndexPreStaleCleanup,
    );
    let failure = publish_boot_replay_with_faults(prepared, &faults).expect_err("postcommit");
    assert_eq!(
        failure.disposition(),
        TransactionFailureDisposition::CommitRecoveryIntent
    );
    assert_eq!(failure.committed_before()[0].owner(), &node_owner("a"));
    assert_eq!(failure.failed_owner().owner(), &node_owner("b"));
    assert_eq!(failure.untouched()[0].owner(), &node_owner("c"));
    assert_ne!(targets[1].state().0, before[1].0);
    assert_ne!(targets[1].state().1, before[1].1);
    assert!(targets[1].state().2, "stale carrier remains until recovery");
    assert!(
        targets[1]
            .boot
            .join(".vibe-boot-artifacts.transaction.toml")
            .exists()
    );
    assert_eq!(targets[2].state(), before[2]);
}

#[test]
fn rollback_entry_recovery_and_invalid_paths_use_the_real_engine_once() {
    let root = TempDir::new().expect("tempdir");
    let (prepared, targets) = three(root.path());
    let faults = RealFaults::at(&targets[0].boot, transaction::WritePoint::IndexReplace)
        .with_secondary(transaction::WritePoint::RollbackStart);
    let failure = publish_boot_replay_with_faults(prepared, &faults).expect_err("rollback intent");
    assert_eq!(
        failure.disposition(),
        TransactionFailureDisposition::RollbackRecoveryIntent
    );
    assert!(failure.committed_before().is_empty());
    assert_eq!(failure.failed_owner().owner(), &node_owner("a"));
    assert_eq!(
        failure
            .untouched()
            .iter()
            .map(BootReplayOwner::owner)
            .collect::<Vec<_>>(),
        [&node_owner("b"), &node_owner("c")]
    );

    let root = TempDir::new().expect("tempdir");
    let (publication, targets) = lane(
        root.path(),
        "recover",
        node_owner("recover"),
        PreparedOwnerKind::Node,
        false,
        (b"OLD-I", b"OLD-S"),
        (b"FINAL-I", b"FINAL-S"),
    );
    let seed = RealFaults::at(
        &targets.boot,
        transaction::WritePoint::PostIndexPreStaleCleanup,
    );
    let seeded = transaction::write_with_faults_detailed(
        ArtifactWrite {
            index_path: &targets.index,
            index_bytes: b"SEEDED-I",
            static_path: &targets.selected,
            static_bytes: Some(b"SEEDED-S"),
            stale_path: &targets.stale,
        },
        &seed,
    );
    assert!(seeded.is_err());
    let faults = RealFaults::none();
    publish_boot_replay_with_faults(PreparedBootReplay::from_test(vec![publication]), &faults)
        .expect("entry recovery and publish");
    assert_eq!(*faults.entries.borrow(), vec![targets.boot.clone()]);
    assert_eq!(fs::read(targets.index).expect("final index"), b"FINAL-I");

    let root = TempDir::new().expect("tempdir");
    let (mut publication, _) = lane(
        root.path(),
        "invalid",
        node_owner("invalid"),
        PreparedOwnerKind::Node,
        false,
        (b"OLD-I", b"OLD-S"),
        (b"NEW-I", b"NEW-S"),
    );
    let mut parts = publication.into_parts();
    parts.stale_path = root.path().join("other").join(STATIC_XML_FILE);
    publication = PreparedOwnerPublication::for_test(
        parts.owner,
        parts.kind,
        parts.index_path,
        parts.static_path,
        parts.stale_path,
        parts.index,
        parts.static_lane,
    );
    let faults = RealFaults::none();
    let failure =
        publish_boot_replay_with_faults(PreparedBootReplay::from_test(vec![publication]), &faults)
            .expect_err("invalid target roles");
    assert_eq!(
        failure.disposition(),
        TransactionFailureDisposition::Uncommitted
    );
    assert!(faults.entries.borrow().is_empty());
}

#[test]
fn publisher_source_names_no_semantic_or_recovery_plane() {
    let source = include_str!("../replay_publish.rs");
    for forbidden in [
        "recover_pending",
        "render_",
        "redirect",
        "Provider",
        "overlay",
        "CompilerNative",
        "OwnerRuntimeEpoch",
        "FsSectionSource",
        "TraceRun",
        "transforms-pending",
        "journal",
        "rollback",
        "process",
        "cargo",
        "sort",
    ] {
        assert!(
            !source.contains(forbidden),
            "publisher contains {forbidden}"
        );
    }
    assert!(!source.contains("index: Box") && !source.contains("static_lane: Option"));
}
