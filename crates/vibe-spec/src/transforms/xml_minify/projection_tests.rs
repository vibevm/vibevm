use super::*;
use crate::compiler::builtin::compile_artifact;
use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
    StaticCompileMode,
};
use crate::{SectionSource, SpecAddress};

struct Source;
impl SectionSource for Source {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        match address.doc_path.as_str() {
            "a" => Ok("# A {#a}\nbefore\n#embed spec://org.demo/pkg/outer\nafter\n".into()),
            "b" => Ok("# B {#b}\nB-TEXT\n".into()),
            "outer" => Ok("OUTER\n#embed spec://org.demo/pkg/inner\nEND\n".into()),
            "inner" => Ok("INNER\n".into()),
            "structures" => Ok("# Structures {#structures}\n\n#embed spec://org.demo/pkg/table\n\n#embed spec://org.demo/pkg/list\n".into()),
            "table" => Ok("| H |\n| --- |\n| cell |\n".into()),
            "list" => Ok("- item one\n- item two\n".into()),
            _ => Err("unexpected source read".into()),
        }
    }
}
fn emit(inputs: Vec<ArtifactInput>) -> crate::compiler::ir::EmittedArtifact {
    let context = ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".into(),
            source_root: "vibevm/vibespecs".into(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    compile_artifact(ArtifactPlan::new(context, inputs).unwrap(), &Source).unwrap()
}
fn normal(name: &str) -> ArtifactInput {
    ArtifactInput::normal(
        "org.demo/pkg",
        format!("{name}.md"),
        SpecAddress::parse(&format!("spec://org.demo/pkg/{name}#{name}")).unwrap(),
    )
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-GENERATED-XML-PROJECTION-LAW"
)]
fn actual_multiple_root_static_projects_anchors_nested_events_and_frames() {
    let emitted = emit(vec![normal("a"), normal("b")]);
    let input = std::str::from_utf8(emitted.bytes()).unwrap();
    assert_eq!(input.matches("<?xml").count(), 2);
    assert!(
        vibe_specdoc::from_xml(input).is_err(),
        "single-document law stays strict"
    );
    let projected = project_emitted_xml_to_markdown(input).unwrap();
    for word in ["before", "OUTER", "INNER", "END", "after", "B-TEXT"] {
        assert!(projected.contains(word), "{projected}");
    }
    for rename in &emitted.provenance().renames {
        assert!(projected.contains(&format!("{{#{}}}", rename.rename.qualified)));
    }
    assert_eq!(projected.matches("<!-- vibe:c1 embed:").count(), 2);
    assert_eq!(projected.matches("<!-- vibe:c1 /embed:").count(), 2);
    for comment in crate::compiler::emit::framing::xml_comment_spans(input) {
        let comment = comment.unwrap();
        if comment.depth == 0 {
            assert!(projected.contains(&input[comment.start..comment.end]));
        }
    }
    assert!(!projected.contains("<?xml"));
    assert!(
        !projected.contains(vibe_specdoc::PROJECTION_NOTICE),
        "position notices are metadata, not inserted prose"
    );
}

#[test]
fn actual_zero_root_elided_and_hoisted_streams_remain_readable() {
    for inputs in [
        vec![],
        vec![ArtifactInput::elided("org.demo/pkg", "none.md").unwrap()],
        vec![
            ArtifactInput::hoisted(
                "org.demo/shared",
                "manual.md",
                SpecAddress::parse("spec://org.demo/shared/manual").unwrap(),
            )
            .unwrap(),
        ],
    ] {
        let emitted = emit(inputs);
        let input = std::str::from_utf8(emitted.bytes()).unwrap();
        assert!(!input.contains("<spec "));
        assert_eq!(project_emitted_xml_to_markdown(input).unwrap(), input);
        assert!(
            super::super::minify_emitted_xml(input).is_err(),
            "projection does not broaden minifier acceptance"
        );
    }
}

#[test]
fn actual_structured_and_interleaved_contributions_project_without_loss() {
    let artifact = emit(vec![
        normal("structures"),
        ArtifactInput::elided("org.demo/pkg", "none.md").unwrap(),
        ArtifactInput::hoisted(
            "org.demo/shared",
            "manual.md",
            SpecAddress::parse("spec://org.demo/shared/manual").unwrap(),
        )
        .unwrap(),
        normal("b"),
    ]);
    let input = std::str::from_utf8(artifact.bytes()).unwrap();
    let projection = project_emitted_xml_to_markdown(input).unwrap();
    for text in [
        "cell",
        "item one",
        "item two",
        "B-TEXT",
        "#use spec://org.demo/shared",
    ] {
        assert!(projection.contains(text), "{projection}");
    }
    assert_eq!(projection.matches("<!-- vibe:c1 embed:").count(), 2);
    assert_eq!(projection.matches("<!-- vibe:c1 /embed:").count(), 2);
}

#[test]
fn escaped_cdata_and_fenced_controls_cannot_mint_projection_events() {
    let input = "<spec xmlns='https://vibevm.org/spec/1'><p>&lt;!-- vibe:c1 embed: literal --&gt;</p><p>&lt;!-- vibe:static literal — example --&gt;</p><fence><![CDATA[<!-- vibe:c1 embed: code -->]]></fence><p>x<!-- vibe:c1 embed: actual -->y<!-- vibe:c1 /embed: actual --></p></spec>";
    let projected = project_emitted_xml_to_markdown(input).unwrap();
    assert!(projected.contains("&lt;!-- vibe:c1 embed: literal -->"));
    assert!(projected.contains("&lt;!-- vibe:static literal — example -->"));
    assert!(projected.contains("```\n<!-- vibe:c1 embed: code -->\n```"));
    assert!(projected.contains("x<!-- vibe:c1 embed: actual -->y<!-- vibe:c1 /embed: actual -->"));
}

#[test]
fn malformed_unknown_and_unowned_top_level_content_refuse() {
    for input in [
        "",
        "<foreign/>",
        "<spec/>",
        "<!DOCTYPE spec><spec xmlns='https://vibevm.org/spec/1'/>",
        "<spec xmlns='https://vibevm.org/spec/1'><p>&unknown;</p></spec>",
        "<!-- vibe:c1 %XX --><spec xmlns='https://vibevm.org/spec/1'/>",
        "<!-- unclosed",
        "<?xml version='1.0'?>",
        "#use spec://org.demo/pkg\n",
        "<?opaque x?><spec xmlns='https://vibevm.org/spec/1'/>",
    ] {
        assert!(
            project_emitted_xml_to_markdown(input).is_err(),
            "accepted {input}"
        );
    }
    let hoisted = crate::compiler::emit::framing::hoisted_marker(
        crate::compiler::emit::framing::CommentSyntax::Xml,
        "org.demo/pkg",
    );
    assert!(project_emitted_xml_to_markdown(&hoisted).is_err());
}
