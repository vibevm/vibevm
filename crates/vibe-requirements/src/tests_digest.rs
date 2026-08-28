//! Digest recipe vectors: independent recomputation, pinned hex, and
//! labeled-framing sensitivity.

use sha2::{Digest as _, Sha256};
use vibe_facts::SourceFileWitness;
use vibe_wire::generated::requirements_report::RequirementSourceKind;

use crate::digest::{observation_id, scope_digest, source_result_digest};

/// The shared longhand frame — the same primitive spelled out so a
/// silent change in the library's framing moves the assert.
struct Manual(Sha256);

impl Manual {
    fn new(domain: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(domain);
        Self(h)
    }
    fn field(&mut self, label: &[u8], value: &[u8]) {
        self.0.update((label.len() as u64).to_be_bytes());
        self.0.update(label);
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }
    fn count(&mut self, label: &[u8], n: usize) {
        self.field(label, n.to_string().as_bytes());
    }
    fn finish(self) -> String {
        format!("sha256:{:x}", self.0.finalize())
    }
}

#[test]
fn recipe_one_source_digest_matches_the_independent_recompute_and_the_pin() {
    let documents = vec![
        SourceFileWitness::of("vibevm/vibespecs/A.md", b"# A\n"),
        SourceFileWitness::of("vibevm/vibespecs/B.md", b"# B\n\nmore bytes\n"),
    ];
    let library =
        source_result_digest(&RequirementSourceKind::Host, "org.example/demo", &documents);
    let mut m = Manual::new(b"vibe-requirements-source-digest\0epoch=1\0");
    m.field(b"kind", b"host");
    m.field(b"package", b"org.example/demo");
    m.count(b"document_count", 2);
    for witness in &documents {
        m.field(b"path", witness.path.as_bytes());
        m.field(b"bytes", witness.bytes.to_string().as_bytes());
        m.field(b"raw_sha256", witness.digest.as_bytes());
    }
    assert_eq!(library, m.finish(), "library framing == longhand framing");
    // The published vector: same inputs, one fixed answer.
    assert_eq!(
        library,
        // Pinned 2026-08-28 (labeled-count framing); regenerate only
        // beside a deliberate recipe change.
        "sha256:adbb78b15b7aae586a01294218dbd956e833f535a8e15efc3996510fe3196be7",
        "the pinned source-digest vector moved"
    );
    // Distinct kind/package change the digest: the framing is
    // domain-separated, not a bare content hash.
    assert_ne!(
        library,
        source_result_digest(
            &RequirementSourceKind::Package,
            "org.example/demo",
            &documents
        )
    );
    assert_ne!(
        library,
        source_result_digest(
            &RequirementSourceKind::Host,
            "org.example/other",
            &documents
        )
    );
    // The canonical empty-source digest exists and is stable.
    let empty = source_result_digest(&RequirementSourceKind::Host, "org.example/demo", &[]);
    let mut m = Manual::new(b"vibe-requirements-source-digest\0epoch=1\0");
    m.field(b"kind", b"host");
    m.field(b"package", b"org.example/demo");
    m.count(b"document_count", 0);
    assert_eq!(empty, m.finish());
    assert_ne!(empty, library);
}

