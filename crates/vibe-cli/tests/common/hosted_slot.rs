//! Shared fixture for the hosted install-family e2e: one published package
//! whose only contribution is a slot-scoped `agent` row, plus a builtin
//! sentinel after it.
//!
//! The sentinel is the mutation detector every case in this family needs: it
//! must be absent on the parking invocation and present, `ok`, only after the
//! park is satisfied.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use super::{run_git, write_project_with_per_package_registry};

/// The provider answer a PAID run would produce. Every hosted case asserts the
/// provider was never reached, so this exists to make a fall-through visible.
pub const PAID_RESULT: &str =
    r#"{"outputs":[{"path":"docs/slot.md","content":"paid slot body\n"}]}"#;

/// What the hosting agent writes to satisfy the declared contract.
pub const HOSTED_BODY: &str = "hosted slot body\n";

pub struct Published {
    pub registry: PathBuf,
    pub source: PathBuf,
    /// Whether the published versions carry a `[boot_snippet]`. Private on
    /// purpose: it is a property of HOW this `Published` was minted, and only
    /// the version-adding helpers below need to consult it — a caller that
    /// wanted to inspect it would be re-deriving a fact it already chose.
    with_boot: bool,
}

/// Publish `org.demo/tools@<version>` with a slot `agent` row at `point`.
/// The package declares no boot snippet, so a consumer links it dynamically
/// and nothing compiles — the shape the park/no-spend family is built on.
pub fn publish_slot_agent(root: &Path, point: &str, version: &str) -> Published {
    publish(root, point, version, false)
}

/// The boot-bearing variant: the same slot `agent` row, plus a `[boot_snippet]`
/// a consumer can link STATICALLY. A traced run over this package really
/// compiles, so "the resume appended" is a number that can move. The boot body
/// names the version, so every bump changes the compiled input.
pub fn publish_slot_agent_with_boot(root: &Path, point: &str, version: &str) -> Published {
    publish(root, point, version, true)
}

fn publish(root: &Path, point: &str, version: &str, with_boot: bool) -> Published {
    let source = root.join(format!("src-{version}"));
    fs::create_dir_all(source.join("vibevm/vibespecs/common")).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    fs::write(source.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    write_version(&source, point, version, with_boot);
    run_git(&source, &["add", "-A"]);
    run_git(&source, &["commit", "-m", "publish"]);
    run_git(&source, &["tag", &format!("v{version}")]);

    let bare = root.join("org.demo.tools.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    Published {
        registry: root.to_path_buf(),
        source,
        with_boot,
    }
}

/// Commit and tag one more version of an already-published source, so
/// `vibe update` has somewhere to move to. A boot-bearing source keeps its
/// boot declaration, with the body bumped to name the new version.
pub fn add_version(published: &Published, point: &str, version: &str) {
    write_version(&published.source, point, version, published.with_boot);
    run_git(&published.source, &["add", "-A"]);
    run_git(&published.source, &["commit", "-m", "next"]);
    run_git(&published.source, &["tag", &format!("v{version}")]);
    let bare = published.registry.join("org.demo.tools.git");
    run_git(&bare, &["fetch", "origin", "+refs/*:refs/*", "--prune"]);
}

/// Commit and tag a version whose slot `agent` DECLARATION IS GONE — only the
/// builtin sentinel remains. A run that parked on the old version has a
/// slot-scoped row nothing in the new plan will ever visit again. A
/// boot-bearing source keeps its boot declaration, body bumped to match.
pub fn add_version_without_agent(published: &Published, point: &str, version: &str) {
    let boot_table = boot_table(published.with_boot);
    fs::write(
        published.source.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.demo"
name = "tools"
kind = "flow"
version = "{version}"
{boot_table}
[[extension]]
id = "after-agent"
point = "{point}"
handler = {{ kind = "builtin", name = "log" }}
config = {{ message = "SENTINEL-AFTER-SLOT-AGENT" }}
"#
        ),
    )
    .unwrap();
    if published.with_boot {
        write_boot_body(&published.source, version);
    }
    fs::write(
        published.source.join("payload.txt"),
        format!(
            "payload {version}
"
        ),
    )
    .unwrap();
    run_git(&published.source, &["add", "-A"]);
    run_git(&published.source, &["commit", "-m", "drop the agent row"]);
    run_git(&published.source, &["tag", &format!("v{version}")]);
    let bare = published.registry.join("org.demo.tools.git");
    run_git(&bare, &["fetch", "origin", "+refs/*:refs/*", "--prune"]);
}

