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
                id: "prebuilt-ok".to_owned(),
                point: "phase:build".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "prebuilt-skip".to_owned(),
                point: "phase:build".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "prebuilt-fail".to_owned(),
                point: "phase:build".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "prebuilt-panic".to_owned(),
                point: "phase:build".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "prebuilt-after".to_owned(),
                point: "phase:build".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "slot-pre-fixture".to_owned(),
                point: "slot:pre-install".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "slot-pre-skip".to_owned(),
                point: "slot:pre-install".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "slot-pre-fail".to_owned(),
                point: "slot:pre-install".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "slot-post-fixture".to_owned(),
                point: "slot:post-install".to_owned(),
                ir_schema: None,
            },
            ManifestExtension {
                id: "slot-post-fail".to_owned(),
                point: "slot:post-install".to_owned(),
                ir_schema: None,
            },
        ],
    }
}

fn handle(context: Context) -> Reply {
    if context.execution.id.ends_with("-panic") {
        panic!("deterministic fixture panic");
    }
    let status = if context.execution.id.ends_with("-skip") {
        ReplyStatus::Skip
    } else if context.execution.id.ends_with("-fail") {
        ReplyStatus::Fail
    } else {
        ReplyStatus::Ok
    };
    let label = if context.execution.id.starts_with("prebuilt-")
        || context.execution.id.ends_with("-skip")
        || context.execution.id.ends_with("-fail")
        || context.execution.id.ends_with("-panic")
    {
        context.execution.id.as_str()
    } else {
        "fixture"
    };
    Reply {
        artifacts: Vec::new(),
        envelope: 1,
        status,
        message: Some(format!("{} handled {}", label, context.point)),
    }
}

vibe_ext::vibe_extension!(manifest = manifest(), handler = handle);
