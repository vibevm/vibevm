mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use common::{UserScratch, slot_dir};
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, SourceUrl};
use vibe_wire::generated::extensions_report::{
    ExtensionsReport, Handler, IrLevel, ManifestKind, PackageKind as ReportPackageKind, PassKind,
    ProviderSource, SelectorSubjectKind, State, Tier,
};

fn project(manifest: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("vibe.toml"), manifest).unwrap();
    project
}

fn locked(group: &str, name: &str, kind: PackageKind, version: &str, hash: &str) -> LockedPackage {
    LockedPackage {
        kind,
        name: PackageName::parse(name).unwrap(),
        group: Group::parse(group).unwrap(),
        version: version.parse().unwrap(),
        registry: None,
        source_url: SourceUrl::new("file:///fixture"),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse(hash).unwrap(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: Vec::new(),
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    }
}

fn seed_slot(root: &Path, package: &LockedPackage, body: &str) -> PathBuf {
    let slot = root.join(slot_dir(
        &format!("{}.{}", package.group, package.name),
        &package.version.to_string(),
    ));
    fs::create_dir_all(&slot).unwrap();
    fs::write(slot.join("vibe.toml"), body).unwrap();
    slot
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TreeEntry {
    Directory,
    File(Vec<u8>),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadOnlySnapshot {
    project: BTreeMap<String, TreeEntry>,
    settings: BTreeMap<String, TreeEntry>,
    cache: BTreeMap<String, TreeEntry>,
    search_cache: BTreeMap<String, TreeEntry>,
}

fn snapshot(root: &Path) -> BTreeMap<String, TreeEntry> {
    fn walk(base: &Path, at: &Path, out: &mut BTreeMap<String, TreeEntry>) {
        let mut entries = fs::read_dir(at)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let file_type = entry.file_type().unwrap();
            if file_type.is_dir() {
                out.insert(relative, TreeEntry::Directory);
                walk(base, &path, out);
            } else if file_type.is_file() {
                out.insert(relative, TreeEntry::File(fs::read(path).unwrap()));
            } else {
                out.insert(relative, TreeEntry::Other);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn read_only_snapshot(user: &UserScratch, project: &Path) -> ReadOnlySnapshot {
    ReadOnlySnapshot {
        project: snapshot(project),
        settings: snapshot(&user.settings),
        cache: snapshot(&user.cache),
        search_cache: snapshot(&user.search_cache),
    }
}

fn query_vibe(user: &UserScratch) -> assert_cmd::Command {
    let mut command = vibe_test_support::cargo_bin("vibe");
    command
        .env(vibe_core::settings::SETTINGS_DIR_ENV, &user.settings)
        .env(vibe_test_support::REGISTRY_CACHE_ENV, &user.cache)
        .env(vibe_test_support::SEARCH_CACHE_ENV, &user.search_cache)
        .env_remove("VIBE_NO_DEFAULT_REGISTRY");
    command
}

fn assert_query_paths_absent(user: &UserScratch, root: &Path, host_only: bool) {
    assert!(!user.settings.join("registry.toml").exists());
    assert!(!root.join(".vibe/lifecycle.toml").exists());
    assert!(!root.join(".vibe/lifecycle").exists());
    assert!(!root.join(".vibe/artifacts").exists());
    assert!(!root.join("target").exists());
    if host_only {
        assert!(!root.join("vibe.lock").exists());
        assert!(
            !root
                .join(vibe_core::layout::current_vibedeps_root())
                .exists()
        );
    }
}

fn json_report(user: &UserScratch, root: &Path, json_first: bool) -> ExtensionsReport {
    let mut command = query_vibe(user);
    if json_first {
        command.args(["--json", "extensions", "--path"]);
    } else {
        command.args(["extensions", "--path"]);
    }
    command.arg(root);
    if !json_first {
        command.arg("--json");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(documents.len(), 1);
    serde_json::from_value(documents.into_iter().next().unwrap()).unwrap()
}

fn row<'a>(
    report: &'a ExtensionsReport,
    suffix: &str,
) -> &'a vibe_wire::generated::extensions_report::ExtensionEntry {
    report
        .declarations
        .iter()
        .find(|row| row.key.ends_with(suffix))
        .unwrap_or_else(|| panic!("missing row {suffix}: {report:?}"))
}

#[test]
fn exhaustive_json_and_human_views_keep_every_state_provider_and_capability() {
    let user = UserScratch::new();
    let project = project(
        r#"[project]
name = "demo"
version = "0.1.0"

[active]
stack = "selected-stack"

[requires.packages]
"org.zed/tools" = "=9.0.0"
"org.stack/selected-stack" = "=1.2.3"
"org.stack/other-stack" = "=1.0.0"

[extensions]
disable = ["org.zed/tools#disabled"]

[[extensions.use]]
ref = "org.zed/tools#activated"
config = {}

[[extension]]
id = "host"
point = "phase:create"
handler = { kind = "agent", prompt = "spec://org.demo/host/prompts/create#root" }
"#,
    );
    let tools = locked("org.zed", "tools", PackageKind::Tool, "9.0.0", "sha256:aa");
    let unrelated = locked(
        "org.skip",
        "unreachable",
        PackageKind::Flow,
        "7.0.0",
        "sha256:ff",
    );
    let selected = locked(
        "org.stack",
        "selected-stack",
        PackageKind::Stack,
        "1.2.3",
        "sha256:bb",
    );
    let other = locked(
        "org.stack",
        "other-stack",
        PackageKind::Stack,
        "1.0.0",
        "sha256:cc",
    );
    seed_slot(
        project.path(),
        &tools,
        r#"[package]
group = "org.zed"
name = "tools"
kind = "tool"
version = "9.0.0"

[[extension]]
id = "disabled"
point = "phase:test"
handler = { kind = "builtin", name = "log" }
config = { message = "disabled" }

[[extension]]
id = "native"
point = "compile:pass"
handler = { kind = "native", crate_dir = "native", prebuilt = { windows-x86_64 = "prebuilt/plugin.dll" } }
auto = true
compiler_internals = true
pass = { kind = "transform", level = "closure", after = "qualify" }
when = { future = true }

[[extension]]
id = "mismatch"
point = "slot:pre-install"
handler = { kind = "script", base = "hooks/select" }
applies_to = { paths = ["src/**"] }

[[extension]]
id = "activated"
point = "compile:source"
handler = { kind = "builtin", name = "log" }
config = { original = "yes" }
auto = false
"#,
    );
    seed_slot(
        project.path(),
        &selected,
        r#"[package]
group = "org.stack"
name = "selected-stack"
kind = "stack"
version = "1.2.3"

[[extension]]
id = "preset"
point = "phase:build"
handler = { kind = "script", base = "scripts/build" }
"#,
    );
    seed_slot(
        project.path(),
        &other,
        r#"[package]
group = "org.stack"
name = "other-stack"
kind = "stack"
version = "1.0.0"

[[extension]]
id = "other"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "other" }
"#,
    );
    let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    lock.packages = vec![tools, unrelated, selected, other];
    lock.write(project.path().join("vibe.lock")).unwrap();

    let before = read_only_snapshot(&user, project.path());
    let report = json_report(&user, project.path(), true);
    assert_eq!(read_only_snapshot(&user, project.path()), before);
    assert_query_paths_absent(&user, project.path(), false);
    assert_eq!(report.command, "extensions");
    assert!(report.ok);
    assert_eq!(report.count, 7);
    assert_eq!(report.effective_count, 4);
    assert_eq!(report.project.identity, "__host__/demo");
    assert_eq!(report.project.manifest_kind, ManifestKind::Project);
    assert_eq!(report.selector_subject.kind, SelectorSubjectKind::Unscoped);
    assert!(report.selector_subject.package.is_none());
    assert!(report.selector_subject.path.is_none());
    assert_eq!(
        report.project.effective_stack.as_deref(),
        Some("org.stack/selected-stack")
    );
    assert_eq!(
        report
            .declarations
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        [
            "org.stack/selected-stack#preset",
            "org.zed/tools#disabled",
            "org.zed/tools#native",
            "org.zed/tools#mismatch",
            "org.stack/other-stack#other",
            "__host__/demo#host",
            "org.zed/tools#activated",
        ]
    );

    let disabled = row(&report, "#disabled");
    assert_eq!(disabled.state, State::Disabled);
    assert!(disabled.disabled && !disabled.effective && disabled.auto);
    assert_eq!(disabled.sequence, 1);
    assert_eq!(disabled.order.provider, Some(0));
    assert_eq!(disabled.order.declaration, 0);
    assert!(disabled.order.activation.is_none());
    assert_eq!(disabled.provider.identity, "org.zed/tools");
    assert_eq!(disabled.provider.kind, Some(ReportPackageKind::Tool));
    assert_eq!(disabled.provider.content_hash.as_deref(), Some("sha256:aa"));
    assert!(
        disabled
            .provider
            .root
            .as_deref()
            .is_some_and(|root| root.ends_with("org.zed.tools/9.0.0"))
    );
    let native = row(&report, "#native");
    assert_eq!(native.state, State::Inactive);
    assert_eq!(native.authored_auto, Some(true));
    assert!(!native.auto);
    assert!(native.compiler_internals);
    assert_eq!(native.order.provider, Some(0));
    assert_eq!(native.order.declaration, 1);
    let Handler::Native(handler) = &native.handler else {
        panic!("expected native handler: {native:?}");
    };
    assert_eq!(handler.crate_dir.as_deref(), Some("native"));
    assert_eq!(
        handler
            .prebuilt
            .as_ref()
            .and_then(|paths| paths.get("windows-x86_64"))
            .map(String::as_str),
        Some("prebuilt/plugin.dll")
    );
    let pass = native.pass.as_ref().unwrap();
    assert_eq!(pass.kind, PassKind::Transform);
    assert_eq!(pass.level, Some(IrLevel::Closure));
    assert_eq!(pass.after.as_deref(), Some("qualify"));
    assert!(pass.from.is_none() && pass.to.is_none());
    assert!(pass.before.is_none() && pass.replace.is_none());
    assert!(pass.formats.is_none() && pass.artifact.is_none());
    assert_eq!(
        native.when.as_ref().unwrap().get("future"),
        Some(&Some(serde_json::Value::Bool(true)))
    );
    let observation = native.native.as_ref().unwrap();
    assert_eq!(observation.build_state, "unavailable");
    assert!(observation.artifact_path.is_none());
    assert!(observation.content_hash.is_none());
    let mismatch = row(&report, "#mismatch");
    assert_eq!(mismatch.state, State::SelectorMismatch);
    assert!(!mismatch.selector_matches);
    let selector = mismatch.applies_to.as_ref().unwrap();
    assert!(selector.packages.is_none());
    assert_eq!(selector.paths.as_deref().unwrap(), ["src/**"]);
    let preset = row(&report, "#preset");
    assert_eq!(preset.natural_tier, Tier::Preset);
    assert_eq!(preset.tier, Tier::Preset);
    assert_eq!(preset.sequence, 0);
    assert_eq!(preset.order.provider, Some(1));
    assert_eq!(preset.order.declaration, 0);
    let non_selected = row(&report, "#other");
    assert_eq!(non_selected.state, State::Effective);
    assert_eq!(non_selected.tier, Tier::Dependency);
    assert!(non_selected.auto && non_selected.effective);
    assert_eq!(non_selected.provider.version, "1.0.0");
    assert_eq!(non_selected.order.provider, Some(2));
    let activated = row(&report, "#activated");
    assert!(activated.activated && activated.effective);
    assert_eq!(activated.natural_tier, Tier::Dependency);
    assert_eq!(activated.tier, Tier::HostActivation);
    assert_eq!(activated.authored_auto, Some(false));
    assert_eq!(activated.order.provider, Some(0));
    assert_eq!(activated.order.declaration, 3);
    assert_eq!(activated.order.activation, Some(0));
    assert!(
        activated
            .authored_config
            .as_ref()
            .is_some_and(|config| config.contains_key("original"))
    );
    assert!(
        activated
            .effective_config
            .as_ref()
            .is_some_and(|config| config.is_empty())
    );
    let host = row(&report, "#host");
    assert!(host.authored_config.is_none());
    assert_eq!(host.provider.source, ProviderSource::Host);
    assert_eq!(host.provider.identity, "__host__/demo");
    assert_eq!(host.provider.version, "0.1.0");
    assert!(host.provider.kind.is_none());
    assert!(host.provider.content_hash.is_none());
    assert_eq!(
        host.provider.root.as_deref(),
        Some(report.project.root.as_str())
    );
    assert_eq!(host.order.provider, None);
    assert_eq!(host.order.declaration, 0);
    let Handler::Agent(handler) = &host.handler else {
        panic!("expected agent handler: {host:?}");
    };
    assert_eq!(handler.prompt, "spec://org.demo/host/prompts/create#root");
    assert_eq!(
        row(&report, "#disabled").provider.source,
        ProviderSource::Dependency
    );
    assert_eq!(row(&report, "#disabled").provider.version, "9.0.0");
    assert!(
        !report
            .declarations
            .iter()
            .any(|row| row.key.contains("unreachable"))
    );

    let again = json_report(&user, project.path(), false);
    assert_eq!(again, report);
    assert_eq!(read_only_snapshot(&user, project.path()), before);

    let output = query_vibe(&user)
        .args(["extensions", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let human = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        report
            .declarations
            .iter()
            .filter(|row| human.matches(&row.key).count() == 1)
            .count(),
        report.declarations.len()
    );
    for state in ["disabled", "inactive", "selector-mismatch", "effective"] {
        assert!(human.contains(&format!("state={state}")), "{human}");
    }
    assert_eq!(read_only_snapshot(&user, project.path()), before);
    assert_query_paths_absent(&user, project.path(), false);
}

#[test]
fn host_only_query_is_pure_and_missing_or_broken_worlds_fail_loudly() {
    let user = UserScratch::new();
    let host = project("[project]\nname='empty'\nversion='0.1.0'\n");
    let before = read_only_snapshot(&user, host.path());
    let report = json_report(&user, host.path(), true);
    assert_eq!(report.count, 0);
    assert_eq!(report.project.identity, "__host__/empty");
    assert_eq!(read_only_snapshot(&user, host.path()), before);
    let human = query_vibe(&user)
        .args(["extensions", "--path"])
        .arg(host.path())
        .output()
        .unwrap();
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains("0 extension declaration(s)")
    );
    assert_eq!(read_only_snapshot(&user, host.path()), before);
    assert_query_paths_absent(&user, host.path(), true);

    let required = project(
        "[project]\nname='required'\nversion='0.1.0'\n[requires.packages]\n\"org.demo/a\"='=1.0.0'\n",
    );
    assert_error(&user, required.path(), "absent from effective-world lock");

    let controlled = project(
        "[project]\nname='controlled'\nversion='0.1.0'\n[[extensions.use]]\nref='org.demo/a#x'\n",
    );
    assert_error(&user, controlled.path(), "unresolved [[extensions.use]]");

    let disabled = project(
        "[project]\nname='disabled'\nversion='0.1.0'\n[extensions]\ndisable=['org.demo/a#x']\n",
    );
    assert_error(&user, disabled.path(), "unknown [extensions].disable");

    let orphan = project("[project]\nname='orphan'\nversion='0.1.0'\n");
    fs::create_dir_all(
        orphan
            .path()
            .join(vibe_core::layout::current_vibedeps_root()),
    )
    .unwrap();
    assert_error(&user, orphan.path(), "exists without");

    for (case, seed, needle) in [
        ("missing-slot", false, "has no materialised"),
        ("missing-manifest", true, "reading reachable slot manifest"),
    ] {
        let broken = project(&format!(
            "[project]\nname='{case}'\nversion='0.1.0'\n[requires.packages]\n\"org.demo/a\"='=1.0.0'\n"
        ));
        let package = locked("org.demo", "a", PackageKind::Flow, "1.0.0", "sha256:aa");
        let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
        lock.packages = vec![package.clone()];
        lock.write(broken.path().join("vibe.lock")).unwrap();
        if seed {
            let slot = broken.path().join(slot_dir("org.demo.a", "1.0.0"));
            fs::create_dir_all(slot).unwrap();
        }
        assert_error(&user, broken.path(), needle);
    }

    let malformed = project(
        "[project]\nname='malformed'\nversion='0.1.0'\n[requires.packages]\n\"org.demo/a\"='=1.0.0'\n",
    );
    let package = locked("org.demo", "a", PackageKind::Flow, "1.0.0", "sha256:aa");
    seed_slot(malformed.path(), &package, "not = [valid");
    let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    lock.packages = vec![package];
    lock.write(malformed.path().join("vibe.lock")).unwrap();
    assert_error(&user, malformed.path(), "reading reachable slot manifest");

    let mismatch = project(
        "[project]\nname='mismatch'\nversion='0.1.0'\n[requires.packages]\n\"org.demo/a\"='=1.0.0'\n",
    );
    let package = locked("org.demo", "a", PackageKind::Flow, "1.0.0", "sha256:aa");
    seed_slot(
        mismatch.path(),
        &package,
        "[package]\ngroup='org.demo'\nname='wrong'\nkind='flow'\nversion='1.0.0'\n",
    );
    let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    lock.packages = vec![package];
    lock.write(mismatch.path().join("vibe.lock")).unwrap();
    assert_error(&user, mismatch.path(), "but the lock requires");
}

fn assert_error(user: &UserScratch, root: &Path, needle: &str) {
    let output = query_vibe(user)
        .args(["extensions", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(needle), "expected `{needle}` in {error}");
    assert!(
        error.contains("run `vibe install`"),
        "expected exact recovery command in {error}"
    );
}
