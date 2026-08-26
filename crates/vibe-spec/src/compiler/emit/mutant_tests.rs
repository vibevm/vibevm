use std::sync::Arc;

use super::*;
use crate::compiler::backend::{BackendRegistry, EmitBackend};
use crate::compiler::builtin::compile_artifact_with_registry;
use crate::compiler::ir::{ArtifactInput, ArtifactPlan, ArtifactTarget, LaneIr};
use crate::{SectionSource, SpecAddress};

struct EmptySource;

impl SectionSource for EmptySource {
    fn section_text(&self, _addr: &SpecAddress) -> Result<String, String> {
        Ok(String::new())
    }
}

#[derive(Clone, Copy)]
enum Mutation {
    DropMarkdownContribution,
    ReorderMarkdownContributions,
    ChangeMarkdownBody,
    WrongXmlTapeWithRightCount,
}

struct MutantBackend {
    id: BackendId,
    pass: PassName,
    mutation: Mutation,
}

impl MutantBackend {
    fn new(id: &str, mutation: Mutation) -> Self {
        Self {
            id: BackendId::new(id).unwrap(),
            pass: PassName::new(format!("emit:{id}")).unwrap(),
            mutation,
        }
    }
}

impl EmitBackend for MutantBackend {
    fn id(&self) -> &BackendId {
        &self.id
    }

    fn pass_name(&self) -> &PassName {
        &self.pass
    }

    fn emit(
        &self,
        lane: &LaneIr,
        witness: &crate::compiler::ir::PreEmissionWitness,
    ) -> Result<Vec<u8>, BackendError> {
        let bytes = if self.id.as_str() == "static-md" {
            static_md::StaticMarkdownBackend::new().emit(lane, witness)?
        } else {
            static_xml::StaticXmlBackend::new().emit(lane, witness)?
        };
        let text = String::from_utf8(bytes).unwrap();
        let mutated = match self.mutation {
            Mutation::DropMarkdownContribution => {
                let marker = "<!-- vibe:static org.demo/b — boot/b.md -->";
                text[..text.find(marker).unwrap()].to_string()
            }
            Mutation::ReorderMarkdownContributions => text
                .replace("org.demo/a — boot/a.md", "org.demo/z — boot/z.md")
                .replace("org.demo/b — boot/b.md", "org.demo/a — boot/a.md")
                .replace("org.demo/z — boot/z.md", "org.demo/b — boot/b.md"),
            Mutation::ChangeMarkdownBody => text.replacen("# A", "# MUTANT", 1),
            Mutation::WrongXmlTapeWithRightCount => swap_first_two_xml_documents(&text),
        };
        Ok(mutated.into_bytes())
    }
}

fn two_simple_plan(target: ArtifactTarget) -> ArtifactPlan {
    let path = if target == ArtifactTarget::StaticMarkdown {
        "vibevm/vibespecs/boot/STATIC.md"
    } else {
        "vibevm/vibespecs/boot/STATIC.xml"
    };
    ArtifactPlan::static_lane(
        target,
        path,
        "vibevm/vibedeps",
        vec![
            ArtifactInput::simple("org.demo/a", "boot/a.md", "# A {#root}\n\nbody a\n").unwrap(),
            ArtifactInput::simple("org.demo/b", "boot/b.md", "# B {#root}\n\nbody b\n").unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn engine_owned_markdown_target_rejects_drop_reorder_and_body_mutants() {
    for mutation in [
        Mutation::DropMarkdownContribution,
        Mutation::ReorderMarkdownContributions,
        Mutation::ChangeMarkdownBody,
    ] {
        let mut registry = BackendRegistry::builtins();
        registry
            .replace(Arc::new(MutantBackend::new("static-md", mutation)))
            .unwrap();
        let error = compile_artifact_with_registry(
            two_simple_plan(ArtifactTarget::StaticMarkdown),
            &EmptySource,
            &registry,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            crate::compiler::builtin::ArtifactCompileError::Backend { .. }
        ));
    }
}

#[test]
fn engine_owned_xml_target_rejects_wrong_tape_with_the_right_payload_count() {
    let mut registry = BackendRegistry::builtins();
    registry
        .replace(Arc::new(MutantBackend::new(
            "static-xml",
            Mutation::WrongXmlTapeWithRightCount,
        )))
        .unwrap();
    let error = compile_artifact_with_registry(
        two_simple_plan(ArtifactTarget::StaticXml),
        &EmptySource,
        &registry,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        crate::compiler::builtin::ArtifactCompileError::Backend { .. }
    ));
}

fn swap_first_two_xml_documents(text: &str) -> String {
    let first_start = text.find("<?xml version=").unwrap();
    let first_end = first_start + text[first_start..].find("</spec>").unwrap() + "</spec>".len();
    let second_start = first_end + text[first_end..].find("<?xml version=").unwrap();
    let second_end = second_start + text[second_start..].find("</spec>").unwrap() + "</spec>".len();
    format!(
        "{}{}{}{}{}",
        &text[..first_start],
        &text[second_start..second_end],
        &text[first_end..second_start],
        &text[first_start..first_end],
        &text[second_end..]
    )
}
