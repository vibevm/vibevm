//! `PROP-068#acceptance-git`: a clone the shell backend runs is a `git`
//! transfer of precision `reported` — journaled with the size on disk it
//! left, streamed beneath the task that observes it — and a fetch that
//! moves nothing leaves no transfer. Fixtures are `git+file://` bare
//! repositories in a temporary directory; nothing reaches the network.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use vibe_core::progress::transfer::{
    Outcome, Precision, Purpose, TransferJournal, TransferKind, Transfers,
};
use vibe_core::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};
use vibe_registry::git_backend::transfers::observed;
use vibe_registry::{GitBackend, ShellGit};

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// A bare repository with a few hundred KiB of incompressible blobs, so
/// git measures a throughput and prints the received size.
fn fixture(root: &Path) -> PathBuf {
    let source = root.join("source");
    std::fs::create_dir_all(&source).unwrap();
    git(&source, &["init", "--initial-branch=main"]);
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let blob: Vec<u8> = (0..400_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    std::fs::write(source.join("blob.bin"), blob).unwrap();
    git(&source, &["add", "-A"]);
    git(
        &source,
        &[
            "-c",
            "user.email=f@example.test",
            "-c",
            "user.name=F",
            "commit",
            "-m",
            "blob",
        ],
    );
    git(&source, &["tag", "v0.1.0"]);
    let bare = root.join("org.example.blob.git");
    git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    bare
}

#[derive(Default)]
struct Recorder(Mutex<Vec<ProgressEvent>>);

impl ProgressObserver for Recorder {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[test]
fn a_clone_is_a_reported_transfer_and_an_up_to_date_fetch_is_none() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let bare = fixture(root.path());
    let url = format!("git+file://{}", bare.to_string_lossy().replace('\\', "/"));
    let settings = tempfile::tempdir().unwrap();
    let journal = TransferJournal::new(settings.path());
    let backend = ShellGit::new();
    backend.attach_transfers(&Transfers::new(Some(journal.clone()), None, "cache add"));

    let recorder = Arc::new(Recorder::default());
    let progress = Progress::new(recorder.clone());
    let task = progress.task("Fetching org.example/blob@0.1.0");
    let dest = root.path().join("clone");
    observed(
        Some(&task.progress()),
        Some("org.example/blob"),
        Some(Purpose::PackageClone),
        || backend.bootstrap(url.strip_prefix("git+").unwrap(), "v0.1.0", &dest),
    )
    .expect("the clone succeeds");
    task.finish();

    let history = journal.history();
    assert_eq!(history.len(), 1, "{history:?}");
    let entry = &history[0];
    assert_eq!(entry.record.kind, TransferKind::Git);
    assert_eq!(entry.record.precision, Precision::Reported);
    assert_eq!(entry.record.purpose, Purpose::PackageClone);
    assert_eq!(entry.record.package.as_deref(), Some("org.example/blob"));
    assert_eq!(entry.record.command, "cache add");
    assert!(!entry.record.limited);
    assert!(
        entry.record.url.starts_with("file://"),
        "{}",
        entry.record.url
    );
    assert_eq!(entry.outcome, Some(Outcome::Finished));
    assert!(entry.bytes > 100_000, "git reported {} bytes", entry.bytes);
    assert_eq!(entry.record.total, Some(entry.bytes));
    assert!(entry.written.is_some_and(|written| written > 0));
    assert!(journal.active().is_empty());

    let events = recorder.0.lock().unwrap().clone();
    let start = events
        .iter()
        .find_map(|event| match &event.kind {
            ProgressEventKind::Started {
                transfer: Some(record),
                ..
            } => Some((event.task_id, event.parent_id, record.clone())),
            _ => None,
        })
        .expect("the transfer's task started beneath the observing one");
    assert_eq!(start.1, Some(task.id()));
    assert_eq!(start.2.kind, TransferKind::Git);
    assert!(events.iter().any(|event| event.task_id == start.0
        && matches!(&event.kind, ProgressEventKind::Finished { transfer: Some(end) } if end.bytes == entry.bytes)));

    // Already up to date: the fetch moves no bytes and leaves no transfer.
    backend
        .update(&dest, "v0.1.0")
        .expect("the refresh succeeds");
    assert_eq!(journal.history().len(), 1);
}
