//! Synthetic, isolated human native-settings previews and direct argv replay.
#![cfg(unix)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#coverage");
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Output, Stdio},
};

/// Never let the fixture credential reach either output stream or assertions.
const VALUE: &str = "SYNTHETIC_NATIVE_HUMAN_KEY_NEVER_OUTPUT";
/// A private HOME and settings root, with no inherited process environment.
struct F(tempfile::TempDir);
impl F {
    /// Allocate a private scratch home.
    fn new() -> Self {
        Self(
            tempfile::Builder::new()
                .permissions(fs::Permissions::from_mode(0o700))
                .tempdir()
                .unwrap(),
        )
    }
    /// Run the built CLI directly, guarding both captured streams before diagnostics.
    fn run(&self, args: &[&str], input: Option<&[u8]>) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", self.0.path())
            .env("VIBE_SETTINGS", self.0.path())
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child.stdin.take().unwrap().write_all(input).unwrap();
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().unwrap();
        assert!(
            !String::from_utf8_lossy(&out.stdout).contains(VALUE),
            "credential on stdout"
        );
        assert!(
            !String::from_utf8_lossy(&out.stderr).contains(VALUE),
            "credential on stderr"
        );
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
    /// Capture the unchanged machine protocol.
    fn json(&self, args: &[&str], input: Option<&[u8]>) -> serde_json::Value {
        let mut args = args.to_vec();
        args.push("--json");
        serde_json::from_slice(&self.run(&args, input).stdout).unwrap()
    }
    /// Capture unattended human output with closed stdin.
    fn human(&self, args: &[&str]) -> String {
        String::from_utf8(self.run(args, None).stdout).unwrap()
    }
    /// Make only synthetic native destinations, all privately owned.
    fn file(&self, path: &str, bytes: &[u8]) {
        let path = self.0.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut parent = path.parent().unwrap();
        while parent != self.0.path() {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).unwrap();
            parent = parent.parent().unwrap();
        }
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    /// Create a local connection without a network request.
    fn connection(&self) -> String {
        let secret = self.json(
            &[
                "secrets",
                "create",
                "--kind",
                "api-key",
                "--label",
                "Synthetic",
                "--value-stdin",
            ],
            Some(VALUE.as_bytes()),
        );
        let added = self.json(
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
                secret["items"][0]["id"].as_str().unwrap(),
            ],
            None,
        );
        added["items"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone()
    }
}

/// Extract the actual printed command, without invoking a shell or eval.
fn command(text: &str) -> Vec<&str> {
    let start = text
        .find("vibe ai setup --native-settings")
        .expect("apply command");
    text[start..].split_whitespace().skip(1).collect()
}

/// Single-target human output retains the incumbent pi replay path.
#[test]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
fn native_settings_preview_prints_rows_and_apply_command_not_json() {
    let f = F::new();
    let id = f.connection();
    let args = [
        "ai",
        "setup",
        "--native-settings",
        "--agent",
        "pi",
        "--connection",
        &id,
    ];
    let json_before = f.run(&[args.as_slice(), &["--json"]].concat(), None).stdout;
    let text = f.human(&args);
    assert!(!text.contains('{'));
    assert!(text.contains("── native settings plan · connection Synthetic"));
    assert!(text.contains("  pi  ~/.pi/agent/models.json  [ready]"));
    assert!(text.contains("✻ previewed native settings for pi · nothing written · apply with"));
    assert!(!f.0.path().join(".pi").exists());
    assert_eq!(
        f.run(&[args.as_slice(), &["--json"]].concat(), None).stdout,
        json_before
    );
    let applied = f.human(&command(&text));
    assert!(applied.contains("✻ wrote connection Synthetic into the native settings of pi"));
    let native: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.path().join(".pi/agent/models.json")).unwrap())
            .unwrap();
    assert!(
        native["providers"]["vibevm"]["apiKey"].as_str() == Some(VALUE),
        "pi credential mismatch"
    );
}

