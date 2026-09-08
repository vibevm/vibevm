#[cfg(windows)]
#[specmark::spec(
    deviates = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NATIVE-NOFOLLOW-BOUNDARY",
    reason = "NT relative open is required to lease the exact no-follow entry and deny write/delete while its retained handle lives"
)]
#[allow(unsafe_code)]
pub(super) fn lease_entry(
    parent: &Pinned,
    name: &str,
    expected: &EntryState,
) -> std::io::Result<std::fs::File> {
    use std::os::windows::io::FromRawHandle;
    use std::os::windows::prelude::{AsRawHandle, OsStrExt};

    let directory = expected.kind == super::tree::EntryStateKind::Directory;
    let wide = std::ffi::OsStr::new(name).encode_wide().collect::<Vec<_>>();
    let bytes = u16::try_from(
        wide.len()
            .checked_mul(2)
            .ok_or_else(|| std::io::Error::other("lease name length overflow"))?,
    )
    .map_err(|_| std::io::Error::other("lease name too long"))?;
    let mut object_name = UnicodeString {
        length: bytes,
        maximum_length: bytes,
        buffer: wide.as_ptr().cast_mut(),
    };
    let mut attributes = ObjectAttributes {
        length: std::mem::size_of::<ObjectAttributes>() as u32,
        root_directory: parent.dir.as_raw_handle(),
        object_name: &mut object_name,
        attributes: OBJ_CASE_INSENSITIVE,
        security_descriptor: std::ptr::null_mut(),
        security_quality_of_service: std::ptr::null_mut(),
    };
    let mut io_status = IoStatusBlock {
        status_or_pointer: 0,
        information: 0,
    };
    let mut handle: Handle = std::ptr::null_mut();
    // SAFETY: relative live parent handle and repr(C) buffers outlive this
    // synchronous open. Share-read-only denies write/delete opens while the
    // returned lease handle lives.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            SYNCHRONIZE
                | FILE_READ_ATTRIBUTES
                | if directory {
                    FILE_LIST_DIRECTORY
                } else {
                    FILE_READ_DATA
                },
            &mut attributes,
            &mut io_status,
            std::ptr::null_mut(),
            0,
            FILE_SHARE_READ,
            FILE_OPEN,
            if directory {
                FILE_DIRECTORY_FILE
            } else {
                FILE_NON_DIRECTORY_FILE
            } | FILE_SYNCHRONOUS_IO_NONALERT
                | FILE_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
            0,
        )
    };
    if status < 0 {
        return Err(ntstatus_error(status));
    }
    // SAFETY: successful NtCreateFile returned one owned handle.
    let mut file = unsafe { std::fs::File::from_raw_handle(handle) };
    let display = parent.join(name);
    let actual =
        crate::file::identity::file_identity(&file, &display).map_err(std::io::Error::other)?;
    if super::tree::entry_identity(actual) != expected.identity {
        return Err(std::io::Error::other("lease identity mismatch"));
    }
    if !directory && verify_held_file_state(&mut file, &display, expected).is_err() {
        return Err(std::io::Error::other("lease file-state mismatch"));
    }
    Ok(file)
}

#[cfg(not(windows))]
pub(super) fn lease_entry(
    _parent: &Pinned,
    _name: &str,
    _expected: &EntryState,
) -> std::io::Result<std::fs::File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "tree manifest leases are Windows-only",
    ))
}