fn write_version(source: &Path, point: &str, version: &str, with_boot: bool) {
    let boot_table = boot_table(with_boot);
    fs::write(
        source.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.demo"
name = "tools"
kind = "flow"
version = "{version}"
{boot_table}
[[extension]]
id = "slot-produce"
point = "{point}"
handler = {{ kind = "agent", prompt = "spec://org.demo/tools/common/agent-prompt#root" }}
config.outputs = [
  {{ path = "docs/slot.md", kind = "file", accept = "non-empty file" }},
]

[[extension]]
id = "after-agent"
point = "{point}"
handler = {{ kind = "builtin", name = "log" }}
config = {{ message = "SENTINEL-AFTER-SLOT-AGENT" }}
"#
        ),
    )
    .unwrap();
    if with_boot {
        write_boot_body(source, version);
    }
    fs::write(
        source.join("vibevm/vibespecs/common/agent-prompt.md"),
        "# Prompt {#root}\n\nWrite the slot document. MARKER=SLOT\n",
    )
    .unwrap();
    fs::write(source.join("payload.txt"), format!("payload {version}\n")).unwrap();
}

/// The `[boot_snippet]` table a boot-bearing version declares — empty for the
/// plain variant, so the non-boot manifest stays byte-for-byte what it was.
fn boot_table(with_boot: bool) -> &'static str {
    if with_boot {
        "\n[boot_snippet]\nsource = \"boot/40-tools.md\"\ncategory = \"flow\"\n"
    } else {
        ""
    }
}

/// The boot body a boot-bearing version compiles from. The version is IN the
/// body on purpose: a bump changes the compiled input, not just the tag, so a
/// consumer's static link really recompiles over the new version.
fn write_boot_body(source: &Path, version: &str) {
    fs::create_dir_all(source.join("boot")).unwrap();
    fs::write(
        source.join("boot/40-tools.md"),
        format!("# Tools {{#root}}\n\nTOOLS BOOT BODY {version}\n"),
    )
    .unwrap();
}

/// Declare a STATIC requirement on `flow:org.demo/tools` — the link mode that
/// makes the node compile at all. A dynamic edge contributes an `INDEX.md`
/// line and no compiled artifact, so a traced run over one records nothing and
/// proves nothing.
pub fn declare_static_tools(project: &Path) {
    let manifest = project.join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(
        "\n[requires]\npackages = { \"flow:org.demo/tools\" = { version = \"^0.1\", link = \"static\" } }\n",
    );
    fs::write(&manifest, text).unwrap();
}

/// Publish a SECOND package with no lifecycle contributions at all, into the
/// same per-package registry root. Used where a case needs a dependency that
/// is resolved but does not move — the difference between "how many packages
/// this run resolved" and "how many slots it wrote".
pub fn publish_plain(registry: &Path, version: &str) {
    let source = registry.join("src-plain");
    fs::create_dir_all(&source).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    fs::write(
        source.join(".gitattributes"),
        "* text=auto eol=lf
",
    )
    .unwrap();
    fs::write(
        source.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.demo"
name = "plain"
kind = "flow"
version = "{version}"
"#
        ),
    )
    .unwrap();
    fs::write(
        source.join("payload.txt"),
        "plain payload
",
    )
    .unwrap();
    run_git(&source, &["add", "-A"]);
    run_git(&source, &["commit", "-m", "org.demo/plain"]);
    run_git(&source, &["tag", &format!("v{version}")]);
    let bare = registry.join("org.demo.plain.git");
    run_git(
        registry,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
}

