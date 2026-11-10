//! The bands in order, each exactly once, and the bytes that were measured.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-REQUEST-ENVELOPE",
    r = 1
);

use vibe_workspace::system_prompt::sha256_hex;

use super::*;

/// A prepared request over files this test writes, so the envelope is built
/// from real bytes and real digests rather than from a stub.
struct Fixture {
    root: tempfile::TempDir,
    prepared: PreparedRequest,
}

impl Fixture {
    fn new(prefix: &str, unconditional: &[(&str, &str)], gated: &[(&str, &str)]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let prefix_path = root.path().join("SYSTEM.md");
        std::fs::write(&prefix_path, prefix).unwrap();
        let mut prepared = PreparedRequest {
            prefix: band("SYSTEM.md", prefix),
            unconditional: Vec::new(),
            gated: Vec::new(),
            unresolved: Vec::new(),
            cache_boundaries: vec![prefix.len()],
        };
        for (name, body) in unconditional {
            std::fs::write(root.path().join(name), body).unwrap();
            prepared.unconditional.push(band(name, body));
        }
        for (name, body) in gated {
            std::fs::write(root.path().join(name), body).unwrap();
            prepared.gated.push(band(name, body));
        }
        let band_u: usize = prepared.unconditional.iter().map(|member| member.len).sum();
        if band_u > 0 {
            prepared.cache_boundaries.push(prefix.len() + band_u);
        }
        Self { root, prepared }
    }

    fn build(&self, task: Option<&str>) -> anyhow::Result<RequestEnvelope> {
        RequestEnvelope::build(
            &self.prepared,
            &self.root.path().join("SYSTEM.md"),
            self.root.path(),
            task,
        )
    }
}

fn band(path: &str, body: &str) -> PreparedBand {
    PreparedBand {
        path: path.to_owned(),
        sha256: sha256_hex(body.as_bytes()),
        len: body.len(),
    }
}

#[test]
fn the_bands_are_p_then_u_then_g_then_t_and_the_tool_block_precedes_them_all() {
    let fixture = Fixture::new(
        "PREFIX",
        &[("00-core.md", "CORE"), ("90-user.md", "USER")],
        &[("os-linux.md", "LINUX")],
    );
    let envelope = fixture.build(Some("do the thing")).unwrap();
    assert_eq!(
        envelope
            .instruction_blocks
            .iter()
            .map(|block| (block.band.as_str(), block.path.as_str()))
            .collect::<Vec<_>>(),
        [
            ("P", "SYSTEM.md"),
            ("U", "00-core.md"),
            ("U", "90-user.md"),
            ("G", "os-linux.md"),
        ]
    );
    // Band T is last, and it is a message rather than an instruction: the
    // caller's task never acquires instruction-role authority.
    assert_eq!(envelope.messages.len(), 1);
    assert_eq!(envelope.messages[0].band, Band::Task);
    assert_eq!(envelope.messages[0].role, Role::Message);
    for block in &envelope.instruction_blocks {
        assert_eq!(block.role, Role::Instruction, "{}", block.path);
    }
    // The tool block's place is stated, not left open: a provider's cache
    // hierarchy puts tools ahead of the instructions.
    assert_eq!(TOOLS_POSITION, "before_instructions");
    assert!(envelope.tools.is_empty(), "vibe contributes no tools");
}

#[test]
fn every_offset_is_the_sum_of_the_blocks_before_it() {
    let fixture = Fixture::new("PREFIX", &[("a.md", "AAAA"), ("b.md", "BB")], &[]);
    let envelope = fixture.build(None).unwrap();
    let mut running = 0;
    for block in &envelope.instruction_blocks {
        assert_eq!(block.offset, running, "{}", block.path);
        running += block.text.len();
    }
    assert_eq!(envelope.instruction_len(), running);
    assert_eq!(running, "PREFIX".len() + 4 + 2);
}

#[test]
fn a_declared_cache_boundary_lands_at_a_block_edge() {
    let fixture = Fixture::new("PREFIX", &[("a.md", "AAAA")], &[]);
    let envelope = fixture.build(None).unwrap();
    let edges: Vec<usize> = std::iter::once(0)
        .chain(
            envelope
                .instruction_blocks
                .iter()
                .map(|block| block.offset + block.text.len()),
        )
        .collect();
    assert!(!envelope.cache_boundaries.is_empty());
    for boundary in &envelope.cache_boundaries {
        assert!(
            edges.contains(boundary),
            "boundary {boundary} is inside a block; edges are {edges:?}"
        );
        assert!(*boundary <= envelope.instruction_len());
    }
    // The last boundary is the end of band U, which is the whole point of
    // keeping the continuation plan late-bound and still cacheable.
    assert_eq!(
        envelope.cache_boundaries.last().copied(),
        Some(envelope.instruction_len())
    );
}

#[test]
fn the_entrypoint_text_inside_the_prefix_is_carried_exactly_once() {
    // The prefix already contains the entrypoint. An adapter that also put
    // the entrypoint in the first message would double the cost this
    // feature exists to reduce — so the envelope's only message content is
    // the caller's own band T.
    let fixture = Fixture::new("frame\nENTRYPOINT-CANARY\nlane", &[("a.md", "AAAA")], &[]);
    let envelope = fixture.build(Some("the task")).unwrap();
    let whole: String = envelope
        .instruction_blocks
        .iter()
        .map(|block| block.text.as_str())
        .chain(
            envelope
                .messages
                .iter()
                .map(|message| message.text.as_str()),
        )
        .collect();
    assert_eq!(whole.matches("ENTRYPOINT-CANARY").count(), 1);
    assert_eq!(whole.matches("frame").count(), 1);
}

#[test]
fn an_absent_task_leaves_the_slot_declared_and_empty() {
    let fixture = Fixture::new("PREFIX", &[], &[]);
    let envelope = fixture.build(None).unwrap();
    assert!(
        envelope.messages.is_empty(),
        "a placeholder band T was invented"
    );
}

#[test]
fn a_band_whose_bytes_changed_since_preparation_is_refused_not_sent() {
    let fixture = Fixture::new("PREFIX", &[("a.md", "AAAA")], &[]);
    std::fs::write(fixture.root.path().join("a.md"), "EDITED").unwrap();
    let error = fixture
        .build(None)
        .expect_err("the digest no longer matches");
    assert!(error.to_string().contains("changed between"), "{error:#}");
    assert!(
        error
            .to_string()
            .contains("bytes sent must be the bytes measured"),
        "{error:#}"
    );
}

#[test]
fn a_missing_band_member_is_refused_naming_the_file() {
    let fixture = Fixture::new("PREFIX", &[("a.md", "AAAA")], &[]);
    std::fs::remove_file(fixture.root.path().join("a.md")).unwrap();
    let error = fixture.build(None).expect_err("the file is gone");
    assert!(error.to_string().contains("a.md"), "{error:#}");
}
