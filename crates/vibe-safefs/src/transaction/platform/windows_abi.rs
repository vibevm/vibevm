#[cfg(windows)]
type Handle = *mut std::ffi::c_void;

#[cfg(windows)]
#[repr(C)]
struct FileRenameInfo {
    replace_if_exists: u8,
    root_directory: Handle,
    file_name_length: u32,
    file_name: [u16; 1],
}

#[cfg(windows)]
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}

#[cfg(windows)]
#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root_directory: Handle,
    object_name: *mut UnicodeString,
    attributes: u32,
    security_descriptor: *mut std::ffi::c_void,
    security_quality_of_service: *mut std::ffi::c_void,
}

#[cfg(windows)]
#[repr(C)]
struct IoStatusBlock {
    status_or_pointer: usize,
    information: usize,
}

#[cfg(windows)]
#[repr(C)]
struct FileDispositionInformation {
    delete_file: u8,
}

#[cfg(windows)]
const FILE_RENAME_INFORMATION_CLASS: u32 = 10;
#[cfg(windows)]
const FILE_DISPOSITION_INFORMATION_CLASS: u32 = 13;

#[cfg(windows)]
const DELETE: u32 = 0x0001_0000;
#[cfg(windows)]
const SYNCHRONIZE: u32 = 0x0010_0000;
#[cfg(windows)]
const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
#[cfg(windows)]
const FILE_READ_DATA: u32 = 0x0000_0001;
#[cfg(windows)]
const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
#[cfg(windows)]
const FILE_SHARE_READ: u32 = 0x0000_0001;
#[cfg(windows)]
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
#[cfg(windows)]
const FILE_OPEN: u32 = 0x0000_0001;
#[cfg(windows)]
const FILE_CREATE: u32 = 0x0000_0002;
#[cfg(windows)]
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
#[cfg(windows)]
const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;
#[cfg(windows)]
const FILE_NON_DIRECTORY_FILE: u32 = 0x0000_0040;
#[cfg(windows)]
const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x0000_0020;
#[cfg(windows)]
const FILE_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
#[cfg(windows)]
const OBJ_CASE_INSENSITIVE: u32 = 0x0000_0040;

#[cfg(windows)]
#[allow(unsafe_code)]
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtCreateFile(
        file_handle: *mut Handle,
        desired_access: u32,
        object_attributes: *mut ObjectAttributes,
        io_status_block: *mut IoStatusBlock,
        allocation_size: *mut i64,
        file_attributes: u32,
        share_access: u32,
        create_disposition: u32,
        create_options: u32,
        ea_buffer: *mut std::ffi::c_void,
        ea_length: u32,
    ) -> i32;
    fn RtlNtStatusToDosError(status: i32) -> u32;
    fn NtSetInformationFile(
        file: Handle,
        io_status_block: *mut IoStatusBlock,
        information: *mut std::ffi::c_void,
        length: u32,
        information_class: u32,
    ) -> i32;
}

#[cfg(windows)]
#[specmark::spec(
    deviates = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NATIVE-NOFOLLOW-BOUNDARY",
    reason = "the reviewed NT calls return NTSTATUS, so this one pure conversion preserves their typed Win32 I/O errors"
)]
#[allow(unsafe_code)]
fn ntstatus_error(status: i32) -> std::io::Error {
    // SAFETY: RtlNtStatusToDosError is a pure total conversion for the status
    // value just returned by NtCreateFile and retains no state or pointer.
    let code = unsafe { RtlNtStatusToDosError(status) };
    std::io::Error::from_raw_os_error(code as i32)
}

#[cfg(windows)]
fn verify_held_file_state(
    file: &mut std::fs::File,
    display: &std::path::Path,
    expected: &EntryState,
) -> Result<(), NoReplaceError> {
    use sha2::{Digest as _, Sha256};
    use std::io::{Read, Seek, SeekFrom};

    crate::file::verify_regular_single_link(file, display).map_err(|error| {
        NoReplaceError::Io(std::io::Error::other(format!(
            "verifying native rename source: {error:#}"
        )))
    })?;
    let opening = file.metadata().map_err(NoReplaceError::Io)?;
    let mut pass = || -> std::io::Result<(u64, String)> {
        file.seek(SeekFrom::Start(0))?;
        let mut digest = Sha256::new();
        let mut bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let used = file.read(&mut buffer)?;
            if used == 0 {
                return Ok((bytes, format!("{:x}", digest.finalize())));
            }
            bytes = bytes
                .checked_add(used as u64)
                .ok_or_else(|| std::io::Error::other("native rename source length exceeds u64"))?;
            digest.update(&buffer[..used]);
        }
    };
    let first = pass().map_err(NoReplaceError::Io)?;
    let second = pass().map_err(NoReplaceError::Io)?;
    let closing = file.metadata().map_err(NoReplaceError::Io)?;
    if first != second
        || first.0 != opening.len()
        || first.0 != closing.len()
        || expected.bytes != Some(first.0)
        || expected.sha256.as_deref() != Some(first.1.as_str())
    {
        return Err(NoReplaceError::Io(std::io::Error::other(
            "native rename source changed from its expected file state",
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn classify(error: std::io::Error, destination: &Pinned, new: &str) -> NoReplaceError {
    match error.kind() {
        std::io::ErrorKind::AlreadyExists => NoReplaceError::Occupied,
        std::io::ErrorKind::CrossesDevices => NoReplaceError::CrossFilesystem,
        std::io::ErrorKind::Unsupported => NoReplaceError::Unsupported,
        _ if occupied(destination, new) => NoReplaceError::Occupied,
        _ => NoReplaceError::Io(error),
    }
}

#[cfg(windows)]
fn occupied(destination: &Pinned, name: &str) -> bool {
    if destination
        .open_child_checked(name)
        .is_ok_and(|value| value.is_some())
    {
        return true;
    }
    let mut options = crate::file::cap_options();
    match destination.dir.open_with(name, options.read(true)) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

