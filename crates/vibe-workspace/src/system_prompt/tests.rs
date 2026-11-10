//! What capture must witness, and what it must never open.
//!
//! The reads are asserted by **counting** them. A recording reader wraps the
//! seam capture observes inputs through and keeps every path asked for, so
//! `#SYS-NO-HOME-EXPANSION` is a mechanical claim rather than a promise: the
//! sentinel files exist in the fixture, and the assertion is that the read
//! list is exactly the roles' own paths.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CAPTURE-REUSE",
    r = 1
);

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::layout;
use vibe_core::manifest::{LinkType, Manifest, SpecFormat, TargetOs, WhenCondition};

use super::*;
use crate::WorkspaceError;
use crate::boot::{BootBand, BootEntry, EffectiveBoot};
use crate::boot_artifacts;

// --- the recording reader -------------------------------------------------

/// A reader over an in-memory node, recording every path asked for in order.
struct RecordingReader {
    files: BTreeMap<String, Vec<u8>>,
    reads: RefCell<Vec<String>>,
}

impl RecordingReader {
    fn new(files: &[(&str, &str)]) -> Self {
        Self {
            files: files
                .iter()
                .map(|(path, body)| ((*path).to_string(), body.as_bytes().to_vec()))
                .collect(),
            reads: RefCell::new(Vec::new()),
        }
    }

    fn reads(&self) -> Vec<String> {
        self.reads.borrow().clone()
    }
}

impl PrefixReader for RecordingReader {
    fn read(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
        self.reads.borrow_mut().push(relative.to_string());
        Ok(self.files.get(relative).cloned())
    }
}

fn template_rel() -> String {
    crate::path_to_slash(&layout::current_boot_system_template())
}

fn entry(path: &str, link: LinkType, when: Option<WhenCondition>) -> BootEntry {
    BootEntry {
        path: path.to_string(),
        band: BootBand::Dependency,
        link,
        when,
        origin: "org.vibevm/demo".to_string(),
        provenance: crate::boot::fixture_provenance("org.vibevm/demo"),
        use_ref: false,
        format: Default::default(),
        unit_substituted: false,
        elided: false,
    }
}

/// A node directory on disk, needed because `resolve_static_path` is the one
/// owner of the lane-spelling question and asks the filesystem.
fn node_dir() -> TempDir {
    TempDir::new().unwrap()
}

fn manifest(toml: &str) -> Manifest {
    Manifest::parse_str(toml).unwrap()
}

fn bare() -> Manifest {
    manifest("[project]\nname='demo'\nversion='0.1.0'\n")
}

fn request<'a>(
    dir: &'a Path,
    manifest: &'a Manifest,
    effective: &'a EffectiveBoot,
    lane: Option<&'a [u8]>,
) -> PrefixCapture<'a> {
    PrefixCapture {
        node_rel: ".",
        node_dir: dir,
        manifest,
        spec_format: SpecFormat::Xml,
        effective,
        lane,
        router: crate::boot_artifacts::RouterFacts {
            node_id: crate::boot_artifacts::router_node_id(
                Some(manifest),
                &vibe_spec::SelfCoordinate::new(None, String::new()),
            ),
            static_hash12: lane.map(super::segment_hash12),
            spec_format: SpecFormat::Xml,
        },
    }
}

// --- opt-in ---------------------------------------------------------------

#[test]
fn a_node_with_no_template_has_not_opted_in_and_nothing_else_is_read() {
    let dir = node_dir();
    let manifest = bare();
    let boot = EffectiveBoot::default();
    let reader = RecordingReader::new(&[("AGENTS.md", "# Mine\n")]);
    let captured =
        capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader).unwrap();
    assert!(captured.is_none(), "the template's presence is the opt-in");
    assert_eq!(
        reader.reads(),
        vec![template_rel()],
        "the opt-in question is asked first and nothing follows a `no`"
    );
}

// --- the prospective entrypoint (T3) -------------------------------------

