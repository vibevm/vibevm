//! The sidecar: what it holds, the four things it must not hold, and the
//! golden document the format registry points at.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SIDECAR",
    r = 1
);

use vibe_core::manifest::{Manifest, TargetOs, WhenCondition};

use super::*;
use crate::system_prompt::{
    CapturedInput, CapturedLane, ContinuationRow, PrefixSettings, compose_prefix,
};

/// The corpus document `[format.boot-system-meta]` names. It is the sidecar
/// of the fixture below, byte for byte, so the writer emits the corpus bytes
/// and the generated reader round-trips them — the wire-parity oracle an
/// authored golden corpus is required to have.
const GOLDEN: &str =
    include_str!("../../../../../formats/corpora/boot-system-meta/e1/fixture.json");

const BLOCK: &str = "<vibevm>\ngenerated boot instructions\n</vibevm>\n";

fn input(role: InputRole, relative: &str, body: &str) -> CapturedInput {
    CapturedInput::new(role, relative.to_string(), body.as_bytes().to_vec())
}

fn fixture() -> CapturedPrefixInputs {
    CapturedPrefixInputs {
        node_rel: ".".to_string(),
        template: input(
            InputRole::Template,
            "vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md",
            "# My frame\n",
        ),
        entrypoint: Some(input(
            InputRole::Entrypoint,
            "AGENTS.md",
            &format!("# Agents\n\n{BLOCK}"),
        )),
        lane: Some(CapturedLane {
            spelling: LaneSpelling::StaticXml,
            input: input(
                InputRole::Lane,
                "vibevm/vibespecs/boot/STATIC.xml",
                "<spec>lane</spec>\n",
            ),
        }),
        plan: ContinuationPlan {
            rows: vec![
                ContinuationRow {
                    path: "vibevm/vibespecs/boot/00-core.xml".to_string(),
                    band: ContinuationBand::Unconditional,
                },
                ContinuationRow {
                    path: "vibevm/vibespecs/boot/70-win.xml".to_string(),
                    band: ContinuationBand::Gated(WhenCondition::Os(TargetOs::Windows)),
                },
            ],
        },
    }
}

fn settings(extra: &str) -> PrefixSettings {
    let manifest = Manifest::parse_str(&format!(
        "[project]\nname = \"demo\"\ngroup = \"org.vibevm.core\"\nversion = \"0.1.0\"\n{extra}"
    ))
    .expect("the fixture manifest parses");
    PrefixSettings::of(&manifest).expect("settings")
}

fn sidecar(extra: &str) -> wire::SystemPromptMeta {
    let inputs = fixture();
    let settings = settings(extra);
    let prefix = compose_prefix(&inputs, &settings).expect("composes");
    let plan = inputs.plan.resolve_host(settings.host);
    compose_sidecar(
        &inputs,
        &settings,
        &prefix,
        SidecarFacts {
            plan: &plan,
            index_bytes: Some(b"schema = 1\n"),
        },
    )
    .expect("the sidecar builds")
}

#[test]
fn the_fixtures_sidecar_is_the_registered_corpus_document_byte_for_byte() {
    let bytes = render_sidecar_bytes(&sidecar("")).expect("renders");
    assert_eq!(
        String::from_utf8(bytes).expect("UTF-8"),
        GOLDEN,
        "regenerate formats/corpora/boot-system-meta/e1/fixture.json deliberately"
    );
    // And the generated reader round-trips the corpus bytes.
    let read: wire::SystemPromptMeta = serde_json::from_str(GOLDEN).expect("the reader accepts it");
    assert_eq!(read, sidecar(""));
}

#[test]
fn it_holds_the_two_identities_and_the_plan_but_never_the_continuations_bytes() {
    let meta = sidecar("");
    assert_eq!(meta.protocol, SIDECAR_PROTOCOL);
    assert_eq!(meta.layer, wire::Layer::Project);
    assert_eq!(meta.node, ".");
    assert_eq!(meta.node_id, "org.vibevm.core/demo");
    assert_eq!(meta.entrypoint.as_deref(), Some("AGENTS.md"));
    assert_eq!(meta.lane, wire::LaneSpelling::StaticXml);
    assert_eq!(meta.recipe, 1);
    assert_eq!(meta.inputs.len(), 3, "template, entrypoint, lane");
    assert_eq!(meta.inputs[0].role, wire::Role::Template);
    assert_eq!(meta.inputs[0].len, "# My frame\n".len() as u32);
    assert_eq!(meta.prefix_sha256.len(), 64);
    assert_eq!(meta.segments.len(), 3);
    assert_eq!(
        meta.index_sha256.as_deref(),
        Some(&*sha256_hex(b"schema = 1\n"))
    );
    assert_eq!(meta.continuation.len(), 2);
    assert_eq!(meta.continuation[1].band, wire::Band::Gated);
    assert_eq!(meta.continuation[1].when.as_deref(), Some("os:windows"));
}

