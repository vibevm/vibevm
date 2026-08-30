use vibe_ext::{Context, Manifest, ManifestExtension, Reply, ReplyStatus};

fn handle(_context: Context) -> Reply {
    Reply {
        artifacts: Vec::new(),
        envelope: 1,
        status: ReplyStatus::Ok,
        message: None,
    }
}

vibe_ext::vibe_extension!(
    manifest = Manifest {
        extensions: vec![ManifestExtension {
            id: "abort-fixture".to_owned(),
            point: "phase:test".to_owned(),
            ir_schema: None,
        }],
    },
    handler = handle,
);
