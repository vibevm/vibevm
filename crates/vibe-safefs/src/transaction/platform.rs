//! The one OS-specific operation scrape needs: atomic rename without replace.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

use super::tree::EntryState;
use crate::Pinned;

#[derive(Debug)]
pub(super) enum NoReplaceError {
    Occupied,
    SourceChanged,
    SourceReappeared,
    CrossFilesystem,
    Unsupported,
    Io(std::io::Error),
}

pub(super) enum NativeCreateError {
    NotCreated(std::io::Error),
    CreatedButUnsealed(std::io::Error),
    #[cfg(not(windows))]
    Unsupported,
}

pub(super) enum NativeRemoveError {
    Changed(String),
    Io(std::io::Error),
    #[cfg(not(windows))]
    Unsupported,
}

include!("platform/operations.rs");
include!("platform/windows_abi.rs");
include!("platform/fallback.rs");
