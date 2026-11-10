//! The real `org.vibevm.node/node` template, driven through the real
//! binary against frozen upstream documents — campaign phase P3
//! (`AV-FMT-006`), PROP-075 `#CHANNELS` / `#TEMPLATE-OPTION-GATING`.
//!
//! Everything here is the production package: the template manifest is
//! byte-identical to the one `/fast/git/vibe-pkg-node` carries at
//! revision `1.0.0`, the version list is a frozen subset of
//! `https://nodejs.org/dist/index.json`, and the digest documents are
//! upstream's own `SHASUMS256.txt`. The only test-shaped thing is one
//! `[[remap]]` row that reads `https://nodejs.org/dist/` off this
//! filesystem instead — the mechanism a closed network uses anyway
//! (PROP-066 `#locator`), so **no run here opens a socket**.
//!
//! `crates/vibe-template/tests/fixtures/node/README.md` says what each
//! fixture is, where it came from, and how the subset was selected. The
//! byte-for-byte regression gate against the retired generator's output
//! is the other suite over the same fixtures,
//! `cargo test -p vibe-template --test node_golden`.

mod common;

use std::fs;
use std::path::Path;

use common::app_registry::{AppRegistry, failed, json, ok};

/// The template of the real package, at its real revision.
const TEMPLATE: &str = include_str!("../../vibe-template/tests/fixtures/node/vibe.toml");
/// The template's own revision — a revision is never a produced version.
const REVISION: &str = "1.0.0";
/// The frozen version list, newest first.
const INDEX: &str = include_str!("../../vibe-template/tests/fixtures/node/index-excerpt.json");

/// The frozen digest documents, by produced version.
const DIGESTS: [(&str, &str); 4] = [
    (
        "26.13.0",
        include_str!("../../vibe-template/tests/fixtures/node/SHASUMS256-26.13.0.txt"),
    ),
    (
        "24.22.2",
        include_str!("../../vibe-template/tests/fixtures/node/SHASUMS256-24.22.2.txt"),
    ),
    (
        "22.23.4",
        include_str!("../../vibe-template/tests/fixtures/node/SHASUMS256-22.23.4.txt"),
    ),
    (
        "12.0.0",
        include_str!("../../vibe-template/tests/fixtures/node/SHASUMS256-12.0.0.txt"),
    ),
];

/// The committed manifest of the newest concrete tag — the published
/// version `#TEMPLATE-CONCRETE-WINS` is exercised with.
const PUBLISHED_24: &str =
    include_str!("../../vibe-template/tests/fixtures/node/vibe-24.22.2.toml");

/// What the package declares, and what the one remap row relocates.
const ORIGIN: &str = "https://nodejs.org/dist/";

/// The registry with the template at its revision, the frozen documents
/// laid out as upstream lays them out, and the remap row that reads them
/// off disk.
fn fixture() -> AppRegistry {
    let fixture = AppRegistry::new();
    let root = fixture.root.path();
    let dir = fixture
        .registry()
        .join("org.vibevm.node/node")
        .join(format!("v{REVISION}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("vibe.toml"), TEMPLATE).unwrap();
    // A document beside the manifest, so materialisation has a tree to
    // carry along exactly as the real package's README is carried.
    fs::write(dir.join("README.md"), "# Node.js, from a template\n").unwrap();

    // `https://nodejs.org/dist/index.json` and
    // `https://nodejs.org/dist/v<V>/SHASUMS256.txt`, mirrored by path:
    // one `prefix` row joins the remainder onto the directory.
    let mirror = root.join("dist");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("index.json"), INDEX).unwrap();
    for (version, document) in DIGESTS {
        let release = mirror.join(format!("v{version}"));
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join("SHASUMS256.txt"), document).unwrap();
    }
    fs::write(
        root.join("remap.toml"),
        format!(
            "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
             reason = \"the suite's frozen copy of the Node.js release host\"\n",
            mirror.display()
        ),
    )
    .unwrap();
    fixture
}

/// Publish the newest concrete tag as a concrete version of the same
/// package, the way the repository publishes it today.
fn publish_concrete_24(fixture: &AppRegistry) {
    let dir = fixture.registry().join("org.vibevm.node/node/v24.22.2");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("vibe.toml"), PUBLISHED_24).unwrap();
}

