//! P7 — the **qualification matrix**: every delivery, every lane spelling,
//! every platform axis, every scope.
//!
//! The differential parity itself is
//! `crates/vibe-workspace/tests/system_prompt_parity.rs`; this file carries
//! the matrix that parity has to hold *across*, and it does it through the
//! real commands rather than through the library — because the claim here is
//! about what a developer's launch actually delivers.
//!
//! **The reference is read from the published contract**, by
//! [`world::Classic`], which chooses the `STATIC.*` spelling with two
//! `exists` calls and reads `INDEX.md` as the TOML document the boot block
//! says it is. Nothing here asks the composer what it composed.
//!
//! **Nothing real is ever started.** The launch tests go to shims on `PATH`
//! that record their argv and exit, exactly as `cli_agent_launch.rs` does.

use std::fs;
use std::path::PathBuf;

use predicates::prelude::*;

mod common;
#[path = "cli_prompt_matrix/world.rs"]
mod world;

use world::{Recipe, World};

// ---------------------------------------------------------------------------
// Format: both lane spellings
// ---------------------------------------------------------------------------

/// The delivered lane equals the classic boot under either `STATIC`
/// spelling. The XML leg matters most: `SYSTEM.md`'s `.md` extension is a
/// name and not a parse, and the lane segment holds XML bytes verbatim
/// (`#SYS-LANE-VERBATIM`).
#[test]
fn the_delivered_lane_equals_the_classic_boot_under_either_lane_spelling() {
    for (format, expected_spelling) in [
        ("markdown", vibe_core::layout::STATIC_MD),
        ("xml", vibe_core::layout::STATIC_XML),
    ] {
        let world = World::build(&Recipe {
            format,
            ..Recipe::default()
        });
        let classic = world.classic();
        let (lane_rel, _) = classic
            .lane
            .as_ref()
            .unwrap_or_else(|| panic!("{format}: a static lane"));
        assert!(
            lane_rel.ends_with(expected_spelling),
            "{format}: the selected spelling is `{expected_spelling}`, not `{lane_rel}`"
        );
        assert_eq!(
            world::split_lane(&world.artifact(vibe_core::layout::AGENT_LANE_MD)),
            classic.delivery(&world.root),
            "{format}: the delivered lane is the classic lane then band U, in INDEX order"
        );
        // The dependency's own bytes really are in there, so the comparison
        // is over a lane with content rather than over two empty lists.
        assert!(
            String::from_utf8_lossy(&world.artifact(vibe_core::layout::AGENT_LANE_MD))
                .contains("CARRIER-LANE-BYTES"),
            "{format}: the static dependency contributed to the lane"
        );
    }
}

// ---------------------------------------------------------------------------
// Delivery: read, launch for claude, launch for codex
// ---------------------------------------------------------------------------

/// What a `launch` delivery hands each harness is the **same** bytes, and
/// those bytes are the classic boot. Claude Code gets a file path in its
/// slot flag; Codex gets the same file's bytes as `developer_instructions`
/// at its layered profile file's top level. Both are compared against the
/// one reference, so a divergence between the two adapters is a failure
/// rather than a thing nobody looked at.
#[cfg(unix)]
#[test]
fn both_launch_adapters_deliver_the_classic_boot_and_the_same_bytes() {
    let world = World::build(&Recipe::default());
    let shims = Shims::beside(&world);
    let expected = world.classic().delivery(&world.root);

    // Claude Code: the slot is a file flag, and the file it names is the
    // lane. No lane bytes on a command line, on any operating system.
    shims.agent(&world, "claude").assert().success();
    let argv = shims.argv();
    let slot = argv
        .windows(2)
        .find(|pair| pair[0] == "--append-system-prompt-file")
        .map(|pair| PathBuf::from(&pair[1]))
        .expect("the claude adapter names a file");
    let claude_bytes = fs::read(&slot).expect("the file the slot names");
    assert_eq!(
        world::split_lane(&claude_bytes),
        expected,
        "the file Claude Code is handed is the classic boot"
    );
    assert!(
        argv.contains(&"--exclude-dynamic-system-prompt-sections".to_owned()),
        "{argv:?}"
    );

    // Codex: the same bytes, through the layered profile file.
    shims.agent(&world, "work-codex").assert().success();
    let argv = shims.argv();
    let profile = argv
        .windows(2)
        .find(|pair| pair[0] == "-p")
        .map(|pair| pair[1].clone())
        .expect("the codex adapter names a profile");
    let body = fs::read_to_string(shims.codex_home().join(format!("{profile}.config.toml")))
        .expect("the launch-time profile file");
    let parsed: toml::Table = body.parse().expect("the carrier is valid TOML");
    let codex_bytes = parsed["developer_instructions"]
        .as_str()
        .expect("top-level, not `[profiles.<name>]`")
        .as_bytes()
        .to_vec();
    assert_eq!(
        codex_bytes, claude_bytes,
        "the two adapters deliver the same bytes"
    );
    assert_eq!(world::split_lane(&codex_bytes), expected);
}