#[cfg(windows)]
#[specmark::spec(
    deviates = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NATIVE-NOFOLLOW-BOUNDARY",
    reason = "NT handle-relative open plus disposition is required to delete only the identity-checked entry without reopening its name"
)]
#[allow(unsafe_code)]
pub(super) fn remove_expected(
    parent: &Pinned,
    name: &str,
    expected: &EntryState,
) -> Result<super::DirectoryDurability, NativeRemoveError> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use std::os::windows::prelude::OsStrExt;

    let directory = expected.kind == super::tree::EntryStateKind::Directory;
    let wide = std::ffi::OsStr::new(name).encode_wide().collect::<Vec<_>>();
    let bytes = u16::try_from(
        wide.len()
            .checked_mul(2)
            .ok_or_else(|| NativeRemoveError::Io(std::io::Error::other("name length overflow")))?,
    )
    .map_err(|_| NativeRemoveError::Io(std::io::Error::other("name too long")))?;
    let mut object_name = UnicodeString {
        length: bytes,
        maximum_length: bytes,
        buffer: wide.as_ptr().cast_mut(),
    };
    let mut attributes = ObjectAttributes {
        length: std::mem::size_of::<ObjectAttributes>() as u32,
        root_directory: parent.dir.as_raw_handle(),
        object_name: &mut object_name,
        attributes: OBJ_CASE_INSENSITIVE,
        security_descriptor: std::ptr::null_mut(),
        security_quality_of_service: std::ptr::null_mut(),
    };
    let mut io_status = IoStatusBlock {
        status_or_pointer: 0,
        information: 0,
    };
    let mut handle: Handle = std::ptr::null_mut();
    // SAFETY: same relative-handle open proof as `create_directory`; the live
    // parent capability and all repr(C) buffers outlive the synchronous call.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            DELETE
                | SYNCHRONIZE
                | FILE_READ_ATTRIBUTES
                | if directory { 0 } else { FILE_READ_DATA },
            &mut attributes,
            &mut io_status,
            std::ptr::null_mut(),
            0,
            FILE_SHARE_READ | if directory { FILE_SHARE_WRITE } else { 0 },
            FILE_OPEN,
            if directory {
                FILE_DIRECTORY_FILE
            } else {
                FILE_NON_DIRECTORY_FILE
            } | FILE_SYNCHRONOUS_IO_NONALERT
                | FILE_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
            0,
        )
    };
    if status < 0 {
        let error = ntstatus_error(status);
        return if error.kind() == std::io::ErrorKind::NotFound {
            Err(NativeRemoveError::Changed("entry is absent".to_owned()))
        } else {
            Err(NativeRemoveError::Io(error))
        };
    }
    // SAFETY: the successful call returned one owned handle.
    let mut held = unsafe { std::fs::File::from_raw_handle(handle) };
    let display = parent.join(name);
    let actual = crate::file::identity::file_identity(&held, &display)
        .map_err(|error| NativeRemoveError::Io(std::io::Error::other(error)))?;
    if super::tree::entry_identity(actual) != expected.identity {
        return Err(NativeRemoveError::Changed(
            "entry identity changed before handle deletion".to_owned(),
        ));
    }
    if !directory && verify_held_file_state(&mut held, &display, expected).is_err() {
        return Err(NativeRemoveError::Changed(
            "file state changed before handle deletion".to_owned(),
        ));
    }
    super::native_mutation_hook::during(parent, name);
    let mut disposition = FileDispositionInformation { delete_file: 1 };
    // SAFETY: the fixed repr(C) disposition buffer and IO status live through
    // the synchronous call. Deletion is bound to `held`, not to a reopened
    // ambient name; close-on-drop completes the disposition.
    let status = unsafe {
        NtSetInformationFile(
            held.as_raw_handle(),
            &mut io_status,
            (&mut disposition as *mut FileDispositionInformation).cast(),
            std::mem::size_of::<FileDispositionInformation>() as u32,
            FILE_DISPOSITION_INFORMATION_CLASS,
        )
    };
    if status < 0 {
        let error = ntstatus_error(status);
        if error.raw_os_error() == Some(145) {
            Err(NativeRemoveError::Changed(
                "directory is no longer empty".to_owned(),
            ))
        } else {
            Err(NativeRemoveError::Io(error))
        }
    } else {
        drop(held);
        match parent.dir.symlink_metadata(name) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(super::DirectoryDurability::JournalRecoverable)
            }
            Ok(_) => Err(NativeRemoveError::Changed(
                "removed name was concurrently recreated".to_owned(),
            )),
            Err(error) => Err(NativeRemoveError::Io(error)),
        }
    }
}

#[cfg(not(windows))]
pub(super) fn remove_expected(
    _parent: &Pinned,
    _name: &str,
    _expected: &EntryState,
) -> Result<super::DirectoryDurability, NativeRemoveError> {
    Err(NativeRemoveError::Unsupported)
}

