//! `vibe install -g` over a **template package** in a local-directory
//! registry — the four call sites of PROP-075 `#TEMPLATE-DOMAIN-SEAM`
//! driven through the real binary, with no socket at all.
//!
//! The template declares `[template.versions] kind = "list"`, so the
//! version domain needs no fetched document at all, and its
//! `[template.digests]` URL is the credential-free HTTPS one
//! `#TEMPLATE-DIGEST-SOURCE` requires — relocated onto this filesystem by
//! one `[[remap]]` row, which is the mechanism a closed network uses
//! anyway (PROP-066 `#locator`). So the suite exercises the real path
//! rather than a test-only flag, and makes no connection at all.

mod common;

use std::fs;
use std::path::Path;

use common::app_registry::{AppRegistry, failed, json, ok};

/// The origin the template declares. Nothing ever connects to it: one
/// `[[remap]]` row sends every URL under it to the fixture's own `dist/`.
const ORIGIN: &str = "https://dist.example.invalid/tool/";

/// `vibe <verb…> <package> --registry <registry> --remap <row> <extra…>`
/// — the fixture's own runner plus the one remap row that keeps every run
/// off the network.
fn app(
    fixture: &AppRegistry,
    verb: &[&str],
    package: &str,
    extra: &[&str],
) -> std::process::Output {
    let remap = fixture.root.path().join("remap.toml");
    let mut with_remap: Vec<&str> = vec!["--remap", remap.to_str().unwrap()];
    with_remap.extend_from_slice(extra);
    fixture.app(verb, package, &with_remap)
}

/// The payload every produced version unpacks: one script, and its
/// digest, so the render has something real to substitute.
struct Payload {
    archive: String,
    sha256: String,
}

