use specmark::verifies;

use super::*;
use crate::package_ref::PackageKind;

#[test]
fn new_project_is_valid_and_roundtrips() {
    let m = Manifest::new_project("demo", "0.0.1");
    m.validate().unwrap();
    assert!(!m.is_package());
    assert!(!m.is_workspace_root());
    let rendered = toml::to_string_pretty(&m).unwrap();
    let back = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(m, back);
}

#[test]
fn minimal_project_parses() {
    let m = Manifest::parse_str("[project]\nname = \"demo\"\nversion = \"0.0.1\"\n").unwrap();
    assert_eq!(m.require_project().unwrap().name, "demo");
    assert!(m.package.is_none());
    assert!(m.registries.is_empty());
}

#[test]
fn full_project_parses() {
    let raw = r#"[project]
name = "my-client"
version = "0.0.1"
authors = ["Oleg <oleg@example.com>"]
[requires.packages]
"org.vibevm/wal" = "^0.3"
"org.vibevm/rust-cli" = "^0.1.0"
[active]
stack = "rust-cli"
[llm]
default_provider = "anthropic"
default_model = "claude-sonnet-4-7"
[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"
[[registry]]
name = "corporate"
url = "git@internal:packages"
naming = "name"
[[mirror]]
of = "vibespecs"
url = "https://mirror.internal/vibespecs"
priority = 1
[[override]]
pkgref = "org.vibevm/wal"
source_url = "git@mycompany:forks/wal"
ref = "my-fix"
reason = "pending upstream PR"
"#;
    let m = Manifest::parse_str(raw).unwrap();
    assert_eq!(m.requires.packages.len(), 2);
    assert_eq!(m.registries.len(), 2);
    assert_eq!(m.primary_registry().unwrap().name, "vibespecs");
    assert_eq!(
        m.registry_by_name("corporate").unwrap().url,
        "git@internal:packages"
    );
    assert_eq!(m.mirrors.len(), 1);
    assert_eq!(m.overrides.len(), 1);
    let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
    assert_eq!(m, back);
}

#[test]
fn package_manifest_parses() {
    let raw = r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.3.0"
license = "EULA"
description = "Write-Ahead Log discipline"
[compatibility]
min_vibe_version = "0.1.0"
[boot_snippet]
source = "boot/10-flow-wal.md"
category = "flow"
[provides]
capabilities = ["discipline:wal@0.3.0"]
[requires.packages]
"org.vibevm/atomic-commits" = "^0.1"
"#;
    let m = Manifest::parse_str(raw).unwrap();
    let pkg = m.require_package().unwrap();
    assert_eq!(pkg.name, "wal");
    assert_eq!(pkg.kind, PackageKind::Flow);
    assert!(pkg.publish.is_default());
    assert_eq!(
        m.boot_snippet.as_ref().unwrap().source.to_string_lossy(),
        "boot/10-flow-wal.md"
    );
    assert_eq!(m.provides.capabilities.len(), 1);
    assert_eq!(m.requires.packages.len(), 1);
    assert_eq!(m.as_package_ref().unwrap().name, "wal");
    let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
    assert_eq!(m, back);
}

#[test]
fn workspace_root_with_members_parses() {
    let raw = r#"[project]
name = "monorepo"
version = "0.0.1"
[workspace]
members = ["packages/flow-wal", "packages/feat-auth", "packages/stack-*"]
"#;
    let m = Manifest::parse_str(raw).unwrap();
    assert!(m.is_workspace_root());
    assert_eq!(m.workspace.as_ref().unwrap().members.len(), 3);
}

#[test]
fn workspace_versions_parse_and_round_trip() {
    let raw = r#"[project]
name = "mono"
version = "0.0.1"
[workspace]
members = ["packages/a"]
[workspace.versions]
core = "0.0.1"
ui = "^0.3"
"#;
    let m = Manifest::parse_str(raw).unwrap();
    let ws = m.workspace.as_ref().unwrap();
    assert_eq!(ws.versions.get("core").map(String::as_str), Some("0.0.1"));
    assert_eq!(ws.versions.get("ui").map(String::as_str), Some("^0.3"));
    let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
    assert_eq!(m, back);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#root-package",
    r = 1
)]
fn root_package_composes_workspace_and_package() {
    // cargo-style: the root crate is itself publishable. PROP-007 §2.9.
    let raw = r#"[package]
group = "org.vibevm"
name = "umbrella"
kind = "stack"
version = "0.1.0"
[workspace]
members = ["packages/core"]
"#;
    let m = Manifest::parse_str(raw).unwrap();
    assert!(m.is_package());
    assert!(m.is_workspace_root());
}

