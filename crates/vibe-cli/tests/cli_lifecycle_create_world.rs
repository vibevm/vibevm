//! Which instance of a provider an agent prompt is read from — and that a
//! slot-scoped agent contribution is executed, not degraded.
//!
//! The oracle is the loopback endpoint's recorded request body: a hit counter
//! proves a call happened, but only the bytes prove WHICH document produced
//! it. Every fixture below plants two candidate documents that differ in one
//! marker and asserts the marker the request actually carried.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use common::agent_provider::{MockProvider, configure_provider};
use vibe_core::manifest::Lockfile;

const RESULT: &str = r#"{"outputs":[{"path":"docs/guide.md","content":"guide body\n"}]}"#;

fn contract(prompt: &str) -> String {
    format!(
        "\n[[extension]]\nid = \"produce-docs\"\npoint = \"phase:create\"\n\
         handler = {{ kind = \"agent\", prompt = \"{prompt}\" }}\n\
         config.outputs = [\n  \
         {{ path = \"docs/guide.md\", kind = \"file\", accept = \"non-empty file\" }},\n]\n",
    )
}

/// A prompt document whose body carries one distinguishing marker.
fn seed_prompt(root: &Path, marker: &str) {
    let specs = root.join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        format!("# Documentation prompt {{#root}}\n\nWrite the guide. MARKER={marker}\n"),
    )
    .unwrap();
}

fn slot(root: &Path, group: &str, name: &str, version: &str) -> PathBuf {
    root.join(vibe_core::layout::current_vibedeps_root())
        .join(format!("{group}.{name}"))
        .join(version)
}

/// Seed one materialised dependency slot that declares the agent contribution
/// and ships its own prompt document.
fn seed_slot(root: &Path, group: &str, name: &str, version: &str, marker: &str) {
    let slot = slot(root, group, name, version);
    fs::create_dir_all(&slot).unwrap();
    fs::write(
        slot.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"tool\"\n\
             version = \"{version}\"\n{}",
            contract(&format!("spec://{group}/{name}/common/agent-prompt#root")),
        ),
    )
    .unwrap();
    seed_prompt(&slot, marker);
}

/// `vibe init` derives the project name from the temp directory, so the fixtures
/// that address the HOST rename it to a stable coordinate first.
fn rename_project_to_demo(project: &Path) {
    let manifest_path = project.join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let mut patched = String::new();
    for line in manifest.lines() {
        if line.starts_with("name = ") && !patched.contains("name = \"demo\"") {
            patched.push_str("name = \"demo\"\ngroup = \"org.demo\"\n");
        } else {
            patched.push_str(line);
            patched.push('\n');
        }
    }
    fs::write(manifest_path, patched).unwrap();
}

fn init(user: &UserScratch, dir: &Path) {
    user.vibe()
        .args(["init", "--no-registry", "--author", "Agent"])
        .arg("--path")
        .arg(dir)
        .assert()
        .success();
}

fn create(user: &UserScratch, project: &Path) -> std::process::Output {
    user.vibe()
        .args(["create", "--json", "--assume-yes", "--path"])
        .arg(project)
        .output()
        .unwrap()
}

/// `vibe create` chains install, which resolves declared requirements, so the
/// fixture registry has to stay reachable for the chained phase too.
fn create_with_registry(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
) -> std::process::Output {
    user.vibe()
        .args(["create", "--json", "--assume-yes", "--path"])
        .arg(project)
        .arg("--registry")
        .arg(registry)
        .output()
        .unwrap()
}

