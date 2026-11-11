//! Flags only, never a program — and never lane bytes on a command line.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-DELIVERY",
    r = 1
);

use std::path::{Path, PathBuf};

use vibe_core::manifest::HarnessDelivery;

use super::super::qwen::{QwenCarrier, QwenMode};
use super::*;

const LANE: &str = "<!-- lane -->\n\"quoted\"\tand \\ a backslash\n";

const LANE_PATH: &str = "/w/vibevm/vibespecs/boot/AGENT-LANE.md";

fn inputs<'a>(supplied: Option<&'a CarrierDocument>) -> LaunchInputs<'a> {
    LaunchInputs {
        node_id: "org.vibevm/demo",
        lane: Path::new(LANE_PATH),
        lane_bytes: LANE,
        codex_limit: 32 * 1024,
        supplied,
        qwen: None,
    }
}

/// A composed identity carrier shaped like the one a Qwen Code launch hands
/// over: a file in vibe's settings tree, named by its own bytes.
fn identity_carrier() -> QwenCarrier {
    QwenCarrier {
        env: "QWEN_SYSTEM_IDENTITY_MD".to_owned(),
        path: PathBuf::from("/h/.vibe/agent/qwen/qwen-identity-interactive-0123456789abcdef.md"),
        mode: QwenMode::Interactive,
        styled: false,
        identity_bytes: 211,
        carrier_bytes: 202_562,
        version: QWEN_MEASURED_VERSION.to_owned(),
    }
}

fn flags(kind: HarnessKind, delivery: HarnessDelivery) -> LaunchFlags {
    launch_flags(kind, delivery, &inputs(None))
}

