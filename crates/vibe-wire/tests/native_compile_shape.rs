use vibe_wire::behaviour::native_compile::{
    CompileReplyStatus, CompileWireSide, IrCardinality, IrCarrier, IrLevel, IrShape,
    NativeCompileError, validate_reply_for_shape,
};
use vibe_wire::generated::native::e1::compile_reply::{
    CompileReply, CompileReplyOk, CompileReplySkip, Ir,
};

fn ir(name: &str) -> Ir {
    let raw = match name {
        "source" => {
            include_str!("../../../formats/corpora/compiler_ir/e1/valid/source_document.json")
        }
        "document" => {
            include_str!("../../../formats/corpora/compiler_ir/e1/valid/document_document.json")
        }
        _ => unreachable!(),
    };
    serde_json::from_str(raw).expect("canonical compiler IR corpus decodes")
}

const SOURCE_SHAPE: IrShape = IrShape {
    carrier: IrCarrier::SourceDocument,
    level: IrLevel::Source,
    cardinality: IrCardinality::Document,
};

#[test]
fn admitted_shape_validates_reply_envelope_schema_status_and_exact_ok_shape() {
    let ok = CompileReply::Ok(Box::new(CompileReplyOk {
        envelope: 1,
        payload: ir("source"),
        message: None,
    }));
    assert_eq!(validate_reply_for_shape(SOURCE_SHAPE, &ok), Ok(()));

    let changed = CompileReply::Ok(Box::new(CompileReplyOk {
        envelope: 1,
        payload: ir("document"),
        message: None,
    }));
    assert!(matches!(
        validate_reply_for_shape(SOURCE_SHAPE, &changed),
        Err(NativeCompileError::ExchangeShape { .. })
    ));

    let bad_envelope = CompileReply::Skip(Box::new(CompileReplySkip {
        envelope: 2,
        message: None,
    }));
    assert_eq!(
        validate_reply_for_shape(SOURCE_SHAPE, &bad_envelope),
        Err(NativeCompileError::Envelope {
            side: CompileWireSide::Reply(CompileReplyStatus::Skip),
            found: 2,
        })
    );
}