/// The lock selects 1.0.0 while a newer 2.0.0 sits installed beside it. The
/// declarations execute from 1.0.0, so the prompt bytes must come from 1.0.0
/// too — resolving by coordinate alone would answer with the semver-newest
/// slot and let an unselected package supply the instructions.
#[test]
fn a_dependency_prompt_comes_from_the_locked_version_not_the_newest_installed() {
    let provider = MockProvider::serving(RESULT);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    init(&user, project.path());
    configure_provider(&user, &provider.endpoint());

    // The registry ships 1.0.0 only, so the ordinary install resolves and
    // materialises exactly that version and writes the lock for it.
    let registry_root = tempfile::tempdir().unwrap();
    let published = registry_root.path().join("registry/org.demo/tools/v1.0.0");
    fs::create_dir_all(&published).unwrap();
    fs::write(
        published.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"tool\"\n\
             version = \"1.0.0\"\n{}",
            contract("spec://org.demo/tools/common/agent-prompt#root"),
        ),
    )
    .unwrap();
    seed_prompt(&published, "LOCKED-ONE");

    let installed = user
        .vibe()
        .arg("install")
        .arg("org.demo/tools@=1.0.0")
        .arg("--registry")
        .arg(registry_root.path().join("registry"))
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    assert!(slot(project.path(), "org.demo", "tools", "1.0.0").is_dir());
    // Now a NEWER slot of the same coordinate exists beside it, unselected by
    // the lock — exactly the state a resolver that searches by coordinate
    // would answer with.
    seed_slot(project.path(), "org.demo", "tools", "2.0.0", "NEWER-TWO");
    let lock = Lockfile::read(project.path().join("vibe.lock")).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].version.to_string(), "1.0.0");

    let output = create_with_registry(
        &user,
        project.path(),
        &registry_root.path().join("registry"),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(provider.hits(), 1);
    let sent = provider.bodies().join("\n");
    assert!(
        sent.contains("LOCKED-ONE"),
        "the selected version's prompt must be the one sent"
    );
    assert!(
        !sent.contains("NEWER-TWO"),
        "an installed-but-unselected version must never supply the instructions"
    );
}

/// A workspace member's prompt is the member's own document, even when the
/// workspace root authors a colliding doc-path. Rooting resolution at the
/// workspace root would silently serve the root project's bytes.
#[test]
fn a_member_prompt_comes_from_the_member_not_the_colliding_workspace_root() {
    let provider = MockProvider::serving(RESULT);
    let user = UserScratch::new();
    let workspace = tempfile::tempdir().unwrap();
    init(&user, workspace.path());
    configure_provider(&user, &provider.endpoint());

    let member = workspace.path().join("members/tools");
    fs::create_dir_all(&member).unwrap();
    fs::write(
        member.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"tool\"\n\
             version = \"0.1.0\"\npublish = false\n{}",
            contract("spec://org.demo/tools/common/agent-prompt#root"),
        ),
    )
    .unwrap();
    seed_prompt(&member, "MEMBER-OWN");

    // The workspace root authors the SAME doc-path with different bytes.
    seed_prompt(workspace.path(), "ROOT-COLLIDING");
    let root_manifest = workspace.path().join("vibe.toml");
    let mut body = fs::read_to_string(&root_manifest).unwrap();
    body.push_str("\n[workspace]\nmembers = [\"members/tools\"]\n");
    fs::write(&root_manifest, body).unwrap();

    let output = create(&user, &member);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(provider.hits(), 1);
    let sent = provider.bodies().join("\n");
    assert!(
        sent.contains("MEMBER-OWN"),
        "the member's own document must be the one sent"
    );
    assert!(
        !sent.contains("ROOT-COLLIDING"),
        "a colliding workspace-root document must never be substituted"
    );
    assert_eq!(
        fs::read_to_string(member.join("docs/guide.md")).unwrap(),
        "guide body\n"
    );
}

/// `agent` is legal at `slot:` points, so a contribution that runs at the
/// install barrier must reach the configured provider exactly as a
/// phase-scoped one does — never the refusing default.
#[test]
fn a_slot_scoped_agent_contribution_reaches_the_configured_provider() {
    let provider = MockProvider::serving(RESULT);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    let registry_root = tempfile::tempdir().unwrap();
    init(&user, project.path());
    configure_provider(&user, &provider.endpoint());

    let package = registry_root.path().join("registry/org.demo/tools/v0.1.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"tool\"\n\
         version = \"0.1.0\"\n\n\
         [[extension]]\nid = \"slot-produce\"\npoint = \"slot:post-install\"\n\
         handler = { kind = \"agent\", prompt = \"spec://org.demo/tools/common/agent-prompt#root\" }\n\
         config.outputs = [\n  \
         { path = \"docs/guide.md\", kind = \"file\", accept = \"non-empty file\" },\n]\n",
    )
    .unwrap();
    seed_prompt(&package, "SLOT-SCOPED");

    let output = user
        .vibe()
        .arg("--json")
        .arg("install")
        .arg("org.demo/tools@=0.1.0")
        .arg("--registry")
        .arg(registry_root.path().join("registry"))
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        provider.hits(),
        1,
        "the install barrier must use the CLI's agent backend, not the refusing default"
    );
    assert!(
        provider.bodies().join("\n").contains("SLOT-SCOPED"),
        "and it must resolve the slot package's own prompt"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "guide body\n",
        "a slot-scoped agent contribution writes its declared outputs"
    );
}