/// A carrier document shaped like the one a composed OpenCode launch hands
/// over: the base, the separator and the lane's path in a file token.
fn carrier_document() -> CarrierDocument {
    CarrierDocument {
        env: "OPENCODE_CONFIG_CONTENT".to_owned(),
        value: format!(
            "{{\"default_agent\":\"vibe\",\"agents\":{{\"vibe\":{{\"mode\":\"primary\",\
             \"system\":\"BASE\\n\\n{{file:{LANE_PATH}}}\"}}}}}}"
        ),
        flags: vec!["--standalone".to_owned()],
        agent_name: "vibe".to_owned(),
    }
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
        &LaunchInputs {
            node_id: "org.vibevm/demo",
            lane: Path::new("/w/boot/AGENT-LANE.md"),
            lane_bytes: LANE,
            codex_limit: 262_144,
            supplied: None,
            qwen: None,
        },
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
fn opencode_in_read_delivery_passes_nothing_and_names_the_carrier_a_launch_would_use() {
    // **The question a note answers does not depend on the tier.** A
    // `read`-delivery OpenCode receives no flag and no assignment, and the
    // note still says where the lane *would* have gone, because that is
    // what a reader who is choosing a tier needs (`#SYS-OPENCODE`).
    let answer = flags(HarnessKind::OpenCode, HarnessDelivery::Read);
    assert!(answer.flags.is_empty());
    assert!(answer.env.is_empty());
    assert!(answer.profile.is_none());
    let said = answer.notes.join(" ");
    assert!(said.contains("OPENCODE_CONFIG_CONTENT"), "{said}");
    assert!(said.contains("default_agent"), "{said}");
    assert!(said.contains("--standalone"), "{said}");
    assert!(said.contains(OPENCODE_MEASURED_VERSION), "{said}");
    // A path, never the lane's own bytes: the document names the file.
    assert!(said.contains(LANE_PATH), "{said}");
    assert!(!said.contains("<!-- lane -->"), "{said}");
}

#[test]
fn opencode_in_launch_delivery_carries_the_document_in_the_environment() {
    let document = carrier_document();
    let answer = launch_flags(
        HarnessKind::OpenCode,
        HarnessDelivery::Launch,
        &inputs(Some(&document)),
    );
    // `--standalone` is the document's own, because whether the carrier is
    // readable at all is a fact about the harness version.
    assert_eq!(answer.flags, ["--standalone"]);
    assert!(answer.profile.is_none());
    assert_eq!(answer.env.len(), 1);
    assert_eq!(answer.env[0].name, "OPENCODE_CONFIG_CONTENT");
    assert_eq!(answer.env[0].value, document.value);
    // **The lane's path, never its bytes — in the environment as on a
    // command line.** An environment string is capped the same way an
    // argument is, and `#SYS-HARNESS-DELIVERY` is a rule rather than a
    // threshold.
    assert!(answer.env[0].value.contains(LANE_PATH), "{:?}", answer.env);
    assert!(!answer.env[0].value.contains("<!-- lane -->"));
    let said = answer.notes.join(" ");
    assert!(said.contains("vibe"), "{said}");
    assert!(said.contains("--standalone"), "{said}");
    assert!(said.contains("subagent"), "{said}");
}

#[test]
fn opencode_with_no_composed_carrier_passes_nothing_at_all() {
    // The gate above this cell has already turned the refusal into a soft
    // launch; what must not happen is a flag with no document behind it.
    let answer = launch_flags(
        HarnessKind::OpenCode,
        HarnessDelivery::Launch,
        &inputs(None),
    );
    assert!(answer.flags.is_empty());
    assert!(answer.env.is_empty());
}

#[test]
fn a_users_own_agent_argument_is_a_conflict_because_it_bypasses_ours() {
    // Not a second prompt but a bypass of the first: `--agent other`
    // selects an agent whose `system` is not the one carrying the lane, and
    // nothing reports it.
    assert_eq!(HarnessKind::OpenCode.conflicting_flags(), ["--agent"]);
    assert!(HarnessKind::Qwen.conflicting_flags().is_empty());
}

#[test]
fn only_a_harness_whose_slot_replaces_its_prompt_names_a_base_package() {
    // The coordinate is packaging, not prompt text, and it exists only for
    // the kind whose slot replaces the harness's own prompt.
    assert_eq!(
        HarnessKind::OpenCode.base_package(),
        Some(("org.vibevm.opencode", "opencode"))
    );
    for kind in [HarnessKind::Claude, HarnessKind::Codex, HarnessKind::Qwen] {
        assert_eq!(kind.base_package(), None, "{kind:?}");
    }
}

#[test]
fn qwen_delivers_one_environment_path_and_never_a_flag() {
    // **The carrier is an environment path and there are no flags at all.**
    // Qwen Code reads the variable on its own prompt path, so nothing has to
    // be passed to make it reachable — which is also why this kind's
    // `conflicting_flags` is empty even under `launch` (`#SYS-QWEN`).
    let carrier = identity_carrier();
    let answer = launch_flags(
        HarnessKind::Qwen,
        HarnessDelivery::Launch,
        &LaunchInputs {
            qwen: Some(&carrier),
            ..inputs(None)
        },
    );
    assert!(answer.flags.is_empty());
    assert!(answer.profile.is_none());
    assert_eq!(
        answer.env,
        [EnvAssignment {
            name: "QWEN_SYSTEM_IDENTITY_MD".to_owned(),
            value: "/h/.vibe/agent/qwen/qwen-identity-interactive-0123456789abcdef.md".to_owned(),
        }]
    );
    let said = answer.notes.join(" ");
    // The value is a PATH, and the note says which file and what is in it.
    assert!(said.contains("QWEN_SYSTEM_IDENTITY_MD"), "{said}");
    assert!(said.contains("FIRST PARAGRAPH"), "{said}");
    assert!(said.contains("sampled at this launch"), "{said}");
    assert!(said.contains(LANE_PATH), "{said}");
    // And never the lane's own bytes.
    assert!(!said.contains("<!-- lane -->"), "{said}");
}

#[test]
fn qwen_names_its_carrier_and_its_closed_slots_under_either_delivery() {
    // **Printed under both deliveries**, because the question it answers —
    // where would this harness's lane have gone — does not depend on the
    // tier the project declared. With no composed carrier (a `read`
    // project, or a `launch` the gate refused) there is no assignment and
    // the note is the whole answer.
    for delivery in [HarnessDelivery::Read, HarnessDelivery::Launch] {
        let answer = flags(HarnessKind::Qwen, delivery);
        assert!(answer.flags.is_empty(), "{delivery:?}");
        assert!(answer.env.is_empty(), "{delivery:?}");
        assert!(answer.profile.is_none(), "{delivery:?}");
        let said = answer.notes.join(" ");
        assert!(said.contains("QWEN_SYSTEM_IDENTITY_MD"), "{said}");
        assert!(said.contains("--append-system-prompt"), "{said}");
        assert!(said.contains("QWEN_SYSTEM_MD"), "{said}");
        assert!(said.contains("128 KiB"), "{said}");
        assert!(said.contains(QWEN_MEASURED_VERSION), "{said}");
        assert!(said.contains(LANE_PATH), "{said}");
        assert!(!said.contains("<!-- lane -->"), "{said}");
    }
}

#[test]
fn the_qwen_carrier_description_names_the_slot_and_why_it_is_clean() {
    let said = qwen_carrier_description();
    assert!(said.contains("QWEN_SYSTEM_IDENTITY_MD"), "{said}");
    // The two properties that make it admissible where `QWEN_SYSTEM_MD` was
    // not: it replaces one paragraph, and the sentence is sampled rather
    // than stored.
    assert!(said.contains("FIRST PARAGRAPH"), "{said}");
    assert!(said.contains("never stored"), "{said}");
    assert!(said.contains("getCoreSystemPrompt"), "{said}");
    assert!(said.contains("INSTALLED"), "{said}");
    // No lane byte on a command line or in an environment value, which is
    // the standing rule rather than this kind's own.
    assert!(said.contains("command line"), "{said}");
    assert!(said.contains("subagent"), "{said}");
    assert!(said.contains(QWEN_MEASURED_VERSION), "{said}");
}

#[test]
fn the_qwen_closed_slots_name_both_and_what_closes_each() {
    // **Two slots, two different reasons, and the sentence carries both.**
    // Stopping at the additive one would leave a reader believing Qwen Code
    // has no file-based slot, which is false and is the more interesting
    // half: it exists, it is subagent-safe, and what closes it is that it
    // REPLACES a prompt whose bytes depend on the session (`#SYS-QWEN`).
    let closed = qwen_closed_slots();
    // Worker exclusion is satisfied here, unlike `opencode` — said out loud
    // so a reader does not carry that kind's conclusion over.
    assert!(closed.contains("worker exclusion is satisfied"), "{closed}");
    assert!(closed.contains("E2BIG"), "{closed}");
    assert!(closed.contains("202,349"), "{closed}");
    assert!(closed.contains("QWEN_SYSTEM_MD"), "{closed}");
    // The measurement that closes the whole-prompt slot: one prompt, two
    // sizes, decided by something a launcher cannot observe.
    assert!(closed.contains("13,856"), "{closed}");
    assert!(closed.contains("18,451"), "{closed}");
    assert!(closed.contains(QWEN_MEASURED_VERSION), "{closed}");
}

#[test]
fn the_opencode_carrier_description_names_the_slot_the_base_and_what_is_closed() {
    let said = opencode_carrier_description();
    // The slot that works, and the cost that makes a supplied base
    // necessary at all.
    assert!(said.contains("OPENCODE_CONFIG_CONTENT"), "{said}");
    assert!(said.contains("default_agent"), "{said}");
    assert!(said.contains("REPLACES"), "{said}");
    assert!(said.contains("--standalone"), "{said}");
    assert!(said.contains("background service"), "{said}");
    // The carrier that is closed for good, and the rule that closes it.
    assert!(said.contains("SYS-WORKER-EXCLUSION"), "{said}");
    assert!(said.contains("instructions"), "{said}");
    assert!(said.contains(OPENCODE_MEASURED_VERSION), "{said}");
    // The base is data in the harness's own package, never vibe's source.
    assert!(said.contains("versioned data with provenance"), "{said}");
}

#[test]
fn a_version_line_yields_its_number_or_nothing() {
    assert_eq!(
        reported_version("opencode v2.2.7").as_deref(),
        Some("2.2.7")
    );
    assert_eq!(
        reported_version("codex-cli 0.167.0").as_deref(),
        Some("0.167.0")
    );
    assert_eq!(reported_version("no number here"), None);
    assert_eq!(reported_version(""), None);
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
    // **The table's own per-kind default, not one default for every kind.**
    // Reading `launch` for everything was harmless while the set held only
    // the two file-slot harnesses, and wrong the moment it did not: `qwen`
    // would have read `launch` from a project that declared nothing at all.
    // `opencode` joined the `launch` side on 2026-11-11, once its supplied
    // base earned `verified` by the whole-request proof, and `qwen` the same
    // day once its identity carrier was measured — and a machine whose
    // installed harness is another version soft-launches, so the default
    // costs nothing where the carrier cannot be proved
    // (`#SYS-SUPPLIED-BASE`, `#SYS-QWEN`, `#SYS-SOFT-LAUNCH`). The per-kind
    // function stays, because the next kind may well not have a tier.
    let manifest = vibe_core::manifest::Manifest::default();
    assert!(manifest.system_prompt.is_none());
    for kind in HarnessKind::ALL {
        assert_eq!(
            kind.delivery(&manifest),
            HarnessDelivery::Launch,
            "{kind:?}"
        );
    }
}