/// `read` delivery: the router alone names the same files, in the same
/// order, and carries the delivered lane's own id so a stale copy is
/// detectable (`#SYS-LANE-MARKERS`).
#[test]
fn read_delivery_points_at_the_same_files_the_classic_boot_reads() {
    let world = World::build(&Recipe::default());
    let classic = world.classic();
    let entrypoint = String::from_utf8(classic.entrypoint.clone()).unwrap();
    let (lane_rel, _) = classic.lane.as_ref().expect("a static lane");
    let index_rel = world::relative(&world.root, &world.boot(vibe_core::layout::INDEX_MD));
    assert!(
        entrypoint.contains(&format!(
            "The boot lane is `{lane_rel}`, then every file listed in `{index_rel}`."
        )),
        "the router names the lane and the INDEX:\n{entrypoint}"
    );
    for row in classic.unconditional() {
        assert!(
            world.root.join(&row.path).exists(),
            "every file a read delivery is sent to exists: {}",
            row.path
        );
    }
}

// ---------------------------------------------------------------------------
// Host, target, and the session gate
// ---------------------------------------------------------------------------

/// `host = "portable"` (the default) keeps `os:` rows in band G, and a
/// `prepare` with no explicit session target leaves them **unresolved**
/// rather than falling back to the generator's own operating system.
#[test]
fn a_portable_host_keeps_os_rows_gated_and_never_guesses_the_session() {
    let world = World::build(&Recipe {
        gated: true,
        ..Recipe::default()
    });
    let classic = world.classic();
    let gated: Vec<&world::Row> = classic
        .rows
        .iter()
        .filter(|row| row.when.is_some())
        .collect();
    assert_eq!(gated.len(), 3, "one gated row per platform: {gated:?}");

    let blind = prepare(&world, None);
    assert!(
        blind["gated"].as_array().unwrap().is_empty(),
        "no row is selected without a target: {blind}"
    );
    assert_eq!(
        blind["unresolved"].as_array().unwrap().len(),
        3,
        "every gated row is reported unresolved: {blind}"
    );

    for os in ["linux", "macos", "windows"] {
        let prepared = prepare(&world, Some(os));
        let selected: Vec<&str> = prepared["gated"]
            .as_array()
            .unwrap()
            .iter()
            .map(|band| band["path"].as_str().unwrap())
            .collect();
        assert_eq!(selected.len(), 1, "{os}: exactly its own row: {prepared}");
        assert!(selected[0].contains(os), "{os}: {selected:?}");
        assert!(
            prepared["unresolved"].as_array().unwrap().is_empty(),
            "{os}: a supplied target resolves every gate: {prepared}"
        );
    }

    // And none of the gated bytes are in either artifact: a portable prefix
    // is the same bytes whatever machine reads it.
    for name in [
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::AGENT_LANE_MD,
    ] {
        let text = String::from_utf8_lossy(&world.artifact(name)).into_owned();
        for os in ["LINUX", "MACOS", "WINDOWS"] {
            assert!(
                !text.contains(&format!("{os}-GATED-BYTES")),
                "`{name}` baked a gated row"
            );
        }
    }
}

/// A fixed `host` resolves the session's-OS gates at build time: the
/// matching row is baked into the lane as an ordinary segment, the others
/// are dropped, band G is empty, and `prepare` needs no session target
/// (`#SYS-HOST-AND-TARGET`).
#[test]
fn a_fixed_host_bakes_its_own_os_rows_and_empties_the_gated_band() {
    let world = World::build(&Recipe {
        gated: true,
        manifest_extra: "\n[system_prompt]\nhost = \"macos\"\n".to_owned(),
        ..Recipe::default()
    });
    let lane =
        String::from_utf8_lossy(&world.artifact(vibe_core::layout::AGENT_LANE_MD)).into_owned();
    assert!(
        lane.contains("MACOS-GATED-BYTES"),
        "the declared host's row is delivered"
    );
    for other in ["LINUX", "WINDOWS"] {
        assert!(
            !lane.contains(&format!("{other}-GATED-BYTES")),
            "a non-matching row is dropped rather than gated:\n{lane}"
        );
    }
    let sidecar: serde_json::Value =
        serde_json::from_slice(&world.artifact(vibe_core::layout::SYSTEM_META_JSON)).unwrap();
    assert_eq!(sidecar["host"].as_str(), Some("macos"));
    assert!(
        sidecar["continuation"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["band"].as_str() == Some("unconditional")),
        "a fixed host leaves no gated row: {sidecar}"
    );
    let prepared = prepare(&world, None);
    assert!(
        prepared["unresolved"].as_array().unwrap().is_empty(),
        "a fixed-host prefix needs no session target: {prepared}"
    );
}