#[cfg(windows)]
#[specmark::spec(
    deviates = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NATIVE-NOFOLLOW-BOUNDARY",
    reason = "NT FILE_CREATE is required to atomically create-and-hold one directory without adopting a raced replacement"
)]
#[allow(unsafe_code)]
pub(super) fn create_directory(
    parent: &Pinned,
    name: &str,
) -> Result<(cap_std::fs::Dir, super::DirectoryDurability), NativeCreateError> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use std::os::windows::prelude::OsStrExt;

    let wide = std::ffi::OsStr::new(name).encode_wide().collect::<Vec<_>>();
    let bytes =
        u16::try_from(wide.len().checked_mul(2).ok_or_else(|| {
            NativeCreateError::NotCreated(std::io::Error::other("name overflow"))
        })?)
        .map_err(|_| NativeCreateError::NotCreated(std::io::Error::other("name too long")))?;
    let mut object_name = UnicodeString {
        length: bytes,
        maximum_length: bytes,
        buffer: wide.as_ptr().cast_mut(),
    };
    let mut attributes = ObjectAttributes {
        length: std::mem::size_of::<ObjectAttributes>() as u32,
        root_directory: parent.dir.as_raw_handle(),
        object_name: &mut object_name,
        attributes: OBJ_CASE_INSENSITIVE,
        security_descriptor: std::ptr::null_mut(),
        security_quality_of_service: std::ptr::null_mut(),
    };
    let mut io_status = IoStatusBlock {
        status_or_pointer: 0,
        information: 0,
    };
    let mut handle: Handle = std::ptr::null_mut();
    // SAFETY: all repr(C) values, UTF-16 storage, and the live parent handle
    // outlive the synchronous call. FILE_CREATE is atomic and returns the
    // handle to the object it created; no later name reopen is adopted.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            SYNCHRONIZE | FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY,
            &mut attributes,
            &mut io_status,
            std::ptr::null_mut(),
            FILE_ATTRIBUTE_NORMAL,
            // Excluding FILE_SHARE_DELETE makes replacement impossible while
            // the returned create handle is live. This is the namespace seal;
            // no check-then-reopen window exists to adopt another directory.
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_CREATE,
            FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
            0,
        )
    };
    if status < 0 {
        return Err(NativeCreateError::NotCreated(ntstatus_error(status)));
    }
    // SAFETY: successful NtCreateFile returned exactly one owned handle.
    let file = unsafe { std::fs::File::from_raw_handle(handle) };
    crate::file::identity::file_identity(&file, &parent.join(name))
        .map_err(|error| NativeCreateError::CreatedButUnsealed(std::io::Error::other(error)))?;
    if let Some(error) = crate::race_hook::after_create_dir(parent, name) {
        return Err(NativeCreateError::CreatedButUnsealed(error));
    }
    Ok((
        cap_std::fs::Dir::from_std_file(file),
        super::DirectoryDurability::JournalRecoverable,
    ))
}

#[cfg(not(windows))]
pub(super) fn create_directory(
    _parent: &Pinned,
    _name: &str,
) -> Result<(cap_std::fs::Dir, super::DirectoryDurability), NativeCreateError> {
    Err(NativeCreateError::Unsupported)
}

