use std::env;
use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::*;

const ROOT_ENV: &str = "VIBEVM_R32_LOCK_ROOT";
const MODE_ENV: &str = "VIBEVM_R32_LOCK_MODE";

#[test]
fn exclusive_lock_serializes_a_second_process_and_process_death_releases_it() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("INDEX.md"), b"OLD-INDEX").unwrap();
    fs::write(temp.path().join("STATIC.md"), b"OLD-STATIC").unwrap();
    let mut holder = spawn(temp.path(), "holder");
    wait_for_marker(&temp.path().join("LOCK-READY"));
    let mut writer = spawn(temp.path(), "writer");
    thread::sleep(Duration::from_millis(250));
    assert!(
        writer.try_wait().unwrap().is_none(),
        "writer bypassed exclusive lock"
    );
    assert_eq!(
        fs::read(temp.path().join("INDEX.md")).unwrap(),
        b"OLD-INDEX"
    );

    holder.kill().unwrap();
    let _ = holder.wait().unwrap();
    let status = writer.wait().unwrap();
    assert!(status.success(), "writer status {status}");
    assert_eq!(
        fs::read(temp.path().join("INDEX.md")).unwrap(),
        b"NEW-INDEX"
    );
    assert_eq!(
        fs::read(temp.path().join("STATIC.xml")).unwrap(),
        b"NEW-STATIC"
    );
    assert!(fs::symlink_metadata(temp.path().join("STATIC.md")).is_err());
}

fn spawn(root: &Path, mode: &str) -> Child {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("boot_artifacts::transaction::lock_tests::lock_process_helper")
        .arg("--nocapture")
        .env(ROOT_ENV, root)
        .env(MODE_ENV, mode)
        .spawn()
        .unwrap()
}

fn wait_for_marker(path: &Path) {
    let start = Instant::now();
    while fs::symlink_metadata(path).is_err() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "lock holder did not start"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn lock_process_helper() {
    let Ok(root) = env::var(ROOT_ENV) else {
        return;
    };
    match env::var(MODE_ENV).unwrap().as_str() {
        "holder" => {
            let _lock = lock::BootArtifactLock::acquire(Path::new(&root)).unwrap();
            fs::write(Path::new(&root).join("LOCK-READY"), b"ready").unwrap();
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        "writer" => {
            let root = Path::new(&root);
            write_with_faults(
                ArtifactWrite {
                    index_path: &root.join("INDEX.md"),
                    index_bytes: b"NEW-INDEX",
                    static_path: &root.join("STATIC.xml"),
                    static_bytes: Some(b"NEW-STATIC"),
                    stale_path: &root.join("STATIC.md"),
                },
                &NoFault,
            )
            .unwrap();
        }
        mode => panic!("unknown lock helper mode {mode}"),
    }
}