/// A prompt whose closure reaches ANOTHER package. `#embed` is the whole of
/// this handler's composition, and it must resolve through the lock-selected
/// world: package B is installed twice, the lock chose 1.0, and the request
/// must carry B1's bytes and never B2's.
#[test]
fn a_cross_package_embed_resolves_only_through_the_lock_selected_world() {
    let provider = MockProvider::serving(RESULT);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    init(&user, project.path());
    configure_provider(&user, &provider.endpoint());

    // A ships the agent contribution; its prompt embeds a section of B.
    let registry_root = tempfile::tempdir().unwrap();
    let registry = registry_root.path().join("registry");
    for (name, version, body) in [("a", "1.0.0", None), ("b", "1.0.0", Some("EMBEDDED-B1"))] {
        let published = registry.join(format!("org.demo/{name}/v{version}"));
        fs::create_dir_all(&published).unwrap();
        let extension = if name == "a" {
            contract("spec://org.demo/a/common/agent-prompt#root")
        } else {
            String::new()
        };
        let requires = if name == "a" {
            "\n[requires.packages]\n\"org.demo/b\" = \"=1.0.0\"\n"
        } else {
            ""
        };
        fs::write(
            published.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.demo\"\nname = \"{name}\"\nkind = \"tool\"\n\
                 version = \"{version}\"\n{requires}{extension}"
            ),
        )
        .unwrap();
        match body {
            Some(marker) => seed_prompt(&published, marker),
            None => {
                let specs = published.join("vibevm/vibespecs/common");
                fs::create_dir_all(&specs).unwrap();
                fs::write(
                    specs.join("agent-prompt.md"),
                    "# Documentation prompt {#root}\n\n\
                     Write the guide.\n\n#embed spec://org.demo/b/common/agent-prompt#root\n",
                )
                .unwrap();
            }
        }
    }

    let installed = user
        .vibe()
        .arg("install")
        .arg("org.demo/a@=1.0.0")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    // A newer B sits installed beside the selected one, unreachable by the lock.
    seed_slot(project.path(), "org.demo", "b", "2.0.0", "EMBEDDED-B2");

    let output = create_with_registry(&user, project.path(), &registry);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(provider.hits(), 1);
    let sent = provider.bodies().join("\n");
    assert!(
        sent.contains("EMBEDDED-B1"),
        "the embedded section must come from the lock-selected instance"
    );
    assert!(
        !sent.contains("EMBEDDED-B2"),
        "an installed-but-unselected version must never be embedded"
    );
}

/// The same law from the other side: a coordinate the lock never selected is
/// unreachable, so the closure refuses instead of scanning for it.
#[test]
fn an_embed_of_an_unselected_coordinate_refuses_before_the_call() {
    let provider = MockProvider::serving(RESULT);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    init(&user, project.path());
    configure_provider(&user, &provider.endpoint());
    rename_project_to_demo(project.path());

    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(&contract("spec://org.demo/demo/common/agent-prompt#root"));
    fs::write(&manifest, body).unwrap();
    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Documentation prompt {#root}\n\nWrite the guide.\n\n\
         #embed spec://org.demo/never-installed/common/x#root\n",
    )
    .unwrap();

    let output = create(&user, project.path());
    assert!(!output.status.success());
    assert_eq!(provider.hits(), 0, "an unselected coordinate costs nothing");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not in this resolver's selected world"),
        "the refusal must name the selected-world law: {stderr}"
    );
}

/// `#use` and `#source` are composition this handler does not perform, and the
/// refusal happens through the real document scan, before any spend.
#[test]
fn a_prompt_using_unsupported_composition_refuses_before_the_call() {
    let provider = MockProvider::serving(RESULT);
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    init(&user, project.path());
    configure_provider(&user, &provider.endpoint());
    rename_project_to_demo(project.path());

    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(&contract("spec://org.demo/demo/common/agent-prompt#root"));
    fs::write(&manifest, body).unwrap();
    let specs = project.path().join("vibevm/vibespecs/common");
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("agent-prompt.md"),
        "# Documentation prompt {#root}\n\nWrite the guide.\n\n\
         #source spec://org.demo/demo/common/other#root\n",
    )
    .unwrap();

    let output = create(&user, project.path());
    assert!(!output.status.success());
    assert_eq!(provider.hits(), 0, "unsupported composition costs nothing");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("composition this handler does not perform"),
        "{stderr}"
    );
    assert!(
        stderr.contains("one addressed section plus recursive `#embed` expansion"),
        "the remediation must name what IS supported: {stderr}"
    );
}
