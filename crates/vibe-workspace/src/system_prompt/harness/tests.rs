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
    assert!(profile.body.contains("[profiles.vibe-org-vibevm-demo]"));
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
        assert_eq!(answer.notes.len(), 1);
        assert!(answer.notes[0].contains("read"));
    }
}

#[test]
fn an_unknown_kind_is_none_rather_than_a_default() {
    assert_eq!(HarnessKind::parse("qwen"), None);
    assert_eq!(HarnessKind::parse("Claude"), None, "the set is exact");
}
