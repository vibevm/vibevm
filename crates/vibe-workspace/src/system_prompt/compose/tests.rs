//! The composer's contract: purity, determinism, framing, offsets, and the
//! size of the thing on this very repository.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-DETERMINISM",
    r = 1
);

use std::path::PathBuf;

use vibe_core::manifest::{Manifest, TargetOs};

use super::*;
use crate::system_prompt::host_repo::{index_rows, repo_root};
use crate::system_prompt::{
    CapturedInput, CapturedLane, ContinuationPlan, InputRole, LaneSpelling,
};

// --- fixtures -------------------------------------------------------------

const BLOCK: &str = "<vibevm>\ngenerated boot instructions\n</vibevm>\n";

fn manifest(extra: &str) -> Manifest {
    Manifest::parse_str(&format!(
        "[project]\nname = \"demo\"\ngroup = \"org.vibevm.core\"\nversion = \"0.1.0\"\n{extra}"
    ))
    .expect("the fixture manifest parses")
}

fn settings() -> PrefixSettings {
    PrefixSettings::of(&manifest("")).expect("settings read off the manifest")
}

fn input(role: InputRole, relative: &str, body: &str) -> CapturedInput {
    CapturedInput::new(role, relative.to_string(), body.as_bytes().to_vec())
}

/// The ordinary case: a template, an entrypoint carrying a managed block,
/// and a compiled lane.
fn inputs(template: &str, entrypoint: &str, lane: Option<&str>) -> CapturedPrefixInputs {
    CapturedPrefixInputs {
        node_rel: ".".to_string(),
        template: input(
            InputRole::Template,
            "vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md",
            template,
        ),
        entrypoint: Some(input(InputRole::Entrypoint, "AGENTS.md", entrypoint)),
        lane: lane.map(|bytes| CapturedLane {
            spelling: LaneSpelling::StaticXml,
            input: input(InputRole::Lane, "vibevm/vibespecs/boot/STATIC.xml", bytes),
        }),
        plan: ContinuationPlan::default(),
    }
}

fn compose(inputs: &CapturedPrefixInputs) -> ComposedPrefix {
    compose_prefix(inputs, &settings()).expect("the fixture composes")
}

fn text(prefix: &ComposedPrefix) -> String {
    String::from_utf8(prefix.bytes.clone()).expect("UTF-8")
}

// --- purity ---------------------------------------------------------------

/// The composer's own sources, named explicitly rather than discovered, so
/// that adding a file to the pure set is a reviewed decision.
const PURE_SOURCES: &[&str] = &[
    "compose.rs",
    "contract.rs",
    "lane.rs",
    "markers.rs",
    "meta.rs",
    "plan.rs",
    "verify.rs",
];

/// Each forbidden spelling with the capability it would smuggle in. The
/// claim `#SYS-DETERMINISM` and `#SYS-NO-VOLATILE` make is that composition
/// cannot observe anything, and a claim about capability is proved by
/// denying the capability rather than by inspecting one output.
const FORBIDDEN: &[(&str, &str)] = &[
    ("std::fs", "the filesystem"),
    ("std::net", "a socket"),
    ("std::env", "the ambient environment"),
    ("std::process", "a child process"),
    ("std::time", "the clock"),
    ("SystemTime", "the clock"),
    ("File::open", "the filesystem"),
    ("include_str!", "the filesystem at compile time"),
    ("current_dir", "the working directory"),
    ("cfg!(target_os", "the host operating system"),
];

#[test]
fn the_composer_reaches_no_file_no_socket_no_clock_and_no_environment() {
    let home = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/system_prompt");
    for name in PURE_SOURCES {
        let path = home.join(name);
        let source = std::fs::read_to_string(&path).expect("a composer source");
        for (spelling, capability) in FORBIDDEN {
            assert!(
                !source.contains(spelling),
                "{}: `{spelling}` would let composition observe {capability}",
                path.display()
            );
        }
    }
}

// --- determinism ----------------------------------------------------------

#[test]
fn determinism() {
    let fixture = inputs(
        "# frame\n",
        &format!("# Agents\n\n{BLOCK}tail\n"),
        Some("<spec/>\n"),
    );
    let first = compose(&fixture);
    let second = compose(&fixture);
    assert_eq!(
        first.bytes, second.bytes,
        "the same inputs give the same bytes"
    );
    assert_eq!(first.sha256, second.sha256);
    assert_eq!(first.segments, second.segments);
}