#[test]
fn virtual_workspace_root_parses() {
    // [workspace] alone — a pure coordinator, neither project nor package.
    let m = Manifest::parse_str("[workspace]\nmembers = [\"a\", \"b\"]\n").unwrap();
    assert!(m.is_workspace_root());
    assert!(!m.is_package());
    assert!(m.project.is_none());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#published-repos",
    r = 1
)]
fn origin_marker_parses() {
    let raw = r#"
[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.3.0"
[origin]
upstream = "https://github.com/you/monorepo"
path = "packages/flow-wal"
commit = "abc123"
generated_by = "vibe 0.1.0"
generated_at = "2026-05-20T00:00:00Z"
"#;
    let m = Manifest::parse_str(raw).unwrap();
    let o = m.origin.as_ref().unwrap();
    assert_eq!(o.path, "packages/flow-wal");
    assert_eq!(o.commit.as_deref(), Some("abc123"));
    let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
    assert_eq!(m, back);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest",
    r = 1
)]
fn rejects_project_and_package_together() {
    let raw = r#"
[project]
name = "demo"
version = "0.0.1"
[package]
group = "org.vibevm"
name = "demo"
kind = "flow"
version = "0.0.1"
"#;
    let err = Manifest::parse_str(raw).unwrap_err();
    assert!(err.to_string().contains("mutually exclusive"), "{err}");
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest",
    r = 1
)]
fn rejects_no_role_section() {
    let err = Manifest::parse_str("[active]\nstack = \"rust\"\n").unwrap_err();
    assert!(err.to_string().contains("declares no role"), "{err}");
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest",
    r = 1
)]
fn rejects_package_role_section_without_package() {
    let raw = r#"
[project]
name = "demo"
version = "0.0.1"
[boot_snippet]
source = "boot/x.md"
"#;
    let err = Manifest::parse_str(raw).unwrap_err();
    assert!(err.to_string().contains("[boot_snippet]"), "{err}");
    assert!(err.to_string().contains("without a [package]"), "{err}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-018#skill-decl", r = 3)]
fn package_declares_skills_and_roundtrips() {
    let raw = r#"
[package]
group = "org.vibevm"
name = "vim"
kind = "tool"
version = "0.1.0"
[[skill]]
name = "vim"
path = "skills/vim"
description = "Drive vim from an agent"
agents = ["claude", "opencode"]
[[skill]]
name = "vim-quickref"
path = "skills/vim-quickref/SKILL.md"
"#;
    let m = Manifest::parse_str(raw).unwrap();
    assert_eq!(m.skills.len(), 2);
    assert_eq!(m.skills[0].name, "vim");
    assert_eq!(m.skills[0].path.to_str(), Some("skills/vim"));
    assert_eq!(m.skills[0].agents, ["claude", "opencode"]);
    assert!(m.skills[1].description.is_none());
    assert!(m.skills[1].agents.is_empty());

    // A package-role section round-trips byte-stably through serde.
    let rendered = toml::to_string_pretty(&m).unwrap();
    let back = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(m, back);
}

const REFERENCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn reference_bridge_manifest(skill_tail: &str) -> String {
    format!(
        r#"
[package]
group = "org.vibevm.bridges"
name = "upstream-skill"
kind = "tool"
version = "1.0.0"
bridge = true
license = "UPL-1.0"

[[embedded_source]]
name = "upstream"
kind = "git"
url = "https://github.com/example/upstream.git"
commit = "{REFERENCE_COMMIT}"
content_hash = "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
ref_hint = "refs/tags/v1.2.3"
auth = "none"
upstream_license = "MIT"
license_path = "LICENSE"
license_url = "https://github.com/example/upstream/blob/{REFERENCE_COMMIT}/LICENSE"

{skill_tail}
"#
    )
}

