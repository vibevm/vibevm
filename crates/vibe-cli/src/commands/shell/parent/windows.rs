//! Windows: the parent process id from a ToolHelp snapshot, its image from
//! `QueryFullProcessImageNameW`, and the fallback `cmd.exe` from the system
//! directory. Declared directly against kernel32, as the proxy's supervision.

#![allow(unsafe_code)]

use std::ffi::{OsString, c_void};
use std::io;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

type Handle = *mut c_void;

const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x0000_1000;
const MAX_PATH: usize = 260;
const WIDE_CAPACITY: usize = 32_768;

/// `PROCESSENTRY32W` from `<tlhelp32.h>`.
#[repr(C)]
struct ProcessEntry {
    size: u32,
    usage: u32,
    process_id: u32,
    default_heap_id: usize,
    module_id: u32,
    threads: u32,
    parent_process_id: u32,
    priority_class_base: i32,
    flags: u32,
    exe_file: [u16; MAX_PATH],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> Handle;
    fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
    fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
    fn OpenProcess(access: u32, inherit: i32, process_id: u32) -> Handle;
    fn QueryFullProcessImageNameW(
        process: Handle,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
}

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: the ToolHelp snapshot and process-image queries have no standard-library wrapper; every buffer is owned and sized for the call and every handle is closed exactly once (spec://org.vibevm.core/vibevm/common/PROP-059#shell-command)."
)]
pub(super) fn parent_image() -> Option<PathBuf> {
    let own = std::process::id();
    // SAFETY: returns an owned snapshot handle or INVALID_HANDLE_VALUE (-1).
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot as isize == -1 {
        return None;
    }
    let mut entry = ProcessEntry {
        size: std::mem::size_of::<ProcessEntry>() as u32,
        usage: 0,
        process_id: 0,
        default_heap_id: 0,
        module_id: 0,
        threads: 0,
        parent_process_id: 0,
        priority_class_base: 0,
        flags: 0,
        exe_file: [0; MAX_PATH],
    };
    let mut parent = None;
    // SAFETY: the snapshot is live and `entry` is writable with `size` set.
    let mut present = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while present {
        if entry.process_id == own {
            parent = Some(entry.parent_process_id);
            break;
        }
        // SAFETY: same live snapshot and writable entry.
        present = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    // SAFETY: this function owns the snapshot and closes it once.
    unsafe { CloseHandle(snapshot) };
    let pid = parent.filter(|pid| *pid != 0)?;
    // SAFETY: opens a query-only handle, or returns null.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return None;
    }
    let mut wide = vec![0_u16; WIDE_CAPACITY];
    let mut length = WIDE_CAPACITY as u32;
    // SAFETY: `wide` is writable for `length` UTF-16 units; the handle is live.
    let ok = unsafe { QueryFullProcessImageNameW(process, 0, wide.as_mut_ptr(), &mut length) } != 0;
    // SAFETY: this function owns the process handle and closes it once.
    unsafe { CloseHandle(process) };
    let length = length as usize;
    if !ok || length == 0 || length >= WIDE_CAPACITY {
        return None;
    }
    wide.truncate(length);
    Some(PathBuf::from(OsString::from_wide(&wide)))
}

#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: GetSystemDirectoryW fills an owned bounded buffer so the cmd.exe fallback never comes from PATH or the environment (spec://org.vibevm.core/vibevm/common/PROP-059#shell-command)."
)]
pub(super) fn fallback() -> io::Result<PathBuf> {
    let mut wide = vec![0_u16; WIDE_CAPACITY];
    // SAFETY: `wide` is writable for its advertised capacity.
    let used = unsafe { GetSystemDirectoryW(wide.as_mut_ptr(), WIDE_CAPACITY as u32) } as usize;
    if used == 0 || used >= WIDE_CAPACITY {
        return Err(io::Error::last_os_error());
    }
    wide.truncate(used);
    Ok(PathBuf::from(OsString::from_wide(&wide)).join("cmd.exe"))
}
