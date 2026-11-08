use super::*;

struct Node {
    _root: tempfile::TempDir,
    dir: PathBuf,
    boot: PathBuf,
}

impl Node {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().to_path_buf();
        let boot = dir.join(layout::current_boot_dir());
        fs::create_dir_all(&boot).unwrap();
        fs::write(boot.join(INDEX_FILE), b"OLD-INDEX").unwrap();
        fs::write(boot.join("90-user.xml"), b"USER").unwrap();
        fs::write(dir.join("CLAUDE.md"), b"# Foreign notes\n").unwrap();
        Self {
            _root: root,
            dir,
            boot,
        }
    }

    fn capture(&self) -> BootImages {
        BootImages::capture([self.dir.clone()], []).unwrap()
    }

    /// What a regeneration does: rewrite INDEX, add a STATIC lane, append a
    /// managed block after the co-tenant bytes, create a missing agent file.
    fn regenerate(&self) {
        fs::write(self.boot.join(INDEX_FILE), b"NEW-INDEX").unwrap();
        fs::write(self.boot.join(STATIC_XML_FILE), b"NEW-STATIC").unwrap();
        fs::write(
            self.dir.join("CLAUDE.md"),
            b"# Foreign notes\n\n<vibevm>\nblock\n</vibevm>\n",
        )
        .unwrap();
        fs::write(self.dir.join("GEMINI.md"), b"<vibevm>\nblock\n</vibevm>\n").unwrap();
    }

    fn assert_before(&self) {
        assert_eq!(fs::read(self.boot.join(INDEX_FILE)).unwrap(), b"OLD-INDEX");
        assert!(!self.boot.join(STATIC_XML_FILE).exists());
        assert_eq!(fs::read(self.boot.join("90-user.xml")).unwrap(), b"USER");
        assert_eq!(
            fs::read(self.dir.join("CLAUDE.md")).unwrap(),
            b"# Foreign notes\n"
        );
        assert!(!self.dir.join("GEMINI.md").exists());
        assert!(!self.dir.join("AGENTS.md").exists());
    }
}

#[test]
fn restore_puts_back_every_file_the_regeneration_changed() {
    let node = Node::new();
    let mut images = node.capture();
    node.regenerate();
    images.settle();
    images.restore().unwrap();
    node.assert_before();
}

#[test]
fn a_file_changed_after_the_regeneration_is_preserved_and_reported() {
    let node = Node::new();
    let mut images = node.capture();
    node.regenerate();
    images.settle();
    let foreign = b"# Foreign notes\nEDITED\n\n<vibevm>\nblock\n</vibevm>\n";
    fs::write(node.dir.join("CLAUDE.md"), foreign).unwrap();
    let error = images.restore().unwrap_err();
    assert!(error.contains("CLAUDE.md"), "{error}");
    assert!(error.contains("preserved"), "{error}");
    assert_eq!(fs::read(node.dir.join("CLAUDE.md")).unwrap(), foreign);
    // The refusal is per boot owner: nothing in that owner was half-restored.
    assert_eq!(fs::read(node.boot.join(INDEX_FILE)).unwrap(), b"NEW-INDEX");
}

#[test]
fn a_pending_transaction_journal_keeps_its_directory() {
    let node = Node::new();
    let mut images = node.capture();
    node.regenerate();
    images.settle();
    let journal = node.boot.join(".vibe-boot-artifacts.transaction.toml");
    fs::write(&journal, b"pending").unwrap();
    let error = images.restore().unwrap_err();
    assert!(error.contains("still pending"), "{error}");
    assert_eq!(fs::read(node.boot.join(INDEX_FILE)).unwrap(), b"NEW-INDEX");
    assert!(journal.exists());
}

#[test]
fn an_unchanged_lane_is_not_touched_and_no_lock_is_created() {
    let root = tempfile::tempdir().unwrap();
    let unit = root.path().join("slot").join(layout::current_boot_dir());
    let mut images = BootImages::capture([], [unit.clone()]).unwrap();
    images.settle();
    images.restore().unwrap();
    assert!(!unit.exists());
}

#[test]
fn a_lane_created_by_the_regeneration_is_removed_with_its_lock() {
    let root = tempfile::tempdir().unwrap();
    let unit = root.path().join("slot").join(layout::current_boot_dir());
    let mut images = BootImages::capture([], [unit.clone()]).unwrap();
    fs::create_dir_all(&unit).unwrap();
    fs::write(unit.join(layout::BOOT_ARTIFACTS_LOCK), b"").unwrap();
    fs::write(unit.join(INDEX_FILE), b"UNIT").unwrap();
    images.settle();
    images.restore().unwrap();
    assert!(!unit.exists());
}
