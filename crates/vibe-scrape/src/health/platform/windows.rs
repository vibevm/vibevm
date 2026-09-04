specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use super::UnsupportedBackend;

pub(super) fn backend() -> UnsupportedBackend {
    UnsupportedBackend::new(
        "Windows health enforcement requires a proven exact-exec, Job Object, filesystem-isolation, and network-deny backend",
    )
}
