//! Rejected command lines use the usage refusal, including its terminal palette.
//! Glyphs asserted here: `✗`.

#![cfg(target_os = "linux")]

use std::process::Command;
use vibe_test_support::UserScratch;

fn plain(user: &UserScratch, args: &[&str]) -> std::process::Output {
    user.vibe()
        .env_clear()
        .env("HOME", &user.settings)
        .env("USERPROFILE", &user.settings)
        .env("VIBE_SETTINGS", &user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("TERM", "xterm-256color")
        .env("COLUMNS", "200")
        .current_dir(&user.settings)
        .args(args)
        .output()
        .unwrap()
}

fn coloured(user: &UserScratch, args: &[&str]) -> (i32, String, String) {
    // Separate real PTYs keep both streams terminal-backed without merging them.
    let script = r#"
import errno, json, os, pty, select, subprocess, sys, tty
pairs = [pty.openpty(), pty.openpty()]
for master, slave in pairs:
    tty.setraw(slave)
child = subprocess.Popen(sys.argv[1:], stdin=subprocess.DEVNULL,
                         stdout=pairs[0][1], stderr=pairs[1][1])
for master, slave in pairs:
    os.close(slave)
streams = {master: bytearray() for master, slave in pairs}
active = list(streams)
while active:
    ready, _, _ = select.select(active, [], [], 20)
    if not ready:
        child.kill()
        raise RuntimeError('usage child PTY timed out')
    for master in ready:
        try:
            data = os.read(master, 65536)
        except OSError as error:
            if error.errno != errno.EIO:
                raise
            data = b''
        if data:
            streams[master].extend(data)
        else:
            active.remove(master)
            os.close(master)
print(json.dumps([child.wait(), *(streams[m].decode() for m, _ in pairs)]))
"#;
    let output = Command::new("/usr/bin/python3")
        .args(["-I", "-c", script, env!("CARGO_BIN_EXE_vibe")])
        .args(args)
        .env_clear()
        .env("HOME", &user.settings)
        .env("USERPROFILE", &user.settings)
        .env("VIBE_SETTINGS", &user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("TERM", "xterm-256color")
        .env("COLUMNS", "200")
        .current_dir(&user.settings)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn clap_usage_error_uses_refusal_shape() {
    let user = UserScratch::new();
    let cases: &[(&[&str], &str, &str)] = &[
        (
            &["kit", "init", "/tmp/x/kit.toml"],
            "✗ usage  unexpected argument /tmp/x/kit.toml\n  fix   vibe kit init --file /tmp/x/kit.toml\n  see   vibe kit init --help\n",
            "vibe kit init --file /tmp/x/kit.toml",
        ),
        (
            &["kit", "init", "/tmp/x space/kit.toml"],
            "✗ usage  unexpected argument '/tmp/x space/kit.toml' found\n  fix   vibe kit init --help\n",
            "vibe kit init --help",
        ),
        (
            &["kit", "init", "/tmp/x;echo/kit.toml"],
            "✗ usage  unexpected argument '/tmp/x;echo/kit.toml' found\n  fix   vibe kit init --help\n",
            "vibe kit init --help",
        ),
        (
            &["registry", "remove", "corp4;echo"],
            "✗ usage  unrecognized subcommand 'corp4;echo'\n  fix   vibe registry remove --help\n",
            "vibe registry remove --help",
        ),
        (
            &["registry", "remove", "corp4"],
            "✗ usage  unrecognized subcommand corp4\n  fix   vibe registry remove registry corp4\n  see   vibe registry remove --help\n",
            "vibe registry remove registry corp4",
        ),
        (
            &["kit", "init", "--fil", "x"],
            "✗ usage  unexpected argument '--fil' found\n  why   a similar argument exists: '--file'\n  fix   vibe kit init --help\n",
            "vibe kit init --help",
        ),
        (
            &["app", "extras", "add"],
            "✗ usage  the following required arguments were not provided: <SLOT> <ITEM>...\n  fix   vibe app extras add --help\n",
            "vibe app extras add --help",
        ),
    ];
    // Match the active roles.rs palette: alert red, labels dim, commands blue.
    let alert = console::Style::new().red().force_styling(true);
    let dim = console::Style::new().dim().force_styling(true);
    let accent = console::Style::new().blue().force_styling(true);
    for &(args, expected, fix) in cases {
        let output = plain(&user, args);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        assert_eq!(String::from_utf8_lossy(&output.stderr), expected);
        let (status, stdout, stderr) = coloured(&user, args);
        assert_eq!(status, 2);
        assert!(stdout.is_empty(), "{stdout:?}");
        assert_eq!(console::strip_ansi_codes(&stderr), expected);
        assert!(
            stderr.contains(&alert.apply_to("✗").to_string()),
            "{stderr:?}"
        );
        let headline = expected.lines().next().unwrap().strip_prefix("✗ ").unwrap();
        assert!(
            stderr.contains(&alert.apply_to(headline).to_string()),
            "{stderr:?}"
        );
        assert!(
            stderr.contains(&accent.apply_to(fix).to_string()),
            "{stderr:?}"
        );
        for label in ["why", "fix", "see"] {
            if expected.contains(&format!("  {label}   ")) {
                assert!(
                    stderr.contains(&dim.apply_to(label).to_string()),
                    "{stderr:?}"
                );
            }
        }
    }
    for flag in ["--help", "--version"] {
        let output = plain(&user, &[flag]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert!(!output.stdout.is_empty());
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains(if flag == "--help" { "Usage:" } else { "vibe " }));
    }
}
