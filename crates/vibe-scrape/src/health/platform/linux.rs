use super::UnsupportedBackend;

pub(super) fn backend() -> UnsupportedBackend {
    UnsupportedBackend::new(
        "Linux health enforcement requires a proven namespace/overlay, process-tree, and network-deny backend",
    )
}
