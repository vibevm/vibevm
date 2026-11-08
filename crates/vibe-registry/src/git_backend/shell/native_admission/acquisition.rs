//! Source transport choice is separate from retained warm checkout metadata.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
pub(in crate::git_backend::shell) fn acquire_source(
    reporter: &super::super::ShellGit,
    address: &str,
    reference: &str,
    reuse: Option<&Path>,
    objects: &Path,
    source_address: SourceAddress,
) -> Result<Option<NativeSource>, GitError> {
    reporter
        .native
        .as_ref()
        .map(|native| {
            native.acquire(Preparation {
                source_address,
                reporter,
                address,
                reference,
                reuse,
                objects,
                destination: objects,
                anonymous: reporter.force_anonymous,
            })
        })
        .transpose()
}
