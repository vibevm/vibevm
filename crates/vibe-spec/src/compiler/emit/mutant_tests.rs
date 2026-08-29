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
    /// R4 §7.1 — the header's tokens spelled RAW, bypassing the shared codec.
    /// The tape then AGREES with the engine's own payload only if the payload
    /// were built the same wrong way, so this is the mutation only a grammar
    /// check can see.
    RawTransformsHeaderToken,
    /// The same token escaped with lowercase hex — a second spelling of one
    /// byte, which the codec refuses by definition.
    LowercaseTransformsHeaderEscape,
    /// A token carrying an escape the canonical spelling does not use: it
    /// decodes, but re-encodes to something else.
    UnnecessaryTransformsHeaderEscape,
    /// A token RE-encoded once more: canonical for a DIFFERENT logical key,
    /// so the grammar admits it and only the identity comparison can refuse
    /// it — the half the grammar check cannot cover, pinned beside it.
    ReEncodedTransformsHeaderToken,
    /// The header line deleted outright.
    DropTransformsHeader,
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
            Mutation::RawTransformsHeaderToken => text.replacen("a-%2Db", "a--b", 1),
            Mutation::LowercaseTransformsHeaderEscape => text.replacen("a-%2Db", "a-%2db", 1),
            Mutation::UnnecessaryTransformsHeaderEscape => text.replacen("#second", "#%73econd", 1),
            Mutation::ReEncodedTransformsHeaderToken => text.replacen("a-%2Db", "a-%252Db", 1),
            Mutation::DropTransformsHeader => {
                let start = text
                    .find("<!-- vibe:transforms")
                    .expect("the header is present");
                let end = start + text[start..].find('\n').expect("the header line ends") + 1;
                format!("{}{}", &text[..start], &text[end..])
            }
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

/// R4 §7.1 — the tape validators know the header's exact grammar: a
/// well-formed one is admitted (proved by the unmutated compiles in
/// `transform::header_e2e_tests`), and a malformed payload is refused with the
/// CODEC's own error, in BOTH lanes.
///
/// Three payload mutations are the three ways a second spelling of one
/// identity can appear — raw where the codec escapes, lowercase hex, and an
/// escape the canonical spelling does not use. Each is invisible to a byte
/// comparison against the engine's own payload when the EMITTER is the thing
/// that went wrong, so the assertion reads the codec's message and not merely
/// "some refusal happened". The fourth is the complement: a re-encoded token
/// is perfectly canonical — for a DIFFERENT key — so the grammar admits it
/// and only the identity comparison refuses it, which is why both halves must
/// exist. The fifth deletes the line, which the cursor-exact observer catches
/// as the missing frame it is.
#[test]
fn both_lanes_refuse_a_malformed_or_missing_transforms_header() {
    use crate::compiler::builtin::compile_artifact_with_registries;
    use crate::compiler::transform::plan::TransformStage;
    use crate::compiler::transform::registry_test_support::{identity_plan, identity_registry};

    for (target, backend) in [
        (ArtifactTarget::StaticMarkdown, "static-md"),
        (ArtifactTarget::StaticXml, "static-xml"),
    ] {
        for (mutation, expected) in [
            (Mutation::RawTransformsHeaderToken, "not canonical"),
            (Mutation::LowercaseTransformsHeaderEscape, "not uppercase"),
            (Mutation::UnnecessaryTransformsHeaderEscape, "not canonical"),
            (
                Mutation::ReEncodedTransformsHeaderToken,
                "transforms header mismatch",
            ),
            (Mutation::DropTransformsHeader, "transforms header"),
        ] {
            let mut registry = BackendRegistry::builtins();
            registry
                .replace(Arc::new(MutantBackend::new(backend, mutation)))
                .unwrap();
            let plan = two_simple_plan(target.clone()).with_transforms(identity_plan(&[
                ("org.demo/tools#first", TransformStage::Lane),
                ("org.demo/a--b#second", TransformStage::Emitted),
            ]));
            let error = compile_artifact_with_registries(
                plan,
                &EmptySource,
                &registry,
                &identity_registry(),
            )
            .unwrap_err();
            let message = error.to_string();
            assert!(
                message.contains(expected),
                "{backend}: expected {expected:?} in {message}"
            );
        }
    }
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