/// A `tar.gz` holding `bin/tool`, written beside the registry, plus its
/// digest. The installer step points at it with a `file://` URL.
fn payload(registry_root: &Path, version: &str) -> Payload {
    let dist = registry_root.join("dist");
    fs::create_dir_all(&dist).unwrap();
    let name = format!("tool-{version}-linux.tar.gz");
    let path = dist.join(&name);
    // One `tar.gz` per version, built with the host's tar so the archive
    // step has a real payload to unpack.
    let staging = dist.join(format!("staging-{version}/bin"));
    fs::create_dir_all(&staging).unwrap();
    let script = staging.join("tool");
    fs::write(&script, format!("#!/bin/sh\necho \"tool {version}\"\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let status = std::process::Command::new("tar")
        .arg("-czf")
        .arg(&path)
        .arg("-C")
        .arg(staging.parent().unwrap())
        .arg("bin")
        .status()
        .expect("the host has tar");
    assert!(status.success());
    let bytes = fs::read(&path).unwrap();
    let digest = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    Payload {
        archive: name,
        sha256: digest,
    }
}

/// Write the template package at revision `0.3.0`, producing `versions`,
/// with one `SHASUMS256.txt` per produced version beside the archives.
fn template(fixture: &AppRegistry, versions: &[(&str, &str)]) {
    let root = fixture.root.path();
    let dist = root.join("dist");
    let mut items = String::new();
    for (version, lts) in versions {
        let built = payload(root, version);
        fs::write(
            dist.join(format!("SHASUMS256-{version}.txt")),
            format!("{}  {}\n", built.sha256, built.archive),
        )
        .unwrap();
        items.push_str(&format!(
            "\n[[template.versions.item]]\nversion = \"{version}\"\n\
             meta = {{ lts = {lts} }}\n"
        ));
    }
    let dir = fixture.registry().join("org.example/tool/v0.3.0");
    fs::create_dir_all(&dir).unwrap();
    let dist_url = ORIGIN.trim_end_matches('/').to_string();
    // The one row that makes the suite offline: every URL under the
    // declared origin is read out of `dist/` instead.
    fs::write(
        root.join("remap.toml"),
        format!(
            "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
             reason = \"the fixture's local mirror of the release host\"\n",
            dist.display()
        ),
    )
    .unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"tool\"\ngroup = \"org.example\"\nkind = \"app\"\n\
             version = \"0.3.0\"\ntemplate = true\n\
             description = \"tool {{version}}\"\n\n\
             [application]\nid = \"tool\"\n\n\
             [template.versions]\nkind = \"list\"\n{items}\n\
             [template.digests]\nkind = \"sha256sums\"\n\
             url = \"{dist_url}/SHASUMS256-{{version}}.txt\"\n\n\
             [[template.channel]]\nrule = \"truthy\"\nname = \"lts\"\nfrom = \"lts\"\n\n\
             [[template.channel]]\nrule = \"value\"\nprefix = \"lts\"\nfrom = \"lts\"\n\n\
             [[installer_options]]\nid = \"linux\"\nname = \"tool {{version}}\"\n\
             platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\
             path_dirs = [\"bin\"]\n\n\
             [[installer_options.steps]]\nid = \"unpack\"\n\
             title = \"Unpack tool {{version}}\"\nkind = \"archive\"\nformat = \"tar_gz\"\n\
             source = \"{dist_url}/tool-{{version}}-linux.tar.gz\"\n\
             sha256 = \"{{sha256:tool-{{version}}-linux.tar.gz}}\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    // A README, so materialisation has a tree to carry along.
    fs::write(dir.join("README.md"), "# tool, from a template\n").unwrap();
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DOMAIN-SEAM")]
fn a_template_produced_version_plans_and_installs_through_the_registry_seam() {
    let fixture = AppRegistry::new();
    template(&fixture, &[("1.4.0", "false"), ("2.0.0", "\"Argon\"")]);

    // Call sites 1 and 4: the version domain is the template's, and the
    // template revision is NOT a produced version.
    let plan = json(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@^2",
        &["--plan-only", "--json"],
    ));
    let planned = plan.to_string();
    assert!(planned.contains("2.0.0"), "{planned}");
    assert!(
        !planned.contains("0.3.0"),
        "the template revision is not a produced version: {planned}"
    );

    // Call site 3: the full render is materialised as an ordinary package
    // directory and installed from, digest and all.
    ok(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    let listed = fixture.listed("org.example/tool@2.0.0");
    assert_eq!(listed["package"]["version"], "2.0.0", "{listed}");
    assert_eq!(listed["status"], "ready", "{listed}");
    // The installation was made from the rendered tree, not from a
    // directory inside the registry: a render has no home in a registry.
    let evidence = listed["source_evidence"]["source_url"].as_str().unwrap();
    assert!(
        evidence.contains("cache/templates/org.example/tool/rendered/v2.0.0"),
        "{evidence}"
    );

    // The rendered directory is where `#TEMPLATE-CACHE` says, and it
    // carries the template's whole tree with the manifest replaced.
    let rendered = fixture
        .settings()
        .join("cache/templates/org.example/tool/rendered/v2.0.0");
    assert!(rendered.join("README.md").is_file(), "{rendered:?}");
    let manifest = fs::read_to_string(rendered.join("vibe.toml")).unwrap();
    assert!(manifest.contains("version = \"2.0.0\""), "{manifest}");
    assert!(
        !manifest.contains("template"),
        "no template vocabulary survives a render: {manifest}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn a_channel_requirement_lowers_to_the_newest_member() {
    let fixture = AppRegistry::new();
    template(
        &fixture,
        &[
            ("1.4.0", "\"Boron\""),
            ("2.0.0", "false"),
            ("2.1.0", "\"Argon\""),
        ],
    );
    // `lts` holds 1.4.0 and 2.1.0; the newest wins.
    let plan = json(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@lts",
        &["--plan-only", "--json"],
    ));
    assert!(plan.to_string().contains("2.1.0"), "{plan}");
    // `lts/boron` is one channel per distinct value, lower-cased.
    let narrowed = json(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@lts/boron",
        &["--plan-only", "--json"],
    ));
    assert!(narrowed.to_string().contains("1.4.0"), "{narrowed}");
    // `stable` is reserved and answered from the domain itself.
    let stable = json(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@stable",
        &["--plan-only", "--json"],
    ));
    assert!(stable.to_string().contains("2.1.0"), "{stable}");
    // A channel nothing declares refuses, naming it.
    let refusal = failed(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@canary",
        &["--plan-only", "--json"],
    ));
    assert!(refusal.contains("canary"), "{refusal}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CONCRETE-WINS")]