#[test]
fn reference_bridge_sources_and_skill_inputs_round_trip() {
    let raw = reference_bridge_manifest(
        r#"
[[skill]]
name = "adapter"
path = "skills/adapter"
include = ["SKILL.md"]

[[skill.resource]]
embedded_source = "upstream"
path = "docs"
include = ["guide.md", "examples/**/*.md"]
target = "references/upstream"

[[skill]]
name = "direct-upstream"
source = "upstream"
path = "skills/direct-upstream"
include = ["SKILL.md", "references/**/*.md"]
"#,
    );
    let manifest = Manifest::parse_str(&raw).unwrap();
    assert_eq!(manifest.embedded_sources.len(), 1);
    assert_eq!(manifest.embedded_sources[0].name, "upstream");
    assert_eq!(manifest.skills.len(), 2);
    assert!(manifest.skills[0].source.is_none());
    assert_eq!(manifest.skills[0].resources.len(), 1);
    assert_eq!(manifest.skills[1].source.as_deref(), Some("upstream"));

    let rendered = toml::to_string_pretty(&manifest).unwrap();
    assert!(rendered.contains("[[embedded_source]]"), "{rendered}");
    assert!(rendered.contains("[[skill.resource]]"), "{rendered}");
    let back = Manifest::parse_str(&rendered).unwrap();
    assert_eq!(manifest, back);
}

#[test]
fn embedded_sources_are_package_only_and_unique() {
    let package_source = format!(
        r#"
[[embedded_source]]
name = "upstream"
kind = "git"
url = "https://github.com/example/upstream.git"
commit = "{REFERENCE_COMMIT}"
content_hash = "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
upstream_license = "MIT"
license_path = "LICENSE"
license_url = "https://github.com/example/upstream/blob/{REFERENCE_COMMIT}/LICENSE"
"#
    );
    let project = format!("[project]\nname = \"demo\"\nversion = \"1.0.0\"\n{package_source}");
    let error = Manifest::parse_str(&project).unwrap_err().to_string();
    assert!(error.contains("[[embedded_source]]"), "{error}");
    assert!(error.contains("without a [package]"), "{error}");

    let duplicate = reference_bridge_manifest(&package_source);
    let error = Manifest::parse_str(&duplicate).unwrap_err().to_string();
    assert!(
        error.contains("duplicate [[embedded_source]] name"),
        "{error}"
    );
}

#[test]
fn embedded_source_wire_rejects_mutable_or_credentialed_identity() {
    let valid = reference_bridge_manifest("");
    for (from, to, needle) in [
        (
            "https://github.com/example/upstream.git",
            "http://github.com/example/upstream.git",
            "credential-free HTTPS",
        ),
        (
            "https://github.com/example/upstream.git",
            "https://token@github.com/example/upstream.git",
            "credential-free HTTPS",
        ),
        (
            REFERENCE_COMMIT,
            "0123456789abcdef",
            "full 40- or 64-character",
        ),
        ("refs/tags/v1.2.3", "v1.2.3", "full safe `refs/...`"),
    ] {
        let error = Manifest::parse_str(&valid.replacen(from, to, 1))
            .unwrap_err()
            .to_string();
        assert!(error.contains(needle), "wanted {needle:?}: {error}");
    }

    let error = Manifest::parse_str(&valid.replace("kind = \"git\"", "kind = \"tar\""))
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant"), "{error}");
    let error = Manifest::parse_str(&valid.replace("auth = \"none\"", "auth = \"token\""))
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant"), "{error}");
}

