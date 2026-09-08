#[cfg(not(windows))]
pub(super) fn rename_noreplace(
    _source: &Pinned,
    _destination: &Pinned,
    _old: &str,
    _new: &str,
    _expected: &EntryState,
) -> Result<super::DirectoryDurability, NoReplaceError> {
    Err(NoReplaceError::Unsupported)
}
