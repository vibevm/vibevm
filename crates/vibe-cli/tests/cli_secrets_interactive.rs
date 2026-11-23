#![cfg(all(test, target_os = "linux"))]
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
};

fn compiled_runner(scope: &Path) -> std::path::PathBuf {
    let c = scope.join("pty.c");
    let runner = scope.join("pty");
    let source=include_str!("common/pty-runner.c").replace("#include <errno.h>","#include <errno.h>\n#include <poll.h>\n#include <signal.h>\n#include <time.h>").replace("    char buffer[4096];","    time_t started = time(NULL);\n    char buffer[4096];").replace("        ssize_t length = read(master, buffer, sizeof(buffer));","        if (time(NULL) - started > 20) { kill(-child, SIGKILL); waitpid(child, NULL, 0); return 131; }\n        struct pollfd ready = { .fd = master, .events = POLLIN };\n        if (poll(&ready, 1, 1000) == 0) continue;\n        ssize_t length = read(master, buffer, sizeof(buffer));");
    let source=source.replace("    if (input != NULL && write_all(master, input, strlen(input)) < 0) return 127;", "    const char *wait_for = getenv(\"VIBE_TEST_PTY_WAIT\");\n    int deferred = wait_for != NULL && strlen(wait_for) > 0;\n    int sent = !deferred;\n    char seen[8192] = {0}; size_t used = 0;\n    if (!deferred && input != NULL && write_all(master, input, strlen(input)) < 0) return 127;")
        .replace("            if (write_all(STDOUT_FILENO, buffer, (size_t)length) < 0) return 127;", "            if (write_all(STDOUT_FILENO, buffer, (size_t)length) < 0) return 127;\n            if (!sent && used + (size_t)length < sizeof(seen)) {\n                memcpy(seen + used, buffer, (size_t)length); used += (size_t)length; seen[used] = 0;\n                if (strstr(seen, wait_for) != NULL) {\n                    struct termios state;\n                    for (int i = 0; i < 100; i++) {\n                        if (tcgetattr(master, &state) == 0 && !(state.c_lflag & ECHO)) break;\n                        usleep(10000);\n                    }\n                    if (input != NULL && write_all(master, input, strlen(input)) < 0) return 127;\n                    sent = 1;\n                }\n            }");
    // A second Escape must arrive after the first question has collapsed;
    // adjacent Escapes can be decoded together as a terminal key sequence.
    let source = source
        .replace("    char buffer[4096];", "    char buffer[4096];\n    const char *after = getenv(\"VIBE_TEST_PTY_AFTER\");\n    const char *next = getenv(\"VIBE_TEST_PTY_NEXT\");\n    char staged[16384] = {0}; size_t staged_used = 0; int staged_sent = 0;")
        .replace("            if (!sent && used", "            if (!staged_sent && after != NULL && next != NULL && staged_used + (size_t)length < sizeof(staged)) {\n                memcpy(staged + staged_used, buffer, (size_t)length); staged_used += (size_t)length; staged[staged_used] = 0;\n                if (strstr(staged, after) != NULL) {\n                    if (write_all(master, next, strlen(next)) < 0) return 127;\n                    staged_sent = 1;\n                    after = getenv(\"VIBE_TEST_PTY_AFTER2\"); next = getenv(\"VIBE_TEST_PTY_NEXT2\");\n                    if (after != NULL && next != NULL) { staged_sent = 0; staged_used = 0; staged[0] = 0; }\n                }\n            }\n            if (!sent && used");
    fs::write(&c, source).unwrap();
    let compile = Command::new("cc")
        .args(["-O0", "-o"])
        .arg(&runner)
        .arg(&c)
        .arg("-lutil")
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    runner
}

#[test]
fn interactive_show_references_lists_consumer_ids() {
    const SENTINEL: &str = "SYNTHETIC_INTERACTIVE_SECRET_DO_NOT_PRINT";
    let scope = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let runner = compiled_runner(scope.path());
    let run = |args: &[&str], input: Option<&str>| -> serde_json::Value {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", scope.path())
            .env("VIBE_SETTINGS", scope.path())
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        } else {
            drop(child.stdin.take());
        }
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stdout.contains(SENTINEL));
        assert!(!stderr.contains(SENTINEL));
        assert!(output.status.success(), "{stdout}\n{stderr}");
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let created = run(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
            "--json",
        ],
        Some(SENTINEL),
    );
    let id = created["items"][0]["id"].as_str().unwrap();
    let added = run(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
            "--json",
        ],
        None,
    );
    let cid = added["items"].as_object().unwrap().keys().next().unwrap();
    let index = scope.path().join("secrets/index.toml");
    let before = fs::read(&index).unwrap();
    let output = Command::new(runner)
        .arg(env!("CARGO_BIN_EXE_vibe"))
        .arg("secrets")
        .env_clear()
        .env("HOME", scope.path())
        .env("VIBE_SETTINGS", scope.path())
        .env("PATH", "/usr/bin:/bin")
        .env("TERM", "xterm")
        .env(
            "VIBE_TEST_PTY_INPUT",
            "\x1b[B\r\x1b[B\x1b[B\x1b[B\x1b[B\r\x1b",
        )
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains(SENTINEL));
    assert!(!stderr.contains(SENTINEL));
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains(&format!("consumers of {id}")), "{stdout}");
    assert!(stdout.contains(&format!("ai:{cid}")), "{stdout}");
    assert_eq!(fs::read(index).unwrap(), before);
}

