//! Signal supervision of the PATH proxy on the index-resolved launch path
//! (PROP-059 `#proxy`): a SIGINT to the proxy alone does not abandon the live
//! child, a SIGTERM is forwarded to it, and the child's exit status or
//! terminating signal (`128 + n`) is the proxy's exit status.

#![cfg(target_os = "linux")]

mod common;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use common::UserScratch;
use sha2::{Digest, Sha256};

/// Writes its pid to `$1`, exits 43 on SIGTERM, otherwise waits for `$2`
/// and exits 42.
const WAITER: &str = "#!/bin/sh\ntrap 'exit 43' TERM\necho $$ > \"$1\"\nwhile [ ! -e \"$2\" ]; do sleep 0.05; done\nexit 42\n";

fn install_waiter(root: &Path, user: &UserScratch) {
    let dir = root.join("registry/org.example/waiter/v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    let mut zip = zip::ZipWriter::new(fs::File::create(dir.join("waiter.zip")).unwrap());
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    zip.start_file("bin/waiter", options).unwrap();
    zip.write_all(WAITER.as_bytes()).unwrap();
    zip.finish().unwrap();
    let sha = format!(
        "{:x}",
        Sha256::digest(fs::read(dir.join("waiter.zip")).unwrap())
    );
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"waiter\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"waiter\"\n\n\
             [[installer_options]]\nid = \"zip\"\nname = \"Zip\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\n\
             [installer_options.installation]\ntype = \"zip\"\nsource = \"waiter.zip\"\nsha256 = \"{sha}\"\n\n\
             [[installer_options.launchers]]\nid = \"main\"\nname = \"Waiter\"\nexecutable = \"$app_bin/waiter\"\ndefault = true\n\n\
             [[installer_options.public_commands]]\nname = \"waiter\"\nlauncher = \"main\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    let output = user
        .vibe()
        .args(["install", "-g", "org.example/waiter", "--registry"])
        .arg(root.join("registry"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn signal(pid: &str, name: &str) {
    assert!(
        Command::new("kill")
            .args([name, pid])
            .status()
            .unwrap()
            .success()
    );
}

fn wait_for(marker: &Path, proxy: &mut Child) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(pid) = fs::read_to_string(marker)
            && pid.ends_with('\n')
        {
            return pid.trim().to_owned();
        }
        if let Some(status) = proxy.try_wait().unwrap() {
            panic!("proxy exited before its child started: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "child never wrote {}",
            marker.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_exit(proxy: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = proxy.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = proxy.kill();
            panic!("proxy did not exit after its child finished");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn the_proxy_supervises_its_child_through_signals() {
    let root = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    install_waiter(root.path(), &user);
    let proxy_path: PathBuf = user.settings.join("opt/bin/waiter");
    assert!(proxy_path.is_file());

    for (mode, expected) in [("interrupt", 42), ("terminate", 43), ("killed", 137)] {
        let started = root.path().join(format!("{mode}.pid"));
        let release = root.path().join(format!("{mode}.release"));
        let mut proxy = Command::new(&proxy_path)
            .args([OsString::from(&started), OsString::from(&release)])
            .env_remove("VIBE_APP_USE")
            .spawn()
            .unwrap();
        let child = wait_for(&started, &mut proxy);
        let proxy_pid = proxy.id().to_string();
        match mode {
            "interrupt" => {
                signal(&proxy_pid, "-INT");
                thread::sleep(Duration::from_millis(150));
                assert_eq!(proxy.try_wait().unwrap(), None, "proxy left a live child");
                fs::write(&release, "go").unwrap();
            }
            "terminate" => signal(&proxy_pid, "-TERM"),
            _ => signal(&child, "-KILL"),
        }
        let status = wait_exit(&mut proxy);
        assert_eq!(status.code(), Some(expected), "{mode}: {status}");
    }
}
