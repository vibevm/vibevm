//! Fixture workspaces for the parity test, built on a real temp filesystem
//! and regenerated through the real publication path.
//!
//! `PLAN.md` P7 names a `tests/fixtures/system_prompt/` tree of checked-in
//! workspaces. These are built programmatically instead, which is the
//! established shape in this crate (`install/tests_target_gate.rs`,
//! `tests/dynamic_lane.rs`): a fixture needs a `ResolvedDep` whose
//! `content_dir` is an absolute path, so a checked-in tree would have to be
//! copied into a temp dir and have its resolution written in code anyway —
//! the checked-in half would carry only the file bodies, and splitting one
//! fixture across two places is how the two halves drift.
//!
//! Every fixture opts in the way a user does — by writing
//! `SYSTEM-TEMPLATE.md` — and nothing here touches `[system_prompt]` unless
//! the case under test is about a declared value.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_workspace::Workspace;
use vibe_workspace::install::{ResolvedDep, apply_resolution_with_spec_format};

/// The template every opted-in fixture writes. It ends with a newline, so a
/// segment body is the input's bytes exactly and the LF policy is tested
/// where it belongs (`compose`'s own unit tests) rather than here.
pub const TEMPLATE: &str = "# My frame\n\nPermanent project instructions.\n";

/// The authored prose a fixture entrypoint carries outside the managed
/// block, with a canary the parity assertions look for.
pub const ENTRYPOINT_PROSE: &str = "# demo\n\nAuthored entrypoint prose: ENTRYPOINT-CANARY-9f21.\n";

/// One regenerated fixture workspace. The temp dirs are fields so the caller
/// keeps the trees alive for the length of the test.
pub struct Fixture {
    /// The workspace root.
    pub ws: TempDir,
    /// The package content trees the resolution points at. Held, not read:
    /// a `ResolvedDep`'s `content_dir` is an absolute path into one of these
    /// temp dirs, so dropping them would delete the trees under the test.
    #[allow(dead_code)]
    pub packages: Vec<TempDir>,
    /// The resolution that was applied.
    pub resolution: Vec<ResolvedDep>,
    /// The spec format the regeneration ran under.
    pub format: SpecFormat,
}

impl Fixture {
    /// Regenerate again, with the same resolution and format.
    pub fn regenerate(&self) {
        let workspace = Workspace::load(self.ws.path()).expect("the fixture workspace loads");
        apply_resolution_with_spec_format(
            &workspace,
            &self.resolution,
            SlotIntegrity::TrustPresence,
            self.format,
            None,
            None,
        )
        .expect("the fixture regenerates");
    }

    /// One path under the fixture's boot directory.
    pub fn boot(&self, name: &str) -> PathBuf {
        self.ws
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join(name)
    }

    /// The bytes of one generated artifact, which must be there.
    pub fn artifact(&self, name: &str) -> Vec<u8> {
        fs::read(self.boot(name)).unwrap_or_else(|error| panic!("the artifact `{name}`: {error}"))
    }

    /// The sidecar, parsed as the plain JSON document a consumer reads.
    pub fn sidecar(&self) -> serde_json::Value {
        serde_json::from_slice(&self.artifact(vibe_core::layout::SYSTEM_META_JSON))
            .expect("the sidecar is JSON")
    }
}

/// Write one file under `dir`, creating its parents.
pub fn write(dir: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().expect("a parented path")).expect("the fixture directory");
    fs::write(path, body).expect("the fixture file");
}

/// One file under the live boot directory, as a relative path.
pub fn boot_rel(tail: &str) -> PathBuf {
    vibe_core::layout::current_boot_dir().join(tail)
}

/// A resolved dependency whose content tree is a fresh temp dir carrying one
/// boot source and the `[boot_snippet]` declaration the caller writes.
pub fn dep(name: &str, snippet_toml: &str, extra_files: &[(&str, &str)]) -> (ResolvedDep, TempDir) {
    let package = TempDir::new().expect("a package temp dir");
    write(
        package.path(),
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\n\
             version = \"1.0.0\"\n\n{snippet_toml}"
        ),
    );
    for (rel, body) in extra_files {
        write(package.path(), rel, body);
    }
    let manifest = Manifest::read(package.path().join("vibe.toml")).expect("the package manifest");
    let resolved = ResolvedDep {
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").expect("the fixture group"),
        name: name.to_owned(),
        version: semver::Version::parse("1.0.0").expect("the fixture version"),
        content_dir: package.path().to_path_buf(),
        source_hash: Some(
            ContentHash::parse(
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            )
            .expect("a fixture content hash"),
        ),
        embedded_sources: Vec::new(),
        manifest,
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
        configuration: None,
    };
    (resolved, package)
}