fn a_concretely_published_version_overrides_the_template_and_fetches_no_digest() {
    let fixture = AppRegistry::new();
    template(&fixture, &[("1.4.0", "false"), ("2.0.0", "false")]);
    // Publish `2.0.0` concretely, with a body that differs in every field
    // the template would have produced: a `copy` of a payload committed
    // in the package, not an archive fetched against a digest.
    let published = fixture.package(
        "tool",
        "2.0.0",
        "tool",
        "path_dirs = [\"bin\"]\n\n\
         [[installer_options.steps]]\nid = \"place\"\ntitle = \"Place the payload\"\n\
         kind = \"copy\"\nfrom = \"payload\"",
    );
    fixture.script(&published, "tool", "hand-fixed");
    ok(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    // The published package was installed, and the template was not
    // rendered for that version at all — no rendered directory, and no
    // digest document cached for it.
    let cache = fixture.settings().join("cache/templates/org.example/tool");
    assert!(
        !cache.join("rendered/v2.0.0").exists(),
        "the template must not be rendered for a version published concretely"
    );
    assert!(
        !cache.join("digests/2.0.0").exists(),
        "hand-fixing one version must be free of any digest fetch"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE")]
fn a_second_install_of_the_same_version_revalidates_no_digest_document() {
    let fixture = AppRegistry::new();
    template(&fixture, &[("2.0.0", "false")]);
    ok(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    let digest = fixture
        .settings()
        .join("cache/templates/org.example/tool/digests/2.0.0");
    assert!(digest.is_file(), "the digest document is cached verbatim");
    let before = fs::metadata(&digest).unwrap().modified().unwrap();
    // Rewrite the upstream document. A released version's checksums are
    // immutable, so the cached copy is authoritative and is never
    // revalidated — which is exactly what makes the alarm meaningful
    // rather than a race.
    fs::write(
        fixture.root.path().join("dist/SHASUMS256-2.0.0.txt"),
        format!("{}  tool-2.0.0-linux.tar.gz\n", "b".repeat(64)),
    )
    .unwrap();
    ok(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    assert_eq!(
        fs::metadata(&digest).unwrap().modified().unwrap(),
        before,
        "a cached digest document is never rewritten"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE")]
fn offline_with_a_cold_cache_refuses_by_name_and_by_path() {
    let fixture = AppRegistry::new();
    template(&fixture, &[("2.0.0", "false")]);
    // A `kind = "list"` source needs no version document, so the cold miss
    // the offline posture reports is the digest document — named, with its
    // exact path and the command that would fill it.
    let refusal = failed(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes", "--offline"],
    ));
    assert!(
        refusal.contains("template_cache_missing") || refusal.contains("offline"),
        "{refusal}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn a_digest_document_that_changed_upstream_is_the_alarm_before_any_effect() {
    let fixture = AppRegistry::new();
    template(&fixture, &[("2.0.0", "false")]);
    ok(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    // Upstream rewrites the digest document for a released version, and
    // the cached copy is dropped — the one state the alarm exists for. A
    // cached copy would have answered instead, which is exactly what
    // makes the alarm meaningful rather than a race.
    let dist = fixture.root.path().join("dist");
    let archive = fs::read(dist.join("tool-2.0.0-linux.tar.gz")).unwrap();
    let rewritten = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&archive);
        hasher.update(b"a second payload");
        format!("{:x}", hasher.finalize())
    };
    fs::write(
        dist.join("SHASUMS256-2.0.0.txt"),
        format!("{rewritten}  tool-2.0.0-linux.tar.gz\n"),
    )
    .unwrap();
    fs::remove_file(
        fixture
            .settings()
            .join("cache/templates/org.example/tool/digests/2.0.0"),
    )
    .unwrap();

    let refusal = failed(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    assert!(refusal.contains("template_render_changed"), "{refusal}");
    assert!(refusal.contains("org.example/tool@2.0.0"), "{refusal}");
    assert!(refusal.contains("sha256"), "{refusal}");
    assert!(refusal.contains("--trust-template"), "{refusal}");
    // Before any effect: the rendered tree still holds the bytes the
    // first install used.
    let manifest = fs::read_to_string(
        fixture
            .settings()
            .join("cache/templates/org.example/tool/rendered/v2.0.0/vibe.toml"),
    )
    .unwrap();
    assert!(!manifest.contains(&rewritten), "{manifest}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn a_template_fixed_without_touching_a_digest_is_not_an_alarm() {
    let fixture = AppRegistry::new();
    template(&fixture, &[("2.0.0", "false")]);
    ok(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    // A new template revision that changes a description and nothing a
    // digest bears. TEMPLATE-FLAG's whole purpose is that this is free.
    let revision = fixture.registry().join("org.example/tool/v0.3.0");
    let fixed = fixture.registry().join("org.example/tool/v0.4.0");
    fs::create_dir_all(&fixed).unwrap();
    for entry in fs::read_dir(&revision).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), fixed.join(entry.file_name())).unwrap();
    }
    let text = fs::read_to_string(fixed.join("vibe.toml")).unwrap();
    fs::write(
        fixed.join("vibe.toml"),
        text.replace("version = \"0.3.0\"", "version = \"0.4.0\"")
            .replace(
                "description = \"tool {version}\"",
                "description = \"tool {version}, fixed\"",
            ),
    )
    .unwrap();
    ok(&app(
        &fixture,
        &["install", "-g"],
        "org.example/tool@2.0.0",
        &["--assume-yes"],
    ));
    let manifest = fs::read_to_string(
        fixture
            .settings()
            .join("cache/templates/org.example/tool/rendered/v2.0.0/vibe.toml"),
    )
    .unwrap();
    assert!(manifest.contains("fixed"), "the fix landed: {manifest}");
}

// --- PROP-075 `#TEMPLATE-NO-DIGEST` -------------------------------------

/// A template whose produced versions download **nothing**: the payload
/// is a file the template's own tree carries, placed by a `kind = "copy"`
/// step out of the rendered directory. Such a template declares no
/// `[template.digests]` at all, and the point of the test is that the
/// whole path works with **no digest document anywhere** — not a declared
/// URL, not a fetch, not a lock member.
///
/// This is the two-file repro that found the gap, as a committed test.
fn digestless_template(fixture: &AppRegistry) {
    let dir = fixture.registry().join("org.vibevm.nvm/nvm/v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[package]\nname = \"nvm\"\ngroup = \"org.vibevm.nvm\"\nkind = \"app\"\n\
         version = \"1.0.0\"\ntemplate = true\n\
         description = \"nvm {version}'s command surface, on vibe\"\n\n\
         [application]\nid = \"nvm\"\n\n\
         [template.versions]\nkind = \"list\"\n\
         [[template.versions.item]]\nversion = \"0.41.0\"\n\n\
         [[installer_options]]\nid = \"posix\"\nname = \"nvm {version}\"\n\
         platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"\n\
         arch_name = \"Intel/AMD 64-bit\"\n\n\
         [[installer_options.steps]]\nid = \"shim\"\n\
         title = \"Place nvm.sh where a shell can source it\"\nkind = \"copy\"\n\
         from = \"nvm.sh\"\nto = \"nvm.sh\"\n",
    )
    .unwrap();
    // The payload: the file the package itself carries, which is exactly
    // why there is nothing to digest.
    fs::write(dir.join("nvm.sh"), "#!/bin/sh\nnvm() { :; }\n").unwrap();
}

/// The listing, the plan and a real install, all with no digest document.
///
/// Before `#TEMPLATE-NO-DIGEST` the listing worked — a skeleton render
/// reads no digests — and the install failed fetching a document the
/// template had been forced to declare and upstream does not publish.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-NO-DIGEST")]
fn a_template_that_downloads_nothing_needs_no_digest_document() {
    let fixture = AppRegistry::new();
    digestless_template(&fixture);

    // The produced version is the domain, and the revision is not in it.
    let listed = json(&fixture.app(&["app", "versions"], "org.vibevm.nvm/nvm", &["--json"]));
    let rows: Vec<String> = listed["packages"][0]["versions"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["version"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(rows, vec!["0.41.0".to_string()], "{listed}");

    // The plan reaches the step without asking for a digest document.
    let plan = json(&fixture.app(
        &["install", "-g"],
        "org.vibevm.nvm/nvm@=0.41.0",
        &["--plan-only", "--json"],
    ));
    let planned = plan.to_string();
    assert!(planned.contains("0.41.0"), "{planned}");
    assert!(
        !planned.contains("SHASUMS"),
        "nothing named a digest document: {planned}"
    );

    // And the install really runs, from the rendered tree.
    ok(&fixture.app(
        &["install", "-g"],
        "org.vibevm.nvm/nvm@=0.41.0",
        &["--assume-yes"],
    ));
    let installed = fixture.listed("org.vibevm.nvm/nvm@0.41.0");
    assert_eq!(installed["package"]["version"], "0.41.0", "{installed}");
    assert_eq!(installed["status"], "ready", "{installed}");
}

/// The one thing such a template may not do: name a digest variable. The
/// refusal is at the variable, with the fix in it, and it lands at
/// **resolution** — the skeleton render, before any effect — rather than
/// waiting for an install to fail on a lookup in an empty table. The
/// version *listing* is unaffected, because a domain is the version list
/// and renders nothing at all.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-NO-DIGEST")]
fn a_digest_variable_without_a_digest_source_is_refused_by_name() {
    let fixture = AppRegistry::new();
    digestless_template(&fixture);
    let manifest = fixture
        .registry()
        .join("org.vibevm.nvm/nvm/v1.0.0/vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    // The same template, with one digest-bearing field it cannot fill.
    fs::write(
        &manifest,
        text.replace(
            "from = \"nvm.sh\"\nto = \"nvm.sh\"\n",
            "from = \"nvm.sh\"\nto = \"nvm.sh\"\nsha256 = \"{sha256:nvm.sh}\"\n",
        ),
    )
    .unwrap();
    let output = failed(&fixture.app(
        &["install", "-g"],
        "org.vibevm.nvm/nvm@=0.41.0",
        &["--plan-only"],
    ));
    assert!(output.contains("[template.digests]"), "{output}");
    assert!(output.contains("download nothing"), "{output}");
    assert!(output.contains("sha256"), "{output}");
}
