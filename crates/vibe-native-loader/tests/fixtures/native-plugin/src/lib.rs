#![deny(unsafe_code)]

use vibe_ext::{Context, Manifest, ManifestExtension, Reply, ReplyStatus};

/// A harmless rlib marker referenced by loader tests to force fixture build.
///
/// ```
/// assert_eq!(
///     vibe_native_loader_fixture::fixture_marker(),
///     "vibe-native-loader-fixture"
/// );
/// ```
pub fn fixture_marker() -> &'static str {
    "vibe-native-loader-fixture"
}

fn manifest() -> Manifest {
    Manifest {
        extensions: vec![
            ManifestExtension {
                id: "fixture".to_owned(),
                point: "phase:build".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "slot-pre-fixture".to_owned(),
                point: "slot:pre-install".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "slot-post-fixture".to_owned(),
                point: "slot:post-install".to_owned(),
                ir_schema: None,
            },
        ],
    }
}

fn handle(context: Context) -> Reply {
    Reply {
        artifacts: Vec::new(),
        envelope: 1,
        status: ReplyStatus::Ok,
        message: Some(format!("fixture handled {}", context.point)),
    }
}

vibe_ext::vibe_extension!(manifest = manifest(), handler = handle);
