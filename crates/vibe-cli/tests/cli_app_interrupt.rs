//! An interrupt reaches everything a step started (PROP-065
//! `#cancellation`): a recipe whose shell waits on a long foreground child
//! stops when vibe is interrupted, the child included, without the recipe
//! trapping anything itself.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::UserScratch;

#[test]
fn an_interrupt_stops_the_recipe_and_what_it_waits_on() {
    let root = tempfile::tempdir().unwrap();
    let registry = root.path().join("registry");
    let package = registry.join("org.example/sleepy/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\nname = \"sleepy\"\ngroup = \"org.example\"\nkind = \"app\"\n\
             version = \"1.0.0\"\n\n[application]\nid = \"sleepy\"\n\n\
             [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\n\
             platform_name = \"Linux\"\narch = \"{}\"\n\
             [[installer_options.steps]]\nid = \"build\"\ntitle = \"Build\"\n\
             kind = \"recipe\"\ninterpreter = \"posix_sh\"\nscript = \"build.sh\"\n",
            std::env::consts::ARCH
        ),
    )
    .unwrap();
    let pid_file = root.path().join("recipe.pid");
    // No trap: the shell waits on a foreground `sleep`, which only an
    // interrupt sent to the whole group stops.
    fs::write(
        package.join("build.sh"),
        format!(
            "echo $$ > '{}.tmp' && mv '{0}.tmp' '{0}'\nsleep 30\nmkdir -p \"$VIBE_OUT\"\n",
            pid_file.display()
        ),
    )
    .unwrap();

    let user = UserScratch::new();
    let started = Instant::now();
    let mut prepared = user.vibe();
    prepared
        .current_dir(root.path())
        .args([
            "install",
            "-g",
            "org.example/sleepy",
            "--unattended",
            "--registry",
        ])
        .arg(&registry);
    // A spawnable std command carrying the scratch environment: the test
    // signals the running vibe.
    let mut command = Command::new(prepared.get_program());
    command.args(prepared.get_args()).current_dir(root.path());
    for (key, value) in prepared.get_envs() {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
    let mut vibe = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let recipe = loop {
        if let Ok(text) = fs::read_to_string(&pid_file) {
            break text.trim().to_owned();
        }
        assert!(Instant::now() < deadline, "the recipe never started");
        std::thread::sleep(Duration::from_millis(50));
    };

    assert!(
        Command::new("kill")
            .args(["-INT", &vibe.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let status = vibe.wait().unwrap();
    assert_eq!(status.code(), Some(1), "an interrupted step exits 1");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "vibe waited for the 30-second sleep instead of stopping it"
    );
    // The recipe led its own process group; nothing of that group is left.
    let group_alive = Command::new("kill")
        .args(["-0", "--", &format!("-{recipe}")])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(!group_alive, "the recipe's group outlived the interrupt");
}