/// `vibe install -g <spelling> --registry … --remap … --plan-only --json`:
/// the plan resolves and renders, and downloads nothing.
fn plan(fixture: &AppRegistry, spelling: &str, extra: &[&str]) -> std::process::Output {
    let remap = fixture.root.path().join("remap.toml");
    let mut args: Vec<&str> = vec!["--remap", remap.to_str().unwrap(), "--plan-only", "--json"];
    args.extend_from_slice(extra);
    fixture.app(&["install", "-g"], spelling, &args)
}

/// The template cache of the Node package under this fixture's settings.
fn cache(fixture: &AppRegistry) -> std::path::PathBuf {
    fixture
        .settings()
        .join("cache/templates/org.vibevm.node/node")
}

/// The manifest the layer rendered for one produced version.
fn rendered(fixture: &AppRegistry, version: &str) -> String {
    let path = cache(fixture).join(format!("rendered/v{version}/vibe.toml"));
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The `[[installer_options]]` ids of a rendered manifest, in order.
fn option_ids(manifest: &str) -> Vec<String> {
    vibe_core::manifest::Manifest::parse_str(manifest)
        .expect("a rendered manifest is an ordinary manifest")
        .installer_options
        .iter()
        .map(|option| option.id.clone())
        .collect()
}

/// Every produced version this run rendered.
fn rendered_versions(fixture: &AppRegistry) -> Vec<String> {
    let dir = cache(fixture).join("rendered");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix('v'))
                .map(str::to_string)
        })
        .collect();
    found.sort();
    found
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNELS")]
fn lts_is_the_newest_release_whose_lts_field_is_truthy() {
    let fixture = fixture();
    let planned = json(&plan(&fixture, "org.vibevm.node/node@lts", &[])).to_string();
    // The frozen document's newest truthy `lts` record is v26.13.0
    // (Lithium); 26.13.1 does not exist and 27.0.0-alpha is a prerelease
    // the template filters, so this is upstream's own answer.
    assert!(planned.contains("26.13.0"), "{planned}");
    assert!(
        !planned.contains(REVISION),
        "the template revision is not a produced version: {planned}"
    );
    // One channel, one render — and the render carries upstream's real
    // digests, not the skeleton's zeros.
    assert_eq!(rendered_versions(&fixture), vec!["26.13.0".to_string()]);
    let manifest = rendered(&fixture, "26.13.0");
    assert!(
        manifest.contains("5848553868b402d3fabb5b09270649977780a390f832155721025a08b5104455"),
        "the linux-x64 digest of v26.13.0 is the frozen document's: {manifest}"
    );
    assert!(
        manifest.contains("version = \"26.13.0\"") && !manifest.contains("template"),
        "no template vocabulary survives a render: {manifest}"
    );
    // v26.13.0 is one of the releases upstream built for all seven
    // targets, so nothing was dropped.
    assert_eq!(option_ids(&manifest).len(), 7);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn lts_jod_is_the_newest_release_of_that_codename() {
    let fixture = fixture();
    let planned = json(&plan(&fixture, "org.vibevm.node/node@lts/jod", &[])).to_string();
    assert!(planned.contains("22.23.4"), "{planned}");
    assert_eq!(rendered_versions(&fixture), vec!["22.23.4".to_string()]);
    // A codename upstream really used, but below the template's `min`, is
    // no channel of this package — refused by name rather than silently
    // widened.
    let refusal = failed(&plan(&fixture, "org.vibevm.node/node@lts/argon", &[]));
    assert!(refusal.contains("argon"), "{refusal}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-OPTION-GATING")]
fn the_oldest_release_in_the_min_window_renders_without_the_targets_it_never_had() {
    let fixture = fixture();
    // v12.0.0 is the oldest version the template's `min` admits. Upstream
    // built neither a musl, nor an Apple-silicon, nor a Windows-ARM
    // archive for it, and `template_optional` is what makes that a
    // narrower manifest instead of three archives that do not exist.
    ok(&plan(&fixture, "org.vibevm.node/node@=12.0.0", &[]));
    let manifest = rendered(&fixture, "12.0.0");
    assert_eq!(
        option_ids(&manifest),
        vec!["linux-x64", "linux-arm64", "macos-x64", "windows-x64"]
    );
    assert!(
        manifest.contains("3268466af05464a7aa698df7cc8bbeec0b8728d8779e12130ef441bb730a8059"),
        "the surviving rows carry upstream's real digests: {manifest}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-OPTION-GATING")]
fn a_target_an_old_release_lacks_cannot_be_selected_and_a_newer_one_can() {
    let fixture = fixture();
    // The two rows `PLAN.md` names for phase P3, each checked in both
    // directions: absent for v12.0.0, present for v24.22.2.
    for (platform, arch) in [("linux-musl", "x86_64"), ("windows", "aarch64")] {
        let refusal = failed(&plan(
            &fixture,
            "org.vibevm.node/node@=12.0.0",
            &["--platform", platform, "--arch", arch],
        ));
        // Refused as a missing installer option, naming the target —
        // not as a failed download of an archive that never existed.
        assert!(
            refusal.contains("\"code\":\"no_option\"") && refusal.contains(platform),
            "{platform}/{arch} for v12.0.0: {refusal}"
        );
        ok(&plan(
            &fixture,
            "org.vibevm.node/node@=24.22.2",
            &["--platform", platform, "--arch", arch],
        ));
    }
    // The newer release kept all seven rows; the older one lost three.
    assert_eq!(option_ids(&rendered(&fixture, "24.22.2")).len(), 7);
    assert_eq!(option_ids(&rendered(&fixture, "12.0.0")).len(), 4);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CONCRETE-WINS")]
fn the_published_tag_wins_over_the_template_rendering_of_the_same_version() {
    let fixture = fixture();
    publish_concrete_24(&fixture);
    // The shape the repository really has from the first day of P3: one
    // template revision plus the two concrete tags it also publishes.
    let planned = json(&plan(&fixture, "org.vibevm.node/node@24.22.2", &[])).to_string();
    assert!(planned.contains("24.22.2"), "{planned}");
    assert!(
        rendered_versions(&fixture).is_empty(),
        "the template must not be rendered for a version published concretely"
    );
    assert!(
        !cache(&fixture).join("digests/24.22.2").exists(),
        "hand-fixing one version must be free of any digest fetch"
    );
    // And the version only the template produces still resolves beside
    // it, from the same registry and the same run of the same binary.
    ok(&plan(&fixture, "org.vibevm.node/node@22.23.4", &[]));
    assert_eq!(rendered_versions(&fixture), vec!["22.23.4".to_string()]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE")]
fn the_frozen_documents_are_read_once_and_the_second_run_is_offline() {
    let fixture = fixture();
    ok(&plan(&fixture, "org.vibevm.node/node@22.23.4", &[]));
    let versions = cache(&fixture).join("versions.json");
    assert!(versions.is_file(), "the version list is cached verbatim");
    assert!(cache(&fixture).join("digests/22.23.4").is_file());
    // With the mirror taken away entirely, `--offline` still resolves the
    // same version out of the cache — which is what a closed network, a
    // flight and a CI runner all need.
    fs::remove_dir_all(fixture.root.path().join("dist")).unwrap();
    let planned = json(&plan(
        &fixture,
        "org.vibevm.node/node@22.23.4",
        &["--offline"],
    ))
    .to_string();
    assert!(planned.contains("22.23.4"), "{planned}");
}

/// The fixture's own invariant: the template this suite drives is the one
/// `vibe-template`'s golden suite proves against the retired generator's
/// output, byte for byte. If the two ever diverge, this suite is no
/// longer testing the production package.
#[test]
fn the_template_under_test_is_the_one_the_golden_suite_proves() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vibe-template/tests/fixtures/node/vibe.toml");
    assert_eq!(
        fs::read_to_string(golden).expect("the golden fixture"),
        TEMPLATE
    );
    assert!(TEMPLATE.contains("template = true"));
    assert!(TEMPLATE.contains(&format!("version = \"{REVISION}\"")));
}