/// Publish `org.demo/tools@<version>` with TWO ORDERED slot `agent` rows at
/// the same point, plus the builtin sentinel after them.
///
/// One slot target, two declared handoffs. The second may only park after the
/// first is satisfied, which is what makes the zero-debt instant between them
/// observable: at that moment no delegated slot row is live, so the durable
/// continuation is correctly dropped — and the run must still be able to name
/// the same target set when the second row parks.
pub fn publish_two_slot_agents(root: &Path, point: &str, version: &str) -> Published {
    let source = root.join(format!("src-two-{version}"));
    fs::create_dir_all(source.join("vibevm/vibespecs/common")).unwrap();
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    fs::write(
        source.join(".gitattributes"),
        "* text=auto eol=lf
",
    )
    .unwrap();
    fs::write(
        source.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.demo"
name = "tools"
kind = "flow"
version = "{version}"

[[extension]]
id = "slot-produce"
point = "{point}"
handler = {{ kind = "agent", prompt = "spec://org.demo/tools/common/agent-prompt#root" }}
config.outputs = [
  {{ path = "docs/slot.md", kind = "file", accept = "non-empty file" }},
]

[[extension]]
id = "slot-produce-second"
point = "{point}"
handler = {{ kind = "agent", prompt = "spec://org.demo/tools/common/agent-prompt#root" }}
config.outputs = [
  {{ path = "docs/slot-second.md", kind = "file", accept = "non-empty file" }},
]

[[extension]]
id = "after-agent"
point = "{point}"
handler = {{ kind = "builtin", name = "log" }}
config = {{ message = "SENTINEL-AFTER-SLOT-AGENT" }}
"#
        ),
    )
    .unwrap();
    fs::write(
        source.join("vibevm/vibespecs/common/agent-prompt.md"),
        "# Prompt {#root}

Write the slot document. MARKER=SLOT
",
    )
    .unwrap();
    fs::write(
        source.join("payload.txt"),
        format!(
            "payload {version}
"
        ),
    )
    .unwrap();
    run_git(&source, &["add", "-A"]);
    run_git(&source, &["commit", "-m", "publish two"]);
    run_git(&source, &["tag", &format!("v{version}")]);

    let bare = root.join("org.demo.tools.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    Published {
        registry: root.to_path_buf(),
        source,
        with_boot: false,
    }
}

/// The second declared output, satisfied by the hosting agent.
pub fn write_second_declared_output(project: &Path) {
    fs::create_dir_all(project.join("docs")).unwrap();
    fs::write(project.join("docs/slot-second.md"), HOSTED_BODY).unwrap();
}

pub fn registry_url(registry: &Path) -> String {
    format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    )
}

/// A project wired to the published registry.
pub fn project_at(user: &super::UserScratch, registry: &Path) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(registry));
    project
}

/// Every JSON document on stdout, in order.
pub fn documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// The invocation's SOLE JSON document, parsed as the caller's registered
/// root. A plan preview, a per-row echo or a second report all fail here.
pub fn sole_document(bytes: &[u8]) -> serde_json::Value {
    let docs = documents(bytes);
    assert_eq!(
        docs.len(),
        1,
        "hosted parking emits exactly one JSON document: {}",
        String::from_utf8_lossy(bytes),
    );
    docs.into_iter().next().unwrap()
}

/// The sole document of the named root command, for a run that COMPLETED and
/// therefore also flushed its plan preview.
pub fn sole_root(bytes: &[u8], command: &str) -> serde_json::Value {
    let docs = documents(bytes);
    let roots: Vec<&serde_json::Value> = docs
        .iter()
        .filter(|doc| doc["command"] == command)
        .collect();
    assert_eq!(
        roots.len(),
        1,
        "exactly one `{command}` root document: {}",
        String::from_utf8_lossy(bytes),
    );
    roots[0].clone()
}

/// The hosting agent performs the declared work.
pub fn write_declared_output(project: &Path) {
    fs::create_dir_all(project.join("docs")).unwrap();
    fs::write(project.join("docs/slot.md"), HOSTED_BODY).unwrap();
}

pub fn lifecycle_state(project: &Path) -> vibe_wire::generated::lifecycle_state::LifecycleState {
    toml::from_str(&fs::read_to_string(project.join(".vibe/lifecycle.toml")).unwrap()).unwrap()
}

pub fn assert_ok(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