/// A `target:` selection is the product's platform, always resolved at build
/// time: the selected row's bytes are in the lane and the others are gone
/// entirely — never gated, and never an `installed:`-style residual.
#[test]
fn a_declared_target_selection_is_resolved_before_a_row_exists() {
    let world = World::build(&Recipe {
        manifest_extra: "\n[system_prompt]\ntarget = [\"windows\"]\n".to_owned(),
        ..Recipe::default()
    });
    let classic = world.classic();
    assert!(
        classic.rows.iter().all(|row| row.when.is_none()),
        "a `target:` gate never reaches an INDEX row: {:?}",
        classic.rows
    );
    let sidecar: serde_json::Value =
        serde_json::from_slice(&world.artifact(vibe_core::layout::SYSTEM_META_JSON)).unwrap();
    assert_eq!(
        sidecar["target"]
            .as_array()
            .unwrap()
            .iter()
            .map(|os| os.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["windows"],
        "the sidecar records the declared selection, which is an authored value"
    );
}

/// An `installed:` gate is a generation-time decision: present, the
/// contribution is ordinary; absent, it is gone entirely. Either way the
/// condition never reaches an INDEX row or a band.
#[test]
fn an_installed_gate_is_decided_at_generation_and_reaches_no_band() {
    for present in [true, false] {
        let world = World::build(&Recipe {
            installed_gate: Some(present),
            ..Recipe::default()
        });
        let lane =
            String::from_utf8_lossy(&world.artifact(vibe_core::layout::AGENT_LANE_MD)).into_owned();
        assert_eq!(
            lane.contains("INSTALLED-GATED-BYTES"),
            present,
            "installed:{present} decides membership, not read timing"
        );
        for row in &world.classic().rows {
            assert!(
                row.when
                    .as_deref()
                    .is_none_or(|when| !when.contains("installed:")),
                "an `installed:` condition reached an INDEX row: {row:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Scope: root, member, and the quiet worker
// ---------------------------------------------------------------------------

/// A member node gets its own prefix from its own scope, and carries no byte
/// of the root's frame. A root's legitimate aggregation of its own subtree
/// stays legitimate; what is excluded is content from outside the selected
/// scope (`#SYS-WORKER-EXCLUSION`, PROP-009 `#MATRYOSHKA-SCOPES`).
#[test]
fn a_member_node_publishes_its_own_prefix_and_not_the_roots() {
    let world = World::build(&Recipe {
        member: true,
        ..Recipe::default()
    });
    world
        .prompt("build")
        .args(["--node", world::MEMBER_REL])
        .assert()
        .success()
        .stdout(predicate::str::contains("system prompt: "));

    let member_boot = world.root.join(world::MEMBER_REL).join(common::boot_dir());
    let prefix = fs::read_to_string(member_boot.join(vibe_core::layout::SYSTEM_MD))
        .expect("the member's own prefix");
    assert!(
        prefix.contains("MEMBER-TEMPLATE-CANARY-8d42"),
        "the member's prefix is built from the member's own template"
    );
    assert!(
        !prefix.contains(world::CENTRAL_CANARY),
        "the member's prefix carries the root's frame"
    );
    assert!(
        !prefix.contains(world::ENTRYPOINT_CANARY),
        "the member's prefix carries the root's entrypoint"
    );
    // The root's own prefix is untouched by the member's build.
    assert!(
        String::from_utf8_lossy(&world.artifact(vibe_core::layout::SYSTEM_MD))
            .contains(world::CENTRAL_CANARY)
    );
}

/// A worker whose packet scopes it to a node that has not opted in receives
/// **no** central prefix at all — not a filtered one, and not briefly. The
/// prefix is never assembled for it, so `capture` has nothing to build and
/// says so, and nothing of the central node's canaries is in the answer.
#[test]
fn a_worker_scoped_to_a_node_without_a_template_receives_no_prefix() {
    let world = World::build(&Recipe {
        member: true,
        ..Recipe::default()
    });
    // Opting the member out: the worker's node has no frame of its own.
    fs::remove_file(
        world
            .root
            .join(world::MEMBER_REL)
            .join(common::boot_rel("SYSTEM-TEMPLATE.md")),
    )
    .unwrap();

    let output = world
        .prompt("capture")
        .args(["--node", world::MEMBER_REL, "--json", "--with-text"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        said.contains("has not opted in") || said.contains("SYSTEM-TEMPLATE"),
        "the refusal names the opt-in: {said}"
    );
    for canary in [
        world::CENTRAL_CANARY,
        world::ENTRYPOINT_CANARY,
        "TEMPLATE-CANARY-7a31",
    ] {
        assert!(
            !said.contains(canary),
            "a worker's answer carried the central canary `{canary}`: {said}"
        );
    }
}

// ---------------------------------------------------------------------------
// Classic mode: no `[system_prompt]`, no template, no prefix machinery
// ---------------------------------------------------------------------------

/// **The classic-mode guarantee.** A project that never opted in boots
/// exactly as it does today: the regenerated router block alone directs a
/// reader to `STATIC.*` first and then INDEX order, and not one file of the
/// prefix machinery is written.
#[test]
fn a_classic_project_gets_a_router_to_static_then_index_and_no_prefix_files() {
    let world = World::build(&Recipe {
        opted_in: false,
        ..Recipe::default()
    });
    let classic = world.classic();
    let (lane_rel, _) = classic.lane.as_ref().expect("a classic project has a lane");
    let entrypoint = String::from_utf8(classic.entrypoint.clone()).unwrap();

    // Rule 4 verbatim — the rule that makes a plain launch read the lane,
    // and the whole of what a classic project relies on.
    assert!(
        entrypoint.contains(&format!(
            "4. A lane file with no lane lines in your context, and which you have\n   \
             not already read in this session: read it from disk — `{lane_rel}`\n   \
             first, then INDEX order."
        )),
        "the router's rule 4 names the lane first, then INDEX order:\n{entrypoint}"
    );
    assert!(
        entrypoint.contains(&format!(
            "The boot lane is `{lane_rel}`, then every file listed in"
        )),
        "{entrypoint}"
    );

    // And nothing of the prefix machinery exists. Asserted over the whole
    // boot directory listing, so a fifth artifact nobody expected would
    // fail here too.
    let present: Vec<String> = fs::read_dir(world.boot(""))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    for reserved in [
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_META_JSON,
        vibe_core::layout::AGENT_LANE_MD,
        vibe_core::layout::BOOT_GITIGNORE,
        vibe_core::layout::SYSTEM_TEMPLATE_MD,
    ] {
        assert!(
            !present.iter().any(|name| name == reserved),
            "`{reserved}` exists in a project that never opted in: {present:?}"
        );
    }

    // The verbs say so rather than inventing an artifact.
    world
        .prompt("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("has not opted in"));
    world
        .prompt("path")
        .assert()
        .failure()
        .stderr(predicate::str::contains("build"));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `vibe prompt prepare --json`, with an optional session target.
fn prepare(world: &World, session_target: Option<&str>) -> serde_json::Value {
    let mut command = world.prompt("prepare");
    command.arg("--json");
    if let Some(os) = session_target {
        command.arg("--session-target").arg(format!("os={os}"));
    }
    let stdout = command.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("`prepare --json` emits one document")
}

/// Shims on `PATH` that record their argv — the only honest way to assert
/// what a launch passes.
#[cfg(unix)]
struct Shims {
    bin: tempfile::TempDir,
}

#[cfg(unix)]
impl Shims {
    /// Write the shims and the user launchers that point at them.
    fn beside(world: &World) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let bin = tempfile::tempdir().unwrap();
        for name in ["my-claude", "codex"] {
            let path = bin.path().join(name);
            fs::write(
                &path,
                "#!/bin/sh\n\
                 if [ \"$1\" = \"--version\" ]; then printf '%s\\n' \"$SHIM_VERSION\"; exit 0; fi\n\
                 printf '%s\\n' \"$@\" > \"$SHIM_ARGV\"\nexit 0\n",
            )
            .unwrap();
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }
        fs::write(
            world.user.settings.join("agents.toml"),
            "[launchers.claude]\nkind = \"claude\"\ncommand = [\"my-claude\"]\n\n\
             [launchers.work-codex]\nkind = \"codex\"\ncommand = [\"codex\"]\n",
        )
        .unwrap();
        Self { bin }
    }

    fn codex_home(&self) -> PathBuf {
        self.bin.path().join("codex-home")
    }

    fn argv(&self) -> Vec<String> {
        fs::read_to_string(self.bin.path().join("argv.txt"))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// `vibe agent <launcher>`, with the shims reachable and the one
    /// verified codex version answered.
    fn agent(&self, world: &World, launcher: &str) -> assert_cmd::Command {
        let existing = std::env::var("PATH").unwrap_or_default();
        let mut command = world.vibe();
        command
            .env("PATH", format!("{}:{existing}", self.bin.path().display()))
            .env("SHIM_ARGV", self.bin.path().join("argv.txt"))
            .env("SHIM_VERSION", "codex-cli 0.167.0")
            .env("CODEX_HOME", self.codex_home())
            .arg("agent")
            .arg(launcher);
        command
    }
}
