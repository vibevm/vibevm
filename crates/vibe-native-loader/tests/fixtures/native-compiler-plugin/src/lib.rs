#![deny(unsafe_code)]

use vibe_ext::{
    CompileReply, CompileReplyFail, CompileReplyOk, CompileReplySkip, CompileRequest, Ir, Manifest,
    ManifestExtension,
};

/// Harmless rlib marker that forces this compiler fixture through Cargo's
/// ordinary loader dev-dependency graph.
///
/// ```
/// assert_eq!(
///     vibe_native_loader_compiler_fixture::fixture_marker(),
///     "vibe-native-loader-compiler-fixture"
/// );
/// ```
pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-compiler-fixture"
}

/// Safe manifest accessor for fixture registration tests.
///
/// ```
/// let manifest = vibe_native_loader_compiler_fixture::fixture_manifest();
/// assert!(manifest.extensions.iter().all(|extension| {
///     extension.point.starts_with("compile:") && extension.ir_schema == Some(1)
/// }));
/// ```
pub fn fixture_manifest() -> Manifest {
    Manifest {
        extensions: [
            ("compiler-ok", "compile:pass"),
            ("compiler-skip", "compile:pass"),
            ("compiler-fail", "compile:pass"),
            ("compiler-panic", "compile:pass"),
            ("compiler-after", "compile:pass"),
            ("compiler-manager-source", "compile:source"),
        ]
        .into_iter()
        .map(|(id, point)| ManifestExtension {
            id: id.to_owned(),
            point: point.to_owned(),
            ir_schema: Some(1),
        })
        .collect(),
    }
}

fn handle(request: CompileRequest) -> CompileReply {
    match request.execution.id.as_str() {
        "compiler-skip" => CompileReply::Skip(Box::new(CompileReplySkip {
            envelope: 1,
            message: Some("deterministic compiler skip".to_owned()),
        })),
        "compiler-fail" => CompileReply::Fail(Box::new(CompileReplyFail {
            envelope: 1,
            message: Some("deterministic compiler failure".to_owned()),
        })),
        "compiler-panic" => panic!("deterministic compiler fixture panic"),
        "compiler-manager-source" => match request.payload {
            Ir::SourceDocument(mut payload) => {
                payload
                    .doc
                    .text
                    .push_str("\nR5.5 real native source marker\n");
                CompileReply::Ok(Box::new(CompileReplyOk {
                    envelope: 1,
                    payload: Ir::SourceDocument(payload),
                    message: Some("handled compiler-manager-source".to_owned()),
                }))
            }
            _ => CompileReply::Fail(Box::new(CompileReplyFail {
                envelope: 1,
                message: Some("compiler-manager-source requires source IR".to_owned()),
            })),
        },
        _ => CompileReply::Ok(Box::new(CompileReplyOk {
            envelope: 1,
            payload: request.payload,
            message: Some(format!("handled {}", request.execution.id)),
        })),
    }
}

vibe_ext::vibe_compile_extension!(manifest = fixture_manifest(), handler = handle);