#[cfg(windows)]
#[specmark::spec(
    deviates = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NATIVE-NOFOLLOW-BOUNDARY",
    reason = "NT handle-relative open plus replace-if-exists zero is required to bind source identity and guarantee no replacement across retained parents"
)]
#[allow(unsafe_code)]
pub(super) fn rename_noreplace(
    source: &Pinned,
    destination: &Pinned,
    old: &str,
    new: &str,
    expected: &EntryState,
) -> Result<super::DirectoryDurability, NoReplaceError> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use std::os::windows::prelude::OsStrExt;

    let directory_source = expected.kind == super::tree::EntryStateKind::Directory;
    let old_wide = std::ffi::OsStr::new(old).encode_wide().collect::<Vec<_>>();
    let old_bytes = u16::try_from(old_wide.len().checked_mul(2).ok_or_else(|| {
        NoReplaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "source name length overflow",
        ))
    })?)
    .map_err(|_| {
        NoReplaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "source name is too long for UNICODE_STRING",
        ))
    })?;
    let mut old_name = UnicodeString {
        length: old_bytes,
        maximum_length: old_bytes,
        buffer: old_wide.as_ptr().cast_mut(),
    };
    let mut attributes = ObjectAttributes {
        length: std::mem::size_of::<ObjectAttributes>() as u32,
        root_directory: source.dir.as_raw_handle(),
        object_name: &mut old_name,
        attributes: OBJ_CASE_INSENSITIVE,
        security_descriptor: std::ptr::null_mut(),
        security_quality_of_service: std::ptr::null_mut(),
    };
    let mut io_status = IoStatusBlock {
        status_or_pointer: 0,
        information: 0,
    };
    let mut source_handle: Handle = std::ptr::null_mut();
    // SAFETY: every pointer names a live, correctly aligned repr(C) value for
    // the synchronous call. The UTF-16 length is checked and excludes a NUL;
    // RootDirectory is the live source capability, so `old` is resolved
    // relative to that handle with FILE_OPEN_REPARSE_POINT. The returned
    // handle is either null on failure or immediately adopted by `File`.
    let status = unsafe {
        NtCreateFile(
            &mut source_handle,
            DELETE
                | SYNCHRONIZE
                | FILE_READ_ATTRIBUTES
                | if directory_source { 0 } else { FILE_READ_DATA },
            &mut attributes,
            &mut io_status,
            std::ptr::null_mut(),
            0,
            FILE_SHARE_READ
                | if directory_source {
                    FILE_SHARE_WRITE
                } else {
                    0
                },
            FILE_OPEN,
            if directory_source {
                FILE_DIRECTORY_FILE
            } else {
                FILE_NON_DIRECTORY_FILE
            } | FILE_SYNCHRONOUS_IO_NONALERT
                | FILE_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
            0,
        )
    };
    if status < 0 {
        return Err(NoReplaceError::Io(ntstatus_error(status)));
    }
    // SAFETY: successful NtCreateFile returned one owned kernel handle and
    // ownership is transferred exactly once to File for close-on-drop.
    let mut held = unsafe { std::fs::File::from_raw_handle(source_handle) };
    let display = source.join(old);
    let actual = crate::file::identity::file_identity(&held, &display).map_err(|error| {
        NoReplaceError::Io(std::io::Error::other(format!(
            "identifying native rename source: {error:#}"
        )))
    })?;
    if super::tree::entry_identity(actual) != expected.identity {
        return Err(NoReplaceError::SourceChanged);
    }
    if !directory_source && verify_held_file_state(&mut held, &display, expected).is_err() {
        return Err(NoReplaceError::SourceChanged);
    }
    super::native_mutation_hook::during(source, old);
    let wide = std::ffi::OsStr::new(new).encode_wide().collect::<Vec<_>>();
    let name_bytes = wide
        .len()
        .checked_mul(2)
        .and_then(|len| u32::try_from(len).ok())
        .ok_or_else(|| {
            NoReplaceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "destination name is too long for FILE_RENAME_INFO",
            ))
        })?;
    let header = std::mem::size_of::<FileRenameInfo>();
    let total = header
        .checked_add(name_bytes as usize)
        .and_then(|len| len.checked_sub(std::mem::size_of::<u16>()))
        .ok_or_else(|| {
            NoReplaceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "FILE_RENAME_INFO allocation overflow",
            ))
        })?;
    let words = total.div_ceil(std::mem::size_of::<usize>());
    let total_u32 = u32::try_from(total).map_err(|_| {
        NoReplaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "FILE_RENAME_INFO length exceeds u32",
        ))
    })?;
    let mut storage = vec![0_usize; words];
    let info = storage.as_mut_ptr().cast::<FileRenameInfo>();
    // SAFETY: `storage` is usize-aligned and sized for the fixed header plus
    // every UTF-16 code unit. All pointers refer to live handles/capabilities
    // and buffers for the duration of the synchronous call; the OS retains
    // none of them. `replace_if_exists = 0` is the no-replace guarantee.
    let status = unsafe {
        (*info).replace_if_exists = 0;
        (*info).root_directory = destination.dir.as_raw_handle();
        (*info).file_name_length = name_bytes;
        std::ptr::copy_nonoverlapping(wide.as_ptr(), (*info).file_name.as_mut_ptr(), wide.len());
        NtSetInformationFile(
            held.as_raw_handle(),
            &mut io_status,
            info.cast(),
            total_u32,
            FILE_RENAME_INFORMATION_CLASS,
        )
    };
    if status >= 0 {
        drop(held);
        match source.dir.symlink_metadata(old) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(super::DirectoryDurability::JournalRecoverable)
            }
            Ok(_) => Err(NoReplaceError::SourceReappeared),
            Err(error) => Err(NoReplaceError::Io(error)),
        }
    } else {
        Err(classify(ntstatus_error(status), destination, new))
    }
}
