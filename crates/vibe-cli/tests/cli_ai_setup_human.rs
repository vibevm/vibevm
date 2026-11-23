//! Synthetic public-process coverage of metadata setup's human plan and replay.
#![cfg(unix)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#coverage");

use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

/// A sentinel that must never reach either output stream.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
const VALUE: &str = "SYNTHETIC_AI_SETUP_KEY_NEVER_OUTPUT";

/// One isolated settings root and HOME, with no inherited environment.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
struct F(tempfile::TempDir);

impl F {
    /// Create a private synthetic home.
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        Self(
            tempfile::Builder::new()
                .permissions(fs::Permissions::from_mode(0o700))
                .tempdir()
                .unwrap(),
        )
    }

    /// Invoke argv directly, close stdin, and check both streams for the sentinel.
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
    fn run(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", self.0.path())
            .env("VIBE_SETTINGS", self.0.path())
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", "xterm")
            .env("NO_COLOR", "1")
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
        let output = child.wait_with_output().unwrap();
        assert!(!String::from_utf8_lossy(&output.stdout).contains(VALUE));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(VALUE));
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    /// Request the unchanged machine document.
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
    fn json(&self, args: &[&str], input: Option<&[u8]>) -> serde_json::Value {
        let mut args = args.to_vec();
        args.push("--json");
        serde_json::from_slice(&self.run(&args, input).stdout).unwrap()
    }

    /// Read a successful human response, never JSON.
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
    fn human<T: AsRef<str>>(&self, args: &[T]) -> String {
        let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();
        let text = String::from_utf8(self.run(&args, None).stdout).unwrap();
        assert!(!text.contains('{'), "{text}");
        text
    }

    /// Seed a synthetic secret and two distinct compatible connections.
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
    fn connections(&self) -> (String, String) {
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
        let key = secret["items"][0]["id"].as_str().unwrap();
        let mut ids: Vec<String> = Vec::new();
        for label in ["A", "B"] {
            let added = self.json(
                &[
                    "ai",
                    "connections",
                    "add",
                    "--label",
                    label,
                    "--endpoint",
                    "http://127.0.0.1:32123/v1",
                    "--api-kind",
                    "openai-chat-completions",
                    "--default-model",
                    "synthetic",
                    "--secret-ref",
                    key,
                ],
                None,
            );
            let items = added["items"].as_object().unwrap();
            let id = items.keys().find(|id| !ids.contains(*id)).unwrap();
            ids.push(id.clone());
        }
        (ids[0].clone(), ids[1].clone())
    }

    /// Resolve an alias through the public binding view.
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
    fn binding(&self, alias: &str) -> serde_json::Value {
        self.json(&["ai", "bindings", "show", alias], None)["binding"].clone()
    }

    /// Inspect only this fixture's durable selected default.
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#coverage")]
    fn default(&self) -> String {
        let index: toml::Value = toml::from_str(
            &fs::read_to_string(self.0.path().join("secrets/ai/index.toml")).unwrap(),
        )
        .unwrap();
        index["selection"]["connection"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}

/// Extract the actual displayed argv without a shell or eval.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
fn displayed_argv(text: &str, quiet: bool) -> Vec<String> {
    if quiet {
        assert_eq!(text.lines().count(), 1, "{text}");
    }
    let mut chars = text.lines().last().unwrap().trim().chars().peekable();
    let mut argv = Vec::new();
    while chars.peek().is_some() {
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut token = String::new();
        let mut quoted = false;
        while let Some(ch) = chars.next() {
            match ch {
                '\'' => quoted = !quoted,
                '\\' if !quoted => {
                    assert_eq!(
                        chars.next(),
                        Some('\''),
                        "only a literal apostrophe escape is generated"
                    );
                    token.push('\'');
                }
                ' ' if !quoted => break,
                ch => token.push(ch),
            }
        }
        assert!(!quoted, "unterminated generated token");
        argv.push(token);
    }
    assert_eq!(argv.first().map(String::as_str), Some("vibe"));
    assert_eq!(&argv[1..3], &["ai", "setup"]);
    assert!(argv.iter().any(|arg| arg == "--apply-preview"));
    argv.remove(0);
    argv
}

/// A measured plan ends with a directly replayable command and then a bound trailer.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
fn ai_setup_preview_prints_plan_and_apply_command_not_json() {
    let f = F::new();
    let (cid, _) = f.connections();
    let before = fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap();
    let text = f.human(&[
        "ai",
        "setup",
        "--agent",
        "pi",
        "--mode",
        "connection",
        "--connection",
        &cid,
    ]);
    assert!(
        text.contains("── ai setup plan · store revision "),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("  pi") && line.contains(&format!("connection {cid}"))),
        "{text}"
    );
    assert!(
        text.contains("✻ previewed 1 binding · nothing written · apply with"),
        "{text}"
    );
    assert_eq!(
        fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap(),
        before
    );
    let argv = displayed_argv(&text, false);
    assert_eq!(
        &argv[..7],
        &[
            "ai",
            "setup",
            "--agent",
            "pi",
            "--mode",
            "connection",
            "--connection"
        ]
    );
    let applied = f.human(&argv);
    assert!(
        applied.contains(&format!(
            "✻ bound 1 agent to connection {cid} · store revision "
        )),
        "{applied}"
    );
    assert_eq!(f.binding("pi")["connection"], cid);
    let preview = f.human(&[
        "ai",
        "setup",
        "--agent",
        "pi",
        "--mode",
        "connection",
        "--connection",
        &cid,
    ]);
    let noop = f.human(&displayed_argv(&preview, false));
    assert!(
        noop.contains("bindings unchanged · store revision "),
        "{noop}"
    );
}