#[test]
fn secrets_keyring_menu_removal_context() {
    const SENTINEL: &str = "SYNTHETIC_INTERACTIVE_SECRET_DO_NOT_PRINT";
    let scope = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let runner = compiled_runner(scope.path());
    let run = |args: &[&str], input: Option<&str>| -> serde_json::Value {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", scope.path())
            .env("VIBE_SETTINGS", scope.path())
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        } else {
            drop(child.stdin.take());
        }
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stdout.contains(SENTINEL));
        assert!(!stderr.contains(SENTINEL));
        assert!(output.status.success(), "{stdout}\n{stderr}");
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let created = run(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "third",
            "--value-stdin",
            "--json",
        ],
        Some(SENTINEL),
    );
    let id = created["items"][0]["id"].as_str().unwrap();
    let index = scope.path().join("secrets/index.toml");
    let interact = |input: &str, after: &str, next: &str| {
        let before = fs::read(&index).unwrap();
        let output = Command::new(&runner)
            .arg(env!("CARGO_BIN_EXE_vibe"))
            .arg("secrets")
            .env_clear()
            .env("HOME", scope.path())
            .env("VIBE_SETTINGS", scope.path())
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", "xterm")
            .env("NO_COLOR", "1")
            .env("VIBE_TEST_PTY_INPUT", input)
            .env("VIBE_TEST_PTY_AFTER", after)
            .env("VIBE_TEST_PTY_NEXT", next)
            .env("VIBE_TEST_PTY_AFTER2", "? Keyring item ›")
            .env("VIBE_TEST_PTY_NEXT2", "\x1b")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stdout.contains(SENTINEL));
        assert!(!stderr.contains(SENTINEL));
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert_eq!(fs::read(&index).unwrap(), before);
        stdout
    };
    let cancelled = interact(
        "\x1b[B\r\x1b[B\x1b[B\x1b[B\r",
        "❯ Keep item\r\n  Remove item",
        "\x1b",
    );
    for expected in [
        "Keyring item",
        "Create item",
        &format!("(api-key)  {id}"),
        "Action",
        "Remove item",
        "Remove third?",
        &format!("{id} · no registered consumers · private bytes remain"),
    ] {
        assert!(
            cancelled.contains(expected),
            "missing {expected}: {cancelled}"
        );
    }
    assert!(cancelled.contains("✓ Keyring item  third"), "{cancelled}");
    assert!(
        cancelled.contains("○ Remove third?  kept unchanged"),
        "{cancelled}"
    );
    // Inspect the drawn options before the collapsed answer, not echoed
    // keystrokes or cursor-control bytes from a redraw.
    let menu = cancelled.split("✓ Keyring item").next().unwrap();
    let plain_menu = console::strip_ansi_codes(menu);
    let options: Vec<_> = plain_menu
        .lines()
        .map(|row| row.trim_end_matches('\r'))
        .filter(|row| {
            (row.starts_with("  third") || row.starts_with("❯ third"))
                && row.contains("— (api-key)")
        })
        .collect();
    assert!(!options.is_empty(), "no third option row: {menu}");
    for row in options {
        assert_eq!(row.matches(id).count(), 1, "{row}");
    }
    let selected = cancelled
        .lines()
        .find(|line| line.contains("✓ Keyring item"))
        .unwrap();
    assert_eq!(selected.matches(id).count(), 1, "{selected}");
    assert_eq!(run(&["secrets", "details", id, "--json"], None), created);
    let added = run(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
            "--json",
        ],
        None,
    );
    let cid = added["items"].as_object().unwrap().keys().next().unwrap();

    let before_refused = run(&["secrets", "details", id, "--json"], None);
    let refused = interact(
        "\x1b[B\r\x1b[B\x1b[B\x1b[B\r",
        "Item is used; removal refused.",
        "",
    );
    assert!(
        refused.contains("Item is used; removal refused."),
        "{refused}"
    );
    assert!(refused.contains(&format!("ai:{cid}")), "{refused}");
    assert!(refused.contains("✗"), "{refused}");
    assert!(!refused.contains("[\""), "{refused}");
    assert!(!refused.contains("\"protocol\""), "{refused}");
    assert!(!refused.contains("Remove third?"), "{refused}");
    assert_eq!(
        run(&["secrets", "details", id, "--json"], None),
        before_refused
    );
}