proptest::proptest! {
    /// Equal bytes and equal digests are the same fact, in both directions.
    ///
    /// `PLAN.md` predicted the stronger "no two distinct input sets collide
    /// on `prefix_sha256`", and that is **false by construction**: the
    /// framing inserts one LF after a body that lacks it, so a template `a`
    /// and a template `a\n` compose to the same bytes on purpose. The true
    /// property is the equivalence, and the normalisation is pinned by its
    /// own test.
    #[test]
    fn equal_bytes_and_equal_digests_are_one_fact(
        left in "[a-z\n #]{0,40}",
        right in "[a-z\n #]{0,40}",
    ) {
        let a = compose(&inputs(&left, &format!("x\n{BLOCK}"), Some("<spec/>\n")));
        let b = compose(&inputs(&right, &format!("x\n{BLOCK}"), Some("<spec/>\n")));
        proptest::prop_assert_eq!(a.bytes == b.bytes, a.sha256 == b.sha256);
        // Recomposing either one reproduces it exactly.
        let again = compose(&inputs(&left, &format!("x\n{BLOCK}"), Some("<spec/>\n")));
        proptest::prop_assert_eq!(a.bytes, again.bytes);
    }
}

#[test]
fn the_trailing_newline_normalisation_is_total_and_deliberate() {
    let with = compose(&inputs("a\n", &format!("x\n{BLOCK}"), None));
    let without = compose(&inputs("a", &format!("x\n{BLOCK}"), None));
    assert_eq!(with.bytes, without.bytes);
    assert_eq!(
        with.segments[0].len, 2,
        "the segment map records the body's own length, not the framed one"
    );
    assert_eq!(without.segments[0].len, 1);
}

// --- framing --------------------------------------------------------------

#[test]
fn a_crlf_body_keeps_crlf_inside_its_own_segment_while_frames_stay_lf() {
    let prefix = compose(&inputs(
        "line one\r\nline two\r\n",
        &format!("x\n{BLOCK}"),
        None,
    ));
    let body = &prefix.bytes[prefix.segments[0].offset..][..prefix.segments[0].len];
    assert_eq!(body, b"line one\r\nline two\r\n");
    let frame = format!("<!-- {SEGMENT_FRAME} project.template -->\n");
    assert!(text(&prefix).starts_with(&frame), "the frame line is LF");
    assert!(!frame.contains('\r'));
}

#[test]
fn the_bytes_are_utf8_with_no_bom_and_end_with_a_newline() {
    let prefix = compose(&inputs("# frame", &format!("x\n{BLOCK}"), Some("<spec/>")));
    assert!(!prefix.bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    assert!(prefix.bytes.ends_with(b"\n"));
    assert_eq!(
        prefix
            .bytes
            .iter()
            .rev()
            .take_while(|b| **b == b'\n')
            .count(),
        1,
        "exactly one newline is the composer's own"
    );
    String::from_utf8(prefix.bytes).expect("UTF-8");
}

#[test]
fn every_segment_offset_and_length_slices_exactly_its_own_body() {
    let template = "# frame\n";
    let entrypoint = format!("# Agents\n\npreamble\n\n{BLOCK}after the block\n");
    let lane = "<spec>lane</spec>\n";
    let prefix = compose(&inputs(template, &entrypoint, Some(lane)));
    let bytes = &prefix.bytes;
    let slice = |segment: &Segment| {
        std::str::from_utf8(&bytes[segment.offset..segment.offset + segment.len]).unwrap()
    };
    assert_eq!(prefix.segments.len(), 3);
    assert_eq!(slice(&prefix.segments[0]), template);
    assert_eq!(slice(&prefix.segments[2]), lane);
    let export = slice(&prefix.segments[1]);
    assert!(export.starts_with("# Agents\n\npreamble\n\n"), "{export}");
    assert!(export.ends_with("after the block\n"), "{export}");
    assert!(export.contains("# What this request already supplies"));
    assert!(
        !export.contains("generated boot instructions"),
        "the block is gone"
    );
}

#[test]
fn the_three_segments_are_in_band_order_and_the_boundaries_are_the_two_useful_ones() {
    let prefix = compose(&inputs("f\n", &format!("x\n{BLOCK}"), Some("<spec/>\n")));
    let ids: Vec<SegmentId> = prefix.segments.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        [SegmentId::Template, SegmentId::Entrypoint, SegmentId::Lane]
    );
    let boundaries: Vec<bool> = prefix.segments.iter().map(|s| s.cache_boundary).collect();
    assert_eq!(
        boundaries,
        [false, true, true],
        "one breakpoint before the lane, one at the end of band P"
    );
}

