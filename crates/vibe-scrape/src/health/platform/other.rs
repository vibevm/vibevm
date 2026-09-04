use super::UnsupportedBackend;

pub(super) fn backend() -> UnsupportedBackend {
    UnsupportedBackend::new("this platform has no proven scrape health enforcement backend")
}