/// The regression test for T3: the entrypoint witness is the bytes this
/// generation will leave in the file, not the bytes in it now. Witnessing
/// the pre-write file must fail this test, which is why the fixture's
/// entrypoint carries a stale managed block.
#[test]
fn the_entrypoint_witness_is_the_post_write_content_of_this_generation() {
    let dir = node_dir();
    let manifest = bare();
    let boot = EffectiveBoot::default();
    let before = "# Mine\n\nAuthored prose.\n\n<vibevm>\nSTALE BODY\n</vibevm>\n";
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n"), ("AGENTS.md", before)]);
    let captured = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .unwrap()
        .expect("the template is present");

    let entrypoint = captured.entrypoint.expect("AGENTS.md is the default");
    let witnessed = String::from_utf8(entrypoint.bytes.clone()).unwrap();
    assert!(
        !witnessed.contains("STALE BODY"),
        "the pre-write block must not be witnessed:\n{witnessed}"
    );
    assert!(
        witnessed.contains("Authored prose."),
        "every authored byte is preserved:\n{witnessed}"
    );

    // Said against the write path itself, so the two cannot drift: the
    // witness equals what the redirect write actually leaves behind, and
    // it is the *same* `RouterFacts` value that decides both — which is why
    // the facts travel in the capture request instead of being derived twice
    // (`#SYS-PROSPECTIVE-ENTRYPOINT`).
    let path = dir.path().join("AGENTS.md");
    fs::write(&path, before).unwrap();
    let request = request(dir.path(), &manifest, &boot, None);
    boot_artifacts::write_redirect_blocks_for(dir.path(), &request.router).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), witnessed);
    assert!(
        witnessed.contains("project demo"),
        "the witnessed block carries the node's declared identity:\n{witnessed}"
    );
    assert_eq!(entrypoint.role, InputRole::Entrypoint);
    assert_eq!(
        entrypoint.sha256,
        crate::vibedeps::sha256_bytes(&entrypoint.bytes)
    );
}

#[test]
fn a_declared_entrypoint_choice_selects_exactly_that_file() {
    let dir = node_dir();
    let manifest = manifest(
        "[project]\nname='demo'\nversion='0.1.0'\n[system_prompt]\nentrypoint = 'GEMINI.md'\n",
    );
    let boot = EffectiveBoot::default();
    let reader = RecordingReader::new(&[
        (&template_rel(), "Frame.\n"),
        ("AGENTS.md", "# never read\n"),
        ("GEMINI.md", "# chosen\n"),
    ]);
    let captured = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .unwrap()
        .unwrap();
    assert_eq!(captured.entrypoint.as_ref().unwrap().relative, "GEMINI.md");
    assert_eq!(
        reader.reads(),
        vec![template_rel(), "GEMINI.md".to_string()],
        "the other two instruction files are never opened"
    );
}

#[test]
fn entrypoint_false_captures_no_entrypoint_segment_and_reads_no_instruction_file() {
    let dir = node_dir();
    let manifest =
        manifest("[project]\nname='demo'\nversion='0.1.0'\n[system_prompt]\nentrypoint = false\n");
    let boot = EffectiveBoot::default();
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n"), ("AGENTS.md", "# Mine\n")]);
    let captured = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .unwrap()
        .unwrap();
    assert!(captured.entrypoint.is_none());
    assert_eq!(reader.reads(), vec![template_rel()]);
}

#[test]
fn a_declared_entrypoint_that_does_not_exist_refuses_naming_the_path() {
    let dir = node_dir();
    let manifest = bare();
    let boot = EffectiveBoot::default();
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n")]);
    let error = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .expect_err("a missing entrypoint is not a silent empty segment");
    let message = error.to_string();
    assert!(message.contains("AGENTS.md"), "{message}");
    assert!(message.contains("entrypoint = false"), "{message}");
}

#[test]
fn a_malformed_marker_pair_refuses_through_the_redirect_owner() {
    let dir = node_dir();
    let manifest = bare();
    let boot = EffectiveBoot::default();
    let reader = RecordingReader::new(&[
        (&template_rel(), "Frame.\n"),
        (
            "AGENTS.md",
            "<vibevm>\na\n</vibevm>\n<vibevm>\nb\n</vibevm>\n",
        ),
    ]);
    let error = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .expect_err("two marker pairs are malformed");
    assert!(
        matches!(error, WorkspaceError::MalformedRedirectBlock { .. }),
        "{error}"
    );
}

// --- the lane -------------------------------------------------------------

#[test]
fn a_node_with_no_static_lane_captures_no_lane_and_is_still_a_valid_input_set() {
    let dir = node_dir();
    let manifest = bare();
    let boot = EffectiveBoot {
        entries: vec![entry(
            "vibevm/vibespecs/boot/00-core.md",
            LinkType::Dynamic,
            None,
        )],
    };
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n"), ("AGENTS.md", "# Mine\n")]);
    let captured = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .unwrap()
        .unwrap();
    assert!(
        captured.lane.is_none(),
        "a first generation is not an error"
    );
    assert_eq!(captured.plan.rows.len(), 1);
    assert_eq!(captured.inputs().count(), 2, "template and entrypoint");
}

