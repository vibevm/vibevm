//! The user documentation, pinned where it can drift.
//!
//! `docs/system-prompt/` makes two kinds of claim a document cannot keep on
//! its own. The first is an **example**: a template a reader is told to copy
//! must be one the composer actually accepts, including the awkward part of
//! the claim — that `{braces}`, a fenced example and the literal string
//! `<vibevm>` are ordinary characters. The second is a **vocabulary**: the
//! invalidation table's verdict words are the `CheckVerdict` variants'
//! own spellings, and a renamed variant must fail here rather than leave the
//! table quietly wrong.
//!
//! Both tests skip outside a host checkout, because they read a file of the
//! repository rather than a fixture.

use std::path::PathBuf;

use vibe_core::manifest::Manifest;

use super::compose::{PrefixSettings, compose_prefix};
use super::host_repo::repo_root;
use super::{
    CapturedInput, CapturedLane, CapturedPrefixInputs, CheckVerdict, ContinuationPlan, InputRole,
    LaneSpelling, SegmentId,
};

/// The documented guide and the documented example template.
const GUIDE: &str = "docs/system-prompt/README.md";
const EXAMPLE: &str = "docs/system-prompt/SYSTEM-TEMPLATE.example.md";

fn documented(relative: &str) -> Option<String> {
    let path: PathBuf = repo_root().join(relative);
    std::fs::read_to_string(path).ok()
}

fn settings() -> PrefixSettings {
    let manifest = Manifest::parse_str(
        "[project]\nname = \"demo\"\ngroup = \"org.vibevm.core\"\nversion = \"0.1.0\"\n",
    )
    .expect("the fixture manifest parses");
    PrefixSettings::of(&manifest).expect("settings read off the manifest")
}

/// The documented example template composes, and its bytes arrive as the
/// template segment verbatim.
///
/// This is the test `PLAN.md` P8 asks for: a documentation example that
/// stopped working fails a test rather than misleading the next reader.
#[test]
fn the_documented_example_template_composes() {
    let Some(template) = documented(EXAMPLE) else {
        eprintln!("skipped: not a host checkout");
        return;
    };
    let inputs = CapturedPrefixInputs {
        node_rel: ".".to_owned(),
        template: CapturedInput::new(
            InputRole::Template,
            "vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md".to_owned(),
            template.clone().into_bytes(),
        ),
        entrypoint: Some(CapturedInput::new(
            InputRole::Entrypoint,
            "AGENTS.md".to_owned(),
            b"# demo\n\n<vibevm>\ngenerated boot instructions\n</vibevm>\n".to_vec(),
        )),
        lane: Some(CapturedLane {
            spelling: LaneSpelling::StaticXml,
            input: CapturedInput::new(
                InputRole::Lane,
                "vibevm/vibespecs/boot/STATIC.xml".to_owned(),
                b"<!-- vibe:c1 -->\n<spec/>\n".to_vec(),
            ),
        }),
        plan: ContinuationPlan::default(),
    };
    let prefix = compose_prefix(&inputs, &settings()).expect("the documented example composes");
    let segment = prefix
        .segments
        .iter()
        .find(|segment| segment.id == SegmentId::Template)
        .expect("a template segment");
    assert_eq!(
        &prefix.bytes[segment.offset..segment.offset + segment.len],
        template.as_bytes(),
        "the documented example is carried verbatim"
    );

    // The awkward half of the literal-text claim: the example deliberately
    // contains a brace pair and the string `<vibevm>` inline, and both are
    // ordinary characters — composition neither acts on them nor refuses.
    for literal in ["{this}", "<vibevm>", "SYSTEM.md"] {
        assert!(
            template.contains(literal),
            "the example no longer demonstrates that `{literal}` is literal text"
        );
    }
    assert!(
        !template
            .lines()
            .any(|line| line.trim() == "<vibevm>" || line.trim() == "</vibevm>"),
        "an exactly standalone marker line in the example would be a refusal, \
         which is the one nuance the guide has to get right"
    );
}

/// The guide's invalidation table speaks the enum's own vocabulary.
///
/// `PLAN.md` P8 asks for a table *generated* from the enum. The `Stale`
/// reasons are rendered sentences rather than codes — deliberately, since a
/// reason that cannot be printed is a reason nobody acts on — so there is no
/// enum of reasons to generate rows from. What is mechanical, and what this
/// asserts, is the part that can silently drift: every verdict word the
/// table uses is a `CheckVerdict` variant's own spelling, and all six are
/// documented.
#[test]
fn the_documented_verdicts_are_the_enums_own_spellings() {
    let Some(guide) = documented(GUIDE) else {
        eprintln!("skipped: not a host checkout");
        return;
    };
    let verdicts = [
        CheckVerdict::Equal,
        CheckVerdict::Stale(Vec::new()),
        CheckVerdict::Missing(Vec::new()),
        CheckVerdict::Conflict(String::new()),
        CheckVerdict::Unavailable(String::new()),
        CheckVerdict::FreshnessUnverifiable(String::new()),
    ];
    for verdict in &verdicts {
        assert!(
            guide.contains(&format!("`{}`", verdict.as_str())),
            "the guide's invalidation table does not name the verdict `{}`",
            verdict.as_str()
        );
    }
    // And the guide names each verb and each generated path, so a renamed
    // command or artifact fails here rather than in a reader's terminal.
    for verb in [
        "vibe prompt build",
        "vibe prompt check",
        "vibe prompt prepare",
        "vibe prompt capture",
        "vibe prompt path",
        "vibe prompt flags",
        "vibe agent",
    ] {
        assert!(guide.contains(verb), "the guide does not document `{verb}`");
    }
    for path in [
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_META_JSON,
        vibe_core::layout::AGENT_LANE_MD,
        vibe_core::layout::SYSTEM_TEMPLATE_MD,
    ] {
        assert!(
            guide.contains(path),
            "the guide does not name the generated path `{path}`"
        );
    }
}
