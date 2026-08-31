#![deny(unsafe_code)]

use vibe_ext::{CompileReply, CompileReplySkip, CompileRequest, Manifest, ManifestExtension};

fn safe_handler(_request: CompileRequest) -> CompileReply {
    CompileReply::Skip(Box::new(CompileReplySkip {
        envelope: 1,
        message: Some("safe typed compiler handler".to_owned()),
    }))
}

vibe_ext::vibe_compile_extension!(
    manifest = Manifest {
        extensions: vec![ManifestExtension {
            id: "safe-compiler-author".to_owned(),
            point: "compile:pass".to_owned(),
            ir_schema: Some(1),
        }],
    },
    handler = safe_handler,
);

#[test]
fn compiler_author_uses_only_safe_generated_types() {
    assert_eq!(vibe_ext_abi(), 1);
    assert!(!vibe_ext_manifest().is_null());
}
