//! K13/K15 public diagnostics against disposable local settings, never a provider.
#![cfg(unix)]

use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

const SECRET: &str = "SYNTHETIC_FIX13_PRIVATE_VALUE_NEVER_PRINT";
type Tree = BTreeMap<PathBuf, (u32, u32, Option<Vec<u8>>)>;

struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("fix13-diagnostic-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        assert_eq!(fs::metadata(root.path()).unwrap().mode() & 0o777, 0o700);
        Self(root)
    }

    fn run(&self, args: &[&str], input: Option<&[u8]>) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", self.0.path())
            .env("VIBE_SETTINGS", self.0.path())
            .env("PATH", "/usr/bin:/bin")
            .env("NO_COLOR", "1")
            .current_dir(self.0.path())
            .args(["--offline", "--plain"])
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
        // Never use an assertion that formats supplied bytes on failure.
        assert!(
            !out.stdout
                .windows(SECRET.len())
                .any(|w| w == SECRET.as_bytes()),
            "synthetic secret leaked on stdout"
        );
        assert!(
            !out.stderr
                .windows(SECRET.len())
                .any(|w| w == SECRET.as_bytes()),
            "synthetic secret leaked on stderr"
        );
        out
    }

    fn json(&self, args: &[&str], input: Option<&[u8]>) -> Value {
        let mut args = args.to_vec();
        args.push("--json");
        let out = self.run(&args, input);
        assert!(out.status.success(), "synthetic setup command failed");
        let value: Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("success did not emit JSON"));
        assert_eq!(value["ok"], true);
        value
    }

    fn secret(&self) -> Value {
        self.json(
            &[
                "secrets",
                "create",
                "--kind",
                "api-key",
                "--label",
                "Synthetic",
                "--value-stdin",
            ],
            Some(SECRET.as_bytes()),
        )
    }

    fn connection(&self, secret: &str, label: &str) -> Value {
        self.json(
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
                "synthetic-model",
                "--secret-ref",
                secret,
            ],
            None,
        )
    }

    fn tree(&self) -> Tree {
        fn visit(root: &Path, path: &Path, tree: &mut Tree) {
            let metadata = fs::symlink_metadata(path).unwrap();
            assert!(
                !metadata.file_type().is_symlink(),
                "unexpected fixture symlink"
            );
            let bytes = metadata.is_file().then(|| fs::read(path).unwrap());
            tree.insert(
                path.strip_prefix(root).unwrap().to_owned(),
                (metadata.mode(), metadata.uid(), bytes),
            );
            if metadata.is_dir() {
                for entry in fs::read_dir(path).unwrap() {
                    visit(root, &entry.unwrap().path(), tree);
                }
            }
        }
        let mut tree = Tree::new();
        visit(self.0.path(), self.0.path(), &mut tree);
        tree
    }

    fn unchanged(&self, before: &Tree) {
        // Tree bytes include the synthetic value; equality must not debug them.
        assert!(
            self.tree() == *before,
            "refusal changed fixture data or modes"
        );
    }

    fn refusals(&self, args: &[&str], code: &str) -> String {
        let before = self.tree();
        let human = self.run(args, None);
        assert_eq!(human.status.code(), Some(1), "human refusal exit contract");
        assert!(human.stdout.is_empty(), "human refusal polluted stdout");
        self.unchanged(&before);
        let text = String::from_utf8(human.stderr).unwrap();
        let mut machine_args = args.to_vec();
        machine_args.push("--json");
        let machine = self.run(&machine_args, None);
        assert_eq!(machine.status.code(), Some(1), "JSON refusal exit contract");
        assert!(machine.stdout.is_empty(), "JSON refusal polluted stdout");
        let envelope: Value = serde_json::from_slice(&machine.stderr)
            .unwrap_or_else(|_| panic!("machine refusal is not a JSON envelope"));
        assert_eq!(envelope["ok"], false);
        assert!(
            envelope["error"]
                .as_str()
                .is_some_and(|s| s.starts_with(&format!("{code}:"))),
            "machine error lost its existing refusal prefix"
        );
        self.unchanged(&before);
        assert!(text.contains(code), "human refusal lost its code");
        text
    }
}

fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn require(text: &str, checks: &[(&str, bool)]) {
    for (name, passed) in checks {
        eprintln!(
            "diagnostic assertion: {name}: {}",
            if *passed { "PASS" } else { "FAIL" }
        );
    }
    // run() has already checked both byte streams for the sentinel.
    assert!(
        checks.iter().all(|(_, passed)| *passed),
        "safe observed stderr:\n{text}"
    );
}