#[test]
fn skill_external_inputs_require_declared_sources_and_safe_selection() {
    let undeclared = reference_bridge_manifest(
        r#"
[[skill]]
name = "direct"
source = "missing"
path = "skills/direct"
include = ["SKILL.md"]
"#,
    );
    let error = Manifest::parse_str(&undeclared).unwrap_err().to_string();
    assert!(
        error.contains("undeclared [[embedded_source]] `missing`"),
        "{error}"
    );

    let no_include = reference_bridge_manifest(
        r#"
[[skill]]
name = "direct"
source = "upstream"
path = "skills/direct"
"#,
    );
    let error = Manifest::parse_str(&no_include).unwrap_err().to_string();
    assert!(error.contains("non-empty include"), "{error}");

    let unsafe_target = reference_bridge_manifest(
        r#"
[[skill]]
name = "adapter"
path = "skills/adapter"

[[skill.resource]]
embedded_source = "upstream"
path = "docs"
include = ["**/*.md"]
target = "SKILL.md"
"#,
    );
    let error = Manifest::parse_str(&unsafe_target).unwrap_err().to_string();
    assert!(error.contains("below `references/`"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-018#skill-decl", r = 3)]
fn rejects_skill_section_without_package() {
    // `[[skill]]` is package-role: a package declares skills about its own
    // files, so a plain `[project]` carrying it is a role error.
    let raw = r#"
[project]
name = "demo"
version = "0.0.1"
[[skill]]
name = "vim"
path = "skills/vim"
"#;
    let err = Manifest::parse_str(raw).unwrap_err();
    assert!(err.to_string().contains("[[skill]]"), "{err}");
    assert!(err.to_string().contains("without a [package]"), "{err}");
}

#[test]
fn require_package_and_project_error_clearly() {
    let proj = Manifest::new_project("demo", "0.0.1");
    assert!(proj.require_package().is_err());
    assert!(proj.require_project().is_ok());
}

#[test]
fn conditional_deps_parse() {
    let raw = r#"
[package]
group = "org.vibevm"
name = "x"
kind = "flow"
version = "0.1.0"
[target."context(stack:rust)".dependencies]
packages = { "org.vibevm/rust-best-practices" = "^0.1" }
"#;
    let m = Manifest::parse_str(raw).unwrap();
    assert_eq!(m.conditional_deps.len(), 1);
    let t = m.conditional_deps.get("context(stack:rust)").unwrap();
    assert_eq!(t.dependencies.packages.len(), 1);
}

#[test]
fn rejects_unknown_top_level_section() {
    let raw = r#"
[project]
name = "demo"
version = "0.0.1"
[mystery]
value = 1
"#;
    assert!(toml::from_str::<Manifest>(raw).is_err());
}

#[test]
fn mirrors_for_filters_and_sorts() {
    let raw = r#"
[project]
name = "demo"
version = "0.1.0"
[[registry]]
name = "vibespecs"
url = "git@host:org"
[[mirror]]
of = "vibespecs"
url = "https://a"
priority = 2
[[mirror]]
of = "vibespecs"
url = "https://b"
priority = 1
[[mirror]]
of = "*"
url = "https://catchall"
priority = 99
"#;
    let m = Manifest::parse_str(raw).unwrap();
    let ms = m.mirrors_for("vibespecs");
    assert_eq!(ms.len(), 3);
    assert_eq!(ms[0].url, "https://b");
    assert_eq!(ms[1].url, "https://a");
    assert_eq!(ms[2].url, "https://catchall");
}

#[test]
fn write_and_read_roundtrip_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vibe.toml");
    let mut m = Manifest::new_project("disk-demo", "0.1.0");
    m.registries.push(RegistrySection {
        name: "vibespecs".into(),
        url: "https://github.com/vibespecs".into(),
        r#ref: "main".into(),
        naming: super::super::NamingConvention::KindName,
        auth: super::super::AuthKind::None,
        token_env: None,
        enabled: true,
        index_url: None,
    });
    m.write(&path).unwrap();
    let back = Manifest::read(&path).unwrap();
    assert_eq!(m, back);
}

#[test]
fn boot_section_parses_and_round_trips() {
    let raw = r#"
[project]
name = "demo"
version = "0.1.0"
[boot]
default_link = "dynamic"
"#;
    let m = Manifest::parse_str(raw).unwrap();
    assert_eq!(m.boot.default_link, Some(LinkType::Dynamic));
    let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
    assert_eq!(m, back);
}

#[test]
fn boot_section_absent_is_empty_and_not_emitted() {
    let m = Manifest::new_project("demo", "0.1.0");
    assert!(m.boot.is_empty());
    let rendered = toml::to_string_pretty(&m).unwrap();
    assert!(!rendered.contains("[boot]"), "{rendered}");
}

#[test]
fn boot_section_is_consumer_side_allowed_without_package() {
    // [boot] is not a package-role section — valid on a plain project.
    let raw = r#"
[project]
name = "demo"
version = "0.1.0"
[boot]
default_link = "static"
"#;
    Manifest::parse_str(raw).unwrap();
}

#[test]
fn boot_section_rejects_unknown_field() {
    let raw = r#"
[project]
name = "demo"
version = "0.1.0"
[boot]
mystery = "x"
"#;
    assert!(toml::from_str::<Manifest>(raw).is_err());
}

/// A minimal, law-abiding `mcp`-kind manifest: one server, its binary,
/// exact-pinned requirement (PROP-027; VIBEVM-SPEC §4.1).
fn mcp_manifest(requires_line: &str, server_tail: &str) -> String {
    format!(
        r#"
[package]
group = "org.vibevm"
name = "rust-ai-native-mcp"
kind = "mcp"
version = "0.6.0"
license = "EULA"
description = "the AI-Native Rust discipline over MCP"
[requires.packages]
{requires_line}
[[binary]]
name = "rust-ai-native-mcp"
crate = "crates/rust-ai-native-mcp"
[[mcp_server]]
name = "rust-ai-native"
binary = "rust-ai-native-mcp"
{server_tail}
"#
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest")]
fn mcp_kind_manifest_parses_under_its_laws() {
    let raw = mcp_manifest(
        "\"stack:org.vibevm/rust-ai-native-lang\" = \"=0.6.0\"",
        "args = [\"--path\", \"{project_root}\"]\n",
    );
    let m = Manifest::parse_str(&raw).unwrap();
    assert_eq!(m.require_package().unwrap().kind, PackageKind::Mcp);
    assert_eq!(m.mcp_servers.len(), 1);
    assert_eq!(m.mcp_servers[0].binary, "rust-ai-native-mcp");
    // Round-trips through serialisation.
    let back = Manifest::parse_str(&toml::to_string_pretty(&m).unwrap()).unwrap();
    assert_eq!(m, back);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest")]
fn mcp_server_table_is_refused_outside_the_mcp_kind() {
    let raw = r#"
[package]
group = "org.vibevm"
name = "rust-ai-native-lang"
kind = "stack"
version = "0.6.0"
license = "EULA"
description = "x"

[[binary]]
name = "rust-ai-native-mcp"
crate = "crates/rust-ai-native-mcp"

[[mcp_server]]
name = "rust-ai-native"
binary = "rust-ai-native-mcp"
"#;
    let err = Manifest::parse_str(raw).unwrap_err().to_string();
    assert!(err.contains("legal only in `mcp`-kind"), "{err}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest")]
fn mcp_kind_without_a_server_is_refused() {
    let raw = r#"
[package]
group = "org.vibevm"
name = "rust-ai-native-mcp"
kind = "mcp"
version = "0.6.0"
license = "EULA"
description = "x"
"#;
    let err = Manifest::parse_str(raw).unwrap_err().to_string();
    assert!(err.contains("at least one [[mcp_server]]"), "{err}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest")]
fn mcp_server_binary_must_resolve_and_names_must_be_unique() {
    // Unresolved binary reference.
    let raw = mcp_manifest("\"stack:org.vibevm/rust-ai-native-lang\" = \"=0.6.0\"", "")
        .replace("binary = \"rust-ai-native-mcp\"", "binary = \"ghost\"");
    let err = Manifest::parse_str(&raw).unwrap_err().to_string();
    assert!(err.contains("no [[binary]] declares it"), "{err}");

    // Duplicate server names.
    let raw = mcp_manifest(
        "\"stack:org.vibevm/rust-ai-native-lang\" = \"=0.6.0\"",
        "\n[[mcp_server]]\nname = \"rust-ai-native\"\nbinary = \"rust-ai-native-mcp\"\n",
    );
    let err = Manifest::parse_str(&raw).unwrap_err().to_string();
    assert!(err.contains("duplicate [[mcp_server]] name"), "{err}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest")]
fn mcp_server_args_substitute_only_the_closed_set() {
    let raw = mcp_manifest(
        "\"stack:org.vibevm/rust-ai-native-lang\" = \"=0.6.0\"",
        "args = [\"--token\", \"{secret}\"]\n",
    );
    let err = Manifest::parse_str(&raw).unwrap_err().to_string();
    assert!(err.contains("unknown substitution variable"), "{err}");
    assert!(err.contains("{secret}"), "{err}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#exact-pin")]
fn mcp_kind_requires_exact_pins() {
    for bad in ["\"^0.6\"", "\"0.6.0\"", "\"=0.6\"", "\">=0.6.0, <0.7\""] {
        let raw = mcp_manifest(
            &format!("\"stack:org.vibevm/rust-ai-native-lang\" = {bad}"),
            "",
        );
        let err = Manifest::parse_str(&raw).unwrap_err().to_string();
        assert!(
            err.contains("pin every package requirement exactly"),
            "spec {bad} must be refused: {err}"
        );
    }
    // The exact form passes.
    let raw = mcp_manifest("\"stack:org.vibevm/rust-ai-native-lang\" = \"=0.6.0\"", "");
    Manifest::parse_str(&raw).unwrap();
}
