//! Flags only, never a program — and never lane bytes on a command line.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-DELIVERY",
    r = 1
);

use std::path::Path;

use vibe_core::manifest::HarnessDelivery;

use super::*;

const LANE: &str = "<!-- lane -->\n\"quoted\"\tand \\ a backslash\n";

fn flags(kind: HarnessKind, delivery: HarnessDelivery) -> LaunchFlags {
    launch_flags(
        kind,
        delivery,
        "org.vibevm/demo",
        Path::new("/w/vibevm/vibespecs/boot/AGENT-LANE.md"),
        LANE,
        32 * 1024,
    )
}

#[test]
fn claude_takes_exactly_one_file_flag_and_the_exclude_flag() {
    let answer = flags(HarnessKind::Claude, HarnessDelivery::Launch);
    assert_eq!(
        answer.flags,
        [
            "--append-system-prompt-file",
            "/w/vibevm/vibespecs/boot/AGENT-LANE.md",
            "--exclude-dynamic-system-prompt-sections",
        ]
    );
    assert!(answer.profile.is_none());
    // Exactly one file flag: a repeated one is how a session silently takes
    // a prompt nobody chose.
    assert_eq!(
        answer
            .flags
            .iter()
            .filter(|flag| *flag == "--append-system-prompt-file")
            .count(),
        1
    );
}

#[test]
fn codex_takes_the_profile_and_the_declared_doc_limit() {
    let answer = launch_flags(
        HarnessKind::Codex,
        HarnessDelivery::Launch,
        "org.vibevm/demo",
        Path::new("/w/boot/AGENT-LANE.md"),
        LANE,
        262_144,
    );
    assert_eq!(
        answer.flags,
        [
            "-p",
            "vibe-org-vibevm-demo",
            "-c",
            "project_doc_max_bytes=262144",
        ]
    );
    let profile = answer.profile.expect("codex launches through a profile");
    assert_eq!(profile.file_name, "vibe-org-vibevm-demo.config.toml");
    // `-p <name>` LAYERS the file on top of the base user config, so the
    // key is top-level. Inside a `[profiles.<name>]` table it is a nested
    // table codex-cli 0.167.0 loads and never reads — measured, and the
    // reason this assertion exists at all.
    assert!(
        !profile.body.contains("[profiles."),
        "a table header would make the carrier a nested table: {}",
        profile.body
    );
    let parsed: toml::Table = profile.body.parse().expect("the carrier parses as TOML");
    assert_eq!(
        parsed
            .get("developer_instructions")
            .and_then(toml::Value::as_str),
        Some(LANE),
        "the top-level key carries the lane byte for byte"
    );
    // A profile name with a dot in it would be a NESTED table header, and
    // the profile `-p` asks for would not exist.
    assert!(!profile.name.contains('.'), "{}", profile.name);
    // The carrier is a basic string with escapes, never a `'''` literal
    // block: a literal block cannot contain `'''` and the lane is arbitrary
    // authored text.
    assert!(!profile.body.contains("'''"));
    assert!(profile.body.contains("\\\"quoted\\\""));
    assert!(profile.body.contains("\\t"));
    assert!(profile.body.contains("\\\\ a backslash"));
}

#[test]
fn no_flag_ever_carries_the_lanes_own_bytes() {
    for kind in HarnessKind::ALL {
        for flag in flags(kind, HarnessDelivery::Launch).flags {
            assert!(
                !flag.contains("<!-- lane -->"),
                "{kind:?} put lane bytes on the command line"
            );
        }
    }
}

#[test]
fn read_delivery_passes_no_flag_and_says_why() {
    for kind in HarnessKind::ALL {
        let answer = flags(kind, HarnessDelivery::Read);
        assert!(answer.flags.is_empty(), "{kind:?}");
        assert!(answer.profile.is_none());
        assert!(!answer.notes.is_empty(), "{kind:?}");
        assert!(answer.notes[0].contains("read"), "{kind:?}");
    }
}

#[test]
fn opencode_passes_no_flag_under_either_delivery_and_names_its_carrier() {
    // **A kind with no deliverable slot is still a kind.** `vibe agent
    // opencode` has to exist and `[system_prompt.harness.opencode]` has to
    // be declarable; what the dialect answers is "nothing, and here is
    // why" (`#SYS-OPENCODE`).
    for delivery in [HarnessDelivery::Read, HarnessDelivery::Launch] {
        let answer = flags(HarnessKind::OpenCode, delivery);
        assert!(answer.flags.is_empty(), "{delivery:?}");
        assert!(answer.profile.is_none(), "{delivery:?}");
        let said = answer.notes.join(" ");
        // The carrier is named, and so is the reason it is not delivered.
        assert!(said.contains("OPENCODE_CONFIG_CONTENT"), "{said}");
        assert!(said.contains("subagents included"), "{said}");
        assert!(said.contains(OPENCODE_MEASURED_VERSION), "{said}");
        // A path, never the lane's own bytes — the carrier would travel in
        // the environment, and an environment is not a safer command line.
        assert!(
            said.contains("/w/vibevm/vibespecs/boot/AGENT-LANE.md"),
            "{said}"
        );
        assert!(!said.contains("<!-- lane -->"), "{said}");
    }
}