#[test]
fn recipe_two_scope_digest_matches_the_independent_recompute_and_the_pin() {
    let sources = vec![
        (
            RequirementSourceKind::Host,
            "org.example/demo".to_string(),
            "sha256:{}".replace("{}", &"1".repeat(64)),
        ),
        (
            RequirementSourceKind::Package,
            "org.example/pkg".to_string(),
            "sha256:{}".replace("{}", &"2".repeat(64)),
        ),
    ];
    let registry = vec![SourceFileWitness::of(
        "vibefacts/org.example.pkg.toml",
        b"schema = 1\n",
    )];
    let library = scope_digest(".", &sources, &registry);

    let mut m = Manual::new(b"vibe-requirements-scope-digest\0epoch=1\0");
    m.field(b"selected", b".");
    m.count(b"source_count", 2);
    m.field(b"kind", b"host");
    m.field(b"package", b"org.example/demo");
    m.field(b"source_digest", sources[0].2.as_bytes());
    m.field(b"kind", b"package");
    m.field(b"package", b"org.example/pkg");
    m.field(b"source_digest", sources[1].2.as_bytes());
    m.count(b"registry_count", 1);
    m.field(b"path", b"vibefacts/org.example.pkg.toml");
    m.field(b"bytes", b"11");
    m.field(b"raw_sha256", registry[0].digest.as_bytes());

    assert_eq!(library, m.finish(), "library framing == longhand framing");
    assert_eq!(
        library,
        // Pinned 2026-08-28 (labeled-count framing); regenerate only
        // beside a deliberate recipe change.
        "sha256:e323aa7ab55e9f3d260b655b527b3cebcb35482b359541260aaba1d1bbefd7b5",
        "the pinned scope-digest vector moved"
    );
    // The scope digest excludes the question — the recipe frames no
    // query member; pure function of its framed inputs.
    assert_eq!(library, scope_digest(".", &sources, &registry));
}

#[test]
fn recipe_three_observation_id_is_pinned_for_a_fixed_answer() {
    // The fixed fixture: host org.example/demo with one document and
    // one marked fact, no lock, no registry, no relations.
    let root = tempfile::TempDir::new().unwrap();
    std::fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let specs = root.path().join(vibe_core::layout::current_specs_root());
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("RULE.md"),
        "# Rules\n\n@fact:A a. @status:impl/done\n",
    )
    .unwrap();
    let context = crate::QueryContext {
        selected_root: root.path().to_path_buf(),
        observed_at: "2026-01-01T00:00:00Z".parse().unwrap(),
        lifecycle_run_id: None,
    };
    let report = crate::query(&crate::RequirementsQuery::default(), &context, None).unwrap();
    assert_eq!(
        report.observation.observation_id,
        // Pinned 2026-08-28 (labeled presence/count framing); regenerate
        // only beside a deliberate recipe change (clock exclusion is
        // pinned by the query-suite test).
        "sha256:46b8284fa7e0e87e094367f6801b2587ca57389f832a377f4a5ffa38e2ac4f46",
        "the pinned observation-id vector moved"
    );
    // And the same fixed answer, byte-stable across runs.
    let again = crate::query(&crate::RequirementsQuery::default(), &context, None).unwrap();
    assert_eq!(
        again.observation.observation_id,
        report.observation.observation_id
    );
}

#[test]
fn flipping_one_labeled_bit_or_truncated_moves_the_observation_id() {
    // C5's mutation: every optional member and the required `truncated`
    // are LABELED framed members — flipping any one moves the id, so no
    // bit can hide at an undocumented position.
    let root = tempfile::TempDir::new().unwrap();
    std::fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let specs = root.path().join(vibe_core::layout::current_specs_root());
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("RULE.md"),
        "# Rules\n\n@fact:A a. @status:impl/done\n",
    )
    .unwrap();
    let context = crate::QueryContext {
        selected_root: root.path().to_path_buf(),
        observed_at: "2026-01-01T00:00:00Z".parse().unwrap(),
        lifecycle_run_id: None,
    };
    let base = crate::query(&crate::RequirementsQuery::default(), &context, None).unwrap();
    let base_id = observation_id(&base);

    // The optional run-id presence bit.
    let mut with_run = base.clone();
    with_run.observation.lifecycle_run_id = Some("0".repeat(32));
    assert_ne!(observation_id(&with_run), base_id);

    // The required `truncated` member, framed as true|false.
    let mut flipped = base.clone();
    flipped.truncated = !flipped.truncated;
    assert_ne!(observation_id(&flipped), base_id);

    // An optional source member: adding a reason to the available host
    // moves the id through the labeled presence+value frames.
    let mut reasoned = base.clone();
    reasoned.sources[0].reason_code = Some("x".to_string());
    assert_ne!(observation_id(&reasoned), base_id);
}