#[test]
fn fix13_keyring_stale_names_expected_and_current_store_revisions() {
    let f = Fixture::new();
    let created = f.secret();
    let id = created["items"][0]["id"].as_str().unwrap();
    let current = created["items"][0]["current_revision"].as_str().unwrap();
    let expected = created["store_revision"].as_u64().unwrap().to_string();
    let changed = f.json(
        &[
            "secrets",
            "edit",
            id,
            "--label",
            "Accepted rename",
            "--expect-store",
            &expected,
            "--expect-current",
            current,
        ],
        None,
    );
    let actual = changed["store_revision"].as_u64().unwrap().to_string();
    assert_ne!(actual, expected);
    let text = f.refusals(
        &[
            "secrets",
            "edit",
            id,
            "--label",
            "Must not publish",
            "--expect-store",
            &expected,
            "--expect-current",
            current,
        ],
        "keyring_stale",
    );
    assert!(
        f.json(&["secrets", "list"], None) == changed,
        "stale edit changed metadata"
    );
    let flat = words(&text);
    require(
        &text,
        &[
            (
                "actual expected store revision is named",
                flat.contains(&format!("revision {expected}")),
            ),
            (
                "actual current store revision is named",
                flat.contains(&format!("revision {actual}")),
            ),
            (
                "executable refresh remedy",
                flat.contains("vibe secrets list"),
            ),
        ],
    );
}

#[test]
fn fix13_keyring_current_owner_0755_names_path_mode_and_corrective_command() {
    let f = Fixture::new();
    f.secret();
    let path = f.0.path().join("secrets");
    let metadata = fs::symlink_metadata(&path).unwrap();
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    assert_eq!(metadata.uid(), fs::metadata(f.0.path()).unwrap().uid());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o755);
    let text = f.refusals(&["secrets", "list"], "keyring_unprotected");
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o755);
    let flat = words(&text);
    let path = path.to_str().unwrap();
    require(
        &text,
        &[
            ("witnessed directory path", flat.contains(path)),
            ("witnessed mode 0755", flat.contains("0755")),
            (
                "copyable correction for this directory",
                flat.contains(&format!("chmod 700 {path}")),
            ),
        ],
    );
}

#[test]
fn fix13_keyring_malformed_id_is_not_a_permissions_diagnosis() {
    let f = Fixture::new();
    f.secret();
    let text = f.refusals(
        &["secrets", "details", "not-an-id"],
        "keyring_input_invalid",
    );
    require(
        &text,
        &[
            ("malformed id has no chmod advice", !text.contains("chmod")),
            (
                "malformed id has no witnessed permission claim",
                !text.contains("0755") && !text.contains("keyring_unprotected"),
            ),
        ],
    );
}

#[test]
fn fix13_connection_unsupported_claude_names_binding_scope_not_all_native_auth() {
    let f = Fixture::new();
    let native = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "claude",
            "--mode",
            "native",
            "--expect-store",
            "0",
        ],
        None,
    );
    assert_eq!(native["binding"]["mode"], "native");
    let secret = f.secret();
    let added = f.connection(
        secret["items"][0]["id"].as_str().unwrap(),
        "Synthetic connection",
    );
    let version = added["store_revision"].to_string();
    let text = f.refusals(
        &[
            "ai",
            "bindings",
            "set",
            "claude",
            "--mode",
            "inherit-default",
            "--expect-store",
            &version,
        ],
        "binding_unsupported",
    );
    assert_eq!(
        f.json(&["ai", "bindings", "show", "claude"], None)["binding"]["mode"],
        "native"
    );
    let flat = words(&text);
    require(
        &text,
        &[
            ("actual unsupported harness claude", flat.contains("claude")),
            (
                "specific binding capability explanation",
                flat.contains("connection")
                    && (flat.contains("endpoint") || flat.contains("bound")),
            ),
            (
                "harness-specific inspection remedy",
                flat.contains("vibe ai bindings show claude"),
            ),
            (
                "does not deny all native authentication",
                !flat.contains("native authentication is unsupported")
                    && !flat.contains("native authentication is not supported"),
            ),
        ],
    );
}