#[test]
fn the_lane_enters_as_the_published_spelling_and_its_bytes_are_opaque() {
    let dir = node_dir();
    let manifest = bare();
    let boot = EffectiveBoot::default();
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n"), ("AGENTS.md", "# Mine\n")]);
    let bytes = b"<spec>#use not followed</spec>\n";
    let captured =
        capture_prefix_inputs(request(dir.path(), &manifest, &boot, Some(bytes)), &reader)
            .unwrap()
            .unwrap();
    let lane = captured.lane.expect("lane bytes were supplied");
    assert_eq!(lane.spelling, LaneSpelling::StaticXml);
    assert_eq!(lane.input.bytes, bytes.to_vec());
    assert_eq!(lane.input.relative, "vibevm/vibespecs/boot/STATIC.xml");
    assert_eq!(
        reader.reads(),
        vec![template_rel(), "AGENTS.md".to_string()],
        "the lane is the caller's bytes, never a read of its own"
    );
}

#[test]
fn both_static_spellings_present_propagates_the_existing_refusal_unchanged() {
    let dir = node_dir();
    let boot_dir = dir.path().join(layout::current_boot_dir());
    fs::create_dir_all(&boot_dir).unwrap();
    fs::write(boot_dir.join(layout::STATIC_MD), "md").unwrap();
    fs::write(boot_dir.join(layout::STATIC_XML), "xml").unwrap();
    let manifest = bare();
    let boot = EffectiveBoot::default();
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n"), ("AGENTS.md", "# Mine\n")]);

    let direct = boot_artifacts::resolve_static_path(dir.path()).expect_err("both present");
    let through_capture =
        capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
            .expect_err("capture does not pick a side");
    assert_eq!(through_capture.to_string(), direct.to_string());
}

// --- the continuation plan ------------------------------------------------

/// The plan's order is asserted against `prepare_index_body` — the function
/// that renders the artifact — rather than against a string this test wrote,
/// so the two cannot agree by coincidence.
#[test]
fn the_plans_order_is_the_index_renderers_row_order() {
    let boot = EffectiveBoot {
        entries: vec![
            entry("a.md", LinkType::Dynamic, None),
            entry("static.md", LinkType::Static, None),
            entry(
                "b.md",
                LinkType::Dynamic,
                Some(WhenCondition::Os(TargetOs::Windows)),
            ),
            entry("c.md", LinkType::Dynamic, None),
        ],
    };
    let index = boot_artifacts::prepare_index_body(&boot, SpecFormat::Mixed).unwrap();
    let document: toml::Value = toml::from_str(&index).unwrap();
    let rendered: Vec<String> = document["entry"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["path"].as_str().unwrap().to_string())
        .collect();

    let plan = ContinuationPlan::of(&boot);
    let planned: Vec<String> = plan.rows.iter().map(|row| row.path.clone()).collect();
    assert_eq!(planned, rendered);
    assert!(
        !planned.contains(&"static.md".to_string()),
        "a static entry is lane material, never a continuation row"
    );
}

#[test]
fn a_gated_row_carries_its_condition_verbatim_and_an_unconditional_one_carries_none() {
    let when = WhenCondition::Os(TargetOs::Macos);
    let boot = EffectiveBoot {
        entries: vec![
            entry("plain.md", LinkType::Dynamic, None),
            entry("gated.md", LinkType::Dynamic, Some(when.clone())),
        ],
    };
    let plan = ContinuationPlan::of(&boot);
    assert_eq!(plan.rows[0].band, ContinuationBand::Unconditional);
    assert_eq!(plan.rows[1].band, ContinuationBand::Gated(when));
    assert_eq!(plan.unconditional().count(), 1);
    assert_eq!(
        plan.gated()
            .next()
            .unwrap()
            .band
            .when()
            .unwrap()
            .to_string(),
        "os:macos"
    );
}

/// `installed:` and `target:` are both decided before an INDEX row exists,
/// so neither can appear in a plan built from a composed effective boot.
/// The plan is built from what the generator produced, and the generator
/// produces only `os:` residuals (PROP-049 §3, PROP-076
/// `#SYS-HOST-AND-TARGET`).
#[test]
fn no_generation_time_predicate_reaches_the_plan() {
    let boot = EffectiveBoot {
        entries: vec![
            entry("resolved.md", LinkType::Dynamic, None),
            entry(
                "session.md",
                LinkType::Dynamic,
                Some(WhenCondition::Os(TargetOs::Linux)),
            ),
        ],
    };
    for row in &ContinuationPlan::of(&boot).rows {
        let Some(when) = row.band.when() else {
            continue;
        };
        assert_eq!(when.installed_identity(), None, "{when}");
        assert_eq!(when.target_os(), None, "{when}");
        assert!(when.session_os().is_some(), "{when}");
    }
}

// --- self-inclusion, from the generator's side ---------------------------