/// The rule a future contributor will want to break, because a digest here
/// would make `check` strictly stronger — and would also dirty a generated
/// file on every edit of `90-user.xml`, destroying the one property dynamic
/// linking exists for (`#SYS-CONTINUATION-WITNESS-IS-RUNTIME`).
#[test]
fn no_continuation_row_carries_a_content_digest() {
    let rendered = String::from_utf8(render_sidecar_bytes(&sidecar("")).unwrap()).unwrap();
    let continuation = rendered
        .split("\"continuation\"")
        .nth(1)
        .expect("the continuation block");
    for forbidden in ["sha256", "hash", "len", "digest"] {
        assert!(
            !continuation.contains(forbidden),
            "a continuation row must carry no `{forbidden}`"
        );
    }
}

#[test]
fn it_holds_no_timestamp_no_absolute_path_and_no_host_observation() {
    let rendered = String::from_utf8(render_sidecar_bytes(&sidecar("")).unwrap()).unwrap();
    for forbidden in [
        "202",
        "/home/",
        "/Users/",
        "C:\\",
        "built_at",
        "generated_at",
    ] {
        assert!(
            !rendered.contains(forbidden),
            "the sidecar carries `{forbidden}`"
        );
    }
    // `host` is present, but it is the DECLARED value, not an observation:
    // absent from the manifest it is `portable` on every machine.
    assert_eq!(sidecar("").host, wire::Host::Portable);
}

#[test]
fn a_fixed_host_records_itself_and_leaves_the_gated_band_empty() {
    let meta = sidecar("[system_prompt]\nhost = \"linux\"\n");
    assert_eq!(meta.host, wire::Host::Linux);
    assert_eq!(
        meta.continuation
            .iter()
            .map(|row| row.path.as_str())
            .collect::<Vec<_>>(),
        ["vibevm/vibespecs/boot/00-core.xml"],
        "the Windows row is dropped at build time"
    );
    assert!(
        meta.continuation
            .iter()
            .all(|row| row.band == wire::Band::Unconditional)
    );
    assert!(meta.continuation.iter().all(|row| row.when.is_none()));
}

#[test]
fn the_declared_target_is_recorded_in_the_authors_own_order() {
    let every = sidecar("");
    assert_eq!(
        every.target,
        vec![wire::Os::Windows, wire::Os::Macos, wire::Os::Linux],
        "an absent `target` is every OS, in the closed set's own order"
    );
    let chosen = sidecar("[system_prompt]\ntarget = [\"windows\", \"linux\"]\n");
    assert_eq!(chosen.target, vec![wire::Os::Windows, wire::Os::Linux]);
}

#[test]
fn a_node_with_no_lane_records_the_absence_as_a_spelling() {
    let mut inputs = fixture();
    inputs.lane = None;
    let settings = settings("");
    let prefix = compose_prefix(&inputs, &settings).expect("composes");
    let meta = compose_sidecar(
        &inputs,
        &settings,
        &prefix,
        SidecarFacts {
            plan: &inputs.plan,
            index_bytes: None,
        },
    )
    .expect("builds");
    assert_eq!(meta.lane, wire::LaneSpelling::None);
    assert_eq!(meta.inputs.len(), 2);
    assert!(meta.index_sha256.is_none());
}

#[test]
fn entrypoint_false_is_an_absent_member_rather_than_an_empty_string() {
    let mut inputs = fixture();
    inputs.entrypoint = None;
    let settings = settings("");
    let prefix = compose_prefix(&inputs, &settings).expect("composes");
    let meta = compose_sidecar(
        &inputs,
        &settings,
        &prefix,
        SidecarFacts {
            plan: &inputs.plan,
            index_bytes: None,
        },
    )
    .expect("builds");
    assert!(meta.entrypoint.is_none());
    let rendered = String::from_utf8(render_sidecar_bytes(&meta).unwrap()).unwrap();
    assert!(!rendered.contains("\"entrypoint\""));
}

#[test]
fn the_segment_map_addresses_the_composed_bytes_it_describes() {
    let inputs = fixture();
    let settings = settings("");
    let prefix = compose_prefix(&inputs, &settings).expect("composes");
    let meta = compose_sidecar(
        &inputs,
        &settings,
        &prefix,
        SidecarFacts {
            plan: &inputs.plan,
            index_bytes: None,
        },
    )
    .expect("builds");
    assert_eq!(meta.prefix_len as usize, prefix.bytes.len());
    for (declared, segment) in meta.segments.iter().zip(&prefix.segments) {
        assert_eq!(declared.id, segment.id.as_str());
        let slice = &prefix.bytes[declared.offset as usize..][..declared.len as usize];
        assert_eq!(slice.len(), segment.len);
    }
    assert!(meta.segments.last().expect("a segment").cache_boundary);
}

#[test]
fn the_same_inputs_give_a_byte_identical_sidecar() {
    assert_eq!(
        render_sidecar_bytes(&sidecar("")).unwrap(),
        render_sidecar_bytes(&sidecar("")).unwrap()
    );
}
