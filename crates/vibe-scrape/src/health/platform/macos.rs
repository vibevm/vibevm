use super::UnsupportedBackend;

pub(super) fn backend() -> UnsupportedBackend {
    UnsupportedBackend::new(
        "macOS health enforcement requires a proven sandbox, filesystem-isolation, and process-tree backend",
    )
}
