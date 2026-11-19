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
    acquire(
        reporter,
        address,
        reference,
        reuse,
        objects,
        source_address,
        false,
        None,
    )
}

/// Acquire into a **durable** store of this capability's own, refreshing it
/// when it is already there instead of cloning the repository again
/// (PROP-073 `#NATIVE-PREPARATION`). Only the registry cache owner, which
/// owns that directory and its lock, asks for this.
pub(in crate::git_backend::shell) fn acquire_cached_source(
    reporter: &super::super::ShellGit,
    address: &str,
    reference: &str,
    namespace: Option<&str>,
    objects: &Path,
) -> Result<Option<NativeSource>, GitError> {
    acquire(
        reporter,
        address,
        reference,
        None,
        objects,
        SourceAddress::WarmOrigin,
        true,
        namespace,
    )
}

#[allow(clippy::too_many_arguments)]
fn acquire(
    reporter: &super::super::ShellGit,
    address: &str,
    reference: &str,
    reuse: Option<&Path>,
    objects: &Path,
    source_address: SourceAddress,
    cache: bool,
    namespace: Option<&str>,
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
                cache,
                namespace,
            })
        })
        .transpose()
}