#[test]
fn qwen_passes_no_flag_under_either_delivery_and_names_its_slot() {
    // **The slot exists, and the note names it rather than denying it.**
    // What closes the tier is that `--append-system-prompt` takes the
    // instructions as text on a command line, where a lane does not fit
    // (`#SYS-QWEN`).
    for delivery in [HarnessDelivery::Read, HarnessDelivery::Launch] {
        let answer = flags(HarnessKind::Qwen, delivery);
        assert!(answer.flags.is_empty(), "{delivery:?}");
        assert!(answer.profile.is_none(), "{delivery:?}");
        let said = answer.notes.join(" ");
        assert!(said.contains("--append-system-prompt"), "{said}");
        assert!(said.contains("QWEN_SYSTEM_MD"), "{said}");
        assert!(said.contains("128 KiB"), "{said}");
        assert!(said.contains(QWEN_MEASURED_VERSION), "{said}");
        // The argument is shown with the lane's PATH inside a `cat`, which
        // is what makes "the lane's own bytes on the command line" legible
        // — and the lane's bytes themselves are never in the note.
        assert!(
            said.contains("/w/vibevm/vibespecs/boot/AGENT-LANE.md"),
            "{said}"
        );
        assert!(!said.contains("<!-- lane -->"), "{said}");
    }
}

#[test]
fn the_qwen_blocker_names_both_slots_and_what_closes_each() {
    // **Two slots, two different reasons, and the sentence carries both.**
    // Stopping at the additive one would leave a reader believing Qwen Code
    // has no file-based slot, which is false and is the more interesting
    // half: it exists, it is subagent-safe, and what closes it is that it
    // REPLACES a prompt whose bytes depend on the session (`#SYS-QWEN`).
    let blocker = qwen_launch_blocker();
    // Worker exclusion is satisfied here, unlike `opencode` — said out loud
    // so a reader does not carry that kind's conclusion over.
    assert!(
        blocker.contains("worker exclusion is satisfied"),
        "{blocker}"
    );
    assert!(blocker.contains("E2BIG"), "{blocker}");
    assert!(blocker.contains("202,349"), "{blocker}");
    assert!(blocker.contains("QWEN_SYSTEM_MD"), "{blocker}");
    assert!(blocker.contains("QWEN_WRITE_SYSTEM_MD"), "{blocker}");
    // The measurement that closes the file-based slot: one prompt, two
    // sizes, decided by something a launcher cannot observe.
    assert!(blocker.contains("13,856"), "{blocker}");
    assert!(blocker.contains("18,451"), "{blocker}");
    assert!(blocker.contains(QWEN_MEASURED_VERSION), "{blocker}");
}

#[test]
fn the_opencode_blocker_is_version_scoped_and_names_worker_exclusion() {
    let blocker = opencode_launch_blocker();
    assert!(blocker.contains("SYS-WORKER-EXCLUSION"), "{blocker}");
    assert!(blocker.contains(OPENCODE_MEASURED_VERSION), "{blocker}");
    assert!(blocker.contains("instructions"), "{blocker}");
    assert!(blocker.contains("background service"), "{blocker}");
}

#[test]
fn the_carrier_verdict_is_one_measured_version_and_not_a_floor() {
    assert_eq!(
        codex_carrier_verdict(&format!("codex-cli {CODEX_VERIFIED_VERSION}")),
        CarrierVerdict::Verified
    );
    assert_eq!(
        codex_carrier_verdict("codex-cli v0.167.0"),
        CarrierVerdict::Verified,
        "a leading `v` is a spelling, not a version"
    );
    // Newer is NOT better here: the per-profile behaviour of this key
    // already changed once under us, and the failure mode is silent.
    assert_eq!(
        codex_carrier_verdict("codex-cli 0.168.0"),
        CarrierVerdict::Unverified {
            installed: "0.168.0".to_owned()
        }
    );
    assert_eq!(
        codex_carrier_verdict("codex-cli 0.166.9"),
        CarrierVerdict::Unverified {
            installed: "0.166.9".to_owned()
        }
    );
}

#[test]
fn a_version_that_cannot_be_read_claims_nothing_either_way() {
    for reported in ["", "codex-cli", "   \n"] {
        assert!(
            matches!(
                codex_carrier_verdict(reported),
                CarrierVerdict::Unknown { .. }
            ),
            "{reported:?}"
        );
    }
}

#[test]
fn an_unknown_kind_is_none_rather_than_a_default() {
    assert_eq!(HarnessKind::parse("gemini"), None);
    assert_eq!(HarnessKind::parse("Claude"), None, "the set is exact");
    assert_eq!(HarnessKind::parse("OpenCode"), None, "the set is exact");
    assert_eq!(HarnessKind::parse("Qwen"), None, "the set is exact");
}

#[test]
fn a_node_with_no_system_prompt_table_takes_each_kinds_own_default() {
    // One default for every kind was harmless while the set held only the
    // two file-slot harnesses. It is not once it does not: `opencode` would
    // have read `launch` from a project that declared nothing at all.
    let manifest = vibe_core::manifest::Manifest::default();
    assert!(manifest.system_prompt.is_none());
    assert_eq!(
        HarnessKind::Claude.delivery(&manifest),
        HarnessDelivery::Launch
    );
    assert_eq!(
        HarnessKind::Codex.delivery(&manifest),
        HarnessDelivery::Launch
    );
    assert_eq!(
        HarnessKind::OpenCode.delivery(&manifest),
        HarnessDelivery::Read
    );
    assert_eq!(HarnessKind::Qwen.delivery(&manifest), HarnessDelivery::Read);
}