// --- volatility -----------------------------------------------------------

#[test]
fn the_bytes_carry_nothing_volatile() {
    let prefix = compose(&inputs(
        "# frame\n",
        &format!("x\n{BLOCK}"),
        Some("<spec/>\n"),
    ));
    let composed = text(&prefix);
    for forbidden in [
        "202", // a year, and so any timestamp
        "/home/", "/Users/", "C:\\", "HEAD", "linux", "windows", "macos",
    ] {
        assert!(
            !composed.contains(forbidden),
            "the prefix must carry no `{forbidden}`"
        );
    }
    // And no digest of itself: a digest before the content would make any
    // late change destroy the whole prefix's match.
    assert!(!composed.contains(&prefix.sha256[..12]));
}

// --- layering -------------------------------------------------------------

#[test]
fn the_project_layer_is_a_byte_identical_prefix_of_anything_appended_to_it() {
    let prefix = compose(&inputs(
        "# frame\n",
        &format!("x\n{BLOCK}"),
        Some("<spec/>\n"),
    ));
    let mut composite = prefix.bytes.clone();
    composite.extend_from_slice(b"<!-- vibe:prompt-segment user.template -->\nmine\n");
    assert!(composite.starts_with(&prefix.bytes));
    assert_eq!(&composite[..prefix.bytes.len()], &prefix.bytes[..]);
}

// --- the other shapes -----------------------------------------------------

#[test]
fn a_node_with_no_lane_composes_a_valid_two_segment_prefix() {
    let prefix = compose(&inputs("# frame\n", &format!("x\n{BLOCK}"), None));
    let ids: Vec<SegmentId> = prefix.segments.iter().map(|s| s.id).collect();
    assert_eq!(ids, [SegmentId::Template, SegmentId::Entrypoint]);
    assert_eq!(
        prefix
            .segments
            .iter()
            .map(|s| s.cache_boundary)
            .collect::<Vec<_>>(),
        [false, true]
    );
    assert!(!text(&prefix).contains("compiled static boot lane"));
}

#[test]
fn an_entrypoint_with_no_managed_block_keeps_its_bytes_whole_and_gets_its_own_contract_segment() {
    let prefix = compose(&inputs("f\n", "# Agents\nall mine\n", Some("<spec/>\n")));
    let ids: Vec<SegmentId> = prefix.segments.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        [
            SegmentId::Template,
            SegmentId::Entrypoint,
            SegmentId::Contract,
            SegmentId::Lane
        ]
    );
    let bytes = &prefix.bytes;
    let export = &bytes[prefix.segments[1].offset..][..prefix.segments[1].len];
    assert_eq!(export, b"# Agents\nall mine\n");
}

#[test]
fn entrypoint_false_still_accounts_for_the_layer_through_its_own_contract_segment() {
    let mut fixture = inputs("f\n", "unused", Some("<spec/>\n"));
    fixture.entrypoint = None;
    let prefix = compose(&fixture);
    let ids: Vec<SegmentId> = prefix.segments.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        [SegmentId::Template, SegmentId::Contract, SegmentId::Lane]
    );
    assert!(!text(&prefix).contains("project entrypoint"));
}

#[test]
fn an_empty_template_opts_in_with_no_frame_of_its_own() {
    let prefix = compose(&inputs("", &format!("x\n{BLOCK}"), None));
    assert_eq!(prefix.segments[0].len, 0);
    assert!(text(&prefix).starts_with(&format!(
        "<!-- {SEGMENT_FRAME} project.template -->\n<!-- {SEGMENT_FRAME} project.entrypoint -->\n"
    )));
}

// --- refusals -------------------------------------------------------------

#[test]
fn a_malformed_managed_block_refuses_and_never_repairs_the_document() {
    let fixture = inputs("f\n", "x\n<vibevm>\nno close\n", None);
    let error = compose_prefix(&fixture, &settings()).expect_err("a malformed pair refuses");
    let message = error.to_string();
    assert!(message.contains("AGENTS.md"), "{message}");
    assert!(message.contains("malformed"), "{message}");
}

#[test]
fn a_non_utf8_body_refuses_and_names_its_role() {
    let mut fixture = inputs("f\n", &format!("x\n{BLOCK}"), None);
    fixture.template.bytes = vec![0xFF, 0xFE];
    let error = compose_prefix(&fixture, &settings()).expect_err("non-UTF-8 refuses");
    assert!(error.to_string().contains("the template input"), "{error}");
}

