//! P7 — **stale refusal on every stale input class**, plus the privacy
//! sentinels, the no-growth claim and the ownership conflict.
//!
//! `cli_prompt.rs` proves the two halves of the band model on one class
//! each. This file is the exhaustive sweep: one case per way a prefix can
//! stop being the composition of its own inputs, each asserted on the
//! verdict **and** on the reason that names it, because a verdict with the
//! wrong reason is a verdict nobody can act on (`#SYS-INVALIDATION`).
//!
//! The other half of "stale" is what happens next: a stale prefix is
//! **refused** by `prepare` and by `capture`, never warned about and sent —
//! which is the difference between a freshness claim and a freshness
//! decoration.

use std::fs;

mod common;
#[path = "cli_prompt_matrix/world.rs"]
mod world;

use world::{Recipe, World};

/// `vibe prompt check --json`, whatever the verdict.
fn check(world: &World) -> serde_json::Value {
    let output = world
        .prompt("check")
        .arg("--json")
        .output()
        .expect("`vibe prompt check` runs");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "`check --json` emits one document: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// The verdict's reasons, joined, for a substring assertion.
fn reasons(report: &serde_json::Value) -> String {
    report["reasons"]
        .as_array()
        .expect("a verdict carries its reasons")
        .iter()
        .map(|reason| reason.as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Every way a published prefix stops being the composition of its own
/// inputs, each with the verdict and the sentence that must name it.
#[test]
fn every_stale_input_class_is_named_by_its_own_reason() {
    struct Case {
        /// What the case is called in a failure message.
        name: &'static str,
        /// The verdict `check` must reach.
        verdict: &'static str,
        /// A fragment the reasons must carry.
        names: &'static str,
        /// The mutation, applied to an installed, built world.
        mutate: fn(&World),
    }

    let cases = [
        Case {
            name: "the template's bytes",
            verdict: "stale",
            names: "the template input",
            mutate: |world| {
                fs::write(world.boot("SYSTEM-TEMPLATE.md"), "# another frame\n").unwrap();
            },
        },
        Case {
            name: "the entrypoint's bytes",
            verdict: "stale",
            names: "the entrypoint input",
            mutate: |world| {
                let path = world.root.join("AGENTS.md");
                let mut body = fs::read_to_string(&path).unwrap();
                body.push_str("\nOne more authored sentence.\n");
                fs::write(path, body).unwrap();
            },
        },
        Case {
            name: "the compiled lane's bytes",
            verdict: "stale",
            names: "the lane input",
            mutate: |world| {
                let path = world.boot(vibe_core::layout::STATIC_MD);
                let mut body = fs::read_to_string(&path).unwrap();
                body.push_str("\n<!-- a hand edit of the committed lane -->\n");
                fs::write(path, body).unwrap();
            },
        },
        Case {
            name: "the declared host",
            verdict: "stale",
            names: "`[system_prompt] host` is now",
            mutate: |world| declare(world, "\n[system_prompt]\nhost = \"linux\"\n"),
        },
        Case {
            // The witness is keyed by **role**, so a changed spelling is
            // reported as that role's input changing — and the reason names
            // the new path, which is what a reader needs.
            name: "the declared entrypoint's spelling",
            verdict: "stale",
            names: "the entrypoint input `CLAUDE.md`",
            mutate: |world| {
                fs::write(
                    world.root.join("CLAUDE.md"),
                    "# demo\n\nAnother spelling.\n",
                )
                .unwrap();
                declare(world, "\n[system_prompt]\nentrypoint = \"CLAUDE.md\"\n");
            },
        },
        Case {
            name: "an entrypoint that stops contributing at all",
            verdict: "stale",
            names: "no longer contributes",
            mutate: |world| declare(world, "\n[system_prompt]\nentrypoint = false\n"),
        },
        Case {
            name: "the continuation plan's composition",
            verdict: "stale",
            names: "the continuation plan changed",
            mutate: |world| {
                let path = world.boot(vibe_core::layout::INDEX_MD);
                let mut body = fs::read_to_string(&path).unwrap();
                body.push_str(
                    "\n[[entry]]\npath = \"vibevm/vibespecs/boot/99-late.md\"\nkind = \"static\"\n",
                );
                fs::write(path, body).unwrap();
            },
        },
        Case {
            name: "the continuation plan's own artifact",
            verdict: "stale",
            names: "`INDEX.md` is absent",
            mutate: |world| {
                fs::remove_file(world.boot(vibe_core::layout::INDEX_MD)).unwrap();
            },
        },
        Case {
            name: "a generated artifact that is gone",
            verdict: "missing",
            names: "has not been generated",
            mutate: |world| {
                fs::remove_file(world.boot(vibe_core::layout::SYSTEM_MD)).unwrap();
            },
        },
        Case {
            name: "an opt-out with the artifacts still on disk",
            verdict: "stale",
            names: "it has opted out",
            mutate: |world| {
                fs::remove_file(world.boot("SYSTEM-TEMPLATE.md")).unwrap();
            },
        },
        Case {
            name: "a witnessed lane with no `STATIC.*` left",
            verdict: "freshness_unverifiable",
            names: "only a compile can reproduce one",
            mutate: |world| {
                fs::remove_file(world.boot(vibe_core::layout::STATIC_MD)).unwrap();
            },
        },
    ];

    for case in cases {
        let world = World::build(&Recipe::default());
        assert_eq!(
            check(&world)["verdict"].as_str(),
            Some("equal"),
            "{}: the world starts current",
            case.name
        );
        (case.mutate)(&world);
        let report = check(&world);
        assert_eq!(
            report["verdict"].as_str(),
            Some(case.verdict),
            "{}: {report}",
            case.name
        );
        let reasons = reasons(&report);
        assert!(
            reasons.contains(case.names),
            "{}: the reasons do not name it — `{}` is not in: {reasons}",
            case.name,
            case.names
        );
        // Every non-equal verdict exits non-zero — so a wrapper cannot read
        // a stale prefix as current — and names the command that resolves
        // it, or it is a verdict nobody can act on.
        let said = world.prompt("check").output().expect("check runs");
        assert!(!said.status.success(), "{}: exited zero", case.name);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&said.stdout),
            String::from_utf8_lossy(&said.stderr)
        );
        assert!(
            text.contains("vibe prompt build"),
            "{}: no remedy named: {text}",
            case.name
        );
    }
}

/// Append a declaration to the root manifest.
fn declare(world: &World, extra: &str) {
    let path = world.root.join("vibe.toml");
    let mut body = fs::read_to_string(&path).unwrap();
    body.push_str(extra);
    fs::write(path, body).unwrap();
}

/// A stale prefix is **refused** by the two verbs that would otherwise hand
/// it to a provider, with the exact build that fixes it — not warned about
/// and sent anyway.
#[test]
fn a_stale_prefix_is_refused_by_prepare_and_by_capture() {
    let world = World::build(&Recipe::default());
    fs::write(world.boot("SYSTEM-TEMPLATE.md"), "# a different frame\n").unwrap();
    for verb in ["prepare", "capture"] {
        world
            .prompt(verb)
            .arg("--json")
            .assert()
            .failure()
            .stderr(predicates::str::contains("stale"))
            .stderr(predicates::str::contains("vibe prompt build"));
    }
}

/// Privacy sentinels. A canary sits in user-local material the authored boot
/// text only *mentions*; it reaches no generated artifact and no capture.
/// The mention itself travels verbatim, because authored text is never
/// rewritten (`#SYS-NO-HOME-EXPANSION`, `#SYS-NO-REWRITE-OF-AUTHORED-TEXT`).
#[test]
fn a_user_local_canary_reaches_no_artifact_and_no_capture() {
    let world = World::build(&Recipe {
        sentinels: true,
        ..Recipe::default()
    });
    assert!(
        world.root.join("home/.vibe/steward/plan.toml").exists(),
        "the sentinel files exist, so the assertion is not vacuous"
    );
    for name in [
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_META_JSON,
        vibe_core::layout::AGENT_LANE_MD,
        vibe_core::layout::INDEX_MD,
        vibe_core::layout::STATIC_MD,
    ] {
        assert!(
            !String::from_utf8_lossy(&world.artifact(name)).contains(world::USER_LOCAL_CANARY),
            "`{name}` carries a user-local file's contents"
        );
    }

    // The envelope, bytes and all: the request a runner would send carries
    // none of it either.
    let envelope = world
        .prompt("capture")
        .args(["--json", "--with-text", "--task", "do the thing"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let envelope = String::from_utf8(envelope).unwrap();
    assert!(
        !envelope.contains(world::USER_LOCAL_CANARY),
        "the request envelope carries a user-local file's contents"
    );
    assert!(
        envelope.contains("~/.vibe/steward/contexts/*/plan.toml"),
        "the authored *mention* is delivered verbatim, which is the other half of the rule"
    );
}

/// No recursive growth: three consecutive explicit builds leave every
/// artifact byte-identical, and nothing of the generated set is ever named
/// as one of its own inputs (`#SYS-NO-EQUAL-REWRITE`, `#SYS-SELF-INCLUSION`).
#[test]
fn repeated_builds_leave_every_artifact_byte_identical() {
    let world = World::build(&Recipe::default());
    let names = [
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_META_JSON,
        vibe_core::layout::AGENT_LANE_MD,
        vibe_core::layout::INDEX_MD,
        vibe_core::layout::STATIC_MD,
    ];
    let first: Vec<Vec<u8>> = names.iter().map(|name| world.artifact(name)).collect();
    for round in 1..=3 {
        world.prompt("build").assert().success();
        for (name, bytes) in names.iter().zip(&first) {
            assert_eq!(
                &world.artifact(name),
                bytes,
                "`{name}` changed on build {round}"
            );
        }
    }
    // And the reserved names are in no INDEX row, in any case spelling.
    let rows = world.classic().rows;
    for row in &rows {
        let lower = row.path.to_ascii_lowercase();
        for reserved in [
            vibe_core::layout::SYSTEM_MD,
            vibe_core::layout::SYSTEM_META_JSON,
            vibe_core::layout::AGENT_LANE_MD,
            vibe_core::layout::SYSTEM_TEMPLATE_MD,
        ] {
            assert!(
                !lower.ends_with(&reserved.to_ascii_lowercase()),
                "`{}` is an output or the authored template, not a boot input",
                row.path
            );
        }
    }
}

/// A generated prefix git tracks is an **ownership conflict**, which is the
/// one place `#SYS-ARTIFACTS-NOT-TRACKED` is enforced rather than
/// documented.
#[test]
fn a_git_tracked_prefix_is_an_ownership_conflict() {
    if !common::git_available() {
        eprintln!("skipped: no git on this machine");
        return;
    }
    let world = World::build(&Recipe::default());
    common::run_git(&world.root, &["init", "-q"]);
    common::run_git(&world.root, &["config", "user.email", "t@example.invalid"]);
    common::run_git(&world.root, &["config", "user.name", "Test"]);
    // Track the artifact the ignore file exists to keep untracked.
    let tracked = world
        .boot(vibe_core::layout::SYSTEM_MD)
        .strip_prefix(&world.root)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    common::run_git(&world.root, &["add", "-f", &tracked]);

    let report = check(&world);
    assert_eq!(report["verdict"].as_str(), Some("conflict"), "{report}");
    let reasons = reasons(&report);
    assert!(
        reasons.contains(vibe_core::layout::SYSTEM_MD),
        "the conflict names the path: {reasons}"
    );
}

/// The offline Codex placement proof (`HARNESS-DELIVERY.md` §10 row X0),
/// behind an explicit opt-in.
///
/// It spawns a third-party binary, so it is **not** part of the hermetic
/// suite: `tools/self-check.sh` must not depend on what happens to be on a
/// developer's `PATH`. Set `VIBE_PROMPT_CODEX_PROBE=1` with codex-cli
/// 0.167.0 installed to run it. `codex debug prompt-input` renders the
/// model-visible prompt input and calls no model; `CODEX_HOME` is a temp
/// directory, so the real one is never read or written.
#[test]
fn codex_renders_the_delivered_lane_whole_through_its_profile_file() {
    if std::env::var("VIBE_PROMPT_CODEX_PROBE").as_deref() != Ok("1") {
        eprintln!("skipped: set VIBE_PROMPT_CODEX_PROBE=1 to probe the installed codex");
        return;
    }
    let world = World::build(&Recipe::default());
    let lane = fs::read_to_string(world.boot(vibe_core::layout::AGENT_LANE_MD)).unwrap();
    let home = world.temp.path().join("codex-home");
    fs::create_dir_all(&home).unwrap();
    let profile = "vibeprobe";
    let mut carrier = toml::Table::new();
    carrier.insert(
        "developer_instructions".to_owned(),
        toml::Value::String(lane.clone()),
    );
    fs::write(
        home.join(format!("{profile}.config.toml")),
        toml::to_string(&carrier).unwrap(),
    )
    .unwrap();

    let rendered = std::process::Command::new("codex")
        .env("CODEX_HOME", &home)
        .args(["-p", profile, "debug", "prompt-input", "hi"])
        .output()
        .expect("codex is on PATH");
    assert!(rendered.status.success(), "{rendered:?}");
    let rendered = String::from_utf8_lossy(&rendered.stdout).into_owned();
    // Both ends of the lane arrive: the first marker line and the last.
    let first = lane.lines().next().unwrap();
    let last = lane.lines().next_back().unwrap();
    for canary in [first, last] {
        assert!(
            rendered.contains(canary),
            "codex dropped `{canary}` from the rendered prompt"
        );
    }
}