/// Both selectors and set tokens survive quiet output and direct replay,
/// with ready, refused and incompatible targets retaining their real outcomes.
#[test]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn quiet_multi_target_replay() {
    for refused in [false, true] {
        let f = F::new();
        let id = f.connection();
        let unsafe_qwen = b"{\"other\":true,}";
        if refused {
            f.file(".qwen/settings.json", unsafe_qwen);
        }
        let args = [
            "ai",
            "setup",
            "--native-settings",
            "--agent",
            "pi",
            "--agent",
            "qwen",
            "--agent",
            "claude",
            "--connection",
            &id,
        ];
        let json_args = [args.as_slice(), &["--json"]].concat();
        let before = f.run(&json_args, None).stdout;
        let preview: serde_json::Value = serde_json::from_slice(&before).unwrap();
        assert!(preview["preview"].get("label").is_none());
        let mut hash = Sha256::new();
        hash.update(b"vibevm.ai-native-settings-set/v1\0");
        hash.update(serde_json::to_vec(&preview["preview"]).unwrap());
        assert_eq!(
            preview["preview_id"],
            format!("ai-native-settings-set/1:sha256:{:x}", hash.finalize())
        );
        let text = f.human(&args);
        assert!(!text.contains('{'));
        assert!(text.contains("  pi      ~/.pi/agent/models.json  [ready]"));
        let state = if refused { "refused" } else { "ready" };
        assert!(text.contains(&format!("  qwen    ~/.qwen/settings.json    [{state}]")));
        assert!(text.contains("[incompatible]"));
        assert!(text.contains("! separate native files are not written as one transaction"));
        let quiet = f.human(&[args.as_slice(), &["--quiet"]].concat());
        assert_eq!(quiet.lines().count(), 1);
        assert_eq!(quiet.bytes().filter(|b| *b == b'\n').count(), 1);
        assert!(quiet.starts_with(
            "✻ previewed native settings for pi, qwen, claude · nothing written · apply with "
        ));
        let argv = command(&quiet);
        assert_eq!(argv, command(&text));
        assert!(argv.windows(2).any(|a| a == ["--agent", "pi"]));
        assert!(argv.windows(2).any(|a| a == ["--agent", "qwen"]));
        assert_eq!(argv.last().copied(), preview["preview_id"].as_str());
        assert!(!f.0.path().join(".pi").exists());
        assert!(!f.0.path().join(".claude").exists());
        if refused {
            assert!(fs::read(f.0.path().join(".qwen/settings.json")).unwrap() == unsafe_qwen);
        } else {
            assert!(!f.0.path().join(".qwen").exists());
        }
        assert_eq!(f.run(&json_args, None).stdout, before);
        // Replay the printed argv itself, only choosing the machine output mode.
        let result = f.json(&argv, None);
        assert_eq!(result["preview"]["rows"][0]["outcome"], "applied");
        assert_eq!(
            result["preview"]["rows"][1]["outcome"],
            if refused { "skipped" } else { "applied" }
        );
        assert_eq!(result["preview"]["rows"][2]["outcome"], "skipped");
        let pi: serde_json::Value =
            serde_json::from_slice(&fs::read(f.0.path().join(".pi/agent/models.json")).unwrap())
                .unwrap();
        assert!(
            pi["providers"]["vibevm"]["apiKey"].as_str() == Some(VALUE),
            "pi credential mismatch"
        );
        if refused {
            assert!(fs::read(f.0.path().join(".qwen/settings.json")).unwrap() == unsafe_qwen);
        } else {
            let qwen: serde_json::Value =
                serde_json::from_slice(&fs::read(f.0.path().join(".qwen/settings.json")).unwrap())
                    .unwrap();
            assert!(
                qwen["security"]["auth"]["apiKey"].as_str() == Some(VALUE),
                "qwen credential mismatch"
            );
        }
        assert!(!f.0.path().join(".claude").exists());
    }
}