#[test]
fn an_output_among_its_own_inputs_refuses_in_any_case_spelling() {
    let dir = node_dir();
    let manifest = bare();
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n"), ("AGENTS.md", "# Mine\n")]);
    for name in [
        layout::SYSTEM_MD,
        layout::SYSTEM_META_JSON,
        layout::SYSTEM_TEMPLATE_MD,
        layout::INDEX_MD,
    ] {
        for spelling in [name.to_string(), name.to_ascii_lowercase()] {
            let row = crate::layout_paths::boot(&spelling);
            let boot = EffectiveBoot {
                entries: vec![entry(&row, LinkType::Dynamic, None)],
            };
            let error = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
                .unwrap_err();
            assert!(error.to_string().contains(&row), "{error}");
        }
    }
}

#[test]
fn a_dependency_slots_own_lane_is_not_a_self_inclusion() {
    // The test is a file identity inside *this* node's boot directory, not
    // a string: another node's generated lane is a legitimate unit lane.
    let dir = node_dir();
    let manifest = bare();
    let reader = RecordingReader::new(&[(&template_rel(), "Frame.\n"), ("AGENTS.md", "# Mine\n")]);
    let row = format!(
        "vibevm/vibedeps/org.vibevm.world.wal/1.0.0/{}",
        crate::layout_paths::boot(layout::STATIC_XML)
    );
    let boot = EffectiveBoot {
        entries: vec![entry(&row, LinkType::Dynamic, None)],
    };
    let captured = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .unwrap()
        .unwrap();
    assert_eq!(captured.plan.rows[0].path, row);
}

// --- the privacy sentinels ------------------------------------------------

/// The fixture's authored text mentions a stewardship directory, a token
/// path and a plan file. Capture must open none of them, however loudly a
/// paragraph names one: a mention is an instruction to a tool-using agent,
/// never permission to embed a file (`#SYS-NO-HOME-EXPANSION`).
#[test]
fn text_that_mentions_a_home_plan_or_token_path_leaves_all_three_unopened() {
    let dir = node_dir();
    let manifest = bare();
    let sentinels = [
        "~/.vibe/steward/contexts/demo/plan.toml",
        "~/.vibe/steward/contexts/demo/custody.toml",
        "~/.vibe/zai.api.token",
    ];
    let mentions = format!(
        "# Mine\n\nRead {}, then {}, and never echo {}.\n\n<vibevm>\nx\n</vibevm>\n",
        sentinels[0], sentinels[1], sentinels[2]
    );
    let template = format!("Resolve the context under {} first.\n", sentinels[0]);
    let reader = RecordingReader::new(&[(&template_rel(), &template), ("AGENTS.md", &mentions)]);
    let boot = EffectiveBoot::default();
    let captured = capture_prefix_inputs(request(dir.path(), &manifest, &boot, None), &reader)
        .unwrap()
        .unwrap();

    assert_eq!(
        reader.reads(),
        vec![template_rel(), "AGENTS.md".to_string()],
        "exactly the two authored roles, and nothing a paragraph named"
    );
    // The mentions themselves pass through verbatim: the generator may not
    // edit an author's words (`#SYS-NO-REWRITE-OF-AUTHORED-TEXT`).
    let entrypoint = String::from_utf8(captured.entrypoint.unwrap().bytes).unwrap();
    assert!(entrypoint.contains(sentinels[0]), "{entrypoint}");
}

// --- the bound reader, over a real held workspace ------------------------

#[test]
fn the_bound_reader_observes_a_prepared_node_and_answers_absence_with_none() {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='host'\nversion='1.0.0'\n",
    )
    .unwrap();
    let boot_dir = root.path().join(layout::current_boot_dir());
    fs::create_dir_all(&boot_dir).unwrap();
    fs::write(boot_dir.join(layout::SYSTEM_TEMPLATE_MD), "Frame.\n").unwrap();
    fs::write(root.path().join("AGENTS.md"), "# Mine\n").unwrap();

    let workspace = crate::Workspace::load(root.path()).unwrap();
    let sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
        &workspace,
        &[],
        Some(SpecFormat::Xml),
    )
    .unwrap()
    .expect("prepared with nodes");
    let reader = super::bound_reader(&sources, ".");
    assert_eq!(
        reader.read(&template_rel()).unwrap(),
        Some(b"Frame.\n".to_vec())
    );
    assert_eq!(
        reader.read("CLAUDE.md").unwrap(),
        None,
        "absence is an answer"
    );

    // And the whole capture over that held workspace, with no lane.
    let captured = capture_node_prefix_inputs(
        &workspace,
        ".",
        SpecFormat::Xml,
        &EffectiveBoot::default(),
        None,
    );
    let captured = match captured {
        Ok(captured) => captured.expect("the template is present"),
        Err(error) => panic!("{error}"),
    };
    assert_eq!(captured.node_rel, ".");
    assert_eq!(captured.template.bytes, b"Frame.\n".to_vec());
    assert!(captured.lane.is_none());
}
