use super::*;
use crate::{ResolvedSource, ResolvedSourceMetadata};
use std::cell::Cell;

// A legal internal seed key, deliberately independent of the physical STATIC
// output name (authored boot/STATIC directives are rejected by the parser).
const ROOT: &str = "spec://org.demo/generated/internal-seed#root";
const CHILD: &str = "spec://org.demo/child/entry#root";

struct RootSource {
    reference: Option<&'static str>,
    root_reads: Cell<usize>,
    include_child: bool,
}
impl SectionSource for RootSource {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        if address.without_pin() == CHILD {
            Ok(format!(
                "# Child {{#root}}\n{}\n",
                self.reference
                    .map(|kind| format!("#{kind} {ROOT}"))
                    .unwrap_or("ordinary child".into())
            ))
        } else {
            Err("generated root is not an ordinary addressed source".into())
        }
    }
    fn read_resolved_root_source(
        &self,
        address: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        if address.without_pin() != ROOT {
            return self.read_resolved_source(address, metadata);
        }
        self.root_reads.set(self.root_reads.get() + 1);
        crate::resolve_captured_declared_source(
            address,
            metadata,
            if self.include_child {
                format!("# Generated {{#root}}\n#use {CHILD}\n")
            } else {
                "# Generated {#root}\n".into()
            }
            .as_bytes(),
            "text/markdown",
            vibe_core::manifest::SourceInterpretation::VibeSpec,
        )
    }
    fn admit_reference(&self, address: &SpecAddress) -> Result<(), String> {
        if address.authority == spec(ROOT).authority && address.doc_path == spec(ROOT).doc_path {
            Err("internal generated root is not a public reference".into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn initial_generated_root_keeps_normal_dependency_and_anchor_behavior() {
    let source = RootSource {
        reference: None,
        root_reads: Cell::new(0),
        include_child: true,
    };
    let text = crate::compile_static(&spec(ROOT), &source).unwrap();
    assert!(text.contains("ordinary child"));
    assert!(text.contains("Generated"));
    assert_eq!(source.root_reads.get(), 1);
}

#[test]
fn nested_references_cannot_reuse_an_internal_generated_root_cache() {
    for kind in ["use", "source", "embed"] {
        let source = RootSource {
            reference: Some(kind),
            root_reads: Cell::new(0),
            include_child: false,
        };
        let plan = ArtifactPlan::new(
            static_context(),
            vec![
                ArtifactInput::normal("org.demo/generated", "boot/STATIC.md", spec(ROOT)).unwrap(),
                ArtifactInput::normal("org.demo/child", "entry.md", spec(CHILD)).unwrap(),
            ],
        )
        .unwrap();
        let error = crate::compiler::builtin::compile_artifact(plan, &source)
            .unwrap_err()
            .to_string();
        assert!(error.contains("internal generated root"), "{kind}: {error}");
        assert_eq!(source.root_reads.get(), 1);
    }
}

#[test]
fn denied_reference_stops_before_later_input_parse_callback() {
    let source = RootSource {
        reference: Some("use"),
        root_reads: Cell::new(0),
        include_child: false,
    };
    let plan = ArtifactPlan::new(
        static_context(),
        vec![
            ArtifactInput::normal("org.demo/generated", "boot/STATIC.md", spec(ROOT)).unwrap(),
            ArtifactInput::normal("org.demo/child", "entry.md", spec(CHILD)).unwrap(),
            ArtifactInput::simple("host", "later.md", "LATE_SENTINEL").unwrap(),
        ],
    )
    .unwrap();
    let result = discover(
        &plan,
        &source,
        |input| {
            assert!(
                !input.text().contains("LATE_SENTINEL"),
                "later parse callback must not run after reference denial"
            );
            Ok::<_, std::convert::Infallible>(parse(input))
        },
        |_, _| {},
    )
    .unwrap();
    assert!(
        result
            .reference_refusal
            .as_ref()
            .unwrap()
            .1
            .contains("internal generated root")
    );
    assert_eq!(source.root_reads.get(), 1);
}

#[test]
fn legacy_default_admission_keeps_cache_reuse_but_explicit_denial_is_honored() {
    struct DefaultAdmission(RootSource);
    impl SectionSource for DefaultAdmission {
        fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
            self.0.section_text(address)
        }
        fn read_resolved_root_source(
            &self,
            address: &SpecAddress,
            metadata: ResolvedSourceMetadata,
        ) -> Result<ResolvedSource, String> {
            self.0.read_resolved_root_source(address, metadata)
        }
    }
    let plan = || {
        ArtifactPlan::new(
            static_context(),
            vec![
                ArtifactInput::normal("org.demo/generated", "boot/STATIC.md", spec(ROOT)).unwrap(),
                ArtifactInput::normal("org.demo/child", "entry.md", spec(CHILD)).unwrap(),
            ],
        )
        .unwrap()
    };
    let control = DefaultAdmission(RootSource {
        reference: Some("use"),
        root_reads: Cell::new(0),
        include_child: false,
    });
    assert!(control.section_text(&spec(ROOT)).is_err());
    assert!(
        crate::compiler::builtin::compile_artifact(plan(), &control).is_ok(),
        "legacy default keeps established address-cache reuse"
    );
    let guarded = RootSource {
        reference: Some("use"),
        root_reads: Cell::new(0),
        include_child: false,
    };
    assert!(
        crate::compiler::builtin::compile_artifact(plan(), &guarded)
            .unwrap_err()
            .to_string()
            .contains("internal generated root")
    );
}