/// What a fixture declares beyond the default.
#[derive(Default, Clone, Copy)]
pub struct Shape {
    /// Extra lines for the root manifest — a `[system_prompt]` table, say.
    pub manifest_extra: &'static str,
    /// Add a dependency whose boot snippet is gated on `os:`, one row per
    /// operating system, so band G is non-empty.
    pub gated_rows: bool,
    /// Opt in by writing the authored template. Off makes a classic project.
    pub opted_in: bool,
    /// Give the node a static dependency, so a `STATIC.*` lane exists.
    pub static_lane: bool,
    /// Plant user-local sentinel files inside the tree, which no output may
    /// mention (`#SYS-NO-HOME-EXPANSION`).
    pub sentinels: bool,
    /// The spec format the regeneration runs under.
    pub format: SpecFormat,
}

impl Shape {
    /// The ordinary case: opted in, with a static lane, Markdown spelling.
    pub fn standard() -> Self {
        Self {
            opted_in: true,
            static_lane: true,
            ..Self::default()
        }
    }
}

/// The marker text a sentinel file carries; no generated artifact may hold
/// it.
pub const SENTINEL: &str = "SENTINEL-USER-LOCAL-4c77";

/// Build and regenerate one fixture.
pub fn build(shape: Shape) -> Fixture {
    let ws = TempDir::new().expect("a workspace temp dir");
    let mut requires = String::new();
    let mut resolution = Vec::new();
    let mut packages = Vec::new();

    if shape.static_lane {
        requires.push_str("\"org.vibevm/carrier\" = { version = \"^1.0\", link = \"static\" }\n");
        let (resolved, package) = dep(
            "carrier",
            "[boot_snippet]\nsource = \"boot/main.md\"\n",
            &[("boot/main.md", "# carrier\n\nCARRIER-LANE-BYTES\n")],
        );
        resolution.push(resolved);
        packages.push(package);
    }
    if shape.gated_rows {
        requires.push_str("\"org.vibevm/gated\" = { version = \"^1.0\", link = \"dynamic\" }\n");
        let (resolved, package) = dep(
            "gated",
            "[boot_snippet]\nsource = \"boot/win.md\"\nwhen = \"os:windows\"\n",
            &[("boot/win.md", "# windows only\n\nWINDOWS-GATED-BYTES\n")],
        );
        resolution.push(resolved);
        packages.push(package);
        requires.push_str("\"org.vibevm/gated2\" = { version = \"^1.0\", link = \"dynamic\" }\n");
        let (resolved, package) = dep(
            "gated2",
            "[boot_snippet]\nsource = \"boot/mac.md\"\nwhen = \"os:macos\"\n",
            &[("boot/mac.md", "# macos only\n\nMACOS-GATED-BYTES\n")],
        );
        resolution.push(resolved);
        packages.push(package);
    }

    let requires_table = if requires.is_empty() {
        String::new()
    } else {
        format!("\n[requires.packages]\n{requires}")
    };
    write(
        ws.path(),
        "vibe.toml",
        &format!(
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n{requires_table}{}",
            shape.manifest_extra
        ),
    );
    write(
        ws.path(),
        boot_rel("00-core.md"),
        "# own core\n\nOWN-CORE-BYTES\n",
    );
    write(
        ws.path(),
        boot_rel("90-user.md"),
        "# own user\n\nOWN-USER-BYTES\n",
    );
    write(ws.path(), "AGENTS.md", ENTRYPOINT_PROSE);
    if shape.opted_in {
        write(
            ws.path(),
            boot_rel(vibe_core::layout::SYSTEM_TEMPLATE_MD),
            TEMPLATE,
        );
    }
    if shape.sentinels {
        // Authored boot text that *mentions* user-local material. The
        // mention is an instruction to a tool-using agent, never permission
        // to embed the file — and the files themselves sit here so the
        // assertion is about bytes that genuinely exist.
        write(
            ws.path(),
            boot_rel("50-mentions.md"),
            "# mentions\n\nResolve `~/.vibe/steward/contexts/*/plan.toml` and \
             `~/.vibe/zai.api.token` before starting.\n",
        );
        write(ws.path(), "home/.vibe/steward/plan.toml", SENTINEL);
        write(ws.path(), "home/.vibe/steward/custody.toml", SENTINEL);
        write(ws.path(), "home/.vibe/zai.api.token", SENTINEL);
    }

    let fixture = Fixture {
        ws,
        packages,
        resolution,
        format: shape.format,
    };
    fixture.regenerate();
    fixture
}