/// Quiet alias and default previews preserve the reviewed subject on direct replay.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
fn quiet_alias_and_default_replay() {
    let f = F::new();
    fs::write(
        f.0.path().join("agents.toml"),
        "[launchers.work-pi]\nkind='pi'\ncommand=['missing-work-pi','--standing-argument']\n",
    )
    .unwrap();
    let (a, b) = f.connections();
    let original_config = fs::read(f.0.path().join("agents.toml")).unwrap();
    let keys = fs::read(f.0.path().join("secrets/index.toml")).unwrap();
    let before = fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap();
    let text = f.human(&[
        "ai",
        "setup",
        "--from-agent-alias",
        "work-pi",
        "--mode",
        "connection",
        "--connection",
        &a,
        "--quiet",
    ]);
    let argv = displayed_argv(&text, true);
    assert_eq!(
        &argv[..7],
        &[
            "ai",
            "setup",
            "--from-agent-alias",
            "work-pi",
            "--mode",
            "connection",
            "--connection"
        ]
    );
    assert!(!argv.iter().any(|arg| arg == "--agent"));
    assert_eq!(
        fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap(),
        before
    );
    let applied = f.human(&argv);
    assert!(
        applied.contains(&format!(
            "bound 1 agent to connection {a} · store revision "
        )),
        "{applied}"
    );
    let explicit = f.binding("work-pi");
    assert_eq!(explicit["harness"], "pi");
    assert_eq!(explicit["mode"], "connection");
    assert_eq!(explicit["connection"], a);

    // First establish A as the default, then change it to B. Both operations
    // replay exactly the argv that quiet mode displayed, retaining pi's A.
    for target in [&a, &b] {
        let before_binding = f.binding("work-pi");
        let request = [
            "ai",
            "setup",
            "--set-default",
            target,
            "--scope",
            "default-only",
        ];
        let before_report = f.json(&request, None);
        let before_revision = before_report["store_revision"].as_u64().unwrap();
        assert_eq!(before_binding["store_revision"], before_revision);
        let default_changed =
            before_report["preview"]["old_default"].as_str() != Some(target.as_str());
        let before = fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap();
        let text = f.human(&[
            "ai",
            "setup",
            "--set-default",
            target,
            "--scope",
            "default-only",
            "--quiet",
        ]);
        let argv = displayed_argv(&text, true);
        assert_eq!(
            &argv[..6],
            &[
                "ai",
                "setup",
                "--set-default",
                target,
                "--scope",
                "default-only"
            ]
        );
        assert_eq!(
            fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap(),
            before
        );
        let applied = f.human(&argv);
        assert!(
            applied.contains(&format!(
                "set the default connection to {target} · store revision "
            )),
            "{applied}"
        );
        assert_eq!(f.default(), *target);
        let after_report = f.json(&request, None);
        let after_revision = after_report["store_revision"].as_u64().unwrap();
        assert_eq!(after_revision, before_revision + u64::from(default_changed));
        let mut after_binding = f.binding("work-pi");
        assert_eq!(after_binding["store_revision"], after_revision);
        // The observation's snapshot revision advances with the store, while
        // every other field of the explicit binding must remain identical.
        let mut expected = explicit.clone();
        expected.as_object_mut().unwrap().remove("store_revision");
        after_binding
            .as_object_mut()
            .unwrap()
            .remove("store_revision");
        assert_eq!(after_binding, expected);
    }
    assert_eq!(f.default(), b);
    assert_eq!(
        fs::read(f.0.path().join("agents.toml")).unwrap(),
        original_config
    );
    assert_eq!(
        fs::read(f.0.path().join("secrets/index.toml")).unwrap(),
        keys
    );
}

/// A valid quoted TOML launcher name must survive displayed argv and direct replay.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
fn spaced_alias_preview_replays_original_binding() {
    for alias in ["work pi", "work'pi"] {
        let f = F::new();
        fs::write(
            f.0.path().join("agents.toml"),
            format!("[launchers.\"{alias}\"]\nkind='pi'\ncommand=['missing-work-pi']\n"),
        )
        .unwrap();
        let (cid, _) = f.connections();
        let before = fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap();
        let text = f.human(&[
            "ai",
            "setup",
            "--from-agent-alias",
            alias,
            "--mode",
            "connection",
            "--connection",
            &cid,
            "--quiet",
        ]);
        assert_eq!(
            fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap(),
            before
        );
        let argv = displayed_argv(&text, true);
        assert_eq!(argv[3], alias, "{text}");
        assert!(!argv.iter().any(|arg| arg == "--agent"));
        assert_eq!(&argv[4..6], &["--mode", "connection"]);
        f.human(&argv);
        let binding = f.binding(alias);
        assert_eq!(binding["harness"], "pi");
        assert_eq!(binding["mode"], "connection");
        assert_eq!(binding["connection"], cid);
    }
}
