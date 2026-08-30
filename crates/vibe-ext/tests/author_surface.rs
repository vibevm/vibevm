#![deny(unsafe_code)]

use vibe_ext::{Context, Manifest, ManifestExtension, Reply, ReplyStatus};

fn safe_handler(_context: Context) -> Reply {
    Reply {
        artifacts: Vec::new(),
        envelope: 1,
        status: ReplyStatus::Skip,
        message: None,
    }
}

vibe_ext::vibe_extension!(
    manifest = Manifest {
        extensions: vec![ManifestExtension {
            id: "safe-author".to_owned(),
            point: "phase:validate".to_owned(),
            ir_schema: None,
        }],
    },
    handler = safe_handler,
);

#[test]
fn author_uses_only_safe_rust() {
    assert_eq!(vibe_ext_abi(), 1);
    assert!(!vibe_ext_manifest().is_null());
}