#[test]
fn a_node_with_no_declared_identity_refuses_before_anything_is_composed() {
    // A bare `[workspace]` carries a role and no identity, which is the
    // real shape this refusal exists for.
    let bare = Manifest::parse_str("[workspace]\nmembers = []\n")
        .expect("a workspace-only manifest parses");
    let error = PrefixSettings::of(&bare).expect_err("no identity refuses");
    let message = error.to_string();
    assert!(
        message.contains("neither `[project]` nor `[package]`"),
        "{message}"
    );
}

#[test]
fn the_settings_come_off_the_manifest_and_nowhere_else() {
    let chosen = PrefixSettings::of(&manifest(
        "[system_prompt]\nhost = \"linux\"\ntarget = [\"windows\"]\nrecipe = 1\n",
    ))
    .expect("settings read");
    assert_eq!(chosen.node_id, "org.vibevm.core/demo");
    assert_eq!(chosen.host.fixed(), Some(TargetOs::Linux));
    assert_eq!(chosen.target.platforms(), vec![TargetOs::Windows]);
    assert_eq!(chosen.recipe, 1);
    assert_eq!(settings().recipe, CURRENT_RECIPE);
}

// --- the size of it, on this repository -----------------------------------

/// Spike S1's measurement, as a standing check rather than a one-off note.
///
/// It asserts the **composition identity** — the prefix is exactly its three
/// bodies plus the framing the composer owns — and prints the sizes. It
/// deliberately does not pin a digest or a byte count: `AGENTS.md` and
/// `STATIC.xml` change with every resolution, so a pinned number would fail
/// on an unrelated commit and teach the next session to delete the test. The
/// recipe, which is the thing a change must not slip past unnoticed, is
/// pinned byte for byte in `contract/tests.rs` instead.
#[test]
fn the_host_repository_composes_to_the_sum_of_its_parts() {
    let root = repo_root();
    let entrypoint_path = root.join("AGENTS.md");
    let lane_path = root.join("vibevm/vibespecs/boot/STATIC.xml");
    if !entrypoint_path.is_file() || !lane_path.is_file() {
        eprintln!("not a host checkout — skipping the size measurement");
        return;
    }
    let entrypoint = std::fs::read_to_string(&entrypoint_path).expect("the entrypoint");
    let lane = std::fs::read_to_string(&lane_path).expect("the lane");
    let fixture = inputs("", &entrypoint, Some(&lane));
    let prefix = compose(&fixture);

    let framing: usize = prefix
        .segments
        .iter()
        .map(|segment| format!("<!-- {SEGMENT_FRAME} {} -->\n", segment.id.as_str()).len())
        .sum();
    let bodies: usize = prefix.segments.iter().map(|segment| segment.len).sum();
    assert_eq!(
        prefix.bytes.len(),
        bodies + framing,
        "every byte of the prefix is a body or a frame line"
    );
    let index_bytes: usize = index_rows(&root)
        .iter()
        .filter_map(|row| std::fs::metadata(root.join(row)).ok())
        .map(|meta| meta.len() as usize)
        .sum();
    let material = prefix.bytes.len() + index_bytes;
    eprintln!(
        "S1 on this repository: entrypoint {} B, lane {} B, prefix {} B ({} segments, {} B of \
         framing); band U {} B across {} files; boot material {} B, prefix share {:.2}%, lane \
         share {:.2}%",
        entrypoint.len(),
        lane.len(),
        prefix.bytes.len(),
        prefix.segments.len(),
        framing,
        index_bytes,
        index_rows(&root).len(),
        material,
        100.0 * prefix.bytes.len() as f64 / material as f64,
        100.0 * lane.len() as f64 / material as f64,
    );
    assert!(
        prefix.bytes.len() > 200_000,
        "the prefix of this repository is the reason the feature exists"
    );
    assert!(
        !prefix.inertness.is_inert(),
        "the lane carries @spec:// tokens"
    );
    eprintln!(
        "import-inertness: {} sites, {} outside a code fence; first: {:?}",
        prefix.inertness.len(),
        prefix.inertness.unfenced(),
        prefix
            .inertness
            .first(3)
            .iter()
            .map(|site| (site.segment.as_str(), site.line, site.token.as_str()))
            .collect::<Vec<_>>()
    );
}
