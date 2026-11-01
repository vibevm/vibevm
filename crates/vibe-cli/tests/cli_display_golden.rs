//! Golden captures of the terminal display language (PROP-067
//! `#acceptance`): `install -g` unattended and failing at a step,
//! `app plan`, `remap list`, `app list -g` and one refusal, each run on a
//! redirected stream and compared as plain text with the volatile parts
//! (times, temporary paths, run stamps, the rule's length) normalised.
//!
//! The attended captures — `install -g` mid-run with its fold, the same run
//! when it ended, and the `--plain` capture of it — are taken under
//! `script` on a real terminal and stored with their attributes stripped;
//! this file checks that the plain capture reads block for block as the
//! attended one.

#![cfg(target_os = "linux")]

mod common;
mod display_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use display_support::{GOLDEN, golden, normalise};

struct Fixture {
    root: tempfile::TempDir,
    registry: PathBuf,
}

impl Fixture {
    /// A registry with `org.example/noisy`, whose recipe prints forty
    /// lines, and `org.example/broken`, whose recipe prints forty lines and
    /// fails.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let fixture = Self { root, registry };
        fixture.package("noisy", "");
        fixture.package("broken", "exit 5\n");
        fixture
    }

    fn package(&self, name: &str, tail: &str) {
        let package = self.registry.join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("vibe.toml"),
            format!(
                "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\n\
                 version = \"1.0.0\"\n\n[application]\nid = \"{name}\"\n\n\
                 [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\n\
                 platform_name = \"Linux\"\narch = \"{}\"\n\
                 [[installer_options.steps]]\nid = \"build\"\ntitle = \"Build\"\n\
                 kind = \"recipe\"\ninterpreter = \"posix_sh\"\nscript = \"build.sh\"\n",
                std::env::consts::ARCH
            ),
        )
        .unwrap();
        fs::write(
            package.join("build.sh"),
            format!(
                "set -e\ni=1\nwhile [ $i -le 40 ]; do echo \"NOISE line $i\"; i=$((i+1)); done\n\
                 {tail}mkdir -p \"$VIBE_OUT/bin\"\necho ok > \"$VIBE_OUT/bin/tool\"\n"
            ),
        )
        .unwrap();
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        let mut command = user.vibe();
        command.current_dir(self.root.path()).args(args);
        for variable in [
            "VIBE_APP_LOG",
            "VIBE_APP_LOG_TAIL",
            "VIBE_INVOKED_BY",
            "VIBE_AGENT_COMMAND",
            "NO_COLOR",
        ] {
            command.env_remove(variable);
        }
        command.output().unwrap()
    }

    fn app(&self, user: &UserScratch, args: &[&str]) -> Output {
        let mut all = args.to_vec();
        all.extend_from_slice(&["--registry", self.registry.to_str().unwrap()]);
        self.vibe(user, &all)
    }
}

/// Stdout then stderr, as a terminal shows a run whose streams it merged
/// in the order the blocks were written: every capture here writes its
/// blocks to one stream or ends its stderr before its stdout begins.
fn screen(output: &Output, user: &UserScratch, fixture: &Fixture) -> String {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    normalise(
        &text,
        &[
            (&user.settings, "<settings>"),
            (&fixture.registry, "<registry>"),
        ],
    )
}

#[test]
fn the_unattended_install_its_listing_and_its_plan_read_as_the_goldens() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    let plan = fixture.app(&user, &["app", "plan", "org.example/noisy"]);
    assert!(plan.status.success());
    golden("app-plan", &screen(&plan, &user, &fixture));

    let installed = fixture.app(
        &user,
        &[
            "install",
            "-g",
            "org.example/noisy",
            "--unattended",
            "--plain",
        ],
    );
    assert!(
        installed.status.success(),
        "{}",
        screen(&installed, &user, &fixture)
    );
    golden("install-unattended", &screen(&installed, &user, &fixture));

    let listed = fixture.vibe(&user, &["app", "list", "-g"]);
    assert!(listed.status.success());
    golden("app-list", &screen(&listed, &user, &fixture));
}

#[test]
fn a_failed_step_and_a_refusal_read_as_the_goldens() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    let failed = fixture.app(
        &user,
        &[
            "install",
            "-g",
            "org.example/broken",
            "--unattended",
            "--plain",
        ],
    );
    assert_eq!(failed.status.code(), Some(1));
    golden("install-failed", &screen(&failed, &user, &fixture));

    let refused = fixture.app(
        &user,
        &[
            "install",
            "-g",
            "org.example/noisy",
            "--unattended",
            "--from-step",
            "nope",
        ],
    );
    assert_eq!(refused.status.code(), Some(2));
    golden("refusal", &screen(&refused, &user, &fixture));
}

#[test]
fn the_remap_listing_reads_as_its_golden() {
    let fixture = Fixture::new();
    let user = UserScratch::new();
    let listed = fixture.vibe(&user, &["--offline", "remap", "list"]);
    assert!(listed.status.success());
    golden("remap-list", &screen(&listed, &user, &fixture));
}

/// The gutter of each block: its glyph and the word after it, or the rule
/// and its label.
fn gutters(text: &str) -> Vec<String> {
    text.split("\n\n")
        .filter_map(|block| block.lines().next())
        .map(|line| {
            line.split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

#[test]
fn the_plain_capture_reads_block_for_block_as_the_attended_one() {
    let read = |name: &str| fs::read_to_string(Path::new(GOLDEN).join(name)).unwrap();
    let attended = read("install-attended.txt");
    let plain = read("install-plain.txt");
    assert_eq!(gutters(&attended), gutters(&plain));
    let running = read("install-attended-running.txt");
    assert!(
        running.contains("\n  │ NOISE line") && running.contains("\n  (l to show the live log)"),
        "the running block shows its fold and the hint: {running}"
    );
    assert!(
        attended.contains("\n✓ build  Build · <time> · 40 lines\n"),
        "{attended}"
    );
}