#[test]
fn fix13_connection_bound_nondefault_qwen_names_and_replays_revision_specific_unbind() {
    let f = Fixture::new();
    let secret = f.secret();
    let sid = secret["items"][0]["id"].as_str().unwrap();
    let a = f.connection(sid, "Default unrelated connection");
    let aid = a["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let b = f.connection(sid, "Explicit Qwen connection");
    let bid = b["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    assert_ne!(aid, bid);
    let brev = b["items"][&bid]["current_revision"].as_str().unwrap();
    let selected = f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            &aid,
            "--expect-store",
            &b["store_revision"].to_string(),
        ],
        None,
    );
    let bound = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "connection",
            "--connection",
            &bid,
            "--expect-store",
            &selected["store_revision"].to_string(),
        ],
        None,
    );
    assert_eq!(bound["binding"]["connection"], bid);
    let version = bound["binding"]["store_revision"].to_string();
    let default_before = f.json(&["ai", "connections", "default", "show"], None);
    assert_eq!(default_before["connection"], aid);
    let pi_before = f.json(&["ai", "bindings", "show", "pi"], None);
    let keyring_before = f.json(&["secrets", "list"], None);
    let before = f.tree();
    let text = f.refusals(
        &[
            "ai",
            "connections",
            "remove",
            &bid,
            "--expect-store",
            &version,
            "--expect-current",
            brev,
            "--yes",
        ],
        "connection_in_use",
    );
    f.unchanged(&before);
    assert!(
        f.json(&["ai", "bindings", "show", "qwen"], None) == bound,
        "removal refusal changed binding"
    );
    let flat = words(&text);
    let remedy = format!("vibe ai bindings set qwen --mode native --expect-store {version}");
    require(
        &text,
        &[
            ("actual bound connection id", flat.contains(&bid)),
            ("actual qwen binding", flat.contains("qwen")),
            ("observed-revision unbind argv", flat.contains(&remedy)),
        ],
    );
    // Only replay a command actually printed, parsed as argv, never shell/eval.
    let start = flat.find(&remedy).unwrap();
    let argv: Vec<_> = flat[start..].split_whitespace().take(9).collect();
    assert_eq!(argv.join(" "), remedy);
    let out = f.run(&argv[1..], None);
    assert!(out.status.success(), "printed unbind remedy failed");
    let native = f.json(&["ai", "bindings", "show", "qwen"], None);
    assert_eq!(native["binding"]["mode"], "native");
    assert!(native["binding"]["connection"].is_null());
    let default_after = f.json(&["ai", "connections", "default", "show"], None);
    assert_eq!(default_after["connection"], aid);
    let pi_after = f.json(&["ai", "bindings", "show", "pi"], None);
    for field in ["harness", "mode", "connection", "resolution"] {
        assert!(
            pi_after["binding"][field] == pi_before["binding"][field],
            "unbind changed unrelated binding"
        );
    }
    assert!(
        f.json(&["secrets", "list"], None) == keyring_before,
        "unbind changed keyring"
    );
    assert!(
        f.json(&["ai", "connections", "details", &aid], None)["items"][&aid] == a["items"][&aid],
        "unbind changed unrelated connection"
    );
    let after = f.tree();
    for (path, entry) in &before {
        if path != Path::new("secrets/ai/index.toml") {
            assert!(
                after.get(path) == Some(entry),
                "unbind changed unrelated fixture data"
            );
        }
    }
    assert_eq!(after.len(), before.len());
    let version = native["binding"]["store_revision"].to_string();
    let removed = f.json(
        &[
            "ai",
            "connections",
            "remove",
            &bid,
            "--expect-store",
            &version,
            "--expect-current",
            brev,
            "--yes",
        ],
        None,
    );
    assert!(removed["items"].get(&bid).is_none());
    assert!(
        removed["items"][&aid] == a["items"][&aid],
        "successful removal changed unrelated connection"
    );
}

#[test]
fn fix13_connection_stale_binding_names_connection_store_and_preserves_state() {
    let f = Fixture::new();
    let secret = f.secret();
    let added = f.connection(
        secret["items"][0]["id"].as_str().unwrap(),
        "Synthetic connection",
    );
    let id = added["items"].as_object().unwrap().keys().next().unwrap();
    let old = added["store_revision"].to_string();
    let bound = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "connection",
            "--connection",
            id,
            "--expect-store",
            &old,
        ],
        None,
    );
    assert_ne!(bound["binding"]["store_revision"], added["store_revision"]);
    let text = f.refusals(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "native",
            "--expect-store",
            &old,
        ],
        "connection_store_stale",
    );
    assert!(
        f.json(&["ai", "bindings", "show", "qwen"], None) == bound,
        "stale unbind changed binding"
    );
    let flat = words(&text);
    require(
        &text,
        &[
            (
                "stale connection store is named",
                flat.contains("connection store"),
            ),
            (
                "stale connection store is not called keyring",
                !flat.contains("keyring"),
            ),
            (
                "connection refresh remedy",
                flat.contains("vibe ai connections list"),
            ),
        ],
    );
}
