specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use super::UnsupportedBackend;

pub(super) fn backend() -> UnsupportedBackend {
    UnsupportedBackend::new("this platform has no proven scrape health enforcement backend")
}
